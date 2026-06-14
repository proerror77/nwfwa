from pathlib import Path

from fastapi.testclient import TestClient

from app.main import app
from app.schemas import TrainRequest
from app.training_jobs import TrainingJobStore
from app.training_worker import run_worker
from training_fixtures import write_training_manifest


client = TestClient(app)


def test_training_api_returns_completed_training_package(tmp_path: Path):
    manifest_path = write_training_manifest(tmp_path)

    response = client.post(
        "/train",
        json={
            "manifest_path": str(manifest_path),
            "artifact_base_uri": str(tmp_path / "artifacts"),
            "model_key": "baseline_fwa",
            "base_model_version": "0.1.0",
            "job_id": "model_retraining_job_1",
            "actor": "trainer-worker",
            "algorithm": "logistic_regression",
        },
    )

    assert response.status_code == 200
    payload = response.json()
    assert payload["artifact_uri"].endswith("/rust_serving_artifact.json")
    assert payload["serving_manifest_uri"].endswith("/serving_manifest.json")
    assert payload["metrics_json"]["rule_mining_status"] == "passed"
    assert payload["mined_rule_owner"] == "external-training-platform"
    assert payload["mined_rule_candidates"][0]["scheme_family"] == "high_risk_claim"


def test_training_job_api_stores_completed_provider_output(tmp_path: Path):
    manifest_path = write_training_manifest(tmp_path)
    db_path = tmp_path / "training_jobs.sqlite3"
    app.state.training_job_store = TrainingJobStore(db_path)

    response = client.post(
        "/training-jobs",
        json={
            "manifest_path": str(manifest_path),
            "artifact_base_uri": str(tmp_path / "artifacts"),
            "model_key": "baseline_fwa",
            "base_model_version": "0.1.0",
            "job_id": "model_retraining_job_1",
            "actor": "external-training-platform",
            "algorithm": "logistic_regression",
        },
    )

    assert response.status_code == 202
    queued = response.json()
    assert queued["job_id"] == "model_retraining_job_1"
    assert queued["status"] == "queued"
    assert queued["handoff_kind"] == "external_training_platform_job"
    assert queued["provider_output"] is None
    assert queued["submit_path"] == "/api/v1/ops/model-retraining-jobs/model_retraining_job_1/output"

    status_response = client.get("/training-jobs/model_retraining_job_1")

    assert status_response.status_code == 200
    stored = status_response.json()
    assert stored["status"] == "completed"
    assert stored["provider_output"]["candidate_model_version"] == (
        "0.1.0-candidate-model_retraining_job_1"
    )
    assert stored["provider_output"]["mined_rule_owner"] == "external-training-platform"
    assert stored["provider_output"]["artifact_registry_uri"].endswith(
        "/artifact_registry.json"
    )
    assert any(
        ref == f"model_artifact_registries:{stored['provider_output']['artifact_registry_uri']}"
        for ref in stored["provider_output"]["evidence_refs"]
    )
    assert stored["artifact_registry"]["registry_kind"] == "training_artifact_registry"
    assert stored["artifact_registry"]["model_key"] == "baseline_fwa"
    assert stored["artifact_registry"]["base_model_version"] == "0.1.0"
    assert stored["artifact_registry"]["candidate_model_version"] == (
        "0.1.0-candidate-model_retraining_job_1"
    )
    assert {
        artifact["artifact_kind"] for artifact in stored["artifact_registry"]["artifacts"]
    } >= {
        "serving_model",
        "training_model",
        "serving_manifest",
        "validation_report",
        "model_artifact_evaluation",
        "permutation_importance",
        "shadow_report",
        "drift_report",
        "fairness_report",
        "mined_rule_candidates",
    }
    serving_artifact = next(
        artifact
        for artifact in stored["artifact_registry"]["artifacts"]
        if artifact["artifact_kind"] == "serving_model"
    )
    assert serving_artifact["storage_uri"] == stored["provider_output"]["artifact_uri"]
    assert serving_artifact["publish_status"] == "local_staging_available"
    assert serving_artifact["immutable"] is True
    assert Path(stored["provider_output"]["artifact_registry_uri"]).exists()
    assert stored["governance_boundary"].startswith("training platform owns training execution")

    app.state.training_job_store = TrainingJobStore(db_path)
    durable_response = client.get("/training-jobs/model_retraining_job_1")

    assert durable_response.status_code == 200
    assert durable_response.json() == stored


def test_training_job_api_persists_failed_training_job(tmp_path: Path):
    db_path = tmp_path / "training_jobs.sqlite3"
    app.state.training_job_store = TrainingJobStore(db_path)

    response = client.post(
        "/training-jobs",
        json={
            "manifest_path": str(tmp_path / "missing_manifest.json"),
            "artifact_base_uri": str(tmp_path / "artifacts"),
            "model_key": "baseline_fwa",
            "base_model_version": "0.1.0",
            "job_id": "failed_training_job",
            "actor": "external-training-platform",
            "algorithm": "logistic_regression",
            "max_attempts": 1,
        },
    )

    assert response.status_code == 202
    status_response = client.get("/training-jobs/failed_training_job")

    assert status_response.status_code == 200
    failed = status_response.json()
    assert failed["status"] == "failed"
    assert failed["provider_output"] is None
    assert failed["error"]["code"] == "TRAINING_FAILED"
    assert "missing_manifest.json" in failed["error"]["message"]


def test_training_job_retry_and_claim_lease_are_durable(tmp_path: Path):
    db_path = tmp_path / "training_jobs.sqlite3"
    store = TrainingJobStore(db_path)
    request = TrainRequest(
        manifest_path=str(tmp_path / "missing_manifest.json"),
        artifact_base_uri=str(tmp_path / "artifacts"),
        model_key="baseline_fwa",
        base_model_version="0.1.0",
        job_id="retry_training_job",
        actor="external-training-platform",
        algorithm="logistic_regression",
        max_attempts=2,
    )
    store.create_queued(request)

    first_claim = store.claim_next(worker_id="worker-a", lease_seconds=-1)
    assert first_claim["status"] == "running"
    assert first_claim["worker_id"] == "worker-a"
    assert first_claim["attempt_count"] == 1

    reclaimed = store.claim_next(worker_id="worker-b", lease_seconds=900)
    assert reclaimed["job_id"] == "retry_training_job"
    assert reclaimed["worker_id"] == "worker-b"
    assert reclaimed["attempt_count"] == 2

    stale_completion = store.mark_completed(
        "retry_training_job",
        "worker-a",
        {
            "candidate_model_version": "0.1.0-candidate-retry_training_job",
            "metrics_json": {"algorithm": "logistic_regression"},
        },
        {"registry_kind": "training_artifact_registry"},
    )
    assert stale_completion is None

    exhausted = store.mark_failed(
        "retry_training_job",
        "worker-b",
        {"code": "TRAINING_FAILED", "message": "boom"},
    )
    assert exhausted["status"] == "failed"
    assert exhausted["error"]["message"] == "boom"

    store = TrainingJobStore(db_path)
    assert store.claim_next(worker_id="worker-c", lease_seconds=900) is None
    assert store.get("retry_training_job")["status"] == "failed"


def test_training_job_claim_run_and_artifact_registry_endpoint(tmp_path: Path):
    manifest_path = write_training_manifest(tmp_path)
    db_path = tmp_path / "training_jobs.sqlite3"
    store = TrainingJobStore(db_path)
    app.state.training_job_store = store
    store.create_queued(
        TrainRequest(
            manifest_path=str(manifest_path),
            artifact_base_uri=str(tmp_path / "artifacts"),
            model_key="baseline_fwa",
            base_model_version="0.1.0",
            job_id="claimed_training_job",
            actor="external-training-platform",
            algorithm="logistic_regression",
        )
    )

    claim_response = client.post(
        "/training-jobs/claim-next",
        json={"worker_id": "trainer-worker", "lease_seconds": 900},
    )
    assert claim_response.status_code == 200
    claimed = claim_response.json()
    assert claimed["job_id"] == "claimed_training_job"
    assert claimed["status"] == "running"
    assert claimed["worker_id"] == "trainer-worker"

    run_response = client.post(
        "/training-jobs/claimed_training_job/run",
        json={"worker_id": "trainer-worker", "lease_seconds": 900},
    )
    assert run_response.status_code == 200
    completed = run_response.json()
    assert completed["status"] == "completed"
    assert completed["provider_output"]["candidate_model_version"] == (
        "0.1.0-candidate-claimed_training_job"
    )

    artifacts_response = client.get("/training-jobs/claimed_training_job/artifacts")
    assert artifacts_response.status_code == 200
    artifact_registry = artifacts_response.json()["artifact_registry"]
    assert artifact_registry["job_id"] == "claimed_training_job"
    assert artifact_registry["artifact_registry_uri"].endswith("/artifact_registry.json")

    registries_response = client.get("/artifact-registries?model_key=baseline_fwa")
    assert registries_response.status_code == 200
    registries = registries_response.json()["registries"]
    assert registries[0]["job_id"] == "claimed_training_job"
    assert registries[0]["artifact_count"] == len(artifact_registry["artifacts"])

    version_response = client.get(
        "/artifact-registries/baseline_fwa/0.1.0-candidate-claimed_training_job"
    )
    assert version_response.status_code == 200
    assert version_response.json()["artifact_registry"]["job_id"] == "claimed_training_job"


def test_training_worker_once_processes_durable_queue(tmp_path: Path):
    manifest_path = write_training_manifest(tmp_path)
    store = TrainingJobStore(tmp_path / "training_jobs.sqlite3")
    store.create_queued(
        TrainRequest(
            manifest_path=str(manifest_path),
            artifact_base_uri=str(tmp_path / "artifacts"),
            model_key="baseline_fwa",
            base_model_version="0.1.0",
            job_id="worker_training_job",
            actor="external-training-platform",
            algorithm="logistic_regression",
        )
    )

    result = run_worker(
        store=store,
        worker_id="trainer-daemon-1",
        lease_seconds=900,
        poll_interval_seconds=0.01,
        once=True,
    )

    assert result["status"] == "processed"
    assert result["processed_jobs"] == 1
    assert result["last_job"]["status"] == "completed"
    stored = store.get("worker_training_job")
    assert stored["status"] == "completed"
    assert stored["provider_output"]["candidate_model_version"] == (
        "0.1.0-candidate-worker_training_job"
    )
    assert stored["artifact_registry"]["artifact_registry_uri"].endswith(
        "/artifact_registry.json"
    )


def test_training_worker_once_reports_idle_queue(tmp_path: Path):
    store = TrainingJobStore(tmp_path / "training_jobs.sqlite3")

    result = run_worker(
        store=store,
        worker_id="trainer-daemon-1",
        lease_seconds=900,
        poll_interval_seconds=0.01,
        once=True,
    )

    assert result["status"] == "idle"
    assert result["processed_jobs"] == 0
    assert result["idle_polls"] == 1
    assert result["last_job"] is None

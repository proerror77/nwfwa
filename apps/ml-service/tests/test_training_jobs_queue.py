from pathlib import Path

from fastapi.testclient import TestClient

from app.main import app
from app.schemas import TrainRequest
from app.training_jobs import TrainingJobStore
from app.training_worker import run_worker


client = TestClient(app)


def training_request(
    tmp_path: Path,
    job_id: str,
    max_attempts: int = 2,
    retry_delay_seconds: int = 60,
) -> TrainRequest:
    return TrainRequest(
        manifest_path=str(tmp_path / "manifest.json"),
        artifact_base_uri=str(tmp_path / "artifacts"),
        model_key="baseline_fwa",
        base_model_version="0.1.0",
        job_id=job_id,
        actor="external-training-platform",
        algorithm="logistic_regression",
        max_attempts=max_attempts,
        retry_delay_seconds=retry_delay_seconds,
    )


def test_training_job_failure_waits_for_retry_backoff(tmp_path: Path):
    store = TrainingJobStore(tmp_path / "training_jobs.sqlite3")
    store.create_queued(
        training_request(tmp_path, "backoff_job", retry_delay_seconds=3600)
    )
    claimed = store.claim_next(worker_id="worker-a", lease_seconds=900)

    queued = store.mark_failed(
        "backoff_job",
        "worker-a",
        {"code": "TRAINING_FAILED", "message": "temporary"},
    )

    assert queued["status"] == "queued"
    assert queued["attempt_count"] == 1
    assert queued["next_attempt_at"] is not None
    assert queued["dead_letter_at"] is None
    assert claimed["job_id"] == "backoff_job"
    assert store.claim_next(worker_id="worker-b", lease_seconds=900) is None


def test_training_job_failure_without_backoff_can_retry_immediately(tmp_path: Path):
    store = TrainingJobStore(tmp_path / "training_jobs.sqlite3")
    store.create_queued(
        training_request(tmp_path, "immediate_retry_job", retry_delay_seconds=0)
    )
    store.claim_job("immediate_retry_job", worker_id="worker-a", lease_seconds=900)
    store.mark_failed(
        "immediate_retry_job",
        "worker-a",
        {"code": "TRAINING_FAILED", "message": "temporary"},
    )

    retry = store.claim_next(worker_id="worker-b", lease_seconds=900)

    assert retry["job_id"] == "immediate_retry_job"
    assert retry["worker_id"] == "worker-b"
    assert retry["attempt_count"] == 2


def test_training_job_exhaustion_records_dead_letter(tmp_path: Path):
    store = TrainingJobStore(tmp_path / "training_jobs.sqlite3")
    store.create_queued(
        training_request(tmp_path, "dead_letter_job", max_attempts=1)
    )
    store.claim_job("dead_letter_job", worker_id="worker-a", lease_seconds=900)

    failed = store.mark_failed(
        "dead_letter_job",
        "worker-a",
        {"code": "TRAINING_FAILED", "message": "permanent"},
    )

    assert failed["status"] == "failed"
    assert failed["dead_letter_at"] is not None
    assert failed["next_attempt_at"] is None
    assert store.claim_next(worker_id="worker-b", lease_seconds=900) is None


def test_training_job_lease_renewal_blocks_reclaim(tmp_path: Path):
    store = TrainingJobStore(tmp_path / "training_jobs.sqlite3")
    store.create_queued(training_request(tmp_path, "renew_job"))
    store.claim_job("renew_job", worker_id="worker-a", lease_seconds=900)

    renewed = store.renew_lease("renew_job", "worker-a", lease_seconds=900)

    assert renewed["status"] == "running"
    assert renewed["worker_id"] == "worker-a"
    assert store.claim_next(worker_id="worker-b", lease_seconds=900) is None


def test_training_job_retry_endpoint_requeues_dead_letter(tmp_path: Path):
    store = TrainingJobStore(tmp_path / "training_jobs.sqlite3")
    app.state.training_job_store = store
    store.create_queued(training_request(tmp_path, "manual_retry_job", max_attempts=1))
    store.claim_next(worker_id="worker-a", lease_seconds=900)
    store.mark_failed(
        "manual_retry_job",
        "worker-a",
        {"code": "TRAINING_FAILED", "message": "permanent"},
    )

    response = client.post("/training-jobs/manual_retry_job/retry")

    assert response.status_code == 200
    requeued = response.json()
    assert requeued["status"] == "queued"
    assert requeued["attempt_count"] == 0
    assert requeued["dead_letter_at"] is None
    assert requeued["next_attempt_at"] is None


def test_training_queue_metrics_and_worker_heartbeat(tmp_path: Path):
    store = TrainingJobStore(tmp_path / "training_jobs.sqlite3")
    store.create_queued(training_request(tmp_path, "ready_job"))
    store.create_queued(
        training_request(tmp_path, "delayed_job", retry_delay_seconds=3600)
    )
    store.claim_job("delayed_job", worker_id="worker-a", lease_seconds=900)
    store.mark_failed(
        "delayed_job",
        "worker-a",
        {"code": "TRAINING_FAILED", "message": "temporary"},
    )
    store.create_queued(training_request(tmp_path, "dead_letter_job", max_attempts=1))
    store.claim_job("dead_letter_job", worker_id="worker-b", lease_seconds=900)
    store.mark_failed(
        "dead_letter_job",
        "worker-b",
        {"code": "TRAINING_FAILED", "message": "permanent"},
    )

    heartbeat = store.record_heartbeat(
        "worker-a",
        "idle",
        processed_jobs=1,
        idle_polls=2,
        metadata={"hostname": "local"},
    )
    metrics = store.queue_metrics()

    assert heartbeat["worker_id"] == "worker-a"
    assert heartbeat["metadata"]["hostname"] == "local"
    assert metrics["jobs_by_status"]["queued"] == 2
    assert metrics["jobs_by_status"]["failed"] == 1
    assert metrics["ready_jobs"] == 1
    assert metrics["delayed_jobs"] == 1
    assert metrics["dead_letter_jobs"] == 1
    assert metrics["registered_workers"] == 1
    assert store.list_workers()[0]["worker_id"] == "worker-a"


def test_training_metrics_and_workers_api(tmp_path: Path):
    store = TrainingJobStore(tmp_path / "training_jobs.sqlite3")
    app.state.training_job_store = store
    store.create_queued(training_request(tmp_path, "api_ready_job"))

    heartbeat_response = client.post(
        "/training-workers/heartbeat",
        json={
            "worker_id": "api-worker",
            "status": "idle",
            "processed_jobs": 3,
            "idle_polls": 1,
            "metadata": {"pod": "worker-0"},
        },
    )
    metrics_response = client.get("/training-jobs/metrics")
    prometheus_response = client.get("/metrics")
    workers_response = client.get("/training-workers")

    assert heartbeat_response.status_code == 200
    assert heartbeat_response.json()["metadata"]["pod"] == "worker-0"
    assert metrics_response.status_code == 200
    assert metrics_response.json()["ready_jobs"] == 1
    assert metrics_response.json()["registered_workers"] == 1
    assert prometheus_response.status_code == 200
    assert 'fwa_ml_training_jobs{status="queued"} 1' in prometheus_response.text
    assert "fwa_ml_training_jobs_ready 1" in prometheus_response.text
    assert 'fwa_ml_training_workers{status="idle"} 1' in prometheus_response.text
    assert workers_response.status_code == 200
    assert workers_response.json()["workers"][0]["worker_id"] == "api-worker"


def test_training_worker_idle_run_records_heartbeat(tmp_path: Path):
    store = TrainingJobStore(tmp_path / "training_jobs.sqlite3")

    result = run_worker(
        store=store,
        worker_id="idle-daemon",
        lease_seconds=900,
        poll_interval_seconds=0.01,
        once=True,
    )

    worker = store.get_worker("idle-daemon")
    assert result["status"] == "idle"
    assert worker["status"] == "idle"
    assert worker["idle_polls"] == 1

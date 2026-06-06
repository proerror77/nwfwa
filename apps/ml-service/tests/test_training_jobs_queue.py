from pathlib import Path

from fastapi.testclient import TestClient

from app.main import app
from app.schemas import TrainRequest
from app.training_jobs import TrainingJobStore


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
    store.claim_next(worker_id="worker-a", lease_seconds=900)
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
    store.claim_next(worker_id="worker-a", lease_seconds=900)

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
    store.claim_next(worker_id="worker-a", lease_seconds=900)

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

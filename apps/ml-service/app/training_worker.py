from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path
from typing import Any

from .training_jobs import TrainingJobStore
from .training_runner import execute_next_training_job


def run_worker(
    store: TrainingJobStore,
    worker_id: str,
    lease_seconds: int,
    poll_interval_seconds: float,
    once: bool,
    max_jobs: int | None = None,
) -> dict[str, Any]:
    processed = 0
    idle_polls = 0
    last_job = None
    store.record_heartbeat(
        worker_id,
        "starting",
        metadata={"lease_seconds": lease_seconds, "once": once},
    )
    while True:
        store.record_heartbeat(
            worker_id,
            "polling",
            processed_jobs=processed,
            idle_polls=idle_polls,
        )
        last_job = execute_next_training_job(store, worker_id, lease_seconds)
        if last_job is None:
            idle_polls += 1
            store.record_heartbeat(
                worker_id,
                "idle",
                processed_jobs=processed,
                idle_polls=idle_polls,
            )
            if once:
                break
            time.sleep(poll_interval_seconds)
            continue
        processed += 1
        store.record_heartbeat(
            worker_id,
            last_job["status"],
            current_job_id=last_job["job_id"],
            processed_jobs=processed,
            idle_polls=idle_polls,
        )
        if once or (max_jobs is not None and processed >= max_jobs):
            break

    return {
        "worker_id": worker_id,
        "status": "processed" if processed else "idle",
        "processed_jobs": processed,
        "idle_polls": idle_polls,
        "last_job": last_job,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run the independent ML training worker against the durable job queue."
    )
    parser.add_argument(
        "--db",
        default=os.getenv("FWA_TRAINING_JOB_DB", "data/ml-service/training_jobs.sqlite3"),
        help="SQLite training job queue path.",
    )
    parser.add_argument("--worker-id", default="ml-training-worker")
    parser.add_argument("--lease-seconds", type=int, default=900)
    parser.add_argument("--poll-interval-seconds", type=float, default=5.0)
    parser.add_argument(
        "--once",
        action="store_true",
        help="Process at most one available job and exit.",
    )
    parser.add_argument(
        "--max-jobs",
        type=int,
        help="Process at most this many jobs before exiting.",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(sys.argv[1:] if argv is None else argv)
    if args.max_jobs is not None and args.max_jobs < 1:
        print(json.dumps({"error": "--max-jobs must be at least 1"}), file=sys.stderr)
        return 2
    store = TrainingJobStore(Path(args.db))
    result = run_worker(
        store=store,
        worker_id=args.worker_id,
        lease_seconds=args.lease_seconds,
        poll_interval_seconds=args.poll_interval_seconds,
        once=args.once,
        max_jobs=args.max_jobs,
    )
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
import json
import sys


def require(condition: bool, message: str) -> None:
    if not condition:
        raise SystemExit(message)


def main() -> None:
    payload = json.load(sys.stdin)
    require(payload.get("status") == "ok", "worker health status must be ok")
    require(payload.get("service") == "worker", "worker health service must be worker")
    require(payload.get("version"), "worker health version is required")

    checks = payload.get("checks")
    require(isinstance(checks, list), "worker health checks must be a list")
    check_status = {
        check.get("name"): check.get("status")
        for check in checks
        if isinstance(check, dict)
    }
    for name in [
        "cli_commands",
        "parquet_profiler",
        "retraining_job_runner",
        "pilot_readiness_checker",
        "analytics_export_plan",
        "ai_evidence_execution_plan",
        "governance_ops_plan",
    ]:
        require(check_status.get(name) == "ok", f"worker health check {name} must be ok")


if __name__ == "__main__":
    main()

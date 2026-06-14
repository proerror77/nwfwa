#!/usr/bin/env python3
"""Validate the production readiness evidence contract."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_GATE_IDS = {
    "production_deployment_apply",
    "production_smoke",
    "observability_stack",
    "secret_and_access_governance",
    "backup_restore_drill",
    "rollback_drill",
    "alert_router_delivery",
    "retention_legal_hold",
    "customer_data_governance",
    "worker_data_pipeline_execution",
    "model_serving_slo",
    "ocr_vector_analytics_execution",
}

CUSTOMER_DATA_REQUIRED_GATE_IDS = {
    "customer_data_governance",
    "worker_data_pipeline_execution",
    "model_serving_slo",
    "ocr_vector_analytics_execution",
}

WORKER_DATA_PIPELINE_REQUIRED_JOB_KINDS = {
    "oig_sam_sanctions_snapshot_fetch",
    "oig_sam_sanctions_sync",
    "provider_profile_window_rollup",
    "provider_graph_signal_rollup",
    "peer_percentile_benchmark",
    "episode_aggregation",
    "clinical_compatibility_reference",
    "unbundling_comparator",
    "scoring_feature_context_materialization",
    "probability_calibration_evidence",
}

WORKER_DATA_PIPELINE_ACCEPTANCE_CHECK_IDS = {
    "report_kind_is_worker_data_pipeline_execution_report",
    "readiness_gate_status_ready",
    "scheduler_status_completed",
    "pending_or_failed_job_count_zero",
    "review_task_count_zero",
    "required_job_kinds_completed",
    "evidence_refs_include_plan_run_status_and_readiness",
    "governance_boundary_no_adjudication",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise AssertionError(f"missing JSON artifact: {path}") from exc


def validate_contract(contract: dict) -> None:
    require(
        contract.get("artifact_kind") == "production_readiness_evidence_contract",
        "wrong production readiness contract artifact kind",
    )
    require(
        contract.get("status") == "blocked_until_live_environment_evidence",
        "contract must not claim production readiness without live evidence",
    )
    gates = contract.get("required_gates")
    require(isinstance(gates, list) and gates, "required_gates must be non-empty")
    gate_ids = {gate.get("gate_id") for gate in gates if isinstance(gate, dict)}
    require(REQUIRED_GATE_IDS == gate_ids, "production readiness contract gate set changed unexpectedly")
    for gate in gates:
        require(gate.get("required_artifact", "").endswith(".json"), f"gate {gate.get('gate_id')} missing JSON artifact")
        require(gate.get("description"), f"gate {gate.get('gate_id')} missing description")
        require(
            gate.get("status") == "requires_customer_environment_evidence",
            f"gate {gate.get('gate_id')} must require customer environment evidence",
        )
        require(
            bool(gate.get("customer_data_required"))
            == (gate.get("gate_id") in CUSTOMER_DATA_REQUIRED_GATE_IDS),
            f"gate {gate.get('gate_id')} has wrong customer_data_required flag",
        )
        if gate.get("gate_id") == "worker_data_pipeline_execution":
            required_job_kinds = set(gate.get("required_job_kinds", []))
            require(
                required_job_kinds == WORKER_DATA_PIPELINE_REQUIRED_JOB_KINDS,
                "worker data pipeline gate required_job_kinds changed unexpectedly",
            )
            acceptance_checks = gate.get("acceptance_checks")
            require(
                isinstance(acceptance_checks, list) and acceptance_checks,
                "worker data pipeline gate missing acceptance_checks",
            )
            check_ids = {
                check.get("check_id")
                for check in acceptance_checks
                if isinstance(check, dict)
            }
            require(
                check_ids == WORKER_DATA_PIPELINE_ACCEPTANCE_CHECK_IDS,
                "worker data pipeline gate acceptance check set changed unexpectedly",
            )
            for check in acceptance_checks:
                require(
                    check.get("description"),
                    f"worker data pipeline acceptance check {check.get('check_id')} missing description",
                )


def validate_worker_data_pipeline_execution_evidence(report: dict) -> None:
    require(
        report.get("report_kind") == "worker_data_pipeline_execution_report",
        "worker data pipeline execution evidence has wrong report_kind",
    )
    require(
        report.get("readiness_gate_status") == "ready",
        "worker data pipeline execution evidence must have readiness_gate_status ready",
    )
    require(
        report.get("scheduler_status") == "completed",
        "worker data pipeline execution evidence must have scheduler_status completed",
    )
    require(
        report.get("pending_or_failed_job_count") == 0,
        "worker data pipeline execution evidence must have zero pending or failed jobs",
    )
    require(
        report.get("review_task_count") == 0,
        "worker data pipeline execution evidence must have zero review tasks",
    )
    job_executions = report.get("job_executions")
    require(
        isinstance(job_executions, list) and job_executions,
        "worker data pipeline execution evidence must include job_executions",
    )
    require(
        report.get("job_count") == len(job_executions),
        "worker data pipeline execution evidence job_count must match job_executions",
    )
    jobs_by_kind = {
        job.get("job_kind"): job for job in job_executions if isinstance(job, dict)
    }
    require(
        set(jobs_by_kind) == WORKER_DATA_PIPELINE_REQUIRED_JOB_KINDS,
        "worker data pipeline execution evidence job kind set changed unexpectedly",
    )
    for job_kind, job in jobs_by_kind.items():
        require(
            job.get("execution_status") == "completed",
            f"worker data pipeline job {job_kind} must be completed",
        )
    evidence_refs = report.get("evidence_refs")
    require(
        isinstance(evidence_refs, list) and evidence_refs,
        "worker data pipeline execution evidence must include evidence_refs",
    )
    for prefix in (
        "worker_data_pipeline_plans:",
        "worker_data_pipeline_run_status:",
        "worker_data_pipeline_readiness_reports:",
    ):
        require(
            any(isinstance(reference, str) and reference.startswith(prefix) for reference in evidence_refs),
            f"worker data pipeline execution evidence_refs missing {prefix}",
        )
    governance_boundary = report.get("governance_boundary", "")
    for forbidden_side_effect in (
        "score claims",
        "assign labels",
        "deny claims",
        "activate models",
        "change routing policy",
    ):
        require(
            forbidden_side_effect in governance_boundary,
            f"worker data pipeline governance boundary missing {forbidden_side_effect}",
        )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--contract-dir", default="artifacts/production-readiness")
    parser.add_argument(
        "--evidence-dir",
        help="Optional directory containing production evidence artifacts to validate.",
    )
    args = parser.parse_args()

    contract_dir = Path(args.contract_dir)
    validate_contract(load_json(contract_dir / "production_readiness_contract.json"))
    index = load_json(contract_dir / "index.json")
    require(index.get("artifact_kind") == "production_readiness_contract_index", "wrong index artifact kind")
    if args.evidence_dir:
        evidence_dir = Path(args.evidence_dir)
        validate_worker_data_pipeline_execution_evidence(
            load_json(evidence_dir / "worker_data_pipeline_execution_report.json")
        )
    print("production readiness contract validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"production readiness contract validation failed: {exc}")
        raise SystemExit(1)

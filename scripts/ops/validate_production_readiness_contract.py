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


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--contract-dir", default="artifacts/production-readiness")
    args = parser.parse_args()

    contract_dir = Path(args.contract_dir)
    validate_contract(load_json(contract_dir / "production_readiness_contract.json"))
    index = load_json(contract_dir / "index.json")
    require(index.get("artifact_kind") == "production_readiness_contract_index", "wrong index artifact kind")
    print("production readiness contract validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"production readiness contract validation failed: {exc}")
        raise SystemExit(1)

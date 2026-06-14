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

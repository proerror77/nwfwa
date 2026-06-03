#!/usr/bin/env python3
"""Validate staging operational drill proof artifacts."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


REQUIRED_DRILLS = {
    "pilot_readiness_gate_drill",
    "restore_drill",
    "rollback_drill",
    "alert_route_drill",
    "incident_tabletop_drill",
}

REQUIRED_CHECKS = {
    "restore_drill_contract_declared",
    "rollback_drill_contract_declared",
    "alert_route_drill_contract_declared",
    "incident_tabletop_contract_declared",
    "destructive_actions_plan_only",
}

REQUIRED_ALERT_EVIDENCE = {
    "observability_proof.json",
    "alert_routes",
    "actor_role",
    "customer_scope_id",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise AssertionError(f"missing operational drill proof: {path}") from exc


def validate_drills(payload: dict) -> None:
    drills = payload.get("drills")
    require(isinstance(drills, list), "drills must be a list")
    drill_by_name = {drill.get("name"): drill for drill in drills if isinstance(drill, dict)}
    require(set(drill_by_name) == REQUIRED_DRILLS, "operational drill proof must declare the required drill set")

    for name, drill in drill_by_name.items():
        require(drill.get("trigger"), f"{name} missing trigger")
        evidence = drill.get("expected_evidence")
        require(isinstance(evidence, list) and evidence, f"{name} missing expected evidence")
        require(drill.get("pass_criteria"), f"{name} missing pass criteria")

    alert_evidence = set(drill_by_name["alert_route_drill"]["expected_evidence"])
    require(
        REQUIRED_ALERT_EVIDENCE.issubset(alert_evidence),
        "alert route drill must carry route, role, customer scope, and observability evidence",
    )


def validate_checks(payload: dict) -> None:
    checks = payload.get("checks")
    require(isinstance(checks, list), "checks must be a list")
    check_by_name = {check.get("name"): check for check in checks if isinstance(check, dict)}
    require(REQUIRED_CHECKS.issubset(set(check_by_name)), "operational drill proof missing required checks")
    for check_name in REQUIRED_CHECKS:
        require(check_by_name[check_name].get("status") == "passed", f"{check_name} must pass")


def validate_payload(payload: dict) -> None:
    require(payload.get("artifact_kind") == "staging_operational_drill_proof", "wrong artifact kind")
    validate_drills(payload)
    validate_checks(payload)
    require(payload.get("destructive_actions") == "plan_only", "destructive actions must remain plan_only")
    require(payload.get("human_approval_required") is True, "human approval must be required")
    boundary = payload.get("boundary", "")
    require("customer environments" in boundary, "boundary must mention customer environment execution")
    require("sign off" in boundary, "boundary must mention customer sign-off")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--proof-dir", default="artifacts/staging-proof")
    parser.add_argument("--proof-file", default="operational_drill_proof.json")
    args = parser.parse_args()

    proof_path = Path(args.proof_dir) / args.proof_file
    validate_payload(load_json(proof_path))
    print("operational drill proof validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"operational drill proof validation failed: {exc}")
        raise SystemExit(1)

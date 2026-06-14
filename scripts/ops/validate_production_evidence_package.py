#!/usr/bin/env python3
"""Validate the customer-fillable production evidence package template."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.ops.validate_production_readiness_contract import (
    validate_contract,
    validate_evidence_dir,
)


DEFAULT_PACKAGE_DIR = Path("artifacts/production-evidence-package")
REQUIRED_SOURCE_TEMPLATES = {
    "sources/customer-data-governance-source.json",
    "sources/retention-legal-hold-source.json",
    "sources/model-serving-slo-source.json",
    "sources/ocr-vector-analytics-source.json",
}
REQUIRED_WORKER_TEMPLATES = {
    "worker/score_request.json",
    "worker/scoring_readback_input.json",
    "worker/worker_data_pipeline_readiness_input.json",
    "worker/worker_data_pipeline_run_status.json",
}
REQUIRED_RUNBOOKS = {"runbooks/worker-data-pipeline-commands.json"}
FORBIDDEN_TEMPLATE_STRINGS = (
    "api_key",
    "patientName",
    "certificateNo",
    "insuredName",
    "accidentPersonName",
)


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def validate_package(package_dir: Path) -> dict:
    index = load_json(package_dir / "index.json")
    require(
        index.get("artifact_kind") == "production_readiness_evidence_package_template",
        "production evidence package has wrong artifact_kind",
    )
    require(
        index.get("status") == "blocked_until_customer_artifacts_are_filled",
        "production evidence package must remain blocked until customer artifacts are filled",
    )
    require(index.get("readiness_claim") is False, "package template must not claim readiness")

    contract_dir = package_dir / index.get("contract_dir", "")
    evidence_dir = package_dir / index.get("evidence_dir", "")
    gates = validate_contract(load_json(contract_dir / "production_readiness_contract.json"))
    artifacts = validate_evidence_dir(evidence_dir, gates)
    require(
        index.get("artifact_count") == len(artifacts),
        "index artifact_count must match required evidence templates",
    )

    validate_index_entries(package_dir, index, "source_templates", REQUIRED_SOURCE_TEMPLATES)
    validate_index_entries(package_dir, index, "worker_templates", REQUIRED_WORKER_TEMPLATES)
    validate_index_entries(package_dir, index, "runbooks", REQUIRED_RUNBOOKS)
    validate_command_includes_package_validator(index.get("validation_command"), "index")
    validate_worker_templates(package_dir)
    validate_runbook(package_dir)
    validate_render_summary_if_present(package_dir, index)

    summary = {
        "artifact_kind": "production_evidence_package_validation",
        "package_dir": str(package_dir),
        "status": "valid_template_package",
        "readiness_claim": False,
        "artifact_count": len(artifacts),
        "source_template_count": len(index.get("source_templates") or []),
        "worker_template_count": len(index.get("worker_templates") or []),
        "runbook_count": len(index.get("runbooks") or []),
    }
    return summary


def validate_index_entries(
    package_dir: Path, index: dict, field_name: str, required_paths: set[str]
) -> None:
    entries = index.get(field_name)
    require(isinstance(entries, list), f"index {field_name} must be a list")
    observed_paths = set()
    for entry in entries:
        path_value = entry.get("source") or entry.get("template") or entry.get("runbook")
        require(isinstance(path_value, str) and path_value, f"{field_name} entry missing path")
        observed_paths.add(path_value)
        path = package_dir / path_value
        require(path.exists(), f"{field_name} path missing: {path_value}")
        load_json(path)
        require(
            entry.get("status") in {"pending_customer_input", "pending_customer_execution"},
            f"{field_name} entry has unexpected status for {path_value}",
        )
    require(
        required_paths.issubset(observed_paths),
        f"index {field_name} missing required paths: {sorted(required_paths - observed_paths)}",
    )


def validate_worker_templates(package_dir: Path) -> None:
    score_request_uri = package_dir / "worker" / "score_request.json"
    score_request = load_json(score_request_uri)
    require(
        score_request.get("artifact_kind") == "scoring_readback_score_request_template",
        "score_request.json has wrong artifact_kind",
    )
    require(score_request.get("review_mode") == "pre_payment", "score request must be pre_payment")

    readback_input = load_json(package_dir / "worker" / "scoring_readback_input.json")
    require(
        readback_input.get("artifact_kind") == "scoring_readback_input_template",
        "scoring_readback_input.json has wrong artifact_kind",
    )
    expected_prefixes = readback_input.get("expected_evidence_prefixes")
    require(
        isinstance(expected_prefixes, list) and expected_prefixes,
        "scoring readback input requires expected_evidence_prefixes",
    )
    for prefix in expected_prefixes:
        require(
            isinstance(prefix, str) and prefix.endswith(":"),
            f"scoring readback expected prefix must end with colon: {prefix}",
        )

    readiness_input = load_json(package_dir / "worker" / "worker_data_pipeline_readiness_input.json")
    require(
        readiness_input.get("artifact_kind") == "worker_data_pipeline_readiness_input_template",
        "worker readiness input has wrong artifact_kind",
    )
    require(readiness_input.get("checks"), "worker readiness input requires checks")

    run_status = load_json(package_dir / "worker" / "worker_data_pipeline_run_status.json")
    require(
        run_status.get("report_kind") == "worker_data_pipeline_run_status",
        "worker run status template has wrong report_kind",
    )
    require(run_status.get("run_status_template") is True, "run status must remain a template")

    for path in REQUIRED_WORKER_TEMPLATES:
        template_text = (package_dir / path).read_text(encoding="utf-8")
        for forbidden in FORBIDDEN_TEMPLATE_STRINGS:
            require(
                forbidden not in template_text,
                f"worker template {path} contains forbidden placeholder string {forbidden}",
            )


def validate_runbook(package_dir: Path) -> None:
    runbook = load_json(package_dir / "runbooks" / "worker-data-pipeline-commands.json")
    require(
        runbook.get("artifact_kind") == "worker_data_pipeline_command_runbook",
        "worker data-pipeline runbook has wrong artifact_kind",
    )
    validate_command_includes_package_validator(runbook.get("validation_command"), "runbook")


def validate_command_includes_package_validator(command: object, owner: str) -> None:
    require(isinstance(command, str) and command, f"{owner} validation_command is required")
    require(
        "validate_production_evidence_package.py" in command,
        f"{owner} validation_command must run validate_production_evidence_package.py",
    )
    require(
        "validate_production_readiness_contract.py" in command,
        f"{owner} validation_command must run validate_production_readiness_contract.py",
    )


def validate_render_summary_if_present(package_dir: Path, index: dict) -> None:
    render_summary_uri = package_dir / "render_summary.json"
    if not render_summary_uri.exists():
        return
    summary = load_json(render_summary_uri)
    require(
        summary.get("artifact_kind") == "production_evidence_package_render_summary",
        "render summary has wrong artifact_kind",
    )
    require(summary.get("readiness_claim") is False, "render summary must not claim readiness")
    require(
        summary.get("worker_template_count") == len(index.get("worker_templates") or []),
        "render summary worker_template_count must match index",
    )
    require(
        summary.get("pending_worker_template_count") == len(index.get("worker_templates") or []),
        "render summary must keep worker templates pending until customer execution",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-dir", default=str(DEFAULT_PACKAGE_DIR))
    args = parser.parse_args()
    print(json.dumps(validate_package(Path(args.package_dir)), indent=2, sort_keys=True))
    print("production evidence package validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

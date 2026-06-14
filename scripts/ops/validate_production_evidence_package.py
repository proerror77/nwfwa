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
REQUIRED_RUNBOOK_STEPS = {
    "build_worker_data_pipeline_plan": (
        "build-worker-data-pipeline-plan",
        "worker/worker_data_pipeline_plan.json",
    ),
    "build_readiness_input_template": (
        "build-worker-data-pipeline-readiness-input-template",
        "worker/worker_data_pipeline_readiness_input_template.json",
    ),
    "build_readiness_report": (
        "build-worker-data-pipeline-readiness-report",
        "worker/worker_data_pipeline_readiness_report.json",
    ),
    "build_run_status_template": (
        "build-worker-data-pipeline-run-status-template",
        "worker/worker_data_pipeline_run_status_template.json",
    ),
    "build_execution_report": (
        "build-worker-data-pipeline-execution-report",
        "evidence/worker_data_pipeline_execution_report.json",
    ),
    "fetch_score_response": (
        "fetch-scoring-readback-response",
        "worker/scoring-readback/score_response.json",
    ),
    "build_scoring_readback_report": (
        "build-scoring-readback-report",
        "evidence/scoring_readback_report.json",
    ),
}
REQUIRED_RUNBOOK_COMMAND_PATHS = {
    "artifacts/production-evidence-package/worker/worker_data_pipeline_plan.json",
    "artifacts/production-evidence-package/worker/worker_data_pipeline_readiness_input.json",
    "artifacts/production-evidence-package/worker/worker_data_pipeline_readiness_report.json",
    "artifacts/production-evidence-package/worker/worker_data_pipeline_run_status.json",
    "artifacts/production-evidence-package/worker/score_request.json",
    "artifacts/production-evidence-package/worker/scoring_readback_input.json",
    "artifacts/production-evidence-package/worker/scoring-readback/score_response.json",
}
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
    validate_evidence_templates(artifacts)

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


def validate_evidence_templates(artifacts: dict[str, dict]) -> None:
    for artifact_name, artifact in artifacts.items():
        require(
            artifact.get("readiness_claim") in (None, False),
            f"evidence template {artifact_name} must not claim readiness",
        )
        require(
            artifact.get("status") in {"pending_customer_evidence", "blocked"}
            or artifact.get("readiness_gate_status") == "blocked"
            or artifact.get("readback_status") == "blocked",
            f"evidence template {artifact_name} must remain blocked or pending",
        )
        artifact_text = json.dumps(artifact, sort_keys=True)
        require(
            "local://template" in artifact_text
            or artifact.get("customer_data_required") is False,
            f"evidence template {artifact_name} must preserve template placeholder URIs",
        )
    worker_execution = artifacts.get("worker_data_pipeline_execution_report.json")
    require(
        worker_execution is not None
        and worker_execution.get("readiness_gate_status") == "blocked",
        "worker data pipeline execution template must remain blocked",
    )
    scoring_readback = artifacts.get("scoring_readback_report.json")
    require(
        scoring_readback is not None and scoring_readback.get("readback_status") == "blocked",
        "scoring readback template must remain blocked",
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
    require(runbook.get("readiness_claim") is False, "runbook must not claim readiness")
    require(
        "runtime-secret-not-persisted" in json.dumps(runbook),
        "runbook must use the runtime-secret-not-persisted API key placeholder",
    )
    commands = runbook.get("commands")
    require(isinstance(commands, list), "runbook commands must be a list")
    commands_by_step = {
        command.get("step"): command for command in commands if isinstance(command, dict)
    }
    for step, (command_needle, output_path) in REQUIRED_RUNBOOK_STEPS.items():
        command = commands_by_step.get(step)
        require(command is not None, f"runbook missing required step {step}")
        command_text = command.get("command")
        require(
            isinstance(command_text, str) and command_needle in command_text,
            f"runbook step {step} missing command {command_needle}",
        )
        require(
            command.get("output") == output_path,
            f"runbook step {step} output must be {output_path}",
        )
    command_text = "\n".join(
        command.get("command", "") for command in commands if isinstance(command, dict)
    )
    for path in REQUIRED_RUNBOOK_COMMAND_PATHS:
        require(path in command_text, f"runbook command paths missing {path}")
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

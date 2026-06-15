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
    MODEL_SERVING_SLO_EVIDENCE_PREFIXES,
    WORKER_DATA_PIPELINE_ADDITIONAL_JOB_EVIDENCE_PREFIXES,
    WORKER_DATA_PIPELINE_REQUIRED_JOB_KINDS,
    WORKER_DATA_PIPELINE_SCORING_READBACK_EVIDENCE_PREFIXES,
    WORKER_DATA_PIPELINE_SOURCE_SNAPSHOT_EVIDENCE_PREFIX,
    WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS,
    WORKER_DATA_PIPELINE_SUBMIT_JOB_EVIDENCE_PREFIXES,
    WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS,
    WORKER_DATA_PIPELINE_SUBMIT_JOB_PERMISSIONS,
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
REQUIRED_SOURCE_TEMPLATE_EVIDENCE_PREFIXES = {
    "sources/customer-data-governance-source.json": {
        "dataset_provenance:",
        "label_provenance:",
        "holdout_split:",
        "shadow_traffic_plan:",
    },
    "sources/retention-legal-hold-source.json": {
        "retention_policy:",
        "legal_hold_policy:",
    },
    "sources/model-serving-slo-source.json": {
        *MODEL_SERVING_SLO_EVIDENCE_PREFIXES,
    },
    "sources/ocr-vector-analytics-source.json": {
        "ai_evidence_execution:",
        "ocr_outputs:",
        "embedding_jobs:",
        "retrieval_audits:",
        "analytics_exports:",
        "clickhouse_dashboard:",
    },
}
REQUIRED_WORKER_TEMPLATES = {
    "worker/score_request.json",
    "worker/scoring_readback_input.json",
    "worker/worker_data_pipeline_readiness_input.json",
    "worker/worker_data_pipeline_run_status.json",
}
REQUIRED_SCORING_READBACK_PREFIXES = {
    "scoring_feature_contexts:",
    "provider_profile_window_rollups:",
    "sanctions_sync_reports:",
    "provider_graph_signal_rollups:",
    "peer_benchmarks:",
    "episode_rollups:",
    "clinical_compatibility:",
    "unbundling_candidates:",
}
REQUIRED_SCORING_READBACK_INPUT_EVIDENCE_PREFIXES = {
    "worker_data_pipeline_executions:",
    "scoring_readback_score_requests:",
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
REQUIRED_RUNBOOK_ARTIFACT_BUILD_COMMANDS = {
    "fetch-oig-sam-sanctions-snapshot",
    "sync-oig-sam-sanctions",
    "build-provider-profile-windows",
    "build-provider-graph-signals",
    "build-peer-benchmarks",
    "build-episode-aggregation",
    "build-clinical-compatibility-reference",
    "build-unbundling-comparator",
    "build-scoring-feature-contexts",
    "build-probability-calibration-report",
}
REQUIRED_RUNBOOK_ARTIFACT_BUILD_OUTPUTS = {
    "fetch_oig_sam_sanctions_snapshot": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/sanctions/<as-of-date>/oig_sam_sanctions_snapshot.json",
    "sync_oig_sam_sanctions": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/sanctions/<as-of-date>/sanctions_sync_report.json",
    "build_provider_profile_windows": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/provider-profile/<as-of-date>/provider_profile_window_rollup_report.json",
    "build_provider_graph_signals": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/provider-graph/<as-of-date>/provider_graph_signal_rollup_report.json",
    "build_peer_benchmarks": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/peer-benchmark/<benchmark-month>/peer_percentile_benchmark.json",
    "build_episode_aggregation": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/episodes/<as-of-date>/episode_aggregation_report.json",
    "build_clinical_compatibility_reference": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/clinical-compatibility/<reference-version>/clinical_compatibility_reference_report.json",
    "build_unbundling_comparator": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/unbundling/<as-of-date>/unbundling_comparator_report.json",
    "build_scoring_feature_contexts": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/scoring-contexts/<as-of-date>/scoring_feature_context_report.json",
    "build_probability_calibration_report": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/probability-calibration/<benchmark-month>/probability_calibration_report.json",
}
REQUIRED_SCORING_CONTEXT_INPUT_URIS = {
    "--episode-rollups-uri": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/episodes/<as-of-date>/episode_aggregation_report.json",
    "--peer-benchmarks-uri": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/peer-benchmark/<benchmark-month>/peer_percentile_benchmark.json",
    "--clinical-compatibility-uri": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/clinical-compatibility/<reference-version>/clinical_compatibility_reference_report.json",
    "--unbundling-candidates-uri": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/unbundling/<as-of-date>/unbundling_comparator_report.json",
}
REQUIRED_RUNBOOK_SUBMIT_COMMANDS = {
    "submit-worker-data-pipeline-readiness-report",
    "submit-sanctions-sync-report",
    "submit-provider-profile-window-rollup",
    "submit-provider-graph-signal-rollup",
    "submit-peer-benchmark",
    "submit-episode-aggregation",
    "submit-clinical-compatibility-reference",
    "submit-unbundling-comparator",
    "submit-scoring-feature-contexts",
    "submit-probability-calibration-report",
    "submit-worker-data-pipeline-execution-report",
}
REQUIRED_RUNBOOK_SUBMIT_OUTPUTS = {
    "submit_readiness_report": "api:/api/v1/ops/worker-data-pipeline-readiness",
    "submit_sanctions_sync_report": (
        f"api:{WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS['oig_sam_sanctions_sync']}"
    ),
    "submit_provider_profile_window_rollup": (
        f"api:{WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS['provider_profile_window_rollup']}"
    ),
    "submit_provider_graph_signal_rollup": (
        f"api:{WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS['provider_graph_signal_rollup']}"
    ),
    "submit_peer_benchmark": (
        f"api:{WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS['peer_percentile_benchmark']}"
    ),
    "submit_episode_aggregation": (
        f"api:{WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS['episode_aggregation']}"
    ),
    "submit_clinical_compatibility_reference": (
        f"api:{WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS['clinical_compatibility_reference']}"
    ),
    "submit_unbundling_comparator": (
        f"api:{WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS['unbundling_comparator']}"
    ),
    "submit_scoring_feature_contexts": (
        f"api:{WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS['scoring_feature_context_materialization']}"
    ),
    "submit_probability_calibration_report": (
        f"api:{WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS['probability_calibration_evidence']}"
    ),
    "submit_execution_report": "api:/api/v1/ops/worker-data-pipeline-executions",
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
    validate_source_templates(package_dir)
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
    validate_worker_execution_template(worker_execution)
    scoring_readback = artifacts.get("scoring_readback_report.json")
    require(
        scoring_readback is not None and scoring_readback.get("readback_status") == "blocked",
        "scoring readback template must remain blocked",
    )
    validate_scoring_readback_template(scoring_readback)
    model_serving_slo = artifacts.get("model_serving_slo_report.json")
    require(
        model_serving_slo is not None
        and model_serving_slo.get("artifact_kind") == "model_serving_slo_report",
        "model serving SLO template must be present",
    )
    validate_model_serving_slo_template(model_serving_slo)


def validate_source_templates(package_dir: Path) -> None:
    for template_path, required_prefixes in REQUIRED_SOURCE_TEMPLATE_EVIDENCE_PREFIXES.items():
        source = load_json(package_dir / template_path)
        require(
            source.get("artifact_kind") == "production_readiness_source_template",
            f"source template {template_path} has wrong artifact_kind",
        )
        require(
            source.get("status") == "pending_customer_input",
            f"source template {template_path} must stay pending customer input",
        )
        require(
            source.get("readiness_claim") is False,
            f"source template {template_path} must not claim readiness",
        )
        evidence_refs = source.get("evidence_refs")
        for prefix in required_prefixes:
            require(
                evidence_refs_include_prefix(evidence_refs, prefix),
                f"source template {template_path} evidence_refs missing {prefix}",
            )


def validate_worker_execution_template(report: dict) -> None:
    require(
        report.get("report_kind") == "worker_data_pipeline_execution_report",
        "worker data pipeline execution template has wrong report_kind",
    )
    job_executions = report.get("job_executions")
    require(
        isinstance(job_executions, list) and job_executions,
        "worker data pipeline execution template requires job_executions",
    )
    require(
        report.get("job_count") == len(job_executions),
        "worker data pipeline execution template job_count must match job_executions",
    )
    jobs_by_kind = {
        job.get("job_kind"): job for job in job_executions if isinstance(job, dict)
    }
    require(
        set(jobs_by_kind) == WORKER_DATA_PIPELINE_REQUIRED_JOB_KINDS,
        "worker data pipeline execution template job kind set changed unexpectedly",
    )
    for job_kind, job in jobs_by_kind.items():
        require(
            job.get("execution_status") == "pending_customer_scheduler_run",
            f"worker data pipeline execution template {job_kind} must stay pending",
        )
        require(
            job.get("reported_status") == "pending",
            f"worker data pipeline execution template {job_kind} reported_status must stay pending",
        )
        require(
            job.get("reported_artifact_uri") == f"local://template/{job_kind}.json",
            f"worker data pipeline execution template {job_kind} has wrong artifact URI",
        )
        validate_worker_required_prefixes(
            job_kind,
            job.get("required_evidence_prefixes"),
            job.get("evidence_refs"),
            "worker data pipeline execution template",
        )
        if job_kind in WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS:
            require(
                job.get("submitted") is False,
                f"worker data pipeline execution template {job_kind} must not be submitted",
            )
            require(
                job.get("api_path") == WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS[job_kind],
                f"worker data pipeline execution template {job_kind} has wrong api_path",
            )
            require(
                job.get("required_permission")
                == WORKER_DATA_PIPELINE_SUBMIT_JOB_PERMISSIONS[job_kind],
                f"worker data pipeline execution template {job_kind} has wrong required_permission",
            )
    evidence_refs = report.get("evidence_refs")
    for prefix in (
        "worker_data_pipeline_plans:",
        "worker_data_pipeline_run_status:",
        "worker_data_pipeline_readiness_reports:",
    ):
        require(
            evidence_refs_include_prefix(evidence_refs, prefix),
            f"worker data pipeline execution template evidence_refs missing {prefix}",
        )


def validate_scoring_readback_template(report: dict) -> None:
    require(
        report.get("report_kind") == "scoring_readback_report",
        "scoring readback template has wrong report_kind",
    )
    require(
        "customer_score_response_artifact_missing" in (report.get("blockers") or []),
        "scoring readback template must remain blocked on score response evidence",
    )
    evidence_refs = report.get("evidence_refs")
    for prefix in WORKER_DATA_PIPELINE_SCORING_READBACK_EVIDENCE_PREFIXES:
        require(
            evidence_refs_include_prefix(evidence_refs, prefix),
            f"scoring readback template evidence_refs missing {prefix}",
        )


def validate_model_serving_slo_template(report: dict) -> None:
    evidence_refs = report.get("evidence_refs")
    for prefix in MODEL_SERVING_SLO_EVIDENCE_PREFIXES:
        require(
            evidence_refs_include_prefix(evidence_refs, prefix),
            f"model serving SLO template evidence_refs missing {prefix}",
        )


def evidence_refs_include_prefix(evidence_refs: object, prefix: str) -> bool:
    return isinstance(evidence_refs, list) and any(
        isinstance(reference, str) and reference.startswith(prefix)
        for reference in evidence_refs
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
    observed_prefixes = set(expected_prefixes)
    require(
        REQUIRED_SCORING_READBACK_PREFIXES.issubset(observed_prefixes),
        "scoring readback input missing required expected evidence prefixes: "
        f"{sorted(REQUIRED_SCORING_READBACK_PREFIXES - observed_prefixes)}",
    )
    readback_evidence_refs = readback_input.get("evidence_refs")
    require(
        isinstance(readback_evidence_refs, list) and readback_evidence_refs,
        "scoring readback input requires evidence_refs",
    )
    for required_prefix in REQUIRED_SCORING_READBACK_INPUT_EVIDENCE_PREFIXES:
        require(
            any(
                isinstance(reference, str) and reference.startswith(required_prefix)
                for reference in readback_evidence_refs
            ),
            f"scoring readback input evidence_refs missing {required_prefix}",
        )
    require(
        readback_input.get("score_request_uri")
        == "artifacts/production-evidence-package/worker/score_request.json",
        "scoring readback input must point at worker/score_request.json",
    )
    require(
        readback_input.get("score_response_uri")
        == "artifacts/production-evidence-package/worker/scoring-readback/score_response.json",
        "scoring readback input must point at worker/scoring-readback/score_response.json",
    )

    readiness_input = load_json(package_dir / "worker" / "worker_data_pipeline_readiness_input.json")
    require(
        readiness_input.get("artifact_kind") == "worker_data_pipeline_readiness_input_template",
        "worker readiness input has wrong artifact_kind",
    )
    validate_worker_readiness_checks(readiness_input.get("checks"))

    run_status = load_json(package_dir / "worker" / "worker_data_pipeline_run_status.json")
    require(
        run_status.get("report_kind") == "worker_data_pipeline_run_status",
        "worker run status template has wrong report_kind",
    )
    require(run_status.get("run_status_template") is True, "run status must remain a template")
    validate_worker_run_status_jobs(run_status.get("job_statuses"))

    for path in REQUIRED_WORKER_TEMPLATES:
        template_text = (package_dir / path).read_text(encoding="utf-8")
        for forbidden in FORBIDDEN_TEMPLATE_STRINGS:
            require(
                forbidden not in template_text,
                f"worker template {path} contains forbidden placeholder string {forbidden}",
            )


def expected_worker_evidence_prefixes(job_kind: str) -> set[str]:
    prefixes = set()
    if job_kind in WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS:
        prefixes.add(WORKER_DATA_PIPELINE_SUBMIT_JOB_EVIDENCE_PREFIXES[job_kind])
    if job_kind == "oig_sam_sanctions_snapshot_fetch":
        prefixes.add(WORKER_DATA_PIPELINE_SOURCE_SNAPSHOT_EVIDENCE_PREFIX)
    if job_kind == "scoring_online_readback":
        prefixes.update(WORKER_DATA_PIPELINE_SCORING_READBACK_EVIDENCE_PREFIXES)
    prefixes.update(WORKER_DATA_PIPELINE_ADDITIONAL_JOB_EVIDENCE_PREFIXES.get(job_kind, ()))
    return prefixes


def validate_worker_readiness_checks(checks: object) -> None:
    require(isinstance(checks, list) and checks, "worker readiness input requires checks")
    checks_by_kind = {
        check.get("job_kind"): check for check in checks if isinstance(check, dict)
    }
    require(
        set(checks_by_kind) == WORKER_DATA_PIPELINE_REQUIRED_JOB_KINDS,
        "worker readiness input job kind set changed unexpectedly",
    )
    for job_kind, check in checks_by_kind.items():
        validate_worker_required_prefixes(
            job_kind,
            check.get("required_evidence_prefixes"),
            check.get("evidence_refs"),
            "worker readiness input",
        )
        require(
            check.get("artifact_uri") == f"local://template/worker/{job_kind}.json",
            f"worker readiness input {job_kind} has wrong artifact_uri",
        )


def validate_worker_run_status_jobs(job_statuses: object) -> None:
    require(
        isinstance(job_statuses, list) and job_statuses,
        "worker run status requires job_statuses",
    )
    jobs_by_kind = {
        job.get("job_kind"): job for job in job_statuses if isinstance(job, dict)
    }
    require(
        set(jobs_by_kind) == WORKER_DATA_PIPELINE_REQUIRED_JOB_KINDS,
        "worker run status job kind set changed unexpectedly",
    )
    for job_kind, job in jobs_by_kind.items():
        validate_worker_required_prefixes(
            job_kind,
            job.get("required_evidence_prefixes"),
            job.get("evidence_refs"),
            "worker run status",
            require_evidence_refs=False,
        )
        if job_kind in WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS:
            require(
                job.get("api_path") == WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS[job_kind],
                f"worker run status {job_kind} has wrong api_path",
            )
            require(
                job.get("required_permission")
                == WORKER_DATA_PIPELINE_SUBMIT_JOB_PERMISSIONS[job_kind],
                f"worker run status {job_kind} has wrong required_permission",
            )


def validate_worker_required_prefixes(
    job_kind: str,
    required_prefixes: object,
    evidence_refs: object,
    owner: str,
    *,
    require_evidence_refs: bool = True,
) -> None:
    require(
        isinstance(required_prefixes, list),
        f"{owner} {job_kind} requires required_evidence_prefixes",
    )
    observed_prefixes = set(required_prefixes)
    expected_prefixes = expected_worker_evidence_prefixes(job_kind)
    require(
        observed_prefixes == expected_prefixes,
        f"{owner} {job_kind} required_evidence_prefixes changed unexpectedly",
    )
    if not require_evidence_refs:
        return
    require(
        isinstance(evidence_refs, list) and evidence_refs,
        f"{owner} {job_kind} requires evidence_refs",
    )
    for prefix in expected_prefixes:
        require(
            any(
                isinstance(reference, str) and reference.startswith(prefix)
                for reference in evidence_refs
            ),
            f"{owner} {job_kind} evidence_refs missing {prefix}",
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
    for build_command in REQUIRED_RUNBOOK_ARTIFACT_BUILD_COMMANDS:
        require(
            build_command in command_text,
            f"runbook artifact build commands missing {build_command}",
        )
    for step, expected_output in REQUIRED_RUNBOOK_ARTIFACT_BUILD_OUTPUTS.items():
        command = commands_by_step.get(step)
        require(command is not None, f"runbook missing artifact build step {step}")
        require(
            command.get("output") == expected_output,
            f"runbook step {step} output must be {expected_output}",
        )
    scoring_context_command = commands_by_step.get("build_scoring_feature_contexts")
    require(
        scoring_context_command is not None,
        "runbook missing artifact build step build_scoring_feature_contexts",
    )
    scoring_context_command_text = scoring_context_command.get("command")
    require(
        isinstance(scoring_context_command_text, str),
        "runbook step build_scoring_feature_contexts command must be text",
    )
    for flag, expected_uri in REQUIRED_SCORING_CONTEXT_INPUT_URIS.items():
        require(
            f"{flag} {expected_uri}" in scoring_context_command_text,
            f"runbook step build_scoring_feature_contexts {flag} must be {expected_uri}",
        )
    for submit_command in REQUIRED_RUNBOOK_SUBMIT_COMMANDS:
        require(
            submit_command in command_text,
            f"runbook submit commands missing {submit_command}",
        )
    for step, expected_output in REQUIRED_RUNBOOK_SUBMIT_OUTPUTS.items():
        command = commands_by_step.get(step)
        require(command is not None, f"runbook missing submit step {step}")
        require(
            command.get("output") == expected_output,
            f"runbook step {step} output must be {expected_output}",
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

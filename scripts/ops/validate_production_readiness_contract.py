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

WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS = (
    WORKER_DATA_PIPELINE_REQUIRED_JOB_KINDS - {"oig_sam_sanctions_snapshot_fetch"}
)

WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS = {
    "oig_sam_sanctions_sync": "/api/v1/ops/providers/sanctions-sync-reports",
    "provider_profile_window_rollup": "/api/v1/ops/providers/profile-window-rollups",
    "provider_graph_signal_rollup": "/api/v1/ops/providers/graph-signal-rollups",
    "peer_percentile_benchmark": "/api/v1/ops/providers/peer-benchmarks",
    "episode_aggregation": "/api/v1/ops/providers/episode-rollups",
    "clinical_compatibility_reference": "/api/v1/ops/clinical-compatibility-references",
    "unbundling_comparator": "/api/v1/ops/unbundling-comparator-candidates",
    "scoring_feature_context_materialization": "/api/v1/ops/scoring-feature-context-materializations",
    "probability_calibration_evidence": "/api/v1/ops/models/{model_key}/probability-calibration-reports",
}

WORKER_DATA_PIPELINE_SUBMIT_JOB_PERMISSIONS = {
    "oig_sam_sanctions_sync": "ops:providers:write",
    "provider_profile_window_rollup": "ops:providers:write",
    "provider_graph_signal_rollup": "ops:providers:write",
    "peer_percentile_benchmark": "ops:providers:write",
    "episode_aggregation": "ops:providers:write",
    "clinical_compatibility_reference": "ops:datasets:write",
    "unbundling_comparator": "ops:datasets:write",
    "scoring_feature_context_materialization": "ops:datasets:write",
    "probability_calibration_evidence": "ops:models:review",
}

WORKER_DATA_PIPELINE_SUBMIT_JOB_EVIDENCE_PREFIXES = {
    "oig_sam_sanctions_sync": "sanctions_sync_reports:",
    "provider_profile_window_rollup": "provider_profile_window_rollups:",
    "provider_graph_signal_rollup": "provider_graph_signal_rollups:",
    "peer_percentile_benchmark": "peer_benchmarks:",
    "episode_aggregation": "episode_rollups:",
    "clinical_compatibility_reference": "clinical_compatibility_references:",
    "unbundling_comparator": "unbundling_comparator_candidates:",
    "scoring_feature_context_materialization": "scoring_feature_contexts:",
    "probability_calibration_evidence": "probability_calibration_reports:",
}

WORKER_DATA_PIPELINE_SOURCE_SNAPSHOT_EVIDENCE_PREFIX = "oig_sam_snapshot:"

WORKER_DATA_PIPELINE_ADDITIONAL_JOB_EVIDENCE_PREFIXES = {
    "provider_profile_window_rollup": ("provider_profile_claim_snapshot:",),
    "provider_graph_signal_rollup": ("provider_graph_claim_snapshot:",),
    "peer_percentile_benchmark": ("peer_benchmark_claim_snapshot:",),
    "episode_aggregation": ("episode_claim_snapshot:",),
    "clinical_compatibility_reference": (
        "clinical_compatibility_reference:",
        "clinical_policy_authority:",
    ),
    "unbundling_comparator": ("unbundling_comparator_input:",),
    "scoring_feature_context_materialization": (
        "episode_rollups:",
        "peer_benchmarks:",
        "clinical_compatibility:",
        "unbundling_candidates:",
    )
}

WORKER_DATA_PIPELINE_ACCEPTANCE_CHECK_IDS = {
    "report_kind_is_worker_data_pipeline_execution_report",
    "readiness_gate_status_ready",
    "required_execution_uris_and_run_identity_present",
    "required_execution_uris_are_production_uris",
    "scheduler_status_completed",
    "pending_or_failed_job_count_zero",
    "review_task_count_zero",
    "required_job_kinds_completed",
    "scheduler_reported_jobs_succeeded_without_dependency_blockers",
    "completed_job_artifact_and_evidence_refs_present",
    "completed_job_artifacts_are_production_uris",
    "governed_submit_jobs_submitted",
    "governed_submit_jobs_include_write_evidence_refs",
    "source_snapshot_artifact_reported",
    "evidence_refs_include_plan_run_status_and_readiness",
    "governance_boundary_no_adjudication",
}

RETENTION_LEGAL_HOLD_ACCEPTANCE_CHECK_IDS = {
    "report_kind_is_retention_legal_hold_report",
    "minimum_six_year_retention_configured",
    "policy_and_archive_refs_present",
    "legal_hold_reconciliation_completed",
    "destruction_requires_human_approval",
}

MODEL_SERVING_SLO_ACCEPTANCE_CHECK_IDS = {
    "report_kind_is_model_serving_slo_report",
    "latency_and_error_slos_passed",
    "artifact_integrity_verified",
    "fallback_and_rollback_ready",
    "calibrated_probability_serving_active",
    "model_serving_evidence_refs_present",
}

CUSTOMER_DATA_GOVERNANCE_ACCEPTANCE_CHECK_IDS = {
    "report_kind_is_customer_data_governance_report",
    "dataset_and_label_provenance_approved",
    "holdout_and_shadow_plan_approved",
    "customer_validation_samples_present",
    "customer_data_evidence_refs_present",
}

OCR_VECTOR_ANALYTICS_ACCEPTANCE_CHECK_IDS = {
    "report_kind_is_ocr_vector_analytics_execution_report",
    "evidence_pipeline_jobs_completed",
    "execution_counts_positive",
    "phi_boundary_preserved",
    "ocr_vector_analytics_evidence_refs_present",
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def load_json(path: Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as exc:
        raise AssertionError(f"missing JSON artifact: {path}") from exc


def validate_contract(contract: dict) -> list[dict]:
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
            governed_submit_job_kinds = set(
                gate.get("governed_submit_job_kinds", [])
            )
            require(
                governed_submit_job_kinds == WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS,
                "worker data pipeline gate governed_submit_job_kinds changed unexpectedly",
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
        if gate.get("gate_id") == "retention_legal_hold":
            acceptance_checks = gate.get("acceptance_checks")
            require(
                isinstance(acceptance_checks, list) and acceptance_checks,
                "retention legal hold gate missing acceptance_checks",
            )
            check_ids = {
                check.get("check_id")
                for check in acceptance_checks
                if isinstance(check, dict)
            }
            require(
                check_ids == RETENTION_LEGAL_HOLD_ACCEPTANCE_CHECK_IDS,
                "retention legal hold gate acceptance check set changed unexpectedly",
            )
            for check in acceptance_checks:
                require(
                    check.get("description"),
                    f"retention legal hold acceptance check {check.get('check_id')} missing description",
                )
        if gate.get("gate_id") == "model_serving_slo":
            acceptance_checks = gate.get("acceptance_checks")
            require(
                isinstance(acceptance_checks, list) and acceptance_checks,
                "model serving SLO gate missing acceptance_checks",
            )
            check_ids = {
                check.get("check_id")
                for check in acceptance_checks
                if isinstance(check, dict)
            }
            require(
                check_ids == MODEL_SERVING_SLO_ACCEPTANCE_CHECK_IDS,
                "model serving SLO gate acceptance check set changed unexpectedly",
            )
            for check in acceptance_checks:
                require(
                    check.get("description"),
                    f"model serving SLO acceptance check {check.get('check_id')} missing description",
                )
        if gate.get("gate_id") == "customer_data_governance":
            acceptance_checks = gate.get("acceptance_checks")
            require(
                isinstance(acceptance_checks, list) and acceptance_checks,
                "customer data governance gate missing acceptance_checks",
            )
            check_ids = {
                check.get("check_id")
                for check in acceptance_checks
                if isinstance(check, dict)
            }
            require(
                check_ids == CUSTOMER_DATA_GOVERNANCE_ACCEPTANCE_CHECK_IDS,
                "customer data governance gate acceptance check set changed unexpectedly",
            )
            for check in acceptance_checks:
                require(
                    check.get("description"),
                    f"customer data governance acceptance check {check.get('check_id')} missing description",
                )
        if gate.get("gate_id") == "ocr_vector_analytics_execution":
            acceptance_checks = gate.get("acceptance_checks")
            require(
                isinstance(acceptance_checks, list) and acceptance_checks,
                "OCR/vector/analytics gate missing acceptance_checks",
            )
            check_ids = {
                check.get("check_id")
                for check in acceptance_checks
                if isinstance(check, dict)
            }
            require(
                check_ids == OCR_VECTOR_ANALYTICS_ACCEPTANCE_CHECK_IDS,
                "OCR/vector/analytics gate acceptance check set changed unexpectedly",
            )
            for check in acceptance_checks:
                require(
                    check.get("description"),
                    f"OCR/vector/analytics acceptance check {check.get('check_id')} missing description",
                )
    return gates


def validate_evidence_dir(evidence_dir: Path, gates: list[dict]) -> dict[str, dict]:
    artifacts = {}
    for gate in gates:
        gate_id = gate.get("gate_id")
        required_artifact = gate.get("required_artifact")
        require(
            isinstance(required_artifact, str) and required_artifact.strip(),
            f"gate {gate_id} missing required_artifact",
        )
        artifact = load_json(evidence_dir / required_artifact)
        require(
            isinstance(artifact, dict),
            f"production evidence artifact {required_artifact} must be a JSON object",
        )
        artifacts[required_artifact] = artifact
    return artifacts


def is_production_artifact_uri(value: object) -> bool:
    if not isinstance(value, str):
        return False
    uri = value.strip()
    return bool(uri) and not uri.startswith("local://") and "{" not in uri and "}" not in uri


def evidence_refs_include_prefix(job: dict, prefix: str) -> bool:
    evidence_refs = job.get("evidence_refs")
    return isinstance(evidence_refs, list) and any(
        isinstance(reference, str) and reference.startswith(prefix)
        for reference in evidence_refs
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
    for field_name in (
        "plan_uri",
        "run_status_uri",
        "readiness_report_uri",
        "run_id",
        "execution_date",
    ):
        require(
            isinstance(report.get(field_name), str) and report[field_name].strip(),
            f"worker data pipeline execution evidence must include {field_name}",
        )
    for field_name in ("plan_uri", "run_status_uri", "readiness_report_uri"):
        require(
            is_production_artifact_uri(report.get(field_name)),
            f"worker data pipeline execution evidence {field_name} must be a production artifact URI, not a local dry-run or template URI",
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
        require(
            job.get("reported_status") == "succeeded",
            f"worker data pipeline job {job_kind} must have reported_status succeeded",
        )
        blocked_dependencies = job.get("blocked_dependencies")
        require(
            not blocked_dependencies,
            f"worker data pipeline job {job_kind} must not have blocked_dependencies",
        )
        require(
            isinstance(job.get("reported_artifact_uri"), str)
            and job["reported_artifact_uri"].strip(),
            f"worker data pipeline job {job_kind} must report artifact URI",
        )
        require(
            is_production_artifact_uri(job.get("reported_artifact_uri")),
            f"worker data pipeline job {job_kind} must report a production artifact URI, not a local dry-run or template URI",
        )
        job_evidence_refs = job.get("evidence_refs")
        require(
            isinstance(job_evidence_refs, list)
            and job_evidence_refs
            and all(
                isinstance(reference, str) and reference.strip()
                for reference in job_evidence_refs
            ),
            f"worker data pipeline job {job_kind} must include non-empty evidence_refs",
        )
    for job_kind in WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS:
        job = jobs_by_kind[job_kind]
        require(
            job.get("submitted") is True,
            f"worker data pipeline submit job {job_kind} must have submitted true",
        )
        require(
            job.get("api_path") == WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS[job_kind],
            f"worker data pipeline submit job {job_kind} has wrong api_path",
        )
        require(
            job.get("required_permission")
            == WORKER_DATA_PIPELINE_SUBMIT_JOB_PERMISSIONS[job_kind],
            f"worker data pipeline submit job {job_kind} has wrong required_permission",
        )
        require(
            isinstance(job.get("reported_artifact_uri"), str)
            and job["reported_artifact_uri"].strip(),
            f"worker data pipeline submit job {job_kind} must report artifact URI",
        )
        required_evidence_prefix = WORKER_DATA_PIPELINE_SUBMIT_JOB_EVIDENCE_PREFIXES[job_kind]
        require(
            evidence_refs_include_prefix(job, required_evidence_prefix),
            f"worker data pipeline submit job {job_kind} evidence_refs missing {required_evidence_prefix}",
        )
        for additional_prefix in WORKER_DATA_PIPELINE_ADDITIONAL_JOB_EVIDENCE_PREFIXES.get(
            job_kind, ()
        ):
            require(
                evidence_refs_include_prefix(job, additional_prefix),
                f"worker data pipeline submit job {job_kind} evidence_refs missing {additional_prefix}",
            )
    snapshot_job = jobs_by_kind["oig_sam_sanctions_snapshot_fetch"]
    require(
        isinstance(snapshot_job.get("reported_artifact_uri"), str)
        and snapshot_job["reported_artifact_uri"].strip(),
        "worker data pipeline source snapshot job must report artifact URI",
    )
    require(
        is_production_artifact_uri(snapshot_job.get("reported_artifact_uri")),
        "worker data pipeline source snapshot job must report a production artifact URI, not a local dry-run or template URI",
    )
    require(
        evidence_refs_include_prefix(
            snapshot_job, WORKER_DATA_PIPELINE_SOURCE_SNAPSHOT_EVIDENCE_PREFIX
        ),
        f"worker data pipeline source snapshot job evidence_refs missing {WORKER_DATA_PIPELINE_SOURCE_SNAPSHOT_EVIDENCE_PREFIX}",
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


def validate_retention_legal_hold_evidence(report: dict) -> None:
    require(
        report.get("artifact_kind") == "retention_legal_hold_report",
        "retention legal-hold evidence has wrong artifact_kind",
    )
    require(
        isinstance(report.get("retention_years"), int)
        and report["retention_years"] >= 6,
        "retention legal-hold evidence must configure at least six retention years",
    )
    for field_name in (
        "retention_policy_id",
        "legal_hold_policy_id",
        "archive_storage_uri",
    ):
        require(
            isinstance(report.get(field_name), str) and report[field_name].strip(),
            f"retention legal-hold evidence must include {field_name}",
        )
    require(
        report.get("legal_hold_reconciliation_status") == "completed",
        "retention legal-hold evidence must have completed legal-hold reconciliation",
    )
    require(
        report.get("destruction_workflow") == "human_approval_required_before_destroy",
        "retention legal-hold evidence must require human approval before destruction",
    )
    require(
        report.get("automated_destruction_enabled") is False,
        "retention legal-hold evidence must keep automated destruction disabled",
    )
    evidence_refs = report.get("evidence_refs")
    require(
        isinstance(evidence_refs, list) and evidence_refs,
        "retention legal-hold evidence must include evidence_refs",
    )
    for prefix in ("retention_policy:", "legal_hold_policy:"):
        require(
            any(isinstance(reference, str) and reference.startswith(prefix) for reference in evidence_refs),
            f"retention legal-hold evidence_refs missing {prefix}",
        )


def validate_model_serving_slo_evidence(report: dict) -> None:
    require(
        report.get("artifact_kind") == "model_serving_slo_report",
        "model serving SLO evidence has wrong artifact_kind",
    )
    for field_name in ("model_key", "model_version"):
        require(
            isinstance(report.get(field_name), str) and report[field_name].strip(),
            f"model serving SLO evidence must include {field_name}",
        )
    latency_slo_ms = report.get("latency_slo_ms")
    p95_latency_ms = report.get("p95_latency_ms")
    require(
        isinstance(latency_slo_ms, (int, float)) and latency_slo_ms > 0,
        "model serving SLO evidence must include positive latency_slo_ms",
    )
    require(
        isinstance(p95_latency_ms, (int, float)) and p95_latency_ms <= latency_slo_ms,
        "model serving SLO evidence p95 latency must be within latency_slo_ms",
    )
    error_rate_slo = report.get("error_rate_slo")
    error_rate = report.get("error_rate")
    require(
        isinstance(error_rate_slo, (int, float)) and 0 <= error_rate_slo <= 1,
        "model serving SLO evidence must include error_rate_slo between 0 and 1",
    )
    require(
        isinstance(error_rate, (int, float)) and 0 <= error_rate <= error_rate_slo,
        "model serving SLO evidence error_rate must be within error_rate_slo",
    )
    require(
        report.get("checksum_verified") is True,
        "model serving SLO evidence must verify model checksum",
    )
    require(
        report.get("signature_verified") is True,
        "model serving SLO evidence must verify model signature",
    )
    require(
        report.get("fallback_status") == "healthy",
        "model serving SLO evidence fallback_status must be healthy",
    )
    require(
        report.get("rollback_ready") is True,
        "model serving SLO evidence rollback_ready must be true",
    )
    require(
        report.get("probability_calibration_status") == "calibrated",
        "model serving SLO evidence probability_calibration_status must be calibrated",
    )
    require(
        report.get("calibrated_probability_serving_active") is True,
        "model serving SLO evidence must activate calibrated probability serving",
    )
    evidence_refs = report.get("evidence_refs")
    require(
        isinstance(evidence_refs, list) and evidence_refs,
        "model serving SLO evidence must include evidence_refs",
    )
    for prefix in ("model_serving:", "model_artifact:", "probability_calibration_reports:"):
        require(
            any(isinstance(reference, str) and reference.startswith(prefix) for reference in evidence_refs),
            f"model serving SLO evidence_refs missing {prefix}",
        )


def validate_customer_data_governance_evidence(report: dict) -> None:
    require(
        report.get("artifact_kind") == "customer_data_governance_report",
        "customer data governance evidence has wrong artifact_kind",
    )
    for field_name in (
        "dataset_provenance_status",
        "label_provenance_status",
        "holdout_split_status",
        "shadow_traffic_plan_status",
    ):
        require(
            report.get(field_name) == "approved",
            f"customer data governance evidence must have {field_name} approved",
        )
    approved_label_count = report.get("approved_label_count")
    holdout_claim_count = report.get("holdout_claim_count")
    require(
        isinstance(approved_label_count, int) and approved_label_count > 0,
        "customer data governance evidence must include positive approved_label_count",
    )
    require(
        isinstance(holdout_claim_count, int) and holdout_claim_count > 0,
        "customer data governance evidence must include positive holdout_claim_count",
    )
    evidence_refs = report.get("evidence_refs")
    require(
        isinstance(evidence_refs, list) and evidence_refs,
        "customer data governance evidence must include evidence_refs",
    )
    for prefix in (
        "dataset_provenance:",
        "label_provenance:",
        "holdout_split:",
        "shadow_traffic_plan:",
    ):
        require(
            any(isinstance(reference, str) and reference.startswith(prefix) for reference in evidence_refs),
            f"customer data governance evidence_refs missing {prefix}",
        )


def validate_ocr_vector_analytics_execution_evidence(report: dict) -> None:
    require(
        report.get("artifact_kind") == "ocr_vector_analytics_execution_report",
        "OCR/vector/analytics execution evidence has wrong artifact_kind",
    )
    for field_name in (
        "ocr_execution_status",
        "embedding_vector_status",
        "retrieval_ranking_status",
        "clickhouse_export_status",
        "dashboard_access_status",
        "analytics_retention_backup_status",
    ):
        require(
            report.get(field_name) == "completed",
            f"OCR/vector/analytics evidence must have {field_name} completed",
        )
    for field_name in (
        "document_count",
        "embedding_job_count",
        "retrieval_audit_count",
        "analytics_export_job_count",
    ):
        require(
            isinstance(report.get(field_name), int) and report[field_name] > 0,
            f"OCR/vector/analytics evidence must include positive {field_name}",
        )
    require(
        report.get("raw_phi_exported") is False,
        "OCR/vector/analytics evidence must not export raw PHI",
    )
    evidence_refs = report.get("evidence_refs")
    require(
        isinstance(evidence_refs, list) and evidence_refs,
        "OCR/vector/analytics evidence must include evidence_refs",
    )
    for prefix in (
        "ai_evidence_execution:",
        "ocr_outputs:",
        "embedding_jobs:",
        "retrieval_audits:",
        "analytics_exports:",
        "clickhouse_dashboard:",
    ):
        require(
            any(isinstance(reference, str) and reference.startswith(prefix) for reference in evidence_refs),
            f"OCR/vector/analytics evidence_refs missing {prefix}",
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
    gates = validate_contract(load_json(contract_dir / "production_readiness_contract.json"))
    index = load_json(contract_dir / "index.json")
    require(index.get("artifact_kind") == "production_readiness_contract_index", "wrong index artifact kind")
    if args.evidence_dir:
        evidence_dir = Path(args.evidence_dir)
        artifacts = validate_evidence_dir(evidence_dir, gates)
        validate_customer_data_governance_evidence(
            artifacts["customer_data_governance_report.json"]
        )
        validate_retention_legal_hold_evidence(
            artifacts["retention_legal_hold_report.json"]
        )
        validate_model_serving_slo_evidence(
            artifacts["model_serving_slo_report.json"]
        )
        validate_ocr_vector_analytics_execution_evidence(
            artifacts["ocr_vector_analytics_execution_report.json"]
        )
        validate_worker_data_pipeline_execution_evidence(
            artifacts["worker_data_pipeline_execution_report.json"]
        )
    print("production readiness contract validation passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except AssertionError as exc:
        print(f"production readiness contract validation failed: {exc}")
        raise SystemExit(1)

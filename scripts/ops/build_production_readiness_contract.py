#!/usr/bin/env python3
"""Build the production readiness evidence contract.

This artifact is intentionally customer-data-free. It defines the evidence that
must be supplied by a real customer environment before production readiness can
be claimed.
"""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path


DEFAULT_OUTPUT_DIR = Path("artifacts/production-readiness")

WORKER_DATA_PIPELINE_JOB_KINDS = [
    "oig_sam_sanctions_snapshot_fetch",
    "oig_sam_sanctions_sync",
    "provider_profile_window_rollup",
    "provider_graph_signal_rollup",
    "peer_percentile_benchmark",
    "episode_aggregation",
    "clinical_compatibility_reference",
    "unbundling_comparator",
    "scoring_feature_context_materialization",
    "scoring_online_readback",
    "probability_calibration_evidence",
]

WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS = [
    job_kind
    for job_kind in WORKER_DATA_PIPELINE_JOB_KINDS
    if job_kind not in {"oig_sam_sanctions_snapshot_fetch", "scoring_online_readback"}
]

WORKER_DATA_PIPELINE_ACCEPTANCE_CHECKS = [
    {
        "check_id": "report_kind_is_worker_data_pipeline_execution_report",
        "description": "Execution evidence artifact has report_kind = worker_data_pipeline_execution_report.",
    },
    {
        "check_id": "readiness_gate_status_ready",
        "description": "Execution evidence references a readiness report and has readiness_gate_status = ready.",
    },
    {
        "check_id": "required_execution_uris_and_run_identity_present",
        "description": "Execution evidence includes non-empty plan_uri, run_status_uri, readiness_report_uri, run_id, and execution_date.",
    },
    {
        "check_id": "required_execution_uris_are_production_uris",
        "description": "Execution evidence plan_uri, run_status_uri, and readiness_report_uri are customer-environment production artifact URIs, not local dry-run or template placeholders.",
    },
    {
        "check_id": "scheduler_status_completed",
        "description": "Customer scheduler report has scheduler_status = completed.",
    },
    {
        "check_id": "pending_or_failed_job_count_zero",
        "description": "Execution evidence has pending_or_failed_job_count = 0.",
    },
    {
        "check_id": "review_task_count_zero",
        "description": "Execution evidence has review_task_count = 0.",
    },
    {
        "check_id": "required_job_kinds_completed",
        "description": "Every required worker data-pipeline job kind is present in job_executions with execution_status = completed.",
    },
    {
        "check_id": "scheduler_reported_jobs_succeeded_without_dependency_blockers",
        "description": "Every worker data-pipeline job has reported_status = succeeded and no blocked_dependencies.",
    },
    {
        "check_id": "completed_job_artifact_and_evidence_refs_present",
        "description": "Every completed worker data-pipeline job has a non-empty reported artifact URI and per-job evidence refs.",
    },
    {
        "check_id": "completed_job_artifacts_are_production_uris",
        "description": "Completed worker data-pipeline job artifact URIs are customer-environment production artifact URIs, not local dry-run or template placeholders.",
    },
    {
        "check_id": "governed_submit_jobs_submitted",
        "description": "Every governed worker data-pipeline submit job has submitted = true, expected API path and permission scope, and a non-empty reported artifact URI.",
    },
    {
        "check_id": "governed_submit_jobs_include_required_submit_flags",
        "description": "Every governed worker data-pipeline submit job carries the required published URI flags for its submit command.",
    },
    {
        "check_id": "governed_submit_jobs_include_write_evidence_refs",
        "description": "Every governed worker data-pipeline submit job includes the expected write evidence reference for its persisted API submission plus source-lineage evidence for provider profiles, graph signals, peer benchmarks, episodes, clinical references, unbundling candidates, scoring context materialization, and probability calibration inputs/labels.",
    },
    {
        "check_id": "source_snapshot_artifact_reported",
        "description": "The artifact-only OIG/SAM source snapshot job reports a non-empty artifact URI and source snapshot evidence reference.",
    },
    {
        "check_id": "scoring_online_readback_artifact_reported",
        "description": "The artifact-only online scoring readback job reports a production artifact URI plus score request, score response, and readback evidence references.",
    },
    {
        "check_id": "evidence_refs_include_plan_run_status_and_readiness",
        "description": "Execution evidence_refs include worker data-pipeline plan, run-status, and readiness report references.",
    },
    {
        "check_id": "governance_boundary_no_adjudication",
        "description": "Execution evidence preserves the no claim scoring, label assignment, denial, model activation, or routing-policy-change boundary.",
    },
]

SCORING_READBACK_ACCEPTANCE_CHECKS = [
    {
        "check_id": "report_kind_is_scoring_readback_report",
        "description": "Scoring readback evidence artifact has report_kind = scoring_readback_report.",
    },
    {
        "check_id": "readback_status_verified",
        "description": "Scoring readback status is verified, not blocked or contract-only.",
    },
    {
        "check_id": "score_request_and_response_uris_present",
        "description": "Scoring readback evidence includes production score request and score response artifact URIs.",
    },
    {
        "check_id": "expected_evidence_prefixes_matched",
        "description": "Every expected worker evidence prefix was observed in the captured score response, including scoring context, provider profile, sanctions, provider graph, peer benchmark, episode, clinical compatibility, and unbundling prefixes.",
    },
    {
        "check_id": "no_scoring_readback_review_tasks",
        "description": "Scoring readback evidence has zero blockers and zero review tasks.",
    },
]

RETENTION_LEGAL_HOLD_ACCEPTANCE_CHECKS = [
    {
        "check_id": "report_kind_is_retention_legal_hold_report",
        "description": "Retention evidence artifact has artifact_kind = retention_legal_hold_report.",
    },
    {
        "check_id": "minimum_six_year_retention_configured",
        "description": "Retention evidence shows retention_years >= 6.",
    },
    {
        "check_id": "policy_and_archive_refs_present",
        "description": "Retention evidence includes retention policy id, legal-hold policy id, archive storage URI, and policy evidence refs.",
    },
    {
        "check_id": "legal_hold_reconciliation_completed",
        "description": "Legal-hold reconciliation status is completed before production readiness is claimed.",
    },
    {
        "check_id": "destruction_requires_human_approval",
        "description": "Destruction workflow requires human approval and automated destruction is disabled.",
    },
]

MODEL_SERVING_SLO_ACCEPTANCE_CHECKS = [
    {
        "check_id": "report_kind_is_model_serving_slo_report",
        "description": "Model serving evidence artifact has artifact_kind = model_serving_slo_report.",
    },
    {
        "check_id": "latency_and_error_slos_passed",
        "description": "Model serving p95 latency and error rate are within the declared SLO thresholds.",
    },
    {
        "check_id": "artifact_integrity_verified",
        "description": "Model artifact checksum and signature verification both passed.",
    },
    {
        "check_id": "fallback_and_rollback_ready",
        "description": "Fallback serving path is healthy and rollback readiness is true.",
    },
    {
        "check_id": "calibrated_probability_serving_active",
        "description": "Model serving evidence shows calibrated probability serving is active with a passing calibration report reference.",
    },
    {
        "check_id": "model_serving_evidence_refs_present",
        "description": "Model serving evidence includes model serving, model artifact, and probability calibration evidence refs.",
    },
]

CUSTOMER_DATA_GOVERNANCE_ACCEPTANCE_CHECKS = [
    {
        "check_id": "report_kind_is_customer_data_governance_report",
        "description": "Customer data governance evidence artifact has artifact_kind = customer_data_governance_report.",
    },
    {
        "check_id": "dataset_and_label_provenance_approved",
        "description": "Dataset provenance and label provenance are approved before production readiness is claimed.",
    },
    {
        "check_id": "holdout_and_shadow_plan_approved",
        "description": "Holdout split and live shadow-traffic plan are approved.",
    },
    {
        "check_id": "customer_validation_samples_present",
        "description": "Customer evidence includes positive approved label and holdout claim counts.",
    },
    {
        "check_id": "customer_data_evidence_refs_present",
        "description": "Customer evidence includes dataset provenance, label provenance, holdout split, and shadow-traffic evidence refs.",
    },
]

OCR_VECTOR_ANALYTICS_ACCEPTANCE_CHECKS = [
    {
        "check_id": "report_kind_is_ocr_vector_analytics_execution_report",
        "description": "OCR/vector/analytics evidence artifact has artifact_kind = ocr_vector_analytics_execution_report.",
    },
    {
        "check_id": "evidence_pipeline_jobs_completed",
        "description": "OCR, embedding/vector, retrieval ranking, ClickHouse export, dashboard access, and retention/backup checks are completed.",
    },
    {
        "check_id": "execution_counts_positive",
        "description": "Execution evidence includes positive document, embedding job, retrieval audit, and analytics export job counts.",
    },
    {
        "check_id": "phi_boundary_preserved",
        "description": "Execution evidence shows raw PHI was not exported into vectors or analytics tables.",
    },
    {
        "check_id": "ocr_vector_analytics_evidence_refs_present",
        "description": "Execution evidence includes AI evidence, OCR output, embedding job, retrieval audit, analytics export, and dashboard evidence refs.",
    },
]

REQUIRED_EVIDENCE = [
    {
        "gate_id": "production_deployment_apply",
        "required_artifact": "production_deployment_apply_report.json",
        "description": "Customer-approved production package was applied with server-side dry-run, rollout status, HPA, Ingress, and NetworkPolicy evidence.",
    },
    {
        "gate_id": "production_smoke",
        "required_artifact": "production_smoke_report.json",
        "description": "API, web console, ML service health, model artifact registry, and representative scoring smoke checks passed in production namespace.",
    },
    {
        "gate_id": "observability_stack",
        "required_artifact": "observability_smoke_report.json",
        "description": "Prometheus scrapes annotated pods and alert rules evaluate successfully with Alertmanager delivery path configured.",
    },
    {
        "gate_id": "secret_and_access_governance",
        "required_artifact": "secret_access_governance_report.json",
        "description": "Secrets, key rotation, principal mapping, production API key scope, SSO/RBAC, and network allowlist were approved.",
    },
    {
        "gate_id": "backup_restore_drill",
        "required_artifact": "backup_restore_drill_report.json",
        "description": "Backup manifest and restore drill completed with approved recovery target and data-loss scope.",
    },
    {
        "gate_id": "rollback_drill",
        "required_artifact": "rollback_drill_report.json",
        "description": "Previous approved deployment package can be restored or reverted with audit evidence.",
    },
    {
        "gate_id": "alert_router_delivery",
        "required_artifact": "alert_router_delivery_report.json",
        "description": "MLOps alert delivery reached the customer alert router and receipt was reviewed.",
    },
    {
        "gate_id": "retention_legal_hold",
        "required_artifact": "retention_legal_hold_report.json",
        "description": "Retention windows, legal holds, masking, and human-approved destruction workflow were configured.",
        "acceptance_checks": RETENTION_LEGAL_HOLD_ACCEPTANCE_CHECKS,
    },
    {
        "gate_id": "customer_data_governance",
        "required_artifact": "customer_data_governance_report.json",
        "description": "Customer dataset provenance, label provenance, holdout split, and live shadow-traffic plan were approved.",
        "acceptance_checks": CUSTOMER_DATA_GOVERNANCE_ACCEPTANCE_CHECKS,
    },
    {
        "gate_id": "worker_data_pipeline_execution",
        "required_artifact": "worker_data_pipeline_execution_report.json",
        "description": "Customer scheduler executed the governed worker data pipeline with readiness evidence, run-status evidence, artifact submit/write evidence, scoring readback evidence, and dependency-blocker review for sanctions, provider profiles, graph signals, peer benchmarks, episodes, clinical references, unbundling, scoring contexts, and probability calibration.",
        "required_job_kinds": WORKER_DATA_PIPELINE_JOB_KINDS,
        "governed_submit_job_kinds": WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS,
        "acceptance_checks": WORKER_DATA_PIPELINE_ACCEPTANCE_CHECKS,
    },
    {
        "gate_id": "scoring_online_readback",
        "required_artifact": "scoring_readback_report.json",
        "description": "Customer captured an online scoring response after governed worker writes and verified that expected worker evidence prefixes were present.",
        "acceptance_checks": SCORING_READBACK_ACCEPTANCE_CHECKS,
    },
    {
        "gate_id": "model_serving_slo",
        "required_artifact": "model_serving_slo_report.json",
        "description": "ONNX/Rust model serving latency, error, fallback, checksum, signature, and rollback SLO evidence passed.",
        "acceptance_checks": MODEL_SERVING_SLO_ACCEPTANCE_CHECKS,
    },
    {
        "gate_id": "ocr_vector_analytics_execution",
        "required_artifact": "ocr_vector_analytics_execution_report.json",
        "description": "OCR, embedding/vector store, retrieval ranking, ClickHouse export, retention, backup, and dashboard access were executed in the customer environment.",
        "acceptance_checks": OCR_VECTOR_ANALYTICS_ACCEPTANCE_CHECKS,
    },
]


def write_json(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def build_contract(output_dir: Path) -> dict:
    generated_at = datetime.now(timezone.utc).isoformat()
    gates = []
    for item in REQUIRED_EVIDENCE:
        gates.append(
            {
                **item,
                "status": "requires_customer_environment_evidence",
                "customer_data_required": item["gate_id"] in {
                    "customer_data_governance",
                    "worker_data_pipeline_execution",
                    "scoring_online_readback",
                    "model_serving_slo",
                    "ocr_vector_analytics_execution",
                },
            }
        )
    contract = {
        "artifact_kind": "production_readiness_evidence_contract",
        "generated_at": generated_at,
        "status": "blocked_until_live_environment_evidence",
        "customer_data_required": False,
        "readiness_claim_boundary": (
            "This contract proves the production evidence checklist exists; it does not "
            "claim production readiness until every required artifact is supplied and validated."
        ),
        "required_gate_count": len(gates),
        "required_gates": gates,
    }
    write_json(output_dir / "production_readiness_contract.json", contract)
    write_json(
        output_dir / "index.json",
        {
            "artifact_kind": "production_readiness_contract_index",
            "generated_at": generated_at,
            "artifacts": ["production_readiness_contract.json"],
            "customer_data_required": False,
        },
    )
    return contract


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", default=str(DEFAULT_OUTPUT_DIR))
    args = parser.parse_args()
    print(json.dumps(build_contract(Path(args.output_dir)), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

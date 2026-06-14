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
    "probability_calibration_evidence",
]

WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS = [
    job_kind
    for job_kind in WORKER_DATA_PIPELINE_JOB_KINDS
    if job_kind != "oig_sam_sanctions_snapshot_fetch"
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
        "check_id": "governed_submit_jobs_submitted",
        "description": "Every governed worker data-pipeline submit job has submitted = true, expected API path and permission scope, and a non-empty reported artifact URI.",
    },
    {
        "check_id": "source_snapshot_artifact_reported",
        "description": "The artifact-only OIG/SAM source snapshot job reports a non-empty artifact URI.",
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
    },
    {
        "gate_id": "customer_data_governance",
        "required_artifact": "customer_data_governance_report.json",
        "description": "Customer dataset provenance, label provenance, holdout split, and live shadow-traffic plan were approved.",
    },
    {
        "gate_id": "worker_data_pipeline_execution",
        "required_artifact": "worker_data_pipeline_execution_report.json",
        "description": "Customer scheduler executed the governed worker data pipeline with readiness evidence, run-status evidence, artifact submit/write evidence, and dependency-blocker review for sanctions, provider profiles, graph signals, peer benchmarks, episodes, clinical references, unbundling, scoring contexts, and probability calibration.",
        "required_job_kinds": WORKER_DATA_PIPELINE_JOB_KINDS,
        "governed_submit_job_kinds": WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS,
        "acceptance_checks": WORKER_DATA_PIPELINE_ACCEPTANCE_CHECKS,
    },
    {
        "gate_id": "model_serving_slo",
        "required_artifact": "model_serving_slo_report.json",
        "description": "ONNX/Rust model serving latency, error, fallback, checksum, signature, and rollback SLO evidence passed.",
    },
    {
        "gate_id": "ocr_vector_analytics_execution",
        "required_artifact": "ocr_vector_analytics_execution_report.json",
        "description": "OCR, embedding/vector store, retrieval ranking, ClickHouse export, retention, backup, and dashboard access were executed in the customer environment.",
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

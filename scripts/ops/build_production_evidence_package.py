#!/usr/bin/env python3
"""Build customer-fillable production readiness evidence templates.

The generated package is intentionally not production-ready evidence. It gives
customers the exact artifact filenames and field shapes to fill, while keeping
every report in a blocked/template state until real environment evidence is
supplied.
"""

from __future__ import annotations

import argparse
import json
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from scripts.ops.build_production_readiness_contract import REQUIRED_EVIDENCE, build_contract
from scripts.ops.validate_production_readiness_contract import (
    WORKER_DATA_PIPELINE_ADDITIONAL_JOB_EVIDENCE_PREFIXES,
    WORKER_DATA_PIPELINE_REQUIRED_JOB_KINDS,
    WORKER_DATA_PIPELINE_SCORING_READBACK_EVIDENCE_PREFIXES,
    WORKER_DATA_PIPELINE_SOURCE_SNAPSHOT_EVIDENCE_PREFIX,
    WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS,
    WORKER_DATA_PIPELINE_SUBMIT_JOB_EVIDENCE_PREFIXES,
    WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS,
    WORKER_DATA_PIPELINE_SUBMIT_JOB_PERMISSIONS,
)


DEFAULT_OUTPUT_DIR = Path("artifacts/production-evidence-package")
SOURCE_TEMPLATE_FILES = {
    "customer_data_governance": "customer-data-governance-source.json",
    "retention_legal_hold": "retention-legal-hold-source.json",
    "model_serving_slo": "model-serving-slo-source.json",
    "ocr_vector_analytics_execution": "ocr-vector-analytics-source.json",
}
WORKER_TEMPLATE_FILES = {
    "score_request": "score_request.json",
    "scoring_readback_input": "scoring_readback_input.json",
    "worker_data_pipeline_readiness_input": "worker_data_pipeline_readiness_input.json",
    "worker_data_pipeline_run_status": "worker_data_pipeline_run_status.json",
}
SCORING_READBACK_EXPECTED_SCORE_RESPONSE_PREFIXES = [
    "scoring_feature_contexts:",
    "provider_profile_window_rollups:",
    "sanctions_sync_reports:",
    "provider_graph_signal_rollups:",
    "peer_benchmarks:",
    "episode_rollups:",
    "clinical_compatibility:",
    "unbundling_candidates:",
]


def write_json(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def blocked_template(
    *,
    generated_at: str,
    gate_id: str,
    artifact_name: str,
    artifact_kind: str,
    customer_data_required: bool,
    fields: dict | None = None,
) -> dict:
    payload = {
        "artifact_kind": artifact_kind,
        "gate_id": gate_id,
        "generated_at": generated_at,
        "status": "pending_customer_evidence",
        "customer_data_required": customer_data_required,
        "readiness_claim": False,
        "template_boundary": (
            "Template only. Replace local://template placeholders with customer "
            "production artifact URIs and live execution evidence before validation."
        ),
        "required_customer_action": f"fill {artifact_name} with live evidence for {gate_id}",
        "evidence_refs": [],
    }
    if fields:
        payload.update(fields)
    return payload


def worker_job_template(job_kind: str) -> dict:
    evidence_refs = []
    if job_kind in WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS:
        evidence_refs.append(WORKER_DATA_PIPELINE_SUBMIT_JOB_EVIDENCE_PREFIXES[job_kind])
    if job_kind == "oig_sam_sanctions_snapshot_fetch":
        evidence_refs.append(WORKER_DATA_PIPELINE_SOURCE_SNAPSHOT_EVIDENCE_PREFIX)
    if job_kind == "scoring_online_readback":
        evidence_refs.extend(WORKER_DATA_PIPELINE_SCORING_READBACK_EVIDENCE_PREFIXES)
    evidence_refs.extend(
        WORKER_DATA_PIPELINE_ADDITIONAL_JOB_EVIDENCE_PREFIXES.get(job_kind, ())
    )
    job = {
        "job_kind": job_kind,
        "execution_status": "pending_customer_scheduler_run",
        "reported_status": "pending",
        "blocked_dependencies": ["customer_scheduler_not_run"],
        "reported_artifact_uri": f"local://template/{job_kind}.json",
        "required_evidence_prefixes": worker_required_evidence_prefixes(job_kind),
        "evidence_refs": [f"{prefix}local://template/{job_kind}.json" for prefix in evidence_refs],
    }
    if job_kind in WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS:
        job["submitted"] = False
        job["api_path"] = WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS[job_kind]
        job["required_permission"] = WORKER_DATA_PIPELINE_SUBMIT_JOB_PERMISSIONS[job_kind]
    return job


def worker_required_evidence_prefixes(job_kind: str) -> list[str]:
    prefixes = []
    if job_kind in WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS:
        prefixes.append(WORKER_DATA_PIPELINE_SUBMIT_JOB_EVIDENCE_PREFIXES[job_kind])
    if job_kind == "oig_sam_sanctions_snapshot_fetch":
        prefixes.append(WORKER_DATA_PIPELINE_SOURCE_SNAPSHOT_EVIDENCE_PREFIX)
    if job_kind == "scoring_online_readback":
        prefixes.extend(WORKER_DATA_PIPELINE_SCORING_READBACK_EVIDENCE_PREFIXES)
    prefixes.extend(WORKER_DATA_PIPELINE_ADDITIONAL_JOB_EVIDENCE_PREFIXES.get(job_kind, ()))
    return prefixes


def worker_readiness_check_template(job_kind: str) -> dict:
    return {
        "job_kind": job_kind,
        "artifact_uri": f"local://template/worker/{job_kind}.json",
        "customer_approved": False,
        "external_fetch_configured": False,
        "row_count": None,
        "minimum_row_count": 1,
        "coverage_window_days": None,
        "data_quality_status": "pending_customer_validation",
        "source_freshness_status": "pending_customer_validation",
        "required_evidence_prefixes": worker_required_evidence_prefixes(job_kind),
        "evidence_refs": [
            f"{prefix}local://template/worker/{job_kind}.json"
            for prefix in worker_required_evidence_prefixes(job_kind)
        ],
    }


def worker_run_status_job_template(job_kind: str) -> dict:
    return {
        "job_kind": job_kind,
        "status": "scheduled_pending_customer_execution",
        "artifact_uri": None,
        "submitted": False,
        "required_permission": WORKER_DATA_PIPELINE_SUBMIT_JOB_PERMISSIONS.get(job_kind),
        "api_path": WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS.get(job_kind),
        "required_evidence_prefixes": worker_required_evidence_prefixes(job_kind),
        "evidence_refs": [],
    }


def worker_template(template_id: str, generated_at: str) -> dict:
    base = {
        "artifact_kind": "worker_data_pipeline_input_template",
        "template_id": template_id,
        "generated_at": generated_at,
        "status": "pending_customer_input",
        "readiness_claim": False,
        "template_boundary": (
            "Template only. Replace local://template values with customer-approved "
            "production artifact URIs before using these inputs for readiness validation."
        ),
    }
    if template_id == "score_request":
        return {
            **base,
            "artifact_kind": "scoring_readback_score_request_template",
            "source_system": "<customer-source-system>",
            "claim_id": "<claim-id-with-governed-worker-context>",
            "review_mode": "pre_payment",
            "governance_boundary": (
                "No API keys, PHI fields, or real patient/member identifiers belong in this "
                "template. Use a customer-approved claim id whose worker-written evidence is "
                "expected to appear in the score response."
            ),
        }
    if template_id == "scoring_readback_input":
        return {
            **base,
            "artifact_kind": "scoring_readback_input_template",
            "customer_scope_id": "",
            "as_of_date": "",
            "score_request_uri": "artifacts/production-evidence-package/worker/score_request.json",
            "score_response_uri": (
                "artifacts/production-evidence-package/worker/scoring-readback/score_response.json"
            ),
            "expected_evidence_prefixes": SCORING_READBACK_EXPECTED_SCORE_RESPONSE_PREFIXES,
            "evidence_refs": [
                "worker_data_pipeline_executions:local://template/worker_data_pipeline_execution_report.json",
                "scoring_readback_score_requests:local://template/worker/score_request.json",
            ],
        }
    if template_id == "worker_data_pipeline_readiness_input":
        return {
            **base,
            "artifact_kind": "worker_data_pipeline_readiness_input_template",
            "checks": [
                worker_readiness_check_template(job_kind)
                for job_kind in sorted(WORKER_DATA_PIPELINE_REQUIRED_JOB_KINDS)
            ],
            "governance_boundary": (
                "Readiness inputs collect customer prerequisite evidence only; they must not "
                "fetch external data, submit artifacts, score claims, assign labels, activate "
                "models, or change routing policy."
            ),
        }
    if template_id == "worker_data_pipeline_run_status":
        return {
            **base,
            "artifact_kind": "worker_data_pipeline_run_status_template",
            "report_kind": "worker_data_pipeline_run_status",
            "run_status_template": True,
            "plan_uri": "local://template/worker_data_pipeline_plan.json",
            "readiness_report_uri": "local://template/worker_data_pipeline_readiness_report.json",
            "run_id": "<customer-scheduler-run-id>",
            "execution_date": "<yyyy-mm-dd>",
            "job_statuses": [
                worker_run_status_job_template(job_kind)
                for job_kind in sorted(WORKER_DATA_PIPELINE_REQUIRED_JOB_KINDS)
            ],
            "evidence_refs": [
                "worker_data_pipeline_plans:local://template/worker_data_pipeline_plan.json",
                "worker_data_pipeline_readiness_reports:local://template/worker_data_pipeline_readiness_report.json",
            ],
            "governance_boundary": (
                "Run-status inputs report customer scheduler results only; they must not score "
                "claims, assign labels, deny claims, activate models, or change routing policy."
            ),
        }
    raise ValueError(f"unknown worker template id: {template_id}")


def artifact_template(gate: dict, generated_at: str) -> dict:
    gate_id = gate["gate_id"]
    artifact_name = gate["required_artifact"]
    customer_data_required = bool(gate.get("customer_data_required"))
    if gate_id == "customer_data_governance":
        return blocked_template(
            generated_at=generated_at,
            gate_id=gate_id,
            artifact_name=artifact_name,
            artifact_kind="customer_data_governance_report",
            customer_data_required=True,
            fields={
                "dataset_provenance_status": "pending_customer_approval",
                "label_provenance_status": "pending_customer_approval",
                "holdout_split_status": "pending_customer_approval",
                "shadow_traffic_plan_status": "pending_customer_approval",
                "approved_label_count": 0,
                "holdout_claim_count": 0,
            },
        )
    if gate_id == "worker_data_pipeline_execution":
        jobs = [
            worker_job_template(job_kind)
            for job_kind in sorted(WORKER_DATA_PIPELINE_REQUIRED_JOB_KINDS)
        ]
        return blocked_template(
            generated_at=generated_at,
            gate_id=gate_id,
            artifact_name=artifact_name,
            artifact_kind="worker_data_pipeline_execution_report",
            customer_data_required=True,
            fields={
                "report_kind": "worker_data_pipeline_execution_report",
                "readiness_gate_status": "blocked",
                "plan_uri": "local://template/worker_data_pipeline_plan.json",
                "run_status_uri": "local://template/worker_data_pipeline_run_status.json",
                "readiness_report_uri": "local://template/worker_data_pipeline_readiness_report.json",
                "run_id": "pending_customer_scheduler_run_id",
                "execution_date": "pending_customer_scheduler_execution_date",
                "scheduler_status": "pending",
                "pending_or_failed_job_count": len(jobs),
                "review_task_count": 1,
                "job_count": len(jobs),
                "job_executions": jobs,
                "evidence_refs": [
                    "worker_data_pipeline_plans:local://template/worker_data_pipeline_plan.json",
                    "worker_data_pipeline_run_status:local://template/worker_data_pipeline_run_status.json",
                    "worker_data_pipeline_readiness_reports:local://template/worker_data_pipeline_readiness_report.json",
                ],
                "governance_boundary": (
                    "must not score claims, assign labels, deny claims, activate models, "
                    "or change routing policy"
                ),
            },
        )
    if gate_id == "scoring_online_readback":
        return blocked_template(
            generated_at=generated_at,
            gate_id=gate_id,
            artifact_name=artifact_name,
            artifact_kind="scoring_readback_report",
            customer_data_required=True,
            fields={
                "report_kind": "scoring_readback_report",
                "report_version": 1,
                "readback_status": "blocked",
                "execution_mode": "template_pending_score_response_artifact",
                "input_uri": "local://template/scoring_readback_input.json",
                "score_request_uri": "local://template/score_request.json",
                "score_response_uri": "local://template/score_response.json",
                "expected_evidence_prefix_count": 0,
                "matched_evidence_prefix_count": 0,
                "checks": [],
                "observed_evidence_refs": [],
                "blockers": ["customer_score_response_artifact_missing"],
                "review_task_count": 1,
                "review_tasks": [
                    {
                        "task_kind": "scoring_readback_evidence_required",
                        "severity": "blocker",
                        "summary": "Capture a customer-authorized score response after governed worker writes.",
                    }
                ],
                "evidence_refs": [
                    "scoring_readback_reports:local://template/scoring_readback_report.json",
                    "scoring_readback_inputs:local://template/scoring_readback_input.json",
                    "scoring_readback_score_requests:local://template/score_request.json",
                    "scoring_readback_score_responses:local://template/score_response.json",
                ],
            },
        )
    if gate_id == "retention_legal_hold":
        return blocked_template(
            generated_at=generated_at,
            gate_id=gate_id,
            artifact_name=artifact_name,
            artifact_kind="retention_legal_hold_report",
            customer_data_required=customer_data_required,
            fields={
                "retention_years": 0,
                "retention_policy_id": "",
                "legal_hold_policy_id": "",
                "archive_storage_uri": "local://template/archive-storage",
                "legal_hold_reconciliation_status": "pending",
                "destruction_workflow": "pending_customer_approval",
                "automated_destruction_enabled": False,
            },
        )
    if gate_id == "model_serving_slo":
        return blocked_template(
            generated_at=generated_at,
            gate_id=gate_id,
            artifact_name=artifact_name,
            artifact_kind="model_serving_slo_report",
            customer_data_required=True,
            fields={
                "model_key": "",
                "model_version": "",
                "latency_slo_ms": None,
                "p95_latency_ms": None,
                "error_rate_slo": None,
                "error_rate": None,
                "checksum_verified": False,
                "signature_verified": False,
                "fallback_status": "pending",
                "rollback_ready": False,
                "probability_calibration_status": "pending",
                "calibrated_probability_serving_active": False,
                "evidence_refs": [
                    "model_serving:local://template/model-serving-slo.json",
                    "model_artifact:local://template/model-artifact.json",
                    "probability_calibration_reports:local://template/probability-calibration-report.json",
                    "probability_calibration_input:local://template/probability-calibration-input.json",
                    "calibration_labels:local://template/calibration-labels.json",
                ],
            },
        )
    if gate_id == "ocr_vector_analytics_execution":
        return blocked_template(
            generated_at=generated_at,
            gate_id=gate_id,
            artifact_name=artifact_name,
            artifact_kind="ocr_vector_analytics_execution_report",
            customer_data_required=True,
            fields={
                "ocr_execution_status": "pending",
                "embedding_vector_status": "pending",
                "retrieval_ranking_status": "pending",
                "clickhouse_export_status": "pending",
                "dashboard_access_status": "pending",
                "analytics_retention_backup_status": "pending",
                "document_count": 0,
                "embedding_job_count": 0,
                "retrieval_audit_count": 0,
                "analytics_export_job_count": 0,
                "raw_phi_exported": None,
            },
        )
    return blocked_template(
        generated_at=generated_at,
        gate_id=gate_id,
        artifact_name=artifact_name,
        artifact_kind=artifact_name.removesuffix(".json"),
        customer_data_required=customer_data_required,
    )


def source_template(gate_id: str, generated_at: str) -> dict | None:
    base = {
        "artifact_kind": "production_readiness_source_template",
        "gate_id": gate_id,
        "generated_at": generated_at,
        "status": "pending_customer_input",
        "readiness_claim": False,
        "template_boundary": (
            "Source template only. Replace pending values with customer-approved "
            "production evidence before running the matching report builder."
        ),
    }
    if gate_id == "customer_data_governance":
        return {
            **base,
            "customer_scope_id": "",
            "as_of_date": "",
            "dataset_provenance_status": "pending_customer_approval",
            "label_provenance_status": "pending_customer_approval",
            "holdout_split_status": "pending_customer_approval",
            "shadow_traffic_plan_status": "pending_customer_approval",
            "approved_label_count": 0,
            "holdout_claim_count": 0,
            "evidence_refs": [
                "dataset_provenance:local://template/dataset-provenance.json",
                "label_provenance:local://template/label-provenance.json",
                "holdout_split:local://template/holdout-split.json",
                "shadow_traffic_plan:local://template/shadow-traffic-plan.json",
            ],
        }
    if gate_id == "retention_legal_hold":
        return {
            **base,
            "retention_years": 0,
            "retention_policy_id": "",
            "legal_hold_policy_id": "",
            "archive_storage_uri": "local://template/archive-storage",
            "legal_hold_reconciliation_status": "pending",
            "destruction_workflow": "pending_customer_approval",
            "automated_destruction_enabled": False,
            "evidence_refs": [
                "retention_policy:local://template/retention-policy.json",
                "legal_hold_policy:local://template/legal-hold-policy.json",
            ],
        }
    if gate_id == "model_serving_slo":
        return {
            **base,
            "model_key": "",
            "model_version": "",
            "latency_slo_ms": None,
            "p95_latency_ms": None,
            "error_rate_slo": None,
            "error_rate": None,
            "checksum_verified": False,
            "signature_verified": False,
            "fallback_status": "pending",
            "rollback_ready": False,
            "probability_calibration_status": "pending",
            "calibrated_probability_serving_active": False,
            "evidence_refs": [
                "model_serving:local://template/model-serving-slo.json",
                "model_artifact:local://template/model-artifact.json",
                "probability_calibration_reports:local://template/probability-calibration-report.json",
                "probability_calibration_input:local://template/probability-calibration-input.json",
                "calibration_labels:local://template/calibration-labels.json",
            ],
        }
    if gate_id == "ocr_vector_analytics_execution":
        return {
            **base,
            "ocr_execution_status": "pending",
            "embedding_vector_status": "pending",
            "retrieval_ranking_status": "pending",
            "clickhouse_export_status": "pending",
            "dashboard_access_status": "pending",
            "analytics_retention_backup_status": "pending",
            "document_count": 0,
            "embedding_job_count": 0,
            "retrieval_audit_count": 0,
            "analytics_export_job_count": 0,
            "raw_phi_exported": None,
            "evidence_refs": [
                "ai_evidence_execution:local://template/ai-evidence-execution.json",
                "ocr_outputs:local://template/ocr-outputs.json",
                "embedding_jobs:local://template/embedding-jobs.json",
                "retrieval_audits:local://template/retrieval-audits.json",
                "analytics_exports:local://template/analytics-exports.json",
                "clickhouse_dashboard:local://template/clickhouse-dashboard.json",
            ],
        }
    return None


def worker_pipeline_command_runbook(generated_at: str) -> dict:
    submit_actor = "worker:worker-data-pipeline-scheduler"
    submit_notes = "customer-approved worker data pipeline artifact write"
    return {
        "artifact_kind": "worker_data_pipeline_command_runbook",
        "generated_at": generated_at,
        "status": "pending_customer_execution",
        "readiness_claim": False,
        "secret_boundary": "No API keys or production secrets belong in this runbook.",
        "required_customer_inputs": [
            "production API base URL",
            "customer object-storage artifact root",
            "customer scope id",
            "daily and monthly scheduler cadence",
            "filled worker readiness input",
            "customer scheduler run-status artifact",
            "customer-authorized scoring request and score response artifacts",
        ],
        "commands": [
            {
                "step": "build_worker_data_pipeline_plan",
                "command": (
                    "cargo run --locked -p worker -- build-worker-data-pipeline-plan "
                    "--api-base-url <production-api-base-url> "
                    "--object-storage-uri <customer-artifact-root> "
                    "--customer-scope-id <customer-scope-id> "
                    "--daily-cron '<daily-cron>' --monthly-cron '<monthly-cron>' "
                    "> artifacts/production-evidence-package/worker/worker_data_pipeline_plan.json"
                ),
                "output": "worker/worker_data_pipeline_plan.json",
            },
            {
                "step": "build_readiness_input_template",
                "command": (
                    "cargo run --locked -p worker -- build-worker-data-pipeline-readiness-input-template "
                    "--plan artifacts/production-evidence-package/worker/worker_data_pipeline_plan.json "
                    "--output-dir artifacts/production-evidence-package/worker"
                ),
                "output": "worker/worker_data_pipeline_readiness_input_template.json",
            },
            {
                "step": "build_readiness_report",
                "command": (
                    "cargo run --locked -p worker -- build-worker-data-pipeline-readiness-report "
                    "--plan artifacts/production-evidence-package/worker/worker_data_pipeline_plan.json "
                    "--readiness-input artifacts/production-evidence-package/worker/worker_data_pipeline_readiness_input.json "
                    "--output-dir artifacts/production-evidence-package/worker"
                ),
                "output": "worker/worker_data_pipeline_readiness_report.json",
            },
            {
                "step": "fetch_oig_sam_sanctions_snapshot",
                "command": (
                    "cargo run --locked -p worker -- fetch-oig-sam-sanctions-snapshot "
                    "--oig-url <customer-approved-oig-compatible-json-url> "
                    "--sam-url <customer-approved-sam-compatible-json-url> "
                    "--source-date <yyyy-mm-dd> "
                    "--output-dir <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/sanctions/<as-of-date>"
                ),
                "output": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/sanctions/<as-of-date>/oig_sam_sanctions_snapshot.json",
            },
            {
                "step": "sync_oig_sam_sanctions",
                "command": (
                    "cargo run --locked -p worker -- sync-oig-sam-sanctions "
                    "--source-uri <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/sanctions/<as-of-date>/oig_sam_sanctions_snapshot.json "
                    "--run-date <as-of-date> "
                    "--output-dir <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/sanctions/<as-of-date>"
                ),
                "output": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/sanctions/<as-of-date>/sanctions_sync_report.json",
            },
            {
                "step": "build_provider_profile_windows",
                "command": (
                    "cargo run --locked -p worker -- build-provider-profile-windows "
                    "--claims-uri <customer-approved-provider-profile-claims-snapshot-uri> "
                    "--output-dir <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/provider-profile/<as-of-date>"
                ),
                "output": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/provider-profile/<as-of-date>/provider_profile_window_rollup_report.json",
            },
            {
                "step": "build_provider_graph_signals",
                "command": (
                    "cargo run --locked -p worker -- build-provider-graph-signals "
                    "--graph-uri <customer-approved-provider-graph-snapshot-uri> "
                    "--output-dir <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/provider-graph/<as-of-date>"
                ),
                "output": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/provider-graph/<as-of-date>/provider_graph_signal_rollup_report.json",
            },
            {
                "step": "build_peer_benchmarks",
                "command": (
                    "cargo run --locked -p worker -- build-peer-benchmarks "
                    "--claims-uri <customer-approved-peer-benchmark-claims-snapshot-uri> "
                    "--output-dir <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/peer-benchmark/<benchmark-month>"
                ),
                "output": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/peer-benchmark/<benchmark-month>/peer_percentile_benchmark.json",
            },
            {
                "step": "build_episode_aggregation",
                "command": (
                    "cargo run --locked -p worker -- build-episode-aggregation "
                    "--claims-uri <customer-approved-episode-claims-snapshot-uri> "
                    "--output-dir <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/episodes/<as-of-date>"
                ),
                "output": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/episodes/<as-of-date>/episode_aggregation_report.json",
            },
            {
                "step": "build_clinical_compatibility_reference",
                "command": (
                    "cargo run --locked -p worker -- build-clinical-compatibility-reference "
                    "--reference-uri <customer-approved-clinical-compatibility-reference-uri> "
                    "--output-dir <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/clinical-compatibility/<reference-version>"
                ),
                "output": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/clinical-compatibility/<reference-version>/clinical_compatibility_reference_report.json",
            },
            {
                "step": "build_unbundling_comparator",
                "command": (
                    "cargo run --locked -p worker -- build-unbundling-comparator "
                    "--input-uri <customer-approved-unbundling-comparator-input-uri> "
                    "--output-dir <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/unbundling/<as-of-date>"
                ),
                "output": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/unbundling/<as-of-date>/unbundling_comparator_report.json",
            },
            {
                "step": "build_scoring_feature_contexts",
                "command": (
                    "cargo run --locked -p worker -- build-scoring-feature-contexts "
                    "--claims-uri <customer-approved-scoring-context-claims-snapshot-uri> "
                    "--episode-rollups-uri <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/episodes/<as-of-date>/episode_rollups.json "
                    "--peer-benchmarks-uri <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/peer-benchmark/<benchmark-month>/peer_benchmarks.json "
                    "--clinical-compatibility-uri <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/clinical-compatibility/<reference-version>/clinical_compatibility_references.json "
                    "--unbundling-candidates-uri <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/unbundling/<as-of-date>/unbundling_comparator_candidates.json "
                    "--output-dir <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/scoring-contexts/<as-of-date>"
                ),
                "output": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/scoring-contexts/<as-of-date>/scoring_feature_context_report.json",
            },
            {
                "step": "build_probability_calibration_report",
                "command": (
                    "cargo run --locked -p worker -- build-probability-calibration-report "
                    "--source-uri <customer-labeled-holdout-predictions-uri> "
                    "--output-dir <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/probability-calibration/<benchmark-month>"
                ),
                "output": "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/probability-calibration/<benchmark-month>/probability_calibration_report.json",
            },
            {
                "step": "submit_readiness_report",
                "command": (
                    "cargo run --locked -p worker -- submit-worker-data-pipeline-readiness-report "
                    "--api-url <production-api-base-url> --api-key <runtime-secret-not-persisted> "
                    "--report artifacts/production-evidence-package/worker/worker_data_pipeline_readiness_report.json "
                    f"--actor {submit_actor} --notes '{submit_notes}'"
                ),
                "output": "api:/api/v1/ops/worker-data-pipeline-readiness",
            },
            {
                "step": "submit_sanctions_sync_report",
                "command": (
                    "cargo run --locked -p worker -- submit-sanctions-sync-report "
                    "--api-url <production-api-base-url> --api-key <runtime-secret-not-persisted> "
                    "--report <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/sanctions/<as-of-date>/sanctions_sync_report.json "
                    f"--actor {submit_actor} --notes '{submit_notes}'"
                ),
                "output": "api:/api/v1/ops/providers/sanctions-sync-reports",
            },
            {
                "step": "submit_provider_profile_window_rollup",
                "command": (
                    "cargo run --locked -p worker -- submit-provider-profile-window-rollup "
                    "--api-url <production-api-base-url> --api-key <runtime-secret-not-persisted> "
                    "--report <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/provider-profile/<as-of-date>/provider_profile_window_rollup_report.json "
                    f"--actor {submit_actor} --notes '{submit_notes}'"
                ),
                "output": "api:/api/v1/ops/providers/profile-window-rollups",
            },
            {
                "step": "submit_provider_graph_signal_rollup",
                "command": (
                    "cargo run --locked -p worker -- submit-provider-graph-signal-rollup "
                    "--api-url <production-api-base-url> --api-key <runtime-secret-not-persisted> "
                    "--report <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/provider-graph/<as-of-date>/provider_graph_signal_rollup_report.json "
                    f"--actor {submit_actor} --notes '{submit_notes}'"
                ),
                "output": "api:/api/v1/ops/providers/graph-signal-rollups",
            },
            {
                "step": "submit_peer_benchmark",
                "command": (
                    "cargo run --locked -p worker -- submit-peer-benchmark "
                    "--api-url <production-api-base-url> --api-key <runtime-secret-not-persisted> "
                    "--report <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/peer-benchmark/<benchmark-month>/peer_percentile_benchmark.json "
                    f"--actor {submit_actor} --notes '{submit_notes}'"
                ),
                "output": "api:/api/v1/ops/providers/peer-benchmarks",
            },
            {
                "step": "submit_episode_aggregation",
                "command": (
                    "cargo run --locked -p worker -- submit-episode-aggregation "
                    "--api-url <production-api-base-url> --api-key <runtime-secret-not-persisted> "
                    "--report <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/episodes/<as-of-date>/episode_aggregation_report.json "
                    f"--actor {submit_actor} --notes '{submit_notes}'"
                ),
                "output": "api:/api/v1/ops/providers/episode-rollups",
            },
            {
                "step": "submit_clinical_compatibility_reference",
                "command": (
                    "cargo run --locked -p worker -- submit-clinical-compatibility-reference "
                    "--api-url <production-api-base-url> --api-key <runtime-secret-not-persisted> "
                    "--report <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/clinical-compatibility/<reference-version>/clinical_compatibility_reference_report.json "
                    f"--actor {submit_actor} --notes '{submit_notes}'"
                ),
                "output": "api:/api/v1/ops/clinical-compatibility-references",
            },
            {
                "step": "submit_unbundling_comparator",
                "command": (
                    "cargo run --locked -p worker -- submit-unbundling-comparator "
                    "--api-url <production-api-base-url> --api-key <runtime-secret-not-persisted> "
                    "--report <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/unbundling/<as-of-date>/unbundling_comparator_report.json "
                    f"--actor {submit_actor} --notes '{submit_notes}'"
                ),
                "output": "api:/api/v1/ops/unbundling-comparator-candidates",
            },
            {
                "step": "submit_scoring_feature_contexts",
                "command": (
                    "cargo run --locked -p worker -- submit-scoring-feature-contexts "
                    "--api-url <production-api-base-url> --api-key <runtime-secret-not-persisted> "
                    "--report <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/scoring-contexts/<as-of-date>/scoring_feature_context_report.json "
                    "--materialization-id <customer-scope-id>:<as-of-date> "
                    f"--actor {submit_actor} --notes '{submit_notes}'"
                ),
                "output": "api:/api/v1/ops/scoring-feature-context-materializations",
            },
            {
                "step": "submit_probability_calibration_report",
                "command": (
                    "cargo run --locked -p worker -- submit-probability-calibration-report "
                    "--api-url <production-api-base-url> --api-key <runtime-secret-not-persisted> "
                    "--report <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/probability-calibration/<benchmark-month>/probability_calibration_report.json "
                    f"--actor {submit_actor} --notes '{submit_notes}'"
                ),
                "output": "api:/api/v1/ops/models/{model_key}/probability-calibration-reports",
            },
            {
                "step": "build_run_status_template",
                "command": (
                    "cargo run --locked -p worker -- build-worker-data-pipeline-run-status-template "
                    "--plan artifacts/production-evidence-package/worker/worker_data_pipeline_plan.json "
                    "--readiness-report artifacts/production-evidence-package/worker/worker_data_pipeline_readiness_report.json "
                    "--run-id <customer-scheduler-run-id> --execution-date <yyyy-mm-dd> "
                    "--output-dir artifacts/production-evidence-package/worker"
                ),
                "output": "worker/worker_data_pipeline_run_status_template.json",
            },
            {
                "step": "build_execution_report",
                "command": (
                    "cargo run --locked -p worker -- build-worker-data-pipeline-execution-report "
                    "--plan artifacts/production-evidence-package/worker/worker_data_pipeline_plan.json "
                    "--run-status artifacts/production-evidence-package/worker/worker_data_pipeline_run_status.json "
                    "--output-dir artifacts/production-evidence-package/evidence"
                ),
                "output": "evidence/worker_data_pipeline_execution_report.json",
            },
            {
                "step": "submit_execution_report",
                "command": (
                    "cargo run --locked -p worker -- submit-worker-data-pipeline-execution-report "
                    "--api-url <production-api-base-url> --api-key <runtime-secret-not-persisted> "
                    "--report artifacts/production-evidence-package/evidence/worker_data_pipeline_execution_report.json "
                    f"--actor {submit_actor} --notes '{submit_notes}'"
                ),
                "output": "api:/api/v1/ops/worker-data-pipeline-executions",
            },
            {
                "step": "fetch_score_response",
                "command": (
                    "cargo run --locked -p worker -- fetch-scoring-readback-response "
                    "--api-url <production-api-base-url> --api-key <runtime-secret-not-persisted> "
                    "--score-request-uri artifacts/production-evidence-package/worker/score_request.json "
                    "--output-dir artifacts/production-evidence-package/worker/scoring-readback"
                ),
                "output": "worker/scoring-readback/score_response.json",
            },
            {
                "step": "build_scoring_readback_report",
                "command": (
                    "cargo run --locked -p worker -- build-scoring-readback-report "
                    "--input-uri artifacts/production-evidence-package/worker/scoring_readback_input.json "
                    "--score-response-uri artifacts/production-evidence-package/worker/scoring-readback/score_response.json "
                    "--output-dir artifacts/production-evidence-package/evidence"
                ),
                "output": "evidence/scoring_readback_report.json",
            },
        ],
        "validation_command": (
            "python3 scripts/ops/validate_production_evidence_package.py "
            "--package-dir artifacts/production-evidence-package && "
            "python3 scripts/ops/validate_production_readiness_contract.py "
            "--contract-dir artifacts/production-evidence-package/contract "
            "--evidence-dir artifacts/production-evidence-package/evidence"
        ),
        "boundary": (
            "These commands package customer evidence. They must not score claims, assign labels, "
            "deny claims, activate models, or change routing policy."
        ),
    }


def build_evidence_package(output_dir: Path) -> dict:
    generated_at = datetime.now(timezone.utc).isoformat()
    contract_dir = output_dir / "contract"
    evidence_dir = output_dir / "evidence"
    source_dir = output_dir / "sources"
    worker_dir = output_dir / "worker"
    runbook_dir = output_dir / "runbooks"
    contract = build_contract(contract_dir)
    artifacts = []
    sources = []
    worker_templates = []
    for gate in contract["required_gates"]:
        artifact_name = gate["required_artifact"]
        gate_id = gate["gate_id"]
        write_json(evidence_dir / artifact_name, artifact_template(gate, generated_at))
        artifacts.append(
            {
                "gate_id": gate_id,
                "artifact": f"evidence/{artifact_name}",
                "status": "pending_customer_evidence",
                "customer_data_required": gate["customer_data_required"],
            }
        )
        source_name = SOURCE_TEMPLATE_FILES.get(gate_id)
        source = source_template(gate_id, generated_at)
        if source_name and source:
            write_json(source_dir / source_name, source)
            sources.append(
                {
                    "gate_id": gate_id,
                    "source": f"sources/{source_name}",
                    "status": "pending_customer_input",
                    "customer_data_required": gate["customer_data_required"],
                }
            )
    for template_id, template_name in WORKER_TEMPLATE_FILES.items():
        write_json(worker_dir / template_name, worker_template(template_id, generated_at))
        worker_templates.append(
            {
                "template_id": template_id,
                "template": f"worker/{template_name}",
                "status": "pending_customer_input",
                "customer_data_required": True,
            }
        )
    write_json(
        runbook_dir / "worker-data-pipeline-commands.json",
        worker_pipeline_command_runbook(generated_at),
    )
    package = {
        "artifact_kind": "production_readiness_evidence_package_template",
        "generated_at": generated_at,
        "status": "blocked_until_customer_artifacts_are_filled",
        "readiness_claim": False,
        "contract_dir": "contract",
        "evidence_dir": "evidence",
        "source_dir": "sources",
        "worker_dir": "worker",
        "runbook_dir": "runbooks",
        "artifact_count": len(artifacts),
        "source_template_count": len(sources),
        "worker_template_count": len(worker_templates),
        "runbook_count": 1,
        "artifacts": artifacts,
        "source_templates": sources,
        "worker_templates": worker_templates,
        "runbooks": [
            {
                "runbook": "runbooks/worker-data-pipeline-commands.json",
                "status": "pending_customer_execution",
                "customer_data_required": True,
            }
        ],
        "validation_command": (
            "python3 scripts/ops/validate_production_evidence_package.py "
            "--package-dir <package> && "
            "python3 scripts/ops/validate_production_readiness_contract.py "
            "--contract-dir <package>/contract --evidence-dir <package>/evidence"
        ),
    }
    write_json(output_dir / "index.json", package)
    return package


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", default=str(DEFAULT_OUTPUT_DIR))
    args = parser.parse_args()
    print(json.dumps(build_evidence_package(Path(args.output_dir)), indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

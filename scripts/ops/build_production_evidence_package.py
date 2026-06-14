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
        "evidence_refs": [f"{prefix}local://template/{job_kind}.json" for prefix in evidence_refs],
    }
    if job_kind in WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS:
        job["submitted"] = False
        job["api_path"] = WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS[job_kind]
        job["required_permission"] = WORKER_DATA_PIPELINE_SUBMIT_JOB_PERMISSIONS[job_kind]
    return job


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


def build_evidence_package(output_dir: Path) -> dict:
    generated_at = datetime.now(timezone.utc).isoformat()
    contract_dir = output_dir / "contract"
    evidence_dir = output_dir / "evidence"
    contract = build_contract(contract_dir)
    artifacts = []
    for gate in contract["required_gates"]:
        artifact_name = gate["required_artifact"]
        write_json(evidence_dir / artifact_name, artifact_template(gate, generated_at))
        artifacts.append(
            {
                "gate_id": gate["gate_id"],
                "artifact": f"evidence/{artifact_name}",
                "status": "pending_customer_evidence",
                "customer_data_required": gate["customer_data_required"],
            }
        )
    package = {
        "artifact_kind": "production_readiness_evidence_package_template",
        "generated_at": generated_at,
        "status": "blocked_until_customer_artifacts_are_filled",
        "readiness_claim": False,
        "contract_dir": "contract",
        "evidence_dir": "evidence",
        "artifact_count": len(artifacts),
        "artifacts": artifacts,
        "validation_command": (
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

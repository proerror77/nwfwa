#!/usr/bin/env python3
"""Regression tests for rendering production evidence package sources."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from scripts.ops.build_production_evidence_package import build_evidence_package
from scripts.ops.render_production_evidence_package import render_package
from scripts.ops.validate_production_readiness_contract import (
    validate_customer_data_governance_evidence,
    validate_model_serving_slo_evidence,
    validate_ocr_vector_analytics_execution_evidence,
    validate_retention_legal_hold_evidence,
)


class ProductionEvidencePackageRendererTests(unittest.TestCase):
    def test_renders_default_sources_as_blocked_reports(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)

            summary = render_package(package_dir)

            self.assertEqual(summary["rendered_count"], 4)
            self.assertEqual(summary["blocked_report_count"], 4)
            self.assertEqual(summary["worker_template_count"], 4)
            self.assertEqual(summary["pending_worker_template_count"], 4)
            self.assertEqual(summary["status"], "rendered_with_blockers")
            self.assertFalse(summary["readiness_claim"])
            self.assertIn("worker_templates", summary)
            self.assertTrue(
                any(
                    template["template"] == "worker/scoring_readback_input.json"
                    for template in summary["worker_templates"]
                )
            )
            self.assertTrue((package_dir / "render_summary.json").exists())

    def test_renders_filled_sources_into_validator_accepted_reports(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            source_dir = package_dir / "sources"
            evidence_dir = package_dir / "evidence"
            build_evidence_package(package_dir)
            _write_json(source_dir / "customer-data-governance-source.json", _customer_source())
            _write_json(source_dir / "retention-legal-hold-source.json", _retention_source())
            _write_json(source_dir / "model-serving-slo-source.json", _model_slo_source())
            _write_json(
                source_dir / "ocr-vector-analytics-source.json",
                _ocr_vector_analytics_source(),
            )

            summary = render_package(package_dir)

            self.assertEqual(summary["status"], "rendered_without_blockers")
            validate_customer_data_governance_evidence(
                _read_json(evidence_dir / "customer_data_governance_report.json")
            )
            validate_retention_legal_hold_evidence(
                _read_json(evidence_dir / "retention_legal_hold_report.json")
            )
            validate_model_serving_slo_evidence(
                _read_json(evidence_dir / "model_serving_slo_report.json")
            )
            validate_ocr_vector_analytics_execution_evidence(
                _read_json(evidence_dir / "ocr_vector_analytics_execution_report.json")
            )


def _write_json(path: Path, payload: dict) -> None:
    path.write_text(json.dumps(payload), encoding="utf-8")


def _read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _customer_source() -> dict:
    return {
        "dataset_provenance_status": "approved",
        "label_provenance_status": "approved",
        "holdout_split_status": "approved",
        "shadow_traffic_plan_status": "approved",
        "approved_label_count": 500,
        "holdout_claim_count": 120,
        "evidence_refs": [
            "dataset_provenance:s3://customer-prod/governance/dataset.json",
            "label_provenance:s3://customer-prod/governance/labels.json",
            "holdout_split:s3://customer-prod/governance/holdout.json",
            "shadow_traffic_plan:s3://customer-prod/governance/shadow.json",
        ],
    }


def _retention_source() -> dict:
    return {
        "retention_years": 6,
        "retention_policy_id": "retention-6y-v1",
        "legal_hold_policy_id": "legal-hold-v1",
        "archive_storage_uri": "s3://customer-prod-archive/audit",
        "legal_hold_reconciliation_status": "completed",
        "destruction_workflow": "human_approval_required_before_destroy",
        "automated_destruction_enabled": False,
        "evidence_refs": [
            "retention_policy:s3://customer-prod/retention/policy.json",
            "legal_hold_policy:s3://customer-prod/retention/legal-hold.json",
        ],
    }


def _model_slo_source() -> dict:
    return {
        "model_key": "baseline_fwa",
        "model_version": "0.1.0",
        "latency_slo_ms": 250,
        "p95_latency_ms": 120,
        "error_rate_slo": 0.01,
        "error_rate": 0.001,
        "checksum_verified": True,
        "signature_verified": True,
        "fallback_status": "healthy",
        "rollback_ready": True,
        "probability_calibration_status": "calibrated",
        "calibrated_probability_serving_active": True,
        "evidence_refs": [
            "model_serving:s3://customer-prod/serving/slo.json",
            "model_artifact:s3://customer-prod/models/baseline.onnx",
            "probability_calibration_reports:s3://customer-prod/calibration/report.json",
        ],
    }


def _ocr_vector_analytics_source() -> dict:
    return {
        "ocr_execution_status": "completed",
        "embedding_vector_status": "completed",
        "retrieval_ranking_status": "completed",
        "clickhouse_export_status": "completed",
        "dashboard_access_status": "completed",
        "analytics_retention_backup_status": "completed",
        "document_count": 10,
        "embedding_job_count": 2,
        "retrieval_audit_count": 8,
        "analytics_export_job_count": 3,
        "raw_phi_exported": False,
        "evidence_refs": [
            "ai_evidence_execution:s3://customer-prod/ai/run.json",
            "ocr_outputs:s3://customer-prod/ocr/output.json",
            "embedding_jobs:s3://customer-prod/vector/jobs.json",
            "retrieval_audits:s3://customer-prod/retrieval/audit.json",
            "analytics_exports:s3://customer-prod/clickhouse/export.json",
            "clickhouse_dashboard:s3://customer-prod/clickhouse/dashboard.json",
        ],
    }


if __name__ == "__main__":
    unittest.main()

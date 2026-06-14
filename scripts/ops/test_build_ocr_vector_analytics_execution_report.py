#!/usr/bin/env python3
"""Regression tests for OCR/vector/analytics execution evidence building."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from scripts.ops.build_ocr_vector_analytics_execution_report import (
    build_from_file,
    build_ocr_vector_analytics_execution_report,
)
from scripts.ops.validate_production_readiness_contract import (
    validate_ocr_vector_analytics_execution_evidence,
)


class OcrVectorAnalyticsExecutionReportTests(unittest.TestCase):
    def test_completed_execution_input_passes_production_validator(self) -> None:
        report = build_ocr_vector_analytics_execution_report(_completed_source())

        self.assertEqual(report["status"], "completed")
        self.assertEqual(report["blockers"], [])
        validate_ocr_vector_analytics_execution_evidence(report)

    def test_incomplete_execution_input_stays_blocked(self) -> None:
        source = _completed_source()
        source["retrieval_ranking_status"] = "pending"
        source["analytics_export_job_count"] = 0
        source["raw_phi_exported"] = True
        source["evidence_refs"] = [
            "ai_evidence_execution:s3://customer-prod/ai/run.json",
            "ocr_outputs:s3://customer-prod/ocr/output.json",
            "embedding_jobs:s3://customer-prod/vector/jobs.json",
        ]

        report = build_ocr_vector_analytics_execution_report(source)

        self.assertEqual(report["status"], "blocked")
        self.assertIn("retrieval_ranking_status_not_completed", report["blockers"])
        self.assertIn("analytics_export_job_count_missing_or_zero", report["blockers"])
        self.assertIn("raw_phi_exported_not_false", report["blockers"])
        self.assertIn("missing_retrieval_audits_evidence_ref", report["blockers"])
        with self.assertRaisesRegex(AssertionError, "retrieval_ranking_status completed"):
            validate_ocr_vector_analytics_execution_evidence(report)

    def test_cli_builder_writes_standard_artifact_name(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source_uri = root / "source.json"
            source_uri.write_text(json.dumps(_completed_source()), encoding="utf-8")

            report = build_from_file(str(source_uri), root / "out")
            saved = json.loads(
                (
                    root / "out" / "ocr_vector_analytics_execution_report.json"
                ).read_text(encoding="utf-8")
            )

        self.assertEqual(report["status"], "completed")
        self.assertEqual(saved["artifact_kind"], "ocr_vector_analytics_execution_report")


def _completed_source() -> dict:
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

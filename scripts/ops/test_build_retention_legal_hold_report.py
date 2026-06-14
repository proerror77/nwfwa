#!/usr/bin/env python3
"""Regression tests for retention/legal-hold evidence report building."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from scripts.ops.build_retention_legal_hold_report import (
    build_from_file,
    build_retention_legal_hold_report,
)
from scripts.ops.validate_production_readiness_contract import (
    validate_retention_legal_hold_evidence,
)


class RetentionLegalHoldReportTests(unittest.TestCase):
    def test_configured_retention_input_passes_production_validator(self) -> None:
        report = build_retention_legal_hold_report(_configured_source())

        self.assertEqual(report["status"], "configured")
        self.assertEqual(report["blockers"], [])
        validate_retention_legal_hold_evidence(report)

    def test_incomplete_retention_input_stays_blocked(self) -> None:
        source = _configured_source()
        source["retention_years"] = 3
        source["legal_hold_reconciliation_status"] = "pending"
        source["destruction_workflow"] = "automated_destroy"
        source["automated_destruction_enabled"] = True
        source["evidence_refs"] = [
            "retention_policy:s3://customer-prod/retention/policy.json"
        ]

        report = build_retention_legal_hold_report(source)

        self.assertEqual(report["status"], "blocked")
        self.assertIn("retention_years_below_six", report["blockers"])
        self.assertIn("legal_hold_reconciliation_not_completed", report["blockers"])
        self.assertIn("destruction_workflow_not_human_approved", report["blockers"])
        self.assertIn("automated_destruction_not_disabled", report["blockers"])
        self.assertIn("missing_legal_hold_policy_evidence_ref", report["blockers"])
        with self.assertRaisesRegex(AssertionError, "six retention years"):
            validate_retention_legal_hold_evidence(report)

    def test_cli_builder_writes_standard_artifact_name(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source_uri = root / "source.json"
            source_uri.write_text(json.dumps(_configured_source()), encoding="utf-8")

            report = build_from_file(str(source_uri), root / "out")
            saved = json.loads(
                (root / "out" / "retention_legal_hold_report.json").read_text(
                    encoding="utf-8"
                )
            )

        self.assertEqual(report["status"], "configured")
        self.assertEqual(saved["artifact_kind"], "retention_legal_hold_report")


def _configured_source() -> dict:
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


if __name__ == "__main__":
    unittest.main()

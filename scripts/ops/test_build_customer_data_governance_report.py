#!/usr/bin/env python3
"""Regression tests for customer data governance evidence report building."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from scripts.ops.build_customer_data_governance_report import (
    build_customer_data_governance_report,
    build_from_file,
)
from scripts.ops.validate_production_readiness_contract import (
    validate_customer_data_governance_evidence,
)


class CustomerDataGovernanceReportTests(unittest.TestCase):
    def test_approved_customer_governance_input_passes_production_validator(self) -> None:
        report = build_customer_data_governance_report(
            {
                "customer_scope_id": "customer-prod",
                "as_of_date": "2026-06-15",
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
        )

        self.assertEqual(report["status"], "approved")
        self.assertEqual(report["blockers"], [])
        validate_customer_data_governance_evidence(report)

    def test_incomplete_customer_governance_input_stays_blocked(self) -> None:
        report = build_customer_data_governance_report(
            {
                "dataset_provenance_status": "approved",
                "label_provenance_status": "pending_customer_approval",
                "holdout_split_status": "approved",
                "shadow_traffic_plan_status": "approved",
                "approved_label_count": 0,
                "holdout_claim_count": 120,
                "evidence_refs": [
                    "dataset_provenance:s3://customer-prod/governance/dataset.json",
                    "holdout_split:s3://customer-prod/governance/holdout.json",
                    "shadow_traffic_plan:s3://customer-prod/governance/shadow.json",
                ],
            }
        )

        self.assertEqual(report["status"], "blocked")
        self.assertIn("label_provenance_status_not_approved", report["blockers"])
        self.assertIn("approved_label_count_missing_or_zero", report["blockers"])
        self.assertIn("missing_label_provenance_evidence_ref", report["blockers"])
        with self.assertRaisesRegex(AssertionError, "label_provenance_status approved"):
            validate_customer_data_governance_evidence(report)

    def test_template_evidence_refs_do_not_approve_customer_governance(self) -> None:
        source = {
            "dataset_provenance_status": "approved",
            "label_provenance_status": "approved",
            "holdout_split_status": "approved",
            "shadow_traffic_plan_status": "approved",
            "approved_label_count": 500,
            "holdout_claim_count": 120,
            "evidence_refs": [
                "dataset_provenance:local://template/sources/customer-data-governance-source.json",
                "label_provenance:s3://customer-prod/governance/labels.json",
                "holdout_split:s3://customer-prod/governance/holdout.json",
                "shadow_traffic_plan:s3://customer-prod/governance/shadow.json",
            ],
        }

        report = build_customer_data_governance_report(source)

        self.assertEqual(report["status"], "blocked")
        self.assertIn("template_evidence_refs_not_replaced", report["blockers"])
        with self.assertRaisesRegex(AssertionError, "local://template"):
            validate_customer_data_governance_evidence(report)

    def test_file_evidence_refs_do_not_approve_customer_governance(self) -> None:
        source = {
            "dataset_provenance_status": "approved",
            "label_provenance_status": "approved",
            "holdout_split_status": "approved",
            "shadow_traffic_plan_status": "approved",
            "approved_label_count": 500,
            "holdout_claim_count": 120,
            "evidence_refs": [
                "dataset_provenance:file://tmp/customer-data-governance-source.json",
                "label_provenance:s3://customer-prod/governance/labels.json",
                "holdout_split:s3://customer-prod/governance/holdout.json",
                "shadow_traffic_plan:s3://customer-prod/governance/shadow.json",
            ],
        }

        report = build_customer_data_governance_report(source)

        self.assertEqual(report["status"], "blocked")
        self.assertIn("non_production_evidence_refs", report["blockers"])
        with self.assertRaisesRegex(AssertionError, "file://"):
            validate_customer_data_governance_evidence(report)

    def test_cli_builder_writes_standard_artifact_name(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source_uri = root / "source.json"
            source_uri.write_text(
                json.dumps(
                    {
                        "dataset_provenance_status": "approved",
                        "label_provenance_status": "approved",
                        "holdout_split_status": "approved",
                        "shadow_traffic_plan_status": "approved",
                        "approved_label_count": 10,
                        "holdout_claim_count": 5,
                        "evidence_refs": [
                            "dataset_provenance:s3://customer-prod/dataset.json",
                            "label_provenance:s3://customer-prod/labels.json",
                            "holdout_split:s3://customer-prod/holdout.json",
                            "shadow_traffic_plan:s3://customer-prod/shadow.json",
                        ],
                    }
                ),
                encoding="utf-8",
            )

            report = build_from_file(str(source_uri), root / "out")
            saved = json.loads(
                (root / "out" / "customer_data_governance_report.json").read_text(
                    encoding="utf-8"
                )
            )

        self.assertEqual(report["status"], "approved")
        self.assertEqual(saved["artifact_kind"], "customer_data_governance_report")


if __name__ == "__main__":
    unittest.main()

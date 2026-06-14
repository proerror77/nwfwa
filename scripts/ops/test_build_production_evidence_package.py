#!/usr/bin/env python3
"""Regression tests for production readiness evidence package templates."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from scripts.ops.build_production_evidence_package import build_evidence_package
from scripts.ops.build_customer_data_governance_report import (
    build_customer_data_governance_report,
)
from scripts.ops.validate_production_readiness_contract import (
    validate_contract,
    validate_customer_data_governance_evidence,
    validate_evidence_dir,
)


class ProductionEvidencePackageTemplateTests(unittest.TestCase):
    def test_builds_all_required_evidence_templates(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package = build_evidence_package(Path(temp_dir))
            gates = validate_contract(
                _read_json(Path(temp_dir) / "contract" / "production_readiness_contract.json")
            )
            artifacts = validate_evidence_dir(Path(temp_dir) / "evidence", gates)

            self.assertEqual(package["artifact_count"], len(gates))
            self.assertEqual(package["source_template_count"], 4)
            self.assertEqual(
                set(artifacts),
                {gate["required_artifact"] for gate in gates},
            )
            self.assertTrue(
                (Path(temp_dir) / "sources" / "customer-data-governance-source.json").exists()
            )
            self.assertTrue(
                (Path(temp_dir) / "sources" / "retention-legal-hold-source.json").exists()
            )
            self.assertTrue(
                (Path(temp_dir) / "sources" / "model-serving-slo-source.json").exists()
            )
            self.assertTrue(
                (Path(temp_dir) / "sources" / "ocr-vector-analytics-source.json").exists()
            )
            self.assertEqual(
                artifacts["worker_data_pipeline_execution_report.json"]["readiness_gate_status"],
                "blocked",
            )
            self.assertEqual(
                artifacts["scoring_readback_report.json"]["readback_status"],
                "blocked",
            )
            model_slo = artifacts["model_serving_slo_report.json"]
            self.assertIn("model_key", model_slo)
            self.assertIn("latency_slo_ms", model_slo)
            self.assertIn("checksum_verified", model_slo)
            retention = artifacts["retention_legal_hold_report.json"]
            self.assertIn("destruction_workflow", retention)
            self.assertNotIn("destruction_requires_human_approval", retention)

    def test_template_does_not_validate_as_customer_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            build_evidence_package(Path(temp_dir))
            report = _read_json(
                Path(temp_dir) / "evidence" / "customer_data_governance_report.json"
            )

        with self.assertRaisesRegex(AssertionError, "dataset_provenance_status approved"):
            validate_customer_data_governance_evidence(report)

    def test_source_templates_stay_blocked_until_customer_fills_them(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            build_evidence_package(Path(temp_dir))
            source = _read_json(
                Path(temp_dir) / "sources" / "customer-data-governance-source.json"
            )

        report = build_customer_data_governance_report(source)

        self.assertEqual(report["status"], "blocked")
        self.assertIn("dataset_provenance_status_not_approved", report["blockers"])


def _read_json(path: Path) -> dict:
    import json

    return json.loads(path.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()

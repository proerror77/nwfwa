#!/usr/bin/env python3
"""Regression tests for production readiness evidence package templates."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from scripts.ops.build_production_evidence_package import build_evidence_package
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
        self.assertEqual(
            set(artifacts),
            {gate["required_artifact"] for gate in gates},
        )
        self.assertEqual(
            artifacts["worker_data_pipeline_execution_report.json"]["readiness_gate_status"],
            "blocked",
        )
        self.assertEqual(
            artifacts["scoring_readback_report.json"]["readback_status"],
            "blocked",
        )

    def test_template_does_not_validate_as_customer_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            build_evidence_package(Path(temp_dir))
            report = _read_json(
                Path(temp_dir) / "evidence" / "customer_data_governance_report.json"
            )

        with self.assertRaisesRegex(AssertionError, "dataset_provenance_status approved"):
            validate_customer_data_governance_evidence(report)


def _read_json(path: Path) -> dict:
    import json

    return json.loads(path.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()

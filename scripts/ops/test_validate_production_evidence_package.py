#!/usr/bin/env python3
"""Regression tests for production evidence package validation."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from scripts.ops.build_production_evidence_package import build_evidence_package
from scripts.ops.validate_production_evidence_package import validate_package


class ProductionEvidencePackageValidatorTests(unittest.TestCase):
    def test_accepts_generated_template_package(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)

            summary = validate_package(package_dir)

        self.assertEqual(summary["status"], "valid_template_package")
        self.assertFalse(summary["readiness_claim"])
        self.assertEqual(summary["source_template_count"], 4)
        self.assertEqual(summary["worker_template_count"], 4)
        self.assertEqual(summary["runbook_count"], 1)

    def test_rejects_package_missing_scoring_readback_input(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            (package_dir / "worker" / "scoring_readback_input.json").unlink()

            with self.assertRaisesRegex(AssertionError, "worker_templates path missing"):
                validate_package(package_dir)

    def test_rejects_worker_template_with_forbidden_phi_placeholder(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            score_request_uri = package_dir / "worker" / "score_request.json"
            score_request_uri.write_text(
                score_request_uri.read_text(encoding="utf-8").replace(
                    "<claim-id-with-governed-worker-context>", "patientName"
                ),
                encoding="utf-8",
            )

            with self.assertRaisesRegex(AssertionError, "forbidden placeholder string"):
                validate_package(package_dir)


if __name__ == "__main__":
    unittest.main()

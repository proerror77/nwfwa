#!/usr/bin/env python3
"""Regression tests for model serving SLO evidence report building."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from scripts.ops.build_model_serving_slo_report import (
    build_from_file,
    build_model_serving_slo_report,
)
from scripts.ops.validate_production_readiness_contract import (
    validate_model_serving_slo_evidence,
)


class ModelServingSloReportTests(unittest.TestCase):
    def test_passing_slo_input_passes_production_validator(self) -> None:
        report = build_model_serving_slo_report(_passing_source())

        self.assertEqual(report["status"], "passed")
        self.assertEqual(report["blockers"], [])
        validate_model_serving_slo_evidence(report)

    def test_failed_slo_input_stays_blocked(self) -> None:
        source = _passing_source()
        source["p95_latency_ms"] = 900
        source["latency_slo_ms"] = 250
        source["probability_calibration_status"] = "uncalibrated_raw_sigmoid"
        source["calibrated_probability_serving_active"] = False
        source["evidence_refs"] = [
            "model_serving:s3://customer-prod/serving/slo.json",
            "model_artifact:s3://customer-prod/models/baseline.onnx",
        ]

        report = build_model_serving_slo_report(source)

        self.assertEqual(report["status"], "blocked")
        self.assertIn("p95_latency_ms_outside_slo", report["blockers"])
        self.assertIn("probability_calibration_status_not_calibrated", report["blockers"])
        self.assertIn("calibrated_probability_serving_active_not_true", report["blockers"])
        self.assertIn("missing_probability_calibration_reports_evidence_ref", report["blockers"])
        self.assertIn("missing_probability_calibration_input_evidence_ref", report["blockers"])
        self.assertIn("missing_calibration_labels_evidence_ref", report["blockers"])
        with self.assertRaisesRegex(AssertionError, "p95 latency"):
            validate_model_serving_slo_evidence(report)

    def test_cli_builder_writes_standard_artifact_name(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            source_uri = root / "source.json"
            source_uri.write_text(json.dumps(_passing_source()), encoding="utf-8")

            report = build_from_file(str(source_uri), root / "out")
            saved = json.loads(
                (root / "out" / "model_serving_slo_report.json").read_text(
                    encoding="utf-8"
                )
            )

        self.assertEqual(report["status"], "passed")
        self.assertEqual(saved["artifact_kind"], "model_serving_slo_report")


def _passing_source() -> dict:
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
            "probability_calibration_input:s3://customer-prod/calibration/input.json",
            "calibration_labels:s3://customer-prod/calibration/labels.json",
        ],
    }


if __name__ == "__main__":
    unittest.main()

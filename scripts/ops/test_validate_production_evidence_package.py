#!/usr/bin/env python3
"""Regression tests for production evidence package validation."""

from __future__ import annotations

import tempfile
import unittest
import json
from pathlib import Path

from scripts.ops.build_production_evidence_package import build_evidence_package
from scripts.ops.render_production_evidence_package import render_package
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

    def test_rejects_scoring_readback_input_missing_provider_graph_prefix(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            input_uri = package_dir / "worker" / "scoring_readback_input.json"
            readback_input = _read_json(input_uri)
            readback_input["expected_evidence_prefixes"] = [
                prefix
                for prefix in readback_input["expected_evidence_prefixes"]
                if prefix != "provider_graph_signal_rollups:"
            ]
            _write_json(input_uri, readback_input)

            with self.assertRaisesRegex(AssertionError, "provider_graph_signal_rollups"):
                validate_package(package_dir)

    def test_rejects_scoring_readback_input_missing_worker_execution_ref(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            input_uri = package_dir / "worker" / "scoring_readback_input.json"
            readback_input = _read_json(input_uri)
            readback_input["evidence_refs"] = [
                reference
                for reference in readback_input["evidence_refs"]
                if not reference.startswith("worker_data_pipeline_executions:")
            ]
            _write_json(input_uri, readback_input)

            with self.assertRaisesRegex(
                AssertionError, "worker_data_pipeline_executions:"
            ):
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

    def test_rejects_evidence_template_that_claims_readiness(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            report_uri = package_dir / "evidence" / "model_serving_slo_report.json"
            report = _read_json(report_uri)
            report["readiness_claim"] = True
            _write_json(report_uri, report)

            with self.assertRaisesRegex(AssertionError, "must not claim readiness"):
                validate_package(package_dir)

    def test_rejects_worker_execution_template_marked_ready(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            report_uri = package_dir / "evidence" / "worker_data_pipeline_execution_report.json"
            report = _read_json(report_uri)
            report["readiness_gate_status"] = "ready"
            _write_json(report_uri, report)

            with self.assertRaisesRegex(
                AssertionError, "worker data pipeline execution template must remain blocked"
            ):
                validate_package(package_dir)

    def test_rejects_runbook_without_package_validator_command(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            runbook_uri = package_dir / "runbooks" / "worker-data-pipeline-commands.json"
            runbook = _read_json(runbook_uri)
            runbook["validation_command"] = runbook["validation_command"].replace(
                "python3 scripts/ops/validate_production_evidence_package.py "
                "--package-dir artifacts/production-evidence-package && ",
                "",
            )
            _write_json(runbook_uri, runbook)

            with self.assertRaisesRegex(
                AssertionError, "validate_production_evidence_package.py"
            ):
                validate_package(package_dir)

    def test_rejects_render_summary_with_missing_worker_templates(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            render_package(package_dir)
            summary_uri = package_dir / "render_summary.json"
            summary = _read_json(summary_uri)
            summary["worker_template_count"] = 0
            _write_json(summary_uri, summary)

            with self.assertRaisesRegex(AssertionError, "worker_template_count"):
                validate_package(package_dir)

    def test_rejects_runbook_missing_scoring_readback_step(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            runbook_uri = package_dir / "runbooks" / "worker-data-pipeline-commands.json"
            runbook = _read_json(runbook_uri)
            runbook["commands"] = [
                command
                for command in runbook["commands"]
                if command["step"] != "build_scoring_readback_report"
            ]
            _write_json(runbook_uri, runbook)

            with self.assertRaisesRegex(AssertionError, "build_scoring_readback_report"):
                validate_package(package_dir)

    def test_rejects_runbook_with_wrong_scoring_readback_input_path(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            runbook_uri = package_dir / "runbooks" / "worker-data-pipeline-commands.json"
            runbook = _read_json(runbook_uri)
            for command in runbook["commands"]:
                command["command"] = command["command"].replace(
                    "artifacts/production-evidence-package/worker/scoring_readback_input.json",
                    "artifacts/production-evidence-package/worker/scoring_readback_input_template.json",
                )
            _write_json(runbook_uri, runbook)

            with self.assertRaisesRegex(AssertionError, "scoring_readback_input.json"):
                validate_package(package_dir)


def _read_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _write_json(path: Path, payload: dict) -> None:
    path.write_text(json.dumps(payload), encoding="utf-8")


if __name__ == "__main__":
    unittest.main()

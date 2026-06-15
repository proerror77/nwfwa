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
            self.assertEqual(package["worker_template_count"], 4)
            self.assertEqual(package["runbook_count"], 1)
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
            self.assertTrue((Path(temp_dir) / "worker" / "score_request.json").exists())
            self.assertTrue(
                (Path(temp_dir) / "worker" / "scoring_readback_input.json").exists()
            )
            self.assertTrue(
                (
                    Path(temp_dir)
                    / "worker"
                    / "worker_data_pipeline_readiness_input.json"
                ).exists()
            )
            self.assertTrue(
                (Path(temp_dir) / "worker" / "worker_data_pipeline_run_status.json").exists()
            )
            runbook = _read_json(
                Path(temp_dir) / "runbooks" / "worker-data-pipeline-commands.json"
            )
            command_text = "\n".join(command["command"] for command in runbook["commands"])
            self.assertEqual(runbook["artifact_kind"], "worker_data_pipeline_command_runbook")
            self.assertIn("build-worker-data-pipeline-plan", command_text)
            self.assertIn("build-worker-data-pipeline-execution-report", command_text)
            self.assertIn("fetch-oig-sam-sanctions-snapshot", command_text)
            self.assertIn("sync-oig-sam-sanctions", command_text)
            self.assertIn("build-provider-profile-windows", command_text)
            self.assertIn("build-provider-graph-signals", command_text)
            self.assertIn("build-peer-benchmarks", command_text)
            self.assertIn("build-episode-aggregation", command_text)
            self.assertIn("build-clinical-compatibility-reference", command_text)
            self.assertIn("build-unbundling-comparator", command_text)
            self.assertIn("build-scoring-feature-contexts", command_text)
            self.assertIn("build-probability-calibration-report", command_text)
            self.assertIn("submit-worker-data-pipeline-readiness-report", command_text)
            self.assertIn("submit-sanctions-sync-report", command_text)
            self.assertIn("submit-provider-profile-window-rollup", command_text)
            self.assertIn("submit-provider-graph-signal-rollup", command_text)
            self.assertIn("submit-peer-benchmark", command_text)
            self.assertIn("submit-episode-aggregation", command_text)
            self.assertIn("submit-clinical-compatibility-reference", command_text)
            self.assertIn("submit-unbundling-comparator", command_text)
            self.assertIn("submit-scoring-feature-contexts", command_text)
            self.assertIn("submit-probability-calibration-report", command_text)
            self.assertIn("submit-worker-data-pipeline-execution-report", command_text)
            self.assertIn(
                "--published-report-uri <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/readiness/<as-of-date>/worker_data_pipeline_readiness_report.json",
                command_text,
            )
            self.assertIn(
                "--published-report-uri <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/execution/<customer-scheduler-run-id>/worker_data_pipeline_execution_report.json",
                command_text,
            )
            self.assertIn(
                "--published-report-uri <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/provider-profile/<as-of-date>/provider_profile_window_rollup_report.json",
                command_text,
            )
            self.assertIn(
                "--published-source-uri <customer-approved-provider-profile-claims-snapshot-uri>",
                command_text,
            )
            self.assertIn(
                "--published-report-uri <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/scoring-contexts/<as-of-date>/scoring_feature_context_report.json",
                command_text,
            )
            self.assertIn(
                "--published-input-uri <customer-labeled-holdout-predictions-uri>",
                command_text,
            )
            self.assertIn(
                "--published-label-uri <customer-approved-calibration-labels-uri>",
                command_text,
            )
            self.assertIn("fetch-scoring-readback-response", command_text)
            self.assertIn("build-scoring-readback-report", command_text)
            self.assertIn("runtime-secret-not-persisted", command_text)
            self.assertIn("No API keys", runbook["secret_boundary"])
            self.assertIn(
                "validate_production_evidence_package.py",
                runbook["validation_command"],
            )
            self.assertIn(
                "validate_production_readiness_contract.py",
                runbook["validation_command"],
            )
            self.assertIn(
                "validate_production_evidence_package.py",
                package["validation_command"],
            )
            self.assertIn(
                "validate_production_readiness_contract.py",
                package["validation_command"],
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
            self.assertIn(
                "probability_calibration_input:local://template/sources/probability-calibration-input.json",
                model_slo["evidence_refs"],
            )
            self.assertIn(
                "calibration_labels:local://template/sources/calibration-labels.json",
                model_slo["evidence_refs"],
            )
            retention = artifacts["retention_legal_hold_report.json"]
            self.assertIn("destruction_workflow", retention)
            self.assertNotIn("destruction_requires_human_approval", retention)
            scoring_readback_input = _read_json(
                Path(temp_dir) / "worker" / "scoring_readback_input.json"
            )
            self.assertEqual(
                scoring_readback_input["artifact_kind"],
                "scoring_readback_input_template",
            )
            self.assertIn(
                "scoring_feature_contexts:",
                scoring_readback_input["expected_evidence_prefixes"],
            )
            self.assertIn(
                "provider_graph_signal_rollups:",
                scoring_readback_input["expected_evidence_prefixes"],
            )
            self.assertTrue(
                scoring_readback_input["score_request_uri"].endswith(
                    "worker/score_request.json"
                )
            )
            score_request = _read_json(Path(temp_dir) / "worker" / "score_request.json")
            score_request_text = (Path(temp_dir) / "worker" / "score_request.json").read_text(
                encoding="utf-8"
            )
            self.assertEqual(
                score_request["artifact_kind"],
                "scoring_readback_score_request_template",
            )
            self.assertEqual(score_request["review_mode"], "pre_payment")
            self.assertNotIn("api_key", score_request_text.lower())
            self.assertNotIn("patientName", score_request_text)
            self.assertNotIn("certificateNo", score_request_text)
            self.assertNotIn("insuredName", score_request_text)
            readiness_input = _read_json(
                Path(temp_dir) / "worker" / "worker_data_pipeline_readiness_input.json"
            )
            self.assertEqual(
                readiness_input["artifact_kind"],
                "worker_data_pipeline_readiness_input_template",
            )
            self.assertEqual(len(readiness_input["checks"]), 11)
            self.assertTrue(
                any(
                    check["job_kind"] == "scoring_online_readback"
                    and "scoring_readback_score_responses:"
                    in check["required_evidence_prefixes"]
                    for check in readiness_input["checks"]
                )
            )
            run_status = _read_json(
                Path(temp_dir) / "worker" / "worker_data_pipeline_run_status.json"
            )
            self.assertEqual(run_status["report_kind"], "worker_data_pipeline_run_status")
            self.assertEqual(len(run_status["job_statuses"]), 11)

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

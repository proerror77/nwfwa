#!/usr/bin/env python3
"""Regression tests for production evidence package validation."""

from __future__ import annotations

import copy
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

    def test_rejects_source_template_that_claims_readiness(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            source_uri = package_dir / "sources" / "model-serving-slo-source.json"
            source = _read_json(source_uri)
            source["readiness_claim"] = True
            _write_json(source_uri, source)

            with self.assertRaisesRegex(AssertionError, "must not claim readiness"):
                validate_package(package_dir)

    def test_rejects_source_template_missing_required_evidence_ref(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            source_uri = package_dir / "sources" / "model-serving-slo-source.json"
            source = _read_json(source_uri)
            source["evidence_refs"] = [
                reference
                for reference in source["evidence_refs"]
                if not reference.startswith("calibration_labels:")
            ]
            _write_json(source_uri, source)

            with self.assertRaisesRegex(
                AssertionError, "calibration_labels:"
            ):
                validate_package(package_dir)

    def test_rejects_model_serving_slo_template_missing_calibration_input_ref(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            report_uri = package_dir / "evidence" / "model_serving_slo_report.json"
            report = _read_json(report_uri)
            report["evidence_refs"] = [
                reference
                for reference in report["evidence_refs"]
                if not reference.startswith("probability_calibration_input:")
            ]
            _write_json(report_uri, report)

            with self.assertRaisesRegex(
                AssertionError, "probability_calibration_input:"
            ):
                validate_package(package_dir)

    def test_rejects_model_serving_slo_template_with_short_calibration_ref(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            report_uri = package_dir / "evidence" / "model_serving_slo_report.json"
            report = _read_json(report_uri)
            report["evidence_refs"] = [
                reference.replace(
                    "local://template/worker/probability-calibration/<benchmark-month>/probability_calibration_report.json",
                    "local://template/probability-calibration-report.json",
                )
                for reference in report["evidence_refs"]
            ]
            _write_json(report_uri, report)

            with self.assertRaisesRegex(
                AssertionError, "package-relative template URIs"
            ):
                validate_package(package_dir)

    def test_rejects_worker_readiness_input_missing_required_job(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            input_uri = package_dir / "worker" / "worker_data_pipeline_readiness_input.json"
            readiness_input = _read_json(input_uri)
            readiness_input["checks"] = [
                check
                for check in readiness_input["checks"]
                if check["job_kind"] != "provider_graph_signal_rollup"
            ]
            _write_json(input_uri, readiness_input)

            with self.assertRaisesRegex(AssertionError, "job kind set"):
                validate_package(package_dir)

    def test_rejects_worker_readiness_input_missing_required_prefix(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            input_uri = package_dir / "worker" / "worker_data_pipeline_readiness_input.json"
            readiness_input = _read_json(input_uri)
            for check in readiness_input["checks"]:
                if check["job_kind"] == "provider_profile_window_rollup":
                    check["required_evidence_prefixes"] = [
                        prefix
                        for prefix in check["required_evidence_prefixes"]
                        if prefix != "provider_profile_window_rollups:"
                    ]
            _write_json(input_uri, readiness_input)

            with self.assertRaisesRegex(AssertionError, "provider_profile_window_rollup"):
                validate_package(package_dir)

    def test_rejects_worker_readiness_input_missing_required_submit_flags(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            input_uri = package_dir / "worker" / "worker_data_pipeline_readiness_input.json"
            readiness_input = _read_json(input_uri)
            for check in readiness_input["checks"]:
                if check["job_kind"] == "provider_profile_window_rollup":
                    check["required_submit_flags"] = ["--published-report-uri"]
            _write_json(input_uri, readiness_input)

            with self.assertRaisesRegex(AssertionError, "required_submit_flags"):
                validate_package(package_dir)

    def test_rejects_worker_run_status_wrong_submit_permission(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            run_status_uri = package_dir / "worker" / "worker_data_pipeline_run_status.json"
            run_status = _read_json(run_status_uri)
            for job in run_status["job_statuses"]:
                if job["job_kind"] == "peer_percentile_benchmark":
                    job["required_permission"] = "ops:providers:read"
            _write_json(run_status_uri, run_status)

            with self.assertRaisesRegex(AssertionError, "required_permission"):
                validate_package(package_dir)

    def test_rejects_worker_run_status_wrong_required_submit_flags(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            run_status_uri = package_dir / "worker" / "worker_data_pipeline_run_status.json"
            run_status = _read_json(run_status_uri)
            for job in run_status["job_statuses"]:
                if job["job_kind"] == "probability_calibration_evidence":
                    job["required_submit_flags"] = [
                        "--published-report-uri",
                        "--published-input-uri",
                    ]
            _write_json(run_status_uri, run_status)

            with self.assertRaisesRegex(AssertionError, "required_submit_flags"):
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

    def test_rejects_worker_execution_template_missing_required_job(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            report_uri = package_dir / "evidence" / "worker_data_pipeline_execution_report.json"
            report = _read_json(report_uri)
            report["job_executions"] = [
                job
                for job in report["job_executions"]
                if job["job_kind"] != "episode_aggregation"
            ]
            report["job_count"] = len(report["job_executions"])
            _write_json(report_uri, report)

            with self.assertRaisesRegex(AssertionError, "job kind set"):
                validate_package(package_dir)

    def test_rejects_worker_execution_template_duplicate_job_kind(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            report_uri = package_dir / "evidence" / "worker_data_pipeline_execution_report.json"
            report = _read_json(report_uri)
            report["job_executions"].append(copy.deepcopy(report["job_executions"][0]))
            report["job_count"] = len(report["job_executions"])
            report["review_tasks"].append(copy.deepcopy(report["review_tasks"][0]))
            report["review_task_count"] = len(report["review_tasks"])
            _write_json(report_uri, report)

            with self.assertRaisesRegex(AssertionError, "duplicate job_kind"):
                validate_package(package_dir)

    def test_rejects_worker_execution_template_missing_job_required_prefix(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            report_uri = package_dir / "evidence" / "worker_data_pipeline_execution_report.json"
            report = _read_json(report_uri)
            provider_profile_job = next(
                job
                for job in report["job_executions"]
                if job["job_kind"] == "provider_profile_window_rollup"
            )
            provider_profile_job["required_evidence_prefixes"] = [
                prefix
                for prefix in provider_profile_job["required_evidence_prefixes"]
                if prefix != "provider_profile_claim_snapshot:"
            ]
            _write_json(report_uri, report)

            with self.assertRaisesRegex(
                AssertionError, "required_evidence_prefixes changed unexpectedly"
            ):
                validate_package(package_dir)

    def test_rejects_worker_execution_template_wrong_job_required_submit_flags(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            report_uri = package_dir / "evidence" / "worker_data_pipeline_execution_report.json"
            report = _read_json(report_uri)
            provider_profile_job = next(
                job
                for job in report["job_executions"]
                if job["job_kind"] == "provider_profile_window_rollup"
            )
            provider_profile_job["required_submit_flags"] = ["--published-report-uri"]
            _write_json(report_uri, report)

            with self.assertRaisesRegex(AssertionError, "required_submit_flags"):
                validate_package(package_dir)

    def test_rejects_worker_execution_template_wrong_review_task_submit_flags(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            report_uri = package_dir / "evidence" / "worker_data_pipeline_execution_report.json"
            report = _read_json(report_uri)
            provider_profile_task = next(
                task
                for task in report["review_tasks"]
                if task["job_kind"] == "provider_profile_window_rollup"
            )
            provider_profile_task["required_submit_flags"] = ["--published-report-uri"]
            _write_json(report_uri, report)

            with self.assertRaisesRegex(AssertionError, "required_submit_flags"):
                validate_package(package_dir)

    def test_rejects_worker_execution_template_wrong_review_task_permission(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            report_uri = package_dir / "evidence" / "worker_data_pipeline_execution_report.json"
            report = _read_json(report_uri)
            provider_profile_task = next(
                task
                for task in report["review_tasks"]
                if task["job_kind"] == "provider_profile_window_rollup"
            )
            provider_profile_task["required_permission"] = "ops:datasets:write"
            _write_json(report_uri, report)

            with self.assertRaisesRegex(AssertionError, "required_permission"):
                validate_package(package_dir)

    def test_rejects_worker_execution_template_missing_review_tasks(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            report_uri = package_dir / "evidence" / "worker_data_pipeline_execution_report.json"
            report = _read_json(report_uri)
            report.pop("review_tasks")
            _write_json(report_uri, report)

            with self.assertRaisesRegex(AssertionError, "requires review_tasks"):
                validate_package(package_dir)

    def test_rejects_worker_execution_template_with_short_job_artifact_uri(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            report_uri = package_dir / "evidence" / "worker_data_pipeline_execution_report.json"
            report = _read_json(report_uri)
            job = report["job_executions"][0]
            job["reported_artifact_uri"] = job["reported_artifact_uri"].replace(
                "local://template/worker/",
                "local://template/",
            )
            _write_json(report_uri, report)

            with self.assertRaisesRegex(AssertionError, "wrong artifact URI"):
                validate_package(package_dir)

    def test_rejects_worker_run_status_template_with_short_plan_uri(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            run_status_uri = package_dir / "worker" / "worker_data_pipeline_run_status.json"
            run_status = _read_json(run_status_uri)
            run_status["plan_uri"] = "local://template/worker_data_pipeline_plan.json"
            _write_json(run_status_uri, run_status)

            with self.assertRaisesRegex(AssertionError, "worker run status plan_uri"):
                validate_package(package_dir)

    def test_rejects_scoring_readback_template_missing_response_ref(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            report_uri = package_dir / "evidence" / "scoring_readback_report.json"
            report = _read_json(report_uri)
            report["evidence_refs"] = [
                reference
                for reference in report["evidence_refs"]
                if not reference.startswith("scoring_readback_score_responses:")
            ]
            _write_json(report_uri, report)

            with self.assertRaisesRegex(
                AssertionError, "scoring_readback_score_responses:"
            ):
                validate_package(package_dir)

    def test_rejects_scoring_readback_template_with_short_score_request_ref(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            report_uri = package_dir / "evidence" / "scoring_readback_report.json"
            report = _read_json(report_uri)
            report["evidence_refs"] = [
                reference.replace(
                    "local://template/worker/score_request.json",
                    "local://template/score_request.json",
                )
                for reference in report["evidence_refs"]
            ]
            _write_json(report_uri, report)

            with self.assertRaisesRegex(
                AssertionError, "package-relative template URIs"
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

    def test_rejects_render_summary_claiming_no_blockers_while_workers_are_pending(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            render_package(package_dir)
            summary_uri = package_dir / "render_summary.json"
            summary = _read_json(summary_uri)
            summary["status"] = "rendered_without_blockers"
            _write_json(summary_uri, summary)

            with self.assertRaisesRegex(AssertionError, "rendered_without_blockers"):
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

    def test_rejects_runbook_missing_scoring_readback_published_uri(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            runbook_uri = package_dir / "runbooks" / "worker-data-pipeline-commands.json"
            runbook = _read_json(runbook_uri)
            command = next(
                command
                for command in runbook["commands"]
                if command["step"] == "build_scoring_readback_report"
            )
            command["command"] = command["command"].replace(
                "--published-score-response-uri <customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/scoring-readback/<customer-scheduler-run-id>/score_response.json ",
                "",
            )
            _write_json(runbook_uri, runbook)

            with self.assertRaisesRegex(AssertionError, "published-score-response-uri"):
                validate_package(package_dir)

    def test_rejects_runbook_missing_governed_submit_command(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            runbook_uri = package_dir / "runbooks" / "worker-data-pipeline-commands.json"
            runbook = _read_json(runbook_uri)
            runbook["commands"] = [
                command
                for command in runbook["commands"]
                if command["step"] != "submit_provider_profile_window_rollup"
            ]
            _write_json(runbook_uri, runbook)

            with self.assertRaisesRegex(
                AssertionError, "submit-provider-profile-window-rollup"
            ):
                validate_package(package_dir)

    def test_rejects_runbook_missing_governed_artifact_build_command(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            runbook_uri = package_dir / "runbooks" / "worker-data-pipeline-commands.json"
            runbook = _read_json(runbook_uri)
            runbook["commands"] = [
                command
                for command in runbook["commands"]
                if command["step"] != "build_peer_benchmarks"
            ]
            _write_json(runbook_uri, runbook)

            with self.assertRaisesRegex(AssertionError, "build-peer-benchmarks"):
                validate_package(package_dir)

    def test_rejects_runbook_artifact_build_step_with_wrong_output(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            runbook_uri = package_dir / "runbooks" / "worker-data-pipeline-commands.json"
            runbook = _read_json(runbook_uri)
            for command in runbook["commands"]:
                if command["step"] == "build_peer_benchmarks":
                    command["output"] = "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/peer-benchmark/<benchmark-month>/wrong.json"
            _write_json(runbook_uri, runbook)

            with self.assertRaisesRegex(
                AssertionError,
                "build_peer_benchmarks output must be "
                "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/peer-benchmark/<benchmark-month>/peer_percentile_benchmark.json",
            ):
                validate_package(package_dir)

    def test_rejects_runbook_scoring_context_with_wrong_peer_input_uri(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            runbook_uri = package_dir / "runbooks" / "worker-data-pipeline-commands.json"
            runbook = _read_json(runbook_uri)
            for command in runbook["commands"]:
                if command["step"] == "build_scoring_feature_contexts":
                    command["command"] = command["command"].replace(
                        "peer_percentile_benchmark.json",
                        "peer_benchmarks.json",
                    )
            _write_json(runbook_uri, runbook)

            with self.assertRaisesRegex(
                AssertionError,
                "--peer-benchmarks-uri must be "
                "<customer-artifact-root>/worker-data-pipelines/<customer-scope-id>/peer-benchmark/<benchmark-month>/peer_percentile_benchmark.json",
            ):
                validate_package(package_dir)

    def test_rejects_runbook_submit_step_with_wrong_api_output(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            runbook_uri = package_dir / "runbooks" / "worker-data-pipeline-commands.json"
            runbook = _read_json(runbook_uri)
            for command in runbook["commands"]:
                if command["step"] == "submit_provider_graph_signal_rollup":
                    command["output"] = "api:/api/v1/ops/providers/profile-window-rollups"
            _write_json(runbook_uri, runbook)

            with self.assertRaisesRegex(
                AssertionError,
                "submit_provider_graph_signal_rollup output must be "
                "api:/api/v1/ops/providers/graph-signal-rollups",
            ):
                validate_package(package_dir)

    def test_rejects_runbook_submit_step_missing_published_uri(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            runbook_uri = package_dir / "runbooks" / "worker-data-pipeline-commands.json"
            runbook = _read_json(runbook_uri)
            for command in runbook["commands"]:
                if command["step"] == "submit_provider_profile_window_rollup":
                    command["command"] = command["command"].replace(
                        "--published-source-uri <customer-approved-provider-profile-claims-snapshot-uri> ",
                        "",
                    )
            _write_json(runbook_uri, runbook)

            with self.assertRaisesRegex(
                AssertionError,
                "--published-source-uri must be "
                "<customer-approved-provider-profile-claims-snapshot-uri>",
            ):
                validate_package(package_dir)

    def test_rejects_probability_calibration_submit_missing_published_label_uri(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            runbook_uri = package_dir / "runbooks" / "worker-data-pipeline-commands.json"
            runbook = _read_json(runbook_uri)
            for command in runbook["commands"]:
                if command["step"] == "submit_probability_calibration_report":
                    command["command"] = command["command"].replace(
                        "--published-label-uri <customer-approved-calibration-labels-uri> ",
                        "",
                    )
            _write_json(runbook_uri, runbook)

            with self.assertRaisesRegex(
                AssertionError,
                "--published-label-uri must be "
                "<customer-approved-calibration-labels-uri>",
            ):
                validate_package(package_dir)

    def test_rejects_probability_calibration_build_missing_expected_label_uri(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            package_dir = Path(temp_dir)
            build_evidence_package(package_dir)
            runbook_uri = package_dir / "runbooks" / "worker-data-pipeline-commands.json"
            runbook = _read_json(runbook_uri)
            for command in runbook["commands"]:
                if command["step"] == "build_probability_calibration_report":
                    command["command"] = command["command"].replace(
                        "--expected-label-source-uri <customer-approved-calibration-labels-uri> ",
                        "",
                    )
            _write_json(runbook_uri, runbook)

            with self.assertRaisesRegex(
                AssertionError,
                "--expected-label-source-uri must be "
                "<customer-approved-calibration-labels-uri>",
            ):
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

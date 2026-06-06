import contextlib
import io
import json
import sys
import unittest
from pathlib import Path
from unittest import mock


sys.path.insert(0, str(Path(__file__).resolve().parent))
import mlops_training_handoff  # noqa: E402


def provider_output():
    return {
        "candidate_model_version": "0.1.0-candidate-job_1",
        "artifact_uri": "data/model-artifacts/baseline_fwa/0.1.0-candidate-job_1/rust_serving_artifact.json",
        "artifact_sha256": "sha256:abc",
        "validation_report_uri": "data/model-artifacts/baseline_fwa/0.1.0-candidate-job_1/validation.json",
        "evaluation_run_id": "eval_baseline_fwa_0_1_0_candidate_job_1",
        "metrics_json": {"algorithm": "logistic_regression"},
        "evidence_refs": ["model_retraining_jobs:job_1"],
        "mined_rule_owner": "external-training-platform",
        "mined_rule_candidates": [
            {
                "rule_id": "candidate_amount_ratio",
                "conditions": [
                    {
                        "field": "claim_amount_to_limit_ratio",
                        "operator": ">=",
                        "value": 0.82,
                    }
                ],
            }
        ],
    }


def training_job():
    output = provider_output()
    return {
        "job_id": "job_1",
        "handoff_kind": "external_training_platform_job",
        "status": "completed",
        "submit_path": "/api/v1/ops/model-retraining-jobs/job_1/output",
        "provider_output": output,
    }


class MlopsTrainingHandoffTest(unittest.TestCase):
    def test_train_provider_output_calls_independent_training_job_api(self):
        with mock.patch.object(
            mlops_training_handoff,
            "request_json",
            return_value=training_job(),
        ) as request_mock:
            output = mlops_training_handoff.train_provider_output(
                "http://127.0.0.1:8001",
                "data/training/manifest.json",
                "data/model-artifacts",
                "baseline_fwa",
                "0.1.0",
                "job_1",
                "external-training-platform",
                "xgboost",
            )

        self.assertEqual(output["candidate_model_version"], "0.1.0-candidate-job_1")
        request_mock.assert_called_once_with(
            "http://127.0.0.1:8001",
            "POST",
            "/training-jobs",
            {
                "manifest_path": "data/training/manifest.json",
                "artifact_base_uri": "data/model-artifacts",
                "model_key": "baseline_fwa",
                "base_model_version": "0.1.0",
                "job_id": "job_1",
                "actor": "external-training-platform",
                "algorithm": "xgboost",
            },
        )

    def test_train_provider_output_polls_until_training_job_completes(self):
        queued_job = {
            "job_id": "job_1",
            "handoff_kind": "external_training_platform_job",
            "status": "queued",
            "submit_path": "/api/v1/ops/model-retraining-jobs/job_1/output",
            "provider_output": None,
        }
        running_job = {**queued_job, "status": "running"}
        with (
            mock.patch.object(
                mlops_training_handoff,
                "request_json",
                side_effect=[queued_job, running_job, training_job()],
            ) as request_mock,
            mock.patch.object(mlops_training_handoff.time, "sleep") as sleep_mock,
        ):
            output = mlops_training_handoff.train_provider_output(
                "http://127.0.0.1:8001",
                "data/training/manifest.json",
                "data/model-artifacts",
                "baseline_fwa",
                "0.1.0",
                "job_1",
                "external-training-platform",
                None,
                poll_interval_seconds=0.01,
                timeout_seconds=5.0,
            )

        self.assertEqual(output["candidate_model_version"], "0.1.0-candidate-job_1")
        self.assertEqual(request_mock.call_count, 3)
        self.assertEqual(
            request_mock.call_args_list[1].args,
            (
                "http://127.0.0.1:8001",
                "GET",
                "/training-jobs/job_1",
            ),
        )
        self.assertEqual(sleep_mock.call_count, 2)

    def test_register_provider_output_posts_to_fwa_retraining_output_api(self):
        with mock.patch.object(
            mlops_training_handoff,
            "request_json",
            return_value={"job_id": "job/1", "status": "completed"},
        ) as request_mock:
            response = mlops_training_handoff.register_provider_output(
                "http://127.0.0.1:8080",
                "dev-secret",
                "job/1",
                provider_output(),
            )

        self.assertEqual(response["status"], "completed")
        self.assertEqual(
            request_mock.call_args.args[2],
            "/api/v1/ops/model-retraining-jobs/job%2F1/output",
        )
        self.assertEqual(request_mock.call_args.kwargs["api_key"], "dev-secret")

    def test_main_can_train_write_output_and_register(self):
        with (
            mock.patch.object(
                mlops_training_handoff,
                "train_provider_output",
                return_value=provider_output(),
            ),
            mock.patch.object(
                mlops_training_handoff,
                "register_provider_output",
                return_value={"job_id": "job_1", "status": "completed"},
            ) as register_mock,
            mock.patch.object(mlops_training_handoff, "write_json_file") as write_mock,
        ):
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                exit_code = mlops_training_handoff.main(
                    [
                        "--manifest",
                        "data/training/manifest.json",
                        "--artifact-base-uri",
                        "data/model-artifacts",
                        "--base-model-version",
                        "0.1.0",
                        "--job-id",
                        "job_1",
                        "--write-provider-output",
                        "/tmp/provider-output.json",
                        "--register",
                        "--api-url",
                        "http://127.0.0.1:8080",
                        "--api-key",
                        "dev-secret",
                    ]
                )

        self.assertEqual(exit_code, 0)
        body = json.loads(stdout.getvalue())
        self.assertEqual(body["handoff_status"], "registered_with_fwa")
        self.assertEqual(body["provider_output_path"], "/tmp/provider-output.json")
        write_mock.assert_called_once()
        register_mock.assert_called_once()

    def test_validate_provider_output_rejects_missing_evidence(self):
        output = provider_output()
        output["evidence_refs"] = []

        with self.assertRaisesRegex(ValueError, "evidence_refs"):
            mlops_training_handoff.validate_provider_output(output)


if __name__ == "__main__":
    unittest.main()

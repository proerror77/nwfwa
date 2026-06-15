#!/usr/bin/env python3
"""Regression tests for production readiness evidence validation."""

from __future__ import annotations

import copy
import unittest

from scripts.ops.validate_production_readiness_contract import (
    SCORING_READBACK_REQUIRED_SCORE_RESPONSE_PREFIXES,
    WORKER_DATA_PIPELINE_ADDITIONAL_JOB_EVIDENCE_PREFIXES,
    WORKER_DATA_PIPELINE_REQUIRED_JOB_KINDS,
    WORKER_DATA_PIPELINE_SCORING_READBACK_EVIDENCE_PREFIXES,
    WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS,
    WORKER_DATA_PIPELINE_SUBMIT_JOB_EVIDENCE_PREFIXES,
    WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS,
    WORKER_DATA_PIPELINE_SUBMIT_JOB_PERMISSIONS,
    validate_model_serving_slo_evidence,
    validate_scoring_readback_evidence,
    validate_worker_data_pipeline_execution_evidence,
)


def worker_execution_report(include_snapshot_evidence: bool = True) -> dict:
    jobs = []
    for job_kind in sorted(WORKER_DATA_PIPELINE_REQUIRED_JOB_KINDS):
        evidence_refs = [f"job_execution:{job_kind}"]
        job = {
            "job_kind": job_kind,
            "execution_status": "completed",
            "reported_status": "succeeded",
            "blocked_dependencies": [],
            "reported_artifact_uri": f"s3://customer-prod-artifacts/{job_kind}.json",
            "evidence_refs": evidence_refs,
        }
        if job_kind in WORKER_DATA_PIPELINE_SUBMIT_JOB_KINDS:
            job["submitted"] = True
            job["api_path"] = WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS[job_kind]
            job["required_permission"] = WORKER_DATA_PIPELINE_SUBMIT_JOB_PERMISSIONS[
                job_kind
            ]
            evidence_refs.append(
                f"{WORKER_DATA_PIPELINE_SUBMIT_JOB_EVIDENCE_PREFIXES[job_kind]}"
                f"s3://customer-prod-artifacts/{job_kind}.json"
            )
        if job_kind == "scoring_feature_context_materialization":
            evidence_refs.extend(
                [
                    "episode_rollups:s3://customer-prod-artifacts/episode.json",
                    "peer_benchmarks:s3://customer-prod-artifacts/peer.json",
                    "clinical_compatibility:s3://customer-prod-artifacts/clinical.json",
                    "unbundling_candidates:s3://customer-prod-artifacts/unbundling.json",
                ]
            )
        for prefix in WORKER_DATA_PIPELINE_ADDITIONAL_JOB_EVIDENCE_PREFIXES.get(
            job_kind, ()
        ):
            if not any(reference.startswith(prefix) for reference in evidence_refs):
                evidence_refs.append(f"{prefix}s3://customer-prod-artifacts/{job_kind}.json")
        if job_kind == "oig_sam_sanctions_snapshot_fetch" and include_snapshot_evidence:
            evidence_refs.append(
                "oig_sam_snapshot:s3://customer-prod-artifacts/oig_sam_snapshot.json"
            )
        if job_kind == "scoring_online_readback":
            evidence_refs.extend(
                f"{prefix}s3://customer-prod-artifacts/scoring_readback.json"
                for prefix in WORKER_DATA_PIPELINE_SCORING_READBACK_EVIDENCE_PREFIXES
            )
        jobs.append(job)

    return {
        "report_kind": "worker_data_pipeline_execution_report",
        "readiness_gate_status": "ready",
        "plan_uri": "s3://customer-prod-artifacts/plan.json",
        "run_status_uri": "s3://customer-prod-artifacts/run_status.json",
        "readiness_report_uri": "s3://customer-prod-artifacts/readiness.json",
        "run_id": "run-2026-06-15",
        "execution_date": "2026-06-15",
        "scheduler_status": "completed",
        "pending_or_failed_job_count": 0,
        "review_task_count": 0,
        "job_count": len(jobs),
        "job_executions": jobs,
        "evidence_refs": [
            "worker_data_pipeline_plans:s3://customer-prod-artifacts/plan.json",
            "worker_data_pipeline_run_status:s3://customer-prod-artifacts/run_status.json",
            "worker_data_pipeline_readiness_reports:s3://customer-prod-artifacts/readiness.json",
        ],
        "governance_boundary": (
            "must not score claims, assign labels, deny claims, activate models, "
            "or change routing policy"
        ),
    }


def scoring_readback_report() -> dict:
    checks = [
        {
            "expected_evidence_prefix": prefix,
            "matched": True,
            "matched_evidence_refs": [
                f"{prefix}s3://customer-prod-artifacts/{prefix.rstrip(':')}/report.json"
            ],
        }
        for prefix in sorted(SCORING_READBACK_REQUIRED_SCORE_RESPONSE_PREFIXES)
    ]
    observed_evidence_refs = [
        reference
        for check in checks
        for reference in check["matched_evidence_refs"]
    ]
    return {
        "report_kind": "scoring_readback_report",
        "report_version": 1,
        "customer_scope_id": "customer-prod",
        "as_of_date": "2026-06-15",
        "readback_status": "verified",
        "execution_mode": "score_response_artifact_readback",
        "input_uri": "s3://customer-prod-artifacts/scoring-readback/input.json",
        "score_request_uri": "s3://customer-prod-artifacts/scoring-readback/request.json",
        "score_response_uri": "s3://customer-prod-artifacts/scoring-readback/response.json",
        "expected_evidence_prefix_count": len(checks),
        "matched_evidence_prefix_count": len(checks),
        "checks": checks,
        "observed_evidence_refs": observed_evidence_refs,
        "blockers": [],
        "review_task_count": 0,
        "review_tasks": [],
        "evidence_refs": [
            "scoring_readback_reports:s3://customer-prod-artifacts/scoring-readback/report.json",
            "scoring_readback_inputs:s3://customer-prod-artifacts/scoring-readback/input.json",
            "scoring_readback_score_requests:s3://customer-prod-artifacts/scoring-readback/request.json",
            "scoring_readback_score_responses:s3://customer-prod-artifacts/scoring-readback/response.json",
        ],
    }


def model_serving_slo_report() -> dict:
    return {
        "artifact_kind": "model_serving_slo_report",
        "status": "passed",
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
            "model_serving:s3://customer-prod-artifacts/serving/slo.json",
            "model_artifact:s3://customer-prod-artifacts/models/baseline.onnx",
            "probability_calibration_reports:s3://customer-prod-artifacts/calibration/report.json",
            "probability_calibration_input:s3://customer-prod-artifacts/calibration/input.json",
            "calibration_labels:s3://customer-prod-artifacts/calibration/labels.json",
        ],
    }


class ProductionReadinessContractValidationTests(unittest.TestCase):
    def test_worker_execution_requires_source_snapshot_evidence_prefix(self) -> None:
        report = worker_execution_report(include_snapshot_evidence=False)

        with self.assertRaisesRegex(AssertionError, "oig_sam_snapshot:"):
            validate_worker_data_pipeline_execution_evidence(report)

    def test_worker_execution_accepts_complete_source_snapshot_lineage(self) -> None:
        report = worker_execution_report(include_snapshot_evidence=True)

        validate_worker_data_pipeline_execution_evidence(copy.deepcopy(report))

    def test_worker_execution_requires_submit_job_source_lineage(self) -> None:
        report = worker_execution_report(include_snapshot_evidence=True)
        scoring_context_job = next(
            job
            for job in report["job_executions"]
            if job["job_kind"] == "scoring_feature_context_materialization"
        )
        scoring_context_job["evidence_refs"] = [
            reference
            for reference in scoring_context_job["evidence_refs"]
            if not reference.startswith("scoring_feature_context_claim_snapshot:")
        ]

        with self.assertRaisesRegex(
            AssertionError, "scoring_feature_context_claim_snapshot:"
        ):
            validate_worker_data_pipeline_execution_evidence(report)

    def test_worker_execution_rejects_template_job_evidence_refs(self) -> None:
        report = worker_execution_report(include_snapshot_evidence=True)
        peer_job = next(
            job
            for job in report["job_executions"]
            if job["job_kind"] == "peer_percentile_benchmark"
        )
        peer_job["evidence_refs"].append(
            "peer_benchmarks:local://template/worker/peer_percentile_benchmark.json"
        )

        with self.assertRaisesRegex(AssertionError, "local://template"):
            validate_worker_data_pipeline_execution_evidence(report)

    def test_worker_execution_rejects_template_top_level_evidence_refs(self) -> None:
        report = worker_execution_report(include_snapshot_evidence=True)
        report["evidence_refs"].append(
            "worker_data_pipeline_plans:local://template/worker/plan.json"
        )

        with self.assertRaisesRegex(AssertionError, "local://template"):
            validate_worker_data_pipeline_execution_evidence(report)

    def test_worker_execution_requires_probability_calibration_label_lineage(
        self,
    ) -> None:
        report = worker_execution_report(include_snapshot_evidence=True)
        probability_calibration_job = next(
            job
            for job in report["job_executions"]
            if job["job_kind"] == "probability_calibration_evidence"
        )
        probability_calibration_job["evidence_refs"] = [
            reference
            for reference in probability_calibration_job["evidence_refs"]
            if not reference.startswith("calibration_labels:")
        ]

        with self.assertRaisesRegex(AssertionError, "calibration_labels:"):
            validate_worker_data_pipeline_execution_evidence(report)

    def test_model_serving_slo_requires_probability_calibration_label_lineage(
        self,
    ) -> None:
        report = model_serving_slo_report()
        report["evidence_refs"] = [
            reference
            for reference in report["evidence_refs"]
            if not reference.startswith("calibration_labels:")
        ]

        with self.assertRaisesRegex(AssertionError, "calibration_labels:"):
            validate_model_serving_slo_evidence(report)

    def test_worker_execution_requires_scoring_readback_response_lineage(self) -> None:
        report = worker_execution_report(include_snapshot_evidence=True)
        readback_job = next(
            job
            for job in report["job_executions"]
            if job["job_kind"] == "scoring_online_readback"
        )
        readback_job["evidence_refs"] = [
            reference
            for reference in readback_job["evidence_refs"]
            if not reference.startswith("scoring_readback_score_responses:")
        ]

        with self.assertRaisesRegex(
            AssertionError, "scoring_readback_score_responses:"
        ):
            validate_worker_data_pipeline_execution_evidence(report)

    def test_scoring_readback_accepts_verified_response_artifact(self) -> None:
        validate_scoring_readback_evidence(scoring_readback_report())

    def test_scoring_readback_requires_all_worker_score_response_prefixes(self) -> None:
        report = scoring_readback_report()
        report["checks"] = [
            check
            for check in report["checks"]
            if check["expected_evidence_prefix"] != "provider_graph_signal_rollups:"
        ]
        report["expected_evidence_prefix_count"] = len(report["checks"])
        report["matched_evidence_prefix_count"] = len(report["checks"])

        with self.assertRaisesRegex(
            AssertionError, "provider_graph_signal_rollups:"
        ):
            validate_scoring_readback_evidence(report)

    def test_scoring_readback_rejects_template_input_uri(self) -> None:
        report = scoring_readback_report()
        report["input_uri"] = "local://template/worker/scoring_readback_input.json"

        with self.assertRaisesRegex(AssertionError, "input_uri"):
            validate_scoring_readback_evidence(report)

    def test_scoring_readback_rejects_template_matched_evidence_refs(self) -> None:
        report = scoring_readback_report()
        report["checks"][0]["matched_evidence_refs"].append(
            "peer_benchmarks:local://template/worker/peer.json"
        )

        with self.assertRaisesRegex(AssertionError, "local://template"):
            validate_scoring_readback_evidence(report)

    def test_scoring_readback_rejects_template_top_level_evidence_refs(self) -> None:
        report = scoring_readback_report()
        report["evidence_refs"].append(
            "scoring_readback_reports:local://template/evidence/scoring_readback_report.json"
        )

        with self.assertRaisesRegex(AssertionError, "local://template"):
            validate_scoring_readback_evidence(report)

    def test_scoring_readback_rejects_blocked_contract_only_report(self) -> None:
        report = scoring_readback_report()
        report["readback_status"] = "blocked"
        report["execution_mode"] = "contract_only_blocked"
        report["score_response_uri"] = None
        report["matched_evidence_prefix_count"] = 0
        report["blockers"] = ["score_response_uri_missing"]
        report["review_task_count"] = 1

        with self.assertRaisesRegex(AssertionError, "readback_status verified"):
            validate_scoring_readback_evidence(report)


if __name__ == "__main__":
    unittest.main()

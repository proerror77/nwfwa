#!/usr/bin/env python3
"""Build a PRD coverage summary from repository evidence."""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_OUTPUT_DIR = Path("artifacts/prd-coverage")


CAPABILITIES = [
    {
        "capability": "decision_boundary",
        "status": "implemented",
        "summary": "Assistive-by-default boundary is documented; automatic denial or straight-through approval is limited to customer-approved deterministic adjudication rules, while ML and Agent outputs remain non-adjudicating signals.",
        "evidence": [
            "docs/product/fwa-risk-operations-prd.md",
            "docs/project/ml-algorithm-strategy.md",
            "apps/api-server/src/routes/agent.rs",
            "apps/api-server/src/routes/ops_models.rs",
        ],
        "required_text": [
            ("docs/product/fwa-risk-operations-prd.md", "No autonomous fraud accusation"),
            ("docs/product/fwa-risk-operations-prd.md", "customer-approved deterministic"),
            ("docs/project/ml-algorithm-strategy.md", "they cannot be the sole denial authority"),
            ("apps/api-server/tests/knowledge_agent.rs", "assistive_only"),
        ],
        "customer_data_required": False,
    },
    {
        "capability": "inbound_claim_inbox_and_canonical_trace",
        "status": "implemented",
        "summary": "Inbox normalization, correction templates, idempotency, canonical trace, and scoring handoff exist.",
        "evidence": [
            "apps/api-server/src/routes/inbox.rs",
            "apps/api-server/src/routes/claims.rs",
            "scripts/demo/tpa_mock_client.py",
        ],
        "required_text": [
            ("apps/api-server/src/routes/inbox.rs", "INBOX_IDEMPOTENCY_CONFLICT"),
            ("apps/api-server/src/routes/claims.rs", "inbox_run_id"),
            ("scripts/demo/tpa_mock_client.py", "correction_overlay_template"),
        ],
        "customer_data_required": False,
    },
    {
        "capability": "core_scoring_rules_and_review_modes",
        "status": "implemented",
        "summary": "Deterministic scoring, standard FWA rule pack, review modes, promotion controls, and the adjudication action-class design are implemented for demo and pilot contracts.",
        "evidence": [
            "crates/fwa-scoring/src/lib.rs",
            "crates/fwa-rules/src/lib.rs",
            "apps/api-server/src/routes/ops_rules.rs",
            "docs/product/fwa-risk-operations-prd.md",
        ],
        "required_text": [
            ("scripts/demo/smoke_demo.py", "assert_standard_rule_pack"),
            ("scripts/demo/seed_demo.sql", "rule_medically_unnecessary_service"),
            ("apps/api-server/src/routes/ops_rules.rs", "promotion"),
            ("docs/product/fwa-risk-operations-prd.md", "Rule Action Classes"),
        ],
        "customer_data_required": False,
    },
    {
        "capability": "lead_case_qa_medical_feedback_loop",
        "status": "implemented",
        "summary": "Lead triage, case status, investigation writeback, QA feedback, medical review, labels, and claim audit timeline are present.",
        "evidence": [
            "apps/api-server/src/routes/ops_cases.rs",
            "apps/api-server/src/routes/ops_medical.rs",
            "apps/api-server/src/routes/pilot_loop.rs",
        ],
        "required_text": [
            ("apps/api-server/src/app/app_routes.rs", "/api/v1/investigations/results"),
            ("apps/api-server/src/app/app_routes.rs", "/api/v1/qa/results"),
            ("apps/api-server/src/app/app_routes.rs", "/api/v1/ops/medical-review/results"),
        ],
        "customer_data_required": False,
    },
    {
        "capability": "model_operations_and_mlops_pipeline",
        "status": "implemented_with_customer_validation_boundary",
        "summary": "Model registry, evaluation, retraining jobs, Rust artifact serving, external training handoff, and MLOps monitoring plans exist; real labels and live shadow evidence remain external.",
        "evidence": [
            "apps/api-server/src/routes/ops_models.rs",
            "crates/fwa-ml-runtime/src/lib.rs",
            "apps/worker/src/lib.rs",
            "docs/project/ml-pipeline-runbook.md",
        ],
        "required_text": [
            ("crates/fwa-ml-runtime/src/lib.rs", "ArtifactModelScorer"),
            ("apps/worker/src/commands/mod.rs", "build-training-handoff"),
            ("apps/worker/src/commands/mod.rs", "build-mlops-monitoring-plan"),
            ("scripts/ops/run_mlops_monitoring_plan.py", "reviewer_disagreement_report.json"),
        ],
        "customer_data_required": True,
    },
    {
        "capability": "worker_data_pipeline_productionization",
        "status": "implemented_with_customer_validation_boundary",
        "summary": "Worker-owned customer-data rollups, governed artifact submit commands, API write paths, readiness gates, run-status templates, and execution evidence contracts exist for sanctions, provider profiles, graph signals, peer benchmarks, episodes, clinical references, unbundling, scoring contexts, and probability calibration.",
        "evidence": [
            "apps/worker/src/commands/mod.rs",
            "apps/worker/src/worker_data_pipeline_readiness.rs",
            "apps/worker/src/worker_data_pipeline_run_status.rs",
            "apps/worker/src/worker_data_pipeline_execution.rs",
            "apps/api-server/src/routes/ops_providers.rs",
            "apps/api-server/src/routes/ops_datasets.rs",
            "apps/api-server/src/routes/claims.rs",
            "apps/api-server/src/routes/ops_models_mlops.rs",
            "apps/api-server/src/repository/trait.rs",
        ],
        "required_text": [
            ("apps/worker/src/commands/mod.rs", "build-worker-data-pipeline-plan"),
            ("apps/worker/src/commands/mod.rs", "build-worker-data-pipeline-readiness-input-template"),
            ("apps/worker/src/commands/mod.rs", "build-worker-data-pipeline-readiness-report"),
            ("apps/worker/src/commands/mod.rs", "build-worker-data-pipeline-run-status-template"),
            ("apps/worker/src/commands/mod.rs", "build-worker-data-pipeline-execution-report"),
            ("apps/worker/src/commands/mod.rs", "fetch-oig-sam-sanctions-snapshot"),
            ("apps/worker/src/ops_plans.rs", "oig_sam_sanctions_snapshot_fetch"),
            ("apps/worker/src/commands/mod.rs", "submit-sanctions-sync-report"),
            ("apps/worker/src/commands/mod.rs", "submit-provider-profile-window-rollup"),
            ("apps/worker/src/commands/mod.rs", "submit-provider-graph-signal-rollup"),
            ("apps/worker/src/commands/mod.rs", "submit-peer-benchmark"),
            ("apps/worker/src/commands/mod.rs", "submit-episode-aggregation"),
            ("apps/worker/src/commands/mod.rs", "submit-clinical-compatibility-reference"),
            ("apps/worker/src/commands/mod.rs", "submit-unbundling-comparator"),
            ("apps/worker/src/commands/mod.rs", "submit-scoring-feature-contexts"),
            ("apps/worker/src/commands/mod.rs", "submit-probability-calibration-report"),
            ("apps/worker/src/worker_data_pipeline_execution.rs", "readiness_gate_status"),
            ("apps/worker/src/worker_data_pipeline_execution.rs", "blocked_dependencies"),
            ("apps/api-server/src/routes/ops_datasets/validation.rs", "dependency_not_completed"),
            ("apps/api-server/src/routes/ops_datasets/validation.rs", "INVALID_WORKER_DATA_PIPELINE_READINESS_BLOCKERS"),
            ("apps/api-server/src/routes/ops_datasets/validation.rs", "INVALID_WORKER_DATA_PIPELINE_READINESS_JOB_STATUS"),
            ("apps/api-server/src/routes/openapi_schemas_data_models_datasets.rs", "blocked_dependencies"),
            ("apps/api-server/src/routes/openapi_schemas_data_models_datasets.rs", "readiness_status"),
            ("apps/worker/src/worker_data_pipeline_readiness.rs", "build_worker_data_pipeline_readiness_input_template"),
            ("apps/worker/src/worker_data_pipeline_run_status.rs", "readiness_report_uri"),
            ("apps/worker/src/worker_data_pipeline_run_status.rs", "build_command"),
            ("apps/worker/src/ops_plans.rs", "required_permission"),
            ("apps/worker/src/worker_data_pipeline_readiness.rs", "required_permission"),
            ("apps/worker/src/worker_data_pipeline_run_status.rs", "required_permission"),
            ("apps/worker/src/worker_data_pipeline_execution.rs", "required_permission"),
            (
                "scripts/ops/build_production_readiness_contract.py",
                "worker_data_pipeline_execution",
            ),
            (
                "scripts/ops/build_production_readiness_contract.py",
                "required_job_kinds_completed",
            ),
            (
                "scripts/ops/build_production_readiness_contract.py",
                "governed_submit_jobs_submitted",
            ),
            (
                "scripts/ops/build_production_readiness_contract.py",
                "scheduler_reported_jobs_succeeded_without_dependency_blockers",
            ),
            (
                "scripts/ops/validate_production_readiness_contract.py",
                "WORKER_DATA_PIPELINE_SUBMIT_JOB_API_PATHS",
            ),
            (
                "scripts/ops/validate_production_readiness_contract.py",
                "WORKER_DATA_PIPELINE_ACCEPTANCE_CHECK_IDS",
            ),
            (
                "scripts/ops/validate_production_readiness_contract.py",
                "validate_worker_data_pipeline_execution_evidence",
            ),
            (
                "scripts/ops/validate_production_readiness_contract.py",
                "validate_evidence_dir",
            ),
            (
                "scripts/ops/build_production_readiness_contract.py",
                "destruction_requires_human_approval",
            ),
            (
                "scripts/ops/validate_production_readiness_contract.py",
                "validate_retention_legal_hold_evidence",
            ),
            (
                "scripts/ops/build_production_readiness_contract.py",
                "fallback_and_rollback_ready",
            ),
            (
                "scripts/ops/validate_production_readiness_contract.py",
                "validate_model_serving_slo_evidence",
            ),
            ("apps/api-server/src/routes/ops_datasets/validation.rs", "INVALID_WORKER_DATA_PIPELINE_EXECUTION_PERMISSION"),
            ("apps/api-server/src/routes/ops_datasets/validation.rs", "INVALID_WORKER_DATA_PIPELINE_READINESS_PERMISSION"),
            ("apps/api-server/src/routes/ops_providers.rs", "save_provider_sanctions"),
            ("apps/api-server/src/routes/ops_providers/validation.rs", "INVALID_SANCTIONS_SYNC_RECORD_COUNT"),
            ("apps/worker/src/sanctions.rs", "valid_record_count"),
            ("apps/api-server/src/routes/claims.rs", "provider_sanctions_for_provider"),
            ("apps/api-server/src/routes/ops_providers.rs", "save_provider_profile_windows"),
            ("apps/api-server/src/routes/ops_providers.rs", "save_provider_graph_signals"),
            ("apps/api-server/src/routes/ops_providers.rs", "save_peer_benchmark_groups"),
            ("apps/api-server/src/routes/ops_providers.rs", "save_episode_rollups"),
            ("apps/api-server/src/routes/ops_datasets.rs", "clinical_compatibility.reference.submitted"),
            ("apps/api-server/src/routes/ops_datasets.rs", "unbundling_comparator.candidates.submitted"),
            ("apps/api-server/src/routes/ops_datasets.rs", "scoring_feature_context.materialization.submitted"),
            ("apps/api-server/src/routes/ops_datasets.rs", "worker_data_pipeline.readiness_report.submitted"),
            ("apps/api-server/src/routes/ops_datasets.rs", "worker_data_pipeline.execution_report.submitted"),
            ("apps/api-server/src/routes/claims.rs", "resolve_scoring_feature_context"),
            ("apps/api-server/src/routes/claims.rs", "latest_scoring_feature_context_for_claim"),
            ("apps/api-server/src/routes/claims.rs", "latest_peer_benchmark_group"),
            ("apps/api-server/src/routes/claims.rs", "resolve_provider_profile_input"),
            ("apps/api-server/src/routes/claims.rs", "latest_provider_profile_windows_for_provider"),
            ("apps/api-server/src/routes/claims.rs", "resolve_provider_relationships_input"),
            ("apps/api-server/src/routes/claims.rs", "latest_provider_graph_signal_for_provider"),
            ("apps/api-server/src/routes/claims.rs", "resolve_clinical_compatibility_context"),
            (
                "apps/api-server/src/routes/claims.rs",
                "clinical_compatibility_reference_for_claim",
            ),
            ("apps/api-server/src/routes/claims.rs", "resolve_episode_utilization_context"),
            (
                "apps/api-server/src/routes/claims.rs",
                "latest_episode_rollup_for_member_provider",
            ),
            (
                "apps/api-server/src/routes/claims.rs",
                "latest_unbundling_comparator_candidates_for_member_provider",
            ),
            ("apps/api-server/src/routes/ops_models_mlops.rs", "submit_probability_calibration_report"),
            ("apps/api-server/src/routes/ops_models_gates.rs", "Probability calibration"),
            ("apps/api-server/src/repository/trait.rs", "save_scoring_feature_context_materialization"),
            ("apps/api-server/src/repository/trait.rs", "latest_scoring_feature_context_for_claim"),
            ("apps/api-server/src/repository/trait.rs", "latest_peer_benchmark_group"),
            ("apps/api-server/src/repository/trait.rs", "provider_sanctions_for_provider"),
            ("apps/api-server/src/repository/trait.rs", "latest_provider_profile_windows_for_provider"),
            ("apps/api-server/src/repository/trait.rs", "latest_provider_graph_signal_for_provider"),
            ("apps/api-server/src/repository/trait.rs", "save_clinical_compatibility_references"),
            (
                "apps/api-server/src/repository/trait.rs",
                "clinical_compatibility_reference_for_claim",
            ),
            (
                "apps/api-server/src/repository/trait.rs",
                "latest_episode_rollup_for_member_provider",
            ),
            (
                "apps/api-server/src/repository/trait.rs",
                "latest_unbundling_comparator_candidates_for_member_provider",
            ),
            ("apps/api-server/src/repository/trait.rs", "latest_probability_calibration_report"),
            ("apps/api-server/src/repository/trait.rs", "save_unbundling_comparator_candidates"),
            ("apps/api-server/src/repository/trait.rs", "save_probability_calibration_report"),
            ("apps/api-server/src/repository/trait.rs", "save_worker_data_pipeline_readiness_report"),
            ("apps/api-server/src/repository/trait.rs", "save_worker_data_pipeline_execution_report"),
            ("apps/worker/src/sanctions.rs", "fetch_oig_sam_sanctions_snapshot"),
        ],
        "customer_data_required": True,
    },
    {
        "capability": "dataset_feature_and_label_governance",
        "status": "implemented_with_customer_validation_boundary",
        "summary": "Dataset, feature set, evaluation lineage, public-data MVP, and label governance gates are represented; real customer labels are still required for production claims.",
        "evidence": [
            "apps/api-server/src/routes/ops_datasets.rs",
            "scripts/data/build_public_data_mvp.py",
            "docs/project/public-data-mvp.md",
        ],
        "required_text": [
            ("apps/api-server/src/app/app_routes.rs", "/api/v1/ops/datasets"),
            ("apps/api-server/src/app/app_routes.rs", "/api/v1/ops/feature-sets"),
            ("scripts/data/build_public_data_mvp.py", "customer_production_model_evidence"),
        ],
        "customer_data_required": True,
    },
    {
        "capability": "knowledge_agent_and_ai_evidence_foundation",
        "status": "staging_proof",
        "summary": "Knowledge search, assistive investigation, document/chunk/OCR/embedding metadata, retrieval audit, and AI evidence execution-plan proof exist.",
        "evidence": [
            "apps/api-server/src/routes/knowledge.rs",
            "apps/api-server/src/routes/ops_evidence/mod.rs",
            "scripts/ops/build_ai_evidence_foundation.py",
            "docs/project/ai-evidence-foundation.md",
        ],
        "required_text": [
            ("apps/api-server/src/app/app_routes.rs", "/api/v1/ops/evidence/documents"),
            ("apps/worker/src/commands/mod.rs", "build-ai-evidence-execution-plan"),
            ("scripts/ops/validate_ai_evidence_foundation.py", "evidence_documents"),
        ],
        "customer_data_required": False,
    },
    {
        "capability": "analytics_scale",
        "status": "staging_proof",
        "summary": "ClickHouse schema, dashboard queries, export plans, provider graph snapshots, and scheduled analytics proof exist.",
        "evidence": [
            "analytics/clickhouse/schema.sql",
            "analytics/clickhouse/dashboard_queries.sql",
            "scripts/ops/build_analytics_export.py",
            "docs/project/analytics-scale.md",
        ],
        "required_text": [
            ("analytics/clickhouse/schema.sql", "analytics_provider_graph_snapshots"),
            ("analytics/clickhouse/dashboard_queries.sql", "false_positive"),
            ("scripts/ops/build_analytics_export.py", "reviewer_capacity"),
        ],
        "customer_data_required": False,
    },
    {
        "capability": "pilot_foundation_and_staging_deployment",
        "status": "staging_proof",
        "summary": "Kubernetes staging manifests, container packaging, GitHub Environment package workflow, object storage, backup, retention, legal hold, and observability proof exist.",
        "evidence": [
            "infra/k8s/staging/kustomization.yaml",
            ".github/workflows/deploy-staging.yml",
            "scripts/ops/build_staging_deployment_package.py",
            "scripts/ops/validate_staging_deployment_package.py",
            "scripts/ops/build_staging_evidence.py",
        ],
        "required_text": [
            (".github/workflows/deploy-staging.yml", "environment:"),
            (".github/workflows/deploy-staging.yml", "validate_staging_deployment_package.py"),
            ("infra/k8s/staging/worker-cronjobs.yaml", "analytics-export-plan"),
            ("scripts/ops/validate_staging_deployment_package.py", "checksum mismatch"),
            ("scripts/ops/build_staging_evidence.py", "retention_legal_hold_proof"),
        ],
        "customer_data_required": False,
    },
    {
        "capability": "web_console_operations_studio",
        "status": "implemented",
        "summary": "Yew/Trunk web console covers operational modules and demo smoke checks visible text.",
        "evidence": [
            "apps/web-console/src/main.rs",
            "apps/web-console/src/styles.css",
            "scripts/demo/smoke_web_console.mjs",
        ],
        "required_text": [
            ("apps/web-console/src/routing.rs", "Leads & Cases"),
            ("apps/web-console/src/routing.rs", "Medical Review"),
            ("scripts/demo/smoke_web_console.mjs", "Review Workbench"),
        ],
        "customer_data_required": False,
    },
]


def require_file(path: str) -> None:
    absolute = ROOT / path
    if not absolute.is_file():
        raise FileNotFoundError(f"missing evidence file: {path}")


def require_text(path: str, needle: str) -> None:
    absolute = ROOT / path
    require_file(path)
    if needle not in absolute.read_text(encoding="utf-8"):
        raise ValueError(f"missing required text in {path}: {needle}")


def build_report() -> dict:
    generated_at = datetime.now(timezone.utc).isoformat()
    capabilities = []
    for capability in CAPABILITIES:
        for path in capability["evidence"]:
            require_file(path)
        for path, needle in capability["required_text"]:
            require_text(path, needle)
        capabilities.append(
            {
                key: capability[key]
                for key in (
                    "capability",
                    "status",
                    "summary",
                    "evidence",
                    "customer_data_required",
                )
            }
        )

    total = len(capabilities)
    customer_data_required = sum(1 for item in capabilities if item["customer_data_required"])
    repository_proved = total - customer_data_required
    return {
        "artifact_kind": "prd_coverage_summary",
        "generated_at": generated_at,
        "prd_ref": "docs/product/fwa-risk-operations-prd.md",
        "capability_count": total,
        "repository_proved_without_customer_data": repository_proved,
        "customer_data_or_environment_required": customer_data_required,
        "repository_proved_percent_excluding_customer_data": 100,
        "capabilities": capabilities,
        "remaining_boundary": [
            "real customer labels and label provenance",
            "customer holdout validation and live shadow traffic",
            "live customer worker data-pipeline scheduler execution and customer-approved source claim history/reference data",
            "customer-approved production deployment, secrets, retention, observability, OCR/vector workers, and analytics execution",
            "customer-executed live restore, rollback, alert, and operational drills beyond the staging drill contract",
        ],
    }


def write_json(path: Path, payload: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-dir", default=str(DEFAULT_OUTPUT_DIR))
    args = parser.parse_args()

    output_dir = Path(args.output_dir)
    report = build_report()
    write_json(output_dir / "prd_coverage_summary.json", report)
    write_json(
        output_dir / "index.json",
        {
            "artifact_kind": "prd_coverage_index",
            "generated_at": report["generated_at"],
            "artifacts": ["prd_coverage_summary.json"],
            "customer_data_required": False,
        },
    )
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

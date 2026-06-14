use super::required_non_empty;

pub fn build_analytics_export_plan(
    object_storage_uri: &str,
    clickhouse_url: &str,
    customer_scope_id: &str,
    cron: &str,
) -> anyhow::Result<serde_json::Value> {
    let object_storage_uri =
        required_non_empty("object_storage_uri", object_storage_uri)?.trim_end_matches('/');
    let clickhouse_url = required_non_empty("clickhouse_url", clickhouse_url)?;
    let customer_scope_id = required_non_empty("customer_scope_id", customer_scope_id)?;
    let cron = required_non_empty("cron", cron)?;
    let export_root = format!("{object_storage_uri}/analytics-exports/{customer_scope_id}");

    Ok(serde_json::json!({
        "plan_kind": "scheduled_analytics_export",
        "plan_version": 1,
        "customer_scope_id": customer_scope_id,
        "data_contract": {
            "source_of_truth": "postgresql_operational_tables",
            "derived_store": "clickhouse",
            "clickhouse_url": clickhouse_url,
            "schema_ref": "analytics/clickhouse/schema.sql",
            "dashboard_queries_ref": "analytics/clickhouse/dashboard_queries.sql",
            "pii_policy": "masked_ids_and_evidence_refs_only"
        },
        "schedule": {
            "cron": cron,
            "concurrency_policy": "forbid",
            "idempotency_key": "customer_scope_id + export_window_start + sink_table"
        },
        "export_root_uri": export_root,
        "jobs": [
            {
                "job_kind": "scoring_events_export",
                "source_tables": ["scoring_runs", "claims"],
                "sink_table": "fwa_analytics.analytics_scoring_events",
                "output_uri": format!("{export_root}/scoring_events/{{window_start}}.ndjson")
            },
            {
                "job_kind": "rule_events_export",
                "source_tables": ["rule_runs", "scoring_runs", "qa_reviews"],
                "sink_table": "fwa_analytics.analytics_rule_events",
                "output_uri": format!("{export_root}/rule_events/{{window_start}}.ndjson")
            },
            {
                "job_kind": "model_events_export",
                "source_tables": ["model_scores", "model_evaluation_runs", "scoring_runs"],
                "sink_table": "fwa_analytics.analytics_model_events",
                "output_uri": format!("{export_root}/model_events/{{window_start}}.ndjson")
            },
            {
                "job_kind": "case_sla_events_export",
                "source_tables": ["investigation_cases", "audit_events"],
                "sink_table": "fwa_analytics.analytics_case_sla_events",
                "output_uri": format!("{export_root}/case_sla_events/{{window_start}}.ndjson")
            },
            {
                "job_kind": "value_events_export",
                "source_tables": ["saving_attributions", "investigation_cases", "qa_reviews"],
                "sink_table": "fwa_analytics.analytics_value_events",
                "output_uri": format!("{export_root}/value_events/{{window_start}}.ndjson")
            },
            {
                "job_kind": "reviewer_capacity_events_export",
                "source_tables": ["investigation_cases", "qa_reviews"],
                "sink_table": "fwa_analytics.analytics_reviewer_capacity_events",
                "output_uri": format!("{export_root}/reviewer_capacity_events/{{window_start}}.ndjson")
            },
            {
                "job_kind": "provider_graph_snapshots_export",
                "source_tables": ["providers", "claims", "rule_runs"],
                "sink_table": "fwa_analytics.analytics_provider_graph_snapshots",
                "output_uri": format!("{export_root}/provider_graph_snapshots/{{window_start}}.ndjson")
            }
        ],
        "dashboard_coverage": [
            "rule_drift",
            "model_drift",
            "sla_reporting",
            "roi_reporting",
            "reviewer_capacity",
            "false_positive_cost",
            "provider_graph_snapshots"
        ]
    }))
}

pub fn build_ai_evidence_execution_plan(
    api_base_url: &str,
    object_storage_uri: &str,
    vector_store_kind: &str,
    vector_store_ref: &str,
    customer_scope_id: &str,
    cron: &str,
) -> anyhow::Result<serde_json::Value> {
    let api_base_url = required_non_empty("api_base_url", api_base_url)?.trim_end_matches('/');
    let object_storage_uri =
        required_non_empty("object_storage_uri", object_storage_uri)?.trim_end_matches('/');
    let vector_store_kind = required_non_empty("vector_store_kind", vector_store_kind)?;
    let vector_store_ref = required_non_empty("vector_store_ref", vector_store_ref)?;
    let customer_scope_id = required_non_empty("customer_scope_id", customer_scope_id)?;
    let cron = required_non_empty("cron", cron)?;
    let evidence_root = format!("{object_storage_uri}/ai-evidence/{customer_scope_id}");

    Ok(serde_json::json!({
        "plan_kind": "scheduled_ai_evidence_execution",
        "plan_version": 1,
        "customer_scope_id": customer_scope_id,
        "runtime_boundary": {
            "raw_document_text": "customer_approved_object_storage_only",
            "raw_ocr_text": "customer_approved_object_storage_only",
            "embedding_vectors": "customer_approved_vector_store_only",
            "retrieval_queries": "query_checksum_only"
        },
        "api_contract": {
            "base_url": api_base_url,
            "document_registry_path": "/api/v1/ops/evidence/documents",
            "chunk_registry_path": "/api/v1/ops/evidence/documents/{document_id}/chunks",
            "ocr_output_registry_path": "/api/v1/ops/evidence/documents/{document_id}/ocr-outputs",
            "embedding_job_registry_path": "/api/v1/ops/evidence/embedding-jobs",
            "retrieval_audit_path": "/api/v1/ops/evidence/retrieval-audit-events"
        },
        "artifact_contract": {
            "document_manifest_uri": format!("{evidence_root}/documents/{{window_start}}/document_manifest.ndjson"),
            "ocr_output_manifest_uri": format!("{evidence_root}/ocr/{{window_start}}/ocr_outputs.ndjson"),
            "chunk_manifest_uri": format!("{evidence_root}/chunks/{{window_start}}/chunks.ndjson"),
            "embedding_manifest_uri": format!("{evidence_root}/embeddings/{{window_start}}/embedding_jobs.ndjson"),
            "retrieval_eval_report_uri": format!("{evidence_root}/retrieval-eval/{{window_start}}/retrieval_eval_report.json")
        },
        "vector_store": {
            "kind": vector_store_kind,
            "ref": vector_store_ref,
            "write_policy": "customer_scope_partitioned_and_redacted"
        },
        "schedule": {
            "cron": cron,
            "concurrency_policy": "forbid",
            "idempotency_key": "customer_scope_id + source_record_ref + content_checksum + execution_window"
        },
        "jobs": [
            {
                "job_kind": "document_ingestion_metadata_sync",
                "input": "customer_approved_document_drop_or_document_management_export",
                "api_path": "/api/v1/ops/evidence/documents",
                "output_ref": "evidence_documents:<document_id>",
                "manifest_uri": format!("{evidence_root}/documents/{{window_start}}/document_manifest.ndjson")
            },
            {
                "job_kind": "ocr_output_registration",
                "input": "customer_ocr_output_uri_and_checksum",
                "api_path": "/api/v1/ops/evidence/documents/{document_id}/ocr-outputs",
                "output_ref": "evidence_ocr_outputs:<ocr_output_id>",
                "manifest_uri": format!("{evidence_root}/ocr/{{window_start}}/ocr_outputs.ndjson")
            },
            {
                "job_kind": "document_chunk_registration",
                "input": "redacted_ocr_output_or_redacted_document_text_uri",
                "api_path": "/api/v1/ops/evidence/documents/{document_id}/chunks",
                "output_ref": "evidence_chunks:<chunk_id>",
                "manifest_uri": format!("{evidence_root}/chunks/{{window_start}}/chunks.ndjson")
            },
            {
                "job_kind": "embedding_job_dispatch",
                "input": "redacted_document_chunk_refs",
                "api_path": "/api/v1/ops/evidence/embedding-jobs",
                "output_ref": "evidence_embedding_jobs:<embedding_job_id>",
                "manifest_uri": format!("{evidence_root}/embeddings/{{window_start}}/embedding_jobs.ndjson")
            },
            {
                "job_kind": "retrieval_ranking_evaluation",
                "input": "retrieval_audit_events_and_reviewer_outcomes",
                "api_path": "/api/v1/ops/evidence/retrieval-audit-events",
                "output_ref": "evidence_retrieval_eval_reports:<retrieval_eval_report_uri>",
                "report_uri": format!("{evidence_root}/retrieval-eval/{{window_start}}/retrieval_eval_report.json")
            }
        ],
        "downstream_contracts": {
            "analytics_export_plan": "build-analytics-export-plan",
            "observability_dashboard": "retrieval_audit_and_agent_workspace_artifacts"
        }
    }))
}

pub fn build_governance_ops_plan(
    object_storage_uri: &str,
    database_ref: &str,
    customer_scope_id: &str,
    retention_policy_id: &str,
    backup_restore_plan_id: &str,
    legal_hold_policy_id: &str,
    cron: &str,
) -> anyhow::Result<serde_json::Value> {
    let object_storage_uri =
        required_non_empty("object_storage_uri", object_storage_uri)?.trim_end_matches('/');
    let database_ref = required_non_empty("database_ref", database_ref)?;
    let customer_scope_id = required_non_empty("customer_scope_id", customer_scope_id)?;
    let retention_policy_id = required_non_empty("retention_policy_id", retention_policy_id)?;
    let backup_restore_plan_id =
        required_non_empty("backup_restore_plan_id", backup_restore_plan_id)?;
    let legal_hold_policy_id = required_non_empty("legal_hold_policy_id", legal_hold_policy_id)?;
    let cron = required_non_empty("cron", cron)?;
    let governance_root = format!("{object_storage_uri}/governance-ops/{customer_scope_id}");

    Ok(serde_json::json!({
        "plan_kind": "scheduled_governance_ops",
        "plan_version": 1,
        "customer_scope_id": customer_scope_id,
        "policies": {
            "retention_policy_id": retention_policy_id,
            "backup_restore_plan_id": backup_restore_plan_id,
            "legal_hold_policy_id": legal_hold_policy_id
        },
        "runtime_boundary": {
            "raw_payloads": "customer_approved_object_storage_only",
            "destructive_actions": "approval_required_plan_only",
            "customer_data": "customer_scope_partitioned"
        },
        "schedule": {
            "cron": cron,
            "concurrency_policy": "forbid",
            "idempotency_key": "customer_scope_id + policy_ids + execution_window"
        },
        "artifact_contract": {
            "backup_manifest_uri": format!("{governance_root}/backup/{{window_start}}/backup_manifest.json"),
            "restore_drill_report_uri": format!("{governance_root}/restore-drills/{{window_start}}/restore_drill_report.json"),
            "retention_scan_report_uri": format!("{governance_root}/retention/{{window_start}}/retention_scan_report.json"),
            "legal_hold_report_uri": format!("{governance_root}/legal-hold/{{window_start}}/legal_hold_report.json"),
            "destruction_review_report_uri": format!("{governance_root}/destruction-review/{{window_start}}/destruction_review_report.json")
        },
        "jobs": [
            {
                "job_kind": "backup_snapshot_manifest",
                "input": database_ref,
                "output_ref": "backup_manifests:<backup_manifest_uri>",
                "backup_uri": format!("{governance_root}/backup/{{window_start}}/postgres.dump"),
                "manifest_uri": format!("{governance_root}/backup/{{window_start}}/backup_manifest.json")
            },
            {
                "job_kind": "restore_drill_validation",
                "input": "latest_backup_manifest",
                "output_ref": "restore_drill_reports:<restore_drill_report_uri>",
                "restore_target": "staging-restore-validation",
                "report_uri": format!("{governance_root}/restore-drills/{{window_start}}/restore_drill_report.json")
            },
            {
                "job_kind": "retention_policy_scan",
                "input": ["audit_events", "api_call_records", "evidence_documents", "agent_workspace_artifacts"],
                "output_ref": "retention_scan_reports:<retention_scan_report_uri>",
                "report_uri": format!("{governance_root}/retention/{{window_start}}/retention_scan_report.json")
            },
            {
                "job_kind": "legal_hold_reconciliation",
                "input": ["investigation_cases", "audit_samples", "evidence_documents"],
                "output_ref": "legal_hold_reports:<legal_hold_report_uri>",
                "report_uri": format!("{governance_root}/legal-hold/{{window_start}}/legal_hold_report.json")
            },
            {
                "job_kind": "destruction_candidate_review",
                "input": "expired_retention_items_without_legal_hold",
                "output_ref": "destruction_review_reports:<destruction_review_report_uri>",
                "approval_gate": "human_approval_required_before_destroy",
                "report_uri": format!("{governance_root}/destruction-review/{{window_start}}/destruction_review_report.json")
            }
        ],
        "downstream_contracts": {
            "pilot_readiness": "check-pilot-readiness",
            "observability_alert": "database.backup.failed | retention.scan.failed | legal_hold.reconciliation.failed"
        }
    }))
}

pub fn build_worker_data_pipeline_plan(
    api_base_url: &str,
    object_storage_uri: &str,
    customer_scope_id: &str,
    daily_cron: &str,
    monthly_cron: &str,
) -> anyhow::Result<serde_json::Value> {
    let api_base_url = required_non_empty("api_base_url", api_base_url)?.trim_end_matches('/');
    let object_storage_uri =
        required_non_empty("object_storage_uri", object_storage_uri)?.trim_end_matches('/');
    let customer_scope_id = required_non_empty("customer_scope_id", customer_scope_id)?;
    let daily_cron = required_non_empty("daily_cron", daily_cron)?;
    let monthly_cron = required_non_empty("monthly_cron", monthly_cron)?;
    let root = format!("{object_storage_uri}/worker-data-pipelines/{customer_scope_id}");

    Ok(serde_json::json!({
        "plan_kind": "scheduled_worker_data_pipeline",
        "plan_version": 1,
        "customer_scope_id": customer_scope_id,
        "runtime_boundary": {
            "execution_contract": "schedule_contract_only",
            "external_fetch": "customer_environment_required_for_live_oig_sam_and_source_claim_history",
            "customer_data_validation": "customer_approved_claim_history_and_reference_data_required",
            "side_effect_policy": "submit commands persist governed artifacts only; no claim scoring, label assignment, claim denial, routing-policy change, model activation, or case creation"
        },
        "api_contract": {
            "base_url": api_base_url,
            "provider_sanctions_path": "/api/v1/ops/providers/sanctions-sync-reports",
            "provider_profile_path": "/api/v1/ops/providers/profile-window-rollups",
            "provider_graph_path": "/api/v1/ops/providers/graph-signal-rollups",
            "peer_benchmark_path": "/api/v1/ops/providers/peer-benchmarks",
            "episode_rollup_path": "/api/v1/ops/providers/episode-rollups",
            "clinical_compatibility_path": "/api/v1/ops/clinical-compatibility-references",
            "unbundling_comparator_path": "/api/v1/ops/unbundling-comparator-candidates",
            "scoring_context_path": "/api/v1/ops/scoring-feature-context-materializations",
            "probability_calibration_path": "/api/v1/ops/models/{model_key}/probability-calibration-reports",
            "worker_data_pipeline_readiness_path": "/api/v1/ops/worker-data-pipeline-readiness",
            "worker_data_pipeline_execution_path": "/api/v1/ops/worker-data-pipeline-executions"
        },
        "schedule": {
            "daily_cron": daily_cron,
            "monthly_cron": monthly_cron,
            "concurrency_policy": "forbid",
            "idempotency_key": "customer_scope_id + job_kind + as_of_date + source_report_uri"
        },
        "artifact_root_uri": root,
        "jobs": [
            {
                "job_kind": "oig_sam_sanctions_snapshot_fetch",
                "cadence": "daily",
                "build_command": "fetch-oig-sam-sanctions-snapshot",
                "source_input": "customer_configured_oig_sam_compatible_endpoints",
                "artifact_kind": "source_snapshot",
                "required_evidence_prefixes": ["oig_sam_snapshot:"],
                "report_uri": format!("{root}/sanctions/{{as_of_date}}/oig_sam_sanctions_snapshot.json")
            },
            {
                "job_kind": "oig_sam_sanctions_sync",
                "cadence": "daily",
                "build_command": "sync-oig-sam-sanctions",
                "submit_command": "submit-sanctions-sync-report",
                "source_input": "governed_oig_sam_sanctions_snapshot",
                "depends_on": ["oig_sam_sanctions_snapshot_fetch"],
                "required_permission": "ops:providers:write",
                "required_evidence_prefixes": ["sanctions_sync_reports:"],
                "report_uri": format!("{root}/sanctions/{{as_of_date}}/sanctions_sync_report.json"),
                "api_path": "/api/v1/ops/providers/sanctions-sync-reports"
            },
            {
                "job_kind": "provider_profile_window_rollup",
                "cadence": "daily",
                "build_command": "build-provider-profile-windows",
                "submit_command": "submit-provider-profile-window-rollup",
                "source_input": "customer_claim_history_30_90_365d",
                "required_permission": "ops:providers:write",
                "required_evidence_prefixes": [
                    "provider_profile_window_rollups:",
                    "provider_profile_claim_snapshot:"
                ],
                "report_uri": format!("{root}/provider-profile/{{as_of_date}}/provider_profile_window_rollup_report.json"),
                "api_path": "/api/v1/ops/providers/profile-window-rollups"
            },
            {
                "job_kind": "provider_graph_signal_rollup",
                "cadence": "daily",
                "build_command": "build-provider-graph-signals",
                "submit_command": "submit-provider-graph-signal-rollup",
                "source_input": "customer_claim_and_referral_history_with_service_dates",
                "required_permission": "ops:providers:write",
                "required_evidence_prefixes": [
                    "provider_graph_signal_rollups:",
                    "provider_graph_claim_snapshot:"
                ],
                "report_uri": format!("{root}/provider-graph/{{as_of_date}}/provider_graph_signal_rollup_report.json"),
                "api_path": "/api/v1/ops/providers/graph-signal-rollups"
            },
            {
                "job_kind": "peer_percentile_benchmark",
                "cadence": "monthly",
                "build_command": "build-peer-benchmarks",
                "submit_command": "submit-peer-benchmark",
                "source_input": "customer_claim_history_grouped_by_specialty_region_service_segment",
                "required_permission": "ops:providers:write",
                "required_evidence_prefixes": [
                    "peer_benchmarks:",
                    "peer_benchmark_claim_snapshot:"
                ],
                "report_uri": format!("{root}/peer-benchmark/{{benchmark_month}}/peer_percentile_benchmark.json"),
                "api_path": "/api/v1/ops/providers/peer-benchmarks"
            },
            {
                "job_kind": "episode_aggregation",
                "cadence": "daily",
                "build_command": "build-episode-aggregation",
                "submit_command": "submit-episode-aggregation",
                "source_input": "customer_member_provider_claim_history",
                "required_permission": "ops:providers:write",
                "required_evidence_prefixes": [
                    "episode_rollups:",
                    "episode_claim_snapshot:"
                ],
                "report_uri": format!("{root}/episodes/{{as_of_date}}/episode_aggregation_report.json"),
                "api_path": "/api/v1/ops/providers/episode-rollups"
            },
            {
                "job_kind": "clinical_compatibility_reference",
                "cadence": "on_reference_update",
                "build_command": "build-clinical-compatibility-reference",
                "submit_command": "submit-clinical-compatibility-reference",
                "source_input": "customer_approved_icd_cpt_or_medical_policy_reference",
                "required_permission": "ops:datasets:write",
                "required_evidence_prefixes": [
                    "clinical_compatibility_references:",
                    "clinical_compatibility_reference:",
                    "clinical_policy_authority:"
                ],
                "report_uri": format!("{root}/clinical-compatibility/{{reference_version}}/clinical_compatibility_reference_report.json"),
                "api_path": "/api/v1/ops/clinical-compatibility-references"
            },
            {
                "job_kind": "unbundling_comparator",
                "cadence": "daily",
                "build_command": "build-unbundling-comparator",
                "submit_command": "submit-unbundling-comparator",
                "source_input": "customer_approved_unbundling_rule_pack_plus_episode_procedure_codes",
                "depends_on": ["episode_aggregation"],
                "required_permission": "ops:datasets:write",
                "required_evidence_prefixes": [
                    "unbundling_comparator_candidates:",
                    "unbundling_comparator_input:"
                ],
                "report_uri": format!("{root}/unbundling/{{as_of_date}}/unbundling_comparator_report.json"),
                "api_path": "/api/v1/ops/unbundling-comparator-candidates"
            },
            {
                "job_kind": "scoring_feature_context_materialization",
                "cadence": "daily",
                "build_command": "build-scoring-feature-contexts",
                "submit_command": "submit-scoring-feature-contexts",
                "source_input": "persisted_or_approved_episode_peer_clinical_and_unbundling_artifacts",
                "depends_on": [
                    "episode_aggregation",
                    "peer_percentile_benchmark",
                    "clinical_compatibility_reference",
                    "unbundling_comparator"
                ],
                "required_permission": "ops:datasets:write",
                "required_evidence_prefixes": [
                    "scoring_feature_contexts:",
                    "scoring_feature_context_claim_snapshot:",
                    "episode_rollups:",
                    "peer_benchmarks:",
                    "clinical_compatibility:",
                    "unbundling_candidates:"
                ],
                "report_uri": format!("{root}/scoring-contexts/{{as_of_date}}/scoring_feature_context_report.json"),
                "api_path": "/api/v1/ops/scoring-feature-context-materializations"
            },
            {
                "job_kind": "scoring_online_readback",
                "cadence": "daily",
                "build_command": "build-scoring-readback-report",
                "score_response_capture_command": "fetch-scoring-readback-response",
                "source_input": "score_response_artifact_from_api_v1_claims_score_after_governed_worker_writes",
                "depends_on": ["scoring_feature_context_materialization"],
                "artifact_kind": "online_scoring_readback",
                "required_evidence_prefixes": [
                    "scoring_readback_reports:",
                    "scoring_readback_inputs:",
                    "scoring_readback_score_requests:",
                    "scoring_readback_score_responses:",
                    "scoring_feature_contexts:",
                    "provider_profile_window_rollups:",
                    "sanctions_sync_reports:",
                    "provider_graph_signal_rollups:",
                    "peer_benchmarks:",
                    "episode_rollups:",
                    "clinical_compatibility:",
                    "unbundling_candidates:"
                ],
                "report_uri": format!("{root}/scoring-readback/{{as_of_date}}/scoring_readback_report.json")
            },
            {
                "job_kind": "probability_calibration_evidence",
                "cadence": "monthly",
                "build_command": "build-probability-calibration-report",
                "submit_command": "submit-probability-calibration-report",
                "source_input": "customer_labeled_holdout_predictions",
                "required_permission": "ops:models:review",
                "required_evidence_prefixes": [
                    "probability_calibration_reports:",
                    "probability_calibration_input:",
                    "calibration_labels:"
                ],
                "report_uri": format!("{root}/probability-calibration/{{benchmark_month}}/probability_calibration_report.json"),
                "api_path": "/api/v1/ops/models/{model_key}/probability-calibration-reports"
            }
        ],
        "readiness_gates": [
            "customer_approved_source_claim_history_available",
            "customer_approved_reference_data_available",
            "api_keys_include_ops_providers_write_ops_datasets_write_and_ops_models_review",
            "artifact_publication_storage_configured",
            "dry_run_reports_reviewed_before_first_submit"
        ],
        "downstream_contracts": {
            "readiness_report": "build-worker-data-pipeline-readiness-report",
            "run_status_template": "build-worker-data-pipeline-run-status-template",
            "scheduler_execution_report": "build-worker-data-pipeline-execution-report"
        }
    }))
}

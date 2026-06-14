use super::*;

#[test]
fn builds_scheduled_analytics_export_plan() {
    let plan = build_analytics_export_plan(
        "s3://nwfwa-staging-artifacts",
        "http://clickhouse:8123",
        "staging-customer",
        "15 * * * *",
    )
    .expect("analytics export plan");

    assert_eq!(plan["plan_kind"], "scheduled_analytics_export");
    assert_eq!(plan["plan_version"], 1);
    assert_eq!(plan["customer_scope_id"], "staging-customer");
    assert_eq!(plan["data_contract"]["derived_store"], "clickhouse");
    assert_eq!(
        plan["data_contract"]["pii_policy"],
        "masked_ids_and_evidence_refs_only"
    );
    assert_eq!(plan["schedule"]["cron"], "15 * * * *");
    assert_eq!(plan["jobs"][0]["job_kind"], "scoring_events_export");
    assert_eq!(plan["jobs"][1]["job_kind"], "rule_events_export");
    assert_eq!(plan["jobs"][2]["job_kind"], "model_events_export");
    assert_eq!(plan["jobs"][3]["job_kind"], "case_sla_events_export");
    assert_eq!(plan["jobs"][4]["job_kind"], "value_events_export");
    assert_eq!(
        plan["jobs"][5]["job_kind"],
        "reviewer_capacity_events_export"
    );
    assert_eq!(
        plan["jobs"][6]["job_kind"],
        "provider_graph_snapshots_export"
    );
    assert_eq!(
        plan["jobs"][6]["sink_table"],
        "fwa_analytics.analytics_provider_graph_snapshots"
    );
    assert!(plan["dashboard_coverage"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("false_positive_cost")));
}

#[test]
fn builds_scheduled_ai_evidence_execution_plan() {
    let plan = build_ai_evidence_execution_plan(
        "http://api-server:8080",
        "s3://nwfwa-staging-artifacts",
        "pgvector",
        "postgres://evidence_vectors",
        "staging-customer",
        "*/20 * * * *",
    )
    .expect("ai evidence execution plan");

    assert_eq!(plan["plan_kind"], "scheduled_ai_evidence_execution");
    assert_eq!(plan["plan_version"], 1);
    assert_eq!(plan["customer_scope_id"], "staging-customer");
    assert_eq!(
        plan["runtime_boundary"]["raw_document_text"],
        "customer_approved_object_storage_only"
    );
    assert_eq!(plan["vector_store"]["kind"], "pgvector");
    assert_eq!(plan["schedule"]["concurrency_policy"], "forbid");
    assert_eq!(
        plan["api_contract"]["embedding_job_registry_path"],
        "/api/v1/ops/evidence/embedding-jobs"
    );
    assert_eq!(
        plan["jobs"][0]["job_kind"],
        "document_ingestion_metadata_sync"
    );
    assert_eq!(plan["jobs"][1]["job_kind"], "ocr_output_registration");
    assert_eq!(plan["jobs"][2]["job_kind"], "document_chunk_registration");
    assert_eq!(plan["jobs"][3]["job_kind"], "embedding_job_dispatch");
    assert_eq!(plan["jobs"][4]["job_kind"], "retrieval_ranking_evaluation");
    assert_eq!(
            plan["artifact_contract"]["retrieval_eval_report_uri"],
            "s3://nwfwa-staging-artifacts/ai-evidence/staging-customer/retrieval-eval/{window_start}/retrieval_eval_report.json"
        );
    assert_eq!(
        plan["downstream_contracts"]["analytics_export_plan"],
        "build-analytics-export-plan"
    );
}

#[test]
fn builds_scheduled_governance_ops_plan() {
    let plan = build_governance_ops_plan(
        "s3://nwfwa-staging-artifacts",
        "postgres://postgres:5432/fwa",
        "staging-customer",
        "staging-retention-v1",
        "staging-backup-restore-v1",
        "staging-legal-hold-v1",
        "45 1 * * *",
    )
    .expect("governance ops plan");

    assert_eq!(plan["plan_kind"], "scheduled_governance_ops");
    assert_eq!(plan["plan_version"], 1);
    assert_eq!(plan["customer_scope_id"], "staging-customer");
    assert_eq!(
        plan["policies"]["retention_policy_id"],
        "staging-retention-v1"
    );
    assert_eq!(
        plan["policies"]["backup_restore_plan_id"],
        "staging-backup-restore-v1"
    );
    assert_eq!(
        plan["policies"]["legal_hold_policy_id"],
        "staging-legal-hold-v1"
    );
    assert_eq!(
        plan["runtime_boundary"]["destructive_actions"],
        "approval_required_plan_only"
    );
    assert_eq!(plan["schedule"]["concurrency_policy"], "forbid");
    assert_eq!(plan["jobs"][0]["job_kind"], "backup_snapshot_manifest");
    assert_eq!(plan["jobs"][1]["job_kind"], "restore_drill_validation");
    assert_eq!(plan["jobs"][2]["job_kind"], "retention_policy_scan");
    assert_eq!(plan["jobs"][3]["job_kind"], "legal_hold_reconciliation");
    assert_eq!(plan["jobs"][4]["job_kind"], "destruction_candidate_review");
    assert_eq!(
        plan["jobs"][4]["approval_gate"],
        "human_approval_required_before_destroy"
    );
    assert_eq!(
            plan["artifact_contract"]["retention_scan_report_uri"],
            "s3://nwfwa-staging-artifacts/governance-ops/staging-customer/retention/{window_start}/retention_scan_report.json"
        );
}

#[test]
fn builds_scheduled_worker_data_pipeline_plan() {
    let plan = build_worker_data_pipeline_plan(
        "http://api-server:8080/",
        "s3://nwfwa-production-artifacts",
        "production-customer",
        "15 1 * * *",
        "30 2 1 * *",
    )
    .expect("worker data pipeline plan");

    assert_eq!(plan["plan_kind"], "scheduled_worker_data_pipeline");
    assert_eq!(plan["plan_version"], 1);
    assert_eq!(plan["customer_scope_id"], "production-customer");
    assert_eq!(plan["api_contract"]["base_url"], "http://api-server:8080");
    assert_eq!(plan["schedule"]["daily_cron"], "15 1 * * *");
    assert_eq!(plan["schedule"]["monthly_cron"], "30 2 1 * *");
    assert_eq!(plan["schedule"]["concurrency_policy"], "forbid");
    assert_eq!(
        plan["runtime_boundary"]["side_effect_policy"],
        "submit commands persist governed artifacts only; no claim scoring, label assignment, claim denial, routing-policy change, model activation, or case creation"
    );
    let jobs = plan["jobs"].as_array().expect("jobs");
    assert_eq!(jobs.len(), 10);
    assert_eq!(jobs[0]["job_kind"], "oig_sam_sanctions_snapshot_fetch");
    assert_eq!(jobs[0]["build_command"], "fetch-oig-sam-sanctions-snapshot");
    assert_eq!(jobs[0]["artifact_kind"], "source_snapshot");
    assert_eq!(jobs[1]["job_kind"], "oig_sam_sanctions_sync");
    assert_eq!(jobs[1]["submit_command"], "submit-sanctions-sync-report");
    assert_eq!(jobs[1]["required_permission"], "ops:providers:write");
    assert_eq!(jobs[1]["depends_on"][0], "oig_sam_sanctions_snapshot_fetch");
    assert_eq!(jobs[2]["job_kind"], "provider_profile_window_rollup");
    assert_eq!(
        jobs[2]["required_evidence_prefixes"],
        serde_json::json!([
            "provider_profile_window_rollups:",
            "provider_profile_claim_snapshot:"
        ])
    );
    assert_eq!(jobs[3]["job_kind"], "provider_graph_signal_rollup");
    assert_eq!(
        jobs[3]["required_evidence_prefixes"],
        serde_json::json!([
            "provider_graph_signal_rollups:",
            "provider_graph_claim_snapshot:"
        ])
    );
    assert_eq!(jobs[4]["job_kind"], "peer_percentile_benchmark");
    assert_eq!(jobs[4]["cadence"], "monthly");
    assert_eq!(
        jobs[4]["required_evidence_prefixes"],
        serde_json::json!(["peer_benchmarks:", "peer_benchmark_claim_snapshot:"])
    );
    assert_eq!(jobs[5]["job_kind"], "episode_aggregation");
    assert_eq!(
        jobs[5]["required_evidence_prefixes"],
        serde_json::json!(["episode_rollups:", "episode_claim_snapshot:"])
    );
    assert_eq!(jobs[6]["job_kind"], "clinical_compatibility_reference");
    assert_eq!(jobs[6]["cadence"], "on_reference_update");
    assert_eq!(jobs[6]["required_permission"], "ops:datasets:write");
    assert_eq!(
        jobs[6]["required_evidence_prefixes"],
        serde_json::json!([
            "clinical_compatibility_references:",
            "clinical_compatibility_reference:",
            "clinical_policy_authority:"
        ])
    );
    assert_eq!(jobs[7]["job_kind"], "unbundling_comparator");
    assert_eq!(jobs[7]["depends_on"][0], "episode_aggregation");
    assert_eq!(
        jobs[7]["required_evidence_prefixes"],
        serde_json::json!([
            "unbundling_comparator_candidates:",
            "unbundling_comparator_input:"
        ])
    );
    assert_eq!(
        jobs[8]["job_kind"],
        "scoring_feature_context_materialization"
    );
    assert!(jobs[8]["depends_on"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("peer_percentile_benchmark")));
    assert_eq!(
        jobs[8]["required_evidence_prefixes"],
        serde_json::json!([
            "scoring_feature_contexts:",
            "episode_rollups:",
            "peer_benchmarks:",
            "clinical_compatibility:",
            "unbundling_candidates:"
        ])
    );
    assert_eq!(jobs[9]["job_kind"], "probability_calibration_evidence");
    assert_eq!(jobs[9]["required_permission"], "ops:models:review");
    assert_eq!(
        jobs[9]["api_path"],
        "/api/v1/ops/models/{model_key}/probability-calibration-reports"
    );
    assert_eq!(
        plan["api_contract"]["unbundling_comparator_path"],
        "/api/v1/ops/unbundling-comparator-candidates"
    );
    assert_eq!(
        plan["api_contract"]["worker_data_pipeline_execution_path"],
        "/api/v1/ops/worker-data-pipeline-executions"
    );
    assert_eq!(
        plan["api_contract"]["worker_data_pipeline_readiness_path"],
        "/api/v1/ops/worker-data-pipeline-readiness"
    );
    assert!(plan["readiness_gates"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!(
            "dry_run_reports_reviewed_before_first_submit"
        )));
    assert_eq!(
        plan["downstream_contracts"]["readiness_report"],
        "build-worker-data-pipeline-readiness-report"
    );
    assert_eq!(
        plan["downstream_contracts"]["run_status_template"],
        "build-worker-data-pipeline-run-status-template"
    );
    assert_eq!(
        plan["downstream_contracts"]["scheduler_execution_report"],
        "build-worker-data-pipeline-execution-report"
    );
}

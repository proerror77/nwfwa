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

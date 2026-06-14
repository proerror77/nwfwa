use super::*;

#[test]
fn builds_worker_data_pipeline_run_status_template() {
    let root = temp_root("worker-data-pipeline-run-status-template");
    let plan_uri = root.join("worker_data_pipeline_plan.json");
    let output_dir = root.join("output");
    let plan = build_worker_data_pipeline_plan(
        "http://api-server:8080",
        "s3://nwfwa-production-artifacts",
        "production-customer",
        "15 1 * * *",
        "30 2 1 * *",
    )
    .expect("worker data pipeline plan");
    write_json(plan_uri.clone(), &plan).expect("write plan");

    let report = build_worker_data_pipeline_run_status_template(
        &plan_uri.to_string_lossy(),
        "s3://nwfwa-production-artifacts/worker-data-pipelines/production-customer/readiness/2026-06-14/worker_data_pipeline_readiness_report.json",
        "wdp_2026_06_14",
        "2026-06-14",
        &output_dir,
    )
    .expect("worker data pipeline run status template");

    assert_eq!(report["report_kind"], "worker_data_pipeline_run_status");
    assert_eq!(report["run_status_template"], true);
    assert_eq!(report["customer_scope_id"], "production-customer");
    assert_eq!(report["run_id"], "wdp_2026_06_14");
    assert_eq!(report["execution_date"], "2026-06-14");
    assert_eq!(report["job_count"], 11);
    assert_eq!(
        report["readiness_report_uri"],
        "s3://nwfwa-production-artifacts/worker-data-pipelines/production-customer/readiness/2026-06-14/worker_data_pipeline_readiness_report.json"
    );
    let job_statuses = report["job_statuses"].as_array().expect("job statuses");
    assert_eq!(
        job_statuses[0]["job_kind"],
        "oig_sam_sanctions_snapshot_fetch"
    );
    assert_eq!(
        job_statuses[0]["build_command"],
        "fetch-oig-sam-sanctions-snapshot"
    );
    assert_eq!(
        job_statuses[0]["source_input"],
        "customer_configured_oig_sam_compatible_endpoints"
    );
    assert_eq!(job_statuses[0]["artifact_kind"], "source_snapshot");
    assert_eq!(job_statuses[0]["depends_on"], serde_json::json!([]));
    assert_eq!(
        job_statuses[0]["planned_report_uri"],
        "s3://nwfwa-production-artifacts/worker-data-pipelines/production-customer/sanctions/{as_of_date}/oig_sam_sanctions_snapshot.json"
    );
    assert_eq!(job_statuses[0]["submit_command"], serde_json::Value::Null);
    assert_eq!(job_statuses[0]["api_path"], serde_json::Value::Null);
    assert_eq!(
        job_statuses[0]["required_permission"],
        serde_json::Value::Null
    );
    assert_eq!(
        job_statuses[0]["status"],
        "scheduled_pending_customer_execution"
    );
    assert_eq!(job_statuses[0]["artifact_uri"], serde_json::Value::Null);
    assert_eq!(job_statuses[0]["submitted"], false);
    assert_eq!(job_statuses[1]["job_kind"], "oig_sam_sanctions_sync");
    assert_eq!(job_statuses[1]["build_command"], "sync-oig-sam-sanctions");
    assert_eq!(
        job_statuses[1]["required_permission"],
        "ops:providers:write"
    );
    assert_eq!(
        job_statuses[1]["depends_on"],
        serde_json::json!(["oig_sam_sanctions_snapshot_fetch"])
    );
    assert_eq!(
        job_statuses[2]["required_evidence_prefixes"],
        serde_json::json!([
            "provider_profile_window_rollups:",
            "provider_profile_claim_snapshot:"
        ])
    );
    assert_eq!(
        job_statuses[7]["required_evidence_prefixes"],
        serde_json::json!([
            "unbundling_comparator_candidates:",
            "unbundling_comparator_input:"
        ])
    );
    assert_eq!(
        job_statuses[8]["required_evidence_prefixes"],
        serde_json::json!([
            "scoring_feature_contexts:",
            "scoring_feature_context_claim_snapshot:",
            "episode_rollups:",
            "peer_benchmarks:",
            "clinical_compatibility:",
            "unbundling_candidates:"
        ])
    );
    assert_eq!(job_statuses[9]["job_kind"], "scoring_online_readback");
    assert_eq!(
        job_statuses[9]["required_evidence_prefixes"],
        serde_json::json!([
            "scoring_readback_reports:",
            "scoring_readback_inputs:",
            "scoring_readback_score_requests:",
            "scoring_readback_score_responses:"
        ])
    );
    assert!(report["evidence_refs"]
        .as_array()
        .expect("evidence refs")
        .iter()
        .any(|reference| reference
            == "worker_data_pipeline_readiness_reports:s3://nwfwa-production-artifacts/worker-data-pipelines/production-customer/readiness/2026-06-14/worker_data_pipeline_readiness_report.json"));
    assert!(output_dir
        .join("worker_data_pipeline_run_status_template.json")
        .exists());
}

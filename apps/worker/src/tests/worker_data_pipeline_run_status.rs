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
    assert_eq!(report["job_count"], 9);
    assert_eq!(
        report["readiness_report_uri"],
        "s3://nwfwa-production-artifacts/worker-data-pipelines/production-customer/readiness/2026-06-14/worker_data_pipeline_readiness_report.json"
    );
    let job_statuses = report["job_statuses"].as_array().expect("job statuses");
    assert_eq!(job_statuses[0]["job_kind"], "oig_sam_sanctions_sync");
    assert_eq!(
        job_statuses[0]["status"],
        "scheduled_pending_customer_execution"
    );
    assert_eq!(job_statuses[0]["artifact_uri"], serde_json::Value::Null);
    assert_eq!(job_statuses[0]["submitted"], false);
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

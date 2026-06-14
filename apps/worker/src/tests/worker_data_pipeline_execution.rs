use super::*;

#[test]
fn builds_worker_data_pipeline_execution_report() {
    let root = temp_root("worker-data-pipeline-execution");
    let plan_uri = root.join("worker_data_pipeline_plan.json");
    let run_status_uri = root.join("worker_data_pipeline_run_status.json");
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
    write_json(
        run_status_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_run_status",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_statuses": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "status": "succeeded",
                    "artifact_uri": "s3://nwfwa-production-artifacts/worker-data-pipelines/production-customer/sanctions/2026-06-14/sanctions_sync_report.json",
                    "submitted": true
                },
                {
                    "job_kind": "provider_profile_window_rollup",
                    "status": "succeeded",
                    "artifact_uri": "s3://nwfwa-production-artifacts/worker-data-pipelines/production-customer/provider-profile/2026-06-14/provider_profile_window_rollup_report.json",
                    "submitted": false
                },
                {
                    "job_kind": "provider_graph_signal_rollup",
                    "status": "failed",
                    "artifact_uri": null,
                    "submitted": false
                }
            ]
        }),
    )
    .expect("write run status");

    let report = build_worker_data_pipeline_execution_report(
        &plan_uri.to_string_lossy(),
        &run_status_uri.to_string_lossy(),
        &output_dir,
    )
    .expect("worker data pipeline execution report");

    assert_eq!(
        report["report_kind"],
        "worker_data_pipeline_execution_report"
    );
    assert_eq!(report["customer_scope_id"], "production-customer");
    assert_eq!(report["run_id"], "wdp_2026_06_14");
    assert_eq!(report["job_count"], 9);
    assert_eq!(
        report["scheduler_status"],
        "completed_with_pending_or_failed_jobs"
    );
    let executions = report["job_executions"].as_array().expect("executions");
    assert_eq!(executions[0]["execution_status"], "completed");
    assert_eq!(
        executions[1]["execution_status"],
        "artifact_pending_submission"
    );
    assert_eq!(executions[2]["execution_status"], "failed");
    assert_eq!(
        executions[3]["execution_status"],
        "scheduled_pending_customer_execution"
    );
    assert_eq!(report["review_task_count"], 8);
    assert_eq!(
        report["governance_boundary"],
        "worker data pipeline execution evidence may open operations review tasks only; it must not score claims, assign labels, deny claims, activate models, or change routing policy"
    );
    assert!(output_dir
        .join("worker_data_pipeline_execution_report.json")
        .exists());
    assert!(output_dir
        .join("worker_data_pipeline_execution_review_tasks.json")
        .exists());
}

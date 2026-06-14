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

#[test]
fn builds_worker_data_pipeline_execution_submission() {
    let root = temp_root("worker-data-pipeline-execution-submission");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "local://plans/worker_data_pipeline_plan.json",
            "run_status_uri": "local://runs/worker_data_pipeline_run_status.json",
            "customer_scope_id": "production-customer",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "completed",
                    "submitted": true
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only; it must not score claims, assign labels, deny claims, activate models, or change routing policy",
            "evidence_refs": [
                "worker_data_pipeline_plans:local://plans/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:local://runs/worker_data_pipeline_run_status.json"
            ]
        }),
    )
    .expect("write report");

    let submission = build_worker_data_pipeline_execution_submission(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
    )
    .expect("worker data pipeline submission");

    assert_eq!(
        submission.report_kind,
        "worker_data_pipeline_execution_report"
    );
    assert_eq!(submission.run_id, "wdp_2026_06_14");
    assert_eq!(submission.job_count, 1);
    assert_eq!(submission.review_task_count, 0);
    assert!(submission.evidence_refs.iter().any(|reference| {
        reference
            == &format!(
                "worker_data_pipeline_execution_reports:{}",
                report_uri.to_string_lossy()
            )
    }));
}

#[tokio::test]
async fn submits_worker_data_pipeline_execution_report_to_api() {
    let root = temp_root("worker-data-pipeline-execution-submit-api");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "local://plans/worker_data_pipeline_plan.json",
            "run_status_uri": "local://runs/worker_data_pipeline_run_status.json",
            "customer_scope_id": "production-customer",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "completed",
                    "submitted": true
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only; it must not score claims, assign labels, deny claims, activate models, or change routing policy",
            "evidence_refs": [
                "worker_data_pipeline_plans:local://plans/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:local://runs/worker_data_pipeline_run_status.json"
            ]
        }),
    )
    .expect("write report");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_http_request(&mut socket).await;
        assert!(request.contains("POST /api/v1/ops/worker-data-pipeline-executions HTTP/1.1"));
        assert!(request.contains(r#""report_kind":"worker_data_pipeline_execution_report""#));
        assert!(request.contains("worker_data_pipeline_execution_reports:"));
        write_json_response(
            &mut socket,
            serde_json::json!({
                "report_kind": "worker_data_pipeline_execution_report",
                "run_id": "wdp_2026_06_14",
                "claim_scoring": false
            }),
        )
        .await;
    });

    let response = submit_worker_data_pipeline_execution_report(
        &format!("http://{addr}"),
        "test-api-key",
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
    )
    .await
    .expect("submit worker data pipeline execution report");
    server.await.unwrap();

    assert_eq!(response["run_id"], "wdp_2026_06_14");
    assert_eq!(response["claim_scoring"], false);
}

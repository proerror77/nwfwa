use super::*;

#[test]
fn blocks_worker_data_pipeline_when_customer_inputs_are_not_ready() {
    let root = temp_root("worker-data-pipeline-readiness-blocked");
    let plan_uri = root.join("worker_data_pipeline_plan.json");
    let readiness_uri = root.join("worker_data_pipeline_readiness_input.json");
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
        readiness_uri.clone(),
        &serde_json::json!({
            "checks": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "artifact_uri": "s3://nwfwa-production-artifacts/sanctions/source.json",
                    "customer_approved": true,
                    "external_fetch_configured": false,
                    "row_count": 12,
                    "minimum_row_count": 1,
                    "data_quality_status": "passed",
                    "evidence_refs": ["customer_approval:sanctions:2026-06-14"]
                },
                {
                    "job_kind": "provider_profile_window_rollup",
                    "artifact_uri": "",
                    "customer_approved": false,
                    "row_count": 20,
                    "minimum_row_count": 100,
                    "data_quality_status": "blocked",
                    "evidence_refs": []
                }
            ]
        }),
    )
    .expect("write readiness input");

    let report = build_worker_data_pipeline_readiness_report(
        &plan_uri.to_string_lossy(),
        &readiness_uri.to_string_lossy(),
        &output_dir,
    )
    .expect("readiness report");

    assert_eq!(
        report["report_kind"],
        "worker_data_pipeline_readiness_report"
    );
    assert_eq!(report["readiness_status"], "blocked");
    assert_eq!(report["job_count"], 9);
    assert_eq!(report["ready_job_count"], 0);
    assert_eq!(report["blocked_job_count"], 9);
    let jobs = report["job_readiness"].as_array().expect("jobs");
    assert_eq!(jobs[0]["job_kind"], "oig_sam_sanctions_sync");
    assert!(jobs[0]["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("external_oig_sam_fetch_not_configured")));
    assert!(jobs[1]["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("row_count_below_minimum")));
    assert!(jobs[1]["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("customer_approval_missing")));
    assert!(jobs[3]["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("missing_customer_readiness_check")));
    assert_eq!(report["review_task_count"], 9);
    assert!(output_dir
        .join("worker_data_pipeline_readiness_report.json")
        .exists());
    assert!(output_dir
        .join("worker_data_pipeline_readiness_review_tasks.json")
        .exists());
}

#[test]
fn marks_worker_data_pipeline_ready_when_all_customer_inputs_pass() {
    let root = temp_root("worker-data-pipeline-readiness-ready");
    let plan_uri = root.join("worker_data_pipeline_plan.json");
    let readiness_uri = root.join("worker_data_pipeline_readiness_input.json");
    let output_dir = root.join("output");
    let plan = build_worker_data_pipeline_plan(
        "http://api-server:8080",
        "s3://nwfwa-production-artifacts",
        "production-customer",
        "15 1 * * *",
        "30 2 1 * *",
    )
    .expect("worker data pipeline plan");
    let checks = plan["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|job| {
            let job_kind = job["job_kind"].as_str().unwrap();
            serde_json::json!({
                "job_kind": job_kind,
                "artifact_uri": format!("s3://nwfwa-production-artifacts/readiness/{job_kind}.json"),
                "customer_approved": true,
                "external_fetch_configured": job_kind == "oig_sam_sanctions_sync",
                "row_count": 100,
                "minimum_row_count": 10,
                "data_quality_status": "passed",
                "evidence_refs": [format!("customer_approval:{job_kind}:2026-06-14")]
            })
        })
        .collect::<Vec<_>>();
    write_json(plan_uri.clone(), &plan).expect("write plan");
    write_json(
        readiness_uri.clone(),
        &serde_json::json!({ "checks": checks }),
    )
    .expect("write readiness input");

    let report = build_worker_data_pipeline_readiness_report(
        &plan_uri.to_string_lossy(),
        &readiness_uri.to_string_lossy(),
        &output_dir,
    )
    .expect("readiness report");

    assert_eq!(report["readiness_status"], "ready");
    assert_eq!(report["ready_job_count"], 9);
    assert_eq!(report["blocked_job_count"], 0);
    assert_eq!(report["review_task_count"], 0);
}

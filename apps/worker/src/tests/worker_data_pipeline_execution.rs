use super::*;

fn evidence_refs_for_job(job: &serde_json::Value) -> Vec<String> {
    let job_kind = job["job_kind"].as_str().expect("job kind");
    job["required_evidence_prefixes"]
        .as_array()
        .expect("required evidence prefixes")
        .iter()
        .map(|prefix| {
            format!(
                "{}s3://nwfwa-production-artifacts/{job_kind}.json",
                prefix.as_str().expect("prefix")
            )
        })
        .chain(std::iter::once(format!(
            "worker_job_artifacts:{job_kind}:2026-06-14"
        )))
        .collect()
}

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
    let jobs = plan["jobs"].as_array().expect("jobs");
    write_json(
        run_status_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_run_status",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_statuses": [
                {
                    "job_kind": "oig_sam_sanctions_snapshot_fetch",
                    "status": "succeeded",
                    "artifact_uri": "s3://nwfwa-production-artifacts/worker-data-pipelines/production-customer/sanctions/2026-06-14/oig_sam_sanctions_snapshot.json",
                    "evidence_refs": evidence_refs_for_job(&jobs[0]),
                    "submitted": false
                },
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "status": "succeeded",
                    "artifact_uri": "s3://nwfwa-production-artifacts/worker-data-pipelines/production-customer/sanctions/2026-06-14/sanctions_sync_report.json",
                    "evidence_refs": evidence_refs_for_job(&jobs[1]),
                    "submitted": true
                },
                {
                    "job_kind": "provider_profile_window_rollup",
                    "status": "succeeded",
                    "artifact_uri": "s3://nwfwa-production-artifacts/worker-data-pipelines/production-customer/provider-profile/2026-06-14/provider_profile_window_rollup_report.json",
                    "evidence_refs": evidence_refs_for_job(&jobs[2]),
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
    assert_eq!(report["job_count"], 11);
    assert_eq!(
        report["scheduler_status"],
        "completed_with_pending_or_failed_jobs"
    );
    assert_eq!(report["readiness_gate_status"], "missing");
    let executions = report["job_executions"].as_array().expect("executions");
    assert_eq!(executions[0]["execution_status"], "completed");
    assert_eq!(executions[0]["submit_command"], serde_json::Value::Null);
    assert_eq!(
        executions[0]["required_submit_flags"],
        serde_json::json!([])
    );
    assert_eq!(
        executions[0]["required_permission"],
        serde_json::Value::Null
    );
    assert_eq!(executions[1]["execution_status"], "completed");
    assert_eq!(
        executions[1]["required_submit_flags"],
        serde_json::json!(["--published-report-uri", "--published-source-uri"])
    );
    assert_eq!(executions[1]["required_permission"], "ops:providers:write");
    assert_eq!(
        executions[1]["reported_artifact_uri"],
        "s3://nwfwa-production-artifacts/worker-data-pipelines/production-customer/sanctions/2026-06-14/sanctions_sync_report.json"
    );
    assert_eq!(
        executions[1]["evidence_refs"],
        serde_json::json!(evidence_refs_for_job(&jobs[1]))
    );
    assert_eq!(
        executions[2]["execution_status"],
        "artifact_pending_submission"
    );
    assert_eq!(executions[2]["required_permission"], "ops:providers:write");
    assert_eq!(executions[3]["execution_status"], "failed");
    assert_eq!(
        executions[4]["execution_status"],
        "scheduled_pending_customer_execution"
    );
    assert_eq!(executions[9]["job_kind"], "scoring_online_readback");
    assert_eq!(
        executions[9]["execution_status"],
        "dependency_not_completed"
    );
    assert_eq!(report["review_task_count"], 10);
    assert!(report["review_tasks"]
        .as_array()
        .expect("review tasks")
        .iter()
        .any(|task| task["job_kind"] == "provider_profile_window_rollup"
            && task["api_path"] == "/api/v1/ops/providers/profile-window-rollups"
            && task["required_submit_flags"]
                == serde_json::json!(["--published-report-uri", "--published-source-uri"])
            && task["required_permission"] == "ops:providers:write"));
    assert!(report["review_tasks"]
        .as_array()
        .expect("review tasks")
        .iter()
        .any(|task| task["task_kind"] == "worker_data_pipeline_readiness_gate_review"));
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
fn rejects_submit_job_without_required_submit_flags() {
    let root = temp_root("worker-data-pipeline-execution-missing-submit-flags");
    let plan_uri = root.join("worker_data_pipeline_plan.json");
    let run_status_uri = root.join("worker_data_pipeline_run_status.json");
    let output_dir = root.join("output");
    let mut plan = build_worker_data_pipeline_plan(
        "http://api-server:8080",
        "s3://nwfwa-production-artifacts",
        "production-customer",
        "15 1 * * *",
        "30 2 1 * *",
    )
    .expect("worker data pipeline plan");
    plan["jobs"][1]
        .as_object_mut()
        .expect("job object")
        .remove("required_submit_flags");
    write_json(plan_uri.clone(), &plan).expect("write plan");
    write_json(
        run_status_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_run_status",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_statuses": []
        }),
    )
    .expect("write run status");

    let error = build_worker_data_pipeline_execution_report(
        &plan_uri.to_string_lossy(),
        &run_status_uri.to_string_lossy(),
        &output_dir,
    )
    .expect_err("missing required submit flags should fail");

    assert!(error
        .to_string()
        .contains("submit-sanctions-sync-report requires non-empty required_submit_flags"));
}

#[test]
fn blocks_worker_data_pipeline_job_when_dependency_is_not_completed() {
    let root = temp_root("worker-data-pipeline-execution-dependency-blocked");
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
    let jobs = plan["jobs"].as_array().expect("jobs");
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
                    "evidence_refs": evidence_refs_for_job(&jobs[1]),
                    "submitted": true
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

    let executions = report["job_executions"].as_array().expect("executions");
    assert_eq!(
        executions[0]["execution_status"],
        "scheduled_pending_customer_execution"
    );
    assert_eq!(executions[1]["job_kind"], "oig_sam_sanctions_sync");
    assert_eq!(
        executions[1]["execution_status"],
        "dependency_not_completed"
    );
    assert_eq!(
        executions[1]["blocked_dependencies"],
        serde_json::json!(["oig_sam_sanctions_snapshot_fetch"])
    );
    assert_eq!(executions[1]["required_permission"], "ops:providers:write");
    assert!(report["review_tasks"]
        .as_array()
        .expect("review tasks")
        .iter()
        .any(|task| task["job_kind"] == "oig_sam_sanctions_sync"
            && task["execution_status"] == "dependency_not_completed"
            && task["api_path"] == "/api/v1/ops/providers/sanctions-sync-reports"
            && task["required_permission"] == "ops:providers:write"));
}

#[test]
fn marks_succeeded_job_without_evidence_refs_for_review() {
    let root = temp_root("worker-data-pipeline-execution-missing-job-evidence");
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
    let jobs = plan["jobs"].as_array().expect("jobs");
    write_json(
        run_status_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_run_status",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_statuses": [
                {
                    "job_kind": "oig_sam_sanctions_snapshot_fetch",
                    "status": "succeeded",
                    "artifact_uri": "s3://nwfwa-production-artifacts/worker-data-pipelines/production-customer/sanctions/2026-06-14/oig_sam_sanctions_snapshot.json",
                    "evidence_refs": evidence_refs_for_job(&jobs[0]),
                    "submitted": false
                },
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "status": "succeeded",
                    "artifact_uri": "s3://nwfwa-production-artifacts/worker-data-pipelines/production-customer/sanctions/2026-06-14/sanctions_sync_report.json",
                    "evidence_refs": [],
                    "submitted": true
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

    let executions = report["job_executions"].as_array().expect("executions");
    assert_eq!(executions[1]["job_kind"], "oig_sam_sanctions_sync");
    assert_eq!(
        executions[1]["execution_status"],
        "artifact_missing_evidence"
    );
    assert_eq!(
        report["scheduler_status"],
        "completed_with_pending_or_failed_jobs"
    );
    assert!(report["review_tasks"]
        .as_array()
        .expect("review tasks")
        .iter()
        .any(|task| task["job_kind"] == "oig_sam_sanctions_sync"
            && task["execution_status"] == "artifact_missing_evidence"));
}

#[test]
fn marks_succeeded_job_missing_required_evidence_prefix_for_review() {
    let root = temp_root("worker-data-pipeline-execution-missing-required-prefix");
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
    let jobs = plan["jobs"].as_array().expect("jobs");
    write_json(
        run_status_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_run_status",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_statuses": [
                {
                    "job_kind": "oig_sam_sanctions_snapshot_fetch",
                    "status": "succeeded",
                    "artifact_uri": "s3://nwfwa-production-artifacts/worker-data-pipelines/production-customer/sanctions/2026-06-14/oig_sam_sanctions_snapshot.json",
                    "evidence_refs": evidence_refs_for_job(&jobs[0]),
                    "submitted": false
                },
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "status": "succeeded",
                    "artifact_uri": "s3://nwfwa-production-artifacts/worker-data-pipelines/production-customer/sanctions/2026-06-14/sanctions_sync_report.json",
                    "evidence_refs": ["worker_job_artifacts:oig_sam_sanctions_sync:2026-06-14"],
                    "submitted": true
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

    let executions = report["job_executions"].as_array().expect("executions");
    assert_eq!(executions[1]["job_kind"], "oig_sam_sanctions_sync");
    assert_eq!(
        executions[1]["required_evidence_prefixes"],
        serde_json::json!(["sanctions_sync_reports:"])
    );
    assert_eq!(
        executions[1]["execution_status"],
        "artifact_missing_evidence"
    );
    assert!(report["review_tasks"]
        .as_array()
        .expect("review tasks")
        .iter()
        .any(|task| task["job_kind"] == "oig_sam_sanctions_sync"
            && task["execution_status"] == "artifact_missing_evidence"));
}

#[test]
fn marks_succeeded_job_with_template_refs_for_review() {
    let root = temp_root("worker-data-pipeline-execution-template-refs");
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
                    "job_kind": "provider_profile_window_rollup",
                    "status": "succeeded",
                    "artifact_uri": "local://template/worker/provider_profile_window_rollup.json",
                    "evidence_refs": [
                        "provider_profile_window_rollups:local://template/worker/provider_profile_window_rollup.json",
                        "provider_profile_claim_snapshot:local://template/worker/provider_profile_window_rollup.json"
                    ],
                    "submitted": true
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

    let executions = report["job_executions"].as_array().expect("executions");
    let provider_profile = executions
        .iter()
        .find(|execution| execution["job_kind"] == "provider_profile_window_rollup")
        .expect("provider profile execution");
    assert_eq!(
        provider_profile["execution_status"],
        "artifact_missing_evidence"
    );
    assert!(report["review_tasks"]
        .as_array()
        .expect("review tasks")
        .iter()
        .any(|task| task["job_kind"] == "provider_profile_window_rollup"
            && task["execution_status"] == "artifact_missing_evidence"));
}

#[test]
fn marks_succeeded_job_with_local_artifact_uri_for_review() {
    let root = temp_root("worker-data-pipeline-execution-local-artifact");
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
    let jobs = plan["jobs"].as_array().expect("jobs");
    write_json(
        run_status_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_run_status",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_statuses": [
                {
                    "job_kind": "peer_percentile_benchmark",
                    "status": "succeeded",
                    "artifact_uri": "local://artifacts/peer_percentile_benchmark.json",
                    "evidence_refs": evidence_refs_for_job(&jobs[4]),
                    "submitted": true
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

    let executions = report["job_executions"].as_array().expect("executions");
    let peer_benchmark = executions
        .iter()
        .find(|execution| execution["job_kind"] == "peer_percentile_benchmark")
        .expect("peer benchmark execution");
    assert_eq!(
        peer_benchmark["execution_status"],
        "artifact_missing_evidence"
    );
    assert!(report["review_tasks"]
        .as_array()
        .expect("review tasks")
        .iter()
        .any(|task| task["job_kind"] == "peer_percentile_benchmark"
            && task["execution_status"] == "artifact_missing_evidence"));
}

#[test]
fn marks_succeeded_job_with_local_evidence_refs_for_review() {
    let root = temp_root("worker-data-pipeline-execution-local-evidence");
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
                    "job_kind": "peer_percentile_benchmark",
                    "status": "succeeded",
                    "artifact_uri": "s3://nwfwa-production-artifacts/peer_percentile_benchmark.json",
                    "evidence_refs": [
                        "peer_benchmarks:local://artifacts/peer_percentile_benchmark.json",
                        "peer_benchmark_claim_snapshot:s3://nwfwa-production-artifacts/peer_claims.json"
                    ],
                    "submitted": true
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

    let executions = report["job_executions"].as_array().expect("executions");
    let peer_benchmark = executions
        .iter()
        .find(|execution| execution["job_kind"] == "peer_percentile_benchmark")
        .expect("peer benchmark execution");
    assert_eq!(
        peer_benchmark["execution_status"],
        "artifact_missing_evidence"
    );
}

#[test]
fn builds_worker_data_pipeline_execution_report_with_ready_gate() {
    let root = temp_root("worker-data-pipeline-execution-ready");
    let plan_uri = root.join("worker_data_pipeline_plan.json");
    let readiness_report_uri = root.join("worker_data_pipeline_readiness_report.json");
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
        readiness_report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_readiness_report",
            "readiness_status": "ready"
        }),
    )
    .expect("write readiness report");
    let job_statuses = plan["jobs"]
        .as_array()
        .expect("jobs")
        .iter()
        .map(|job| {
            let job_kind = job["job_kind"].as_str().expect("job kind");
            serde_json::json!({
                "job_kind": job_kind,
                "status": "succeeded",
                "artifact_uri": format!("s3://nwfwa-production-artifacts/{job_kind}.json"),
                "evidence_refs": evidence_refs_for_job(job),
                "submitted": true
            })
        })
        .collect::<Vec<_>>();
    write_json(
        run_status_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_run_status",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "readiness_report_uri": readiness_report_uri.to_string_lossy(),
            "job_statuses": job_statuses
        }),
    )
    .expect("write run status");

    let report = build_worker_data_pipeline_execution_report(
        &plan_uri.to_string_lossy(),
        &run_status_uri.to_string_lossy(),
        &output_dir,
    )
    .expect("worker data pipeline execution report");

    assert_eq!(report["readiness_gate_status"], "ready");
    assert_eq!(
        report["readiness_report_uri"],
        readiness_report_uri.to_string_lossy().to_string()
    );
    assert_eq!(report["scheduler_status"], "completed");
    assert_eq!(report["pending_or_failed_job_count"], 0);
    assert_eq!(report["review_task_count"], 0);
    assert!(report["evidence_refs"]
        .as_array()
        .expect("evidence refs")
        .iter()
        .any(|reference| reference
            == &serde_json::json!(format!(
                "worker_data_pipeline_readiness_reports:{}",
                readiness_report_uri.to_string_lossy()
            ))));
}

#[test]
fn builds_worker_data_pipeline_execution_report_with_published_lineage() {
    let root = temp_root("worker-data-pipeline-execution-published-lineage");
    let plan_uri = root.join("worker_data_pipeline_plan.json");
    let readiness_report_uri = root.join("worker_data_pipeline_readiness_report.json");
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
        readiness_report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_readiness_report",
            "readiness_status": "ready"
        }),
    )
    .expect("write readiness report");
    let job_statuses = plan["jobs"]
        .as_array()
        .expect("jobs")
        .iter()
        .map(|job| {
            let job_kind = job["job_kind"].as_str().expect("job kind");
            serde_json::json!({
                "job_kind": job_kind,
                "status": "succeeded",
                "artifact_uri": format!("s3://nwfwa-production-artifacts/{job_kind}.json"),
                "evidence_refs": evidence_refs_for_job(job),
                "submitted": true
            })
        })
        .collect::<Vec<_>>();
    write_json(
        run_status_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_run_status",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "readiness_report_uri": readiness_report_uri.to_string_lossy(),
            "job_statuses": job_statuses
        }),
    )
    .expect("write run status");

    let published_plan_uri =
        "s3://customer-prod-artifacts/worker/plan/worker_data_pipeline_plan.json";
    let published_run_status_uri =
        "s3://customer-prod-artifacts/worker/run-status/worker_data_pipeline_run_status.json";
    let published_readiness_report_uri =
        "s3://customer-prod-artifacts/worker/readiness/worker_data_pipeline_readiness_report.json";
    let report = build_worker_data_pipeline_execution_report_with_published_uris(
        &plan_uri.to_string_lossy(),
        &run_status_uri.to_string_lossy(),
        &output_dir,
        Some(published_plan_uri),
        Some(published_run_status_uri),
        Some(published_readiness_report_uri),
    )
    .expect("worker data pipeline execution report");

    assert_eq!(report["plan_uri"], published_plan_uri);
    assert_eq!(report["run_status_uri"], published_run_status_uri);
    assert_eq!(
        report["readiness_report_uri"],
        published_readiness_report_uri
    );
    let evidence_refs = report["evidence_refs"].as_array().expect("evidence refs");
    assert!(evidence_refs.iter().all(|reference| !reference
        .as_str()
        .unwrap_or_default()
        .contains(&*root.to_string_lossy())));
    assert!(evidence_refs.iter().any(|reference| {
        reference == &serde_json::json!(format!("worker_data_pipeline_plans:{published_plan_uri}"))
    }));
    assert!(evidence_refs.iter().any(|reference| {
        reference
            == &serde_json::json!(format!(
                "worker_data_pipeline_run_status:{published_run_status_uri}"
            ))
    }));
    assert!(evidence_refs.iter().any(|reference| {
        reference
            == &serde_json::json!(format!(
                "worker_data_pipeline_readiness_reports:{published_readiness_report_uri}"
            ))
    }));
}

#[test]
fn builds_worker_data_pipeline_execution_report_with_blocked_gate() {
    let root = temp_root("worker-data-pipeline-execution-blocked");
    let plan_uri = root.join("worker_data_pipeline_plan.json");
    let readiness_report_uri = root.join("worker_data_pipeline_readiness_report.json");
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
        readiness_report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_readiness_report",
            "readiness_status": "blocked"
        }),
    )
    .expect("write readiness report");
    let job_statuses = plan["jobs"]
        .as_array()
        .expect("jobs")
        .iter()
        .map(|job| {
            let job_kind = job["job_kind"].as_str().expect("job kind");
            serde_json::json!({
                "job_kind": job_kind,
                "status": "succeeded",
                "artifact_uri": format!("s3://nwfwa-production-artifacts/{job_kind}.json"),
                "evidence_refs": evidence_refs_for_job(job),
                "submitted": true
            })
        })
        .collect::<Vec<_>>();
    write_json(
        run_status_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_run_status",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "readiness_report_uri": readiness_report_uri.to_string_lossy(),
            "job_statuses": job_statuses
        }),
    )
    .expect("write run status");

    let report = build_worker_data_pipeline_execution_report(
        &plan_uri.to_string_lossy(),
        &run_status_uri.to_string_lossy(),
        &output_dir,
    )
    .expect("worker data pipeline execution report");

    assert_eq!(report["readiness_gate_status"], "blocked");
    assert_eq!(
        report["scheduler_status"],
        "completed_with_pending_or_failed_jobs"
    );
    assert_eq!(report["pending_or_failed_job_count"], 0);
    assert_eq!(report["review_task_count"], 1);
    assert_eq!(
        report["review_tasks"][0]["task_kind"],
        "worker_data_pipeline_readiness_gate_review"
    );
}

#[test]
fn builds_worker_data_pipeline_execution_submission() {
    let root = temp_root("worker-data-pipeline-execution-submission");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "readiness_report_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
            "readiness_gate_status": "ready",
            "customer_scope_id": "production-customer",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "completed",
                    "reported_status": "succeeded",
                    "reported_artifact_uri": "s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json",
                    "api_path": "/api/v1/ops/providers/sanctions-sync-reports",
                    "required_permission": "ops:providers:write",
                    "required_submit_flags": ["--published-report-uri", "--published-source-uri"],
                    "required_evidence_prefixes": ["sanctions_sync_reports:"],
                    "evidence_refs": [
                        "worker_job_artifacts:oig_sam_sanctions_sync:2026-06-14",
                        "sanctions_sync_reports:s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json"
                    ],
                    "submitted": true
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only; it must not score claims, assign labels, deny claims, activate models, or change routing policy",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
                "worker_data_pipeline_readiness_reports:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json"
            ]
        }),
    )
    .expect("write report");

    let published_report_uri =
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json";
    let submission = build_worker_data_pipeline_execution_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        published_report_uri,
    )
    .expect("worker data pipeline submission");

    assert_eq!(
        submission.report_kind,
        "worker_data_pipeline_execution_report"
    );
    assert_eq!(submission.run_id, "wdp_2026_06_14");
    assert_eq!(
        submission.readiness_report_uri.as_deref(),
        Some("s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json")
    );
    assert_eq!(submission.readiness_gate_status, "ready");
    assert_eq!(submission.job_count, 1);
    assert_eq!(submission.review_task_count, 0);
    assert!(submission.evidence_refs.iter().any(|reference| {
        reference == &format!("worker_data_pipeline_execution_reports:{published_report_uri}")
    }));
}

#[test]
fn rejects_worker_data_pipeline_execution_submission_with_duplicate_job_kind() {
    let root = temp_root("worker-data-pipeline-execution-submission-duplicate-job");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 2,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "artifact_pending_submission"
                },
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "artifact_pending_submission"
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_execution_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
    )
    .expect_err("duplicate job kind should fail before API submission");

    assert!(error
        .to_string()
        .contains("duplicate worker data pipeline job_kind oig_sam_sanctions_sync"));
}

#[test]
fn rejects_worker_data_pipeline_execution_submission_with_completed_job_permission_drift() {
    let root = temp_root("worker-data-pipeline-execution-submission-permission-drift");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "readiness_report_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
            "readiness_gate_status": "ready",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "completed",
                    "reported_status": "succeeded",
                    "reported_artifact_uri": "s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json",
                    "api_path": "/api/v1/ops/providers/sanctions-sync-reports",
                    "required_permission": "ops:datasets:write",
                    "required_submit_flags": ["--published-report-uri", "--published-source-uri"],
                    "required_evidence_prefixes": ["sanctions_sync_reports:"],
                    "evidence_refs": [
                        "sanctions_sync_reports:s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json"
                    ],
                    "submitted": true
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
                "worker_data_pipeline_readiness_reports:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_execution_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
    )
    .expect_err("completed job with wrong permission should fail before API submission");

    assert!(error
        .to_string()
        .contains("oig_sam_sanctions_sync requires required_permission ops:providers:write"));
}

#[test]
fn rejects_worker_data_pipeline_execution_submission_with_completed_job_reported_status_drift() {
    let root = temp_root("worker-data-pipeline-execution-submission-reported-status-drift");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "readiness_report_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
            "readiness_gate_status": "ready",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "completed",
                    "reported_status": "failed",
                    "reported_artifact_uri": "s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json",
                    "api_path": "/api/v1/ops/providers/sanctions-sync-reports",
                    "required_permission": "ops:providers:write",
                    "required_submit_flags": ["--published-report-uri", "--published-source-uri"],
                    "required_evidence_prefixes": ["sanctions_sync_reports:"],
                    "evidence_refs": [
                        "sanctions_sync_reports:s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json"
                    ],
                    "submitted": true
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
                "worker_data_pipeline_readiness_reports:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_execution_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
    )
    .expect_err("completed job with failed reported_status should fail before API submission");

    assert!(error
        .to_string()
        .contains("completed job executions require reported_status succeeded"));
}

#[test]
fn rejects_worker_data_pipeline_execution_submission_with_job_count_drift() {
    let root = temp_root("worker-data-pipeline-execution-submission-count-drift");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 2,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "artifact_pending_submission"
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_execution_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
    )
    .expect_err("job_count drift should fail before API submission");

    assert!(error
        .to_string()
        .contains("job_count must match job_executions length"));
}

#[test]
fn rejects_worker_data_pipeline_execution_submission_without_pending_job_review_task() {
    let root = temp_root("worker-data-pipeline-execution-submission-missing-review-task");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "readiness_report_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
            "readiness_gate_status": "ready",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 1,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "artifact_pending_submission"
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_execution_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
    )
    .expect_err("pending job without review task should fail before API submission");

    assert!(error
        .to_string()
        .contains("non-completed job requires matching worker_data_pipeline_execution_review"));
}

#[test]
fn rejects_worker_data_pipeline_execution_submission_with_review_task_permission_drift() {
    let root = temp_root("worker-data-pipeline-execution-submission-review-permission-drift");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "readiness_report_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
            "readiness_gate_status": "ready",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 1,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "artifact_pending_submission"
                }
            ],
            "review_task_count": 1,
            "review_tasks": [
                {
                    "task_kind": "worker_data_pipeline_execution_review",
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "artifact_pending_submission",
                    "api_path": "/api/v1/ops/providers/sanctions-sync-reports",
                    "required_permission": "ops:datasets:write",
                    "required_submit_flags": ["--published-report-uri", "--published-source-uri"]
                }
            ],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
                "worker_data_pipeline_readiness_reports:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_execution_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
    )
    .expect_err("review task with wrong permission should fail before API submission");

    assert!(error
        .to_string()
        .contains("oig_sam_sanctions_sync requires required_permission ops:providers:write"));
}

#[test]
fn rejects_worker_data_pipeline_execution_submission_without_readiness_gate_review_task() {
    let root = temp_root("worker-data-pipeline-execution-submission-missing-gate-review");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "readiness_report_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
            "readiness_gate_status": "blocked",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "completed",
                    "reported_artifact_uri": "s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json",
                    "required_evidence_prefixes": ["sanctions_sync_reports:"],
                    "evidence_refs": [
                        "sanctions_sync_reports:s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json"
                    ]
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
                "worker_data_pipeline_readiness_reports:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_execution_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
    )
    .expect_err("blocked readiness gate without review task should fail before API submission");

    assert!(error
        .to_string()
        .contains("non-ready readiness_gate_status requires matching worker_data_pipeline_readiness_gate_review"));
}

#[test]
fn rejects_worker_data_pipeline_execution_submission_without_missing_gate_review_task() {
    let root = temp_root("worker-data-pipeline-execution-submission-missing-readiness-gate");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "completed",
                    "reported_artifact_uri": "s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json",
                    "required_evidence_prefixes": ["sanctions_sync_reports:"],
                    "evidence_refs": [
                        "sanctions_sync_reports:s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json"
                    ]
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_execution_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
    )
    .expect_err("missing readiness gate without review task should fail before API submission");

    assert!(error
        .to_string()
        .contains("non-ready readiness_gate_status requires matching worker_data_pipeline_readiness_gate_review"));
}

#[test]
fn rejects_worker_data_pipeline_execution_submission_without_published_report_uri() {
    let root = temp_root("worker-data-pipeline-execution-submission-unpublished-report");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "completed",
                    "reported_status": "succeeded",
                    "reported_artifact_uri": "s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json",
                    "api_path": "/api/v1/ops/providers/sanctions-sync-reports",
                    "required_permission": "ops:providers:write",
                    "required_submit_flags": ["--published-report-uri", "--published-source-uri"],
                    "required_evidence_prefixes": ["sanctions_sync_reports:"],
                    "evidence_refs": [
                        "worker_job_artifacts:oig_sam_sanctions_sync:2026-06-14",
                        "sanctions_sync_reports:s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json"
                    ],
                    "submitted": true
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_execution_submission(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
    )
    .expect_err("local report path must not be used as published execution URI");

    assert!(error.to_string().contains(
        "worker data pipeline execution published_report_uri must use production evidence"
    ));
}

#[test]
fn rejects_worker_data_pipeline_execution_submission_without_source_evidence() {
    let root = temp_root("worker-data-pipeline-execution-submission-missing-source-evidence");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "readiness_report_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
            "readiness_gate_status": "ready",
            "customer_scope_id": "production-customer",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "completed",
                    "reported_artifact_uri": "s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json",
                    "api_path": "/api/v1/ops/providers/sanctions-sync-reports",
                    "required_permission": "ops:providers:write",
                    "required_submit_flags": ["--published-report-uri", "--published-source-uri"],
                    "required_evidence_prefixes": ["sanctions_sync_reports:"],
                    "evidence_refs": [
                        "worker_job_artifacts:oig_sam_sanctions_sync:2026-06-14",
                        "sanctions_sync_reports:s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json"
                    ],
                    "submitted": true
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only; it must not score claims, assign labels, deny claims, activate models, or change routing policy",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_execution_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
    )
    .expect_err("execution submission without readiness evidence must fail");

    assert!(error.to_string().contains(
        "worker_data_pipeline_readiness_reports:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json"
    ));
}

#[test]
fn rejects_worker_data_pipeline_execution_submission_with_local_lineage_uri() {
    let root = temp_root("worker-data-pipeline-execution-submission-local-lineage");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "local://plans/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "readiness_report_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
            "readiness_gate_status": "ready",
            "customer_scope_id": "production-customer",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "completed",
                    "reported_artifact_uri": "s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json",
                    "required_evidence_prefixes": ["sanctions_sync_reports:"],
                    "evidence_refs": [
                        "worker_job_artifacts:oig_sam_sanctions_sync:2026-06-14",
                        "sanctions_sync_reports:s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json"
                    ],
                    "submitted": true
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only; it must not score claims, assign labels, deny claims, activate models, or change routing policy",
            "evidence_refs": [
                "worker_data_pipeline_plans:local://plans/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_execution_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
    )
    .expect_err("execution submission with local lineage must fail");

    assert!(error
        .to_string()
        .contains("plan_uri must use production evidence"));
}

#[test]
fn rejects_worker_data_pipeline_execution_submission_with_relative_artifact_uri() {
    let root = temp_root("worker-data-pipeline-execution-submission-relative-artifact");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "readiness_report_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
            "readiness_gate_status": "ready",
            "customer_scope_id": "production-customer",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "completed",
                    "reported_status": "succeeded",
                    "reported_artifact_uri": "artifacts/worker/sanctions_sync_report.json",
                    "api_path": "/api/v1/ops/providers/sanctions-sync-reports",
                    "required_permission": "ops:providers:write",
                    "required_submit_flags": ["--published-report-uri", "--published-source-uri"],
                    "required_evidence_prefixes": ["sanctions_sync_reports:"],
                    "evidence_refs": [
                        "sanctions_sync_reports:s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json"
                    ],
                    "submitted": true
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
                "worker_data_pipeline_readiness_reports:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_execution_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
    )
    .expect_err("execution submission with relative artifact URI must fail");

    assert!(error
        .to_string()
        .contains("completed job executions require a production reported_artifact_uri"));
}

#[test]
fn rejects_worker_data_pipeline_execution_submission_with_local_top_level_evidence() {
    let root = temp_root("worker-data-pipeline-execution-submission-local-evidence");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "readiness_report_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
            "readiness_gate_status": "ready",
            "customer_scope_id": "production-customer",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "completed",
                    "reported_artifact_uri": "s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json",
                    "required_evidence_prefixes": ["sanctions_sync_reports:"],
                    "evidence_refs": [
                        "worker_job_artifacts:oig_sam_sanctions_sync:2026-06-14",
                        "sanctions_sync_reports:s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json"
                    ],
                    "submitted": true
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only; it must not score claims, assign labels, deny claims, activate models, or change routing policy",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
                "worker_data_pipeline_readiness_reports:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
                "scheduler_notes:local://notes/run.txt"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_execution_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
    )
    .expect_err("execution submission with local top-level evidence must fail");

    assert!(error
        .to_string()
        .contains("evidence_refs must not use local dry-run"));
}

#[test]
fn rejects_worker_data_pipeline_execution_submission_with_file_top_level_evidence() {
    let root = temp_root("worker-data-pipeline-execution-submission-file-evidence");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "readiness_report_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
            "readiness_gate_status": "ready",
            "customer_scope_id": "production-customer",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "completed",
                    "reported_artifact_uri": "s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json",
                    "required_evidence_prefixes": ["sanctions_sync_reports:"],
                    "evidence_refs": [
                        "worker_job_artifacts:oig_sam_sanctions_sync:2026-06-14",
                        "sanctions_sync_reports:s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json"
                    ],
                    "submitted": true
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only; it must not score claims, assign labels, deny claims, activate models, or change routing policy",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
                "worker_data_pipeline_readiness_reports:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
                "scheduler_notes:file://tmp/run.txt"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_execution_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
    )
    .expect_err("execution submission with file top-level evidence must fail");

    assert!(error
        .to_string()
        .contains("evidence_refs must not use local dry-run"));
}

#[test]
fn rejects_worker_data_pipeline_execution_submission_without_canonical_job_lineage() {
    let root = temp_root("worker-data-pipeline-execution-submission-missing-job-lineage");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "customer_scope_id": "production-customer",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "provider_profile_window_rollup",
                    "execution_status": "completed",
                    "reported_artifact_uri": "s3://customer-prod-artifacts/worker-data-pipeline/provider_profile_window_rollup_report.json",
                    "required_evidence_prefixes": ["provider_profile_window_rollups:"],
                    "evidence_refs": [
                        "provider_profile_window_rollups:s3://customer-prod-artifacts/worker-data-pipeline/provider_profile_window_rollup_report.json"
                    ],
                    "submitted": true
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only; it must not score claims, assign labels, deny claims, activate models, or change routing policy",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_execution_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
    )
    .expect_err("execution submission without canonical job lineage must fail");

    assert!(error
        .to_string()
        .contains("provider_profile_claim_snapshot:"));
}

#[test]
fn rejects_worker_data_pipeline_execution_submission_with_template_job_evidence() {
    let root = temp_root("worker-data-pipeline-execution-submission-template-evidence");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "customer_scope_id": "production-customer",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "provider_profile_window_rollup",
                    "execution_status": "completed",
                    "reported_artifact_uri": "local://template/worker/provider_profile_window_rollup.json",
                    "required_evidence_prefixes": [
                        "provider_profile_window_rollups:",
                        "provider_profile_claim_snapshot:"
                    ],
                    "evidence_refs": [
                        "provider_profile_window_rollups:local://template/worker/provider_profile_window_rollup.json",
                        "provider_profile_claim_snapshot:local://template/worker/provider_profile_window_rollup.json"
                    ],
                    "submitted": true
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only; it must not score claims, assign labels, deny claims, activate models, or change routing policy",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_execution_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
    )
    .expect_err("execution submission with template evidence must fail");

    assert!(error
        .to_string()
        .contains("reported_artifact_uri must not use local://template evidence"));
}

#[test]
fn rejects_worker_data_pipeline_execution_submission_with_file_job_evidence() {
    let root = temp_root("worker-data-pipeline-execution-submission-file-job-evidence");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "customer_scope_id": "production-customer",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "provider_profile_window_rollup",
                    "execution_status": "completed",
                    "reported_status": "succeeded",
                    "reported_artifact_uri": "s3://customer-prod-artifacts/worker-data-pipeline/provider_profile_window_rollup.json",
                    "api_path": "/api/v1/ops/providers/profile-window-rollups",
                    "required_permission": "ops:providers:write",
                    "required_submit_flags": ["--published-report-uri", "--published-source-uri"],
                    "required_evidence_prefixes": [
                        "provider_profile_window_rollups:",
                        "provider_profile_claim_snapshot:"
                    ],
                    "evidence_refs": [
                        "provider_profile_window_rollups:s3://customer-prod-artifacts/worker-data-pipeline/provider_profile_window_rollup.json",
                        "provider_profile_claim_snapshot:file://tmp/provider-claims.json"
                    ],
                    "submitted": true
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only; it must not score claims, assign labels, deny claims, activate models, or change routing policy",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_execution_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
    )
    .expect_err("execution submission with file job evidence must fail");

    assert!(error
        .to_string()
        .contains("provider_profile_window_rollup evidence_refs must not use local"));
}

#[tokio::test]
async fn submits_worker_data_pipeline_execution_report_to_api() {
    let root = temp_root("worker-data-pipeline-execution-submit-api");
    let report_uri = root.join("worker_data_pipeline_execution_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_execution_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "run_status_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
            "readiness_report_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
            "readiness_gate_status": "ready",
            "customer_scope_id": "production-customer",
            "run_id": "wdp_2026_06_14",
            "execution_date": "2026-06-14",
            "job_count": 1,
            "pending_or_failed_job_count": 0,
            "job_executions": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "execution_status": "completed",
                    "reported_status": "succeeded",
                    "reported_artifact_uri": "s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json",
                    "api_path": "/api/v1/ops/providers/sanctions-sync-reports",
                    "required_permission": "ops:providers:write",
                    "required_submit_flags": ["--published-report-uri", "--published-source-uri"],
                    "required_evidence_prefixes": ["sanctions_sync_reports:"],
                    "evidence_refs": [
                        "worker_job_artifacts:oig_sam_sanctions_sync:2026-06-14",
                        "sanctions_sync_reports:s3://customer-prod-artifacts/worker-data-pipeline/sanctions_sync_report.json"
                    ],
                    "submitted": true
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "worker data pipeline execution evidence may open operations review tasks only; it must not score claims, assign labels, deny claims, activate models, or change routing policy",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_run_status:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_run_status.json",
                "worker_data_pipeline_readiness_reports:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json"
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
        assert!(request.contains(r#""readiness_gate_status":"ready""#));
        assert!(request.contains("worker_data_pipeline_readiness_reports:"));
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

    let response = submit_worker_data_pipeline_execution_report_with_published_uri(
        &format!("http://{addr}"),
        "test-api-key",
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-scheduler",
        "daily execution evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_execution_report.json",
    )
    .await
    .expect("submit worker data pipeline execution report");
    server.await.unwrap();

    assert_eq!(response["run_id"], "wdp_2026_06_14");
    assert_eq!(response["claim_scoring"], false);
}

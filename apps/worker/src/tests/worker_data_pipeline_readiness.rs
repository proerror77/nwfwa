use super::*;

#[test]
fn builds_worker_data_pipeline_readiness_input_template() {
    let root = temp_root("worker-data-pipeline-readiness-input-template");
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

    let template = build_worker_data_pipeline_readiness_input_template(
        &plan_uri.to_string_lossy(),
        &output_dir,
    )
    .expect("readiness input template");

    assert_eq!(
        template["report_kind"],
        "worker_data_pipeline_readiness_input_template"
    );
    assert_eq!(template["template_only"], true);
    let checks = template["checks"].as_array().expect("checks");
    assert_eq!(checks.len(), 11);
    assert_eq!(checks[0]["job_kind"], "oig_sam_sanctions_snapshot_fetch");
    assert_eq!(
        checks[0]["build_command"],
        "fetch-oig-sam-sanctions-snapshot"
    );
    assert_eq!(
        checks[0]["artifact_uri"],
        "s3://nwfwa-production-artifacts/worker-data-pipelines/production-customer/sanctions/{as_of_date}/oig_sam_sanctions_snapshot.json"
    );
    assert_eq!(checks[0]["customer_approved"], false);
    assert_eq!(checks[0]["external_fetch_configured"], false);
    assert_eq!(checks[0]["api_path"], serde_json::Value::Null);
    assert_eq!(checks[0]["required_permission"], serde_json::Value::Null);
    assert_eq!(checks[0]["required_submit_flags"], serde_json::json!([]));
    assert_eq!(checks[0]["minimum_row_count"], 1);
    assert_eq!(checks[0]["coverage_window_days"], serde_json::Value::Null);
    assert_eq!(
        checks[0]["source_freshness_status"],
        "pending_customer_validation"
    );
    assert_eq!(
        checks[1]["depends_on"],
        serde_json::json!(["oig_sam_sanctions_snapshot_fetch"])
    );
    assert_eq!(
        checks[1]["api_path"],
        "/api/v1/ops/providers/sanctions-sync-reports"
    );
    assert_eq!(checks[1]["submit_command"], "submit-sanctions-sync-report");
    assert_eq!(
        checks[1]["required_submit_flags"],
        serde_json::json!(["--published-report-uri", "--published-source-uri"])
    );
    assert_eq!(checks[1]["required_permission"], "ops:providers:write");
    assert_eq!(
        checks[2]["required_evidence_prefixes"],
        serde_json::json!([
            "provider_profile_window_rollups:",
            "provider_profile_claim_snapshot:"
        ])
    );
    assert_eq!(
        checks[6]["required_evidence_prefixes"],
        serde_json::json!([
            "clinical_compatibility_references:",
            "clinical_compatibility_reference:",
            "clinical_policy_authority:"
        ])
    );
    assert_eq!(
        checks[8]["required_evidence_prefixes"],
        serde_json::json!([
            "scoring_feature_contexts:",
            "scoring_feature_context_claim_snapshot:",
            "episode_rollups:",
            "peer_benchmarks:",
            "clinical_compatibility:",
            "unbundling_candidates:"
        ])
    );
    assert_eq!(
        checks[8]["required_submit_flags"],
        serde_json::json!(["--published-report-uri"])
    );
    assert_eq!(checks[9]["job_kind"], "scoring_online_readback");
    assert_eq!(
        checks[9]["score_response_capture_command"],
        "fetch-scoring-readback-response"
    );
    assert_eq!(
        checks[9]["required_evidence_prefixes"],
        serde_json::json!([
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
        ])
    );
    assert!(output_dir
        .join("worker_data_pipeline_readiness_input_template.json")
        .exists());
}

#[test]
fn readiness_input_template_remains_blocked_until_customer_evidence_is_filled() {
    let root = temp_root("worker-data-pipeline-readiness-template-blocked");
    let plan_uri = root.join("worker_data_pipeline_plan.json");
    let template_dir = root.join("template");
    let report_dir = root.join("report");
    let template_uri = template_dir.join("worker_data_pipeline_readiness_input_template.json");
    let plan = build_worker_data_pipeline_plan(
        "http://api-server:8080",
        "s3://nwfwa-production-artifacts",
        "production-customer",
        "15 1 * * *",
        "30 2 1 * *",
    )
    .expect("worker data pipeline plan");
    write_json(plan_uri.clone(), &plan).expect("write plan");
    build_worker_data_pipeline_readiness_input_template(&plan_uri.to_string_lossy(), &template_dir)
        .expect("readiness input template");

    let report = build_worker_data_pipeline_readiness_report(
        &plan_uri.to_string_lossy(),
        &template_uri.to_string_lossy(),
        &report_dir,
    )
    .expect("readiness report from template");

    assert_eq!(report["readiness_status"], "blocked");
    assert_eq!(report["ready_job_count"], 0);
    assert_eq!(report["blocked_job_count"], 11);
    let first_job = &report["job_readiness"].as_array().expect("jobs")[0];
    assert!(first_job["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("customer_approval_missing")));
    assert!(first_job["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("row_count_below_minimum")));
    assert!(first_job["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("missing_evidence_refs")));
    assert!(first_job["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("missing_required_evidence_prefixes")));
    assert!(first_job["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("missing_coverage_window")));
    assert!(first_job["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("source_freshness_not_confirmed")));
}

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
                    "job_kind": "oig_sam_sanctions_snapshot_fetch",
                    "artifact_uri": "s3://nwfwa-production-artifacts/sanctions/source.json",
                    "customer_approved": true,
                    "external_fetch_configured": false,
                    "row_count": 12,
                    "minimum_row_count": 1,
                    "data_quality_status": "passed",
                    "coverage_window_days": 1,
                    "source_freshness_status": "fresh",
                    "evidence_refs": ["customer_approval:sanctions:2026-06-14"]
                },
                {
                    "job_kind": "provider_profile_window_rollup",
                    "artifact_uri": "",
                    "customer_approved": false,
                    "row_count": 20,
                    "minimum_row_count": 100,
                    "data_quality_status": "blocked",
                    "coverage_window_days": 0,
                    "source_freshness_status": "stale",
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
    assert_eq!(report["job_count"], 11);
    assert_eq!(report["ready_job_count"], 0);
    assert_eq!(report["blocked_job_count"], 11);
    let jobs = report["job_readiness"].as_array().expect("jobs");
    assert_eq!(jobs[0]["job_kind"], "oig_sam_sanctions_snapshot_fetch");
    assert_eq!(jobs[0]["api_path"], serde_json::Value::Null);
    assert_eq!(jobs[0]["required_permission"], serde_json::Value::Null);
    assert!(jobs[0]["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("external_oig_sam_fetch_not_configured")));
    assert_eq!(
        jobs[2]["api_path"],
        "/api/v1/ops/providers/profile-window-rollups"
    );
    assert_eq!(jobs[2]["required_permission"], "ops:providers:write");
    assert_eq!(
        jobs[2]["required_submit_flags"],
        serde_json::json!(["--published-report-uri", "--published-source-uri"])
    );
    assert!(jobs[2]["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("row_count_below_minimum")));
    assert!(jobs[2]["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("customer_approval_missing")));
    assert!(jobs[2]["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("missing_coverage_window")));
    assert!(jobs[2]["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("source_freshness_not_confirmed")));
    assert!(report["review_tasks"]
        .as_array()
        .expect("review tasks")
        .iter()
        .any(|task| task["job_kind"] == "provider_profile_window_rollup"
            && task["required_submit_flags"]
                == serde_json::json!(["--published-report-uri", "--published-source-uri"])));
    assert!(jobs[4]["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("missing_customer_readiness_check")));
    assert_eq!(report["review_task_count"], 11);
    assert!(report["review_tasks"]
        .as_array()
        .expect("review tasks")
        .iter()
        .any(|task| task["job_kind"] == "provider_profile_window_rollup"
            && task["api_path"] == "/api/v1/ops/providers/profile-window-rollups"
            && task["required_permission"] == "ops:providers:write"));
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
            let evidence_refs = job["required_evidence_prefixes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|prefix| {
                    format!(
                        "{}s3://nwfwa-production-artifacts/readiness/{job_kind}.json",
                        prefix.as_str().unwrap()
                    )
                })
                .chain(std::iter::once(format!(
                    "customer_approval:{job_kind}:2026-06-14"
                )))
                .collect::<Vec<_>>();
            serde_json::json!({
                "job_kind": job_kind,
                "artifact_uri": format!("s3://nwfwa-production-artifacts/readiness/{job_kind}.json"),
                "customer_approved": true,
                "external_fetch_configured": job_kind == "oig_sam_sanctions_snapshot_fetch",
                "row_count": 100,
                "minimum_row_count": 10,
                "data_quality_status": "passed",
                "coverage_window_days": if job_kind == "peer_percentile_benchmark" { 365 } else { 90 },
                "source_freshness_status": "fresh",
                "evidence_refs": evidence_refs
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
    assert_eq!(report["ready_job_count"], 11);
    assert_eq!(report["blocked_job_count"], 0);
    assert_eq!(report["review_task_count"], 0);
}

#[test]
fn blocks_worker_data_pipeline_readiness_when_template_refs_are_not_replaced() {
    let root = temp_root("worker-data-pipeline-readiness-template-refs");
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
                    "job_kind": "provider_profile_window_rollup",
                    "artifact_uri": "local://template/worker/provider_profile_window_rollup.json",
                    "customer_approved": true,
                    "external_fetch_configured": false,
                    "row_count": 100,
                    "minimum_row_count": 10,
                    "data_quality_status": "passed",
                    "coverage_window_days": 90,
                    "source_freshness_status": "fresh",
                    "evidence_refs": [
                        "provider_profile_window_rollups:local://template/worker/provider_profile_window_rollup.json",
                        "provider_profile_claim_snapshot:local://template/worker/provider_profile_window_rollup.json"
                    ]
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

    let jobs = report["job_readiness"].as_array().expect("jobs");
    let provider_profile = jobs
        .iter()
        .find(|job| job["job_kind"] == "provider_profile_window_rollup")
        .expect("provider profile readiness");
    assert_eq!(provider_profile["readiness_status"], "blocked");
    assert!(provider_profile["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("template_artifact_uri_not_replaced")));
    assert!(provider_profile["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("non_production_evidence_refs")));
}

#[test]
fn blocks_worker_data_pipeline_readiness_when_artifact_uri_has_placeholders() {
    let root = temp_root("worker-data-pipeline-readiness-placeholder-artifact");
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
                    "job_kind": "peer_percentile_benchmark",
                    "artifact_uri": "s3://nwfwa-production-artifacts/peer-benchmark/{benchmark_month}/peer_percentile_benchmark.json",
                    "customer_approved": true,
                    "external_fetch_configured": false,
                    "row_count": 100,
                    "minimum_row_count": 10,
                    "data_quality_status": "passed",
                    "coverage_window_days": 365,
                    "source_freshness_status": "fresh",
                    "evidence_refs": [
                        "peer_benchmarks:s3://nwfwa-production-artifacts/peer-benchmark/2026-06/peer_percentile_benchmark.json",
                        "peer_benchmark_claim_snapshot:s3://nwfwa-production-artifacts/peer-benchmark/2026-06/claims.json"
                    ]
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

    let jobs = report["job_readiness"].as_array().expect("jobs");
    let peer_benchmark = jobs
        .iter()
        .find(|job| job["job_kind"] == "peer_percentile_benchmark")
        .expect("peer benchmark readiness");
    assert_eq!(peer_benchmark["readiness_status"], "blocked");
    assert!(peer_benchmark["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("non_production_artifact_uri")));
}

#[test]
fn blocks_worker_data_pipeline_readiness_when_evidence_refs_are_local() {
    let root = temp_root("worker-data-pipeline-readiness-local-evidence");
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
                    "job_kind": "peer_percentile_benchmark",
                    "artifact_uri": "s3://nwfwa-production-artifacts/peer-benchmark/2026-06/peer_percentile_benchmark.json",
                    "customer_approved": true,
                    "external_fetch_configured": false,
                    "row_count": 100,
                    "minimum_row_count": 10,
                    "data_quality_status": "passed",
                    "coverage_window_days": 365,
                    "source_freshness_status": "fresh",
                    "evidence_refs": [
                        "peer_benchmarks:local://artifacts/peer_percentile_benchmark.json",
                        "peer_benchmark_claim_snapshot:s3://nwfwa-production-artifacts/peer-benchmark/2026-06/claims.json"
                    ]
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

    let jobs = report["job_readiness"].as_array().expect("jobs");
    let peer_benchmark = jobs
        .iter()
        .find(|job| job["job_kind"] == "peer_percentile_benchmark")
        .expect("peer benchmark readiness");
    assert_eq!(peer_benchmark["readiness_status"], "blocked");
    assert!(peer_benchmark["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("non_production_evidence_refs")));
}

#[test]
fn blocks_worker_data_pipeline_readiness_when_required_evidence_prefix_is_blank() {
    let root = temp_root("worker-data-pipeline-readiness-blank-prefix");
    let plan_uri = root.join("worker_data_pipeline_plan.json");
    let readiness_uri = root.join("worker_data_pipeline_readiness_input.json");
    let output_dir = root.join("output");
    let mut plan = build_worker_data_pipeline_plan(
        "http://api-server:8080",
        "s3://nwfwa-production-artifacts",
        "production-customer",
        "15 1 * * *",
        "30 2 1 * *",
    )
    .expect("worker data pipeline plan");
    plan["jobs"][0]["required_evidence_prefixes"] = serde_json::json!([" "]);
    write_json(plan_uri.clone(), &plan).expect("write plan");
    write_json(
        readiness_uri.clone(),
        &serde_json::json!({
            "checks": [
                {
                    "job_kind": "oig_sam_sanctions_snapshot_fetch",
                    "artifact_uri": "s3://nwfwa-production-artifacts/readiness/oig_sam_sanctions_snapshot_fetch.json",
                    "customer_approved": true,
                    "external_fetch_configured": true,
                    "row_count": 100,
                    "minimum_row_count": 10,
                    "data_quality_status": "passed",
                    "coverage_window_days": 90,
                    "source_freshness_status": "fresh",
                    "evidence_refs": ["customer_approval:oig_sam_sanctions_snapshot_fetch:2026-06-14"]
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

    let first_job = &report["job_readiness"].as_array().expect("jobs")[0];
    assert_eq!(first_job["readiness_status"], "blocked");
    assert!(first_job["blockers"]
        .as_array()
        .unwrap()
        .contains(&serde_json::json!("blank_required_evidence_prefixes")));
}

#[test]
fn builds_worker_data_pipeline_readiness_submission() {
    let root = temp_root("worker-data-pipeline-readiness-submission");
    let report_uri = root.join("worker_data_pipeline_readiness_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_readiness_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "readiness_input_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json",
            "customer_scope_id": "production-customer",
            "readiness_status": "ready",
            "job_count": 1,
            "ready_job_count": 1,
            "blocked_job_count": 0,
            "job_readiness": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "readiness_status": "ready"
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "readiness report validates customer data prerequisites only; it must not fetch external data, submit artifacts, score claims, assign labels, activate models, or change routing policy",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_readiness_inputs:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json"
            ]
        }),
    )
    .expect("write report");

    let published_report_uri =
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json";
    let submission = build_worker_data_pipeline_readiness_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-readiness",
        "daily readiness evidence",
        published_report_uri,
    )
    .expect("worker data pipeline readiness submission");

    assert_eq!(
        submission.report_kind,
        "worker_data_pipeline_readiness_report"
    );
    assert_eq!(submission.readiness_status, "ready");
    assert_eq!(submission.job_count, 1);
    assert_eq!(submission.ready_job_count, 1);
    assert!(submission.evidence_refs.iter().any(|reference| {
        reference == &format!("worker_data_pipeline_readiness_reports:{published_report_uri}")
    }));
}

#[test]
fn rejects_worker_data_pipeline_readiness_submission_with_unknown_job_kind() {
    let root = temp_root("worker-data-pipeline-readiness-submission-unknown-job");
    let report_uri = root.join("worker_data_pipeline_readiness_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_readiness_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "readiness_input_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json",
            "readiness_status": "ready",
            "job_count": 1,
            "ready_job_count": 1,
            "blocked_job_count": 0,
            "job_readiness": [
                {
                    "job_kind": "unknown_worker_job",
                    "readiness_status": "ready"
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "readiness report validates customer data prerequisites only",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_readiness_inputs:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_readiness_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-readiness",
        "daily readiness evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
    )
    .expect_err("unknown job kind should fail before API submission");

    assert!(error
        .to_string()
        .contains("unknown worker data pipeline job_kind unknown_worker_job"));
}

#[test]
fn rejects_worker_data_pipeline_readiness_submission_with_ready_count_drift() {
    let root = temp_root("worker-data-pipeline-readiness-submission-count-drift");
    let report_uri = root.join("worker_data_pipeline_readiness_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_readiness_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "readiness_input_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json",
            "readiness_status": "ready",
            "job_count": 1,
            "ready_job_count": 0,
            "blocked_job_count": 0,
            "job_readiness": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "readiness_status": "ready"
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "readiness report validates customer data prerequisites only",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_readiness_inputs:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_readiness_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-readiness",
        "daily readiness evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
    )
    .expect_err("ready count drift should fail before API submission");

    assert!(error
        .to_string()
        .contains("ready_job_count and blocked_job_count must match job_readiness statuses"));
}

#[test]
fn rejects_worker_data_pipeline_readiness_submission_without_blocked_job_review_task() {
    let root = temp_root("worker-data-pipeline-readiness-submission-missing-review-task");
    let report_uri = root.join("worker_data_pipeline_readiness_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_readiness_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "readiness_input_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json",
            "readiness_status": "blocked",
            "job_count": 1,
            "ready_job_count": 0,
            "blocked_job_count": 1,
            "job_readiness": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "readiness_status": "blocked"
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "readiness report validates customer data prerequisites only",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_readiness_inputs:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_readiness_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-readiness",
        "daily readiness evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
    )
    .expect_err("blocked job without review task should fail before API submission");

    assert!(error
        .to_string()
        .contains("blocked job requires matching worker_data_pipeline_readiness_review"));
}

#[test]
fn rejects_worker_data_pipeline_readiness_submission_without_published_report_uri() {
    let root = temp_root("worker-data-pipeline-readiness-submission-unpublished-report");
    let report_uri = root.join("worker_data_pipeline_readiness_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_readiness_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "readiness_input_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json",
            "readiness_status": "ready",
            "job_count": 1,
            "ready_job_count": 1,
            "blocked_job_count": 0,
            "job_readiness": [{"job_kind": "oig_sam_sanctions_sync", "readiness_status": "ready"}],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "readiness report validates customer data prerequisites only",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_readiness_inputs:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_readiness_submission(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-readiness",
        "daily readiness evidence",
    )
    .expect_err("local report path must not be used as published readiness URI");

    assert!(error.to_string().contains(
        "worker data pipeline readiness published_report_uri must use production evidence"
    ));
}

#[test]
fn rejects_worker_data_pipeline_readiness_submission_without_source_evidence() {
    let root = temp_root("worker-data-pipeline-readiness-submission-missing-source-evidence");
    let report_uri = root.join("worker_data_pipeline_readiness_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_readiness_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "readiness_input_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json",
            "customer_scope_id": "production-customer",
            "readiness_status": "ready",
            "job_count": 1,
            "ready_job_count": 1,
            "blocked_job_count": 0,
            "job_readiness": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "readiness_status": "ready"
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "readiness report validates customer data prerequisites only; it must not fetch external data, submit artifacts, score claims, assign labels, activate models, or change routing policy",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_readiness_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-readiness",
        "daily readiness evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
    )
    .expect_err("readiness submission without input evidence must fail");

    assert!(error.to_string().contains(
        "worker_data_pipeline_readiness_inputs:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json"
    ));
}

#[test]
fn rejects_worker_data_pipeline_readiness_submission_with_local_lineage_uri() {
    let root = temp_root("worker-data-pipeline-readiness-submission-local-lineage");
    let report_uri = root.join("worker_data_pipeline_readiness_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_readiness_report",
            "plan_uri": "local://plans/worker_data_pipeline_plan.json",
            "readiness_input_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json",
            "customer_scope_id": "production-customer",
            "readiness_status": "ready",
            "job_count": 1,
            "ready_job_count": 1,
            "blocked_job_count": 0,
            "job_readiness": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "readiness_status": "ready"
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "readiness report validates customer data prerequisites only; it must not fetch external data, submit artifacts, score claims, assign labels, activate models, or change routing policy",
            "evidence_refs": [
                "worker_data_pipeline_plans:local://plans/worker_data_pipeline_plan.json",
                "worker_data_pipeline_readiness_inputs:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_readiness_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-readiness",
        "daily readiness evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
    )
    .expect_err("readiness submission with local lineage must fail");

    assert!(error
        .to_string()
        .contains("plan_uri must use production evidence"));
}

#[test]
fn rejects_worker_data_pipeline_readiness_submission_with_local_top_level_evidence() {
    let root = temp_root("worker-data-pipeline-readiness-submission-local-evidence");
    let report_uri = root.join("worker_data_pipeline_readiness_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_readiness_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "readiness_input_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json",
            "customer_scope_id": "production-customer",
            "readiness_status": "ready",
            "job_count": 1,
            "ready_job_count": 1,
            "blocked_job_count": 0,
            "job_readiness": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "readiness_status": "ready"
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "readiness report validates customer data prerequisites only; it must not fetch external data, submit artifacts, score claims, assign labels, activate models, or change routing policy",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_readiness_inputs:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json",
                "customer_readiness_notes:local://notes/readiness.txt"
            ]
        }),
    )
    .expect("write report");

    let error = build_worker_data_pipeline_readiness_submission_with_published_uri(
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-readiness",
        "daily readiness evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
    )
    .expect_err("readiness submission with local top-level evidence must fail");

    assert!(error
        .to_string()
        .contains("evidence_refs must not use local dry-run"));
}

#[tokio::test]
async fn submits_worker_data_pipeline_readiness_report_to_api() {
    let root = temp_root("worker-data-pipeline-readiness-submit-api");
    let report_uri = root.join("worker_data_pipeline_readiness_report.json");
    write_json(
        report_uri.clone(),
        &serde_json::json!({
            "report_kind": "worker_data_pipeline_readiness_report",
            "plan_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
            "readiness_input_uri": "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json",
            "customer_scope_id": "production-customer",
            "readiness_status": "ready",
            "job_count": 1,
            "ready_job_count": 1,
            "blocked_job_count": 0,
            "job_readiness": [
                {
                    "job_kind": "oig_sam_sanctions_sync",
                    "readiness_status": "ready"
                }
            ],
            "review_task_count": 0,
            "review_tasks": [],
            "governance_boundary": "readiness report validates customer data prerequisites only; it must not fetch external data, submit artifacts, score claims, assign labels, activate models, or change routing policy",
            "evidence_refs": [
                "worker_data_pipeline_plans:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_plan.json",
                "worker_data_pipeline_readiness_inputs:s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_input.json"
            ]
        }),
    )
    .expect("write report");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let request = read_http_request(&mut socket).await;
        assert!(request.contains("POST /api/v1/ops/worker-data-pipeline-readiness HTTP/1.1"));
        assert!(request.contains(r#""report_kind":"worker_data_pipeline_readiness_report""#));
        assert!(request.contains("worker_data_pipeline_readiness_reports:"));
        write_json_response(
            &mut socket,
            serde_json::json!({
                "report_kind": "worker_data_pipeline_readiness_report",
                "readiness_status": "ready",
                "external_fetch_execution": false
            }),
        )
        .await;
    });

    let response = submit_worker_data_pipeline_readiness_report_with_published_uri(
        &format!("http://{addr}"),
        "test-api-key",
        &report_uri.to_string_lossy(),
        "worker:worker-data-pipeline-readiness",
        "daily readiness evidence",
        "s3://customer-prod-artifacts/worker-data-pipeline/worker_data_pipeline_readiness_report.json",
    )
    .await
    .expect("submit worker data pipeline readiness report");
    server.await.unwrap();

    assert_eq!(response["readiness_status"], "ready");
    assert_eq!(response["external_fetch_execution"], false);
}

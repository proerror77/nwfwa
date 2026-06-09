use super::*;

#[test]
fn builds_scheduled_mlops_monitoring_plan() {
    let plan = build_mlops_monitoring_plan(
        "data/training/manifest.json",
        "s3://fwa-models/baseline_fwa/0.2.0/rust_serving_artifact.json",
        "baseline_fwa",
        "0.2.0",
        "0 2 * * *",
    )
    .expect("mlops monitoring plan");

    assert_eq!(plan["plan_kind"], "scheduled_mlops_monitoring");
    assert_eq!(plan["plan_version"], 2);
    assert_eq!(plan["model"]["model_key"], "baseline_fwa");
    assert_eq!(plan["model"]["model_version"], "0.2.0");
    assert_eq!(plan["schedule"]["cron"], "0 2 * * *");
    assert_eq!(
        plan["data_contract"]["source"],
        "same_parquet_dataset_manifest"
    );
    assert_eq!(plan["jobs"][0]["job_kind"], "shadow_traffic_evaluation");
    assert_eq!(plan["jobs"][1]["job_kind"], "drift_monitoring");
    assert_eq!(plan["jobs"][2]["job_kind"], "segment_fairness_review");
    assert_eq!(plan["jobs"][3]["job_kind"], "reviewer_disagreement_review");
    assert_eq!(plan["jobs"][4]["job_kind"], "label_delay_review");
    assert_eq!(
        plan["jobs"][1]["drift_report_uri"],
        "s3://fwa-models/baseline_fwa/0.2.0/drift_report.json"
    );
    assert_eq!(
        plan["jobs"][3]["reviewer_disagreement_report_uri"],
        "s3://fwa-models/baseline_fwa/0.2.0/reviewer_disagreement_report.json"
    );
    assert_eq!(
        plan["jobs"][4]["label_delay_report_uri"],
        "s3://fwa-models/baseline_fwa/0.2.0/label_delay_report.json"
    );
}

#[test]
fn runs_mlops_monitoring_plan_runtime_report_producer() {
    let root = temp_root("mlops-runtime-report-producer");
    let plan = build_mlops_monitoring_plan(
        "data/training/manifest.json",
        "s3://fwa-models/baseline_fwa/0.2.0/rust_serving_artifact.json",
        "baseline_fwa",
        "0.2.0",
        "0 2 * * *",
    )
    .expect("mlops monitoring plan");
    let plan_uri = root.join("mlops_monitoring_plan.json");
    write_json(plan_uri.clone(), &plan).unwrap();

    let index = run_mlops_monitoring_plan(&plan_uri.to_string_lossy(), root.join("runtime"))
        .expect("runtime reports");

    assert_eq!(
        index["artifact_kind"],
        "rust_mlops_monitoring_runtime_reports"
    );
    assert_eq!(index["model_key"], "baseline_fwa");
    assert_eq!(index["model_version"], "0.2.0");
    assert_eq!(index["status"], "completed");
    assert_eq!(
        index["artifacts"]["shadow_traffic_evaluation"],
        "shadow_report.json"
    );
    assert!(index["governance_boundary"]
        .as_str()
        .unwrap()
        .contains("must not create retraining jobs"));
    assert!(root.join("runtime/index.json").is_file());
    assert!(root.join("runtime/shadow_report.json").is_file());
    assert!(root.join("runtime/drift_report.json").is_file());
    assert!(root.join("runtime/fairness_report.json").is_file());
    assert!(root
        .join("runtime/reviewer_disagreement_report.json")
        .is_file());
    assert!(root.join("runtime/label_delay_report.json").is_file());
    let drift = read_json_report(&root.join("runtime/drift_report.json").to_string_lossy())
        .expect("drift report");
    assert_eq!(
        drift["runtime_source"],
        "rust_worker_monitoring_plan_runner"
    );
    assert_eq!(drift["status"], "stable");
}

#[test]
fn runs_mlops_monitoring_plan_with_legacy_flat_plan_shape() {
    let root = temp_root("mlops-runtime-flat-plan");
    let plan = serde_json::json!({
        "plan_kind": "scheduled_mlops_monitoring",
        "manifest_uri": "s3://nwfwa-staging-artifacts/datasets/public-mvp/manifest.json",
        "artifact_uri": "s3://nwfwa-staging-artifacts/models/baseline_fwa/staging/rust_serving_artifact.json",
        "model_key": "baseline_fwa",
        "model_version": "staging",
        "cron": "0 2 * * *",
        "jobs": [
            {"job_kind": "shadow_traffic_evaluation", "output_ref": "model_shadow_reports:<shadow_report_uri>", "shadow_report_uri": "s3://nwfwa-staging-artifacts/models/baseline_fwa/staging/shadow_report.json"},
            {"job_kind": "drift_monitoring", "output_ref": "model_drift_reports:<drift_report_uri>", "drift_report_uri": "s3://nwfwa-staging-artifacts/models/baseline_fwa/staging/drift_report.json"},
            {"job_kind": "segment_fairness_review", "output_ref": "model_fairness_reports:<fairness_report_uri>", "fairness_report_uri": "s3://nwfwa-staging-artifacts/models/baseline_fwa/staging/fairness_report.json"},
            {"job_kind": "reviewer_disagreement_review", "output_ref": "reviewer_disagreement_reports:<reviewer_disagreement_report_uri>", "reviewer_disagreement_report_uri": "s3://nwfwa-staging-artifacts/models/baseline_fwa/staging/reviewer_disagreement_report.json"},
            {"job_kind": "label_delay_review", "output_ref": "label_delay_reports:<label_delay_report_uri>", "label_delay_report_uri": "s3://nwfwa-staging-artifacts/models/baseline_fwa/staging/label_delay_report.json"}
        ]
    });
    let plan_uri = root.join("sample_mlops_monitoring_plan.json");
    write_json(plan_uri.clone(), &plan).unwrap();

    let index = run_mlops_monitoring_plan(&plan_uri.to_string_lossy(), root.join("runtime"))
        .expect("runtime reports");

    assert_eq!(index["model_version"], "staging");
    assert_eq!(index["manifest_uri"], plan["manifest_uri"]);
    assert_eq!(
        index["artifacts"]["label_delay_review"],
        "label_delay_report.json"
    );
    assert!(root.join("runtime/label_delay_report.json").is_file());
}

#[test]
fn runs_scheduled_mlops_monitoring_from_parameters() {
    let root = temp_root("scheduled-mlops-monitoring");
    let index = run_scheduled_mlops_monitoring(
        "data/training/manifest.json",
        "s3://fwa-models/baseline_fwa/0.2.0/rust_serving_artifact.json",
        "baseline_fwa",
        "0.2.0",
        "0 2 * * *",
        root.join("runtime"),
    )
    .expect("scheduled runtime reports");

    assert_eq!(
        index["artifact_kind"],
        "rust_mlops_monitoring_runtime_reports"
    );
    assert_eq!(
        index["plan_uri"].as_str(),
        Some(
            root.join("runtime/mlops_monitoring_plan.json")
                .to_string_lossy()
                .as_ref()
        )
    );
    assert!(root.join("runtime/mlops_monitoring_plan.json").is_file());
    assert!(root.join("runtime/shadow_report.json").is_file());
    assert!(root
        .join("runtime/reviewer_disagreement_report.json")
        .is_file());
    assert!(root.join("runtime/label_delay_report.json").is_file());
}

#[test]
fn scheduled_mlops_monitoring_writes_artifact_publication_manifest() {
    let root = temp_root("scheduled-mlops-monitoring-publication");
    let index = run_scheduled_mlops_monitoring_with_artifact_base_uri(
        "data/training/manifest.json",
        "s3://fwa-models/baseline_fwa/0.2.0/rust_serving_artifact.json",
        "baseline_fwa",
        "0.2.0",
        "0 2 * * *",
        root.join("runtime"),
        Some("s3://fwa-models/baseline_fwa/0.2.0/mlops-monitoring"),
    )
    .expect("scheduled runtime reports");

    assert_eq!(
        index["artifact_publication_status"],
        "publication_manifest_ready"
    );
    let manifest_path = root
        .join("runtime")
        .join("mlops_monitoring_artifact_publication_manifest.json");
    assert!(manifest_path.is_file());
    let manifest =
        read_json_report(&manifest_path.to_string_lossy()).expect("publication manifest");
    assert_eq!(
        manifest["artifact_kind"],
        "mlops_monitoring_artifact_publication_manifest"
    );
    assert_eq!(manifest["artifact_count"], 7);
    assert!(manifest["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|artifact| artifact["target_uri"]
            == "s3://fwa-models/baseline_fwa/0.2.0/mlops-monitoring/shadow_report.json"
            && artifact["sha256"].as_str().unwrap().starts_with("sha256:")));
    let index_checksum = sha256_prefixed_hex(
        &fs::read(root.join("runtime/index.json")).expect("final runtime index"),
    );
    assert!(manifest["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .any(|artifact| artifact["file_name"] == "index.json"
            && artifact["sha256"] == index_checksum));
}

#[test]
fn scheduled_mlops_monitoring_binds_customer_monitoring_inputs() {
    let root = temp_root("scheduled-mlops-monitoring-inputs");
    let inputs_path = root.join("monitoring_inputs.json");
    write_json(
        inputs_path.clone(),
        &serde_json::json!({
            "artifact_kind": "mlops_monitoring_inputs",
            "source": "pilot_shadow_window",
            "jobs": {
                "shadow_traffic_evaluation": {
                    "status": "passed",
                    "comparison_count": 240,
                    "average_abs_probability_delta": 0.02,
                    "max_abs_probability_delta": 0.07
                },
                "drift_monitoring": {
                    "status": "watch",
                    "score_psi": 0.14,
                    "max_feature_psi": 0.18
                },
                "segment_fairness_review": {
                    "status": "passed",
                    "segments": [{"segment_column": "provider_region", "segment_value": "north"}]
                },
                "reviewer_disagreement_review": {
                    "reviewer_disagreement_rate": 0.05,
                    "review_sample_count": 240
                },
                "label_delay_review": {
                    "label_delay_p95_days": 9,
                    "delayed_label_count": 2
                }
            }
        }),
    )
    .unwrap();

    let index = run_scheduled_mlops_monitoring_with_options(
        "data/training/manifest.json",
        "s3://fwa-models/baseline_fwa/0.2.0/rust_serving_artifact.json",
        "baseline_fwa",
        "0.2.0",
        "0 2 * * *",
        root.join("runtime"),
        None,
        Some(&inputs_path.to_string_lossy()),
    )
    .expect("scheduled runtime reports");

    assert_eq!(index["customer_data_bound"], true);
    assert_eq!(index["customer_data_required"], false);
    assert_eq!(index["input_binding_status"], "provided");
    let drift = read_json_report(&root.join("runtime/drift_report.json").to_string_lossy())
        .expect("drift report");
    assert_eq!(drift["status"], "watch");
    assert_eq!(drift["score_psi"], 0.14);
    assert_eq!(drift["customer_data_bound"], true);
    assert_eq!(drift["input_binding_job_kind"], "drift_monitoring");
    let shadow = read_json_report(&root.join("runtime/shadow_report.json").to_string_lossy())
        .expect("shadow report");
    assert_eq!(shadow["comparison_count"], 240);
    assert_eq!(shadow["max_abs_probability_delta"], 0.07);
}

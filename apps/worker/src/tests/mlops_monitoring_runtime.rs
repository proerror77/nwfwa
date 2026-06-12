use super::*;
use crate::mlops_monitoring_plan::compute_psi;

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
    assert_eq!(plan["jobs"][2]["job_kind"], "feature_distribution_psi");
    assert_eq!(plan["jobs"][3]["job_kind"], "rule_hit_rate_trend");
    assert_eq!(plan["jobs"][4]["job_kind"], "segment_fairness_review");
    assert_eq!(plan["jobs"][5]["job_kind"], "reviewer_disagreement_review");
    assert_eq!(plan["jobs"][6]["job_kind"], "label_delay_review");
    assert_eq!(
        plan["jobs"][1]["drift_report_uri"],
        "s3://fwa-models/baseline_fwa/0.2.0/drift_report.json"
    );
    assert_eq!(
        plan["jobs"][5]["reviewer_disagreement_report_uri"],
        "s3://fwa-models/baseline_fwa/0.2.0/reviewer_disagreement_report.json"
    );
    assert_eq!(
        plan["jobs"][6]["label_delay_report_uri"],
        "s3://fwa-models/baseline_fwa/0.2.0/label_delay_report.json"
    );
    assert_eq!(
        plan["jobs"][2]["feature_psi_report_uri"],
        "s3://fwa-models/baseline_fwa/0.2.0/feature_psi_report.json"
    );
    assert_eq!(
        plan["jobs"][3]["rule_hit_rate_report_uri"],
        "s3://fwa-models/baseline_fwa/0.2.0/rule_hit_rate_report.json"
    );
}

#[test]
fn computes_population_stability_index_for_percentile_buckets() {
    let baseline = [5.0, 15.0, 25.0, 35.0, 45.0, 55.0, 65.0, 75.0, 85.0, 95.0];
    let stable = [6.0, 14.0, 26.0, 34.0, 46.0, 54.0, 66.0, 74.0, 86.0, 94.0];
    let shifted = [85.0, 86.0, 87.0, 88.0, 89.0, 90.0, 91.0, 92.0, 93.0, 94.0];

    assert!(compute_psi(&baseline, &stable, 10) < 0.1);
    assert!(compute_psi(&baseline, &shifted, 10) > 0.25);
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
    assert!(root.join("runtime/feature_psi_report.json").is_file());
    assert!(root.join("runtime/rule_hit_rate_report.json").is_file());
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
    let feature_psi = read_json_report(
        &root
            .join("runtime/feature_psi_report.json")
            .to_string_lossy(),
    )
    .expect("feature PSI report");
    assert_eq!(feature_psi["feature"], "claim_amount_peer_percentile");
    let rule_hit_rate = read_json_report(
        &root
            .join("runtime/rule_hit_rate_report.json")
            .to_string_lossy(),
    )
    .expect("rule hit rate report");
    assert_eq!(
        rule_hit_rate["alert_condition"],
        "hit_rate_7d < 0.5 * hit_rate_90d"
    );
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
            {"job_kind": "feature_distribution_psi", "output_ref": "feature_distribution_psi_reports:<feature_psi_report_uri>", "feature_psi_report_uri": "s3://nwfwa-staging-artifacts/models/baseline_fwa/staging/feature_psi_report.json"},
            {"job_kind": "rule_hit_rate_trend", "output_ref": "rule_drift_reports:<rule_hit_rate_report_uri>", "rule_hit_rate_report_uri": "s3://nwfwa-staging-artifacts/models/baseline_fwa/staging/rule_hit_rate_report.json"},
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
    assert!(root.join("runtime/feature_psi_report.json").is_file());
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
    assert_eq!(manifest["artifact_count"], 9);
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
                "feature_distribution_psi": {
                    "status": "watch",
                    "feature": "claim_amount_peer_percentile",
                    "psi": 0.19
                },
                "rule_hit_rate_trend": {
                    "status": "watch",
                    "rules_evaluated": 14,
                    "rule_drift_alerts": [{"rule_id": "R-1", "hit_rate_7d": 0.02, "hit_rate_90d": 0.08}]
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
    let feature_psi = read_json_report(
        &root
            .join("runtime/feature_psi_report.json")
            .to_string_lossy(),
    )
    .expect("feature PSI report");
    assert_eq!(feature_psi["status"], "watch");
    assert_eq!(feature_psi["psi"], 0.19);
    let rule_hit_rate = read_json_report(
        &root
            .join("runtime/rule_hit_rate_report.json")
            .to_string_lossy(),
    )
    .expect("rule hit rate report");
    assert_eq!(rule_hit_rate["rules_evaluated"], 14);
    let shadow = read_json_report(&root.join("runtime/shadow_report.json").to_string_lossy())
        .expect("shadow report");
    assert_eq!(shadow["comparison_count"], 240);
    assert_eq!(shadow["max_abs_probability_delta"], 0.07);
}

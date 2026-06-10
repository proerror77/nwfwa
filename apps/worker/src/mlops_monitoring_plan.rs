use super::{artifact_parent_uri, required_non_empty};

pub fn build_mlops_monitoring_plan(
    manifest_uri: &str,
    artifact_uri: &str,
    model_key: &str,
    model_version: &str,
    cron: &str,
) -> anyhow::Result<serde_json::Value> {
    let manifest_uri = required_non_empty("manifest_uri", manifest_uri)?;
    let artifact_uri = required_non_empty("artifact_uri", artifact_uri)?;
    let model_key = required_non_empty("model_key", model_key)?;
    let model_version = required_non_empty("model_version", model_version)?;
    let cron = required_non_empty("cron", cron)?;
    let artifact_dir = artifact_parent_uri(artifact_uri);

    Ok(serde_json::json!({
        "plan_kind": "scheduled_mlops_monitoring",
        "plan_version": 2,
        "data_contract": {
            "source": "same_parquet_dataset_manifest",
            "manifest_uri": manifest_uri
        },
        "model": {
            "model_key": model_key,
            "model_version": model_version,
            "artifact_uri": artifact_uri
        },
        "schedule": {
            "cron": cron
        },
        "jobs": [
            {
                "job_kind": "shadow_traffic_evaluation",
                "input": "live_routing_and_qa_outcomes",
                "output_ref": "model_shadow_reports:<shadow_report_uri>",
                "shadow_report_uri": format!("{artifact_dir}/shadow_report.json")
            },
            {
                "job_kind": "drift_monitoring",
                "input": "scoring_features_and_scores",
                "output_ref": "model_drift_reports:<drift_report_uri>",
                "drift_report_uri": format!("{artifact_dir}/drift_report.json")
            },
            {
                "job_kind": "segment_fairness_review",
                "input": "customer_approved_segments",
                "output_ref": "model_fairness_reports:<fairness_report_uri>",
                "fairness_report_uri": format!("{artifact_dir}/fairness_report.json")
            },
            {
                "job_kind": "reviewer_disagreement_review",
                "input": "qa_reviews_and_investigation_outcomes",
                "output_ref": "model_reviewer_disagreement_reports:<reviewer_disagreement_report_uri>",
                "reviewer_disagreement_report_uri": format!("{artifact_dir}/reviewer_disagreement_report.json")
            },
            {
                "job_kind": "label_delay_review",
                "input": "scoring_runs_and_outcome_label_timestamps",
                "output_ref": "model_label_delay_reports:<label_delay_report_uri>",
                "label_delay_report_uri": format!("{artifact_dir}/label_delay_report.json")
            }
        ]
    }))
}

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
                "job_kind": "feature_distribution_psi",
                "input": "monthly_claim_amount_peer_percentile_distribution",
                "feature": "claim_amount_peer_percentile",
                "bucket_count": 10,
                "thresholds": {
                    "stable_below": 0.10,
                    "watch_below": 0.25
                },
                "output_ref": "feature_distribution_psi_reports:<feature_psi_report_uri>",
                "feature_psi_report_uri": format!("{artifact_dir}/feature_psi_report.json")
            },
            {
                "job_kind": "rule_hit_rate_trend",
                "input": "daily_rule_hit_rates",
                "alert_condition": "hit_rate_7d < 0.5 * hit_rate_90d",
                "output_ref": "rule_drift_reports:<rule_hit_rate_report_uri>",
                "rule_hit_rate_report_uri": format!("{artifact_dir}/rule_hit_rate_report.json")
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

pub fn compute_psi(baseline: &[f64], current: &[f64], n_bins: usize) -> f64 {
    if baseline.is_empty() || current.is_empty() || n_bins == 0 {
        return 0.0;
    }
    let mut baseline_counts = vec![0_u32; n_bins];
    let mut current_counts = vec![0_u32; n_bins];
    for value in baseline {
        baseline_counts[psi_bucket(*value, n_bins)] += 1;
    }
    for value in current {
        current_counts[psi_bucket(*value, n_bins)] += 1;
    }

    let baseline_total = baseline.len() as f64;
    let current_total = current.len() as f64;
    let epsilon = 1.0e-6_f64;
    baseline_counts
        .iter()
        .zip(current_counts.iter())
        .map(|(baseline_count, current_count)| {
            let expected = (*baseline_count as f64 / baseline_total).max(epsilon);
            let actual = (*current_count as f64 / current_total).max(epsilon);
            (actual - expected) * (actual / expected).ln()
        })
        .sum()
}

fn psi_bucket(value: f64, n_bins: usize) -> usize {
    let clamped = value.clamp(0.0, 100.0);
    let bucket_width = 100.0 / n_bins as f64;
    ((clamped / bucket_width).floor() as usize).min(n_bins - 1)
}

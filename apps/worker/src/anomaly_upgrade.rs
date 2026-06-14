use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

use crate::{read_json_report, write_json};

const MIN_CONFIRMED_FWA_LABELS: u64 = 500;
const MIN_RECALL_30D: f64 = 0.70;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyUpgradeReadinessInput {
    pub as_of_date: String,
    pub confirmed_fwa_label_count: u64,
    pub total_labeled_claim_count: Option<u64>,
    pub anomaly_recall_30d: Option<f64>,
    pub current_detector: Option<String>,
    pub label_source_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyUpgradeReviewTask {
    pub task_kind: String,
    pub priority: String,
    pub reason: String,
    pub decision_options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyUpgradeReadinessReport {
    pub report_kind: String,
    pub report_version: u8,
    pub as_of_date: String,
    pub source_uri: String,
    pub current_detector: String,
    pub confirmed_fwa_label_count: u64,
    pub total_labeled_claim_count: Option<u64>,
    pub minimum_confirmed_fwa_labels: u64,
    pub anomaly_recall_30d: Option<f64>,
    pub recall_review_threshold: f64,
    pub label_threshold_met: bool,
    pub recall_below_threshold: bool,
    pub readiness_status: String,
    pub recommended_actions: Vec<String>,
    pub review_tasks: Vec<AnomalyUpgradeReviewTask>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

pub fn build_anomaly_upgrade_readiness_report(
    source_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<AnomalyUpgradeReadinessReport> {
    let input: AnomalyUpgradeReadinessInput = serde_json::from_value(read_json_report(source_uri)?)
        .context("parse anomaly upgrade readiness input")?;
    if input.as_of_date.trim().is_empty() {
        bail!("anomaly upgrade readiness input requires as_of_date");
    }
    if let Some(recall) = input.anomaly_recall_30d {
        if !(0.0..=1.0).contains(&recall) {
            bail!("anomaly_recall_30d must be between 0 and 1");
        }
    }

    let label_threshold_met = input.confirmed_fwa_label_count >= MIN_CONFIRMED_FWA_LABELS;
    let recall_below_threshold = input
        .anomaly_recall_30d
        .is_some_and(|recall| recall < MIN_RECALL_30D);
    let current_detector = input
        .current_detector
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("heuristic_l3_baseline")
        .to_string();

    let mut recommended_actions = Vec::new();
    let mut review_tasks = Vec::new();
    let readiness_status = if label_threshold_met {
        recommended_actions.push("prepare_iqr_mad_statistical_baseline_evaluation".into());
        recommended_actions.push("compare_statistical_baseline_against_current_l3_detector".into());
        let priority = if recall_below_threshold {
            "high"
        } else {
            "medium"
        };
        review_tasks.push(AnomalyUpgradeReviewTask {
            task_kind: "l3_anomaly_upgrade_review".into(),
            priority: priority.into(),
            reason: if recall_below_threshold {
                "confirmed FWA label threshold is met and 30-day anomaly recall is below threshold"
                    .into()
            } else {
                "confirmed FWA label threshold is met for L3 statistical baseline evaluation".into()
            },
            decision_options: vec![
                "open_iqr_mad_evaluation".into(),
                "continue_heuristic_baseline".into(),
                "request_label_quality_review".into(),
            ],
        });
        "ready_for_statistical_baseline_evaluation"
    } else {
        recommended_actions.push("continue_confirmed_fwa_label_collection".into());
        if recall_below_threshold {
            recommended_actions.push("review_low_recall_before_threshold_met".into());
        }
        "insufficient_confirmed_fwa_labels"
    };

    let mut evidence_refs = vec![format!("anomaly_upgrade_readiness_input:{source_uri}")];
    if let Some(label_source_uri) = input
        .label_source_uri
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        evidence_refs.push(format!("confirmed_fwa_labels:{label_source_uri}"));
    }

    let report = AnomalyUpgradeReadinessReport {
        report_kind: "anomaly_upgrade_readiness_report".into(),
        report_version: 1,
        as_of_date: input.as_of_date,
        source_uri: source_uri.into(),
        current_detector,
        confirmed_fwa_label_count: input.confirmed_fwa_label_count,
        total_labeled_claim_count: input.total_labeled_claim_count,
        minimum_confirmed_fwa_labels: MIN_CONFIRMED_FWA_LABELS,
        anomaly_recall_30d: input.anomaly_recall_30d,
        recall_review_threshold: MIN_RECALL_30D,
        label_threshold_met,
        recall_below_threshold,
        readiness_status: readiness_status.into(),
        recommended_actions,
        review_tasks,
        evidence_refs,
        governance_boundary: "readiness report may open L3 anomaly review tasks only; it must not replace scoring logic, assign fraud labels, or activate statistical anomaly models".into(),
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create anomaly upgrade readiness output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("anomaly_upgrade_readiness_report.json"),
        &report,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("anomaly_upgrade_review_tasks.json"),
        &report.review_tasks,
    )?;
    Ok(report)
}

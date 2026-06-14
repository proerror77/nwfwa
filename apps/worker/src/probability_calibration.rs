use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

use crate::{api_url, read_json_report, required_non_empty, round4, write_json};

const DEFAULT_BIN_COUNT: usize = 10;
const MIN_CALIBRATION_ROWS: usize = 100;
const MAX_EXPECTED_CALIBRATION_ERROR: f64 = 0.05;
const MAX_BRIER_SCORE: f64 = 0.20;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityCalibrationInput {
    pub model_key: String,
    pub model_version: String,
    pub as_of_date: String,
    pub label_source_uri: Option<String>,
    #[serde(default)]
    pub rows: Vec<ProbabilityCalibrationRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityCalibrationRow {
    pub observation_id: String,
    pub predicted_probability: f64,
    pub actual_label: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProbabilityCalibrationBin {
    pub bin_index: usize,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub row_count: usize,
    pub average_predicted_probability: f64,
    pub observed_positive_rate: f64,
    pub calibration_error: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProbabilityCalibrationReviewTask {
    pub task_kind: String,
    pub priority: String,
    pub reason: String,
    pub decision_options: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityCalibrationReport {
    pub report_kind: String,
    pub report_version: u8,
    pub model_key: String,
    pub model_version: String,
    pub as_of_date: String,
    pub source_uri: String,
    pub label_source_uri: Option<String>,
    pub row_count: usize,
    pub minimum_calibration_rows: usize,
    pub bin_count: usize,
    pub expected_calibration_error: f64,
    pub max_expected_calibration_error: f64,
    pub brier_score: f64,
    pub max_brier_score: f64,
    pub calibration_status: String,
    pub bins: Vec<ProbabilityCalibrationBin>,
    pub review_tasks: Vec<ProbabilityCalibrationReviewTask>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityCalibrationSubmission {
    pub actor: String,
    pub notes: String,
    pub report_uri: String,
    pub report_kind: String,
    pub model_version: String,
    pub as_of_date: String,
    pub row_count: usize,
    pub minimum_calibration_rows: usize,
    pub bin_count: usize,
    pub expected_calibration_error: f64,
    pub max_expected_calibration_error: f64,
    pub brier_score: f64,
    pub max_brier_score: f64,
    pub calibration_status: String,
    pub bins: Vec<ProbabilityCalibrationBin>,
    pub review_tasks: Vec<ProbabilityCalibrationReviewTask>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

pub fn build_probability_calibration_report(
    source_uri: &str,
    output_dir: impl AsRef<Path>,
    bin_count: Option<usize>,
) -> anyhow::Result<ProbabilityCalibrationReport> {
    let bin_count = bin_count.unwrap_or(DEFAULT_BIN_COUNT);
    if bin_count == 0 {
        bail!("bin_count must be greater than zero");
    }
    let input: ProbabilityCalibrationInput = serde_json::from_value(read_json_report(source_uri)?)
        .context("parse probability calibration input")?;
    if input.model_key.trim().is_empty() {
        bail!("probability calibration input requires model_key");
    }
    if input.model_version.trim().is_empty() {
        bail!("probability calibration input requires model_version");
    }
    if input.as_of_date.trim().is_empty() {
        bail!("probability calibration input requires as_of_date");
    }
    let label_source_uri = input
        .label_source_uri
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| anyhow::anyhow!("probability calibration input requires label_source_uri"))?
        .to_string();
    if input.rows.is_empty() {
        bail!("probability calibration input requires rows");
    }

    for row in &input.rows {
        if row.observation_id.trim().is_empty() {
            bail!("probability calibration row missing observation_id");
        }
        if !(0.0..=1.0).contains(&row.predicted_probability) {
            bail!(
                "predicted_probability for {} must be between 0 and 1",
                row.observation_id
            );
        }
        if row.actual_label > 1 {
            bail!("actual_label for {} must be 0 or 1", row.observation_id);
        }
    }

    let bins = calibration_bins(&input.rows, bin_count);
    let expected_calibration_error = round4(
        bins.iter()
            .map(|bin| bin.calibration_error * bin.row_count as f64 / input.rows.len() as f64)
            .sum::<f64>(),
    );
    let brier_score = round4(
        input
            .rows
            .iter()
            .map(|row| {
                let label = f64::from(row.actual_label);
                (row.predicted_probability - label).powi(2)
            })
            .sum::<f64>()
            / input.rows.len() as f64,
    );

    let mut review_tasks = Vec::new();
    let calibration_status = if input.rows.len() < MIN_CALIBRATION_ROWS {
        review_tasks.push(calibration_review_task(
            "medium",
            "sample count is below minimum calibration evidence threshold",
        ));
        "insufficient_sample"
    } else if expected_calibration_error > MAX_EXPECTED_CALIBRATION_ERROR
        || brier_score > MAX_BRIER_SCORE
    {
        review_tasks.push(calibration_review_task(
            "high",
            "probability calibration metrics exceed review thresholds",
        ));
        "needs_calibration_review"
    } else {
        "passed"
    };

    let evidence_refs = vec![
        format!("probability_calibration_input:{source_uri}"),
        format!("calibration_labels:{label_source_uri}"),
    ];

    let report = ProbabilityCalibrationReport {
        report_kind: "probability_calibration_report".into(),
        report_version: 1,
        model_key: input.model_key.trim().into(),
        model_version: input.model_version.trim().into(),
        as_of_date: input.as_of_date.trim().into(),
        source_uri: source_uri.into(),
        label_source_uri: Some(label_source_uri),
        row_count: input.rows.len(),
        minimum_calibration_rows: MIN_CALIBRATION_ROWS,
        bin_count,
        expected_calibration_error,
        max_expected_calibration_error: MAX_EXPECTED_CALIBRATION_ERROR,
        brier_score,
        max_brier_score: MAX_BRIER_SCORE,
        calibration_status: calibration_status.into(),
        bins,
        review_tasks,
        evidence_refs,
        governance_boundary: "calibration report is evidence only; it must not relabel outcomes, rewrite model probabilities, change routing thresholds, or activate calibrated serving".into(),
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create probability calibration output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("probability_calibration_report.json"),
        &report,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("probability_calibration_bins.json"),
        &report.bins,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("probability_calibration_review_tasks.json"),
        &report.review_tasks,
    )?;
    Ok(report)
}

pub fn build_probability_calibration_submission(
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<(String, ProbabilityCalibrationSubmission)> {
    let report_uri = required_non_empty("report_uri", report_uri)?;
    let actor = required_non_empty("actor", actor)?;
    let notes = required_non_empty("notes", notes)?;
    let report: ProbabilityCalibrationReport =
        serde_json::from_value(read_json_report(report_uri)?)
            .context("parse probability calibration report")?;
    if report.report_kind != "probability_calibration_report" {
        bail!("report_kind must be probability_calibration_report");
    }
    for required_prefix in ["probability_calibration_input:", "calibration_labels:"] {
        if !report
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().starts_with(required_prefix))
        {
            bail!("probability calibration report requires {required_prefix} evidence");
        }
    }
    let mut evidence_refs = report.evidence_refs;
    evidence_refs.push(format!(
        "model_versions:{}:{}",
        report.model_key, report.model_version
    ));
    evidence_refs.push(format!("probability_calibration_reports:{report_uri}"));
    evidence_refs.sort();
    evidence_refs.dedup();
    let model_key = report.model_key.clone();
    Ok((
        model_key,
        ProbabilityCalibrationSubmission {
            actor: actor.into(),
            notes: notes.into(),
            report_uri: report_uri.into(),
            report_kind: report.report_kind,
            model_version: report.model_version,
            as_of_date: report.as_of_date,
            row_count: report.row_count,
            minimum_calibration_rows: report.minimum_calibration_rows,
            bin_count: report.bin_count,
            expected_calibration_error: report.expected_calibration_error,
            max_expected_calibration_error: report.max_expected_calibration_error,
            brier_score: report.brier_score,
            max_brier_score: report.max_brier_score,
            calibration_status: report.calibration_status,
            bins: report.bins,
            review_tasks: report.review_tasks,
            evidence_refs,
            governance_boundary: report.governance_boundary,
        },
    ))
}

pub async fn submit_probability_calibration_report(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    let (model_key, payload) = build_probability_calibration_submission(report_uri, actor, notes)?;
    let response = reqwest::Client::new()
        .post(api_url(
            api_base_url,
            &format!("/api/v1/ops/models/{model_key}/probability-calibration-reports"),
        ))
        .header("x-api-key", api_key)
        .json(&payload)
        .send()
        .await
        .context("submit probability calibration report")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("submit probability calibration report failed with {status}: {body}");
    }
    response
        .json::<serde_json::Value>()
        .await
        .context("parse probability calibration response")
}

fn calibration_bins(
    rows: &[ProbabilityCalibrationRow],
    bin_count: usize,
) -> Vec<ProbabilityCalibrationBin> {
    (0..bin_count)
        .map(|bin_index| {
            let lower = bin_index as f64 / bin_count as f64;
            let upper = (bin_index + 1) as f64 / bin_count as f64;
            let bin_rows = rows
                .iter()
                .filter(|row| {
                    let probability = row.predicted_probability;
                    if bin_index + 1 == bin_count {
                        probability >= lower && probability <= upper
                    } else {
                        probability >= lower && probability < upper
                    }
                })
                .collect::<Vec<_>>();
            let row_count = bin_rows.len();
            let average_predicted_probability = if row_count == 0 {
                0.0
            } else {
                bin_rows
                    .iter()
                    .map(|row| row.predicted_probability)
                    .sum::<f64>()
                    / row_count as f64
            };
            let observed_positive_rate = if row_count == 0 {
                0.0
            } else {
                bin_rows
                    .iter()
                    .map(|row| f64::from(row.actual_label))
                    .sum::<f64>()
                    / row_count as f64
            };
            ProbabilityCalibrationBin {
                bin_index,
                lower_bound: round4(lower),
                upper_bound: round4(upper),
                row_count,
                average_predicted_probability: round4(average_predicted_probability),
                observed_positive_rate: round4(observed_positive_rate),
                calibration_error: round4(
                    (average_predicted_probability - observed_positive_rate).abs(),
                ),
            }
        })
        .collect()
}

fn calibration_review_task(priority: &str, reason: &str) -> ProbabilityCalibrationReviewTask {
    ProbabilityCalibrationReviewTask {
        task_kind: "probability_calibration_review".into(),
        priority: priority.into(),
        reason: reason.into(),
        decision_options: vec![
            "collect_more_labels".into(),
            "fit_calibration_model".into(),
            "keep_probabilities_uncalibrated".into(),
        ],
    }
}

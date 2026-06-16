use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersionRecord {
    pub model_key: String,
    pub version: String,
    pub model_type: String,
    pub runtime_kind: String,
    pub execution_provider: String,
    pub status: String,
    pub review_mode: String,
    pub artifact_uri: Option<String>,
    pub endpoint_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPerformanceRecord {
    pub model_key: String,
    pub data_status: String,
    pub scored_runs: u32,
    pub average_score: f64,
    pub high_risk_count: u32,
    pub score_psi: Option<f64>,
    pub drift_status: String,
    pub latest_scored_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPromotionReviewRecord {
    pub model_key: String,
    pub model_version: String,
    pub decision: String,
    pub reviewer: String,
    pub notes: String,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbabilityCalibrationReportRecord {
    pub model_key: String,
    pub model_version: String,
    pub report_uri: String,
    pub report_kind: String,
    pub as_of_date: String,
    pub row_count: usize,
    pub minimum_calibration_rows: usize,
    pub bin_count: usize,
    pub expected_calibration_error: f64,
    pub max_expected_calibration_error: f64,
    pub brier_score: f64,
    pub max_brier_score: f64,
    pub calibration_status: String,
    pub bins_json: Value,
    pub review_tasks_json: Value,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
    pub submitted_by: String,
    pub notes: String,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRetrainingJobRecord {
    pub job_id: String,
    pub model_key: String,
    pub model_version: String,
    pub status: String,
    pub requested_by: String,
    pub request_notes: String,
    pub status_note: String,
    pub updated_by: String,
    pub readiness_recommendation: String,
    pub latest_evaluation_id: String,
    pub source_dataset_id: String,
    pub source_data_quality_score: Option<f64>,
    pub source_data_quality_status: String,
    pub trigger_summary: Vec<String>,
    pub blocker_summary: Vec<String>,
    pub candidate_model_version: Option<String>,
    pub candidate_artifact_uri: Option<String>,
    pub candidate_endpoint_url: Option<String>,
    pub validation_report_uri: Option<String>,
    pub output_evaluation_id: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompleteModelRetrainingJobInput<'a> {
    pub job_id: &'a str,
    pub actor: &'a str,
    pub status_note: &'a str,
    pub candidate_model_version: &'a str,
    pub candidate_artifact_uri: &'a str,
    pub candidate_endpoint_url: Option<&'a str>,
    pub validation_report_uri: &'a str,
    pub output_evaluation_id: &'a str,
}

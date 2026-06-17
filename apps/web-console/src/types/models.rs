use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ModelListResponse {
    pub(crate) models: Vec<ModelVersion>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ModelVersion {
    pub(crate) model_key: String,
    pub(crate) version: String,
    pub(crate) model_type: String,
    pub(crate) runtime_kind: String,
    pub(crate) execution_provider: String,
    pub(crate) status: String,
    pub(crate) review_mode: String,
    pub(crate) artifact_uri: Option<String>,
    pub(crate) endpoint_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ModelPerformance {
    pub(crate) model_key: String,
    pub(crate) data_status: String,
    pub(crate) scored_runs: u32,
    pub(crate) average_score: f64,
    pub(crate) high_risk_count: u32,
    pub(crate) score_psi: Option<f64>,
    pub(crate) drift_status: String,
    pub(crate) latest_scored_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ModelPromotionGates {
    pub(crate) model_key: String,
    pub(crate) model_version: String,
    pub(crate) decision: String,
    pub(crate) passed_count: u32,
    pub(crate) total_count: u32,
    pub(crate) latest_evaluation_id: String,
    pub(crate) source_data_quality_status: String,
    pub(crate) unresolved_model_feedback_count: u32,
    pub(crate) approved_label_count: u32,
    pub(crate) artifact_evidence: ModelArtifactEvidence,
    pub(crate) gates: Vec<ModelPromotionGate>,
    pub(crate) blockers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ModelArtifactEvidence {
    pub(crate) serving_manifest_uri: Option<String>,
    pub(crate) model_artifact_evaluation_report_uri: Option<String>,
    pub(crate) rust_serving_status: Option<String>,
    pub(crate) rust_serving_latency_status: Option<String>,
    pub(crate) rust_serving_p95_latency_ms: Option<u64>,
    pub(crate) rust_serving_latency_measurement_kind: Option<String>,
    pub(crate) rust_serving_latency_sample_count: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ModelPromotionGate {
    pub(crate) label: String,
    pub(crate) passed: bool,
    pub(crate) blocker: String,
    pub(crate) evidence_source: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ModelRetrainingReadiness {
    pub(crate) recommendation: String,
    pub(crate) drift_status: String,
    pub(crate) source_data_quality_status: String,
    pub(crate) open_model_feedback_count: u32,
    pub(crate) approved_label_count: u32,
    pub(crate) needs_review_label_count: u32,
    pub(crate) retraining_triggers: Vec<String>,
    pub(crate) blockers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ModelOpsSnapshot {
    pub(crate) models: Vec<ModelVersion>,
    pub(crate) performance: ModelPerformance,
    pub(crate) gates: ModelPromotionGates,
    pub(crate) retraining: ModelRetrainingReadiness,
}

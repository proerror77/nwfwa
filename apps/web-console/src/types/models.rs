use super::DataSourcesSnapshot;
use serde::Deserialize;
use serde_json::Value;

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

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ModelRetrainingJobListResponse {
    pub(crate) jobs: Vec<ModelRetrainingJobRecord>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ModelRetrainingJobRecord {
    pub(crate) job_id: String,
    pub(crate) model_key: String,
    pub(crate) model_version: String,
    pub(crate) status: String,
    pub(crate) requested_by: String,
    pub(crate) request_notes: String,
    pub(crate) status_note: String,
    pub(crate) updated_by: String,
    pub(crate) readiness_recommendation: String,
    pub(crate) latest_evaluation_id: String,
    pub(crate) source_dataset_id: String,
    pub(crate) source_data_quality_score: Option<f64>,
    pub(crate) source_data_quality_status: String,
    pub(crate) trigger_summary: Vec<String>,
    pub(crate) blocker_summary: Vec<String>,
    pub(crate) candidate_model_version: Option<String>,
    pub(crate) candidate_artifact_uri: Option<String>,
    pub(crate) candidate_endpoint_url: Option<String>,
    pub(crate) validation_report_uri: Option<String>,
    pub(crate) output_evaluation_id: Option<String>,
    pub(crate) created_at: Option<String>,
    pub(crate) updated_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ModelMonitoringReviewQueueResponse {
    pub(crate) tasks: Vec<ModelMonitoringReviewTask>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct ModelMonitoringReviewTask {
    pub(crate) task_id: String,
    pub(crate) audit_id: String,
    pub(crate) model_key: String,
    pub(crate) model_version: String,
    pub(crate) report_uri: String,
    pub(crate) monitoring_status: String,
    pub(crate) retraining_recommendation: String,
    pub(crate) task_kind: String,
    pub(crate) trigger: String,
    pub(crate) review_status: String,
    pub(crate) reviewer: Option<String>,
    pub(crate) review_audit_id: Option<String>,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct MlopsAlertDeliveryQueueResponse {
    pub(crate) tasks: Vec<MlopsAlertDeliveryTask>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct MlopsAlertDeliveryTask {
    pub(crate) task_id: String,
    pub(crate) audit_id: String,
    pub(crate) model_key: String,
    pub(crate) model_version: String,
    pub(crate) scheduler_execution_report_uri: String,
    pub(crate) alert_delivery_status: String,
    pub(crate) task_kind: String,
    pub(crate) trigger: String,
    pub(crate) route_key: String,
    pub(crate) delivery_status: String,
    pub(crate) review_status: String,
    pub(crate) reviewer: Option<String>,
    pub(crate) review_audit_id: Option<String>,
    pub(crate) evidence_refs: Vec<String>,
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AnomalyReviewQueueResponse {
    pub(crate) tasks: Vec<AnomalyReviewQueueTask>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct AnomalyReviewQueueTask {
    pub(crate) candidate_kind: String,
    pub(crate) candidate_id: String,
    pub(crate) task_kind: String,
    pub(crate) review_queue: String,
    pub(crate) required_review: String,
    pub(crate) decision_options: Vec<String>,
    pub(crate) source_report_uri: String,
    pub(crate) report_kind: String,
    pub(crate) dataset_key: String,
    pub(crate) dataset_version: String,
    pub(crate) label_policy: String,
    pub(crate) governance_boundary: String,
    pub(crate) review_status: String,
    pub(crate) reviewer: Option<String>,
    pub(crate) decision: Option<String>,
    pub(crate) candidate_payload: Value,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MlopsWorkspaceSnapshot {
    pub(crate) data_sources: DataSourcesSnapshot,
    pub(crate) model_ops: ModelOpsSnapshot,
    pub(crate) retraining_jobs: Vec<ModelRetrainingJobRecord>,
    pub(crate) monitoring_review_tasks: Vec<ModelMonitoringReviewTask>,
    pub(crate) alert_delivery_tasks: Vec<MlopsAlertDeliveryTask>,
    pub(crate) anomaly_review_tasks: Vec<AnomalyReviewQueueTask>,
}

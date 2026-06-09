use crate::repository::{
    ModelEvaluationRecord, ModelRetrainingJobRecord, ModelVersionRecord, RuleDetailRecord,
};
use fwa_rules::Rule;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct ModelListResponse {
    pub models: Vec<ModelVersionRecord>,
}

#[derive(Debug, Serialize)]
pub struct ModelPromotionGate {
    pub label: String,
    pub passed: bool,
    pub blocker: String,
    pub evidence_source: String,
}

#[derive(Debug, Serialize)]
pub struct ModelPromotionGatesResponse {
    pub model_key: String,
    pub model_version: String,
    pub review_mode: String,
    pub decision: String,
    pub passed_count: usize,
    pub total_count: usize,
    pub latest_evaluation_id: String,
    pub source_dataset_id: String,
    pub source_data_quality_score: Option<f64>,
    pub source_data_quality_status: String,
    pub data_status: String,
    pub scored_runs: u32,
    pub open_model_feedback_count: usize,
    pub unresolved_model_feedback_count: usize,
    pub approved_label_count: usize,
    pub needs_review_label_count: usize,
    pub artifact_evidence: ModelArtifactEvidenceSummary,
    pub gates: Vec<ModelPromotionGate>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ModelArtifactEvidenceSummary {
    pub serving_manifest_uri: Option<String>,
    pub model_artifact_evaluation_report_uri: Option<String>,
    pub permutation_importance_uri: Option<String>,
    pub rust_serving_status: Option<String>,
    pub rust_serving_latency_status: Option<String>,
    pub rust_serving_p95_latency_ms: Option<u64>,
    pub rust_serving_latency_measurement_kind: Option<String>,
    pub rust_serving_latency_sample_count: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct ModelRetrainingReadinessResponse {
    pub model_key: String,
    pub model_version: String,
    pub recommendation: String,
    pub latest_evaluation_id: String,
    pub drift_status: String,
    pub source_dataset_id: String,
    pub source_data_quality_score: Option<f64>,
    pub source_data_quality_status: String,
    pub open_model_feedback_count: usize,
    pub approved_label_count: usize,
    pub needs_review_label_count: usize,
    pub retraining_triggers: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ModelRetrainingJobListResponse {
    pub jobs: Vec<ModelRetrainingJobRecord>,
}

#[derive(Debug, Serialize)]
pub struct ModelMonitoringReviewQueueResponse {
    pub tasks: Vec<ModelMonitoringReviewTask>,
}

#[derive(Debug, Serialize)]
pub struct ModelMonitoringReviewTask {
    pub task_id: String,
    pub audit_id: String,
    pub model_key: String,
    pub model_version: String,
    pub report_uri: String,
    pub monitoring_status: String,
    pub retraining_recommendation: String,
    pub task_kind: String,
    pub trigger: String,
    pub review_status: String,
    pub reviewer: Option<String>,
    pub review_audit_id: Option<String>,
    pub task: Value,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitModelMonitoringReviewTaskReviewRequest {
    pub decision: String,
    pub reviewer: String,
    pub notes: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ModelMonitoringReviewTaskReviewResponse {
    pub task_id: String,
    pub model_key: String,
    pub model_version: String,
    pub decision: String,
    pub reviewer: String,
    pub governance_boundary: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitModelPromotionReviewRequest {
    pub decision: String,
    pub reviewer: String,
    pub notes: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ModelLifecycleRequest {
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateModelRetrainingJobRequest {
    pub requested_by: String,
    pub notes: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitMlopsMonitoringReportRequest {
    pub actor: String,
    pub notes: String,
    pub report_uri: String,
    pub report_kind: String,
    pub model_version: String,
    pub overall_status: String,
    pub retraining_recommendation: String,
    pub triggers: Vec<String>,
    pub review_tasks: Vec<Value>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SubmitMlopsMonitoringReportResponse {
    pub model_key: String,
    pub model_version: String,
    pub report_uri: String,
    pub monitoring_status: String,
    pub retraining_recommendation: String,
    pub trigger_count: usize,
    pub review_task_count: usize,
    pub next_actions: Vec<String>,
    pub governance_boundary: String,
}

#[derive(Debug, Deserialize)]
pub struct SubmitMlopsAlertDeliveryRequest {
    pub actor: String,
    pub notes: String,
    pub scheduler_execution_report_uri: String,
    pub report_kind: String,
    pub model_version: String,
    pub alert_delivery_status: String,
    pub alert_delivery_tasks: Vec<Value>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SubmitMlopsAlertDeliveryResponse {
    pub model_key: String,
    pub model_version: String,
    pub scheduler_execution_report_uri: String,
    pub alert_delivery_status: String,
    pub alert_delivery_task_count: usize,
    pub alert_routing_policy_configured: bool,
    pub next_actions: Vec<String>,
    pub governance_boundary: String,
}

#[derive(Debug, Serialize)]
pub struct MlopsAlertDeliveryQueueResponse {
    pub tasks: Vec<MlopsAlertDeliveryTask>,
}

#[derive(Debug, Serialize)]
pub struct MlopsAlertDeliveryTask {
    pub task_id: String,
    pub audit_id: String,
    pub model_key: String,
    pub model_version: String,
    pub scheduler_execution_report_uri: String,
    pub alert_delivery_status: String,
    pub task_kind: String,
    pub trigger: String,
    pub route_key: String,
    pub delivery_status: String,
    pub review_status: String,
    pub reviewer: Option<String>,
    pub review_audit_id: Option<String>,
    pub task: Value,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitMlopsAlertDeliveryTaskReviewRequest {
    pub decision: String,
    pub reviewer: String,
    pub notes: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct MlopsAlertDeliveryTaskReviewResponse {
    pub task_id: String,
    pub model_key: String,
    pub model_version: String,
    pub decision: String,
    pub reviewer: String,
    pub governance_boundary: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateModelRetrainingJobStatusRequest {
    pub status: String,
    pub actor: String,
    pub notes: String,
}

#[derive(Debug, Deserialize)]
pub struct ClaimModelRetrainingJobRequest {
    pub actor: String,
    pub notes: String,
    pub model_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompleteModelRetrainingJobRequest {
    pub actor: String,
    pub notes: String,
    pub candidate_model_version: String,
    pub artifact_uri: String,
    pub artifact_sha256: Option<String>,
    pub training_artifact_uri: Option<String>,
    pub training_artifact_sha256: Option<String>,
    pub serving_manifest_uri: Option<String>,
    pub endpoint_url: Option<String>,
    pub validation_report_uri: String,
    pub evaluation_run_id: String,
    pub evidence_refs: Vec<String>,
    pub auc: Option<Decimal>,
    pub ks: Option<Decimal>,
    pub precision: Option<Decimal>,
    pub recall: Option<Decimal>,
    pub f1: Option<Decimal>,
    pub accuracy: Option<Decimal>,
    pub threshold: Option<Decimal>,
    pub confusion_matrix_json: Value,
    pub feature_importance_uri: Option<String>,
    pub permutation_importance_uri: Option<String>,
    pub metrics_json: Value,
    pub mined_rule_owner: Option<String>,
    pub mined_rule_candidates: Option<Vec<Rule>>,
}

#[derive(Debug, Serialize)]
pub struct CompleteModelRetrainingJobResponse {
    pub job: ModelRetrainingJobRecord,
    pub candidate_model: ModelVersionRecord,
    pub evaluation: ModelEvaluationRecord,
    pub mined_rule_candidates: Vec<RuleDetailRecord>,
}

#[derive(Debug, Serialize)]
pub struct ModelLifecycleResponse {
    pub model_key: String,
    pub version: String,
    pub status: String,
}

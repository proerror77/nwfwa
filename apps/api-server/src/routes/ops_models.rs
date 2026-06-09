use super::ops_models_audit::{
    record_mlops_alert_delivery_audit, record_mlops_alert_delivery_task_review_audit,
    record_mlops_monitoring_audit, record_mlops_monitoring_review_task_audit,
    record_model_activation_audit, record_model_promotion_audit, record_model_retraining_audit,
    record_model_retraining_output_audit, record_model_rollback_audit,
    save_training_package_rule_candidates,
};
use super::ops_models_gates::{
    activation_blockers, alert_delivery_tasks_from_events, build_mlops_alert_delivery_response,
    build_mlops_monitoring_report_response, build_model_promotion_gates,
    build_model_retraining_readiness, monitoring_review_tasks_from_events,
};
use super::ops_models_validation::{
    retraining_metrics_with_artifacts, validate_alert_delivery_evidence,
    validate_alert_delivery_task_evidence, validate_alert_delivery_task_review_request,
    validate_mlops_alert_delivery_request, validate_mlops_monitoring_report_request,
    validate_monitoring_report_evidence, validate_monitoring_review_task_evidence,
    validate_monitoring_review_task_review_request, validate_retraining_notes_without_pii,
    validate_retraining_output_request,
};
use crate::{
    app::AppState,
    auth::{AuthenticatedActor, AuthenticatedApiPrincipal},
    error::ApiError,
    repository::{
        AuditEventListFilter, CompleteModelRetrainingJobInput, ModelEvaluationRecord,
        ModelPerformanceRecord, ModelPromotionReviewRecord, ModelRetrainingJobRecord,
        ModelVersionRecord, RegisterModelEvaluationInput, RuleDetailRecord,
    },
    routes::pii,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use fwa_audit::ActorContext;
use fwa_auth::AuthenticatedPrincipal;
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

pub async fn list_models(
    State(state): State<AppState>,
    _actor: AuthenticatedActor,
) -> Result<Json<ModelListResponse>, ApiError> {
    let models = state
        .repository
        .list_models()
        .await
        .map_err(internal_error("MODEL_LIST_FAILED"))?;
    Ok(Json(ModelListResponse { models }))
}

pub async fn model_performance(
    State(state): State<AppState>,
    _actor: AuthenticatedActor,
    Path(model_key): Path<String>,
) -> Result<Json<crate::repository::ModelPerformanceRecord>, ApiError> {
    let performance = state
        .repository
        .model_performance(&model_key)
        .await
        .map_err(internal_error("MODEL_PERFORMANCE_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "MODEL_NOT_FOUND", "model not found")
        })?;
    Ok(Json(performance))
}

pub async fn model_promotion_gates(
    State(state): State<AppState>,
    _actor: AuthenticatedActor,
    Path(model_key): Path<String>,
) -> Result<Json<ModelPromotionGatesResponse>, ApiError> {
    let (_, gates) = load_model_promotion_gates(&state, &model_key).await?;
    Ok(Json(gates))
}

pub async fn model_version_promotion_gates(
    State(state): State<AppState>,
    _actor: AuthenticatedActor,
    Path((model_key, model_version)): Path<(String, String)>,
) -> Result<Json<ModelPromotionGatesResponse>, ApiError> {
    let (_, gates) =
        load_model_promotion_gates_for_version(&state, &model_key, &model_version).await?;
    Ok(Json(gates))
}

pub async fn model_retraining_readiness(
    State(state): State<AppState>,
    _actor: AuthenticatedActor,
    Path(model_key): Path<String>,
) -> Result<Json<ModelRetrainingReadinessResponse>, ApiError> {
    Ok(Json(
        load_model_retraining_readiness(&state, &model_key).await?,
    ))
}

pub async fn list_model_retraining_jobs(
    State(state): State<AppState>,
    _actor: AuthenticatedActor,
    Path(model_key): Path<String>,
) -> Result<Json<ModelRetrainingJobListResponse>, ApiError> {
    ensure_model_exists(&state, &model_key).await?;
    let jobs = state
        .repository
        .list_model_retraining_jobs(&model_key)
        .await
        .map_err(internal_error("MODEL_RETRAINING_JOB_LIST_FAILED"))?;
    Ok(Json(ModelRetrainingJobListResponse { jobs }))
}

pub async fn model_monitoring_review_queue(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(model_key): Path<String>,
) -> Result<Json<ModelMonitoringReviewQueueResponse>, ApiError> {
    ensure_model_exists(&state, &model_key).await?;
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 50,
            event_type: Some("model.mlops_monitoring.report_submitted".into()),
            model_key: Some(model_key.clone()),
            customer_scope_id: Some(actor.customer_scope_id.clone()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("MODEL_MONITORING_REVIEW_QUEUE_LIST_FAILED"))?;
    let review_events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 100,
            event_type: Some("model.mlops_monitoring.review_task_reviewed".into()),
            model_key: Some(model_key),
            customer_scope_id: Some(actor.customer_scope_id),
            ..Default::default()
        })
        .await
        .map_err(internal_error("MODEL_MONITORING_REVIEW_QUEUE_LIST_FAILED"))?;

    Ok(Json(ModelMonitoringReviewQueueResponse {
        tasks: monitoring_review_tasks_from_events(events, review_events),
    }))
}

pub async fn submit_model_monitoring_review_task_review(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path((model_key, task_id)): Path<(String, String)>,
    Json(request): Json<SubmitModelMonitoringReviewTaskReviewRequest>,
) -> Result<Json<ModelMonitoringReviewTaskReviewResponse>, ApiError> {
    let actor = require_permission(principal, "ops:models:review")?;
    validate_monitoring_review_task_review_request(&request)?;
    ensure_model_exists(&state, &model_key).await?;

    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 50,
            event_type: Some("model.mlops_monitoring.report_submitted".into()),
            model_key: Some(model_key.clone()),
            customer_scope_id: Some(actor.customer_scope_id.clone()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("MODEL_MONITORING_REVIEW_TASK_LIST_FAILED"))?;
    let task = monitoring_review_tasks_from_events(events, Vec::new())
        .into_iter()
        .find(|task| task.task_id == task_id)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MODEL_MONITORING_REVIEW_TASK_NOT_FOUND",
                "MLOps monitoring review task not found",
            )
        })?;

    validate_target_model_version_evidence(
        &request.evidence_refs,
        &task.model_key,
        &task.model_version,
        "MLOps monitoring review task review",
    )?;
    validate_monitoring_review_task_evidence(&request.evidence_refs, &task)?;

    let response = ModelMonitoringReviewTaskReviewResponse {
        task_id: task.task_id.clone(),
        model_key: task.model_key.clone(),
        model_version: task.model_version.clone(),
        decision: request.decision.clone(),
        reviewer: request.reviewer.clone(),
        governance_boundary:
            "monitoring review task decisions record human governance only; they must not auto-create retraining jobs, activate models, rollback models, or assign fraud labels"
                .into(),
    };
    record_mlops_monitoring_review_task_audit(&state, &actor, &task, &request, &response)
        .await
        .map_err(internal_error(
            "MLOPS_MONITORING_REVIEW_TASK_AUDIT_SAVE_FAILED",
        ))?;
    Ok(Json(response))
}

pub async fn create_model_retraining_job(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(model_key): Path<String>,
    Json(request): Json<CreateModelRetrainingJobRequest>,
) -> Result<Json<ModelRetrainingJobRecord>, ApiError> {
    if request.requested_by.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUESTED_BY",
            "requested_by is required",
        ));
    }
    if request.notes.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_JOB_NOTES",
            "retraining job notes are required",
        ));
    }
    validate_retraining_notes_without_pii(&request.notes)?;

    let readiness = load_model_retraining_readiness(&state, &model_key).await?;
    if readiness.recommendation != "prepare_retraining" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "MODEL_RETRAINING_NOT_READY",
            "model retraining can only be queued when readiness recommends prepare_retraining",
        ));
    }

    let job = state
        .repository
        .save_model_retraining_job(ModelRetrainingJobRecord {
            job_id: String::new(),
            model_key: readiness.model_key.clone(),
            model_version: readiness.model_version.clone(),
            status: "queued".into(),
            requested_by: request.requested_by,
            request_notes: request.notes,
            status_note: "queued from readiness".into(),
            updated_by: actor.actor_id.clone(),
            readiness_recommendation: readiness.recommendation,
            latest_evaluation_id: readiness.latest_evaluation_id,
            source_dataset_id: readiness.source_dataset_id,
            source_data_quality_score: readiness.source_data_quality_score,
            source_data_quality_status: readiness.source_data_quality_status,
            trigger_summary: readiness.retraining_triggers,
            blocker_summary: readiness.blockers,
            candidate_model_version: None,
            candidate_artifact_uri: None,
            candidate_endpoint_url: None,
            validation_report_uri: None,
            output_evaluation_id: None,
            created_at: None,
            updated_at: None,
        })
        .await
        .map_err(internal_error("MODEL_RETRAINING_JOB_SAVE_FAILED"))?;
    record_model_retraining_audit(&state, &actor, &job, "model.retraining.queued", &[])
        .await
        .map_err(internal_error("MODEL_RETRAINING_AUDIT_SAVE_FAILED"))?;
    Ok(Json(job))
}

pub async fn submit_mlops_monitoring_report(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(model_key): Path<String>,
    Json(request): Json<SubmitMlopsMonitoringReportRequest>,
) -> Result<Json<SubmitMlopsMonitoringReportResponse>, ApiError> {
    let actor = require_permission(principal, "ops:models:review")?;
    validate_mlops_monitoring_report_request(&request)?;
    let model = state
        .repository
        .list_models()
        .await
        .map_err(internal_error("MODEL_LIST_FAILED"))?
        .into_iter()
        .find(|model| model.model_key == model_key && model.version == request.model_version)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MODEL_VERSION_NOT_FOUND",
                "model version not found",
            )
        })?;
    validate_target_model_version_evidence(
        &request.evidence_refs,
        &model.model_key,
        &model.version,
        "MLOps monitoring report",
    )?;
    validate_monitoring_report_evidence(&request)?;
    let response = build_mlops_monitoring_report_response(&model, &request);
    record_mlops_monitoring_audit(&state, &actor, &model, &request, &response)
        .await
        .map_err(internal_error("MLOPS_MONITORING_AUDIT_SAVE_FAILED"))?;
    Ok(Json(response))
}

pub async fn submit_mlops_alert_delivery(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(model_key): Path<String>,
    Json(request): Json<SubmitMlopsAlertDeliveryRequest>,
) -> Result<Json<SubmitMlopsAlertDeliveryResponse>, ApiError> {
    let actor = require_permission(principal, "ops:models:review")?;
    validate_mlops_alert_delivery_request(&request)?;
    let model = state
        .repository
        .list_models()
        .await
        .map_err(internal_error("MODEL_LIST_FAILED"))?
        .into_iter()
        .find(|model| model.model_key == model_key && model.version == request.model_version)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MODEL_VERSION_NOT_FOUND",
                "model version not found",
            )
        })?;
    validate_target_model_version_evidence(
        &request.evidence_refs,
        &model.model_key,
        &model.version,
        "MLOps alert delivery",
    )?;
    validate_alert_delivery_evidence(&request)?;
    let response = build_mlops_alert_delivery_response(&state, &model, &request);
    record_mlops_alert_delivery_audit(&state, &actor, &model, &request, &response)
        .await
        .map_err(internal_error("MLOPS_ALERT_DELIVERY_AUDIT_SAVE_FAILED"))?;
    Ok(Json(response))
}

pub async fn mlops_alert_delivery_queue(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(model_key): Path<String>,
) -> Result<Json<MlopsAlertDeliveryQueueResponse>, ApiError> {
    ensure_model_exists(&state, &model_key).await?;
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 50,
            event_type: Some("model.mlops_alert_delivery.submitted".into()),
            model_key: Some(model_key.clone()),
            customer_scope_id: Some(actor.customer_scope_id.clone()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("MLOPS_ALERT_DELIVERY_QUEUE_LIST_FAILED"))?;
    let review_events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 100,
            event_type: Some("model.mlops_alert_delivery.task_reviewed".into()),
            model_key: Some(model_key),
            customer_scope_id: Some(actor.customer_scope_id),
            ..Default::default()
        })
        .await
        .map_err(internal_error("MLOPS_ALERT_DELIVERY_QUEUE_LIST_FAILED"))?;

    Ok(Json(MlopsAlertDeliveryQueueResponse {
        tasks: alert_delivery_tasks_from_events(events, review_events),
    }))
}

pub async fn submit_mlops_alert_delivery_task_review(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path((model_key, task_id)): Path<(String, String)>,
    Json(request): Json<SubmitMlopsAlertDeliveryTaskReviewRequest>,
) -> Result<Json<MlopsAlertDeliveryTaskReviewResponse>, ApiError> {
    let actor = require_permission(principal, "ops:models:review")?;
    validate_alert_delivery_task_review_request(&request)?;
    ensure_model_exists(&state, &model_key).await?;
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: 50,
            event_type: Some("model.mlops_alert_delivery.submitted".into()),
            model_key: Some(model_key.clone()),
            customer_scope_id: Some(actor.customer_scope_id.clone()),
            ..Default::default()
        })
        .await
        .map_err(internal_error("MLOPS_ALERT_DELIVERY_TASK_LIST_FAILED"))?;
    let task = alert_delivery_tasks_from_events(events, Vec::new())
        .into_iter()
        .find(|task| task.task_id == task_id)
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MLOPS_ALERT_DELIVERY_TASK_NOT_FOUND",
                "MLOps alert delivery task not found",
            )
        })?;
    validate_target_model_version_evidence(
        &request.evidence_refs,
        &task.model_key,
        &task.model_version,
        "MLOps alert delivery task review",
    )?;
    validate_alert_delivery_task_evidence(&request.evidence_refs, &task)?;

    let response = MlopsAlertDeliveryTaskReviewResponse {
        task_id: task.task_id.clone(),
        model_key: task.model_key.clone(),
        model_version: task.model_version.clone(),
        decision: request.decision.clone(),
        reviewer: request.reviewer.clone(),
        governance_boundary:
            "alert delivery task reviews record customer alert-router handoff only; they must not create retraining jobs, activate models, rollback models, or assign fraud labels"
                .into(),
    };
    record_mlops_alert_delivery_task_review_audit(&state, &actor, &task, &request, &response)
        .await
        .map_err(internal_error(
            "MLOPS_ALERT_DELIVERY_TASK_AUDIT_SAVE_FAILED",
        ))?;
    Ok(Json(response))
}

pub async fn update_model_retraining_job_status(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(job_id): Path<String>,
    Json(request): Json<UpdateModelRetrainingJobStatusRequest>,
) -> Result<Json<ModelRetrainingJobRecord>, ApiError> {
    if !matches!(
        request.status.as_str(),
        "queued" | "running" | "validation" | "failed" | "cancelled"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_JOB_STATUS",
            "status must be queued, running, validation, failed, or cancelled; completed is only set by registering external training output",
        ));
    }
    if request.actor.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_JOB_ACTOR",
            "actor is required",
        ));
    }
    if request.notes.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_JOB_NOTES",
            "retraining job notes are required",
        ));
    }
    validate_retraining_notes_without_pii(&request.notes)?;
    let job = state
        .repository
        .update_model_retraining_job_status(
            &job_id,
            &request.status,
            &request.actor,
            &request.notes,
        )
        .await
        .map_err(internal_error("MODEL_RETRAINING_JOB_UPDATE_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MODEL_RETRAINING_JOB_NOT_FOUND",
                "model retraining job not found",
            )
        })?;
    record_model_retraining_audit(&state, &actor, &job, "model.retraining.status_updated", &[])
        .await
        .map_err(internal_error("MODEL_RETRAINING_AUDIT_SAVE_FAILED"))?;
    Ok(Json(job))
}

pub async fn claim_next_model_retraining_job(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Json(request): Json<ClaimModelRetrainingJobRequest>,
) -> Result<Json<ModelRetrainingJobRecord>, ApiError> {
    if request.actor.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_JOB_ACTOR",
            "actor is required",
        ));
    }
    if request.notes.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_JOB_NOTES",
            "retraining job notes are required",
        ));
    }
    validate_retraining_notes_without_pii(&request.notes)?;
    let job = state
        .repository
        .claim_next_model_retraining_job(
            request.model_key.as_deref(),
            &request.actor,
            &request.notes,
        )
        .await
        .map_err(internal_error("MODEL_RETRAINING_JOB_CLAIM_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MODEL_RETRAINING_JOB_NOT_FOUND",
                "queued model retraining job not found",
            )
        })?;
    record_model_retraining_audit(&state, &actor, &job, "model.retraining.claimed", &[])
        .await
        .map_err(internal_error("MODEL_RETRAINING_AUDIT_SAVE_FAILED"))?;
    Ok(Json(job))
}

pub async fn complete_model_retraining_job(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(job_id): Path<String>,
    Json(request): Json<CompleteModelRetrainingJobRequest>,
) -> Result<Json<CompleteModelRetrainingJobResponse>, ApiError> {
    validate_retraining_output_request(&request)?;
    let job = state
        .repository
        .get_model_retraining_job(&job_id)
        .await
        .map_err(internal_error("MODEL_RETRAINING_JOB_LOAD_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MODEL_RETRAINING_JOB_NOT_FOUND",
                "model retraining job not found",
            )
        })?;
    if job.status != "validation" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "MODEL_RETRAINING_JOB_NOT_IN_VALIDATION",
            "model retraining output can only be registered from validation status",
        ));
    }
    let base_evaluation = state
        .repository
        .get_model_evaluation(&job.latest_evaluation_id)
        .await
        .map_err(internal_error(
            "MODEL_RETRAINING_BASE_EVALUATION_LOAD_FAILED",
        ))?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::CONFLICT,
                "MODEL_RETRAINING_BASE_EVALUATION_MISSING",
                "base model evaluation is required before registering retraining output",
            )
        })?;
    let base_model = state
        .repository
        .list_models()
        .await
        .map_err(internal_error("MODEL_LIST_FAILED"))?
        .into_iter()
        .find(|model| model.model_key == job.model_key && model.version == job.model_version)
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "MODEL_NOT_FOUND", "model not found")
        })?;
    let candidate_model = state
        .repository
        .save_model_version(ModelVersionRecord {
            model_key: job.model_key.clone(),
            version: request.candidate_model_version.clone(),
            model_type: base_model.model_type,
            runtime_kind: base_model.runtime_kind,
            execution_provider: base_model.execution_provider,
            status: "candidate".into(),
            review_mode: base_model.review_mode,
            artifact_uri: Some(request.artifact_uri.clone()),
            endpoint_url: request.endpoint_url.clone(),
        })
        .await
        .map_err(internal_error("MODEL_VERSION_SAVE_FAILED"))?;
    let metrics_json = retraining_metrics_with_artifacts(&request);
    let evaluation = state
        .repository
        .register_model_evaluation(RegisterModelEvaluationInput {
            evaluation_run_id: request.evaluation_run_id.clone(),
            model_key: candidate_model.model_key.clone(),
            model_version: candidate_model.version.clone(),
            model_dataset_id: base_evaluation.model_dataset_id,
            scheme_family: base_evaluation.scheme_family,
            auc: request.auc,
            ks: request.ks,
            precision: request.precision,
            recall: request.recall,
            f1: request.f1,
            accuracy: request.accuracy,
            threshold: request.threshold,
            confusion_matrix_json: request.confusion_matrix_json.clone(),
            feature_importance_uri: request.feature_importance_uri.clone(),
            permutation_importance_uri: request.permutation_importance_uri.clone(),
            metrics_json,
        })
        .await
        .map_err(internal_error(
            "MODEL_RETRAINING_EVALUATION_REGISTER_FAILED",
        ))?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MODEL_DATASET_NOT_FOUND",
                "model evaluation dataset was not found",
            )
        })?;
    let completed_job = state
        .repository
        .complete_model_retraining_job(CompleteModelRetrainingJobInput {
            job_id: &job_id,
            actor: &request.actor,
            status_note: &request.notes,
            candidate_model_version: &candidate_model.version,
            candidate_artifact_uri: request.artifact_uri.as_str(),
            candidate_endpoint_url: request.endpoint_url.as_deref(),
            validation_report_uri: request.validation_report_uri.as_str(),
            output_evaluation_id: &evaluation.evaluation_run_id,
        })
        .await
        .map_err(internal_error("MODEL_RETRAINING_JOB_COMPLETE_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MODEL_RETRAINING_JOB_NOT_FOUND",
                "model retraining job not found",
            )
        })?;
    let mined_rule_candidates =
        save_training_package_rule_candidates(&state, &actor, &request, &completed_job).await?;
    record_model_retraining_output_audit(
        &state,
        &actor,
        &completed_job,
        &request.evidence_refs,
        mined_rule_candidates.len(),
    )
    .await
    .map_err(internal_error("MODEL_RETRAINING_AUDIT_SAVE_FAILED"))?;
    Ok(Json(CompleteModelRetrainingJobResponse {
        job: completed_job,
        candidate_model,
        evaluation,
        mined_rule_candidates,
    }))
}

async fn load_model_retraining_readiness(
    state: &AppState,
    model_key: &str,
) -> Result<ModelRetrainingReadinessResponse, ApiError> {
    let model = state
        .repository
        .list_models()
        .await
        .map_err(internal_error("MODEL_LIST_FAILED"))?
        .into_iter()
        .find(|model| model.model_key == model_key)
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "MODEL_NOT_FOUND", "model not found")
        })?;
    let performance = state
        .repository
        .model_performance(model_key)
        .await
        .map_err(internal_error("MODEL_PERFORMANCE_FAILED"))?
        .unwrap_or_else(|| ModelPerformanceRecord {
            model_key: model_key.to_string(),
            data_status: "unknown".into(),
            scored_runs: 0,
            average_score: 0.0,
            high_risk_count: 0,
            score_psi: None,
            drift_status: "not_available".into(),
            latest_scored_at: None,
        });
    let evaluations = state
        .repository
        .list_model_evaluations()
        .await
        .map_err(internal_error("MODEL_EVALUATION_LIST_FAILED"))?;
    let latest_evaluation = evaluations.iter().find(|evaluation| {
        evaluation.model_key == model.model_key && evaluation.model_version == model.version
    });
    let source_dataset = match latest_evaluation {
        Some(evaluation) => state
            .repository
            .get_model_dataset_source_dataset(&evaluation.model_dataset_id)
            .await
            .map_err(internal_error("MODEL_DATASET_LINEAGE_FAILED"))?,
        None => None,
    };
    let outcome_labels = state
        .repository
        .list_outcome_labels(None)
        .await
        .map_err(internal_error("OUTCOME_LABEL_LIST_FAILED"))?;
    let feedback_items = state
        .repository
        .list_qa_feedback_items(None)
        .await
        .map_err(internal_error("QA_FEEDBACK_LIST_FAILED"))?;

    Ok(build_model_retraining_readiness(
        &model,
        &performance,
        latest_evaluation,
        &outcome_labels,
        &feedback_items,
        source_dataset.as_ref(),
    ))
}

async fn ensure_model_exists(state: &AppState, model_key: &str) -> Result<(), ApiError> {
    let exists = state
        .repository
        .list_models()
        .await
        .map_err(internal_error("MODEL_LIST_FAILED"))?
        .into_iter()
        .any(|model| model.model_key == model_key);
    if exists {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "MODEL_NOT_FOUND",
            "model not found",
        ))
    }
}

pub async fn submit_model_promotion_review(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(model_key): Path<String>,
    Json(request): Json<SubmitModelPromotionReviewRequest>,
) -> Result<Json<ModelPromotionReviewRecord>, ApiError> {
    let actor = require_permission(principal, "ops:models:review")?;
    submit_model_promotion_review_for_target(state, actor, model_key, None, request).await
}

pub async fn submit_model_version_promotion_review(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path((model_key, model_version)): Path<(String, String)>,
    Json(request): Json<SubmitModelPromotionReviewRequest>,
) -> Result<Json<ModelPromotionReviewRecord>, ApiError> {
    let actor = require_permission(principal, "ops:models:review")?;
    submit_model_promotion_review_for_target(state, actor, model_key, Some(model_version), request)
        .await
}

async fn submit_model_promotion_review_for_target(
    state: AppState,
    actor: ActorContext,
    model_key: String,
    model_version: Option<String>,
    request: SubmitModelPromotionReviewRequest,
) -> Result<Json<ModelPromotionReviewRecord>, ApiError> {
    if !matches!(request.decision.as_str(), "approved" | "rejected") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROMOTION_DECISION",
            "decision must be approved or rejected",
        ));
    }
    if request.reviewer.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_REVIEWER",
            "reviewer is required",
        ));
    }
    if request.notes.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_PROMOTION_REVIEW_NOTES",
            "promotion review notes are required",
        ));
    }
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_PROMOTION_REVIEW_EVIDENCE",
            "promotion review evidence_refs are required",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.notes.as_str())
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_PROMOTION_REVIEW",
            "promotion review notes and evidence_refs must not contain PII",
        ));
    }
    let model = load_model_target(&state, &model_key, model_version.as_deref()).await?;
    validate_target_model_version_evidence(
        &request.evidence_refs,
        &model.model_key,
        &model.version,
        "model promotion review",
    )?;
    let review = state
        .repository
        .save_model_promotion_review(ModelPromotionReviewRecord {
            model_key: model.model_key.clone(),
            model_version: model.version.clone(),
            decision: request.decision,
            reviewer: request.reviewer,
            notes: request.notes,
            evidence_refs: request.evidence_refs,
            created_at: None,
        })
        .await
        .map_err(internal_error("MODEL_PROMOTION_REVIEW_SAVE_FAILED"))?;
    record_model_promotion_audit(&state, &actor, &review)
        .await
        .map_err(internal_error("MODEL_PROMOTION_AUDIT_SAVE_FAILED"))?;
    Ok(Json(review))
}

pub async fn activate_model(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(model_key): Path<String>,
    Json(request): Json<ModelLifecycleRequest>,
) -> Result<Json<ModelLifecycleResponse>, ApiError> {
    let actor = require_permission(principal, "ops:models:activate")?;
    activate_model_target(state, actor, model_key, None, request).await
}

pub async fn activate_model_version(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path((model_key, model_version)): Path<(String, String)>,
    Json(request): Json<ModelLifecycleRequest>,
) -> Result<Json<ModelLifecycleResponse>, ApiError> {
    let actor = require_permission(principal, "ops:models:activate")?;
    activate_model_target(state, actor, model_key, Some(model_version), request).await
}

async fn activate_model_target(
    state: AppState,
    actor: ActorContext,
    model_key: String,
    model_version: Option<String>,
    request: ModelLifecycleRequest,
) -> Result<Json<ModelLifecycleResponse>, ApiError> {
    validate_model_lifecycle_request(&request)?;
    let (candidate, gates) = load_model_promotion_gates_for_optional_version(
        &state,
        &model_key,
        model_version.as_deref(),
    )
    .await?;
    if candidate.status == "active" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "MODEL_ALREADY_ACTIVE",
            "latest model version is already active",
        ));
    }

    let blockers = activation_blockers(&gates);
    validate_target_model_version_evidence(
        &request.evidence_refs,
        &candidate.model_key,
        &candidate.version,
        "model activation",
    )?;
    if !blockers.is_empty() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "MODEL_PROMOTION_GATES_BLOCKED",
            format!(
                "model {}:{} promotion gates blocked: {}",
                candidate.model_key,
                candidate.version,
                blockers.join(", ")
            ),
        ));
    }

    let models = state
        .repository
        .list_models()
        .await
        .map_err(internal_error("MODEL_LIST_FAILED"))?;
    let previous_active_versions = models
        .iter()
        .filter(|model| {
            model.model_key == candidate.model_key
                && model.version != candidate.version
                && model.status == "active"
        })
        .map(|model| model.version.clone())
        .collect::<Vec<_>>();
    for model in models {
        if model.model_key == candidate.model_key
            && model.version != candidate.version
            && model.status == "active"
        {
            state
                .repository
                .update_model_status(&model.model_key, &model.version, "approved")
                .await
                .map_err(internal_error("MODEL_STATUS_UPDATE_FAILED"))?;
        }
    }

    let activated = state
        .repository
        .update_model_status(&candidate.model_key, &candidate.version, "active")
        .await
        .map_err(internal_error("MODEL_STATUS_UPDATE_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "MODEL_NOT_FOUND", "model not found")
        })?;
    record_model_activation_audit(
        &state,
        &actor,
        &activated,
        Some(&candidate.status),
        previous_active_versions.first().map(String::as_str),
        request.evidence_refs,
    )
    .await
    .map_err(internal_error("MODEL_AUDIT_SAVE_FAILED"))?;
    state.scoring_lookup_cache.invalidate_all().await;
    Ok(Json(ModelLifecycleResponse {
        model_key: activated.model_key,
        version: activated.version,
        status: activated.status,
    }))
}

pub async fn rollback_model(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(model_key): Path<String>,
    Json(request): Json<ModelLifecycleRequest>,
) -> Result<Json<ModelLifecycleResponse>, ApiError> {
    validate_model_lifecycle_request(&request)?;
    let actor = require_permission(principal, "ops:models:rollback")?;
    let models = state
        .repository
        .list_models()
        .await
        .map_err(internal_error("MODEL_LIST_FAILED"))?;
    if !models.iter().any(|model| model.model_key == model_key) {
        return Err(ApiError::new(
            StatusCode::NOT_FOUND,
            "MODEL_NOT_FOUND",
            "model not found",
        ));
    }
    let active = models
        .iter()
        .find(|model| model.model_key == model_key && model.status == "active")
        .cloned()
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::CONFLICT,
                "MODEL_ROLLBACK_REQUIRES_ACTIVE",
                "only active models can be rolled back",
            )
        })?;
    validate_target_model_version_evidence(
        &request.evidence_refs,
        &active.model_key,
        &active.version,
        "model rollback",
    )?;
    let target = previous_active_model_target(&state, &model_key, &active, &models).await?;

    state
        .repository
        .update_model_status(&active.model_key, &active.version, "approved")
        .await
        .map_err(internal_error("MODEL_STATUS_UPDATE_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "MODEL_NOT_FOUND", "model not found")
        })?;
    let restored = state
        .repository
        .update_model_status(&target.model_key, &target.version, "active")
        .await
        .map_err(internal_error("MODEL_STATUS_UPDATE_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "MODEL_NOT_FOUND", "model not found")
        })?;
    record_model_rollback_audit(
        &state,
        &actor,
        &restored,
        &active,
        &target.status,
        request.evidence_refs,
    )
    .await
    .map_err(internal_error("MODEL_AUDIT_SAVE_FAILED"))?;
    state.scoring_lookup_cache.invalidate_all().await;
    Ok(Json(ModelLifecycleResponse {
        model_key: restored.model_key,
        version: restored.version,
        status: restored.status,
    }))
}

async fn previous_active_model_target(
    state: &AppState,
    model_key: &str,
    active: &ModelVersionRecord,
    models: &[ModelVersionRecord],
) -> Result<ModelVersionRecord, ApiError> {
    let events = state
        .repository
        .list_audit_events(AuditEventListFilter {
            limit: u32::MAX,
            event_group: Some("governance".into()),
            model_key: Some(model_key.to_string()),
            model_version: Some(active.version.clone()),
            ..AuditEventListFilter::default()
        })
        .await
        .map_err(internal_error("MODEL_AUDIT_LIST_FAILED"))?;
    let previous_active_version = events.into_iter().find_map(|event| {
        if event.payload["model_key"].as_str() != Some(model_key)
            || event.payload["model_version"].as_str() != Some(active.version.as_str())
        {
            return None;
        }
        match event.event_type.as_str() {
            "model.activation.completed" => event.payload["previous_active_version"]
                .as_str()
                .map(str::to_string),
            "model.rollback.completed" => event.payload["previous_active_version"]
                .as_str()
                .filter(|version| *version != active.version.as_str())
                .or_else(|| event.payload["replaced_active_version"].as_str())
                .map(str::to_string),
            _ => None,
        }
    });
    let Some(previous_active_version) = previous_active_version else {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "MODEL_ROLLBACK_TARGET_NOT_FOUND",
            "model rollback requires a recorded previous active version",
        ));
    };
    models
        .iter()
        .find(|model| {
            model.model_key == model_key
                && model.version == previous_active_version
                && model.status == "approved"
        })
        .cloned()
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::CONFLICT,
                "MODEL_ROLLBACK_TARGET_NOT_FOUND",
                "previous active model version is not available for rollback",
            )
        })
}

fn validate_model_lifecycle_request(request: &ModelLifecycleRequest) -> Result<(), ApiError> {
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MODEL_LIFECYCLE_EVIDENCE",
            "model lifecycle evidence_refs are required",
        ));
    }
    if pii::contains_pii(request.evidence_refs.iter().map(String::as_str)) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_MODEL_LIFECYCLE",
            "model lifecycle evidence_refs must not contain PII",
        ));
    }
    Ok(())
}

fn validate_target_model_version_evidence(
    evidence_refs: &[String],
    model_key: &str,
    model_version: &str,
    action: &str,
) -> Result<(), ApiError> {
    let expected_ref = model_version_evidence_ref(model_key, model_version);
    if evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_ref)
    {
        return Ok(());
    }
    Err(ApiError::new(
        StatusCode::BAD_REQUEST,
        "MISSING_TARGET_MODEL_VERSION_EVIDENCE",
        format!("{action} evidence_refs must include {expected_ref}"),
    ))
}

fn model_version_evidence_ref(model_key: &str, model_version: &str) -> String {
    format!("model_versions:{model_key}:{model_version}")
}

async fn load_model_promotion_gates(
    state: &AppState,
    model_key: &str,
) -> Result<(ModelVersionRecord, ModelPromotionGatesResponse), ApiError> {
    load_model_promotion_gates_for_optional_version(state, model_key, None).await
}

async fn load_model_promotion_gates_for_version(
    state: &AppState,
    model_key: &str,
    model_version: &str,
) -> Result<(ModelVersionRecord, ModelPromotionGatesResponse), ApiError> {
    load_model_promotion_gates_for_optional_version(state, model_key, Some(model_version)).await
}

async fn load_model_promotion_gates_for_optional_version(
    state: &AppState,
    model_key: &str,
    model_version: Option<&str>,
) -> Result<(ModelVersionRecord, ModelPromotionGatesResponse), ApiError> {
    let model = load_model_target(state, model_key, model_version).await?;
    let performance = state
        .repository
        .model_performance(model_key)
        .await
        .map_err(internal_error("MODEL_PERFORMANCE_FAILED"))?
        .unwrap_or_else(|| ModelPerformanceRecord {
            model_key: model_key.to_string(),
            data_status: "unknown".into(),
            scored_runs: 0,
            average_score: 0.0,
            high_risk_count: 0,
            score_psi: None,
            drift_status: "not_available".into(),
            latest_scored_at: None,
        });
    let evaluations = state
        .repository
        .list_model_evaluations()
        .await
        .map_err(internal_error("MODEL_EVALUATION_LIST_FAILED"))?;
    let latest_evaluation = evaluations.iter().find(|evaluation| {
        evaluation.model_key == model.model_key && evaluation.model_version == model.version
    });
    let source_dataset = match latest_evaluation {
        Some(evaluation) => state
            .repository
            .get_model_dataset_source_dataset(&evaluation.model_dataset_id)
            .await
            .map_err(internal_error("MODEL_DATASET_LINEAGE_FAILED"))?,
        None => None,
    };
    let latest_review = state
        .repository
        .latest_model_promotion_review(&model.model_key, &model.version)
        .await
        .map_err(internal_error("MODEL_PROMOTION_REVIEW_LOAD_FAILED"))?;
    let outcome_labels = state
        .repository
        .list_outcome_labels(None)
        .await
        .map_err(internal_error("OUTCOME_LABEL_LIST_FAILED"))?;
    let feedback_items = state
        .repository
        .list_qa_feedback_items(None)
        .await
        .map_err(internal_error("QA_FEEDBACK_LIST_FAILED"))?;
    let gates = build_model_promotion_gates(
        &model,
        &performance,
        &evaluations,
        &outcome_labels,
        &feedback_items,
        latest_review.as_ref(),
        source_dataset.as_ref(),
    );
    Ok((model, gates))
}

async fn load_model_target(
    state: &AppState,
    model_key: &str,
    model_version: Option<&str>,
) -> Result<ModelVersionRecord, ApiError> {
    let model = state
        .repository
        .list_models()
        .await
        .map_err(internal_error("MODEL_LIST_FAILED"))?
        .into_iter()
        .find(|model| {
            model.model_key == model_key
                && model_version
                    .map(|version| model.version == version)
                    .unwrap_or(true)
        })
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "MODEL_NOT_FOUND", "model not found")
        })?;
    Ok(model)
}

fn require_permission(
    principal: AuthenticatedPrincipal,
    permission: &str,
) -> Result<ActorContext, ApiError> {
    if !principal.has_permission(permission) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "PERMISSION_DENIED",
            format!("missing permission: {permission}"),
        ));
    }
    Ok(principal.actor)
}

pub(super) fn internal_error<E: std::fmt::Display>(
    code: &'static str,
) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::internal(code, error)
}

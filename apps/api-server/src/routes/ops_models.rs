use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        canonical_feedback_target, AuditEventListFilter, CompleteModelRetrainingJobInput,
        DatasetRecord, ModelEvaluationRecord, ModelPerformanceRecord, ModelPromotionReviewRecord,
        ModelRetrainingJobRecord, ModelVersionRecord, PersistedAuditEvent, QaFeedbackItemRecord,
        RegisterModelEvaluationInput,
    },
    routes::{ops_datasets::build_dataset_health_record, pii},
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_audit::ActorContext;
use fwa_auth::{authenticate_api_key, validate_api_key};
use fwa_core::{AuditEventId, ScoringRunId};
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
    pub rust_serving_status: Option<String>,
    pub rust_serving_latency_status: Option<String>,
    pub rust_serving_p95_latency_ms: Option<u64>,
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

struct SourceDataQualityGate {
    dataset_id: String,
    score: Option<f64>,
    status: String,
    passed: bool,
    blocker: &'static str,
    evidence_source: &'static str,
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
    pub metrics_json: Value,
}

#[derive(Debug, Serialize)]
pub struct CompleteModelRetrainingJobResponse {
    pub job: ModelRetrainingJobRecord,
    pub candidate_model: ModelVersionRecord,
    pub evaluation: ModelEvaluationRecord,
}

#[derive(Debug, Serialize)]
pub struct ModelLifecycleResponse {
    pub model_key: String,
    pub version: String,
    pub status: String,
}

pub async fn list_models(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ModelListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let models = state
        .repository
        .list_models()
        .await
        .map_err(internal_error("MODEL_LIST_FAILED"))?;
    Ok(Json(ModelListResponse { models }))
}

pub async fn model_performance(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(model_key): Path<String>,
) -> Result<Json<crate::repository::ModelPerformanceRecord>, ApiError> {
    authorize(&state, &headers)?;
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
    headers: HeaderMap,
    Path(model_key): Path<String>,
) -> Result<Json<ModelPromotionGatesResponse>, ApiError> {
    authorize(&state, &headers)?;
    let (_, gates) = load_model_promotion_gates(&state, &model_key).await?;
    Ok(Json(gates))
}

pub async fn model_retraining_readiness(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(model_key): Path<String>,
) -> Result<Json<ModelRetrainingReadinessResponse>, ApiError> {
    authorize(&state, &headers)?;
    Ok(Json(
        load_model_retraining_readiness(&state, &model_key).await?,
    ))
}

pub async fn list_model_retraining_jobs(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(model_key): Path<String>,
) -> Result<Json<ModelRetrainingJobListResponse>, ApiError> {
    authorize(&state, &headers)?;
    ensure_model_exists(&state, &model_key).await?;
    let jobs = state
        .repository
        .list_model_retraining_jobs(&model_key)
        .await
        .map_err(internal_error("MODEL_RETRAINING_JOB_LIST_FAILED"))?;
    Ok(Json(ModelRetrainingJobListResponse { jobs }))
}

pub async fn create_model_retraining_job(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(model_key): Path<String>,
    Json(request): Json<CreateModelRetrainingJobRequest>,
) -> Result<Json<ModelRetrainingJobRecord>, ApiError> {
    let actor = authorize(&state, &headers)?;
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
    headers: HeaderMap,
    Path(model_key): Path<String>,
    Json(request): Json<SubmitMlopsMonitoringReportRequest>,
) -> Result<Json<SubmitMlopsMonitoringReportResponse>, ApiError> {
    let actor = authorize_permission(&state, &headers, "ops:models:review")?;
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
    headers: HeaderMap,
    Path(model_key): Path<String>,
    Json(request): Json<SubmitMlopsAlertDeliveryRequest>,
) -> Result<Json<SubmitMlopsAlertDeliveryResponse>, ApiError> {
    let actor = authorize_permission(&state, &headers, "ops:models:review")?;
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

pub async fn update_model_retraining_job_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(job_id): Path<String>,
    Json(request): Json<UpdateModelRetrainingJobStatusRequest>,
) -> Result<Json<ModelRetrainingJobRecord>, ApiError> {
    let actor = authorize(&state, &headers)?;
    if !matches!(
        request.status.as_str(),
        "queued" | "running" | "validation" | "completed" | "failed" | "cancelled"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_JOB_STATUS",
            "status must be queued, running, validation, completed, failed, or cancelled",
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
    headers: HeaderMap,
    Json(request): Json<ClaimModelRetrainingJobRequest>,
) -> Result<Json<ModelRetrainingJobRecord>, ApiError> {
    let actor = authorize(&state, &headers)?;
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
    headers: HeaderMap,
    Path(job_id): Path<String>,
    Json(request): Json<CompleteModelRetrainingJobRequest>,
) -> Result<Json<CompleteModelRetrainingJobResponse>, ApiError> {
    let actor = authorize(&state, &headers)?;
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
            confusion_matrix_json: request.confusion_matrix_json,
            feature_importance_uri: request.feature_importance_uri,
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
    record_model_retraining_audit(
        &state,
        &actor,
        &completed_job,
        "model.retraining.output_registered",
        &request.evidence_refs,
    )
    .await
    .map_err(internal_error("MODEL_RETRAINING_AUDIT_SAVE_FAILED"))?;
    Ok(Json(CompleteModelRetrainingJobResponse {
        job: completed_job,
        candidate_model,
        evaluation,
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

fn validate_retraining_output_request(
    request: &CompleteModelRetrainingJobRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_RETRAINING_OUTPUT_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_RETRAINING_OUTPUT_NOTES",
            "retraining output notes are required",
        ),
        (
            request.candidate_model_version.as_str(),
            "INVALID_CANDIDATE_MODEL_VERSION",
            "candidate_model_version is required",
        ),
        (
            request.artifact_uri.as_str(),
            "INVALID_MODEL_ARTIFACT_URI",
            "artifact_uri is required",
        ),
        (
            request.validation_report_uri.as_str(),
            "INVALID_VALIDATION_REPORT_URI",
            "validation_report_uri is required",
        ),
        (
            request.evaluation_run_id.as_str(),
            "INVALID_EVALUATION_RUN_ID",
            "evaluation_run_id is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if let Some(endpoint_url) = &request.endpoint_url {
        let endpoint_url = endpoint_url.trim();
        if endpoint_url.is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_RETRAINING_OUTPUT_ENDPOINT",
                "endpoint_url must not be blank when provided",
            ));
        }
        if !endpoint_url.starts_with("http://") && !endpoint_url.starts_with("https://") {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_RETRAINING_OUTPUT_ENDPOINT",
                "endpoint_url must use http or https",
            ));
        }
    }
    validate_model_artifact_uri(&request.artifact_uri, "INVALID_MODEL_ARTIFACT_URI")?;
    if let Some(artifact_sha256) = &request.artifact_sha256 {
        validate_sha256_digest(
            artifact_sha256,
            "INVALID_MODEL_ARTIFACT_SHA256",
            "artifact_sha256 must start with sha256:",
        )?;
    }
    if let Some(training_artifact_uri) = &request.training_artifact_uri {
        if training_artifact_uri.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_TRAINING_ARTIFACT_URI",
                "training_artifact_uri must not be blank when provided",
            ));
        }
        validate_training_artifact_uri(training_artifact_uri, "INVALID_TRAINING_ARTIFACT_URI")?;
    }
    if let Some(training_artifact_sha256) = &request.training_artifact_sha256 {
        if request.training_artifact_uri.is_none() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_TRAINING_ARTIFACT_URI",
                "training_artifact_sha256 requires training_artifact_uri",
            ));
        }
        validate_sha256_digest(
            training_artifact_sha256,
            "INVALID_TRAINING_ARTIFACT_SHA256",
            "training_artifact_sha256 must start with sha256:",
        )?;
    }
    if let Some(serving_manifest_uri) = &request.serving_manifest_uri {
        if serving_manifest_uri.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_SERVING_MANIFEST_URI",
                "serving_manifest_uri must not be blank when provided",
            ));
        }
        validate_json_artifact_uri(
            serving_manifest_uri,
            "INVALID_SERVING_MANIFEST_URI",
            "serving_manifest_uri must point to a JSON serving manifest",
        )?;
    }
    validate_json_report_uri(
        &request.validation_report_uri,
        "INVALID_VALIDATION_REPORT_URI",
    )?;
    validate_retraining_notes_without_pii(&request.notes)?;
    validate_retraining_output_evidence_refs(request)?;
    for (metric_name, metric) in [
        ("auc", &request.auc),
        ("ks", &request.ks),
        ("precision", &request.precision),
        ("recall", &request.recall),
        ("f1", &request.f1),
        ("accuracy", &request.accuracy),
        ("threshold", &request.threshold),
    ] {
        validate_unit_interval_metric(metric_name, metric)?;
    }
    let confusion_matrix = request.confusion_matrix_json.as_object();
    if confusion_matrix.is_none()
        || confusion_matrix.is_some_and(|confusion_matrix| confusion_matrix.is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_CONFUSION_MATRIX",
            "confusion_matrix_json must be a non-empty object",
        ));
    }
    let metrics = request.metrics_json.as_object();
    if metrics.is_none() || metrics.is_some_and(|metrics| metrics.is_empty()) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_RETRAINING_OUTPUT_METRICS",
            "metrics_json must be a non-empty object",
        ));
    }
    if let Some(feature_importance_uri) = &request.feature_importance_uri {
        if feature_importance_uri.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_RETRAINING_OUTPUT_FEATURE_IMPORTANCE",
                "feature_importance_uri must not be blank when provided",
            ));
        }
        validate_parquet_artifact_uri(
            feature_importance_uri,
            "INVALID_RETRAINING_OUTPUT_FEATURE_IMPORTANCE",
        )?;
    }
    Ok(())
}

fn validate_mlops_monitoring_report_request(
    request: &SubmitMlopsMonitoringReportRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_MLOPS_MONITORING_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_MLOPS_MONITORING_NOTES",
            "MLOps monitoring notes are required",
        ),
        (
            request.report_uri.as_str(),
            "INVALID_MLOPS_MONITORING_REPORT_URI",
            "report_uri is required",
        ),
        (
            request.model_version.as_str(),
            "INVALID_MLOPS_MONITORING_MODEL_VERSION",
            "model_version is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if request.report_kind != "mlops_monitoring_report" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_MONITORING_REPORT_KIND",
            "report_kind must be mlops_monitoring_report",
        ));
    }
    if !matches!(
        request.overall_status.as_str(),
        "passed" | "watch" | "blocked"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_MONITORING_STATUS",
            "overall_status must be passed, watch, or blocked",
        ));
    }
    if !matches!(
        request.retraining_recommendation.as_str(),
        "monitor" | "prepare_retraining" | "blocked"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_RETRAINING_RECOMMENDATION",
            "retraining_recommendation must be monitor, prepare_retraining, or blocked",
        ));
    }
    validate_json_artifact_uri(
        &request.report_uri,
        "INVALID_MLOPS_MONITORING_REPORT_URI",
        "MLOps monitoring report_uri must point to a JSON report",
    )?;
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MLOPS_MONITORING_EVIDENCE",
            "MLOps monitoring evidence_refs are required",
        ));
    }
    if request
        .triggers
        .iter()
        .any(|trigger| trigger.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_MONITORING_TRIGGER",
            "MLOps monitoring triggers must not be blank",
        ));
    }
    if request
        .review_tasks
        .iter()
        .any(|task| match task.as_object() {
            Some(object) => object.is_empty(),
            None => true,
        })
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_MONITORING_REVIEW_TASK",
            "MLOps monitoring review_tasks must be non-empty objects",
        ));
    }
    if request.overall_status != "passed" && request.review_tasks.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MLOPS_MONITORING_REVIEW_TASK",
            "watch or blocked monitoring reports require review_tasks",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.actor.as_str())
            .chain(std::iter::once(request.notes.as_str()))
            .chain(std::iter::once(request.report_uri.as_str()))
            .chain(request.triggers.iter().map(String::as_str))
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_MLOPS_MONITORING_REPORT",
            "MLOps monitoring actor, notes, report_uri, triggers, and evidence_refs must not contain PII",
        ));
    }
    Ok(())
}

fn validate_monitoring_report_evidence(
    request: &SubmitMlopsMonitoringReportRequest,
) -> Result<(), ApiError> {
    let expected_ref = format!("model_monitoring_reports:{}", request.report_uri);
    if request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_ref)
    {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MLOPS_MONITORING_EVIDENCE",
            format!("MLOps monitoring evidence_refs must include {expected_ref}"),
        ))
    }
}

fn validate_mlops_alert_delivery_request(
    request: &SubmitMlopsAlertDeliveryRequest,
) -> Result<(), ApiError> {
    for (value, code, message) in [
        (
            request.actor.as_str(),
            "INVALID_MLOPS_ALERT_DELIVERY_ACTOR",
            "actor is required",
        ),
        (
            request.notes.as_str(),
            "INVALID_MLOPS_ALERT_DELIVERY_NOTES",
            "MLOps alert delivery notes are required",
        ),
        (
            request.scheduler_execution_report_uri.as_str(),
            "INVALID_MLOPS_SCHEDULER_REPORT_URI",
            "scheduler_execution_report_uri is required",
        ),
        (
            request.model_version.as_str(),
            "INVALID_MLOPS_ALERT_DELIVERY_MODEL_VERSION",
            "model_version is required",
        ),
    ] {
        if value.trim().is_empty() {
            return Err(ApiError::new(StatusCode::BAD_REQUEST, code, message));
        }
    }
    if request.report_kind != "mlops_scheduler_execution_report" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_SCHEDULER_REPORT_KIND",
            "report_kind must be mlops_scheduler_execution_report",
        ));
    }
    if !matches!(
        request.alert_delivery_status.as_str(),
        "no_alerts_required" | "queued_for_external_alert_router"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_ALERT_DELIVERY_STATUS",
            "alert_delivery_status must be no_alerts_required or queued_for_external_alert_router",
        ));
    }
    validate_json_artifact_uri(
        &request.scheduler_execution_report_uri,
        "INVALID_MLOPS_SCHEDULER_REPORT_URI",
        "scheduler_execution_report_uri must point to a JSON report",
    )?;
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MLOPS_ALERT_DELIVERY_EVIDENCE",
            "MLOps alert delivery evidence_refs are required",
        ));
    }
    if request.alert_delivery_status == "queued_for_external_alert_router"
        && request.alert_delivery_tasks.is_empty()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MLOPS_ALERT_DELIVERY_TASK",
            "queued alert delivery requires alert_delivery_tasks",
        ));
    }
    if request
        .alert_delivery_tasks
        .iter()
        .any(|task| match task.as_object() {
            Some(object) => {
                object.is_empty()
                    || task.get("task_kind").and_then(|value| value.as_str())
                        != Some("mlops_alert_delivery")
            }
            None => true,
        })
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MLOPS_ALERT_DELIVERY_TASK",
            "alert_delivery_tasks must be non-empty mlops_alert_delivery objects",
        ));
    }
    if pii::contains_pii(
        std::iter::once(request.actor.as_str())
            .chain(std::iter::once(request.notes.as_str()))
            .chain(std::iter::once(
                request.scheduler_execution_report_uri.as_str(),
            ))
            .chain(request.evidence_refs.iter().map(String::as_str)),
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_MLOPS_ALERT_DELIVERY",
            "MLOps alert delivery actor, notes, scheduler report URI, and evidence_refs must not contain PII",
        ));
    }
    Ok(())
}

fn validate_alert_delivery_evidence(
    request: &SubmitMlopsAlertDeliveryRequest,
) -> Result<(), ApiError> {
    let expected_ref = format!(
        "mlops_scheduler_execution_reports:{}",
        request.scheduler_execution_report_uri
    );
    if request
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == expected_ref)
    {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_MLOPS_ALERT_DELIVERY_EVIDENCE",
            format!("MLOps alert delivery evidence_refs must include {expected_ref}"),
        ))
    }
}

fn validate_retraining_output_evidence_refs(
    request: &CompleteModelRetrainingJobRequest,
) -> Result<(), ApiError> {
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_RETRAINING_OUTPUT_EVIDENCE",
            "model retraining output evidence_refs are required",
        ));
    }
    if pii::contains_pii(request.evidence_refs.iter().map(String::as_str)) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_MODEL_RETRAINING_JOB",
            "model retraining output evidence_refs must not contain PII",
        ));
    }
    for expected_ref in [
        format!("model_artifacts:{}", request.artifact_uri),
        format!("model_validation_reports:{}", request.validation_report_uri),
        format!("model_evaluations:{}", request.evaluation_run_id),
    ] {
        if !request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim() == expected_ref)
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "MISSING_RETRAINING_OUTPUT_EVIDENCE",
                format!("model retraining output evidence_refs must include {expected_ref}"),
            ));
        }
    }
    if let Some(training_artifact_uri) = &request.training_artifact_uri {
        let expected_ref = format!("model_training_artifacts:{training_artifact_uri}");
        if !request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim() == expected_ref)
        {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "MISSING_RETRAINING_OUTPUT_EVIDENCE",
                format!("model retraining output evidence_refs must include {expected_ref}"),
            ));
        }
    }
    if let Some(serving_manifest_uri) = &request.serving_manifest_uri {
        let expected_refs = [
            format!("model_serving_manifests:{serving_manifest_uri}"),
            format!("serving_manifests:{serving_manifest_uri}"),
        ];
        if !expected_refs.iter().any(|expected_ref| {
            request
                .evidence_refs
                .iter()
                .any(|reference| reference.trim() == expected_ref)
        }) {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "MISSING_RETRAINING_OUTPUT_EVIDENCE",
                format!(
                    "model retraining output evidence_refs must include {}",
                    expected_refs[0]
                ),
            ));
        }
    }
    Ok(())
}

fn retraining_metrics_with_artifacts(request: &CompleteModelRetrainingJobRequest) -> Value {
    let mut metrics_json = request.metrics_json.clone();
    let Some(metrics) = metrics_json.as_object_mut() else {
        return metrics_json;
    };
    if let Some(artifact_sha256) = &request.artifact_sha256 {
        metrics.insert(
            "artifact_sha256".into(),
            Value::String(artifact_sha256.clone()),
        );
    }
    if let Some(training_artifact_uri) = &request.training_artifact_uri {
        metrics.insert(
            "training_artifact_uri".into(),
            Value::String(training_artifact_uri.clone()),
        );
    }
    if let Some(training_artifact_sha256) = &request.training_artifact_sha256 {
        metrics.insert(
            "training_artifact_sha256".into(),
            Value::String(training_artifact_sha256.clone()),
        );
    }
    if let Some(serving_manifest_uri) = &request.serving_manifest_uri {
        metrics.insert(
            "serving_manifest_uri".into(),
            Value::String(serving_manifest_uri.clone()),
        );
    }
    metrics_json
}

fn validate_retraining_notes_without_pii(notes: &str) -> Result<(), ApiError> {
    if pii::contains_pii([notes]) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_MODEL_RETRAINING_JOB",
            "model retraining notes must not contain PII",
        ));
    }
    Ok(())
}

fn validate_unit_interval_metric(
    metric_name: &'static str,
    metric: &Option<Decimal>,
) -> Result<(), ApiError> {
    if let Some(metric) = metric {
        if *metric < Decimal::ZERO || *metric > Decimal::ONE {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_RETRAINING_OUTPUT_METRIC",
                format!("{metric_name} must be between 0 and 1"),
            ));
        }
    }
    Ok(())
}

fn validate_parquet_artifact_uri(value: &str, code: &'static str) -> Result<(), ApiError> {
    if has_supported_uri_suffix(value, &[".parquet"]) || has_supported_uri_suffix(value, &["/"]) {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            code,
            "model evaluation artifact URIs must point to parquet files or parquet partition directories",
        ))
    }
}

fn validate_model_artifact_uri(value: &str, code: &'static str) -> Result<(), ApiError> {
    if has_supported_uri_suffix(value, &[".onnx", ".pkl", ".joblib", ".json"]) {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            code,
            "model retraining artifact_uri must use a supported model artifact format: .onnx, .pkl, .joblib, or .json",
        ))
    }
}

fn validate_training_artifact_uri(value: &str, code: &'static str) -> Result<(), ApiError> {
    if has_supported_uri_suffix(value, &[".pkl", ".joblib"]) {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            code,
            "training_artifact_uri must use a supported training artifact format: .pkl or .joblib",
        ))
    }
}

fn validate_sha256_digest(
    value: &str,
    code: &'static str,
    message: &'static str,
) -> Result<(), ApiError> {
    if value.starts_with("sha256:") && value.len() > "sha256:".len() {
        Ok(())
    } else {
        Err(ApiError::new(StatusCode::BAD_REQUEST, code, message))
    }
}

fn validate_json_report_uri(value: &str, code: &'static str) -> Result<(), ApiError> {
    validate_json_artifact_uri(
        value,
        code,
        "model retraining validation_report_uri must point to a JSON report",
    )
}

fn validate_json_artifact_uri(
    value: &str,
    code: &'static str,
    message: &'static str,
) -> Result<(), ApiError> {
    if has_supported_uri_suffix(value, &[".json"]) {
        Ok(())
    } else {
        Err(ApiError::new(StatusCode::BAD_REQUEST, code, message))
    }
}

fn has_supported_uri_suffix(value: &str, suffixes: &[&str]) -> bool {
    let normalized = value
        .trim()
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    suffixes.iter().any(|suffix| normalized.ends_with(suffix))
}

pub async fn submit_model_promotion_review(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(model_key): Path<String>,
    Json(request): Json<SubmitModelPromotionReviewRequest>,
) -> Result<Json<ModelPromotionReviewRecord>, ApiError> {
    let actor = authorize_permission(&state, &headers, "ops:models:review")?;
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
    headers: HeaderMap,
    Path(model_key): Path<String>,
    Json(request): Json<ModelLifecycleRequest>,
) -> Result<Json<ModelLifecycleResponse>, ApiError> {
    validate_model_lifecycle_request(&request)?;
    let actor = authorize_permission(&state, &headers, "ops:models:activate")?;
    let (candidate, gates) = load_model_promotion_gates(&state, &model_key).await?;
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
    Ok(Json(ModelLifecycleResponse {
        model_key: activated.model_key,
        version: activated.version,
        status: activated.status,
    }))
}

pub async fn rollback_model(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(model_key): Path<String>,
    Json(request): Json<ModelLifecycleRequest>,
) -> Result<Json<ModelLifecycleResponse>, ApiError> {
    validate_model_lifecycle_request(&request)?;
    let actor = authorize_permission(&state, &headers, "ops:models:rollback")?;
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

fn activation_blockers(gates: &ModelPromotionGatesResponse) -> Vec<String> {
    gates
        .gates
        .iter()
        .filter(|gate| gate.label != "Active version" && !gate.passed)
        .map(|gate| gate.blocker.clone())
        .collect()
}

fn build_model_promotion_gates(
    model: &ModelVersionRecord,
    performance: &ModelPerformanceRecord,
    evaluations: &[ModelEvaluationRecord],
    outcome_labels: &[crate::repository::OutcomeLabelRecord],
    feedback_items: &[QaFeedbackItemRecord],
    latest_review: Option<&ModelPromotionReviewRecord>,
    source_dataset: Option<&DatasetRecord>,
) -> ModelPromotionGatesResponse {
    let latest_evaluation = evaluations.iter().find(|evaluation| {
        evaluation.model_key == model.model_key && evaluation.model_version == model.version
    });
    let metrics = latest_evaluation
        .map(|evaluation| &evaluation.metrics_json)
        .unwrap_or(&serde_json::Value::Null);
    let has_out_of_time_metric = metrics.get("out_of_time_auc").is_some()
        || metrics.get("out_of_time_precision").is_some()
        || metrics.get("out_of_time_recall").is_some();
    let time_group_split_strategy = time_group_split_strategy_gate(metrics);
    let immutable_dataset = latest_evaluation
        .map(|evaluation| !evaluation.model_dataset_id.is_empty())
        .unwrap_or(false);
    let holdout_metrics = latest_evaluation
        .map(|evaluation| {
            evaluation.auc.is_some()
                && evaluation.precision.is_some()
                && evaluation.recall.is_some()
        })
        .unwrap_or(false);
    let review_capacity_threshold = latest_evaluation
        .map(|evaluation| {
            evaluation.threshold.is_some()
                && metrics
                    .get("review_capacity_threshold_status")
                    .and_then(|value| value.as_str())
                    == Some("passed")
        })
        .unwrap_or(false);
    let explanation_artifact = latest_evaluation
        .and_then(|evaluation| evaluation.feature_importance_uri.as_ref())
        .is_some();
    let leakage_check = metrics
        .get("leakage_check_status")
        .and_then(|value| value.as_str())
        == Some("passed");
    let shadow_comparison = metrics
        .get("shadow_comparison_status")
        .and_then(|value| value.as_str())
        == Some("passed");
    let serving_version_lock = metrics
        .get("serving_version_lock_status")
        .and_then(|value| value.as_str())
        == Some("passed");
    let artifact_integrity = metrics
        .get("artifact_integrity_status")
        .and_then(|value| value.as_str())
        == Some("passed");
    let feature_store_materialization = feature_materialization_gate(metrics);
    let segment_fairness = metrics
        .get("segment_fairness_status")
        .and_then(|value| value.as_str())
        == Some("passed");
    let rust_serving_evaluation = rust_serving_evaluation_gate(metrics);
    let source_data_quality = source_data_quality_gate(metrics, source_dataset);
    let feature_reproducibility = metrics
        .get("feature_reproducibility_hash")
        .and_then(|value| value.as_str())
        .map(|hash| hash.starts_with("sha256:") && hash.len() > "sha256:".len())
        .unwrap_or(false);
    let label_provenance = metrics
        .get("label_provenance_status")
        .and_then(|value| value.as_str())
        == Some("passed")
        && metrics
            .get("label_reviewer_source")
            .and_then(|value| value.as_str())
            .map(|source| !source.trim().is_empty())
            .unwrap_or(false);
    let pilot_customer_validation = pilot_customer_validation_gate(metrics);
    let approval = latest_review
        .map(|review| review.decision == "approved")
        .unwrap_or_else(|| {
            metrics
                .get("approval_status")
                .and_then(|value| value.as_str())
                == Some("approved")
        });
    let drift_status = performance.drift_status.as_str();
    let drift_gate_passed = drift_status == "stable";
    let active_version = model.status == "active";
    let open_model_feedback_count = feedback_items
        .iter()
        .filter(|item| {
            canonical_feedback_target(&item.feedback_target) == "model"
                && item.status == "open"
                && evidence_refs_apply_to_model_version(&item.evidence_refs, model)
        })
        .count();
    let unresolved_model_feedback_count = feedback_items
        .iter()
        .filter(|item| {
            canonical_feedback_target(&item.feedback_target) == "model"
                && is_unresolved_feedback_status(&item.status)
                && evidence_refs_apply_to_model_version(&item.evidence_refs, model)
        })
        .count();
    let model_labels = outcome_labels
        .iter()
        .filter(|label| {
            canonical_feedback_target(&label.feedback_target) == "model"
                && evidence_refs_apply_to_model_version(&label.evidence_refs, model)
        })
        .collect::<Vec<_>>();
    let approved_model_labels = model_labels
        .iter()
        .filter(|label| label.governance_status == "approved_for_training")
        .count();
    let needs_review_model_labels = model_labels
        .iter()
        .filter(|label| label.governance_status == "needs_review")
        .count();
    let label_governance = approved_model_labels > 0 && needs_review_model_labels == 0;
    let artifact_evidence = model_artifact_evidence_summary(metrics);

    let gates = vec![
        gate(
            "Immutable dataset",
            immutable_dataset,
            "dataset version missing",
            evidence_source(immutable_dataset, "evaluation"),
        ),
        gate(
            "Holdout metrics",
            holdout_metrics,
            "holdout metrics missing",
            evidence_source(holdout_metrics, "evaluation"),
        ),
        gate(
            "Out-of-time evidence",
            has_out_of_time_metric,
            "out-of-time metrics missing",
            evidence_source(has_out_of_time_metric, "evaluation"),
        ),
        gate(
            "Time/group split strategy",
            time_group_split_strategy,
            "time/group split strategy missing",
            evidence_source(time_group_split_strategy, "evaluation"),
        ),
        gate(
            "Review-capacity threshold",
            review_capacity_threshold,
            "review-capacity threshold missing",
            evidence_source(review_capacity_threshold, "evaluation"),
        ),
        gate(
            "Explanation artifact",
            explanation_artifact,
            "feature importance missing",
            evidence_source(explanation_artifact, "evaluation"),
        ),
        gate(
            "Leakage check",
            leakage_check,
            "leakage check missing",
            evidence_source(leakage_check, "evaluation"),
        ),
        gate(
            "Shadow comparison",
            shadow_comparison,
            "shadow comparison missing",
            evidence_source(shadow_comparison, "evaluation"),
        ),
        gate(
            "Serving version lock",
            serving_version_lock,
            "serving version lock missing",
            evidence_source(serving_version_lock, "evaluation"),
        ),
        gate(
            "Artifact integrity",
            artifact_integrity,
            "artifact integrity missing",
            evidence_source(artifact_integrity, "evaluation"),
        ),
        gate(
            "Feature materialization",
            feature_store_materialization,
            "rust feature-set materialization missing",
            evidence_source(feature_store_materialization, "evaluation"),
        ),
        gate(
            "Segment fairness",
            segment_fairness,
            "segment fairness review missing",
            evidence_source(segment_fairness, "evaluation"),
        ),
        gate(
            "Rust serving evaluation",
            rust_serving_evaluation,
            "rust serving artifact evaluation missing",
            evidence_source(rust_serving_evaluation, "evaluation"),
        ),
        gate(
            "Source data quality",
            source_data_quality.passed,
            source_data_quality.blocker,
            source_data_quality.evidence_source,
        ),
        gate(
            "Feature reproducibility",
            feature_reproducibility,
            "feature reproducibility hash missing",
            evidence_source(feature_reproducibility, "evaluation"),
        ),
        gate(
            "Label provenance",
            label_provenance,
            label_provenance_blocker(metrics),
            evidence_source(label_provenance, "evaluation"),
        ),
        gate(
            "Pilot/customer validation",
            pilot_customer_validation,
            "pilot/customer validation missing",
            pilot_customer_validation_evidence_source(metrics, pilot_customer_validation),
        ),
        gate(
            "Drift status",
            drift_gate_passed,
            drift_blocker(drift_status),
            drift_evidence_source(drift_status),
        ),
        gate(
            "Model QA feedback closure",
            unresolved_model_feedback_count == 0,
            "unresolved model QA feedback",
            "qa_feedback",
        ),
        gate(
            "Label governance",
            label_governance,
            label_governance_blocker(approved_model_labels, needs_review_model_labels),
            if model_labels.is_empty() {
                "missing"
            } else {
                "labels"
            },
        ),
        gate(
            "Approval",
            approval,
            "approval missing",
            evidence_source(approval, "approval"),
        ),
        gate(
            "Active version",
            active_version,
            "model is not active",
            evidence_source(active_version, "metadata"),
        ),
    ];
    let blockers = gates
        .iter()
        .filter(|gate| !gate.passed)
        .map(|gate| gate.blocker.clone())
        .collect::<Vec<_>>();

    ModelPromotionGatesResponse {
        model_key: model.model_key.clone(),
        model_version: model.version.clone(),
        review_mode: model.review_mode.clone(),
        decision: if blockers.is_empty() {
            "routing_allowed".into()
        } else {
            "routing_blocked".into()
        },
        passed_count: gates.len() - blockers.len(),
        total_count: gates.len(),
        latest_evaluation_id: latest_evaluation
            .map(|evaluation| evaluation.evaluation_run_id.clone())
            .unwrap_or_else(|| "none".into()),
        source_dataset_id: source_data_quality.dataset_id,
        source_data_quality_score: source_data_quality.score,
        source_data_quality_status: source_data_quality.status,
        data_status: performance.data_status.clone(),
        scored_runs: performance.scored_runs,
        open_model_feedback_count,
        unresolved_model_feedback_count,
        approved_label_count: approved_model_labels,
        needs_review_label_count: needs_review_model_labels,
        artifact_evidence,
        gates,
        blockers,
    }
}

fn build_model_retraining_readiness(
    model: &ModelVersionRecord,
    performance: &ModelPerformanceRecord,
    latest_evaluation: Option<&ModelEvaluationRecord>,
    outcome_labels: &[crate::repository::OutcomeLabelRecord],
    feedback_items: &[QaFeedbackItemRecord],
    source_dataset: Option<&DatasetRecord>,
) -> ModelRetrainingReadinessResponse {
    let metrics = latest_evaluation
        .map(|evaluation| &evaluation.metrics_json)
        .unwrap_or(&serde_json::Value::Null);
    let source_data_quality = source_data_quality_gate(metrics, source_dataset);
    let open_model_feedback_count = feedback_items
        .iter()
        .filter(|item| {
            canonical_feedback_target(&item.feedback_target) == "model" && item.status == "open"
        })
        .count();
    let model_labels = outcome_labels
        .iter()
        .filter(|label| canonical_feedback_target(&label.feedback_target) == "model")
        .collect::<Vec<_>>();
    let approved_label_count = model_labels
        .iter()
        .filter(|label| label.governance_status == "approved_for_training")
        .count();
    let needs_review_label_count = model_labels
        .iter()
        .filter(|label| label.governance_status == "needs_review")
        .count();

    let mut retraining_triggers = Vec::new();
    if matches!(performance.drift_status.as_str(), "watch" | "drift") {
        retraining_triggers.push(format!("score drift status: {}", performance.drift_status));
    }
    if open_model_feedback_count > 0 {
        retraining_triggers.push("open model QA feedback".into());
    }
    if approved_label_count > 0 {
        retraining_triggers.push("approved model labels available".into());
    }

    let mut blockers = Vec::new();
    if latest_evaluation.is_none() {
        blockers.push("latest model evaluation missing".into());
    }
    if !source_data_quality.passed {
        blockers.push(source_data_quality.blocker.into());
    }
    if approved_label_count == 0 {
        blockers.push("approved model outcome labels missing".into());
    }
    if needs_review_label_count > 0 {
        blockers.push("model outcome labels need review".into());
    }

    let recommendation = if !blockers.is_empty() {
        "blocked"
    } else if retraining_triggers.is_empty() {
        "monitor"
    } else {
        "prepare_retraining"
    };

    ModelRetrainingReadinessResponse {
        model_key: model.model_key.clone(),
        model_version: model.version.clone(),
        recommendation: recommendation.into(),
        latest_evaluation_id: latest_evaluation
            .map(|evaluation| evaluation.evaluation_run_id.clone())
            .unwrap_or_else(|| "none".into()),
        drift_status: performance.drift_status.clone(),
        source_dataset_id: source_data_quality.dataset_id,
        source_data_quality_score: source_data_quality.score,
        source_data_quality_status: source_data_quality.status,
        open_model_feedback_count,
        approved_label_count,
        needs_review_label_count,
        retraining_triggers,
        blockers,
    }
}

fn build_mlops_monitoring_report_response(
    model: &ModelVersionRecord,
    request: &SubmitMlopsMonitoringReportRequest,
) -> SubmitMlopsMonitoringReportResponse {
    let mut next_actions = Vec::new();
    match request.retraining_recommendation.as_str() {
        "prepare_retraining" => {
            next_actions.push("review_monitoring_report".into());
            next_actions.push("prepare_retraining_job_after_human_approval".into());
        }
        "blocked" => {
            next_actions.push("open_model_governance_review".into());
            next_actions.push("consider_rollback_review_after_human_approval".into());
        }
        _ => next_actions.push("continue_monitoring".into()),
    }
    if request.triggers.iter().any(|trigger| {
        trigger == "rust_serving_latency_budget_failed"
            || trigger == "segment_fairness_review_required"
    }) {
        next_actions.push("open_serving_or_fairness_review".into());
    }
    next_actions.sort();
    next_actions.dedup();

    SubmitMlopsMonitoringReportResponse {
        model_key: model.model_key.clone(),
        model_version: model.version.clone(),
        report_uri: request.report_uri.clone(),
        monitoring_status: request.overall_status.clone(),
        retraining_recommendation: request.retraining_recommendation.clone(),
        trigger_count: request.triggers.len(),
        review_task_count: request.review_tasks.len(),
        next_actions,
        governance_boundary:
            "monitoring report submission records review and retraining readiness only; it must not auto-create retraining jobs, activate models, or rollback models"
                .into(),
    }
}

fn build_mlops_alert_delivery_response(
    state: &AppState,
    model: &ModelVersionRecord,
    request: &SubmitMlopsAlertDeliveryRequest,
) -> SubmitMlopsAlertDeliveryResponse {
    let mut next_actions = vec!["record_alert_router_delivery_evidence".into()];
    if request.alert_delivery_status == "queued_for_external_alert_router" {
        next_actions.push("confirm_customer_alert_router_receipt".into());
        next_actions.push("review_alert_delivery_tasks".into());
    } else {
        next_actions.push("continue_monitoring".into());
    }
    next_actions.sort();
    next_actions.dedup();

    SubmitMlopsAlertDeliveryResponse {
        model_key: model.model_key.clone(),
        model_version: model.version.clone(),
        scheduler_execution_report_uri: request.scheduler_execution_report_uri.clone(),
        alert_delivery_status: request.alert_delivery_status.clone(),
        alert_delivery_task_count: request.alert_delivery_tasks.len(),
        alert_routing_policy_configured: !state.config.alert_routing_policy_id.trim().is_empty(),
        next_actions,
        governance_boundary:
            "alert delivery submission records customer alert-router handoff only; it must not create retraining jobs, activate models, rollback models, or assign fraud labels"
                .into(),
    }
}

fn is_unresolved_feedback_status(status: &str) -> bool {
    matches!(status, "open" | "in_progress")
}

fn evidence_refs_apply_to_model_version(
    evidence_refs: &[String],
    model: &ModelVersionRecord,
) -> bool {
    let mut has_model_version_ref = false;
    let expected = format!("{}:{}", model.model_key, model.version);
    for evidence_ref in evidence_refs {
        let Some(model_version_ref) = evidence_ref.trim().strip_prefix("model_versions:") else {
            continue;
        };
        has_model_version_ref = true;
        if model_version_ref == expected {
            return true;
        }
    }
    !has_model_version_ref
}

fn evidence_source(passed: bool, source: &'static str) -> &'static str {
    if passed {
        source
    } else {
        "missing"
    }
}

fn drift_blocker(status: &str) -> &'static str {
    match status {
        "not_available" => "model drift status unavailable",
        _ => "model drift detected",
    }
}

fn drift_evidence_source(status: &str) -> &'static str {
    match status {
        "not_available" => "missing",
        _ => "evaluation",
    }
}

fn label_governance_blocker(approved_count: usize, needs_review_count: usize) -> &'static str {
    if approved_count == 0 {
        "approved model outcome labels missing"
    } else if needs_review_count > 0 {
        "model outcome labels need review"
    } else {
        "none"
    }
}

fn time_group_split_strategy_gate(metrics: &serde_json::Value) -> bool {
    let status_passed = metrics
        .get("time_group_split_status")
        .and_then(|value| value.as_str())
        == Some("passed");
    let has_time_field = metrics
        .get("time_split_field")
        .and_then(|value| value.as_str())
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let has_group_field = metrics
        .get("group_split_fields")
        .and_then(|value| value.as_array())
        .map(|fields| {
            fields
                .iter()
                .any(|field| field.as_str().is_some_and(|value| !value.trim().is_empty()))
        })
        .unwrap_or(false);
    status_passed && has_time_field && has_group_field
}

fn feature_materialization_gate(metrics: &serde_json::Value) -> bool {
    let feature_store_status = metrics
        .get("feature_store_materialization_status")
        .and_then(|value| value.as_str())
        == Some("passed");
    let rust_feature_set_status = metrics
        .get("rust_feature_set_status")
        .and_then(|value| value.as_str())
        == Some("passed");
    let has_rust_feature_set_manifest = metrics
        .get("rust_feature_set_manifest_uri")
        .and_then(|value| value.as_str())
        .is_some_and(|value| !value.trim().is_empty());
    feature_store_status && rust_feature_set_status && has_rust_feature_set_manifest
}

fn pilot_customer_validation_gate(metrics: &serde_json::Value) -> bool {
    let validation_status_passed = ["pilot_validation_status", "customer_validation_status"]
        .into_iter()
        .any(|field| metrics.get(field).and_then(|value| value.as_str()) == Some("passed"));
    let usage_scope_validated = metrics
        .get("dataset_usage_scope")
        .and_then(|value| value.as_str())
        .is_some_and(|scope| {
            matches!(
                scope,
                "customer_pilot_validated"
                    | "customer_production_validated"
                    | "customer_validated"
                    | "pilot_validated"
            )
        });
    validation_status_passed || usage_scope_validated
}

fn pilot_customer_validation_evidence_source(
    metrics: &serde_json::Value,
    passed: bool,
) -> &'static str {
    if passed
        || metrics.get("dataset_usage_scope").is_some()
        || metrics.get("pilot_validation_status").is_some()
        || metrics.get("customer_validation_status").is_some()
    {
        "evaluation"
    } else {
        "missing"
    }
}

fn rust_serving_evaluation_gate(metrics: &Value) -> bool {
    if metrics
        .get("model_artifact_evaluation_status")
        .and_then(|value| value.as_str())
        == Some("passed")
    {
        return true;
    }
    if metrics
        .get("model_artifact_evaluation_gate_status")
        .and_then(|value| value.as_str())
        == Some("passed")
    {
        return true;
    }
    if metrics.get("report_kind").and_then(|value| value.as_str())
        == Some("model_artifact_evaluation")
        && metrics.get("gate_status").and_then(|value| value.as_str()) == Some("passed")
    {
        return true;
    }
    metrics
        .get("model_artifact_evaluation")
        .is_some_and(|value| {
            value.get("report_kind").and_then(|value| value.as_str())
                == Some("model_artifact_evaluation")
                && value.get("gate_status").and_then(|value| value.as_str()) == Some("passed")
        })
}

fn model_artifact_evidence_summary(metrics: &Value) -> ModelArtifactEvidenceSummary {
    ModelArtifactEvidenceSummary {
        serving_manifest_uri: optional_metric_string(metrics, "serving_manifest_uri"),
        model_artifact_evaluation_report_uri: optional_metric_string(
            metrics,
            "model_artifact_evaluation_report_uri",
        ),
        rust_serving_status: optional_metric_string(metrics, "rust_serving_status"),
        rust_serving_latency_status: optional_metric_string(metrics, "rust_serving_latency_status"),
        rust_serving_p95_latency_ms: optional_metric_u64(metrics, "rust_serving_p95_latency_ms"),
    }
}

fn optional_metric_string(metrics: &Value, key: &str) -> Option<String> {
    metrics
        .get(key)
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn optional_metric_u64(metrics: &Value, key: &str) -> Option<u64> {
    metrics.get(key).and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_str().and_then(|value| value.parse::<u64>().ok()))
    })
}

fn source_data_quality_gate(
    metrics: &serde_json::Value,
    source_dataset: Option<&DatasetRecord>,
) -> SourceDataQualityGate {
    if let Some(dataset) = source_dataset {
        let health = build_dataset_health_record(dataset);
        return SourceDataQualityGate {
            dataset_id: health.dataset_id,
            score: Some(health.data_quality_score),
            status: health.data_quality_status,
            passed: health.data_quality_score >= 0.8,
            blocker: if health.data_quality_score >= 0.8 {
                "none"
            } else {
                "source dataset data quality below threshold"
            },
            evidence_source: "dataset",
        };
    }

    match metrics
        .get("data_quality_score")
        .and_then(|value| value.as_f64())
    {
        Some(score) => SourceDataQualityGate {
            dataset_id: "none".into(),
            score: Some(score),
            status: data_quality_status_for_score(score).into(),
            passed: score >= 0.8,
            blocker: if score >= 0.8 {
                "none"
            } else {
                "source data quality score below threshold"
            },
            evidence_source: "evaluation",
        },
        None => SourceDataQualityGate {
            dataset_id: "none".into(),
            score: None,
            status: "missing".into(),
            passed: false,
            blocker: "source data quality score missing",
            evidence_source: "missing",
        },
    }
}

fn data_quality_status_for_score(score: f64) -> &'static str {
    if score >= 0.85 {
        "ready"
    } else if score >= 0.65 {
        "watch"
    } else {
        "blocked"
    }
}

fn label_provenance_blocker(metrics: &serde_json::Value) -> &'static str {
    let status = metrics
        .get("label_provenance_status")
        .and_then(|value| value.as_str());
    let reviewer_source_present = metrics
        .get("label_reviewer_source")
        .and_then(|value| value.as_str())
        .map(|source| !source.trim().is_empty())
        .unwrap_or(false);
    if status == Some("passed") && !reviewer_source_present {
        "label reviewer source missing"
    } else {
        "label provenance missing"
    }
}

fn gate(label: &str, passed: bool, blocker: &str, evidence_source: &str) -> ModelPromotionGate {
    ModelPromotionGate {
        label: label.into(),
        passed,
        blocker: blocker.into(),
        evidence_source: evidence_source.into(),
    }
}

async fn record_model_promotion_audit(
    state: &AppState,
    actor: &ActorContext,
    review: &ModelPromotionReviewRecord,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "model.promotion.reviewed".into(),
            event_status: "succeeded".into(),
            summary: format!("Model promotion review: {}", review.decision),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "model_key": review.model_key,
                "model_version": review.model_version,
                "decision": review.decision,
                "reviewer": review.reviewer,
                "note_present": !review.notes.trim().is_empty(),
            }),
            evidence_refs: review
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

async fn record_model_retraining_audit(
    state: &AppState,
    actor: &ActorContext,
    job: &ModelRetrainingJobRecord,
    event_type: &'static str,
    output_evidence_refs: &[String],
) -> anyhow::Result<()> {
    let mut evidence_refs = vec![serde_json::json!(format!(
        "model_retraining_jobs:{}",
        job.job_id
    ))];
    evidence_refs.extend(
        output_evidence_refs
            .iter()
            .cloned()
            .map(serde_json::Value::String),
    );
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: event_type.into(),
            event_status: "succeeded".into(),
            summary: format!("Model retraining job {} is {}", job.job_id, job.status),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "job_id": job.job_id,
                "model_key": job.model_key,
                "model_version": job.model_version,
                "status": job.status,
                "requested_by": job.requested_by,
                "trigger_count": job.trigger_summary.len(),
                "blocker_count": job.blocker_summary.len(),
                "candidate_model_version": &job.candidate_model_version,
                "candidate_artifact_uri": &job.candidate_artifact_uri,
                "validation_report_uri": &job.validation_report_uri,
                "output_evaluation_id": &job.output_evaluation_id,
            }),
            evidence_refs,
        })
        .await
}

async fn record_mlops_monitoring_audit(
    state: &AppState,
    actor: &ActorContext,
    model: &ModelVersionRecord,
    request: &SubmitMlopsMonitoringReportRequest,
    response: &SubmitMlopsMonitoringReportResponse,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "model.mlops_monitoring.report_submitted".into(),
            event_status: "succeeded".into(),
            summary: format!(
                "MLOps monitoring report submitted: {}",
                request.overall_status
            ),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "model_key": model.model_key,
                "model_version": model.version,
                "report_uri": request.report_uri,
                "report_kind": request.report_kind,
                "monitoring_status": request.overall_status,
                "retraining_recommendation": request.retraining_recommendation,
                "trigger_count": request.triggers.len(),
                "review_task_count": request.review_tasks.len(),
                "next_actions": response.next_actions,
                "submitted_by": request.actor,
                "note_present": !request.notes.trim().is_empty(),
                "governance_boundary": response.governance_boundary,
            }),
            evidence_refs: request
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

async fn record_mlops_alert_delivery_audit(
    state: &AppState,
    actor: &ActorContext,
    model: &ModelVersionRecord,
    request: &SubmitMlopsAlertDeliveryRequest,
    response: &SubmitMlopsAlertDeliveryResponse,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "model.mlops_alert_delivery.submitted".into(),
            event_status: "succeeded".into(),
            summary: format!(
                "MLOps alert delivery submitted: {}",
                request.alert_delivery_status
            ),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "model_key": model.model_key,
                "model_version": model.version,
                "scheduler_execution_report_uri": request.scheduler_execution_report_uri,
                "report_kind": request.report_kind,
                "alert_delivery_status": request.alert_delivery_status,
                "alert_delivery_task_count": request.alert_delivery_tasks.len(),
                "alert_routing_policy_configured": response.alert_routing_policy_configured,
                "alert_routing_policy_ref": "configured_alert_routing_policy",
                "next_actions": response.next_actions,
                "submitted_by": request.actor,
                "note_present": !request.notes.trim().is_empty(),
                "governance_boundary": response.governance_boundary,
            }),
            evidence_refs: request
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

async fn record_model_activation_audit(
    state: &AppState,
    actor: &ActorContext,
    model: &ModelVersionRecord,
    from_status: Option<&str>,
    previous_active_version: Option<&str>,
    evidence_refs: Vec<String>,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "model.activation.completed".into(),
            event_status: "succeeded".into(),
            summary: "Model activation completed".into(),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "model_key": model.model_key,
                "model_version": model.version,
                "from_status": from_status,
                "to_status": model.status,
                "previous_active_version": previous_active_version,
                "runtime_kind": model.runtime_kind,
                "execution_provider": model.execution_provider,
            }),
            evidence_refs: evidence_refs
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

async fn record_model_rollback_audit(
    state: &AppState,
    actor: &ActorContext,
    restored: &ModelVersionRecord,
    replaced_active: &ModelVersionRecord,
    restored_from_status: &str,
    evidence_refs: Vec<String>,
) -> anyhow::Result<()> {
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: "model.rollback.completed".into(),
            event_status: "succeeded".into(),
            summary: "Model rollback completed".into(),
            payload: serde_json::json!({
                "customer_scope_id": actor.customer_scope_id,
                "model_key": restored.model_key,
                "model_version": restored.version,
                "from_status": restored_from_status,
                "to_status": restored.status,
                "previous_active_version": restored.version,
                "replaced_active_version": replaced_active.version,
                "replaced_active_to_status": "approved",
                "runtime_kind": restored.runtime_kind,
                "execution_provider": restored.execution_provider,
            }),
            evidence_refs: evidence_refs
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<ActorContext, ApiError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    validate_api_key(api_key, &state.config.api_key_config()).map_err(|_| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "INVALID_API_KEY",
            "invalid api key",
        )
    })
}

fn authorize_permission(
    state: &AppState,
    headers: &HeaderMap,
    permission: &str,
) -> Result<ActorContext, ApiError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    let principal =
        authenticate_api_key(api_key, &state.config.api_key_config()).map_err(|_| {
            ApiError::new(
                StatusCode::UNAUTHORIZED,
                "INVALID_API_KEY",
                "invalid api key",
            )
        })?;
    if !principal.has_permission(permission) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "PERMISSION_DENIED",
            format!("missing permission: {permission}"),
        ));
    }
    Ok(principal.actor)
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, code, error.to_string())
}

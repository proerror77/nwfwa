use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        CompleteModelRetrainingJobInput, DatasetRecord, ModelEvaluationRecord,
        ModelPerformanceRecord, ModelPromotionReviewRecord, ModelRetrainingJobRecord,
        ModelVersionRecord, PersistedAuditEvent, QaFeedbackItemRecord,
        RegisterModelEvaluationInput,
    },
    routes::ops_datasets::build_dataset_health_record,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_audit::ActorContext;
use fwa_auth::{validate_api_key, ApiKeyConfig};
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
    pub gates: Vec<ModelPromotionGate>,
    pub blockers: Vec<String>,
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
}

#[derive(Debug, Deserialize)]
pub struct CreateModelRetrainingJobRequest {
    pub requested_by: String,
    pub notes: String,
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
    pub endpoint_url: Option<String>,
    pub validation_report_uri: String,
    pub evaluation_run_id: String,
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
    record_model_retraining_audit(&state, &actor, &job, "model.retraining.queued")
        .await
        .map_err(internal_error("MODEL_RETRAINING_AUDIT_SAVE_FAILED"))?;
    Ok(Json(job))
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
    record_model_retraining_audit(&state, &actor, &job, "model.retraining.status_updated")
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
    record_model_retraining_audit(&state, &actor, &job, "model.retraining.claimed")
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
    let evaluation = state
        .repository
        .register_model_evaluation(RegisterModelEvaluationInput {
            evaluation_run_id: request.evaluation_run_id.clone(),
            model_key: candidate_model.model_key.clone(),
            model_version: candidate_model.version.clone(),
            model_dataset_id: base_evaluation.model_dataset_id,
            auc: request.auc,
            ks: request.ks,
            precision: request.precision,
            recall: request.recall,
            f1: request.f1,
            accuracy: request.accuracy,
            threshold: request.threshold,
            confusion_matrix_json: request.confusion_matrix_json,
            feature_importance_uri: request.feature_importance_uri,
            metrics_json: request.metrics_json,
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
        .list_outcome_labels()
        .await
        .map_err(internal_error("OUTCOME_LABEL_LIST_FAILED"))?;
    let feedback_items = state
        .repository
        .list_qa_feedback_items()
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
    Ok(())
}

pub async fn submit_model_promotion_review(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(model_key): Path<String>,
    Json(request): Json<SubmitModelPromotionReviewRequest>,
) -> Result<Json<ModelPromotionReviewRecord>, ApiError> {
    let actor = authorize(&state, &headers)?;
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
    let review = state
        .repository
        .save_model_promotion_review(ModelPromotionReviewRecord {
            model_key: model.model_key.clone(),
            model_version: model.version.clone(),
            decision: request.decision,
            reviewer: request.reviewer,
            notes: request.notes,
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
) -> Result<Json<ModelLifecycleResponse>, ApiError> {
    let actor = authorize(&state, &headers)?;
    let (candidate, gates) = load_model_promotion_gates(&state, &model_key).await?;
    if candidate.status == "active" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "MODEL_ALREADY_ACTIVE",
            "latest model version is already active",
        ));
    }

    let blockers = activation_blockers(&gates);
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
    record_model_lifecycle_audit(
        &state,
        &actor,
        &activated,
        "model.activation.completed",
        Some(&candidate.status),
        "Model activation completed",
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
) -> Result<Json<ModelLifecycleResponse>, ApiError> {
    let actor = authorize(&state, &headers)?;
    let previous = state
        .repository
        .list_models()
        .await
        .map_err(internal_error("MODEL_LIST_FAILED"))?
        .into_iter()
        .find(|model| model.model_key == model_key)
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "MODEL_NOT_FOUND", "model not found")
        })?;
    if previous.status != "active" {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "MODEL_ROLLBACK_REQUIRES_ACTIVE",
            "only active models can be rolled back",
        ));
    }
    let model = state
        .repository
        .update_model_status(&previous.model_key, &previous.version, "approved")
        .await
        .map_err(internal_error("MODEL_STATUS_UPDATE_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(StatusCode::NOT_FOUND, "MODEL_NOT_FOUND", "model not found")
        })?;
    record_model_lifecycle_audit(
        &state,
        &actor,
        &model,
        "model.rollback.completed",
        Some(&previous.status),
        "Model rollback completed",
    )
    .await
    .map_err(internal_error("MODEL_AUDIT_SAVE_FAILED"))?;
    Ok(Json(ModelLifecycleResponse {
        model_key: model.model_key,
        version: model.version,
        status: model.status,
    }))
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
        .list_outcome_labels()
        .await
        .map_err(internal_error("OUTCOME_LABEL_LIST_FAILED"))?;
    let gates = build_model_promotion_gates(
        &model,
        &performance,
        &evaluations,
        &outcome_labels,
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
    let model_labels = outcome_labels
        .iter()
        .filter(|label| label.feedback_target == "models")
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
            "Drift status",
            drift_gate_passed,
            drift_blocker(drift_status),
            drift_evidence_source(drift_status),
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
        .filter(|item| item.feedback_target == "models" && item.status == "open")
        .count();
    let model_labels = outcome_labels
        .iter()
        .filter(|label| label.feedback_target == "models")
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
                "model_key": review.model_key,
                "model_version": review.model_version,
                "decision": review.decision,
                "reviewer": review.reviewer,
                "note_present": !review.notes.trim().is_empty(),
            }),
            evidence_refs: vec![serde_json::json!(format!(
                "model_versions:{}:{}",
                review.model_key, review.model_version
            ))],
        })
        .await
}

async fn record_model_retraining_audit(
    state: &AppState,
    actor: &ActorContext,
    job: &ModelRetrainingJobRecord,
    event_type: &'static str,
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
            event_type: event_type.into(),
            event_status: "succeeded".into(),
            summary: format!("Model retraining job {} is {}", job.job_id, job.status),
            payload: serde_json::json!({
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
            evidence_refs: vec![serde_json::json!(format!(
                "model_retraining_jobs:{}",
                job.job_id
            ))],
        })
        .await
}

async fn record_model_lifecycle_audit(
    state: &AppState,
    actor: &ActorContext,
    model: &ModelVersionRecord,
    event_type: &'static str,
    from_status: Option<&str>,
    summary: &'static str,
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
            event_type: event_type.into(),
            event_status: "succeeded".into(),
            summary: summary.into(),
            payload: serde_json::json!({
                "model_key": model.model_key,
                "model_version": model.version,
                "from_status": from_status,
                "to_status": model.status,
                "runtime_kind": model.runtime_kind,
                "execution_provider": model.execution_provider,
            }),
            evidence_refs: vec![serde_json::json!(format!(
                "model_versions:{}:{}",
                model.model_key, model.version
            ))],
        })
        .await
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<ActorContext, ApiError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    validate_api_key(
        api_key,
        &ApiKeyConfig {
            key: state.config.api_key.clone(),
            source_system: state.config.source_system.clone(),
        },
    )
    .map_err(|_| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "INVALID_API_KEY",
            "invalid api key",
        )
    })
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, code, error.to_string())
}

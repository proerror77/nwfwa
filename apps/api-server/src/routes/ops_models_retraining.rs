use super::ops_models::{ensure_model_exists, internal_error};
use super::ops_models_audit::{
    record_model_retraining_audit, record_model_retraining_output_audit,
    save_training_package_rule_candidates,
};
use super::ops_models_gates::build_model_retraining_readiness;
use super::ops_models_types::{
    ClaimModelRetrainingJobRequest, CompleteModelRetrainingJobRequest,
    CompleteModelRetrainingJobResponse, CreateModelRetrainingJobRequest,
    ModelRetrainingJobListResponse, ModelRetrainingReadinessResponse,
    UpdateModelRetrainingJobStatusRequest,
};
use super::ops_models_validation::{
    retraining_metrics_with_artifacts, validate_retraining_notes_without_pii,
    validate_retraining_output_request,
};
use crate::{
    app::AppState,
    auth::AuthenticatedApiPrincipal,
    error::ApiError,
    repository::{
        CompleteModelRetrainingJobInput, ModelPerformanceRecord, ModelRetrainingJobRecord,
        ModelVersionRecord, RegisterModelEvaluationInput,
    },
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use fwa_auth::AuthenticatedPrincipal;

pub async fn model_retraining_readiness(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(model_key): Path<String>,
) -> Result<Json<ModelRetrainingReadinessResponse>, ApiError> {
    let _ = require_permission(principal, "ops:models:read")?;
    Ok(Json(
        load_model_retraining_readiness(&state, &model_key).await?,
    ))
}

pub async fn list_model_retraining_jobs(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(model_key): Path<String>,
) -> Result<Json<ModelRetrainingJobListResponse>, ApiError> {
    let _ = require_permission(principal, "ops:models:read")?;
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
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(model_key): Path<String>,
    Json(request): Json<CreateModelRetrainingJobRequest>,
) -> Result<Json<ModelRetrainingJobRecord>, ApiError> {
    let actor = require_permission(principal, "ops:models:write")?;
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

pub async fn update_model_retraining_job_status(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(job_id): Path<String>,
    Json(request): Json<UpdateModelRetrainingJobStatusRequest>,
) -> Result<Json<ModelRetrainingJobRecord>, ApiError> {
    let actor = require_permission(principal, "ops:models:write")?;
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
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Json(request): Json<ClaimModelRetrainingJobRequest>,
) -> Result<Json<ModelRetrainingJobRecord>, ApiError> {
    let actor = require_permission(principal, "ops:models:write")?;
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
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path(job_id): Path<String>,
    Json(request): Json<CompleteModelRetrainingJobRequest>,
) -> Result<Json<CompleteModelRetrainingJobResponse>, ApiError> {
    let actor = require_permission(principal, "ops:models:write")?;
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
    let model_dataset_lineage = match latest_evaluation {
        Some(evaluation) => state
            .repository
            .get_model_dataset_lineage(&evaluation.model_dataset_id)
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
        model_dataset_lineage.as_ref(),
    ))
}

fn require_permission(
    principal: AuthenticatedPrincipal,
    permission: &str,
) -> Result<fwa_audit::ActorContext, ApiError> {
    if !principal.has_permission(permission) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "PERMISSION_DENIED",
            format!("missing permission: {permission}"),
        ));
    }
    Ok(principal.actor)
}

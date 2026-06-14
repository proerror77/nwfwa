use super::ops_models_audit::{
    record_model_activation_audit, record_model_promotion_audit, record_model_rollback_audit,
};
use super::ops_models_gates::{activation_blockers, build_model_promotion_gates};
pub use super::ops_models_retraining::{
    claim_next_model_retraining_job, complete_model_retraining_job, create_model_retraining_job,
    list_model_retraining_jobs, model_retraining_readiness, update_model_retraining_job_status,
};
use super::ops_models_validation::{
    validate_model_lifecycle_request, validate_model_promotion_review_request,
    validate_target_model_version_evidence,
};
use crate::{
    app::AppState,
    auth::{AuthenticatedActor, AuthenticatedApiPrincipal},
    error::ApiError,
    repository::{
        AuditEventListFilter, ModelPerformanceRecord, ModelPromotionReviewRecord,
        ModelVersionRecord,
    },
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use fwa_audit::ActorContext;
use fwa_auth::AuthenticatedPrincipal;

pub use super::ops_models_mlops::{
    mlops_alert_delivery_queue, model_monitoring_review_queue, submit_mlops_alert_delivery,
    submit_mlops_alert_delivery_task_review, submit_mlops_monitoring_report,
    submit_model_monitoring_review_task_review, submit_probability_calibration_report,
};
pub use super::ops_models_types::*;

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

pub(super) async fn ensure_model_exists(state: &AppState, model_key: &str) -> Result<(), ApiError> {
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
    validate_model_promotion_review_request(&request)?;
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
    let latest_calibration_report = state
        .repository
        .latest_probability_calibration_report(&model.model_key, &model.version)
        .await
        .map_err(internal_error("PROBABILITY_CALIBRATION_LOAD_FAILED"))?;
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
        latest_calibration_report.as_ref(),
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

pub(super) fn require_permission(
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

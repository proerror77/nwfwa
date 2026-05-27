use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        ModelEvaluationRecord, ModelPerformanceRecord, ModelPromotionReviewRecord,
        ModelVersionRecord, PersistedAuditEvent,
    },
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_audit::ActorContext;
use fwa_auth::{validate_api_key, ApiKeyConfig};
use fwa_core::{AuditEventId, ScoringRunId};
use serde::{Deserialize, Serialize};

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
    pub decision: String,
    pub passed_count: usize,
    pub total_count: usize,
    pub latest_evaluation_id: String,
    pub data_status: String,
    pub scored_runs: u32,
    pub gates: Vec<ModelPromotionGate>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitModelPromotionReviewRequest {
    pub decision: String,
    pub reviewer: String,
    pub notes: String,
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
        .model_performance(&model_key)
        .await
        .map_err(internal_error("MODEL_PERFORMANCE_FAILED"))?
        .unwrap_or_else(|| ModelPerformanceRecord {
            model_key: model_key.clone(),
            data_status: "unknown".into(),
            scored_runs: 0,
            average_score: 0.0,
            high_risk_count: 0,
            latest_scored_at: None,
        });
    let evaluations = state
        .repository
        .list_model_evaluations()
        .await
        .map_err(internal_error("MODEL_EVALUATION_LIST_FAILED"))?;
    let latest_review = state
        .repository
        .latest_model_promotion_review(&model.model_key, &model.version)
        .await
        .map_err(internal_error("MODEL_PROMOTION_REVIEW_LOAD_FAILED"))?;

    Ok(Json(build_model_promotion_gates(
        &model,
        &performance,
        &evaluations,
        latest_review.as_ref(),
    )))
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

fn build_model_promotion_gates(
    model: &ModelVersionRecord,
    performance: &ModelPerformanceRecord,
    evaluations: &[ModelEvaluationRecord],
    latest_review: Option<&ModelPromotionReviewRecord>,
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
    let approval = latest_review
        .map(|review| review.decision == "approved")
        .unwrap_or_else(|| {
            metrics
                .get("approval_status")
                .and_then(|value| value.as_str())
                == Some("approved")
        });
    let active_version = model.status == "active";

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
        data_status: performance.data_status.clone(),
        scored_runs: performance.scored_runs,
        gates,
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

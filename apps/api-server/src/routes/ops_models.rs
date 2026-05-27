use crate::{
    app::AppState,
    error::ApiError,
    repository::{ModelEvaluationRecord, ModelPerformanceRecord, ModelVersionRecord},
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_auth::{validate_api_key, ApiKeyConfig};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ModelListResponse {
    pub models: Vec<ModelVersionRecord>,
}

#[derive(Debug, Serialize)]
pub struct ModelPromotionGate {
    pub label: String,
    pub passed: bool,
    pub blocker: String,
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

    Ok(Json(build_model_promotion_gates(
        &model,
        &performance,
        &evaluations,
    )))
}

fn build_model_promotion_gates(
    model: &ModelVersionRecord,
    performance: &ModelPerformanceRecord,
    evaluations: &[ModelEvaluationRecord],
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

    let gates = vec![
        gate(
            "Immutable dataset",
            latest_evaluation
                .map(|evaluation| !evaluation.model_dataset_id.is_empty())
                .unwrap_or(false),
            "dataset version missing",
        ),
        gate(
            "Holdout metrics",
            latest_evaluation
                .map(|evaluation| {
                    evaluation.auc.is_some()
                        && evaluation.precision.is_some()
                        && evaluation.recall.is_some()
                })
                .unwrap_or(false),
            "holdout metrics missing",
        ),
        gate(
            "Out-of-time evidence",
            has_out_of_time_metric,
            "out-of-time metrics missing",
        ),
        gate(
            "Review-capacity threshold",
            latest_evaluation
                .map(|evaluation| {
                    evaluation.threshold.is_some()
                        && metrics
                            .get("review_capacity_threshold_status")
                            .and_then(|value| value.as_str())
                            == Some("passed")
                })
                .unwrap_or(false),
            "review-capacity threshold missing",
        ),
        gate(
            "Explanation artifact",
            latest_evaluation
                .and_then(|evaluation| evaluation.feature_importance_uri.as_ref())
                .is_some(),
            "feature importance missing",
        ),
        gate(
            "Leakage check",
            metrics
                .get("leakage_check_status")
                .and_then(|value| value.as_str())
                == Some("passed"),
            "leakage check missing",
        ),
        gate(
            "Shadow comparison",
            metrics
                .get("shadow_comparison_status")
                .and_then(|value| value.as_str())
                == Some("passed"),
            "shadow comparison missing",
        ),
        gate(
            "Approval",
            metrics
                .get("approval_status")
                .and_then(|value| value.as_str())
                == Some("approved"),
            "approval missing",
        ),
        gate(
            "Active version",
            model.status == "active",
            "model is not active",
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

fn gate(label: &str, passed: bool, blocker: &str) -> ModelPromotionGate {
    ModelPromotionGate {
        label: label.into(),
        passed,
        blocker: blocker.into(),
    }
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
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
    .map(|_| ())
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

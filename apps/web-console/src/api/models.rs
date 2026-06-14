use super::{get_data_sources_snapshot, request_get_json};
use crate::types::*;

pub(crate) async fn get_model_ops_snapshot(
    api_key: String,
    model_key: String,
    model_version: Option<String>,
) -> Result<ModelOpsSnapshot, String> {
    let models = request_get_json::<ModelListResponse>("/api/v1/ops/models", api_key.clone())
        .await?
        .models;
    let selected_model_key = models
        .iter()
        .find(|model| model.model_key == model_key)
        .map(|model| model.model_key.clone())
        .or_else(|| models.first().map(|model| model.model_key.clone()))
        .unwrap_or(model_key);
    let performance = request_get_json::<ModelPerformance>(
        &format!("/api/v1/ops/models/{selected_model_key}/performance"),
        api_key.clone(),
    )
    .await?;
    let selected_model_version = model_version
        .as_deref()
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .filter(|version| {
            models
                .iter()
                .any(|model| model.model_key == selected_model_key && model.version == *version)
        });
    let gates_path = if let Some(model_version) = selected_model_version {
        format!("/api/v1/ops/models/{selected_model_key}/versions/{model_version}/promotion-gates")
    } else {
        format!("/api/v1/ops/models/{selected_model_key}/promotion-gates")
    };
    let gates = request_get_json::<ModelPromotionGates>(&gates_path, api_key.clone()).await?;
    let retraining = request_get_json::<ModelRetrainingReadiness>(
        &format!("/api/v1/ops/models/{selected_model_key}/retraining-readiness"),
        api_key,
    )
    .await?;
    Ok(ModelOpsSnapshot {
        models,
        performance,
        gates,
        retraining,
    })
}

pub(crate) async fn get_mlops_workspace_snapshot(
    api_key: String,
    model_key: String,
    candidate_model_version: String,
) -> Result<MlopsWorkspaceSnapshot, String> {
    let data_sources = get_data_sources_snapshot(api_key.clone()).await?;
    let model_ops =
        get_model_ops_snapshot(api_key.clone(), model_key, Some(candidate_model_version)).await?;
    let retraining_jobs = request_get_json::<ModelRetrainingJobListResponse>(
        &format!(
            "/api/v1/ops/models/{}/retraining-jobs",
            model_ops.performance.model_key
        ),
        api_key.clone(),
    )
    .await?
    .jobs;
    let monitoring_review_tasks = request_get_json::<ModelMonitoringReviewQueueResponse>(
        &format!(
            "/api/v1/ops/models/{}/mlops-monitoring-review-queue",
            model_ops.performance.model_key
        ),
        api_key.clone(),
    )
    .await?
    .tasks;
    let alert_delivery_tasks = request_get_json::<MlopsAlertDeliveryQueueResponse>(
        &format!(
            "/api/v1/ops/models/{}/mlops-alert-delivery-queue",
            model_ops.performance.model_key
        ),
        api_key.clone(),
    )
    .await?
    .tasks;
    let anomaly_review_tasks = request_get_json::<AnomalyReviewQueueResponse>(
        "/api/v1/ops/providers/anomaly-review-queue",
        api_key,
    )
    .await?
    .tasks;
    Ok(MlopsWorkspaceSnapshot {
        data_sources,
        model_ops,
        retraining_jobs,
        monitoring_review_tasks,
        alert_delivery_tasks,
        anomaly_review_tasks,
    })
}

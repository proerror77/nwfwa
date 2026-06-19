use super::request_get_json;
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

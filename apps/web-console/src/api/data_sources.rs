use super::request_get_json;
use crate::types::*;

pub(crate) async fn get_data_sources_snapshot(
    api_key: String,
) -> Result<DataSourcesSnapshot, String> {
    let datasets =
        request_get_json::<DatasetListResponse>("/api/v1/ops/datasets", api_key.clone()).await?;
    let evaluations =
        request_get_json::<ModelEvaluationListResponse>("/api/v1/ops/model-evaluations", api_key)
            .await?;
    Ok(DataSourcesSnapshot {
        datasets: datasets.datasets,
        health: datasets.health,
        evaluations: evaluations.evaluations,
        lineage: evaluations.lineage,
    })
}

use super::{request_get_json, request_json};
use crate::types::*;
use serde_json::Value;

pub(crate) async fn get_medical_review_queue(
    api_key: String,
    limit: String,
) -> Result<Vec<MedicalReviewQueueItem>, String> {
    let limit = limit
        .trim()
        .parse::<u32>()
        .ok()
        .map(|value| value.clamp(1, 200))
        .unwrap_or(100);
    Ok(request_get_json::<MedicalReviewQueueResponse>(
        &format!("/api/v1/ops/medical-review/queue?limit={limit}"),
        api_key,
    )
    .await?
    .items)
}

pub(crate) async fn post_medical_review_result(
    api_key: String,
    payload: Value,
) -> Result<MedicalReviewResultResponse, String> {
    request_json("/api/v1/ops/medical-review/results", api_key, payload).await
}

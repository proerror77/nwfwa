use super::{request_get_json, request_json};
use crate::types::*;
use serde_json::Value;

pub(crate) async fn get_qa_review_snapshot(api_key: String) -> Result<QaReviewSnapshot, String> {
    let queue = request_get_json::<QaQueueListResponse>("/api/v1/ops/qa/queue", api_key.clone())
        .await?
        .items;
    let summary =
        request_get_json::<QaQueueSummary>("/api/v1/ops/qa/queue-summary", api_key.clone()).await?;
    let feedback_items =
        request_get_json::<QaFeedbackItemListResponse>("/api/v1/ops/qa/feedback-items", api_key)
            .await?
            .items;
    Ok(QaReviewSnapshot {
        queue,
        summary,
        feedback_items,
    })
}

pub(crate) async fn post_qa_review(
    api_key: String,
    payload: Value,
) -> Result<PilotWritebackResponse, String> {
    request_json("/api/v1/qa/results", api_key, payload).await
}

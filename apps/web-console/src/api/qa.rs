use super::request_get_json;
use crate::types::*;

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

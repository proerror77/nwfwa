use super::request_json;
use crate::types::*;
use serde_json::Value;

pub(crate) async fn normalize_claim(
    payload: Value,
    api_key: String,
) -> Result<InboxNormalizeResponse, String> {
    request_json("/api/v1/inbox/claims/normalize", api_key, payload).await
}

pub(crate) async fn score_canonical_claim(
    payload: Value,
    api_key: String,
) -> Result<ScoreResponse, String> {
    request_json("/api/v1/claims/score", api_key, payload).await
}

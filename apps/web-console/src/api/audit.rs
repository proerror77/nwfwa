use super::{request_get_json, request_json};
use crate::types::*;
use serde_json::Value;

pub(crate) async fn get_audit_samples(api_key: String) -> Result<Vec<AuditSampleRecord>, String> {
    Ok(
        request_get_json::<AuditSampleListResponse>("/api/v1/ops/audit-samples", api_key)
            .await?
            .samples,
    )
}

pub(crate) async fn post_audit_sample(
    api_key: String,
    payload: Value,
) -> Result<AuditSampleRecord, String> {
    request_json("/api/v1/ops/audit-samples", api_key, payload).await
}

pub(crate) async fn get_audit_events_for_sample(
    api_key: String,
    sample_id: String,
) -> Result<Vec<AuditEventRecord>, String> {
    let sample_id = sample_id.trim();
    if sample_id.is_empty() {
        return Err("audit sample id is required".into());
    }
    Ok(request_get_json::<AuditEventListResponse>(
        &format!("/api/v1/ops/audit-events?sample_id={sample_id}&limit=20"),
        api_key,
    )
    .await?
    .events)
}

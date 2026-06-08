use super::{request_get_json, request_json};
use crate::types::*;
use serde_json::json;

pub(crate) async fn get_bootstrap_ops_snapshot(
    api_key: String,
) -> Result<BootstrapOpsSnapshot, String> {
    let backfills = request_get_json::<HistoricalBackfillListResponse>(
        "/api/v1/ops/backfills",
        api_key.clone(),
    )
    .await?
    .jobs;
    let evidence_requests = request_get_json::<EvidenceRequestListResponse>(
        "/api/v1/ops/evidence-requests",
        api_key.clone(),
    )
    .await?
    .requests;
    let label_items = request_get_json::<LabelBootstrapQueueResponse>(
        "/api/v1/ops/label-bootstrap/queue",
        api_key,
    )
    .await?
    .items;
    Ok(BootstrapOpsSnapshot {
        backfills,
        evidence_requests,
        label_items,
    })
}

pub(crate) async fn create_bootstrap_backfill(
    api_key: String,
) -> Result<HistoricalBackfillResponse, String> {
    request_json(
        "/api/v1/ops/backfills",
        api_key,
        json!({
            "dataset_refs": ["ops:current_scoring_audit"],
            "rule_refs": ["ops:active_rule_library"],
            "reviewer": "ops-lead",
            "notes": "Create a governed replay snapshot for label handoff.",
            "limit": 25,
        }),
    )
    .await
}

pub(crate) async fn generate_bootstrap_evidence_requests(
    api_key: String,
) -> Result<EvidenceRequestGenerateResponse, String> {
    request_json(
        "/api/v1/ops/evidence-requests/generate",
        api_key,
        json!({
            "requested_by": "clinical-ops",
            "reviewer_queue": "clinical-evidence",
            "notes": "Generate missing-evidence requests from scoring audits.",
            "limit": 50,
        }),
    )
    .await
}

pub(crate) async fn mark_bootstrap_evidence_received(
    api_key: String,
    request_id: String,
    evidence_refs: Vec<String>,
    notes: String,
) -> Result<EvidenceRequestRecord, String> {
    request_json(
        &format!("/api/v1/ops/evidence-requests/{request_id}/status"),
        api_key,
        json!({
            "status": "received",
            "actor_id": "clinical-ops",
            "notes": notes,
            "evidence_refs": evidence_refs,
        }),
    )
    .await
}

pub(crate) async fn review_bootstrap_label(
    api_key: String,
    item_id: String,
    label_name: String,
    label_value: String,
    governance_status: String,
    feedback_target: String,
    notes: String,
    evidence_refs: Vec<String>,
) -> Result<LabelBootstrapReviewResponse, String> {
    request_json(
        &format!("/api/v1/ops/label-bootstrap/items/{item_id}/review"),
        api_key,
        json!({
            "reviewer": "label-governance",
            "label_name": label_name,
            "label_value": label_value,
            "governance_status": governance_status,
            "feedback_target": feedback_target,
            "notes": notes,
            "evidence_refs": evidence_refs,
        }),
    )
    .await
}

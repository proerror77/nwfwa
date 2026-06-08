use super::{request_get_json, request_json};
use crate::types::*;
use serde_json::Value;

pub(crate) async fn get_leads_cases_snapshot(
    api_key: String,
) -> Result<LeadsCasesSnapshot, String> {
    let leads = request_get_json::<LeadListResponse>("/api/v1/ops/leads", api_key.clone())
        .await?
        .leads;
    let cases = request_get_json::<CaseListResponse>("/api/v1/ops/cases", api_key)
        .await?
        .cases;
    Ok(LeadsCasesSnapshot { leads, cases })
}

pub(crate) async fn post_triage_lead(
    api_key: String,
    lead_id: String,
    payload: Value,
) -> Result<TriageLeadRecord, String> {
    request_json(
        &format!("/api/v1/ops/leads/{lead_id}/triage"),
        api_key,
        payload,
    )
    .await
}

pub(crate) async fn post_case_status(
    api_key: String,
    case_id: String,
    payload: Value,
) -> Result<UpdateCaseStatusRecord, String> {
    request_json(
        &format!("/api/v1/ops/cases/{case_id}/status"),
        api_key,
        payload,
    )
    .await
}

pub(crate) async fn post_investigation_result(
    api_key: String,
    payload: Value,
) -> Result<PilotWritebackResponse, String> {
    request_json("/api/v1/investigations/results", api_key, payload).await
}

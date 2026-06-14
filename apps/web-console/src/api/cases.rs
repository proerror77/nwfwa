use futures::join;

use super::{request_get_json, request_json};
use crate::types::*;
use serde_json::Value;

pub(crate) async fn get_leads_cases_snapshot(
    api_key: String,
) -> Result<LeadsCasesSnapshot, String> {
    let (leads_res, cases_res) = join!(
        request_get_json::<LeadListResponse>("/api/v1/ops/leads", api_key.clone()),
        request_get_json::<CaseListResponse>("/api/v1/ops/cases", api_key),
    );
    let leads = leads_res?.leads;
    let cases = cases_res?.cases;
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

pub(crate) async fn get_claim_audit_history(
    api_key: String,
    claim_id: String,
) -> Result<ClaimAuditHistoryResponse, String> {
    super::request_get_json(&format!("/api/v1/audit/claims/{claim_id}"), api_key).await
}

pub(crate) async fn get_similar_cases_for_claim(
    api_key: String,
    claim_id: String,
    scheme_family: String,
) -> Result<SimilarCasesResponse, String> {
    super::request_json(
        "/api/v1/knowledge/search-similar",
        api_key,
        serde_json::json!({
            "claim_id": claim_id,
            "scheme_family": scheme_family,
            "limit": 5
        }),
    )
    .await
}

pub(crate) async fn load_investigation_context(
    api_key: String,
    case: CaseRecord,
    leads: &[LeadRecord],
) -> InvestigationContext {
    let lead = leads.iter().find(|l| l.lead_id == case.lead_id).cloned();

    // Fetch in parallel using futures::join
    let member_fut = super::get_member_profile_summary(api_key.clone(), case.member_id.clone());
    let providers_fut = super::get_provider_risk_summary(api_key.clone());
    let audit_fut = get_claim_audit_history(api_key.clone(), case.claim_id.clone());
    let similar_fut = get_similar_cases_for_claim(
        api_key.clone(),
        case.claim_id.clone(),
        case.scheme_family.clone(),
    );

    let (member_res, providers_res, audit_res, similar_res) =
        futures::join!(member_fut, providers_fut, audit_fut, similar_fut);

    InvestigationContext {
        case,
        lead,
        member: member_res.ok(),
        providers: providers_res.map(|s| s.providers).unwrap_or_default(),
        audit_events: audit_res.map(|r| r.events).unwrap_or_default(),
        similar_cases: similar_res.map(|r| r.cases).unwrap_or_default(),
    }
}

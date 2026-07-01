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
    let tag = if scheme_family.trim().is_empty() {
        "high_risk_claim"
    } else {
        scheme_family.trim()
    };
    super::request_json(
        "/api/v1/knowledge/search-similar",
        api_key,
        serde_json::json!({
            "claim_id": claim_id,
            "diagnosis_code": "UNKNOWN",
            "provider_region": "UNKNOWN",
            "tags": [tag],
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
    let audit_fut = get_claim_audit_history(api_key.clone(), case.claim_id.clone());
    let similar_fut = get_similar_cases_for_claim(
        api_key.clone(),
        case.claim_id.clone(),
        case.scheme_family.clone(),
    );

    let (member_res, audit_res, similar_res) = futures::join!(member_fut, audit_fut, similar_fut);
    // ponytail: demo key lacks provider-read; fetch when a privileged key is used.
    let providers = if api_key.trim() == "aiclaim-demo-key" {
        Vec::new()
    } else {
        super::get_provider_risk_summary(api_key)
            .await
            .map(|summary| summary.providers)
            .unwrap_or_default()
    };

    InvestigationContext {
        case,
        lead,
        member: member_res.ok(),
        providers,
        audit_events: audit_res.map(|r| r.events).unwrap_or_default(),
        similar_cases: similar_res.map(|r| r.results).unwrap_or_default(),
    }
}

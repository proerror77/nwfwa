use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        AuditSampleLeadRecord, AuditSampleRecord, CaseRecord, InvestigationResultRecord,
        LeadRecord, MemberProfileSummaryRecord, OutcomeLabelRecord, QaFeedbackItemRecord,
        QaReviewRecord, WebhookEventRecord,
    },
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_auth::{validate_api_key, ApiKeyConfig};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PilotWritebackResponse {
    pub claim_id: String,
    pub event_type: String,
    pub event_status: String,
    pub audit_id: String,
    pub run_id: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ClaimAuditHistoryResponse {
    pub claim_id: String,
    pub events: Vec<crate::repository::AuditHistoryEventRecord>,
}

#[derive(Debug, Serialize)]
pub struct WebhookEventListResponse {
    pub events: Vec<WebhookEventRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpsAlertRecord {
    pub alert_id: String,
    pub alert_type: String,
    pub severity: String,
    pub status: String,
    pub claim_id: String,
    pub lead_id: Option<String>,
    pub case_id: Option<String>,
    pub scheme_family: String,
    pub message: String,
    pub recommended_action: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct OpsAlertListResponse {
    pub alerts: Vec<OpsAlertRecord>,
}

#[derive(Debug, Serialize)]
pub struct QaFeedbackItemListResponse {
    pub items: Vec<QaFeedbackItemRecord>,
}

#[derive(Debug, Serialize)]
pub struct QaQueueItemResponse {
    pub qa_case_id: String,
    pub sample_id: String,
    pub lead_id: String,
    pub claim_id: String,
    pub scheme_family: String,
    pub rag: String,
    pub risk_score: u8,
    pub reviewer: String,
    pub assignment_queue: String,
    pub status: String,
    pub qa_conclusion: Option<String>,
    pub issue_type: Option<String>,
    pub feedback_target: Option<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct QaQueueListResponse {
    pub items: Vec<QaQueueItemResponse>,
}

#[derive(Debug, Serialize)]
pub struct QaQueueSummaryResponse {
    pub open_count: u32,
    pub rules_feedback_count: u32,
    pub models_feedback_count: u32,
    pub tpa_feedback_count: u32,
    pub high_priority_count: u32,
    pub evidence_backed_count: u32,
    pub highest_priority: String,
}

#[derive(Debug, Serialize)]
pub struct OutcomeLabelListResponse {
    pub labels: Vec<OutcomeLabelRecord>,
}

pub async fn member_profile_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(member_id): Path<String>,
) -> Result<Json<MemberProfileSummaryRecord>, ApiError> {
    authorize(&state, &headers)?;
    let profile = state
        .repository
        .member_profile_summary(&member_id)
        .await
        .map_err(internal_error("MEMBER_PROFILE_SUMMARY_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MEMBER_NOT_FOUND",
                "member not found",
            )
        })?;
    Ok(Json(profile))
}

pub async fn write_investigation_result(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<InvestigationResultRecord>,
) -> Result<Json<PilotWritebackResponse>, ApiError> {
    authorize(&state, &headers)?;
    let claim_id = request.claim_id.clone();
    let event = state
        .repository
        .save_investigation_result(request)
        .await
        .map_err(internal_error("INVESTIGATION_RESULT_SAVE_FAILED"))?;
    Ok(Json(PilotWritebackResponse {
        claim_id,
        event_type: event.event_type,
        event_status: event.event_status,
        audit_id: event.audit_id,
        run_id: event.run_id,
        evidence_refs: event.evidence_refs,
    }))
}

pub async fn write_qa_result(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<QaReviewRecord>,
) -> Result<Json<PilotWritebackResponse>, ApiError> {
    authorize(&state, &headers)?;
    let claim_id = request.claim_id.clone();
    let event = state
        .repository
        .save_qa_review(request)
        .await
        .map_err(internal_error("QA_RESULT_SAVE_FAILED"))?;
    Ok(Json(PilotWritebackResponse {
        claim_id,
        event_type: event.event_type,
        event_status: event.event_status,
        audit_id: event.audit_id,
        run_id: event.run_id,
        evidence_refs: event.evidence_refs,
    }))
}

pub async fn list_qa_feedback_items(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<QaFeedbackItemListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let items = state
        .repository
        .list_qa_feedback_items()
        .await
        .map_err(internal_error("QA_FEEDBACK_LIST_FAILED"))?;
    Ok(Json(QaFeedbackItemListResponse { items }))
}

pub async fn list_qa_queue(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<QaQueueListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let samples = state
        .repository
        .list_audit_samples()
        .await
        .map_err(internal_error("AUDIT_SAMPLE_LIST_FAILED"))?;
    let reviews = state
        .repository
        .list_qa_reviews()
        .await
        .map_err(internal_error("QA_REVIEW_LIST_FAILED"))?;
    Ok(Json(QaQueueListResponse {
        items: build_qa_queue_items(&samples, &reviews),
    }))
}

pub async fn qa_queue_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<QaQueueSummaryResponse>, ApiError> {
    authorize(&state, &headers)?;
    let items = state
        .repository
        .list_qa_feedback_items()
        .await
        .map_err(internal_error("QA_FEEDBACK_LIST_FAILED"))?;
    Ok(Json(build_qa_queue_summary(&items)))
}

pub async fn list_outcome_labels(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<OutcomeLabelListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let labels = state
        .repository
        .list_outcome_labels()
        .await
        .map_err(internal_error("OUTCOME_LABEL_LIST_FAILED"))?;
    Ok(Json(OutcomeLabelListResponse { labels }))
}

pub async fn claim_audit_history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(claim_id): Path<String>,
) -> Result<Json<ClaimAuditHistoryResponse>, ApiError> {
    authorize(&state, &headers)?;
    let events = state
        .repository
        .claim_audit_history(&claim_id)
        .await
        .map_err(internal_error("CLAIM_AUDIT_HISTORY_FAILED"))?;
    Ok(Json(ClaimAuditHistoryResponse { claim_id, events }))
}

pub async fn list_webhook_events(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<WebhookEventListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let events = state
        .repository
        .list_webhook_events()
        .await
        .map_err(internal_error("WEBHOOK_EVENT_LIST_FAILED"))?;
    Ok(Json(WebhookEventListResponse { events }))
}

pub async fn list_ops_alerts(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<OpsAlertListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let leads = state
        .repository
        .list_leads()
        .await
        .map_err(internal_error("LEAD_LIST_FAILED"))?;
    let cases = state
        .repository
        .list_cases()
        .await
        .map_err(internal_error("CASE_LIST_FAILED"))?;
    Ok(Json(OpsAlertListResponse {
        alerts: build_ops_alerts(&leads, &cases),
    }))
}

fn build_qa_queue_items(
    samples: &[AuditSampleRecord],
    reviews: &[QaReviewRecord],
) -> Vec<QaQueueItemResponse> {
    let reviews_by_case_id = reviews
        .iter()
        .map(|review| (review.qa_case_id.as_str(), review))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut items = samples
        .iter()
        .flat_map(|sample| {
            let reviews_by_case_id = &reviews_by_case_id;
            sample.selected_leads.iter().map(move |lead| {
                let qa_case_id = qa_case_id_for_sample_lead(sample, lead);
                let review = reviews_by_case_id.get(qa_case_id.as_str()).copied();
                qa_queue_item_from_sample(sample, lead, qa_case_id, review)
            })
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        right
            .risk_score
            .cmp(&left.risk_score)
            .then_with(|| left.qa_case_id.cmp(&right.qa_case_id))
    });
    items
}

fn qa_queue_item_from_sample(
    sample: &AuditSampleRecord,
    lead: &AuditSampleLeadRecord,
    qa_case_id: String,
    review: Option<&QaReviewRecord>,
) -> QaQueueItemResponse {
    QaQueueItemResponse {
        qa_case_id,
        sample_id: sample.sample_id.clone(),
        lead_id: lead.lead_id.clone(),
        claim_id: lead.claim_id.clone(),
        scheme_family: lead.scheme_family.clone(),
        rag: lead.rag.clone(),
        risk_score: lead.risk_score,
        reviewer: sample.reviewer.clone(),
        assignment_queue: sample.assignment_queue.clone(),
        status: if review.is_some() { "reviewed" } else { "open" }.into(),
        qa_conclusion: review.map(|review| review.qa_conclusion.clone()),
        issue_type: review.map(|review| review.issue_type.clone()),
        feedback_target: review.map(|review| review.feedback_target.clone()),
        evidence_refs: lead.evidence_refs.clone(),
    }
}

fn qa_case_id_for_sample_lead(sample: &AuditSampleRecord, lead: &AuditSampleLeadRecord) -> String {
    format!("qa_{}_{}", sample.sample_id, lead.lead_id)
}

fn build_qa_queue_summary(items: &[QaFeedbackItemRecord]) -> QaQueueSummaryResponse {
    let open_items = items
        .iter()
        .filter(|item| item.status == "open")
        .collect::<Vec<_>>();
    QaQueueSummaryResponse {
        open_count: open_items.len() as u32,
        rules_feedback_count: open_items
            .iter()
            .filter(|item| item.feedback_target == "rules")
            .count() as u32,
        models_feedback_count: open_items
            .iter()
            .filter(|item| item.feedback_target == "models")
            .count() as u32,
        tpa_feedback_count: open_items
            .iter()
            .filter(|item| item.feedback_target == "tpa")
            .count() as u32,
        high_priority_count: open_items
            .iter()
            .filter(|item| item.priority == "high")
            .count() as u32,
        evidence_backed_count: open_items
            .iter()
            .filter(|item| !item.evidence_refs.is_empty())
            .count() as u32,
        highest_priority: highest_priority(&open_items).into(),
    }
}

fn build_ops_alerts(leads: &[LeadRecord], cases: &[CaseRecord]) -> Vec<OpsAlertRecord> {
    let mut alerts = leads
        .iter()
        .filter(|lead| lead.status != "triaged" && (lead.risk_score >= 70 || lead.rag == "RED"))
        .map(high_risk_routing_alert)
        .chain(
            cases
                .iter()
                .filter(|case| matches!(case.sla_status.as_str(), "breached" | "closed_breached"))
                .map(sla_breach_alert),
        )
        .collect::<Vec<_>>();
    alerts.sort_by(|left, right| {
        severity_rank(&left.severity)
            .cmp(&severity_rank(&right.severity))
            .then_with(|| left.alert_type.cmp(&right.alert_type))
            .then_with(|| left.alert_id.cmp(&right.alert_id))
    });
    alerts
}

fn high_risk_routing_alert(lead: &LeadRecord) -> OpsAlertRecord {
    OpsAlertRecord {
        alert_id: format!("alert_high_risk_{}", lead.lead_id),
        alert_type: "high_risk_routing".into(),
        severity: if lead.risk_score >= 90 || lead.rag == "RED" {
            "critical".into()
        } else {
            "high".into()
        },
        status: "open".into(),
        claim_id: lead.claim_id.clone(),
        lead_id: Some(lead.lead_id.clone()),
        case_id: None,
        scheme_family: lead.scheme_family.clone(),
        message: format!(
            "High-risk FWA lead {} for claim {} is pending triage.",
            lead.lead_id, lead.claim_id
        ),
        recommended_action: "Open an investigation case and assign reviewer ownership.".into(),
        evidence_refs: lead.evidence_refs.clone(),
    }
}

fn sla_breach_alert(case: &CaseRecord) -> OpsAlertRecord {
    OpsAlertRecord {
        alert_id: format!("alert_sla_{}", case.case_id),
        alert_type: "sla_breach".into(),
        severity: match case.priority.as_str() {
            "critical" | "high" => "critical",
            "medium" => "high",
            _ => "medium",
        }
        .into(),
        status: if case.sla_status == "closed_breached" {
            "closed".into()
        } else {
            "open".into()
        },
        claim_id: case.claim_id.clone(),
        lead_id: Some(case.lead_id.clone()),
        case_id: Some(case.case_id.clone()),
        scheme_family: case.scheme_family.clone(),
        message: format!(
            "Case {} for claim {} breached the {}h SLA target.",
            case.case_id, case.claim_id, case.sla_target_hours
        ),
        recommended_action: "Escalate the overdue case and record owner follow-up.".into(),
        evidence_refs: case_evidence_refs(case),
    }
}

fn case_evidence_refs(case: &CaseRecord) -> Vec<String> {
    let refs = case
        .evidence_package
        .get("evidence_refs")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    if refs.is_empty() {
        vec![format!("investigation_cases:{}", case.case_id)]
    } else {
        refs
    }
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "critical" => 0,
        "high" => 1,
        "medium" => 2,
        _ => 3,
    }
}

fn highest_priority(items: &[&QaFeedbackItemRecord]) -> &'static str {
    if items.iter().any(|item| item.priority == "high") {
        "high"
    } else if items.iter().any(|item| item.priority == "medium") {
        "medium"
    } else if items.iter().any(|item| item.priority == "low") {
        "low"
    } else {
        "none"
    }
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    validate_api_key(
        api_key,
        &ApiKeyConfig {
            key: state.config.api_key.clone(),
            source_system: state.config.source_system.clone(),
        },
    )
    .map(|_| ())
    .map_err(|_| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "INVALID_API_KEY",
            "invalid api key",
        )
    })
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, code, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_ops_alerts_includes_sla_breach_alerts() {
        let case = CaseRecord {
            case_id: "case_CLM-SLA-1".into(),
            lead_id: "lead_CLM-SLA-1".into(),
            claim_id: "CLM-SLA-1".into(),
            member_id: "MBR-SLA-1".into(),
            provider_id: "PRV-SLA-1".into(),
            source_system: "tpa-demo".into(),
            scheme_family: "provider_peer_outlier".into(),
            lead_source: "scoring_run".into(),
            status: "investigating".into(),
            assignee: "siu-owner".into(),
            reviewer: "medical-owner".into(),
            priority: "high".into(),
            routing_reason: "Provider peer outlier".into(),
            evidence_package: serde_json::json!({
                "evidence_refs": ["rule_runs:PROVIDER_PROFILE_HIGH", "case_workflow:overdue"]
            }),
            sla_target_hours: 24,
            sla_status: "breached".into(),
            time_to_triage_hours: 0.0,
            time_to_closure_hours: None,
        };

        let alerts = build_ops_alerts(&[], &[case]);

        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].alert_type, "sla_breach");
        assert_eq!(alerts[0].severity, "critical");
        assert_eq!(alerts[0].status, "open");
        assert_eq!(alerts[0].claim_id, "CLM-SLA-1");
        assert_eq!(alerts[0].case_id.as_deref(), Some("case_CLM-SLA-1"));
        assert_eq!(
            alerts[0].evidence_refs,
            vec![
                "rule_runs:PROVIDER_PROFILE_HIGH".to_string(),
                "case_workflow:overdue".to_string()
            ]
        );
    }
}

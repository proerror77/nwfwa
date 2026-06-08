use crate::constants::API_UNAVAILABLE_MESSAGE;
use crate::types::*;
use gloo_net::http::Request;
use serde::Deserialize;
use serde_json::Value;

mod bootstrap;
mod cases;
mod evidence;
mod models;

pub(crate) use bootstrap::*;
pub(crate) use cases::*;
pub(crate) use evidence::*;
pub(crate) use models::*;

pub(crate) async fn request_json<T>(
    path: &str,
    api_key: String,
    payload: Value,
) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let request = Request::post(path)
        .header("content-type", "application/json")
        .header("x-api-key", &api_key)
        .body(payload.to_string())
        .map_err(|error| error.to_string())?;
    let response = request.send().await.map_err(|error| error.to_string())?;
    let status = response.status();
    let body = response.text().await.map_err(|error| error.to_string())?;
    parse_json_response(path, status, &body)
}

pub(crate) async fn request_get_json<T>(path: &str, api_key: String) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let response = Request::get(path)
        .header("x-api-key", &api_key)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let body = response.text().await.map_err(|error| error.to_string())?;
    parse_json_response(path, status, &body)
}

fn parse_json_response<T>(path: &str, status: u16, body: &str) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let body = body.trim();
    if !(200..300).contains(&status) {
        return Err(api_error_message(path, status, body));
    }
    if body.is_empty() {
        return Err(API_UNAVAILABLE_MESSAGE.to_string());
    }
    let body: Value = serde_json::from_str(body)
        .map_err(|error| format!("Invalid API response from {path}: {error}"))?;
    serde_json::from_value(body).map_err(|error| error.to_string())
}

fn api_error_message(path: &str, status: u16, body: &str) -> String {
    if body.is_empty() {
        return API_UNAVAILABLE_MESSAGE.to_string();
    }
    match serde_json::from_str::<Value>(body) {
        Ok(body) => body
            .get("message")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| format!("HTTP {status}: {}", pretty_json(&body))),
        Err(_) => format!("HTTP {status} from {path}: {body}"),
    }
}

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

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

pub(crate) async fn get_dashboard_summary(api_key: String) -> Result<DashboardSummary, String> {
    request_get_json("/api/v1/ops/dashboard/summary", api_key).await
}

pub(crate) async fn get_rule_ops_snapshot(
    api_key: String,
    rule_id: String,
) -> Result<RuleOpsSnapshot, String> {
    let rules = request_get_json::<RuleListResponse>("/api/v1/ops/rules", api_key.clone())
        .await?
        .rules;
    let selected_rule_id = rules
        .iter()
        .find(|rule| rule.rule_id == rule_id)
        .map(|rule| rule.rule_id.clone())
        .or_else(|| rules.first().map(|rule| rule.rule_id.clone()))
        .unwrap_or(rule_id);
    let performance = request_get_json::<RulePerformanceResponse>(
        "/api/v1/ops/rules/performance",
        api_key.clone(),
    )
    .await?
    .rules;
    let gates = request_get_json::<RulePromotionGates>(
        &format!("/api/v1/ops/rules/{selected_rule_id}/promotion-gates"),
        api_key,
    )
    .await?;
    Ok(RuleOpsSnapshot {
        rules,
        performance,
        gates,
    })
}

pub(crate) async fn get_factor_readiness(
    api_key: String,
) -> Result<FactorReadinessResponse, String> {
    request_get_json("/api/v1/ops/factors/readiness", api_key).await
}

pub(crate) async fn get_data_sources_snapshot(
    api_key: String,
) -> Result<DataSourcesSnapshot, String> {
    let datasets =
        request_get_json::<DatasetListResponse>("/api/v1/ops/datasets", api_key.clone()).await?;
    let evaluations =
        request_get_json::<ModelEvaluationListResponse>("/api/v1/ops/model-evaluations", api_key)
            .await?;
    Ok(DataSourcesSnapshot {
        datasets: datasets.datasets,
        health: datasets.health,
        evaluations: evaluations.evaluations,
        lineage: evaluations.lineage,
    })
}

pub(crate) async fn get_member_profile_summary(
    api_key: String,
    member_id: String,
) -> Result<MemberProfileSummary, String> {
    let member_id = member_id.trim();
    if member_id.is_empty() {
        return Err("member id is required".into());
    }
    request_get_json(
        &format!("/api/v1/members/{member_id}/profile-summary"),
        api_key,
    )
    .await
}

pub(crate) async fn get_provider_risk_summary(
    api_key: String,
) -> Result<ProviderRiskSummary, String> {
    request_get_json("/api/v1/ops/providers/risk-summary", api_key).await
}

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

pub(crate) async fn get_agent_runs(api_key: String) -> Result<Vec<AgentRunRecord>, String> {
    Ok(
        request_get_json::<AgentRunListResponse>("/api/v1/ops/agent-runs", api_key)
            .await?
            .runs,
    )
}

pub(crate) async fn post_agent_investigation(
    api_key: String,
    payload: Value,
) -> Result<AgentInvestigationResponse, String> {
    request_json("/api/v1/agent/cases/investigate", api_key, payload).await
}

pub(crate) async fn get_governance_snapshot(
    api_key: String,
    event_group: String,
) -> Result<GovernanceSnapshot, String> {
    let health = request_get_json::<HealthResponse>("/api/v1/health", api_key.clone()).await?;
    let event_group = event_group.trim();
    let audit_path = if event_group.is_empty() {
        "/api/v1/ops/audit-events?limit=20".to_string()
    } else {
        format!("/api/v1/ops/audit-events?event_group={event_group}&limit=20")
    };
    let audit_events = request_get_json::<AuditEventListResponse>(&audit_path, api_key.clone())
        .await?
        .events;
    let api_calls =
        request_get_json::<ApiCallListResponse>("/api/v1/ops/api-calls?limit=20", api_key.clone())
            .await?
            .calls;
    let agent_runs = get_agent_runs(api_key).await?;
    Ok(GovernanceSnapshot {
        health,
        audit_events,
        api_calls,
        agent_runs,
    })
}

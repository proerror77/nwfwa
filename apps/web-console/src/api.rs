use crate::constants::API_UNAVAILABLE_MESSAGE;
use crate::types::*;
use gloo_net::http::Request;
use serde::Deserialize;
use serde_json::Value;

mod audit;
mod bootstrap;
mod cases;
mod evidence;
mod governance;
mod medical;
mod models;
mod rules;
mod scoring;

pub(crate) use audit::*;
pub(crate) use bootstrap::*;
pub(crate) use cases::*;
pub(crate) use evidence::*;
pub(crate) use governance::*;
pub(crate) use medical::*;
pub(crate) use models::*;
pub(crate) use rules::*;
pub(crate) use scoring::*;

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

pub(crate) async fn get_dashboard_summary(api_key: String) -> Result<DashboardSummary, String> {
    request_get_json("/api/v1/ops/dashboard/summary", api_key).await
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

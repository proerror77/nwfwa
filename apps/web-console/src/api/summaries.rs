use super::{request_get_json, request_json};
use crate::types::*;
use serde_json::json;

pub(crate) async fn get_dashboard_summary(api_key: String) -> Result<DashboardSummary, String> {
    request_get_json("/api/v1/ops/dashboard/summary", api_key).await
}

pub(crate) async fn get_factor_readiness(
    api_key: String,
) -> Result<FactorReadinessResponse, String> {
    request_get_json("/api/v1/ops/factors/readiness", api_key).await
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

/// Fetch leads+cases snapshot and extract provider-member edges for the graph
pub(crate) async fn get_graph_network_data(api_key: String) -> Result<GraphNetworkData, String> {
    // Sequential fetches — WASM futures::join! needs careful boxing
    let leads = request_get_json::<LeadListResponse>("/api/v1/ops/leads", api_key.clone())
        .await?
        .leads;
    let providers =
        request_get_json::<ProviderRiskSummary>("/api/v1/ops/providers/risk-summary", api_key)
            .await?
            .providers;
    Ok(GraphNetworkData { leads, providers })
}

/// Search similar cases for knowledge graph context
pub(crate) async fn search_knowledge_cases(
    api_key: String,
    scheme_family: String,
) -> Result<serde_json::Value, String> {
    request_json(
        "/api/v1/knowledge/search-similar",
        api_key,
        json!({ "scheme_family": scheme_family, "limit": 8 }),
    )
    .await
}

use super::request_get_json;
use crate::types::*;

pub(crate) async fn get_dashboard_summary(api_key: String) -> Result<DashboardSummary, String> {
    request_get_json("/api/v1/ops/dashboard/summary", api_key).await
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

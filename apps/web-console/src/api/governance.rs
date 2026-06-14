use futures::join;

use super::{request_get_json, request_json};
use crate::types::*;
use serde_json::Value;

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
    let event_group = event_group.trim();
    let audit_path = if event_group.is_empty() {
        "/api/v1/ops/audit-events?limit=20".to_string()
    } else {
        format!("/api/v1/ops/audit-events?event_group={event_group}&limit=20")
    };
    let (health_res, audit_res, api_calls_res, agent_runs_res) = join!(
        request_get_json::<HealthResponse>("/api/v1/health", api_key.clone()),
        request_get_json::<AuditEventListResponse>(&audit_path, api_key.clone()),
        request_get_json::<ApiCallListResponse>("/api/v1/ops/api-calls?limit=20", api_key.clone()),
        request_get_json::<AgentRunListResponse>("/api/v1/ops/agent-runs", api_key),
    );
    let health = health_res?;
    let audit_events = audit_res?.events;
    let api_calls = api_calls_res?.calls;
    let agent_runs = agent_runs_res?.runs;
    Ok(GovernanceSnapshot {
        health,
        audit_events,
        api_calls,
        agent_runs,
    })
}

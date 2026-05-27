use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        AgentApprovalRecord, AgentContextSnapshotRecord, AgentPolicyCheckRecord,
        AgentToolCallRecord, AgentToolResultRecord, PersistedAgentRun, PersistedAuditEvent,
        SimilarCaseQuery,
    },
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_agent::{DeterministicInvestigator, InvestigationRequest, SimilarCaseInput};
use fwa_auth::{validate_api_key, ApiKeyConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::hash::{Hash, Hasher};

#[derive(Debug, Deserialize)]
pub struct AgentInvestigationRequest {
    pub claim_id: String,
    pub risk_score: u8,
    pub rag: String,
    pub top_reasons: Vec<String>,
    pub similar_case_query: AgentSimilarCaseQuery,
}

#[derive(Debug, Deserialize)]
pub struct AgentSimilarCaseQuery {
    pub diagnosis_code: String,
    pub provider_region: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct AgentInvestigationResponse {
    pub agent_run_id: String,
    pub decision_boundary: String,
    pub risk_summary: String,
    pub findings: Vec<fwa_agent::InvestigationFinding>,
    pub investigation_checklist: Vec<String>,
    pub similar_cases: Vec<SimilarCaseInput>,
    pub qa_opinion_draft: String,
    pub evidence_refs: Vec<String>,
}

pub async fn investigate_case(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AgentInvestigationRequest>,
) -> Result<Json<AgentInvestigationResponse>, ApiError> {
    authorize(&state, &headers)?;
    let tool_call_id = format!("tool_call_{}", request.claim_id);
    let tool_result_id = format!("tool_result_{}", request.claim_id);
    let policy_check = AgentPolicyCheckRecord {
        policy_check_id: format!("policy_check_{}", request.claim_id),
        agent_run_id: format!("agent_{}", request.claim_id),
        tool_call_id: tool_call_id.clone(),
        tool_name: "knowledge.search_similar".into(),
        policy_name: "agent_tool_allowlist".into(),
        decision: "allowed".into(),
        reason: "Tool is allowlisted for read-only similar-case evidence retrieval.".into(),
        evidence_refs: vec![
            "policy:agent_tool_allowlist".into(),
            format!("knowledge_query:{}", request.claim_id),
        ],
        created_at: None,
    };
    let tool_input = serde_json::json!({
        "claim_id": request.claim_id,
        "diagnosis_code": request.similar_case_query.diagnosis_code,
        "provider_region": request.similar_case_query.provider_region,
        "tags": request.similar_case_query.tags,
    });
    let query_evidence_refs = vec![format!("knowledge_query:{}", request.claim_id)];
    let similar_cases = state
        .repository
        .search_similar_cases(SimilarCaseQuery {
            claim_id: Some(request.claim_id.clone()),
            diagnosis_code: request.similar_case_query.diagnosis_code.clone(),
            provider_region: request.similar_case_query.provider_region.clone(),
            tags: request.similar_case_query.tags.clone(),
        })
        .await
        .map_err(internal_error("AGENT_SIMILAR_CASE_SEARCH_FAILED"))?
        .into_iter()
        .map(|case| SimilarCaseInput {
            case_id: case.case_id,
            similarity_score: case.similarity_score,
            matched_signals: case.matched_signals,
            evidence_refs: case.evidence_refs,
        })
        .collect::<Vec<_>>();
    let result_evidence_refs = similar_cases
        .iter()
        .flat_map(|case| case.evidence_refs.iter().cloned())
        .collect::<Vec<_>>();
    let context_json = serde_json::json!({
        "claim_id": request.claim_id,
        "risk_score": request.risk_score,
        "rag": request.rag.clone(),
        "top_reasons": request.top_reasons.clone(),
        "similar_case_query": {
            "diagnosis_code": request.similar_case_query.diagnosis_code.clone(),
            "provider_region": request.similar_case_query.provider_region.clone(),
            "tags": request.similar_case_query.tags.clone(),
        }
    });
    let context_source_refs = vec![
        format!("claims:{}", request.claim_id),
        format!("risk_summary:{}", request.claim_id),
        format!("knowledge_query:{}", request.claim_id),
    ];

    let package = DeterministicInvestigator.investigate(InvestigationRequest {
        claim_id: request.claim_id.clone(),
        risk_score: request.risk_score,
        rag: request.rag,
        top_reasons: request.top_reasons,
        similar_cases,
    });
    let output_json = serde_json::to_value(&package).map_err(|error| {
        ApiError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "AGENT_ENCODE_FAILED",
            error.to_string(),
        )
    })?;
    let evidence_refs = package
        .evidence_refs
        .iter()
        .map(|reference| Value::String(reference.clone()))
        .collect::<Vec<_>>();
    let steps = package
        .findings
        .iter()
        .map(|finding| {
            serde_json::json!({
                "step_name": "evidence_finding",
                "finding": finding.finding,
                "evidence_refs": finding.evidence_refs,
            })
        })
        .collect::<Vec<_>>();

    state
        .repository
        .save_agent_run(PersistedAgentRun {
            agent_run_id: package.agent_run_id.clone(),
            claim_id: request.claim_id.clone(),
            status: "succeeded".into(),
            decision_boundary: package.decision_boundary.clone(),
            output_json: output_json.clone(),
            evidence_refs: evidence_refs.clone(),
            steps,
            context_snapshots: vec![AgentContextSnapshotRecord {
                snapshot_id: format!("snapshot_{}", package.agent_run_id),
                redaction_status: "pii_masked".into(),
                checksum: context_checksum(&context_json),
                context_json,
                source_refs: context_source_refs,
            }],
            policy_checks: vec![AgentPolicyCheckRecord {
                agent_run_id: package.agent_run_id.clone(),
                ..policy_check
            }],
            tool_calls: vec![AgentToolCallRecord {
                tool_call_id: tool_call_id.clone(),
                tool_name: "knowledge.search_similar".into(),
                status: "succeeded".into(),
                input_json: tool_input,
                evidence_refs: query_evidence_refs,
            }],
            tool_results: vec![AgentToolResultRecord {
                tool_result_id,
                tool_call_id,
                tool_name: "knowledge.search_similar".into(),
                status: "succeeded".into(),
                output_json: serde_json::json!({
                    "result_count": package.similar_cases.len(),
                    "case_ids": package
                        .similar_cases
                        .iter()
                        .map(|case| case.case_id.clone())
                        .collect::<Vec<_>>()
                }),
                evidence_refs: result_evidence_refs,
            }],
            approvals: vec![AgentApprovalRecord {
                approval_id: format!("approval_{}", package.agent_run_id),
                agent_run_id: package.agent_run_id.clone(),
                proposed_action: "manual_review_required".into(),
                decision: "pending".into(),
                approver: "unassigned".into(),
                reason: "Agent output requires human approval before downstream action.".into(),
                evidence_refs: package.evidence_refs.clone(),
                created_at: None,
            }],
        })
        .await
        .map_err(internal_error("AGENT_RUN_SAVE_FAILED"))?;

    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: format!("audit_{}", package.agent_run_id),
            run_id: package.agent_run_id.clone(),
            claim_id: request.claim_id,
            source_system: state.config.source_system.clone(),
            actor_id: "agent-case-investigator".into(),
            actor_role: "agent".into(),
            event_type: "agent.investigation.completed".into(),
            event_status: "succeeded".into(),
            summary: "Agent investigation package generated".into(),
            payload: output_json,
            evidence_refs,
        })
        .await
        .map_err(internal_error("AGENT_AUDIT_SAVE_FAILED"))?;

    Ok(Json(AgentInvestigationResponse {
        agent_run_id: package.agent_run_id,
        decision_boundary: package.decision_boundary,
        risk_summary: package.risk_summary,
        findings: package.findings,
        investigation_checklist: package.investigation_checklist,
        similar_cases: package.similar_cases,
        qa_opinion_draft: package.qa_opinion_draft,
        evidence_refs: package.evidence_refs,
    }))
}

fn context_checksum(context_json: &Value) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    context_json.to_string().hash(&mut hasher);
    format!("snapshot:{:016x}", hasher.finish())
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

fn internal_error(
    code: &'static str,
) -> impl Fn(anyhow::Error) -> ApiError + Clone + Send + Sync + 'static {
    move |error| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, code, error.to_string())
}

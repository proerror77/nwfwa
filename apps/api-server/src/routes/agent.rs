use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        AgentApprovalRecord, AgentContextSnapshotRecord, AgentPolicyCheckRecord,
        AgentToolCallRecord, AgentToolResultRecord, PersistedAgentRun, PersistedAuditEvent,
        SimilarCaseQuery,
    },
    routes::pii,
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_agent::{
    DeterministicInvestigator, EvidenceSufficiency, InvestigationRequest, SimilarCaseInput,
};
use fwa_audit::ActorContext;
use fwa_auth::validate_api_key;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::hash::{Hash, Hasher};

#[derive(Debug, Deserialize)]
pub struct AgentInvestigationRequest {
    pub claim_id: String,
    pub risk_score: u8,
    pub rag: String,
    pub scheme_family: Option<String>,
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
    pub evidence_sufficiency: EvidenceSufficiency,
    pub evidence_refs: Vec<String>,
    pub evidence_refs_by_type: fwa_agent::EvidenceReferenceBuckets,
}

pub async fn investigate_case(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AgentInvestigationRequest>,
) -> Result<Json<AgentInvestigationResponse>, ApiError> {
    let actor = authorize(&state, &headers)?;
    validate_agent_investigation_request(&request)?;
    let masked_claim_ref = mask_agent_claim_ref(&request.claim_id);
    let scheme_family = request
        .scheme_family
        .clone()
        .unwrap_or_else(|| infer_scheme_family(&request));
    let governed_top_reasons = request
        .top_reasons
        .iter()
        .map(|reason| sanitize_agent_free_text(reason))
        .collect::<Vec<_>>();
    let governed_tags = request
        .similar_case_query
        .tags
        .iter()
        .map(|tag| sanitize_agent_free_text(tag))
        .collect::<Vec<_>>();
    let agent_policy_id = state.config.agent_policy_id.clone();
    let tool_call_id = format!("tool_call_{}", masked_claim_ref);
    let tool_result_id = format!("tool_result_{}", masked_claim_ref);
    let policy_check = AgentPolicyCheckRecord {
        policy_check_id: format!("policy_check_{}", masked_claim_ref),
        agent_run_id: format!("agent_{}", masked_claim_ref),
        tool_call_id: tool_call_id.clone(),
        tool_name: "knowledge.search_similar".into(),
        policy_name: agent_policy_id.clone(),
        decision: "allowed".into(),
        reason: "Tool is allowlisted for read-only similar-case evidence retrieval.".into(),
        evidence_refs: vec![
            format!("policy:{agent_policy_id}"),
            format!("knowledge_query:{}", masked_claim_ref),
        ],
        created_at: None,
    };
    let tool_input = serde_json::json!({
        "claim_id": masked_claim_ref,
        "diagnosis_code": request.similar_case_query.diagnosis_code,
        "provider_region": request.similar_case_query.provider_region,
        "tags": governed_tags.clone(),
    });
    let query_evidence_refs = vec![format!("knowledge_query:{}", masked_claim_ref)];
    let similar_cases = state
        .repository
        .search_similar_cases(SimilarCaseQuery {
            claim_id: Some(masked_claim_ref.clone()),
            diagnosis_code: request.similar_case_query.diagnosis_code.clone(),
            provider_region: request.similar_case_query.provider_region.clone(),
            tags: governed_tags.clone(),
        })
        .await
        .map_err(internal_error("AGENT_SIMILAR_CASE_SEARCH_FAILED"))?
        .into_iter()
        .map(|case| SimilarCaseInput {
            case_id: case.case_id,
            similarity_score: case.similarity_score,
            matched_signals: case.matched_signals,
            provenance_refs: case.provenance_refs,
            evidence_refs: case.evidence_refs,
        })
        .collect::<Vec<_>>();
    let result_evidence_refs = similar_cases
        .iter()
        .flat_map(|case| {
            case.evidence_refs
                .iter()
                .chain(case.provenance_refs.iter())
                .cloned()
        })
        .collect::<Vec<_>>();
    let canonical_trace =
        latest_canonical_claim_context_trace(&state, &request.claim_id, &actor.customer_scope_id)
            .await?;
    let context_json = serde_json::json!({
        "claim_id": masked_claim_ref,
        "risk_score": request.risk_score,
        "rag": request.rag.clone(),
        "scheme_family": &scheme_family,
        "top_reasons": governed_top_reasons.clone(),
        "similar_case_query": {
            "diagnosis_code": request.similar_case_query.diagnosis_code.clone(),
            "provider_region": request.similar_case_query.provider_region.clone(),
            "tags": governed_tags,
        },
        "canonical_claim_context_trace": canonical_trace,
    });
    let mut context_source_refs = vec![
        format!("claims:{}", masked_claim_ref),
        format!("risk_summary:{}", masked_claim_ref),
        format!("knowledge_query:{}", masked_claim_ref),
    ];
    context_source_refs.extend(json_string_values(
        &context_json["canonical_claim_context_trace"]["source_refs"],
    ));

    let package = DeterministicInvestigator.investigate(InvestigationRequest {
        claim_id: masked_claim_ref,
        risk_score: request.risk_score,
        rag: request.rag,
        scheme_family,
        top_reasons: governed_top_reasons,
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
    let audit_policy_check_id = policy_check.policy_check_id.clone();
    let audit_tool_call_id = tool_call_id.clone();
    let mut audit_payload = output_json.clone();
    if let Value::Object(payload) = &mut audit_payload {
        payload.insert(
            "agent_policy_id".into(),
            Value::String(agent_policy_id.clone()),
        );
        payload.insert(
            "customer_scope_id".into(),
            Value::String(actor.customer_scope_id),
        );
        payload.insert(
            "policy_check_id".into(),
            Value::String(audit_policy_check_id),
        );
        payload.insert("tool_call_id".into(), Value::String(audit_tool_call_id));
        payload.insert(
            "tool_name".into(),
            Value::String("knowledge.search_similar".into()),
        );
    }
    let mut audit_evidence_refs = evidence_refs.clone();
    let policy_evidence_ref = Value::String(format!("policy:{agent_policy_id}"));
    if !audit_evidence_refs.contains(&policy_evidence_ref) {
        audit_evidence_refs.push(policy_evidence_ref);
    }

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
            payload: audit_payload,
            evidence_refs: audit_evidence_refs,
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
        evidence_sufficiency: package.evidence_sufficiency,
        evidence_refs: package.evidence_refs,
        evidence_refs_by_type: package.evidence_refs_by_type,
    }))
}

fn validate_agent_investigation_request(
    request: &AgentInvestigationRequest,
) -> Result<(), ApiError> {
    if request.claim_id.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_AGENT_CLAIM_ID",
            "claim_id is required",
        ));
    }
    if request.risk_score > 100 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_AGENT_RISK_SCORE",
            "risk_score must be between 0 and 100",
        ));
    }
    if !matches!(request.rag.as_str(), "GREEN" | "AMBER" | "RED") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_AGENT_RAG",
            "rag must be GREEN, AMBER, or RED",
        ));
    }
    if request.top_reasons.is_empty()
        || request
            .top_reasons
            .iter()
            .any(|reason| reason.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_AGENT_TOP_REASONS",
            "at least one non-empty top reason is required",
        ));
    }
    if request.similar_case_query.diagnosis_code.trim().is_empty()
        || request.similar_case_query.provider_region.trim().is_empty()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_AGENT_SIMILAR_CASE_QUERY",
            "diagnosis_code and provider_region are required",
        ));
    }
    if request.similar_case_query.tags.is_empty()
        || request
            .similar_case_query
            .tags
            .iter()
            .any(|tag| tag.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_AGENT_SIMILAR_CASE_QUERY",
            "at least one non-empty similar case tag is required",
        ));
    }
    Ok(())
}

async fn latest_canonical_claim_context_trace(
    state: &AppState,
    claim_id: &str,
    customer_scope_id: &str,
) -> Result<Value, ApiError> {
    let events = state
        .repository
        .claim_audit_history(claim_id, Some(customer_scope_id))
        .await
        .map_err(internal_error("AGENT_CANONICAL_TRACE_LOOKUP_FAILED"))?;
    Ok(events
        .iter()
        .rev()
        .find_map(|event| {
            if event.event_type == "scoring.completed" && event.event_status == "succeeded" {
                event
                    .payload
                    .get("canonical_claim_context_trace")
                    .and_then(Value::as_object)
                    .map(|_| event.payload["canonical_claim_context_trace"].clone())
            } else {
                None
            }
        })
        .unwrap_or(Value::Null))
}

fn json_string_values(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn infer_scheme_family(request: &AgentInvestigationRequest) -> String {
    let text = request
        .top_reasons
        .iter()
        .chain(request.similar_case_query.tags.iter())
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    if text.contains("provider") {
        "provider_peer_outlier".into()
    } else if text.contains("diagnosis")
        || text.contains("medical")
        || text.contains("诊断")
        || text.contains("项目")
    {
        "diagnosis_procedure_mismatch".into()
    } else if text.contains("lab") {
        "laboratory_testing_abuse".into()
    } else if text.contains("early") || text.contains("high_amount") {
        "early_high_value_claim".into()
    } else {
        "high_risk_claim".into()
    }
}

fn context_checksum(context_json: &Value) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    context_json.to_string().hash(&mut hasher);
    format!("snapshot:{:016x}", hasher.finish())
}

fn mask_agent_claim_ref(claim_id: &str) -> String {
    format!("masked:claim:{:016x}", stable_fnv1a64("claim", claim_id))
}

fn sanitize_unconfirmed_fraud_language(value: &str) -> String {
    [
        ("confirmed fraud", "suspected FWA risk"),
        ("confirmed FWA", "suspected FWA risk"),
        ("已确认欺诈", "疑似 FWA 风险"),
        ("确认欺诈", "疑似 FWA 风险"),
        ("确认为欺诈", "疑似 FWA 风险"),
    ]
    .into_iter()
    .fold(value.to_string(), |current, (needle, replacement)| {
        replace_case_insensitive(&current, needle, replacement)
    })
}

fn sanitize_agent_free_text(value: &str) -> String {
    pii::redact_text(&sanitize_unconfirmed_fraud_language(value))
}

fn replace_case_insensitive(value: &str, needle: &str, replacement: &str) -> String {
    let mut result = String::new();
    let lower_value = value.to_ascii_lowercase();
    let lower_needle = needle.to_ascii_lowercase();
    let mut cursor = 0;
    while let Some(offset) = lower_value[cursor..].find(&lower_needle) {
        let start = cursor + offset;
        result.push_str(&value[cursor..start]);
        result.push_str(replacement);
        cursor = start + needle.len();
    }
    result.push_str(&value[cursor..]);
    result
}

fn stable_fnv1a64(scope: &str, value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in scope.bytes().chain([0xff]).chain(value.bytes()) {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<ActorContext, ApiError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    validate_api_key(api_key, &state.config.api_key_config()).map_err(|_| {
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

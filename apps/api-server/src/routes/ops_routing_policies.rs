use crate::{
    app::AppState,
    error::ApiError,
    repository::{PersistedAuditEvent, RoutingPolicyRecord},
    routes::pii,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_audit::ActorContext;
use fwa_auth::{validate_api_key, ApiKeyConfig};
use fwa_core::{AuditEventId, ScoringRunId};
use fwa_scoring::RoutingPolicy;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct RoutingPolicyListResponse {
    pub policies: Vec<RoutingPolicyRecord>,
}

#[derive(Debug, Serialize)]
pub struct RoutingPolicyPromotionGate {
    pub label: String,
    pub passed: bool,
    pub blocker: String,
    pub evidence_source: String,
}

#[derive(Debug, Serialize)]
pub struct RoutingPolicyPromotionGatesResponse {
    pub policy_id: String,
    pub version: u32,
    pub review_mode: String,
    pub status: String,
    pub decision: String,
    pub passed_count: usize,
    pub total_count: usize,
    pub gates: Vec<RoutingPolicyPromotionGate>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SaveRoutingPolicyCandidateRequest {
    pub policy: RoutingPolicy,
    pub owner: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RoutingPolicyLifecycleRequest {
    pub evidence_refs: Vec<String>,
}

pub async fn list_routing_policies(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<RoutingPolicyListResponse>, ApiError> {
    let _actor = authorize(&state, &headers)?;
    let policies = state
        .repository
        .list_routing_policies()
        .await
        .map_err(internal_error("ROUTING_POLICY_LIST_FAILED"))?;
    Ok(Json(RoutingPolicyListResponse { policies }))
}

pub async fn routing_policy_promotion_gates(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((policy_id, review_mode, version)): Path<(String, String, u32)>,
) -> Result<Json<RoutingPolicyPromotionGatesResponse>, ApiError> {
    let _actor = authorize(&state, &headers)?;
    let record = load_routing_policy(&state, &policy_id, version, &review_mode).await?;
    Ok(Json(build_routing_policy_promotion_gates(&record)))
}

pub async fn save_routing_policy_candidate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SaveRoutingPolicyCandidateRequest>,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
    let actor = authorize(&state, &headers)?;
    validate_routing_policy_candidate(&request)?;
    let owner = request.owner.unwrap_or_else(|| "policy-ops".into());
    if let Some(existing) = state
        .repository
        .get_routing_policy(
            &request.policy.policy_id,
            request.policy.version,
            &request.policy.review_mode,
        )
        .await
        .map_err(internal_error("ROUTING_POLICY_LOAD_FAILED"))?
    {
        if existing.status == "active" {
            return Err(ApiError::new(
                StatusCode::CONFLICT,
                "ROUTING_POLICY_ACTIVE_OVERWRITE_BLOCKED",
                "active routing policy versions cannot be overwritten as draft",
            ));
        }
    }
    let record = state
        .repository
        .save_routing_policy_candidate(request.policy, owner)
        .await
        .map_err(internal_error("ROUTING_POLICY_CANDIDATE_SAVE_FAILED"))?;
    record_routing_policy_audit(
        &state,
        &actor,
        RoutingPolicyAuditInput {
            record: &record,
            event_type: "routing_policy.candidate.saved",
            from_status: None,
            summary: "Routing policy candidate saved",
            evidence_refs: default_routing_policy_evidence_refs(&record),
        },
    )
    .await
    .map_err(internal_error("ROUTING_POLICY_AUDIT_SAVE_FAILED"))?;
    Ok(Json(record))
}

fn validate_routing_policy_candidate(
    request: &SaveRoutingPolicyCandidateRequest,
) -> Result<(), ApiError> {
    if request.policy.policy_id.trim().is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ROUTING_POLICY_ID",
            "policy_id is required",
        ));
    }
    if request.policy.version == 0 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ROUTING_POLICY_VERSION",
            "version must be greater than zero",
        ));
    }
    if !matches!(
        request.policy.review_mode.as_str(),
        "pre_payment" | "post_payment" | "both"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ROUTING_POLICY_REVIEW_MODE",
            "review_mode must be pre_payment, post_payment, or both",
        ));
    }
    if request
        .owner
        .as_ref()
        .is_some_and(|owner| owner.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_ROUTING_POLICY_OWNER",
            "owner must not be blank when provided",
        ));
    }
    Ok(())
}

pub async fn submit_routing_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((policy_id, review_mode, version)): Path<(String, String, u32)>,
    Json(request): Json<RoutingPolicyLifecycleRequest>,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
    validate_routing_policy_lifecycle_request(&request)?;
    update_routing_policy_status(
        state,
        headers,
        RoutingPolicyStatusChange {
            policy_id,
            version,
            review_mode,
            required_status: "draft",
            next_status: "submitted",
            evidence_refs: request.evidence_refs,
        },
    )
    .await
}

pub async fn approve_routing_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((policy_id, review_mode, version)): Path<(String, String, u32)>,
    Json(request): Json<RoutingPolicyLifecycleRequest>,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
    validate_routing_policy_lifecycle_request(&request)?;
    update_routing_policy_status(
        state,
        headers,
        RoutingPolicyStatusChange {
            policy_id,
            version,
            review_mode,
            required_status: "submitted",
            next_status: "approved",
            evidence_refs: request.evidence_refs,
        },
    )
    .await
}

pub async fn activate_routing_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((policy_id, review_mode, version)): Path<(String, String, u32)>,
    Json(request): Json<RoutingPolicyLifecycleRequest>,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
    validate_routing_policy_lifecycle_request(&request)?;
    let actor = authorize(&state, &headers)?;
    let previous = load_routing_policy(&state, &policy_id, version, &review_mode).await?;
    require_status(&previous, "approved", "ROUTING_POLICY_APPROVAL_REQUIRED")?;
    let gates = build_routing_policy_promotion_gates(&previous);
    let blockers = activation_blockers(&gates);
    if !blockers.is_empty() {
        return Err(ApiError::new(
            StatusCode::CONFLICT,
            "ROUTING_POLICY_PROMOTION_GATES_BLOCKED",
            format!(
                "routing policy {}:{}:{} promotion gates blocked: {}",
                previous.policy_id,
                previous.review_mode,
                previous.version,
                blockers.join(", ")
            ),
        ));
    }
    let record = state
        .repository
        .activate_routing_policy(&policy_id, version, &review_mode)
        .await
        .map_err(internal_error("ROUTING_POLICY_STATUS_UPDATE_FAILED"))?
        .ok_or_else(routing_policy_not_found)?;
    record_routing_policy_audit(
        &state,
        &actor,
        RoutingPolicyAuditInput {
            record: &record,
            event_type: "routing_policy.activation.completed",
            from_status: Some(&previous.status),
            summary: "Routing policy activation completed",
            evidence_refs: request.evidence_refs,
        },
    )
    .await
    .map_err(internal_error("ROUTING_POLICY_AUDIT_SAVE_FAILED"))?;
    Ok(Json(record))
}

pub async fn rollback_routing_policy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((policy_id, review_mode, version)): Path<(String, String, u32)>,
    Json(request): Json<RoutingPolicyLifecycleRequest>,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
    validate_routing_policy_lifecycle_request(&request)?;
    update_routing_policy_status(
        state,
        headers,
        RoutingPolicyStatusChange {
            policy_id,
            version,
            review_mode,
            required_status: "active",
            next_status: "approved",
            evidence_refs: request.evidence_refs,
        },
    )
    .await
}

fn validate_routing_policy_lifecycle_request(
    request: &RoutingPolicyLifecycleRequest,
) -> Result<(), ApiError> {
    if request.evidence_refs.is_empty()
        || request
            .evidence_refs
            .iter()
            .any(|reference| reference.trim().is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "MISSING_ROUTING_POLICY_LIFECYCLE_EVIDENCE",
            "routing policy lifecycle evidence_refs are required",
        ));
    }
    if pii::contains_pii(request.evidence_refs.iter().map(String::as_str)) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_ROUTING_POLICY_LIFECYCLE",
            "routing policy lifecycle evidence_refs must not contain PII",
        ));
    }
    Ok(())
}

async fn update_routing_policy_status(
    state: AppState,
    headers: HeaderMap,
    change: RoutingPolicyStatusChange,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
    let actor = authorize(&state, &headers)?;
    let previous = load_routing_policy(
        &state,
        &change.policy_id,
        change.version,
        &change.review_mode,
    )
    .await?;
    require_status(
        &previous,
        change.required_status,
        "ROUTING_POLICY_STATUS_REQUIRED",
    )?;
    let record = state
        .repository
        .update_routing_policy_status(
            &change.policy_id,
            change.version,
            &change.review_mode,
            change.next_status,
        )
        .await
        .map_err(internal_error("ROUTING_POLICY_STATUS_UPDATE_FAILED"))?
        .ok_or_else(routing_policy_not_found)?;
    record_routing_policy_audit(
        &state,
        &actor,
        RoutingPolicyAuditInput {
            record: &record,
            event_type: if change.next_status == "approved" && change.required_status == "active" {
                "routing_policy.rollback.completed"
            } else {
                "routing_policy.status.changed"
            },
            from_status: Some(&previous.status),
            summary: if change.next_status == "approved" && change.required_status == "active" {
                "Routing policy rollback completed"
            } else {
                "Routing policy status changed"
            },
            evidence_refs: change.evidence_refs,
        },
    )
    .await
    .map_err(internal_error("ROUTING_POLICY_AUDIT_SAVE_FAILED"))?;
    Ok(Json(record))
}

struct RoutingPolicyStatusChange {
    policy_id: String,
    version: u32,
    review_mode: String,
    required_status: &'static str,
    next_status: &'static str,
    evidence_refs: Vec<String>,
}

fn build_routing_policy_promotion_gates(
    record: &RoutingPolicyRecord,
) -> RoutingPolicyPromotionGatesResponse {
    let approved = record.status == "approved" || record.status == "active";
    let risk_thresholds = record.risk_thresholds.low_max < record.risk_thresholds.medium_min
        && record.risk_thresholds.medium_min <= record.risk_thresholds.high_min
        && record.risk_thresholds.high_min <= record.risk_thresholds.critical_min;
    let confidence_thresholds = record.confidence_thresholds.low_confidence_below
        < record.confidence_thresholds.high_confidence_min;
    let provider_threshold = record.provider_review_threshold >= record.risk_thresholds.high_min
        && record.provider_review_threshold <= record.risk_thresholds.critical_min;
    let gates = vec![
        promotion_gate(
            "Governance approval",
            approved,
            "approval missing",
            if approved { "metadata" } else { "approval" },
        ),
        promotion_gate(
            "Risk thresholds",
            risk_thresholds,
            "risk thresholds must satisfy low < medium <= high <= critical",
            "policy_json",
        ),
        promotion_gate(
            "Confidence thresholds",
            confidence_thresholds,
            "confidence thresholds must satisfy low confidence < high confidence",
            "policy_json",
        ),
        promotion_gate(
            "Provider review threshold",
            provider_threshold,
            "provider review threshold must sit between high and critical risk thresholds",
            "policy_json",
        ),
    ];
    let blockers = gates
        .iter()
        .filter(|gate| !gate.passed)
        .map(|gate| gate.blocker.clone())
        .collect::<Vec<_>>();

    RoutingPolicyPromotionGatesResponse {
        policy_id: record.policy_id.clone(),
        version: record.version,
        review_mode: record.review_mode.clone(),
        status: record.status.clone(),
        decision: if blockers.is_empty() {
            "activation_allowed".into()
        } else {
            "activation_blocked".into()
        },
        passed_count: gates.len() - blockers.len(),
        total_count: gates.len(),
        gates,
        blockers,
    }
}

fn activation_blockers(gates: &RoutingPolicyPromotionGatesResponse) -> Vec<String> {
    gates.blockers.clone()
}

fn promotion_gate(
    label: &str,
    passed: bool,
    blocker: &str,
    evidence_source: &str,
) -> RoutingPolicyPromotionGate {
    RoutingPolicyPromotionGate {
        label: label.into(),
        passed,
        blocker: blocker.into(),
        evidence_source: evidence_source.into(),
    }
}

async fn load_routing_policy(
    state: &AppState,
    policy_id: &str,
    version: u32,
    review_mode: &str,
) -> Result<RoutingPolicyRecord, ApiError> {
    state
        .repository
        .get_routing_policy(policy_id, version, review_mode)
        .await
        .map_err(internal_error("ROUTING_POLICY_LOAD_FAILED"))?
        .ok_or_else(routing_policy_not_found)
}

fn require_status(
    record: &RoutingPolicyRecord,
    required_status: &'static str,
    code: &'static str,
) -> Result<(), ApiError> {
    if record.status == required_status {
        return Ok(());
    }
    Err(ApiError::new(
        StatusCode::CONFLICT,
        code,
        format!(
            "routing policy {}:{}:{} must be {required_status}",
            record.policy_id, record.review_mode, record.version
        ),
    ))
}

fn routing_policy_not_found() -> ApiError {
    ApiError::new(
        StatusCode::NOT_FOUND,
        "ROUTING_POLICY_NOT_FOUND",
        "routing policy not found",
    )
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<ActorContext, ApiError> {
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
    .map_err(|_| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "INVALID_API_KEY",
            "invalid api key",
        )
    })
}

struct RoutingPolicyAuditInput<'a> {
    record: &'a RoutingPolicyRecord,
    event_type: &'static str,
    from_status: Option<&'a str>,
    summary: &'static str,
    evidence_refs: Vec<String>,
}

async fn record_routing_policy_audit(
    state: &AppState,
    actor: &ActorContext,
    input: RoutingPolicyAuditInput<'_>,
) -> anyhow::Result<()> {
    let payload = serde_json::json!({
        "policy_id": &input.record.policy_id,
        "version": input.record.version,
        "review_mode": &input.record.review_mode,
        "from_status": input.from_status,
        "to_status": &input.record.status,
        "owner": &input.record.owner,
        "risk_thresholds": &input.record.risk_thresholds,
        "confidence_thresholds": &input.record.confidence_thresholds,
        "provider_review_threshold": input.record.provider_review_threshold,
    });
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: input.event_type.into(),
            event_status: "succeeded".into(),
            summary: input.summary.into(),
            payload,
            evidence_refs: input.evidence_refs.into_iter().map(Value::String).collect(),
        })
        .await
}

fn default_routing_policy_evidence_refs(record: &RoutingPolicyRecord) -> Vec<String> {
    vec![format!(
        "routing_policies:{}:v{}:{}",
        record.policy_id, record.version, record.review_mode
    )]
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, code, error.to_string())
}

use super::ops_routing_policies::{
    internal_error, record_routing_policy_audit, require_permission, RoutingPolicyAuditInput,
    RoutingPolicyLifecycleRequest, RoutingPolicyPromotionGate, RoutingPolicyPromotionGatesResponse,
};
use crate::{
    app::AppState, auth::AuthenticatedApiPrincipal, error::ApiError,
    repository::RoutingPolicyRecord, routes::pii,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

pub async fn routing_policy_promotion_gates(
    State(state): State<AppState>,
    _principal: AuthenticatedApiPrincipal,
    Path((policy_id, review_mode, version)): Path<(String, String, u32)>,
) -> Result<Json<RoutingPolicyPromotionGatesResponse>, ApiError> {
    let record = load_routing_policy(&state, &policy_id, version, &review_mode).await?;
    Ok(Json(build_routing_policy_promotion_gates(&record)))
}

pub async fn submit_routing_policy(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path((policy_id, review_mode, version)): Path<(String, String, u32)>,
    Json(request): Json<RoutingPolicyLifecycleRequest>,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
    validate_routing_policy_lifecycle_request(&request)?;
    let actor = require_permission(principal, "ops:routing:write")?;
    update_routing_policy_status(
        state,
        actor,
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
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path((policy_id, review_mode, version)): Path<(String, String, u32)>,
    Json(request): Json<RoutingPolicyLifecycleRequest>,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
    validate_routing_policy_lifecycle_request(&request)?;
    let actor = require_permission(principal, "ops:routing:approve")?;
    update_routing_policy_status(
        state,
        actor,
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
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path((policy_id, review_mode, version)): Path<(String, String, u32)>,
    Json(request): Json<RoutingPolicyLifecycleRequest>,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
    validate_routing_policy_lifecycle_request(&request)?;
    let actor = require_permission(principal, "ops:routing:activate")?;
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
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Path((policy_id, review_mode, version)): Path<(String, String, u32)>,
    Json(request): Json<RoutingPolicyLifecycleRequest>,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
    validate_routing_policy_lifecycle_request(&request)?;
    let actor = require_permission(principal, "ops:routing:rollback")?;
    update_routing_policy_status(
        state,
        actor,
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
    actor: fwa_audit::ActorContext,
    change: RoutingPolicyStatusChange,
) -> Result<Json<RoutingPolicyRecord>, ApiError> {
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

pub(super) async fn load_routing_policy(
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

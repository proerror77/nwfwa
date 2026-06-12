use super::claims::{
    ClaimItemPayload, DocumentPayload, FullClaimPayload, MemberPayload, PolicyPayload,
    ProviderPayload, ProviderProfilePayload, ProviderProfileWindowPayload,
    ProviderRelationshipGraphPayload, ScoreClaimRequest,
};
use crate::error::ApiError;
use fwa_audit::ActorContext;
use rust_decimal::Decimal;

pub(super) fn validate_score_request_contract(request: &ScoreClaimRequest) -> Result<(), ApiError> {
    require_nonblank(&request.source_system, "source_system")?;
    if let Some(claim_id) = &request.claim_id {
        require_nonblank(claim_id, "claim_id")?;
    }
    if let Some(peer_percentile) = request.claim_amount_peer_percentile {
        require_percentile(peer_percentile, "claim_amount_peer_percentile")?;
    }
    if let Some(inbox_run_id) = &request.inbox_run_id {
        require_nonblank(inbox_run_id, "inbox_run_id")?;
    }
    if let Some(inbox_idempotency_key) = &request.inbox_idempotency_key {
        require_nonblank(inbox_idempotency_key, "inbox_idempotency_key")?;
    }
    if request.inbox_run_id.is_some() && request.inbox_idempotency_key.is_some() {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            "AMBIGUOUS_SCORE_REQUEST",
            "only one inbox handoff locator is allowed",
        ));
    }
    if let Some(claim) = &request.claim {
        validate_full_claim_payload(claim)?;
    }
    if let Some(items) = &request.items {
        for item in items {
            validate_claim_item_payload(item)?;
        }
    }
    if let Some(member) = &request.member {
        validate_member_payload(member)?;
    }
    if let Some(policy) = &request.policy {
        validate_policy_payload(policy)?;
    }
    if let Some(provider) = &request.provider {
        validate_provider_payload(provider)?;
    }
    if let Some(documents) = &request.documents {
        for document in documents {
            validate_document_payload(document)?;
        }
    }
    if let Some(provider_profile) = &request.provider_profile {
        validate_provider_profile_payload(provider_profile)?;
    }
    if let Some(provider_relationships) = &request.provider_relationships {
        validate_provider_relationship_graph_payload(provider_relationships)?;
    }
    Ok(())
}

pub(super) fn validate_source_system_matches_actor(
    request: &ScoreClaimRequest,
    actor: &ActorContext,
) -> Result<(), ApiError> {
    if request.source_system == actor.source_system {
        Ok(())
    } else {
        Err(ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            "SOURCE_SYSTEM_MISMATCH",
            "source_system must match authenticated API key source system",
        ))
    }
}

fn validate_full_claim_payload(payload: &FullClaimPayload) -> Result<(), ApiError> {
    require_nonblank(&payload.external_claim_id, "claim.external_claim_id")?;
    require_positive_decimal(payload.claim_amount, "claim.claim_amount")?;
    require_nonblank(&payload.currency, "claim.currency")?;
    if let Some(diagnosis_code) = &payload.diagnosis_code {
        require_nonblank(diagnosis_code, "claim.diagnosis_code")?;
    }
    if let Some(peer_percentile) = payload.claim_amount_peer_percentile {
        require_percentile(peer_percentile, "claim.claim_amount_peer_percentile")?;
    }
    if let Some(items) = &payload.items {
        for item in items {
            validate_claim_item_payload(item)?;
        }
    }
    if let Some(member) = &payload.member {
        validate_member_payload(member)?;
    }
    if let Some(policy) = &payload.policy {
        validate_policy_payload(policy)?;
    }
    if let Some(provider) = &payload.provider {
        validate_provider_payload(provider)?;
    }
    if let Some(documents) = &payload.documents {
        for document in documents {
            validate_document_payload(document)?;
        }
    }
    if let Some(provider_profile) = &payload.provider_profile {
        validate_provider_profile_payload(provider_profile)?;
    }
    if let Some(provider_relationships) = &payload.provider_relationships {
        validate_provider_relationship_graph_payload(provider_relationships)?;
    }
    Ok(())
}

fn validate_claim_item_payload(payload: &ClaimItemPayload) -> Result<(), ApiError> {
    require_nonblank(&payload.item_code, "item.item_code")?;
    require_nonblank(&payload.item_type, "item.item_type")?;
    require_nonblank(&payload.description, "item.description")?;
    if payload.quantity == 0 {
        return invalid_score_field("item.quantity");
    }
    require_nonnegative_decimal(payload.unit_amount, "item.unit_amount")?;
    require_nonnegative_decimal(payload.total_amount, "item.total_amount")?;
    if let Some(currency) = &payload.currency {
        require_nonblank(currency, "item.currency")?;
    }
    Ok(())
}

fn validate_member_payload(payload: &MemberPayload) -> Result<(), ApiError> {
    require_nonblank(&payload.external_member_id, "member.external_member_id")
}

fn validate_policy_payload(payload: &PolicyPayload) -> Result<(), ApiError> {
    require_nonblank(&payload.external_policy_id, "policy.external_policy_id")?;
    if let Some(product_code) = &payload.product_code {
        require_nonblank(product_code, "policy.product_code")?;
    }
    if payload.coverage_end_date < payload.coverage_start_date {
        return invalid_score_field("policy.coverage_end_date");
    }
    require_positive_decimal(payload.coverage_limit, "policy.coverage_limit")?;
    if let Some(currency) = &payload.currency {
        require_nonblank(currency, "policy.currency")?;
    }
    Ok(())
}

fn validate_provider_payload(payload: &ProviderPayload) -> Result<(), ApiError> {
    require_nonblank(
        &payload.external_provider_id,
        "provider.external_provider_id",
    )?;
    require_nonblank(&payload.name, "provider.name")?;
    require_nonblank(&payload.provider_type, "provider.provider_type")?;
    require_nonblank(&payload.region, "provider.region")
}

fn validate_document_payload(payload: &DocumentPayload) -> Result<(), ApiError> {
    require_nonblank(
        &payload.external_document_id,
        "document.external_document_id",
    )?;
    require_nonblank(&payload.document_type, "document.document_type")?;
    if let Some(linked_item_codes) = &payload.linked_item_codes {
        for item_code in linked_item_codes {
            require_nonblank(item_code, "document.linked_item_codes")?;
        }
    }
    Ok(())
}

fn validate_provider_profile_payload(payload: &ProviderProfilePayload) -> Result<(), ApiError> {
    if let Some(specialty) = &payload.specialty {
        require_nonblank(specialty, "provider_profile.specialty")?;
    }
    if let Some(network_status) = &payload.network_status {
        require_nonblank(network_status, "provider_profile.network_status")?;
    }
    if payload.windows.is_empty() {
        return invalid_score_field("provider_profile.windows");
    }
    for window in &payload.windows {
        validate_provider_profile_window_payload(window)?;
    }
    Ok(())
}

fn validate_provider_profile_window_payload(
    payload: &ProviderProfileWindowPayload,
) -> Result<(), ApiError> {
    if !matches!(payload.window_days, 30 | 90 | 365) {
        return invalid_score_field("provider_profile.windows.window_days");
    }
    require_nonnegative_decimal(
        payload.total_claim_amount,
        "provider_profile.windows.total_claim_amount",
    )?;
    require_unit_interval(
        payload.high_cost_item_ratio,
        "provider_profile.windows.high_cost_item_ratio",
    )?;
    require_unit_interval(
        payload.diagnosis_procedure_mismatch_rate,
        "provider_profile.windows.diagnosis_procedure_mismatch_rate",
    )?;
    require_percentile(
        payload.peer_amount_percentile,
        "provider_profile.windows.peer_amount_percentile",
    )?;
    require_percentile(
        payload.peer_frequency_percentile,
        "provider_profile.windows.peer_frequency_percentile",
    )
}

fn validate_provider_relationship_graph_payload(
    payload: &ProviderRelationshipGraphPayload,
) -> Result<(), ApiError> {
    require_unit_interval(
        payload.high_risk_neighbor_ratio,
        "provider_relationships.high_risk_neighbor_ratio",
    )?;
    require_unit_interval(
        payload.provider_patient_overlap_score,
        "provider_relationships.provider_patient_overlap_score",
    )?;
    if let Some(referral_concentration_score) = payload.referral_concentration_score {
        require_unit_interval(
            referral_concentration_score,
            "provider_relationships.referral_concentration_score",
        )?;
    }
    if let Some(temporal_co_billing_score) = payload.temporal_co_billing_score {
        require_unit_interval(
            temporal_co_billing_score,
            "provider_relationships.temporal_co_billing_score",
        )?;
    }
    if let Some(network_component_risk_score) = payload.network_component_risk_score {
        require_percentile(
            network_component_risk_score,
            "provider_relationships.network_component_risk_score",
        )?;
    }
    if let Some(evidence_refs) = &payload.evidence_refs {
        for evidence_ref in evidence_refs {
            require_nonblank(evidence_ref, "provider_relationships.evidence_refs")?;
        }
    }
    Ok(())
}

fn require_unit_interval(value: f64, field: &'static str) -> Result<(), ApiError> {
    if (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        invalid_score_field(field)
    }
}

fn require_percentile(value: u8, field: &'static str) -> Result<(), ApiError> {
    if value <= 100 {
        Ok(())
    } else {
        invalid_score_field(field)
    }
}

fn require_positive_decimal(value: Decimal, field: &'static str) -> Result<(), ApiError> {
    if value > Decimal::ZERO {
        Ok(())
    } else {
        invalid_score_field(field)
    }
}

fn require_nonnegative_decimal(value: Decimal, field: &'static str) -> Result<(), ApiError> {
    if value >= Decimal::ZERO {
        Ok(())
    } else {
        invalid_score_field(field)
    }
}

fn require_nonblank(value: &str, field: &'static str) -> Result<(), ApiError> {
    if value.trim().is_empty() {
        invalid_score_field(field)
    } else {
        Ok(())
    }
}

fn invalid_score_field(field: &'static str) -> Result<(), ApiError> {
    Err(ApiError::new(
        axum::http::StatusCode::BAD_REQUEST,
        "INVALID_SCORE_REQUEST",
        format!("{field} is invalid"),
    ))
}

use super::claims_canonical::{canonical_score_input, demo_context, duplicate_payload_fields};
use super::claims_evidence::{
    apply_clinical_evidence_features, apply_provider_profile_features,
    apply_provider_relationship_features, expand_dynamic_required_evidence,
    persist_rule_evidence_request, RuleEvidenceRequestInput,
};
use super::claims_validation::{
    validate_score_request_contract, validate_source_system_matches_actor,
};
use crate::{
    app::AppState,
    auth::AuthenticatedApiPrincipal,
    error::ApiError,
    repository::{
        ModelVersionRecord, PersistedAuditEvent, PersistedScoringRun, SimilarCaseQuery,
        SimilarCaseRecord,
    },
};
use axum::{extract::State, Json};
use fwa_anomaly::detect_anomaly;
use fwa_audit::ActorContext;
use fwa_clinical::{assess_clinical_evidence, ClinicalDocumentEvidence};
use fwa_core::*;
use fwa_features::{calculate_features, FeatureMap};
use fwa_ml_runtime::{ModelRuntimeError, ModelScore, ModelScoreRequest};
use fwa_provider::{
    assess_provider_profile, assess_provider_relationship_graph, ProviderProfileInput,
    ProviderRelationshipGraphInput,
};
use fwa_rules::{evaluate_rules, Rule, RuleMatch};

pub use super::claims_types::*;

const SCORING_MODEL_KEY: &str = "baseline_fwa";

pub async fn score_claim(
    State(state): State<AppState>,
    AuthenticatedApiPrincipal(principal): AuthenticatedApiPrincipal,
    Json(request): Json<ScoreClaimRequest>,
) -> Result<Json<ScoreClaimResponse>, ApiError> {
    if !principal.has_permission("tpa:claims:score") {
        return Err(ApiError::new(
            axum::http::StatusCode::FORBIDDEN,
            "PERMISSION_DENIED",
            "missing permission: tpa:claims:score",
        ));
    }
    let actor = principal.actor;

    validate_score_request_contract(&request)?;
    validate_source_system_matches_actor(&request, &actor)?;
    let has_full_payload = request.claim.is_some()
        || request.items.is_some()
        || request.member.is_some()
        || request.policy.is_some()
        || request.provider.is_some()
        || request.documents.is_some()
        || request.provider_profile.is_some()
        || request.provider_relationships.is_some();
    let has_inbox_locator =
        request.inbox_run_id.is_some() || request.inbox_idempotency_key.is_some();
    if request.claim_id.is_some()
        && (has_full_payload || request.canonical_claim_context.is_some() || has_inbox_locator)
    {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            "AMBIGUOUS_SCORE_REQUEST",
            "claim_id, full claim payload, canonical_claim_context, and inbox handoff locators are mutually exclusive",
        ));
    }
    if request.claim_id.is_none()
        && request.claim.is_none()
        && request.canonical_claim_context.is_none()
        && !has_inbox_locator
    {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_SCORE_REQUEST",
            "claim_id, claim payload, canonical_claim_context, or inbox handoff locator is required",
        ));
    }
    let review_mode = normalize_review_mode(request.review_mode.as_deref())?;

    let (
        context,
        clinical_documents,
        provider_profile_input,
        provider_relationships_input,
        request_evidence_refs,
        canonical_claim_context_trace,
    ) = if let Some(claim_id) = request.claim_id.clone() {
        let context = state
            .repository
            .load_claim_context(&claim_id, Some(&actor.customer_scope_id))
            .await
            .map_err(internal_error("CLAIM_LOAD_FAILED"))?
            .ok_or_else(|| {
                ApiError::new(
                    axum::http::StatusCode::NOT_FOUND,
                    "CLAIM_NOT_FOUND",
                    "claim_id was not found",
                )
            })?;
        (context, Vec::new(), None, None, Vec::new(), None)
    } else if has_inbox_locator {
        if has_full_payload || request.canonical_claim_context.is_some() {
            return Err(ApiError::new(
                axum::http::StatusCode::BAD_REQUEST,
                "AMBIGUOUS_SCORE_REQUEST",
                "inbox handoff locators cannot be combined with full claim payload fields or canonical_claim_context",
            ));
        }
        let inbox_run = load_scoring_ready_inbox_run(&state, &request, &actor).await?;
        let canonical = canonical_score_input(&inbox_run.canonical_claim_context)?;
        state
            .repository
            .upsert_claim_context(
                canonical.context.clone(),
                serde_json::to_value(&canonical.context).unwrap_or_else(|_| serde_json::json!({})),
            )
            .await
            .map_err(internal_error("CLAIM_PERSISTENCE_FAILED"))?;
        let mut evidence_refs = canonical.evidence_refs;
        evidence_refs.push(serde_json::Value::String(format!(
            "inbox_claim_runs:{}",
            inbox_run.run_id
        )));
        evidence_refs.push(serde_json::Value::String(format!(
            "audit_events:{}",
            inbox_run.audit_id
        )));
        evidence_refs.extend(
            json_string_values(&inbox_run.evidence_refs)
                .into_iter()
                .map(serde_json::Value::String),
        );
        evidence_refs.sort_by_key(|value| value.to_string());
        evidence_refs.dedup();
        (
            canonical.context,
            canonical.clinical_documents,
            None,
            None,
            evidence_refs,
            Some(inbox_claim_context_trace(canonical.trace, &inbox_run)),
        )
    } else if let Some(canonical_claim_context) = request.canonical_claim_context.clone() {
        if has_full_payload {
            return Err(ApiError::new(
                axum::http::StatusCode::BAD_REQUEST,
                "AMBIGUOUS_SCORE_REQUEST",
                "canonical_claim_context cannot be combined with full claim payload fields",
            ));
        }
        let canonical = canonical_score_input(&canonical_claim_context)?;
        state
            .repository
            .upsert_claim_context(
                canonical.context.clone(),
                serde_json::to_value(&canonical.context).unwrap_or_else(|_| serde_json::json!({})),
            )
            .await
            .map_err(internal_error("CLAIM_PERSISTENCE_FAILED"))?;
        (
            canonical.context,
            canonical.clinical_documents,
            None,
            None,
            canonical.evidence_refs,
            Some(canonical.trace),
        )
    } else {
        let mut payload = request.claim.clone().expect("validated claim payload");
        let duplicate_fields = duplicate_payload_fields(&request, &payload);
        if !duplicate_fields.is_empty() {
            return Err(ApiError::new(
                axum::http::StatusCode::BAD_REQUEST,
                "DUPLICATE_SCORE_PAYLOAD",
                format!(
                    "duplicate nested and top-level payload fields: {}",
                    duplicate_fields.join(", ")
                ),
            ));
        }
        payload.items = payload.items.or_else(|| request.items.clone());
        payload.member = payload.member.or_else(|| request.member.clone());
        payload.policy = payload.policy.or_else(|| request.policy.clone());
        payload.provider = payload.provider.or_else(|| request.provider.clone());
        payload.documents = payload.documents.or_else(|| request.documents.clone());
        payload.provider_profile = payload
            .provider_profile
            .or_else(|| request.provider_profile.clone());
        payload.provider_relationships = payload
            .provider_relationships
            .or_else(|| request.provider_relationships.clone());
        let clinical_documents = payload
            .documents
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(ClinicalDocumentEvidence::from)
            .collect::<Vec<_>>();
        let provider_profile_input = payload
            .provider_profile
            .clone()
            .map(ProviderProfileInput::from);
        let provider_relationships_input = payload
            .provider_relationships
            .clone()
            .map(ProviderRelationshipGraphInput::from);
        let context = demo_context(payload);
        state
            .repository
            .upsert_claim_context(
                context.clone(),
                serde_json::to_value(&context).unwrap_or_else(|_| serde_json::json!({})),
            )
            .await
            .map_err(internal_error("CLAIM_PERSISTENCE_FAILED"))?;
        (
            context,
            clinical_documents,
            provider_profile_input,
            provider_relationships_input,
            Vec::new(),
            None,
        )
    };

    let run_id = ScoringRunId::new();
    let mut features = calculate_features(&context);
    let clinical_evidence = assess_clinical_evidence(&context, &clinical_documents);
    let provider_profile =
        assess_provider_profile(&context.provider, provider_profile_input.as_ref());
    let provider_relationships = assess_provider_relationship_graph(
        &context.provider,
        provider_relationships_input.as_ref(),
    );
    apply_clinical_evidence_features(&mut features, &context, &clinical_evidence);
    apply_provider_profile_features(&mut features, &context, &provider_profile);
    apply_provider_relationship_features(&mut features, &context, &provider_relationships);
    let mut evidence_refs = features
        .values()
        .flat_map(|feature| {
            feature.evidence_refs.iter().map(|evidence| {
                serde_json::to_value(evidence).unwrap_or_else(|_| serde_json::json!({}))
            })
        })
        .collect::<Vec<_>>();
    evidence_refs.extend(request_evidence_refs);
    evidence_refs.extend(
        clinical_evidence
            .evidence_refs
            .iter()
            .map(|evidence| serde_json::json!(evidence)),
    );
    evidence_refs.extend(
        provider_profile
            .evidence_refs
            .iter()
            .map(|evidence| serde_json::json!(evidence)),
    );
    evidence_refs.extend(
        provider_relationships
            .evidence_refs
            .iter()
            .map(|evidence| serde_json::json!(evidence)),
    );
    let (rules_result, active_model_result) = tokio::join!(
        cached_active_rules(&state),
        cached_active_scoring_model(&state, &review_mode)
    );
    let rules = rules_result.map_err(internal_error("RULE_LOAD_FAILED"))?;
    let active_model = active_model_result?;
    let rules = rules
        .into_iter()
        .filter(|rule| review_mode_applies(&rule.review_mode, &review_mode))
        .collect::<Vec<_>>();
    let mut rule_matches =
        evaluate_rules(&rules, &features).map_err(internal_error("RULE_EVALUATION_FAILED"))?;
    expand_dynamic_required_evidence(&mut rule_matches, &clinical_evidence);
    let anomaly_score = detect_anomaly(&features);
    let similar_case_tags = similar_case_tags(&features, &rule_matches);
    let similar_case_query = SimilarCaseQuery {
        claim_id: Some(context.claim.external_claim_id.clone()),
        diagnosis_code: context.claim.diagnosis_code.clone(),
        provider_region: context.provider.region.clone(),
        tags: similar_case_tags.clone(),
    };
    let model_score_request = ModelScoreRequest {
        run_id: run_id.clone(),
        claim_id: context.claim.id.clone(),
        model_key: active_model.model_key.clone(),
        model_version: active_model.version.clone(),
        endpoint_url: active_model.endpoint_url.clone(),
        features: features.clone(),
    };
    let (similar_cases_result, model_score_result) = tokio::join!(
        state.repository.search_similar_cases(similar_case_query),
        state.scorer.score(model_score_request)
    );
    let similar_cases =
        similar_cases_result.map_err(internal_error("SIMILAR_CASE_SEARCH_FAILED"))?;
    let similar_case_score = similar_case_score(&similar_cases);
    evidence_refs.extend(
        similar_cases
            .iter()
            .flat_map(|case| case.provenance_refs.iter().chain(case.evidence_refs.iter()))
            .cloned()
            .map(serde_json::Value::String),
    );
    let model_score = match model_score_result {
        Ok(score) => score,
        Err(error) => {
            let error_message = error.to_string();
            persist_failed_audit(FailedAuditInput {
                state: &state,
                run_id: run_id.clone(),
                context: &context,
                actor: &actor,
                source_system: &request.source_system,
                review_mode: &review_mode,
                summary: "model scoring failed",
                error_message: &error_message,
                evidence_refs: evidence_refs.clone(),
            })
            .await
            .map_err(internal_error("FAILED_AUDIT_PERSISTENCE_FAILED"))?;
            return Err(model_runtime_error(error));
        }
    };
    evidence_refs.extend(rule_matches.iter().map(|rule_match| {
        serde_json::Value::String(format!("rule_runs:{}", rule_match.alert_code))
    }));
    evidence_refs.push(serde_json::Value::String(format!(
        "model_scores:{}",
        model_score.model_key
    )));
    evidence_refs.push(serde_json::Value::String(format!(
        "model_versions:{}:{}",
        model_score.model_key, model_score.model_version
    )));
    let routing_policy = active_routing_policy(&state, &review_mode).await?;
    let decision = fwa_scoring::aggregate_with_routing_policy(
        &features,
        &rule_matches,
        &model_score,
        &anomaly_score,
        similar_case_score,
        routing_policy,
    );
    let audit_id = AuditEventId::new();
    let alerts: Vec<AlertResponse> = rule_matches
        .iter()
        .map(|rule_match| AlertResponse {
            alert_code: rule_match.alert_code.clone(),
            severity: "HIGH".into(),
            reason: rule_match.reason.clone(),
            rule_id: rule_match.rule_id.clone(),
            rule_version: rule_match.rule_version,
            required_evidence: rule_match.required_evidence.clone(),
        })
        .collect();
    let scores = ScoreBreakdown {
        peer_deviation_score: decision.peer_deviation_score,
        rule_score: decision.rule_score,
        anomaly_score: decision.anomaly_score,
        ml_score: decision.ml_score,
        medical_reasonableness_score: decision.medical_reasonableness_score,
        provider_network_score: decision.provider_network_score,
        similar_case_score: decision.similar_case_score,
        final_score: decision.risk_score.value(),
    };
    let feature_values = features.values().cloned().collect::<Vec<_>>();
    let agent_prefill_evidence_refs = build_agent_prefill_evidence_refs(
        &similar_cases,
        &model_score,
        &alerts,
        &run_id,
        &audit_id,
    );
    let agent_investigation_prefill = build_agent_investigation_prefill(
        &context,
        &decision,
        &similar_case_tags,
        &similar_cases,
        agent_prefill_evidence_refs,
    );
    let audit_payload = serde_json::json!({
        "claim_id": context.claim.external_claim_id,
        "source_system": &request.source_system,
        "customer_scope_id": &actor.customer_scope_id,
        "review_mode": &review_mode,
        "risk_score": decision.risk_score.value(),
        "rag": format!("{:?}", decision.rag),
        "risk_level": &decision.risk_level,
        "recommended_action": format!("{:?}", decision.recommended_action),
        "decision_outcome": &decision.decision_outcome,
        "decision_authority": &decision.decision_authority,
        "decision_confidence": &decision.decision_confidence,
        "appeal_or_review_required": decision.appeal_or_review_required,
        "reason_code": &decision.reason_code,
        "confidence_score": decision.confidence_score,
        "confidence": &decision.confidence,
        "routing_reason": &decision.routing_reason,
        "routing_policy": &decision.routing_policy,
        "scores": &scores,
        "top_reasons": &decision.top_reasons,
        "layers": &decision.layers,
        "clinical_evidence": &clinical_evidence,
        "provider_profile": &provider_profile,
        "provider_relationships": &provider_relationships,
        "similar_cases": &similar_cases,
        "canonical_claim_context_trace": &canonical_claim_context_trace,
        "feature_values": &feature_values,
        "model_score": &model_score,
        "agent_investigation_prefill": &agent_investigation_prefill,
        "triggered_rules": &alerts,
        "event_type": "scoring.completed",
        "event_status": "succeeded"
    });
    state
        .repository
        .save_scoring_run(PersistedScoringRun {
            run_id: run_id.to_string(),
            audit_id: audit_id.to_string(),
            claim_id: context.claim.external_claim_id.clone(),
            source_system: request.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            risk_score: decision.risk_score.value(),
            rag: format!("{:?}", decision.rag),
            risk_level: decision.risk_level.clone(),
            recommended_action: format!("{:?}", decision.recommended_action),
            confidence_score: decision.confidence_score,
            confidence: decision.confidence.clone(),
            routing_reason: decision.routing_reason.clone(),
            routing_policy: serde_json::to_value(&decision.routing_policy)
                .unwrap_or_else(|_| serde_json::json!({})),
            score_breakdown: serde_json::to_value(&scores)
                .unwrap_or_else(|_| serde_json::json!({})),
            feature_values: feature_values
                .iter()
                .map(|feature| {
                    serde_json::to_value(feature).unwrap_or_else(|_| serde_json::json!({}))
                })
                .collect(),
            rule_runs: rule_matches
                .iter()
                .map(|rule_match| {
                    serde_json::to_value(rule_match).unwrap_or_else(|_| serde_json::json!({}))
                })
                .collect(),
            model_score: serde_json::to_value(&model_score)
                .unwrap_or_else(|_| serde_json::json!({})),
            evidence_refs: evidence_refs.clone(),
            audit_event: audit_payload,
        })
        .await
        .map_err(internal_error("SCORING_PERSISTENCE_FAILED"))?;
    persist_rule_evidence_request(RuleEvidenceRequestInput {
        state: &state,
        run_id: &run_id,
        audit_id: &audit_id,
        context: &context,
        actor: &actor,
        source_system: &request.source_system,
        alerts: &alerts,
        evidence_refs: &evidence_refs,
    })
    .await?;

    tracing::info!(
        run_id = %run_id,
        audit_id = %audit_id,
        source_system = %request.source_system,
        review_mode = %review_mode,
        risk_score = decision.risk_score.value(),
        risk_level = %decision.risk_level,
        rag = ?decision.rag,
        decision_outcome = ?decision.decision_outcome,
        decision_authority = ?decision.decision_authority,
        decision_confidence = ?decision.decision_confidence,
        routing_policy_id = %decision.routing_policy.policy_id,
        routing_policy_version = decision.routing_policy.version,
        rule_match_count = rule_matches.len(),
        alert_count = alerts.len(),
        similar_case_count = similar_cases.len(),
        evidence_ref_count = evidence_refs.len(),
        model_key = %model_score.model_key,
        model_version = %model_score.model_version,
        "scoring run completed"
    );

    Ok(Json(ScoreClaimResponse {
        run_id: run_id.to_string(),
        audit_id: audit_id.to_string(),
        claim_id: context.claim.external_claim_id,
        review_mode,
        risk_score: decision.risk_score.value(),
        rag: decision.rag,
        risk_level: decision.risk_level,
        recommended_action: decision.recommended_action,
        decision_outcome: decision.decision_outcome,
        decision_authority: decision.decision_authority,
        decision_confidence: decision.decision_confidence,
        appeal_or_review_required: decision.appeal_or_review_required,
        reason_code: decision.reason_code,
        confidence_score: decision.confidence_score,
        confidence: decision.confidence,
        routing_reason: decision.routing_reason,
        routing_policy: decision.routing_policy,
        scores,
        model_score,
        alerts,
        top_reasons: decision.top_reasons,
        layers: decision.layers,
        clinical_evidence,
        provider_profile,
        provider_relationships,
        similar_cases,
        feature_values,
        evidence_refs,
        agent_investigation_prefill,
    }))
}

fn build_agent_investigation_prefill(
    context: &ClaimContext,
    decision: &fwa_scoring::ScoringDecision,
    similar_case_tags: &[String],
    similar_cases: &[SimilarCaseRecord],
    evidence_refs: Vec<String>,
) -> AgentInvestigationPrefill {
    AgentInvestigationPrefill {
        claim_id: context.claim.external_claim_id.clone(),
        risk_score: decision.risk_score.value(),
        rag: agent_rag_label(decision.rag),
        scheme_family: similar_cases.first().map(|case| case.scheme_family.clone()),
        top_reasons: agent_top_reasons(decision),
        similar_case_query: AgentInvestigationSimilarCaseQuery {
            claim_id: context.claim.external_claim_id.clone(),
            diagnosis_code: context.claim.diagnosis_code.clone(),
            provider_region: context.provider.region.clone(),
            tags: agent_similar_case_tags(similar_case_tags),
        },
        evidence_refs,
    }
}

fn build_agent_prefill_evidence_refs(
    similar_cases: &[SimilarCaseRecord],
    model_score: &ModelScore,
    alerts: &[AlertResponse],
    run_id: &ScoringRunId,
    audit_id: &AuditEventId,
) -> Vec<String> {
    let mut evidence_refs = vec![
        format!("scoring_runs:{run_id}"),
        format!("audit_events:{audit_id}"),
        format!(
            "model_versions:{}:{}",
            model_score.model_key, model_score.model_version
        ),
    ];
    evidence_refs.extend(
        alerts
            .iter()
            .map(|alert| format!("rule_runs:{}", alert.alert_code)),
    );
    evidence_refs.extend(
        similar_cases
            .iter()
            .flat_map(|case| case.provenance_refs.iter().chain(case.evidence_refs.iter()))
            .cloned(),
    );
    evidence_refs.sort();
    evidence_refs.dedup();
    evidence_refs
}

fn agent_rag_label(rag: RiskLevel) -> String {
    match rag {
        RiskLevel::Green => "GREEN",
        RiskLevel::Amber => "AMBER",
        RiskLevel::Red => "RED",
    }
    .into()
}

fn agent_top_reasons(decision: &fwa_scoring::ScoringDecision) -> Vec<String> {
    if decision.top_reasons.is_empty() {
        vec![decision.routing_reason.clone()]
    } else {
        decision.top_reasons.clone()
    }
}

fn agent_similar_case_tags(tags: &[String]) -> Vec<String> {
    if tags.is_empty() {
        vec!["runtime_scoring".into()]
    } else {
        tags.to_vec()
    }
}

async fn load_scoring_ready_inbox_run(
    state: &AppState,
    request: &ScoreClaimRequest,
    actor: &ActorContext,
) -> Result<crate::repository::PersistedInboxClaimRun, ApiError> {
    let inbox_run = if let Some(run_id) = request.inbox_run_id.as_deref() {
        state
            .repository
            .get_inbox_claim_run_by_run_id(run_id, Some(&actor.customer_scope_id))
            .await
            .map_err(internal_error("INBOX_RECORD_LOAD_FAILED"))?
    } else if let Some(idempotency_key) = request.inbox_idempotency_key.as_deref() {
        state
            .repository
            .get_inbox_claim_run_by_idempotency_key(idempotency_key, Some(&actor.customer_scope_id))
            .await
            .map_err(internal_error("INBOX_RECORD_LOAD_FAILED"))?
    } else {
        None
    }
    .ok_or_else(|| {
        ApiError::new(
            axum::http::StatusCode::NOT_FOUND,
            "INBOX_RECORD_NOT_FOUND",
            "inbox handoff record was not found",
        )
    })?;

    if !inbox_run.scoring_ready {
        return Err(ApiError::new(
            axum::http::StatusCode::CONFLICT,
            "INBOX_NOT_SCORING_READY",
            "inbox handoff record is not scoring_ready",
        ));
    }
    if inbox_run.source_system != request.source_system {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            "SOURCE_SYSTEM_MISMATCH",
            "inbox handoff source system must match score request source_system",
        ));
    }
    Ok(inbox_run)
}

async fn active_scoring_model(
    state: &AppState,
    review_mode: &str,
) -> Result<ModelVersionRecord, ApiError> {
    state
        .repository
        .list_models()
        .await
        .map_err(internal_error("MODEL_LIST_FAILED"))?
        .into_iter()
        .find(|model| {
            model.model_key == SCORING_MODEL_KEY
                && model.status == "active"
                && model_review_mode_applies(&model.review_mode, review_mode)
        })
        .ok_or_else(|| {
            ApiError::new(
                axum::http::StatusCode::CONFLICT,
                "ACTIVE_MODEL_NOT_FOUND",
                format!("no active scoring model is available for review_mode {review_mode}"),
            )
        })
}

async fn cached_active_rules(state: &AppState) -> anyhow::Result<Vec<Rule>> {
    if let Some(rules) = state.scoring_lookup_cache.active_rules().await {
        return Ok(rules);
    }
    let rules = state.repository.list_active_rules().await?;
    state
        .scoring_lookup_cache
        .store_active_rules(rules.clone())
        .await;
    Ok(rules)
}

async fn cached_active_scoring_model(
    state: &AppState,
    review_mode: &str,
) -> Result<ModelVersionRecord, ApiError> {
    if let Some(model) = state.scoring_lookup_cache.active_model(review_mode).await {
        return Ok(model);
    }
    let model = active_scoring_model(state, review_mode).await?;
    state
        .scoring_lookup_cache
        .store_active_model(review_mode, model.clone())
        .await;
    Ok(model)
}

async fn active_routing_policy(
    state: &AppState,
    review_mode: &str,
) -> Result<fwa_scoring::RoutingPolicy, ApiError> {
    Ok(state
        .repository
        .active_routing_policy(review_mode)
        .await
        .map_err(internal_error("ROUTING_POLICY_LOAD_FAILED"))?
        .unwrap_or_else(|| fwa_scoring::default_routing_policy(review_mode)))
}

fn model_review_mode_applies(model_review_mode: &str, review_mode: &str) -> bool {
    review_mode_applies(model_review_mode, review_mode)
}

fn review_mode_applies(configured_review_mode: &str, requested_review_mode: &str) -> bool {
    configured_review_mode == "both" || configured_review_mode == requested_review_mode
}

fn normalize_review_mode(value: Option<&str>) -> Result<String, ApiError> {
    let review_mode = value.unwrap_or("pre_payment");
    match review_mode {
        "pre_payment" | "post_payment" => Ok(review_mode.to_string()),
        _ => Err(ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_REVIEW_MODE",
            "review_mode must be one of: pre_payment, post_payment",
        )),
    }
}

fn similar_case_score(similar_cases: &[SimilarCaseRecord]) -> u8 {
    similar_cases
        .iter()
        .map(|case| (case.similarity_score * 100.0).round().clamp(0.0, 100.0) as u8)
        .max()
        .unwrap_or(0)
}

fn similar_case_tags(features: &FeatureMap, rule_matches: &[RuleMatch]) -> Vec<String> {
    let mut tags = std::collections::BTreeSet::new();
    if numeric_feature(features, "days_since_policy_start").unwrap_or(f64::MAX) <= 7.0 {
        tags.insert("early_claim".to_string());
    }
    if numeric_feature(features, "claim_amount_peer_percentile").unwrap_or(0.0) >= 95.0
        || numeric_feature(features, "claim_amount_to_limit_ratio").unwrap_or(0.0) >= 0.8
    {
        tags.insert("high_amount".to_string());
    }
    if numeric_feature(features, "diagnosis_procedure_match_score").unwrap_or(1.0) < 0.5 {
        tags.insert("medical_mismatch".to_string());
    }
    if numeric_feature(features, "provider_profile_score").unwrap_or(0.0) >= 70.0
        || numeric_feature(features, "provider_graph_risk_score").unwrap_or(0.0) >= 70.0
    {
        tags.insert("provider_pattern".to_string());
    }
    if numeric_feature(features, "high_cost_item_ratio").unwrap_or(0.0) >= 0.6 {
        tags.insert("high_cost_item".to_string());
    }
    for rule_match in rule_matches {
        let alert_code = rule_match.alert_code.to_ascii_lowercase();
        if alert_code.contains("early") {
            tags.insert("early_claim".to_string());
        }
        if alert_code.contains("amount") || alert_code.contains("high") {
            tags.insert("high_amount".to_string());
        }
        if alert_code.contains("medical") || alert_code.contains("diagnosis") {
            tags.insert("medical_mismatch".to_string());
        }
        if alert_code.contains("provider") {
            tags.insert("provider_pattern".to_string());
        }
    }
    tags.into_iter().collect()
}

fn numeric_feature(features: &FeatureMap, name: &str) -> Option<f64> {
    features.get(name).and_then(|feature| {
        feature
            .value
            .as_f64()
            .or_else(|| feature.value.as_i64().map(|value| value as f64))
    })
}

fn inbox_claim_context_trace(
    mut trace: serde_json::Value,
    inbox_run: &crate::repository::PersistedInboxClaimRun,
) -> serde_json::Value {
    if let Some(trace) = trace.as_object_mut() {
        trace.insert("input_mode".into(), serde_json::json!("inbox_run"));
        trace.insert("inbox_run_id".into(), serde_json::json!(inbox_run.run_id));
        trace.insert(
            "inbox_audit_id".into(),
            serde_json::json!(inbox_run.audit_id),
        );
        trace.insert(
            "inbox_idempotency_key".into(),
            serde_json::json!(inbox_run.idempotency_key),
        );
        trace.insert(
            "raw_payload_checksum".into(),
            serde_json::json!(inbox_run.raw_payload_checksum),
        );
    }
    trace
}

fn json_string_values(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .collect()
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::internal(code, error)
}

fn model_runtime_error(error: ModelRuntimeError) -> ApiError {
    match error {
        ModelRuntimeError::ServiceUnavailable => ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MODEL_SERVICE_UNAVAILABLE",
            "model service unavailable",
        ),
        ModelRuntimeError::InvalidResponse(message) => {
            tracing::error!(error = %message, "model response invalid");
            ApiError::new(
                axum::http::StatusCode::BAD_GATEWAY,
                "MODEL_RESPONSE_INVALID",
                "model response invalid",
            )
        }
    }
}

struct FailedAuditInput<'a> {
    state: &'a AppState,
    run_id: ScoringRunId,
    context: &'a ClaimContext,
    actor: &'a ActorContext,
    source_system: &'a str,
    review_mode: &'a str,
    summary: &'a str,
    error_message: &'a str,
    evidence_refs: Vec<serde_json::Value>,
}

async fn persist_failed_audit(input: FailedAuditInput<'_>) -> anyhow::Result<()> {
    input
        .state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: input.run_id.to_string(),
            claim_id: input.context.claim.external_claim_id.clone(),
            source_system: input.source_system.to_string(),
            actor_id: input.actor.actor_id.clone(),
            actor_role: input.actor.actor_role.clone(),
            event_type: "scoring.failed".into(),
            event_status: "failed".into(),
            summary: input.summary.to_string(),
            payload: serde_json::json!({
                "claim_id": input.context.claim.external_claim_id,
                "review_mode": input.review_mode,
                "customer_scope_id": &input.actor.customer_scope_id,
                "error": input.error_message
            }),
            evidence_refs: input.evidence_refs,
        })
        .await
}

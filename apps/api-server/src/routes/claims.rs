use super::claims_agent::{build_agent_investigation_prefill, build_agent_prefill_evidence_refs};
use super::claims_canonical::{canonical_score_input, demo_context, duplicate_payload_fields};
use super::claims_evidence::{
    apply_clinical_evidence_features, apply_provider_profile_features,
    apply_provider_relationship_features, expand_dynamic_required_evidence,
    persist_rule_evidence_request, RuleEvidenceRequestInput,
};
use super::claims_lookup::{
    active_routing_policy, cached_active_rules, cached_active_scoring_model, normalize_review_mode,
    review_mode_applies, similar_case_score, similar_case_tags,
};
use super::claims_validation::{
    validate_score_request_contract, validate_source_system_matches_actor,
};
use crate::{
    app::AppState,
    auth::AuthenticatedApiPrincipal,
    error::ApiError,
    repository::{PersistedAuditEvent, PersistedScoringRun, SimilarCaseQuery},
};
use axum::{extract::State, Json};
use fwa_anomaly::detect_anomaly;
use fwa_audit::ActorContext;
use fwa_clinical::{assess_clinical_evidence, ClinicalDocumentEvidence};
use fwa_core::*;
use fwa_features::{
    calculate_features_with_operational_contexts, ClinicalCompatibilityFeatureContext,
    EpisodeUtilizationFeatureContext, PeerFeatureContext, ProviderProfileFeatureContext,
};
use fwa_ml_runtime::{ModelRuntimeError, ModelScoreRequest};
use fwa_provider::{
    assess_provider_profile, assess_provider_relationship_graph, ProviderProfileInput,
    ProviderRelationshipGraphInput,
};
use fwa_rules::evaluate_rules;
use std::collections::BTreeSet;

pub use super::claims_types::*;

#[derive(Debug, Clone)]
struct ResolvedScoringFeatureContext {
    payload: ScoringFeatureContextPayload,
    evidence_refs: Vec<serde_json::Value>,
}

fn peer_feature_context_from_request(
    request: &ScoreClaimRequest,
    scoring_feature_context: Option<&ScoringFeatureContextPayload>,
) -> Result<Option<PeerFeatureContext>, ApiError> {
    let direct_percentile = request.claim_amount_peer_percentile.or_else(|| {
        request
            .claim
            .as_ref()
            .and_then(|claim| claim.claim_amount_peer_percentile)
    });
    let materialized_percentile = scoring_feature_context
        .and_then(|context| context.peer_context.as_ref())
        .and_then(|context| context.claim_amount_peer_percentile);
    if let (Some(direct), Some(materialized)) = (direct_percentile, materialized_percentile) {
        if direct != materialized {
            return Err(ApiError::new(
                axum::http::StatusCode::BAD_REQUEST,
                "AMBIGUOUS_SCORE_REQUEST",
                "claim_amount_peer_percentile must match scoring_feature_context.peer_context.claim_amount_peer_percentile when both are supplied",
            ));
        }
    }
    let claim_amount_peer_percentile = materialized_percentile.or(direct_percentile);
    if claim_amount_peer_percentile.is_some() {
        Ok(Some(PeerFeatureContext {
            claim_amount_peer_percentile,
        }))
    } else {
        Ok(None)
    }
}

fn clinical_compatibility_context_from_request(
    scoring_feature_context: Option<&ScoringFeatureContextPayload>,
) -> Option<ClinicalCompatibilityFeatureContext> {
    scoring_feature_context
        .and_then(|context| context.clinical_compatibility_context.as_ref())
        .and_then(|context| {
            context.diagnosis_procedure_match_score.map(|score| {
                ClinicalCompatibilityFeatureContext {
                    diagnosis_procedure_match_score: Some(score),
                    data_source: context.data_source.clone(),
                }
            })
        })
}

fn episode_utilization_context_from_request(
    scoring_feature_context: Option<&ScoringFeatureContextPayload>,
) -> Option<EpisodeUtilizationFeatureContext> {
    scoring_feature_context
        .and_then(|context| context.episode_utilization_context.as_ref())
        .map(|context| EpisodeUtilizationFeatureContext {
            member_provider_claim_count_30d: context.member_provider_claim_count_30d,
            duplicate_claim_similarity_score: context.duplicate_claim_similarity_score,
            procedure_frequency_peer_percentile: context.procedure_frequency_peer_percentile,
            unbundling_candidate_count: context.unbundling_candidate_count,
            data_source: context.data_source.clone(),
        })
}

fn inline_scoring_feature_context_from_request(
    request: &ScoreClaimRequest,
) -> Option<&ScoringFeatureContextPayload> {
    request
        .scoring_feature_context
        .as_ref()
        .or_else(|| request.claim.as_ref()?.scoring_feature_context.as_ref())
}

fn scoring_feature_context_payload_evidence_refs(
    context: &ScoringFeatureContextPayload,
) -> Vec<serde_json::Value> {
    context
        .evidence_refs
        .as_ref()
        .into_iter()
        .flat_map(|refs| refs.iter())
        .map(|reference| serde_json::Value::String(reference.clone()))
        .collect()
}

async fn resolve_scoring_feature_context(
    state: &AppState,
    request: &ScoreClaimRequest,
    actor: &ActorContext,
    claim_id: &str,
) -> Result<Option<ResolvedScoringFeatureContext>, ApiError> {
    if let Some(payload) = inline_scoring_feature_context_from_request(request) {
        return Ok(Some(ResolvedScoringFeatureContext {
            payload: payload.clone(),
            evidence_refs: scoring_feature_context_payload_evidence_refs(payload),
        }));
    }

    let Some(record) = state
        .repository
        .latest_scoring_feature_context_for_claim(claim_id, Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("SCORING_FEATURE_CONTEXT_LOAD_FAILED"))?
    else {
        return Ok(None);
    };

    let payload = serde_json::from_value::<ScoringFeatureContextPayload>(record.context_json)
        .map_err(internal_error("SCORING_FEATURE_CONTEXT_PARSE_FAILED"))?;
    let mut evidence_ref_strings = BTreeSet::new();
    if let Some(context_refs) = &payload.evidence_refs {
        evidence_ref_strings.extend(context_refs.iter().cloned());
    }
    evidence_ref_strings.extend(record.evidence_refs);
    evidence_ref_strings.insert(format!(
        "scoring_feature_context_materializations:{}",
        record.materialization_id
    ));
    evidence_ref_strings.insert(format!(
        "scoring_feature_context_materialization_reports:{}",
        record.report_uri
    ));
    let evidence_refs = evidence_ref_strings
        .into_iter()
        .map(serde_json::Value::String)
        .collect();

    Ok(Some(ResolvedScoringFeatureContext {
        payload,
        evidence_refs,
    }))
}

async fn resolve_provider_profile_input(
    state: &AppState,
    actor: &ActorContext,
    provider_id: &str,
    inline_profile: Option<ProviderProfileInput>,
) -> Result<(Option<ProviderProfileInput>, Vec<serde_json::Value>), ApiError> {
    let mut profile = inline_profile;
    let mut evidence_refs = Vec::new();

    if profile.is_none() {
        if let Some(record) = state
            .repository
            .latest_provider_profile_windows_for_provider(
                provider_id,
                Some(&actor.customer_scope_id),
            )
            .await
            .map_err(internal_error("PROVIDER_PROFILE_LOAD_FAILED"))?
        {
            let windows = serde_json::from_value::<Vec<ProviderProfileWindowPayload>>(
                serde_json::Value::Array(record.windows),
            )
            .map_err(internal_error("PROVIDER_PROFILE_PARSE_FAILED"))?;
            profile = Some(ProviderProfileInput::from(ProviderProfilePayload {
                specialty: record.specialty,
                network_status: record.network_status,
                oig_excluded: None,
                sam_debarred: None,
                windows,
            }));
            evidence_refs.extend(
                record
                    .evidence_refs
                    .into_iter()
                    .chain([
                        format!(
                            "provider_profile_windows:{}:{}",
                            record.provider_id, record.as_of_date
                        ),
                        format!(
                            "provider_profile_window_rollups:{}",
                            record.source_report_uri
                        ),
                    ])
                    .map(serde_json::Value::String),
            );
        }
    }

    let sanctions = state
        .repository
        .provider_sanctions_for_provider(provider_id, Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("PROVIDER_SANCTIONS_LOAD_FAILED"))?;
    let oig_excluded = sanctions
        .iter()
        .any(|record| record.list.eq_ignore_ascii_case("OIG"));
    let sam_debarred = sanctions
        .iter()
        .any(|record| record.list.eq_ignore_ascii_case("SAM"));
    if oig_excluded || sam_debarred {
        let profile = profile.get_or_insert_with(|| ProviderProfileInput {
            specialty: None,
            network_status: None,
            oig_excluded: None,
            sam_debarred: None,
            windows: Vec::new(),
        });
        if oig_excluded {
            profile.oig_excluded = Some(true);
        }
        if sam_debarred {
            profile.sam_debarred = Some(true);
        }
        evidence_refs.extend(sanctions.into_iter().flat_map(|record| {
            [
                serde_json::Value::String(format!("provider_sanctions:{}", record.sanction_key)),
                serde_json::Value::String(format!(
                    "sanctions_sync_reports:{}",
                    record.source_report_uri
                )),
            ]
        }));
    }

    evidence_refs.sort_by_key(|value| value.to_string());
    evidence_refs.dedup();
    Ok((profile, evidence_refs))
}

async fn resolve_provider_relationships_input(
    state: &AppState,
    actor: &ActorContext,
    provider_id: &str,
    inline_relationships: Option<ProviderRelationshipGraphInput>,
) -> Result<Option<ProviderRelationshipGraphInput>, ApiError> {
    if inline_relationships.is_some() {
        return Ok(inline_relationships);
    }

    let Some(record) = state
        .repository
        .latest_provider_graph_signal_for_provider(provider_id, Some(&actor.customer_scope_id))
        .await
        .map_err(internal_error("PROVIDER_GRAPH_SIGNAL_LOAD_FAILED"))?
    else {
        return Ok(None);
    };

    let (
        Some(high_risk_neighbor_ratio),
        Some(provider_patient_overlap_score),
        Some(connected_confirmed_fwa_count),
    ) = (
        record.high_risk_neighbor_ratio,
        record.provider_patient_overlap_score,
        record.connected_confirmed_fwa_count,
    )
    else {
        return Ok(None);
    };

    let mut evidence_refs = record
        .evidence_refs
        .into_iter()
        .chain([
            format!(
                "provider_graph_signals:{}:{}",
                record.provider_id, record.as_of_date
            ),
            format!("provider_graph_signal_rollups:{}", record.source_report_uri),
        ])
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    evidence_refs.sort();

    Ok(Some(ProviderRelationshipGraphInput {
        high_risk_neighbor_ratio,
        provider_patient_overlap_score,
        referral_concentration_score: record.referral_concentration_score,
        referral_concentration_entropy: record.referral_concentration_entropy,
        temporal_co_billing_score: None,
        temporal_co_billing_frequency_7d: Some(record.temporal_co_billing_frequency_7d),
        billing_ring_membership: Some(record.billing_ring_membership),
        connected_confirmed_fwa_count,
        network_component_risk_score: record.network_component_risk_score,
        evidence_refs,
    }))
}

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
        let mut payload = request.claim.clone().ok_or_else(|| ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            "INVALID_SCORE_REQUEST",
            "claim payload is required when no claim_id, canonical_claim_context, or inbox locator is provided",
        ))?;
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
        payload.scoring_feature_context = payload
            .scoring_feature_context
            .or_else(|| request.scoring_feature_context.clone());
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

    let resolved_scoring_feature_context =
        resolve_scoring_feature_context(&state, &request, &actor, &context.claim.external_claim_id)
            .await?;
    let scoring_feature_context = resolved_scoring_feature_context
        .as_ref()
        .map(|context| &context.payload);
    let peer_feature_context =
        peer_feature_context_from_request(&request, scoring_feature_context)?;
    let clinical_compatibility_context =
        clinical_compatibility_context_from_request(scoring_feature_context);
    let episode_utilization_context =
        episode_utilization_context_from_request(scoring_feature_context);
    let run_id = ScoringRunId::new();
    let (provider_profile_input, provider_profile_materialization_evidence_refs) =
        resolve_provider_profile_input(
            &state,
            &actor,
            &context.provider.external_provider_id,
            provider_profile_input,
        )
        .await?;
    let provider_relationships_input = resolve_provider_relationships_input(
        &state,
        &actor,
        &context.provider.external_provider_id,
        provider_relationships_input,
    )
    .await?;
    let provider_profile =
        assess_provider_profile(&context.provider, provider_profile_input.as_ref());
    let provider_profile_feature_context = ProviderProfileFeatureContext {
        risk_score: Some(provider_profile.risk_score),
    };
    let mut features = calculate_features_with_operational_contexts(
        &context,
        peer_feature_context.as_ref(),
        Some(&provider_profile_feature_context),
        clinical_compatibility_context.as_ref(),
        episode_utilization_context.as_ref(),
    );
    let clinical_evidence = assess_clinical_evidence(&context, &clinical_documents);
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
    if let Some(context) = &resolved_scoring_feature_context {
        evidence_refs.extend(context.evidence_refs.clone());
    }
    evidence_refs.extend(provider_profile_materialization_evidence_refs);
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

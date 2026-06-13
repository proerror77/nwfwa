use super::claims_types::{
    AlertResponse, DocumentPayload, ProviderProfilePayload, ProviderProfileWindowPayload,
    ProviderRelationshipGraphPayload,
};
use crate::{app::AppState, error::ApiError, repository::PersistedAuditEvent};
use fwa_audit::ActorContext;
use fwa_clinical::{ClinicalDocumentEvidence, ClinicalEvidenceAssessment};
use fwa_core::{AuditEventId, ClaimContext, ScoringRunId};
use fwa_features::{EvidenceRef, FeatureMap, FeatureValue};
use fwa_provider::{
    ProviderProfileAssessment, ProviderProfileInput, ProviderProfileWindow,
    ProviderRelationshipGraphAssessment, ProviderRelationshipGraphInput,
};
use fwa_rules::{RequiredEvidence, RuleMatch};

pub(super) struct RuleEvidenceRequestInput<'a> {
    pub(super) state: &'a AppState,
    pub(super) run_id: &'a ScoringRunId,
    pub(super) audit_id: &'a AuditEventId,
    pub(super) context: &'a ClaimContext,
    pub(super) actor: &'a ActorContext,
    pub(super) source_system: &'a str,
    pub(super) alerts: &'a [AlertResponse],
    pub(super) evidence_refs: &'a [serde_json::Value],
}

pub(super) async fn persist_rule_evidence_request(
    input: RuleEvidenceRequestInput<'_>,
) -> Result<(), ApiError> {
    let mut required_evidence = Vec::new();
    for alert in input.alerts {
        for evidence in &alert.required_evidence {
            if !required_evidence
                .iter()
                .any(|existing: &RequiredEvidence| existing.evidence_type == evidence.evidence_type)
            {
                required_evidence.push(evidence.clone());
            }
        }
    }
    if required_evidence.is_empty() {
        return Ok(());
    }

    let request_id = format!("evidence_request_{}", input.audit_id);
    let missing_evidence = required_evidence
        .iter()
        .map(|evidence| evidence.evidence_type.clone())
        .collect::<Vec<_>>();
    let items = required_evidence
        .iter()
        .enumerate()
        .map(|(index, evidence)| {
            serde_json::json!({
                "item_id": format!("{request_id}_item_{}", index + 1),
                "document_type": evidence.evidence_type,
                "status": "open",
                "reason": evidence.evidence_request_type.as_deref().unwrap_or("rule_required_evidence"),
                "blocking": evidence.blocking,
                "policy_authority_ref": evidence.policy_authority_ref,
                "exception_check": evidence.exception_check,
            })
        })
        .collect::<Vec<_>>();
    let event_audit_id = AuditEventId::new();
    input
        .state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: event_audit_id.to_string(),
            run_id: input.run_id.to_string(),
            claim_id: input.context.claim.external_claim_id.clone(),
            source_system: input.source_system.to_string(),
            actor_id: input.actor.actor_id.clone(),
            actor_role: input.actor.actor_role.clone(),
            event_type: "evidence.request.generated".into(),
            event_status: "succeeded".into(),
            summary: format!("Evidence request generated: {request_id}"),
            payload: serde_json::json!({
                "customer_scope_id": input.actor.customer_scope_id,
                "request_id": request_id,
                "claim_id": input.context.claim.external_claim_id,
                "scoring_audit_id": input.audit_id.to_string(),
                "status": "open",
                "request_reason": "rule_required_evidence",
                "missing_evidence": missing_evidence,
                "items": items,
                "reviewer_queue": "clinical-evidence",
                "requested_by": input.actor.actor_id,
                "notes": "Generated from pending-evidence rule action.",
                "source_event_type": "scoring.completed",
            }),
            evidence_refs: input.evidence_refs.to_vec(),
        })
        .await
        .map_err(|error| ApiError::internal("EVIDENCE_REQUEST_SAVE_FAILED", error))?;
    Ok(())
}

pub(super) fn expand_dynamic_required_evidence(
    rule_matches: &mut [RuleMatch],
    clinical_evidence: &ClinicalEvidenceAssessment,
) {
    for rule_match in rule_matches {
        if !rule_match
            .required_evidence
            .iter()
            .any(|evidence| evidence.evidence_type == "clinical_missing_evidence")
        {
            continue;
        }

        let mut expanded = Vec::new();
        for evidence in &rule_match.required_evidence {
            if evidence.evidence_type == "clinical_missing_evidence" {
                for evidence_type in &clinical_evidence.missing_evidence {
                    push_unique_required_evidence(
                        &mut expanded,
                        RequiredEvidence {
                            evidence_type: evidence_type.clone(),
                            evidence_request_type: evidence.evidence_request_type.clone(),
                            blocking: evidence.blocking,
                            policy_authority_ref: evidence.policy_authority_ref.clone(),
                            exception_check: evidence.exception_check.clone(),
                        },
                    );
                }
            } else {
                push_unique_required_evidence(&mut expanded, evidence.clone());
            }
        }
        rule_match.required_evidence = expanded;
    }
}

fn push_unique_required_evidence(items: &mut Vec<RequiredEvidence>, evidence: RequiredEvidence) {
    if !items
        .iter()
        .any(|item| item.evidence_type == evidence.evidence_type)
    {
        items.push(evidence);
    }
}

impl From<DocumentPayload> for ClinicalDocumentEvidence {
    fn from(value: DocumentPayload) -> Self {
        Self {
            document_id: value.external_document_id,
            document_type: value.document_type,
            linked_item_codes: value.linked_item_codes.unwrap_or_default(),
        }
    }
}

impl From<ProviderProfilePayload> for ProviderProfileInput {
    fn from(value: ProviderProfilePayload) -> Self {
        Self {
            specialty: value.specialty,
            network_status: value.network_status,
            oig_excluded: value.oig_excluded,
            sam_debarred: value.sam_debarred,
            windows: value
                .windows
                .into_iter()
                .map(ProviderProfileWindow::from)
                .collect(),
        }
    }
}

impl From<ProviderProfileWindowPayload> for ProviderProfileWindow {
    fn from(value: ProviderProfileWindowPayload) -> Self {
        Self {
            window_days: value.window_days,
            claim_count: value.claim_count,
            total_claim_amount: value.total_claim_amount,
            high_cost_item_ratio: value.high_cost_item_ratio,
            diagnosis_procedure_mismatch_rate: value.diagnosis_procedure_mismatch_rate,
            peer_amount_percentile: value.peer_amount_percentile,
            peer_frequency_percentile: value.peer_frequency_percentile,
            review_failure_count: value.review_failure_count,
            confirmed_fwa_count: value.confirmed_fwa_count,
            false_positive_count: value.false_positive_count,
        }
    }
}

impl From<ProviderRelationshipGraphPayload> for ProviderRelationshipGraphInput {
    fn from(value: ProviderRelationshipGraphPayload) -> Self {
        Self {
            high_risk_neighbor_ratio: value.high_risk_neighbor_ratio,
            provider_patient_overlap_score: value.provider_patient_overlap_score,
            referral_concentration_score: value.referral_concentration_score,
            temporal_co_billing_score: value.temporal_co_billing_score,
            connected_confirmed_fwa_count: value.connected_confirmed_fwa_count,
            network_component_risk_score: value.network_component_risk_score,
            evidence_refs: value.evidence_refs.unwrap_or_default(),
        }
    }
}

pub(super) fn apply_clinical_evidence_features(
    features: &mut FeatureMap,
    context: &ClaimContext,
    clinical_evidence: &ClinicalEvidenceAssessment,
) {
    let evidence_ref = EvidenceRef {
        entity_type: "claim".into(),
        entity_id: context.claim.external_claim_id.clone(),
        field: "clinical_evidence".into(),
    };
    for (name, value) in [
        (
            "clinical_missing_evidence_count",
            clinical_evidence.missing_evidence.len() as i64,
        ),
        (
            "clinical_item_finding_count",
            clinical_evidence.item_findings.len() as i64,
        ),
        (
            "clinical_review_required",
            if clinical_evidence.review_required {
                1
            } else {
                0
            },
        ),
    ] {
        features.insert(
            name.into(),
            FeatureValue {
                name: name.into(),
                version: 1,
                value: serde_json::json!(value),
                evidence_refs: vec![evidence_ref.clone()],
            },
        );
    }
}

pub(super) fn apply_provider_profile_features(
    features: &mut FeatureMap,
    context: &ClaimContext,
    provider_profile: &ProviderProfileAssessment,
) {
    features.insert(
        "provider_profile_score".into(),
        FeatureValue {
            name: "provider_profile_score".into(),
            version: 1,
            value: serde_json::json!(provider_profile.risk_score),
            evidence_refs: vec![EvidenceRef {
                entity_type: "provider".into(),
                entity_id: context.provider.external_provider_id.clone(),
                field: "provider_profile_score".into(),
            }],
        },
    );
    features.insert(
        "provider_peer_amount_percentile".into(),
        FeatureValue {
            name: "provider_peer_amount_percentile".into(),
            version: 1,
            value: serde_json::json!(provider_profile
                .window_findings
                .iter()
                .filter_map(|finding| {
                    finding.outlier_flags.iter().find_map(|flag| {
                        flag.strip_prefix("peer_amount_p")
                            .and_then(|value| value.parse::<u8>().ok())
                    })
                })
                .max()
                .unwrap_or(0)),
            evidence_refs: vec![EvidenceRef {
                entity_type: "provider".into(),
                entity_id: context.provider.external_provider_id.clone(),
                field: "peer_amount_percentile".into(),
            }],
        },
    );
}

pub(super) fn apply_provider_relationship_features(
    features: &mut FeatureMap,
    context: &ClaimContext,
    provider_relationships: &ProviderRelationshipGraphAssessment,
) {
    features.insert(
        "provider_graph_risk_score".into(),
        FeatureValue {
            name: "provider_graph_risk_score".into(),
            version: 1,
            value: serde_json::json!(provider_relationships.risk_score),
            evidence_refs: vec![EvidenceRef {
                entity_type: "provider".into(),
                entity_id: context.provider.external_provider_id.clone(),
                field: "provider_graph_risk_score".into(),
            }],
        },
    );
    features.insert(
        "provider_high_risk_neighbor_signal".into(),
        FeatureValue {
            name: "provider_high_risk_neighbor_signal".into(),
            version: 1,
            value: serde_json::json!(provider_relationships
                .findings
                .iter()
                .any(|finding| finding.signal == "high_risk_neighbor_ratio")),
            evidence_refs: vec![EvidenceRef {
                entity_type: "provider".into(),
                entity_id: context.provider.external_provider_id.clone(),
                field: "high_risk_neighbor_ratio".into(),
            }],
        },
    );
}

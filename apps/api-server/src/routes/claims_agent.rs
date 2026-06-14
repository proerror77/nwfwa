use crate::repository::SimilarCaseRecord;
use fwa_core::{AuditEventId, ClaimContext, RiskLevel, ScoringRunId};
use fwa_ml_runtime::ModelScore;

use super::claims_types::{
    AgentInvestigationPrefill, AgentInvestigationSimilarCaseQuery, AlertResponse,
};

pub(super) fn build_agent_investigation_prefill(
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

pub(super) fn build_agent_prefill_evidence_refs(
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

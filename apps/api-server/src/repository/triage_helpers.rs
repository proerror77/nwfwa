use super::case_rows::load_lead_in_tx;
use super::{
    evidence_values_to_strings, normalize_scheme_family, CaseRecord, LeadRecord,
    PersistedScoringRun, TriageLeadInput,
};
use fwa_core::{assess_evidence_sufficiency, ClaimContext};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use std::collections::{BTreeSet, HashMap};

pub(super) fn lead_from_scoring_run(
    run: &PersistedScoringRun,
    context: Option<&ClaimContext>,
) -> Option<LeadRecord> {
    if run.risk_score < 70 {
        return None;
    }
    let evidence_refs = evidence_values_to_strings(&run.evidence_refs);
    Some(LeadRecord {
        lead_id: format!("lead_{}", run.claim_id),
        run_id: run.run_id.clone(),
        claim_id: run.claim_id.clone(),
        member_id: context
            .map(|context| context.member.external_member_id.clone())
            .unwrap_or_default(),
        provider_id: context
            .map(|context| context.provider.external_provider_id.clone())
            .unwrap_or_default(),
        source_system: run.source_system.clone(),
        review_mode: run
            .audit_event
            .get("review_mode")
            .and_then(Value::as_str)
            .unwrap_or("pre_payment")
            .to_string(),
        scheme_family: scheme_family_from_rule_runs(&run.rule_runs),
        lead_source: "scoring_run".into(),
        status: "new".into(),
        disposition: "pending_triage".into(),
        risk_score: run.risk_score,
        rag: run.rag.clone(),
        reason: run.routing_reason.clone(),
        evidence_refs,
    })
}

pub(super) fn control_lead_from_scoring_run(
    run: &PersistedScoringRun,
    context: Option<&ClaimContext>,
) -> LeadRecord {
    let mut evidence_refs = evidence_values_to_strings(&run.evidence_refs);
    if evidence_refs.is_empty() {
        evidence_refs.push(format!("audit:{}", run.audit_id));
    }
    LeadRecord {
        lead_id: format!("control_lead_{}", run.claim_id),
        run_id: run.run_id.clone(),
        claim_id: run.claim_id.clone(),
        member_id: context
            .map(|context| context.member.external_member_id.clone())
            .unwrap_or_default(),
        provider_id: context
            .map(|context| context.provider.external_provider_id.clone())
            .unwrap_or_default(),
        source_system: run.source_system.clone(),
        review_mode: run
            .audit_event
            .get("review_mode")
            .and_then(Value::as_str)
            .unwrap_or("pre_payment")
            .to_string(),
        scheme_family: if run.risk_score >= 70 {
            scheme_family_from_rule_runs(&run.rule_runs)
        } else {
            "control_baseline".into()
        },
        lead_source: "random_control_scoring_run".into(),
        status: "new".into(),
        disposition: "pending_control_review".into(),
        risk_score: run.risk_score,
        rag: run.rag.clone(),
        reason: format!("Random control baseline sample: {}", run.routing_reason),
        evidence_refs,
    }
}

fn scheme_family_from_rule_runs(rule_runs: &[Value]) -> String {
    let alert_codes = rule_runs
        .iter()
        .filter_map(|run| run["alert_code"].as_str())
        .collect::<Vec<_>>();
    if alert_codes
        .iter()
        .any(|code| code.contains("DIAGNOSIS") || code.contains("MEDICAL"))
    {
        "diagnosis_procedure_mismatch".into()
    } else if alert_codes.iter().any(|code| code.contains("PROVIDER")) {
        "provider_peer_outlier".into()
    } else if alert_codes
        .iter()
        .any(|code| code.contains("EARLY") || code.contains("LIMIT"))
    {
        "early_high_value_claim".into()
    } else {
        "high_risk_claim".into()
    }
}

pub(super) fn scheme_family_from_dsl(dsl: &Value) -> String {
    dsl.get("scheme_family")
        .and_then(Value::as_str)
        .or_else(|| dsl["action"]["scheme_family"].as_str())
        .map(normalize_scheme_family)
        .unwrap_or_else(|| {
            scheme_family_from_alert_code(dsl["action"]["alert_code"].as_str().unwrap_or(""))
        })
}

pub(super) fn scheme_family_from_alert_code(alert_code: &str) -> String {
    let code = alert_code.to_ascii_uppercase();
    if code.contains("DUPLICATE") {
        "duplicate_billing".into()
    } else if code.contains("UPCOD") {
        "upcoding".into()
    } else if code.contains("UNBUND") {
        "unbundling".into()
    } else if code.contains("UNNECESSARY") {
        "medically_unnecessary_service".into()
    } else if code.contains("REPEATED")
        || code.contains("EXCESSIVE")
        || code.contains("UTILIZATION")
    {
        "excessive_utilization".into()
    } else if code.contains("DIAGNOSIS") || code.contains("MEDICAL") || code.contains("LOW_MEDICAL")
    {
        "diagnosis_procedure_mismatch".into()
    } else if code.contains("LAB") {
        "laboratory_testing_abuse".into()
    } else if code.contains("TELE") {
        "telehealth_abuse".into()
    } else if code.contains("GENETIC") {
        "genetic_testing_abuse".into()
    } else if code.contains("PHARMACY") || code.contains("OPIOID") || code.contains("CONTROLLED") {
        "pharmacy_controlled_substance_abuse".into()
    } else if code.contains("DME")
        || code.contains("HOME_HEALTH")
        || code.contains("HOSPICE")
        || code.contains("REHAB")
    {
        "dme_home_health_hospice_rehab_risk".into()
    } else if code.contains("PROVIDER") {
        "provider_peer_outlier".into()
    } else if code.contains("REFERRAL") || code.contains("OWNERSHIP") || code.contains("RELATION") {
        "relationship_concentration".into()
    } else if code.contains("EARLY") || code.contains("LIMIT") {
        "early_high_value_claim".into()
    } else if code.contains("MANY") || code.contains("HIGH_COST") || code.contains("PEER") {
        "excessive_utilization".into()
    } else {
        "high_risk_claim".into()
    }
}

pub(super) fn case_from_lead(lead: &LeadRecord, input: &TriageLeadInput) -> CaseRecord {
    let sla_target_hours = sla_target_hours_for_priority(&input.priority);
    let evidence_sufficiency =
        assess_evidence_sufficiency(&lead.scheme_family, &case_evidence_text(lead, input));
    CaseRecord {
        case_id: format!("case_{}", lead.claim_id),
        lead_id: lead.lead_id.clone(),
        claim_id: lead.claim_id.clone(),
        member_id: lead.member_id.clone(),
        provider_id: lead.provider_id.clone(),
        source_system: lead.source_system.clone(),
        review_mode: lead.review_mode.clone(),
        scheme_family: lead.scheme_family.clone(),
        lead_source: lead.lead_source.clone(),
        status: "triage".into(),
        assignee: input.assignee.clone(),
        reviewer: input.reviewer.clone(),
        priority: input.priority.clone(),
        routing_reason: lead.reason.clone(),
        evidence_package: serde_json::json!({
            "lead_id": lead.lead_id.clone(),
            "claim_id": lead.claim_id.clone(),
            "review_mode": lead.review_mode.clone(),
            "risk_score": lead.risk_score,
            "rag": lead.rag.clone(),
            "reason": lead.reason.clone(),
            "triage_notes": input.notes.clone(),
            "evidence_sufficiency": evidence_sufficiency,
            "evidence_refs": triage_case_evidence_refs(lead, input),
            "evidence_refs_by_type": triage_case_evidence_refs_by_type(lead, input)
        }),
        sla_target_hours,
        sla_status: case_sla_status("triage", sla_target_hours, 0.0),
        time_to_triage_hours: 0.0,
        time_to_closure_hours: None,
        final_outcome: None,
        reviewer_notes: None,
        investigation_result_id: None,
    }
}

fn case_evidence_text(lead: &LeadRecord, input: &TriageLeadInput) -> String {
    let mut parts = vec![
        lead.claim_id.clone(),
        lead.member_id.clone(),
        lead.provider_id.clone(),
        lead.scheme_family.clone(),
        lead.reason.clone(),
        input.notes.clone(),
    ];
    parts.extend(lead.evidence_refs.clone());
    parts.extend(input.evidence_refs.clone());
    parts.join(" ")
}

fn triage_case_evidence_refs(lead: &LeadRecord, input: &TriageLeadInput) -> Vec<String> {
    let mut refs = lead.evidence_refs.clone();
    refs.extend(input.evidence_refs.clone());
    refs.push(format!("claims:{}", lead.claim_id));
    refs.push(format!("scoring_runs:{}:anomaly_score", lead.run_id));
    refs.sort();
    refs.dedup();
    refs
}

fn triage_case_evidence_refs_by_type(lead: &LeadRecord, input: &TriageLeadInput) -> Value {
    let mut claim = BTreeSet::from([format!("claims:{}", lead.claim_id)]);
    let mut rule = BTreeSet::new();
    let mut model = BTreeSet::new();
    let mut anomaly = BTreeSet::from([format!("scoring_runs:{}:anomaly_score", lead.run_id)]);
    let mut document = BTreeSet::new();
    let mut similar_case = BTreeSet::new();

    for reference in triage_case_evidence_refs(lead, input) {
        match evidence_ref_bucket(&reference) {
            Some("claim") => {
                claim.insert(reference);
            }
            Some("rule") => {
                rule.insert(reference);
            }
            Some("model") => {
                model.insert(reference);
            }
            Some("anomaly") => {
                anomaly.insert(reference);
            }
            Some("document") => {
                document.insert(reference);
            }
            Some("similar_case") => {
                similar_case.insert(reference);
            }
            _ => {}
        }
    }

    serde_json::json!({
        "claim": claim.into_iter().collect::<Vec<_>>(),
        "rule": rule.into_iter().collect::<Vec<_>>(),
        "model": model.into_iter().collect::<Vec<_>>(),
        "anomaly": anomaly.into_iter().collect::<Vec<_>>(),
        "document": document.into_iter().collect::<Vec<_>>(),
        "similar_case": similar_case.into_iter().collect::<Vec<_>>(),
    })
}

fn evidence_ref_bucket(reference: &str) -> Option<&'static str> {
    if let Ok(value) = serde_json::from_str::<Value>(reference) {
        if let Some(entity_type) = value.get("entity_type").and_then(Value::as_str) {
            return match entity_type {
                "claim" | "member" | "policy" | "provider" | "claim_item" => Some("claim"),
                "rule" | "rule_run" => Some("rule"),
                "model" | "model_score" | "model_version" => Some("model"),
                "document" | "document_chunk" | "ocr" => Some("document"),
                _ => None,
            };
        }
    }

    if reference.starts_with("knowledge_cases:")
        || reference.starts_with("retrieval:")
        || reference.starts_with("matched_signal:")
        || reference.starts_with("query_claim:")
    {
        Some("similar_case")
    } else if reference.starts_with("rule_runs:") || reference.starts_with("rules:") {
        Some("rule")
    } else if reference.starts_with("model_scores:") || reference.starts_with("model_versions:") {
        Some("model")
    } else if reference.starts_with("documents:")
        || reference.starts_with("document_chunks:")
        || reference.starts_with("ocr:")
    {
        Some("document")
    } else if reference.starts_with("claims:")
        || reference.starts_with("claim:")
        || reference.starts_with("members:")
        || reference.starts_with("policies:")
        || reference.starts_with("providers:")
        || reference.starts_with("claim_items:")
    {
        Some("claim")
    } else if reference.starts_with("anomaly:")
        || (reference.starts_with("scoring_runs:") && reference.contains("anomaly"))
    {
        Some("anomaly")
    } else {
        None
    }
}

pub(super) fn triage_audit_payload(
    lead: &LeadRecord,
    input: &TriageLeadInput,
    case: Option<&CaseRecord>,
) -> Value {
    let evidence_sufficiency = case
        .and_then(|case| case.evidence_package.get("evidence_sufficiency"))
        .cloned();
    serde_json::json!({
        "claim_id": lead.claim_id.clone(),
        "lead_id": lead.lead_id.clone(),
        "case_id": case.map(|case| case.case_id.clone()),
        "review_mode": lead.review_mode.clone(),
        "decision": input.decision.clone(),
        "disposition": lead.disposition.clone(),
        "merge_target_lead_id": input.merge_target_lead_id.clone(),
        "notes": input.notes.clone(),
        "customer_scope_id": input.customer_scope_id.clone(),
        "evidence_sufficiency": evidence_sufficiency,
        "evidence_refs_by_type": case.and_then(|case| case.evidence_package.get("evidence_refs_by_type")).cloned(),
        "evidence_refs": input.evidence_refs.clone()
    })
}

pub(super) fn triage_status_for_decision(decision: &str) -> &'static str {
    match decision {
        "open_case" => "triaged",
        "reject_lead" => "closed",
        "request_evidence" => "pending_evidence",
        "merge_lead" => "closed",
        _ => "triaged",
    }
}

pub(super) fn triage_disposition_for_decision(decision: &str) -> &'static str {
    match decision {
        "open_case" => "open_case",
        "reject_lead" => "rejected",
        "request_evidence" => "pending_evidence",
        "merge_lead" => "merged",
        _ => "pending_triage",
    }
}

fn merge_target_lead_id(input: &TriageLeadInput) -> Option<&str> {
    input
        .merge_target_lead_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub(super) fn merge_target_exists_in_memory(
    leads: &HashMap<String, LeadRecord>,
    input: &TriageLeadInput,
    visible_claim_ids: Option<&BTreeSet<String>>,
) -> bool {
    merge_target_lead_id(input).is_some_and(|target_lead_id| {
        leads.get(target_lead_id).is_some_and(|lead| {
            visible_claim_ids.is_none_or(|claim_ids| claim_ids.contains(&lead.claim_id))
        })
    })
}

pub(super) async fn merge_target_lead_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    input: &TriageLeadInput,
) -> anyhow::Result<Option<LeadRecord>> {
    match merge_target_lead_id(input) {
        Some(target_lead_id) => {
            load_lead_in_tx(tx, target_lead_id, input.customer_scope_id.as_deref()).await
        }
        None => Ok(None),
    }
}

pub(super) fn sla_target_hours_for_priority(priority: &str) -> u32 {
    match priority {
        "critical" => 8,
        "high" => 24,
        "medium" => 72,
        "low" => 168,
        _ => 72,
    }
}

pub(super) fn is_terminal_case_status(status: &str) -> bool {
    matches!(status, "confirmed" | "rejected" | "closed")
}

pub(super) fn case_sla_status(status: &str, sla_target_hours: u32, elapsed_hours: f64) -> String {
    if is_terminal_case_status(status) {
        if elapsed_hours > sla_target_hours as f64 {
            "closed_breached".into()
        } else {
            "closed_within_sla".into()
        }
    } else if elapsed_hours > sla_target_hours as f64 {
        "breached".into()
    } else {
        "on_track".into()
    }
}

pub(super) fn hours_between(
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> f64 {
    end.signed_duration_since(start).num_seconds().max(0) as f64 / 3600.0
}

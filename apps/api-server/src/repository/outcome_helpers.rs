use super::types::QaFeedbackStatusUpdate;
use super::{
    canonical_feedback_target, json_array_to_strings, AuditHistoryEventRecord, CaseRecord,
    InvestigationResultRecord, OutcomeLabelRecord, QaFeedbackItemRecord, QaReviewRecord,
};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

pub(super) fn qa_review_to_feedback_item(
    review: QaReviewRecord,
    created_at: Option<String>,
    status: &str,
    status_update: Option<&QaFeedbackStatusUpdate>,
) -> QaFeedbackItemRecord {
    let priority = if review.qa_conclusion.contains("escalate") {
        "high"
    } else if review.qa_conclusion.contains("return") {
        "medium"
    } else {
        "low"
    };
    QaFeedbackItemRecord {
        feedback_id: qa_feedback_id(&review.qa_case_id),
        qa_case_id: review.qa_case_id.clone(),
        claim_id: review.claim_id.clone(),
        feedback_target: canonical_feedback_target(&review.feedback_target).into(),
        issue_type: review.issue_type.clone(),
        qa_conclusion: review.qa_conclusion.clone(),
        source: "qa_review".into(),
        status: status.into(),
        priority: priority.into(),
        summary: format!(
            "QA {} flagged {} feedback for claim {}",
            review.qa_case_id, review.feedback_target, review.claim_id
        ),
        note_present: !review.notes.trim().is_empty(),
        evidence_refs: review.evidence_refs,
        created_at,
        status_updated_by: status_update.and_then(|update| update.actor_id.clone()),
        status_audit_id: status_update.map(|update| update.audit_id.clone()),
        status_updated_at: status_update.and_then(|update| update.updated_at.clone()),
        status_evidence_refs: status_update
            .map(|update| update.evidence_refs.clone())
            .unwrap_or_default(),
    }
}

pub(super) fn qa_feedback_id(qa_case_id: &str) -> String {
    format!("qa_feedback_{qa_case_id}")
}

pub(super) fn qa_case_id_from_feedback_id(feedback_id: &str) -> Option<&str> {
    feedback_id.strip_prefix("qa_feedback_")
}

pub(super) fn latest_qa_feedback_statuses(
    events: &[(String, AuditHistoryEventRecord)],
) -> HashMap<String, QaFeedbackStatusUpdate> {
    let mut statuses = HashMap::new();
    for (_, event) in events {
        if event.event_type == "qa.feedback.status.updated" {
            let Some(feedback_id) = event.payload["feedback_id"].as_str() else {
                continue;
            };
            let Some(status) = event.payload["to_status"].as_str() else {
                continue;
            };
            statuses.insert(
                feedback_id.to_string(),
                QaFeedbackStatusUpdate {
                    status: status.to_string(),
                    actor_id: event.payload["actor_id"].as_str().map(str::to_string),
                    audit_id: event.audit_id.clone(),
                    updated_at: event.created_at.clone(),
                    evidence_refs: event.evidence_refs.clone(),
                },
            );
        }
    }
    statuses
}

pub(super) fn sort_qa_feedback_items(items: &mut [QaFeedbackItemRecord]) {
    items.sort_by(|left, right| {
        feedback_target_rank(&left.feedback_target)
            .cmp(&feedback_target_rank(&right.feedback_target))
            .then_with(|| left.qa_case_id.cmp(&right.qa_case_id))
    });
}

fn feedback_target_rank(target: &str) -> u8 {
    match canonical_feedback_target(target) {
        "rules" => 0,
        "model" => 1,
        _ => 2,
    }
}

#[derive(Debug, Clone)]
pub(super) struct FinancialImpactRecord {
    pub(super) impact_type: String,
    pub(super) amount: Decimal,
    pub(super) currency: Option<String>,
}

pub(super) fn financial_impact_from_investigation(
    record: &InvestigationResultRecord,
) -> Option<FinancialImpactRecord> {
    financial_impact_from_parts(
        record.confirmed_fwa,
        record.financial_impact_type.as_deref(),
        record.saving_amount,
        record.currency.clone(),
    )
}

pub(super) fn financial_impact_from_parts(
    confirmed_fwa: bool,
    financial_impact_type: Option<&str>,
    saving_amount: Option<Decimal>,
    currency: Option<String>,
) -> Option<FinancialImpactRecord> {
    if !confirmed_fwa {
        return None;
    }
    let amount = saving_amount?;
    if amount <= Decimal::ZERO {
        return None;
    }
    Some(FinancialImpactRecord {
        impact_type: normalize_financial_impact_type(financial_impact_type).into(),
        amount,
        currency,
    })
}

pub(super) fn normalize_financial_impact_type(value: Option<&str>) -> &'static str {
    match value.unwrap_or("prevented_payment") {
        "recovered_amount" => "recovered_amount",
        "avoided_future_exposure" => "avoided_future_exposure",
        "deterrence_estimate" => "deterrence_estimate",
        "estimated_impact" => "estimated_impact",
        _ => "prevented_payment",
    }
}

pub(super) fn labels_from_investigation_result(
    record: InvestigationResultRecord,
) -> Vec<OutcomeLabelRecord> {
    let mut labels = vec![OutcomeLabelRecord {
        label_id: format!(
            "label_investigation_{}_confirmed_fwa",
            record.investigation_id
        ),
        claim_id: record.claim_id.clone(),
        label_name: "confirmed_fwa".into(),
        label_value: record.confirmed_fwa.to_string(),
        source_type: "investigation_result".into(),
        source_id: record.investigation_id.clone(),
        governance_status: if record.confirmed_fwa {
            "approved_for_training".into()
        } else {
            "needs_review".into()
        },
        feedback_target: "model".into(),
        currency: None,
        evidence_refs: record.evidence_refs.clone(),
    }];

    if !record.confirmed_fwa {
        labels.push(OutcomeLabelRecord {
            label_id: format!(
                "label_investigation_{}_false_positive",
                record.investigation_id
            ),
            claim_id: record.claim_id.clone(),
            label_name: "false_positive".into(),
            label_value: "true".into(),
            source_type: "investigation_result".into(),
            source_id: record.investigation_id.clone(),
            governance_status: "needs_review".into(),
            feedback_target: "rules".into(),
            currency: None,
            evidence_refs: record.evidence_refs.clone(),
        });
    }

    if let Some(saving_amount) = record.saving_amount {
        let impact_type = normalize_financial_impact_type(record.financial_impact_type.as_deref());
        let label_name = match impact_type {
            "recovered_amount" => "amount_recovered",
            "avoided_future_exposure" => "avoided_future_exposure",
            "deterrence_estimate" => "deterrence_estimate",
            "estimated_impact" => "estimated_impact",
            _ => "amount_prevented",
        };
        labels.push(OutcomeLabelRecord {
            label_id: format!(
                "label_investigation_{}_{}",
                record.investigation_id, label_name
            ),
            claim_id: record.claim_id,
            label_name: label_name.into(),
            label_value: saving_amount.to_string(),
            source_type: "investigation_result".into(),
            source_id: record.investigation_id,
            governance_status: "approved_for_training".into(),
            feedback_target: "workflow".into(),
            currency: record.currency,
            evidence_refs: record.evidence_refs,
        });
    }

    labels
}

pub(super) fn label_from_qa_review(
    record: QaReviewRecord,
    feedback_status: &str,
) -> OutcomeLabelRecord {
    OutcomeLabelRecord {
        label_id: format!("label_qa_{}_{}", record.qa_case_id, record.issue_type),
        claim_id: record.claim_id,
        label_name: record.issue_type,
        label_value: "true".into(),
        source_type: "qa_review".into(),
        source_id: record.qa_case_id,
        governance_status: qa_label_governance_status(feedback_status).into(),
        feedback_target: canonical_feedback_target(&record.feedback_target).into(),
        currency: None,
        evidence_refs: record.evidence_refs,
    }
}

fn qa_label_governance_status(feedback_status: &str) -> &'static str {
    if feedback_status == "resolved" {
        "approved_for_training"
    } else {
        "needs_review"
    }
}

pub(super) fn labels_from_medical_review_event(
    event: &AuditHistoryEventRecord,
) -> Vec<OutcomeLabelRecord> {
    let Some(claim_id) = event.payload["claim_id"].as_str() else {
        return Vec::new();
    };
    medical_review_outcome_labels(event)
        .into_iter()
        .map(|label_name| {
            let (label_value, governance_status, feedback_target) =
                medical_review_label_fields(&label_name);
            OutcomeLabelRecord {
                label_id: format!("label_medical_review_{}_{}", event.audit_id, label_name),
                claim_id: claim_id.to_string(),
                label_name,
                label_value: label_value.into(),
                source_type: "medical_review".into(),
                source_id: event.audit_id.clone(),
                governance_status: governance_status.into(),
                feedback_target: feedback_target.into(),
                currency: None,
                evidence_refs: event.evidence_refs.clone(),
            }
        })
        .collect()
}

fn medical_review_outcome_labels(event: &AuditHistoryEventRecord) -> Vec<String> {
    let outcomes = event
        .payload
        .get("clinical_outcomes")
        .and_then(Value::as_array)
        .map(|outcomes| {
            outcomes
                .iter()
                .filter_map(Value::as_str)
                .filter(|outcome| is_allowed_medical_review_label(outcome))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !outcomes.is_empty() {
        return unique_strings(outcomes);
    }
    event.payload["decision"]
        .as_str()
        .map(|decision| vec![medical_review_label_from_decision(decision).to_string()])
        .unwrap_or_default()
}

fn medical_review_label_from_decision(decision: &str) -> &'static str {
    match decision {
        "request_more_evidence" => "insufficient_evidence",
        "medical_necessity_issue" => "medical_necessity_issue",
        "no_medical_issue" => "false_positive",
        _ => "clinical_evidence_sufficient",
    }
}

fn medical_review_label_fields(label_name: &str) -> (&'static str, &'static str, &'static str) {
    match label_name {
        "insufficient_evidence" | "medical_necessity_review_required" => {
            ("true", "needs_review", "workflow")
        }
        "documentation_issue" => ("true", "approved_for_training", "workflow"),
        "medical_necessity_issue" | "false_positive" => ("true", "approved_for_training", "model"),
        _ => ("true", "approved_for_training", "workflow"),
    }
}

fn is_allowed_medical_review_label(label_name: &str) -> bool {
    matches!(
        label_name,
        "documentation_issue"
            | "medical_necessity_review_required"
            | "insufficient_evidence"
            | "medical_necessity_issue"
            | "clinical_evidence_sufficient"
            | "false_positive"
    )
}

fn unique_strings(values: Vec<String>) -> Vec<String> {
    values.into_iter().fold(Vec::new(), |mut unique, value| {
        if !unique.contains(&value) {
            unique.push(value);
        }
        unique
    })
}

pub(super) fn labels_from_lead_triage_events(
    events: impl IntoIterator<Item = AuditHistoryEventRecord>,
) -> Vec<OutcomeLabelRecord> {
    let mut labels_by_lead = BTreeMap::new();
    for event in events {
        if let Some(label) = label_from_lead_triage_event(&event) {
            labels_by_lead.insert(label.source_id.clone(), label);
        }
    }
    labels_by_lead.into_values().collect()
}

fn label_from_lead_triage_event(event: &AuditHistoryEventRecord) -> Option<OutcomeLabelRecord> {
    let claim_id = event.payload["claim_id"].as_str()?.to_string();
    let lead_id = event.payload["lead_id"].as_str()?.to_string();
    let disposition = lead_disposition_label_value(
        event.payload["decision"].as_str(),
        event.payload["disposition"].as_str(),
    )?;
    if event.evidence_refs.is_empty() {
        return None;
    }
    Some(OutcomeLabelRecord {
        label_id: format!("label_lead_{}_lead_disposition", lead_id),
        claim_id,
        label_name: "lead_disposition".into(),
        label_value: disposition.into(),
        source_type: "lead_triage".into(),
        source_id: lead_id,
        governance_status: "needs_review".into(),
        feedback_target: "workflow".into(),
        currency: None,
        evidence_refs: event.evidence_refs.clone(),
    })
}

pub(super) fn label_from_bootstrap_review_event(
    event: &AuditHistoryEventRecord,
) -> Option<OutcomeLabelRecord> {
    let item_id = event.payload["item_id"].as_str()?.to_string();
    Some(OutcomeLabelRecord {
        label_id: format!(
            "label_bootstrap_{}_{}",
            item_id,
            event.payload["label_name"].as_str()?
        ),
        claim_id: event.payload["claim_id"].as_str()?.to_string(),
        label_name: event.payload["label_name"].as_str()?.to_string(),
        label_value: event.payload["label_value"].as_str()?.to_string(),
        source_type: "label_bootstrap".into(),
        source_id: item_id,
        governance_status: event.payload["governance_status"].as_str()?.to_string(),
        feedback_target: event.payload["feedback_target"]
            .as_str()
            .unwrap_or("workflow")
            .to_string(),
        currency: None,
        evidence_refs: event.evidence_refs.clone(),
    })
}

fn lead_disposition_label_value(
    decision: Option<&str>,
    disposition: Option<&str>,
) -> Option<&'static str> {
    match decision.or(disposition)? {
        "open_case" => Some("promoted"),
        "reject_lead" | "rejected" => Some("rejected"),
        "request_evidence" | "pending_evidence" => Some("requested_more_evidence"),
        "merge_lead" | "merged" => Some("merged"),
        _ => None,
    }
}

pub(super) fn labels_from_case_status(record: CaseRecord) -> Vec<OutcomeLabelRecord> {
    let confirmed_fwa = match record.status.as_str() {
        "confirmed" => true,
        "rejected" => false,
        _ => return Vec::new(),
    };
    let evidence_refs = case_label_evidence_refs(&record);
    let mut labels = vec![OutcomeLabelRecord {
        label_id: format!("label_case_{}_confirmed_fwa", record.case_id),
        claim_id: record.claim_id.clone(),
        label_name: "confirmed_fwa".into(),
        label_value: confirmed_fwa.to_string(),
        source_type: "case_status".into(),
        source_id: record.case_id.clone(),
        governance_status: if confirmed_fwa {
            "approved_for_training".into()
        } else {
            "needs_review".into()
        },
        feedback_target: "model".into(),
        currency: None,
        evidence_refs: evidence_refs.clone(),
    }];

    if !confirmed_fwa {
        labels.push(OutcomeLabelRecord {
            label_id: format!("label_case_{}_false_positive", record.case_id),
            claim_id: record.claim_id,
            label_name: "false_positive".into(),
            label_value: "true".into(),
            source_type: "case_status".into(),
            source_id: record.case_id,
            governance_status: "needs_review".into(),
            feedback_target: "rules".into(),
            currency: None,
            evidence_refs,
        });
    }

    labels
}

fn case_label_evidence_refs(record: &CaseRecord) -> Vec<String> {
    let mut refs = json_array_to_strings(record.evidence_package["evidence_refs"].clone());
    refs.push(format!("investigation_cases:{}", record.case_id));
    refs
}

pub(super) fn sort_outcome_labels(labels: &mut [OutcomeLabelRecord]) {
    labels.sort_by(|left, right| {
        left.claim_id
            .cmp(&right.claim_id)
            .then_with(|| left.source_type.cmp(&right.source_type))
            .then_with(|| left.source_id.cmp(&right.source_id))
            .then_with(|| left.label_name.cmp(&right.label_name))
    });
}

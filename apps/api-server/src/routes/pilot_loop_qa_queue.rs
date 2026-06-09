use crate::repository::{
    canonical_feedback_target, AuditHistoryEventRecord, AuditSampleLeadRecord, AuditSampleRecord,
    QaFeedbackItemRecord, QaReviewRecord,
};
use serde_json::Value;
use std::collections::BTreeMap;

use super::pilot_loop_types::{QaQueueItemResponse, QaQueueSummaryResponse};

#[derive(Debug, Clone, Default)]
struct CanonicalTraceRefs {
    source_refs: Vec<String>,
    evidence_refs: Vec<String>,
}

pub(super) fn build_qa_queue_items_from_scoring_events(
    samples: &[AuditSampleRecord],
    reviews: &[QaReviewRecord],
    scoring_events: &[AuditHistoryEventRecord],
) -> Vec<QaQueueItemResponse> {
    let canonical_traces = canonical_traces_by_claim(scoring_events);
    build_qa_queue_items(samples, reviews, &canonical_traces)
}

fn build_qa_queue_items(
    samples: &[AuditSampleRecord],
    reviews: &[QaReviewRecord],
    canonical_traces: &BTreeMap<String, CanonicalTraceRefs>,
) -> Vec<QaQueueItemResponse> {
    let reviews_by_case_id = reviews
        .iter()
        .map(|review| (review.qa_case_id.as_str(), review))
        .collect::<BTreeMap<_, _>>();
    let mut items = samples
        .iter()
        .flat_map(|sample| {
            let reviews_by_case_id = &reviews_by_case_id;
            sample.selected_leads.iter().map(move |lead| {
                let qa_case_id = qa_case_id_for_sample_lead(sample, lead);
                let review = reviews_by_case_id.get(qa_case_id.as_str()).copied();
                qa_queue_item_from_sample(
                    sample,
                    lead,
                    qa_case_id,
                    review,
                    canonical_traces.get(&lead.claim_id),
                )
            })
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| {
        right
            .risk_score
            .cmp(&left.risk_score)
            .then_with(|| left.qa_case_id.cmp(&right.qa_case_id))
    });
    items
}

fn qa_queue_item_from_sample(
    sample: &AuditSampleRecord,
    lead: &AuditSampleLeadRecord,
    qa_case_id: String,
    review: Option<&QaReviewRecord>,
    canonical_trace: Option<&CanonicalTraceRefs>,
) -> QaQueueItemResponse {
    QaQueueItemResponse {
        qa_case_id,
        sample_id: sample.sample_id.clone(),
        lead_id: lead.lead_id.clone(),
        claim_id: lead.claim_id.clone(),
        scheme_family: lead.scheme_family.clone(),
        rag: lead.rag.clone(),
        risk_score: lead.risk_score,
        reviewer: sample.reviewer.clone(),
        assignment_queue: sample.assignment_queue.clone(),
        status: if review.is_some() { "reviewed" } else { "open" }.into(),
        qa_conclusion: review.map(|review| review.qa_conclusion.clone()),
        issue_type: review.map(|review| review.issue_type.clone()),
        feedback_target: review
            .map(|review| canonical_feedback_target(&review.feedback_target).into()),
        evidence_refs: lead.evidence_refs.clone(),
        canonical_source_refs: canonical_trace
            .map(|trace| trace.source_refs.clone())
            .unwrap_or_default(),
        canonical_evidence_refs: canonical_trace
            .map(|trace| trace.evidence_refs.clone())
            .unwrap_or_default(),
    }
}

fn qa_case_id_for_sample_lead(sample: &AuditSampleRecord, lead: &AuditSampleLeadRecord) -> String {
    format!("qa_{}_{}", sample.sample_id, lead.lead_id)
}

fn canonical_traces_by_claim(
    scoring_events: &[AuditHistoryEventRecord],
) -> BTreeMap<String, CanonicalTraceRefs> {
    let mut traces = BTreeMap::new();
    for event in scoring_events {
        let Some(claim_id) = event.payload["claim_id"].as_str() else {
            continue;
        };
        let trace = &event.payload["canonical_claim_context_trace"];
        if !trace.is_object() {
            continue;
        }
        traces.insert(
            claim_id.to_string(),
            CanonicalTraceRefs {
                source_refs: unique_json_string_values(&trace["source_refs"]),
                evidence_refs: unique_json_string_values(&trace["evidence_refs"]),
            },
        );
    }
    traces
}

fn unique_json_string_values(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .fold(Vec::new(), |mut values, value| {
                    let value = value.to_string();
                    if !values.contains(&value) {
                        values.push(value);
                    }
                    values
                })
        })
        .unwrap_or_default()
}

pub(super) fn build_qa_queue_summary(items: &[QaFeedbackItemRecord]) -> QaQueueSummaryResponse {
    let open_items = items
        .iter()
        .filter(|item| item.status == "open")
        .collect::<Vec<_>>();
    let in_progress_count = items
        .iter()
        .filter(|item| item.status == "in_progress")
        .count() as u32;
    QaQueueSummaryResponse {
        open_count: open_items.len() as u32,
        in_progress_count,
        resolved_count: items
            .iter()
            .filter(|item| item.status == "resolved")
            .count() as u32,
        dismissed_count: items
            .iter()
            .filter(|item| item.status == "dismissed")
            .count() as u32,
        unresolved_count: open_items.len() as u32 + in_progress_count,
        rules_feedback_count: open_items
            .iter()
            .filter(|item| item.feedback_target == "rules")
            .count() as u32,
        models_feedback_count: open_items
            .iter()
            .filter(|item| canonical_feedback_target(&item.feedback_target) == "model")
            .count() as u32,
        features_feedback_count: open_items
            .iter()
            .filter(|item| item.feedback_target == "features")
            .count() as u32,
        provider_profile_feedback_count: open_items
            .iter()
            .filter(|item| item.feedback_target == "provider_profile")
            .count() as u32,
        workflow_feedback_count: open_items
            .iter()
            .filter(|item| item.feedback_target == "workflow")
            .count() as u32,
        tpa_feedback_count: open_items
            .iter()
            .filter(|item| item.feedback_target == "tpa")
            .count() as u32,
        high_priority_count: open_items
            .iter()
            .filter(|item| item.priority == "high")
            .count() as u32,
        evidence_backed_count: open_items
            .iter()
            .filter(|item| !item.evidence_refs.is_empty())
            .count() as u32,
        highest_priority: highest_priority(&open_items).into(),
    }
}

fn highest_priority(items: &[&QaFeedbackItemRecord]) -> &'static str {
    if items.iter().any(|item| item.priority == "high") {
        "high"
    } else if items.iter().any(|item| item.priority == "medium") {
        "medium"
    } else if items.iter().any(|item| item.priority == "low") {
        "low"
    } else {
        "none"
    }
}

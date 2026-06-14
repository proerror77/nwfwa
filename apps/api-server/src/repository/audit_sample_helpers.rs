use super::{
    canonical_feedback_target, AuditSampleLeadRecord, AuditSampleRecord, AuditSampleStrataContext,
    CreateAuditSampleInput, LeadRecord, QaReviewRecord,
};
use fwa_core::ClaimContext;
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    hash::{Hash, Hasher},
};

pub(super) fn build_audit_sample(
    sample_id: String,
    input: CreateAuditSampleInput,
    leads: Vec<LeadRecord>,
    strata_contexts: &HashMap<String, AuditSampleStrataContext>,
    reviewer_history: &HashMap<String, u32>,
    created_at: Option<String>,
) -> AuditSampleRecord {
    let selection_method = selection_method_for_mode(&input.sample_mode).to_string();
    let mut candidates = leads
        .into_iter()
        .filter(|lead| lead_matches_inclusion(lead, &input.inclusion_criteria, strata_contexts))
        .collect::<Vec<_>>();
    if input.sample_mode == "post_payment_audit" {
        candidates.retain(|lead| lead.review_mode == "post_payment");
    }

    let selected_candidates = match selection_method.as_str() {
        "deterministic_hash" => {
            let seed = input
                .deterministic_seed
                .as_deref()
                .unwrap_or("default-seed");
            candidates.sort_by_key(|lead| deterministic_rank(seed, &lead.lead_id));
            candidates.into_iter().take(input.sample_size).collect()
        }
        "stratified_round_robin" => {
            select_stratified_candidates(candidates, strata_contexts, input.sample_size)
        }
        "reviewer_consistency_rotation" => {
            select_reviewer_rotation_candidates(candidates, reviewer_history, input.sample_size)
        }
        _ => {
            candidates.sort_by(|left, right| {
                right
                    .risk_score
                    .cmp(&left.risk_score)
                    .then_with(|| left.lead_id.cmp(&right.lead_id))
            });
            candidates.into_iter().take(input.sample_size).collect()
        }
    };

    let selected_leads = selected_candidates
        .into_iter()
        .map(|lead| audit_sample_lead_record(lead, strata_contexts, reviewer_history))
        .collect::<Vec<_>>();

    let mut sample = AuditSampleRecord {
        sample_id,
        customer_scope_id: input.customer_scope_id.unwrap_or_default(),
        sample_mode: input.sample_mode,
        population_definition: input.population_definition,
        inclusion_criteria: input.inclusion_criteria,
        deterministic_seed: input.deterministic_seed,
        selection_method,
        sample_size: selected_leads.len(),
        reviewer: input.reviewer,
        assignment_queue: input.assignment_queue,
        selected_leads,
        outcome_distribution: serde_json::json!({}),
        created_at,
    };
    sample.outcome_distribution = audit_sample_outcome_distribution(&sample, &[]);
    sample
}

fn select_stratified_candidates(
    candidates: Vec<LeadRecord>,
    strata_contexts: &HashMap<String, AuditSampleStrataContext>,
    sample_size: usize,
) -> Vec<LeadRecord> {
    let mut strata = BTreeMap::<String, Vec<LeadRecord>>::new();
    for lead in candidates {
        strata
            .entry(strata_key_for_lead(&lead, strata_contexts))
            .or_default()
            .push(lead);
    }

    let mut strata = strata
        .into_iter()
        .map(|(key, mut leads)| {
            leads.sort_by(|left, right| {
                right
                    .risk_score
                    .cmp(&left.risk_score)
                    .then_with(|| left.lead_id.cmp(&right.lead_id))
            });
            (key, VecDeque::from(leads))
        })
        .collect::<BTreeMap<_, _>>();

    let mut selected = Vec::new();
    while selected.len() < sample_size && strata.values().any(|leads| !leads.is_empty()) {
        for leads in strata.values_mut() {
            if selected.len() >= sample_size {
                break;
            }
            if let Some(lead) = leads.pop_front() {
                selected.push(lead);
            }
        }
    }
    selected
}

fn select_reviewer_rotation_candidates(
    mut candidates: Vec<LeadRecord>,
    reviewer_history: &HashMap<String, u32>,
    sample_size: usize,
) -> Vec<LeadRecord> {
    candidates.sort_by(|left, right| {
        reviewer_history
            .get(&left.lead_id)
            .unwrap_or(&0)
            .cmp(reviewer_history.get(&right.lead_id).unwrap_or(&0))
            .then_with(|| right.risk_score.cmp(&left.risk_score))
            .then_with(|| left.lead_id.cmp(&right.lead_id))
    });
    candidates.into_iter().take(sample_size).collect()
}

fn audit_sample_lead_record(
    lead: LeadRecord,
    strata_contexts: &HashMap<String, AuditSampleStrataContext>,
    reviewer_history: &HashMap<String, u32>,
) -> AuditSampleLeadRecord {
    let context = strata_context_for_lead(&lead, strata_contexts);
    let risk_band = risk_band_for_score(lead.risk_score).to_string();
    let strata_key = strata_key(
        &lead.scheme_family,
        &context.provider_type,
        &context.provider_region,
        &context.policy_type,
        &risk_band,
    );
    let prior_reviewer_sample_count = *reviewer_history.get(&lead.lead_id).unwrap_or(&0);
    AuditSampleLeadRecord {
        lead_id: lead.lead_id,
        claim_id: lead.claim_id,
        scheme_family: lead.scheme_family,
        review_mode: lead.review_mode,
        provider_id: lead.provider_id,
        provider_type: context.provider_type,
        provider_region: context.provider_region,
        policy_type: context.policy_type,
        risk_band,
        strata_key,
        prior_reviewer_sample_count,
        risk_score: lead.risk_score,
        rag: lead.rag,
        evidence_refs: lead.evidence_refs,
    }
}

fn strata_key_for_lead(
    lead: &LeadRecord,
    strata_contexts: &HashMap<String, AuditSampleStrataContext>,
) -> String {
    let context = strata_context_for_lead(lead, strata_contexts);
    strata_key(
        &lead.scheme_family,
        &context.provider_type,
        &context.provider_region,
        &context.policy_type,
        risk_band_for_score(lead.risk_score),
    )
}

fn strata_context_for_lead(
    lead: &LeadRecord,
    strata_contexts: &HashMap<String, AuditSampleStrataContext>,
) -> AuditSampleStrataContext {
    strata_contexts
        .get(&lead.claim_id)
        .cloned()
        .unwrap_or_else(|| AuditSampleStrataContext {
            provider_type: "unknown".into(),
            provider_region: "unknown".into(),
            policy_type: "unknown".into(),
        })
}

fn strata_key(
    scheme_family: &str,
    provider_type: &str,
    provider_region: &str,
    policy_type: &str,
    risk_band: &str,
) -> String {
    format!(
        "scheme={scheme_family}|provider_type={provider_type}|region={provider_region}|policy_type={policy_type}|risk_band={risk_band}"
    )
}

fn risk_band_for_score(risk_score: u8) -> &'static str {
    match risk_score {
        85..=100 => "critical",
        70..=84 => "high",
        40..=69 => "medium",
        _ => "low",
    }
}

#[cfg(test)]
mod tests {
    use super::risk_band_for_score;

    #[test]
    fn risk_band_matches_default_scoring_thresholds() {
        assert_eq!(risk_band_for_score(85), "critical");
        assert_eq!(risk_band_for_score(84), "high");
        assert_eq!(risk_band_for_score(70), "high");
        assert_eq!(risk_band_for_score(69), "medium");
        assert_eq!(risk_band_for_score(39), "low");
    }
}

pub(super) fn reviewer_lead_sample_counts<'a>(
    samples: impl IntoIterator<Item = &'a AuditSampleRecord>,
    reviewer: &str,
) -> HashMap<String, u32> {
    let mut counts = HashMap::<String, u32>::new();
    for sample in samples {
        if sample.reviewer != reviewer {
            continue;
        }
        for lead in &sample.selected_leads {
            *counts.entry(lead.lead_id.clone()).or_insert(0) += 1;
        }
    }
    counts
}

pub(super) fn audit_sample_strata_contexts_from_claims(
    claims: &HashMap<String, ClaimContext>,
) -> HashMap<String, AuditSampleStrataContext> {
    claims
        .iter()
        .map(|(claim_id, context)| {
            (
                claim_id.clone(),
                AuditSampleStrataContext {
                    provider_type: context.provider.provider_type.clone(),
                    provider_region: context.provider.region.clone(),
                    policy_type: context.policy.product_code.clone(),
                },
            )
        })
        .collect()
}

pub(super) fn with_sample_outcome_distributions(
    mut samples: Vec<AuditSampleRecord>,
    reviews: &[QaReviewRecord],
) -> Vec<AuditSampleRecord> {
    for sample in &mut samples {
        sample.outcome_distribution = audit_sample_outcome_distribution(sample, reviews);
    }
    samples
}

fn audit_sample_outcome_distribution(
    sample: &AuditSampleRecord,
    reviews: &[QaReviewRecord],
) -> Value {
    let reviews_by_case_id = reviews
        .iter()
        .map(|review| (review.qa_case_id.as_str(), review))
        .collect::<BTreeMap<_, _>>();
    let mut qa_conclusions = BTreeMap::<String, u32>::new();
    let mut issue_types = BTreeMap::<String, u32>::new();
    let mut feedback_targets = BTreeMap::<String, u32>::new();
    let mut strata_distribution = BTreeMap::<String, u32>::new();
    let mut review_mode_distribution = BTreeMap::<String, u32>::new();
    let mut reviewer_history_distribution = BTreeMap::<String, u32>::new();
    let mut reviewed_count = 0_u32;

    for lead in &sample.selected_leads {
        *strata_distribution
            .entry(lead.strata_key.clone())
            .or_insert(0) += 1;
        *review_mode_distribution
            .entry(lead.review_mode.clone())
            .or_insert(0) += 1;
        let history_bucket = if lead.prior_reviewer_sample_count == 0 {
            "new_to_reviewer"
        } else {
            "previously_sampled_by_reviewer"
        };
        *reviewer_history_distribution
            .entry(history_bucket.to_string())
            .or_insert(0) += 1;
    }

    for lead in &sample.selected_leads {
        let qa_case_id = format!("qa_{}_{}", sample.sample_id, lead.lead_id);
        let Some(review) = reviews_by_case_id.get(qa_case_id.as_str()) else {
            continue;
        };
        reviewed_count += 1;
        *qa_conclusions
            .entry(review.qa_conclusion.clone())
            .or_insert(0) += 1;
        *issue_types.entry(review.issue_type.clone()).or_insert(0) += 1;
        *feedback_targets
            .entry(canonical_feedback_target(&review.feedback_target).into())
            .or_insert(0) += 1;
    }

    let selected_count = sample.selected_leads.len() as u32;
    let mut distribution = serde_json::json!({
        "selected_count": selected_count,
        "reviewed_count": reviewed_count,
        "open_count": selected_count.saturating_sub(reviewed_count),
        "qa_conclusions": qa_conclusions,
        "issue_types": issue_types,
        "feedback_targets": feedback_targets,
        "strata_distribution": strata_distribution,
        "review_mode_distribution": review_mode_distribution,
        "reviewer_history_distribution": reviewer_history_distribution
    });
    if sample.sample_mode == "random_control" {
        let missed_risk_review_targets = sample
            .selected_leads
            .iter()
            .filter(|lead| matches!(lead.risk_band.as_str(), "low" | "medium"))
            .count() as u32;
        let false_positive_review_targets = sample
            .selected_leads
            .iter()
            .filter(|lead| matches!(lead.risk_band.as_str(), "high" | "critical"))
            .count() as u32;
        distribution["baseline_measurement"] = serde_json::json!({
            "control_cohort": true,
            "measurement_goal": "false_positive_and_missed_risk_baseline",
            "missed_risk_review_targets": missed_risk_review_targets,
            "false_positive_review_targets": false_positive_review_targets
        });
    }
    distribution
}

fn selection_method_for_mode(sample_mode: &str) -> &'static str {
    match sample_mode {
        "random_control" => "deterministic_hash",
        "stratified" => "stratified_round_robin",
        "qa_calibration" => "reviewer_consistency_rotation",
        "post_payment_audit" => "risk_score_desc_post_payment",
        _ => "risk_score_desc",
    }
}

fn lead_matches_inclusion(
    lead: &LeadRecord,
    criteria: &Value,
    strata_contexts: &HashMap<String, AuditSampleStrataContext>,
) -> bool {
    if let Some(min_risk_score) = criteria["min_risk_score"].as_u64() {
        if lead.risk_score < min_risk_score as u8 {
            return false;
        }
    }
    if let Some(scheme_family) = criteria["scheme_family"].as_str() {
        if lead.scheme_family != scheme_family {
            return false;
        }
    }
    if let Some(rag) = criteria["rag"].as_str() {
        if !lead.rag.eq_ignore_ascii_case(rag) {
            return false;
        }
    }
    if let Some(review_mode) = criteria["review_mode"].as_str() {
        if lead.review_mode != review_mode {
            return false;
        }
    }
    let context = strata_context_for_lead(lead, strata_contexts);
    if let Some(provider_type) = criteria["provider_type"].as_str() {
        if context.provider_type != provider_type {
            return false;
        }
    }
    if let Some(provider_region) = criteria["provider_region"].as_str() {
        if context.provider_region != provider_region {
            return false;
        }
    }
    if let Some(policy_type) = criteria["policy_type"].as_str() {
        if context.policy_type != policy_type {
            return false;
        }
    }
    if let Some(risk_band) = criteria["risk_band"].as_str() {
        if risk_band_for_score(lead.risk_score) != risk_band {
            return false;
        }
    }
    true
}

fn deterministic_rank(seed: &str, lead_id: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    seed.hash(&mut hasher);
    lead_id.hash(&mut hasher);
    hasher.finish()
}

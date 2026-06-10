use crate::RoutingPolicy;
use fwa_core::{
    DecisionAuthority, DecisionConfidence, DecisionOutcome, RecommendedAction, RuleActionClass,
};
use fwa_rules::RuleMatch;

pub(crate) struct DecisionContext {
    pub(crate) outcome: DecisionOutcome,
    pub(crate) authority: DecisionAuthority,
    pub(crate) confidence: DecisionConfidence,
    pub(crate) appeal_or_review_required: bool,
    pub(crate) reason_code: String,
}

pub(crate) fn decision_context(
    rule_matches: &[RuleMatch],
    recommended_action: RecommendedAction,
    confidence: &str,
    policy: &RoutingPolicy,
) -> DecisionContext {
    if policy.review_mode == "post_payment" {
        if let Some(rule_match) = rule_matches.iter().find(|rule_match| {
            rule_match.action_class != RuleActionClass::ScoreOnly
                && rule_match.action_class != RuleActionClass::StraightThrough
        }) {
            return rule_decision_context(
                rule_match,
                DecisionOutcome::PostPaymentAudit,
                true,
                rule_authority(rule_match),
            );
        }
    }
    if let Some(rule_match) = first_rule_with_action_class(rule_matches, RuleActionClass::HardDeny)
    {
        if deterministic_adjudication_ready(rule_match) {
            return rule_decision_context(
                rule_match,
                DecisionOutcome::AutoDeny,
                true,
                rule_authority(rule_match),
            );
        }
        return rule_decision_context(
            rule_match,
            DecisionOutcome::ManualReview,
            true,
            DecisionAuthority::HumanReviewer,
        );
    }
    if let Some(rule_match) =
        first_rule_with_action_class(rule_matches, RuleActionClass::PendingEvidence)
    {
        return rule_decision_context(
            rule_match,
            DecisionOutcome::PendingEvidence,
            true,
            rule_authority(rule_match),
        );
    }
    if let Some(rule_match) =
        first_rule_with_action_class(rule_matches, RuleActionClass::ManualReview)
    {
        return rule_decision_context(
            rule_match,
            DecisionOutcome::ManualReview,
            true,
            rule_authority(rule_match),
        );
    }
    if let Some(rule_match) =
        first_rule_with_action_class(rule_matches, RuleActionClass::StraightThrough)
    {
        if !deterministic_adjudication_ready(rule_match) {
            return rule_decision_context(
                rule_match,
                DecisionOutcome::ManualReview,
                true,
                DecisionAuthority::HumanReviewer,
            );
        }
        return rule_decision_context(
            rule_match,
            DecisionOutcome::StraightThrough,
            false,
            DecisionAuthority::CustomerPolicyRule,
        );
    }

    let outcome = outcome_for_recommended_action(recommended_action, policy);
    DecisionContext {
        outcome,
        authority: authority_for_outcome(outcome),
        confidence: decision_confidence(confidence),
        appeal_or_review_required: outcome != DecisionOutcome::StraightThrough,
        reason_code: format!(
            "routing_policies:{}:v{}:{}",
            policy.policy_id, policy.version, policy.review_mode
        ),
    }
}

fn first_rule_with_action_class(
    rule_matches: &[RuleMatch],
    action_class: RuleActionClass,
) -> Option<&RuleMatch> {
    rule_matches
        .iter()
        .find(|rule_match| rule_match.action_class == action_class)
}

fn deterministic_adjudication_ready(rule_match: &RuleMatch) -> bool {
    rule_match
        .adjudication_policy
        .as_ref()
        .is_some_and(|policy| {
            non_empty(&policy.customer_approval_ref)
                && non_empty(&policy.appeal_or_override_route)
                && non_empty(&policy.effective_date)
                && non_empty(&policy.rollback_plan_ref)
                && non_empty(&policy.production_threshold_ref)
                && non_empty(&policy.routing_impact_ref)
        })
        && rule_match.required_evidence.iter().any(|evidence| {
            evidence
                .policy_authority_ref
                .as_deref()
                .is_some_and(non_empty)
                && evidence.exception_check.as_deref().is_some_and(non_empty)
        })
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn rule_decision_context(
    rule_match: &RuleMatch,
    outcome: DecisionOutcome,
    appeal_or_review_required: bool,
    authority: DecisionAuthority,
) -> DecisionContext {
    DecisionContext {
        outcome,
        authority,
        confidence: DecisionConfidence::Deterministic,
        appeal_or_review_required,
        reason_code: rule_match.alert_code.clone(),
    }
}

fn rule_authority(rule_match: &RuleMatch) -> DecisionAuthority {
    let text = format!(
        "{} {} {}",
        rule_match.rule_id, rule_match.alert_code, rule_match.reason
    )
    .to_ascii_lowercase();
    if ["clinical", "medical", "diagnosis", "procedure"]
        .iter()
        .any(|needle| text.contains(needle))
    {
        DecisionAuthority::ClinicalPolicyRule
    } else {
        DecisionAuthority::CustomerPolicyRule
    }
}

fn outcome_for_recommended_action(
    recommended_action: RecommendedAction,
    policy: &RoutingPolicy,
) -> DecisionOutcome {
    if policy.review_mode == "post_payment"
        && !matches!(recommended_action, RecommendedAction::StandardProcessing)
    {
        return DecisionOutcome::PostPaymentAudit;
    }
    match recommended_action {
        RecommendedAction::StandardProcessing => DecisionOutcome::StraightThrough,
        RecommendedAction::QaSample => DecisionOutcome::QaSample,
        RecommendedAction::RequestEvidence => DecisionOutcome::PendingEvidence,
        RecommendedAction::PostPaymentAudit
        | RecommendedAction::ProviderReview
        | RecommendedAction::RecoveryReview => DecisionOutcome::PostPaymentAudit,
        RecommendedAction::ManualReview | RecommendedAction::EscalateInvestigation => {
            DecisionOutcome::ManualReview
        }
    }
}

fn authority_for_outcome(outcome: DecisionOutcome) -> DecisionAuthority {
    match outcome {
        DecisionOutcome::QaSample => DecisionAuthority::QaPolicy,
        DecisionOutcome::ManualReview => DecisionAuthority::HumanReviewer,
        _ => DecisionAuthority::RiskRoutingPolicy,
    }
}

fn decision_confidence(confidence: &str) -> DecisionConfidence {
    match confidence {
        "High" => DecisionConfidence::High,
        "Medium" => DecisionConfidence::Medium,
        _ => DecisionConfidence::Low,
    }
}

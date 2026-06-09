use super::ops_rules::{RulePromotionGate, RulePromotionGatesResponse};
use crate::repository::{
    OutcomeLabelRecord, QaFeedbackItemRecord, RuleBacktestRecord, RuleDetailRecord,
    RulePerformanceRecord, RulePromotionReviewRecord, RuleShadowRunRecord, RuleSummaryRecord,
};
use fwa_core::RuleActionClass;
use fwa_rules::RuleAction;
use rust_decimal::Decimal;

pub(super) fn build_rule_promotion_gates(
    rule: &RuleSummaryRecord,
    performance: &RulePerformanceRecord,
    latest_backtest: Option<&RuleBacktestRecord>,
    latest_shadow_run: Option<&RuleShadowRunRecord>,
    outcome_labels: &[OutcomeLabelRecord],
    feedback_items: &[QaFeedbackItemRecord],
    latest_review: Option<&RulePromotionReviewRecord>,
    latest_action: Option<&RuleAction>,
) -> RulePromotionGatesResponse {
    let effective_reviewed_count = performance.reviewed_count.max(
        latest_backtest
            .map(|backtest| backtest.reviewed_count)
            .unwrap_or(0),
    );
    let effective_false_positive_rate = if performance.reviewed_count > 0 {
        performance.false_positive_rate
    } else {
        latest_backtest
            .map(|backtest| backtest.false_positive_rate)
            .unwrap_or(0.0)
    };
    let performance_saving = decimal_from_string(&performance.saving_amount);
    let backtest_saving = latest_backtest
        .map(|backtest| decimal_from_string(&backtest.estimated_saving))
        .unwrap_or(Decimal::ZERO);
    let effective_saving = if performance_saving > Decimal::ZERO {
        performance_saving
    } else {
        backtest_saving
    };
    let has_review_evidence = effective_reviewed_count > 0;
    let backtest_blockers_clear = latest_backtest
        .map(|backtest| backtest.blockers.is_empty())
        .unwrap_or(true);
    let has_saving_evidence = effective_saving > Decimal::ZERO;
    let review_evidence_source = review_evidence_source(performance, latest_backtest);
    let saving_evidence_source = saving_evidence_source(performance, latest_backtest);
    let approved = latest_review
        .map(|review| review.decision == "approved")
        .unwrap_or_else(|| matches!(rule.status.as_str(), "approved" | "active"));
    let passed_shadow_run =
        latest_shadow_run.filter(|run| run.decision == "shadow_passed" && run.blockers.is_empty());
    let runtime_shadow_rollout = performance.trigger_count > 0 && performance.reviewed_count > 0;
    let shadow_rollout = passed_shadow_run.is_some() || runtime_shadow_rollout;
    let shadow_evidence_source = if passed_shadow_run.is_some() {
        "shadow"
    } else if runtime_shadow_rollout {
        "runtime"
    } else {
        "missing"
    };
    let rule_feedback_items = feedback_items
        .iter()
        .filter(|item| {
            item.feedback_target == "rules" && feedback_targets_rule(&item.evidence_refs, rule)
        })
        .collect::<Vec<_>>();
    let open_rule_feedback_count = rule_feedback_items
        .iter()
        .filter(|item| item.status == "open")
        .count();
    let unresolved_rule_feedback_count = rule_feedback_items
        .iter()
        .filter(|item| is_unresolved_feedback_status(&item.status))
        .count();
    let rule_feedback_labels = outcome_labels
        .iter()
        .filter(|label| {
            label.feedback_target == "rules" && feedback_targets_rule(&label.evidence_refs, rule)
        })
        .collect::<Vec<_>>();
    let approved_rule_feedback = rule_feedback_labels
        .iter()
        .filter(|label| label.governance_status == "approved_for_training")
        .count();
    let needs_review_rule_feedback = rule_feedback_labels
        .iter()
        .filter(|label| label.governance_status == "needs_review")
        .count();
    let unresolved_rule_feedback = rule_feedback_labels
        .iter()
        .any(|label| label.governance_status == "needs_review");
    let rule_feedback_governance = !unresolved_rule_feedback;
    let mut gates = vec![
        rule_gate(
            "Named owner",
            !rule.owner.trim().is_empty(),
            "owner missing",
            if rule.owner.trim().is_empty() {
                "missing"
            } else {
                "metadata"
            },
        ),
        rule_gate(
            "Deterministic backtest evidence",
            has_review_evidence && backtest_blockers_clear,
            backtest_evidence_blocker(has_review_evidence, backtest_blockers_clear),
            review_evidence_source,
        ),
        rule_gate(
            "Estimated saving",
            has_saving_evidence,
            "estimated saving missing",
            saving_evidence_source,
        ),
        rule_gate(
            "False-positive burden",
            has_review_evidence && effective_false_positive_rate <= 0.30,
            "false-positive burden missing",
            if has_review_evidence && effective_false_positive_rate <= 0.30 {
                review_evidence_source
            } else {
                "missing"
            },
        ),
        rule_gate(
            "Approval before routing",
            approved,
            "approval missing",
            if approved { "approval" } else { "missing" },
        ),
        rule_gate(
            "Rule QA feedback closure",
            unresolved_rule_feedback_count == 0,
            "unresolved rule QA feedback",
            "qa_feedback",
        ),
        rule_gate(
            "Rule feedback governance",
            rule_feedback_governance,
            "rule feedback labels need review",
            if rule_feedback_labels.is_empty() {
                "missing"
            } else {
                "labels"
            },
        ),
        rule_gate(
            "Shadow or limited rollout",
            shadow_rollout,
            "shadow rollout missing",
            shadow_evidence_source,
        ),
        rule_gate(
            "Rollback path",
            rule.latest_version > 0,
            "rollback path missing",
            if rule.latest_version > 0 {
                "metadata"
            } else {
                "missing"
            },
        ),
    ];
    if let Some(action) = latest_action.filter(|action| deterministic_adjudication_action(action)) {
        gates.extend(adjudication_policy_gates(action, shadow_rollout));
    }
    let blockers = gates
        .iter()
        .filter(|gate| !gate.passed)
        .map(|gate| gate.blocker.clone())
        .collect::<Vec<_>>();

    RulePromotionGatesResponse {
        rule_id: rule.rule_id.clone(),
        rule_version: rule.latest_version,
        review_mode: rule.review_mode.clone(),
        decision: if blockers.is_empty() {
            "routing_allowed".into()
        } else {
            "routing_blocked".into()
        },
        status: rule.status.clone(),
        passed_count: gates.len() - blockers.len(),
        total_count: gates.len(),
        trigger_count: performance.trigger_count,
        reviewed_count: effective_reviewed_count,
        false_positive_rate: effective_false_positive_rate,
        saving_amount: format!("{:.2}", effective_saving.round_dp(2)),
        open_rule_feedback_count,
        unresolved_rule_feedback_count,
        approved_label_count: approved_rule_feedback,
        needs_review_label_count: needs_review_rule_feedback,
        gates,
        blockers,
    }
}

pub(super) fn latest_rule_action(detail: &RuleDetailRecord) -> Option<RuleAction> {
    detail
        .versions
        .iter()
        .find(|version| version.version == detail.summary.latest_version)
        .and_then(|version| serde_json::from_value(version.dsl["action"].clone()).ok())
}

fn deterministic_adjudication_action(action: &RuleAction) -> bool {
    matches!(
        action.action_class,
        RuleActionClass::HardDeny | RuleActionClass::StraightThrough
    )
}

fn adjudication_policy_gates(action: &RuleAction, shadow_rollout: bool) -> Vec<RulePromotionGate> {
    let policy = action.adjudication_policy.as_ref();
    let has_customer_approval = policy
        .map(|policy| non_empty(&policy.customer_approval_ref))
        .unwrap_or(false);
    let has_authority_and_exception = action.required_evidence.iter().any(|evidence| {
        evidence
            .policy_authority_ref
            .as_deref()
            .is_some_and(non_empty)
            && evidence.exception_check.as_deref().is_some_and(non_empty)
    });
    let has_appeal_or_override = policy
        .map(|policy| non_empty(&policy.appeal_or_override_route))
        .unwrap_or(false);
    let has_effective_date_and_rollback = policy
        .map(|policy| non_empty(&policy.effective_date) && non_empty(&policy.rollback_plan_ref))
        .unwrap_or(false);
    let has_production_threshold = policy
        .map(|policy| non_empty(&policy.production_threshold_ref))
        .unwrap_or(false);
    let has_routing_impact = policy
        .map(|policy| non_empty(&policy.routing_impact_ref))
        .unwrap_or(false)
        && shadow_rollout;
    vec![
        rule_gate(
            "Customer-approved adjudication rule list",
            has_customer_approval,
            "customer-approved rule list missing",
            if has_customer_approval {
                "approval"
            } else {
                "missing"
            },
        ),
        rule_gate(
            "Policy authority and exception check",
            has_authority_and_exception,
            "policy authority or exception check missing",
            if has_authority_and_exception {
                "metadata"
            } else {
                "missing"
            },
        ),
        rule_gate(
            "Appeal or override route",
            has_appeal_or_override,
            "appeal or override route missing",
            if has_appeal_or_override {
                "metadata"
            } else {
                "missing"
            },
        ),
        rule_gate(
            "Effective date and rollback plan",
            has_effective_date_and_rollback,
            "effective date or rollback plan missing",
            if has_effective_date_and_rollback {
                "metadata"
            } else {
                "missing"
            },
        ),
        rule_gate(
            "Production thresholds",
            has_production_threshold,
            "production thresholds missing",
            if has_production_threshold {
                "metadata"
            } else {
                "missing"
            },
        ),
        rule_gate(
            "Routing impact promotion",
            has_routing_impact,
            "routing impact evidence missing",
            if has_routing_impact {
                "runtime"
            } else {
                "missing"
            },
        ),
    ]
}

fn non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn decimal_from_string(value: &str) -> Decimal {
    value.parse::<Decimal>().unwrap_or(Decimal::ZERO)
}

fn is_unresolved_feedback_status(status: &str) -> bool {
    matches!(status, "open" | "in_progress")
}

fn review_evidence_source(
    performance: &RulePerformanceRecord,
    latest_backtest: Option<&RuleBacktestRecord>,
) -> &'static str {
    if performance.reviewed_count > 0 {
        "runtime"
    } else if latest_backtest
        .map(|backtest| backtest.reviewed_count > 0)
        .unwrap_or(false)
    {
        "backtest"
    } else {
        "missing"
    }
}

fn saving_evidence_source(
    performance: &RulePerformanceRecord,
    latest_backtest: Option<&RuleBacktestRecord>,
) -> &'static str {
    if decimal_from_string(&performance.saving_amount) > Decimal::ZERO {
        "runtime"
    } else if latest_backtest
        .map(|backtest| decimal_from_string(&backtest.estimated_saving) > Decimal::ZERO)
        .unwrap_or(false)
    {
        "backtest"
    } else {
        "missing"
    }
}

fn backtest_evidence_blocker(
    has_review_evidence: bool,
    backtest_blockers_clear: bool,
) -> &'static str {
    if !has_review_evidence {
        "backtest evidence missing"
    } else if !backtest_blockers_clear {
        "backtest blockers unresolved"
    } else {
        "none"
    }
}

fn rule_gate(label: &str, passed: bool, blocker: &str, evidence_source: &str) -> RulePromotionGate {
    RulePromotionGate {
        label: label.into(),
        passed,
        blocker: blocker.into(),
        evidence_source: evidence_source.into(),
    }
}

fn feedback_targets_rule(evidence_refs: &[String], rule: &RuleSummaryRecord) -> bool {
    let rule_run_ref = format!("rule_runs:{}", rule.alert_code);
    evidence_refs.iter().any(|reference| {
        reference == &rule_run_ref
            || reference
                .strip_prefix("rules:")
                .and_then(|source_id| source_id.split(":v").next())
                == Some(rule.rule_id.as_str())
    })
}

pub(super) fn empty_rule_performance(rule: &RuleSummaryRecord) -> RulePerformanceRecord {
    RulePerformanceRecord {
        rule_id: rule.rule_id.clone(),
        alert_code: rule.alert_code.clone(),
        trigger_count: 0,
        reviewed_count: 0,
        confirmed_fwa_count: 0,
        false_positive_count: 0,
        mark_rate: 0.0,
        precision: 0.0,
        false_positive_rate: 0.0,
        saving_amount: "0.00".into(),
        roi: 0.0,
    }
}

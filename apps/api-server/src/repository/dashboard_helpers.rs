use super::{
    canonical_feedback_target, drift_summary, format_decimal_cents, is_terminal_case_status, ratio,
    AgentRunLogRecord, AuditSampleRecord, CaseRecord, DashboardAgentGovernanceRecord,
    DashboardAuditCoverageRecord, DashboardCaseSlaRecord, DashboardLabelPoolRecord,
    DashboardModelGovernanceRecord, DashboardQaQueueRecord, DashboardRuleGovernanceRecord,
    DashboardValueMeasurementRecord, FinancialImpactRecord, ModelEvaluationRecord,
    ModelVersionRecord, OutcomeLabelRecord, QaFeedbackItemRecord, QaReviewRecord,
    RulePerformanceRecord, RuleSummaryRecord, RULE_REVIEW_COST_AMOUNT,
};
use rust_decimal::Decimal;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn summarize_dashboard_audit_coverage(
    scoring_runs: u32,
    canonical_trace_runs: u32,
) -> DashboardAuditCoverageRecord {
    DashboardAuditCoverageRecord {
        scoring_runs,
        canonical_trace_runs,
        canonical_trace_coverage: if scoring_runs == 0 {
            0.0
        } else {
            canonical_trace_runs as f64 / scoring_runs as f64
        },
    }
}

pub(super) fn summarize_dashboard_label_pool(
    labels: &[OutcomeLabelRecord],
) -> DashboardLabelPoolRecord {
    DashboardLabelPoolRecord {
        total_labels: labels.len() as u32,
        approved_for_training: labels
            .iter()
            .filter(|label| label.governance_status == "approved_for_training")
            .count() as u32,
        needs_review: labels
            .iter()
            .filter(|label| label.governance_status == "needs_review")
            .count() as u32,
        rule_feedback: labels
            .iter()
            .filter(|label| label.feedback_target == "rules")
            .count() as u32,
        model_feedback: labels
            .iter()
            .filter(|label| canonical_feedback_target(&label.feedback_target) == "model")
            .count() as u32,
        features_feedback: labels
            .iter()
            .filter(|label| label.feedback_target == "features")
            .count() as u32,
        provider_profile_feedback: labels
            .iter()
            .filter(|label| label.feedback_target == "provider_profile")
            .count() as u32,
        workflow_feedback: labels
            .iter()
            .filter(|label| label.feedback_target == "workflow")
            .count() as u32,
        case_status_labels: labels
            .iter()
            .filter(|label| label.source_type == "case_status")
            .count() as u32,
        medical_review_labels: labels
            .iter()
            .filter(|label| label.source_type == "medical_review")
            .count() as u32,
        false_positive_labels: labels
            .iter()
            .filter(|label| label.label_name == "false_positive")
            .count() as u32,
        evidence_backed_labels: labels
            .iter()
            .filter(|label| !label.evidence_refs.is_empty())
            .count() as u32,
    }
}

pub(super) fn summarize_dashboard_qa_queue(
    samples: &[AuditSampleRecord],
    reviews: &[QaReviewRecord],
    feedback_items: &[QaFeedbackItemRecord],
) -> DashboardQaQueueRecord {
    let reviewed_case_ids = reviews
        .iter()
        .map(|review| review.qa_case_id.as_str())
        .collect::<BTreeSet<_>>();
    let disagreement_case_ids = reviews
        .iter()
        .filter(|review| review.qa_conclusion != "pass")
        .map(|review| review.qa_case_id.as_str())
        .collect::<BTreeSet<_>>();
    let sampled_cases = samples
        .iter()
        .map(|sample| sample.selected_leads.len() as u32)
        .sum::<u32>();
    let sampled_qa_case_ids = samples
        .iter()
        .flat_map(|sample| {
            sample.selected_leads.iter().map(move |lead| {
                format!("qa_{}_{}", sample.sample_id.as_str(), lead.lead_id.as_str())
            })
        })
        .collect::<Vec<_>>();
    let reviewed_cases = sampled_qa_case_ids
        .iter()
        .filter(|qa_case_id| reviewed_case_ids.contains(qa_case_id.as_str()))
        .count() as u32;
    let disagreement_cases = sampled_qa_case_ids
        .iter()
        .filter(|qa_case_id| disagreement_case_ids.contains(qa_case_id.as_str()))
        .count() as u32;
    let disagreement_rate = if reviewed_cases == 0 {
        0.0
    } else {
        disagreement_cases as f64 / reviewed_cases as f64
    };
    let feedback_open_count = count_feedback_status(feedback_items, "open");
    let feedback_in_progress_count = count_feedback_status(feedback_items, "in_progress");
    let unresolved_feedback_items = feedback_items
        .iter()
        .filter(|item| matches!(item.status.as_str(), "open" | "in_progress"))
        .collect::<Vec<_>>();

    DashboardQaQueueRecord {
        sampled_cases,
        open_cases: sampled_cases.saturating_sub(reviewed_cases),
        reviewed_cases,
        disagreement_cases,
        disagreement_rate,
        feedback_open_count,
        feedback_in_progress_count,
        feedback_resolved_count: count_feedback_status(feedback_items, "resolved"),
        feedback_dismissed_count: count_feedback_status(feedback_items, "dismissed"),
        unresolved_feedback_count: feedback_open_count + feedback_in_progress_count,
        rules_unresolved_feedback_count: count_feedback_target(&unresolved_feedback_items, "rules"),
        models_unresolved_feedback_count: count_feedback_target(
            &unresolved_feedback_items,
            "model",
        ),
        features_unresolved_feedback_count: count_feedback_target(
            &unresolved_feedback_items,
            "features",
        ),
        provider_profile_unresolved_feedback_count: count_feedback_target(
            &unresolved_feedback_items,
            "provider_profile",
        ),
        workflow_unresolved_feedback_count: count_feedback_target(
            &unresolved_feedback_items,
            "workflow",
        ),
        tpa_unresolved_feedback_count: count_feedback_target(&unresolved_feedback_items, "tpa"),
    }
}

fn count_feedback_status(items: &[QaFeedbackItemRecord], status: &str) -> u32 {
    items.iter().filter(|item| item.status == status).count() as u32
}

fn count_feedback_target(items: &[&QaFeedbackItemRecord], feedback_target: &str) -> u32 {
    items
        .iter()
        .filter(|item| {
            canonical_feedback_target(&item.feedback_target)
                == canonical_feedback_target(feedback_target)
        })
        .count() as u32
}

pub(super) fn summarize_dashboard_case_sla(cases: &[CaseRecord]) -> DashboardCaseSlaRecord {
    let total_cases = cases.len() as u32;
    let closed_cases = cases
        .iter()
        .filter(|case| is_terminal_case_status(&case.status))
        .count() as u32;
    let breached_cases = cases
        .iter()
        .filter(|case| case.sla_status == "breached" || case.sla_status == "closed_breached")
        .count() as u32;
    let closure_times = cases
        .iter()
        .filter_map(|case| case.time_to_closure_hours)
        .collect::<Vec<_>>();
    DashboardCaseSlaRecord {
        total_cases,
        open_cases: total_cases.saturating_sub(closed_cases),
        closed_cases,
        breached_cases,
        sla_breach_rate: if total_cases == 0 {
            0.0
        } else {
            breached_cases as f64 / total_cases as f64
        },
        average_time_to_triage_hours: average_hours(
            cases.iter().map(|case| case.time_to_triage_hours),
        ),
        average_time_to_closure_hours: average_hours(closure_times.into_iter()),
    }
}

fn average_hours(values: impl Iterator<Item = f64>) -> f64 {
    let mut count = 0_u32;
    let mut sum = 0.0;
    for value in values {
        count += 1;
        sum += value;
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

pub(super) fn summarize_dashboard_agent_governance(
    runs: &[AgentRunLogRecord],
) -> DashboardAgentGovernanceRecord {
    let mut pending_approvals = 0_u32;
    let mut approved_approvals = 0_u32;
    let mut rejected_approvals = 0_u32;

    for approval in runs.iter().flat_map(|run| run.approvals.iter()) {
        match approval.decision.as_str() {
            "pending" => pending_approvals += 1,
            "approved" => approved_approvals += 1,
            "rejected" => rejected_approvals += 1,
            _ => {}
        }
    }

    DashboardAgentGovernanceRecord {
        total_runs: runs.len() as u32,
        successful_runs: runs.iter().filter(|run| run.status == "succeeded").count() as u32,
        evidence_backed_runs: runs
            .iter()
            .filter(|run| !run.evidence_refs.is_empty())
            .count() as u32,
        tool_call_count: runs.iter().map(|run| run.tool_calls.len() as u32).sum(),
        policy_check_count: runs.iter().map(|run| run.policy_checks.len() as u32).sum(),
        denied_policy_check_count: runs
            .iter()
            .flat_map(|run| run.policy_checks.iter())
            .filter(|check| check.decision == "denied")
            .count() as u32,
        failed_tool_call_count: runs
            .iter()
            .flat_map(|run| run.tool_calls.iter())
            .filter(|call| call.status == "failed")
            .count() as u32,
        pending_approvals,
        approved_approvals,
        rejected_approvals,
    }
}

pub(super) fn summarize_dashboard_model_governance(
    models: &[ModelVersionRecord],
    evaluations: &[ModelEvaluationRecord],
) -> DashboardModelGovernanceRecord {
    let known_models = models
        .iter()
        .map(|model| (model.model_key.as_str(), model.version.as_str()))
        .collect::<BTreeSet<_>>();
    let mut latest_evaluations = BTreeMap::<(&str, &str), &ModelEvaluationRecord>::new();
    for evaluation in evaluations {
        let key = (
            evaluation.model_key.as_str(),
            evaluation.model_version.as_str(),
        );
        if !known_models.contains(&key) {
            continue;
        }
        latest_evaluations
            .entry(key)
            .and_modify(|existing| {
                if evaluation.evaluation_run_id > existing.evaluation_run_id {
                    *existing = evaluation;
                }
            })
            .or_insert(evaluation);
    }

    let mut drift_watch_count = 0_u32;
    let mut drift_detected_count = 0_u32;
    let mut precision_values = Vec::new();
    let mut recall_values = Vec::new();

    for evaluation in latest_evaluations.values() {
        match drift_summary(&evaluation.metrics_json).1.as_str() {
            "watch" => drift_watch_count += 1,
            "drift" => drift_detected_count += 1,
            _ => {}
        }
        if let Some(precision) = evaluation.precision.as_ref() {
            precision_values.push(decimal_to_f64(precision));
        }
        if let Some(recall) = evaluation.recall.as_ref() {
            recall_values.push(decimal_to_f64(recall));
        }
    }

    DashboardModelGovernanceRecord {
        total_models: models.len() as u32,
        evaluated_models: latest_evaluations.len() as u32,
        drift_watch_count,
        drift_detected_count,
        average_precision: average_f64(&precision_values),
        average_recall: average_f64(&recall_values),
    }
}

pub(super) fn summarize_dashboard_rule_governance(
    rules: &[RuleSummaryRecord],
    performance: &[RulePerformanceRecord],
) -> DashboardRuleGovernanceRecord {
    let total_trigger_count = performance
        .iter()
        .map(|record| record.trigger_count)
        .sum::<u32>();
    let reviewed_count = performance
        .iter()
        .map(|record| record.reviewed_count)
        .sum::<u32>();
    let confirmed_fwa_count = performance
        .iter()
        .map(|record| record.confirmed_fwa_count)
        .sum::<u32>();
    let false_positive_count = performance
        .iter()
        .map(|record| record.false_positive_count)
        .sum::<u32>();
    let saving_amount = performance
        .iter()
        .map(|record| {
            record
                .saving_amount
                .parse::<Decimal>()
                .unwrap_or(Decimal::ZERO)
        })
        .sum::<Decimal>();
    let saving = decimal_to_f64(&saving_amount);
    let review_cost = total_trigger_count as f64 * RULE_REVIEW_COST_AMOUNT;

    DashboardRuleGovernanceRecord {
        total_rules: rules.len() as u32,
        active_rules: rules.iter().filter(|rule| rule.status == "active").count() as u32,
        triggered_rules: performance
            .iter()
            .filter(|record| record.trigger_count > 0)
            .count() as u32,
        total_trigger_count,
        reviewed_count,
        confirmed_fwa_count,
        false_positive_count,
        precision: ratio(confirmed_fwa_count, reviewed_count),
        false_positive_rate: ratio(false_positive_count, reviewed_count),
        saving_amount: format_decimal_cents(saving_amount),
        roi: if review_cost == 0.0 {
            0.0
        } else {
            saving / review_cost
        },
    }
}

pub(super) fn summarize_dashboard_value_measurement(
    impacts: &[FinancialImpactRecord],
    review_events: u32,
    false_positive_events: u32,
) -> DashboardValueMeasurementRecord {
    let mut prevented_payment = Decimal::ZERO;
    let mut recovered_amount = Decimal::ZERO;
    let mut avoided_future_exposure = Decimal::ZERO;
    let mut deterrence_estimate = Decimal::ZERO;
    let mut other_estimated_impact = Decimal::ZERO;
    let mut currency = None;

    for impact in impacts {
        if currency.is_none() {
            currency = impact.currency.clone();
        }
        match impact.impact_type.as_str() {
            "recovered_amount" => recovered_amount += impact.amount,
            "avoided_future_exposure" => avoided_future_exposure += impact.amount,
            "deterrence_estimate" => deterrence_estimate += impact.amount,
            "estimated_impact" => other_estimated_impact += impact.amount,
            _ => prevented_payment += impact.amount,
        }
    }

    let review_cost = Decimal::from(review_events) * Decimal::from(RULE_REVIEW_COST_AMOUNT as u32);
    let false_positive_operational_cost =
        Decimal::from(false_positive_events) * Decimal::from(RULE_REVIEW_COST_AMOUNT as u32);
    let reviewer_capacity_hours = Decimal::from(review_events) * Decimal::new(25, 2);
    let estimated_impact = avoided_future_exposure + deterrence_estimate + other_estimated_impact;
    let net_value = prevented_payment + recovered_amount + estimated_impact - review_cost;

    DashboardValueMeasurementRecord {
        prevented_payment: format_decimal_cents(prevented_payment),
        recovered_amount: format_decimal_cents(recovered_amount),
        avoided_future_exposure: format_decimal_cents(avoided_future_exposure),
        deterrence_estimate: format_decimal_cents(deterrence_estimate),
        estimated_impact: format_decimal_cents(estimated_impact),
        review_cost: format_decimal_cents(review_cost),
        false_positive_operational_cost: format_decimal_cents(false_positive_operational_cost),
        reviewer_capacity_hours: format_decimal_cents(reviewer_capacity_hours),
        net_value: format_decimal_cents(net_value),
        currency: currency.unwrap_or_else(|| "CNY".into()),
        evidence_caveat:
            "Observed values come from confirmed investigation outcomes; avoided exposure and deterrence remain estimated until validated."
                .into(),
    }
}

pub(super) fn decimal_to_f64(value: &Decimal) -> f64 {
    value.to_string().parse().unwrap_or(0.0)
}

fn average_f64(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

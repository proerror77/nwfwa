use super::{RulePerformanceRecord, RuleSummaryRecord};
use rust_decimal::Decimal;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub(super) const RULE_REVIEW_COST_AMOUNT: f64 = 100.0;

#[derive(Debug, Clone)]
pub(super) struct InvestigationOutcome {
    pub(super) confirmed_fwa: bool,
    pub(super) saving_amount: Decimal,
}

#[derive(Debug, Clone)]
pub(super) struct RulePerformanceAccumulator {
    pub(super) rule_id: String,
    pub(super) alert_code: String,
    pub(super) trigger_count: u32,
    pub(super) triggered_claim_ids: BTreeSet<String>,
}

pub(super) fn rule_accumulators_from_rules(
    rules: &[RuleSummaryRecord],
) -> BTreeMap<String, RulePerformanceAccumulator> {
    rules
        .iter()
        .map(|rule| {
            (
                rule.rule_id.clone(),
                RulePerformanceAccumulator {
                    rule_id: rule.rule_id.clone(),
                    alert_code: rule.alert_code.clone(),
                    trigger_count: 0,
                    triggered_claim_ids: BTreeSet::new(),
                },
            )
        })
        .collect()
}

pub(super) fn rule_performance_records(
    accumulators: BTreeMap<String, RulePerformanceAccumulator>,
    outcomes: &HashMap<String, InvestigationOutcome>,
    total_scoring_runs: u32,
) -> Vec<RulePerformanceRecord> {
    accumulators
        .into_values()
        .map(|accumulator| {
            let mut reviewed_count = 0_u32;
            let mut confirmed_fwa_count = 0_u32;
            let mut false_positive_count = 0_u32;
            let mut saving_amount = Decimal::ZERO;

            for claim_id in &accumulator.triggered_claim_ids {
                let Some(outcome) = outcomes.get(claim_id) else {
                    continue;
                };
                reviewed_count += 1;
                if outcome.confirmed_fwa {
                    confirmed_fwa_count += 1;
                    saving_amount += outcome.saving_amount;
                } else {
                    false_positive_count += 1;
                }
            }

            let trigger_count = accumulator.trigger_count;
            let mark_rate = ratio(trigger_count, total_scoring_runs);
            let precision = ratio(confirmed_fwa_count, reviewed_count);
            let false_positive_rate = ratio(false_positive_count, reviewed_count);
            let roi = if trigger_count == 0 {
                0.0
            } else {
                let saving = saving_amount.to_string().parse::<f64>().unwrap_or(0.0);
                saving / (trigger_count as f64 * RULE_REVIEW_COST_AMOUNT)
            };

            RulePerformanceRecord {
                rule_id: accumulator.rule_id,
                alert_code: accumulator.alert_code,
                trigger_count,
                reviewed_count,
                confirmed_fwa_count,
                false_positive_count,
                mark_rate,
                precision,
                false_positive_rate,
                saving_amount: format!("{:.2}", saving_amount.round_dp(2)),
                roi,
            }
        })
        .collect()
}

pub(super) fn ratio(numerator: u32, denominator: u32) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

pub(super) fn decimal_from_json(value: &Value) -> Decimal {
    if let Some(value) = value.as_str() {
        return value.parse::<Decimal>().unwrap_or(Decimal::ZERO);
    }
    if let Some(value) = value.as_f64() {
        return Decimal::from_f64_retain(value).unwrap_or(Decimal::ZERO);
    }
    Decimal::ZERO
}

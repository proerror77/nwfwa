use super::ops_rules::{RuleDiscoveryCandidate, RuleDiscoveryRequest};
use super::ops_rules_mining_samples::{normalized_optional_str, MiningSample};
use fwa_core::{RecommendedAction, RuleActionClass};
use fwa_rules::{Condition, Rule, RuleAction};
use rust_decimal::Decimal;

mod tree;
use tree::mine_tree_rule_candidates;

#[derive(Debug)]
struct FeatureSplitCandidate {
    feature: String,
    operator: &'static str,
    threshold: f64,
    support: usize,
    true_positive_count: usize,
    false_positive_count: usize,
    precision: f64,
    recall: f64,
    lift: f64,
    false_positive_rate: f64,
    saving: Decimal,
    matched_claim_ids: Vec<String>,
    positive_mean: f64,
    negative_mean: f64,
    negative_stddev: f64,
    statistical_threshold: f64,
    model_reason: Option<String>,
}

pub(super) fn mine_statistical_rule_candidates(
    request: &RuleDiscoveryRequest,
    samples: &[MiningSample],
    min_support: usize,
    positive_count: usize,
    baseline_rate: f64,
    evidence_refs: &[String],
) -> Vec<RuleDiscoveryCandidate> {
    let mut features = samples
        .iter()
        .flat_map(|sample| sample.features.keys().cloned())
        .collect::<Vec<_>>();
    features.sort();
    features.dedup();
    if let Some(requested_features) = &request.candidate_feature_fields {
        if !requested_features.is_empty() {
            features.retain(|feature| {
                requested_features
                    .iter()
                    .any(|requested| requested == feature)
            });
        }
    }

    let mut candidates = mine_tree_rule_candidates(
        request,
        samples,
        &features,
        min_support,
        positive_count,
        baseline_rate,
        evidence_refs,
    );
    candidates.extend(
        features
            .into_iter()
            .filter_map(|feature| {
                best_feature_split(
                    &feature,
                    request,
                    samples,
                    min_support,
                    positive_count,
                    baseline_rate,
                )
            })
            .map(|split| split.into_response(evidence_refs)),
    );
    candidates.sort_by(|left, right| {
        right
            .precision
            .partial_cmp(&left.precision)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                right
                    .lift
                    .partial_cmp(&left.lift)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| right.support.cmp(&left.support))
            .then_with(|| left.rule.rule_id.cmp(&right.rule.rule_id))
    });
    candidates
}

fn best_feature_split(
    feature: &str,
    request: &RuleDiscoveryRequest,
    samples: &[MiningSample],
    min_support: usize,
    positive_count: usize,
    baseline_rate: f64,
) -> Option<FeatureSplitCandidate> {
    let values = samples
        .iter()
        .filter_map(|sample| {
            sample
                .features
                .get(feature)
                .filter(|value| value.is_finite())
                .map(|value| (sample, *value))
        })
        .collect::<Vec<_>>();
    if values.len() < min_support {
        return None;
    }

    let positive_values = values
        .iter()
        .filter_map(|(sample, value)| (sample.confirmed_fwa == Some(true)).then_some(*value))
        .collect::<Vec<_>>();
    let negative_values = values
        .iter()
        .filter_map(|(sample, value)| (sample.confirmed_fwa == Some(false)).then_some(*value))
        .collect::<Vec<_>>();
    if positive_values.is_empty() || negative_values.is_empty() {
        return None;
    }

    let positive_mean = mean(&positive_values);
    let negative_mean = mean(&negative_values);
    let negative_stddev = stddev(&negative_values, negative_mean);
    let high_risk_when_higher = positive_mean >= negative_mean;
    let operator = if high_risk_when_higher { ">=" } else { "<=" };
    let statistical_threshold = if high_risk_when_higher {
        negative_mean + (1.5 * negative_stddev)
    } else {
        negative_mean - (1.5 * negative_stddev)
    };

    let mut thresholds = values.iter().map(|(_, value)| *value).collect::<Vec<_>>();
    thresholds.push(statistical_threshold);
    thresholds.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    thresholds.dedup_by(|left, right| (*left - *right).abs() < 0.000001);

    thresholds
        .into_iter()
        .filter_map(|threshold| {
            score_feature_threshold(
                feature,
                operator,
                threshold,
                samples,
                min_support,
                positive_count,
                baseline_rate,
                positive_mean,
                negative_mean,
                negative_stddev,
                statistical_threshold,
                model_explanation_reason(request, feature),
            )
        })
        .max_by(|left, right| {
            left.precision
                .partial_cmp(&right.precision)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    left.lift
                        .partial_cmp(&right.lift)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| left.support.cmp(&right.support))
        })
}

fn score_feature_threshold(
    feature: &str,
    operator: &'static str,
    threshold: f64,
    samples: &[MiningSample],
    min_support: usize,
    positive_count: usize,
    baseline_rate: f64,
    positive_mean: f64,
    negative_mean: f64,
    negative_stddev: f64,
    statistical_threshold: f64,
    model_reason: Option<String>,
) -> Option<FeatureSplitCandidate> {
    let mut matched_claim_ids = Vec::new();
    let mut true_positive_count = 0_usize;
    let mut false_positive_count = 0_usize;
    let mut saving = Decimal::ZERO;

    for sample in samples {
        let Some(value) = sample.features.get(feature).copied() else {
            continue;
        };
        let matched = if operator == ">=" {
            value >= threshold
        } else {
            value <= threshold
        };
        if !matched {
            continue;
        }
        matched_claim_ids.push(sample.claim_id.clone());
        match sample.confirmed_fwa {
            Some(true) => {
                true_positive_count += 1;
                saving += sample.claim_amount * Decimal::new(10, 2);
            }
            Some(false) => false_positive_count += 1,
            None => {}
        }
    }

    let support = matched_claim_ids.len();
    if support < min_support || true_positive_count == 0 {
        return None;
    }
    let precision = true_positive_count as f64 / support as f64;
    if baseline_rate > 0.0 && precision <= baseline_rate {
        return None;
    }
    let recall = true_positive_count as f64 / positive_count as f64;
    let lift = if baseline_rate == 0.0 {
        0.0
    } else {
        precision / baseline_rate
    };
    Some(FeatureSplitCandidate {
        feature: feature.into(),
        operator,
        threshold,
        support,
        true_positive_count,
        false_positive_count,
        precision,
        recall,
        lift,
        false_positive_rate: false_positive_count as f64 / support as f64,
        saving,
        matched_claim_ids,
        positive_mean,
        negative_mean,
        negative_stddev,
        statistical_threshold,
        model_reason,
    })
}

impl FeatureSplitCandidate {
    fn into_response(self, base_evidence_refs: &[String]) -> RuleDiscoveryCandidate {
        let feature_slug = rule_id_slug(&self.feature);
        let threshold_slug = threshold_slug(self.threshold);
        let op_slug = if self.operator == ">=" { "gte" } else { "lte" };
        let rule = Rule {
            rule_id: format!("candidate_mined_{feature_slug}_{op_slug}_{threshold_slug}"),
            version: 1,
            name: format!(
                "Mined rule: {} {} {}",
                self.feature,
                self.operator,
                format_threshold(self.threshold)
            ),
            review_mode: "both".into(),
            scheme_family: Some("high_risk_claim".into()),
            conditions: vec![Condition {
                field: self.feature.clone(),
                operator: self.operator.into(),
                value: serde_json::json!(round_float(self.threshold)),
            }],
            action: RuleAction {
                score: mined_rule_score(self.precision, self.lift),
                alert_code: format!("MINED_{}", feature_slug.to_uppercase()),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                required_evidence: vec![],
                adjudication_policy: None,
                reason: format!(
                    "数据集挖掘显示 {} {} {} 的样本 FWA 命中率高于基线，需人工解释性 review",
                    self.feature,
                    self.operator,
                    format_threshold(self.threshold)
                ),
            },
        };
        let mut evidence_refs = base_evidence_refs.to_vec();
        evidence_refs.push(format!("rule_mining:{}:decision_stump", self.feature));
        evidence_refs.push(format!(
            "rule_mining:{}:negative_mean_{}_stddev_{}",
            self.feature,
            format_threshold(self.negative_mean),
            format_threshold(self.negative_stddev)
        ));
        let model_clause = self
            .model_reason
            .as_deref()
            .map(|reason| format!(" 模型解释备注：{reason}"))
            .unwrap_or_default();
        let explanation = format!(
            "{} {} {} 是从标签数据集挖掘出的单层决策树阈值规则：正样本均值 {}，负样本均值 {}，负样本标准差 {}，统计参考阈值 {}；该候选命中 {} 条，其中确认 FWA {} 条、非 FWA {} 条，precision {:.1}%，recall {:.1}%，lift {:.2}。{}仍需人工接受或拒绝后才可进入规则库。",
            self.feature,
            self.operator,
            format_threshold(self.threshold),
            format_threshold(self.positive_mean),
            format_threshold(self.negative_mean),
            format_threshold(self.negative_stddev),
            format_threshold(self.statistical_threshold),
            self.support,
            self.true_positive_count,
            self.false_positive_count,
            self.precision * 100.0,
            self.recall * 100.0,
            self.lift,
            model_clause,
        );
        RuleDiscoveryCandidate {
            explanation,
            condition_refs: condition_refs_for_rule(&rule),
            rule,
            support: self.support,
            precision: self.precision,
            recall: self.recall,
            lift: self.lift,
            estimated_saving: format!("{:.2}", self.saving.round_dp(2)),
            false_positive_rate: self.false_positive_rate,
            matched_claim_ids: self.matched_claim_ids,
            evidence_refs,
        }
    }
}

fn model_explanation_reason(request: &RuleDiscoveryRequest, feature: &str) -> Option<String> {
    let min_abs_contribution = request.min_abs_contribution.unwrap_or(0.10);
    request
        .model_explanations
        .iter()
        .find(|explanation| {
            explanation.feature == feature
                && explanation.direction == "increases_risk"
                && explanation.contribution.is_finite()
                && explanation.contribution.abs() >= min_abs_contribution
        })
        .map(|explanation| explanation.reason.clone())
}

fn mined_rule_score(precision: f64, lift: f64) -> u8 {
    ((precision * 25.0) + lift.min(4.0) * 5.0)
        .round()
        .clamp(10.0, 45.0) as u8
}

fn condition_refs_for_rule(rule: &Rule) -> Vec<String> {
    rule.conditions
        .iter()
        .enumerate()
        .map(|(index, _)| {
            format!(
                "rule_conditions:{}_v{}_c{}",
                rule_id_slug(&rule.rule_id),
                rule.version,
                index + 1
            )
        })
        .collect()
}

pub(super) fn rule_id_slug(value: &str) -> String {
    let slug = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if slug.is_empty() {
        "feature".into()
    } else {
        slug
    }
}

pub(super) fn rule_discovery_evidence_refs(request: &RuleDiscoveryRequest) -> Vec<String> {
    let mut refs = Vec::new();
    if let Some(dataset_uri) = normalized_optional_str(request.dataset_uri.as_deref()) {
        refs.push(format!("dataset:{dataset_uri}"));
    } else {
        refs.push("dataset:inline_labeled_samples".into());
    }
    if let (Some(model_key), Some(model_version)) = (
        request.source_model_key.as_deref(),
        request.source_model_version.as_deref(),
    ) {
        refs.push(format!("model_versions:{model_key}:{model_version}"));
    }
    if let Some(feature_importance_uri) = request.feature_importance_uri.as_deref() {
        if !feature_importance_uri.trim().is_empty() {
            refs.push(format!("feature_importance:{feature_importance_uri}"));
        }
    }
    refs
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn stddev(values: &[f64], mean: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / values.len() as f64;
    variance.sqrt()
}

fn threshold_slug(value: f64) -> String {
    format_threshold(value).replace(['.', '-'], "_")
}

fn format_threshold(value: f64) -> String {
    format!("{:.4}", round_float(value))
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

fn round_float(value: f64) -> f64 {
    (value * 10000.0).round() / 10000.0
}

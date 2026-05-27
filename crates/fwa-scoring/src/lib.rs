use fwa_core::{RecommendedAction, RiskLevel, RiskScore};
use fwa_features::FeatureMap;
use fwa_ml_runtime::ModelScore;
use fwa_rules::RuleMatch;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoringDecision {
    pub risk_score: RiskScore,
    pub rag: RiskLevel,
    pub recommended_action: RecommendedAction,
    pub peer_deviation_score: u8,
    pub rule_score: u8,
    pub anomaly_score: u8,
    pub ml_score: u8,
    pub medical_reasonableness_score: u8,
    pub provider_network_score: u8,
    pub similar_case_score: u8,
    pub top_reasons: Vec<String>,
}

pub fn aggregate(
    features: &FeatureMap,
    rule_matches: &[RuleMatch],
    model_score: &ModelScore,
) -> ScoringDecision {
    let peer_deviation_score = amount_ratio_score(features);
    let rule_score = rule_matches
        .iter()
        .map(|rule_match| rule_match.score_contribution)
        .sum::<u8>()
        .min(100);
    let anomaly_score = 0;
    let medical_reasonableness_score = medical_reasonableness_score(features);
    let provider_network_score = provider_network_score(features);
    let similar_case_score = 0;
    let final_score_value = weighted_final_score(&[
        (peer_deviation_score, 0.15),
        (rule_score, 0.20),
        (model_score.score, 0.25),
        (medical_reasonableness_score, 0.10),
        (provider_network_score, 0.10),
    ]);
    let risk_score = RiskScore::new(final_score_value).expect("clamped score is valid");
    let rag = RiskLevel::from_score(risk_score);
    let recommended_action = match rag {
        RiskLevel::Green => RecommendedAction::AutoApprove,
        RiskLevel::Amber => RecommendedAction::ManualReview,
        RiskLevel::Red => RecommendedAction::EscalateInvestigation,
    };
    let mut top_reasons: Vec<String> = rule_matches
        .iter()
        .map(|rule_match| rule_match.reason.clone())
        .collect();
    if peer_deviation_score >= 80 {
        top_reasons.push("理赔金额相对保障额度显著偏高".into());
    }
    if medical_reasonableness_score >= 50 {
        top_reasons.push("账单项目数量或结构需要医疗合理性复核".into());
    }
    if provider_network_score >= 70 {
        top_reasons.push("Provider 风险画像偏高".into());
    }
    top_reasons.extend(
        model_score
            .explanations
            .iter()
            .map(|explanation| explanation.reason.clone()),
    );
    top_reasons.truncate(5);

    ScoringDecision {
        risk_score,
        rag,
        recommended_action,
        peer_deviation_score,
        rule_score,
        anomaly_score,
        ml_score: model_score.score,
        medical_reasonableness_score,
        provider_network_score,
        similar_case_score,
        top_reasons,
    }
}

fn weighted_final_score(available_scores: &[(u8, f64)]) -> u8 {
    let total_weight = available_scores
        .iter()
        .map(|(_, weight)| weight)
        .sum::<f64>();
    if total_weight == 0.0 {
        return 0;
    }
    let weighted = available_scores
        .iter()
        .map(|(score, weight)| *score as f64 * weight)
        .sum::<f64>();
    (weighted / total_weight).round().clamp(0.0, 100.0) as u8
}

fn amount_ratio_score(features: &FeatureMap) -> u8 {
    numeric_feature(features, "claim_amount_to_limit_ratio")
        .map(|ratio| (ratio * 100.0).round().clamp(0.0, 100.0) as u8)
        .unwrap_or(0)
}

fn medical_reasonableness_score(features: &FeatureMap) -> u8 {
    let item_count = numeric_feature(features, "claim_item_count").unwrap_or(0.0);
    (item_count * 12.0).round().clamp(0.0, 60.0) as u8
}

fn provider_network_score(features: &FeatureMap) -> u8 {
    match features
        .get("provider_risk_tier")
        .and_then(|feature| feature.value.as_str())
    {
        Some("HIGH") => 80,
        Some("MEDIUM") => 45,
        Some("LOW") => 10,
        _ => 0,
    }
}

fn numeric_feature(features: &FeatureMap, name: &str) -> Option<f64> {
    features.get(name).and_then(|feature| {
        feature
            .value
            .as_f64()
            .or_else(|| feature.value.as_i64().map(|value| value as f64))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use fwa_features::FeatureValue;
    use fwa_ml_runtime::ModelExplanation;
    use std::collections::BTreeMap;

    #[test]
    fn aggregates_seven_layer_scores() {
        let mut features = BTreeMap::new();
        features.insert(
            "claim_amount_to_limit_ratio".into(),
            FeatureValue {
                name: "claim_amount_to_limit_ratio".into(),
                version: 1,
                value: serde_json::json!(0.8),
                evidence_refs: vec![],
            },
        );
        features.insert(
            "claim_item_count".into(),
            FeatureValue {
                name: "claim_item_count".into(),
                version: 1,
                value: serde_json::json!(1),
                evidence_refs: vec![],
            },
        );
        features.insert(
            "provider_risk_tier".into(),
            FeatureValue {
                name: "provider_risk_tier".into(),
                version: 1,
                value: serde_json::json!("HIGH"),
                evidence_refs: vec![],
            },
        );
        let rules = vec![RuleMatch {
            rule_id: "rule_1".into(),
            rule_version: 1,
            score_contribution: 80,
            alert_code: "EARLY_HIGH_AMOUNT".into(),
            reason: "早期高额理赔".into(),
            recommended_action: RecommendedAction::ManualReview,
        }];
        let model = ModelScore {
            model_key: "baseline".into(),
            model_version: "0.1.0".into(),
            runtime_kind: "heuristic".into(),
            execution_provider: "cpu".into(),
            score: 90,
            label: "HIGH_RISK".into(),
            explanations: vec![ModelExplanation {
                feature: "claim_amount_to_limit_ratio".into(),
                direction: "increases_risk".into(),
                contribution: 0.8,
                reason: "金额比例高".into(),
            }],
            metadata: serde_json::json!({}),
            latency_ms: 0,
        };

        let decision = aggregate(&features, &rules, &model);
        assert_eq!(decision.peer_deviation_score, 80);
        assert_eq!(decision.rule_score, 80);
        assert_eq!(decision.anomaly_score, 0);
        assert_eq!(decision.ml_score, 90);
        assert_eq!(decision.medical_reasonableness_score, 12);
        assert_eq!(decision.provider_network_score, 80);
        assert_eq!(decision.similar_case_score, 0);
        assert_eq!(decision.risk_score.value(), 75);
        assert_eq!(decision.rag, RiskLevel::Red);
        assert_eq!(
            decision.recommended_action,
            RecommendedAction::EscalateInvestigation
        );
    }
}

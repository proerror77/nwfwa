use fwa_anomaly::AnomalyScore;
use fwa_core::{RecommendedAction, RiskLevel, RiskScore};
use fwa_features::FeatureMap;
use fwa_ml_runtime::ModelScore;
use fwa_rules::RuleMatch;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoringDecision {
    pub risk_score: RiskScore,
    pub rag: RiskLevel,
    pub risk_level: String,
    pub recommended_action: RecommendedAction,
    pub confidence_score: u8,
    pub confidence: String,
    pub routing_reason: String,
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
    anomaly_score: &AnomalyScore,
) -> ScoringDecision {
    let peer_deviation_score = amount_ratio_score(features);
    let rule_score = rule_matches
        .iter()
        .map(|rule_match| rule_match.score_contribution as u32)
        .sum::<u32>()
        .min(100) as u8;
    let medical_reasonableness_score = medical_reasonableness_score(features);
    let provider_network_score = provider_network_score(features);
    let similar_case_score = 0;
    let final_score_value = weighted_final_score(&[
        (peer_deviation_score, 0.15),
        (rule_score, 0.20),
        (anomaly_score.score, 0.15),
        (model_score.score, 0.25),
        (medical_reasonableness_score, 0.10),
        (provider_network_score, 0.10),
    ]);
    let risk_score = RiskScore::new(final_score_value).expect("clamped score is valid");
    let rag = RiskLevel::from_score(risk_score);
    let risk_level = risk_level(risk_score.value()).to_string();
    let confidence_score = confidence_score(rule_score, anomaly_score.score, model_score.score);
    let confidence = confidence_level(confidence_score).to_string();
    let recommended_action = recommended_action(&risk_level, confidence_score);
    let routing_reason = routing_reason(&risk_level, confidence_score).to_string();
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
        anomaly_score
            .explanations
            .iter()
            .map(|explanation| explanation.reason.clone()),
    );
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
        risk_level,
        recommended_action,
        confidence_score,
        confidence,
        routing_reason,
        peer_deviation_score,
        rule_score,
        anomaly_score: anomaly_score.score,
        ml_score: model_score.score,
        medical_reasonableness_score,
        provider_network_score,
        similar_case_score,
        top_reasons,
    }
}

fn risk_level(score: u8) -> &'static str {
    match score {
        0..=39 => "Low",
        40..=69 => "Medium",
        70..=84 => "High",
        _ => "Critical",
    }
}

fn confidence_score(rule_score: u8, anomaly_score: u8, ml_score: u8) -> u8 {
    let supporting_layers = [rule_score, anomaly_score, ml_score]
        .into_iter()
        .filter(|score| *score >= 60)
        .count() as u8;
    (55 + supporting_layers * 15).min(100)
}

fn confidence_level(score: u8) -> &'static str {
    match score {
        0..=59 => "Low",
        60..=79 => "Medium",
        _ => "High",
    }
}

fn recommended_action(risk_level: &str, confidence_score: u8) -> RecommendedAction {
    if confidence_score < 60 {
        return RecommendedAction::ManualReview;
    }
    match risk_level {
        "Low" => RecommendedAction::AutoApprove,
        "Medium" | "High" => RecommendedAction::ManualReview,
        "Critical" => RecommendedAction::EscalateInvestigation,
        _ => RecommendedAction::ManualReview,
    }
}

fn routing_reason(risk_level: &str, confidence_score: u8) -> &'static str {
    if confidence_score < 60 {
        return "置信度偏低，建议补材料或二审";
    }
    match risk_level {
        "Low" => "低风险且置信度充足，建议自动通过",
        "Medium" => "中风险，建议进入 QA 抽样",
        "High" => "高风险，建议人工审核",
        "Critical" => "关键风险，建议人工审核、医务复核并升级调查",
        _ => "风险等级未知，建议人工审核",
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
    if let Some(percentile) = numeric_feature(features, "claim_amount_peer_percentile") {
        return percentile.round().clamp(0.0, 100.0) as u8;
    }
    numeric_feature(features, "claim_amount_to_limit_ratio")
        .map(|ratio| (ratio * 100.0).round().clamp(0.0, 100.0) as u8)
        .unwrap_or(0)
}

fn medical_reasonableness_score(features: &FeatureMap) -> u8 {
    if let Some(match_score) = numeric_feature(features, "diagnosis_procedure_match_score") {
        let mismatch_risk = ((1.0 - match_score) * 100.0).round().clamp(0.0, 100.0);
        let high_cost_risk = numeric_feature(features, "high_cost_item_ratio")
            .map(|ratio| (ratio * 25.0).round().clamp(0.0, 25.0))
            .unwrap_or(0.0);
        return (mismatch_risk + high_cost_risk).round().clamp(0.0, 100.0) as u8;
    }
    let item_count = numeric_feature(features, "claim_item_count").unwrap_or(0.0);
    (item_count * 12.0).round().clamp(0.0, 60.0) as u8
}

fn provider_network_score(features: &FeatureMap) -> u8 {
    if let Some(score) = numeric_feature(features, "provider_profile_score") {
        return score.round().clamp(0.0, 100.0) as u8;
    }
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
    use fwa_anomaly::{AnomalyExplanation, AnomalyScore};
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
            "claim_amount_peer_percentile".into(),
            FeatureValue {
                name: "claim_amount_peer_percentile".into(),
                version: 1,
                value: serde_json::json!(95),
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
            "high_cost_item_ratio".into(),
            FeatureValue {
                name: "high_cost_item_ratio".into(),
                version: 1,
                value: serde_json::json!(1.0),
                evidence_refs: vec![],
            },
        );
        features.insert(
            "diagnosis_procedure_match_score".into(),
            FeatureValue {
                name: "diagnosis_procedure_match_score".into(),
                version: 1,
                value: serde_json::json!(0.35),
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
        features.insert(
            "provider_profile_score".into(),
            FeatureValue {
                name: "provider_profile_score".into(),
                version: 1,
                value: serde_json::json!(80),
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
        let anomaly = AnomalyScore {
            score: 72,
            anomaly_type: "rare_claim_pattern".into(),
            explanations: vec![AnomalyExplanation {
                signal: "high_peer_percentile_with_medical_mismatch".into(),
                contribution: 0.72,
                reason: "金额分位和医疗匹配信号组合异常".into(),
            }],
        };

        let decision = aggregate(&features, &rules, &model, &anomaly);
        assert_eq!(decision.peer_deviation_score, 95);
        assert_eq!(decision.rule_score, 80);
        assert_eq!(decision.anomaly_score, 72);
        assert_eq!(decision.ml_score, 90);
        assert_eq!(decision.medical_reasonableness_score, 90);
        assert_eq!(decision.provider_network_score, 80);
        assert_eq!(decision.similar_case_score, 0);
        assert_eq!(decision.risk_score.value(), 85);
        assert_eq!(decision.rag, RiskLevel::Red);
        assert_eq!(decision.risk_level, "Critical");
        assert_eq!(decision.confidence_score, 100);
        assert_eq!(decision.confidence, "High");
        assert!(decision.routing_reason.contains("医务复核"));
        assert_eq!(
            decision.recommended_action,
            RecommendedAction::EscalateInvestigation
        );
    }
}

use fwa_anomaly::AnomalyScore;
use fwa_core::{RecommendedAction, RiskLevel, RiskScore};
use fwa_features::FeatureMap;
use fwa_ml_runtime::ModelScore;
use fwa_rules::RuleMatch;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectionLayerScore {
    pub layer_id: String,
    pub name: String,
    pub score: u8,
    pub status: String,
    pub reason: String,
}

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
    pub layers: Vec<DetectionLayerScore>,
    pub top_reasons: Vec<String>,
}

pub fn aggregate(
    features: &FeatureMap,
    rule_matches: &[RuleMatch],
    model_score: &ModelScore,
    anomaly_score: &AnomalyScore,
    similar_case_score: u8,
) -> ScoringDecision {
    aggregate_for_review_mode(
        features,
        rule_matches,
        model_score,
        anomaly_score,
        similar_case_score,
        "pre_payment",
    )
}

pub fn aggregate_for_review_mode(
    features: &FeatureMap,
    rule_matches: &[RuleMatch],
    model_score: &ModelScore,
    anomaly_score: &AnomalyScore,
    similar_case_score: u8,
    review_mode: &str,
) -> ScoringDecision {
    let peer_deviation_score = amount_ratio_score(features);
    let rule_score = rule_matches
        .iter()
        .map(|rule_match| rule_match.score_contribution as u32)
        .sum::<u32>()
        .min(100) as u8;
    let medical_reasonableness_score = medical_reasonableness_score(features);
    let provider_network_score = provider_network_score(features);
    let final_score_value = weighted_final_score(&[
        (peer_deviation_score, 0.15),
        (rule_score, 0.20),
        (anomaly_score.score, 0.15),
        (model_score.score, 0.25),
        (medical_reasonableness_score, 0.10),
        (provider_network_score, 0.10),
        (similar_case_score, 0.05),
    ]);
    let risk_score = RiskScore::new(final_score_value).expect("clamped score is valid");
    let rag = RiskLevel::from_score(risk_score);
    let risk_level = risk_level(risk_score.value()).to_string();
    let confidence_score = confidence_score(rule_score, anomaly_score.score, model_score.score);
    let confidence = confidence_level(confidence_score).to_string();
    let recommended_action = recommended_action(
        &risk_level,
        confidence_score,
        review_mode,
        provider_network_score,
    );
    let routing_reason = routing_reason(
        &risk_level,
        confidence_score,
        review_mode,
        provider_network_score,
    )
    .to_string();
    let layers = vec![
        layer(
            "L1_PEER_BENCHMARK",
            "Peer Benchmark",
            peer_deviation_score,
            "active",
            "统计偏离检测：同类金额、频次或费用结构偏离",
        ),
        layer(
            "L2_RULE_DETECTION",
            "Rule Detection",
            rule_score,
            "active",
            "规则命中检测：确定性业务规则和规则版本贡献",
        ),
        layer(
            "L3_UNSUPERVISED_ANOMALY",
            "Unsupervised Anomaly",
            anomaly_score.score,
            "baseline",
            "无监督异常检测：当前使用可解释启发式异常信号",
        ),
        layer(
            "L4_SUPERVISED_ML",
            "Supervised ML",
            model_score.score,
            "active",
            "监督式 ML 分类：模型运行时返回的风险分",
        ),
        layer(
            "L5_MEDICAL_REASONABLENESS",
            "Medical Reasonableness",
            medical_reasonableness_score,
            "baseline",
            "医疗合理性检测：诊断、项目和证据支持度",
        ),
        layer(
            "L6_PROVIDER_GRAPH_RISK",
            "Provider / Graph Risk",
            provider_network_score,
            "baseline",
            "Provider / 图谱风险：Provider 画像和关系风险基线",
        ),
        layer(
            "L7_RISK_FUSION_ROUTING",
            "Risk Fusion & Routing",
            risk_score.value(),
            "active",
            &routing_reason,
        ),
    ];
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
        top_reasons.push("Provider 画像或关系网络风险偏高".into());
    }
    if similar_case_score >= 70 {
        top_reasons.push("命中相似历史 FWA 案例信号".into());
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
        layers,
        top_reasons,
    }
}

fn layer(layer_id: &str, name: &str, score: u8, status: &str, reason: &str) -> DetectionLayerScore {
    DetectionLayerScore {
        layer_id: layer_id.into(),
        name: name.into(),
        score,
        status: status.into(),
        reason: reason.into(),
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

fn recommended_action(
    risk_level: &str,
    confidence_score: u8,
    review_mode: &str,
    provider_network_score: u8,
) -> RecommendedAction {
    if confidence_score < 60 {
        return RecommendedAction::RequestEvidence;
    }
    if review_mode == "post_payment" {
        if risk_level == "Critical" {
            return RecommendedAction::RecoveryReview;
        }
        if provider_network_score >= 70 && risk_level == "High" {
            return RecommendedAction::ProviderReview;
        }
        return match risk_level {
            "Low" => RecommendedAction::AutoApprove,
            "Medium" | "High" => RecommendedAction::PostPaymentAudit,
            _ => RecommendedAction::PostPaymentAudit,
        };
    }
    match risk_level {
        "Low" => RecommendedAction::AutoApprove,
        "Medium" => RecommendedAction::QaSample,
        "High" => RecommendedAction::ManualReview,
        "Critical" => RecommendedAction::EscalateInvestigation,
        _ => RecommendedAction::ManualReview,
    }
}

fn routing_reason(
    risk_level: &str,
    confidence_score: u8,
    review_mode: &str,
    provider_network_score: u8,
) -> &'static str {
    if confidence_score < 60 {
        return "置信度偏低，建议补材料或二审";
    }
    if review_mode == "post_payment" {
        if risk_level == "Critical" {
            return "赔后关键风险，建议调查复核并评估追偿";
        }
        if provider_network_score >= 70 && risk_level == "High" {
            return "赔后 Provider / 图谱风险偏高，建议进入 Provider 复核";
        }
        return match risk_level {
            "Low" => "赔后低风险，建议不进入审计队列",
            "Medium" => "赔后中风险，建议进入审计抽样队列",
            "High" => "赔后高风险，建议进入专项审计队列",
            _ => "赔后风险等级未知，建议进入审计队列",
        };
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
    let clinical_gap_risk = numeric_feature(features, "clinical_missing_evidence_count")
        .map(|count| (count * 15.0).round().clamp(0.0, 45.0))
        .unwrap_or(0.0);
    let clinical_review_risk = numeric_feature(features, "clinical_review_required")
        .map(|flag| if flag > 0.0 { 15.0 } else { 0.0 })
        .unwrap_or(0.0);

    if let Some(match_score) = numeric_feature(features, "diagnosis_procedure_match_score") {
        let mismatch_risk = ((1.0 - match_score) * 100.0).round().clamp(0.0, 100.0);
        let high_cost_risk = numeric_feature(features, "high_cost_item_ratio")
            .map(|ratio| (ratio * 25.0).round().clamp(0.0, 25.0))
            .unwrap_or(0.0);
        return (mismatch_risk + high_cost_risk + clinical_gap_risk + clinical_review_risk)
            .round()
            .clamp(0.0, 100.0) as u8;
    }
    let item_count = numeric_feature(features, "claim_item_count").unwrap_or(0.0);
    (item_count * 12.0 + clinical_gap_risk + clinical_review_risk)
        .round()
        .clamp(0.0, 100.0) as u8
}

fn provider_network_score(features: &FeatureMap) -> u8 {
    let profile_score = numeric_feature(features, "provider_profile_score")
        .map(|score| score.round().clamp(0.0, 100.0) as u8)
        .unwrap_or_else(|| {
            match features
                .get("provider_risk_tier")
                .and_then(|feature| feature.value.as_str())
            {
                Some("HIGH") => 80,
                Some("MEDIUM") => 45,
                Some("LOW") => 10,
                _ => 0,
            }
        });
    let graph_score = numeric_feature(features, "provider_graph_risk_score")
        .map(|score| score.round().clamp(0.0, 100.0) as u8)
        .unwrap_or(0);
    profile_score.max(graph_score)
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

    fn feature(value: serde_json::Value) -> FeatureValue {
        FeatureValue {
            name: "test_feature".into(),
            version: 1,
            value,
            evidence_refs: vec![],
        }
    }

    fn model(score: u8) -> ModelScore {
        ModelScore {
            model_key: "baseline".into(),
            model_version: "0.1.0".into(),
            runtime_kind: "heuristic".into(),
            execution_provider: "cpu".into(),
            score,
            label: "TEST".into(),
            explanations: vec![],
            metadata: serde_json::json!({}),
            latency_ms: 0,
        }
    }

    fn anomaly(score: u8) -> AnomalyScore {
        AnomalyScore {
            score,
            anomaly_type: "test_pattern".into(),
            explanations: vec![],
        }
    }

    fn rule(score: u8) -> RuleMatch {
        RuleMatch {
            rule_id: "rule_1".into(),
            rule_version: 1,
            score_contribution: score,
            alert_code: "TEST".into(),
            reason: "test rule".into(),
            recommended_action: RecommendedAction::ManualReview,
        }
    }

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
        features.insert(
            "provider_graph_risk_score".into(),
            FeatureValue {
                name: "provider_graph_risk_score".into(),
                version: 1,
                value: serde_json::json!(92),
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

        let decision = aggregate(&features, &rules, &model, &anomaly, 90);
        assert_eq!(decision.peer_deviation_score, 95);
        assert_eq!(decision.rule_score, 80);
        assert_eq!(decision.anomaly_score, 72);
        assert_eq!(decision.ml_score, 90);
        assert_eq!(decision.medical_reasonableness_score, 90);
        assert_eq!(decision.provider_network_score, 92);
        assert_eq!(decision.similar_case_score, 90);
        let layer_ids = decision
            .layers
            .iter()
            .map(|layer| layer.layer_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            layer_ids,
            vec![
                "L1_PEER_BENCHMARK",
                "L2_RULE_DETECTION",
                "L3_UNSUPERVISED_ANOMALY",
                "L4_SUPERVISED_ML",
                "L5_MEDICAL_REASONABLENESS",
                "L6_PROVIDER_GRAPH_RISK",
                "L7_RISK_FUSION_ROUTING",
            ]
        );
        assert_eq!(decision.layers[0].score, 95);
        assert_eq!(decision.layers[6].score, decision.risk_score.value());
        assert_eq!(decision.layers[6].status, "active");
        assert_eq!(decision.risk_score.value(), 86);
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

    #[test]
    fn routes_medium_risk_to_qa_sample() {
        let mut features = BTreeMap::new();
        features.insert(
            "claim_amount_peer_percentile".into(),
            feature(serde_json::json!(95)),
        );
        let rules = vec![rule(80)];

        let decision = aggregate(&features, &rules, &model(80), &anomaly(0), 0);

        assert_eq!(decision.risk_level, "Medium");
        assert_eq!(decision.confidence, "High");
        assert_eq!(decision.recommended_action, RecommendedAction::QaSample);
        assert!(decision.routing_reason.contains("QA 抽样"));
    }

    #[test]
    fn routes_post_payment_medium_risk_to_audit_queue() {
        let mut features = BTreeMap::new();
        features.insert(
            "claim_amount_peer_percentile".into(),
            feature(serde_json::json!(95)),
        );
        let rules = vec![rule(80)];

        let decision = aggregate_for_review_mode(
            &features,
            &rules,
            &model(80),
            &anomaly(0),
            0,
            "post_payment",
        );

        assert_eq!(decision.risk_level, "Medium");
        assert_eq!(
            decision.recommended_action,
            RecommendedAction::PostPaymentAudit
        );
        assert!(decision.routing_reason.contains("赔后"));
    }

    #[test]
    fn routes_post_payment_provider_risk_to_provider_review() {
        let mut features = BTreeMap::new();
        features.insert(
            "claim_amount_peer_percentile".into(),
            feature(serde_json::json!(98)),
        );
        features.insert(
            "provider_graph_risk_score".into(),
            feature(serde_json::json!(90)),
        );
        let rules = vec![rule(90)];

        let decision = aggregate_for_review_mode(
            &features,
            &rules,
            &model(95),
            &anomaly(95),
            80,
            "post_payment",
        );

        assert_eq!(decision.risk_level, "High");
        assert_eq!(
            decision.recommended_action,
            RecommendedAction::ProviderReview
        );
        assert!(decision.routing_reason.contains("Provider"));
    }

    #[test]
    fn routes_low_confidence_to_evidence_request() {
        let features = BTreeMap::new();
        let decision = aggregate(&features, &[], &model(20), &anomaly(0), 0);

        assert_eq!(decision.confidence, "Low");
        assert_eq!(
            decision.recommended_action,
            RecommendedAction::RequestEvidence
        );
        assert!(decision.routing_reason.contains("补材料"));
    }

    #[test]
    fn clinical_evidence_gaps_raise_medical_reasonableness_score() {
        let mut features = BTreeMap::new();
        features.insert(
            "diagnosis_procedure_match_score".into(),
            feature(serde_json::json!(0.80)),
        );
        features.insert(
            "high_cost_item_ratio".into(),
            feature(serde_json::json!(0.0)),
        );
        features.insert(
            "clinical_missing_evidence_count".into(),
            feature(serde_json::json!(2)),
        );
        features.insert(
            "clinical_review_required".into(),
            feature(serde_json::json!(1)),
        );

        let decision = aggregate(&features, &[], &model(20), &anomaly(0), 0);

        assert_eq!(decision.medical_reasonableness_score, 65);
        assert_eq!(decision.layers[4].layer_id, "L5_MEDICAL_REASONABLENESS");
        assert_eq!(decision.layers[4].score, 65);
        assert!(decision
            .top_reasons
            .contains(&"账单项目数量或结构需要医疗合理性复核".to_string()));
    }
}

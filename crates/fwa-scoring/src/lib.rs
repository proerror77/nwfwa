use fwa_anomaly::AnomalyScore;
use fwa_core::{
    DecisionAuthority, DecisionConfidence, DecisionOutcome, RecommendedAction, RiskLevel, RiskScore,
};
use fwa_features::FeatureMap;
use fwa_ml_runtime::ModelScore;
use fwa_rules::RuleMatch;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod decision;
mod evidence;

use decision::decision_context;
use evidence::{
    anomaly_layer_evidence_refs, feature_evidence_refs, layer, model_layer_evidence_refs,
    rule_layer_evidence_refs,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DetectionLayerScore {
    pub layer_id: String,
    pub name: String,
    pub score: u8,
    pub status: String,
    pub reason: String,
    pub evidence_refs: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoutingPolicy {
    pub policy_id: String,
    pub version: u32,
    pub review_mode: String,
    pub risk_thresholds: RiskThresholds,
    pub confidence_thresholds: ConfidenceThresholds,
    pub provider_review_threshold: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RiskThresholds {
    pub low_max: u8,
    pub medium_min: u8,
    pub high_min: u8,
    pub critical_min: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfidenceThresholds {
    pub low_confidence_below: u8,
    pub high_confidence_min: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScoringDecision {
    pub risk_score: RiskScore,
    pub rag: RiskLevel,
    pub risk_level: String,
    pub recommended_action: RecommendedAction,
    pub decision_outcome: DecisionOutcome,
    pub decision_authority: DecisionAuthority,
    pub decision_confidence: DecisionConfidence,
    pub appeal_or_review_required: bool,
    pub reason_code: String,
    pub confidence_score: u8,
    pub confidence: String,
    pub routing_reason: String,
    pub routing_policy: RoutingPolicy,
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
    aggregate_with_routing_policy(
        features,
        rule_matches,
        model_score,
        anomaly_score,
        similar_case_score,
        default_routing_policy(review_mode),
    )
}

pub fn aggregate_with_routing_policy(
    features: &FeatureMap,
    rule_matches: &[RuleMatch],
    model_score: &ModelScore,
    anomaly_score: &AnomalyScore,
    similar_case_score: u8,
    routing_policy: RoutingPolicy,
) -> ScoringDecision {
    let peer_deviation_score = amount_ratio_score(features);
    let peer_benchmark_score_for_weight = peer_benchmark_weighted_score(features);
    let rule_score = rule_matches
        .iter()
        .map(|rule_match| rule_match.score_contribution as u32)
        .sum::<u32>()
        .min(100) as u8;
    let medical_reasonableness_score = medical_reasonableness_score(features);
    let provider_network_score = provider_network_score(features);
    let final_score_value = weighted_final_score(&[
        (peer_benchmark_score_for_weight, 0.15),
        (Some(rule_score), 0.20),
        (Some(anomaly_score.score), 0.15),
        (Some(model_score.score), 0.25),
        (Some(medical_reasonableness_score), 0.10),
        (Some(provider_network_score), 0.10),
        (Some(similar_case_score), 0.05),
    ]);
    let risk_score = RiskScore::saturating(final_score_value);
    let risk_level = risk_level_for_policy(risk_score.value(), &routing_policy).to_string();
    let rag = rag_for_policy(risk_score, &routing_policy);
    let confidence_score = confidence_score(rule_score, anomaly_score.score, model_score.score);
    let confidence = confidence_level_for_policy(confidence_score, &routing_policy).to_string();
    let recommended_action = recommended_action(
        &risk_level,
        confidence_score,
        &routing_policy,
        provider_network_score,
    );
    let routing_reason = routing_reason(
        &risk_level,
        confidence_score,
        &routing_policy,
        provider_network_score,
    )
    .to_string();
    let decision_context = decision_context(
        rule_matches,
        recommended_action,
        &confidence,
        &routing_policy,
    );
    let layers = vec![
        layer(
            "L1_PEER_BENCHMARK",
            "Peer Benchmark",
            peer_deviation_score,
            peer_benchmark_layer_status(peer_benchmark_score_for_weight),
            peer_benchmark_layer_reason(peer_benchmark_score_for_weight),
            feature_evidence_refs(
                features,
                &[
                    "claim_amount_peer_percentile",
                    "claim_amount_to_limit_ratio",
                    "claim_item_count",
                ],
            ),
        ),
        layer(
            "L2_RULE_DETECTION",
            "Rule Detection",
            rule_score,
            "active",
            "规则命中检测：确定性业务规则和规则版本贡献",
            rule_layer_evidence_refs(rule_matches),
        ),
        layer(
            "L3_UNSUPERVISED_ANOMALY",
            "Unsupervised Anomaly",
            anomaly_score.score,
            "baseline",
            "无监督异常检测：当前使用可解释启发式异常信号",
            anomaly_layer_evidence_refs(features, anomaly_score),
        ),
        layer(
            "L4_SUPERVISED_ML",
            "Supervised ML",
            model_score.score,
            "active",
            "监督式 ML 分类：模型运行时返回的风险分",
            model_layer_evidence_refs(features, model_score),
        ),
        layer(
            "L5_MEDICAL_REASONABLENESS",
            "Medical Reasonableness",
            medical_reasonableness_score,
            "baseline",
            "医疗合理性检测：诊断、项目和证据支持度",
            feature_evidence_refs(
                features,
                &[
                    "diagnosis_procedure_match_score",
                    "high_cost_item_ratio",
                    "clinical_missing_evidence_count",
                    "clinical_item_finding_count",
                    "clinical_review_required",
                ],
            ),
        ),
        layer(
            "L6_PROVIDER_GRAPH_RISK",
            "Provider / Graph Risk",
            provider_network_score,
            "baseline",
            "Provider / 图谱风险：Provider 画像和关系风险基线",
            feature_evidence_refs(
                features,
                &[
                    "provider_risk_tier",
                    "provider_profile_score",
                    "provider_peer_amount_percentile",
                    "provider_graph_risk_score",
                    "provider_high_risk_neighbor_signal",
                ],
            ),
        ),
        layer(
            "L7_RISK_FUSION_ROUTING",
            "Risk Fusion & Routing",
            risk_score.value(),
            "active",
            &routing_reason,
            vec![Value::String(format!(
                "routing_policies:{}:v{}:{}",
                routing_policy.policy_id, routing_policy.version, routing_policy.review_mode
            ))],
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
        decision_outcome: decision_context.outcome,
        decision_authority: decision_context.authority,
        decision_confidence: decision_context.confidence,
        appeal_or_review_required: decision_context.appeal_or_review_required,
        reason_code: decision_context.reason_code,
        confidence_score,
        confidence,
        routing_reason,
        routing_policy,
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

fn risk_level_for_policy(score: u8, policy: &RoutingPolicy) -> &'static str {
    if score >= policy.risk_thresholds.critical_min {
        "Critical"
    } else if score >= policy.risk_thresholds.high_min {
        "High"
    } else if score >= policy.risk_thresholds.medium_min {
        "Medium"
    } else {
        "Low"
    }
}

fn rag_for_policy(score: RiskScore, policy: &RoutingPolicy) -> RiskLevel {
    RiskLevel::from_thresholds(
        score,
        policy.risk_thresholds.medium_min,
        policy.risk_thresholds.high_min,
    )
}

fn confidence_score(rule_score: u8, anomaly_score: u8, ml_score: u8) -> u8 {
    let supporting_layers = [rule_score, anomaly_score, ml_score]
        .into_iter()
        .filter(|score| *score >= 60)
        .count() as u8;
    (55 + supporting_layers * 15).min(100)
}

fn confidence_level_for_policy(score: u8, policy: &RoutingPolicy) -> &'static str {
    if score < policy.confidence_thresholds.low_confidence_below {
        "Low"
    } else if score >= policy.confidence_thresholds.high_confidence_min {
        "High"
    } else {
        "Medium"
    }
}

pub fn default_routing_policy(review_mode: &str) -> RoutingPolicy {
    RoutingPolicy {
        policy_id: "fwa_risk_fusion_routing".into(),
        version: 1,
        review_mode: review_mode.into(),
        risk_thresholds: RiskThresholds {
            low_max: 39,
            medium_min: 40,
            high_min: 70,
            critical_min: 85,
        },
        confidence_thresholds: ConfidenceThresholds {
            low_confidence_below: 60,
            high_confidence_min: 80,
        },
        provider_review_threshold: 70,
    }
}

fn recommended_action(
    risk_level: &str,
    confidence_score: u8,
    policy: &RoutingPolicy,
    provider_network_score: u8,
) -> RecommendedAction {
    if confidence_score < policy.confidence_thresholds.low_confidence_below {
        return RecommendedAction::RequestEvidence;
    }
    if policy.review_mode == "post_payment" {
        if risk_level == "Critical" {
            return RecommendedAction::RecoveryReview;
        }
        if provider_network_score >= policy.provider_review_threshold && risk_level == "High" {
            return RecommendedAction::ProviderReview;
        }
        return match risk_level {
            "Low" => RecommendedAction::StandardProcessing,
            "Medium" | "High" => RecommendedAction::PostPaymentAudit,
            _ => RecommendedAction::PostPaymentAudit,
        };
    }
    match risk_level {
        "Low" => RecommendedAction::StandardProcessing,
        "Medium" => RecommendedAction::QaSample,
        "High" => RecommendedAction::ManualReview,
        "Critical" => RecommendedAction::EscalateInvestigation,
        _ => RecommendedAction::ManualReview,
    }
}

fn routing_reason(
    risk_level: &str,
    confidence_score: u8,
    policy: &RoutingPolicy,
    provider_network_score: u8,
) -> &'static str {
    if confidence_score < policy.confidence_thresholds.low_confidence_below {
        return "置信度偏低，建议补材料或二审";
    }
    if policy.review_mode == "post_payment" {
        if risk_level == "Critical" {
            return "赔后关键风险，建议调查复核并评估追偿";
        }
        if provider_network_score >= policy.provider_review_threshold && risk_level == "High" {
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

fn weighted_final_score(available_scores: &[(Option<u8>, f64)]) -> u8 {
    let total_weight = available_scores
        .iter()
        .filter(|(score, _)| score.is_some())
        .map(|(_, weight)| *weight)
        .sum::<f64>();
    if total_weight == 0.0 {
        return 0;
    }
    let weighted = available_scores
        .iter()
        .filter_map(|(score, weight)| score.map(|score| score as f64 * weight))
        .sum::<f64>();
    (weighted / total_weight).round().clamp(0.0, 100.0) as u8
}

fn peer_benchmark_weighted_score(features: &FeatureMap) -> Option<u8> {
    numeric_feature(features, "claim_amount_peer_percentile")
        .map(|percentile| percentile.round().clamp(0.0, 100.0) as u8)
}

fn peer_benchmark_layer_status(weighted_score: Option<u8>) -> &'static str {
    if weighted_score.is_some() {
        "active"
    } else {
        "proxy_excluded"
    }
}

fn peer_benchmark_layer_reason(weighted_score: Option<u8>) -> &'static str {
    if weighted_score.is_some() {
        "统计偏离检测：同类金额、频次或费用结构偏离"
    } else {
        "Peer benchmark 缺少真实同侪分布，仅展示金额/保额代理值，已从最终加权分排除"
    }
}

fn amount_ratio_score(features: &FeatureMap) -> u8 {
    if let Some(percentile) = numeric_feature(features, "claim_amount_peer_percentile") {
        return percentile.round().clamp(0.0, 100.0) as u8;
    }
    // PROXY BASELINE: this fallback is an amount-to-limit ratio, not a real
    // peer percentile. Production L1 scoring must use peer distribution data or
    // downweight/exclude this layer when peer context is missing.
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
mod tests;

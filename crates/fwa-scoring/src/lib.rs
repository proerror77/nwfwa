use fwa_anomaly::AnomalyScore;
use fwa_core::{
    DecisionAuthority, DecisionConfidence, DecisionOutcome, RecommendedAction, RiskLevel,
    RiskScore, RuleActionClass,
};
use fwa_features::FeatureMap;
use fwa_ml_runtime::ModelScore;
use fwa_rules::RuleMatch;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    let risk_level = risk_level_for_policy(risk_score.value(), &routing_policy).to_string();
    let rag = rag_for_policy(&risk_level);
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
            "active",
            "统计偏离检测：同类金额、频次或费用结构偏离",
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

fn layer(
    layer_id: &str,
    name: &str,
    score: u8,
    status: &str,
    reason: &str,
    evidence_refs: Vec<Value>,
) -> DetectionLayerScore {
    DetectionLayerScore {
        layer_id: layer_id.into(),
        name: name.into(),
        score,
        status: status.into(),
        reason: reason.into(),
        evidence_refs: unique_evidence_refs(evidence_refs),
    }
}

fn feature_evidence_refs(features: &FeatureMap, names: &[&str]) -> Vec<Value> {
    let mut evidence_refs = Vec::new();
    for name in names {
        if let Some(feature) = features.get(*name) {
            evidence_refs.push(Value::String(format!(
                "feature_values:{}:v{}",
                feature.name, feature.version
            )));
            evidence_refs.extend(
                feature
                    .evidence_refs
                    .iter()
                    .filter_map(|evidence| serde_json::to_value(evidence).ok()),
            );
        }
    }
    unique_evidence_refs(evidence_refs)
}

fn rule_layer_evidence_refs(rule_matches: &[RuleMatch]) -> Vec<Value> {
    if rule_matches.is_empty() {
        return vec![Value::String("rules:evaluated:no_match".into())];
    }
    unique_evidence_refs(
        rule_matches
            .iter()
            .flat_map(|rule_match| rule_match.evidence_refs.clone())
            .collect(),
    )
}

fn anomaly_layer_evidence_refs(
    features: &FeatureMap,
    anomaly_score: &fwa_anomaly::AnomalyScore,
) -> Vec<Value> {
    let mut evidence_refs = vec![Value::String(format!(
        "anomaly_scores:{}",
        anomaly_score.anomaly_type
    ))];
    for explanation in &anomaly_score.explanations {
        evidence_refs.extend(feature_evidence_refs(
            features,
            &[explanation.signal.as_str()],
        ));
    }
    unique_evidence_refs(evidence_refs)
}

fn model_layer_evidence_refs(features: &FeatureMap, model_score: &ModelScore) -> Vec<Value> {
    let mut evidence_refs = vec![Value::String(format!(
        "model_versions:{}:{}",
        model_score.model_key, model_score.model_version
    ))];
    for explanation in &model_score.explanations {
        evidence_refs.extend(feature_evidence_refs(
            features,
            &[explanation.feature.as_str()],
        ));
    }
    unique_evidence_refs(evidence_refs)
}

fn unique_evidence_refs(evidence_refs: Vec<Value>) -> Vec<Value> {
    let mut unique = Vec::new();
    for evidence_ref in evidence_refs {
        if !unique.iter().any(|existing| existing == &evidence_ref) {
            unique.push(evidence_ref);
        }
    }
    unique
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

fn rag_for_policy(risk_level: &str) -> RiskLevel {
    match risk_level {
        "Low" => RiskLevel::Green,
        "Medium" => RiskLevel::Amber,
        "High" | "Critical" => RiskLevel::Red,
        _ => RiskLevel::Red,
    }
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

struct DecisionContext {
    outcome: DecisionOutcome,
    authority: DecisionAuthority,
    confidence: DecisionConfidence,
    appeal_or_review_required: bool,
    reason_code: String,
}

fn decision_context(
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

    fn feature_with_source(name: &str, value: serde_json::Value, field: &str) -> FeatureValue {
        FeatureValue {
            name: name.into(),
            version: 1,
            value,
            evidence_refs: vec![fwa_features::EvidenceRef {
                entity_type: "claim".into(),
                entity_id: "CLM-LAYER-EVIDENCE".into(),
                field: field.into(),
            }],
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
            action_class: RuleActionClass::ManualReview,
            required_evidence: vec![],
            adjudication_policy: None,
            evidence_refs: vec![],
        }
    }

    fn adjudication_policy() -> fwa_rules::AdjudicationPolicy {
        fwa_rules::AdjudicationPolicy {
            customer_approval_ref: "customer_rule_list:demo:v1".into(),
            appeal_or_override_route: "appeals:manual-review:v1".into(),
            effective_date: "2026-01-01".into(),
            rollback_plan_ref: "rollback:rules:v1".into(),
            production_threshold_ref: "thresholds:prepay:v1".into(),
            routing_impact_ref: "routing-impact:shadow:v1".into(),
        }
    }

    fn adjudication_evidence() -> Vec<fwa_rules::RequiredEvidence> {
        vec![fwa_rules::RequiredEvidence {
            evidence_type: "policy_eligibility".into(),
            evidence_request_type: None,
            blocking: true,
            policy_authority_ref: Some("policy:eligibility:v1".into()),
            exception_check: Some("no_approved_exception".into()),
        }]
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
            action_class: RuleActionClass::ManualReview,
            required_evidence: vec![],
            adjudication_policy: None,
            evidence_refs: vec![],
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
    fn every_detection_layer_carries_evidence_refs() {
        let mut features = BTreeMap::new();
        features.insert(
            "claim_amount_peer_percentile".into(),
            feature_with_source(
                "claim_amount_peer_percentile",
                serde_json::json!(95),
                "claim_amount",
            ),
        );
        features.insert(
            "diagnosis_procedure_match_score".into(),
            feature_with_source(
                "diagnosis_procedure_match_score",
                serde_json::json!(0.35),
                "diagnosis_code",
            ),
        );
        features.insert(
            "provider_graph_risk_score".into(),
            feature_with_source(
                "provider_graph_risk_score",
                serde_json::json!(90),
                "provider_graph_risk_score",
            ),
        );
        let rules = vec![RuleMatch {
            rule_id: "rule_layer_evidence".into(),
            rule_version: 2,
            score_contribution: 40,
            alert_code: "LAYER_EVIDENCE".into(),
            reason: "layer evidence test".into(),
            recommended_action: RecommendedAction::ManualReview,
            action_class: RuleActionClass::ManualReview,
            required_evidence: vec![],
            adjudication_policy: None,
            evidence_refs: vec![serde_json::json!("rules:rule_layer_evidence:v2")],
        }];
        let model = ModelScore {
            model_key: "baseline_fwa".into(),
            model_version: "0.1.0".into(),
            runtime_kind: "heuristic".into(),
            execution_provider: "cpu".into(),
            score: 70,
            label: "HIGH_RISK".into(),
            explanations: vec![ModelExplanation {
                feature: "claim_amount_peer_percentile".into(),
                direction: "increases_risk".into(),
                contribution: 0.40,
                reason: "peer percentile contributes to model score".into(),
            }],
            metadata: serde_json::json!({}),
            latency_ms: 0,
        };
        let anomaly = AnomalyScore {
            score: 60,
            anomaly_type: "rare_claim_pattern".into(),
            explanations: vec![AnomalyExplanation {
                signal: "diagnosis_procedure_match_score".into(),
                contribution: 0.25,
                reason: "medical mismatch signal contributes to anomaly score".into(),
            }],
        };

        let decision = aggregate(&features, &rules, &model, &anomaly, 80);
        let layers = serde_json::to_value(&decision.layers).unwrap();
        let layers = layers.as_array().unwrap();

        assert!(layers.iter().all(|layer| {
            layer["evidence_refs"]
                .as_array()
                .is_some_and(|refs| !refs.is_empty())
        }));
        assert!(layers[0]["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "feature_values:claim_amount_peer_percentile:v1"
            )));
        assert!(layers[1]["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("rules:rule_layer_evidence:v2")));
        assert!(layers[3]["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("model_versions:baseline_fwa:0.1.0")));
        assert!(layers[6]["evidence_refs"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!(
                "routing_policies:fwa_risk_fusion_routing:v1:pre_payment"
            )));
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
    fn routes_low_risk_to_standard_processing() {
        let policy = RoutingPolicy {
            policy_id: "low_risk_standard_processing".into(),
            version: 1,
            review_mode: "pre_payment".into(),
            risk_thresholds: RiskThresholds {
                low_max: 100,
                medium_min: 101,
                high_min: 150,
                critical_min: 200,
            },
            confidence_thresholds: ConfidenceThresholds {
                low_confidence_below: 50,
                high_confidence_min: 80,
            },
            provider_review_threshold: 70,
        };

        let decision = aggregate_with_routing_policy(
            &BTreeMap::new(),
            &[],
            &model(80),
            &anomaly(80),
            0,
            policy,
        );

        assert_eq!(decision.risk_level, "Low");
        assert_eq!(decision.confidence, "High");
        assert_eq!(
            format!("{:?}", decision.recommended_action),
            "StandardProcessing"
        );
        assert_eq!(decision.decision_outcome, DecisionOutcome::StraightThrough);
        assert_eq!(
            decision.decision_authority,
            DecisionAuthority::RiskRoutingPolicy
        );
        assert_eq!(decision.decision_confidence, DecisionConfidence::High);
        assert!(!decision.appeal_or_review_required);
    }

    #[test]
    fn hard_deny_rule_is_the_only_auto_deny_path() {
        let mut hard_deny_rule = rule(10);
        hard_deny_rule.alert_code = "MALE_ONLY_DRUG_FOR_FEMALE_MEMBER".into();
        hard_deny_rule.action_class = RuleActionClass::HardDeny;
        hard_deny_rule.recommended_action = RecommendedAction::ManualReview;
        hard_deny_rule.required_evidence = adjudication_evidence();
        hard_deny_rule.adjudication_policy = Some(adjudication_policy());

        let decision = aggregate(
            &BTreeMap::new(),
            &[hard_deny_rule],
            &model(10),
            &anomaly(0),
            0,
        );

        assert_eq!(decision.decision_outcome, DecisionOutcome::AutoDeny);
        assert_eq!(
            decision.decision_authority,
            DecisionAuthority::CustomerPolicyRule
        );
        assert_eq!(
            decision.decision_confidence,
            DecisionConfidence::Deterministic
        );
        assert!(decision.appeal_or_review_required);
        assert_eq!(decision.reason_code, "MALE_ONLY_DRUG_FOR_FEMALE_MEMBER");
    }

    #[test]
    fn hard_deny_without_customer_adjudication_policy_falls_back_to_manual_review() {
        let mut hard_deny_rule = rule(10);
        hard_deny_rule.alert_code = "UNAPPROVED_HARD_DENY".into();
        hard_deny_rule.action_class = RuleActionClass::HardDeny;
        hard_deny_rule.required_evidence = adjudication_evidence();

        let decision = aggregate(
            &BTreeMap::new(),
            &[hard_deny_rule],
            &model(10),
            &anomaly(0),
            0,
        );

        assert_eq!(decision.decision_outcome, DecisionOutcome::ManualReview);
        assert_eq!(
            decision.decision_authority,
            DecisionAuthority::HumanReviewer
        );
        assert_ne!(decision.decision_outcome, DecisionOutcome::AutoDeny);
        assert!(decision.appeal_or_review_required);
    }

    #[test]
    fn hard_deny_without_policy_authority_and_exception_check_falls_back_to_manual_review() {
        let mut hard_deny_rule = rule(10);
        hard_deny_rule.alert_code = "MISSING_AUTHORITY".into();
        hard_deny_rule.action_class = RuleActionClass::HardDeny;
        hard_deny_rule.adjudication_policy = Some(adjudication_policy());

        let decision = aggregate(
            &BTreeMap::new(),
            &[hard_deny_rule],
            &model(10),
            &anomaly(0),
            0,
        );

        assert_eq!(decision.decision_outcome, DecisionOutcome::ManualReview);
        assert_eq!(
            decision.decision_authority,
            DecisionAuthority::HumanReviewer
        );
        assert!(decision.appeal_or_review_required);
    }

    #[test]
    fn straight_through_without_customer_adjudication_policy_does_not_bypass_review() {
        let mut straight_through_rule = rule(0);
        straight_through_rule.alert_code = "UNAPPROVED_STP".into();
        straight_through_rule.action_class = RuleActionClass::StraightThrough;
        straight_through_rule.recommended_action = RecommendedAction::StandardProcessing;

        let decision = aggregate(
            &BTreeMap::new(),
            &[straight_through_rule],
            &model(10),
            &anomaly(0),
            0,
        );

        assert_eq!(decision.decision_outcome, DecisionOutcome::ManualReview);
        assert_eq!(
            decision.decision_authority,
            DecisionAuthority::HumanReviewer
        );
        assert!(decision.appeal_or_review_required);
    }

    #[test]
    fn approved_straight_through_rule_can_pass_without_review() {
        let mut straight_through_rule = rule(0);
        straight_through_rule.alert_code = "APPROVED_STP".into();
        straight_through_rule.action_class = RuleActionClass::StraightThrough;
        straight_through_rule.recommended_action = RecommendedAction::StandardProcessing;
        straight_through_rule.required_evidence = adjudication_evidence();
        straight_through_rule.adjudication_policy = Some(adjudication_policy());

        let decision = aggregate(
            &BTreeMap::new(),
            &[straight_through_rule],
            &model(10),
            &anomaly(0),
            0,
        );

        assert_eq!(decision.decision_outcome, DecisionOutcome::StraightThrough);
        assert_eq!(
            decision.decision_authority,
            DecisionAuthority::CustomerPolicyRule
        );
        assert!(!decision.appeal_or_review_required);
    }

    #[test]
    fn hard_deny_takes_precedence_over_approved_straight_through_rule() {
        let mut hard_deny_rule = rule(10);
        hard_deny_rule.alert_code = "APPROVED_HARD_DENY".into();
        hard_deny_rule.action_class = RuleActionClass::HardDeny;
        hard_deny_rule.required_evidence = adjudication_evidence();
        hard_deny_rule.adjudication_policy = Some(adjudication_policy());

        let mut straight_through_rule = rule(0);
        straight_through_rule.alert_code = "APPROVED_STP".into();
        straight_through_rule.action_class = RuleActionClass::StraightThrough;
        straight_through_rule.recommended_action = RecommendedAction::StandardProcessing;
        straight_through_rule.required_evidence = adjudication_evidence();
        straight_through_rule.adjudication_policy = Some(adjudication_policy());

        let decision = aggregate(
            &BTreeMap::new(),
            &[straight_through_rule, hard_deny_rule],
            &model(10),
            &anomaly(0),
            0,
        );

        assert_eq!(decision.decision_outcome, DecisionOutcome::AutoDeny);
        assert_eq!(decision.reason_code, "APPROVED_HARD_DENY");
        assert!(decision.appeal_or_review_required);
    }

    #[test]
    fn pending_evidence_takes_precedence_over_approved_straight_through_rule() {
        let mut pending_evidence_rule = rule(0);
        pending_evidence_rule.alert_code = "MISSING_REQUIRED_EVIDENCE".into();
        pending_evidence_rule.action_class = RuleActionClass::PendingEvidence;
        pending_evidence_rule.recommended_action = RecommendedAction::RequestEvidence;

        let mut straight_through_rule = rule(0);
        straight_through_rule.alert_code = "APPROVED_STP".into();
        straight_through_rule.action_class = RuleActionClass::StraightThrough;
        straight_through_rule.recommended_action = RecommendedAction::StandardProcessing;
        straight_through_rule.required_evidence = adjudication_evidence();
        straight_through_rule.adjudication_policy = Some(adjudication_policy());

        let decision = aggregate(
            &BTreeMap::new(),
            &[straight_through_rule, pending_evidence_rule],
            &model(10),
            &anomaly(0),
            0,
        );

        assert_eq!(decision.decision_outcome, DecisionOutcome::PendingEvidence);
        assert_eq!(decision.reason_code, "MISSING_REQUIRED_EVIDENCE");
        assert!(decision.appeal_or_review_required);
    }

    #[test]
    fn model_and_anomaly_risk_do_not_auto_deny_without_hard_rule() {
        let decision = aggregate(&BTreeMap::new(), &[], &model(100), &anomaly(100), 100);

        assert_ne!(decision.decision_outcome, DecisionOutcome::AutoDeny);
    }

    #[test]
    fn custom_routing_policy_controls_l7_thresholds() {
        let mut features = BTreeMap::new();
        features.insert(
            "claim_amount_peer_percentile".into(),
            feature(serde_json::json!(80)),
        );
        let policy = RoutingPolicy {
            policy_id: "custom_prepay_routing".into(),
            version: 3,
            review_mode: "pre_payment".into(),
            risk_thresholds: RiskThresholds {
                low_max: 24,
                medium_min: 25,
                high_min: 50,
                critical_min: 75,
            },
            confidence_thresholds: ConfidenceThresholds {
                low_confidence_below: 50,
                high_confidence_min: 90,
            },
            provider_review_threshold: 65,
        };

        let decision = aggregate_with_routing_policy(
            &features,
            &[rule(60)],
            &model(80),
            &anomaly(60),
            0,
            policy,
        );

        assert_eq!(decision.risk_score.value(), 53);
        assert_eq!(decision.risk_level, "High");
        assert_eq!(decision.confidence, "High");
        assert_eq!(decision.recommended_action, RecommendedAction::ManualReview);
        assert_eq!(decision.routing_policy.policy_id, "custom_prepay_routing");
        assert_eq!(decision.routing_policy.version, 3);
        assert_eq!(decision.layers[6].reason, "高风险，建议人工审核");
    }

    #[test]
    fn custom_routing_policy_controls_rag_thresholds() {
        let mut features = BTreeMap::new();
        features.insert(
            "claim_amount_peer_percentile".into(),
            feature(serde_json::json!(100)),
        );
        let policy = RoutingPolicy {
            policy_id: "raised_high_threshold".into(),
            version: 2,
            review_mode: "pre_payment".into(),
            risk_thresholds: RiskThresholds {
                low_max: 39,
                medium_min: 40,
                high_min: 90,
                critical_min: 95,
            },
            confidence_thresholds: ConfidenceThresholds {
                low_confidence_below: 50,
                high_confidence_min: 80,
            },
            provider_review_threshold: 90,
        };

        let decision = aggregate_with_routing_policy(
            &features,
            &[rule(100)],
            &model(100),
            &anomaly(100),
            100,
            policy,
        );

        assert_eq!(decision.risk_score.value(), 80);
        assert_eq!(decision.risk_level, "Medium");
        assert_eq!(decision.rag, RiskLevel::Amber);
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

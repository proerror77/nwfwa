use super::*;
use fwa_anomaly::{AnomalyExplanation, AnomalyScore};
use fwa_core::RuleActionClass;
use fwa_features::FeatureValue;
use fwa_ml_runtime::ModelExplanation;
use std::collections::BTreeMap;

fn feature(value: serde_json::Value) -> FeatureValue {
    FeatureValue {
        name: "test_feature".into(),
        version: 1,
        value,
        is_proxy: false,
        data_source: "test_fixture".into(),
        evidence_refs: vec![],
    }
}

fn feature_with_source(name: &str, value: serde_json::Value, field: &str) -> FeatureValue {
    FeatureValue {
        name: name.into(),
        version: 1,
        value,
        is_proxy: false,
        data_source: "test_fixture".into(),
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
            is_proxy: false,
            data_source: "test_fixture".into(),
            evidence_refs: vec![],
        },
    );
    features.insert(
        "claim_amount_peer_percentile".into(),
        FeatureValue {
            name: "claim_amount_peer_percentile".into(),
            version: 1,
            value: serde_json::json!(95),
            is_proxy: false,
            data_source: "test_fixture".into(),
            evidence_refs: vec![],
        },
    );
    features.insert(
        "claim_item_count".into(),
        FeatureValue {
            name: "claim_item_count".into(),
            version: 1,
            value: serde_json::json!(1),
            is_proxy: false,
            data_source: "test_fixture".into(),
            evidence_refs: vec![],
        },
    );
    features.insert(
        "high_cost_item_ratio".into(),
        FeatureValue {
            name: "high_cost_item_ratio".into(),
            version: 1,
            value: serde_json::json!(1.0),
            is_proxy: false,
            data_source: "test_fixture".into(),
            evidence_refs: vec![],
        },
    );
    features.insert(
        "diagnosis_procedure_match_score".into(),
        FeatureValue {
            name: "diagnosis_procedure_match_score".into(),
            version: 1,
            value: serde_json::json!(0.35),
            is_proxy: true,
            data_source: "test_fixture".into(),
            evidence_refs: vec![],
        },
    );
    features.insert(
        "provider_risk_tier".into(),
        FeatureValue {
            name: "provider_risk_tier".into(),
            version: 1,
            value: serde_json::json!("HIGH"),
            is_proxy: false,
            data_source: "test_fixture".into(),
            evidence_refs: vec![],
        },
    );
    features.insert(
        "provider_profile_score".into(),
        FeatureValue {
            name: "provider_profile_score".into(),
            version: 1,
            value: serde_json::json!(80),
            is_proxy: false,
            data_source: "test_fixture".into(),
            evidence_refs: vec![],
        },
    );
    features.insert(
        "provider_graph_risk_score".into(),
        FeatureValue {
            name: "provider_graph_risk_score".into(),
            version: 1,
            value: serde_json::json!(92),
            is_proxy: false,
            data_source: "test_fixture".into(),
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
fn excludes_peer_proxy_from_final_weight_when_peer_percentile_is_missing() {
    let mut features = BTreeMap::new();
    features.insert(
        "claim_amount_to_limit_ratio".into(),
        feature(serde_json::json!(1.0)),
    );

    let decision = aggregate(&features, &[], &model(0), &anomaly(0), 0);

    assert_eq!(decision.peer_deviation_score, 100);
    assert_eq!(decision.layers[0].status, "proxy_excluded");
    assert!(decision.layers[0].reason.contains("已从最终加权分排除"));
    assert_eq!(decision.risk_score.value(), 0);
}

#[test]
fn includes_real_zero_peer_percentile_in_final_weight() {
    let mut features = BTreeMap::new();
    features.insert(
        "claim_amount_peer_percentile".into(),
        feature(serde_json::json!(0)),
    );

    let decision = aggregate(&features, &[], &model(0), &anomaly(0), 100);

    assert_eq!(decision.peer_deviation_score, 0);
    assert_eq!(decision.layers[0].status, "active");
    assert_eq!(decision.risk_score.value(), 5);
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

    let decision =
        aggregate_with_routing_policy(&BTreeMap::new(), &[], &model(80), &anomaly(80), 0, policy);

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

    let decision =
        aggregate_with_routing_policy(&features, &[rule(60)], &model(80), &anomaly(60), 0, policy);

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
fn clinical_and_provider_signals_raise_confidence_without_rule_model_agreement() {
    let mut features = BTreeMap::new();
    features.insert(
        "diagnosis_procedure_match_score".into(),
        feature(serde_json::json!(0.35)),
    );
    features.insert(
        "high_cost_item_ratio".into(),
        feature(serde_json::json!(0.0)),
    );
    features.insert(
        "provider_graph_risk_score".into(),
        feature(serde_json::json!(80)),
    );

    let decision = aggregate(&features, &[], &model(0), &anomaly(0), 0);

    assert_eq!(decision.medical_reasonableness_score, 65);
    assert_eq!(decision.provider_network_score, 80);
    assert_eq!(decision.confidence_score, 85);
    assert_eq!(decision.confidence, "High");
}

#[test]
fn peer_proxy_score_does_not_raise_confidence() {
    let mut features = BTreeMap::new();
    features.insert(
        "claim_amount_to_limit_ratio".into(),
        feature(serde_json::json!(1.0)),
    );

    let decision = aggregate(&features, &[], &model(0), &anomaly(0), 0);

    assert_eq!(decision.peer_deviation_score, 100);
    assert_eq!(decision.layers[0].status, "proxy_excluded");
    assert_eq!(decision.confidence_score, 55);
    assert_eq!(decision.confidence, "Low");
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

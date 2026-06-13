use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fwa_anomaly::{AnomalyExplanation, AnomalyScore};
use fwa_core::{RecommendedAction, RuleActionClass};
use fwa_features::{EvidenceRef, FeatureMap, FeatureValue};
use fwa_ml_runtime::{ModelExplanation, ModelScore};
use fwa_rules::RuleMatch;
use fwa_scoring::{
    aggregate_with_routing_policy, ConfidenceThresholds, RiskThresholds, RoutingPolicy,
};
use std::collections::BTreeMap;

fn feature(name: &str, value: serde_json::Value, field: &str) -> FeatureValue {
    FeatureValue {
        name: name.into(),
        version: 1,
        value,
        is_proxy: false,
        data_source: "benchmark_fixture".into(),
        evidence_refs: vec![EvidenceRef {
            entity_type: "claim".into(),
            entity_id: "CLM-BENCH".into(),
            field: field.into(),
        }],
    }
}

fn features() -> FeatureMap {
    BTreeMap::from([
        (
            "claim_amount_to_limit_ratio".into(),
            feature(
                "claim_amount_to_limit_ratio",
                serde_json::json!(0.82),
                "claim_amount",
            ),
        ),
        (
            "claim_amount_peer_percentile".into(),
            feature(
                "claim_amount_peer_percentile",
                serde_json::json!(96),
                "claim_amount",
            ),
        ),
        (
            "claim_item_count".into(),
            feature("claim_item_count", serde_json::json!(6), "items"),
        ),
        (
            "high_cost_item_ratio".into(),
            feature("high_cost_item_ratio", serde_json::json!(0.9), "items"),
        ),
        (
            "diagnosis_procedure_match_score".into(),
            feature(
                "diagnosis_procedure_match_score",
                serde_json::json!(0.32),
                "diagnosis_code",
            ),
        ),
        (
            "provider_risk_tier".into(),
            feature(
                "provider_risk_tier",
                serde_json::json!("HIGH"),
                "provider_id",
            ),
        ),
        (
            "provider_profile_score".into(),
            feature(
                "provider_profile_score",
                serde_json::json!(84),
                "provider_id",
            ),
        ),
        (
            "provider_graph_risk_score".into(),
            feature(
                "provider_graph_risk_score",
                serde_json::json!(91),
                "provider_relationships",
            ),
        ),
    ])
}

fn rule_matches() -> Vec<RuleMatch> {
    vec![
        RuleMatch {
            rule_id: "rule_early_high_amount".into(),
            rule_version: 1,
            score_contribution: 40,
            alert_code: "EARLY_HIGH_AMOUNT".into(),
            reason: "early high amount claim".into(),
            recommended_action: RecommendedAction::ManualReview,
            action_class: RuleActionClass::ManualReview,
            required_evidence: vec![],
            adjudication_policy: None,
            evidence_refs: vec![serde_json::json!("rules:rule_early_high_amount:v1")],
        },
        RuleMatch {
            rule_id: "rule_provider_graph_risk".into(),
            rule_version: 2,
            score_contribution: 35,
            alert_code: "PROVIDER_GRAPH_RISK".into(),
            reason: "provider graph risk".into(),
            recommended_action: RecommendedAction::EscalateInvestigation,
            action_class: RuleActionClass::ManualReview,
            required_evidence: vec![],
            adjudication_policy: None,
            evidence_refs: vec![serde_json::json!("rules:rule_provider_graph_risk:v2")],
        },
    ]
}

fn model_score() -> ModelScore {
    ModelScore {
        model_key: "baseline_fwa".into(),
        model_version: "0.2.0-active".into(),
        runtime_kind: "rust_logistic_regression".into(),
        execution_provider: "cpu".into(),
        score: 88,
        label: "HIGH_RISK".into(),
        explanations: vec![ModelExplanation {
            feature: "claim_amount_peer_percentile".into(),
            direction: "increases_risk".into(),
            contribution: 0.42,
            reason: "peer percentile contributes to model score".into(),
        }],
        metadata: serde_json::json!({ "bench": "scoring_aggregate" }),
        latency_ms: 3,
    }
}

fn anomaly_score() -> AnomalyScore {
    AnomalyScore {
        score: 76,
        anomaly_type: "rare_claim_pattern".into(),
        explanations: vec![AnomalyExplanation {
            signal: "peer_percentile_with_medical_mismatch".into(),
            contribution: 0.76,
            reason: "high peer percentile and diagnosis mismatch".into(),
        }],
    }
}

fn routing_policy() -> RoutingPolicy {
    RoutingPolicy {
        policy_id: "benchmark_routing".into(),
        version: 1,
        review_mode: "pre_payment".into(),
        risk_thresholds: RiskThresholds {
            low_max: 34,
            medium_min: 35,
            high_min: 65,
            critical_min: 85,
        },
        confidence_thresholds: ConfidenceThresholds {
            low_confidence_below: 50,
            high_confidence_min: 80,
        },
        provider_review_threshold: 70,
    }
}

fn bench_aggregate_with_routing_policy(c: &mut Criterion) {
    let features = features();
    let rule_matches = rule_matches();
    let model_score = model_score();
    let anomaly_score = anomaly_score();
    let policy = routing_policy();

    c.bench_function(
        "aggregate_with_routing_policy/seven_layer_full_context",
        |b| {
            b.iter(|| {
                let decision = aggregate_with_routing_policy(
                    black_box(&features),
                    black_box(&rule_matches),
                    black_box(&model_score),
                    black_box(&anomaly_score),
                    black_box(90),
                    black_box(policy.clone()),
                );
                black_box((decision.risk_score.value(), decision.layers.len()));
            })
        },
    );
}

criterion_group!(benches, bench_aggregate_with_routing_policy);
criterion_main!(benches);

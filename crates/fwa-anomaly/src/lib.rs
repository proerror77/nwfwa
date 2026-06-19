use fwa_features::FeatureMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnomalyExplanation {
    pub signal: String,
    pub contribution: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnomalyScore {
    pub score: u8,
    pub anomaly_type: String,
    pub explanations: Vec<AnomalyExplanation>,
}

// BASELINE: heuristic thresholds; replace with IQR/MAD or unsupervised ensemble
// scoring after accumulated labels/history are sufficient, initially defined as
// >=500 confirmed_fwa labels or 30-day recall below 0.70 in monitoring.
//
// Signal weights (6 signals, max achievable = 100):
//
//   claim_amount_peer_percentile       contribution 0.25   score +25
//   diagnosis_procedure_match_score    contribution 0.20   score +20
//   high_cost_item_ratio               contribution 0.20   score +20
//   provider_profile_score             contribution 0.15   score +15
//   member_provider_claim_count_30d    contribution 0.10   score +10  (temporal)
//   duplicate_claim_similarity_score   contribution 0.10   score +10  (near-duplicate)
//                                      ────────────        ───────
//                                      1.00                100
//
// `contribution` is the relative weight as a fraction of the theoretical maximum.
// It is NOT computed dynamically — a future ML scorer should replace with SHAP values.
pub fn detect_anomaly(features: &FeatureMap) -> AnomalyScore {
    let mut score = 0_u16;
    let mut explanations = Vec::new();

    // L1: claim amount relative to peer population
    if numeric_feature(features, "claim_amount_peer_percentile").unwrap_or(0.0) >= 95.0 {
        score += 25;
        explanations.push(AnomalyExplanation {
            signal: "claim_amount_peer_percentile".into(),
            contribution: 0.25,
            reason: "金额处于同类样本高分位".into(),
        });
    }

    // L2: clinical consistency between diagnosis and billed procedures
    if numeric_feature(features, "diagnosis_procedure_match_score").unwrap_or(1.0) < 0.5 {
        score += 20;
        explanations.push(AnomalyExplanation {
            signal: "diagnosis_procedure_match_score".into(),
            contribution: 0.20,
            reason: "诊断与项目匹配度偏低".into(),
        });
    }

    // L3: high proportion of expensive line items
    if numeric_feature(features, "high_cost_item_ratio").unwrap_or(0.0) >= 0.8 {
        score += 20;
        explanations.push(AnomalyExplanation {
            signal: "high_cost_item_ratio".into(),
            contribution: 0.20,
            reason: "高价项目占比较高".into(),
        });
    }

    // L4: provider risk profile
    if numeric_feature(features, "provider_profile_score").unwrap_or(0.0) >= 70.0 {
        score += 15;
        explanations.push(AnomalyExplanation {
            signal: "provider_profile_score".into(),
            contribution: 0.15,
            reason: "Provider 风险画像偏高".into(),
        });
    }

    // L5: temporal frequency — unusually high claim volume from same member-provider
    // pair within a 30-day window indicates potential churning or repeated billing.
    if numeric_feature(features, "member_provider_claim_count_30d").unwrap_or(0.0) >= 3.0 {
        score += 10;
        explanations.push(AnomalyExplanation {
            signal: "member_provider_claim_count_30d".into(),
            contribution: 0.10,
            reason: "30天内同一成员-Provider 就诊频次异常".into(),
        });
    }

    // L6: near-duplicate billing — high similarity to a recently submitted claim
    // from the same provider suggests duplicate submission or phantom billing.
    if numeric_feature(features, "duplicate_claim_similarity_score").unwrap_or(0.0) >= 0.8 {
        score += 10;
        explanations.push(AnomalyExplanation {
            signal: "duplicate_claim_similarity_score".into(),
            contribution: 0.10,
            reason: "与近期已提交索赔高度相似，疑似重复或幻影账单".into(),
        });
    }

    let score = score.min(100) as u8;
    AnomalyScore {
        score,
        anomaly_type: if score >= 70 {
            "rare_claim_pattern".into()
        } else {
            "routine_pattern".into()
        },
        explanations,
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
    use std::collections::BTreeMap;

    fn feature(name: &str, value: serde_json::Value) -> FeatureValue {
        FeatureValue {
            name: name.into(),
            version: 1,
            value,
            is_proxy: false,
            data_source: "test_fixture".into(),
            evidence_refs: vec![],
        }
    }

    #[test]
    fn detects_rare_claim_pattern_from_explainable_signals() {
        let mut features = BTreeMap::new();
        features.insert(
            "claim_amount_peer_percentile".into(),
            feature("claim_amount_peer_percentile", serde_json::json!(95)),
        );
        features.insert(
            "high_cost_item_ratio".into(),
            feature("high_cost_item_ratio", serde_json::json!(1.0)),
        );
        features.insert(
            "diagnosis_procedure_match_score".into(),
            feature("diagnosis_procedure_match_score", serde_json::json!(0.35)),
        );

        let anomaly = detect_anomaly(&features);

        // 25 (peer) + 20 (diagnosis) + 20 (high_cost) = 65
        assert_eq!(anomaly.score, 65);
        assert_eq!(anomaly.anomaly_type, "routine_pattern");
        assert_eq!(anomaly.explanations.len(), 3);
    }

    #[test]
    fn all_six_signals_reach_max_score_100() {
        let mut features = BTreeMap::new();
        for (name, value) in [
            ("claim_amount_peer_percentile", serde_json::json!(99)),
            ("high_cost_item_ratio", serde_json::json!(0.9)),
            ("diagnosis_procedure_match_score", serde_json::json!(0.1)),
            ("provider_profile_score", serde_json::json!(80)),
            ("member_provider_claim_count_30d", serde_json::json!(5)),
            ("duplicate_claim_similarity_score", serde_json::json!(0.9)),
        ] {
            features.insert(name.into(), feature(name, value));
        }

        let anomaly = detect_anomaly(&features);
        // 25 + 20 + 20 + 15 + 10 + 10 = 100
        assert_eq!(anomaly.score, 100);
        assert_eq!(anomaly.anomaly_type, "rare_claim_pattern");
        assert_eq!(anomaly.explanations.len(), 6);
        // Contributions must sum to exactly 1.0
        let total: f64 = anomaly.explanations.iter().map(|e| e.contribution).sum();
        assert!((total - 1.0).abs() < 1e-9, "contributions sum to {total}");
    }

    #[test]
    fn no_signals_produce_zero_score() {
        let anomaly = detect_anomaly(&BTreeMap::new());
        assert_eq!(anomaly.score, 0);
        assert_eq!(anomaly.anomaly_type, "routine_pattern");
        assert!(anomaly.explanations.is_empty());
    }

    #[test]
    fn temporal_signal_fires_at_threshold() {
        let mut features = BTreeMap::new();
        features.insert(
            "member_provider_claim_count_30d".into(),
            feature("member_provider_claim_count_30d", serde_json::json!(3)),
        );
        let anomaly = detect_anomaly(&features);
        assert_eq!(anomaly.score, 10);
        assert_eq!(
            anomaly.explanations[0].signal,
            "member_provider_claim_count_30d"
        );
    }

    #[test]
    fn duplicate_signal_fires_at_threshold() {
        let mut features = BTreeMap::new();
        features.insert(
            "duplicate_claim_similarity_score".into(),
            feature("duplicate_claim_similarity_score", serde_json::json!(0.85)),
        );
        let anomaly = detect_anomaly(&features);
        assert_eq!(anomaly.score, 10);
        assert_eq!(
            anomaly.explanations[0].signal,
            "duplicate_claim_similarity_score"
        );
    }

    #[test]
    fn single_peer_outlier_signal() {
        let mut features = BTreeMap::new();
        features.insert(
            "claim_amount_peer_percentile".into(),
            feature("claim_amount_peer_percentile", serde_json::json!(98)),
        );
        let anomaly = detect_anomaly(&features);
        assert_eq!(anomaly.score, 25);
        assert_eq!(anomaly.anomaly_type, "routine_pattern");
    }
}

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
// Signal weights:                      contribution  score weight
//   claim_amount_peer_percentile        0.30          +30
//   diagnosis_procedure_match_score     0.25          +25
//   high_cost_item_ratio                0.25          +25  (was +20 — raised to
//                                                           reach max score 100)
//   provider_profile_score              0.20          +20
//                                       ────          ────
//   maximum achievable score            1.00          100
//
// `contribution` is the relative weight as a fraction of the theoretical maximum;
// it is NOT computed dynamically per-evaluation.  A future ML-driven anomaly scorer
// should replace these with SHAP or integrated-gradient values.
pub fn detect_anomaly(features: &FeatureMap) -> AnomalyScore {
    let mut score = 0_u16;
    let mut explanations = Vec::new();

    if numeric_feature(features, "claim_amount_peer_percentile").unwrap_or(0.0) >= 95.0 {
        score += 30;
        explanations.push(AnomalyExplanation {
            signal: "claim_amount_peer_percentile".into(),
            contribution: 0.30,
            reason: "金额处于同类样本高分位".into(),
        });
    }

    if numeric_feature(features, "high_cost_item_ratio").unwrap_or(0.0) >= 0.8 {
        score += 25;
        explanations.push(AnomalyExplanation {
            signal: "high_cost_item_ratio".into(),
            contribution: 0.25,
            reason: "高价项目占比较高".into(),
        });
    }

    if numeric_feature(features, "diagnosis_procedure_match_score").unwrap_or(1.0) < 0.5 {
        score += 25;
        explanations.push(AnomalyExplanation {
            signal: "diagnosis_procedure_match_score".into(),
            contribution: 0.25,
            reason: "诊断与项目匹配度偏低".into(),
        });
    }

    if numeric_feature(features, "provider_profile_score").unwrap_or(0.0) >= 70.0 {
        score += 20;
        explanations.push(AnomalyExplanation {
            signal: "provider_profile_score".into(),
            contribution: 0.20,
            reason: "Provider 风险画像偏高".into(),
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

    #[test]
    fn detects_rare_claim_pattern_from_explainable_signals() {
        let mut features = BTreeMap::new();
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

        let anomaly = detect_anomaly(&features);

        // 30 (peer) + 25 (high_cost) + 25 (diagnosis) = 80
        assert_eq!(anomaly.score, 80);
        assert_eq!(anomaly.anomaly_type, "rare_claim_pattern");
        assert_eq!(anomaly.explanations.len(), 3);
    }

    #[test]
    fn all_four_signals_reach_max_score_100() {
        let mut features = BTreeMap::new();
        for (name, value) in [
            ("claim_amount_peer_percentile", serde_json::json!(99)),
            ("high_cost_item_ratio", serde_json::json!(0.9)),
            ("diagnosis_procedure_match_score", serde_json::json!(0.1)),
            ("provider_profile_score", serde_json::json!(80)),
        ] {
            features.insert(
                name.into(),
                FeatureValue {
                    name: name.into(),
                    version: 1,
                    value,
                    is_proxy: false,
                    data_source: "test_fixture".into(),
                    evidence_refs: vec![],
                },
            );
        }

        let anomaly = detect_anomaly(&features);
        // 30 + 25 + 25 + 20 = 100 — all signals fire, max score is now reachable
        assert_eq!(anomaly.score, 100);
        assert_eq!(anomaly.anomaly_type, "rare_claim_pattern");
        assert_eq!(anomaly.explanations.len(), 4);
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
    fn single_peer_outlier_signal_does_not_exceed_30() {
        let mut features = BTreeMap::new();
        features.insert(
            "claim_amount_peer_percentile".into(),
            FeatureValue {
                name: "claim_amount_peer_percentile".into(),
                version: 1,
                value: serde_json::json!(98),
                is_proxy: false,
                data_source: "test_fixture".into(),
                evidence_refs: vec![],
            },
        );
        let anomaly = detect_anomaly(&features);
        assert_eq!(anomaly.score, 30);
        assert_eq!(anomaly.anomaly_type, "routine_pattern");
    }
}

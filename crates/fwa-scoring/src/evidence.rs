use crate::DetectionLayerScore;
use fwa_anomaly::AnomalyScore;
use fwa_features::FeatureMap;
use fwa_ml_runtime::ModelScore;
use fwa_rules::RuleMatch;
use serde_json::Value;

pub(crate) fn layer(
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

pub(crate) fn feature_evidence_refs(features: &FeatureMap, names: &[&str]) -> Vec<Value> {
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

pub(crate) fn rule_layer_evidence_refs(rule_matches: &[RuleMatch]) -> Vec<Value> {
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

pub(crate) fn anomaly_layer_evidence_refs(
    features: &FeatureMap,
    anomaly_score: &AnomalyScore,
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

pub(crate) fn model_layer_evidence_refs(
    features: &FeatureMap,
    model_score: &ModelScore,
) -> Vec<Value> {
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

use fwa_core::{RecommendedAction, RuleActionClass};
use fwa_features::FeatureMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RuleError {
    #[error("unsupported operator: {0}")]
    UnsupportedOperator(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub rule_id: String,
    pub version: u32,
    pub name: String,
    #[serde(default = "default_review_mode")]
    pub review_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme_family: Option<String>,
    pub conditions: Vec<Condition>,
    pub action: RuleAction,
}

fn default_review_mode() -> String {
    "both".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub field: String,
    pub operator: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleAction {
    pub score: u8,
    pub alert_code: String,
    pub recommended_action: RecommendedAction,
    #[serde(default = "default_rule_action_class")]
    pub action_class: RuleActionClass,
    pub reason: String,
}

fn default_rule_action_class() -> RuleActionClass {
    RuleActionClass::ManualReview
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleMatch {
    pub rule_id: String,
    pub rule_version: u32,
    pub score_contribution: u8,
    pub alert_code: String,
    pub reason: String,
    pub recommended_action: RecommendedAction,
    pub action_class: RuleActionClass,
    #[serde(default)]
    pub evidence_refs: Vec<Value>,
}

pub fn evaluate_rules(rules: &[Rule], features: &FeatureMap) -> Result<Vec<RuleMatch>, RuleError> {
    let mut matches = Vec::new();
    for rule in rules {
        let matched = rule
            .conditions
            .iter()
            .map(|condition| evaluate_condition(condition, features))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .all(|value| value);

        if matched {
            matches.push(RuleMatch {
                rule_id: rule.rule_id.clone(),
                rule_version: rule.version,
                score_contribution: rule.action.score,
                alert_code: rule.action.alert_code.clone(),
                reason: rule.action.reason.clone(),
                recommended_action: rule.action.recommended_action,
                action_class: rule.action.action_class,
                evidence_refs: rule_evidence_refs(rule, features),
            });
        }
    }
    Ok(matches)
}

fn rule_evidence_refs(rule: &Rule, features: &FeatureMap) -> Vec<Value> {
    let mut refs = vec![Value::String(format!(
        "rules:{}:v{}",
        rule.rule_id, rule.version
    ))];
    for condition in &rule.conditions {
        if let Some(feature) = features.get(&condition.field) {
            refs.push(Value::String(format!(
                "feature_values:{}:v{}",
                feature.name, feature.version
            )));
            refs.extend(
                feature
                    .evidence_refs
                    .iter()
                    .map(|evidence| serde_json::to_value(evidence).unwrap_or(Value::Null)),
            );
        }
    }
    refs
}

fn evaluate_condition(condition: &Condition, features: &FeatureMap) -> Result<bool, RuleError> {
    let Some(feature) = features.get(&condition.field) else {
        return Ok(false);
    };

    match condition.operator.as_str() {
        "<=" => Ok(as_f64(&feature.value) <= as_f64(&condition.value)),
        ">=" => Ok(as_f64(&feature.value) >= as_f64(&condition.value)),
        "==" => Ok(feature.value == condition.value),
        other => Err(RuleError::UnsupportedOperator(other.to_string())),
    }
}

fn as_f64(value: &Value) -> f64 {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|v| v as f64))
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fwa_features::FeatureValue;
    use std::collections::BTreeMap;

    #[test]
    fn matches_rule_when_all_conditions_match() {
        let mut features = BTreeMap::new();
        features.insert(
            "days_since_policy_start".into(),
            FeatureValue {
                name: "days_since_policy_start".into(),
                version: 1,
                value: serde_json::json!(5),
                evidence_refs: vec![fwa_features::EvidenceRef {
                    entity_type: "claim".into(),
                    entity_id: "CLM-1".into(),
                    field: "service_date".into(),
                }],
            },
        );

        let rules = vec![Rule {
            rule_id: "rule_early_claim".into(),
            version: 1,
            name: "Early claim".into(),
            review_mode: "both".into(),
            scheme_family: None,
            conditions: vec![Condition {
                field: "days_since_policy_start".into(),
                operator: "<=".into(),
                value: serde_json::json!(7),
            }],
            action: RuleAction {
                score: 25,
                alert_code: "EARLY_CLAIM".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                reason: "保单生效后 7 天内发生理赔".into(),
            },
        }];

        let matches = evaluate_rules(&rules, &features).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].alert_code, "EARLY_CLAIM");
        assert!(matches[0]
            .evidence_refs
            .contains(&serde_json::json!("rules:rule_early_claim:v1")));
        assert!(matches[0].evidence_refs.contains(&serde_json::json!(
            "feature_values:days_since_policy_start:v1"
        )));
        assert!(matches[0].evidence_refs.contains(&serde_json::json!({
            "entity_type": "claim",
            "entity_id": "CLM-1",
            "field": "service_date"
        })));
    }

    #[test]
    fn missing_feature_does_not_match() {
        let rules = vec![Rule {
            rule_id: "rule_missing".into(),
            version: 1,
            name: "Missing".into(),
            review_mode: "both".into(),
            scheme_family: None,
            conditions: vec![Condition {
                field: "unknown_feature".into(),
                operator: "==".into(),
                value: serde_json::json!(1),
            }],
            action: RuleAction {
                score: 10,
                alert_code: "MISSING".into(),
                recommended_action: RecommendedAction::ManualReview,
                action_class: RuleActionClass::ManualReview,
                reason: "missing".into(),
            },
        }];

        let matches = evaluate_rules(&rules, &BTreeMap::new()).unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn defaults_rule_action_class_for_legacy_dsl() {
        let action: RuleAction = serde_json::from_value(serde_json::json!({
            "score": 25,
            "alert_code": "LEGACY",
            "recommended_action": "ManualReview",
            "reason": "legacy rule"
        }))
        .unwrap();

        assert_eq!(action.action_class, RuleActionClass::ManualReview);
    }
}

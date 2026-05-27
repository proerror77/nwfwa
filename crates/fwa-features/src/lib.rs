use fwa_core::{ClaimContext, ProviderRiskTier};
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceRef {
    pub entity_type: String,
    pub entity_id: String,
    pub field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeatureValue {
    pub name: String,
    pub version: u16,
    pub value: Value,
    pub evidence_refs: Vec<EvidenceRef>,
}

pub type FeatureMap = BTreeMap<String, FeatureValue>;

pub fn calculate_features(context: &ClaimContext) -> FeatureMap {
    let mut features = FeatureMap::new();
    let claim_id = context.claim.external_claim_id.clone();

    let days_since_policy_start =
        (context.claim.service_date - context.policy.coverage_start_date).num_days();
    insert_number(
        &mut features,
        "days_since_policy_start",
        days_since_policy_start,
        "claim",
        &claim_id,
        "service_date",
    );

    let claim_amount = context.claim.amount.amount;
    let limit = context.policy.coverage_limit.amount;
    let ratio = if limit.is_zero() {
        0.0
    } else {
        (claim_amount / limit).to_f64().unwrap_or(0.0)
    };
    insert_number(
        &mut features,
        "claim_amount_to_limit_ratio",
        ratio,
        "claim",
        &claim_id,
        "claim_amount",
    );

    insert_number(
        &mut features,
        "claim_item_count",
        context.items.len() as i64,
        "claim",
        &claim_id,
        "claim_items",
    );

    let provider_risk = match context.provider.risk_tier {
        ProviderRiskTier::Low => "LOW",
        ProviderRiskTier::Medium => "MEDIUM",
        ProviderRiskTier::High => "HIGH",
    };
    insert_string(
        &mut features,
        "provider_risk_tier",
        provider_risk,
        "provider",
        &context.provider.external_provider_id,
        "risk_tier",
    );

    features
}

fn insert_number(
    features: &mut FeatureMap,
    name: &str,
    value: impl serde::Serialize,
    entity_type: &str,
    entity_id: &str,
    field: &str,
) {
    features.insert(
        name.to_string(),
        FeatureValue {
            name: name.to_string(),
            version: 1,
            value: serde_json::to_value(value).expect("feature value serializes"),
            evidence_refs: vec![EvidenceRef {
                entity_type: entity_type.to_string(),
                entity_id: entity_id.to_string(),
                field: field.to_string(),
            }],
        },
    );
}

fn insert_string(
    features: &mut FeatureMap,
    name: &str,
    value: &str,
    entity_type: &str,
    entity_id: &str,
    field: &str,
) {
    features.insert(
        name.to_string(),
        FeatureValue {
            name: name.to_string(),
            version: 1,
            value: Value::String(value.to_string()),
            evidence_refs: vec![EvidenceRef {
                entity_type: entity_type.to_string(),
                entity_id: entity_id.to_string(),
                field: field.to_string(),
            }],
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use fwa_core::*;
    use rust_decimal::Decimal;

    fn context() -> ClaimContext {
        let member_id = MemberId::from_external("MBR-1");
        let policy_id = PolicyId::from_external("POL-1");
        let provider_id = ProviderId::from_external("PRV-1");
        ClaimContext {
            claim: Claim {
                id: ClaimId::from_external("CLM-1"),
                external_claim_id: "CLM-1".into(),
                member_id: member_id.clone(),
                policy_id: policy_id.clone(),
                provider_id: provider_id.clone(),
                diagnosis_code: "J10".into(),
                service_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 6).unwrap(),
                amount: Money::new(Decimal::new(8000, 0), "CNY"),
            },
            items: vec![],
            member: Member {
                id: member_id.clone(),
                external_member_id: "MBR-1".into(),
                dob: None,
                gender: None,
            },
            policy: Policy {
                id: policy_id,
                external_policy_id: "POL-1".into(),
                member_id,
                product_code: "MED".into(),
                coverage_start_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
                coverage_end_date: chrono::NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
                coverage_limit: Money::new(Decimal::new(10000, 0), "CNY"),
            },
            provider: Provider {
                id: provider_id,
                external_provider_id: "PRV-1".into(),
                name: "Demo Hospital".into(),
                provider_type: "hospital".into(),
                region: "SH".into(),
                risk_tier: ProviderRiskTier::Medium,
            },
        }
    }

    #[test]
    fn calculates_policy_age_and_amount_ratio() {
        let features = calculate_features(&context());
        assert_eq!(
            features["days_since_policy_start"].value,
            serde_json::json!(5)
        );
        assert_eq!(
            features["claim_amount_to_limit_ratio"].value,
            serde_json::json!(0.8)
        );
    }

    #[test]
    fn captures_provider_risk_evidence() {
        let features = calculate_features(&context());
        let risk = &features["provider_risk_tier"];
        assert_eq!(risk.value, serde_json::json!("MEDIUM"));
        assert_eq!(risk.evidence_refs[0].entity_type, "provider");
    }
}

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
    #[serde(default)]
    pub is_proxy: bool,
    #[serde(default = "default_feature_data_source")]
    pub data_source: String,
    pub evidence_refs: Vec<EvidenceRef>,
}

pub type FeatureMap = BTreeMap<String, FeatureValue>;

fn default_feature_data_source() -> String {
    "unknown".into()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PeerFeatureContext {
    pub claim_amount_peer_percentile: Option<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderProfileFeatureContext {
    pub risk_score: Option<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ClinicalCompatibilityFeatureContext {
    pub diagnosis_procedure_match_score: Option<f64>,
    pub data_source: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EpisodeUtilizationFeatureContext {
    pub member_provider_claim_count_30d: Option<u32>,
    pub duplicate_claim_similarity_score: Option<f64>,
    pub procedure_frequency_peer_percentile: Option<u8>,
    pub unbundling_candidate_count: Option<u32>,
    pub data_source: Option<String>,
}

pub fn calculate_features(context: &ClaimContext) -> FeatureMap {
    calculate_features_with_peer_context(context, None)
}

pub fn calculate_features_with_peer_context(
    context: &ClaimContext,
    peer_context: Option<&PeerFeatureContext>,
) -> FeatureMap {
    calculate_features_with_contexts(context, peer_context, None)
}

pub fn calculate_features_with_contexts(
    context: &ClaimContext,
    peer_context: Option<&PeerFeatureContext>,
    provider_profile_context: Option<&ProviderProfileFeatureContext>,
) -> FeatureMap {
    calculate_features_with_all_contexts(context, peer_context, provider_profile_context, None)
}

pub fn calculate_features_with_all_contexts(
    context: &ClaimContext,
    peer_context: Option<&PeerFeatureContext>,
    provider_profile_context: Option<&ProviderProfileFeatureContext>,
    clinical_compatibility_context: Option<&ClinicalCompatibilityFeatureContext>,
) -> FeatureMap {
    calculate_features_with_operational_contexts(
        context,
        peer_context,
        provider_profile_context,
        clinical_compatibility_context,
        None,
    )
}

pub fn calculate_features_with_operational_contexts(
    context: &ClaimContext,
    peer_context: Option<&PeerFeatureContext>,
    provider_profile_context: Option<&ProviderProfileFeatureContext>,
    clinical_compatibility_context: Option<&ClinicalCompatibilityFeatureContext>,
    episode_utilization_context: Option<&EpisodeUtilizationFeatureContext>,
) -> FeatureMap {
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
    if let Some(peer_percentile) =
        peer_context.and_then(|context| context.claim_amount_peer_percentile)
    {
        insert_number_with_metadata(
            &mut features,
            "claim_amount_peer_percentile",
            peer_percentile.min(100),
            "claim_peer_stats",
            &claim_id,
            "claim_amount_peer_percentile",
            false,
            "worker.peer_percentile_benchmark_rollup",
        );
    }

    insert_number(
        &mut features,
        "claim_item_count",
        context.items.len() as i64,
        "claim",
        &claim_id,
        "claim_items",
    );
    insert_number(
        &mut features,
        "high_cost_item_ratio",
        high_cost_item_ratio(context),
        "claim",
        &claim_id,
        "claim_items",
    );
    insert_episode_utilization_features(&mut features, &claim_id, episode_utilization_context);
    let clinical_match_score = clinical_compatibility_context
        .and_then(|context| {
            context
                .diagnosis_procedure_match_score
                .map(|score| (score.clamp(0.0, 1.0), false, context.data_source.as_deref()))
        })
        .unwrap_or_else(|| {
            (
                diagnosis_procedure_match_score(context),
                true,
                Some("diagnosis_procedure_heuristic"),
            )
        });
    insert_number_with_metadata(
        &mut features,
        "diagnosis_procedure_match_score",
        clinical_match_score.0,
        "claim",
        &claim_id,
        "diagnosis_code",
        clinical_match_score.1,
        clinical_match_score
            .2
            .unwrap_or("worker.icd_cpt_compatibility_reference"),
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
    let provider_profile_score = provider_profile_context
        .and_then(|context| context.risk_score)
        .map(|score| (score, false, "worker.provider_profile_window_rollup"))
        .unwrap_or_else(|| {
            (
                provider_profile_score(context.provider.risk_tier),
                true,
                "provider.risk_tier_baseline",
            )
        });
    insert_number_with_metadata(
        &mut features,
        "provider_profile_score",
        provider_profile_score.0.min(100),
        "provider",
        &context.provider.external_provider_id,
        "risk_tier",
        provider_profile_score.1,
        provider_profile_score.2,
    );

    features
}

fn insert_episode_utilization_features(
    features: &mut FeatureMap,
    claim_id: &str,
    context: Option<&EpisodeUtilizationFeatureContext>,
) {
    let Some(context) = context else {
        return;
    };
    let data_source = context
        .data_source
        .as_deref()
        .unwrap_or("worker.episode_utilization_rollup");
    if let Some(count) = context.member_provider_claim_count_30d {
        insert_number_with_metadata(
            features,
            "member_provider_claim_count_30d",
            count,
            "member_provider_episode",
            claim_id,
            "claim_count_30d",
            false,
            data_source,
        );
    }
    if let Some(score) = context.duplicate_claim_similarity_score {
        insert_number_with_metadata(
            features,
            "duplicate_claim_similarity_score",
            score.clamp(0.0, 1.0),
            "member_provider_episode",
            claim_id,
            "duplicate_claim_similarity_score",
            false,
            data_source,
        );
    }
    if let Some(percentile) = context.procedure_frequency_peer_percentile {
        insert_number_with_metadata(
            features,
            "procedure_frequency_peer_percentile",
            percentile.min(100),
            "provider_peer_stats",
            claim_id,
            "procedure_frequency_peer_percentile",
            false,
            data_source,
        );
    }
    if let Some(count) = context.unbundling_candidate_count {
        insert_number_with_metadata(
            features,
            "unbundling_candidate_count",
            count,
            "member_provider_episode",
            claim_id,
            "unbundling_candidate_count",
            false,
            data_source,
        );
    }
}

fn high_cost_item_ratio(context: &ClaimContext) -> f64 {
    if context.items.is_empty() || context.claim.amount.amount.is_zero() {
        return 0.0;
    }
    let claim_amount = context.claim.amount.amount;
    let high_cost_items = context
        .items
        .iter()
        .filter(|item| {
            (item.total_amount.amount / claim_amount)
                .to_f64()
                .unwrap_or(0.0)
                >= 0.5
        })
        .count();
    high_cost_items as f64 / context.items.len() as f64
}

fn diagnosis_procedure_match_score(context: &ClaimContext) -> f64 {
    // PLACEHOLDER: hard-coded ICD prefix heuristic. Replace with governed
    // ICD-10/CPT compatibility or medical-policy reference data before using as
    // a production clinical consistency score.
    let has_imaging = context.items.iter().any(|item| {
        item.item_type.eq_ignore_ascii_case("procedure")
            && item.description.to_ascii_lowercase().contains("imaging")
    });
    if has_imaging && context.claim.diagnosis_code.starts_with('J') {
        0.35
    } else if has_imaging {
        0.55
    } else {
        0.80
    }
}

fn provider_profile_score(risk_tier: ProviderRiskTier) -> u8 {
    match risk_tier {
        ProviderRiskTier::Low => 10,
        ProviderRiskTier::Medium => 45,
        ProviderRiskTier::High => 80,
    }
}

fn insert_number(
    features: &mut FeatureMap,
    name: &str,
    value: impl serde::Serialize,
    entity_type: &str,
    entity_id: &str,
    field: &str,
) {
    insert_number_with_metadata(
        features,
        name,
        value,
        entity_type,
        entity_id,
        field,
        false,
        entity_type,
    )
}

fn insert_number_with_metadata(
    features: &mut FeatureMap,
    name: &str,
    value: impl serde::Serialize,
    entity_type: &str,
    entity_id: &str,
    field: &str,
    is_proxy: bool,
    data_source: &str,
) {
    features.insert(
        name.to_string(),
        FeatureValue {
            name: name.to_string(),
            version: 1,
            value: serde_json::to_value(value).expect("feature value serializes"),
            is_proxy,
            data_source: data_source.to_string(),
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
            is_proxy: false,
            data_source: entity_type.to_string(),
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
            items: vec![ClaimItem {
                item_code: "IMG-001".into(),
                item_type: "procedure".into(),
                description: "High cost imaging".into(),
                quantity: 1,
                unit_amount: Money::new(Decimal::new(8000, 0), "CNY"),
                total_amount: Money::new(Decimal::new(8000, 0), "CNY"),
            }],
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
        assert!(!features["claim_amount_to_limit_ratio"].is_proxy);
        assert_eq!(features["claim_amount_to_limit_ratio"].data_source, "claim");
    }

    #[test]
    fn captures_provider_risk_evidence() {
        let features = calculate_features(&context());
        let risk = &features["provider_risk_tier"];
        assert_eq!(risk.value, serde_json::json!("MEDIUM"));
        assert_eq!(risk.evidence_refs[0].entity_type, "provider");
    }

    #[test]
    fn does_not_invent_peer_percentile_from_amount_ratio() {
        let features = calculate_features(&context());

        assert!(!features.contains_key("claim_amount_peer_percentile"));
        assert_eq!(
            features["claim_amount_to_limit_ratio"].value,
            serde_json::json!(0.8)
        );
    }

    #[test]
    fn accepts_real_peer_percentile_from_peer_context() {
        let peer_context = PeerFeatureContext {
            claim_amount_peer_percentile: Some(95),
        };
        let features = calculate_features_with_peer_context(&context(), Some(&peer_context));

        assert_eq!(
            features["claim_amount_peer_percentile"].value,
            serde_json::json!(95)
        );
        assert_eq!(
            features["claim_amount_peer_percentile"].evidence_refs[0].entity_type,
            "claim_peer_stats"
        );
        assert!(!features["claim_amount_peer_percentile"].is_proxy);
        assert_eq!(
            features["claim_amount_peer_percentile"].data_source,
            "worker.peer_percentile_benchmark_rollup"
        );
    }

    #[test]
    fn calculates_medical_and_provider_layer_features() {
        let features = calculate_features(&context());

        assert_eq!(
            features["high_cost_item_ratio"].value,
            serde_json::json!(1.0)
        );
        assert_eq!(
            features["diagnosis_procedure_match_score"].value,
            serde_json::json!(0.35)
        );
        assert!(features["diagnosis_procedure_match_score"].is_proxy);
        assert_eq!(
            features["diagnosis_procedure_match_score"].data_source,
            "diagnosis_procedure_heuristic"
        );
        assert_eq!(
            features["provider_profile_score"].value,
            serde_json::json!(45)
        );
        assert!(features["provider_profile_score"].is_proxy);
        assert_eq!(
            features["provider_profile_score"].data_source,
            "provider.risk_tier_baseline"
        );
    }

    #[test]
    fn accepts_provider_profile_score_from_profile_context() {
        let provider_context = ProviderProfileFeatureContext {
            risk_score: Some(100),
        };
        let features = calculate_features_with_contexts(&context(), None, Some(&provider_context));

        assert_eq!(
            features["provider_profile_score"].value,
            serde_json::json!(100)
        );
        assert!(!features["provider_profile_score"].is_proxy);
        assert_eq!(
            features["provider_profile_score"].data_source,
            "worker.provider_profile_window_rollup"
        );
    }

    #[test]
    fn accepts_real_clinical_compatibility_score_from_reference_context() {
        let clinical_context = ClinicalCompatibilityFeatureContext {
            diagnosis_procedure_match_score: Some(1.2),
            data_source: Some("worker.icd_cpt_compatibility_reference".into()),
        };
        let features =
            calculate_features_with_all_contexts(&context(), None, None, Some(&clinical_context));

        assert_eq!(
            features["diagnosis_procedure_match_score"].value,
            serde_json::json!(1.0)
        );
        assert!(!features["diagnosis_procedure_match_score"].is_proxy);
        assert_eq!(
            features["diagnosis_procedure_match_score"].data_source,
            "worker.icd_cpt_compatibility_reference"
        );
    }

    #[test]
    fn accepts_episode_utilization_features_from_worker_context() {
        let episode_context = EpisodeUtilizationFeatureContext {
            member_provider_claim_count_30d: Some(4),
            duplicate_claim_similarity_score: Some(1.4),
            procedure_frequency_peer_percentile: Some(99),
            unbundling_candidate_count: Some(2),
            data_source: Some("worker.episode_and_unbundling_rollup".into()),
        };
        let features = calculate_features_with_operational_contexts(
            &context(),
            None,
            None,
            None,
            Some(&episode_context),
        );

        assert_eq!(
            features["member_provider_claim_count_30d"].value,
            serde_json::json!(4)
        );
        assert_eq!(
            features["duplicate_claim_similarity_score"].value,
            serde_json::json!(1.0)
        );
        assert_eq!(
            features["procedure_frequency_peer_percentile"].value,
            serde_json::json!(99)
        );
        assert_eq!(
            features["unbundling_candidate_count"].value,
            serde_json::json!(2)
        );
        for name in [
            "member_provider_claim_count_30d",
            "duplicate_claim_similarity_score",
            "procedure_frequency_peer_percentile",
            "unbundling_candidate_count",
        ] {
            assert!(!features[name].is_proxy);
            assert_eq!(
                features[name].data_source,
                "worker.episode_and_unbundling_rollup"
            );
        }
    }
}

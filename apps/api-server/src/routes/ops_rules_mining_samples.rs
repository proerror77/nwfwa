use super::ops_rules::{RuleBacktestRequest, RuleBacktestSample, RuleDiscoveryRequest};
use super::ops_rules_mining_data::read_parquet_mining_samples;
use fwa_core::{
    Claim, ClaimContext, ClaimId, Member, MemberId, Money, Policy, PolicyId, Provider, ProviderId,
    ProviderRiskTier,
};
use fwa_features::{calculate_features, FeatureMap, FeatureValue};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub(super) struct MiningSample {
    pub(super) claim_id: String,
    pub(super) claim_amount: Decimal,
    pub(super) confirmed_fwa: Option<bool>,
    pub(super) features: BTreeMap<String, f64>,
}

pub(super) fn discovery_mining_samples(
    request: &RuleDiscoveryRequest,
) -> anyhow::Result<Vec<MiningSample>> {
    if let Some(dataset_uri) = normalized_optional_str(request.dataset_uri.as_deref()) {
        return read_parquet_mining_samples(
            dataset_uri,
            request.label_column.as_deref(),
            request.claim_id_column.as_deref(),
            request.candidate_feature_fields.as_deref(),
        );
    }

    Ok(request
        .samples
        .iter()
        .map(|sample| {
            mining_sample_from_backtest_sample(&sample.sample, Some(sample.confirmed_fwa))
        })
        .collect())
}

pub(super) fn backtest_mining_samples(
    request: &RuleBacktestRequest,
) -> anyhow::Result<Vec<MiningSample>> {
    if let Some(dataset_uri) = normalized_optional_str(request.dataset_uri.as_deref()) {
        return read_parquet_mining_samples(
            dataset_uri,
            request.label_column.as_deref(),
            request.claim_id_column.as_deref(),
            None,
        );
    }

    Ok(request
        .samples
        .iter()
        .map(|sample| mining_sample_from_backtest_sample(sample, sample.confirmed_fwa))
        .collect())
}

fn mining_sample_from_backtest_sample(
    sample: &RuleBacktestSample,
    confirmed_fwa: Option<bool>,
) -> MiningSample {
    let context = sample_context(sample);
    let features = calculate_features(&context)
        .into_iter()
        .filter_map(|(name, feature)| feature.value.as_f64().map(|value| (name, value)))
        .collect::<BTreeMap<_, _>>();
    MiningSample {
        claim_id: sample.external_claim_id.clone(),
        claim_amount: sample.claim_amount,
        confirmed_fwa,
        features,
    }
}

pub(super) fn feature_map_from_mining_sample(sample: &MiningSample) -> FeatureMap {
    sample
        .features
        .iter()
        .map(|(name, value)| {
            (
                name.clone(),
                FeatureValue {
                    name: name.clone(),
                    version: 1,
                    value: serde_json::json!(value),
                    is_proxy: false,
                    data_source: "rule_mining_sample".into(),
                    evidence_refs: vec![],
                },
            )
        })
        .collect()
}

pub(super) fn normalized_optional_str(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn sample_context(sample: &RuleBacktestSample) -> ClaimContext {
    let member_id = MemberId::from_external(format!("MBR-{}", sample.external_claim_id));
    let policy_id = PolicyId::from_external(sample.policy.external_policy_id.clone());
    let provider_id = ProviderId::from_external("PRV-BACKTEST");
    ClaimContext {
        claim: Claim {
            id: ClaimId::from_external(sample.external_claim_id.clone()),
            external_claim_id: sample.external_claim_id.clone(),
            member_id: member_id.clone(),
            policy_id: policy_id.clone(),
            provider_id: provider_id.clone(),
            diagnosis_code: "J10".into(),
            diagnosis_codes: vec![],
            service_date: sample.service_date,
            amount: Money::new(sample.claim_amount, sample.currency.clone()),
        },
        items: vec![],
        member: Member {
            id: member_id.clone(),
            external_member_id: member_id.to_string(),
            dob: None,
            gender: None,
        },
        policy: Policy {
            id: policy_id,
            external_policy_id: sample.policy.external_policy_id.clone(),
            member_id,
            product_code: "MED".into(),
            coverage_start_date: sample.policy.coverage_start_date,
            coverage_end_date: sample.policy.coverage_end_date,
            coverage_limit: Money::new(sample.policy.coverage_limit, sample.currency.clone()),
        },
        provider: Provider {
            id: provider_id,
            external_provider_id: "PRV-BACKTEST".into(),
            name: "Backtest Provider".into(),
            provider_type: "hospital".into(),
            region: "SH".into(),
            risk_tier: ProviderRiskTier::Medium,
        },
    }
}

use super::{ProviderRiskSummaryItemRecord, ProviderRiskSummaryRecord};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Default)]
struct ProviderRiskAccumulator {
    provider_id: String,
    risk_score: u8,
    risk_tier: String,
    review_required: bool,
    review_route: String,
    claim_count: u32,
    specialty: Option<String>,
    network_status: Option<String>,
    review_failure_count: u32,
    confirmed_fwa_count: u32,
    false_positive_count: u32,
    network_risk_score: Option<u8>,
    latest_claim_id: Option<String>,
    outlier_flags: BTreeSet<String>,
    graph_reasons: BTreeSet<String>,
    evidence_refs: BTreeSet<String>,
}

pub(super) fn summarize_provider_risk_profiles<'a>(
    payloads: impl Iterator<Item = &'a Value>,
) -> ProviderRiskSummaryRecord {
    let mut providers = BTreeMap::<String, ProviderRiskAccumulator>::new();

    for payload in payloads {
        let mut counted_provider_id = None::<String>;

        if let Some(profile) = payload.get("provider_profile") {
            if let Some(provider_id) = profile.get("provider_id").and_then(Value::as_str) {
                let risk_score = profile
                    .get("risk_score")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    .min(100) as u8;
                let entry = provider_accumulator_entry(&mut providers, provider_id);

                touch_provider_accumulator(entry, payload);
                counted_provider_id = Some(provider_id.to_string());
                entry.review_required |= profile
                    .get("review_required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);

                if risk_score >= entry.risk_score {
                    entry.risk_score = risk_score;
                    entry.risk_tier = profile
                        .get("risk_tier")
                        .and_then(Value::as_str)
                        .unwrap_or("low")
                        .to_string();
                    entry.review_route = profile
                        .get("review_route")
                        .and_then(Value::as_str)
                        .unwrap_or("none")
                        .to_string();
                    entry.specialty = profile
                        .get("specialty")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    entry.network_status = profile
                        .get("network_status")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                }
                entry.review_failure_count = entry.review_failure_count.max(
                    profile
                        .get("review_failure_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        .min(u32::MAX as u64) as u32,
                );
                entry.confirmed_fwa_count = entry.confirmed_fwa_count.max(
                    profile
                        .get("confirmed_fwa_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        .min(u32::MAX as u64) as u32,
                );
                entry.false_positive_count = entry.false_positive_count.max(
                    profile
                        .get("false_positive_count")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        .min(u32::MAX as u64) as u32,
                );

                extend_string_set(&mut entry.outlier_flags, profile.get("outlier_flags"));
                extend_string_set(&mut entry.evidence_refs, profile.get("evidence_refs"));
            }
        }

        if let Some(graph) = payload.get("provider_relationships") {
            if let Some(provider_id) = graph.get("provider_id").and_then(Value::as_str) {
                let risk_score = graph
                    .get("risk_score")
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    .min(100) as u8;
                let entry = provider_accumulator_entry(&mut providers, provider_id);

                if counted_provider_id.as_deref() != Some(provider_id) {
                    touch_provider_accumulator(entry, payload);
                }
                entry.review_required |= graph
                    .get("review_required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                entry.network_risk_score =
                    Some(entry.network_risk_score.unwrap_or(0).max(risk_score));

                if risk_score >= entry.risk_score {
                    entry.risk_score = risk_score;
                    entry.risk_tier = graph
                        .get("risk_tier")
                        .and_then(Value::as_str)
                        .unwrap_or("low")
                        .to_string();
                    entry.review_route = graph
                        .get("review_route")
                        .and_then(Value::as_str)
                        .unwrap_or("none")
                        .to_string();
                }

                extend_string_set(&mut entry.graph_reasons, graph.get("graph_reasons"));
                extend_string_set(&mut entry.evidence_refs, graph.get("evidence_refs"));
            }
        }
    }

    let mut providers = providers
        .into_values()
        .map(|provider| ProviderRiskSummaryItemRecord {
            provider_id: provider.provider_id,
            risk_score: provider.risk_score,
            risk_tier: provider.risk_tier,
            review_required: provider.review_required,
            review_route: provider.review_route,
            claim_count: provider.claim_count,
            specialty: provider.specialty,
            network_status: provider.network_status,
            review_failure_count: provider.review_failure_count,
            confirmed_fwa_count: provider.confirmed_fwa_count,
            false_positive_count: provider.false_positive_count,
            network_risk_score: provider.network_risk_score,
            latest_claim_id: provider.latest_claim_id,
            outlier_flags: provider.outlier_flags.into_iter().collect(),
            graph_reasons: provider.graph_reasons.into_iter().collect(),
            evidence_refs: provider.evidence_refs.into_iter().collect(),
        })
        .collect::<Vec<_>>();
    providers.sort_by(|left, right| {
        right
            .risk_score
            .cmp(&left.risk_score)
            .then_with(|| left.provider_id.cmp(&right.provider_id))
    });

    ProviderRiskSummaryRecord {
        provider_count: providers.len() as u32,
        review_required_count: providers
            .iter()
            .filter(|provider| provider.review_required)
            .count() as u32,
        high_risk_count: providers
            .iter()
            .filter(|provider| provider.risk_score >= 70)
            .count() as u32,
        providers,
    }
}

fn provider_accumulator_entry<'a>(
    providers: &'a mut BTreeMap<String, ProviderRiskAccumulator>,
    provider_id: &str,
) -> &'a mut ProviderRiskAccumulator {
    providers
        .entry(provider_id.to_string())
        .or_insert_with(|| ProviderRiskAccumulator {
            provider_id: provider_id.to_string(),
            risk_tier: "low".into(),
            review_route: "none".into(),
            ..ProviderRiskAccumulator::default()
        })
}

fn touch_provider_accumulator(entry: &mut ProviderRiskAccumulator, payload: &Value) {
    entry.claim_count += 1;
    entry.latest_claim_id = payload
        .get("claim_id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| entry.latest_claim_id.clone());
}

fn extend_string_set(target: &mut BTreeSet<String>, value: Option<&Value>) {
    if let Some(items) = value.and_then(Value::as_array) {
        target.extend(items.iter().filter_map(Value::as_str).map(str::to_string));
    }
}

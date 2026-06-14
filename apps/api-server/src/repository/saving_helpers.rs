use super::{
    decimal_to_f64, normalize_financial_impact_type, DashboardSavingAttributionRecord,
    DashboardSavingSegmentRecord, InvestigationResultRecord, LeadRecord, SavingAttributionRecord,
    RULE_REVIEW_COST_AMOUNT,
};
use rust_decimal::Decimal;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn derive_saving_attributions(
    record: &InvestigationResultRecord,
) -> Vec<SavingAttributionRecord> {
    if !record.confirmed_fwa {
        return Vec::new();
    }
    let Some(total_saving) = record.saving_amount else {
        return Vec::new();
    };
    if total_saving <= Decimal::ZERO {
        return Vec::new();
    }

    let sources = record
        .evidence_refs
        .iter()
        .filter_map(|reference| recognized_attribution_source(reference))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if sources.is_empty() {
        return Vec::new();
    }

    let share = (total_saving / Decimal::from(sources.len() as u32)).round_dp(2);
    let currency = record.currency.clone().unwrap_or_else(|| "UNKNOWN".into());
    let financial_impact_type =
        normalize_financial_impact_type(record.financial_impact_type.as_deref()).to_string();

    sources
        .into_iter()
        .map(|(source_type, source_id)| SavingAttributionRecord {
            attribution_id: format!(
                "saving_{}_{}_{}",
                sanitize_identifier(&record.investigation_id),
                source_type,
                sanitize_identifier(&source_id)
            ),
            claim_id: record.claim_id.clone(),
            investigation_id: record.investigation_id.clone(),
            source_type,
            source_id,
            financial_impact_type: financial_impact_type.clone(),
            action: "investigation_confirmed".into(),
            saving_amount: share,
            currency: currency.clone(),
            evidence_refs: record.evidence_refs.clone(),
        })
        .collect()
}

fn recognized_attribution_source(reference: &str) -> Option<(String, String)> {
    if let Some(source_id) = reference.strip_prefix("agent_run:") {
        return Some(("agent".into(), source_id.to_string()));
    }
    if let Some(source_id) = reference.strip_prefix("rule_runs:") {
        return Some(("rule".into(), source_id.to_string()));
    }
    if let Some(source_id) = reference.strip_prefix("rules:") {
        return non_empty_prefix_before_version(source_id)
            .map(|source_id| ("rule".into(), source_id.to_string()));
    }
    if let Some(source_id) = reference.strip_prefix("model_scores:") {
        return Some(("model".into(), source_id.to_string()));
    }
    if let Some(source_id) = reference.strip_prefix("model_versions:") {
        return non_empty_prefix_before_version(source_id)
            .map(|source_id| ("model".into(), source_id.to_string()));
    }
    None
}

fn non_empty_prefix_before_version(reference_body: &str) -> Option<&str> {
    reference_body
        .split(':')
        .next()
        .map(str::trim)
        .filter(|source_id| !source_id.is_empty())
}

pub(super) fn summarize_saving_attributions(
    records: &[SavingAttributionRecord],
) -> Vec<DashboardSavingAttributionRecord> {
    let mut accumulators = BTreeMap::<
        (String, String, String, String, String),
        (Decimal, u32, BTreeSet<String>),
    >::new();
    for record in records {
        let key = (
            record.source_type.clone(),
            record.source_id.clone(),
            record.financial_impact_type.clone(),
            record.action.clone(),
            record.currency.clone(),
        );
        let entry = accumulators
            .entry(key)
            .or_insert((Decimal::ZERO, 0, BTreeSet::new()));
        entry.0 += record.saving_amount;
        entry.1 += 1;
        entry.2.extend(record.evidence_refs.iter().cloned());
    }

    accumulators
        .into_iter()
        .map(
            |(
                (source_type, source_id, financial_impact_type, action, currency),
                (saving_amount, claim_count, evidence_refs),
            )| {
                DashboardSavingAttributionRecord {
                    source_type,
                    source_id,
                    financial_impact_type,
                    action,
                    saving_amount: format_decimal_cents(saving_amount),
                    currency,
                    claim_count,
                    evidence_refs: evidence_refs.into_iter().collect(),
                }
            },
        )
        .collect()
}

pub(super) fn summarize_saving_segments(
    records: &[SavingAttributionRecord],
    leads: &[LeadRecord],
) -> Vec<DashboardSavingSegmentRecord> {
    let claim_segments = leads
        .iter()
        .map(|lead| {
            (
                lead.claim_id.as_str(),
                (lead.provider_id.as_str(), lead.scheme_family.as_str()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut accumulators =
        BTreeMap::<(String, String, String), (Decimal, BTreeSet<String>, u32)>::new();

    for record in records {
        let (provider_id, scheme_family) = claim_segments
            .get(record.claim_id.as_str())
            .copied()
            .unwrap_or(("unknown", "unknown"));
        let mut segments = vec![
            ("provider", provider_id.to_string()),
            ("scheme", scheme_family.to_string()),
        ];
        segments.extend(
            campaign_ids_from_evidence_refs(&record.evidence_refs)
                .into_iter()
                .map(|campaign_id| ("campaign", campaign_id)),
        );
        for (segment_type, segment_id) in segments {
            let key = (
                segment_type.to_string(),
                segment_id,
                record.currency.clone(),
            );
            let entry = accumulators
                .entry(key)
                .or_insert((Decimal::ZERO, BTreeSet::new(), 0));
            entry.0 += record.saving_amount;
            entry.1.insert(record.claim_id.clone());
            entry.2 += 1;
        }
    }

    accumulators
        .into_iter()
        .map(
            |((segment_type, segment_id, currency), (saving_amount, claims, attribution_count))| {
                let claim_count = claims.len() as u32;
                DashboardSavingSegmentRecord {
                    segment_type,
                    segment_id,
                    saving_amount: format_decimal_cents(saving_amount),
                    currency,
                    claim_count,
                    attribution_count,
                    roi: segment_roi(saving_amount, claim_count),
                }
            },
        )
        .collect()
}

fn campaign_ids_from_evidence_refs(evidence_refs: &[String]) -> BTreeSet<String> {
    evidence_refs
        .iter()
        .filter_map(|reference| {
            reference
                .strip_prefix("campaigns:")
                .or_else(|| reference.strip_prefix("campaign:"))
        })
        .map(str::trim)
        .filter(|campaign_id| !campaign_id.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub(super) fn segment_roi(saving_amount: Decimal, claim_count: u32) -> f64 {
    if claim_count == 0 {
        return 0.0;
    }
    let review_cost = claim_count as f64 * RULE_REVIEW_COST_AMOUNT;
    if review_cost == 0.0 {
        0.0
    } else {
        decimal_to_f64(&saving_amount) / review_cost
    }
}

pub(super) fn format_decimal_cents(value: Decimal) -> String {
    format!("{:.2}", value.round_dp(2))
}

fn sanitize_identifier(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

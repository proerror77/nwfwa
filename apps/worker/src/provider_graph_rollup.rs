use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use crate::{api_url, read_json_report, required_non_empty, write_json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderGraphRollupInput {
    pub as_of_date: String,
    #[serde(default)]
    pub claims: Vec<ProviderGraphClaimInput>,
    #[serde(default)]
    pub referrals: Vec<ProviderReferralInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderGraphClaimInput {
    pub claim_id: String,
    pub provider_id: String,
    pub member_id: String,
    pub service_day: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderReferralInput {
    pub provider_id: String,
    pub referring_provider_id: String,
    pub referral_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderGraphSignalRollup {
    pub provider_id: String,
    pub billing_ring_membership: bool,
    pub temporal_co_billing_frequency_7d: f64,
    pub referral_concentration_entropy: Option<f64>,
    pub shared_member_provider_count: usize,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderGraphSignalRollupReport {
    pub report_kind: String,
    pub report_version: u8,
    pub as_of_date: String,
    pub source_uri: String,
    pub provider_count: usize,
    pub claim_count: usize,
    pub provider_relationships: Vec<ProviderGraphSignalRollup>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderGraphSignalRollupSubmission {
    pub actor: String,
    pub notes: String,
    pub source_report_uri: String,
    pub report_kind: String,
    pub as_of_date: String,
    pub source_uri: String,
    pub provider_count: usize,
    pub claim_count: usize,
    pub provider_relationships: Vec<ProviderGraphSignalRollup>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

pub fn build_provider_graph_signal_rollup(
    graph_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<ProviderGraphSignalRollupReport> {
    let input: ProviderGraphRollupInput = serde_json::from_value(read_json_report(graph_uri)?)
        .context("parse provider graph rollup input")?;
    if input.as_of_date.trim().is_empty() {
        bail!("provider graph rollup input requires as_of_date");
    }

    let mut claims_by_provider = BTreeMap::<String, Vec<ProviderGraphClaimInput>>::new();
    for claim in input.claims {
        if claim.provider_id.trim().is_empty() {
            bail!(
                "provider graph claim {} missing provider_id",
                claim.claim_id
            );
        }
        if claim.member_id.trim().is_empty() {
            bail!("provider graph claim {} missing member_id", claim.claim_id);
        }
        claims_by_provider
            .entry(claim.provider_id.trim().into())
            .or_default()
            .push(claim);
    }
    let referral_entropy = referral_entropy_by_provider(&input.referrals);
    let shared_member_counts = shared_member_provider_counts(&claims_by_provider);

    let provider_relationships = claims_by_provider
        .iter()
        .map(|(provider_id, claims)| {
            let shared_member_provider_count =
                shared_member_counts.get(provider_id).copied().unwrap_or(0);
            ProviderGraphSignalRollup {
                provider_id: provider_id.clone(),
                billing_ring_membership: shared_member_provider_count >= 1,
                temporal_co_billing_frequency_7d: temporal_co_billing_frequency(
                    provider_id,
                    claims,
                    &claims_by_provider,
                ),
                referral_concentration_entropy: referral_entropy
                    .get(provider_id)
                    .copied()
                    .flatten(),
                shared_member_provider_count,
                evidence_refs: vec![format!("provider_graph_rollups:{provider_id}")],
            }
        })
        .collect::<Vec<_>>();

    let report = ProviderGraphSignalRollupReport {
        report_kind: "provider_graph_signal_rollup".into(),
        report_version: 1,
        as_of_date: input.as_of_date,
        source_uri: graph_uri.into(),
        provider_count: provider_relationships.len(),
        claim_count: provider_relationships
            .iter()
            .filter_map(|relationship| claims_by_provider.get(&relationship.provider_id))
            .map(Vec::len)
            .sum(),
        provider_relationships,
        evidence_refs: vec![format!("provider_graph_claim_snapshot:{graph_uri}")],
        governance_boundary: "rollup computes provider graph signals only; it must not assign fraud labels, open cases, or change scoring/routing policy".into(),
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create provider graph rollup output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("provider_graph_signal_rollup.json"),
        &report,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("provider_relationship_inputs.json"),
        &report.provider_relationships,
    )?;
    Ok(report)
}

pub fn build_provider_graph_signal_rollup_submission(
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<ProviderGraphSignalRollupSubmission> {
    let report_uri = required_non_empty("report_uri", report_uri)?;
    let actor = required_non_empty("actor", actor)?;
    let notes = required_non_empty("notes", notes)?;
    let report: ProviderGraphSignalRollupReport =
        serde_json::from_value(read_json_report(report_uri)?)
            .context("parse provider graph signal rollup report")?;
    if report.report_kind != "provider_graph_signal_rollup" {
        bail!("report_kind must be provider_graph_signal_rollup");
    }
    if report.provider_relationships.is_empty() {
        bail!("provider graph signal rollup requires provider_relationships before API submission");
    }
    let mut evidence_refs = report.evidence_refs;
    evidence_refs.push(format!("provider_graph_signal_rollups:{report_uri}"));
    evidence_refs.sort();
    evidence_refs.dedup();
    Ok(ProviderGraphSignalRollupSubmission {
        actor: actor.into(),
        notes: notes.into(),
        source_report_uri: report_uri.into(),
        report_kind: report.report_kind,
        as_of_date: report.as_of_date,
        source_uri: report.source_uri,
        provider_count: report.provider_count,
        claim_count: report.claim_count,
        provider_relationships: report.provider_relationships,
        evidence_refs,
        governance_boundary: report.governance_boundary,
    })
}

pub async fn submit_provider_graph_signal_rollup(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    let payload = build_provider_graph_signal_rollup_submission(report_uri, actor, notes)?;
    let response = reqwest::Client::new()
        .post(api_url(
            api_base_url,
            "/api/v1/ops/providers/graph-signal-rollups",
        ))
        .header("x-api-key", api_key)
        .json(&payload)
        .send()
        .await
        .context("submit provider graph signal rollup")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("submit provider graph signal rollup failed with {status}: {body}");
    }
    response
        .json::<serde_json::Value>()
        .await
        .context("parse provider graph signal rollup response")
}

fn temporal_co_billing_frequency(
    provider_id: &str,
    claims: &[ProviderGraphClaimInput],
    claims_by_provider: &BTreeMap<String, Vec<ProviderGraphClaimInput>>,
) -> f64 {
    if claims.is_empty() {
        return 0.0;
    }
    let co_billed = claims
        .iter()
        .filter(|claim| {
            claims_by_provider
                .iter()
                .filter(|(other_provider_id, _)| other_provider_id.as_str() != provider_id)
                .flat_map(|(_, other_claims)| other_claims.iter())
                .any(|other_claim| {
                    other_claim.member_id == claim.member_id
                        && (other_claim.service_day - claim.service_day).abs() <= 7
                })
        })
        .count();
    co_billed as f64 / claims.len() as f64
}

fn shared_member_provider_counts(
    claims_by_provider: &BTreeMap<String, Vec<ProviderGraphClaimInput>>,
) -> BTreeMap<String, usize> {
    let members_by_provider = claims_by_provider
        .iter()
        .map(|(provider_id, claims)| {
            (
                provider_id.clone(),
                claims
                    .iter()
                    .map(|claim| claim.member_id.clone())
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    members_by_provider
        .iter()
        .map(|(provider_id, members)| {
            let count = members_by_provider
                .iter()
                .filter(|(other_provider_id, other_members)| {
                    other_provider_id != &provider_id
                        && members.intersection(other_members).count() >= 2
                })
                .count();
            (provider_id.clone(), count)
        })
        .collect()
}

fn referral_entropy_by_provider(
    referrals: &[ProviderReferralInput],
) -> BTreeMap<String, Option<f64>> {
    let mut counts = BTreeMap::<String, Vec<u32>>::new();
    for referral in referrals {
        if referral.provider_id.trim().is_empty()
            || referral.referring_provider_id.trim().is_empty()
            || referral.referral_count == 0
        {
            continue;
        }
        counts
            .entry(referral.provider_id.trim().into())
            .or_default()
            .push(referral.referral_count);
    }
    counts
        .into_iter()
        .map(|(provider_id, counts)| (provider_id, normalized_entropy(&counts)))
        .collect()
}

fn normalized_entropy(counts: &[u32]) -> Option<f64> {
    if counts.len() < 2 {
        return Some(0.0);
    }
    let total = counts.iter().sum::<u32>() as f64;
    if total == 0.0 {
        return None;
    }
    let entropy = counts
        .iter()
        .map(|count| *count as f64 / total)
        .filter(|probability| *probability > 0.0)
        .map(|probability| -probability * probability.ln())
        .sum::<f64>();
    Some(entropy / (counts.len() as f64).ln())
}

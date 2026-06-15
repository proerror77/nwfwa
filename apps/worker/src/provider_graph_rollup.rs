use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use crate::{
    api_url, ensure_production_artifact_uri, ensure_production_evidence_refs,
    published_submission_evidence_refs, read_json_report, required_non_empty, write_json,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderGraphRollupInput {
    pub as_of_date: String,
    #[serde(default)]
    pub claims: Vec<ProviderGraphClaimInput>,
    #[serde(default)]
    pub referrals: Vec<ProviderReferralInput>,
    #[serde(default)]
    pub provider_risks: Vec<ProviderRiskInput>,
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
pub struct ProviderRiskInput {
    pub provider_id: String,
    pub high_risk: bool,
    #[serde(default)]
    pub confirmed_fwa_count: u32,
    pub network_component_risk_score: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderGraphSignalRollup {
    pub provider_id: String,
    pub high_risk_neighbor_ratio: Option<f64>,
    pub provider_patient_overlap_score: Option<f64>,
    pub referral_concentration_score: Option<f64>,
    pub billing_ring_membership: bool,
    pub temporal_co_billing_frequency_7d: f64,
    pub referral_concentration_entropy: Option<f64>,
    pub shared_member_provider_count: usize,
    pub connected_confirmed_fwa_count: Option<u32>,
    pub network_component_risk_score: Option<u8>,
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
        if claim.claim_id.trim().is_empty() {
            bail!("provider graph claim missing claim_id");
        }
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
    let provider_risks = input
        .provider_risks
        .into_iter()
        .filter(|risk| !risk.provider_id.trim().is_empty())
        .map(|risk| (risk.provider_id.trim().to_string(), risk))
        .collect::<BTreeMap<_, _>>();
    let shared_member_counts = shared_member_provider_counts(&claims_by_provider);

    let provider_relationships = claims_by_provider
        .iter()
        .map(|(provider_id, claims)| {
            let shared_member_provider_count =
                shared_member_counts.get(provider_id).copied().unwrap_or(0);
            let neighbor_ids = shared_member_neighbor_ids(provider_id, claims, &claims_by_provider);
            let high_risk_neighbor_ratio = high_risk_neighbor_ratio(&neighbor_ids, &provider_risks);
            let connected_confirmed_fwa_count =
                connected_confirmed_fwa_count(&neighbor_ids, &provider_risks);
            let referral_concentration_entropy =
                referral_entropy.get(provider_id).copied().flatten();
            ProviderGraphSignalRollup {
                provider_id: provider_id.clone(),
                high_risk_neighbor_ratio,
                provider_patient_overlap_score: Some(provider_patient_overlap_score(
                    provider_id,
                    claims,
                    &claims_by_provider,
                )),
                referral_concentration_score: referral_concentration_entropy
                    .map(|entropy| (1.0 - entropy).clamp(0.0, 1.0)),
                billing_ring_membership: shared_member_provider_count >= 1,
                temporal_co_billing_frequency_7d: temporal_co_billing_frequency(
                    provider_id,
                    claims,
                    &claims_by_provider,
                ),
                referral_concentration_entropy,
                shared_member_provider_count,
                connected_confirmed_fwa_count,
                network_component_risk_score: provider_risks
                    .get(provider_id)
                    .and_then(|risk| risk.network_component_risk_score),
                evidence_refs: provider_graph_evidence_refs(provider_id, claims, &neighbor_ids),
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
    let report: ProviderGraphSignalRollupReport =
        serde_json::from_value(read_json_report(report_uri)?)
            .context("parse provider graph signal rollup report")?;
    let published_source_uri = report.source_uri.clone();
    build_provider_graph_signal_rollup_submission_from_report(
        report_uri,
        &published_source_uri,
        actor,
        notes,
        report,
    )
}

pub fn build_provider_graph_signal_rollup_submission_with_published_uris(
    report_uri: &str,
    published_report_uri: &str,
    published_source_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<ProviderGraphSignalRollupSubmission> {
    let report_uri = required_non_empty("report_uri", report_uri)?;
    let published_report_uri = required_non_empty("published_report_uri", published_report_uri)?;
    let published_source_uri = required_non_empty("published_source_uri", published_source_uri)?;
    let report: ProviderGraphSignalRollupReport =
        serde_json::from_value(read_json_report(report_uri)?)
            .context("parse provider graph signal rollup report")?;
    build_provider_graph_signal_rollup_submission_from_report(
        published_report_uri,
        published_source_uri,
        actor,
        notes,
        report,
    )
}

fn build_provider_graph_signal_rollup_submission_from_report(
    published_report_uri: &str,
    published_source_uri: &str,
    actor: &str,
    notes: &str,
    report: ProviderGraphSignalRollupReport,
) -> anyhow::Result<ProviderGraphSignalRollupSubmission> {
    let actor = required_non_empty("actor", actor)?;
    let notes = required_non_empty("notes", notes)?;
    if report.report_kind != "provider_graph_signal_rollup" {
        bail!("report_kind must be provider_graph_signal_rollup");
    }
    if report.provider_relationships.is_empty() {
        bail!("provider graph signal rollup requires provider_relationships before API submission");
    }
    ensure_production_artifact_uri("provider graph published_report_uri", published_report_uri)?;
    ensure_production_artifact_uri("provider graph published_source_uri", published_source_uri)?;
    for relationship in &report.provider_relationships {
        ensure_production_evidence_refs(
            "provider graph record evidence_refs",
            &relationship.evidence_refs,
        )?;
    }
    let evidence_refs = published_submission_evidence_refs(
        "provider graph evidence_refs",
        &report.evidence_refs,
        "provider_graph_claim_snapshot",
        &report.source_uri,
        published_source_uri,
        "provider_graph_signal_rollups",
        published_report_uri,
    )?;
    Ok(ProviderGraphSignalRollupSubmission {
        actor: actor.into(),
        notes: notes.into(),
        source_report_uri: published_report_uri.into(),
        report_kind: report.report_kind,
        as_of_date: report.as_of_date,
        source_uri: published_source_uri.into(),
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
    submit_provider_graph_signal_rollup_payload(api_base_url, api_key, &payload).await
}

pub async fn submit_provider_graph_signal_rollup_with_published_uris(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    published_report_uri: &str,
    published_source_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    let payload = build_provider_graph_signal_rollup_submission_with_published_uris(
        report_uri,
        published_report_uri,
        published_source_uri,
        actor,
        notes,
    )?;
    submit_provider_graph_signal_rollup_payload(api_base_url, api_key, &payload).await
}

async fn submit_provider_graph_signal_rollup_payload(
    api_base_url: &str,
    api_key: &str,
    payload: &ProviderGraphSignalRollupSubmission,
) -> anyhow::Result<serde_json::Value> {
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

fn provider_graph_evidence_refs(
    provider_id: &str,
    claims: &[ProviderGraphClaimInput],
    neighbor_ids: &[String],
) -> Vec<String> {
    let mut evidence_refs = BTreeSet::from([format!("provider_graph_rollups:{provider_id}")]);
    evidence_refs.extend(
        claims
            .iter()
            .map(|claim| format!("claims:{}", claim.claim_id.trim())),
    );
    evidence_refs.extend(
        neighbor_ids
            .iter()
            .map(|neighbor_id| format!("provider_graph_neighbor:{neighbor_id}")),
    );
    evidence_refs.into_iter().collect()
}

fn shared_member_neighbor_ids(
    provider_id: &str,
    claims: &[ProviderGraphClaimInput],
    claims_by_provider: &BTreeMap<String, Vec<ProviderGraphClaimInput>>,
) -> Vec<String> {
    let members = claims
        .iter()
        .map(|claim| claim.member_id.clone())
        .collect::<BTreeSet<_>>();
    claims_by_provider
        .iter()
        .filter(|(other_provider_id, _)| other_provider_id.as_str() != provider_id)
        .filter(|(_, other_claims)| {
            other_claims
                .iter()
                .any(|claim| members.contains(&claim.member_id))
        })
        .map(|(other_provider_id, _)| other_provider_id.clone())
        .collect()
}

fn high_risk_neighbor_ratio(
    neighbor_ids: &[String],
    provider_risks: &BTreeMap<String, ProviderRiskInput>,
) -> Option<f64> {
    if neighbor_ids.is_empty()
        || neighbor_ids
            .iter()
            .any(|provider_id| !provider_risks.contains_key(provider_id))
    {
        return None;
    }
    let high_risk_count = neighbor_ids
        .iter()
        .filter(|provider_id| {
            provider_risks
                .get(provider_id.as_str())
                .is_some_and(|risk| risk.high_risk)
        })
        .count();
    Some(high_risk_count as f64 / neighbor_ids.len() as f64)
}

fn connected_confirmed_fwa_count(
    neighbor_ids: &[String],
    provider_risks: &BTreeMap<String, ProviderRiskInput>,
) -> Option<u32> {
    if neighbor_ids
        .iter()
        .any(|provider_id| !provider_risks.contains_key(provider_id))
    {
        return None;
    }
    Some(
        neighbor_ids
            .iter()
            .filter_map(|provider_id| provider_risks.get(provider_id.as_str()))
            .map(|risk| risk.confirmed_fwa_count)
            .sum(),
    )
}

fn provider_patient_overlap_score(
    provider_id: &str,
    claims: &[ProviderGraphClaimInput],
    claims_by_provider: &BTreeMap<String, Vec<ProviderGraphClaimInput>>,
) -> f64 {
    let members = claims
        .iter()
        .map(|claim| claim.member_id.clone())
        .collect::<BTreeSet<_>>();
    if members.is_empty() {
        return 0.0;
    }
    claims_by_provider
        .iter()
        .filter(|(other_provider_id, _)| other_provider_id.as_str() != provider_id)
        .map(|(_, other_claims)| {
            let other_members = other_claims
                .iter()
                .map(|claim| claim.member_id.clone())
                .collect::<BTreeSet<_>>();
            members.intersection(&other_members).count() as f64 / members.len() as f64
        })
        .fold(0.0, f64::max)
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

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use crate::{
    api_url, ensure_production_artifact_uri, ensure_production_evidence_refs,
    ensure_production_json_artifact_uri, published_submission_evidence_refs, read_json_report,
    required_non_empty, write_json,
};

const EPISODE_WINDOWS: [u16; 3] = [30, 90, 365];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeRollupInput {
    pub as_of_date: String,
    #[serde(default)]
    pub claims: Vec<EpisodeClaimInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeClaimInput {
    pub claim_id: String,
    pub member_id: String,
    pub provider_id: String,
    pub service_age_days: u16,
    pub claim_amount: f64,
    #[serde(default)]
    pub procedure_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeWindowRollup {
    pub window_days: u16,
    pub claim_count: usize,
    pub total_claim_amount: f64,
    pub unique_procedure_code_count: usize,
    pub max_procedure_code_frequency: usize,
    pub duplicate_amount_day_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemberProviderEpisodeRollup {
    pub member_id: String,
    pub provider_id: String,
    pub episode_key: String,
    pub windows: Vec<EpisodeWindowRollup>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeAggregationReport {
    pub report_kind: String,
    pub report_version: u8,
    pub as_of_date: String,
    pub source_uri: String,
    pub episode_count: usize,
    pub claim_count: usize,
    pub windows: Vec<u16>,
    pub episodes: Vec<MemberProviderEpisodeRollup>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeAggregationSubmission {
    pub actor: String,
    pub notes: String,
    pub source_report_uri: String,
    pub report_kind: String,
    pub as_of_date: String,
    pub source_uri: String,
    pub episode_count: usize,
    pub claim_count: usize,
    pub episodes: Vec<MemberProviderEpisodeRollup>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

pub fn build_episode_aggregation_report(
    claims_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<EpisodeAggregationReport> {
    let input: EpisodeRollupInput = serde_json::from_value(read_json_report(claims_uri)?)
        .context("parse episode aggregation input")?;
    if input.as_of_date.trim().is_empty() {
        bail!("episode aggregation input requires as_of_date");
    }
    let claim_count = input.claims.len();
    let mut claims_by_episode = BTreeMap::<String, Vec<EpisodeClaimInput>>::new();
    for claim in input.claims {
        if claim.member_id.trim().is_empty() {
            bail!("episode claim {} missing member_id", claim.claim_id);
        }
        if claim.provider_id.trim().is_empty() {
            bail!("episode claim {} missing provider_id", claim.claim_id);
        }
        if claim.claim_amount < 0.0 || !claim.claim_amount.is_finite() {
            bail!("episode claim {} has invalid claim_amount", claim.claim_id);
        }
        claims_by_episode
            .entry(episode_key(&claim.member_id, &claim.provider_id))
            .or_default()
            .push(claim);
    }

    let episodes = claims_by_episode
        .into_iter()
        .map(|(episode_key, claims)| episode_rollup(episode_key, claims))
        .collect::<Vec<_>>();
    let report = EpisodeAggregationReport {
        report_kind: "member_provider_episode_aggregation".into(),
        report_version: 1,
        as_of_date: input.as_of_date,
        source_uri: claims_uri.into(),
        episode_count: episodes.len(),
        claim_count,
        windows: EPISODE_WINDOWS.into(),
        episodes,
        evidence_refs: vec![format!("episode_claim_snapshot:{claims_uri}")],
        governance_boundary: "episode aggregation computes member-provider utilization evidence only; it must not assign fraud labels, deny claims, or write rules".into(),
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create episode aggregation output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir.as_ref().join("episode_aggregation_report.json"),
        &report,
    )?;
    write_json(
        output_dir.as_ref().join("episode_rollups.json"),
        &report.episodes,
    )?;
    Ok(report)
}

pub fn build_episode_aggregation_submission(
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<EpisodeAggregationSubmission> {
    let report_uri = required_non_empty("report_uri", report_uri)?;
    let report: EpisodeAggregationReport = serde_json::from_value(read_json_report(report_uri)?)
        .context("parse episode aggregation report")?;
    let published_source_uri = report.source_uri.clone();
    build_episode_aggregation_submission_from_report(
        report_uri,
        &published_source_uri,
        actor,
        notes,
        report,
    )
}

pub fn build_episode_aggregation_submission_with_published_uris(
    report_uri: &str,
    published_report_uri: &str,
    published_source_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<EpisodeAggregationSubmission> {
    let report_uri = required_non_empty("report_uri", report_uri)?;
    let published_report_uri = required_non_empty("published_report_uri", published_report_uri)?;
    let published_source_uri = required_non_empty("published_source_uri", published_source_uri)?;
    let report: EpisodeAggregationReport = serde_json::from_value(read_json_report(report_uri)?)
        .context("parse episode aggregation report")?;
    build_episode_aggregation_submission_from_report(
        published_report_uri,
        published_source_uri,
        actor,
        notes,
        report,
    )
}

fn build_episode_aggregation_submission_from_report(
    published_report_uri: &str,
    published_source_uri: &str,
    actor: &str,
    notes: &str,
    report: EpisodeAggregationReport,
) -> anyhow::Result<EpisodeAggregationSubmission> {
    let actor = required_non_empty("actor", actor)?;
    let notes = required_non_empty("notes", notes)?;
    if report.report_kind != "member_provider_episode_aggregation" {
        bail!("report_kind must be member_provider_episode_aggregation");
    }
    if report.episodes.is_empty() {
        bail!("episode aggregation requires episodes before API submission");
    }
    ensure_production_json_artifact_uri(
        "episode aggregation published_report_uri",
        published_report_uri,
    )?;
    ensure_production_artifact_uri(
        "episode aggregation published_source_uri",
        published_source_uri,
    )?;
    for episode in &report.episodes {
        ensure_production_evidence_refs("episode rollup evidence_refs", &episode.evidence_refs)?;
    }
    let evidence_refs = published_submission_evidence_refs(
        "episode aggregation evidence_refs",
        &report.evidence_refs,
        "episode_claim_snapshot",
        &report.source_uri,
        published_source_uri,
        "episode_rollups",
        published_report_uri,
    )?;
    Ok(EpisodeAggregationSubmission {
        actor: actor.into(),
        notes: notes.into(),
        source_report_uri: published_report_uri.into(),
        report_kind: report.report_kind,
        as_of_date: report.as_of_date,
        source_uri: published_source_uri.into(),
        episode_count: report.episode_count,
        claim_count: report.claim_count,
        episodes: report.episodes,
        evidence_refs,
        governance_boundary: report.governance_boundary,
    })
}

pub async fn submit_episode_aggregation(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    let payload = build_episode_aggregation_submission(report_uri, actor, notes)?;
    submit_episode_aggregation_payload(api_base_url, api_key, &payload).await
}

pub async fn submit_episode_aggregation_with_published_uris(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    published_report_uri: &str,
    published_source_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    let payload = build_episode_aggregation_submission_with_published_uris(
        report_uri,
        published_report_uri,
        published_source_uri,
        actor,
        notes,
    )?;
    submit_episode_aggregation_payload(api_base_url, api_key, &payload).await
}

async fn submit_episode_aggregation_payload(
    api_base_url: &str,
    api_key: &str,
    payload: &EpisodeAggregationSubmission,
) -> anyhow::Result<serde_json::Value> {
    let response = reqwest::Client::new()
        .post(api_url(
            api_base_url,
            "/api/v1/ops/providers/episode-rollups",
        ))
        .header("x-api-key", api_key)
        .json(&payload)
        .send()
        .await
        .context("submit episode aggregation")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("submit episode aggregation failed with {status}: {body}");
    }
    response
        .json::<serde_json::Value>()
        .await
        .context("parse episode aggregation response")
}

fn episode_rollup(
    episode_key: String,
    claims: Vec<EpisodeClaimInput>,
) -> MemberProviderEpisodeRollup {
    debug_assert!(
        !claims.is_empty(),
        "episode_rollup called with empty claims vec"
    );
    let first = &claims[0];
    let member_id = first.member_id.trim().to_string();
    let provider_id = first.provider_id.trim().to_string();
    let evidence_refs = claims
        .iter()
        .map(|claim| format!("claims:{}", claim.claim_id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let windows = EPISODE_WINDOWS
        .into_iter()
        .map(|window_days| episode_window(&claims, window_days))
        .collect();

    MemberProviderEpisodeRollup {
        member_id,
        provider_id,
        episode_key,
        windows,
        evidence_refs,
    }
}

fn episode_window(claims: &[EpisodeClaimInput], window_days: u16) -> EpisodeWindowRollup {
    let window_claims = claims
        .iter()
        .filter(|claim| claim.service_age_days <= window_days)
        .collect::<Vec<_>>();
    let mut code_counts = BTreeMap::<String, usize>::new();
    let mut amount_day_counts = BTreeMap::<String, usize>::new();
    for claim in &window_claims {
        for code in &claim.procedure_codes {
            let code = code.trim();
            if !code.is_empty() {
                *code_counts.entry(code.into()).or_default() += 1;
            }
        }
        let amount_day_key = format!("{:.2}|{}", claim.claim_amount, claim.service_age_days);
        *amount_day_counts.entry(amount_day_key).or_default() += 1;
    }

    EpisodeWindowRollup {
        window_days,
        claim_count: window_claims.len(),
        total_claim_amount: window_claims.iter().map(|claim| claim.claim_amount).sum(),
        unique_procedure_code_count: code_counts.len(),
        max_procedure_code_frequency: code_counts.values().copied().max().unwrap_or(0),
        duplicate_amount_day_count: amount_day_counts
            .values()
            .filter(|count| **count > 1)
            .map(|count| count - 1)
            .sum(),
    }
}

fn episode_key(member_id: &str, provider_id: &str) -> String {
    format!("{}|{}", member_id.trim(), provider_id.trim())
}

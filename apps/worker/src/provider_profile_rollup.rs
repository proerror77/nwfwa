use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use crate::{
    api_url, ensure_no_template_evidence_refs, ensure_no_template_uri, read_json_report,
    required_non_empty, write_json,
};

const PROFILE_WINDOWS: [u16; 3] = [30, 90, 365];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfileRollupInput {
    pub as_of_date: String,
    #[serde(default)]
    pub claims: Vec<ProviderProfileClaimInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfileClaimInput {
    pub claim_id: String,
    pub provider_id: String,
    pub service_age_days: u16,
    pub claim_amount: String,
    #[serde(default)]
    pub high_cost_item: bool,
    #[serde(default)]
    pub diagnosis_procedure_mismatch: bool,
    pub peer_amount_percentile: u8,
    pub peer_frequency_percentile: u8,
    pub review_outcome: Option<String>,
    pub specialty: Option<String>,
    pub network_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfileWindowOutput {
    pub window_days: u16,
    pub claim_count: u32,
    pub total_claim_amount: String,
    pub high_cost_item_ratio: f64,
    pub diagnosis_procedure_mismatch_rate: f64,
    pub peer_amount_percentile: u8,
    pub peer_frequency_percentile: u8,
    pub review_failure_count: u32,
    pub confirmed_fwa_count: u32,
    pub false_positive_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfileRollup {
    pub provider_id: String,
    pub specialty: Option<String>,
    pub network_status: Option<String>,
    pub windows: Vec<ProviderProfileWindowOutput>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfileWindowRollupReport {
    pub report_kind: String,
    pub report_version: u8,
    pub as_of_date: String,
    pub source_uri: String,
    pub provider_count: usize,
    pub claim_count: usize,
    pub windows: Vec<u16>,
    pub provider_profiles: Vec<ProviderProfileRollup>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfileWindowRollupSubmission {
    pub actor: String,
    pub notes: String,
    pub source_report_uri: String,
    pub report_kind: String,
    pub as_of_date: String,
    pub source_uri: String,
    pub provider_count: usize,
    pub claim_count: usize,
    pub provider_profiles: Vec<ProviderProfileRollup>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

pub fn build_provider_profile_window_rollup(
    claims_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<ProviderProfileWindowRollupReport> {
    let input: ProviderProfileRollupInput = serde_json::from_value(read_json_report(claims_uri)?)
        .context("parse provider profile rollup input")?;
    if input.as_of_date.trim().is_empty() {
        bail!("provider profile rollup input requires as_of_date");
    }

    let mut claims_by_provider = BTreeMap::<String, Vec<ProviderProfileClaimInput>>::new();
    for claim in input.claims {
        if claim.provider_id.trim().is_empty() {
            bail!(
                "provider profile claim {} missing provider_id",
                claim.claim_id
            );
        }
        validate_percentile("peer_amount_percentile", claim.peer_amount_percentile)?;
        validate_percentile("peer_frequency_percentile", claim.peer_frequency_percentile)?;
        parse_amount_cents(&claim.claim_amount)
            .with_context(|| format!("invalid claim_amount for {}", claim.claim_id))?;
        claims_by_provider
            .entry(claim.provider_id.trim().into())
            .or_default()
            .push(claim);
    }

    let provider_profiles = claims_by_provider
        .into_iter()
        .map(|(provider_id, claims)| provider_rollup(provider_id, claims))
        .collect::<anyhow::Result<Vec<_>>>()?;

    let report = ProviderProfileWindowRollupReport {
        report_kind: "provider_profile_window_rollup".into(),
        report_version: 1,
        as_of_date: input.as_of_date,
        source_uri: claims_uri.into(),
        provider_count: provider_profiles.len(),
        claim_count: provider_profiles
            .iter()
            .flat_map(|profile| profile.windows.iter())
            .filter(|window| window.window_days == 365)
            .map(|window| window.claim_count as usize)
            .sum(),
        windows: PROFILE_WINDOWS.into(),
        provider_profiles,
        evidence_refs: vec![format!("provider_profile_claim_snapshot:{claims_uri}")],
        governance_boundary: "rollup computes provider profile windows only; it must not assign fraud labels, change routing policy, or write provider sanctions".into(),
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create provider profile rollup output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("provider_profile_window_rollup_report.json"),
        &report,
    )?;
    write_json(
        output_dir.as_ref().join("provider_profile_windows.json"),
        &report.provider_profiles,
    )?;
    Ok(report)
}

pub fn build_provider_profile_window_rollup_submission(
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<ProviderProfileWindowRollupSubmission> {
    let report_uri = required_non_empty("report_uri", report_uri)?;
    let actor = required_non_empty("actor", actor)?;
    let notes = required_non_empty("notes", notes)?;
    let report: ProviderProfileWindowRollupReport =
        serde_json::from_value(read_json_report(report_uri)?)
            .context("parse provider profile window rollup report")?;
    if report.report_kind != "provider_profile_window_rollup" {
        bail!("report_kind must be provider_profile_window_rollup");
    }
    if report.provider_profiles.is_empty() {
        bail!("provider profile window rollup requires provider_profiles before API submission");
    }
    ensure_no_template_uri("provider profile source_uri", &report.source_uri)?;
    ensure_no_template_evidence_refs("provider profile evidence_refs", &report.evidence_refs)?;
    for profile in &report.provider_profiles {
        ensure_no_template_evidence_refs(
            "provider profile record evidence_refs",
            &profile.evidence_refs,
        )?;
    }
    let required_ref = format!("provider_profile_claim_snapshot:{}", report.source_uri);
    if !report
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == required_ref)
    {
        bail!("provider profile window rollup requires {required_ref} evidence");
    }
    let mut evidence_refs = report.evidence_refs;
    evidence_refs.push(format!("provider_profile_window_rollups:{report_uri}"));
    evidence_refs.sort();
    evidence_refs.dedup();
    Ok(ProviderProfileWindowRollupSubmission {
        actor: actor.into(),
        notes: notes.into(),
        source_report_uri: report_uri.into(),
        report_kind: report.report_kind,
        as_of_date: report.as_of_date,
        source_uri: report.source_uri,
        provider_count: report.provider_count,
        claim_count: report.claim_count,
        provider_profiles: report.provider_profiles,
        evidence_refs,
        governance_boundary: report.governance_boundary,
    })
}

pub async fn submit_provider_profile_window_rollup(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    let payload = build_provider_profile_window_rollup_submission(report_uri, actor, notes)?;
    let response = reqwest::Client::new()
        .post(api_url(
            api_base_url,
            "/api/v1/ops/providers/profile-window-rollups",
        ))
        .header("x-api-key", api_key)
        .json(&payload)
        .send()
        .await
        .context("submit provider profile window rollup")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("submit provider profile window rollup failed with {status}: {body}");
    }
    response
        .json::<serde_json::Value>()
        .await
        .context("parse provider profile window rollup response")
}

fn provider_rollup(
    provider_id: String,
    claims: Vec<ProviderProfileClaimInput>,
) -> anyhow::Result<ProviderProfileRollup> {
    let specialty = first_non_empty(claims.iter().filter_map(|claim| claim.specialty.as_deref()));
    let network_status = first_non_empty(
        claims
            .iter()
            .filter_map(|claim| claim.network_status.as_deref()),
    );
    let evidence_refs = claims
        .iter()
        .map(|claim| format!("claims:{}", claim.claim_id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let windows = PROFILE_WINDOWS
        .into_iter()
        .map(|window_days| provider_window(&claims, window_days))
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(ProviderProfileRollup {
        provider_id,
        specialty,
        network_status,
        windows,
        evidence_refs,
    })
}

fn provider_window(
    claims: &[ProviderProfileClaimInput],
    window_days: u16,
) -> anyhow::Result<ProviderProfileWindowOutput> {
    let window_claims = claims
        .iter()
        .filter(|claim| claim.service_age_days <= window_days)
        .collect::<Vec<_>>();
    let claim_count = window_claims.len() as u32;
    let total_claim_amount_cents = window_claims.iter().try_fold(0_i64, |sum, claim| {
        parse_amount_cents(&claim.claim_amount)
            .map(|amount| sum + amount)
            .with_context(|| format!("invalid claim_amount for {}", claim.claim_id))
    })?;

    Ok(ProviderProfileWindowOutput {
        window_days,
        claim_count,
        total_claim_amount: format_cents(total_claim_amount_cents),
        high_cost_item_ratio: ratio_count(&window_claims, |claim| claim.high_cost_item),
        diagnosis_procedure_mismatch_rate: ratio_count(&window_claims, |claim| {
            claim.diagnosis_procedure_mismatch
        }),
        peer_amount_percentile: window_claims
            .iter()
            .map(|claim| claim.peer_amount_percentile)
            .max()
            .unwrap_or(0),
        peer_frequency_percentile: window_claims
            .iter()
            .map(|claim| claim.peer_frequency_percentile)
            .max()
            .unwrap_or(0),
        review_failure_count: outcome_count(&window_claims, "review_failure"),
        confirmed_fwa_count: outcome_count(&window_claims, "confirmed_fwa"),
        false_positive_count: outcome_count(&window_claims, "false_positive"),
    })
}

fn ratio_count(
    claims: &[&ProviderProfileClaimInput],
    predicate: impl Fn(&ProviderProfileClaimInput) -> bool,
) -> f64 {
    if claims.is_empty() {
        return 0.0;
    }
    let matching = claims.iter().filter(|claim| predicate(claim)).count();
    matching as f64 / claims.len() as f64
}

fn outcome_count(claims: &[&ProviderProfileClaimInput], outcome: &str) -> u32 {
    claims
        .iter()
        .filter(|claim| claim.review_outcome.as_deref() == Some(outcome))
        .count() as u32
}

fn first_non_empty<'a>(values: impl Iterator<Item = &'a str>) -> Option<String> {
    values
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(str::to_string)
}

fn validate_percentile(field: &str, value: u8) -> anyhow::Result<()> {
    if value > 100 {
        bail!("{field} must be between 0 and 100");
    }
    Ok(())
}

fn parse_amount_cents(value: &str) -> anyhow::Result<i64> {
    let value = value.trim();
    if value.is_empty() {
        bail!("amount is empty");
    }
    let negative = value.starts_with('-');
    let value = value.trim_start_matches('-');
    let (whole, fractional) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty() || !whole.chars().all(|character| character.is_ascii_digit()) {
        bail!("amount whole component is invalid");
    }
    if fractional.len() > 2
        || !fractional
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        bail!("amount fractional component is invalid");
    }
    let whole_cents = whole.parse::<i64>()? * 100;
    let fractional_cents = match fractional.len() {
        0 => 0,
        1 => fractional.parse::<i64>()? * 10,
        2 => fractional.parse::<i64>()?,
        _ => unreachable!(),
    };
    let cents = whole_cents + fractional_cents;
    Ok(if negative { -cents } else { cents })
}

fn format_cents(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let absolute = cents.abs();
    format!("{sign}{}.{:02}", absolute / 100, absolute % 100)
}

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, path::Path};

use crate::{api_url, read_json_report, required_non_empty, write_json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerBenchmarkInput {
    pub benchmark_month: String,
    #[serde(default)]
    pub claims: Vec<PeerBenchmarkClaimInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerBenchmarkClaimInput {
    pub claim_id: String,
    pub specialty: String,
    pub region: String,
    pub service_segment: String,
    pub claim_amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerBenchmarkGroup {
    pub peer_group_key: String,
    pub specialty: String,
    pub region: String,
    pub service_segment: String,
    pub claim_count: usize,
    pub p25: f64,
    pub p50: f64,
    pub p75: f64,
    pub p90: f64,
    pub p99: f64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerBenchmarkReport {
    pub report_kind: String,
    pub report_version: u8,
    pub benchmark_month: String,
    pub source_uri: String,
    pub claim_count: usize,
    pub peer_group_count: usize,
    pub peer_groups: Vec<PeerBenchmarkGroup>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerBenchmarkSubmission {
    pub actor: String,
    pub notes: String,
    pub source_report_uri: String,
    pub report_kind: String,
    pub benchmark_month: String,
    pub source_uri: String,
    pub claim_count: usize,
    pub peer_group_count: usize,
    pub peer_groups: Vec<PeerBenchmarkGroup>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

pub fn build_peer_percentile_benchmark(
    claims_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<PeerBenchmarkReport> {
    let input: PeerBenchmarkInput = serde_json::from_value(read_json_report(claims_uri)?)
        .context("parse peer benchmark input")?;
    if input.benchmark_month.trim().is_empty() {
        bail!("peer benchmark input requires benchmark_month");
    }
    let mut groups = BTreeMap::<String, (String, String, String, Vec<f64>)>::new();
    for claim in input.claims {
        if claim.claim_amount < 0.0 || !claim.claim_amount.is_finite() {
            bail!("claim {} has invalid claim_amount", claim.claim_id);
        }
        let specialty = required_segment("specialty", &claim.specialty, &claim.claim_id)?;
        let region = required_segment("region", &claim.region, &claim.claim_id)?;
        let service_segment =
            required_segment("service_segment", &claim.service_segment, &claim.claim_id)?;
        let key = peer_group_key(&specialty, &region, &service_segment);
        groups
            .entry(key)
            .or_insert_with(|| (specialty, region, service_segment, Vec::new()))
            .3
            .push(claim.claim_amount);
    }

    let peer_groups = groups
        .into_iter()
        .map(
            |(peer_group_key, (specialty, region, service_segment, mut amounts))| {
                amounts.sort_by(|left, right| left.total_cmp(right));
                PeerBenchmarkGroup {
                    peer_group_key: peer_group_key.clone(),
                    specialty,
                    region,
                    service_segment,
                    claim_count: amounts.len(),
                    p25: percentile(&amounts, 0.25),
                    p50: percentile(&amounts, 0.50),
                    p75: percentile(&amounts, 0.75),
                    p90: percentile(&amounts, 0.90),
                    p99: percentile(&amounts, 0.99),
                    evidence_refs: vec![format!("peer_benchmark_groups:{peer_group_key}")],
                }
            },
        )
        .collect::<Vec<_>>();

    let report = PeerBenchmarkReport {
        report_kind: "peer_percentile_benchmark".into(),
        report_version: 1,
        benchmark_month: input.benchmark_month,
        source_uri: claims_uri.into(),
        claim_count: peer_groups.iter().map(|group| group.claim_count).sum(),
        peer_group_count: peer_groups.len(),
        peer_groups,
        evidence_refs: vec![format!("peer_benchmark_claim_snapshot:{claims_uri}")],
        governance_boundary: "benchmark computes peer percentile reference data only; it must not score claims, assign labels, or change routing policy".into(),
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create peer benchmark output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir.as_ref().join("peer_percentile_benchmark.json"),
        &report,
    )?;
    write_json(
        output_dir.as_ref().join("peer_benchmark_groups.json"),
        &report.peer_groups,
    )?;
    Ok(report)
}

pub fn build_peer_benchmark_submission(
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<PeerBenchmarkSubmission> {
    let report_uri = required_non_empty("report_uri", report_uri)?;
    let actor = required_non_empty("actor", actor)?;
    let notes = required_non_empty("notes", notes)?;
    let report: PeerBenchmarkReport = serde_json::from_value(read_json_report(report_uri)?)
        .context("parse peer benchmark report")?;
    if report.report_kind != "peer_percentile_benchmark" {
        bail!("report_kind must be peer_percentile_benchmark");
    }
    if report.peer_groups.is_empty() {
        bail!("peer benchmark requires peer_groups before API submission");
    }
    let mut evidence_refs = report.evidence_refs;
    evidence_refs.push(format!("peer_benchmarks:{report_uri}"));
    evidence_refs.sort();
    evidence_refs.dedup();
    Ok(PeerBenchmarkSubmission {
        actor: actor.into(),
        notes: notes.into(),
        source_report_uri: report_uri.into(),
        report_kind: report.report_kind,
        benchmark_month: report.benchmark_month,
        source_uri: report.source_uri,
        claim_count: report.claim_count,
        peer_group_count: report.peer_group_count,
        peer_groups: report.peer_groups,
        evidence_refs,
        governance_boundary: report.governance_boundary,
    })
}

pub async fn submit_peer_benchmark(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    let payload = build_peer_benchmark_submission(report_uri, actor, notes)?;
    let response = reqwest::Client::new()
        .post(api_url(
            api_base_url,
            "/api/v1/ops/providers/peer-benchmarks",
        ))
        .header("x-api-key", api_key)
        .json(&payload)
        .send()
        .await
        .context("submit peer benchmark")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("submit peer benchmark failed with {status}: {body}");
    }
    response
        .json::<serde_json::Value>()
        .await
        .context("parse peer benchmark response")
}

fn percentile(sorted_values: &[f64], percentile: f64) -> f64 {
    if sorted_values.is_empty() {
        return 0.0;
    }
    let index = ((sorted_values.len() - 1) as f64 * percentile).round() as usize;
    sorted_values[index]
}

fn required_segment(field: &str, value: &str, claim_id: &str) -> anyhow::Result<String> {
    let value = value.trim();
    if value.is_empty() {
        bail!("claim {claim_id} missing {field}");
    }
    Ok(value.into())
}

fn peer_group_key(specialty: &str, region: &str, service_segment: &str) -> String {
    format!("{specialty}|{region}|{service_segment}")
}

use anyhow::{bail, Context};
use fwa_features::{
    ClinicalCompatibilityFeatureContext, EpisodeUtilizationFeatureContext, PeerFeatureContext,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use crate::{
    api_url, read_json_report, required_non_empty, write_json, ClinicalCompatibilityRecord,
    EpisodeAggregationReport, MemberProviderEpisodeRollup, PeerBenchmarkGroup, PeerBenchmarkReport,
    UnbundlingComparatorCandidate, UnbundlingComparatorReport,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringFeatureContextInput {
    pub as_of_date: String,
    #[serde(default)]
    pub claims: Vec<ScoringFeatureContextClaimInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringFeatureContextClaimInput {
    pub claim_id: String,
    pub member_id: String,
    pub provider_id: String,
    pub claim_amount: f64,
    pub specialty: String,
    pub region: String,
    pub service_segment: String,
    pub diagnosis_code: String,
    #[serde(default)]
    pub procedure_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringFeatureContextSourceUris {
    pub claims_uri: String,
    pub episode_rollups_uri: String,
    pub peer_benchmarks_uri: String,
    pub clinical_compatibility_uri: String,
    pub unbundling_candidates_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimScoringFeatureContext {
    pub claim_id: String,
    pub member_id: String,
    pub provider_id: String,
    pub peer_context: Option<PeerFeatureContext>,
    pub clinical_compatibility_context: Option<ClinicalCompatibilityFeatureContext>,
    pub episode_utilization_context: Option<EpisodeUtilizationFeatureContext>,
    pub evidence_refs: Vec<String>,
    pub data_sources: Vec<String>,
    pub missing_contexts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringFeatureContextReport {
    pub report_kind: String,
    pub report_version: u8,
    pub as_of_date: String,
    pub source_uris: ScoringFeatureContextSourceUris,
    pub claim_count: usize,
    pub context_count: usize,
    pub contexts: Vec<ClaimScoringFeatureContext>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringFeatureContextMaterializationSubmission {
    pub materialization_id: String,
    pub actor: String,
    pub notes: String,
    pub report_uri: String,
    pub report_kind: String,
    pub as_of_date: String,
    pub source_uris: serde_json::Value,
    pub claim_count: usize,
    pub context_count: usize,
    pub contexts: Vec<serde_json::Value>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

pub fn build_scoring_feature_context_report(
    claims_uri: &str,
    episode_rollups_uri: &str,
    peer_benchmarks_uri: &str,
    clinical_compatibility_uri: &str,
    unbundling_candidates_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<ScoringFeatureContextReport> {
    let input: ScoringFeatureContextInput = serde_json::from_value(read_json_report(claims_uri)?)
        .context("parse scoring feature context input")?;
    if input.as_of_date.trim().is_empty() {
        bail!("scoring feature context input requires as_of_date");
    }
    let episodes_by_key = load_episode_rollups(episode_rollups_uri)?;
    let peer_groups_by_key = load_peer_groups(peer_benchmarks_uri)?;
    let clinical_records = load_clinical_records(clinical_compatibility_uri)?;
    let unbundling_counts_by_episode = load_unbundling_counts(unbundling_candidates_uri)?;

    let contexts = input
        .claims
        .iter()
        .map(|claim| {
            materialize_claim_context(
                claim,
                &episodes_by_key,
                &peer_groups_by_key,
                &clinical_records,
                &unbundling_counts_by_episode,
            )
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let evidence_refs = contexts
        .iter()
        .flat_map(|context| context.evidence_refs.iter().cloned())
        .chain([
            format!("scoring_feature_context_claim_snapshot:{claims_uri}"),
            format!("episode_rollups:{episode_rollups_uri}"),
            format!("peer_benchmarks:{peer_benchmarks_uri}"),
            format!("clinical_compatibility:{clinical_compatibility_uri}"),
            format!("unbundling_candidates:{unbundling_candidates_uri}"),
        ])
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let report = ScoringFeatureContextReport {
        report_kind: "scoring_feature_context_materialization".into(),
        report_version: 1,
        as_of_date: input.as_of_date,
        source_uris: ScoringFeatureContextSourceUris {
            claims_uri: claims_uri.into(),
            episode_rollups_uri: episode_rollups_uri.into(),
            peer_benchmarks_uri: peer_benchmarks_uri.into(),
            clinical_compatibility_uri: clinical_compatibility_uri.into(),
            unbundling_candidates_uri: unbundling_candidates_uri.into(),
        },
        claim_count: input.claims.len(),
        context_count: contexts.len(),
        contexts,
        evidence_refs,
        governance_boundary: "materialized scoring feature contexts map governed worker artifacts into online scoring inputs only; they must not assign fraud labels, deny claims, or write production state without an approved API/repository integration".into(),
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create scoring feature context output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("scoring_feature_context_report.json"),
        &report,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("claim_scoring_feature_contexts.json"),
        &report.contexts,
    )?;
    Ok(report)
}

pub fn build_scoring_feature_context_materialization_submission(
    report_uri: &str,
    materialization_id: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<ScoringFeatureContextMaterializationSubmission> {
    let report_uri = required_non_empty("report_uri", report_uri)?;
    let materialization_id = required_non_empty("materialization_id", materialization_id)?;
    let actor = required_non_empty("actor", actor)?;
    let notes = required_non_empty("notes", notes)?;
    let report: ScoringFeatureContextReport = serde_json::from_value(read_json_report(report_uri)?)
        .context("parse scoring feature context materialization report")?;
    if report.report_kind != "scoring_feature_context_materialization" {
        bail!("report_kind must be scoring_feature_context_materialization");
    }
    if report.context_count != report.contexts.len() {
        bail!("context_count must match contexts length");
    }
    if report.context_count > report.claim_count {
        bail!("context_count must not exceed claim_count");
    }
    if report.evidence_refs.is_empty() {
        bail!("scoring feature context materialization requires evidence_refs");
    }

    Ok(ScoringFeatureContextMaterializationSubmission {
        materialization_id: materialization_id.into(),
        actor: actor.into(),
        notes: notes.into(),
        report_uri: report_uri.into(),
        report_kind: report.report_kind,
        as_of_date: report.as_of_date,
        source_uris: serde_json::to_value(report.source_uris)?,
        claim_count: report.claim_count,
        context_count: report.context_count,
        contexts: report
            .contexts
            .into_iter()
            .map(serde_json::to_value)
            .collect::<Result<Vec<_>, _>>()?,
        evidence_refs: report.evidence_refs,
        governance_boundary: report.governance_boundary,
    })
}

pub async fn submit_scoring_feature_context_materialization(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    materialization_id: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    let payload = build_scoring_feature_context_materialization_submission(
        report_uri,
        materialization_id,
        actor,
        notes,
    )?;
    let response = reqwest::Client::new()
        .post(api_url(
            api_base_url,
            "/api/v1/ops/scoring-feature-context-materializations",
        ))
        .header("x-api-key", api_key)
        .json(&payload)
        .send()
        .await
        .context("submit scoring feature context materialization")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("submit scoring feature context materialization failed with {status}: {body}");
    }
    response
        .json::<serde_json::Value>()
        .await
        .context("parse scoring feature context materialization response")
}

fn materialize_claim_context(
    claim: &ScoringFeatureContextClaimInput,
    episodes_by_key: &BTreeMap<String, MemberProviderEpisodeRollup>,
    peer_groups_by_key: &BTreeMap<String, PeerBenchmarkGroup>,
    clinical_records: &[ClinicalCompatibilityRecord],
    unbundling_counts_by_episode: &BTreeMap<String, u32>,
) -> anyhow::Result<ClaimScoringFeatureContext> {
    validate_claim(claim)?;
    let episode_key = episode_key(&claim.member_id, &claim.provider_id);
    let mut evidence_refs = BTreeSet::from([format!("claims:{}", claim.claim_id)]);
    let mut data_sources = BTreeSet::new();
    let mut missing_contexts = BTreeSet::new();

    let peer_key = peer_group_key(&claim.specialty, &claim.region, &claim.service_segment);
    let peer_context = peer_groups_by_key.get(&peer_key).map(|group| {
        evidence_refs.extend(group.evidence_refs.iter().cloned());
        data_sources.insert("worker.peer_percentile_benchmark_rollup".into());
        PeerFeatureContext {
            claim_amount_peer_percentile: Some(claim_amount_percentile(claim.claim_amount, group)),
        }
    });
    if peer_context.is_none() {
        missing_contexts.insert("claim_amount_peer_percentile".into());
    }

    let clinical_compatibility_context =
        clinical_context_for_claim(claim, clinical_records).map(|(record, score)| {
            evidence_refs.extend(record.evidence_refs.iter().cloned());
            evidence_refs.insert(record.policy_authority_ref.clone());
            data_sources.insert(record.data_source.clone());
            ClinicalCompatibilityFeatureContext {
                diagnosis_procedure_match_score: Some(score),
                data_source: Some(record.data_source.clone()),
            }
        });
    if clinical_compatibility_context.is_none() {
        missing_contexts.insert("diagnosis_procedure_match_score".into());
    }

    let episode = episodes_by_key.get(&episode_key);
    let unbundling_count = unbundling_counts_by_episode
        .get(&episode_key)
        .copied()
        .unwrap_or(0);
    let episode_utilization_context = episode.map(|episode| {
        evidence_refs.extend(episode.evidence_refs.iter().cloned());
        data_sources.insert("worker.episode_utilization_rollup".into());
        let window_30 = episode
            .windows
            .iter()
            .find(|window| window.window_days == 30);
        EpisodeUtilizationFeatureContext {
            member_provider_claim_count_30d: window_30.map(|window| window.claim_count as u32),
            duplicate_claim_similarity_score: window_30.map(duplicate_similarity_score),
            procedure_frequency_peer_percentile: None,
            unbundling_candidate_count: Some(unbundling_count),
            data_source: Some("worker.episode_utilization_rollup".into()),
        }
    });
    if episode_utilization_context.is_none() {
        missing_contexts.insert("episode_utilization_context".into());
    }
    if unbundling_count > 0 {
        data_sources.insert("worker.unbundling_comparator".into());
        evidence_refs.insert(format!("unbundling:{episode_key}"));
    }

    Ok(ClaimScoringFeatureContext {
        claim_id: claim.claim_id.trim().into(),
        member_id: claim.member_id.trim().into(),
        provider_id: claim.provider_id.trim().into(),
        peer_context,
        clinical_compatibility_context,
        episode_utilization_context,
        evidence_refs: evidence_refs.into_iter().collect(),
        data_sources: data_sources.into_iter().collect(),
        missing_contexts: missing_contexts.into_iter().collect(),
    })
}

fn validate_claim(claim: &ScoringFeatureContextClaimInput) -> anyhow::Result<()> {
    if claim.claim_id.trim().is_empty() {
        bail!("scoring feature context claim missing claim_id");
    }
    if claim.member_id.trim().is_empty() {
        bail!(
            "scoring feature context claim {} missing member_id",
            claim.claim_id
        );
    }
    if claim.provider_id.trim().is_empty() {
        bail!(
            "scoring feature context claim {} missing provider_id",
            claim.claim_id
        );
    }
    if claim.claim_amount < 0.0 || !claim.claim_amount.is_finite() {
        bail!(
            "scoring feature context claim {} has invalid claim_amount",
            claim.claim_id
        );
    }
    if claim.specialty.trim().is_empty() {
        bail!(
            "scoring feature context claim {} missing specialty",
            claim.claim_id
        );
    }
    if claim.region.trim().is_empty() {
        bail!(
            "scoring feature context claim {} missing region",
            claim.claim_id
        );
    }
    if claim.service_segment.trim().is_empty() {
        bail!(
            "scoring feature context claim {} missing service_segment",
            claim.claim_id
        );
    }
    Ok(())
}

fn load_episode_rollups(
    uri: &str,
) -> anyhow::Result<BTreeMap<String, MemberProviderEpisodeRollup>> {
    let value = read_json_report(uri)?;
    let episodes = if value.get("report_kind").and_then(|value| value.as_str())
        == Some("member_provider_episode_aggregation")
    {
        serde_json::from_value::<EpisodeAggregationReport>(value)
            .context("parse episode aggregation report")?
            .episodes
    } else {
        serde_json::from_value(value).context("parse episode rollups")?
    };
    Ok(episodes
        .into_iter()
        .map(|episode| (episode.episode_key.clone(), episode))
        .collect())
}

fn load_peer_groups(uri: &str) -> anyhow::Result<BTreeMap<String, PeerBenchmarkGroup>> {
    let value = read_json_report(uri)?;
    let peer_groups = if value.get("report_kind").and_then(|value| value.as_str())
        == Some("peer_percentile_benchmark")
    {
        serde_json::from_value::<PeerBenchmarkReport>(value)
            .context("parse peer benchmark report")?
            .peer_groups
    } else {
        serde_json::from_value(value).context("parse peer benchmark groups")?
    };
    Ok(peer_groups
        .into_iter()
        .map(|group| (group.peer_group_key.clone(), group))
        .collect())
}

fn load_clinical_records(uri: &str) -> anyhow::Result<Vec<ClinicalCompatibilityRecord>> {
    let value = read_json_report(uri)?;
    if value.get("report_kind").and_then(|value| value.as_str())
        == Some("clinical_compatibility_reference")
    {
        Ok(value
            .get("records")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .context("parse clinical compatibility records")?
            .unwrap_or_default())
    } else {
        serde_json::from_value(value).context("parse clinical compatibility records")
    }
}

fn load_unbundling_counts(uri: &str) -> anyhow::Result<BTreeMap<String, u32>> {
    let value = read_json_report(uri)?;
    let candidates = if value.get("report_kind").and_then(|value| value.as_str())
        == Some("unbundling_comparator")
    {
        serde_json::from_value::<UnbundlingComparatorReport>(value)
            .context("parse unbundling comparator report")?
            .candidates
    } else {
        serde_json::from_value::<Vec<UnbundlingComparatorCandidate>>(value)
            .context("parse unbundling candidates")?
    };
    let mut counts = BTreeMap::<String, u32>::new();
    for candidate in candidates {
        *counts.entry(candidate.episode_key).or_default() += 1;
    }
    Ok(counts)
}

fn clinical_context_for_claim<'a>(
    claim: &ScoringFeatureContextClaimInput,
    records: &'a [ClinicalCompatibilityRecord],
) -> Option<(&'a ClinicalCompatibilityRecord, f64)> {
    let diagnosis_code = claim.diagnosis_code.trim().to_ascii_uppercase();
    let procedure_codes = claim
        .procedure_codes
        .iter()
        .map(|code| code.trim().to_ascii_uppercase())
        .collect::<BTreeSet<_>>();
    records
        .iter()
        .filter(|record| diagnosis_code.starts_with(&record.diagnosis_code_prefix))
        .filter(|record| procedure_codes.contains(&record.procedure_code))
        .min_by(|left, right| {
            left.diagnosis_procedure_match_score
                .total_cmp(&right.diagnosis_procedure_match_score)
        })
        .map(|record| (record, record.diagnosis_procedure_match_score))
}

fn claim_amount_percentile(claim_amount: f64, group: &PeerBenchmarkGroup) -> u8 {
    if claim_amount <= group.p25 {
        25
    } else if claim_amount <= group.p50 {
        50
    } else if claim_amount <= group.p75 {
        75
    } else if claim_amount <= group.p90 {
        90
    } else if claim_amount <= group.p99 {
        99
    } else {
        100
    }
}

fn duplicate_similarity_score(window: &crate::EpisodeWindowRollup) -> f64 {
    if window.claim_count <= 1 {
        return 0.0;
    }
    (window.duplicate_amount_day_count as f64 / (window.claim_count - 1) as f64).clamp(0.0, 1.0)
}

fn episode_key(member_id: &str, provider_id: &str) -> String {
    format!("{}|{}", member_id.trim(), provider_id.trim())
}

fn peer_group_key(specialty: &str, region: &str, service_segment: &str) -> String {
    format!(
        "{}|{}|{}",
        specialty.trim(),
        region.trim(),
        service_segment.trim()
    )
}

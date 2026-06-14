use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};

use crate::{api_url, read_json_report, required_non_empty, write_json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnbundlingComparatorInput {
    pub as_of_date: String,
    #[serde(default)]
    pub rules: Vec<UnbundlingRuleInput>,
    #[serde(default)]
    pub episodes: Vec<UnbundlingEpisodeInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnbundlingRuleInput {
    pub rule_id: String,
    pub bundled_code: String,
    #[serde(default)]
    pub component_codes: Vec<String>,
    pub policy_authority_ref: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnbundlingEpisodeInput {
    pub episode_key: String,
    pub member_id: String,
    pub provider_id: String,
    pub window_days: u16,
    #[serde(default)]
    pub claim_ids: Vec<String>,
    #[serde(default)]
    pub procedure_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnbundlingComparatorCandidate {
    pub candidate_id: String,
    pub rule_id: String,
    pub episode_key: String,
    pub member_id: String,
    pub provider_id: String,
    pub window_days: u16,
    pub bundled_code: String,
    pub matched_component_codes: Vec<String>,
    pub claim_ids: Vec<String>,
    pub policy_authority_ref: String,
    pub evidence_refs: Vec<String>,
    pub recommended_review: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnbundlingComparatorReport {
    pub report_kind: String,
    pub report_version: u8,
    pub as_of_date: String,
    pub source_uri: String,
    pub rule_count: usize,
    pub episode_count: usize,
    pub candidate_count: usize,
    pub candidates: Vec<UnbundlingComparatorCandidate>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnbundlingComparatorSubmission {
    pub actor: String,
    pub notes: String,
    pub source_report_uri: String,
    pub report_kind: String,
    pub as_of_date: String,
    pub source_uri: String,
    pub rule_count: usize,
    pub episode_count: usize,
    pub candidate_count: usize,
    pub candidates: Vec<UnbundlingComparatorCandidate>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

pub fn build_unbundling_comparator_report(
    input_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<UnbundlingComparatorReport> {
    let input: UnbundlingComparatorInput = serde_json::from_value(read_json_report(input_uri)?)
        .context("parse unbundling comparator input")?;
    if input.as_of_date.trim().is_empty() {
        bail!("unbundling comparator input requires as_of_date");
    }
    let rules = normalize_rules(input.rules)?;
    let episodes = normalize_episodes(input.episodes)?;
    let candidates = build_candidates(&rules, &episodes);
    let report = UnbundlingComparatorReport {
        report_kind: "unbundling_comparator".into(),
        report_version: 1,
        as_of_date: input.as_of_date,
        source_uri: input_uri.into(),
        rule_count: rules.len(),
        episode_count: episodes.len(),
        candidate_count: candidates.len(),
        candidates,
        evidence_refs: vec![format!("unbundling_comparator_input:{input_uri}")],
        governance_boundary: "unbundling comparator emits medical-review candidates from governed bundled/component code references; it must not assign fraud labels or deny claims".into(),
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create unbundling comparator output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("unbundling_comparator_report.json"),
        &report,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("unbundling_comparator_candidates.json"),
        &report.candidates,
    )?;
    Ok(report)
}

pub fn build_unbundling_comparator_submission(
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<UnbundlingComparatorSubmission> {
    let report_uri = required_non_empty("report_uri", report_uri)?;
    let actor = required_non_empty("actor", actor)?;
    let notes = required_non_empty("notes", notes)?;
    let report: UnbundlingComparatorReport = serde_json::from_value(read_json_report(report_uri)?)
        .context("parse unbundling comparator report")?;
    if report.report_kind != "unbundling_comparator" {
        bail!("report_kind must be unbundling_comparator");
    }
    if report.candidates.is_empty() {
        bail!("unbundling comparator requires candidates before API submission");
    }
    let required_ref = format!("unbundling_comparator_input:{}", report.source_uri);
    if !report
        .evidence_refs
        .iter()
        .any(|reference| reference.trim() == required_ref)
    {
        bail!("unbundling comparator requires {required_ref} evidence");
    }
    let mut evidence_refs = report.evidence_refs;
    evidence_refs.push(format!("unbundling_comparator_candidates:{report_uri}"));
    evidence_refs.sort();
    evidence_refs.dedup();
    Ok(UnbundlingComparatorSubmission {
        actor: actor.into(),
        notes: notes.into(),
        source_report_uri: report_uri.into(),
        report_kind: report.report_kind,
        as_of_date: report.as_of_date,
        source_uri: report.source_uri,
        rule_count: report.rule_count,
        episode_count: report.episode_count,
        candidate_count: report.candidate_count,
        candidates: report.candidates,
        evidence_refs,
        governance_boundary: report.governance_boundary,
    })
}

pub async fn submit_unbundling_comparator_candidates(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    let payload = build_unbundling_comparator_submission(report_uri, actor, notes)?;
    let response = reqwest::Client::new()
        .post(api_url(
            api_base_url,
            "/api/v1/ops/unbundling-comparator-candidates",
        ))
        .header("x-api-key", api_key)
        .json(&payload)
        .send()
        .await
        .context("submit unbundling comparator candidates")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("submit unbundling comparator candidates failed with {status}: {body}");
    }
    response
        .json::<serde_json::Value>()
        .await
        .context("parse unbundling comparator response")
}

fn normalize_rules(rules: Vec<UnbundlingRuleInput>) -> anyhow::Result<Vec<UnbundlingRuleInput>> {
    let mut normalized = Vec::with_capacity(rules.len());
    let mut seen = BTreeSet::new();
    for mut rule in rules {
        rule.rule_id = rule.rule_id.trim().to_string();
        rule.bundled_code = normalize_code(&rule.bundled_code);
        rule.policy_authority_ref = rule.policy_authority_ref.trim().to_string();
        rule.component_codes = rule
            .component_codes
            .into_iter()
            .map(|code| normalize_code(&code))
            .filter(|code| !code.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        rule.evidence_refs = normalize_refs(rule.evidence_refs);
        if rule.rule_id.is_empty() {
            bail!("unbundling rule missing rule_id");
        }
        if rule.bundled_code.is_empty() {
            bail!("unbundling rule {} missing bundled_code", rule.rule_id);
        }
        if rule.component_codes.is_empty() {
            bail!("unbundling rule {} requires component_codes", rule.rule_id);
        }
        if rule.policy_authority_ref.is_empty() {
            bail!(
                "unbundling rule {} missing policy_authority_ref",
                rule.rule_id
            );
        }
        if rule.evidence_refs.is_empty() {
            bail!("unbundling rule {} requires evidence_refs", rule.rule_id);
        }
        if !rule
            .evidence_refs
            .iter()
            .any(|reference| reference == &rule.policy_authority_ref)
        {
            rule.evidence_refs.push(rule.policy_authority_ref.clone());
            rule.evidence_refs.sort();
            rule.evidence_refs.dedup();
        }
        if !seen.insert(rule.rule_id.clone()) {
            bail!("duplicate unbundling rule {}", rule.rule_id);
        }
        normalized.push(rule);
    }
    Ok(normalized)
}

fn normalize_episodes(
    episodes: Vec<UnbundlingEpisodeInput>,
) -> anyhow::Result<Vec<UnbundlingEpisodeInput>> {
    let mut normalized = Vec::with_capacity(episodes.len());
    for mut episode in episodes {
        episode.episode_key = episode.episode_key.trim().to_string();
        episode.member_id = episode.member_id.trim().to_string();
        episode.provider_id = episode.provider_id.trim().to_string();
        episode.claim_ids = normalize_refs(episode.claim_ids);
        episode.procedure_codes = episode
            .procedure_codes
            .into_iter()
            .map(|code| normalize_code(&code))
            .filter(|code| !code.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        if episode.episode_key.is_empty() {
            bail!("unbundling episode missing episode_key");
        }
        if episode.member_id.is_empty() {
            bail!(
                "unbundling episode {} missing member_id",
                episode.episode_key
            );
        }
        if episode.provider_id.is_empty() {
            bail!(
                "unbundling episode {} missing provider_id",
                episode.episode_key
            );
        }
        normalized.push(episode);
    }
    Ok(normalized)
}

fn build_candidates(
    rules: &[UnbundlingRuleInput],
    episodes: &[UnbundlingEpisodeInput],
) -> Vec<UnbundlingComparatorCandidate> {
    let mut candidates = Vec::new();
    for episode in episodes {
        let episode_codes = episode.procedure_codes.iter().collect::<BTreeSet<_>>();
        for rule in rules {
            if !episode_codes.contains(&rule.bundled_code) {
                continue;
            }
            let matched_component_codes = rule
                .component_codes
                .iter()
                .filter(|code| episode_codes.contains(code))
                .cloned()
                .collect::<Vec<_>>();
            if matched_component_codes.is_empty() {
                continue;
            }
            let mut evidence_refs = rule.evidence_refs.clone();
            evidence_refs.extend(
                episode
                    .claim_ids
                    .iter()
                    .map(|claim_id| format!("claims:{claim_id}")),
            );
            evidence_refs.sort();
            evidence_refs.dedup();
            candidates.push(UnbundlingComparatorCandidate {
                candidate_id: candidate_id(&rule.rule_id, &episode.episode_key),
                rule_id: rule.rule_id.clone(),
                episode_key: episode.episode_key.clone(),
                member_id: episode.member_id.clone(),
                provider_id: episode.provider_id.clone(),
                window_days: episode.window_days,
                bundled_code: rule.bundled_code.clone(),
                matched_component_codes,
                claim_ids: episode.claim_ids.clone(),
                policy_authority_ref: rule.policy_authority_ref.clone(),
                evidence_refs,
                recommended_review: "medical_review_candidate".into(),
            });
        }
    }
    candidates.sort_by(|left, right| {
        (left.episode_key.as_str(), left.rule_id.as_str())
            .cmp(&(right.episode_key.as_str(), right.rule_id.as_str()))
    });
    candidates
}

fn candidate_id(rule_id: &str, episode_key: &str) -> String {
    format!("unbundling:{rule_id}:{episode_key}")
}

fn normalize_code(code: &str) -> String {
    code.trim().to_ascii_uppercase()
}

fn normalize_refs(refs: Vec<String>) -> Vec<String> {
    refs.into_iter()
        .map(|reference| reference.trim().to_string())
        .filter(|reference| !reference.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};

use crate::{api_url, read_json_report, required_non_empty, write_json};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringReadbackInput {
    pub customer_scope_id: String,
    pub as_of_date: String,
    pub score_request_uri: String,
    #[serde(default)]
    pub score_response_uri: Option<String>,
    #[serde(default)]
    pub expected_evidence_prefixes: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringReadbackCheck {
    pub expected_evidence_prefix: String,
    pub matched: bool,
    pub matched_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringReadbackReviewTask {
    pub task_kind: String,
    pub customer_scope_id: String,
    pub review_queue: String,
    pub blockers: Vec<String>,
    pub required_review: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringReadbackReport {
    pub report_kind: String,
    pub report_version: u8,
    pub customer_scope_id: String,
    pub as_of_date: String,
    pub readback_status: String,
    pub execution_mode: String,
    pub input_uri: String,
    pub score_request_uri: String,
    pub score_response_uri: Option<String>,
    pub expected_evidence_prefix_count: usize,
    pub matched_evidence_prefix_count: usize,
    pub checks: Vec<ScoringReadbackCheck>,
    pub observed_evidence_refs: Vec<String>,
    pub blockers: Vec<String>,
    pub review_task_count: usize,
    pub review_tasks: Vec<ScoringReadbackReviewTask>,
    pub evidence_refs: Vec<String>,
    pub governance_boundary: String,
}

pub async fn fetch_scoring_readback_response(
    api_base_url: &str,
    api_key: &str,
    score_request_uri: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<serde_json::Value> {
    let api_base_url = required_non_empty("api_base_url", api_base_url)?;
    let api_key = required_non_empty("api_key", api_key)?;
    let score_request_uri = required_non_empty("score_request_uri", score_request_uri)?;
    let score_request = read_json_report(score_request_uri)
        .with_context(|| format!("read scoring readback request {score_request_uri}"))?;
    let response = reqwest::Client::new()
        .post(api_url(api_base_url, "/api/v1/claims/score"))
        .header("x-api-key", api_key)
        .json(&score_request)
        .send()
        .await
        .context("submit scoring readback request")?
        .error_for_status()
        .context("scoring readback request failed")?
        .json::<serde_json::Value>()
        .await
        .context("parse scoring readback response")?;

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create scoring readback response output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(output_dir.as_ref().join("score_response.json"), &response)?;
    Ok(response)
}

pub fn build_scoring_readback_report(
    input_uri: &str,
    score_response_uri_override: Option<&str>,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<ScoringReadbackReport> {
    let input_uri = required_non_empty("input_uri", input_uri)?;
    let input: ScoringReadbackInput = serde_json::from_value(read_json_report(input_uri)?)
        .context("parse scoring readback input")?;
    let customer_scope_id = required_non_empty("customer_scope_id", &input.customer_scope_id)?;
    let as_of_date = required_non_empty("as_of_date", &input.as_of_date)?;
    let score_request_uri = required_non_empty("score_request_uri", &input.score_request_uri)?;
    let expected_prefixes = normalized_expected_prefixes(&input.expected_evidence_prefixes)?;
    let score_response_uri = score_response_uri_override
        .map(|value| required_non_empty("score_response_uri", value).map(str::to_string))
        .transpose()?
        .or_else(|| {
            input
                .score_response_uri
                .as_deref()
                .and_then(|value| required_non_empty("score_response_uri", value).ok())
                .map(str::to_string)
        });

    let mut blockers = Vec::new();
    let observed_evidence_refs = if let Some(score_response_uri) = score_response_uri.as_deref() {
        let response = read_json_report(score_response_uri)
            .with_context(|| format!("read scoring response {score_response_uri}"))?;
        collect_evidence_refs(&response)
    } else {
        blockers.push("score_response_uri_missing".to_string());
        Vec::new()
    };

    let checks = expected_prefixes
        .iter()
        .map(|prefix| {
            let matched_evidence_refs = observed_evidence_refs
                .iter()
                .filter(|reference| reference.starts_with(prefix))
                .cloned()
                .collect::<Vec<_>>();
            if matched_evidence_refs.is_empty() && score_response_uri.is_some() {
                blockers.push(format!("missing_expected_evidence_prefix:{prefix}"));
            }
            ScoringReadbackCheck {
                expected_evidence_prefix: prefix.clone(),
                matched: !matched_evidence_refs.is_empty(),
                matched_evidence_refs,
            }
        })
        .collect::<Vec<_>>();
    blockers.sort();
    blockers.dedup();
    let matched_evidence_prefix_count = checks.iter().filter(|check| check.matched).count();
    let readback_status = if blockers.is_empty() {
        "verified"
    } else {
        "blocked"
    };
    let execution_mode = if score_response_uri.is_some() {
        "score_response_artifact_readback"
    } else {
        "contract_only_blocked"
    };
    let review_tasks = if blockers.is_empty() {
        Vec::new()
    } else {
        vec![ScoringReadbackReviewTask {
            task_kind: "scoring_online_readback_review".into(),
            customer_scope_id: customer_scope_id.into(),
            review_queue: "worker_data_pipeline_ops".into(),
            blockers: blockers.clone(),
            required_review:
                "provide a scored response artifact whose evidence refs prove online scoring consumed governed worker writes"
                    .into(),
        }]
    };
    let mut evidence_refs = input.evidence_refs;
    evidence_refs.push(format!("scoring_readback_inputs:{input_uri}"));
    evidence_refs.push(format!(
        "scoring_readback_score_requests:{score_request_uri}"
    ));
    if let Some(score_response_uri) = &score_response_uri {
        evidence_refs.push(format!(
            "scoring_readback_score_responses:{score_response_uri}"
        ));
    }
    evidence_refs.sort();
    evidence_refs.dedup();

    let report = ScoringReadbackReport {
        report_kind: "scoring_readback_report".into(),
        report_version: 1,
        customer_scope_id: customer_scope_id.into(),
        as_of_date: as_of_date.into(),
        readback_status: readback_status.into(),
        execution_mode: execution_mode.into(),
        input_uri: input_uri.into(),
        score_request_uri: score_request_uri.into(),
        score_response_uri,
        expected_evidence_prefix_count: expected_prefixes.len(),
        matched_evidence_prefix_count,
        checks,
        observed_evidence_refs,
        blockers,
        review_task_count: review_tasks.len(),
        review_tasks,
        evidence_refs,
        governance_boundary: "scoring readback reports validate that governed worker artifacts are visible in online scoring responses only; they must not score claims live, assign fraud labels, deny claims, activate models, or change routing policy".into(),
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create scoring readback output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir.as_ref().join("scoring_readback_report.json"),
        &report,
    )?;
    Ok(report)
}

fn normalized_expected_prefixes(prefixes: &[String]) -> anyhow::Result<Vec<String>> {
    if prefixes.is_empty() {
        bail!("expected_evidence_prefixes is required");
    }
    let mut normalized = Vec::new();
    for prefix in prefixes {
        let prefix = required_non_empty("expected_evidence_prefix", prefix)?;
        if !prefix.ends_with(':') {
            bail!("expected_evidence_prefix must end with ':': {prefix}");
        }
        normalized.push(prefix.to_string());
    }
    normalized.sort();
    normalized.dedup();
    Ok(normalized)
}

fn collect_evidence_refs(value: &serde_json::Value) -> Vec<String> {
    let mut refs = BTreeSet::new();
    collect_evidence_refs_inner(value, &mut refs);
    refs.into_iter().collect()
}

fn collect_evidence_refs_inner(value: &serde_json::Value, refs: &mut BTreeSet<String>) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                collect_evidence_refs_inner(value, refs);
            }
        }
        serde_json::Value::Object(object) => {
            if let Some(evidence_refs) = object
                .get("evidence_refs")
                .and_then(|value| value.as_array())
            {
                for evidence_ref in evidence_refs {
                    if let Some(evidence_ref) = evidence_ref.as_str() {
                        let evidence_ref = evidence_ref.trim();
                        if !evidence_ref.is_empty() {
                            refs.insert(evidence_ref.to_string());
                        }
                    }
                }
            }
            for value in object.values() {
                collect_evidence_refs_inner(value, refs);
            }
        }
        _ => {}
    }
}

use anyhow::{bail, Context};
use serde::Serialize;

use super::{
    api_url, ensure_production_evidence_refs, ensure_production_json_artifact_uri, json_string,
    read_json_report, required_non_empty,
};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AnomalyClusteringReportSubmission {
    pub actor: String,
    pub notes: String,
    pub source_report_uri: String,
    pub report_kind: String,
    pub dataset_key: String,
    pub dataset_version: String,
    pub label_policy: String,
    pub governance_boundary: String,
    pub review_tasks: Vec<AnomalyClusteringReviewTaskSubmission>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AnomalyClusteringReviewTaskSubmission {
    pub candidate_kind: String,
    pub candidate_id: String,
    pub task_kind: String,
    pub review_queue: String,
    pub required_review: String,
    pub decision_options: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub candidate_payload: serde_json::Value,
}

pub fn build_anomaly_clustering_report_submission(
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<AnomalyClusteringReportSubmission> {
    let report_uri = required_non_empty("report_uri", report_uri)?;
    let actor = required_non_empty("actor", actor)?;
    let notes = required_non_empty("notes", notes)?;
    let report = read_json_report(report_uri)?;
    build_anomaly_clustering_report_submission_from_report(report_uri, actor, notes, &report)
}

pub fn build_anomaly_clustering_report_submission_with_published_uri(
    report_uri: &str,
    actor: &str,
    notes: &str,
    published_report_uri: &str,
) -> anyhow::Result<AnomalyClusteringReportSubmission> {
    let report_uri = required_non_empty("report_uri", report_uri)?;
    let actor = required_non_empty("actor", actor)?;
    let notes = required_non_empty("notes", notes)?;
    let published_report_uri = required_non_empty("published_report_uri", published_report_uri)?;
    ensure_production_json_artifact_uri(
        "anomaly clustering published_report_uri",
        published_report_uri,
    )?;
    let report = read_json_report(report_uri)?;
    build_anomaly_clustering_report_submission_from_report(
        published_report_uri,
        actor,
        notes,
        &report,
    )
}

fn build_anomaly_clustering_report_submission_from_report(
    published_report_uri: &str,
    actor: &str,
    notes: &str,
    report: &serde_json::Value,
) -> anyhow::Result<AnomalyClusteringReportSubmission> {
    let report_kind = json_string(report, "report_kind")
        .filter(|value| {
            matches!(
                value.as_str(),
                "provider_peer_clustering"
                    | "provider_graph_community_clustering"
                    | "claim_entity_clustering"
            )
        })
        .context("report_kind must be provider_peer_clustering, provider_graph_community_clustering, or claim_entity_clustering")?;
    let dataset_key = json_string(report, "dataset_key")
        .filter(|value| !value.trim().is_empty())
        .context("anomaly clustering report requires dataset_key")?;
    let dataset_version = json_string(report, "dataset_version")
        .filter(|value| !value.trim().is_empty())
        .context("anomaly clustering report requires dataset_version")?;
    let label_policy = json_string(report, "label_policy")
        .filter(|value| !value.trim().is_empty())
        .context("anomaly clustering report requires label_policy")?;
    let governance_boundary = json_string(report, "governance_boundary")
        .filter(|value| !value.trim().is_empty())
        .context("anomaly clustering report requires governance_boundary")?;
    let mut evidence_refs = report
        .get("evidence_refs")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    evidence_refs.push(format!("anomaly_clustering_reports:{published_report_uri}"));
    evidence_refs.sort();
    evidence_refs.dedup();
    ensure_production_evidence_refs("anomaly clustering report evidence_refs", &evidence_refs)?;

    let review_tasks =
        anomaly_clustering_review_tasks_from_report(published_report_uri, &report_kind, report)?;
    if review_tasks.is_empty() {
        bail!(
            "anomaly clustering report requires anomaly_candidates for API review queue submission"
        );
    }

    Ok(AnomalyClusteringReportSubmission {
        actor: actor.into(),
        notes: notes.into(),
        source_report_uri: published_report_uri.into(),
        report_kind,
        dataset_key,
        dataset_version,
        label_policy,
        governance_boundary,
        review_tasks,
        evidence_refs,
    })
}

pub async fn submit_anomaly_clustering_report(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    actor: &str,
    notes: &str,
) -> anyhow::Result<serde_json::Value> {
    submit_anomaly_clustering_report_with_published_uri(
        api_base_url,
        api_key,
        report_uri,
        actor,
        notes,
        report_uri,
    )
    .await
}

pub async fn submit_anomaly_clustering_report_with_published_uri(
    api_base_url: &str,
    api_key: &str,
    report_uri: &str,
    actor: &str,
    notes: &str,
    published_report_uri: &str,
) -> anyhow::Result<serde_json::Value> {
    let payload = build_anomaly_clustering_report_submission_with_published_uri(
        report_uri,
        actor,
        notes,
        published_report_uri,
    )?;
    let response = reqwest::Client::new()
        .post(api_url(
            api_base_url,
            "/api/v1/ops/providers/anomaly-clustering-reports",
        ))
        .header("x-api-key", api_key)
        .json(&payload)
        .send()
        .await
        .context("submit anomaly clustering report")?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        bail!("submit anomaly clustering report failed with {status}: {body}");
    }
    response
        .json::<serde_json::Value>()
        .await
        .context("parse anomaly clustering report response")
}

fn anomaly_clustering_review_tasks_from_report(
    report_uri: &str,
    report_kind: &str,
    report: &serde_json::Value,
) -> anyhow::Result<Vec<AnomalyClusteringReviewTaskSubmission>> {
    let candidates = report
        .get("anomaly_candidates")
        .and_then(|value| value.as_array())
        .context("anomaly clustering report requires anomaly_candidates")?;
    let local_review_tasks = report
        .get("review_tasks")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    candidates
        .iter()
        .map(|candidate| {
            let local_task =
                matching_local_anomaly_review_task(report_kind, candidate, &local_review_tasks);
            anomaly_clustering_review_task_from_candidate(
                report_uri,
                report_kind,
                candidate,
                local_task,
            )
        })
        .collect()
}

fn anomaly_clustering_review_task_from_candidate(
    report_uri: &str,
    report_kind: &str,
    candidate: &serde_json::Value,
    local_task: Option<&serde_json::Value>,
) -> anyhow::Result<AnomalyClusteringReviewTaskSubmission> {
    let report_evidence_ref = format!("anomaly_clustering_reports:{report_uri}");
    let candidate_payload = candidate.clone();
    let candidate_evidence_refs = candidate
        .get("evidence_refs")
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string));
    let local_evidence_refs = local_task
        .and_then(|task| task.get("evidence_refs"))
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string));
    let mut evidence_refs = std::iter::once(report_evidence_ref)
        .chain(candidate_evidence_refs)
        .chain(local_evidence_refs)
        .collect::<Vec<_>>();
    evidence_refs.sort();
    evidence_refs.dedup();
    ensure_production_evidence_refs(
        "anomaly clustering review task evidence_refs",
        &evidence_refs,
    )?;

    let task = match report_kind {
        "provider_peer_clustering" => {
            let provider_id = required_json_string(candidate, "provider_id")?;
            let service_month = required_json_string(candidate, "service_month")?;
            AnomalyClusteringReviewTaskSubmission {
                candidate_kind: "provider_peer_anomaly".into(),
                candidate_id: format!("provider_peer:{provider_id}:{service_month}"),
                task_kind: "provider_peer_anomaly_review".into(),
                review_queue: "provider_anomaly_candidate_review".into(),
                required_review: "human_review_required_before_case_creation_or_label_assignment"
                    .into(),
                decision_options: anomaly_review_decision_options(
                    local_task,
                    &[
                        "dismiss_as_peer_variation",
                        "request_more_evidence",
                        "open_investigation_candidate",
                    ],
                ),
                evidence_refs,
                candidate_payload,
            }
        }
        "provider_graph_community_clustering" => {
            let provider_id = required_json_string(candidate, "provider_id")?;
            let community_id = required_json_i64(candidate, "community_id")?;
            AnomalyClusteringReviewTaskSubmission {
                candidate_kind: "provider_graph_anomaly".into(),
                candidate_id: format!("provider_graph:{provider_id}:{community_id}"),
                task_kind: "provider_graph_anomaly_review".into(),
                review_queue: "provider_graph_anomaly_candidate_review".into(),
                required_review: "human_review_required_before_case_creation_or_label_assignment"
                    .into(),
                decision_options: anomaly_review_decision_options(
                    local_task,
                    &[
                        "dismiss_as_network_variation",
                        "request_more_evidence",
                        "open_investigation_candidate",
                    ],
                ),
                evidence_refs,
                candidate_payload,
            }
        }
        "claim_entity_clustering" => {
            let claim_id = required_json_string(candidate, "claim_id")?;
            AnomalyClusteringReviewTaskSubmission {
                candidate_kind: "claim_entity_anomaly".into(),
                candidate_id: format!("claim_entity:{claim_id}"),
                task_kind: "claim_entity_anomaly_review".into(),
                review_queue: "claim_entity_anomaly_candidate_review".into(),
                required_review:
                    "human_review_required_before_case_creation_label_assignment_or_rule_writeback"
                        .into(),
                decision_options: anomaly_review_decision_options(
                    local_task,
                    &[
                        "dismiss_as_entity_variation",
                        "request_more_evidence",
                        "open_investigation_candidate",
                        "prepare_rule_candidate_backtest",
                    ],
                ),
                evidence_refs,
                candidate_payload,
            }
        }
        other => bail!("unsupported anomaly clustering report_kind {other}"),
    };
    Ok(task)
}

fn matching_local_anomaly_review_task<'a>(
    report_kind: &str,
    candidate: &serde_json::Value,
    tasks: &'a [serde_json::Value],
) -> Option<&'a serde_json::Value> {
    tasks.iter().find(|task| match report_kind {
        "provider_peer_clustering" => task.get("provider_id") == candidate.get("provider_id"),
        "provider_graph_community_clustering" => {
            task.get("provider_id") == candidate.get("provider_id")
                && task.get("community_id") == candidate.get("community_id")
        }
        "claim_entity_clustering" => task.get("claim_id") == candidate.get("claim_id"),
        _ => false,
    })
}

fn anomaly_review_decision_options(
    local_task: Option<&serde_json::Value>,
    fallback: &[&str],
) -> Vec<String> {
    let options = local_task
        .and_then(|task| task.get("decision_options"))
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    if options.is_empty() {
        fallback.iter().map(|value| (*value).into()).collect()
    } else {
        options
    }
}

fn required_json_string(value: &serde_json::Value, key: &str) -> anyhow::Result<String> {
    value
        .get(key)
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .with_context(|| format!("anomaly candidate requires {key}"))
}

fn required_json_i64(value: &serde_json::Value, key: &str) -> anyhow::Result<i64> {
    value
        .get(key)
        .and_then(|value| value.as_i64())
        .with_context(|| format!("anomaly candidate requires integer {key}"))
}

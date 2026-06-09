use anyhow::{bail, Context};
use arrow_array::RecordBatch;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    fs::File,
    path::Path,
};

use super::{
    column_value_at, reject_csv_uri, resolve_parquet_files, round4, write_json,
    ParquetSplitManifest,
};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderPeerClusteringReport {
    pub report_kind: String,
    pub report_version: u8,
    pub dataset_key: String,
    pub dataset_version: String,
    pub algorithm: String,
    pub label_policy: String,
    pub governance_boundary: String,
    pub feature_columns: Vec<String>,
    pub cluster_count: usize,
    pub cluster_summaries: Vec<ProviderPeerClusterSummary>,
    pub provider_assignments: Vec<ProviderPeerClusterAssignment>,
    pub anomaly_candidates: Vec<ProviderPeerAnomalyCandidate>,
    pub factor_ranking: UnsupervisedFactorRanking,
    pub review_tasks: Vec<ProviderPeerReviewTask>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderPeerClusterSummary {
    pub cluster_id: usize,
    pub provider_count: usize,
    pub average_outlier_score: f64,
    pub average_claim_count: f64,
    pub average_high_cost_rate: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderPeerClusterAssignment {
    pub provider_id: String,
    pub cohort_key: String,
    pub service_month: String,
    pub cluster_id: usize,
    pub outlier_score: f64,
    pub anomaly_candidate: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderPeerAnomalyCandidate {
    pub provider_id: String,
    pub cohort_key: String,
    pub service_month: String,
    pub cluster_id: usize,
    pub outlier_score: f64,
    pub reason: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderPeerReviewTask {
    pub task_kind: String,
    pub provider_id: String,
    pub review_queue: String,
    pub required_review: String,
    pub decision_options: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ClaimEntityClusteringReport {
    pub report_kind: String,
    pub report_version: u8,
    pub dataset_key: String,
    pub dataset_version: String,
    pub algorithm: String,
    pub label_policy: String,
    pub governance_boundary: String,
    pub feature_columns: Vec<String>,
    pub cluster_count: usize,
    pub cluster_summaries: Vec<ClaimEntityClusterSummary>,
    pub entity_assignments: Vec<ClaimEntityClusterAssignment>,
    pub anomaly_candidates: Vec<ClaimEntityAnomalyCandidate>,
    pub factor_ranking: UnsupervisedFactorRanking,
    pub review_tasks: Vec<ClaimEntityReviewTask>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ClaimEntityClusterSummary {
    pub cluster_id: usize,
    pub claim_count: usize,
    pub average_outlier_score: f64,
    pub average_claim_amount: f64,
    pub average_provider_degree: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ClaimEntityClusterAssignment {
    pub claim_id: String,
    pub member_id: String,
    pub provider_id: String,
    pub cluster_id: usize,
    pub outlier_score: f64,
    pub anomaly_candidate: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ClaimEntityAnomalyCandidate {
    pub claim_id: String,
    pub member_id: String,
    pub provider_id: String,
    pub cluster_id: usize,
    pub outlier_score: f64,
    pub reason: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ClaimEntityReviewTask {
    pub task_kind: String,
    pub claim_id: String,
    pub member_id: String,
    pub provider_id: String,
    pub review_queue: String,
    pub required_review: String,
    pub decision_options: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderGraphCommunityReport {
    pub report_kind: String,
    pub report_version: u8,
    pub dataset_key: String,
    pub dataset_version: String,
    pub algorithm: String,
    pub label_policy: String,
    pub governance_boundary: String,
    pub community_summaries: Vec<ProviderGraphCommunitySummary>,
    pub provider_assignments: Vec<ProviderGraphCommunityAssignment>,
    pub anomaly_candidates: Vec<ProviderGraphAnomalyCandidate>,
    pub factor_ranking: UnsupervisedFactorRanking,
    pub review_tasks: Vec<ProviderGraphReviewTask>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct UnsupervisedFactorRanking {
    pub report_kind: String,
    pub ranking_policy: String,
    pub ranked_factor_count: usize,
    pub ranked_factors: Vec<UnsupervisedFactorRank>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct UnsupervisedFactorRank {
    pub rank: usize,
    pub feature: String,
    pub ranking_score: f64,
    pub anomaly_candidate_count: usize,
    pub average_abs_centroid_deviation: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderGraphCommunitySummary {
    pub community_id: i32,
    pub provider_count: usize,
    pub average_graph_degree: f64,
    pub average_peer_z_score: f64,
    pub anomaly_candidate_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderGraphCommunityAssignment {
    pub provider_id: String,
    pub community_id: i32,
    pub graph_degree: f64,
    pub peer_z_score: f64,
    pub anomaly_candidate: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderGraphAnomalyCandidate {
    pub provider_id: String,
    pub community_id: i32,
    pub graph_degree: f64,
    pub peer_z_score: f64,
    pub reason: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ProviderGraphReviewTask {
    pub task_kind: String,
    pub provider_id: String,
    pub community_id: i32,
    pub review_queue: String,
    pub required_review: String,
    pub decision_options: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct UnlabeledDatasetManifest {
    dataset_key: String,
    dataset_version: String,
    label_policy: String,
    #[serde(default)]
    label_column: Option<String>,
    splits: Vec<ParquetSplitManifest>,
}

#[derive(Debug, Clone)]
struct ProviderPeerFeatureRow {
    provider_id: String,
    cohort_key: String,
    service_month: String,
    claim_count: f64,
    avg_claim_amount: f64,
    high_cost_rate: f64,
    peer_z_score: f64,
    graph_degree: f64,
    community_id: i32,
}

#[derive(Debug, Clone)]
struct ClaimEntityFeatureRow {
    claim_id: String,
    member_id: String,
    provider_id: String,
    claim_amount: f64,
    amount_to_limit_ratio: f64,
    peer_percentile: f64,
    item_count: f64,
    high_cost_item_ratio: f64,
    provider_risk_tier: f64,
    diagnosis_procedure_mismatch: f64,
    member_degree: f64,
    provider_degree: f64,
}

pub fn cluster_provider_peers(
    manifest_path: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<ProviderPeerClusteringReport> {
    let manifest_path = Path::new(manifest_path);
    let manifest_json = fs::read_to_string(manifest_path)
        .with_context(|| format!("read provider peer manifest {}", manifest_path.display()))?;
    let manifest: UnlabeledDatasetManifest =
        serde_json::from_str(&manifest_json).context("parse provider peer manifest")?;
    if manifest.label_column.is_some() {
        bail!("provider peer clustering requires an unlabeled manifest");
    }
    if !manifest.label_policy.contains("unlabeled") {
        bail!("provider peer clustering requires an unlabeled label_policy");
    }
    let base_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let rows = read_provider_peer_rows(&manifest, base_dir)?;
    if rows.len() < 2 {
        bail!("provider peer clustering requires at least two provider rows");
    }

    let feature_columns = vec![
        "claim_count".into(),
        "avg_claim_amount".into(),
        "high_cost_rate".into(),
        "peer_z_score".into(),
        "graph_degree".into(),
    ];
    let normalized = normalize_provider_rows(&rows);
    let cluster_count = rows.len().clamp(1, 3);
    let cluster_ids = assign_provider_clusters(&normalized, cluster_count);
    let distances = cluster_distances(&normalized, &cluster_ids, cluster_count);
    let threshold = anomaly_threshold(&distances);
    let mut provider_assignments = Vec::new();
    let mut anomaly_candidates = Vec::new();
    let mut anomaly_indexes = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let outlier_score = round4(distances[index]);
        let anomaly_candidate = distances[index] >= threshold;
        provider_assignments.push(ProviderPeerClusterAssignment {
            provider_id: row.provider_id.clone(),
            cohort_key: row.cohort_key.clone(),
            service_month: row.service_month.clone(),
            cluster_id: cluster_ids[index],
            outlier_score,
            anomaly_candidate,
        });
        if anomaly_candidate {
            anomaly_indexes.push(index);
            anomaly_candidates.push(ProviderPeerAnomalyCandidate {
                provider_id: row.provider_id.clone(),
                cohort_key: row.cohort_key.clone(),
                service_month: row.service_month.clone(),
                cluster_id: cluster_ids[index],
                outlier_score,
                reason: "Provider-month is far from its peer-cluster centroid; review as an anomaly candidate, not a confirmed FWA label.".into(),
                evidence_refs: vec![
                    format!("dataset_manifest:{}", manifest_path.display()),
                    format!("provider_peer_cluster:{}:{}", manifest.dataset_key, row.provider_id),
                ],
            });
        }
    }
    anomaly_candidates.sort_by(|left, right| {
        right
            .outlier_score
            .partial_cmp(&left.outlier_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.provider_id.cmp(&right.provider_id))
    });

    let review_tasks = anomaly_candidates
        .iter()
        .map(|candidate| ProviderPeerReviewTask {
            task_kind: "provider_peer_anomaly_review".into(),
            provider_id: candidate.provider_id.clone(),
            review_queue: "provider_anomaly_candidate_review".into(),
            required_review: "human_review_required_before_case_creation_or_label_assignment"
                .into(),
            decision_options: vec![
                "dismiss_as_peer_variation".into(),
                "request_more_evidence".into(),
                "open_investigation_candidate".into(),
            ],
            evidence_refs: candidate.evidence_refs.clone(),
        })
        .collect::<Vec<_>>();
    let cluster_summaries =
        summarize_provider_clusters(&rows, &cluster_ids, &distances, cluster_count);
    let factor_ranking = standardized_factor_ranking(
        "provider_peer_unsupervised_factor_ranking",
        &feature_columns,
        &normalized,
        &cluster_ids,
        cluster_count,
        &anomaly_indexes,
    );
    let report = ProviderPeerClusteringReport {
        report_kind: "provider_peer_clustering".into(),
        report_version: 1,
        dataset_key: manifest.dataset_key,
        dataset_version: manifest.dataset_version,
        algorithm: "rust_standardized_kmeans_v1".into(),
        label_policy: manifest.label_policy,
        governance_boundary:
            "unlabeled clustering creates anomaly review candidates only; it must not create confirmed FWA labels or automatic claim disposition"
                .into(),
        feature_columns,
        cluster_count,
        cluster_summaries,
        provider_assignments,
        anomaly_candidates,
        factor_ranking,
        review_tasks,
        evidence_refs: vec![
            format!("dataset_manifest:{}", manifest_path.display()),
            format!(
                "unsupervised_factor_rankings:{}",
                output_dir
                    .as_ref()
                    .join("provider_peer_factor_ranking.json")
                    .display()
            ),
        ],
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create provider peer clustering output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("provider_peer_clustering_report.json"),
        &report,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("provider_peer_factor_ranking.json"),
        &report.factor_ranking,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("provider_anomaly_review_tasks.json"),
        &report.review_tasks,
    )?;
    Ok(report)
}

pub fn cluster_claim_entities(
    manifest_path: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<ClaimEntityClusteringReport> {
    let manifest_path = Path::new(manifest_path);
    let manifest_json = fs::read_to_string(manifest_path)
        .with_context(|| format!("read claim entity manifest {}", manifest_path.display()))?;
    let manifest: UnlabeledDatasetManifest =
        serde_json::from_str(&manifest_json).context("parse claim entity manifest")?;
    if manifest.label_column.is_some() {
        bail!("claim entity clustering requires an unlabeled manifest");
    }
    if !manifest.label_policy.contains("unlabeled") {
        bail!("claim entity clustering requires an unlabeled label_policy");
    }
    let base_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let rows = read_claim_entity_rows(&manifest, base_dir)?;
    if rows.len() < 2 {
        bail!("claim entity clustering requires at least two claim rows");
    }

    let feature_columns = vec![
        "claim_amount".into(),
        "amount_to_limit_ratio".into(),
        "peer_percentile".into(),
        "item_count".into(),
        "high_cost_item_ratio".into(),
        "provider_risk_tier".into(),
        "diagnosis_procedure_mismatch".into(),
        "member_degree".into(),
        "provider_degree".into(),
    ];
    let normalized = normalize_claim_entity_rows(&rows);
    let cluster_count = rows.len().clamp(1, 4);
    let cluster_ids = assign_standardized_clusters(&normalized, 2, cluster_count);
    let distances = standardized_cluster_distances(&normalized, &cluster_ids, cluster_count);
    let threshold = anomaly_threshold(&distances);
    let mut entity_assignments = Vec::new();
    let mut anomaly_candidates = Vec::new();
    let mut anomaly_indexes = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        let outlier_score = round4(distances[index]);
        let anomaly_candidate = distances[index] >= threshold;
        entity_assignments.push(ClaimEntityClusterAssignment {
            claim_id: row.claim_id.clone(),
            member_id: row.member_id.clone(),
            provider_id: row.provider_id.clone(),
            cluster_id: cluster_ids[index],
            outlier_score,
            anomaly_candidate,
        });
        if anomaly_candidate {
            anomaly_indexes.push(index);
            anomaly_candidates.push(ClaimEntityAnomalyCandidate {
                claim_id: row.claim_id.clone(),
                member_id: row.member_id.clone(),
                provider_id: row.provider_id.clone(),
                cluster_id: cluster_ids[index],
                outlier_score,
                reason: "Claim/member/provider entity context is far from its cluster centroid; review as an anomaly candidate, not a confirmed FWA label.".into(),
                evidence_refs: vec![
                    format!("dataset_manifest:{}", manifest_path.display()),
                    format!("claim_entity_cluster:{}:{}", manifest.dataset_key, row.claim_id),
                ],
            });
        }
    }
    anomaly_candidates.sort_by(|left, right| {
        right
            .outlier_score
            .partial_cmp(&left.outlier_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.claim_id.cmp(&right.claim_id))
    });

    let review_tasks = anomaly_candidates
        .iter()
        .map(|candidate| ClaimEntityReviewTask {
            task_kind: "claim_entity_anomaly_review".into(),
            claim_id: candidate.claim_id.clone(),
            member_id: candidate.member_id.clone(),
            provider_id: candidate.provider_id.clone(),
            review_queue: "claim_entity_anomaly_candidate_review".into(),
            required_review:
                "human_review_required_before_case_creation_label_assignment_or_rule_writeback"
                    .into(),
            decision_options: vec![
                "dismiss_as_entity_variation".into(),
                "request_more_evidence".into(),
                "open_investigation_candidate".into(),
                "prepare_rule_candidate_backtest".into(),
            ],
            evidence_refs: candidate.evidence_refs.clone(),
        })
        .collect::<Vec<_>>();
    let cluster_summaries =
        summarize_claim_entity_clusters(&rows, &cluster_ids, &distances, cluster_count);
    let factor_ranking = standardized_factor_ranking(
        "claim_entity_unsupervised_factor_ranking",
        &feature_columns,
        &normalized,
        &cluster_ids,
        cluster_count,
        &anomaly_indexes,
    );
    let report = ClaimEntityClusteringReport {
        report_kind: "claim_entity_clustering".into(),
        report_version: 1,
        dataset_key: manifest.dataset_key,
        dataset_version: manifest.dataset_version,
        algorithm: "rust_standardized_entity_kmeans_v1".into(),
        label_policy: manifest.label_policy,
        governance_boundary:
            "unlabeled entity clustering creates anomaly review candidates only; it must not create confirmed FWA labels, automatic claim disposition, or rule-library writeback"
                .into(),
        feature_columns,
        cluster_count,
        cluster_summaries,
        entity_assignments,
        anomaly_candidates,
        factor_ranking,
        review_tasks,
        evidence_refs: vec![
            format!("dataset_manifest:{}", manifest_path.display()),
            format!(
                "unsupervised_factor_rankings:{}",
                output_dir
                    .as_ref()
                    .join("claim_entity_factor_ranking.json")
                    .display()
            ),
        ],
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create claim entity clustering output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("claim_entity_clustering_report.json"),
        &report,
    )?;
    write_json(
        output_dir.as_ref().join("claim_entity_factor_ranking.json"),
        &report.factor_ranking,
    )?;
    write_json(
        output_dir.as_ref().join("claim_entity_review_tasks.json"),
        &report.review_tasks,
    )?;
    Ok(report)
}

pub fn cluster_provider_graph_communities(
    manifest_path: &str,
    output_dir: impl AsRef<Path>,
) -> anyhow::Result<ProviderGraphCommunityReport> {
    let manifest_path = Path::new(manifest_path);
    let manifest_json = fs::read_to_string(manifest_path)
        .with_context(|| format!("read provider graph manifest {}", manifest_path.display()))?;
    let manifest: UnlabeledDatasetManifest =
        serde_json::from_str(&manifest_json).context("parse provider graph manifest")?;
    if manifest.label_column.is_some() {
        bail!("provider graph clustering requires an unlabeled manifest");
    }
    if !manifest.label_policy.contains("unlabeled") {
        bail!("provider graph clustering requires an unlabeled label_policy");
    }
    let base_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let rows = read_provider_peer_rows(&manifest, base_dir)?;
    if rows.len() < 2 {
        bail!("provider graph clustering requires at least two provider rows");
    }

    let graph_degree_threshold =
        anomaly_threshold(&rows.iter().map(|row| row.graph_degree).collect::<Vec<_>>());
    let mut provider_assignments = Vec::new();
    let mut anomaly_candidates = Vec::new();
    for row in &rows {
        let anomaly_candidate =
            row.graph_degree >= graph_degree_threshold || row.peer_z_score >= 2.0;
        provider_assignments.push(ProviderGraphCommunityAssignment {
            provider_id: row.provider_id.clone(),
            community_id: row.community_id,
            graph_degree: row.graph_degree,
            peer_z_score: row.peer_z_score,
            anomaly_candidate,
        });
        if anomaly_candidate {
            anomaly_candidates.push(ProviderGraphAnomalyCandidate {
                provider_id: row.provider_id.clone(),
                community_id: row.community_id,
                graph_degree: row.graph_degree,
                peer_z_score: row.peer_z_score,
                reason: "Provider is unusually central or high-risk inside the provider graph community; review as a graph anomaly candidate, not a confirmed FWA label.".into(),
                evidence_refs: vec![
                    format!("dataset_manifest:{}", manifest_path.display()),
                    format!(
                        "provider_graph_community:{}:{}",
                        manifest.dataset_key, row.provider_id
                    ),
                ],
            });
        }
    }
    anomaly_candidates.sort_by(|left, right| {
        right
            .graph_degree
            .partial_cmp(&left.graph_degree)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                right
                    .peer_z_score
                    .partial_cmp(&left.peer_z_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| left.provider_id.cmp(&right.provider_id))
    });
    let review_tasks = anomaly_candidates
        .iter()
        .map(|candidate| ProviderGraphReviewTask {
            task_kind: "provider_graph_anomaly_review".into(),
            provider_id: candidate.provider_id.clone(),
            community_id: candidate.community_id,
            review_queue: "provider_graph_anomaly_candidate_review".into(),
            required_review: "human_review_required_before_case_creation_or_label_assignment"
                .into(),
            decision_options: vec![
                "dismiss_as_network_variation".into(),
                "request_more_evidence".into(),
                "open_investigation_candidate".into(),
            ],
            evidence_refs: candidate.evidence_refs.clone(),
        })
        .collect::<Vec<_>>();
    let community_summaries = summarize_provider_graph_communities(&rows, &provider_assignments);
    let factor_ranking = provider_graph_factor_ranking(&anomaly_candidates);
    let report = ProviderGraphCommunityReport {
        report_kind: "provider_graph_community_clustering".into(),
        report_version: 1,
        dataset_key: manifest.dataset_key,
        dataset_version: manifest.dataset_version,
        algorithm: "rust_provider_graph_community_v1".into(),
        label_policy: manifest.label_policy,
        governance_boundary:
            "unlabeled graph clustering creates anomaly review candidates only; it must not create confirmed FWA labels or automatic claim disposition"
                .into(),
        community_summaries,
        provider_assignments,
        anomaly_candidates,
        factor_ranking,
        review_tasks,
        evidence_refs: vec![
            format!("dataset_manifest:{}", manifest_path.display()),
            format!(
                "unsupervised_factor_rankings:{}",
                output_dir
                    .as_ref()
                    .join("provider_graph_factor_ranking.json")
                    .display()
            ),
        ],
    };

    fs::create_dir_all(output_dir.as_ref()).with_context(|| {
        format!(
            "create provider graph clustering output dir {}",
            output_dir.as_ref().display()
        )
    })?;
    write_json(
        output_dir
            .as_ref()
            .join("provider_graph_community_report.json"),
        &report,
    )?;
    write_json(
        output_dir
            .as_ref()
            .join("provider_graph_factor_ranking.json"),
        &report.factor_ranking,
    )?;
    write_json(
        output_dir.as_ref().join("provider_graph_review_tasks.json"),
        &report.review_tasks,
    )?;
    Ok(report)
}

fn read_provider_peer_rows(
    manifest: &UnlabeledDatasetManifest,
    base_dir: &Path,
) -> anyhow::Result<Vec<ProviderPeerFeatureRow>> {
    let mut rows = Vec::new();
    for split in &manifest.splits {
        reject_csv_uri(&split.data_uri)?;
        let parquet_files = resolve_parquet_files(base_dir, &split.data_uri)?;
        if parquet_files.is_empty() {
            bail!("split {} has no parquet files", split.split_name);
        }
        for parquet_file in parquet_files {
            let file = File::open(&parquet_file)
                .with_context(|| format!("open parquet file {}", parquet_file.display()))?;
            let builder = ParquetRecordBatchReaderBuilder::try_new(file)
                .with_context(|| format!("read parquet metadata {}", parquet_file.display()))?;
            let mut reader = builder.with_batch_size(4096).build()?;
            for batch in &mut reader {
                let batch = batch?;
                for row_index in 0..batch.num_rows() {
                    rows.push(ProviderPeerFeatureRow {
                        provider_id: required_string_cell(&batch, "provider_id", row_index)?,
                        cohort_key: required_string_cell(&batch, "cohort_key", row_index)?,
                        service_month: required_string_cell(&batch, "service_month", row_index)?,
                        claim_count: required_numeric_cell(&batch, "claim_count", row_index)?,
                        avg_claim_amount: required_numeric_cell(
                            &batch,
                            "avg_claim_amount",
                            row_index,
                        )?,
                        high_cost_rate: required_numeric_cell(&batch, "high_cost_rate", row_index)?,
                        peer_z_score: required_numeric_cell(&batch, "peer_z_score", row_index)?,
                        graph_degree: required_numeric_cell(&batch, "graph_degree", row_index)?,
                        community_id: required_numeric_cell(&batch, "community_id", row_index)?
                            as i32,
                    });
                }
            }
        }
    }
    Ok(rows)
}

fn read_claim_entity_rows(
    manifest: &UnlabeledDatasetManifest,
    base_dir: &Path,
) -> anyhow::Result<Vec<ClaimEntityFeatureRow>> {
    let mut raw_rows = Vec::new();
    for split in &manifest.splits {
        reject_csv_uri(&split.data_uri)?;
        let parquet_files = resolve_parquet_files(base_dir, &split.data_uri)?;
        if parquet_files.is_empty() {
            bail!("split {} has no parquet files", split.split_name);
        }
        for parquet_file in parquet_files {
            let file = File::open(&parquet_file)
                .with_context(|| format!("open parquet file {}", parquet_file.display()))?;
            let builder = ParquetRecordBatchReaderBuilder::try_new(file)
                .with_context(|| format!("read parquet metadata {}", parquet_file.display()))?;
            let mut reader = builder.with_batch_size(4096).build()?;
            for batch in &mut reader {
                let batch = batch?;
                for row_index in 0..batch.num_rows() {
                    raw_rows.push((
                        required_string_cell(&batch, "claim_id", row_index)?,
                        required_string_cell(&batch, "member_id", row_index)?,
                        required_string_cell(&batch, "provider_id", row_index)?,
                        required_numeric_cell(&batch, "claim_amount", row_index)?,
                        required_numeric_cell(&batch, "amount_to_limit_ratio", row_index)?,
                        required_numeric_cell(&batch, "peer_percentile", row_index)?,
                        required_numeric_cell(&batch, "item_count", row_index)?,
                        required_numeric_cell(&batch, "high_cost_item_ratio", row_index)?,
                        required_numeric_cell(&batch, "provider_risk_tier", row_index)?,
                        required_numeric_cell(&batch, "diagnosis_procedure_mismatch", row_index)?,
                    ));
                }
            }
        }
    }

    let mut member_counts = BTreeMap::<String, u64>::new();
    let mut provider_counts = BTreeMap::<String, u64>::new();
    for (_, member_id, provider_id, ..) in &raw_rows {
        *member_counts.entry(member_id.clone()).or_default() += 1;
        *provider_counts.entry(provider_id.clone()).or_default() += 1;
    }

    Ok(raw_rows
        .into_iter()
        .map(
            |(
                claim_id,
                member_id,
                provider_id,
                claim_amount,
                amount_to_limit_ratio,
                peer_percentile,
                item_count,
                high_cost_item_ratio,
                provider_risk_tier,
                diagnosis_procedure_mismatch,
            )| {
                let member_degree = member_counts.get(&member_id).copied().unwrap_or(1) as f64;
                let provider_degree =
                    provider_counts.get(&provider_id).copied().unwrap_or(1) as f64;
                ClaimEntityFeatureRow {
                    claim_id,
                    member_id,
                    provider_id,
                    claim_amount,
                    amount_to_limit_ratio,
                    peer_percentile,
                    item_count,
                    high_cost_item_ratio,
                    provider_risk_tier,
                    diagnosis_procedure_mismatch,
                    member_degree,
                    provider_degree,
                }
            },
        )
        .collect())
}

fn required_string_cell(
    batch: &RecordBatch,
    column_name: &str,
    row_index: usize,
) -> anyhow::Result<String> {
    let column_index = batch
        .schema()
        .index_of(column_name)
        .with_context(|| format!("missing provider peer column {column_name}"))?;
    column_value_at(batch.column(column_index).as_ref(), row_index)
        .filter(|value| !value.trim().is_empty())
        .with_context(|| format!("missing provider peer value {column_name} at row {row_index}"))
}

fn required_numeric_cell(
    batch: &RecordBatch,
    column_name: &str,
    row_index: usize,
) -> anyhow::Result<f64> {
    let value = required_string_cell(batch, column_name, row_index)?;
    value
        .parse::<f64>()
        .with_context(|| format!("invalid numeric provider peer value {column_name}: {value}"))
}

fn normalize_provider_rows(rows: &[ProviderPeerFeatureRow]) -> Vec<[f64; 5]> {
    let raw = rows
        .iter()
        .map(|row| {
            [
                row.claim_count,
                row.avg_claim_amount,
                row.high_cost_rate,
                row.peer_z_score,
                row.graph_degree,
            ]
        })
        .collect::<Vec<_>>();
    let mut means = [0.0; 5];
    for values in &raw {
        for index in 0..5 {
            means[index] += values[index];
        }
    }
    for mean in &mut means {
        *mean /= raw.len() as f64;
    }
    let mut stddevs = [0.0; 5];
    for values in &raw {
        for index in 0..5 {
            stddevs[index] += (values[index] - means[index]).powi(2);
        }
    }
    for stddev in &mut stddevs {
        *stddev = (*stddev / raw.len() as f64).sqrt();
        if *stddev == 0.0 {
            *stddev = 1.0;
        }
    }
    raw.iter()
        .map(|values| {
            let mut normalized = [0.0; 5];
            for index in 0..5 {
                normalized[index] = (values[index] - means[index]) / stddevs[index];
            }
            normalized
        })
        .collect()
}

fn normalize_claim_entity_rows(rows: &[ClaimEntityFeatureRow]) -> Vec<[f64; 9]> {
    let raw = rows
        .iter()
        .map(|row| {
            [
                row.claim_amount,
                row.amount_to_limit_ratio,
                row.peer_percentile,
                row.item_count,
                row.high_cost_item_ratio,
                row.provider_risk_tier,
                row.diagnosis_procedure_mismatch,
                row.member_degree,
                row.provider_degree,
            ]
        })
        .collect::<Vec<_>>();
    let mut means = [0.0; 9];
    for values in &raw {
        for index in 0..9 {
            means[index] += values[index];
        }
    }
    for mean in &mut means {
        *mean /= raw.len() as f64;
    }
    let mut stddevs = [0.0; 9];
    for values in &raw {
        for index in 0..9 {
            stddevs[index] += (values[index] - means[index]).powi(2);
        }
    }
    for stddev in &mut stddevs {
        *stddev = (*stddev / raw.len() as f64).sqrt();
        if *stddev == 0.0 {
            *stddev = 1.0;
        }
    }
    raw.iter()
        .map(|values| {
            let mut normalized = [0.0; 9];
            for index in 0..9 {
                normalized[index] = (values[index] - means[index]) / stddevs[index];
            }
            normalized
        })
        .collect()
}

fn assign_provider_clusters(rows: &[[f64; 5]], cluster_count: usize) -> Vec<usize> {
    assign_standardized_clusters(rows, 3, cluster_count)
}

fn assign_standardized_clusters<const N: usize>(
    rows: &[[f64; N]],
    ordering_feature_index: usize,
    cluster_count: usize,
) -> Vec<usize> {
    let mut ordered = rows.iter().enumerate().collect::<Vec<_>>();
    ordered.sort_by(|left, right| {
        left.1[ordering_feature_index]
            .partial_cmp(&right.1[ordering_feature_index])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut centroids = (0..cluster_count)
        .map(|cluster_index| {
            let source_index = cluster_index * (ordered.len() - 1) / cluster_count.max(1);
            *ordered[source_index].1
        })
        .collect::<Vec<_>>();
    let mut assignments = vec![0; rows.len()];
    for _ in 0..12 {
        for (row_index, row) in rows.iter().enumerate() {
            assignments[row_index] = nearest_centroid(row, &centroids);
        }
        let mut sums = vec![[0.0; N]; cluster_count];
        let mut counts = vec![0_usize; cluster_count];
        for (row, cluster_id) in rows.iter().zip(assignments.iter()) {
            counts[*cluster_id] += 1;
            for index in 0..N {
                sums[*cluster_id][index] += row[index];
            }
        }
        for cluster_id in 0..cluster_count {
            if counts[cluster_id] == 0 {
                continue;
            }
            for index in 0..N {
                centroids[cluster_id][index] = sums[cluster_id][index] / counts[cluster_id] as f64;
            }
        }
    }
    assignments
}

fn nearest_centroid<const N: usize>(row: &[f64; N], centroids: &[[f64; N]]) -> usize {
    centroids
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            squared_distance(row, *left)
                .partial_cmp(&squared_distance(row, *right))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn cluster_distances(rows: &[[f64; 5]], assignments: &[usize], cluster_count: usize) -> Vec<f64> {
    standardized_cluster_distances(rows, assignments, cluster_count)
}

fn standardized_cluster_distances<const N: usize>(
    rows: &[[f64; N]],
    assignments: &[usize],
    cluster_count: usize,
) -> Vec<f64> {
    let mut sums = vec![[0.0; N]; cluster_count];
    let mut counts = vec![0_usize; cluster_count];
    for (row, cluster_id) in rows.iter().zip(assignments.iter()) {
        counts[*cluster_id] += 1;
        for index in 0..N {
            sums[*cluster_id][index] += row[index];
        }
    }
    let mut centroids = vec![[0.0; N]; cluster_count];
    for cluster_id in 0..cluster_count {
        if counts[cluster_id] == 0 {
            continue;
        }
        for index in 0..N {
            centroids[cluster_id][index] = sums[cluster_id][index] / counts[cluster_id] as f64;
        }
    }
    rows.iter()
        .zip(assignments.iter())
        .map(|(row, cluster_id)| squared_distance(row, &centroids[*cluster_id]).sqrt())
        .collect()
}

fn standardized_factor_ranking<const N: usize>(
    report_kind: &str,
    feature_columns: &[String],
    rows: &[[f64; N]],
    assignments: &[usize],
    cluster_count: usize,
    anomaly_indexes: &[usize],
) -> UnsupervisedFactorRanking {
    let mut sums = vec![[0.0; N]; cluster_count];
    let mut counts = vec![0_usize; cluster_count];
    for (row, cluster_id) in rows.iter().zip(assignments.iter()) {
        counts[*cluster_id] += 1;
        for index in 0..N {
            sums[*cluster_id][index] += row[index];
        }
    }
    let mut centroids = vec![[0.0; N]; cluster_count];
    for cluster_id in 0..cluster_count {
        if counts[cluster_id] == 0 {
            continue;
        }
        for index in 0..N {
            centroids[cluster_id][index] = sums[cluster_id][index] / counts[cluster_id] as f64;
        }
    }

    let mut contribution_totals = vec![0.0; N];
    for row_index in anomaly_indexes {
        let row = &rows[*row_index];
        let centroid = &centroids[assignments[*row_index]];
        for index in 0..N {
            contribution_totals[index] += (row[index] - centroid[index]).abs();
        }
    }
    let divisor = anomaly_indexes.len().max(1) as f64;
    let mut ranked_factors = feature_columns
        .iter()
        .enumerate()
        .map(|(index, feature)| UnsupervisedFactorRank {
            rank: 0,
            feature: feature.clone(),
            ranking_score: round4(contribution_totals[index] / divisor),
            anomaly_candidate_count: anomaly_indexes.len(),
            average_abs_centroid_deviation: round4(contribution_totals[index] / divisor),
        })
        .collect::<Vec<_>>();
    ranked_factors.sort_by(|left, right| {
        right
            .ranking_score
            .partial_cmp(&left.ranking_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.feature.cmp(&right.feature))
    });
    for (index, factor) in ranked_factors.iter_mut().enumerate() {
        factor.rank = index + 1;
    }
    UnsupervisedFactorRanking {
        report_kind: report_kind.into(),
        ranking_policy:
            "average_absolute_standardized_anomaly_deviation_from_assigned_cluster_centroid".into(),
        ranked_factor_count: ranked_factors.len(),
        ranked_factors,
    }
}

fn provider_graph_factor_ranking(
    anomaly_candidates: &[ProviderGraphAnomalyCandidate],
) -> UnsupervisedFactorRanking {
    let count = anomaly_candidates.len();
    let graph_degree = anomaly_candidates
        .iter()
        .map(|candidate| candidate.graph_degree.abs())
        .sum::<f64>()
        / count.max(1) as f64;
    let peer_z_score = anomaly_candidates
        .iter()
        .map(|candidate| candidate.peer_z_score.abs())
        .sum::<f64>()
        / count.max(1) as f64;
    let mut ranked_factors = vec![
        UnsupervisedFactorRank {
            rank: 0,
            feature: "graph_degree".into(),
            ranking_score: round4(graph_degree),
            anomaly_candidate_count: count,
            average_abs_centroid_deviation: round4(graph_degree),
        },
        UnsupervisedFactorRank {
            rank: 0,
            feature: "peer_z_score".into(),
            ranking_score: round4(peer_z_score),
            anomaly_candidate_count: count,
            average_abs_centroid_deviation: round4(peer_z_score),
        },
    ];
    ranked_factors.sort_by(|left, right| {
        right
            .ranking_score
            .partial_cmp(&left.ranking_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.feature.cmp(&right.feature))
    });
    for (index, factor) in ranked_factors.iter_mut().enumerate() {
        factor.rank = index + 1;
    }
    UnsupervisedFactorRanking {
        report_kind: "provider_graph_unsupervised_factor_ranking".into(),
        ranking_policy: "average_absolute_graph_anomaly_signal_for_review_candidates".into(),
        ranked_factor_count: ranked_factors.len(),
        ranked_factors,
    }
}

fn squared_distance(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| (*left - *right).powi(2))
        .sum()
}

fn anomaly_threshold(distances: &[f64]) -> f64 {
    let mean = distances.iter().sum::<f64>() / distances.len() as f64;
    let variance = distances
        .iter()
        .map(|distance| (distance - mean).powi(2))
        .sum::<f64>()
        / distances.len() as f64;
    let threshold = mean + variance.sqrt();
    if distances.iter().any(|distance| *distance >= threshold) {
        threshold
    } else {
        distances
            .iter()
            .copied()
            .fold(0.0, |current, distance| current.max(distance))
    }
}

fn summarize_provider_clusters(
    rows: &[ProviderPeerFeatureRow],
    assignments: &[usize],
    distances: &[f64],
    cluster_count: usize,
) -> Vec<ProviderPeerClusterSummary> {
    (0..cluster_count)
        .map(|cluster_id| {
            let indexes = assignments
                .iter()
                .enumerate()
                .filter_map(|(index, assigned)| (*assigned == cluster_id).then_some(index))
                .collect::<Vec<_>>();
            let provider_count = indexes.len();
            let divisor = provider_count.max(1) as f64;
            ProviderPeerClusterSummary {
                cluster_id,
                provider_count,
                average_outlier_score: round4(
                    indexes.iter().map(|index| distances[*index]).sum::<f64>() / divisor,
                ),
                average_claim_count: round4(
                    indexes
                        .iter()
                        .map(|index| rows[*index].claim_count)
                        .sum::<f64>()
                        / divisor,
                ),
                average_high_cost_rate: round4(
                    indexes
                        .iter()
                        .map(|index| rows[*index].high_cost_rate)
                        .sum::<f64>()
                        / divisor,
                ),
            }
        })
        .collect()
}

fn summarize_claim_entity_clusters(
    rows: &[ClaimEntityFeatureRow],
    assignments: &[usize],
    distances: &[f64],
    cluster_count: usize,
) -> Vec<ClaimEntityClusterSummary> {
    (0..cluster_count)
        .map(|cluster_id| {
            let indexes = assignments
                .iter()
                .enumerate()
                .filter_map(|(index, assigned)| (*assigned == cluster_id).then_some(index))
                .collect::<Vec<_>>();
            let claim_count = indexes.len();
            let divisor = claim_count.max(1) as f64;
            ClaimEntityClusterSummary {
                cluster_id,
                claim_count,
                average_outlier_score: round4(
                    indexes.iter().map(|index| distances[*index]).sum::<f64>() / divisor,
                ),
                average_claim_amount: round4(
                    indexes
                        .iter()
                        .map(|index| rows[*index].claim_amount)
                        .sum::<f64>()
                        / divisor,
                ),
                average_provider_degree: round4(
                    indexes
                        .iter()
                        .map(|index| rows[*index].provider_degree)
                        .sum::<f64>()
                        / divisor,
                ),
            }
        })
        .collect()
}

fn summarize_provider_graph_communities(
    rows: &[ProviderPeerFeatureRow],
    assignments: &[ProviderGraphCommunityAssignment],
) -> Vec<ProviderGraphCommunitySummary> {
    let mut community_ids = rows
        .iter()
        .map(|row| row.community_id)
        .collect::<BTreeSet<_>>();
    if community_ids.is_empty() {
        community_ids.insert(0);
    }
    community_ids
        .into_iter()
        .map(|community_id| {
            let indexes = rows
                .iter()
                .enumerate()
                .filter_map(|(index, row)| (row.community_id == community_id).then_some(index))
                .collect::<Vec<_>>();
            let provider_count = indexes.len();
            let divisor = provider_count.max(1) as f64;
            let anomaly_candidate_count = assignments
                .iter()
                .filter(|assignment| {
                    assignment.community_id == community_id && assignment.anomaly_candidate
                })
                .count();
            ProviderGraphCommunitySummary {
                community_id,
                provider_count,
                average_graph_degree: round4(
                    indexes
                        .iter()
                        .map(|index| rows[*index].graph_degree)
                        .sum::<f64>()
                        / divisor,
                ),
                average_peer_z_score: round4(
                    indexes
                        .iter()
                        .map(|index| rows[*index].peer_z_score)
                        .sum::<f64>()
                        / divisor,
                ),
                anomaly_candidate_count,
            }
        })
        .collect()
}

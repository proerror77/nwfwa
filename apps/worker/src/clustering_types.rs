use serde::Serialize;

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

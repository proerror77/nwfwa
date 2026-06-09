use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCaseRecord {
    pub case_id: String,
    pub title: String,
    pub fwa_type: String,
    pub scheme_family: String,
    pub diagnosis_code: String,
    pub provider_region: String,
    pub provider_type: String,
    pub summary: String,
    pub outcome: String,
    pub tags: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarCaseQuery {
    pub claim_id: Option<String>,
    pub diagnosis_code: String,
    pub provider_region: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarCaseRecord {
    pub case_id: String,
    pub title: String,
    pub scheme_family: String,
    pub similarity_score: f64,
    pub matched_signals: Vec<String>,
    pub retrieval_method: String,
    pub provenance_refs: Vec<String>,
    pub summary: String,
    pub outcome: String,
    pub evidence_refs: Vec<String>,
}

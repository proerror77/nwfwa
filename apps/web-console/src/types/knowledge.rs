use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct KnowledgeCaseListResponse {
    pub(crate) cases: Vec<KnowledgeCase>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct KnowledgeCase {
    pub(crate) case_id: String,
    pub(crate) title: String,
    pub(crate) fwa_type: String,
    pub(crate) scheme_family: String,
    pub(crate) diagnosis_code: String,
    pub(crate) provider_region: String,
    pub(crate) provider_type: String,
    pub(crate) summary: String,
    pub(crate) outcome: String,
    pub(crate) tags: Vec<String>,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct SimilarCaseSearchResponse {
    pub(crate) results: Vec<SimilarCase>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct SimilarCase {
    pub(crate) case_id: String,
    pub(crate) title: String,
    pub(crate) scheme_family: String,
    pub(crate) similarity_score: f64,
    pub(crate) matched_signals: Vec<String>,
    pub(crate) retrieval_method: String,
    pub(crate) provenance_refs: Vec<String>,
    pub(crate) summary: String,
    pub(crate) outcome: String,
    pub(crate) evidence_refs: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct KnowledgeSnapshot {
    pub(crate) cases: Vec<KnowledgeCase>,
    pub(crate) results: Vec<SimilarCase>,
}

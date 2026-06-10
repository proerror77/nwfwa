use async_trait::async_trait;
use fwa_core::{ClaimId, ScoringRunId};
use fwa_features::FeatureMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ModelRuntimeError {
    #[error("model service unavailable")]
    ServiceUnavailable,
    #[error("model response invalid: {0}")]
    InvalidResponse(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelScoreRequest {
    pub run_id: ScoringRunId,
    pub claim_id: ClaimId,
    pub model_key: String,
    pub model_version: String,
    pub endpoint_url: Option<String>,
    pub features: FeatureMap,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelExplanation {
    pub feature: String,
    pub direction: String,
    pub contribution: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelScore {
    pub model_key: String,
    pub model_version: String,
    pub runtime_kind: String,
    pub execution_provider: String,
    pub score: u8,
    pub label: String,
    pub explanations: Vec<ModelExplanation>,
    pub metadata: Value,
    pub latency_ms: u64,
}

#[async_trait]
pub trait ModelScorer: Send + Sync {
    async fn score(&self, request: ModelScoreRequest) -> Result<ModelScore, ModelRuntimeError>;
}

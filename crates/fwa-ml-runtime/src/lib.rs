use async_trait::async_trait;
use fwa_core::{ClaimId, ScoringRunId};
use fwa_features::FeatureMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
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

#[derive(Debug, Default)]
pub struct HeuristicModelScorer;

#[async_trait]
impl ModelScorer for HeuristicModelScorer {
    async fn score(&self, request: ModelScoreRequest) -> Result<ModelScore, ModelRuntimeError> {
        let ratio = request
            .features
            .get("claim_amount_to_limit_ratio")
            .and_then(|feature| feature.value.as_f64())
            .unwrap_or(0.0);
        let score = (ratio * 100.0).round().clamp(0.0, 100.0) as u8;
        Ok(ModelScore {
            model_key: request.model_key,
            model_version: request.model_version,
            runtime_kind: "heuristic".into(),
            execution_provider: "cpu".into(),
            score,
            label: if score >= 70 { "HIGH_RISK" } else { "LOW_RISK" }.into(),
            explanations: vec![ModelExplanation {
                feature: "claim_amount_to_limit_ratio".into(),
                direction: "increases_risk".into(),
                contribution: ratio,
                reason: "理赔金额占保障额度比例影响模型分".into(),
            }],
            metadata: serde_json::json!({
                "source": "heuristic",
                "calibration": "none",
                "fraud_probability": score as f64 / 100.0,
                "abuse_probability": (ratio * 0.70).clamp(0.0, 1.0),
                "waste_probability": (ratio * 0.40).clamp(0.0, 1.0)
            }),
            latency_ms: 0,
        })
    }
}

#[derive(Debug, Clone)]
pub struct HttpModelScorer {
    client: reqwest::Client,
    base_url: String,
}

impl HttpModelScorer {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
struct HttpScoreRequest {
    run_id: String,
    claim_id: String,
    model_key: String,
    model_version: String,
    features: BTreeMap<String, Value>,
}

#[derive(Debug, Deserialize)]
struct HttpScoreResponse {
    model_key: String,
    model_version: String,
    score: u8,
    label: String,
    explanations: Vec<ModelExplanation>,
    #[serde(default)]
    metadata: Value,
}

#[async_trait]
impl ModelScorer for HttpModelScorer {
    async fn score(&self, request: ModelScoreRequest) -> Result<ModelScore, ModelRuntimeError> {
        let features = request
            .features
            .into_iter()
            .map(|(name, value)| (name, value.value))
            .collect();
        let payload = HttpScoreRequest {
            run_id: request.run_id.to_string(),
            claim_id: request.claim_id.to_string(),
            model_key: request.model_key,
            model_version: request.model_version,
            features,
        };
        let target_url = request
            .endpoint_url
            .unwrap_or_else(|| format!("{}/score", self.base_url));
        let response = self
            .client
            .post(target_url)
            .json(&payload)
            .send()
            .await
            .map_err(|_| ModelRuntimeError::ServiceUnavailable)?;
        if !response.status().is_success() {
            return Err(ModelRuntimeError::ServiceUnavailable);
        }
        let body = response
            .json::<HttpScoreResponse>()
            .await
            .map_err(|error| ModelRuntimeError::InvalidResponse(error.to_string()))?;
        Ok(ModelScore {
            model_key: body.model_key,
            model_version: body.model_version,
            runtime_kind: "python_http".into(),
            execution_provider: "cpu".into(),
            score: body.score,
            label: body.label,
            explanations: body.explanations,
            metadata: body.metadata,
            latency_ms: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fwa_features::FeatureValue;
    use std::collections::BTreeMap;

    #[tokio::test]
    async fn heuristic_scorer_maps_amount_ratio_to_score() {
        let mut features = BTreeMap::new();
        features.insert(
            "claim_amount_to_limit_ratio".into(),
            FeatureValue {
                name: "claim_amount_to_limit_ratio".into(),
                version: 1,
                value: serde_json::json!(0.82),
                evidence_refs: vec![],
            },
        );

        let scorer = HeuristicModelScorer;
        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_test"),
                claim_id: ClaimId::from_external("CLM-1"),
                model_key: "baseline_fwa".into(),
                model_version: "0.1.0".into(),
                endpoint_url: None,
                features,
            })
            .await
            .unwrap();

        assert_eq!(result.score, 82);
        assert_eq!(result.model_version, "0.1.0");
        assert_eq!(result.runtime_kind, "heuristic");
        assert_eq!(
            result.metadata["fraud_probability"],
            serde_json::json!(0.82)
        );
        assert_eq!(
            result.metadata["abuse_probability"],
            serde_json::json!(0.574)
        );
        assert_eq!(
            result.metadata["waste_probability"],
            serde_json::json!(0.328)
        );
    }

    #[test]
    fn http_scorer_normalizes_base_url() {
        let scorer = HttpModelScorer::new("http://localhost:8001/");
        assert_eq!(scorer.base_url, "http://localhost:8001");
    }
}

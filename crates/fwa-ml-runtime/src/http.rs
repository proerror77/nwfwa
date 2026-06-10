use super::{ModelExplanation, ModelRuntimeError, ModelScore, ModelScoreRequest, ModelScorer};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct HttpModelScorer {
    client: reqwest::Client,
    pub(crate) base_url: String,
}

impl HttpModelScorer {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("HTTP model scorer client configuration should be valid"),
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
        let started_at = Instant::now();
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
        if body.model_key != payload.model_key || body.model_version != payload.model_version {
            return Err(ModelRuntimeError::InvalidResponse(format!(
                "model identity mismatch: expected {}:{}, got {}:{}",
                payload.model_key, payload.model_version, body.model_key, body.model_version
            )));
        }
        if body.score > 100 {
            return Err(ModelRuntimeError::InvalidResponse(format!(
                "model score out of range: {}",
                body.score
            )));
        }
        Ok(ModelScore {
            model_key: body.model_key,
            model_version: body.model_version,
            runtime_kind: "python_http".into(),
            execution_provider: "cpu".into(),
            score: body.score,
            label: body.label,
            explanations: body.explanations,
            metadata: body.metadata,
            latency_ms: started_at
                .elapsed()
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX),
        })
    }
}

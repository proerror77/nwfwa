use crate::types::{
    ModelExplanation, ModelRuntimeError, ModelScore, ModelScoreRequest, ModelScorer,
};
use crate::verify::{sha256_hex, verify_artifact_signature};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ArtifactModelScorer {
    pub(crate) artifact_uri: String,
    expected_sha256: Option<String>,
    version_lock: Option<String>,
    expected_signature: Option<String>,
    signing_key: Option<String>,
}

impl ArtifactModelScorer {
    pub fn new(artifact_uri: impl Into<String>) -> Self {
        Self {
            artifact_uri: artifact_uri.into(),
            expected_sha256: None,
            version_lock: None,
            expected_signature: None,
            signing_key: None,
        }
    }

    pub fn with_expected_sha256(mut self, expected_sha256: impl Into<String>) -> Self {
        self.expected_sha256 = Some(expected_sha256.into());
        self
    }

    pub fn with_version_lock(mut self, version_lock: impl Into<String>) -> Self {
        self.version_lock = Some(version_lock.into());
        self
    }

    pub fn with_signature(
        mut self,
        expected_signature: impl Into<String>,
        signing_key: impl Into<String>,
    ) -> Self {
        self.expected_signature = Some(expected_signature.into());
        self.signing_key = Some(signing_key.into());
        self
    }

    pub fn from_env(
        artifact_uri: impl Into<String>,
        version_lock: Option<String>,
        expected_sha256: Option<String>,
        expected_signature: Option<String>,
        signing_key: Option<String>,
    ) -> Self {
        Self {
            artifact_uri: artifact_uri.into(),
            expected_sha256: expected_sha256.filter(|value| !value.trim().is_empty()),
            version_lock: version_lock.filter(|value| !value.trim().is_empty()),
            expected_signature: expected_signature.filter(|value| !value.trim().is_empty()),
            signing_key: signing_key.filter(|value| !value.trim().is_empty()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct LogisticRegressionArtifact {
    pub(crate) model_key: String,
    pub(crate) model_version: String,
    #[serde(default = "default_artifact_runtime_kind")]
    pub(crate) runtime_kind: String,
    #[serde(default = "default_execution_provider")]
    pub(crate) execution_provider: String,
    #[serde(default = "default_threshold")]
    pub(crate) threshold: f64,
    pub(crate) feature_columns: Vec<String>,
    pub(crate) intercept: f64,
    pub(crate) coefficients: BTreeMap<String, f64>,
}

fn default_artifact_runtime_kind() -> String {
    "rust_logistic_regression".into()
}

fn default_execution_provider() -> String {
    "cpu".into()
}

fn default_threshold() -> f64 {
    0.5
}

pub(crate) fn local_artifact_path(artifact_uri: &str) -> Result<&str, ModelRuntimeError> {
    if artifact_uri.is_empty() {
        return Err(ModelRuntimeError::InvalidResponse(
            "artifact URI is empty".into(),
        ));
    }
    Ok(artifact_uri
        .strip_prefix("artifact://")
        .or_else(|| artifact_uri.strip_prefix("file://"))
        .unwrap_or(artifact_uri))
}

pub(crate) fn sigmoid(value: f64) -> f64 {
    1.0 / (1.0 + (-value).exp())
}

pub(crate) fn round_probability(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

#[async_trait]
impl ModelScorer for ArtifactModelScorer {
    async fn score(&self, request: ModelScoreRequest) -> Result<ModelScore, ModelRuntimeError> {
        let started_at = Instant::now();
        let artifact_path = local_artifact_path(&self.artifact_uri)?;
        let artifact_bytes = fs::read(artifact_path)
            .map_err(|error| ModelRuntimeError::InvalidResponse(error.to_string()))?;
        let artifact_sha256 = sha256_hex(&artifact_bytes);

        if let Some(expected_sha256) = &self.expected_sha256 {
            if expected_sha256 != &artifact_sha256 {
                return Err(ModelRuntimeError::InvalidResponse(format!(
                    "artifact checksum mismatch: expected {expected_sha256}, got {artifact_sha256}"
                )));
            }
        }

        let artifact: LogisticRegressionArtifact = serde_json::from_slice(&artifact_bytes)
            .map_err(|error| ModelRuntimeError::InvalidResponse(error.to_string()))?;
        if artifact.model_key != request.model_key
            || artifact.model_version != request.model_version
        {
            return Err(ModelRuntimeError::InvalidResponse(format!(
                "model identity mismatch: expected {}:{}, got {}:{}",
                request.model_key,
                request.model_version,
                artifact.model_key,
                artifact.model_version
            )));
        }

        if let Some(version_lock) = &self.version_lock {
            if version_lock != &artifact.model_version {
                return Err(ModelRuntimeError::InvalidResponse(format!(
                    "serving version lock mismatch: expected {version_lock}, got {}",
                    artifact.model_version
                )));
            }
        }
        let signature_status = verify_artifact_signature(
            &artifact.model_key,
            &artifact.model_version,
            &artifact_sha256,
            self.expected_signature.as_deref(),
            self.signing_key.as_deref(),
        )?;

        let mut logit = artifact.intercept;
        let mut explanations = Vec::new();
        for feature_name in &artifact.feature_columns {
            let feature_value = request
                .features
                .get(feature_name)
                .and_then(|feature| feature.value.as_f64())
                .unwrap_or(0.0);
            let coefficient = artifact
                .coefficients
                .get(feature_name)
                .copied()
                .unwrap_or(0.0);
            let contribution = feature_value * coefficient;
            logit += contribution;
            explanations.push(ModelExplanation {
                feature: feature_name.clone(),
                direction: if contribution >= 0.0 {
                    "increases_risk".into()
                } else {
                    "decreases_risk".into()
                },
                contribution,
                reason: "Rust artifact logistic-regression contribution".into(),
            });
        }

        let probability = sigmoid(logit);
        let score = (probability * 100.0).round().clamp(0.0, 100.0) as u8;
        let version_lock_status = if self.version_lock.is_some() {
            "passed"
        } else {
            "not_configured"
        };
        let serving_version_lock = self
            .version_lock
            .as_deref()
            .unwrap_or(&artifact.model_version)
            .to_string();

        Ok(ModelScore {
            model_key: artifact.model_key,
            model_version: artifact.model_version,
            runtime_kind: artifact.runtime_kind,
            execution_provider: artifact.execution_provider,
            score,
            label: if probability >= artifact.threshold {
                "HIGH_RISK"
            } else {
                "LOW_RISK"
            }
            .into(),
            explanations,
            metadata: serde_json::json!({
                "artifact_uri": self.artifact_uri,
                "artifact_sha256": artifact_sha256,
                "artifact_integrity_status": "passed",
                "artifact_signature_status": signature_status,
                "serving_version_lock": serving_version_lock,
                "serving_version_lock_status": version_lock_status,
                "feature_count": artifact.feature_columns.len(),
                "fraud_probability": round_probability(probability),
                "threshold": artifact.threshold
            }),
            latency_ms: started_at
                .elapsed()
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX),
        })
    }
}

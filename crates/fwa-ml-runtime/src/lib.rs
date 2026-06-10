use async_trait::async_trait;
use hmac::{Hmac, Mac};
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    value::Tensor,
};
use serde::Deserialize;
use std::collections::{btree_map::Entry, BTreeMap};
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::Instant;

mod types;
pub use types::{ModelExplanation, ModelRuntimeError, ModelScore, ModelScoreRequest, ModelScorer};

#[derive(Debug, Clone)]
pub struct ArtifactModelScorer {
    artifact_uri: String,
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

#[derive(Clone)]
pub struct ServingManifestModelScorer {
    manifest_uri: String,
    signing_key: Option<String>,
    onnx_sessions: Arc<Mutex<BTreeMap<String, CachedOnnxSession>>>,
}

impl std::fmt::Debug for ServingManifestModelScorer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ServingManifestModelScorer")
            .field("manifest_uri", &self.manifest_uri)
            .field(
                "signing_key",
                &self.signing_key.as_ref().map(|_| "<configured>"),
            )
            .finish_non_exhaustive()
    }
}

struct CachedOnnxSession {
    session: Session,
    input_name: String,
}

impl ServingManifestModelScorer {
    pub fn new(manifest_uri: impl Into<String>) -> Self {
        Self {
            manifest_uri: manifest_uri.into(),
            signing_key: None,
            onnx_sessions: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub fn with_signing_key(mut self, signing_key: impl Into<String>) -> Self {
        self.signing_key = Some(signing_key.into());
        self
    }

    pub fn from_env(manifest_uri: impl Into<String>, signing_key: Option<String>) -> Self {
        Self {
            manifest_uri: manifest_uri.into(),
            signing_key: signing_key.filter(|value| !value.trim().is_empty()),
            onnx_sessions: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

#[derive(Debug, Deserialize)]
struct LogisticRegressionArtifact {
    model_key: String,
    model_version: String,
    #[serde(default = "default_artifact_runtime_kind")]
    runtime_kind: String,
    #[serde(default = "default_execution_provider")]
    execution_provider: String,
    #[serde(default = "default_threshold")]
    threshold: f64,
    feature_columns: Vec<String>,
    intercept: f64,
    coefficients: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct ServingManifest {
    model_key: String,
    model_version: String,
    runtime_kind: String,
    artifact_uri: String,
    artifact_sha256: String,
    artifact_signature: Option<String>,
    version_lock: String,
    feature_columns: Vec<String>,
    threshold: f64,
    training_artifact_uri: Option<String>,
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

#[async_trait]
impl ModelScorer for ServingManifestModelScorer {
    async fn score(&self, request: ModelScoreRequest) -> Result<ModelScore, ModelRuntimeError> {
        let manifest_path = local_artifact_path(&self.manifest_uri)?;
        let manifest_bytes = fs::read(manifest_path)
            .map_err(|error| ModelRuntimeError::InvalidResponse(error.to_string()))?;
        let manifest: ServingManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|error| ModelRuntimeError::InvalidResponse(error.to_string()))?;
        validate_serving_manifest(&manifest, &request)?;
        validate_manifest_feature_order(&manifest, &request)?;

        match manifest.runtime_kind.as_str() {
            "rust_logistic_regression" => {
                let mut result = ArtifactModelScorer::new(manifest.artifact_uri.clone())
                    .with_expected_sha256(manifest.artifact_sha256.clone())
                    .with_version_lock(manifest.version_lock.clone());
                if let Some(signature) = manifest.artifact_signature.clone() {
                    if let Some(signing_key) = self.signing_key.clone() {
                        result = result.with_signature(signature, signing_key);
                    } else {
                        return Err(ModelRuntimeError::InvalidResponse(
                            "model artifact signature key missing".into(),
                        ));
                    }
                }
                let mut score = result.score(request).await?;
                merge_serving_manifest_metadata(&mut score, &self.manifest_uri, &manifest);
                Ok(score)
            }
            "rust_onnx" | "xgboost_onnx" | "lightgbm_onnx" | "deep_learning_onnx" => {
                score_onnx_manifest(
                    &manifest,
                    &request,
                    &self.manifest_uri,
                    self.signing_key.as_deref(),
                    &self.onnx_sessions,
                )
            }
            "xgboost_classifier" | "lightgbm_classifier" => {
                Err(ModelRuntimeError::InvalidResponse(format!(
                    "{} is a training artifact runtime, not a Rust serving runtime; export ONNX or use the governed HTTP fallback",
                    manifest.runtime_kind
                )))
            }
            other => Err(ModelRuntimeError::InvalidResponse(format!(
                "unsupported serving manifest runtime_kind: {other}"
            ))),
        }
    }
}

fn score_onnx_manifest(
    manifest: &ServingManifest,
    request: &ModelScoreRequest,
    manifest_uri: &str,
    signing_key: Option<&str>,
    onnx_sessions: &Mutex<BTreeMap<String, CachedOnnxSession>>,
) -> Result<ModelScore, ModelRuntimeError> {
    let started_at = Instant::now();
    validate_onnx_artifact(manifest)?;
    let signature_status = verify_artifact_signature(
        &manifest.model_key,
        &manifest.model_version,
        &manifest.artifact_sha256,
        manifest.artifact_signature.as_deref(),
        signing_key,
    )?;
    let artifact_path = local_artifact_path(&manifest.artifact_uri)?;
    let feature_values = manifest
        .feature_columns
        .iter()
        .map(|feature_name| {
            request
                .features
                .get(feature_name)
                .and_then(|feature| feature.value.as_f64())
                .map(|value| value as f32)
                .ok_or_else(|| {
                    ModelRuntimeError::InvalidResponse(format!(
                        "serving manifest feature must be numeric: {feature_name}"
                    ))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let feature_count = feature_values.len();
    let input_tensor = Tensor::from_array(([1usize, feature_count], feature_values))
        .map_err(onnx_runtime_error)?;
    let cache_key = onnx_session_cache_key(manifest);
    let mut sessions = onnx_sessions.lock().map_err(|_| {
        ModelRuntimeError::InvalidResponse("ONNX session cache lock poisoned".into())
    })?;
    let mut cache_status = "hit";
    let cached = match sessions.entry(cache_key) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => {
            cache_status = "miss";
            entry.insert(build_cached_onnx_session(artifact_path)?)
        }
    };
    let input_name = cached.input_name.clone();
    let outputs = cached
        .session
        .run(ort::inputs![input_name.as_str() => input_tensor])
        .map_err(onnx_runtime_error)?;
    let (probability, output_name) = extract_positive_probability(&outputs)?;
    let probability = normalize_probability(probability)?;
    let score = (probability * 100.0).round().clamp(0.0, 100.0) as u8;

    Ok(ModelScore {
        model_key: manifest.model_key.clone(),
        model_version: manifest.model_version.clone(),
        runtime_kind: manifest.runtime_kind.clone(),
        execution_provider: "onnxruntime_cpu".into(),
        score,
        label: if probability >= manifest.threshold {
            "HIGH_RISK"
        } else {
            "LOW_RISK"
        }
        .into(),
        explanations: Vec::new(),
        metadata: serde_json::json!({
            "artifact_uri": manifest.artifact_uri,
            "artifact_sha256": manifest.artifact_sha256,
            "artifact_integrity_status": "passed",
            "artifact_signature_status": signature_status,
            "serving_manifest_uri": manifest_uri,
            "serving_manifest_status": "passed",
            "serving_runtime_kind": manifest.runtime_kind,
            "serving_feature_columns": manifest.feature_columns,
            "serving_threshold": manifest.threshold,
            "serving_version_lock": manifest.version_lock,
            "serving_version_lock_status": "passed",
            "training_artifact_uri": manifest.training_artifact_uri,
            "feature_count": feature_count,
            "fraud_probability": round_probability(probability),
            "onnx_input_name": input_name,
            "onnx_output_name": output_name,
            "onnx_session_cache_status": cache_status
        }),
        latency_ms: started_at
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX),
    })
}

fn onnx_session_cache_key(manifest: &ServingManifest) -> String {
    format!("{}|{}", manifest.artifact_uri, manifest.artifact_sha256)
}

fn build_cached_onnx_session(artifact_path: &str) -> Result<CachedOnnxSession, ModelRuntimeError> {
    let session = Session::builder()
        .map_err(onnx_runtime_error)?
        .with_optimization_level(GraphOptimizationLevel::Level3)
        .map_err(onnx_runtime_error)?
        .commit_from_file(artifact_path)
        .map_err(onnx_runtime_error)?;
    let input_name = session
        .inputs()
        .first()
        .map(|input| input.name().to_string())
        .ok_or_else(|| ModelRuntimeError::InvalidResponse("ONNX model has no inputs".into()))?;
    Ok(CachedOnnxSession {
        session,
        input_name,
    })
}

fn local_artifact_path(artifact_uri: &str) -> Result<&str, ModelRuntimeError> {
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

fn validate_serving_manifest(
    manifest: &ServingManifest,
    request: &ModelScoreRequest,
) -> Result<(), ModelRuntimeError> {
    if manifest.model_key != request.model_key || manifest.model_version != request.model_version {
        return Err(ModelRuntimeError::InvalidResponse(format!(
            "serving manifest model identity mismatch: expected {}:{}, got {}:{}",
            request.model_key, request.model_version, manifest.model_key, manifest.model_version
        )));
    }
    if manifest.version_lock != manifest.model_version {
        return Err(ModelRuntimeError::InvalidResponse(format!(
            "serving manifest version_lock mismatch: expected {}, got {}",
            manifest.model_version, manifest.version_lock
        )));
    }
    if manifest.artifact_sha256.trim().is_empty() {
        return Err(ModelRuntimeError::InvalidResponse(
            "serving manifest artifact_sha256 is required".into(),
        ));
    }
    if manifest.feature_columns.is_empty() {
        return Err(ModelRuntimeError::InvalidResponse(
            "serving manifest feature_columns must not be empty".into(),
        ));
    }
    Ok(())
}

fn validate_manifest_feature_order(
    manifest: &ServingManifest,
    request: &ModelScoreRequest,
) -> Result<(), ModelRuntimeError> {
    for feature_name in &manifest.feature_columns {
        let Some(feature) = request.features.get(feature_name) else {
            return Err(ModelRuntimeError::InvalidResponse(format!(
                "serving manifest feature missing from request: {feature_name}"
            )));
        };
        if feature.value.as_f64().is_none() {
            return Err(ModelRuntimeError::InvalidResponse(format!(
                "serving manifest feature must be numeric: {feature_name}"
            )));
        }
    }
    Ok(())
}

fn validate_onnx_artifact(manifest: &ServingManifest) -> Result<(), ModelRuntimeError> {
    if !manifest.artifact_uri.ends_with(".onnx") {
        return Err(ModelRuntimeError::InvalidResponse(format!(
            "{} requires an .onnx artifact_uri",
            manifest.runtime_kind
        )));
    }
    let artifact_path = local_artifact_path(&manifest.artifact_uri)?;
    let artifact_bytes = fs::read(artifact_path)
        .map_err(|error| ModelRuntimeError::InvalidResponse(error.to_string()))?;
    let artifact_sha256 = sha256_hex(&artifact_bytes);
    if artifact_sha256 != manifest.artifact_sha256 {
        return Err(ModelRuntimeError::InvalidResponse(format!(
            "ONNX artifact checksum mismatch: expected {}, got {}",
            manifest.artifact_sha256, artifact_sha256
        )));
    }
    Ok(())
}

fn extract_positive_probability(
    outputs: &ort::session::SessionOutputs<'_>,
) -> Result<(f64, String), ModelRuntimeError> {
    if let Some(output) = outputs.get("probabilities") {
        if let Ok((shape, values)) = output.try_extract_tensor::<f32>() {
            if let Some(probability) = positive_probability_from_tensor(shape, values) {
                return Ok((probability, "probabilities".into()));
            }
        }
    }

    let output_values = outputs.iter().collect::<Vec<_>>();
    for (name, output) in output_values.into_iter().rev() {
        if let Ok((shape, values)) = output.try_extract_tensor::<f32>() {
            if let Some(probability) = positive_probability_from_tensor(shape, values) {
                return Ok((probability, name.to_string()));
            }
        }
    }

    Err(ModelRuntimeError::InvalidResponse(
        "ONNX output does not expose usable positive-class probabilities".into(),
    ))
}

fn positive_probability_from_tensor(shape: &[i64], values: &[f32]) -> Option<f64> {
    match shape {
        [1, columns] if *columns >= 2 && values.len() >= *columns as usize => {
            Some(values[1] as f64)
        }
        [_rows, columns] if *columns >= 2 && values.len() >= *columns as usize => {
            Some(values[1] as f64)
        }
        [1] if !values.is_empty() => Some(values[0] as f64),
        [_rows] if !values.is_empty() => Some(values[0] as f64),
        [] if !values.is_empty() => Some(values[0] as f64),
        _ => None,
    }
}

fn normalize_probability(value: f64) -> Result<f64, ModelRuntimeError> {
    if !value.is_finite() {
        return Err(ModelRuntimeError::InvalidResponse(
            "ONNX probability output is not finite".into(),
        ));
    }
    if !(-1e-6..=1.0 + 1e-6).contains(&value) {
        return Err(ModelRuntimeError::InvalidResponse(format!(
            "ONNX probability output is out of range: {value}"
        )));
    }
    Ok(value.clamp(0.0, 1.0))
}

fn onnx_runtime_error<T>(error: ort::Error<T>) -> ModelRuntimeError {
    ModelRuntimeError::InvalidResponse(format!("ONNX runtime error: {error}"))
}

fn merge_serving_manifest_metadata(
    score: &mut ModelScore,
    manifest_uri: &str,
    manifest: &ServingManifest,
) {
    if let Some(metadata) = score.metadata.as_object_mut() {
        metadata.insert(
            "serving_manifest_uri".into(),
            serde_json::json!(manifest_uri),
        );
        metadata.insert(
            "serving_manifest_status".into(),
            serde_json::json!("passed"),
        );
        metadata.insert(
            "serving_runtime_kind".into(),
            serde_json::json!(manifest.runtime_kind),
        );
        metadata.insert(
            "serving_feature_columns".into(),
            serde_json::json!(manifest.feature_columns.clone()),
        );
        metadata.insert(
            "serving_threshold".into(),
            serde_json::json!(manifest.threshold),
        );
        if let Some(training_artifact_uri) = &manifest.training_artifact_uri {
            metadata.insert(
                "training_artifact_uri".into(),
                serde_json::json!(training_artifact_uri),
            );
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    let digest = Sha256::digest(bytes);
    format!("sha256:{digest:x}")
}

fn verify_artifact_signature(
    model_key: &str,
    model_version: &str,
    artifact_sha256: &str,
    expected_signature: Option<&str>,
    signing_key: Option<&str>,
) -> Result<&'static str, ModelRuntimeError> {
    let Some(expected_signature) = expected_signature else {
        return Ok("not_configured");
    };
    let Some(signing_key) = signing_key else {
        return Err(ModelRuntimeError::InvalidResponse(
            "model artifact signature key missing".into(),
        ));
    };
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(signing_key.as_bytes())
        .map_err(|error| ModelRuntimeError::InvalidResponse(error.to_string()))?;
    mac.update(format!("{model_key}:{model_version}:{artifact_sha256}").as_bytes());
    let actual_signature = format!("hmac-sha256:{}", to_hex(&mac.finalize().into_bytes()));
    if actual_signature != expected_signature {
        return Err(ModelRuntimeError::InvalidResponse(
            "model artifact signature mismatch".into(),
        ));
    }
    Ok("passed")
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn sigmoid(value: f64) -> f64 {
    1.0 / (1.0 + (-value).exp())
}

fn round_probability(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

mod heuristic;
pub use heuristic::HeuristicModelScorer;

mod http;
pub use http::HttpModelScorer;

#[cfg(test)]
mod tests;

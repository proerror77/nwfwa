use async_trait::async_trait;
use fwa_core::{ClaimId, ScoringRunId};
use fwa_features::FeatureMap;
use hmac::{Hmac, Mac};
use ort::{
    session::{builder::GraphOptimizationLevel, Session},
    value::Tensor,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{btree_map::Entry, BTreeMap};
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
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

#[cfg(test)]
mod tests {
    use super::*;
    use fwa_features::FeatureValue;
    use std::collections::BTreeMap;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::thread;
    use std::time::Duration;

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

    #[tokio::test]
    async fn http_scorer_records_request_latency() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 2048];
            let _ = stream.read(&mut buffer);
            thread::sleep(Duration::from_millis(5));
            let body = r#"{"model_key":"baseline_fwa","model_version":"0.1.0","score":74,"label":"HIGH_RISK","explanations":[],"metadata":{"fraud_probability":0.74}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let scorer = HttpModelScorer::new(format!("http://{address}"));
        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_http_latency"),
                claim_id: ClaimId::from_external("CLM-HTTP-LATENCY"),
                model_key: "baseline_fwa".into(),
                model_version: "0.1.0".into(),
                endpoint_url: None,
                features: BTreeMap::new(),
            })
            .await
            .unwrap();

        server.join().unwrap();
        assert_eq!(result.score, 74);
        assert!(result.latency_ms >= 5);
    }

    #[tokio::test]
    async fn http_scorer_rejects_mismatched_model_version() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 2048];
            let _ = stream.read(&mut buffer);
            let body = r#"{"model_key":"baseline_fwa","model_version":"0.2.0","score":74,"label":"HIGH_RISK","explanations":[],"metadata":{}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let scorer = HttpModelScorer::new(format!("http://{address}"));
        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_http_mismatch"),
                claim_id: ClaimId::from_external("CLM-HTTP-MISMATCH"),
                model_key: "baseline_fwa".into(),
                model_version: "0.1.0".into(),
                endpoint_url: None,
                features: BTreeMap::new(),
            })
            .await;

        server.join().unwrap();
        assert!(matches!(result, Err(ModelRuntimeError::InvalidResponse(_))));
    }

    #[tokio::test]
    async fn http_scorer_rejects_out_of_range_score() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 2048];
            let _ = stream.read(&mut buffer);
            let body = r#"{"model_key":"baseline_fwa","model_version":"0.1.0","score":101,"label":"HIGH_RISK","explanations":[],"metadata":{}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });

        let scorer = HttpModelScorer::new(format!("http://{address}"));
        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_http_score_range"),
                claim_id: ClaimId::from_external("CLM-HTTP-SCORE-RANGE"),
                model_key: "baseline_fwa".into(),
                model_version: "0.1.0".into(),
                endpoint_url: None,
                features: BTreeMap::new(),
            })
            .await;

        server.join().unwrap();
        assert!(matches!(result, Err(ModelRuntimeError::InvalidResponse(_))));
    }

    #[tokio::test]
    async fn artifact_scorer_scores_logistic_regression_json() {
        let artifact_path = write_artifact(
            "rust-logistic",
            serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.2.0-rust",
                "runtime_kind": "rust_logistic_regression",
                "execution_provider": "cpu",
                "threshold": 0.5,
                "feature_columns": ["claim_amount_to_limit_ratio", "provider_profile_score"],
                "intercept": -2.0,
                "coefficients": {
                    "claim_amount_to_limit_ratio": 4.0,
                    "provider_profile_score": 0.01
                }
            }),
        );
        let expected_sha256 = artifact_sha256(&artifact_path);
        let scorer = ArtifactModelScorer::new(artifact_path.to_string_lossy());
        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_artifact"),
                claim_id: ClaimId::from_external("CLM-ARTIFACT"),
                model_key: "baseline_fwa".into(),
                model_version: "0.2.0-rust".into(),
                endpoint_url: None,
                features: features([
                    ("claim_amount_to_limit_ratio", 0.8),
                    ("provider_profile_score", 20.0),
                ]),
            })
            .await
            .unwrap();

        assert_eq!(result.model_key, "baseline_fwa");
        assert_eq!(result.model_version, "0.2.0-rust");
        assert_eq!(result.runtime_kind, "rust_logistic_regression");
        assert_eq!(result.execution_provider, "cpu");
        assert_eq!(result.score, 80);
        assert_eq!(result.label, "HIGH_RISK");
        assert_eq!(
            result.metadata["artifact_sha256"],
            serde_json::json!(expected_sha256)
        );
        assert_eq!(
            result.metadata["artifact_integrity_status"],
            serde_json::json!("passed")
        );
        assert_eq!(
            result.metadata["serving_version_lock_status"],
            serde_json::json!("not_configured")
        );
        assert_eq!(result.metadata["feature_count"], serde_json::json!(2));

        fs::remove_file(artifact_path).unwrap();
    }

    #[tokio::test]
    async fn artifact_scorer_rejects_checksum_mismatch() {
        let artifact_path = write_artifact(
            "rust-logistic-checksum",
            serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.2.0-rust",
                "feature_columns": ["claim_amount_to_limit_ratio"],
                "intercept": 0.0,
                "coefficients": {"claim_amount_to_limit_ratio": 1.0}
            }),
        );
        let scorer = ArtifactModelScorer::new(artifact_path.to_string_lossy())
            .with_expected_sha256("sha256:wrong");

        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_artifact_checksum"),
                claim_id: ClaimId::from_external("CLM-ARTIFACT-CHECKSUM"),
                model_key: "baseline_fwa".into(),
                model_version: "0.2.0-rust".into(),
                endpoint_url: None,
                features: BTreeMap::new(),
            })
            .await;

        assert!(matches!(result, Err(ModelRuntimeError::InvalidResponse(_))));
        fs::remove_file(artifact_path).unwrap();
    }

    #[tokio::test]
    async fn artifact_scorer_rejects_version_lock_mismatch() {
        let artifact_path = write_artifact(
            "rust-logistic-version",
            serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.2.0-rust",
                "feature_columns": ["claim_amount_to_limit_ratio"],
                "intercept": 0.0,
                "coefficients": {"claim_amount_to_limit_ratio": 1.0}
            }),
        );
        let scorer = ArtifactModelScorer::new(artifact_path.to_string_lossy())
            .with_version_lock("0.3.0-active");

        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_artifact_version"),
                claim_id: ClaimId::from_external("CLM-ARTIFACT-VERSION"),
                model_key: "baseline_fwa".into(),
                model_version: "0.2.0-rust".into(),
                endpoint_url: None,
                features: BTreeMap::new(),
            })
            .await;

        assert!(matches!(result, Err(ModelRuntimeError::InvalidResponse(_))));
        fs::remove_file(artifact_path).unwrap();
    }

    #[tokio::test]
    async fn artifact_scorer_verifies_hmac_signature() {
        let artifact_path = write_artifact(
            "rust-logistic-signature",
            serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.2.0-rust",
                "feature_columns": ["claim_amount_to_limit_ratio"],
                "intercept": 0.0,
                "coefficients": {"claim_amount_to_limit_ratio": 1.0}
            }),
        );
        let artifact_sha256 = artifact_sha256(&artifact_path);
        let signature = artifact_signature(
            "baseline_fwa",
            "0.2.0-rust",
            &artifact_sha256,
            "test-signing-key",
        );
        let scorer = ArtifactModelScorer::new(artifact_path.to_string_lossy())
            .with_signature(signature, "test-signing-key");

        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_artifact_signature"),
                claim_id: ClaimId::from_external("CLM-ARTIFACT-SIGNATURE"),
                model_key: "baseline_fwa".into(),
                model_version: "0.2.0-rust".into(),
                endpoint_url: None,
                features: BTreeMap::new(),
            })
            .await
            .unwrap();

        assert_eq!(
            result.metadata["artifact_signature_status"],
            serde_json::json!("passed")
        );
        fs::remove_file(artifact_path).unwrap();
    }

    #[tokio::test]
    async fn artifact_scorer_rejects_signature_without_key() {
        let artifact_path = write_artifact(
            "rust-logistic-missing-key",
            serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.2.0-rust",
                "feature_columns": ["claim_amount_to_limit_ratio"],
                "intercept": 0.0,
                "coefficients": {"claim_amount_to_limit_ratio": 1.0}
            }),
        );
        let scorer = ArtifactModelScorer::from_env(
            artifact_path.to_string_lossy(),
            None,
            None,
            Some("hmac-sha256:configured".into()),
            None,
        );

        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_artifact_missing_key"),
                claim_id: ClaimId::from_external("CLM-ARTIFACT-MISSING-KEY"),
                model_key: "baseline_fwa".into(),
                model_version: "0.2.0-rust".into(),
                endpoint_url: None,
                features: BTreeMap::new(),
            })
            .await;

        assert!(matches!(result, Err(ModelRuntimeError::InvalidResponse(_))));
        fs::remove_file(artifact_path).unwrap();
    }

    #[tokio::test]
    async fn serving_manifest_scorer_scores_rust_logistic_artifact() {
        let artifact_path = write_artifact(
            "serving-manifest-rust-logistic-artifact",
            serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.3.0-active",
                "runtime_kind": "rust_logistic_regression",
                "execution_provider": "cpu",
                "threshold": 0.5,
                "feature_columns": ["claim_amount_to_limit_ratio", "provider_profile_score"],
                "intercept": -1.5,
                "coefficients": {
                    "claim_amount_to_limit_ratio": 3.0,
                    "provider_profile_score": 0.02
                }
            }),
        );
        let artifact_sha256 = artifact_sha256(&artifact_path);
        let signature = artifact_signature(
            "baseline_fwa",
            "0.3.0-active",
            &artifact_sha256,
            "test-signing-key",
        );
        let manifest_path = write_artifact(
            "serving-manifest-rust-logistic",
            serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.3.0-active",
                "runtime_kind": "rust_logistic_regression",
                "artifact_uri": artifact_path.to_string_lossy(),
                "artifact_sha256": artifact_sha256,
                "artifact_signature": signature,
                "signature_algorithm": "hmac-sha256",
                "version_lock": "0.3.0-active",
                "feature_columns": ["claim_amount_to_limit_ratio", "provider_profile_score"],
                "threshold": 0.5,
                "training_artifact_uri": "/tmp/model.joblib"
            }),
        );
        let scorer = ServingManifestModelScorer::new(manifest_path.to_string_lossy())
            .with_signing_key("test-signing-key");

        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_serving_manifest"),
                claim_id: ClaimId::from_external("CLM-SERVING-MANIFEST"),
                model_key: "baseline_fwa".into(),
                model_version: "0.3.0-active".into(),
                endpoint_url: None,
                features: features([
                    ("claim_amount_to_limit_ratio", 0.82),
                    ("provider_profile_score", 18.0),
                ]),
            })
            .await
            .unwrap();

        assert_eq!(result.runtime_kind, "rust_logistic_regression");
        assert_eq!(
            result.metadata["serving_manifest_status"],
            serde_json::json!("passed")
        );
        assert_eq!(
            result.metadata["serving_runtime_kind"],
            serde_json::json!("rust_logistic_regression")
        );
        assert_eq!(
            result.metadata["training_artifact_uri"],
            serde_json::json!("/tmp/model.joblib")
        );
        assert_eq!(
            result.metadata["artifact_signature_status"],
            serde_json::json!("passed")
        );

        fs::remove_file(artifact_path).unwrap();
        fs::remove_file(manifest_path).unwrap();
    }

    #[tokio::test]
    async fn serving_manifest_scorer_rejects_missing_ordered_feature() {
        let artifact_path = write_artifact(
            "serving-manifest-missing-feature-artifact",
            serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.3.0-active",
                "feature_columns": ["claim_amount_to_limit_ratio"],
                "intercept": 0.0,
                "coefficients": {"claim_amount_to_limit_ratio": 1.0}
            }),
        );
        let manifest_path = write_artifact(
            "serving-manifest-missing-feature",
            serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.3.0-active",
                "runtime_kind": "rust_logistic_regression",
                "artifact_uri": artifact_path.to_string_lossy(),
                "artifact_sha256": artifact_sha256(&artifact_path),
                "version_lock": "0.3.0-active",
                "feature_columns": ["claim_amount_to_limit_ratio", "provider_profile_score"],
                "threshold": 0.5
            }),
        );
        let scorer = ServingManifestModelScorer::new(manifest_path.to_string_lossy());

        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_serving_manifest_missing_feature"),
                claim_id: ClaimId::from_external("CLM-SERVING-MANIFEST-MISSING-FEATURE"),
                model_key: "baseline_fwa".into(),
                model_version: "0.3.0-active".into(),
                endpoint_url: None,
                features: features([("claim_amount_to_limit_ratio", 0.82)]),
            })
            .await;

        assert!(matches!(result, Err(ModelRuntimeError::InvalidResponse(_))));
        fs::remove_file(artifact_path).unwrap();
        fs::remove_file(manifest_path).unwrap();
    }

    #[tokio::test]
    async fn serving_manifest_scorer_rejects_joblib_xgboost_as_rust_serving() {
        let artifact_path =
            write_artifact("serving-manifest-xgboost-joblib", serde_json::json!({}));
        let manifest_path = write_artifact(
            "serving-manifest-xgboost-joblib-manifest",
            serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.3.0-xgboost",
                "runtime_kind": "xgboost_classifier",
                "artifact_uri": artifact_path.to_string_lossy(),
                "artifact_sha256": artifact_sha256(&artifact_path),
                "version_lock": "0.3.0-xgboost",
                "feature_columns": ["claim_amount_to_limit_ratio"],
                "threshold": 0.5,
                "training_artifact_uri": artifact_path.to_string_lossy()
            }),
        );
        let scorer = ServingManifestModelScorer::new(manifest_path.to_string_lossy());

        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_serving_manifest_joblib"),
                claim_id: ClaimId::from_external("CLM-SERVING-MANIFEST-JOBLIB"),
                model_key: "baseline_fwa".into(),
                model_version: "0.3.0-xgboost".into(),
                endpoint_url: None,
                features: features([("claim_amount_to_limit_ratio", 0.82)]),
            })
            .await;

        let Err(ModelRuntimeError::InvalidResponse(message)) = result else {
            panic!("expected invalid response for xgboost joblib serving manifest");
        };
        assert!(message.contains("not a Rust serving runtime"));
        fs::remove_file(artifact_path).unwrap();
        fs::remove_file(manifest_path).unwrap();
    }

    #[tokio::test]
    async fn serving_manifest_scorer_reaches_onnx_runtime_after_contract_validation() {
        let onnx_path = std::env::temp_dir().join(format!(
            "nwfwa-serving-manifest-onnx-{}.onnx",
            ScoringRunId::new()
        ));
        fs::write(&onnx_path, b"fake onnx bytes for contract validation").unwrap();
        let manifest_path = write_artifact(
            "serving-manifest-xgboost-onnx",
            serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.3.0-xgboost-onnx",
                "runtime_kind": "xgboost_onnx",
                "artifact_uri": onnx_path.to_string_lossy(),
                "artifact_sha256": artifact_sha256(&onnx_path),
                "version_lock": "0.3.0-xgboost-onnx",
                "feature_columns": ["claim_amount_to_limit_ratio"],
                "threshold": 0.5
            }),
        );
        let scorer = ServingManifestModelScorer::new(manifest_path.to_string_lossy());

        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_serving_manifest_onnx"),
                claim_id: ClaimId::from_external("CLM-SERVING-MANIFEST-ONNX"),
                model_key: "baseline_fwa".into(),
                model_version: "0.3.0-xgboost-onnx".into(),
                endpoint_url: None,
                features: features([("claim_amount_to_limit_ratio", 0.82)]),
            })
            .await;

        let Err(ModelRuntimeError::InvalidResponse(message)) = result else {
            panic!("expected invalid response for fake ONNX model");
        };
        assert!(message.contains("ONNX runtime error"));
        fs::remove_file(onnx_path).unwrap();
        fs::remove_file(manifest_path).unwrap();
    }

    #[tokio::test]
    async fn serving_manifest_accepts_deep_learning_onnx_runtime() {
        let onnx_path = std::env::temp_dir().join(format!(
            "nwfwa-serving-manifest-deep-learning-onnx-{}.onnx",
            ScoringRunId::new()
        ));
        fs::write(
            &onnx_path,
            b"fake deep learning onnx bytes for contract validation",
        )
        .unwrap();
        let manifest_path = write_artifact(
            "serving-manifest-deep-learning-onnx",
            serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.4.0-deep-learning-onnx",
                "runtime_kind": "deep_learning_onnx",
                "artifact_uri": onnx_path.to_string_lossy(),
                "artifact_sha256": artifact_sha256(&onnx_path),
                "version_lock": "0.4.0-deep-learning-onnx",
                "feature_columns": ["claim_amount_to_limit_ratio"],
                "threshold": 0.5
            }),
        );
        let scorer = ServingManifestModelScorer::new(manifest_path.to_string_lossy());

        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_serving_manifest_deep_learning_onnx"),
                claim_id: ClaimId::from_external("CLM-SERVING-MANIFEST-DL-ONNX"),
                model_key: "baseline_fwa".into(),
                model_version: "0.4.0-deep-learning-onnx".into(),
                endpoint_url: None,
                features: features([("claim_amount_to_limit_ratio", 0.82)]),
            })
            .await;

        let Err(ModelRuntimeError::InvalidResponse(message)) = result else {
            panic!("expected invalid response for fake ONNX model");
        };
        assert!(message.contains("ONNX runtime error"));
        fs::remove_file(onnx_path).unwrap();
        fs::remove_file(manifest_path).unwrap();
    }

    #[test]
    fn onnx_session_cache_key_binds_artifact_uri_and_checksum() {
        let base = ServingManifest {
            model_key: "baseline_fwa".into(),
            model_version: "0.3.0-xgboost-onnx".into(),
            runtime_kind: "xgboost_onnx".into(),
            artifact_uri: "/tmp/model-a.onnx".into(),
            artifact_sha256: "sha256:a".into(),
            artifact_signature: None,
            version_lock: "0.3.0-xgboost-onnx".into(),
            feature_columns: vec!["claim_amount_to_limit_ratio".into()],
            threshold: 0.5,
            training_artifact_uri: None,
        };
        let mut changed_checksum = base.clone();
        changed_checksum.artifact_sha256 = "sha256:b".into();
        let mut changed_uri = base.clone();
        changed_uri.artifact_uri = "/tmp/model-b.onnx".into();

        assert_ne!(
            onnx_session_cache_key(&base),
            onnx_session_cache_key(&changed_checksum)
        );
        assert_ne!(
            onnx_session_cache_key(&base),
            onnx_session_cache_key(&changed_uri)
        );
    }

    #[test]
    fn positive_probability_from_tensor_prefers_positive_class_column() {
        assert_eq!(
            positive_probability_from_tensor(&[1, 2], &[0.25, 0.75]),
            Some(0.75)
        );
        let probability = positive_probability_from_tensor(&[3], &[0.61, 0.2, 0.1]).unwrap();
        assert!((probability - 0.61).abs() < 1e-6);
        assert_eq!(positive_probability_from_tensor(&[1, 1], &[0.2]), None);
    }

    fn features(
        values: impl IntoIterator<Item = (&'static str, f64)>,
    ) -> BTreeMap<String, FeatureValue> {
        values
            .into_iter()
            .map(|(name, value)| {
                (
                    name.to_string(),
                    FeatureValue {
                        name: name.to_string(),
                        version: 1,
                        value: serde_json::json!(value),
                        evidence_refs: vec![],
                    },
                )
            })
            .collect()
    }

    fn write_artifact(name: &str, payload: serde_json::Value) -> PathBuf {
        let path = std::env::temp_dir().join(format!("nwfwa-{name}-{}.json", ScoringRunId::new()));
        fs::write(&path, serde_json::to_vec(&payload).unwrap()).unwrap();
        path
    }

    fn artifact_sha256(path: &PathBuf) -> String {
        use sha2::{Digest, Sha256};

        let digest = Sha256::digest(fs::read(path).unwrap());
        format!("sha256:{digest:x}")
    }

    fn artifact_signature(
        model_key: &str,
        model_version: &str,
        artifact_sha256: &str,
        signing_key: &str,
    ) -> String {
        let mut mac = Hmac::<sha2::Sha256>::new_from_slice(signing_key.as_bytes()).unwrap();
        mac.update(format!("{model_key}:{model_version}:{artifact_sha256}").as_bytes());
        format!("hmac-sha256:{}", to_hex(&mac.finalize().into_bytes()))
    }
}

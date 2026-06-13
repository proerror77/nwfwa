use crate::{
    config::AppConfig,
    repository::{InMemoryScoringRepository, ModelVersionRecord, SharedRepository},
};
use axum::Router;
use fwa_ml_runtime::{
    ArtifactModelScorer, HeuristicModelScorer, HttpModelScorer, ModelScorer,
    ServingManifestModelScorer,
};
use fwa_rules::Rule;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

const SCORING_LOOKUP_CACHE_TTL: Duration = Duration::from_secs(30);

mod app_routes;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub scorer: Arc<dyn ModelScorer>,
    pub repository: SharedRepository,
    pub scoring_lookup_cache: ScoringLookupCache,
}

#[derive(Clone)]
pub struct ScoringLookupCache {
    inner: Arc<RwLock<ScoringLookupCacheState>>,
    ttl: Duration,
}

#[derive(Default)]
struct ScoringLookupCacheState {
    active_rules: Option<CachedLookup<Vec<Rule>>>,
    active_models: HashMap<String, CachedLookup<ModelVersionRecord>>,
}

struct CachedLookup<T> {
    value: T,
    expires_at: Instant,
}

impl Default for ScoringLookupCache {
    fn default() -> Self {
        Self::new(SCORING_LOOKUP_CACHE_TTL)
    }
}

impl ScoringLookupCache {
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: Arc::new(RwLock::new(ScoringLookupCacheState::default())),
            ttl,
        }
    }

    pub async fn active_rules(&self) -> Option<Vec<Rule>> {
        let cache = self.inner.read().await;
        cache
            .active_rules
            .as_ref()
            .filter(|entry| entry.expires_at > Instant::now())
            .map(|entry| entry.value.clone())
    }

    pub async fn store_active_rules(&self, rules: Vec<Rule>) {
        let mut cache = self.inner.write().await;
        cache.active_rules = Some(CachedLookup {
            value: rules,
            expires_at: Instant::now() + self.ttl,
        });
    }

    pub async fn active_model(&self, review_mode: &str) -> Option<ModelVersionRecord> {
        let cache = self.inner.read().await;
        cache
            .active_models
            .get(review_mode)
            .filter(|entry| entry.expires_at > Instant::now())
            .map(|entry| entry.value.clone())
    }

    pub async fn store_active_model(&self, review_mode: &str, model: ModelVersionRecord) {
        let mut cache = self.inner.write().await;
        cache.active_models.insert(
            review_mode.to_string(),
            CachedLookup {
                value: model,
                expires_at: Instant::now() + self.ttl,
            },
        );
    }

    pub async fn invalidate_all(&self) {
        let mut cache = self.inner.write().await;
        cache.active_rules = None;
        cache.active_models.clear();
    }
}

pub fn build_app(config: AppConfig) -> Router {
    let scorer = configured_model_scorer(&config).expect("failed to configure model scorer");
    build_app_with_parts(config, scorer, InMemoryScoringRepository::shared())
}

pub fn configured_model_scorer(
    config: &AppConfig,
) -> anyhow::Result<Arc<dyn ModelScorer>> {
    if let Some(manifest_uri) = config.model_serving_manifest_uri() {
        Ok(Arc::new(ServingManifestModelScorer::from_env(
            manifest_uri,
            config.model_signature_key(),
        )))
    } else if let Some(artifact_uri) = config.model_artifact_uri() {
        Ok(Arc::new(ArtifactModelScorer::from_env(
            artifact_uri,
            config.model_version_lock(),
            config.model_artifact_sha256(),
            config.model_artifact_signature(),
            config.model_signature_key(),
        )))
    } else if config.model_runtime_kind() == "heuristic" {
        Ok(Arc::new(HeuristicModelScorer))
    } else {
        Ok(Arc::new(
            HttpModelScorer::new(config.model_service_url.clone())
                .map_err(|e| anyhow::anyhow!("failed to build HTTP model scorer: {e}"))?,
        ))
    }
}

pub fn build_app_with_parts(
    config: AppConfig,
    scorer: Arc<dyn ModelScorer>,
    repository: SharedRepository,
) -> Router {
    let state = AppState {
        config,
        scorer,
        repository,
        scoring_lookup_cache: ScoringLookupCache::default(),
    };

    app_routes::register_api_routes(Router::new()).with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fwa_core::{ClaimId, RecommendedAction, RuleActionClass, ScoringRunId};
    use fwa_features::FeatureValue;
    use fwa_ml_runtime::ModelScoreRequest;
    use std::{
        collections::BTreeMap,
        fs,
        io::{Read, Write},
        net::TcpListener,
        path::PathBuf,
        sync::{Mutex, OnceLock},
        thread,
    };

    fn config(model_service_url: String) -> AppConfig {
        AppConfig {
            api_key: "dev-secret".into(),
            api_key_principals: vec![],
            source_system: "tpa-demo".into(),
            database_url: "postgres://unused".into(),
            model_service_url,
            object_storage_uri: "local://demo-artifacts".into(),
            customer_scope_id: "demo-customer".into(),
            retention_policy_id: "demo-retention-policy".into(),
            backup_restore_plan_id: "demo-backup-restore-plan".into(),
            pii_masking_policy_id: "demo-pii-masking-policy".into(),
            key_rotation_policy_id: "demo-key-rotation-policy".into(),
            network_allowlist_id: "demo-network-allowlist".into(),
            alert_routing_policy_id: "demo-alert-routing-policy".into(),
            observability_exporter_endpoint: "local://demo-observability".into(),
            agent_policy_id: "demo-agent-policy".into(),
        }
    }

    fn scorer_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[tokio::test]
    async fn scoring_lookup_cache_stores_rules_until_invalidated() {
        let cache = ScoringLookupCache::new(Duration::from_secs(60));
        assert!(cache.active_rules().await.is_none());

        cache
            .store_active_rules(vec![Rule {
                rule_id: "rule_cache_test".into(),
                version: 1,
                name: "Cache test".into(),
                review_mode: "both".into(),
                scheme_family: None,
                conditions: vec![],
                action: fwa_rules::RuleAction {
                    score: 10,
                    alert_code: "CACHE_TEST".into(),
                    recommended_action: RecommendedAction::ManualReview,
                    action_class: RuleActionClass::ManualReview,
                    required_evidence: vec![],
                    adjudication_policy: None,
                    reason: "cache test".into(),
                },
            }])
            .await;

        let cached_rules = cache.active_rules().await.expect("cached rules");
        assert_eq!(cached_rules[0].rule_id, "rule_cache_test");

        cache.invalidate_all().await;
        assert!(cache.active_rules().await.is_none());
    }

    #[tokio::test]
    async fn scoring_lookup_cache_stores_models_by_review_mode() {
        let cache = ScoringLookupCache::new(Duration::from_secs(60));
        let model = ModelVersionRecord {
            model_key: "baseline_fwa".into(),
            version: "0.2.0-active".into(),
            model_type: "baseline_classifier".into(),
            runtime_kind: "python_http".into(),
            execution_provider: "cpu".into(),
            status: "active".into(),
            review_mode: "pre_payment".into(),
            artifact_uri: None,
            endpoint_url: Some("http://127.0.0.1:8001/score".into()),
        };

        cache.store_active_model("pre_payment", model).await;

        assert_eq!(
            cache
                .active_model("pre_payment")
                .await
                .expect("cached model")
                .version,
            "0.2.0-active"
        );
        assert!(cache.active_model("post_payment").await.is_none());

        cache.invalidate_all().await;
        assert!(cache.active_model("pre_payment").await.is_none());
    }

    fn clear_model_artifact_env() {
        for name in [
            "FWA_MODEL_SERVING_MANIFEST_URI",
            "FWA_MODEL_ARTIFACT_URI",
            "FWA_MODEL_VERSION_LOCK",
            "FWA_MODEL_ARTIFACT_SHA256",
            "FWA_MODEL_ARTIFACT_CHECKSUM",
            "FWA_MODEL_ARTIFACT_SIGNATURE",
            "FWA_MODEL_SIGNATURE_KEY",
        ] {
            std::env::remove_var(name);
        }
    }

    #[tokio::test]
    async fn configured_model_scorer_uses_http_for_service_url() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 2048];
            let _ = stream.read(&mut buffer);
            let body = r#"{"model_key":"baseline_fwa","model_version":"0.2.0-active","score":73,"label":"HIGH_RISK","explanations":[],"metadata":{"fraud_probability":0.73}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let scorer = {
            let _guard = scorer_env_lock().lock().unwrap();
            clear_model_artifact_env();
            configured_model_scorer(&config(format!("http://{address}"))).unwrap()
        };

        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_configured_http"),
                claim_id: ClaimId::from_external("CLM-CONFIGURED-HTTP"),
                model_key: "baseline_fwa".into(),
                model_version: "0.2.0-active".into(),
                endpoint_url: None,
                features: BTreeMap::new(),
            })
            .await
            .unwrap();

        server.join().unwrap();
        assert_eq!(result.runtime_kind, "python_http");
        assert_eq!(result.model_version, "0.2.0-active");
        assert_eq!(result.score, 73);
    }

    #[tokio::test]
    async fn configured_model_scorer_allows_explicit_heuristic_fallback() {
        let scorer = {
            let _guard = scorer_env_lock().lock().unwrap();
            clear_model_artifact_env();
            configured_model_scorer(&config("heuristic://local".into())).unwrap()
        };

        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_configured_heuristic"),
                claim_id: ClaimId::from_external("CLM-CONFIGURED-HEURISTIC"),
                model_key: "baseline_fwa".into(),
                model_version: "0.1.0".into(),
                endpoint_url: None,
                features: BTreeMap::new(),
            })
            .await
            .unwrap();

        assert_eq!(result.runtime_kind, "heuristic");
        assert_eq!(result.score, 0);
    }

    #[tokio::test]
    async fn configured_model_scorer_prefers_rust_artifact_uri() {
        let artifact_path = write_artifact(serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.2.0-rust",
            "runtime_kind": "rust_logistic_regression",
            "execution_provider": "cpu",
            "threshold": 0.5,
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "intercept": -1.0,
            "coefficients": {"claim_amount_to_limit_ratio": 4.0}
        }));
        let scorer = {
            let _guard = scorer_env_lock().lock().unwrap();
            clear_model_artifact_env();
            std::env::set_var("FWA_MODEL_ARTIFACT_URI", &artifact_path);
            let scorer = configured_model_scorer(&config("http://127.0.0.1:1".into())).unwrap();
            clear_model_artifact_env();
            scorer
        };

        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_configured_artifact"),
                claim_id: ClaimId::from_external("CLM-CONFIGURED-ARTIFACT"),
                model_key: "baseline_fwa".into(),
                model_version: "0.2.0-rust".into(),
                endpoint_url: None,
                features: features([("claim_amount_to_limit_ratio", 0.8)]),
            })
            .await
            .unwrap();

        assert_eq!(result.runtime_kind, "rust_logistic_regression");
        assert_eq!(result.model_version, "0.2.0-rust");
        assert_eq!(result.score, 90);
        fs::remove_file(artifact_path).unwrap();
    }

    #[tokio::test]
    async fn configured_model_scorer_prefers_serving_manifest_uri() {
        let artifact_path = write_artifact(serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.3.0-active",
            "runtime_kind": "rust_logistic_regression",
            "execution_provider": "cpu",
            "threshold": 0.5,
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "intercept": -1.0,
            "coefficients": {"claim_amount_to_limit_ratio": 4.0}
        }));
        let manifest_path = write_artifact(serde_json::json!({
            "model_key": "baseline_fwa",
            "model_version": "0.3.0-active",
            "runtime_kind": "rust_logistic_regression",
            "artifact_uri": artifact_path.to_string_lossy(),
            "artifact_sha256": artifact_sha256(&artifact_path),
            "version_lock": "0.3.0-active",
            "feature_columns": ["claim_amount_to_limit_ratio"],
            "threshold": 0.5,
            "training_artifact_uri": "/tmp/model.joblib"
        }));
        let scorer = {
            let _guard = scorer_env_lock().lock().unwrap();
            clear_model_artifact_env();
            std::env::set_var("FWA_MODEL_SERVING_MANIFEST_URI", &manifest_path);
            let scorer = configured_model_scorer(&config("http://127.0.0.1:1".into())).unwrap();
            clear_model_artifact_env();
            scorer
        };

        let result = scorer
            .score(ModelScoreRequest {
                run_id: ScoringRunId::from_external("run_configured_serving_manifest"),
                claim_id: ClaimId::from_external("CLM-CONFIGURED-SERVING-MANIFEST"),
                model_key: "baseline_fwa".into(),
                model_version: "0.3.0-active".into(),
                endpoint_url: None,
                features: features([("claim_amount_to_limit_ratio", 0.8)]),
            })
            .await
            .unwrap();

        assert_eq!(result.runtime_kind, "rust_logistic_regression");
        assert_eq!(
            result.metadata["serving_manifest_status"],
            serde_json::json!("passed")
        );
        assert_eq!(
            result.metadata["training_artifact_uri"],
            serde_json::json!("/tmp/model.joblib")
        );
        fs::remove_file(artifact_path).unwrap();
        fs::remove_file(manifest_path).unwrap();
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

    fn write_artifact(payload: serde_json::Value) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("nwfwa-api-artifact-{}.json", ScoringRunId::new()));
        fs::write(&path, serde_json::to_vec(&payload).unwrap()).unwrap();
        path
    }

    fn artifact_sha256(path: &PathBuf) -> String {
        use sha2::{Digest, Sha256};

        let digest = Sha256::digest(fs::read(path).unwrap());
        format!("sha256:{digest:x}")
    }
}

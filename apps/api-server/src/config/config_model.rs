use super::AppConfig;

fn configured_env_value(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

impl AppConfig {
    pub fn model_runtime_kind(&self) -> &'static str {
        if self.model_serving_manifest_uri().is_some() {
            "rust_serving_manifest"
        } else if self.model_artifact_uri().is_some() {
            "rust_artifact"
        } else if self.model_service_url == "heuristic"
            || self.model_service_url.starts_with("heuristic://")
        {
            "heuristic"
        } else {
            "python_http"
        }
    }

    pub fn model_service_configuration_status(&self) -> &'static str {
        if self.model_serving_manifest_uri().is_some() || self.model_artifact_uri().is_some() {
            "configured"
        } else if self.model_service_url == "heuristic"
            || self.model_service_url.starts_with("heuristic://")
        {
            "heuristic_model_scorer"
        } else if self.model_service_url == "http://127.0.0.1:8001" {
            "local_dev_model_service"
        } else {
            "configured"
        }
    }

    pub fn model_artifact_uri(&self) -> Option<String> {
        configured_env_value("FWA_MODEL_ARTIFACT_URI")
    }

    pub fn model_serving_manifest_uri(&self) -> Option<String> {
        configured_env_value("FWA_MODEL_SERVING_MANIFEST_URI")
    }

    pub fn model_version_lock(&self) -> Option<String> {
        configured_env_value("FWA_MODEL_VERSION_LOCK")
    }

    pub fn model_artifact_sha256(&self) -> Option<String> {
        configured_env_value("FWA_MODEL_ARTIFACT_SHA256")
            .or_else(|| configured_env_value("FWA_MODEL_ARTIFACT_CHECKSUM"))
    }

    pub fn model_artifact_signature(&self) -> Option<String> {
        configured_env_value("FWA_MODEL_ARTIFACT_SIGNATURE")
    }

    pub fn model_signature_key(&self) -> Option<String> {
        configured_env_value("FWA_MODEL_SIGNATURE_KEY")
    }
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub api_key: String,
    pub source_system: String,
    pub database_url: String,
    pub model_service_url: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("FWA_API_KEY").unwrap_or_else(|_| "dev-secret".into()),
            source_system: std::env::var("FWA_SOURCE_SYSTEM").unwrap_or_else(|_| "tpa-demo".into()),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/fwa".into()),
            model_service_url: std::env::var("FWA_MODEL_SERVICE_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8001".into()),
        }
    }

    pub fn model_runtime_kind(&self) -> &'static str {
        if self.model_service_url == "heuristic"
            || self.model_service_url.starts_with("heuristic://")
        {
            "heuristic"
        } else {
            "python_http"
        }
    }

    pub fn api_key_configuration_status(&self) -> &'static str {
        if self.api_key == "dev-secret" {
            "local_dev_key"
        } else {
            "configured"
        }
    }

    pub fn source_system_configuration_status(&self) -> &'static str {
        if self.source_system == "tpa-demo" {
            "local_demo_source"
        } else {
            "configured"
        }
    }

    pub fn database_configuration_status(&self) -> &'static str {
        if self.database_url == "postgres://postgres:postgres@localhost:5432/fwa" {
            "local_dev_database"
        } else {
            "configured"
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

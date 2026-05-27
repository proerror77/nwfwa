#[derive(Debug, Clone)]
pub struct AppConfig {
    pub api_key: String,
    pub source_system: String,
    pub database_url: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("FWA_API_KEY").unwrap_or_else(|_| "dev-secret".into()),
            source_system: std::env::var("FWA_SOURCE_SYSTEM").unwrap_or_else(|_| "tpa-demo".into()),
            database_url: std::env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/fwa".into()),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

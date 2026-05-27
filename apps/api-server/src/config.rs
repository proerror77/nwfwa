#[derive(Debug, Clone)]
pub struct AppConfig {
    pub api_key: String,
    pub source_system: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_key: "dev-secret".into(),
            source_system: "tpa-demo".into(),
        }
    }
}

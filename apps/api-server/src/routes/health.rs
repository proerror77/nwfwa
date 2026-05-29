use axum::Json;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub service: &'static str,
    pub version: &'static str,
    pub checks: Vec<HealthCheck>,
}

#[derive(Debug, Serialize)]
pub struct HealthCheck {
    pub name: &'static str,
    pub status: &'static str,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "api-server",
        version: env!("CARGO_PKG_VERSION"),
        checks: vec![
            HealthCheck {
                name: "http_router",
                status: "ok",
            },
            HealthCheck {
                name: "openapi_contract",
                status: "ok",
            },
        ],
    })
}

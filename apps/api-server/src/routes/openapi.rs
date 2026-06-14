use axum::Json;
use serde_json::{json, Value};

#[path = "openapi_components.rs"]
mod openapi_components;
#[path = "openapi_paths.rs"]
mod openapi_paths;

pub async fn openapi_schema() -> Json<Value> {
    Json(json!({
        "openapi": "3.1.0",
        "info": {
            "title": "FWA Core Runtime API",
            "version": "0.1.0",
            "description": "MVP API contract for claim scoring and runtime health checks."
        },
        "paths": openapi_paths::openapi_paths(),
        "components": openapi_components::openapi_components()
    }))
}

use crate::{
    app::AppState,
    error::ApiError,
    repository::{KnowledgeCaseRecord, SimilarCaseQuery, SimilarCaseRecord},
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_auth::{validate_api_key, ApiKeyConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct KnowledgeCaseListResponse {
    pub cases: Vec<KnowledgeCaseRecord>,
}

#[derive(Debug, Deserialize)]
pub struct SimilarCaseSearchRequest {
    pub claim_id: Option<String>,
    pub diagnosis_code: String,
    pub provider_region: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct SimilarCaseSearchResponse {
    pub results: Vec<SimilarCaseRecord>,
}

pub async fn list_cases(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<KnowledgeCaseListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let cases = state
        .repository
        .list_knowledge_cases()
        .await
        .map_err(internal_error("KNOWLEDGE_CASE_LIST_FAILED"))?;
    Ok(Json(KnowledgeCaseListResponse { cases }))
}

pub async fn search_similar(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SimilarCaseSearchRequest>,
) -> Result<Json<SimilarCaseSearchResponse>, ApiError> {
    authorize(&state, &headers)?;
    let results = state
        .repository
        .search_similar_cases(SimilarCaseQuery {
            claim_id: request.claim_id,
            diagnosis_code: request.diagnosis_code,
            provider_region: request.provider_region,
            tags: request.tags,
        })
        .await
        .map_err(internal_error("KNOWLEDGE_SEARCH_FAILED"))?;
    Ok(Json(SimilarCaseSearchResponse { results }))
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    validate_api_key(
        api_key,
        &ApiKeyConfig {
            key: state.config.api_key.clone(),
            source_system: state.config.source_system.clone(),
        },
    )
    .map(|_| ())
    .map_err(|_| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "INVALID_API_KEY",
            "invalid api key",
        )
    })
}

fn internal_error(
    code: &'static str,
) -> impl Fn(anyhow::Error) -> ApiError + Clone + Send + Sync + 'static {
    move |error| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, code, error.to_string())
}

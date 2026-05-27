use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        CreateFieldMappingInput, DatasetRecord, FieldMappingRecord, RegisterDatasetInput,
    },
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_auth::{validate_api_key, ApiKeyConfig};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct DatasetListResponse {
    pub datasets: Vec<DatasetRecord>,
}

#[derive(Debug, Serialize)]
pub struct FieldMappingResponse {
    pub mapping: FieldMappingRecord,
}

pub async fn register_dataset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegisterDatasetInput>,
) -> Result<Json<DatasetRecord>, ApiError> {
    authorize(&state, &headers)?;
    validate_parquet_dataset(&request.storage_format)?;
    let dataset = state
        .repository
        .register_dataset(request)
        .await
        .map_err(internal_error("DATASET_REGISTER_FAILED"))?;
    Ok(Json(dataset))
}

pub async fn list_datasets(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<DatasetListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let datasets = state
        .repository
        .list_datasets()
        .await
        .map_err(internal_error("DATASET_LIST_FAILED"))?;
    Ok(Json(DatasetListResponse { datasets }))
}

pub async fn get_dataset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(dataset_id): Path<String>,
) -> Result<Json<DatasetRecord>, ApiError> {
    authorize(&state, &headers)?;
    let dataset = state
        .repository
        .get_dataset(&dataset_id)
        .await
        .map_err(internal_error("DATASET_LOAD_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "DATASET_NOT_FOUND",
                "dataset not found",
            )
        })?;
    Ok(Json(dataset))
}

pub async fn add_field_mapping(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(dataset_id): Path<String>,
    Json(request): Json<CreateFieldMappingInput>,
) -> Result<Json<FieldMappingResponse>, ApiError> {
    authorize(&state, &headers)?;
    let mapping = state
        .repository
        .add_field_mapping(&dataset_id, request)
        .await
        .map_err(internal_error("FIELD_MAPPING_CREATE_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "DATASET_NOT_FOUND",
                "dataset not found",
            )
        })?;
    Ok(Json(FieldMappingResponse { mapping }))
}

fn validate_parquet_dataset(storage_format: &str) -> Result<(), ApiError> {
    if storage_format == "parquet" {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "DATASET_FORMAT_NOT_SUPPORTED",
            "registered analytical datasets must use parquet storage_format",
        ))
    }
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

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, code, error.to_string())
}

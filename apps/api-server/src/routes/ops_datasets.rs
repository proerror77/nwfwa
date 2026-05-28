use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        CreateFieldMappingInput, DatasetRecord, FeatureSetRecord, FieldMappingRecord,
        ModelDatasetRecord, ModelEvaluationRecord, RegisterDatasetInput, RegisterFeatureSetInput,
        RegisterModelDatasetInput, RegisterModelEvaluationInput,
    },
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_auth::{validate_api_key, ApiKeyConfig};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct DatasetListResponse {
    pub datasets: Vec<DatasetRecord>,
}

#[derive(Debug, Serialize)]
pub struct FieldMappingResponse {
    pub mapping: FieldMappingRecord,
}

#[derive(Debug, Serialize)]
pub struct ModelEvaluationResponse {
    pub evaluation: ModelEvaluationRecord,
}

#[derive(Debug, Serialize)]
pub struct ModelEvaluationListResponse {
    pub evaluations: Vec<ModelEvaluationRecord>,
}

#[derive(Debug, Serialize)]
pub struct FactorReadinessResponse {
    pub dataset_count: u32,
    pub factor_count: u32,
    pub label_count: u32,
    pub entity_key_count: u32,
    pub data_quality_score: f64,
    pub data_quality_status: String,
    pub online_ready_count: u32,
    pub rule_convertible_count: u32,
    pub mapped_factor_count: u32,
    pub high_missing_count: u32,
    pub unstable_factor_count: u32,
    pub unowned_factor_count: u32,
}

pub async fn register_dataset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegisterDatasetInput>,
) -> Result<Json<DatasetRecord>, ApiError> {
    authorize(&state, &headers)?;
    validate_dataset_contract(&request)?;
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

pub async fn factor_readiness(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<FactorReadinessResponse>, ApiError> {
    authorize(&state, &headers)?;
    let datasets = state
        .repository
        .list_datasets()
        .await
        .map_err(internal_error("DATASET_LIST_FAILED"))?;
    Ok(Json(build_factor_readiness(&datasets)))
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

pub async fn register_feature_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegisterFeatureSetInput>,
) -> Result<Json<FeatureSetRecord>, ApiError> {
    authorize(&state, &headers)?;
    validate_parquet_uri(&request.features_uri, "FEATURE_SET_FORMAT_INVALID")?;
    let feature_set = state
        .repository
        .register_feature_set(request)
        .await
        .map_err(internal_error("FEATURE_SET_REGISTER_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "DATASET_NOT_FOUND",
                "feature set dataset was not found",
            )
        })?;
    Ok(Json(feature_set))
}

pub async fn register_model_dataset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegisterModelDatasetInput>,
) -> Result<Json<ModelDatasetRecord>, ApiError> {
    authorize(&state, &headers)?;
    validate_parquet_uri(&request.train_uri, "MODEL_DATASET_FORMAT_INVALID")?;
    validate_parquet_uri(&request.validation_uri, "MODEL_DATASET_FORMAT_INVALID")?;
    if let Some(test_uri) = &request.test_uri {
        validate_parquet_uri(test_uri, "MODEL_DATASET_FORMAT_INVALID")?;
    }
    let model_dataset = state
        .repository
        .register_model_dataset(request)
        .await
        .map_err(internal_error("MODEL_DATASET_REGISTER_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "FEATURE_SET_NOT_FOUND",
                "model dataset feature set was not found",
            )
        })?;
    Ok(Json(model_dataset))
}

pub async fn register_model_evaluation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegisterModelEvaluationInput>,
) -> Result<Json<ModelEvaluationResponse>, ApiError> {
    authorize(&state, &headers)?;
    let evaluation = state
        .repository
        .register_model_evaluation(request)
        .await
        .map_err(internal_error("MODEL_EVALUATION_REGISTER_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MODEL_DATASET_NOT_FOUND",
                "model evaluation dataset was not found",
            )
        })?;
    Ok(Json(ModelEvaluationResponse { evaluation }))
}

pub async fn get_model_evaluation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(evaluation_run_id): Path<String>,
) -> Result<Json<ModelEvaluationResponse>, ApiError> {
    authorize(&state, &headers)?;
    let evaluation = state
        .repository
        .get_model_evaluation(&evaluation_run_id)
        .await
        .map_err(internal_error("MODEL_EVALUATION_LOAD_FAILED"))?
        .ok_or_else(|| {
            ApiError::new(
                StatusCode::NOT_FOUND,
                "MODEL_EVALUATION_NOT_FOUND",
                "model evaluation was not found",
            )
        })?;
    Ok(Json(ModelEvaluationResponse { evaluation }))
}

pub async fn list_model_evaluations(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ModelEvaluationListResponse>, ApiError> {
    authorize(&state, &headers)?;
    let evaluations = state
        .repository
        .list_model_evaluations()
        .await
        .map_err(internal_error("MODEL_EVALUATION_LIST_FAILED"))?;
    Ok(Json(ModelEvaluationListResponse { evaluations }))
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

fn build_factor_readiness(datasets: &[DatasetRecord]) -> FactorReadinessResponse {
    let mut response = FactorReadinessResponse {
        dataset_count: datasets.len() as u32,
        factor_count: 0,
        label_count: 0,
        entity_key_count: 0,
        data_quality_score: 0.0,
        data_quality_status: "empty".into(),
        online_ready_count: 0,
        rule_convertible_count: 0,
        mapped_factor_count: 0,
        high_missing_count: 0,
        unstable_factor_count: 0,
        unowned_factor_count: 0,
    };

    for dataset in datasets {
        for field in &dataset.fields {
            response.factor_count += 1;
            let is_label = field.semantic_role == "label";
            let is_entity_key = dataset.entity_keys.contains(&field.field_name);
            let missing_rate = numeric_profile_value(&field.profile_json, "missing_rate");
            if is_label {
                response.label_count += 1;
            }
            if is_entity_key {
                response.entity_key_count += 1;
            }
            if !is_label && !field.nullable && missing_rate.unwrap_or(0.0) <= 0.05 {
                response.online_ready_count += 1;
            }
            if !is_label && is_rule_convertible_type(&field.logical_type) {
                response.rule_convertible_count += 1;
            }
            if missing_rate.unwrap_or(0.0) > 0.20 {
                response.high_missing_count += 1;
            }
            if numeric_profile_value(&field.profile_json, "psi").unwrap_or(0.0) >= 0.25 {
                response.unstable_factor_count += 1;
            }
            if field
                .profile_json
                .get("owner")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
            {
                response.unowned_factor_count += 1;
            }
            if dataset
                .mappings
                .iter()
                .any(|mapping| mapping.feature_name.as_deref() == Some(field.field_name.as_str()))
            {
                response.mapped_factor_count += 1;
            }
        }
    }

    response.data_quality_score = factor_data_quality_score(&response);
    response.data_quality_status = factor_data_quality_status(response.data_quality_score).into();

    response
}

fn factor_data_quality_score(response: &FactorReadinessResponse) -> f64 {
    if response.factor_count == 0 {
        return 0.0;
    }
    let max_issue_count = response.factor_count * 3;
    let issue_count = response.high_missing_count
        + response.unstable_factor_count
        + response.unowned_factor_count;
    let score = 1.0 - (issue_count as f64 / max_issue_count as f64);
    score.clamp(0.0, 1.0)
}

fn factor_data_quality_status(score: f64) -> &'static str {
    if score >= 0.85 {
        "ready"
    } else if score >= 0.65 {
        "watch"
    } else {
        "blocked"
    }
}

fn numeric_profile_value(profile: &Value, key: &str) -> Option<f64> {
    profile.get(key).and_then(|value| {
        value
            .as_f64()
            .or_else(|| value.as_i64().map(|value| value as f64))
    })
}

fn is_rule_convertible_type(logical_type: &str) -> bool {
    matches!(
        logical_type,
        "decimal" | "float" | "float64" | "int" | "int8" | "int32" | "int64" | "boolean"
    )
}

fn validate_dataset_contract(request: &RegisterDatasetInput) -> Result<(), ApiError> {
    validate_parquet_dataset(&request.storage_format)?;
    require_suffix(
        &request.manifest_uri,
        "manifest.json",
        "DATASET_MANIFEST_INVALID",
    )?;
    require_suffix(&request.schema_uri, "schema.json", "DATASET_SCHEMA_INVALID")?;
    require_suffix(
        &request.profile_uri,
        "profile.json",
        "DATASET_PROFILE_INVALID",
    )?;

    if request
        .splits
        .iter()
        .any(|split| split.data_uri.to_ascii_lowercase().contains(".csv"))
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "DATASET_SPLIT_FORMAT_INVALID",
            "dataset split URIs must point to parquet files or parquet partition directories",
        ));
    }

    let split_rows = request
        .splits
        .iter()
        .map(|split| split.row_count)
        .sum::<u64>();
    if split_rows != request.row_count {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "DATASET_ROW_COUNT_MISMATCH",
            "dataset row_count must equal the sum of split row counts",
        ));
    }

    let Some(label_field) = request
        .fields
        .iter()
        .find(|field| field.field_name == request.label_column)
    else {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "DATASET_LABEL_FIELD_MISSING",
            "label_column must exist in schema fields",
        ));
    };
    if label_field.semantic_role != "label" {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "DATASET_LABEL_ROLE_INVALID",
            "label_column schema field must have semantic_role label",
        ));
    }

    for key in &request.entity_keys {
        let Some(field) = request.fields.iter().find(|field| field.field_name == *key) else {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "DATASET_ENTITY_KEY_MISSING",
                "entity_keys must exist in schema fields",
            ));
        };
        if field.logical_type != "string" {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "DATASET_ENTITY_KEY_TYPE_INVALID",
                "entity key fields must use string logical_type",
            ));
        }
    }

    Ok(())
}

fn require_suffix(value: &str, suffix: &str, code: &'static str) -> Result<(), ApiError> {
    if value.ends_with(suffix) {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            code,
            format!("dataset URI must end with {suffix}"),
        ))
    }
}

fn validate_parquet_uri(value: &str, code: &'static str) -> Result<(), ApiError> {
    if value.to_ascii_lowercase().contains(".csv") {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            code,
            "dataset artifact URIs must point to parquet files or parquet partition directories",
        ))
    } else {
        Ok(())
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

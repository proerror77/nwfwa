use crate::{
    app::AppState,
    error::ApiError,
    repository::{
        CreateFieldMappingInput, DatasetRecord, FeatureSetRecord, FieldMappingRecord,
        ModelDatasetRecord, ModelEvaluationRecord, PersistedAuditEvent, RegisterDatasetInput,
        RegisterFeatureSetInput, RegisterModelDatasetInput, RegisterModelEvaluationInput,
        SchemaFieldRecord,
    },
    routes::pii,
};
use std::collections::BTreeMap;

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use fwa_audit::ActorContext;
use fwa_auth::validate_api_key;
use fwa_core::{canonical_scheme_family, AuditEventId, ScoringRunId};
use rust_decimal::Decimal;
use serde::Serialize;
use serde_json::{json, Map, Value};

#[derive(Debug, Serialize)]
pub struct DatasetListResponse {
    pub datasets: Vec<DatasetRecord>,
    pub health: Vec<DatasetHealthRecord>,
}

#[derive(Debug, Serialize)]
pub struct DatasetHealthRecord {
    pub dataset_id: String,
    pub dataset_key: String,
    pub dataset_version: String,
    pub data_quality_score: f64,
    pub data_quality_status: String,
    pub field_count: u32,
    pub label_count: u32,
    pub entity_key_count: u32,
    pub high_missing_count: u32,
    pub unstable_field_count: u32,
    pub unowned_field_count: u32,
    pub online_ready_count: u32,
    pub issue_count: u32,
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
    pub lineage: Vec<ModelEvaluationLineageRecord>,
}

#[derive(Debug, Serialize)]
pub struct ModelEvaluationLineageRecord {
    pub evaluation_run_id: String,
    pub model_key: String,
    pub model_version: String,
    pub model_dataset_id: String,
    pub source_dataset_id: Option<String>,
    pub source_dataset_key: Option<String>,
    pub source_dataset_version: Option<String>,
    pub source_data_quality_score: Option<f64>,
    pub source_data_quality_status: Option<String>,
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
    pub ready_factor_count: u32,
    pub review_factor_count: u32,
    pub readiness_issue_counts: Map<String, Value>,
    pub scheme_readiness: Vec<FactorSchemeReadinessRecord>,
    pub factor_cards: Vec<FactorCardRecord>,
}

#[derive(Debug, Serialize)]
pub struct FactorSchemeReadinessRecord {
    pub scheme_family: String,
    pub factor_count: u32,
    pub ready_factor_count: u32,
    pub review_factor_count: u32,
    pub online_ready_count: u32,
    pub rule_convertible_count: u32,
    pub readiness_issue_counts: Map<String, Value>,
}

#[derive(Debug, Serialize)]
pub struct FactorCardRecord {
    pub dataset_id: String,
    pub dataset_key: String,
    pub dataset_version: String,
    pub factor_name: String,
    pub scheme_family: String,
    pub chinese_name: String,
    pub entity_type: String,
    pub semantic_role: String,
    pub logical_type: String,
    pub calculation_window: String,
    pub calculation_logic: String,
    pub source_table: String,
    pub source_fields: Vec<String>,
    pub business_meaning: String,
    pub risk_direction: String,
    pub missing_rate: Option<f64>,
    pub iv: Option<f64>,
    pub auc_gain: Option<f64>,
    pub lift: Option<f64>,
    pub psi: Option<f64>,
    pub stability: String,
    pub model_contribution: Option<f64>,
    pub rule_convertible: bool,
    pub online_available: bool,
    pub readiness_status: String,
    pub readiness_issues: Vec<String>,
    pub version: String,
    pub owner: String,
    pub is_label: bool,
    pub is_entity_key: bool,
    pub evidence_refs: Vec<String>,
}

pub async fn register_dataset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegisterDatasetInput>,
) -> Result<Json<DatasetRecord>, ApiError> {
    let actor = authorize(&state, &headers)?;
    validate_dataset_contract(&request)?;
    let dataset = state
        .repository
        .register_dataset(request)
        .await
        .map_err(internal_error("DATASET_REGISTER_FAILED"))?;
    record_data_lineage_audit(
        &state,
        &actor,
        DataLineageAuditInput {
            event_type: "dataset.registered",
            summary: "Dataset registered",
            payload: json!({
                "dataset_id": dataset.dataset_id,
                "dataset_key": dataset.dataset_key,
                "dataset_version": dataset.dataset_version,
                "business_domain": dataset.business_domain,
                "source_key": dataset.source_key,
                "storage_format": dataset.storage_format,
                "row_count": dataset.row_count,
                "to_status": dataset.status,
                "owner": actor.actor_id.clone(),
            }),
            evidence_refs: vec![format!(
                "datasets:{}:{}",
                dataset.dataset_key, dataset.dataset_version
            )],
        },
    )
    .await
    .map_err(internal_error("DATASET_AUDIT_SAVE_FAILED"))?;
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
    let health = build_dataset_health(&datasets);
    Ok(Json(DatasetListResponse { datasets, health }))
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
    let actor = authorize(&state, &headers)?;
    validate_field_mapping(&request)?;
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
    record_data_lineage_audit(
        &state,
        &actor,
        DataLineageAuditInput {
            event_type: "dataset.field_mapping.added",
            summary: "Dataset field mapping added",
            payload: json!({
                "dataset_id": mapping.dataset_id,
                "external_field": mapping.external_field,
                "canonical_target": mapping.canonical_target,
                "feature_name": mapping.feature_name,
                "transform_kind": mapping.transform_kind,
                "to_status": mapping.status,
                "owner": actor.actor_id.clone(),
            }),
            evidence_refs: vec![format!(
                "dataset_field_mappings:{}:{}",
                mapping.dataset_id, mapping.external_field
            )],
        },
    )
    .await
    .map_err(internal_error("FIELD_MAPPING_AUDIT_SAVE_FAILED"))?;
    Ok(Json(FieldMappingResponse { mapping }))
}

fn validate_field_mapping(request: &CreateFieldMappingInput) -> Result<(), ApiError> {
    if request.external_field.trim().is_empty()
        || request.canonical_target.trim().is_empty()
        || request.transform_kind.trim().is_empty()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FIELD_MAPPING",
            "external_field, canonical_target, and transform_kind are required",
        ));
    }
    if let Some(feature_name) = &request.feature_name {
        if feature_name.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_FIELD_MAPPING",
                "feature_name must not be blank when provided",
            ));
        }
    }
    if !matches!(
        request.transform_kind.as_str(),
        "direct" | "cast" | "enum_map" | "derived" | "aggregate"
    ) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FIELD_MAPPING",
            "transform_kind must be direct, cast, enum_map, derived, or aggregate",
        ));
    }
    if !matches!(request.status.as_str(), "draft" | "active" | "deprecated") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FIELD_MAPPING",
            "status must be draft, active, or deprecated",
        ));
    }
    Ok(())
}

pub async fn register_feature_set(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegisterFeatureSetInput>,
) -> Result<Json<FeatureSetRecord>, ApiError> {
    let actor = authorize(&state, &headers)?;
    validate_feature_set_registration(&request)?;
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
    record_data_lineage_audit(
        &state,
        &actor,
        DataLineageAuditInput {
            event_type: "feature_set.registered",
            summary: "Feature set registered",
            payload: json!({
                "feature_set_id": feature_set.feature_set_id,
                "feature_set_key": feature_set.feature_set_key,
                "version": feature_set.version,
                "dataset_id": feature_set.dataset_id,
                "business_domain": feature_set.business_domain,
                "row_count": feature_set.row_count,
                "label_column": feature_set.label_column,
                "to_status": feature_set.status,
                "owner": actor.actor_id.clone(),
            }),
            evidence_refs: vec![format!(
                "feature_sets:{}:{}",
                feature_set.feature_set_key, feature_set.version
            )],
        },
    )
    .await
    .map_err(internal_error("FEATURE_SET_AUDIT_SAVE_FAILED"))?;
    Ok(Json(feature_set))
}

fn validate_feature_set_registration(request: &RegisterFeatureSetInput) -> Result<(), ApiError> {
    if request.business_domain.trim().is_empty()
        || request.feature_set_key.trim().is_empty()
        || request.version.trim().is_empty()
        || request.dataset_id.trim().is_empty()
        || request.features_uri.trim().is_empty()
        || request.label_column.trim().is_empty()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FEATURE_SET",
            "business_domain, feature_set_key, version, dataset_id, features_uri, and label_column are required",
        ));
    }
    let feature_list = request.feature_list_json.as_array();
    if feature_list.is_none() || feature_list.is_some_and(|features| features.is_empty()) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FEATURE_SET",
            "feature_list_json must be a non-empty array",
        ));
    }
    if request.row_count == 0 {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FEATURE_SET",
            "row_count must be greater than zero",
        ));
    }
    if !matches!(request.status.as_str(), "draft" | "active" | "deprecated") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_FEATURE_SET",
            "status must be draft, active, or deprecated",
        ));
    }
    Ok(())
}

pub async fn register_model_dataset(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RegisterModelDatasetInput>,
) -> Result<Json<ModelDatasetRecord>, ApiError> {
    let actor = authorize(&state, &headers)?;
    validate_model_dataset_registration(&request)?;
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
    record_data_lineage_audit(
        &state,
        &actor,
        DataLineageAuditInput {
            event_type: "model_dataset.registered",
            summary: "Model dataset registered",
            payload: json!({
                "model_dataset_id": model_dataset.model_dataset_id,
                "feature_set_id": model_dataset.feature_set_id,
                "business_domain": model_dataset.business_domain,
                "task_type": model_dataset.task_type,
                "label_name": model_dataset.label_name,
                "to_status": model_dataset.status,
                "owner": actor.actor_id.clone(),
            }),
            evidence_refs: vec![format!("model_datasets:{}", model_dataset.model_dataset_id)],
        },
    )
    .await
    .map_err(internal_error("MODEL_DATASET_AUDIT_SAVE_FAILED"))?;
    Ok(Json(model_dataset))
}

fn validate_model_dataset_registration(
    request: &RegisterModelDatasetInput,
) -> Result<(), ApiError> {
    if request.business_domain.trim().is_empty()
        || request.task_type.trim().is_empty()
        || request.label_name.trim().is_empty()
        || request.feature_set_id.trim().is_empty()
        || request.train_uri.trim().is_empty()
        || request.validation_uri.trim().is_empty()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MODEL_DATASET",
            "business_domain, task_type, label_name, feature_set_id, train_uri, and validation_uri are required",
        ));
    }
    if let Some(test_uri) = &request.test_uri {
        if test_uri.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_MODEL_DATASET",
                "test_uri must not be blank when provided",
            ));
        }
    }
    let row_counts = request.row_counts_json.as_object();
    if row_counts.is_none() || row_counts.is_some_and(|row_counts| row_counts.is_empty()) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MODEL_DATASET",
            "row_counts_json must be a non-empty object",
        ));
    }
    let label_distribution = request.label_distribution_json.as_object();
    if label_distribution.is_none()
        || label_distribution.is_some_and(|label_distribution| label_distribution.is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MODEL_DATASET",
            "label_distribution_json must be a non-empty object",
        ));
    }
    if !matches!(request.status.as_str(), "draft" | "active" | "deprecated") {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MODEL_DATASET",
            "status must be draft, active, or deprecated",
        ));
    }
    Ok(())
}

pub async fn register_model_evaluation(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<RegisterModelEvaluationInput>,
) -> Result<Json<ModelEvaluationResponse>, ApiError> {
    let actor = authorize(&state, &headers)?;
    validate_model_evaluation_registration(&request)?;
    request.scheme_family = canonical_scheme_family(&request.scheme_family).unwrap();
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
    record_data_lineage_audit(
        &state,
        &actor,
        DataLineageAuditInput {
            event_type: "model_evaluation.registered",
            summary: "Model evaluation registered",
            payload: json!({
                "evaluation_run_id": evaluation.evaluation_run_id,
                "model_key": evaluation.model_key,
                "model_version": evaluation.model_version,
                "model_dataset_id": evaluation.model_dataset_id,
                "scheme_family": evaluation.scheme_family,
                "to_status": "registered",
                "owner": actor.actor_id.clone(),
            }),
            evidence_refs: vec![format!(
                "model_evaluations:{}",
                evaluation.evaluation_run_id
            )],
        },
    )
    .await
    .map_err(internal_error("MODEL_EVALUATION_AUDIT_SAVE_FAILED"))?;
    Ok(Json(ModelEvaluationResponse { evaluation }))
}

fn validate_model_evaluation_registration(
    request: &RegisterModelEvaluationInput,
) -> Result<(), ApiError> {
    if request.evaluation_run_id.trim().is_empty()
        || request.model_key.trim().is_empty()
        || request.model_version.trim().is_empty()
        || request.model_dataset_id.trim().is_empty()
        || request.scheme_family.trim().is_empty()
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MODEL_EVALUATION",
            "evaluation_run_id, model_key, model_version, model_dataset_id, and scheme_family are required",
        ));
    }
    if canonical_scheme_family(&request.scheme_family).is_none() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MODEL_EVALUATION",
            "scheme_family must map to a known FWA scheme family",
        ));
    }
    for (metric_name, metric) in [
        ("auc", &request.auc),
        ("ks", &request.ks),
        ("precision", &request.precision),
        ("recall", &request.recall),
        ("f1", &request.f1),
        ("accuracy", &request.accuracy),
        ("threshold", &request.threshold),
    ] {
        validate_unit_interval_metric(metric_name, metric)?;
    }
    let confusion_matrix = request.confusion_matrix_json.as_object();
    if confusion_matrix.is_none()
        || confusion_matrix.is_some_and(|confusion_matrix| confusion_matrix.is_empty())
    {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MODEL_EVALUATION",
            "confusion_matrix_json must be a non-empty object",
        ));
    }
    let metrics = request.metrics_json.as_object();
    if metrics.is_none() || metrics.is_some_and(|metrics| metrics.is_empty()) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "INVALID_MODEL_EVALUATION",
            "metrics_json must be a non-empty object",
        ));
    }
    if let Some(feature_importance_uri) = &request.feature_importance_uri {
        if feature_importance_uri.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_MODEL_EVALUATION",
                "feature_importance_uri must not be blank when provided",
            ));
        }
        validate_parquet_uri(
            feature_importance_uri,
            "MODEL_EVALUATION_FEATURE_IMPORTANCE_FORMAT_INVALID",
        )?;
    }
    if let Some(permutation_importance_uri) = &request.permutation_importance_uri {
        if permutation_importance_uri.trim().is_empty() {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_MODEL_EVALUATION",
                "permutation_importance_uri must not be blank when provided",
            ));
        }
        validate_parquet_uri(
            permutation_importance_uri,
            "MODEL_EVALUATION_PERMUTATION_IMPORTANCE_FORMAT_INVALID",
        )?;
    }
    Ok(())
}

fn validate_unit_interval_metric(
    metric_name: &'static str,
    metric: &Option<Decimal>,
) -> Result<(), ApiError> {
    if let Some(metric) = metric {
        if *metric < Decimal::ZERO || *metric > Decimal::ONE {
            return Err(ApiError::new(
                StatusCode::BAD_REQUEST,
                "INVALID_MODEL_EVALUATION",
                format!("{metric_name} must be between 0 and 1"),
            ));
        }
    }
    Ok(())
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
    let lineage = build_model_evaluation_lineage(&state, &evaluations).await?;
    Ok(Json(ModelEvaluationListResponse {
        evaluations,
        lineage,
    }))
}

async fn build_model_evaluation_lineage(
    state: &AppState,
    evaluations: &[ModelEvaluationRecord],
) -> Result<Vec<ModelEvaluationLineageRecord>, ApiError> {
    let mut lineage = Vec::with_capacity(evaluations.len());
    for evaluation in evaluations {
        let source_dataset = state
            .repository
            .get_model_dataset_source_dataset(&evaluation.model_dataset_id)
            .await
            .map_err(internal_error("MODEL_DATASET_LINEAGE_FAILED"))?;
        let source_health = source_dataset.as_ref().map(build_dataset_health_record);
        lineage.push(ModelEvaluationLineageRecord {
            evaluation_run_id: evaluation.evaluation_run_id.clone(),
            model_key: evaluation.model_key.clone(),
            model_version: evaluation.model_version.clone(),
            model_dataset_id: evaluation.model_dataset_id.clone(),
            source_dataset_id: source_dataset
                .as_ref()
                .map(|dataset| dataset.dataset_id.clone()),
            source_dataset_key: source_dataset
                .as_ref()
                .map(|dataset| dataset.dataset_key.clone()),
            source_dataset_version: source_dataset
                .as_ref()
                .map(|dataset| dataset.dataset_version.clone()),
            source_data_quality_score: source_health
                .as_ref()
                .map(|health| health.data_quality_score),
            source_data_quality_status: source_health.map(|health| health.data_quality_status),
        });
    }
    Ok(lineage)
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
        ready_factor_count: 0,
        review_factor_count: 0,
        readiness_issue_counts: Map::new(),
        scheme_readiness: Vec::new(),
        factor_cards: Vec::new(),
    };
    let mut scheme_readiness = BTreeMap::<String, FactorSchemeReadinessRecord>::new();

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
            let factor_card = build_factor_card(dataset, field);
            if factor_card.readiness_status == "ready" {
                response.ready_factor_count += 1;
            } else {
                response.review_factor_count += 1;
            }
            update_scheme_readiness(&mut scheme_readiness, &factor_card);
            for issue in &factor_card.readiness_issues {
                let count = response
                    .readiness_issue_counts
                    .get(issue)
                    .and_then(Value::as_u64)
                    .unwrap_or(0)
                    + 1;
                response
                    .readiness_issue_counts
                    .insert(issue.clone(), Value::from(count));
            }
            response.factor_cards.push(factor_card);
        }
    }

    response.scheme_readiness = scheme_readiness.into_values().collect();
    response.data_quality_score = factor_data_quality_score(&response);
    response.data_quality_status = factor_data_quality_status(response.data_quality_score).into();

    response
}

fn update_scheme_readiness(
    scheme_readiness: &mut BTreeMap<String, FactorSchemeReadinessRecord>,
    factor_card: &FactorCardRecord,
) {
    let summary = scheme_readiness
        .entry(factor_card.scheme_family.clone())
        .or_insert_with(|| FactorSchemeReadinessRecord {
            scheme_family: factor_card.scheme_family.clone(),
            factor_count: 0,
            ready_factor_count: 0,
            review_factor_count: 0,
            online_ready_count: 0,
            rule_convertible_count: 0,
            readiness_issue_counts: Map::new(),
        });

    summary.factor_count += 1;
    if factor_card.readiness_status == "ready" {
        summary.ready_factor_count += 1;
    } else {
        summary.review_factor_count += 1;
    }
    if factor_card.online_available {
        summary.online_ready_count += 1;
    }
    if factor_card.rule_convertible {
        summary.rule_convertible_count += 1;
    }
    for issue in &factor_card.readiness_issues {
        let count = summary
            .readiness_issue_counts
            .get(issue)
            .and_then(Value::as_u64)
            .unwrap_or(0)
            + 1;
        summary
            .readiness_issue_counts
            .insert(issue.clone(), Value::from(count));
    }
}

fn build_factor_card(dataset: &DatasetRecord, field: &SchemaFieldRecord) -> FactorCardRecord {
    let is_label = field.semantic_role == "label";
    let is_entity_key = dataset.entity_keys.contains(&field.field_name);
    let missing_rate = numeric_profile_value(&field.profile_json, "missing_rate");
    let psi = numeric_profile_value(&field.profile_json, "psi");
    let source_fields = string_array_profile_value(&field.profile_json, "source_fields")
        .unwrap_or_else(|| vec![field.field_name.clone()]);
    let owner = string_profile_value(&field.profile_json, "owner").unwrap_or_default();
    let online_available = !is_label
        && bool_profile_value(&field.profile_json, "online_available").unwrap_or(!field.nullable);
    let readiness_issues =
        factor_readiness_issues(is_label, online_available, missing_rate, psi, &owner);
    let readiness_status = if readiness_issues.is_empty() {
        "ready"
    } else {
        "needs_review"
    };
    let mut evidence_refs = vec![format!(
        "dataset_fields:{}:{}:{}",
        dataset.dataset_key, dataset.dataset_version, field.field_name
    )];
    if let Some(profile_refs) = string_array_profile_value(&field.profile_json, "evidence_refs") {
        evidence_refs.extend(profile_refs);
        evidence_refs.sort();
        evidence_refs.dedup();
    }

    FactorCardRecord {
        dataset_id: dataset.dataset_id.clone(),
        dataset_key: dataset.dataset_key.clone(),
        dataset_version: dataset.dataset_version.clone(),
        factor_name: field.field_name.clone(),
        scheme_family: factor_scheme_family(field),
        chinese_name: string_profile_value(&field.profile_json, "chinese_name")
            .or_else(|| string_profile_value(&field.profile_json, "display_label"))
            .unwrap_or_else(|| titleize(&field.field_name)),
        entity_type: string_profile_value(&field.profile_json, "entity_type")
            .unwrap_or_else(|| dataset.sample_grain.clone()),
        semantic_role: field.semantic_role.clone(),
        logical_type: field.logical_type.clone(),
        calculation_window: string_profile_value(&field.profile_json, "calculation_window")
            .unwrap_or_else(|| dataset.sample_grain.clone()),
        calculation_logic: string_profile_value(&field.profile_json, "calculation_logic")
            .unwrap_or_else(|| "registered_dataset_field".into()),
        source_table: string_profile_value(&field.profile_json, "source_table")
            .unwrap_or_else(|| dataset.dataset_key.clone()),
        source_fields,
        business_meaning: string_profile_value(&field.profile_json, "business_meaning")
            .unwrap_or_else(|| field.description.clone()),
        risk_direction: string_profile_value(&field.profile_json, "risk_direction").unwrap_or_else(
            || {
                if is_label {
                    "label".into()
                } else {
                    "unknown".into()
                }
            },
        ),
        missing_rate,
        iv: numeric_profile_value(&field.profile_json, "iv"),
        auc_gain: numeric_profile_value(&field.profile_json, "auc_gain"),
        lift: numeric_profile_value(&field.profile_json, "lift"),
        psi,
        stability: stability_label(psi).into(),
        model_contribution: numeric_profile_value(&field.profile_json, "model_contribution"),
        rule_convertible: !is_label
            && bool_profile_value(&field.profile_json, "convertible_to_rule")
                .unwrap_or_else(|| is_rule_convertible_type(&field.logical_type)),
        online_available,
        readiness_status: readiness_status.into(),
        readiness_issues,
        version: format_factor_version(field.profile_json.get("version")),
        owner,
        is_label,
        is_entity_key,
        evidence_refs,
    }
}

pub(crate) fn build_dataset_health(datasets: &[DatasetRecord]) -> Vec<DatasetHealthRecord> {
    datasets.iter().map(build_dataset_health_record).collect()
}

pub(crate) fn build_dataset_health_record(dataset: &DatasetRecord) -> DatasetHealthRecord {
    let mut record = DatasetHealthRecord {
        dataset_id: dataset.dataset_id.clone(),
        dataset_key: dataset.dataset_key.clone(),
        dataset_version: dataset.dataset_version.clone(),
        data_quality_score: 0.0,
        data_quality_status: "empty".into(),
        field_count: dataset.fields.len() as u32,
        label_count: 0,
        entity_key_count: 0,
        high_missing_count: 0,
        unstable_field_count: 0,
        unowned_field_count: 0,
        online_ready_count: 0,
        issue_count: 0,
    };

    for field in &dataset.fields {
        let is_label = field.semantic_role == "label";
        let is_entity_key = dataset.entity_keys.contains(&field.field_name);
        let missing_rate = numeric_profile_value(&field.profile_json, "missing_rate");
        if is_label {
            record.label_count += 1;
        }
        if is_entity_key {
            record.entity_key_count += 1;
        }
        if !is_label && !field.nullable && missing_rate.unwrap_or(0.0) <= 0.05 {
            record.online_ready_count += 1;
        }
        if missing_rate.unwrap_or(0.0) > 0.20 {
            record.high_missing_count += 1;
        }
        if numeric_profile_value(&field.profile_json, "psi").unwrap_or(0.0) >= 0.25 {
            record.unstable_field_count += 1;
        }
        if field
            .profile_json
            .get("owner")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
        {
            record.unowned_field_count += 1;
        }
    }

    record.issue_count =
        record.high_missing_count + record.unstable_field_count + record.unowned_field_count;
    if record.field_count > 0 {
        record.data_quality_score =
            dataset_data_quality_score(record.field_count, record.issue_count);
        record.data_quality_status = factor_data_quality_status(record.data_quality_score).into();
    }

    record
}

fn dataset_data_quality_score(field_count: u32, issue_count: u32) -> f64 {
    let max_issue_count = field_count * 3;
    let score = 1.0 - (issue_count as f64 / max_issue_count as f64);
    score.clamp(0.0, 1.0)
}

fn factor_data_quality_score(response: &FactorReadinessResponse) -> f64 {
    if response.factor_count == 0 {
        return 0.0;
    }
    let issue_count = response.high_missing_count
        + response.unstable_factor_count
        + response.unowned_factor_count;
    dataset_data_quality_score(response.factor_count, issue_count)
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

fn string_profile_value(profile: &Value, key: &str) -> Option<String> {
    profile
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn string_array_profile_value(profile: &Value, key: &str) -> Option<Vec<String>> {
    let values = profile.get(key)?.as_array()?;
    let values = values
        .iter()
        .filter_map(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    (!values.is_empty()).then_some(values)
}

fn bool_profile_value(profile: &Value, key: &str) -> Option<bool> {
    profile.get(key).and_then(Value::as_bool)
}

fn format_factor_version(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(version)) if !version.is_empty() => version.clone(),
        Some(Value::Number(version)) => version
            .as_u64()
            .map(|version| format!("v{version}"))
            .unwrap_or_else(|| "v1".into()),
        _ => "v1".into(),
    }
}

fn stability_label(psi: Option<f64>) -> &'static str {
    match psi {
        None => "unmeasured",
        Some(value) if value < 0.10 => "stable",
        Some(value) if value < 0.25 => "watch",
        Some(_) => "drift",
    }
}

fn factor_readiness_issues(
    is_label: bool,
    online_available: bool,
    missing_rate: Option<f64>,
    psi: Option<f64>,
    owner: &str,
) -> Vec<String> {
    let mut issues = Vec::new();
    if is_label {
        issues.push("label_field".into());
    }
    if !online_available {
        issues.push("not_online_available".into());
    }
    if missing_rate.unwrap_or(0.0) > 0.05 {
        issues.push("online_missing_rate_above_threshold".into());
    }
    if missing_rate.unwrap_or(0.0) > 0.20 {
        issues.push("high_missing_rate".into());
    }
    if psi.unwrap_or(0.0) >= 0.25 {
        issues.push("unstable_distribution".into());
    }
    if owner.trim().is_empty() {
        issues.push("missing_owner".into());
    }
    issues
}

fn factor_scheme_family(field: &SchemaFieldRecord) -> String {
    if let Some(scheme_family) = string_profile_value(&field.profile_json, "scheme_family")
        .and_then(|value| canonical_scheme_family(&value))
    {
        return scheme_family;
    }

    let factor_name = field.field_name.as_str();
    let description = field.description.to_ascii_lowercase();
    let text = format!("{}_{}", factor_name, description);
    let inferred = if text.contains("duplicate") {
        "duplicate_billing"
    } else if text.contains("diagnosis_procedure") || text.contains("diagnosis procedure") {
        "diagnosis_procedure_mismatch"
    } else if text.contains("clinical_review")
        || text.contains("medical_reasonableness")
        || text.contains("medical necessity")
    {
        "medically_unnecessary_service"
    } else if text.contains("provider") {
        "provider_peer_outlier"
    } else if text.contains("service_count")
        || text.contains("utilization")
        || text.contains("item_count")
    {
        "excessive_utilization"
    } else if text.contains("days_since_policy_start")
        || text.contains("amount_to_limit")
        || text.contains("early")
    {
        "early_high_value_claim"
    } else {
        "high_risk_claim"
    };

    inferred.into()
}

fn titleize(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
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
    validate_dataset_metadata_has_no_pii(request)?;

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

fn validate_dataset_metadata_has_no_pii(request: &RegisterDatasetInput) -> Result<(), ApiError> {
    let mut metadata = Vec::new();
    metadata.extend([
        request.source_key.as_str(),
        request.display_name.as_str(),
        request.business_domain.as_str(),
        request.owner.as_str(),
        request.description.as_str(),
        request.dataset_key.as_str(),
        request.dataset_version.as_str(),
        request.sample_grain.as_str(),
        request.label_column.as_str(),
        request.manifest_uri.as_str(),
        request.schema_uri.as_str(),
        request.profile_uri.as_str(),
        request.schema_hash.as_str(),
        request.status.as_str(),
    ]);
    metadata.extend(request.entity_keys.iter().map(String::as_str));
    for split in &request.splits {
        metadata.extend([split.split_name.as_str(), split.data_uri.as_str()]);
    }
    for field in &request.fields {
        metadata.extend([
            field.field_name.as_str(),
            field.logical_type.as_str(),
            field.semantic_role.as_str(),
            field.description.as_str(),
        ]);
    }

    if pii::contains_pii(metadata) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "PII_NOT_ALLOWED_IN_DATASET_METADATA",
            "dataset and factor metadata must not contain PII",
        ));
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
    let normalized = value
        .trim()
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if normalized.ends_with(".parquet") || normalized.ends_with('/') {
        Ok(())
    } else {
        Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            code,
            "dataset artifact URIs must point to parquet files or parquet partition directories",
        ))
    }
}

struct DataLineageAuditInput {
    event_type: &'static str,
    summary: &'static str,
    payload: Value,
    evidence_refs: Vec<String>,
}

async fn record_data_lineage_audit(
    state: &AppState,
    actor: &ActorContext,
    input: DataLineageAuditInput,
) -> anyhow::Result<()> {
    let mut payload = input.payload;
    if let Some(payload) = payload.as_object_mut() {
        payload.insert(
            "customer_scope_id".into(),
            serde_json::json!(actor.customer_scope_id),
        );
    }
    state
        .repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: AuditEventId::new().to_string(),
            run_id: ScoringRunId::new().to_string(),
            claim_id: String::new(),
            source_system: actor.source_system.clone(),
            actor_id: actor.actor_id.clone(),
            actor_role: actor.actor_role.clone(),
            event_type: input.event_type.into(),
            event_status: "succeeded".into(),
            summary: input.summary.into(),
            payload,
            evidence_refs: input
                .evidence_refs
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        })
        .await
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<ActorContext, ApiError> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    validate_api_key(api_key, &state.config.api_key_config()).map_err(|_| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "INVALID_API_KEY",
            "invalid api key",
        )
    })
}

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::internal(code, error)
}

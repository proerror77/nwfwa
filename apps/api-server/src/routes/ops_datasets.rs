use crate::{
    app::AppState,
    auth::AuthenticatedActor,
    error::ApiError,
    repository::{
        CreateFieldMappingInput, DatasetRecord, FeatureSetRecord, ModelDatasetRecord,
        ModelEvaluationRecord, PersistedAuditEvent, RegisterDatasetInput, RegisterFeatureSetInput,
        RegisterModelDatasetInput, RegisterModelEvaluationInput,
    },
    routes::pii,
};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use fwa_audit::ActorContext;
use fwa_core::{canonical_scheme_family, AuditEventId, ScoringRunId};
use rust_decimal::Decimal;
use serde_json::{json, Value};

pub(crate) use super::ops_datasets_readiness::build_dataset_health_record;
use super::ops_datasets_readiness::{build_dataset_health, build_factor_readiness};
pub use super::ops_datasets_types::*;

pub async fn register_dataset(
    State(state): State<AppState>,
    AuthenticatedActor(actor): AuthenticatedActor,
    Json(request): Json<RegisterDatasetInput>,
) -> Result<Json<DatasetRecord>, ApiError> {
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
    _actor: AuthenticatedActor,
) -> Result<Json<DatasetListResponse>, ApiError> {
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
    _actor: AuthenticatedActor,
) -> Result<Json<FactorReadinessResponse>, ApiError> {
    let datasets = state
        .repository
        .list_datasets()
        .await
        .map_err(internal_error("DATASET_LIST_FAILED"))?;
    Ok(Json(build_factor_readiness(&datasets)))
}

pub async fn get_dataset(
    State(state): State<AppState>,
    _actor: AuthenticatedActor,
    Path(dataset_id): Path<String>,
) -> Result<Json<DatasetRecord>, ApiError> {
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
    AuthenticatedActor(actor): AuthenticatedActor,
    Path(dataset_id): Path<String>,
    Json(request): Json<CreateFieldMappingInput>,
) -> Result<Json<FieldMappingResponse>, ApiError> {
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
    AuthenticatedActor(actor): AuthenticatedActor,
    Json(request): Json<RegisterFeatureSetInput>,
) -> Result<Json<FeatureSetRecord>, ApiError> {
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
    AuthenticatedActor(actor): AuthenticatedActor,
    Json(request): Json<RegisterModelDatasetInput>,
) -> Result<Json<ModelDatasetRecord>, ApiError> {
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
    AuthenticatedActor(actor): AuthenticatedActor,
    Json(mut request): Json<RegisterModelEvaluationInput>,
) -> Result<Json<ModelEvaluationResponse>, ApiError> {
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
    _actor: AuthenticatedActor,
    Path(evaluation_run_id): Path<String>,
) -> Result<Json<ModelEvaluationResponse>, ApiError> {
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
    _actor: AuthenticatedActor,
) -> Result<Json<ModelEvaluationListResponse>, ApiError> {
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

fn internal_error<E: std::fmt::Display>(code: &'static str) -> impl FnOnce(E) -> ApiError {
    move |error| ApiError::internal(code, error)
}

use crate::{
    app::AppState,
    auth::AuthenticatedActor,
    error::ApiError,
    repository::{
        CreateFieldMappingInput, DatasetRecord, FeatureSetRecord, ModelDatasetRecord,
        ModelEvaluationRecord, PersistedAuditEvent, RegisterDatasetInput, RegisterFeatureSetInput,
        RegisterModelDatasetInput, RegisterModelEvaluationInput,
    },
};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use fwa_audit::ActorContext;
use fwa_core::{canonical_scheme_family, AuditEventId, ScoringRunId};
use serde_json::{json, Value};

mod validation;

pub(crate) use super::ops_datasets_readiness::build_dataset_health_record;
use super::ops_datasets_readiness::{build_dataset_health, build_factor_readiness};
pub use super::ops_datasets_types::*;
use validation::{
    validate_dataset_contract, validate_feature_set_registration, validate_field_mapping,
    validate_model_dataset_registration, validate_model_evaluation_registration,
    validate_parquet_uri,
};

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

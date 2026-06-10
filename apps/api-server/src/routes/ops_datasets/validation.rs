use crate::routes::pii;
use rust_decimal::Decimal;

use super::*;

pub(super) fn validate_field_mapping(request: &CreateFieldMappingInput) -> Result<(), ApiError> {
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

pub(super) fn validate_feature_set_registration(
    request: &RegisterFeatureSetInput,
) -> Result<(), ApiError> {
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

pub(super) fn validate_model_dataset_registration(
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

pub(super) fn validate_model_evaluation_registration(
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

pub(super) fn validate_dataset_contract(request: &RegisterDatasetInput) -> Result<(), ApiError> {
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

pub(super) fn validate_parquet_uri(value: &str, code: &'static str) -> Result<(), ApiError> {
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

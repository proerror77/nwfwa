use super::{
    json_array_to_strings, DatasetRecord, DatasetSplitRecord, FieldMappingRecord, SchemaFieldRecord,
};
use serde_json::Value;
use sqlx::PgPool;

type DatasetRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    Value,
    String,
    String,
    String,
    String,
    String,
    i64,
    String,
);
type DatasetSplitRow = (String, String, i64, Option<i64>, Option<i64>, Value);
type DatasetMappingRow = (
    String,
    String,
    String,
    Option<String>,
    String,
    Value,
    String,
);

pub(super) async fn load_dataset_record(
    pool: &PgPool,
    dataset_id: &str,
) -> anyhow::Result<Option<DatasetRecord>> {
    let row: Option<DatasetRow> = sqlx::query_as(
        "SELECT d.id::text,
                d.source_key,
                s.display_name,
                s.business_domain,
                d.dataset_key,
                d.dataset_version,
                d.sample_grain,
                d.label_column,
                d.entity_keys,
                d.manifest_uri,
                d.schema_uri,
                d.profile_uri,
                d.storage_format,
                d.schema_hash,
                d.row_count,
                d.status
         FROM external_dataset_versions d
         JOIN external_data_sources s ON s.source_key = d.source_key
         WHERE d.id = $1::uuid",
    )
    .bind(dataset_id)
    .fetch_optional(pool)
    .await?;

    let Some((
        dataset_id,
        source_key,
        display_name,
        business_domain,
        dataset_key,
        dataset_version,
        sample_grain,
        label_column,
        entity_keys,
        manifest_uri,
        schema_uri,
        profile_uri,
        storage_format,
        schema_hash,
        row_count,
        status,
    )) = row
    else {
        return Ok(None);
    };

    let split_rows: Vec<DatasetSplitRow> = sqlx::query_as(
        "SELECT split_name, data_uri, row_count, positive_count, negative_count, label_distribution_json
         FROM external_dataset_splits
         WHERE dataset_id = $1::uuid
         ORDER BY split_name",
    )
    .bind(&dataset_id)
    .fetch_all(pool)
    .await?;

    let field_rows: Vec<(String, String, bool, String, String, Value)> = sqlx::query_as(
        "SELECT field_name, logical_type, nullable, semantic_role, description, profile_json
         FROM external_schema_fields
         WHERE dataset_id = $1::uuid
         ORDER BY field_name",
    )
    .bind(&dataset_id)
    .fetch_all(pool)
    .await?;

    let mapping_rows: Vec<DatasetMappingRow> = sqlx::query_as(
        "SELECT id::text, external_field, canonical_target, feature_name, transform_kind, transform_json, status
             FROM external_field_mappings
             WHERE dataset_id = $1::uuid
             ORDER BY created_at, external_field",
    )
    .bind(&dataset_id)
    .fetch_all(pool)
    .await?;

    Ok(Some(DatasetRecord {
        dataset_id: dataset_id.clone(),
        source_key,
        display_name,
        business_domain,
        dataset_key,
        dataset_version,
        sample_grain,
        label_column,
        entity_keys: json_array_to_strings(entity_keys),
        manifest_uri,
        schema_uri,
        profile_uri,
        storage_format,
        schema_hash,
        row_count: row_count as u64,
        status,
        splits: split_rows
            .into_iter()
            .map(
                |(
                    split_name,
                    data_uri,
                    row_count,
                    positive_count,
                    negative_count,
                    label_distribution_json,
                )| DatasetSplitRecord {
                    split_name,
                    data_uri,
                    row_count: row_count as u64,
                    positive_count: positive_count.map(|value| value as u64),
                    negative_count: negative_count.map(|value| value as u64),
                    label_distribution_json,
                },
            )
            .collect(),
        fields: field_rows
            .into_iter()
            .map(
                |(field_name, logical_type, nullable, semantic_role, description, profile_json)| {
                    SchemaFieldRecord {
                        field_name,
                        logical_type,
                        nullable,
                        semantic_role,
                        description,
                        profile_json,
                    }
                },
            )
            .collect(),
        mappings: mapping_rows
            .into_iter()
            .map(
                |(
                    mapping_id,
                    external_field,
                    canonical_target,
                    feature_name,
                    transform_kind,
                    transform_json,
                    status,
                )| FieldMappingRecord {
                    mapping_id,
                    dataset_id: dataset_id.clone(),
                    external_field,
                    canonical_target,
                    feature_name,
                    transform_kind,
                    transform_json,
                    status,
                },
            )
            .collect(),
    }))
}

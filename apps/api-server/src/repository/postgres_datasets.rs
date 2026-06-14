use super::dataset_rows::load_dataset_record;
use super::*;

pub(super) async fn register_dataset(
    repository: &PostgresScoringRepository,
    input: RegisterDatasetInput,
) -> anyhow::Result<DatasetRecord> {
    let mut tx = repository.pool.begin().await?;
    sqlx::query(
        "INSERT INTO external_data_sources
             (source_key, display_name, business_domain, owner, description, status)
             VALUES ($1, $2, $3, $4, $5, 'active')
             ON CONFLICT (source_key) DO UPDATE
             SET display_name = EXCLUDED.display_name,
                 business_domain = EXCLUDED.business_domain,
                 owner = EXCLUDED.owner,
                 description = EXCLUDED.description,
                 updated_at = now()",
    )
    .bind(&input.source_key)
    .bind(&input.display_name)
    .bind(&input.business_domain)
    .bind(&input.owner)
    .bind(&input.description)
    .execute(&mut *tx)
    .await?;

    let dataset_row: (String,) = sqlx::query_as(
        "INSERT INTO external_dataset_versions
             (source_key, dataset_key, dataset_version, sample_grain, label_column, entity_keys, manifest_uri, schema_uri, profile_uri, storage_format, schema_hash, row_count, status)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (dataset_key, dataset_version) DO UPDATE
             SET manifest_uri = EXCLUDED.manifest_uri,
                 schema_uri = EXCLUDED.schema_uri,
                 profile_uri = EXCLUDED.profile_uri,
                 schema_hash = EXCLUDED.schema_hash,
                 row_count = EXCLUDED.row_count,
                 status = EXCLUDED.status
             RETURNING id::text",
    )
    .bind(&input.source_key)
    .bind(&input.dataset_key)
    .bind(&input.dataset_version)
    .bind(&input.sample_grain)
    .bind(&input.label_column)
    .bind(serde_json::json!(input.entity_keys))
    .bind(&input.manifest_uri)
    .bind(&input.schema_uri)
    .bind(&input.profile_uri)
    .bind(&input.storage_format)
    .bind(&input.schema_hash)
    .bind(input.row_count as i64)
    .bind(&input.status)
    .fetch_one(&mut *tx)
    .await?;

    for split in &input.splits {
        sqlx::query(
            "INSERT INTO external_dataset_splits
                 (dataset_id, split_name, data_uri, row_count, positive_count, negative_count, label_distribution_json)
                 VALUES ($1::uuid, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (dataset_id, split_name) DO UPDATE
                 SET data_uri = EXCLUDED.data_uri,
                     row_count = EXCLUDED.row_count,
                     positive_count = EXCLUDED.positive_count,
                     negative_count = EXCLUDED.negative_count,
                     label_distribution_json = EXCLUDED.label_distribution_json",
        )
        .bind(&dataset_row.0)
        .bind(&split.split_name)
        .bind(&split.data_uri)
        .bind(split.row_count as i64)
        .bind(split.positive_count.map(|value| value as i64))
        .bind(split.negative_count.map(|value| value as i64))
        .bind(&split.label_distribution_json)
        .execute(&mut *tx)
        .await?;
    }

    for field in &input.fields {
        sqlx::query(
            "INSERT INTO external_schema_fields
                 (dataset_id, field_name, logical_type, nullable, semantic_role, description, profile_json)
                 VALUES ($1::uuid, $2, $3, $4, $5, $6, $7)
                 ON CONFLICT (dataset_id, field_name) DO UPDATE
                 SET logical_type = EXCLUDED.logical_type,
                     nullable = EXCLUDED.nullable,
                     semantic_role = EXCLUDED.semantic_role,
                     description = EXCLUDED.description,
                     profile_json = EXCLUDED.profile_json",
        )
        .bind(&dataset_row.0)
        .bind(&field.field_name)
        .bind(&field.logical_type)
        .bind(field.nullable)
        .bind(&field.semantic_role)
        .bind(&field.description)
        .bind(&field.profile_json)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    load_dataset_record(&repository.pool, &dataset_row.0)
        .await?
        .ok_or_else(|| anyhow::anyhow!("registered dataset was not found"))
}

pub(super) async fn list_datasets(
    repository: &PostgresScoringRepository,
) -> anyhow::Result<Vec<DatasetRecord>> {
    let ids: Vec<(String,)> = sqlx::query_as(
        "SELECT id::text FROM external_dataset_versions ORDER BY dataset_key, dataset_version",
    )
    .fetch_all(&repository.pool)
    .await?;
    let mut datasets = Vec::new();
    for (id,) in ids {
        if let Some(dataset) = load_dataset_record(&repository.pool, &id).await? {
            datasets.push(dataset);
        }
    }
    Ok(datasets)
}

pub(super) async fn get_dataset(
    repository: &PostgresScoringRepository,
    dataset_id: &str,
) -> anyhow::Result<Option<DatasetRecord>> {
    load_dataset_record(&repository.pool, dataset_id).await
}

pub(super) async fn add_field_mapping(
    repository: &PostgresScoringRepository,
    dataset_id: &str,
    input: CreateFieldMappingInput,
) -> anyhow::Result<Option<FieldMappingRecord>> {
    if load_dataset_record(&repository.pool, dataset_id)
        .await?
        .is_none()
    {
        return Ok(None);
    }

    let row: (String,) = sqlx::query_as(
        "INSERT INTO external_field_mappings
             (dataset_id, external_field, canonical_target, feature_name, transform_kind, transform_json, status)
             VALUES ($1::uuid, $2, $3, $4, $5, $6, $7)
             RETURNING id::text",
    )
    .bind(dataset_id)
    .bind(&input.external_field)
    .bind(&input.canonical_target)
    .bind(&input.feature_name)
    .bind(&input.transform_kind)
    .bind(&input.transform_json)
    .bind(&input.status)
    .fetch_one(&repository.pool)
    .await?;

    Ok(Some(FieldMappingRecord {
        mapping_id: row.0,
        dataset_id: dataset_id.to_string(),
        external_field: input.external_field,
        canonical_target: input.canonical_target,
        feature_name: input.feature_name,
        transform_kind: input.transform_kind,
        transform_json: input.transform_json,
        status: input.status,
    }))
}

pub(super) async fn register_feature_set(
    repository: &PostgresScoringRepository,
    input: RegisterFeatureSetInput,
) -> anyhow::Result<Option<FeatureSetRecord>> {
    if load_dataset_record(&repository.pool, &input.dataset_id)
        .await?
        .is_none()
    {
        return Ok(None);
    }
    let row: (String,) = sqlx::query_as(
        "INSERT INTO feature_set_versions
             (feature_set_key, business_domain, version, dataset_id, features_uri, feature_list_json, row_count, label_column, status)
             VALUES ($1, $2, $3, $4::uuid, $5, $6, $7, $8, $9)
             ON CONFLICT (feature_set_key, version) DO UPDATE
             SET features_uri = EXCLUDED.features_uri,
                 feature_list_json = EXCLUDED.feature_list_json,
                 row_count = EXCLUDED.row_count,
                 status = EXCLUDED.status
             RETURNING id::text",
    )
    .bind(&input.feature_set_key)
    .bind(&input.business_domain)
    .bind(&input.version)
    .bind(&input.dataset_id)
    .bind(&input.features_uri)
    .bind(&input.feature_list_json)
    .bind(input.row_count as i64)
    .bind(&input.label_column)
    .bind(&input.status)
    .fetch_one(&repository.pool)
    .await?;
    Ok(Some(FeatureSetRecord {
        feature_set_id: row.0,
        business_domain: input.business_domain,
        feature_set_key: input.feature_set_key,
        version: input.version,
        dataset_id: input.dataset_id,
        features_uri: input.features_uri,
        feature_list_json: input.feature_list_json,
        row_count: input.row_count,
        label_column: input.label_column,
        status: input.status,
    }))
}

pub(super) async fn register_model_dataset(
    repository: &PostgresScoringRepository,
    input: RegisterModelDatasetInput,
) -> anyhow::Result<Option<ModelDatasetRecord>> {
    let feature_set_known: Option<(String,)> =
        sqlx::query_as("SELECT id::text FROM feature_set_versions WHERE id = $1::uuid")
            .bind(&input.feature_set_id)
            .fetch_optional(&repository.pool)
            .await?;
    if feature_set_known.is_none() {
        return Ok(None);
    }

    let row: (String,) = sqlx::query_as(
        "INSERT INTO model_dataset_versions
             (business_domain, task_type, label_name, feature_set_id, train_uri, validation_uri, test_uri, row_counts_json, label_distribution_json, status)
             VALUES ($1, $2, $3, $4::uuid, $5, $6, $7, $8, $9, $10)
             RETURNING id::text",
    )
    .bind(&input.business_domain)
    .bind(&input.task_type)
    .bind(&input.label_name)
    .bind(&input.feature_set_id)
    .bind(&input.train_uri)
    .bind(&input.validation_uri)
    .bind(&input.test_uri)
    .bind(&input.row_counts_json)
    .bind(&input.label_distribution_json)
    .bind(&input.status)
    .fetch_one(&repository.pool)
    .await?;

    Ok(Some(ModelDatasetRecord {
        model_dataset_id: row.0,
        business_domain: input.business_domain,
        task_type: input.task_type,
        label_name: input.label_name,
        feature_set_id: input.feature_set_id,
        train_uri: input.train_uri,
        validation_uri: input.validation_uri,
        test_uri: input.test_uri,
        row_counts_json: input.row_counts_json,
        label_distribution_json: input.label_distribution_json,
        status: input.status,
    }))
}

pub(super) async fn get_model_dataset_source_dataset(
    repository: &PostgresScoringRepository,
    model_dataset_id: &str,
) -> anyhow::Result<Option<DatasetRecord>> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT fs.dataset_id::text
             FROM model_dataset_versions md
             JOIN feature_set_versions fs ON fs.id = md.feature_set_id
             WHERE md.id = $1::uuid",
    )
    .bind(model_dataset_id)
    .fetch_optional(&repository.pool)
    .await?;

    let Some((dataset_id,)) = row else {
        return Ok(None);
    };
    load_dataset_record(&repository.pool, &dataset_id).await
}

pub(super) async fn register_model_evaluation(
    repository: &PostgresScoringRepository,
    input: RegisterModelEvaluationInput,
) -> anyhow::Result<Option<ModelEvaluationRecord>> {
    let model_dataset_known: Option<(String,)> =
        sqlx::query_as("SELECT id::text FROM model_dataset_versions WHERE id = $1::uuid")
            .bind(&input.model_dataset_id)
            .fetch_optional(&repository.pool)
            .await?;
    if model_dataset_known.is_none() {
        return Ok(None);
    }

    sqlx::query(
        "INSERT INTO model_evaluation_runs
             (evaluation_run_id, model_key, model_version, model_dataset_id, scheme_family, auc, ks, precision_value, recall_value, f1, accuracy, threshold, confusion_matrix_json, feature_importance_uri, permutation_importance_uri, metrics_json)
             VALUES ($1, $2, $3, $4::uuid, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
             ON CONFLICT (evaluation_run_id) DO UPDATE
             SET model_key = EXCLUDED.model_key,
                 model_version = EXCLUDED.model_version,
                 model_dataset_id = EXCLUDED.model_dataset_id,
                 scheme_family = EXCLUDED.scheme_family,
                 auc = EXCLUDED.auc,
                 ks = EXCLUDED.ks,
                 precision_value = EXCLUDED.precision_value,
                 recall_value = EXCLUDED.recall_value,
                 f1 = EXCLUDED.f1,
                 accuracy = EXCLUDED.accuracy,
                 threshold = EXCLUDED.threshold,
                 confusion_matrix_json = EXCLUDED.confusion_matrix_json,
                 feature_importance_uri = EXCLUDED.feature_importance_uri,
                 permutation_importance_uri = EXCLUDED.permutation_importance_uri,
                 metrics_json = EXCLUDED.metrics_json",
    )
    .bind(&input.evaluation_run_id)
    .bind(&input.model_key)
    .bind(&input.model_version)
    .bind(&input.model_dataset_id)
    .bind(&input.scheme_family)
    .bind(input.auc)
    .bind(input.ks)
    .bind(input.precision)
    .bind(input.recall)
    .bind(input.f1)
    .bind(input.accuracy)
    .bind(input.threshold)
    .bind(&input.confusion_matrix_json)
    .bind(&input.feature_importance_uri)
    .bind(&input.permutation_importance_uri)
    .bind(&input.metrics_json)
    .execute(&repository.pool)
    .await?;

    get_model_evaluation(repository, &input.evaluation_run_id).await
}

pub(super) async fn get_model_evaluation(
    repository: &PostgresScoringRepository,
    evaluation_run_id: &str,
) -> anyhow::Result<Option<ModelEvaluationRecord>> {
    let row: Option<(
        String,
        String,
        String,
        String,
        String,
        Option<Decimal>,
        Option<Decimal>,
        Option<Decimal>,
        Option<Decimal>,
        Option<Decimal>,
        Option<Decimal>,
        Option<Decimal>,
        Value,
        Option<String>,
        Option<String>,
        Value,
    )> = sqlx::query_as(
        "SELECT evaluation_run_id, model_key, model_version, model_dataset_id::text, scheme_family, auc, ks, precision_value, recall_value, f1, accuracy, threshold, confusion_matrix_json, feature_importance_uri, permutation_importance_uri, metrics_json
             FROM model_evaluation_runs
             WHERE evaluation_run_id = $1",
    )
    .bind(evaluation_run_id)
    .fetch_optional(&repository.pool)
    .await?;

    Ok(row.map(
        |(
            evaluation_run_id,
            model_key,
            model_version,
            model_dataset_id,
            scheme_family,
            auc,
            ks,
            precision,
            recall,
            f1,
            accuracy,
            threshold,
            confusion_matrix_json,
            feature_importance_uri,
            permutation_importance_uri,
            metrics_json,
        )| ModelEvaluationRecord {
            evaluation_run_id,
            model_key,
            model_version,
            model_dataset_id,
            scheme_family,
            auc,
            ks,
            precision,
            recall,
            f1,
            accuracy,
            threshold,
            confusion_matrix_json,
            feature_importance_uri,
            permutation_importance_uri,
            metrics_json,
        },
    ))
}

pub(super) async fn list_model_evaluations(
    repository: &PostgresScoringRepository,
) -> anyhow::Result<Vec<ModelEvaluationRecord>> {
    let rows: Vec<(
        String,
        String,
        String,
        String,
        String,
        Option<Decimal>,
        Option<Decimal>,
        Option<Decimal>,
        Option<Decimal>,
        Option<Decimal>,
        Option<Decimal>,
        Option<Decimal>,
        Value,
        Option<String>,
        Option<String>,
        Value,
    )> = sqlx::query_as(
        "SELECT evaluation_run_id, model_key, model_version, model_dataset_id::text, scheme_family, auc, ks, precision_value, recall_value, f1, accuracy, threshold, confusion_matrix_json, feature_importance_uri, permutation_importance_uri, metrics_json
             FROM model_evaluation_runs
             ORDER BY evaluation_run_id",
    )
    .fetch_all(&repository.pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(
            |(
                evaluation_run_id,
                model_key,
                model_version,
                model_dataset_id,
                scheme_family,
                auc,
                ks,
                precision,
                recall,
                f1,
                accuracy,
                threshold,
                confusion_matrix_json,
                feature_importance_uri,
                permutation_importance_uri,
                metrics_json,
            )| ModelEvaluationRecord {
                evaluation_run_id,
                model_key,
                model_version,
                model_dataset_id,
                scheme_family,
                auc,
                ks,
                precision,
                recall,
                f1,
                accuracy,
                threshold,
                confusion_matrix_json,
                feature_importance_uri,
                permutation_importance_uri,
                metrics_json,
            },
        )
        .collect())
}

pub(super) async fn save_scoring_feature_context_materialization(
    repository: &PostgresScoringRepository,
    input: SaveScoringFeatureContextMaterializationInput,
) -> anyhow::Result<ScoringFeatureContextMaterializationRecord> {
    sqlx::query(
        "INSERT INTO scoring_feature_context_materializations
             (materialization_id, customer_scope_id, as_of_date, report_uri, report_kind, source_uris, claim_count, context_count, contexts_json, evidence_refs, governance_boundary, submitted_by, notes)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
             ON CONFLICT (customer_scope_id, materialization_id) DO UPDATE
             SET customer_scope_id = EXCLUDED.customer_scope_id,
                 as_of_date = EXCLUDED.as_of_date,
                 report_uri = EXCLUDED.report_uri,
                 report_kind = EXCLUDED.report_kind,
                 source_uris = EXCLUDED.source_uris,
                 claim_count = EXCLUDED.claim_count,
                 context_count = EXCLUDED.context_count,
                 contexts_json = EXCLUDED.contexts_json,
                 evidence_refs = EXCLUDED.evidence_refs,
                 governance_boundary = EXCLUDED.governance_boundary,
                 submitted_by = EXCLUDED.submitted_by,
                 notes = EXCLUDED.notes",
    )
    .bind(&input.materialization_id)
    .bind(&input.customer_scope_id)
    .bind(&input.as_of_date)
    .bind(&input.report_uri)
    .bind(&input.report_kind)
    .bind(&input.source_uris)
    .bind(input.claim_count as i64)
    .bind(input.context_count as i64)
    .bind(&input.contexts_json)
    .bind(serde_json::json!(input.evidence_refs))
    .bind(&input.governance_boundary)
    .bind(&input.submitted_by)
    .bind(&input.notes)
    .execute(&repository.pool)
    .await?;

    get_scoring_feature_context_materialization(
        repository,
        &input.materialization_id,
        Some(&input.customer_scope_id),
    )
    .await?
    .ok_or_else(|| anyhow::anyhow!("saved scoring feature context materialization was not found"))
}

pub(super) async fn get_scoring_feature_context_materialization(
    repository: &PostgresScoringRepository,
    materialization_id: &str,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Option<ScoringFeatureContextMaterializationRecord>> {
    let row: Option<(
        String,
        String,
        String,
        String,
        String,
        Value,
        i32,
        i32,
        Value,
        Value,
        String,
        String,
        String,
    )> = sqlx::query_as(
        "SELECT materialization_id, customer_scope_id, as_of_date, report_uri, report_kind, source_uris, claim_count, context_count, contexts_json, evidence_refs, governance_boundary, submitted_by, notes
             FROM scoring_feature_context_materializations
             WHERE materialization_id = $1
               AND ($2::text IS NULL OR customer_scope_id = $2)",
    )
    .bind(materialization_id)
    .bind(customer_scope_id)
    .fetch_optional(&repository.pool)
    .await?;

    Ok(row.map(
        |(
            materialization_id,
            customer_scope_id,
            as_of_date,
            report_uri,
            report_kind,
            source_uris,
            claim_count,
            context_count,
            contexts_json,
            evidence_refs,
            governance_boundary,
            submitted_by,
            notes,
        )| ScoringFeatureContextMaterializationRecord {
            materialization_id,
            customer_scope_id,
            as_of_date,
            report_uri,
            report_kind,
            source_uris,
            claim_count: claim_count as u64,
            context_count: context_count as u64,
            contexts_json,
            evidence_refs: serde_json::from_value(evidence_refs).unwrap_or_default(),
            governance_boundary,
            submitted_by,
            notes,
        },
    ))
}

pub(super) async fn latest_scoring_feature_context_for_claim(
    repository: &PostgresScoringRepository,
    claim_id: &str,
    customer_scope_id: Option<&str>,
) -> anyhow::Result<Option<ScoringFeatureContextForClaimRecord>> {
    let row: Option<(String, String, String, Value, Value)> = sqlx::query_as(
        "SELECT materialization_id, as_of_date, report_uri, context_json, evidence_refs
             FROM scoring_feature_context_materializations
             CROSS JOIN LATERAL jsonb_array_elements(contexts_json) AS context_json
             WHERE context_json->>'claim_id' = $1
               AND ($2::text IS NULL OR customer_scope_id = $2)
             ORDER BY as_of_date DESC, created_at DESC, materialization_id DESC
             LIMIT 1",
    )
    .bind(claim_id)
    .bind(customer_scope_id)
    .fetch_optional(&repository.pool)
    .await?;

    Ok(row.map(
        |(materialization_id, as_of_date, report_uri, context_json, evidence_refs)| {
            ScoringFeatureContextForClaimRecord {
                materialization_id,
                as_of_date,
                report_uri,
                context_json,
                evidence_refs: serde_json::from_value(evidence_refs).unwrap_or_default(),
            }
        },
    ))
}

pub(super) async fn save_clinical_compatibility_references(
    repository: &PostgresScoringRepository,
    input: SaveClinicalCompatibilityReferencesInput,
) -> anyhow::Result<Vec<ClinicalCompatibilityReferenceRecord>> {
    let mut saved = Vec::with_capacity(input.records.len());
    let mut tx = repository.pool.begin().await?;
    for record in input.records {
        let evidence_refs = serde_json::Value::Array(
            record
                .evidence_refs
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        );
        sqlx::query(
            "INSERT INTO clinical_compatibility_references
                 (customer_scope_id, compatibility_key, reference_version, effective_date, source_authority, diagnosis_code_prefix, procedure_code, diagnosis_procedure_match_score, data_source, policy_authority_ref, rationale, evidence_refs, source_report_uri, submitted_by, notes)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                 ON CONFLICT (customer_scope_id, compatibility_key, reference_version) DO UPDATE
                 SET effective_date = EXCLUDED.effective_date,
                     source_authority = EXCLUDED.source_authority,
                     diagnosis_code_prefix = EXCLUDED.diagnosis_code_prefix,
                     procedure_code = EXCLUDED.procedure_code,
                     diagnosis_procedure_match_score = EXCLUDED.diagnosis_procedure_match_score,
                     data_source = EXCLUDED.data_source,
                     policy_authority_ref = EXCLUDED.policy_authority_ref,
                     rationale = EXCLUDED.rationale,
                     evidence_refs = EXCLUDED.evidence_refs,
                     source_report_uri = EXCLUDED.source_report_uri,
                     submitted_by = EXCLUDED.submitted_by,
                     notes = EXCLUDED.notes,
                     updated_at = now()",
        )
        .bind(&input.customer_scope_id)
        .bind(&record.compatibility_key)
        .bind(&input.reference_version)
        .bind(&input.effective_date)
        .bind(&input.source_authority)
        .bind(&record.diagnosis_code_prefix)
        .bind(&record.procedure_code)
        .bind(record.diagnosis_procedure_match_score)
        .bind(&record.data_source)
        .bind(&record.policy_authority_ref)
        .bind(&record.rationale)
        .bind(&evidence_refs)
        .bind(&input.source_report_uri)
        .bind(&input.submitted_by)
        .bind(&input.notes)
        .execute(&mut *tx)
        .await?;
        saved.push(ClinicalCompatibilityReferenceRecord {
            customer_scope_id: input.customer_scope_id.clone(),
            compatibility_key: record.compatibility_key,
            diagnosis_code_prefix: record.diagnosis_code_prefix,
            procedure_code: record.procedure_code,
            diagnosis_procedure_match_score: record.diagnosis_procedure_match_score,
            data_source: record.data_source,
            policy_authority_ref: record.policy_authority_ref,
            rationale: record.rationale,
            evidence_refs: record.evidence_refs,
            reference_version: input.reference_version.clone(),
            effective_date: input.effective_date.clone(),
            source_authority: input.source_authority.clone(),
            source_report_uri: input.source_report_uri.clone(),
            submitted_by: input.submitted_by.clone(),
            notes: input.notes.clone(),
        });
    }
    tx.commit().await?;
    Ok(saved)
}

pub(super) async fn save_unbundling_comparator_candidates(
    repository: &PostgresScoringRepository,
    input: SaveUnbundlingComparatorCandidatesInput,
) -> anyhow::Result<Vec<UnbundlingComparatorCandidateRecord>> {
    let mut saved = Vec::with_capacity(input.candidates.len());
    let mut tx = repository.pool.begin().await?;
    for candidate in input.candidates {
        sqlx::query(
            "INSERT INTO unbundling_comparator_candidates
                 (customer_scope_id, candidate_id, as_of_date, rule_id, episode_key, member_id, provider_id, window_days, bundled_code, matched_component_codes, claim_ids, policy_authority_ref, evidence_refs, recommended_review, source_report_uri, submitted_by, notes)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
                 ON CONFLICT (customer_scope_id, candidate_id, as_of_date) DO UPDATE
                 SET rule_id = EXCLUDED.rule_id,
                     episode_key = EXCLUDED.episode_key,
                     member_id = EXCLUDED.member_id,
                     provider_id = EXCLUDED.provider_id,
                     window_days = EXCLUDED.window_days,
                     bundled_code = EXCLUDED.bundled_code,
                     matched_component_codes = EXCLUDED.matched_component_codes,
                     claim_ids = EXCLUDED.claim_ids,
                     policy_authority_ref = EXCLUDED.policy_authority_ref,
                     evidence_refs = EXCLUDED.evidence_refs,
                     recommended_review = EXCLUDED.recommended_review,
                     source_report_uri = EXCLUDED.source_report_uri,
                     submitted_by = EXCLUDED.submitted_by,
                     notes = EXCLUDED.notes,
                     updated_at = now()",
        )
        .bind(&input.customer_scope_id)
        .bind(&candidate.candidate_id)
        .bind(&input.as_of_date)
        .bind(&candidate.rule_id)
        .bind(&candidate.episode_key)
        .bind(&candidate.member_id)
        .bind(&candidate.provider_id)
        .bind(candidate.window_days as i32)
        .bind(&candidate.bundled_code)
        .bind(string_values(&candidate.matched_component_codes))
        .bind(string_values(&candidate.claim_ids))
        .bind(&candidate.policy_authority_ref)
        .bind(string_values(&candidate.evidence_refs))
        .bind(&candidate.recommended_review)
        .bind(&input.source_report_uri)
        .bind(&input.submitted_by)
        .bind(&input.notes)
        .execute(&mut *tx)
        .await?;
        saved.push(UnbundlingComparatorCandidateRecord {
            customer_scope_id: input.customer_scope_id.clone(),
            candidate_id: candidate.candidate_id,
            as_of_date: input.as_of_date.clone(),
            rule_id: candidate.rule_id,
            episode_key: candidate.episode_key,
            member_id: candidate.member_id,
            provider_id: candidate.provider_id,
            window_days: candidate.window_days,
            bundled_code: candidate.bundled_code,
            matched_component_codes: candidate.matched_component_codes,
            claim_ids: candidate.claim_ids,
            policy_authority_ref: candidate.policy_authority_ref,
            evidence_refs: candidate.evidence_refs,
            recommended_review: candidate.recommended_review,
            source_report_uri: input.source_report_uri.clone(),
            submitted_by: input.submitted_by.clone(),
            notes: input.notes.clone(),
        });
    }
    tx.commit().await?;
    Ok(saved)
}

pub(super) async fn save_worker_data_pipeline_readiness_report(
    repository: &PostgresScoringRepository,
    input: SaveWorkerDataPipelineReadinessReportInput,
) -> anyhow::Result<WorkerDataPipelineReadinessReportRecord> {
    sqlx::query(
        "INSERT INTO worker_data_pipeline_readiness_reports
             (customer_scope_id, source_report_uri, report_kind, plan_uri, readiness_input_uri,
              readiness_status, job_count, ready_job_count, blocked_job_count, review_task_count,
              job_readiness_json, review_tasks_json, evidence_refs, governance_boundary,
              submitted_by, notes)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
             ON CONFLICT (customer_scope_id, source_report_uri) DO UPDATE
             SET report_kind = EXCLUDED.report_kind,
                 plan_uri = EXCLUDED.plan_uri,
                 readiness_input_uri = EXCLUDED.readiness_input_uri,
                 readiness_status = EXCLUDED.readiness_status,
                 job_count = EXCLUDED.job_count,
                 ready_job_count = EXCLUDED.ready_job_count,
                 blocked_job_count = EXCLUDED.blocked_job_count,
                 review_task_count = EXCLUDED.review_task_count,
                 job_readiness_json = EXCLUDED.job_readiness_json,
                 review_tasks_json = EXCLUDED.review_tasks_json,
                 evidence_refs = EXCLUDED.evidence_refs,
                 governance_boundary = EXCLUDED.governance_boundary,
                 submitted_by = EXCLUDED.submitted_by,
                 notes = EXCLUDED.notes,
                 updated_at = now()",
    )
    .bind(&input.customer_scope_id)
    .bind(&input.source_report_uri)
    .bind(&input.report_kind)
    .bind(&input.plan_uri)
    .bind(&input.readiness_input_uri)
    .bind(&input.readiness_status)
    .bind(input.job_count as i64)
    .bind(input.ready_job_count as i64)
    .bind(input.blocked_job_count as i64)
    .bind(input.review_task_count as i64)
    .bind(&input.job_readiness_json)
    .bind(&input.review_tasks_json)
    .bind(serde_json::json!(input.evidence_refs.clone()))
    .bind(&input.governance_boundary)
    .bind(&input.submitted_by)
    .bind(&input.notes)
    .execute(&repository.pool)
    .await?;
    Ok(WorkerDataPipelineReadinessReportRecord {
        customer_scope_id: input.customer_scope_id,
        source_report_uri: input.source_report_uri,
        report_kind: input.report_kind,
        plan_uri: input.plan_uri,
        readiness_input_uri: input.readiness_input_uri,
        readiness_status: input.readiness_status,
        job_count: input.job_count,
        ready_job_count: input.ready_job_count,
        blocked_job_count: input.blocked_job_count,
        review_task_count: input.review_task_count,
        job_readiness_json: input.job_readiness_json,
        review_tasks_json: input.review_tasks_json,
        evidence_refs: input.evidence_refs,
        governance_boundary: input.governance_boundary,
        submitted_by: input.submitted_by,
        notes: input.notes,
    })
}

pub(super) async fn save_worker_data_pipeline_execution_report(
    repository: &PostgresScoringRepository,
    input: SaveWorkerDataPipelineExecutionReportInput,
) -> anyhow::Result<WorkerDataPipelineExecutionReportRecord> {
    sqlx::query(
        "INSERT INTO worker_data_pipeline_execution_reports
             (customer_scope_id, source_report_uri, report_kind, plan_uri, run_status_uri,
              readiness_report_uri, readiness_gate_status, run_id, execution_date, job_count,
              pending_or_failed_job_count, review_task_count, job_executions_json,
              review_tasks_json, evidence_refs, governance_boundary, submitted_by, notes)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)
             ON CONFLICT (customer_scope_id, source_report_uri) DO UPDATE
             SET report_kind = EXCLUDED.report_kind,
                 plan_uri = EXCLUDED.plan_uri,
                 run_status_uri = EXCLUDED.run_status_uri,
                 readiness_report_uri = EXCLUDED.readiness_report_uri,
                 readiness_gate_status = EXCLUDED.readiness_gate_status,
                 run_id = EXCLUDED.run_id,
                 execution_date = EXCLUDED.execution_date,
                 job_count = EXCLUDED.job_count,
                 pending_or_failed_job_count = EXCLUDED.pending_or_failed_job_count,
                 review_task_count = EXCLUDED.review_task_count,
                 job_executions_json = EXCLUDED.job_executions_json,
                 review_tasks_json = EXCLUDED.review_tasks_json,
                 evidence_refs = EXCLUDED.evidence_refs,
                 governance_boundary = EXCLUDED.governance_boundary,
                 submitted_by = EXCLUDED.submitted_by,
                 notes = EXCLUDED.notes,
                 updated_at = now()",
    )
    .bind(&input.customer_scope_id)
    .bind(&input.source_report_uri)
    .bind(&input.report_kind)
    .bind(&input.plan_uri)
    .bind(&input.run_status_uri)
    .bind(&input.readiness_report_uri)
    .bind(&input.readiness_gate_status)
    .bind(&input.run_id)
    .bind(&input.execution_date)
    .bind(input.job_count as i64)
    .bind(input.pending_or_failed_job_count as i64)
    .bind(input.review_task_count as i64)
    .bind(&input.job_executions_json)
    .bind(&input.review_tasks_json)
    .bind(serde_json::json!(input.evidence_refs.clone()))
    .bind(&input.governance_boundary)
    .bind(&input.submitted_by)
    .bind(&input.notes)
    .execute(&repository.pool)
    .await?;
    Ok(WorkerDataPipelineExecutionReportRecord {
        customer_scope_id: input.customer_scope_id,
        source_report_uri: input.source_report_uri,
        report_kind: input.report_kind,
        plan_uri: input.plan_uri,
        run_status_uri: input.run_status_uri,
        readiness_report_uri: input.readiness_report_uri,
        readiness_gate_status: input.readiness_gate_status,
        run_id: input.run_id,
        execution_date: input.execution_date,
        job_count: input.job_count,
        pending_or_failed_job_count: input.pending_or_failed_job_count,
        review_task_count: input.review_task_count,
        job_executions_json: input.job_executions_json,
        review_tasks_json: input.review_tasks_json,
        evidence_refs: input.evidence_refs,
        governance_boundary: input.governance_boundary,
        submitted_by: input.submitted_by,
        notes: input.notes,
    })
}

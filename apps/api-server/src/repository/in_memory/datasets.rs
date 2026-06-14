use super::*;

impl InMemoryScoringRepository {
    pub(super) async fn in_memory_register_dataset(
        &self,
        input: RegisterDatasetInput,
    ) -> anyhow::Result<DatasetRecord> {
        let mut sequence = self.dataset_sequence.lock().await;
        *sequence += 1;
        let dataset_id = format!("dataset_{}", *sequence);
        let record = DatasetRecord {
            dataset_id: dataset_id.clone(),
            source_key: input.source_key,
            display_name: input.display_name,
            business_domain: input.business_domain,
            dataset_key: input.dataset_key,
            dataset_version: input.dataset_version,
            sample_grain: input.sample_grain,
            label_column: input.label_column,
            entity_keys: input.entity_keys,
            manifest_uri: input.manifest_uri,
            schema_uri: input.schema_uri,
            profile_uri: input.profile_uri,
            storage_format: input.storage_format,
            schema_hash: input.schema_hash,
            row_count: input.row_count,
            status: input.status,
            splits: input.splits,
            fields: input.fields,
            mappings: vec![],
        };
        self.datasets
            .lock()
            .await
            .insert(dataset_id, record.clone());
        Ok(record)
    }

    pub(super) async fn in_memory_list_datasets(&self) -> anyhow::Result<Vec<DatasetRecord>> {
        let mut datasets = self
            .datasets
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        datasets.sort_by(|left, right| left.dataset_key.cmp(&right.dataset_key));
        Ok(datasets)
    }

    pub(super) async fn in_memory_get_dataset(
        &self,
        dataset_id: &str,
    ) -> anyhow::Result<Option<DatasetRecord>> {
        Ok(self.datasets.lock().await.get(dataset_id).cloned())
    }

    pub(super) async fn in_memory_add_field_mapping(
        &self,
        dataset_id: &str,
        input: CreateFieldMappingInput,
    ) -> anyhow::Result<Option<FieldMappingRecord>> {
        let mut datasets = self.datasets.lock().await;
        let Some(dataset) = datasets.get_mut(dataset_id) else {
            return Ok(None);
        };
        let mut sequence = self.mapping_sequence.lock().await;
        *sequence += 1;
        let mapping = FieldMappingRecord {
            mapping_id: format!("mapping_{}", *sequence),
            dataset_id: dataset_id.to_string(),
            external_field: input.external_field,
            canonical_target: input.canonical_target,
            feature_name: input.feature_name,
            transform_kind: input.transform_kind,
            transform_json: input.transform_json,
            status: input.status,
        };
        dataset.mappings.push(mapping.clone());
        Ok(Some(mapping))
    }

    pub(super) async fn in_memory_register_feature_set(
        &self,
        input: RegisterFeatureSetInput,
    ) -> anyhow::Result<Option<FeatureSetRecord>> {
        if self
            .in_memory_get_dataset(&input.dataset_id)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        let mut sequence = self.feature_set_sequence.lock().await;
        *sequence += 1;
        let feature_set_id = format!("feature_set_{}", *sequence);
        let record = FeatureSetRecord {
            feature_set_id: feature_set_id.clone(),
            business_domain: input.business_domain,
            feature_set_key: input.feature_set_key,
            version: input.version,
            dataset_id: input.dataset_id,
            features_uri: input.features_uri,
            feature_list_json: input.feature_list_json,
            row_count: input.row_count,
            label_column: input.label_column,
            status: input.status,
        };
        self.feature_sets
            .lock()
            .await
            .insert(feature_set_id, record.clone());
        Ok(Some(record))
    }

    pub(super) async fn in_memory_register_model_dataset(
        &self,
        input: RegisterModelDatasetInput,
    ) -> anyhow::Result<Option<ModelDatasetRecord>> {
        if !self
            .feature_sets
            .lock()
            .await
            .contains_key(&input.feature_set_id)
        {
            return Ok(None);
        }
        let mut sequence = self.model_dataset_sequence.lock().await;
        *sequence += 1;
        let model_dataset_id = format!("model_dataset_{}", *sequence);
        let record = ModelDatasetRecord {
            model_dataset_id: model_dataset_id.clone(),
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
        };
        self.model_datasets
            .lock()
            .await
            .insert(model_dataset_id, record.clone());
        Ok(Some(record))
    }

    pub(super) async fn in_memory_get_model_dataset_source_dataset(
        &self,
        model_dataset_id: &str,
    ) -> anyhow::Result<Option<DatasetRecord>> {
        let model_dataset = self
            .model_datasets
            .lock()
            .await
            .get(model_dataset_id)
            .cloned();
        let Some(model_dataset) = model_dataset else {
            return Ok(None);
        };
        let feature_set = self
            .feature_sets
            .lock()
            .await
            .get(&model_dataset.feature_set_id)
            .cloned();
        let Some(feature_set) = feature_set else {
            return Ok(None);
        };
        self.in_memory_get_dataset(&feature_set.dataset_id).await
    }

    pub(super) async fn in_memory_register_model_evaluation(
        &self,
        input: RegisterModelEvaluationInput,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>> {
        if !self
            .model_datasets
            .lock()
            .await
            .contains_key(&input.model_dataset_id)
        {
            return Ok(None);
        }
        let record = ModelEvaluationRecord {
            evaluation_run_id: input.evaluation_run_id,
            model_key: input.model_key,
            model_version: input.model_version,
            model_dataset_id: input.model_dataset_id,
            scheme_family: input.scheme_family,
            auc: input.auc,
            ks: input.ks,
            precision: input.precision,
            recall: input.recall,
            f1: input.f1,
            accuracy: input.accuracy,
            threshold: input.threshold,
            confusion_matrix_json: input.confusion_matrix_json,
            feature_importance_uri: input.feature_importance_uri,
            permutation_importance_uri: input.permutation_importance_uri,
            metrics_json: input.metrics_json,
        };
        self.model_evaluations
            .lock()
            .await
            .insert(record.evaluation_run_id.clone(), record.clone());
        Ok(Some(record))
    }

    pub(super) async fn in_memory_get_model_evaluation(
        &self,
        evaluation_run_id: &str,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>> {
        Ok(self
            .model_evaluations
            .lock()
            .await
            .get(evaluation_run_id)
            .cloned())
    }

    pub(super) async fn in_memory_list_model_evaluations(
        &self,
    ) -> anyhow::Result<Vec<ModelEvaluationRecord>> {
        let mut evaluations = self
            .model_evaluations
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        evaluations.sort_by(|left, right| left.evaluation_run_id.cmp(&right.evaluation_run_id));
        Ok(evaluations)
    }

    pub(super) async fn in_memory_save_scoring_feature_context_materialization(
        &self,
        input: SaveScoringFeatureContextMaterializationInput,
    ) -> anyhow::Result<ScoringFeatureContextMaterializationRecord> {
        let record = ScoringFeatureContextMaterializationRecord {
            materialization_id: input.materialization_id,
            customer_scope_id: input.customer_scope_id,
            as_of_date: input.as_of_date,
            report_uri: input.report_uri,
            report_kind: input.report_kind,
            source_uris: input.source_uris,
            claim_count: input.claim_count,
            context_count: input.context_count,
            contexts_json: input.contexts_json,
            evidence_refs: input.evidence_refs,
            governance_boundary: input.governance_boundary,
            submitted_by: input.submitted_by,
            notes: input.notes,
        };
        self.scoring_feature_context_materializations
            .lock()
            .await
            .insert(
                scoring_context_materialization_key(
                    &record.customer_scope_id,
                    &record.materialization_id,
                ),
                record.clone(),
            );
        Ok(record)
    }

    pub(super) async fn in_memory_get_scoring_feature_context_materialization(
        &self,
        materialization_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<ScoringFeatureContextMaterializationRecord>> {
        let record = self
            .scoring_feature_context_materializations
            .lock()
            .await
            .values()
            .find(|record| record.materialization_id == materialization_id)
            .cloned()
            .filter(|record| {
                customer_scope_id
                    .map(|scope| record.customer_scope_id == scope)
                    .unwrap_or(true)
            });
        Ok(record)
    }

    pub(super) async fn in_memory_save_clinical_compatibility_references(
        &self,
        input: SaveClinicalCompatibilityReferencesInput,
    ) -> anyhow::Result<Vec<ClinicalCompatibilityReferenceRecord>> {
        let mut records = self.clinical_compatibility_references.lock().await;
        let mut saved = Vec::with_capacity(input.records.len());
        for upsert in input.records {
            let record = ClinicalCompatibilityReferenceRecord {
                customer_scope_id: input.customer_scope_id.clone(),
                compatibility_key: upsert.compatibility_key,
                diagnosis_code_prefix: upsert.diagnosis_code_prefix,
                procedure_code: upsert.procedure_code,
                diagnosis_procedure_match_score: upsert.diagnosis_procedure_match_score,
                data_source: upsert.data_source,
                policy_authority_ref: upsert.policy_authority_ref,
                rationale: upsert.rationale,
                evidence_refs: upsert.evidence_refs,
                reference_version: input.reference_version.clone(),
                effective_date: input.effective_date.clone(),
                source_authority: input.source_authority.clone(),
                source_report_uri: input.source_report_uri.clone(),
                submitted_by: input.submitted_by.clone(),
                notes: input.notes.clone(),
            };
            records.insert(
                clinical_compatibility_key(
                    &record.customer_scope_id,
                    &record.compatibility_key,
                    &record.reference_version,
                ),
                record.clone(),
            );
            saved.push(record);
        }
        Ok(saved)
    }
}

fn scoring_context_materialization_key(
    customer_scope_id: &str,
    materialization_id: &str,
) -> String {
    format!("{customer_scope_id}::{materialization_id}")
}

fn clinical_compatibility_key(
    customer_scope_id: &str,
    compatibility_key: &str,
    reference_version: &str,
) -> String {
    format!("{customer_scope_id}::{compatibility_key}::{reference_version}")
}

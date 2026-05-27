use async_trait::async_trait;
use fwa_core::ClaimContext;
use fwa_core::RecommendedAction;
use fwa_rules::{Condition, Rule, RuleAction};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct PersistedScoringRun {
    pub run_id: String,
    pub audit_id: String,
    pub claim_id: String,
    pub source_system: String,
    pub actor_id: String,
    pub risk_score: u8,
    pub rag: String,
    pub recommended_action: String,
    pub feature_values: Vec<Value>,
    pub rule_runs: Vec<Value>,
    pub model_score: Value,
    pub audit_event: Value,
    pub evidence_refs: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct PersistedAuditEvent {
    pub audit_id: String,
    pub run_id: String,
    pub claim_id: String,
    pub source_system: String,
    pub actor_id: String,
    pub actor_role: String,
    pub event_type: String,
    pub event_status: String,
    pub summary: String,
    pub payload: Value,
    pub evidence_refs: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSummaryRecord {
    pub rule_id: String,
    pub name: String,
    pub status: String,
    pub owner: String,
    pub active_version: Option<u32>,
    pub latest_version: u32,
    pub score: u8,
    pub alert_code: String,
    pub recommended_action: RecommendedAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleVersionRecord {
    pub version: u32,
    pub status: String,
    pub dsl: Value,
    pub score: u8,
    pub alert_code: String,
    pub recommended_action: RecommendedAction,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDetailRecord {
    pub summary: RuleSummaryRecord,
    pub versions: Vec<RuleVersionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVersionRecord {
    pub model_key: String,
    pub version: String,
    pub model_type: String,
    pub runtime_kind: String,
    pub execution_provider: String,
    pub status: String,
    pub artifact_uri: Option<String>,
    pub endpoint_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPerformanceRecord {
    pub model_key: String,
    pub data_status: String,
    pub scored_runs: u32,
    pub average_score: f64,
    pub high_risk_count: u32,
    pub latest_scored_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeCaseRecord {
    pub case_id: String,
    pub title: String,
    pub fwa_type: String,
    pub diagnosis_code: String,
    pub provider_region: String,
    pub provider_type: String,
    pub summary: String,
    pub outcome: String,
    pub tags: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarCaseQuery {
    pub claim_id: Option<String>,
    pub diagnosis_code: String,
    pub provider_region: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarCaseRecord {
    pub case_id: String,
    pub title: String,
    pub similarity_score: f64,
    pub matched_signals: Vec<String>,
    pub summary: String,
    pub outcome: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PersistedAgentRun {
    pub agent_run_id: String,
    pub claim_id: String,
    pub status: String,
    pub decision_boundary: String,
    pub output_json: Value,
    pub evidence_refs: Vec<Value>,
    pub steps: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetSplitRecord {
    pub split_name: String,
    pub data_uri: String,
    pub row_count: u64,
    pub positive_count: Option<u64>,
    pub negative_count: Option<u64>,
    pub label_distribution_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaFieldRecord {
    pub field_name: String,
    pub logical_type: String,
    pub nullable: bool,
    pub semantic_role: String,
    pub description: String,
    pub profile_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRecord {
    pub dataset_id: String,
    pub source_key: String,
    pub display_name: String,
    pub business_domain: String,
    pub dataset_key: String,
    pub dataset_version: String,
    pub sample_grain: String,
    pub label_column: String,
    pub entity_keys: Vec<String>,
    pub manifest_uri: String,
    pub schema_uri: String,
    pub profile_uri: String,
    pub storage_format: String,
    pub schema_hash: String,
    pub row_count: u64,
    pub status: String,
    pub splits: Vec<DatasetSplitRecord>,
    pub fields: Vec<SchemaFieldRecord>,
    pub mappings: Vec<FieldMappingRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterDatasetInput {
    pub source_key: String,
    pub display_name: String,
    pub business_domain: String,
    pub owner: String,
    pub description: String,
    pub dataset_key: String,
    pub dataset_version: String,
    pub sample_grain: String,
    pub label_column: String,
    pub entity_keys: Vec<String>,
    pub manifest_uri: String,
    pub schema_uri: String,
    pub profile_uri: String,
    pub storage_format: String,
    pub schema_hash: String,
    pub row_count: u64,
    pub status: String,
    pub splits: Vec<DatasetSplitRecord>,
    pub fields: Vec<SchemaFieldRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMappingRecord {
    pub mapping_id: String,
    pub dataset_id: String,
    pub external_field: String,
    pub canonical_target: String,
    pub feature_name: Option<String>,
    pub transform_kind: String,
    pub transform_json: Value,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFieldMappingInput {
    pub external_field: String,
    pub canonical_target: String,
    pub feature_name: Option<String>,
    pub transform_kind: String,
    pub transform_json: Value,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestigationResultRecord {
    pub investigation_id: String,
    pub claim_id: String,
    pub outcome: String,
    pub confirmed_fwa: bool,
    pub saving_amount: Option<Decimal>,
    pub currency: Option<String>,
    pub notes: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaReviewRecord {
    pub qa_case_id: String,
    pub claim_id: String,
    pub qa_conclusion: String,
    pub issue_type: String,
    pub feedback_target: String,
    pub notes: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditHistoryEventRecord {
    pub audit_id: String,
    pub run_id: String,
    pub event_type: String,
    pub event_status: String,
    pub summary: String,
    pub payload: Value,
    pub evidence_refs: Vec<String>,
    pub created_at: Option<String>,
}

#[async_trait]
pub trait ScoringRepository: Send + Sync {
    async fn upsert_claim_context(
        &self,
        context: ClaimContext,
        raw_payload: Value,
    ) -> anyhow::Result<()>;

    async fn load_claim_context(
        &self,
        external_claim_id: &str,
    ) -> anyhow::Result<Option<ClaimContext>>;

    async fn save_scoring_run(&self, run: PersistedScoringRun) -> anyhow::Result<()>;

    async fn save_audit_event(&self, event: PersistedAuditEvent) -> anyhow::Result<()>;

    async fn list_rules(&self) -> anyhow::Result<Vec<RuleSummaryRecord>>;

    async fn get_rule(&self, rule_id: &str) -> anyhow::Result<Option<RuleDetailRecord>>;

    async fn update_rule_status(
        &self,
        rule_id: &str,
        status: &str,
    ) -> anyhow::Result<Option<RuleSummaryRecord>>;

    async fn list_models(&self) -> anyhow::Result<Vec<ModelVersionRecord>>;

    async fn model_performance(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Option<ModelPerformanceRecord>>;

    async fn list_knowledge_cases(&self) -> anyhow::Result<Vec<KnowledgeCaseRecord>>;

    async fn search_similar_cases(
        &self,
        query: SimilarCaseQuery,
    ) -> anyhow::Result<Vec<SimilarCaseRecord>>;

    async fn save_agent_run(&self, run: PersistedAgentRun) -> anyhow::Result<()>;

    async fn register_dataset(&self, input: RegisterDatasetInput) -> anyhow::Result<DatasetRecord>;

    async fn list_datasets(&self) -> anyhow::Result<Vec<DatasetRecord>>;

    async fn get_dataset(&self, dataset_id: &str) -> anyhow::Result<Option<DatasetRecord>>;

    async fn add_field_mapping(
        &self,
        dataset_id: &str,
        input: CreateFieldMappingInput,
    ) -> anyhow::Result<Option<FieldMappingRecord>>;

    async fn save_investigation_result(
        &self,
        record: InvestigationResultRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord>;

    async fn save_qa_review(
        &self,
        record: QaReviewRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord>;

    async fn claim_audit_history(
        &self,
        claim_id: &str,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>>;
}

pub type SharedRepository = Arc<dyn ScoringRepository>;

#[derive(Debug, Default)]
pub struct InMemoryScoringRepository {
    claims: Mutex<HashMap<String, ClaimContext>>,
    runs: Mutex<Vec<PersistedScoringRun>>,
    audit_events: Mutex<Vec<PersistedAuditEvent>>,
    agent_runs: Mutex<Vec<PersistedAgentRun>>,
    rule_statuses: Mutex<HashMap<String, String>>,
    datasets: Mutex<HashMap<String, DatasetRecord>>,
    dataset_sequence: Mutex<u64>,
    mapping_sequence: Mutex<u64>,
    pilot_audit_events: Mutex<Vec<(String, AuditHistoryEventRecord)>>,
}

impl InMemoryScoringRepository {
    pub fn shared() -> SharedRepository {
        Arc::new(Self::default())
    }
}

#[async_trait]
impl ScoringRepository for InMemoryScoringRepository {
    async fn upsert_claim_context(
        &self,
        context: ClaimContext,
        _raw_payload: Value,
    ) -> anyhow::Result<()> {
        self.claims
            .lock()
            .await
            .insert(context.claim.external_claim_id.clone(), context);
        Ok(())
    }

    async fn load_claim_context(
        &self,
        external_claim_id: &str,
    ) -> anyhow::Result<Option<ClaimContext>> {
        Ok(self.claims.lock().await.get(external_claim_id).cloned())
    }

    async fn save_scoring_run(&self, run: PersistedScoringRun) -> anyhow::Result<()> {
        self.runs.lock().await.push(run);
        Ok(())
    }

    async fn save_audit_event(&self, event: PersistedAuditEvent) -> anyhow::Result<()> {
        self.audit_events.lock().await.push(event);
        Ok(())
    }

    async fn list_rules(&self) -> anyhow::Result<Vec<RuleSummaryRecord>> {
        let statuses = self.rule_statuses.lock().await;
        Ok(default_rule_details()
            .into_iter()
            .map(|mut detail| {
                if let Some(status) = statuses.get(&detail.summary.rule_id) {
                    detail.summary.status = status.clone();
                    for version in &mut detail.versions {
                        version.status = status.clone();
                    }
                }
                detail.summary
            })
            .collect())
    }

    async fn get_rule(&self, rule_id: &str) -> anyhow::Result<Option<RuleDetailRecord>> {
        let statuses = self.rule_statuses.lock().await;
        Ok(default_rule_details()
            .into_iter()
            .find(|detail| detail.summary.rule_id == rule_id)
            .map(|mut detail| {
                if let Some(status) = statuses.get(rule_id) {
                    detail.summary.status = status.clone();
                    for version in &mut detail.versions {
                        version.status = status.clone();
                    }
                }
                detail
            }))
    }

    async fn update_rule_status(
        &self,
        rule_id: &str,
        status: &str,
    ) -> anyhow::Result<Option<RuleSummaryRecord>> {
        if self.get_rule(rule_id).await?.is_none() {
            return Ok(None);
        }
        self.rule_statuses
            .lock()
            .await
            .insert(rule_id.to_string(), status.to_string());
        Ok(self.get_rule(rule_id).await?.map(|detail| detail.summary))
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelVersionRecord>> {
        Ok(default_model_versions())
    }

    async fn model_performance(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Option<ModelPerformanceRecord>> {
        if default_model_versions()
            .iter()
            .any(|model| model.model_key == model_key)
        {
            Ok(Some(empty_model_performance(model_key)))
        } else {
            Ok(None)
        }
    }

    async fn list_knowledge_cases(&self) -> anyhow::Result<Vec<KnowledgeCaseRecord>> {
        Ok(default_knowledge_cases())
    }

    async fn search_similar_cases(
        &self,
        query: SimilarCaseQuery,
    ) -> anyhow::Result<Vec<SimilarCaseRecord>> {
        Ok(search_cases(default_knowledge_cases(), &query))
    }

    async fn save_agent_run(&self, run: PersistedAgentRun) -> anyhow::Result<()> {
        self.agent_runs.lock().await.push(run);
        Ok(())
    }

    async fn register_dataset(&self, input: RegisterDatasetInput) -> anyhow::Result<DatasetRecord> {
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

    async fn list_datasets(&self) -> anyhow::Result<Vec<DatasetRecord>> {
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

    async fn get_dataset(&self, dataset_id: &str) -> anyhow::Result<Option<DatasetRecord>> {
        Ok(self.datasets.lock().await.get(dataset_id).cloned())
    }

    async fn add_field_mapping(
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

    async fn save_investigation_result(
        &self,
        record: InvestigationResultRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        let event = AuditHistoryEventRecord {
            audit_id: format!("audit_investigation_{}", record.investigation_id),
            run_id: format!("pilot_investigation_{}", record.investigation_id),
            event_type: "investigation.result.received".into(),
            event_status: "succeeded".into(),
            summary: format!("Investigation result received: {}", record.outcome),
            payload: serde_json::to_value(&record)?,
            evidence_refs: record.evidence_refs.clone(),
            created_at: None,
        };
        self.pilot_audit_events
            .lock()
            .await
            .push((record.claim_id, event.clone()));
        Ok(event)
    }

    async fn save_qa_review(
        &self,
        record: QaReviewRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        let event = AuditHistoryEventRecord {
            audit_id: format!("audit_qa_{}", record.qa_case_id),
            run_id: format!("pilot_qa_{}", record.qa_case_id),
            event_type: "qa.result.received".into(),
            event_status: "succeeded".into(),
            summary: format!("QA result received: {}", record.qa_conclusion),
            payload: serde_json::to_value(&record)?,
            evidence_refs: record.evidence_refs.clone(),
            created_at: None,
        };
        self.pilot_audit_events
            .lock()
            .await
            .push((record.claim_id, event.clone()));
        Ok(event)
    }

    async fn claim_audit_history(
        &self,
        claim_id: &str,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        Ok(self
            .pilot_audit_events
            .lock()
            .await
            .iter()
            .filter(|(event_claim_id, _)| event_claim_id == claim_id)
            .map(|(_, event)| event.clone())
            .collect())
    }
}

#[derive(Debug, Clone)]
pub struct PostgresScoringRepository {
    pool: PgPool,
}

impl PostgresScoringRepository {
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl ScoringRepository for PostgresScoringRepository {
    async fn upsert_claim_context(
        &self,
        context: ClaimContext,
        raw_payload: Value,
    ) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;

        let member_row: (String,) = sqlx::query_as(
            "INSERT INTO members (external_member_id)
             VALUES ($1)
             ON CONFLICT (external_member_id) DO UPDATE SET updated_at = now()
             RETURNING id::text",
        )
        .bind(&context.member.external_member_id)
        .fetch_one(&mut *tx)
        .await?;

        let policy_row: (String,) = sqlx::query_as(
            "INSERT INTO policies
             (external_policy_id, member_id, product_code, coverage_start_date, coverage_end_date, coverage_limit_amount, currency)
             VALUES ($1, $2::uuid, $3, $4, $5, $6, $7)
             ON CONFLICT (external_policy_id) DO UPDATE SET updated_at = now()
             RETURNING id::text",
        )
        .bind(&context.policy.external_policy_id)
        .bind(&member_row.0)
        .bind(&context.policy.product_code)
        .bind(context.policy.coverage_start_date)
        .bind(context.policy.coverage_end_date)
        .bind(context.policy.coverage_limit.amount)
        .bind(&context.policy.coverage_limit.currency)
        .fetch_one(&mut *tx)
        .await?;

        let provider_row: (String,) = sqlx::query_as(
            "INSERT INTO providers (external_provider_id, name, provider_type, region, risk_tier)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (external_provider_id) DO UPDATE SET updated_at = now()
             RETURNING id::text",
        )
        .bind(&context.provider.external_provider_id)
        .bind(&context.provider.name)
        .bind(&context.provider.provider_type)
        .bind(&context.provider.region)
        .bind(format!("{:?}", context.provider.risk_tier))
        .fetch_one(&mut *tx)
        .await?;

        let claim_row: (String,) = sqlx::query_as(
            "INSERT INTO claims
             (external_claim_id, member_id, policy_id, provider_id, claim_type, diagnosis_code, service_date, claim_amount, currency, status, raw_payload)
             VALUES ($1, $2::uuid, $3::uuid, $4::uuid, 'medical', $5, $6, $7, $8, 'submitted', $9)
             ON CONFLICT (external_claim_id) DO UPDATE
             SET updated_at = now(), raw_payload = EXCLUDED.raw_payload, claim_amount = EXCLUDED.claim_amount
             RETURNING id::text",
        )
        .bind(&context.claim.external_claim_id)
        .bind(&member_row.0)
        .bind(&policy_row.0)
        .bind(&provider_row.0)
        .bind(&context.claim.diagnosis_code)
        .bind(context.claim.service_date)
        .bind(context.claim.amount.amount)
        .bind(&context.claim.amount.currency)
        .bind(raw_payload)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM claim_items WHERE claim_id = $1::uuid")
            .bind(&claim_row.0)
            .execute(&mut *tx)
            .await?;

        for item in &context.items {
            sqlx::query(
                "INSERT INTO claim_items
                 (claim_id, item_code, item_type, description, quantity, unit_amount, total_amount, currency)
                 VALUES ($1::uuid, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(&claim_row.0)
            .bind(&item.item_code)
            .bind(&item.item_type)
            .bind(&item.description)
            .bind(item.quantity as i32)
            .bind(item.unit_amount.amount)
            .bind(item.total_amount.amount)
            .bind(&item.total_amount.currency)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn load_claim_context(
        &self,
        external_claim_id: &str,
    ) -> anyhow::Result<Option<ClaimContext>> {
        let raw_payload: Option<(Value,)> =
            sqlx::query_as("SELECT raw_payload FROM claims WHERE external_claim_id = $1")
                .bind(external_claim_id)
                .fetch_optional(&self.pool)
                .await?;

        raw_payload
            .map(|(value,)| serde_json::from_value(value).map_err(Into::into))
            .transpose()
    }

    async fn save_scoring_run(&self, run: PersistedScoringRun) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        let claim_row: Option<(String,)> =
            sqlx::query_as("SELECT id::text FROM claims WHERE external_claim_id = $1")
                .bind(&run.claim_id)
                .fetch_optional(&mut *tx)
                .await?;

        let claim_uuid = claim_row.map(|row| row.0);
        sqlx::query(
            "INSERT INTO scoring_runs
             (run_id, claim_id, source_system, actor_id, status, risk_score, rag, recommended_action, completed_at)
             VALUES ($1, $2::uuid, $3, $4, 'succeeded', $5, $6, $7, now())",
        )
        .bind(&run.run_id)
        .bind(claim_uuid.as_deref())
        .bind(&run.source_system)
        .bind(&run.actor_id)
        .bind(run.risk_score as i32)
        .bind(&run.rag)
        .bind(&run.recommended_action)
        .execute(&mut *tx)
        .await?;

        for feature in &run.feature_values {
            let feature_name = feature["name"].as_str().unwrap_or("unknown");
            let feature_version = feature["version"].as_i64().unwrap_or(1) as i32;
            sqlx::query(
                "INSERT INTO feature_values
                 (run_id, claim_id, feature_name, feature_version, value_json, evidence_json)
                 VALUES ($1, $2::uuid, $3, $4, $5, $6)",
            )
            .bind(&run.run_id)
            .bind(claim_uuid.as_deref())
            .bind(feature_name)
            .bind(feature_version)
            .bind(feature["value"].clone())
            .bind(feature["evidence_refs"].clone())
            .execute(&mut *tx)
            .await?;
        }

        for rule_run in &run.rule_runs {
            sqlx::query(
                "INSERT INTO rule_runs
                 (run_id, matched, score_contribution, alert_code, reason, evidence_json)
                 VALUES ($1, true, $2, $3, $4, '[]'::jsonb)",
            )
            .bind(&run.run_id)
            .bind(rule_run["score_contribution"].as_i64().unwrap_or(0) as i32)
            .bind(rule_run["alert_code"].as_str())
            .bind(rule_run["reason"].as_str())
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            "INSERT INTO model_scores
             (run_id, model_key, runtime_kind, execution_provider, score, label, explanation_json, latency_ms)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&run.run_id)
        .bind(run.model_score["model_key"].as_str().unwrap_or("unknown"))
        .bind(run.model_score["runtime_kind"].as_str().unwrap_or("unknown"))
        .bind(run.model_score["execution_provider"].as_str().unwrap_or("cpu"))
        .bind(run.model_score["score"].as_i64().unwrap_or(0) as i32)
        .bind(run.model_score["label"].as_str().unwrap_or("UNKNOWN"))
        .bind(run.model_score["explanations"].clone())
        .bind(run.model_score["latency_ms"].as_i64().unwrap_or(0) as i32)
        .execute(&mut *tx)
        .await?;

        insert_audit_event(
            &mut tx,
            &PersistedAuditEvent {
                audit_id: run.audit_id,
                run_id: run.run_id,
                claim_id: run.claim_id,
                source_system: run.source_system,
                actor_id: run.actor_id,
                actor_role: "tpa_system".into(),
                event_type: "scoring.completed".into(),
                event_status: "succeeded".into(),
                summary: "FWA scoring completed".into(),
                payload: run.audit_event,
                evidence_refs: run.evidence_refs,
            },
            claim_uuid.as_deref(),
        )
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn save_audit_event(&self, event: PersistedAuditEvent) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        let claim_row: Option<(String,)> =
            sqlx::query_as("SELECT id::text FROM claims WHERE external_claim_id = $1")
                .bind(&event.claim_id)
                .fetch_optional(&mut *tx)
                .await?;
        sqlx::query(
            "INSERT INTO scoring_runs
             (run_id, claim_id, source_system, actor_id, status, completed_at, error_code, error_message)
             VALUES ($1, $2::uuid, $3, $4, $5, now(), $6, $7)
             ON CONFLICT (run_id) DO NOTHING",
        )
        .bind(&event.run_id)
        .bind(claim_row.as_ref().map(|row| row.0.as_str()))
        .bind(&event.source_system)
        .bind(&event.actor_id)
        .bind(&event.event_status)
        .bind(&event.event_type)
        .bind(event.payload["error"].as_str())
        .execute(&mut *tx)
        .await?;
        insert_audit_event(
            &mut tx,
            &event,
            claim_row.as_ref().map(|row| row.0.as_str()),
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn list_rules(&self) -> anyhow::Result<Vec<RuleSummaryRecord>> {
        ensure_default_rules_seeded(&self.pool).await?;
        let rows: Vec<(String, String, String, String, i32, Value, i32, String)> = sqlx::query_as(
            "SELECT r.rule_key, r.name, r.status, r.owner, rv.version, rv.dsl, rv.score, rv.recommended_action
             FROM rules r
             JOIN LATERAL (
               SELECT version, dsl, score, recommended_action
               FROM rule_versions
               WHERE rule_id = r.id
               ORDER BY version DESC
               LIMIT 1
             ) rv ON true
             ORDER BY r.rule_key",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(rule_id, name, status, owner, version, dsl, score, recommended_action)| {
                    let action = dsl.get("action").cloned().unwrap_or(Value::Null);
                    RuleSummaryRecord {
                        rule_id,
                        name,
                        active_version: if status == "active" {
                            Some(version as u32)
                        } else {
                            None
                        },
                        latest_version: version as u32,
                        status,
                        owner,
                        score: score as u8,
                        alert_code: action["alert_code"]
                            .as_str()
                            .unwrap_or("UNKNOWN")
                            .to_string(),
                        recommended_action: parse_recommended_action(&recommended_action),
                    }
                },
            )
            .collect())
    }

    async fn get_rule(&self, rule_id: &str) -> anyhow::Result<Option<RuleDetailRecord>> {
        ensure_default_rules_seeded(&self.pool).await?;
        let summary = self
            .list_rules()
            .await?
            .into_iter()
            .find(|rule| rule.rule_id == rule_id);
        let Some(summary) = summary else {
            return Ok(None);
        };

        let rows: Vec<(i32, Value, i32, String)> = sqlx::query_as(
            "SELECT rv.version, rv.dsl, rv.score, rv.recommended_action
             FROM rule_versions rv
             JOIN rules r ON r.id = rv.rule_id
             WHERE r.rule_key = $1
             ORDER BY rv.version DESC",
        )
        .bind(rule_id)
        .fetch_all(&self.pool)
        .await?;

        let versions = rows
            .into_iter()
            .map(|(version, dsl, score, recommended_action)| {
                let action = dsl.get("action").cloned().unwrap_or(Value::Null);
                RuleVersionRecord {
                    version: version as u32,
                    status: summary.status.clone(),
                    dsl,
                    score: score as u8,
                    alert_code: action["alert_code"]
                        .as_str()
                        .unwrap_or("UNKNOWN")
                        .to_string(),
                    recommended_action: parse_recommended_action(&recommended_action),
                    reason: action["reason"].as_str().unwrap_or("").to_string(),
                }
            })
            .collect();

        Ok(Some(RuleDetailRecord { summary, versions }))
    }

    async fn update_rule_status(
        &self,
        rule_id: &str,
        status: &str,
    ) -> anyhow::Result<Option<RuleSummaryRecord>> {
        ensure_default_rules_seeded(&self.pool).await?;
        let result =
            sqlx::query("UPDATE rules SET status = $1, updated_at = now() WHERE rule_key = $2")
                .bind(status)
                .bind(rule_id)
                .execute(&self.pool)
                .await?;
        if result.rows_affected() == 0 {
            return Ok(None);
        }
        Ok(self
            .list_rules()
            .await?
            .into_iter()
            .find(|rule| rule.rule_id == rule_id))
    }

    async fn list_models(&self) -> anyhow::Result<Vec<ModelVersionRecord>> {
        ensure_default_models_seeded(&self.pool).await?;
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT model_key, version, model_type, runtime_kind, execution_provider, status, artifact_uri, endpoint_url
             FROM model_versions
             ORDER BY model_key, version DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(
                |(
                    model_key,
                    version,
                    model_type,
                    runtime_kind,
                    execution_provider,
                    status,
                    artifact_uri,
                    endpoint_url,
                )| ModelVersionRecord {
                    model_key,
                    version,
                    model_type,
                    runtime_kind,
                    execution_provider,
                    status,
                    artifact_uri,
                    endpoint_url,
                },
            )
            .collect())
    }

    async fn model_performance(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Option<ModelPerformanceRecord>> {
        ensure_default_models_seeded(&self.pool).await?;
        let known = self
            .list_models()
            .await?
            .into_iter()
            .any(|model| model.model_key == model_key);
        if !known {
            return Ok(None);
        }

        let row: (
            i64,
            Option<Decimal>,
            Option<i64>,
            Option<chrono::DateTime<chrono::Utc>>,
        ) = sqlx::query_as(
            "SELECT
                   COUNT(*)::bigint,
                   AVG(score),
                   SUM(CASE WHEN score >= 70 THEN 1 ELSE 0 END)::bigint,
                   MAX(created_at)
                 FROM model_scores
                 WHERE model_key = $1",
        )
        .bind(model_key)
        .fetch_one(&self.pool)
        .await?;

        let scored_runs = row.0 as u32;
        if scored_runs == 0 {
            return Ok(Some(empty_model_performance(model_key)));
        }

        Ok(Some(ModelPerformanceRecord {
            model_key: model_key.to_string(),
            data_status: "ready".into(),
            scored_runs,
            average_score: row
                .1
                .map(|value| value.to_string().parse().unwrap_or(0.0))
                .unwrap_or(0.0),
            high_risk_count: row.2.unwrap_or(0) as u32,
            latest_scored_at: row.3.map(|timestamp| timestamp.to_rfc3339()),
        }))
    }

    async fn list_knowledge_cases(&self) -> anyhow::Result<Vec<KnowledgeCaseRecord>> {
        ensure_default_knowledge_cases_seeded(&self.pool).await?;
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Value,
            Value,
        )> = sqlx::query_as(
            "SELECT case_id, title, fwa_type, diagnosis_code, provider_region, provider_type, summary, outcome, tags, evidence_refs
             FROM knowledge_cases
             ORDER BY case_id",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    case_id,
                    title,
                    fwa_type,
                    diagnosis_code,
                    provider_region,
                    provider_type,
                    summary,
                    outcome,
                    tags,
                    evidence_refs,
                )| KnowledgeCaseRecord {
                    case_id,
                    title,
                    fwa_type,
                    diagnosis_code,
                    provider_region,
                    provider_type,
                    summary,
                    outcome,
                    tags: json_array_to_strings(tags),
                    evidence_refs: json_array_to_strings(evidence_refs),
                },
            )
            .collect())
    }

    async fn search_similar_cases(
        &self,
        query: SimilarCaseQuery,
    ) -> anyhow::Result<Vec<SimilarCaseRecord>> {
        let cases = self.list_knowledge_cases().await?;
        Ok(search_cases(cases, &query))
    }

    async fn save_agent_run(&self, run: PersistedAgentRun) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO agent_runs
             (agent_run_id, claim_id, status, decision_boundary, output_json, evidence_refs, completed_at)
             VALUES ($1, $2, $3, $4, $5, $6, now())
             ON CONFLICT (agent_run_id) DO UPDATE
             SET status = EXCLUDED.status,
                 decision_boundary = EXCLUDED.decision_boundary,
                 output_json = EXCLUDED.output_json,
                 evidence_refs = EXCLUDED.evidence_refs,
                 completed_at = EXCLUDED.completed_at",
        )
        .bind(&run.agent_run_id)
        .bind(&run.claim_id)
        .bind(&run.status)
        .bind(&run.decision_boundary)
        .bind(&run.output_json)
        .bind(Value::Array(run.evidence_refs.clone()))
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM agent_steps WHERE agent_run_id = $1")
            .bind(&run.agent_run_id)
            .execute(&mut *tx)
            .await?;

        for step in &run.steps {
            sqlx::query(
                "INSERT INTO agent_steps
                 (agent_run_id, step_name, status, output_json, evidence_refs)
                 VALUES ($1, $2, 'succeeded', $3, $4)",
            )
            .bind(&run.agent_run_id)
            .bind(step["step_name"].as_str().unwrap_or("investigate"))
            .bind(step)
            .bind(step["evidence_refs"].clone())
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn register_dataset(&self, input: RegisterDatasetInput) -> anyhow::Result<DatasetRecord> {
        let mut tx = self.pool.begin().await?;
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
        load_dataset_record(&self.pool, &dataset_row.0)
            .await?
            .ok_or_else(|| anyhow::anyhow!("registered dataset was not found"))
    }

    async fn list_datasets(&self) -> anyhow::Result<Vec<DatasetRecord>> {
        let ids: Vec<(String,)> = sqlx::query_as(
            "SELECT id::text FROM external_dataset_versions ORDER BY dataset_key, dataset_version",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut datasets = Vec::new();
        for (id,) in ids {
            if let Some(dataset) = load_dataset_record(&self.pool, &id).await? {
                datasets.push(dataset);
            }
        }
        Ok(datasets)
    }

    async fn get_dataset(&self, dataset_id: &str) -> anyhow::Result<Option<DatasetRecord>> {
        load_dataset_record(&self.pool, dataset_id).await
    }

    async fn add_field_mapping(
        &self,
        dataset_id: &str,
        input: CreateFieldMappingInput,
    ) -> anyhow::Result<Option<FieldMappingRecord>> {
        if load_dataset_record(&self.pool, dataset_id).await?.is_none() {
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
        .fetch_one(&self.pool)
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

    async fn save_investigation_result(
        &self,
        record: InvestigationResultRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO investigation_results
             (investigation_id, claim_id, outcome, confirmed_fwa, saving_amount, currency, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (investigation_id) DO UPDATE
             SET outcome = EXCLUDED.outcome,
                 confirmed_fwa = EXCLUDED.confirmed_fwa,
                 saving_amount = EXCLUDED.saving_amount,
                 currency = EXCLUDED.currency,
                 notes = EXCLUDED.notes,
                 evidence_refs = EXCLUDED.evidence_refs",
        )
        .bind(&record.investigation_id)
        .bind(&record.claim_id)
        .bind(&record.outcome)
        .bind(record.confirmed_fwa)
        .bind(record.saving_amount)
        .bind(&record.currency)
        .bind(&record.notes)
        .bind(serde_json::json!(record.evidence_refs))
        .execute(&mut *tx)
        .await?;

        let event = AuditHistoryEventRecord {
            audit_id: format!("audit_investigation_{}", record.investigation_id),
            run_id: format!("pilot_investigation_{}", record.investigation_id),
            event_type: "investigation.result.received".into(),
            event_status: "succeeded".into(),
            summary: format!("Investigation result received: {}", record.outcome),
            payload: serde_json::to_value(&record)?,
            evidence_refs: record.evidence_refs.clone(),
            created_at: None,
        };
        insert_pilot_audit_event(&mut tx, &record.claim_id, &event, "tpa_system").await?;
        tx.commit().await?;
        Ok(event)
    }

    async fn save_qa_review(
        &self,
        record: QaReviewRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO qa_reviews
             (qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (qa_case_id) DO UPDATE
             SET qa_conclusion = EXCLUDED.qa_conclusion,
                 issue_type = EXCLUDED.issue_type,
                 feedback_target = EXCLUDED.feedback_target,
                 notes = EXCLUDED.notes,
                 evidence_refs = EXCLUDED.evidence_refs",
        )
        .bind(&record.qa_case_id)
        .bind(&record.claim_id)
        .bind(&record.qa_conclusion)
        .bind(&record.issue_type)
        .bind(&record.feedback_target)
        .bind(&record.notes)
        .bind(serde_json::json!(record.evidence_refs))
        .execute(&mut *tx)
        .await?;

        let event = AuditHistoryEventRecord {
            audit_id: format!("audit_qa_{}", record.qa_case_id),
            run_id: format!("pilot_qa_{}", record.qa_case_id),
            event_type: "qa.result.received".into(),
            event_status: "succeeded".into(),
            summary: format!("QA result received: {}", record.qa_conclusion),
            payload: serde_json::to_value(&record)?,
            evidence_refs: record.evidence_refs.clone(),
            created_at: None,
        };
        insert_pilot_audit_event(&mut tx, &record.claim_id, &event, "qa_reviewer").await?;
        tx.commit().await?;
        Ok(event)
    }

    async fn claim_audit_history(
        &self,
        claim_id: &str,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        let rows: Vec<(String, String, String, String, String, Value, Value, chrono::DateTime<chrono::Utc>)> =
            sqlx::query_as(
                "SELECT audit_id, run_id, event_type, event_status, summary, payload, evidence_refs, created_at
                 FROM audit_events
                 WHERE payload ->> 'claim_id' = $1
                 ORDER BY created_at, audit_id",
            )
            .bind(claim_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    audit_id,
                    run_id,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs,
                    created_at,
                )| AuditHistoryEventRecord {
                    audit_id,
                    run_id,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: Some(created_at.to_rfc3339()),
                },
            )
            .collect())
    }
}

fn _decimal_keeps_sqlx_feature_linked(_: Decimal) {}

pub fn default_runtime_rules() -> Vec<Rule> {
    vec![Rule {
        rule_id: "rule_early_claim".into(),
        version: 1,
        name: "Early claim".into(),
        conditions: vec![Condition {
            field: "days_since_policy_start".into(),
            operator: "<=".into(),
            value: serde_json::json!(7),
        }],
        action: RuleAction {
            score: 75,
            alert_code: "EARLY_CLAIM".into(),
            recommended_action: RecommendedAction::ManualReview,
            reason: "保单生效后 7 天内发生理赔".into(),
        },
    }]
}

fn default_rule_details() -> Vec<RuleDetailRecord> {
    default_runtime_rules()
        .into_iter()
        .map(|rule| {
            let dsl = serde_json::json!({
                "conditions": rule.conditions,
                "action": rule.action
            });
            let summary = RuleSummaryRecord {
                rule_id: rule.rule_id.clone(),
                name: rule.name.clone(),
                status: "active".into(),
                owner: "rules-ops".into(),
                active_version: Some(rule.version),
                latest_version: rule.version,
                score: rule.action.score,
                alert_code: rule.action.alert_code.clone(),
                recommended_action: rule.action.recommended_action,
            };
            let version = RuleVersionRecord {
                version: rule.version,
                status: "active".into(),
                dsl,
                score: rule.action.score,
                alert_code: rule.action.alert_code,
                recommended_action: rule.action.recommended_action,
                reason: rule.action.reason,
            };
            RuleDetailRecord {
                summary,
                versions: vec![version],
            }
        })
        .collect()
}

fn parse_recommended_action(value: &str) -> RecommendedAction {
    match value {
        "AutoApprove" => RecommendedAction::AutoApprove,
        "EscalateInvestigation" => RecommendedAction::EscalateInvestigation,
        _ => RecommendedAction::ManualReview,
    }
}

async fn ensure_default_rules_seeded(pool: &PgPool) -> anyhow::Result<()> {
    for detail in default_rule_details() {
        let mut tx = pool.begin().await?;
        let row: (String,) = sqlx::query_as(
            "INSERT INTO rules (rule_key, name, status, owner)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (rule_key) DO UPDATE SET updated_at = now()
             RETURNING id::text",
        )
        .bind(&detail.summary.rule_id)
        .bind(&detail.summary.name)
        .bind(&detail.summary.status)
        .bind(&detail.summary.owner)
        .fetch_one(&mut *tx)
        .await?;

        for version in detail.versions {
            sqlx::query(
                "INSERT INTO rule_versions
                 (rule_id, version, dsl, score, recommended_action, created_by, approved_by, published_at)
                 VALUES ($1::uuid, $2, $3, $4, $5, 'system', 'system', now())
                 ON CONFLICT (rule_id, version) DO NOTHING",
            )
            .bind(&row.0)
            .bind(version.version as i32)
            .bind(&version.dsl)
            .bind(version.score as i32)
            .bind(format!("{:?}", version.recommended_action))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
    }
    Ok(())
}

fn default_model_versions() -> Vec<ModelVersionRecord> {
    vec![ModelVersionRecord {
        model_key: "baseline_fwa".into(),
        version: "0.1.0".into(),
        model_type: "baseline_classifier".into(),
        runtime_kind: "python_http".into(),
        execution_provider: "cpu".into(),
        status: "active".into(),
        artifact_uri: None,
        endpoint_url: Some("http://127.0.0.1:8001/score".into()),
    }]
}

fn empty_model_performance(model_key: &str) -> ModelPerformanceRecord {
    ModelPerformanceRecord {
        model_key: model_key.to_string(),
        data_status: "empty".into(),
        scored_runs: 0,
        average_score: 0.0,
        high_risk_count: 0,
        latest_scored_at: None,
    }
}

fn default_knowledge_cases() -> Vec<KnowledgeCaseRecord> {
    vec![
        KnowledgeCaseRecord {
            case_id: "KC-1001".into(),
            title: "Early high-amount respiratory claim".into(),
            fwa_type: "Abuse".into(),
            diagnosis_code: "J10".into(),
            provider_region: "Shanghai".into(),
            provider_type: "hospital".into(),
            summary: "保单生效早期发生高额呼吸系统相关理赔，项目组合与相似已确认案例接近。".into(),
            outcome: "Manual review confirmed over-treatment pattern".into(),
            tags: vec![
                "early_claim".into(),
                "high_amount".into(),
                "medical_mismatch".into(),
            ],
            evidence_refs: vec![
                "knowledge_cases:KC-1001".into(),
                "rule_runs:EARLY_CLAIM".into(),
            ],
        },
        KnowledgeCaseRecord {
            case_id: "KC-1002".into(),
            title: "Provider repeated high-cost package pattern".into(),
            fwa_type: "Waste".into(),
            diagnosis_code: "M54".into(),
            provider_region: "Beijing".into(),
            provider_type: "clinic".into(),
            summary: "同一 provider 在短期内重复出现高价项目组合，金额分布显著偏离同地区 peer。"
                .into(),
            outcome: "Provider education and pre-payment review added".into(),
            tags: vec!["provider_pattern".into(), "high_amount".into()],
            evidence_refs: vec![
                "knowledge_cases:KC-1002".into(),
                "feature_values:provider_high_cost_item_ratio_30d".into(),
            ],
        },
    ]
}

fn search_cases(
    cases: Vec<KnowledgeCaseRecord>,
    query: &SimilarCaseQuery,
) -> Vec<SimilarCaseRecord> {
    let mut results = cases
        .into_iter()
        .filter_map(|case| {
            let mut score: f64 = 0.0;
            let mut matched_signals = Vec::new();

            if case.diagnosis_code == query.diagnosis_code {
                score += 0.45;
                matched_signals.push(format!("diagnosis:{}", query.diagnosis_code));
            }
            if case.provider_region == query.provider_region {
                score += 0.25;
                matched_signals.push(format!("region:{}", query.provider_region));
            }
            for tag in &query.tags {
                if case.tags.iter().any(|case_tag| case_tag == tag) {
                    score += 0.15;
                    matched_signals.push(format!("tag:{tag}"));
                }
            }

            if score <= 0.0 {
                None
            } else {
                Some(SimilarCaseRecord {
                    case_id: case.case_id,
                    title: case.title,
                    similarity_score: score.min(1.0),
                    matched_signals,
                    summary: case.summary,
                    outcome: case.outcome,
                    evidence_refs: case.evidence_refs,
                })
            }
        })
        .collect::<Vec<_>>();

    results.sort_by(|left, right| {
        right
            .similarity_score
            .partial_cmp(&left.similarity_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results
}

fn json_array_to_strings(value: Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

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

async fn load_dataset_record(
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

async fn ensure_default_models_seeded(pool: &PgPool) -> anyhow::Result<()> {
    for model in default_model_versions() {
        sqlx::query(
            "INSERT INTO model_versions
             (model_key, version, model_type, runtime_kind, artifact_uri, endpoint_url, execution_provider, status, metrics, activated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, '{}'::jsonb, now())
             ON CONFLICT (model_key, version) DO UPDATE SET status = EXCLUDED.status",
        )
        .bind(&model.model_key)
        .bind(&model.version)
        .bind(&model.model_type)
        .bind(&model.runtime_kind)
        .bind(&model.artifact_uri)
        .bind(&model.endpoint_url)
        .bind(&model.execution_provider)
        .bind(&model.status)
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn ensure_default_knowledge_cases_seeded(pool: &PgPool) -> anyhow::Result<()> {
    for case in default_knowledge_cases() {
        sqlx::query(
            "INSERT INTO knowledge_cases
             (case_id, title, fwa_type, diagnosis_code, provider_region, provider_type, summary, outcome, tags, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (case_id) DO UPDATE SET updated_at = now()",
        )
        .bind(&case.case_id)
        .bind(&case.title)
        .bind(&case.fwa_type)
        .bind(&case.diagnosis_code)
        .bind(&case.provider_region)
        .bind(&case.provider_type)
        .bind(&case.summary)
        .bind(&case.outcome)
        .bind(serde_json::json!(case.tags))
        .bind(serde_json::json!(case.evidence_refs))
        .execute(pool)
        .await?;
    }
    Ok(())
}

async fn insert_audit_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    event: &PersistedAuditEvent,
    claim_uuid: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO audit_events
         (audit_id, run_id, claim_id, actor_id, actor_role, source_system, event_type, event_status, summary, payload, evidence_refs)
         VALUES ($1, $2, $3::uuid, $4, $5, $6, $7, $8, $9, $10, $11)",
    )
    .bind(&event.audit_id)
    .bind(&event.run_id)
    .bind(claim_uuid)
    .bind(&event.actor_id)
    .bind(&event.actor_role)
    .bind(&event.source_system)
    .bind(&event.event_type)
    .bind(&event.event_status)
    .bind(&event.summary)
    .bind(&event.payload)
    .bind(serde_json::Value::Array(event.evidence_refs.clone()))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn insert_pilot_audit_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    claim_id: &str,
    event: &AuditHistoryEventRecord,
    actor_role: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO scoring_runs
         (run_id, source_system, actor_id, status, completed_at)
         VALUES ($1, 'pilot-loop', $2, 'succeeded', now())
         ON CONFLICT (run_id) DO NOTHING",
    )
    .bind(&event.run_id)
    .bind(actor_role)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        "INSERT INTO audit_events
         (audit_id, run_id, claim_id, actor_id, actor_role, source_system, event_type, event_status, summary, payload, evidence_refs)
         VALUES ($1, $2, NULL, $3, $4, 'pilot-loop', $5, $6, $7, $8, $9)
         ON CONFLICT (audit_id) DO UPDATE
         SET event_status = EXCLUDED.event_status,
             summary = EXCLUDED.summary,
             payload = EXCLUDED.payload,
             evidence_refs = EXCLUDED.evidence_refs",
    )
    .bind(&event.audit_id)
    .bind(&event.run_id)
    .bind(claim_id)
    .bind(actor_role)
    .bind(&event.event_type)
    .bind(&event.event_status)
    .bind(&event.summary)
    .bind(&event.payload)
    .bind(serde_json::json!(event.evidence_refs))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

use async_trait::async_trait;
use fwa_core::ClaimContext;
use rust_decimal::Decimal;
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
}

pub type SharedRepository = Arc<dyn ScoringRepository>;

#[derive(Debug, Default)]
pub struct InMemoryScoringRepository {
    claims: Mutex<HashMap<String, ClaimContext>>,
    runs: Mutex<Vec<PersistedScoringRun>>,
    audit_events: Mutex<Vec<PersistedAuditEvent>>,
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
}

fn _decimal_keeps_sqlx_feature_linked(_: Decimal) {}

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

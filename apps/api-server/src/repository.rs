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
}

pub type SharedRepository = Arc<dyn ScoringRepository>;

#[derive(Debug, Default)]
pub struct InMemoryScoringRepository {
    claims: Mutex<HashMap<String, ClaimContext>>,
    runs: Mutex<Vec<PersistedScoringRun>>,
    audit_events: Mutex<Vec<PersistedAuditEvent>>,
    rule_statuses: Mutex<HashMap<String, String>>,
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

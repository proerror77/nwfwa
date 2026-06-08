use super::*;
use std::time::Duration;

const DEFAULT_DB_MAX_CONNECTIONS: u32 = 20;
const DEFAULT_DB_ACQUIRE_TIMEOUT_SECONDS: u64 = 5;

#[derive(Debug, Clone)]
pub struct PostgresScoringRepository {
    pub(super) pool: PgPool,
}

fn configured_db_max_connections() -> u32 {
    positive_u32_env("FWA_DB_MAX_CONNECTIONS", DEFAULT_DB_MAX_CONNECTIONS)
}

fn configured_db_acquire_timeout_seconds() -> u64 {
    positive_u64_env(
        "FWA_DB_ACQUIRE_TIMEOUT_SECONDS",
        DEFAULT_DB_ACQUIRE_TIMEOUT_SECONDS,
    )
}

fn positive_u32_env(name: &str, default: u32) -> u32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn positive_u64_env(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

impl PostgresScoringRepository {
    pub async fn connect(database_url: &str) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(configured_db_max_connections())
            .acquire_timeout(Duration::from_secs(configured_db_acquire_timeout_seconds()))
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_u32_env_accepts_positive_values() {
        let name = "NWFWA_TEST_POSITIVE_U32_ENV";
        std::env::set_var(name, "32");

        assert_eq!(positive_u32_env(name, 20), 32);

        std::env::remove_var(name);
    }

    #[test]
    fn positive_env_helpers_fall_back_for_invalid_values() {
        let u32_name = "NWFWA_TEST_INVALID_U32_ENV";
        let u64_name = "NWFWA_TEST_INVALID_U64_ENV";
        std::env::set_var(u32_name, "0");
        std::env::set_var(u64_name, "not-a-number");

        assert_eq!(positive_u32_env(u32_name, 20), 20);
        assert_eq!(positive_u64_env(u64_name, 5), 5);

        std::env::remove_var(u32_name);
        std::env::remove_var(u64_name);
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
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<ClaimContext>> {
        let row: Option<ClaimContextRow> = sqlx::query_as(
            "SELECT c.external_claim_id,
                    c.diagnosis_code,
                    c.service_date,
                    c.claim_amount,
                    c.currency AS claim_currency,
                    m.external_member_id,
                    m.dob,
                    m.gender,
                    p.external_policy_id,
                    p.product_code,
                    p.coverage_start_date,
                    p.coverage_end_date,
                    p.coverage_limit_amount,
                    p.currency AS policy_currency,
                    pr.external_provider_id,
                    pr.name AS provider_name,
                    pr.provider_type,
                    pr.region AS provider_region,
                    pr.risk_tier AS provider_risk_tier
             FROM claims c
             JOIN members m ON m.id = c.member_id
             JOIN policies p ON p.id = c.policy_id
             JOIN providers pr ON pr.id = c.provider_id
             WHERE c.external_claim_id = $1
               AND (
                 $2::text IS NULL OR EXISTS (
                   SELECT 1
                   FROM audit_events ae
                   WHERE ae.claim_id = c.id
                     AND ae.payload ->> 'customer_scope_id' = $2
                 )
               )",
        )
        .bind(external_claim_id)
        .bind(customer_scope_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let item_rows: Vec<ClaimItemRow> = sqlx::query_as(
            "SELECT ci.item_code, ci.item_type, ci.description, ci.quantity, ci.unit_amount, ci.total_amount, ci.currency
             FROM claim_items ci
             JOIN claims c ON c.id = ci.claim_id
             WHERE c.external_claim_id = $1
             ORDER BY ci.created_at, ci.item_code",
        )
        .bind(external_claim_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(Some(row.into_context(item_rows)))
    }

    async fn member_profile_summary(
        &self,
        member_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<MemberProfileSummaryRecord>> {
        let member_exists: Option<(String,)> = sqlx::query_as(
            "SELECT m.external_member_id
             FROM members m
             WHERE m.external_member_id = $1
               AND (
                 $2::text IS NULL OR EXISTS (
                   SELECT 1
                   FROM claims c
                   JOIN audit_events ae ON ae.claim_id = c.id
                   WHERE c.member_id = m.id
                     AND ae.payload ->> 'customer_scope_id' = $2
                 )
               )",
        )
        .bind(member_id)
        .bind(customer_scope_id)
        .fetch_optional(&self.pool)
        .await?;
        if member_exists.is_none() {
            return Ok(None);
        }

        let row: (i64, i64, Option<Decimal>, Option<String>) = sqlx::query_as(
            "SELECT COUNT(c.id)::bigint,
                    COUNT(DISTINCT p.id)::bigint,
                    SUM(c.claim_amount),
                    MIN(c.currency)
             FROM members m
             LEFT JOIN claims c ON c.member_id = m.id
             LEFT JOIN policies p ON p.id = c.policy_id
             WHERE m.external_member_id = $1
               AND (
                 $2::text IS NULL OR EXISTS (
                   SELECT 1
                   FROM audit_events ae
                   WHERE ae.claim_id = c.id
                     AND ae.payload ->> 'customer_scope_id' = $2
                 )
               )",
        )
        .bind(member_id)
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;
        let latest_claim: Option<(String,)> = sqlx::query_as(
            "SELECT c.external_claim_id
             FROM claims c
             JOIN members m ON m.id = c.member_id
             WHERE m.external_member_id = $1
               AND (
                 $2::text IS NULL OR EXISTS (
                   SELECT 1
                   FROM audit_events ae
                   WHERE ae.claim_id = c.id
                     AND ae.payload ->> 'customer_scope_id' = $2
                 )
               )
             ORDER BY c.service_date DESC, c.external_claim_id DESC
             LIMIT 1",
        )
        .bind(member_id)
        .bind(customer_scope_id)
        .fetch_optional(&self.pool)
        .await?;
        let high_risk: (i64,) = sqlx::query_as(
            "SELECT COUNT(DISTINCT c.id)::bigint
             FROM members m
             JOIN claims c ON c.member_id = m.id
             JOIN scoring_runs sr ON sr.claim_id = c.id
             WHERE m.external_member_id = $1
               AND (
                 $2::text IS NULL OR EXISTS (
                   SELECT 1
                   FROM audit_events ae
                   WHERE ae.claim_id = c.id
                     AND ae.payload ->> 'customer_scope_id' = $2
                 )
               )
               AND sr.risk_score >= 70",
        )
        .bind(member_id)
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(Some(member_profile_summary_record(
            MemberProfileSummaryInput {
                member_id: member_id.into(),
                claim_count: row.0 as u32,
                policy_count: row.1 as u32,
                total_claim_amount: row.2.unwrap_or(Decimal::ZERO),
                currency: row.3.unwrap_or_else(|| "UNKNOWN".into()),
                high_risk_claim_count: high_risk.0 as u32,
                latest_claim_id: latest_claim.map(|claim| claim.0),
                evidence_refs: BTreeSet::from([format!("members:{member_id}")]),
            },
        )))
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
             (run_id, claim_id, source_system, actor_id, status, risk_score, rag, risk_level, recommended_action, confidence_score, confidence, routing_reason, routing_policy, score_breakdown, completed_at)
             VALUES ($1, $2::uuid, $3, $4, 'succeeded', $5, $6, $7, $8, $9, $10, $11, $12, $13, now())",
        )
        .bind(&run.run_id)
        .bind(claim_uuid.as_deref())
        .bind(&run.source_system)
        .bind(&run.actor_id)
        .bind(run.risk_score as i32)
        .bind(&run.rag)
        .bind(&run.risk_level)
        .bind(&run.recommended_action)
        .bind(run.confidence_score as i32)
        .bind(&run.confidence)
        .bind(&run.routing_reason)
        .bind(&run.routing_policy)
        .bind(&run.score_breakdown)
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
            let rule_evidence = rule_run
                .get("evidence_refs")
                .filter(|evidence| evidence.is_array())
                .cloned()
                .unwrap_or_else(|| serde_json::json!([]));
            sqlx::query(
                "INSERT INTO rule_runs
                 (run_id, rule_id, rule_version_id, matched, score_contribution, alert_code, reason, evidence_json)
                 VALUES (
                   $1,
                   (SELECT id FROM rules WHERE rule_key = $2),
                   (
                     SELECT rv.id
                     FROM rule_versions rv
                     JOIN rules r ON r.id = rv.rule_id
                     WHERE r.rule_key = $2 AND rv.version = $3
                   ),
                   true,
                   $4,
                   $5,
                   $6,
                   $7
                 )",
            )
            .bind(&run.run_id)
            .bind(rule_run["rule_id"].as_str())
            .bind(rule_run["rule_version"].as_i64().unwrap_or(1) as i32)
            .bind(rule_run["score_contribution"].as_i64().unwrap_or(0) as i32)
            .bind(rule_run["alert_code"].as_str())
            .bind(rule_run["reason"].as_str())
            .bind(rule_evidence)
            .execute(&mut *tx)
            .await?;
        }

        sqlx::query(
            "INSERT INTO model_scores
             (run_id, model_version_id, model_key, runtime_kind, execution_provider, score, label, explanation_json, latency_ms)
             VALUES (
               $1,
               (
                 SELECT id
                 FROM model_versions
                 WHERE model_key = $2 AND version = $3
               ),
               $2,
               $4,
               $5,
               $6,
               $7,
               $8,
               $9
             )",
        )
        .bind(&run.run_id)
        .bind(run.model_score["model_key"].as_str().unwrap_or("unknown"))
        .bind(run.model_score["model_version"].as_str().unwrap_or("unknown"))
        .bind(run.model_score["runtime_kind"].as_str().unwrap_or("unknown"))
        .bind(run.model_score["execution_provider"].as_str().unwrap_or("cpu"))
        .bind(run.model_score["score"].as_i64().unwrap_or(0) as i32)
        .bind(run.model_score["label"].as_str().unwrap_or("UNKNOWN"))
        .bind(run.model_score["explanations"].clone())
        .bind(run.model_score["latency_ms"].as_i64().unwrap_or(0) as i32)
        .execute(&mut *tx)
        .await?;

        if let Some(mut lead) = lead_from_scoring_run(&run, None) {
            if let Some((member_id, provider_id)) = sqlx::query_as::<_, (String, String)>(
                "SELECT m.external_member_id, pr.external_provider_id
                 FROM claims c
                 JOIN members m ON m.id = c.member_id
                 JOIN providers pr ON pr.id = c.provider_id
                 WHERE c.external_claim_id = $1",
            )
            .bind(&run.claim_id)
            .fetch_optional(&mut *tx)
            .await?
            {
                lead.member_id = member_id;
                lead.provider_id = provider_id;
            }
            sqlx::query(
                "INSERT INTO fwa_leads
                 (lead_id, run_id, claim_id, member_id, provider_id, source_system, review_mode, scheme_family, lead_source, status, disposition, risk_score, rag, reason, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
                 ON CONFLICT (lead_id) DO UPDATE
                 SET run_id = EXCLUDED.run_id,
                     claim_id = EXCLUDED.claim_id,
                     member_id = EXCLUDED.member_id,
                     provider_id = EXCLUDED.provider_id,
                     source_system = EXCLUDED.source_system,
                     review_mode = EXCLUDED.review_mode,
                     scheme_family = EXCLUDED.scheme_family,
                     lead_source = EXCLUDED.lead_source,
                     status = EXCLUDED.status,
                     disposition = EXCLUDED.disposition,
                     risk_score = EXCLUDED.risk_score,
                     rag = EXCLUDED.rag,
                     reason = EXCLUDED.reason,
                     evidence_refs = EXCLUDED.evidence_refs,
                     updated_at = now()",
            )
            .bind(&lead.lead_id)
            .bind(&lead.run_id)
            .bind(&lead.claim_id)
            .bind(&lead.member_id)
            .bind(&lead.provider_id)
            .bind(&lead.source_system)
            .bind(&lead.review_mode)
            .bind(&lead.scheme_family)
            .bind(&lead.lead_source)
            .bind(&lead.status)
            .bind(&lead.disposition)
            .bind(lead.risk_score as i32)
            .bind(&lead.rag)
            .bind(&lead.reason)
            .bind(serde_json::json!(lead.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }

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

    async fn save_inbox_claim_run(&self, run: PersistedInboxClaimRun) -> anyhow::Result<()> {
        postgres_inbox::save_inbox_claim_run(self, run).await
    }

    async fn get_inbox_claim_run_by_idempotency_key(
        &self,
        idempotency_key: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>> {
        postgres_inbox::get_inbox_claim_run_by_idempotency_key(
            self,
            idempotency_key,
            customer_scope_id,
        )
        .await
    }

    async fn get_inbox_claim_run_by_run_id(
        &self,
        run_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<PersistedInboxClaimRun>> {
        postgres_inbox::get_inbox_claim_run_by_run_id(self, run_id, customer_scope_id).await
    }

    async fn active_routing_policy(
        &self,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicy>> {
        postgres_routing_policies::active_routing_policy(self, review_mode).await
    }

    async fn list_routing_policies(&self) -> anyhow::Result<Vec<RoutingPolicyRecord>> {
        postgres_routing_policies::list_routing_policies(self).await
    }

    async fn save_routing_policy_candidate(
        &self,
        policy: RoutingPolicy,
        owner: String,
    ) -> anyhow::Result<RoutingPolicyRecord> {
        postgres_routing_policies::save_routing_policy_candidate(self, policy, owner).await
    }

    async fn get_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        postgres_routing_policies::get_routing_policy(self, policy_id, version, review_mode).await
    }

    async fn update_routing_policy_status(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
        status: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        postgres_routing_policies::update_routing_policy_status(
            self,
            policy_id,
            version,
            review_mode,
            status,
        )
        .await
    }

    async fn activate_routing_policy(
        &self,
        policy_id: &str,
        version: u32,
        review_mode: &str,
    ) -> anyhow::Result<Option<RoutingPolicyRecord>> {
        postgres_routing_policies::activate_routing_policy(self, policy_id, version, review_mode)
            .await
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

        let mut summaries = rows
            .into_iter()
            .map(
                |(rule_id, name, status, owner, version, dsl, score, recommended_action)| {
                    let action = dsl.get("action").cloned().unwrap_or(Value::Null);
                    let review_mode = review_mode_from_dsl(&dsl);
                    let scheme_family = scheme_family_from_dsl(&dsl);
                    RuleSummaryRecord {
                        rule_id: rule_id.clone(),
                        name,
                        active_version: if status == "active" {
                            Some(version as u32)
                        } else {
                            None
                        },
                        latest_version: version as u32,
                        review_mode: review_mode.clone(),
                        scheme_family: scheme_family.clone(),
                        status,
                        owner,
                        score: score as u8,
                        alert_code: action["alert_code"]
                            .as_str()
                            .unwrap_or("UNKNOWN")
                            .to_string(),
                        recommended_action: parse_recommended_action(&recommended_action),
                        applicability_scope: rule_applicability_scope(&review_mode, &scheme_family),
                        backtest_result: default_rule_backtest_summary(),
                        estimated_saving: "0.00".into(),
                        false_positive_history: default_rule_false_positive_history(),
                        evidence_refs: rule_governance_evidence_refs(&rule_id, version as u32),
                    }
                },
            )
            .collect::<Vec<_>>();

        for summary in &mut summaries {
            let latest_backtest = self
                .latest_rule_backtest(&summary.rule_id, summary.latest_version)
                .await?;
            apply_rule_backtest_metadata(summary, latest_backtest.as_ref());
        }

        Ok(summaries)
    }

    async fn list_active_rules(&self) -> anyhow::Result<Vec<Rule>> {
        ensure_default_rules_seeded(&self.pool).await?;
        let rows: Vec<(String, String, i32, Value)> = sqlx::query_as(
            "SELECT r.rule_key, r.name, rv.version, rv.dsl
             FROM rules r
             JOIN LATERAL (
               SELECT version, dsl
               FROM rule_versions
               WHERE rule_id = r.id
               ORDER BY version DESC
               LIMIT 1
             ) rv ON true
             WHERE r.status = 'active'
             ORDER BY r.rule_key",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter()
            .map(|(rule_id, name, version, dsl)| {
                runtime_rule_from_parts(rule_id, name, version as u32, dsl)
            })
            .collect()
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
                    review_mode: review_mode_from_dsl(&dsl),
                    scheme_family: scheme_family_from_dsl(&dsl),
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

        let audit_events = self.rule_audit_history(rule_id).await?;

        Ok(Some(RuleDetailRecord {
            summary,
            versions,
            audit_events,
        }))
    }

    async fn rule_audit_history(
        &self,
        rule_id: &str,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        let rows: Vec<(String, String, String, String, String, String, Value, Value, chrono::DateTime<chrono::Utc>)> =
            sqlx::query_as(
                "SELECT audit_id, run_id, actor_role, event_type, event_status, summary, payload, evidence_refs, created_at
                 FROM audit_events
                 WHERE payload ->> 'rule_id' = $1
                 ORDER BY created_at, audit_id",
            )
            .bind(rule_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs,
                    created_at,
                )| AuditHistoryEventRecord {
                    audit_id,
                    run_id,
                    actor_role,
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

    async fn save_rule_candidate(
        &self,
        rule: Rule,
        owner: String,
    ) -> anyhow::Result<RuleDetailRecord> {
        ensure_default_rules_seeded(&self.pool).await?;
        ensure_rule_condition_library_table(&self.pool).await?;
        let detail = rule_detail_from_rule(rule, "draft", owner);
        let mut tx = self.pool.begin().await?;
        let row: (String,) = sqlx::query_as(
            "INSERT INTO rules (rule_key, name, status, owner)
             VALUES ($1, $2, 'draft', $3)
             ON CONFLICT (rule_key) DO UPDATE
             SET name = EXCLUDED.name,
                 status = 'draft',
                 owner = EXCLUDED.owner,
                 updated_at = now()
             RETURNING id::text",
        )
        .bind(&detail.summary.rule_id)
        .bind(&detail.summary.name)
        .bind(&detail.summary.owner)
        .fetch_one(&mut *tx)
        .await?;

        let version = &detail.versions[0];
        sqlx::query(
            "INSERT INTO rule_versions
             (rule_id, version, dsl, score, recommended_action, created_by)
             VALUES ($1::uuid, $2, $3, $4, $5, $6)
             ON CONFLICT (rule_id, version) DO UPDATE
             SET dsl = EXCLUDED.dsl,
                 score = EXCLUDED.score,
                 recommended_action = EXCLUDED.recommended_action",
        )
        .bind(&row.0)
        .bind(version.version as i32)
        .bind(&version.dsl)
        .bind(version.score as i32)
        .bind(format!("{:?}", version.recommended_action))
        .bind(&detail.summary.owner)
        .execute(&mut *tx)
        .await?;

        upsert_rule_conditions_tx(&mut tx, &row.0, &detail).await?;

        tx.commit().await?;
        Ok(detail)
    }

    async fn update_rule_status(
        &self,
        rule_id: &str,
        status: &str,
    ) -> anyhow::Result<Option<RuleSummaryRecord>> {
        ensure_default_rules_seeded(&self.pool).await?;
        ensure_rule_condition_library_table(&self.pool).await?;
        let result =
            sqlx::query("UPDATE rules SET status = $1, updated_at = now() WHERE rule_key = $2")
                .bind(status)
                .bind(rule_id)
                .execute(&self.pool)
                .await?;
        if result.rows_affected() == 0 {
            return Ok(None);
        }
        sqlx::query(
            "UPDATE rule_condition_library
             SET status = $1, updated_at = now()
             WHERE source_rule_key = $2",
        )
        .bind(rule_condition_status(status))
        .bind(rule_id)
        .execute(&self.pool)
        .await?;
        Ok(self
            .list_rules()
            .await?
            .into_iter()
            .find(|rule| rule.rule_id == rule_id))
    }

    async fn list_rule_conditions(&self) -> anyhow::Result<Vec<RuleConditionLibraryRecord>> {
        ensure_default_rules_seeded(&self.pool).await?;
        ensure_rule_condition_library_table(&self.pool).await?;
        let rows: Vec<(
            String,
            String,
            i32,
            i32,
            String,
            String,
            Value,
            String,
            String,
            String,
            String,
            Value,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT condition_key, source_rule_key, source_rule_version, condition_index,
                    field_name, operator, value, review_mode, scheme_family, status, owner,
                    evidence_refs, created_at, updated_at
             FROM rule_condition_library
             ORDER BY source_rule_key, source_rule_version, condition_index",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    condition_key,
                    source_rule_key,
                    source_rule_version,
                    condition_index,
                    field,
                    operator,
                    value,
                    review_mode,
                    scheme_family,
                    status,
                    owner,
                    evidence_refs,
                    created_at,
                    updated_at,
                )| RuleConditionLibraryRecord {
                    condition_key,
                    source_rule_key,
                    source_rule_version: source_rule_version.max(0) as u32,
                    condition_index: condition_index.max(0) as u32,
                    field,
                    operator,
                    value,
                    review_mode,
                    scheme_family,
                    status,
                    owner,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: Some(created_at.to_rfc3339()),
                    updated_at: Some(updated_at.to_rfc3339()),
                },
            )
            .collect())
    }

    async fn rule_performance(&self) -> anyhow::Result<Vec<RulePerformanceRecord>> {
        ensure_default_rules_seeded(&self.pool).await?;
        let rules = self.list_rules().await?;
        let total_runs: (i64,) =
            sqlx::query_as("SELECT COUNT(*)::bigint FROM scoring_runs WHERE status = 'succeeded'")
                .fetch_one(&self.pool)
                .await?;

        let rule_run_rows: Vec<(Option<String>, Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT r.rule_key, rr.alert_code, c.external_claim_id
             FROM rule_runs rr
             JOIN scoring_runs sr ON sr.run_id = rr.run_id
             LEFT JOIN rules r ON r.id = rr.rule_id
             LEFT JOIN claims c ON c.id = sr.claim_id
             WHERE rr.matched = true",
        )
        .fetch_all(&self.pool)
        .await?;

        let outcome_rows: Vec<(String, bool, Option<Decimal>)> = sqlx::query_as(
            "SELECT claim_id, confirmed_fwa, saving_amount
             FROM investigation_results",
        )
        .fetch_all(&self.pool)
        .await?;
        let outcomes = outcome_rows
            .into_iter()
            .map(|(claim_id, confirmed_fwa, saving_amount)| {
                (
                    claim_id,
                    InvestigationOutcome {
                        confirmed_fwa,
                        saving_amount: saving_amount.unwrap_or(Decimal::ZERO),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        let alert_to_rule = rules
            .iter()
            .map(|rule| (rule.alert_code.clone(), rule.rule_id.clone()))
            .collect::<HashMap<_, _>>();
        let mut accumulators = rule_accumulators_from_rules(&rules);
        for (rule_id, alert_code, claim_id) in rule_run_rows {
            let rule_id = rule_id.or_else(|| {
                alert_code
                    .as_ref()
                    .and_then(|alert_code| alert_to_rule.get(alert_code).cloned())
            });
            let (Some(rule_id), Some(claim_id)) = (rule_id, claim_id) else {
                continue;
            };
            let Some(accumulator) = accumulators.get_mut(&rule_id) else {
                continue;
            };
            accumulator.trigger_count += 1;
            accumulator.triggered_claim_ids.insert(claim_id);
        }

        Ok(rule_performance_records(
            accumulators,
            &outcomes,
            total_runs.0.max(0) as u32,
        ))
    }

    async fn save_rule_backtest(
        &self,
        record: RuleBacktestRecord,
    ) -> anyhow::Result<RuleBacktestRecord> {
        let row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
            "INSERT INTO rule_backtest_runs
             (rule_id, rule_version, sample_count, matched_count, reviewed_count,
              confirmed_fwa_count, false_positive_count, precision_value, recall_value,
              lift, false_positive_rate, estimated_saving, promotion_recommendation,
              blockers, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             RETURNING created_at",
        )
        .bind(&record.rule_id)
        .bind(record.rule_version as i32)
        .bind(record.sample_count as i32)
        .bind(record.matched_count as i32)
        .bind(record.reviewed_count as i32)
        .bind(record.confirmed_fwa_count as i32)
        .bind(record.false_positive_count as i32)
        .bind(record.precision)
        .bind(record.recall)
        .bind(record.lift)
        .bind(record.false_positive_rate)
        .bind(&record.estimated_saving)
        .bind(&record.promotion_recommendation)
        .bind(serde_json::json!(record.blockers))
        .bind(serde_json::json!(record.evidence_refs))
        .fetch_one(&self.pool)
        .await?;
        Ok(RuleBacktestRecord {
            created_at: Some(row.0.to_rfc3339()),
            ..record
        })
    }

    async fn latest_rule_backtest(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleBacktestRecord>> {
        let row: Option<(
            String,
            i32,
            i32,
            i32,
            i32,
            i32,
            i32,
            f64,
            f64,
            f64,
            f64,
            String,
            String,
            Value,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT rule_id, rule_version, sample_count, matched_count, reviewed_count,
                    confirmed_fwa_count, false_positive_count, precision_value, recall_value,
                    lift, false_positive_rate, estimated_saving, promotion_recommendation,
                    blockers, evidence_refs, created_at
             FROM rule_backtest_runs
             WHERE rule_id = $1 AND rule_version = $2
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
        )
        .bind(rule_id)
        .bind(rule_version as i32)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(
                rule_id,
                rule_version,
                sample_count,
                matched_count,
                reviewed_count,
                confirmed_fwa_count,
                false_positive_count,
                precision,
                recall,
                lift,
                false_positive_rate,
                estimated_saving,
                promotion_recommendation,
                blockers,
                evidence_refs,
                created_at,
            )| RuleBacktestRecord {
                rule_id,
                rule_version: rule_version as u32,
                sample_count: sample_count.max(0) as u32,
                matched_count: matched_count.max(0) as u32,
                reviewed_count: reviewed_count.max(0) as u32,
                confirmed_fwa_count: confirmed_fwa_count.max(0) as u32,
                false_positive_count: false_positive_count.max(0) as u32,
                precision,
                recall,
                lift,
                false_positive_rate,
                estimated_saving,
                promotion_recommendation,
                blockers: json_array_to_strings(blockers),
                evidence_refs: json_array_to_strings(evidence_refs),
                created_at: Some(created_at.to_rfc3339()),
            },
        ))
    }

    async fn save_rule_shadow_run(
        &self,
        record: RuleShadowRunRecord,
    ) -> anyhow::Result<RuleShadowRunRecord> {
        let row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
            "INSERT INTO rule_shadow_runs
             (rule_id, rule_version, report_uri, decision, reviewer, notes,
              reviewed_count, matched_count, false_positive_count, false_positive_rate,
              blockers, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             RETURNING created_at",
        )
        .bind(&record.rule_id)
        .bind(record.rule_version as i32)
        .bind(&record.report_uri)
        .bind(&record.decision)
        .bind(&record.reviewer)
        .bind(&record.notes)
        .bind(record.reviewed_count as i32)
        .bind(record.matched_count as i32)
        .bind(record.false_positive_count as i32)
        .bind(record.false_positive_rate)
        .bind(serde_json::json!(record.blockers.clone()))
        .bind(serde_json::json!(record.evidence_refs.clone()))
        .fetch_one(&self.pool)
        .await?;
        Ok(RuleShadowRunRecord {
            created_at: Some(row.0.to_rfc3339()),
            ..record
        })
    }

    async fn latest_rule_shadow_run(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleShadowRunRecord>> {
        let row: Option<(
            String,
            i32,
            String,
            String,
            String,
            String,
            i32,
            i32,
            i32,
            f64,
            Value,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT rule_id, rule_version, report_uri, decision, reviewer, notes,
                    reviewed_count, matched_count, false_positive_count, false_positive_rate,
                    blockers, evidence_refs, created_at
             FROM rule_shadow_runs
             WHERE rule_id = $1 AND rule_version = $2
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
        )
        .bind(rule_id)
        .bind(rule_version as i32)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(
                rule_id,
                rule_version,
                report_uri,
                decision,
                reviewer,
                notes,
                reviewed_count,
                matched_count,
                false_positive_count,
                false_positive_rate,
                blockers,
                evidence_refs,
                created_at,
            )| RuleShadowRunRecord {
                rule_id,
                rule_version: rule_version as u32,
                report_uri,
                decision,
                reviewer,
                notes,
                reviewed_count: reviewed_count.max(0) as u32,
                matched_count: matched_count.max(0) as u32,
                false_positive_count: false_positive_count.max(0) as u32,
                false_positive_rate,
                blockers: json_array_to_strings(blockers),
                evidence_refs: json_array_to_strings(evidence_refs),
                created_at: Some(created_at.to_rfc3339()),
            },
        ))
    }

    async fn save_rule_promotion_review(
        &self,
        record: RulePromotionReviewRecord,
    ) -> anyhow::Result<RulePromotionReviewRecord> {
        let row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
            "INSERT INTO rule_promotion_reviews
             (rule_id, rule_version, decision, reviewer, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING created_at",
        )
        .bind(&record.rule_id)
        .bind(record.rule_version as i32)
        .bind(&record.decision)
        .bind(&record.reviewer)
        .bind(&record.notes)
        .bind(serde_json::json!(record.evidence_refs.clone()))
        .fetch_one(&self.pool)
        .await?;
        Ok(RulePromotionReviewRecord {
            created_at: Some(row.0.to_rfc3339()),
            ..record
        })
    }

    async fn latest_rule_promotion_review(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RulePromotionReviewRecord>> {
        let row: Option<(
            String,
            i32,
            String,
            String,
            String,
            serde_json::Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT rule_id, rule_version, decision, reviewer, notes, evidence_refs, created_at
                 FROM rule_promotion_reviews
                 WHERE rule_id = $1 AND rule_version = $2
                 ORDER BY created_at DESC
                 LIMIT 1",
        )
        .bind(rule_id)
        .bind(rule_version as i32)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(rule_id, rule_version, decision, reviewer, notes, evidence_refs, created_at)| {
                RulePromotionReviewRecord {
                    rule_id,
                    rule_version: rule_version as u32,
                    decision,
                    reviewer,
                    notes,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: Some(created_at.to_rfc3339()),
                }
            },
        ))
    }

    async fn list_leads(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<LeadRecord>> {
        postgres_cases::list_leads(self, customer_scope_id).await
    }

    async fn triage_lead(
        &self,
        lead_id: &str,
        input: TriageLeadInput,
    ) -> anyhow::Result<Option<TriageLeadRecord>> {
        postgres_cases::triage_lead(self, lead_id, input).await
    }

    async fn list_cases(&self, customer_scope_id: Option<&str>) -> anyhow::Result<Vec<CaseRecord>> {
        postgres_cases::list_cases(self, customer_scope_id).await
    }

    async fn update_case_status(
        &self,
        case_id: &str,
        input: UpdateCaseStatusInput,
    ) -> anyhow::Result<Option<UpdateCaseStatusRecord>> {
        postgres_cases::update_case_status(self, case_id, input).await
    }

    async fn create_audit_sample(
        &self,
        input: CreateAuditSampleInput,
    ) -> anyhow::Result<AuditSampleRecord> {
        postgres_audit_samples::create_audit_sample(self, input).await
    }

    async fn list_audit_samples(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AuditSampleRecord>> {
        postgres_audit_samples::list_audit_samples(self, customer_scope_id).await
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
            String,
            Option<String>,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT model_key, version, model_type, runtime_kind, execution_provider, status, COALESCE(metrics ->> 'review_mode', 'both'), artifact_uri, endpoint_url
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
                    review_mode,
                    artifact_uri,
                    endpoint_url,
                )| ModelVersionRecord {
                    model_key,
                    version,
                    model_type,
                    runtime_kind,
                    execution_provider,
                    status,
                    review_mode: normalize_review_mode(&review_mode),
                    artifact_uri,
                    endpoint_url,
                },
            )
            .collect())
    }

    async fn save_model_version(
        &self,
        record: ModelVersionRecord,
    ) -> anyhow::Result<ModelVersionRecord> {
        sqlx::query(
            "INSERT INTO model_versions
             (model_key, version, model_type, runtime_kind, artifact_uri, endpoint_url, execution_provider, status, metrics, activated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, CASE WHEN $8 = 'active' THEN now() ELSE NULL END)
             ON CONFLICT (model_key, version) DO UPDATE
             SET model_type = EXCLUDED.model_type,
                 runtime_kind = EXCLUDED.runtime_kind,
                 artifact_uri = EXCLUDED.artifact_uri,
                 endpoint_url = EXCLUDED.endpoint_url,
                 execution_provider = EXCLUDED.execution_provider,
                 status = EXCLUDED.status,
                 metrics = model_versions.metrics || EXCLUDED.metrics,
                 activated_at = CASE WHEN EXCLUDED.status = 'active' THEN now() ELSE model_versions.activated_at END",
        )
        .bind(&record.model_key)
        .bind(&record.version)
        .bind(&record.model_type)
        .bind(&record.runtime_kind)
        .bind(&record.artifact_uri)
        .bind(&record.endpoint_url)
        .bind(&record.execution_provider)
        .bind(&record.status)
        .bind(serde_json::json!({ "review_mode": record.review_mode }))
        .execute(&self.pool)
        .await?;
        Ok(record)
    }

    async fn update_model_status(
        &self,
        model_key: &str,
        model_version: &str,
        status: &str,
    ) -> anyhow::Result<Option<ModelVersionRecord>> {
        ensure_default_models_seeded(&self.pool).await?;
        let row: Option<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
        )> = sqlx::query_as(
            "UPDATE model_versions
             SET status = $3,
                 activated_at = CASE WHEN $3 = 'active' THEN now() ELSE NULL END
             WHERE model_key = $1 AND version = $2
             RETURNING model_key, version, model_type, runtime_kind, execution_provider, status, COALESCE(metrics ->> 'review_mode', 'both'), artifact_uri, endpoint_url",
        )
        .bind(model_key)
        .bind(model_version)
        .bind(status)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(
                model_key,
                version,
                model_type,
                runtime_kind,
                execution_provider,
                status,
                review_mode,
                artifact_uri,
                endpoint_url,
            )| ModelVersionRecord {
                model_key,
                version,
                model_type,
                runtime_kind,
                execution_provider,
                status,
                review_mode: normalize_review_mode(&review_mode),
                artifact_uri,
                endpoint_url,
            },
        ))
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
        let drift_metrics: Option<(Value,)> = sqlx::query_as(
            "SELECT metrics_json
             FROM model_evaluation_runs
             WHERE model_key = $1
             ORDER BY created_at DESC, evaluation_run_id DESC
             LIMIT 1",
        )
        .bind(model_key)
        .fetch_optional(&self.pool)
        .await?;
        let drift = drift_summary(
            drift_metrics
                .as_ref()
                .map(|row| &row.0)
                .unwrap_or(&Value::Null),
        );

        let scored_runs = row.0 as u32;
        if scored_runs == 0 {
            return Ok(Some(model_performance_with_drift(
                empty_model_performance(model_key),
                drift,
            )));
        }

        Ok(Some(model_performance_with_drift(
            ModelPerformanceRecord {
                model_key: model_key.to_string(),
                data_status: "ready".into(),
                scored_runs,
                average_score: row
                    .1
                    .map(|value| value.to_string().parse().unwrap_or(0.0))
                    .unwrap_or(0.0),
                high_risk_count: row.2.unwrap_or(0) as u32,
                score_psi: None,
                drift_status: "not_available".into(),
                latest_scored_at: row.3.map(|timestamp| timestamp.to_rfc3339()),
            },
            drift,
        )))
    }

    async fn save_model_promotion_review(
        &self,
        record: ModelPromotionReviewRecord,
    ) -> anyhow::Result<ModelPromotionReviewRecord> {
        let row: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
            "INSERT INTO model_promotion_reviews
             (model_key, model_version, decision, reviewer, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING created_at",
        )
        .bind(&record.model_key)
        .bind(&record.model_version)
        .bind(&record.decision)
        .bind(&record.reviewer)
        .bind(&record.notes)
        .bind(serde_json::json!(record.evidence_refs.clone()))
        .fetch_one(&self.pool)
        .await?;
        Ok(ModelPromotionReviewRecord {
            created_at: Some(row.0.to_rfc3339()),
            ..record
        })
    }

    async fn latest_model_promotion_review(
        &self,
        model_key: &str,
        model_version: &str,
    ) -> anyhow::Result<Option<ModelPromotionReviewRecord>> {
        let row: Option<(
            String,
            String,
            String,
            String,
            String,
            serde_json::Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT model_key, model_version, decision, reviewer, notes, evidence_refs, created_at
                 FROM model_promotion_reviews
                 WHERE model_key = $1 AND model_version = $2
                 ORDER BY created_at DESC
                 LIMIT 1",
        )
        .bind(model_key)
        .bind(model_version)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(
            |(model_key, model_version, decision, reviewer, notes, evidence_refs, created_at)| {
                ModelPromotionReviewRecord {
                    model_key,
                    model_version,
                    decision,
                    reviewer,
                    notes,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: Some(created_at.to_rfc3339()),
                }
            },
        ))
    }

    async fn save_model_retraining_job(
        &self,
        record: ModelRetrainingJobRecord,
    ) -> anyhow::Result<ModelRetrainingJobRecord> {
        let row: (
            String,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        ) = sqlx::query_as(
            "INSERT INTO model_retraining_jobs
                 (model_key, model_version, status, requested_by, request_notes, status_note,
                  updated_by, readiness_recommendation, latest_evaluation_id, source_dataset_id,
                  source_data_quality_score, source_data_quality_status, trigger_summary_json,
                  blocker_summary_json)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                 RETURNING id::text, created_at, updated_at",
        )
        .bind(&record.model_key)
        .bind(&record.model_version)
        .bind(&record.status)
        .bind(&record.requested_by)
        .bind(&record.request_notes)
        .bind(&record.status_note)
        .bind(&record.updated_by)
        .bind(&record.readiness_recommendation)
        .bind(&record.latest_evaluation_id)
        .bind(&record.source_dataset_id)
        .bind(record.source_data_quality_score)
        .bind(&record.source_data_quality_status)
        .bind(serde_json::json!(record.trigger_summary))
        .bind(serde_json::json!(record.blocker_summary))
        .fetch_one(&self.pool)
        .await?;
        Ok(ModelRetrainingJobRecord {
            job_id: row.0,
            created_at: Some(row.1.to_rfc3339()),
            updated_at: Some(row.2.to_rfc3339()),
            ..record
        })
    }

    async fn list_model_retraining_jobs(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Vec<ModelRetrainingJobRecord>> {
        let rows = sqlx::query(
            "SELECT id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                    status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                    source_dataset_id, source_data_quality_score, source_data_quality_status,
                    trigger_summary_json, blocker_summary_json, candidate_model_version,
                    candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                    output_evaluation_id, created_at, updated_at
             FROM model_retraining_jobs
             WHERE model_key = $1
             ORDER BY created_at DESC",
        )
        .bind(model_key)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(model_retraining_job_from_pg_row)
            .collect())
    }

    async fn get_model_retraining_job(
        &self,
        job_id: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let row = sqlx::query(
            "SELECT id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                    status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                    source_dataset_id, source_data_quality_score, source_data_quality_status,
                    trigger_summary_json, blocker_summary_json, candidate_model_version,
                    candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                    output_evaluation_id, created_at, updated_at
             FROM model_retraining_jobs
             WHERE id = $1::uuid",
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(model_retraining_job_from_pg_row))
    }

    async fn claim_next_model_retraining_job(
        &self,
        model_key: Option<&str>,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let row = sqlx::query(
            "WITH next_job AS (
                 SELECT id
                 FROM model_retraining_jobs
                 WHERE status = 'queued'
                   AND ($3::text IS NULL OR model_key = $3)
                 ORDER BY created_at ASC
                 LIMIT 1
                 FOR UPDATE SKIP LOCKED
             )
             UPDATE model_retraining_jobs
             SET status = 'running', updated_by = $1, status_note = $2, updated_at = now()
             WHERE id = (SELECT id FROM next_job)
             RETURNING id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                       status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                       source_dataset_id, source_data_quality_score, source_data_quality_status,
                       trigger_summary_json, blocker_summary_json, candidate_model_version,
                       candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                       output_evaluation_id, created_at, updated_at",
        )
        .bind(actor)
        .bind(status_note)
        .bind(model_key)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(model_retraining_job_from_pg_row))
    }

    async fn update_model_retraining_job_status(
        &self,
        job_id: &str,
        status: &str,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let row = sqlx::query(
            "UPDATE model_retraining_jobs
             SET status = $2, updated_by = $3, status_note = $4, updated_at = now()
             WHERE id = $1::uuid
             RETURNING id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                       status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                       source_dataset_id, source_data_quality_score, source_data_quality_status,
                       trigger_summary_json, blocker_summary_json, candidate_model_version,
                       candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                       output_evaluation_id, created_at, updated_at",
        )
        .bind(job_id)
        .bind(status)
        .bind(actor)
        .bind(status_note)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(model_retraining_job_from_pg_row))
    }

    async fn complete_model_retraining_job(
        &self,
        input: CompleteModelRetrainingJobInput<'_>,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        let row = sqlx::query(
            "UPDATE model_retraining_jobs
             SET status = 'completed',
                 updated_by = $2,
                 status_note = $3,
                 candidate_model_version = $4,
                 candidate_artifact_uri = $5,
                 candidate_endpoint_url = $6,
                 validation_report_uri = $7,
                 output_evaluation_id = $8,
                 updated_at = now()
             WHERE id = $1::uuid
             RETURNING id::text AS job_id, model_key, model_version, status, requested_by, request_notes,
                       status_note, updated_by, readiness_recommendation, latest_evaluation_id,
                       source_dataset_id, source_data_quality_score, source_data_quality_status,
                       trigger_summary_json, blocker_summary_json, candidate_model_version,
                       candidate_artifact_uri, candidate_endpoint_url, validation_report_uri,
                       output_evaluation_id, created_at, updated_at",
        )
        .bind(input.job_id)
        .bind(input.actor)
        .bind(input.status_note)
        .bind(input.candidate_model_version)
        .bind(input.candidate_artifact_uri)
        .bind(input.candidate_endpoint_url)
        .bind(input.validation_report_uri)
        .bind(input.output_evaluation_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(model_retraining_job_from_pg_row))
    }

    async fn dashboard_summary(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<DashboardSummaryRecord> {
        let suspected: (i64, Option<Decimal>) = sqlx::query_as(
            "SELECT COUNT(*)::bigint, COALESCE(SUM(c.claim_amount), 0)
             FROM scoring_runs sr
             LEFT JOIN claims c ON c.id = sr.claim_id
             WHERE sr.risk_score >= 70
               AND ($1::text IS NULL OR EXISTS (
                 SELECT 1 FROM audit_events ae
                 WHERE ae.run_id = sr.run_id
                   AND ae.event_type = 'scoring.completed'
                   AND ae.event_status = 'succeeded'
                   AND ae.payload ->> 'customer_scope_id' = $1
               ))",
        )
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;

        let rag_rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT COALESCE(rag, 'UNKNOWN'), COUNT(*)::bigint
             FROM scoring_runs sr
             WHERE rag IS NOT NULL
               AND ($1::text IS NULL OR EXISTS (
                 SELECT 1 FROM audit_events ae
                 WHERE ae.run_id = sr.run_id
                   AND ae.event_type = 'scoring.completed'
                   AND ae.event_status = 'succeeded'
                   AND ae.payload ->> 'customer_scope_id' = $1
               ))
             GROUP BY rag
             ORDER BY rag",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;

        let rule_hits: (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint
             FROM rule_runs rr
             JOIN scoring_runs sr ON sr.run_id = rr.run_id
             WHERE rr.matched = true
               AND ($1::text IS NULL OR EXISTS (
                 SELECT 1 FROM audit_events ae
                 WHERE ae.run_id = sr.run_id
                   AND ae.event_type = 'scoring.completed'
                   AND ae.event_status = 'succeeded'
                   AND ae.payload ->> 'customer_scope_id' = $1
               ))",
        )
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;

        let model_rows: Vec<(String, i64, Option<Decimal>, Option<i64>)> = sqlx::query_as(
            "SELECT model_key,
                    COUNT(*)::bigint,
                    AVG(score),
                    SUM(CASE WHEN score >= 70 THEN 1 ELSE 0 END)::bigint
             FROM model_scores ms
             JOIN scoring_runs sr ON sr.run_id = ms.run_id
             WHERE ($1::text IS NULL OR EXISTS (
                 SELECT 1 FROM audit_events ae
                 WHERE ae.run_id = sr.run_id
                   AND ae.event_type = 'scoring.completed'
                   AND ae.event_status = 'succeeded'
                   AND ae.payload ->> 'customer_scope_id' = $1
               ))
             GROUP BY model_key
             ORDER BY model_key",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;

        let layer_payloads: Vec<(Value,)> = sqlx::query_as(
            "SELECT payload
             FROM audit_events
             WHERE event_type = 'scoring.completed'
               AND event_status = 'succeeded'
               AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        let audit_coverage_row: (i64, Option<i64>) = sqlx::query_as(
            "SELECT COUNT(*)::bigint,
                    SUM(
                        CASE
                            WHEN jsonb_typeof(payload->'canonical_claim_context_trace') = 'object'
                            THEN 1
                            ELSE 0
                        END
                    )::bigint
             FROM audit_events
             WHERE event_type = 'scoring.completed'
               AND event_status = 'succeeded'
               AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)",
        )
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;
        let audit_coverage = summarize_dashboard_audit_coverage(
            audit_coverage_row.0 as u32,
            audit_coverage_row.1.unwrap_or(0) as u32,
        );
        let mut layer_accumulators = BTreeMap::<String, (String, u32, u32, u32)>::new();
        for (payload,) in layer_payloads {
            for layer in payload
                .get("layers")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
            {
                let layer_id = layer["layer_id"].as_str().unwrap_or("UNKNOWN").to_string();
                let layer_name = layer["name"].as_str().unwrap_or("Unknown").to_string();
                let layer_score = layer["score"].as_u64().unwrap_or(0) as u32;
                let entry =
                    layer_accumulators
                        .entry(layer_id)
                        .or_insert((layer_name.clone(), 0, 0, 0));
                entry.0 = layer_name;
                entry.1 += 1;
                entry.2 += layer_score;
                if layer_score >= 70 {
                    entry.3 += 1;
                }
            }
        }

        let investigation: (i64, i64, Option<Decimal>) = sqlx::query_as(
            "SELECT COUNT(*)::bigint,
                    COALESCE(SUM(CASE WHEN confirmed_fwa THEN 1 ELSE 0 END), 0)::bigint,
                    COALESCE(SUM(saving_amount), 0)
             FROM investigation_results ir
             WHERE ($1::text IS NULL OR EXISTS (
               SELECT 1 FROM audit_events ae
               WHERE ae.event_type = 'investigation.result.received'
                 AND ae.event_status = 'succeeded'
                 AND ae.payload ->> 'investigation_id' = ir.investigation_id
                 AND ae.payload ->> 'customer_scope_id' = $1
             ))",
        )
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;

        let qa_reviews: (i64,) = sqlx::query_as(
            "SELECT COUNT(*)::bigint
             FROM qa_reviews qr
             WHERE ($1::text IS NULL OR EXISTS (
               SELECT 1 FROM audit_events ae
               WHERE ae.event_type = 'qa.result.received'
                 AND ae.event_status = 'succeeded'
                 AND ae.payload ->> 'qa_case_id' = qr.qa_case_id
                 AND ae.payload ->> 'customer_scope_id' = $1
             ))",
        )
        .bind(customer_scope_id)
        .fetch_one(&self.pool)
        .await?;

        let scheme_rows: Vec<(String, i64)> = sqlx::query_as(
            "SELECT scheme_family, COUNT(*)::bigint
             FROM fwa_leads l
             WHERE ($1::text IS NULL OR EXISTS (
               SELECT 1 FROM audit_events ae
               WHERE ae.run_id = l.run_id
                 AND ae.event_type = 'scoring.completed'
                 AND ae.event_status = 'succeeded'
                 AND ae.payload ->> 'customer_scope_id' = $1
             ))
             GROUP BY scheme_family
             ORDER BY scheme_family",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;

        let financial_impact_rows: Vec<(bool, Option<String>, Option<Decimal>, Option<String>)> =
            sqlx::query_as(
                "SELECT confirmed_fwa, financial_impact_type, saving_amount, currency
                 FROM investigation_results ir
                 WHERE ($1::text IS NULL OR EXISTS (
                   SELECT 1 FROM audit_events ae
                   WHERE ae.event_type = 'investigation.result.received'
                     AND ae.event_status = 'succeeded'
                     AND ae.payload ->> 'investigation_id' = ir.investigation_id
                     AND ae.payload ->> 'customer_scope_id' = $1
                 ))
                 ORDER BY created_at, investigation_id",
            )
            .bind(customer_scope_id)
            .fetch_all(&self.pool)
            .await?;
        let financial_impacts = financial_impact_rows
            .into_iter()
            .filter_map(
                |(confirmed_fwa, financial_impact_type, saving_amount, currency)| {
                    financial_impact_from_parts(
                        confirmed_fwa,
                        financial_impact_type.as_deref(),
                        saving_amount,
                        currency,
                    )
                },
            )
            .collect::<Vec<_>>();

        let saving_attributions: Vec<(
            String,
            String,
            String,
            String,
            Option<Decimal>,
            String,
            i64,
            Vec<String>,
        )> = sqlx::query_as(
            "SELECT source_type,
                        source_id,
                        financial_impact_type,
                        action,
                        COALESCE(SUM(saving_amount), 0),
                        currency,
                        COUNT(DISTINCT claim_id)::bigint,
                        ARRAY_REMOVE(ARRAY_AGG(DISTINCT ref.value ORDER BY ref.value), NULL)
                 FROM saving_attributions s
                 LEFT JOIN LATERAL jsonb_array_elements_text(s.evidence_refs) AS ref(value) ON TRUE
                 WHERE ($1::text IS NULL OR EXISTS (
                   SELECT 1 FROM audit_events ae
                   WHERE ae.event_type = 'investigation.result.received'
                     AND ae.event_status = 'succeeded'
                     AND ae.payload ->> 'investigation_id' = s.investigation_id
                     AND ae.payload ->> 'customer_scope_id' = $1
                 ))
                 GROUP BY source_type, source_id, financial_impact_type, action, currency
                 ORDER BY source_type, source_id, financial_impact_type, action, currency",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        let saving_segments: Vec<(String, String, Option<Decimal>, String, i64, i64)> =
            sqlx::query_as(
                "SELECT segment_type,
                        segment_id,
                        COALESCE(SUM(saving_amount), 0),
                        currency,
                        COUNT(DISTINCT claim_id)::bigint,
                        COUNT(*)::bigint
                 FROM (
                   SELECT 'provider'::text AS segment_type,
                          COALESCE(l.provider_id, 'unknown') AS segment_id,
                          s.saving_amount,
                          s.currency,
                          s.claim_id
                   FROM saving_attributions s
                   LEFT JOIN fwa_leads l ON l.claim_id = s.claim_id
                   WHERE ($1::text IS NULL OR EXISTS (
                     SELECT 1 FROM audit_events ae
                     WHERE ae.event_type = 'investigation.result.received'
                       AND ae.event_status = 'succeeded'
                       AND ae.payload ->> 'investigation_id' = s.investigation_id
                       AND ae.payload ->> 'customer_scope_id' = $1
                   ))
                   UNION ALL
                   SELECT 'scheme'::text AS segment_type,
                          COALESCE(l.scheme_family, 'unknown') AS segment_id,
                          s.saving_amount,
                          s.currency,
                          s.claim_id
                   FROM saving_attributions s
                   LEFT JOIN fwa_leads l ON l.claim_id = s.claim_id
                   WHERE ($1::text IS NULL OR EXISTS (
                     SELECT 1 FROM audit_events ae
                     WHERE ae.event_type = 'investigation.result.received'
                       AND ae.event_status = 'succeeded'
                       AND ae.payload ->> 'investigation_id' = s.investigation_id
                       AND ae.payload ->> 'customer_scope_id' = $1
                   ))
                   UNION ALL
                   SELECT 'campaign'::text AS segment_type,
                          COALESCE(NULLIF(regexp_replace(ref.value, '^campaigns?:', ''), ''), 'unknown') AS segment_id,
                          s.saving_amount,
                          s.currency,
                          s.claim_id
                   FROM saving_attributions s
                   CROSS JOIN LATERAL jsonb_array_elements_text(s.evidence_refs) AS ref(value)
                   WHERE (ref.value LIKE 'campaign:%'
                      OR ref.value LIKE 'campaigns:%')
                     AND ($1::text IS NULL OR EXISTS (
                       SELECT 1 FROM audit_events ae
                       WHERE ae.event_type = 'investigation.result.received'
                         AND ae.event_status = 'succeeded'
                         AND ae.payload ->> 'investigation_id' = s.investigation_id
                         AND ae.payload ->> 'customer_scope_id' = $1
                     ))
                 ) segments
                 GROUP BY segment_type, segment_id, currency
                 ORDER BY segment_type, segment_id, currency",
            )
            .bind(customer_scope_id)
            .fetch_all(&self.pool)
            .await?;
        let outcome_labels = self.list_outcome_labels(customer_scope_id).await?;
        let audit_samples = self.list_audit_samples(customer_scope_id).await?;
        let qa_review_records = self.list_qa_reviews(customer_scope_id).await?;
        let qa_feedback_items = self.list_qa_feedback_items(customer_scope_id).await?;
        let agent_runs = self.list_agent_runs(customer_scope_id).await?;
        let models = self.list_models().await?;
        let model_evaluations = self.list_model_evaluations().await?;
        let rules = self.list_rules().await?;
        let rule_performance = self.rule_performance().await?;

        Ok(DashboardSummaryRecord {
            suspected_claims: suspected.0 as u32,
            confirmed_fwa: investigation.1 as u32,
            risk_amount: suspected.1.unwrap_or(Decimal::ZERO).to_string(),
            saving_amount: investigation.2.unwrap_or(Decimal::ZERO).to_string(),
            rag_distribution: rag_rows
                .into_iter()
                .map(|(rag, count)| (rag, count as u32))
                .collect(),
            scheme_distribution: scheme_rows
                .into_iter()
                .map(|(scheme_family, count)| (scheme_family, count as u32))
                .collect(),
            rule_hits: rule_hits.0 as u32,
            model_scores: model_rows
                .into_iter()
                .map(|(model_key, scored_runs, average_score, high_risk_count)| {
                    (
                        model_key,
                        DashboardModelScoreRecord {
                            scored_runs: scored_runs as u32,
                            average_score: average_score
                                .map(|value| value.to_string().parse().unwrap_or(0.0))
                                .unwrap_or(0.0),
                            high_risk_count: high_risk_count.unwrap_or(0) as u32,
                        },
                    )
                })
                .collect(),
            layer_scores: layer_accumulators
                .into_iter()
                .map(
                    |(layer_id, (name, scored_runs, score_sum, high_risk_count))| {
                        let average_score = if scored_runs == 0 {
                            0.0
                        } else {
                            score_sum as f64 / scored_runs as f64
                        };
                        (
                            layer_id,
                            DashboardLayerScoreRecord {
                                name,
                                scored_runs,
                                average_score,
                                high_risk_count,
                            },
                        )
                    },
                )
                .collect(),
            saving_attributions: saving_attributions
                .into_iter()
                .map(
                    |(
                        source_type,
                        source_id,
                        financial_impact_type,
                        action,
                        saving_amount,
                        currency,
                        claim_count,
                        evidence_refs,
                    )| {
                        DashboardSavingAttributionRecord {
                            source_type,
                            source_id,
                            financial_impact_type,
                            action,
                            saving_amount: format_decimal_cents(
                                saving_amount.unwrap_or(Decimal::ZERO),
                            ),
                            currency,
                            claim_count: claim_count as u32,
                            evidence_refs,
                        }
                    },
                )
                .collect(),
            saving_segments: saving_segments
                .into_iter()
                .map(
                    |(
                        segment_type,
                        segment_id,
                        saving_amount,
                        currency,
                        claim_count,
                        attribution_count,
                    )| {
                        let saving_amount = saving_amount.unwrap_or(Decimal::ZERO);
                        let claim_count = claim_count as u32;
                        DashboardSavingSegmentRecord {
                            segment_type,
                            segment_id,
                            saving_amount: format_decimal_cents(saving_amount),
                            currency,
                            claim_count,
                            attribution_count: attribution_count as u32,
                            roi: segment_roi(saving_amount, claim_count),
                        }
                    },
                )
                .collect(),
            value_measurement: summarize_dashboard_value_measurement(
                &financial_impacts,
                rule_hits.0 as u32,
                rule_performance
                    .iter()
                    .map(|record| record.false_positive_count)
                    .sum::<u32>(),
            ),
            audit_coverage,
            label_pool: summarize_dashboard_label_pool(&outcome_labels),
            qa_queue: summarize_dashboard_qa_queue(
                &audit_samples,
                &qa_review_records,
                &qa_feedback_items,
            ),
            case_sla: summarize_dashboard_case_sla(&self.list_cases(customer_scope_id).await?),
            agent_governance: summarize_dashboard_agent_governance(&agent_runs),
            model_governance: summarize_dashboard_model_governance(&models, &model_evaluations),
            rule_governance: summarize_dashboard_rule_governance(&rules, &rule_performance),
            investigation_results: investigation.0 as u32,
            qa_reviews: qa_reviews.0 as u32,
        })
    }

    async fn provider_risk_summary(&self) -> anyhow::Result<ProviderRiskSummaryRecord> {
        let rows: Vec<(Value,)> = sqlx::query_as(
            "SELECT payload
             FROM audit_events
             WHERE event_type = 'scoring.completed'
               AND event_status = 'succeeded'",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(summarize_provider_risk_profiles(
            rows.iter().map(|(payload,)| payload),
        ))
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
            String,
            Value,
            Value,
        )> = sqlx::query_as(
            "SELECT case_id, title, fwa_type, scheme_family, diagnosis_code, provider_region, provider_type, summary, outcome, tags, evidence_refs
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
                    scheme_family,
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
                    scheme_family,
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

    async fn save_knowledge_case(
        &self,
        record: KnowledgeCaseRecord,
    ) -> anyhow::Result<KnowledgeCaseRecord> {
        sqlx::query(
            "INSERT INTO knowledge_cases
             (case_id, title, fwa_type, scheme_family, diagnosis_code, provider_region, provider_type, summary, outcome, tags, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             ON CONFLICT (case_id) DO UPDATE
             SET title = EXCLUDED.title,
                 fwa_type = EXCLUDED.fwa_type,
                 scheme_family = EXCLUDED.scheme_family,
                 diagnosis_code = EXCLUDED.diagnosis_code,
                 provider_region = EXCLUDED.provider_region,
                 provider_type = EXCLUDED.provider_type,
                 summary = EXCLUDED.summary,
                 outcome = EXCLUDED.outcome,
                 tags = EXCLUDED.tags,
                 evidence_refs = EXCLUDED.evidence_refs,
                 updated_at = now()",
        )
        .bind(&record.case_id)
        .bind(&record.title)
        .bind(&record.fwa_type)
        .bind(&record.scheme_family)
        .bind(&record.diagnosis_code)
        .bind(&record.provider_region)
        .bind(&record.provider_type)
        .bind(&record.summary)
        .bind(&record.outcome)
        .bind(serde_json::json!(record.tags))
        .bind(serde_json::json!(record.evidence_refs))
        .execute(&self.pool)
        .await?;
        Ok(record)
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
        sqlx::query("DELETE FROM agent_context_snapshots WHERE agent_run_id = $1")
            .bind(&run.agent_run_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM tool_results WHERE agent_run_id = $1")
            .bind(&run.agent_run_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM agent_policy_checks WHERE agent_run_id = $1")
            .bind(&run.agent_run_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM tool_calls WHERE agent_run_id = $1")
            .bind(&run.agent_run_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM agent_approvals WHERE agent_run_id = $1")
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
        for snapshot in &run.context_snapshots {
            sqlx::query(
                "INSERT INTO agent_context_snapshots
                 (snapshot_id, agent_run_id, redaction_status, context_json, source_refs, checksum)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(&snapshot.snapshot_id)
            .bind(&run.agent_run_id)
            .bind(&snapshot.redaction_status)
            .bind(&snapshot.context_json)
            .bind(string_values(&snapshot.source_refs))
            .bind(&snapshot.checksum)
            .execute(&mut *tx)
            .await?;
        }
        for call in &run.tool_calls {
            sqlx::query(
                "INSERT INTO tool_calls
                 (tool_call_id, agent_run_id, tool_name, status, input_json, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(&call.tool_call_id)
            .bind(&run.agent_run_id)
            .bind(&call.tool_name)
            .bind(&call.status)
            .bind(&call.input_json)
            .bind(string_values(&call.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }
        for check in &run.policy_checks {
            sqlx::query(
                "INSERT INTO agent_policy_checks
                 (policy_check_id, agent_run_id, tool_call_id, tool_name, policy_name, decision, reason, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(&check.policy_check_id)
            .bind(&run.agent_run_id)
            .bind(&check.tool_call_id)
            .bind(&check.tool_name)
            .bind(&check.policy_name)
            .bind(&check.decision)
            .bind(&check.reason)
            .bind(string_values(&check.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }
        for result in &run.tool_results {
            sqlx::query(
                "INSERT INTO tool_results
                 (tool_result_id, tool_call_id, agent_run_id, tool_name, status, output_json, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(&result.tool_result_id)
            .bind(&result.tool_call_id)
            .bind(&run.agent_run_id)
            .bind(&result.tool_name)
            .bind(&result.status)
            .bind(&result.output_json)
            .bind(string_values(&result.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }
        for approval in &run.approvals {
            sqlx::query(
                "INSERT INTO agent_approvals
                 (approval_id, agent_run_id, proposed_action, decision, approver, reason, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)",
            )
            .bind(&approval.approval_id)
            .bind(&run.agent_run_id)
            .bind(&approval.proposed_action)
            .bind(&approval.decision)
            .bind(&approval.approver)
            .bind(&approval.reason)
            .bind(string_values(&approval.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn list_agent_runs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AgentRunLogRecord>> {
        let rows: Vec<(
            String,
            String,
            String,
            String,
            Value,
            Value,
            chrono::DateTime<chrono::Utc>,
            Option<chrono::DateTime<chrono::Utc>>,
        )> = sqlx::query_as(
            "SELECT agent_run_id, claim_id, status, decision_boundary, output_json, evidence_refs, created_at, completed_at
             FROM agent_runs ar
             WHERE (
               $1::text IS NULL OR EXISTS (
                 SELECT 1
                 FROM audit_events ae
                 LEFT JOIN claims c ON c.id = ae.claim_id
                 WHERE ae.payload ->> 'customer_scope_id' = $1
                   AND (
                     ae.payload ->> 'claim_id' = ar.claim_id
                     OR c.external_claim_id = ar.claim_id
                     OR ae.claim_id::text = ar.claim_id
                   )
               )
             )
             ORDER BY created_at DESC, agent_run_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;

        let mut runs = Vec::with_capacity(rows.len());
        for (
            agent_run_id,
            claim_id,
            status,
            decision_boundary,
            output_json,
            evidence_refs,
            created_at,
            completed_at,
        ) in rows
        {
            let steps: Vec<(Value,)> = sqlx::query_as(
                "SELECT output_json
                 FROM agent_steps
                 WHERE agent_run_id = $1
                 ORDER BY created_at, id",
            )
            .bind(&agent_run_id)
            .fetch_all(&self.pool)
            .await?;
            let context_snapshots = self.load_agent_context_snapshots(&agent_run_id).await?;
            let policy_checks = self.load_agent_policy_checks(&agent_run_id).await?;
            let tool_calls = self.load_agent_tool_calls(&agent_run_id).await?;
            let tool_results = self.load_agent_tool_results(&agent_run_id).await?;
            let approvals = self.load_agent_approvals(&agent_run_id).await?;
            runs.push(AgentRunLogRecord {
                agent_run_id,
                claim_id,
                status,
                decision_boundary,
                output_json,
                evidence_refs: json_array_to_strings(evidence_refs),
                steps: steps.into_iter().map(|row| row.0).collect(),
                context_snapshots,
                policy_checks,
                tool_calls,
                tool_results,
                approvals,
                created_at: Some(created_at.to_rfc3339()),
                completed_at: completed_at.map(|value| value.to_rfc3339()),
            });
        }

        Ok(runs)
    }

    async fn save_agent_approval(
        &self,
        approval: AgentApprovalRecord,
    ) -> anyhow::Result<AgentApprovalRecord> {
        sqlx::query(
            "INSERT INTO agent_approvals
             (approval_id, agent_run_id, proposed_action, decision, approver, reason, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (approval_id) DO UPDATE
             SET decision = EXCLUDED.decision,
                 approver = EXCLUDED.approver,
                 reason = EXCLUDED.reason,
                 evidence_refs = EXCLUDED.evidence_refs",
        )
        .bind(&approval.approval_id)
        .bind(&approval.agent_run_id)
        .bind(&approval.proposed_action)
        .bind(&approval.decision)
        .bind(&approval.approver)
        .bind(&approval.reason)
        .bind(string_values(&approval.evidence_refs))
        .execute(&self.pool)
        .await?;
        Ok(approval)
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
        let saving_attributions = derive_saving_attributions(&record);
        let mut tx = self.pool.begin().await?;
        let previous_case_id: Option<String> = sqlx::query_scalar(
            "SELECT case_id FROM investigation_results WHERE investigation_id = $1",
        )
        .bind(&record.investigation_id)
        .fetch_optional(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO investigation_results
             (investigation_id, case_id, claim_id, outcome, confirmed_fwa, financial_impact_type, saving_amount, currency, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (investigation_id) DO UPDATE
             SET case_id = EXCLUDED.case_id,
                 claim_id = EXCLUDED.claim_id,
                 outcome = EXCLUDED.outcome,
                 confirmed_fwa = EXCLUDED.confirmed_fwa,
                 financial_impact_type = EXCLUDED.financial_impact_type,
                 saving_amount = EXCLUDED.saving_amount,
                 currency = EXCLUDED.currency,
                 notes = EXCLUDED.notes,
                 evidence_refs = EXCLUDED.evidence_refs",
        )
        .bind(&record.investigation_id)
        .bind(&record.case_id)
        .bind(&record.claim_id)
        .bind(&record.outcome)
        .bind(record.confirmed_fwa)
        .bind(normalize_financial_impact_type(
            record.financial_impact_type.as_deref(),
        ))
        .bind(record.saving_amount)
        .bind(&record.currency)
        .bind(&record.notes)
        .bind(serde_json::json!(record.evidence_refs))
        .execute(&mut *tx)
        .await?;

        if previous_case_id.as_deref() != record.case_id.as_deref() {
            if let Some(case_id) = previous_case_id.as_deref() {
                sqlx::query(
                    "UPDATE investigation_cases
                     SET final_outcome = NULL,
                         reviewer_notes = NULL,
                         investigation_result_id = NULL,
                         updated_at = now()
                     WHERE case_id = $1
                       AND investigation_result_id = $2",
                )
                .bind(case_id)
                .bind(&record.investigation_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        if let Some(case_id) = record.case_id.as_deref() {
            let update = sqlx::query(
                "UPDATE investigation_cases
                 SET final_outcome = $1,
                     reviewer_notes = $2,
                     investigation_result_id = $3,
                     updated_at = now()
                 WHERE case_id = $4
                   AND claim_id = $5",
            )
            .bind(&record.outcome)
            .bind(&record.notes)
            .bind(&record.investigation_id)
            .bind(case_id)
            .bind(&record.claim_id)
            .execute(&mut *tx)
            .await?;
            if update.rows_affected() == 0 {
                anyhow::bail!("case not found for investigation result: {case_id}");
            }
        }

        sqlx::query("DELETE FROM saving_attributions WHERE investigation_id = $1")
            .bind(&record.investigation_id)
            .execute(&mut *tx)
            .await?;
        for attribution in saving_attributions {
            sqlx::query(
                "INSERT INTO saving_attributions
                 (attribution_id, claim_id, investigation_id, source_type, source_id, financial_impact_type, action, saving_amount, currency, evidence_refs)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(&attribution.attribution_id)
            .bind(&attribution.claim_id)
            .bind(&attribution.investigation_id)
            .bind(&attribution.source_type)
            .bind(&attribution.source_id)
            .bind(&attribution.financial_impact_type)
            .bind(&attribution.action)
            .bind(attribution.saving_amount)
            .bind(&attribution.currency)
            .bind(serde_json::json!(attribution.evidence_refs))
            .execute(&mut *tx)
            .await?;
        }

        let event = AuditHistoryEventRecord {
            audit_id: format!("audit_investigation_{}", record.investigation_id),
            run_id: format!("pilot_investigation_{}", record.investigation_id),
            actor_role: record
                .actor_role
                .clone()
                .unwrap_or_else(|| "tpa_system".into()),
            event_type: "investigation.result.received".into(),
            event_status: "succeeded".into(),
            summary: format!("Investigation result received: {}", record.outcome),
            payload: serde_json::to_value(&record)?,
            evidence_refs: record.evidence_refs.clone(),
            created_at: None,
        };
        insert_pilot_audit_event(&mut tx, &record.claim_id, &event).await?;
        tx.commit().await?;
        Ok(event)
    }

    async fn save_qa_review(
        &self,
        mut record: QaReviewRecord,
    ) -> anyhow::Result<AuditHistoryEventRecord> {
        record.feedback_target = canonical_feedback_target(&record.feedback_target).into();
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO qa_reviews
             (qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, feedback_status, notes, evidence_refs)
             VALUES ($1, $2, $3, $4, $5, 'open', $6, $7)
             ON CONFLICT (qa_case_id) DO UPDATE
             SET qa_conclusion = EXCLUDED.qa_conclusion,
                 issue_type = EXCLUDED.issue_type,
                 feedback_target = EXCLUDED.feedback_target,
                 feedback_status = EXCLUDED.feedback_status,
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
            actor_role: record
                .actor_role
                .clone()
                .unwrap_or_else(|| "tpa_system".into()),
            event_type: "qa.result.received".into(),
            event_status: "succeeded".into(),
            summary: format!("QA result received: {}", record.qa_conclusion),
            payload: serde_json::to_value(&record)?,
            evidence_refs: record.evidence_refs.clone(),
            created_at: None,
        };
        insert_pilot_audit_event(&mut tx, &record.claim_id, &event).await?;
        tx.commit().await?;
        Ok(event)
    }

    async fn list_qa_feedback_items(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaFeedbackItemRecord>> {
        let allowed_qa_case_ids = if let Some(scope) = customer_scope_id {
            Some(
                self.list_audit_events(AuditEventListFilter {
                    limit: 10_000,
                    event_type: Some("qa.result.received".into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .filter_map(|event| event.payload["qa_case_id"].as_str().map(str::to_string))
                .collect::<BTreeSet<_>>(),
            )
        } else {
            None
        };
        let mut status_events = self
            .list_audit_events(AuditEventListFilter {
                limit: 10_000,
                event_type: Some("qa.feedback.status.updated".into()),
                customer_scope_id: customer_scope_id.map(str::to_string),
                ..Default::default()
            })
            .await?;
        status_events.reverse();
        let feedback_statuses = latest_qa_feedback_statuses(
            &status_events
                .into_iter()
                .map(|event| {
                    (
                        event.payload["claim_id"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        event,
                    )
                })
                .collect::<Vec<_>>(),
        );
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, feedback_status, notes, evidence_refs, created_at
             FROM qa_reviews
             WHERE qa_conclusion <> 'pass'
             ORDER BY created_at, qa_case_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut items = rows
            .into_iter()
            .filter(|(qa_case_id, _, _, _, _, _, _, _, _)| {
                allowed_qa_case_ids
                    .as_ref()
                    .is_none_or(|ids| ids.contains(qa_case_id))
            })
            .map(
                |(
                    qa_case_id,
                    claim_id,
                    qa_conclusion,
                    issue_type,
                    feedback_target,
                    feedback_status,
                    notes,
                    evidence_refs,
                    created_at,
                )| {
                    let feedback_id = qa_feedback_id(&qa_case_id);
                    let status_update = feedback_statuses.get(&feedback_id);
                    qa_review_to_feedback_item(
                        QaReviewRecord {
                            qa_case_id,
                            claim_id,
                            qa_conclusion,
                            issue_type,
                            feedback_target,
                            notes,
                            evidence_refs: json_array_to_strings(evidence_refs),
                            customer_scope_id: None,
                            actor_id: None,
                            actor_role: None,
                        },
                        Some(created_at.to_rfc3339()),
                        &feedback_status,
                        status_update,
                    )
                },
            )
            .collect::<Vec<_>>();
        sort_qa_feedback_items(&mut items);
        Ok(items)
    }

    async fn update_qa_feedback_status(
        &self,
        feedback_id: &str,
        input: UpdateQaFeedbackStatusInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<UpdateQaFeedbackStatusRecord>> {
        let Some(qa_case_id) = qa_case_id_from_feedback_id(feedback_id) else {
            return Ok(None);
        };
        if let Some(scope) = customer_scope_id {
            let is_in_scope = self
                .list_audit_events(AuditEventListFilter {
                    limit: 1,
                    event_type: Some("qa.result.received".into()),
                    qa_case_id: Some(qa_case_id.into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .next()
                .is_some();
            if !is_in_scope {
                return Ok(None);
            }
        }
        let mut tx = self.pool.begin().await?;
        let row: Option<(
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            String,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "WITH existing AS (
                 SELECT qa_case_id, feedback_status AS from_status
                 FROM qa_reviews
                 WHERE qa_case_id = $1 AND qa_conclusion <> 'pass'
             ),
             updated AS (
                 UPDATE qa_reviews
                 SET feedback_status = $2
                 FROM existing
                 WHERE qa_reviews.qa_case_id = existing.qa_case_id
                 RETURNING existing.from_status,
                           qa_reviews.qa_case_id,
                           qa_reviews.claim_id,
                           qa_reviews.qa_conclusion,
                           qa_reviews.issue_type,
                           qa_reviews.feedback_target,
                           qa_reviews.feedback_status,
                           qa_reviews.notes,
                           qa_reviews.evidence_refs,
                           qa_reviews.created_at
             )
             SELECT * FROM updated",
        )
        .bind(qa_case_id)
        .bind(&input.status)
        .fetch_optional(&mut *tx)
        .await?;
        let Some((
            from_status,
            qa_case_id,
            claim_id,
            qa_conclusion,
            issue_type,
            feedback_target,
            feedback_status,
            notes,
            evidence_refs,
            created_at,
        )) = row
        else {
            return Ok(None);
        };
        let audit_id = AuditEventId::new().to_string();
        let item = qa_review_to_feedback_item(
            QaReviewRecord {
                qa_case_id,
                claim_id: claim_id.clone(),
                qa_conclusion,
                issue_type,
                feedback_target,
                notes,
                evidence_refs: json_array_to_strings(evidence_refs),
                customer_scope_id: None,
                actor_id: None,
                actor_role: None,
            },
            Some(created_at.to_rfc3339()),
            &feedback_status,
            Some(&QaFeedbackStatusUpdate {
                status: feedback_status.clone(),
                actor_id: Some(input.actor_id.clone()),
                audit_id: audit_id.clone(),
                updated_at: None,
                evidence_refs: input.evidence_refs.clone(),
            }),
        );
        insert_pilot_audit_event(
            &mut tx,
            &claim_id,
            &AuditHistoryEventRecord {
                audit_id: audit_id.clone(),
                run_id: format!("qa_feedback_status_{}", item.feedback_id),
                actor_role: "fwa_operator".into(),
                event_type: "qa.feedback.status.updated".into(),
                event_status: "succeeded".into(),
                summary: format!("QA feedback status updated: {}", item.status),
                payload: serde_json::json!({
                    "feedback_id": item.feedback_id,
                    "qa_case_id": item.qa_case_id,
                    "claim_id": item.claim_id,
                    "feedback_target": item.feedback_target,
                    "from_status": from_status,
                    "to_status": item.status,
                    "actor_id": input.actor_id,
                    "notes": input.notes,
                    "customer_scope_id": input.customer_scope_id
                }),
                evidence_refs: input.evidence_refs,
                created_at: None,
            },
        )
        .await?;
        tx.commit().await?;
        Ok(Some(UpdateQaFeedbackStatusRecord { item, audit_id }))
    }

    async fn list_qa_reviews(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<QaReviewRecord>> {
        let allowed_qa_case_ids = if let Some(scope) = customer_scope_id {
            Some(
                self.list_audit_events(AuditEventListFilter {
                    limit: 10_000,
                    event_type: Some("qa.result.received".into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .filter_map(|event| event.payload["qa_case_id"].as_str().map(str::to_string))
                .collect::<BTreeSet<_>>(),
            )
        } else {
            None
        };
        let rows: Vec<(String, String, String, String, String, String, Value)> = sqlx::query_as(
            "SELECT qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, notes, evidence_refs
             FROM qa_reviews
             ORDER BY qa_case_id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter(|(qa_case_id, _, _, _, _, _, _)| {
                allowed_qa_case_ids
                    .as_ref()
                    .is_none_or(|ids| ids.contains(qa_case_id))
            })
            .map(
                |(
                    qa_case_id,
                    claim_id,
                    qa_conclusion,
                    issue_type,
                    feedback_target,
                    notes,
                    evidence_refs,
                )| {
                    let feedback_target = canonical_feedback_target(&feedback_target).into();
                    QaReviewRecord {
                        qa_case_id,
                        claim_id,
                        qa_conclusion,
                        issue_type,
                        feedback_target,
                        notes,
                        evidence_refs: json_array_to_strings(evidence_refs),
                        customer_scope_id: None,
                        actor_id: None,
                        actor_role: None,
                    }
                },
            )
            .collect())
    }

    async fn list_outcome_labels(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<OutcomeLabelRecord>> {
        let allowed_investigation_ids = if let Some(scope) = customer_scope_id {
            Some(
                self.list_audit_events(AuditEventListFilter {
                    limit: 10_000,
                    event_type: Some("investigation.result.received".into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .filter_map(|event| {
                    event.payload["investigation_id"]
                        .as_str()
                        .map(str::to_string)
                })
                .collect::<BTreeSet<_>>(),
            )
        } else {
            None
        };
        let allowed_qa_case_ids = if let Some(scope) = customer_scope_id {
            Some(
                self.list_audit_events(AuditEventListFilter {
                    limit: 10_000,
                    event_type: Some("qa.result.received".into()),
                    customer_scope_id: Some(scope.into()),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .filter_map(|event| event.payload["qa_case_id"].as_str().map(str::to_string))
                .collect::<BTreeSet<_>>(),
            )
        } else {
            None
        };
        let investigation_rows: Vec<(
            String,
            String,
            String,
            bool,
            Option<String>,
            Option<Decimal>,
            Option<String>,
            String,
            Value,
        )> = sqlx::query_as(
            "SELECT investigation_id, claim_id, outcome, confirmed_fwa, financial_impact_type, saving_amount, currency, notes, evidence_refs
             FROM investigation_results
             ORDER BY created_at, investigation_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let qa_rows: Vec<(String, String, String, String, String, String, String, Value)> =
            sqlx::query_as(
                "SELECT qa_case_id, claim_id, qa_conclusion, issue_type, feedback_target, feedback_status, notes, evidence_refs
                 FROM qa_reviews
                 ORDER BY created_at, qa_case_id",
            )
            .fetch_all(&self.pool)
            .await?;
        let medical_review_rows: Vec<(String, String, Value, Value)> = sqlx::query_as(
            "SELECT audit_id, actor_role, payload, evidence_refs
             FROM audit_events
             WHERE event_type = 'medical.review.recorded'
               AND event_status = 'succeeded'
               AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)
             ORDER BY created_at, audit_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        let lead_triage_rows: Vec<(String, String, String, Value, Value)> = sqlx::query_as(
            "SELECT audit_id, run_id, actor_role, payload, evidence_refs
             FROM audit_events
             WHERE event_type = 'lead.triaged'
               AND event_status = 'succeeded'
               AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)
             ORDER BY created_at, audit_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;
        let label_bootstrap_rows: Vec<(String, String, String, Value, Value)> = sqlx::query_as(
            "SELECT audit_id, run_id, actor_role, payload, evidence_refs
             FROM audit_events
             WHERE event_type = 'label.bootstrap.reviewed'
               AND event_status = 'succeeded'
               AND ($1::text IS NULL OR payload ->> 'customer_scope_id' = $1)
             ORDER BY created_at, audit_id",
        )
        .bind(customer_scope_id)
        .fetch_all(&self.pool)
        .await?;

        let mut labels = investigation_rows
            .into_iter()
            .filter(|(investigation_id, _, _, _, _, _, _, _, _)| {
                allowed_investigation_ids
                    .as_ref()
                    .is_none_or(|ids| ids.contains(investigation_id))
            })
            .flat_map(
                |(
                    investigation_id,
                    claim_id,
                    outcome,
                    confirmed_fwa,
                    financial_impact_type,
                    saving_amount,
                    currency,
                    notes,
                    evidence_refs,
                )| {
                    labels_from_investigation_result(InvestigationResultRecord {
                        investigation_id,
                        case_id: None,
                        claim_id,
                        outcome,
                        confirmed_fwa,
                        financial_impact_type,
                        saving_amount,
                        currency,
                        notes,
                        evidence_refs: json_array_to_strings(evidence_refs),
                        customer_scope_id: None,
                        actor_id: None,
                        actor_role: None,
                    })
                },
            )
            .chain(
                qa_rows
                    .into_iter()
                    .filter(|(qa_case_id, _, _, _, _, _, _, _)| {
                        allowed_qa_case_ids
                            .as_ref()
                            .is_none_or(|ids| ids.contains(qa_case_id))
                    })
                    .map(
                        |(
                            qa_case_id,
                            claim_id,
                            qa_conclusion,
                            issue_type,
                            feedback_target,
                            feedback_status,
                            notes,
                            evidence_refs,
                        )| {
                            label_from_qa_review(
                                QaReviewRecord {
                                    qa_case_id,
                                    claim_id,
                                    qa_conclusion,
                                    issue_type,
                                    feedback_target,
                                    notes,
                                    evidence_refs: json_array_to_strings(evidence_refs),
                                    customer_scope_id: None,
                                    actor_id: None,
                                    actor_role: None,
                                },
                                &feedback_status,
                            )
                        },
                    ),
            )
            .chain(medical_review_rows.into_iter().flat_map(
                |(audit_id, actor_role, payload, evidence_refs)| {
                    labels_from_medical_review_event(&AuditHistoryEventRecord {
                        audit_id,
                        run_id: String::new(),
                        actor_role,
                        event_type: "medical.review.recorded".into(),
                        event_status: "succeeded".into(),
                        summary: String::new(),
                        payload,
                        evidence_refs: json_array_to_strings(evidence_refs),
                        created_at: None,
                    })
                },
            ))
            .chain(label_bootstrap_rows.into_iter().filter_map(
                |(audit_id, run_id, actor_role, payload, evidence_refs)| {
                    label_from_bootstrap_review_event(&AuditHistoryEventRecord {
                        audit_id,
                        run_id,
                        actor_role,
                        event_type: "label.bootstrap.reviewed".into(),
                        event_status: "succeeded".into(),
                        summary: String::new(),
                        payload,
                        evidence_refs: json_array_to_strings(evidence_refs),
                        created_at: None,
                    })
                },
            ))
            .collect::<Vec<_>>();
        labels.extend(labels_from_lead_triage_events(
            lead_triage_rows.into_iter().map(
                |(audit_id, run_id, actor_role, payload, evidence_refs)| AuditHistoryEventRecord {
                    audit_id,
                    run_id,
                    actor_role,
                    event_type: "lead.triaged".into(),
                    event_status: "succeeded".into(),
                    summary: String::new(),
                    payload,
                    evidence_refs: json_array_to_strings(evidence_refs),
                    created_at: None,
                },
            ),
        ));
        labels.extend(
            self.list_cases(None)
                .await?
                .into_iter()
                .flat_map(labels_from_case_status),
        );
        sort_outcome_labels(&mut labels);
        Ok(labels)
    }

    async fn claim_audit_history(
        &self,
        claim_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        let rows: Vec<(String, String, String, String, String, String, Value, Value, chrono::DateTime<chrono::Utc>)> =
            sqlx::query_as(
                "SELECT ae.audit_id, ae.run_id, ae.actor_role, ae.event_type, ae.event_status, ae.summary, ae.payload, ae.evidence_refs, ae.created_at
                 FROM audit_events ae
                 LEFT JOIN claims c ON c.id = ae.claim_id
                 WHERE (payload ->> 'claim_id' = $1 OR c.external_claim_id = $1)
                   AND ($2::text IS NULL OR ae.payload ->> 'customer_scope_id' = $2)
                 ORDER BY ae.created_at, ae.audit_id",
            )
            .bind(claim_id)
            .bind(customer_scope_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs,
                    created_at,
                )| AuditHistoryEventRecord {
                    audit_id,
                    run_id,
                    actor_role,
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

    async fn list_audit_events(
        &self,
        filter: AuditEventListFilter,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            Value,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT ae.audit_id, ae.run_id, ae.actor_role, ae.event_type, ae.event_status, ae.summary, ae.payload, ae.evidence_refs, ae.created_at
             FROM audit_events ae
             LEFT JOIN claims c ON c.id = ae.claim_id
             WHERE ($2::text IS NULL OR ae.event_type = $2)
               AND ($3::text IS NULL OR ae.actor_id = $3)
               AND ($4::text IS NULL OR ae.run_id = $4)
               AND (
                 $5::text IS NULL
                 OR ae.payload ->> 'claim_id' = $5
                 OR c.external_claim_id = $5
                 OR ae.claim_id::text = $5
               )
               AND ($6::text IS NULL OR ae.payload ->> 'policy_id' = $6)
               AND ($7::text IS NULL OR ae.payload ->> 'version' = $7)
               AND ($8::text IS NULL OR ae.payload ->> 'review_mode' = $8)
               AND ($9::text IS NULL OR ae.payload ->> 'rule_id' = $9)
               AND ($10::text IS NULL OR ae.payload ->> 'rule_version' = $10)
               AND ($11::text IS NULL OR ae.payload ->> 'model_key' = $11)
               AND ($12::text IS NULL OR ae.payload ->> 'model_version' = $12)
               AND (
                 $13::text IS NULL
                 OR (
                   $13 = 'governance'
                   AND ae.event_type = ANY($14::text[])
                 )
               )
               AND ($15::text IS NULL OR ae.payload ->> 'sample_id' = $15)
               AND ($16::text IS NULL OR ae.payload ->> 'agent_run_id' = $16)
               AND ($17::text IS NULL OR ae.payload ->> 'dataset_id' = $17)
               AND ($18::text IS NULL OR ae.payload ->> 'feature_set_id' = $18)
               AND ($19::text IS NULL OR ae.payload ->> 'model_dataset_id' = $19)
               AND ($20::text IS NULL OR ae.payload ->> 'evaluation_run_id' = $20)
               AND ($21::bool IS NULL OR $21 = false OR ae.payload ? 'canonical_claim_context_trace')
               AND ($22::text IS NULL OR ae.payload ->> 'customer_scope_id' = $22)
             ORDER BY ae.created_at DESC, ae.audit_id DESC
             LIMIT $1",
        )
        .bind(filter.limit as i64)
        .bind(filter.event_type.as_deref())
        .bind(filter.actor_id.as_deref())
        .bind(filter.run_id.as_deref())
        .bind(filter.claim_id.as_deref())
        .bind(filter.routing_policy_id.as_deref())
        .bind(filter.routing_policy_version.as_deref())
        .bind(filter.review_mode.as_deref())
        .bind(filter.rule_id.as_deref())
        .bind(filter.rule_version.as_deref())
        .bind(filter.model_key.as_deref())
        .bind(filter.model_version.as_deref())
        .bind(filter.event_group.as_deref())
        .bind(GOVERNANCE_AUDIT_EVENT_TYPES)
        .bind(filter.sample_id.as_deref())
        .bind(filter.agent_run_id.as_deref())
        .bind(filter.dataset_id.as_deref())
        .bind(filter.feature_set_id.as_deref())
        .bind(filter.model_dataset_id.as_deref())
        .bind(filter.evaluation_run_id.as_deref())
        .bind(filter.has_canonical_trace)
        .bind(filter.customer_scope_id.as_deref())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs,
                    created_at,
                )| AuditHistoryEventRecord {
                    audit_id,
                    run_id,
                    actor_role,
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

    async fn list_webhook_events(&self) -> anyhow::Result<Vec<WebhookEventRecord>> {
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            Value,
            Value,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT audit_id, run_id, actor_role, event_type, event_status, summary, payload, evidence_refs, created_at
             FROM audit_events
             ORDER BY created_at, audit_id",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut events = rows
            .into_iter()
            .filter_map(
                |(
                    audit_id,
                    run_id,
                    actor_role,
                    event_type,
                    event_status,
                    summary,
                    payload,
                    evidence_refs,
                    created_at,
                )| {
                    webhook_event_from_audit(
                        None,
                        &AuditHistoryEventRecord {
                            audit_id,
                            run_id,
                            actor_role,
                            event_type,
                            event_status,
                            summary,
                            payload,
                            evidence_refs: json_array_to_strings(evidence_refs),
                            created_at: Some(created_at.to_rfc3339()),
                        },
                    )
                },
            )
            .collect::<Vec<_>>();
        let attempt_rows: Vec<(
            String,
            i32,
            String,
            Option<i32>,
            Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
            chrono::DateTime<chrono::Utc>,
        )> = sqlx::query_as(
            "SELECT event_id, attempt_number, delivery_status, response_status_code, error_message, next_attempt_at, attempted_at
             FROM webhook_delivery_attempts
             ORDER BY event_id, attempt_number",
        )
        .fetch_all(&self.pool)
        .await?;
        let attempts = attempt_rows
            .into_iter()
            .map(
                |(
                    event_id,
                    attempt_number,
                    delivery_status,
                    response_status_code,
                    error_message,
                    next_attempt_at,
                    attempted_at,
                )| WebhookDeliveryAttemptRecord {
                    event_id,
                    attempt_number: attempt_number.max(0) as u32,
                    delivery_status,
                    response_status_code: response_status_code.map(|value| value.max(0) as u16),
                    error_message,
                    next_attempt_at: next_attempt_at.map(|timestamp| timestamp.to_rfc3339()),
                    attempted_at: Some(attempted_at.to_rfc3339()),
                },
            )
            .collect::<Vec<_>>();
        apply_webhook_delivery_state(&mut events, &attempts);
        sort_webhook_events(&mut events);
        Ok(events)
    }

    async fn save_webhook_delivery_attempt(
        &self,
        input: WebhookDeliveryAttemptInput,
    ) -> anyhow::Result<WebhookDeliveryAttemptRecord> {
        let row: (Option<i32>,) = sqlx::query_as(
            "SELECT MAX(attempt_number)
             FROM webhook_delivery_attempts
             WHERE event_id = $1",
        )
        .bind(&input.event_id)
        .fetch_one(&self.pool)
        .await?;
        let attempt_number = row.0.unwrap_or(0) + 1;
        let attempted_at = chrono::Utc::now();
        let next_attempt_at =
            next_webhook_attempt_at(&input.delivery_status, attempt_number as u32, attempted_at);
        let inserted: (chrono::DateTime<chrono::Utc>,) = sqlx::query_as(
            "INSERT INTO webhook_delivery_attempts
             (event_id, attempt_number, delivery_status, response_status_code, error_message, next_attempt_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING attempted_at",
        )
        .bind(&input.event_id)
        .bind(attempt_number)
        .bind(&input.delivery_status)
        .bind(input.response_status_code.map(i32::from))
        .bind(&input.error_message)
        .bind(next_attempt_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(WebhookDeliveryAttemptRecord {
            event_id: input.event_id,
            attempt_number: attempt_number as u32,
            delivery_status: input.delivery_status,
            response_status_code: input.response_status_code,
            error_message: input.error_message,
            next_attempt_at: next_attempt_at.map(|timestamp| timestamp.to_rfc3339()),
            attempted_at: Some(inserted.0.to_rfc3339()),
        })
    }

    async fn register_feature_set(
        &self,
        input: RegisterFeatureSetInput,
    ) -> anyhow::Result<Option<FeatureSetRecord>> {
        if load_dataset_record(&self.pool, &input.dataset_id)
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
        .fetch_one(&self.pool)
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

    async fn register_model_dataset(
        &self,
        input: RegisterModelDatasetInput,
    ) -> anyhow::Result<Option<ModelDatasetRecord>> {
        let feature_set_known: Option<(String,)> =
            sqlx::query_as("SELECT id::text FROM feature_set_versions WHERE id = $1::uuid")
                .bind(&input.feature_set_id)
                .fetch_optional(&self.pool)
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
        .fetch_one(&self.pool)
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

    async fn get_model_dataset_source_dataset(
        &self,
        model_dataset_id: &str,
    ) -> anyhow::Result<Option<DatasetRecord>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT fs.dataset_id::text
             FROM model_dataset_versions md
             JOIN feature_set_versions fs ON fs.id = md.feature_set_id
             WHERE md.id = $1::uuid",
        )
        .bind(model_dataset_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some((dataset_id,)) = row else {
            return Ok(None);
        };
        load_dataset_record(&self.pool, &dataset_id).await
    }

    async fn register_model_evaluation(
        &self,
        input: RegisterModelEvaluationInput,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>> {
        let model_dataset_known: Option<(String,)> =
            sqlx::query_as("SELECT id::text FROM model_dataset_versions WHERE id = $1::uuid")
                .bind(&input.model_dataset_id)
                .fetch_optional(&self.pool)
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
        .execute(&self.pool)
        .await?;

        self.get_model_evaluation(&input.evaluation_run_id).await
    }

    async fn get_model_evaluation(
        &self,
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
        .fetch_optional(&self.pool)
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

    async fn list_model_evaluations(&self) -> anyhow::Result<Vec<ModelEvaluationRecord>> {
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
        .fetch_all(&self.pool)
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

    async fn save_evidence_document(
        &self,
        input: CreateEvidenceDocumentInput,
    ) -> anyhow::Result<EvidenceDocumentRecord> {
        postgres_evidence::save_evidence_document(self, input).await
    }

    async fn list_evidence_documents(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentRecord>> {
        postgres_evidence::list_evidence_documents(self, customer_scope_id).await
    }

    async fn get_evidence_document(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentRecord>> {
        postgres_evidence::get_evidence_document(self, document_id, customer_scope_id).await
    }

    async fn save_evidence_document_chunk(
        &self,
        input: CreateEvidenceDocumentChunkInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceDocumentChunkRecord>> {
        postgres_evidence::save_evidence_document_chunk(self, input, customer_scope_id).await
    }

    async fn list_evidence_document_chunks(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceDocumentChunkRecord>> {
        postgres_evidence::list_evidence_document_chunks(self, document_id, customer_scope_id).await
    }

    async fn save_evidence_ocr_output(
        &self,
        input: CreateEvidenceOcrOutputInput,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Option<EvidenceOcrOutputRecord>> {
        postgres_evidence::save_evidence_ocr_output(self, input, customer_scope_id).await
    }

    async fn list_evidence_ocr_outputs(
        &self,
        document_id: &str,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceOcrOutputRecord>> {
        postgres_evidence::list_evidence_ocr_outputs(self, document_id, customer_scope_id).await
    }

    async fn save_evidence_embedding_job(
        &self,
        input: CreateEvidenceEmbeddingJobInput,
    ) -> anyhow::Result<EvidenceEmbeddingJobRecord> {
        postgres_evidence::save_evidence_embedding_job(self, input).await
    }

    async fn list_evidence_embedding_jobs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceEmbeddingJobRecord>> {
        postgres_evidence::list_evidence_embedding_jobs(self, customer_scope_id).await
    }

    async fn save_evidence_retrieval_audit_event(
        &self,
        input: CreateEvidenceRetrievalAuditEventInput,
    ) -> anyhow::Result<EvidenceRetrievalAuditEventRecord> {
        postgres_evidence::save_evidence_retrieval_audit_event(self, input).await
    }

    async fn list_evidence_retrieval_audit_events(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<EvidenceRetrievalAuditEventRecord>> {
        postgres_evidence::list_evidence_retrieval_audit_events(self, customer_scope_id).await
    }
}

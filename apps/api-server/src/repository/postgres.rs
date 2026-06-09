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
        postgres_rules::list_rules(self).await
    }

    async fn list_active_rules(&self) -> anyhow::Result<Vec<Rule>> {
        postgres_rules::list_active_rules(self).await
    }

    async fn get_rule(&self, rule_id: &str) -> anyhow::Result<Option<RuleDetailRecord>> {
        postgres_rules::get_rule(self, rule_id).await
    }

    async fn rule_audit_history(
        &self,
        rule_id: &str,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        postgres_rules::rule_audit_history(self, rule_id).await
    }

    async fn save_rule_candidate(
        &self,
        rule: Rule,
        owner: String,
    ) -> anyhow::Result<RuleDetailRecord> {
        postgres_rules::save_rule_candidate(self, rule, owner).await
    }

    async fn update_rule_status(
        &self,
        rule_id: &str,
        status: &str,
    ) -> anyhow::Result<Option<RuleSummaryRecord>> {
        postgres_rules::update_rule_status(self, rule_id, status).await
    }

    async fn list_rule_conditions(&self) -> anyhow::Result<Vec<RuleConditionLibraryRecord>> {
        postgres_rules::list_rule_conditions(self).await
    }

    async fn rule_performance(&self) -> anyhow::Result<Vec<RulePerformanceRecord>> {
        postgres_rules::rule_performance(self).await
    }

    async fn save_rule_backtest(
        &self,
        record: RuleBacktestRecord,
    ) -> anyhow::Result<RuleBacktestRecord> {
        postgres_rules::save_rule_backtest(self, record).await
    }

    async fn latest_rule_backtest(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleBacktestRecord>> {
        postgres_rules::latest_rule_backtest(self, rule_id, rule_version).await
    }

    async fn save_rule_shadow_run(
        &self,
        record: RuleShadowRunRecord,
    ) -> anyhow::Result<RuleShadowRunRecord> {
        postgres_rules::save_rule_shadow_run(self, record).await
    }

    async fn latest_rule_shadow_run(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RuleShadowRunRecord>> {
        postgres_rules::latest_rule_shadow_run(self, rule_id, rule_version).await
    }

    async fn save_rule_promotion_review(
        &self,
        record: RulePromotionReviewRecord,
    ) -> anyhow::Result<RulePromotionReviewRecord> {
        postgres_rules::save_rule_promotion_review(self, record).await
    }

    async fn latest_rule_promotion_review(
        &self,
        rule_id: &str,
        rule_version: u32,
    ) -> anyhow::Result<Option<RulePromotionReviewRecord>> {
        postgres_rules::latest_rule_promotion_review(self, rule_id, rule_version).await
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
        postgres_models::list_models(self).await
    }

    async fn save_model_version(
        &self,
        record: ModelVersionRecord,
    ) -> anyhow::Result<ModelVersionRecord> {
        postgres_models::save_model_version(self, record).await
    }

    async fn update_model_status(
        &self,
        model_key: &str,
        model_version: &str,
        status: &str,
    ) -> anyhow::Result<Option<ModelVersionRecord>> {
        postgres_models::update_model_status(self, model_key, model_version, status).await
    }

    async fn model_performance(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Option<ModelPerformanceRecord>> {
        postgres_models::model_performance(self, model_key).await
    }

    async fn save_model_promotion_review(
        &self,
        record: ModelPromotionReviewRecord,
    ) -> anyhow::Result<ModelPromotionReviewRecord> {
        postgres_models::save_model_promotion_review(self, record).await
    }

    async fn latest_model_promotion_review(
        &self,
        model_key: &str,
        model_version: &str,
    ) -> anyhow::Result<Option<ModelPromotionReviewRecord>> {
        postgres_models::latest_model_promotion_review(self, model_key, model_version).await
    }

    async fn save_model_retraining_job(
        &self,
        record: ModelRetrainingJobRecord,
    ) -> anyhow::Result<ModelRetrainingJobRecord> {
        postgres_models::save_model_retraining_job(self, record).await
    }

    async fn list_model_retraining_jobs(
        &self,
        model_key: &str,
    ) -> anyhow::Result<Vec<ModelRetrainingJobRecord>> {
        postgres_models::list_model_retraining_jobs(self, model_key).await
    }

    async fn get_model_retraining_job(
        &self,
        job_id: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        postgres_models::get_model_retraining_job(self, job_id).await
    }

    async fn claim_next_model_retraining_job(
        &self,
        model_key: Option<&str>,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        postgres_models::claim_next_model_retraining_job(self, model_key, actor, status_note).await
    }

    async fn update_model_retraining_job_status(
        &self,
        job_id: &str,
        status: &str,
        actor: &str,
        status_note: &str,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        postgres_models::update_model_retraining_job_status(
            self,
            job_id,
            status,
            actor,
            status_note,
        )
        .await
    }

    async fn complete_model_retraining_job(
        &self,
        input: CompleteModelRetrainingJobInput<'_>,
    ) -> anyhow::Result<Option<ModelRetrainingJobRecord>> {
        postgres_models::complete_model_retraining_job(self, input).await
    }

    async fn dashboard_summary(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<DashboardSummaryRecord> {
        postgres_dashboard::dashboard_summary(self, customer_scope_id).await
    }

    async fn provider_risk_summary(&self) -> anyhow::Result<ProviderRiskSummaryRecord> {
        postgres_providers::provider_risk_summary(self).await
    }

    async fn list_knowledge_cases(&self) -> anyhow::Result<Vec<KnowledgeCaseRecord>> {
        postgres_knowledge::list_knowledge_cases(self).await
    }

    async fn save_knowledge_case(
        &self,
        record: KnowledgeCaseRecord,
    ) -> anyhow::Result<KnowledgeCaseRecord> {
        postgres_knowledge::save_knowledge_case(self, record).await
    }

    async fn search_similar_cases(
        &self,
        query: SimilarCaseQuery,
    ) -> anyhow::Result<Vec<SimilarCaseRecord>> {
        postgres_knowledge::search_similar_cases(self, query).await
    }

    async fn save_agent_run(&self, run: PersistedAgentRun) -> anyhow::Result<()> {
        postgres_agents::save_agent_run(self, run).await
    }

    async fn list_agent_runs(
        &self,
        customer_scope_id: Option<&str>,
    ) -> anyhow::Result<Vec<AgentRunLogRecord>> {
        postgres_agents::list_agent_runs(self, customer_scope_id).await
    }

    async fn save_agent_approval(
        &self,
        approval: AgentApprovalRecord,
    ) -> anyhow::Result<AgentApprovalRecord> {
        postgres_agents::save_agent_approval(self, approval).await
    }

    async fn register_dataset(&self, input: RegisterDatasetInput) -> anyhow::Result<DatasetRecord> {
        postgres_datasets::register_dataset(self, input).await
    }

    async fn list_datasets(&self) -> anyhow::Result<Vec<DatasetRecord>> {
        postgres_datasets::list_datasets(self).await
    }

    async fn get_dataset(&self, dataset_id: &str) -> anyhow::Result<Option<DatasetRecord>> {
        postgres_datasets::get_dataset(self, dataset_id).await
    }

    async fn add_field_mapping(
        &self,
        dataset_id: &str,
        input: CreateFieldMappingInput,
    ) -> anyhow::Result<Option<FieldMappingRecord>> {
        postgres_datasets::add_field_mapping(self, dataset_id, input).await
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
        postgres_audit::claim_audit_history(self, claim_id, customer_scope_id).await
    }

    async fn list_audit_events(
        &self,
        filter: AuditEventListFilter,
    ) -> anyhow::Result<Vec<AuditHistoryEventRecord>> {
        postgres_audit::list_audit_events(self, filter).await
    }

    async fn list_webhook_events(&self) -> anyhow::Result<Vec<WebhookEventRecord>> {
        postgres_webhooks::list_webhook_events(self).await
    }

    async fn save_webhook_delivery_attempt(
        &self,
        input: WebhookDeliveryAttemptInput,
    ) -> anyhow::Result<WebhookDeliveryAttemptRecord> {
        postgres_webhooks::save_webhook_delivery_attempt(self, input).await
    }

    async fn register_feature_set(
        &self,
        input: RegisterFeatureSetInput,
    ) -> anyhow::Result<Option<FeatureSetRecord>> {
        postgres_datasets::register_feature_set(self, input).await
    }

    async fn register_model_dataset(
        &self,
        input: RegisterModelDatasetInput,
    ) -> anyhow::Result<Option<ModelDatasetRecord>> {
        postgres_datasets::register_model_dataset(self, input).await
    }

    async fn get_model_dataset_source_dataset(
        &self,
        model_dataset_id: &str,
    ) -> anyhow::Result<Option<DatasetRecord>> {
        postgres_datasets::get_model_dataset_source_dataset(self, model_dataset_id).await
    }

    async fn register_model_evaluation(
        &self,
        input: RegisterModelEvaluationInput,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>> {
        postgres_datasets::register_model_evaluation(self, input).await
    }

    async fn get_model_evaluation(
        &self,
        evaluation_run_id: &str,
    ) -> anyhow::Result<Option<ModelEvaluationRecord>> {
        postgres_datasets::get_model_evaluation(self, evaluation_run_id).await
    }

    async fn list_model_evaluations(&self) -> anyhow::Result<Vec<ModelEvaluationRecord>> {
        postgres_datasets::list_model_evaluations(self).await
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

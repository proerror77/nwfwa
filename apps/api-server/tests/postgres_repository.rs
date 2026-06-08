use api_server::repository::{
    PersistedAuditEvent, PersistedScoringRun, PostgresScoringRepository, ScoringRepository,
};
use chrono::NaiveDate;
use fwa_core::{
    Claim, ClaimContext, ClaimId, ClaimItem, Member, MemberId, Money, Policy, PolicyId, Provider,
    ProviderId, ProviderRiskTier,
};
use rust_decimal::Decimal;
use sqlx::PgPool;
use testcontainers::{
    core::ContainerAsync,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    GenericImage, ImageExt,
};

#[tokio::test]
async fn postgres_repository_seeds_and_loads_default_rules() -> anyhow::Result<()> {
    let database = migrated_postgres().await?;
    let repository = PostgresScoringRepository::connect(&database.url).await?;
    let rules = repository.list_rules().await?;
    let active_rules = repository.list_active_rules().await?;

    assert!(
        rules.iter().any(|rule| rule.rule_id == "rule_early_claim"),
        "default rule library should be seeded in Postgres"
    );
    assert!(
        active_rules
            .iter()
            .any(|rule| rule.rule_id == "rule_early_claim"),
        "active default rules should be readable from Postgres"
    );

    Ok(())
}

#[tokio::test]
async fn postgres_repository_masks_pii_in_audit_payloads() -> anyhow::Result<()> {
    let database = migrated_postgres().await?;
    let repository = PostgresScoringRepository::connect(&database.url).await?;

    repository
        .save_audit_event(PersistedAuditEvent {
            audit_id: "audit-pii-mask".into(),
            run_id: "run-pii-mask".into(),
            claim_id: "CLM-PII-MASK".into(),
            source_system: "tpa-demo".into(),
            actor_id: "reviewer-1".into(),
            actor_role: "reviewer".into(),
            event_type: "claim.reviewed".into(),
            event_status: "succeeded".into(),
            summary: "reviewed claim".into(),
            payload: serde_json::json!({
                "external_member_id": "MBR-12345",
                "dob": "1988-03-12",
                "gender": "F",
                "nested": {
                    "member_birth_date": "1991-07-09",
                    "member_gender": "M"
                }
            }),
            evidence_refs: vec![serde_json::json!("claims:CLM-PII-MASK")],
        })
        .await?;

    let (payload,): (serde_json::Value,) =
        sqlx::query_as("SELECT payload FROM audit_events WHERE audit_id = $1")
            .bind("audit-pii-mask")
            .fetch_one(&database.pool)
            .await?;

    assert_ne!(payload["external_member_id"], "MBR-12345");
    assert!(payload["external_member_id"]
        .as_str()
        .is_some_and(|value| value.starts_with("sha256:")));
    assert_eq!(payload["dob"], "1988-XX-XX");
    assert_eq!(payload["gender"], "MASKED");
    assert_eq!(payload["nested"]["member_birth_date"], "1991-XX-XX");
    assert_eq!(payload["nested"]["member_gender"], "MASKED");

    Ok(())
}

#[tokio::test]
async fn postgres_repository_persists_claim_context_and_scoring_run_tables() -> anyhow::Result<()> {
    let database = migrated_postgres().await?;
    let repository = PostgresScoringRepository::connect(&database.url).await?;

    let active_rules = repository.list_active_rules().await?;
    let models = repository.list_models().await?;
    assert!(active_rules
        .iter()
        .any(|rule| rule.rule_id == "rule_early_claim"));
    assert!(models
        .iter()
        .any(|model| model.model_key == "baseline_fwa" && model.version == "0.1.0"));

    let context = claim_context("CLM-PG-SCORE-1");
    repository
        .upsert_claim_context(
            context.clone(),
            serde_json::json!({ "source": "postgres_repository_test" }),
        )
        .await?;

    let loaded = repository
        .load_claim_context("CLM-PG-SCORE-1", None)
        .await?
        .expect("claim context should load from Postgres");
    assert_eq!(
        loaded.claim.external_claim_id,
        context.claim.external_claim_id
    );
    assert_eq!(
        loaded.member.external_member_id,
        context.member.external_member_id
    );
    assert_eq!(
        loaded.provider.external_provider_id,
        context.provider.external_provider_id
    );
    assert_eq!(loaded.items.len(), 1);
    assert_eq!(loaded.items[0].item_code, "ITEM-PG-1");

    repository
        .save_scoring_run(PersistedScoringRun {
            run_id: "run-pg-score-1".into(),
            audit_id: "audit-pg-score-1".into(),
            claim_id: "CLM-PG-SCORE-1".into(),
            source_system: "tpa-demo".into(),
            actor_id: "scoring-worker".into(),
            risk_score: 72,
            rag: "Red".into(),
            risk_level: "High".into(),
            recommended_action: "ManualReview".into(),
            confidence_score: 83,
            confidence: "High".into(),
            routing_reason: "test routing reason".into(),
            routing_policy: serde_json::json!({ "policy_id": "default_pre_payment" }),
            score_breakdown: serde_json::json!({ "final_score": 72 }),
            feature_values: vec![serde_json::json!({
                "name": "days_since_policy_start",
                "version": 1,
                "value": 5,
                "evidence_refs": [{ "entity_type": "claim", "entity_id": "CLM-PG-SCORE-1", "field": "service_date" }]
            })],
            rule_runs: vec![serde_json::json!({
                "rule_id": "rule_early_claim",
                "rule_version": 1,
                "score_contribution": 25,
                "alert_code": "EARLY_HIGH_AMOUNT",
                "reason": "early high amount",
                "evidence_refs": ["rules:rule_early_claim:v1"]
            })],
            model_score: serde_json::json!({
                "model_key": "baseline_fwa",
                "model_version": "0.1.0",
                "runtime_kind": "python_http",
                "execution_provider": "cpu",
                "score": 64,
                "label": "HIGH_RISK",
                "explanations": [],
                "latency_ms": 18
            }),
            audit_event: serde_json::json!({
                "claim_id": "CLM-PG-SCORE-1",
                "customer_scope_id": "customer-alpha",
                "risk_score": 72
            }),
            evidence_refs: vec![serde_json::json!("claims:CLM-PG-SCORE-1")],
        })
        .await?;

    let counts: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT
           (SELECT COUNT(*) FROM scoring_runs WHERE run_id = $1),
           (SELECT COUNT(*) FROM feature_values WHERE run_id = $1),
           (SELECT COUNT(*) FROM rule_runs WHERE run_id = $1 AND rule_id IS NOT NULL AND rule_version_id IS NOT NULL),
           (SELECT COUNT(*) FROM model_scores WHERE run_id = $1 AND model_version_id IS NOT NULL),
           (SELECT COUNT(*) FROM audit_events WHERE run_id = $1 AND audit_id = $2)",
    )
    .bind("run-pg-score-1")
    .bind("audit-pg-score-1")
    .fetch_one(&database.pool)
    .await?;

    assert_eq!(counts, (1, 1, 1, 1, 1));

    let scoped = repository
        .load_claim_context("CLM-PG-SCORE-1", Some("customer-alpha"))
        .await?;
    assert!(scoped.is_some());
    let other_scope = repository
        .load_claim_context("CLM-PG-SCORE-1", Some("customer-beta"))
        .await?;
    assert!(other_scope.is_none());

    Ok(())
}

struct MigratedPostgres {
    _container: ContainerAsync<GenericImage>,
    pool: PgPool,
    url: String,
}

fn claim_context(external_claim_id: &str) -> ClaimContext {
    let member_id = MemberId::from_external("member-pg-score-1");
    let policy_id = PolicyId::from_external("policy-pg-score-1");
    let provider_id = ProviderId::from_external("provider-pg-score-1");
    ClaimContext {
        claim: Claim {
            id: ClaimId::from_external(external_claim_id),
            external_claim_id: external_claim_id.into(),
            member_id: member_id.clone(),
            policy_id: policy_id.clone(),
            provider_id: provider_id.clone(),
            diagnosis_code: "J10".into(),
            service_date: NaiveDate::from_ymd_opt(2026, 1, 6).unwrap(),
            amount: Money::new(Decimal::new(8800, 0), "CNY"),
        },
        items: vec![ClaimItem {
            item_code: "ITEM-PG-1".into(),
            item_type: "procedure".into(),
            description: "High cost imaging".into(),
            quantity: 1,
            unit_amount: Money::new(Decimal::new(8800, 0), "CNY"),
            total_amount: Money::new(Decimal::new(8800, 0), "CNY"),
        }],
        member: Member {
            id: member_id.clone(),
            external_member_id: "MBR-PG-SCORE-1".into(),
            dob: Some(NaiveDate::from_ymd_opt(1988, 3, 12).unwrap()),
            gender: Some("F".into()),
        },
        policy: Policy {
            id: policy_id,
            external_policy_id: "POL-PG-SCORE-1".into(),
            member_id,
            product_code: "MED".into(),
            coverage_start_date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            coverage_end_date: NaiveDate::from_ymd_opt(2026, 12, 31).unwrap(),
            coverage_limit: Money::new(Decimal::new(10000, 0), "CNY"),
        },
        provider: Provider {
            id: provider_id,
            external_provider_id: "PRV-PG-SCORE-1".into(),
            name: "Postgres Test Hospital".into(),
            provider_type: "hospital".into(),
            region: "SH".into(),
            risk_tier: ProviderRiskTier::High,
        },
    }
}

async fn migrated_postgres() -> anyhow::Result<MigratedPostgres> {
    let container = GenericImage::new("postgres", "16-alpine")
        .with_exposed_port(5432.tcp())
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_USER", "test")
        .with_env_var("POSTGRES_PASSWORD", "test")
        .with_env_var("POSTGRES_DB", "fwa_test")
        .start()
        .await?;

    let host = container.get_host().await?;
    let port = container.get_host_port_ipv4(5432.tcp()).await?;
    let database_url = format!("postgres://test:test@{host}:{port}/fwa_test");

    let pool = PgPool::connect(&database_url).await?;
    sqlx::migrate!("../../migrations").run(&pool).await?;
    Ok(MigratedPostgres {
        _container: container,
        pool,
        url: database_url,
    })
}

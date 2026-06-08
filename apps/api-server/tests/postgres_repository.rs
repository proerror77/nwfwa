use api_server::repository::{PersistedAuditEvent, PostgresScoringRepository, ScoringRepository};
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

struct MigratedPostgres {
    _container: ContainerAsync<GenericImage>,
    pool: PgPool,
    url: String,
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

use api_server::repository::{PostgresScoringRepository, ScoringRepository};
use sqlx::PgPool;
use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    GenericImage, ImageExt,
};

#[tokio::test]
async fn postgres_repository_seeds_and_loads_default_rules() -> anyhow::Result<()> {
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
    pool.close().await;

    let repository = PostgresScoringRepository::connect(&database_url).await?;
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

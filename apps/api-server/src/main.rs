use api_server::{
    app::{build_app_with_parts, configured_model_scorer, warmup_model_scorer},
    config::AppConfig,
    repository::{InMemoryScoringRepository, PostgresScoringRepository, SharedRepository},
};
use axum::{extract::DefaultBodyLimit, Router};
use std::sync::Arc;
use tower::limit::ConcurrencyLimitLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Respect RUST_LOG for runtime log-level control.  Default to "info" so
    // production logs are not silent when the env var is absent.
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env();
    let repository = configured_repository(&config).await?;
    let scorer = configured_model_scorer(&config)?;
    // Warm up the model artifact cache before serving traffic so the first
    // scoring request doesn't pay the disk-I/O + write-lock cost.
    warmup_model_scorer(&config).await;
    let app = apply_runtime_limits(build_app_with_parts(config, scorer, repository));
    let bind_addr = std::env::var("FWA_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into());
    let listener = tokio::net::TcpListener::bind(bind_addr.as_str()).await?;
    tracing::info!("api-server listening on {}", bind_addr);

    // Graceful shutdown: wait for SIGTERM or Ctrl-C before dropping in-flight
    // requests.  Without this, SIGTERM abruptly terminates active connections.
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("api-server shut down cleanly");
    Ok(())
}

/// Resolves when SIGTERM (Unix) or Ctrl-C is received, whichever comes first.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    {
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install SIGTERM handler")
                .recv()
                .await;
        };
        tokio::select! {
            () = ctrl_c => {},
            () = terminate => {},
        }
    }

    #[cfg(not(unix))]
    ctrl_c.await;

    tracing::info!("shutdown signal received");
}

async fn configured_repository(config: &AppConfig) -> anyhow::Result<SharedRepository> {
    match std::env::var("FWA_REPOSITORY_KIND")
        .unwrap_or_else(|_| "postgres".into())
        .as_str()
    {
        "postgres" => Ok(Arc::new(
            PostgresScoringRepository::connect(&config.database_url).await?,
        )),
        "in_memory" => {
            tracing::warn!("api-server using in-memory repository; data is not durable");
            Ok(InMemoryScoringRepository::shared())
        }
        other => {
            anyhow::bail!("unsupported FWA_REPOSITORY_KIND={other}; expected postgres or in_memory")
        }
    }
}

fn apply_runtime_limits(app: Router) -> Router {
    let max_concurrent_requests = env_usize("FWA_MAX_CONCURRENT_REQUESTS").unwrap_or(256);
    let body_limit_bytes = env_usize("FWA_REQUEST_BODY_LIMIT_BYTES").unwrap_or(2 * 1024 * 1024);

    tracing::info!(
        max_concurrent_requests,
        body_limit_bytes,
        "api-server runtime intake limits configured"
    );

    app.layer(ConcurrencyLimitLayer::new(max_concurrent_requests))
        .layer(DefaultBodyLimit::max(body_limit_bytes))
}

fn env_usize(name: &str) -> Option<usize> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
}

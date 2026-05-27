use api_server::{app::build_app, config::AppConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let app = build_app(AppConfig::default());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    tracing::info!("api-server listening on 127.0.0.1:8080");
    axum::serve(listener, app).await?;
    Ok(())
}

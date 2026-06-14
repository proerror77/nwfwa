#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    worker::commands::dispatch(args).await
}

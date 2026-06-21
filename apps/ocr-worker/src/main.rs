use anyhow::{bail, Context};
use fwa_ocr::{
    HttpDocumentFetcher, HttpOcrProvider, InProcessStub, NoopOcrProvider, OcrProcessingLoop,
    WritebackClient,
};
use std::{sync::Arc, time::Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().init();
    let config = WorkerConfig::from_env()?;
    let queue = config.queue()?;
    let provider = config.provider()?;
    let writeback = Arc::new(WritebackClient::new(config.api_url, config.api_key));
    let loop_runner = OcrProcessingLoop::new(
        queue,
        provider,
        Arc::new(HttpDocumentFetcher::new()),
        writeback,
        Duration::from_millis(config.poll_interval_ms),
    );
    loop_runner
        .run_until_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await;
    Ok(())
}

#[derive(Debug, Clone)]
struct WorkerConfig {
    queue_driver: String,
    ocr_provider: String,
    ocr_endpoint: Option<String>,
    api_url: String,
    api_key: String,
    poll_interval_ms: u64,
}

impl WorkerConfig {
    fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            queue_driver: env_or_default("QUEUE_DRIVER", "stub"),
            ocr_provider: env_or_default("OCR_PROVIDER", "noop"),
            ocr_endpoint: std::env::var("OCR_ENDPOINT").ok(),
            api_url: std::env::var("API_URL").context("API_URL is required")?,
            api_key: std::env::var("API_KEY").context("API_KEY is required")?,
            poll_interval_ms: env_or_default("POLL_INTERVAL_MS", "2000")
                .parse()
                .context("POLL_INTERVAL_MS must be an integer")?,
        })
    }

    fn queue(&self) -> anyhow::Result<Arc<dyn fwa_ocr::MessageQueue>> {
        match self.queue_driver.as_str() {
            "stub" => Ok(Arc::new(InProcessStub::default())),
            "redis" | "sqs" => bail!(
                "QUEUE_DRIVER={} is reserved for customer queue adapters and is not wired in this build",
                self.queue_driver
            ),
            value => bail!("unsupported QUEUE_DRIVER: {value}"),
        }
    }

    fn provider(&self) -> anyhow::Result<Arc<dyn fwa_ocr::OcrProvider>> {
        match self.ocr_provider.as_str() {
            "noop" => Ok(Arc::new(NoopOcrProvider)),
            "http" => Ok(Arc::new(HttpOcrProvider::new(required(
                "OCR_ENDPOINT",
                self.ocr_endpoint.as_deref(),
            )?))),
            value => bail!("unsupported OCR_PROVIDER: {value}"),
        }
    }
}

fn env_or_default(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.into())
}

fn required(name: &str, value: Option<&str>) -> anyhow::Result<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .with_context(|| format!("{name} is required"))
}

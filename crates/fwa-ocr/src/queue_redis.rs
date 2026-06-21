use crate::{MessageQueue, OcrTask};
use anyhow::bail;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct RedisQueue {
    pub url: String,
}

impl RedisQueue {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
}

#[async_trait]
impl MessageQueue for RedisQueue {
    async fn receive(&self) -> anyhow::Result<Option<OcrTask>> {
        bail!("QUEUE_DRIVER=redis is not wired in this build; provide a client-backed RedisQueue implementation for the customer environment")
    }

    async fn ack(&self, _receipt: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn nack(&self, _receipt: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

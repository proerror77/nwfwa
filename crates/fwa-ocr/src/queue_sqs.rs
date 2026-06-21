use crate::{MessageQueue, OcrTask};
use anyhow::bail;
use async_trait::async_trait;

#[derive(Debug, Clone)]
pub struct SqsQueue {
    pub queue_url: String,
}

impl SqsQueue {
    pub fn new(queue_url: impl Into<String>) -> Self {
        Self {
            queue_url: queue_url.into(),
        }
    }
}

#[async_trait]
impl MessageQueue for SqsQueue {
    async fn receive(&self) -> anyhow::Result<Option<OcrTask>> {
        bail!("QUEUE_DRIVER=sqs is not wired in this build; provide an AWS SDK backed SqsQueue implementation for the customer environment")
    }

    async fn ack(&self, _receipt: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn nack(&self, _receipt: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

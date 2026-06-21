use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OcrTask {
    pub receipt: String,
    pub document_id: String,
    pub storage_uri: String,
    pub customer_scope_id: String,
    pub claim_id: Option<String>,
}

#[async_trait]
pub trait MessageQueue: Send + Sync {
    async fn receive(&self) -> anyhow::Result<Option<OcrTask>>;
    async fn ack(&self, receipt: &str) -> anyhow::Result<()>;
    async fn nack(&self, receipt: &str) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, Default)]
pub struct InProcessStub {
    tasks: Arc<Mutex<VecDeque<OcrTask>>>,
    acked: Arc<Mutex<Vec<String>>>,
    nacked: Arc<Mutex<Vec<String>>>,
}

impl InProcessStub {
    pub fn new(tasks: Vec<OcrTask>) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(tasks.into())),
            acked: Arc::default(),
            nacked: Arc::default(),
        }
    }

    pub async fn acked_receipts(&self) -> Vec<String> {
        self.acked.lock().await.clone()
    }

    pub async fn nacked_receipts(&self) -> Vec<String> {
        self.nacked.lock().await.clone()
    }
}

#[async_trait]
impl MessageQueue for InProcessStub {
    async fn receive(&self) -> anyhow::Result<Option<OcrTask>> {
        Ok(self.tasks.lock().await.pop_front())
    }

    async fn ack(&self, receipt: &str) -> anyhow::Result<()> {
        self.acked.lock().await.push(receipt.to_string());
        Ok(())
    }

    async fn nack(&self, receipt: &str) -> anyhow::Result<()> {
        self.nacked.lock().await.push(receipt.to_string());
        Ok(())
    }
}

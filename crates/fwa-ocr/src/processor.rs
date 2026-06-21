use crate::{DocumentFetcher, MessageQueue, OcrProvider, OcrWriteback};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use tracing::{error, info};
use uuid::Uuid;

pub struct OcrProcessingLoop {
    queue: Arc<dyn MessageQueue>,
    provider: Arc<dyn OcrProvider>,
    fetcher: Arc<dyn DocumentFetcher>,
    writeback: Arc<dyn OcrWriteback>,
    poll_interval: Duration,
}

impl OcrProcessingLoop {
    pub fn new(
        queue: Arc<dyn MessageQueue>,
        provider: Arc<dyn OcrProvider>,
        fetcher: Arc<dyn DocumentFetcher>,
        writeback: Arc<dyn OcrWriteback>,
        poll_interval: Duration,
    ) -> Self {
        Self {
            queue,
            provider,
            fetcher,
            writeback,
            poll_interval,
        }
    }

    pub async fn run_until_shutdown(self, shutdown: impl std::future::Future<Output = ()>) {
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                _ = &mut shutdown => {
                    info!("ocr worker shutdown requested");
                    return;
                }
                _ = self.process_one_or_sleep() => {}
            }
        }
    }

    pub async fn process_one_or_sleep(&self) {
        match self.queue.receive().await {
            Ok(Some(task)) => {
                if let Err(error) = self.process_task(task).await {
                    error!(?error, "ocr task failed");
                }
            }
            Ok(None) => tokio::time::sleep(self.poll_interval).await,
            Err(error) => {
                error!(?error, "ocr queue receive failed");
                tokio::time::sleep(self.poll_interval).await;
            }
        }
    }

    pub async fn process_available(&self) {
        while let Ok(Some(task)) = self.queue.receive().await {
            if let Err(error) = self.process_task(task).await {
                error!(?error, "ocr task failed");
            }
        }
    }

    async fn process_task(&self, task: crate::OcrTask) -> anyhow::Result<()> {
        let receipt = task.receipt.clone();
        let result = async {
            let bytes = self.fetcher.fetch(&task.storage_uri).await?;
            let ocr_result = self
                .provider
                .process(&bytes, infer_mime(&task.storage_uri))
                .await?;
            let ocr_output_id = Uuid::new_v4().to_string();
            let output_uri =
                inline_output_uri(&task.document_id, &ocr_output_id, &ocr_result.output_text);
            let output_checksum = output_checksum(&ocr_result.output_text);
            self.writeback
                .post_ocr_output(
                    &task,
                    &ocr_result,
                    &output_uri,
                    &output_checksum,
                    &ocr_output_id,
                )
                .await
        }
        .await;

        match result {
            Ok(()) => self.queue.ack(&receipt).await?,
            Err(error) => {
                self.queue.nack(&receipt).await?;
                return Err(error);
            }
        }
        Ok(())
    }
}

pub fn infer_mime(storage_uri: &str) -> &'static str {
    let lower = storage_uri.to_ascii_lowercase();
    if lower.ends_with(".pdf") {
        "application/pdf"
    } else if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "application/octet-stream"
    }
}

pub fn inline_output_uri(document_id: &str, ocr_output_id: &str, output_text: &str) -> String {
    let encoded = URL_SAFE_NO_PAD.encode(output_text.as_bytes());
    format!("ocr://inline/{document_id}/{ocr_output_id}?text={encoded}")
}

fn output_checksum(output_text: &str) -> String {
    let digest = Sha256::digest(output_text.as_bytes());
    format!("sha256:{digest:x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InProcessStub, NoopOcrProvider, OcrResult, OcrTask};
    use anyhow::bail;
    use async_trait::async_trait;
    use tokio::sync::Mutex;

    #[derive(Default)]
    struct StaticFetcher;

    #[async_trait]
    impl DocumentFetcher for StaticFetcher {
        async fn fetch(&self, _storage_uri: &str) -> anyhow::Result<Vec<u8>> {
            Ok(b"document bytes".to_vec())
        }
    }

    #[derive(Default)]
    struct RecordingWriteback {
        calls: Mutex<Vec<String>>,
        fail: bool,
    }

    #[async_trait]
    impl crate::OcrWriteback for RecordingWriteback {
        async fn post_ocr_output(
            &self,
            task: &crate::OcrTask,
            _result: &OcrResult,
            _output_uri: &str,
            _output_checksum: &str,
            _ocr_output_id: &str,
        ) -> anyhow::Result<()> {
            self.calls.lock().await.push(task.document_id.clone());
            if self.fail {
                bail!("writeback failed");
            }
            Ok(())
        }
    }

    fn task(receipt: &str, document_id: &str) -> OcrTask {
        OcrTask {
            receipt: receipt.into(),
            document_id: document_id.into(),
            storage_uri: "https://example.test/document.pdf".into(),
            customer_scope_id: "demo-customer".into(),
            claim_id: None,
        }
    }

    #[tokio::test]
    async fn processing_loop_acks_successful_task() {
        let queue = InProcessStub::new(vec![task("receipt-1", "doc-1")]);
        let writeback = Arc::new(RecordingWriteback::default());
        let loop_runner = OcrProcessingLoop::new(
            Arc::new(queue.clone()),
            Arc::new(NoopOcrProvider),
            Arc::new(StaticFetcher),
            writeback,
            Duration::from_millis(1),
        );

        loop_runner.process_available().await;

        assert_eq!(queue.acked_receipts().await, vec!["receipt-1"]);
        assert!(queue.nacked_receipts().await.is_empty());
    }

    #[tokio::test]
    async fn processing_loop_nacks_writeback_error_and_continues() {
        let queue =
            InProcessStub::new(vec![task("receipt-1", "doc-1"), task("receipt-2", "doc-2")]);
        let failing_writeback = Arc::new(RecordingWriteback {
            fail: true,
            ..Default::default()
        });
        let loop_runner = OcrProcessingLoop::new(
            Arc::new(queue.clone()),
            Arc::new(NoopOcrProvider),
            Arc::new(StaticFetcher),
            failing_writeback,
            Duration::from_millis(1),
        );

        loop_runner.process_available().await;

        assert!(queue.acked_receipts().await.is_empty());
        assert_eq!(
            queue.nacked_receipts().await,
            vec!["receipt-1", "receipt-2"]
        );
    }

    #[test]
    fn inline_output_uri_keeps_document_and_output_ids() {
        let uri = inline_output_uri("doc-1", "ocr-1", "hello world");

        assert!(uri.starts_with("ocr://inline/doc-1/ocr-1?text="));
        assert!(uri.contains("aGVsbG8gd29ybGQ"));
    }
}

use crate::{OcrResult, OcrTask};
use anyhow::bail;
use async_trait::async_trait;
use serde::Serialize;

#[async_trait]
pub trait OcrWriteback: Send + Sync {
    async fn post_ocr_output(
        &self,
        task: &OcrTask,
        result: &OcrResult,
        output_uri: &str,
        output_checksum: &str,
        ocr_output_id: &str,
    ) -> anyhow::Result<()>;
}

#[derive(Debug, Clone)]
pub struct WritebackClient {
    client: reqwest::Client,
    api_url: String,
    api_key: String,
}

impl WritebackClient {
    pub fn new(api_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_url: api_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
        }
    }
}

#[derive(Debug, Serialize)]
struct CreateEvidenceOcrOutputRequest<'a> {
    ocr_output_id: &'a str,
    ocr_engine: &'a str,
    ocr_engine_version: &'a str,
    output_uri: &'a str,
    output_checksum: &'a str,
    confidence_score: Option<String>,
    quality_status: &'a str,
    evidence_refs: Vec<String>,
}

#[async_trait]
impl OcrWriteback for WritebackClient {
    async fn post_ocr_output(
        &self,
        task: &OcrTask,
        result: &OcrResult,
        output_uri: &str,
        output_checksum: &str,
        ocr_output_id: &str,
    ) -> anyhow::Result<()> {
        let mut url = reqwest::Url::parse(&self.api_url)?;
        url.path_segments_mut()
            .map_err(|_| anyhow::anyhow!("API_URL cannot be a base URL"))?
            .extend([
                "api",
                "v1",
                "ops",
                "evidence",
                "documents",
                &task.document_id,
                "ocr-outputs",
            ]);
        let request = CreateEvidenceOcrOutputRequest {
            ocr_output_id,
            ocr_engine: &result.engine,
            ocr_engine_version: &result.engine_version,
            output_uri,
            output_checksum,
            confidence_score: result.confidence_score.map(|value| value.to_string()),
            quality_status: &result.quality_status,
            evidence_refs: vec![format!("evidence_documents:{}", task.document_id)],
        };
        let response = self
            .client
            .post(url)
            .header("x-api-key", &self.api_key)
            .json(&request)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            bail!("OCR writeback failed with status {status}: {body}");
        }
        Ok(())
    }
}

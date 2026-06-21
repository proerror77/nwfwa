use anyhow::bail;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrResult {
    pub engine: String,
    pub engine_version: String,
    pub output_text: String,
    pub confidence_score: Option<f64>,
    pub quality_status: String,
}

#[async_trait]
pub trait OcrProvider: Send + Sync {
    async fn process(&self, document_bytes: &[u8], mime_type: &str) -> anyhow::Result<OcrResult>;
}

#[derive(Debug, Clone, Default)]
pub struct NoopOcrProvider;

#[async_trait]
impl OcrProvider for NoopOcrProvider {
    async fn process(&self, _document_bytes: &[u8], _mime_type: &str) -> anyhow::Result<OcrResult> {
        Ok(OcrResult {
            engine: "noop".into(),
            engine_version: "noop-v1".into(),
            output_text: "[noop ocr output]".into(),
            confidence_score: Some(1.0),
            quality_status: "accepted".into(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct HttpOcrProvider {
    client: reqwest::Client,
    endpoint: String,
}

impl HttpOcrProvider {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoint: endpoint.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct HttpOcrResponse {
    text: String,
    confidence: Option<f64>,
    engine_version: String,
    #[serde(default)]
    engine: Option<String>,
}

#[async_trait]
impl OcrProvider for HttpOcrProvider {
    async fn process(&self, document_bytes: &[u8], mime_type: &str) -> anyhow::Result<OcrResult> {
        let part = reqwest::multipart::Part::bytes(document_bytes.to_vec())
            .mime_str(mime_type)?
            .file_name("document");
        let form = reqwest::multipart::Form::new().part("file", part);
        let response = self
            .client
            .post(&self.endpoint)
            .multipart(form)
            .send()
            .await?;
        let status = response.status();
        if !status.is_success() {
            bail!("OCR provider failed with status {status}");
        }
        let body = response.json::<HttpOcrResponse>().await?;
        Ok(OcrResult {
            engine: body.engine.unwrap_or_else(|| "http-ocr".into()),
            engine_version: body.engine_version,
            output_text: body.text,
            confidence_score: body.confidence,
            quality_status: quality_status(body.confidence),
        })
    }
}

fn quality_status(confidence: Option<f64>) -> String {
    match confidence {
        Some(value) if value < 0.70 => "low_confidence".into(),
        Some(_) | None => "accepted".into(),
    }
}

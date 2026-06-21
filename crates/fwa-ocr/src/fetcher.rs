use anyhow::bail;
use async_trait::async_trait;

#[async_trait]
pub trait DocumentFetcher: Send + Sync {
    async fn fetch(&self, storage_uri: &str) -> anyhow::Result<Vec<u8>>;
}

#[derive(Debug, Clone, Default)]
pub struct HttpDocumentFetcher {
    client: reqwest::Client,
}

impl HttpDocumentFetcher {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl DocumentFetcher for HttpDocumentFetcher {
    async fn fetch(&self, storage_uri: &str) -> anyhow::Result<Vec<u8>> {
        if !storage_uri.starts_with("http://") && !storage_uri.starts_with("https://") {
            bail!("only http(s) evidence document downloads are supported by HttpDocumentFetcher");
        }
        let response = self.client.get(storage_uri).send().await?;
        let status = response.status();
        if !status.is_success() {
            bail!("document fetch failed with status {status}");
        }
        Ok(response.bytes().await?.to_vec())
    }
}

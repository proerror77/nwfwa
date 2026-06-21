mod fetcher;
mod processor;
mod provider;
mod queue;
mod queue_redis;
mod queue_sqs;
mod writeback;

pub use fetcher::{DocumentFetcher, HttpDocumentFetcher};
pub use processor::{infer_mime, inline_output_uri, OcrProcessingLoop};
pub use provider::{HttpOcrProvider, NoopOcrProvider, OcrProvider, OcrResult};
pub use queue::{InProcessStub, MessageQueue, OcrTask};
pub use queue_redis::RedisQueue;
pub use queue_sqs::SqsQueue;
pub use writeback::{OcrWriteback, WritebackClient};

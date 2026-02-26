use async_trait::async_trait;

use crate::agent::RawDocument;
use crate::entity::ExtractionResult;
use crate::error::Result;

#[async_trait]
pub trait ExtractionPipeline: Send + Sync {
    async fn extract(&self, document: &RawDocument) -> Result<ExtractionResult>;
    async fn extract_batch(&self, documents: &[RawDocument]) -> Result<Vec<ExtractionResult>>;
}

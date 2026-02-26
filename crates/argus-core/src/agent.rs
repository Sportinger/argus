use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawDocument {
    pub source: String,
    pub source_id: String,
    pub title: Option<String>,
    pub content: String,
    pub url: Option<String>,
    pub collected_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub name: String,
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub documents_collected: u64,
    pub error: Option<String>,
}

#[async_trait]
pub trait Agent: Send + Sync {
    fn name(&self) -> &str;
    fn source_type(&self) -> &str;
    async fn collect(&self) -> Result<Vec<RawDocument>>;
    async fn status(&self) -> AgentStatus;
}

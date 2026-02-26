use std::any::Any;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::entity::EntityType;
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

    /// Downcast support for cross-referencing between agents.
    fn as_any(&self) -> &dyn Any;
}

/// Trait for agents that can look up entities from their data source.
/// Used for cross-referencing: when one agent discovers an entity,
/// other agents can be queried to enrich it with additional data.
#[async_trait]
pub trait AgentLookup: Send + Sync {
    /// Which entity types can this agent look up?
    fn can_lookup(&self, entity_type: &EntityType) -> bool;

    /// Search for a specific entity by name and type at this data source.
    async fn lookup(&self, name: &str, entity_type: &EntityType) -> Result<Vec<RawDocument>>;
}

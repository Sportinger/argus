use async_trait::async_trait;
use uuid::Uuid;

use crate::entity::{Entity, ExtractionResult, Relationship};
use crate::error::Result;

#[derive(Debug, Clone)]
pub struct GraphQuery {
    pub cypher: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphNeighbors {
    pub entity: Entity,
    pub relationships: Vec<Relationship>,
    pub neighbors: Vec<Entity>,
}

#[async_trait]
pub trait GraphStore: Send + Sync {
    async fn store_extraction(&self, result: &ExtractionResult) -> Result<()>;
    async fn get_entity(&self, id: Uuid) -> Result<Option<Entity>>;
    async fn search_entities(&self, query: &str, limit: usize) -> Result<Vec<Entity>>;
    async fn get_neighbors(&self, entity_id: Uuid, depth: u32) -> Result<GraphNeighbors>;
    async fn execute_cypher(&self, query: &GraphQuery) -> Result<serde_json::Value>;
    async fn entity_count(&self) -> Result<u64>;
    async fn relationship_count(&self) -> Result<u64>;
}

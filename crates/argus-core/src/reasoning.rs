use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::entity::Entity;
use crate::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningQuery {
    pub question: String,
    pub context: Option<String>,
    pub max_hops: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub description: String,
    pub cypher: Option<String>,
    pub result_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningResponse {
    pub answer: String,
    pub confidence: f64,
    pub steps: Vec<ReasoningStep>,
    pub entities_referenced: Vec<Entity>,
    pub sources: Vec<String>,
}

#[async_trait]
pub trait ReasoningEngine: Send + Sync {
    async fn query(&self, query: &ReasoningQuery) -> Result<ReasoningResponse>;
}

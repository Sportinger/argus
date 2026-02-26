use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::agent::AgentStatus;
use crate::entity::{Entity, EntityType, Relationship};
use crate::reasoning::{ReasoningResponse, ReasoningStep};

// --- Health ---

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub neo4j_connected: bool,
    pub qdrant_connected: bool,
    pub entity_count: u64,
    pub relationship_count: u64,
}

// --- Agents ---

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentListResponse {
    pub agents: Vec<AgentStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentTriggerRequest {
    pub agent_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentTriggerResponse {
    pub run_id: String,
    pub agent_name: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunStatus {
    pub run_id: String,
    pub agent_name: String,
    pub status: AgentRunState,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub documents_collected: u64,
    pub entities_extracted: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunState {
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentRunsResponse {
    pub runs: Vec<AgentRunStatus>,
}

// --- Entities ---

#[derive(Debug, Serialize, Deserialize)]
pub struct EntitySearchRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    pub entity_type: Option<EntityType>,
}

fn default_limit() -> usize {
    20
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntitySearchResponse {
    pub entities: Vec<Entity>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityDetailResponse {
    pub entity: Entity,
    pub relationships: Vec<Relationship>,
    pub neighbors: Vec<Entity>,
}

// --- Graph ---

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphQueryRequest {
    pub cypher: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphQueryResponse {
    pub result: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphStatsResponse {
    pub entity_count: u64,
    pub relationship_count: u64,
    pub entity_types: Vec<EntityTypeStat>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EntityTypeStat {
    pub entity_type: EntityType,
    pub count: u64,
}

// --- Reasoning ---

#[derive(Debug, Serialize, Deserialize)]
pub struct ReasoningRequest {
    pub question: String,
    pub context: Option<String>,
    pub max_hops: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReasoningApiResponse {
    pub answer: String,
    pub confidence: f64,
    pub steps: Vec<ReasoningStep>,
    pub entities_referenced: Vec<Entity>,
    pub sources: Vec<String>,
}

impl From<ReasoningResponse> for ReasoningApiResponse {
    fn from(r: ReasoningResponse) -> Self {
        Self {
            answer: r.answer,
            confidence: r.confidence,
            steps: r.steps,
            entities_referenced: r.entities_referenced,
            sources: r.sources,
        }
    }
}

// --- Timeline ---

#[derive(Debug, Serialize, Deserialize)]
pub struct TimelineRequest {
    pub entity_id: Option<Uuid>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub timestamp: DateTime<Utc>,
    pub entity: Entity,
    pub event_type: String,
    pub description: String,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimelineResponse {
    pub events: Vec<TimelineEvent>,
}

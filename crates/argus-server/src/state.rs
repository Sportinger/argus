use std::collections::HashMap;
use std::sync::Arc;

use argus_core::{Agent, AppConfig};
use argus_extraction::LlmExtractionPipeline;
use argus_graph::Neo4jGraphStore;
use argus_reasoning::LlmReasoningEngine;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub agents: HashMap<String, Arc<dyn Agent>>,
    pub graph: Arc<Neo4jGraphStore>,
    pub extraction: Arc<LlmExtractionPipeline>,
    pub reasoning: Arc<LlmReasoningEngine>,
}

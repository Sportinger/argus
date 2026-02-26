use axum::{
    routing::{get, post},
    Router,
};

use crate::handlers;
use crate::state::AppState;

pub fn create_router() -> Router<AppState> {
    Router::new()
        // Health
        .route("/api/health", get(handlers::health::health_check))
        // Agents
        .route("/api/agents", get(handlers::agents::list_agents))
        .route("/api/agents/trigger", post(handlers::agents::trigger_agent))
        .route("/api/agents/runs", get(handlers::agents::list_runs))
        // Entities
        .route("/api/entities/search", post(handlers::entities::search_entities))
        .route("/api/entities/{id}", get(handlers::entities::get_entity))
        // Graph
        .route("/api/graph/query", post(handlers::graph::query_graph))
        .route("/api/graph/stats", get(handlers::graph::graph_stats))
        .route("/api/graph/neighbors/{id}", get(handlers::graph::get_neighbors))
        // Reasoning
        .route("/api/reasoning/query", post(handlers::reasoning::query_reasoning))
        // Timeline
        .route("/api/timeline", post(handlers::entities::get_timeline))
}

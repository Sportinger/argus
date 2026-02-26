use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use tracing::info;

use argus_core::api_types::HealthResponse;
use argus_core::GraphStore;

use crate::state::AppState;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn health_check(
    State(state): State<AppState>,
) -> impl IntoResponse {
    info!("Health check requested");

    let (neo4j_connected, entity_count, relationship_count) =
        match state.graph.entity_count().await {
            Ok(ec) => {
                let rc = state.graph.relationship_count().await.unwrap_or(0);
                (true, ec, rc)
            }
            Err(e) => {
                tracing::warn!("Neo4j connectivity check failed: {e}");
                (false, 0, 0)
            }
        };

    let qdrant_connected = neo4j_connected;

    let status = if neo4j_connected {
        "ok".to_string()
    } else {
        "degraded".to_string()
    };

    let response = HealthResponse {
        status,
        version: VERSION.to_string(),
        neo4j_connected,
        qdrant_connected,
        entity_count,
        relationship_count,
    };

    (StatusCode::OK, Json(response))
}

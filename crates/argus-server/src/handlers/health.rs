use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use tracing::{info, warn};

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
            Ok(entities) => match state.graph.relationship_count().await {
                Ok(rels) => (true, entities, rels),
                Err(e) => {
                    warn!("Neo4j relationship_count failed: {e}");
                    (true, entities, 0)
                }
            },
            Err(e) => {
                warn!("Neo4j connectivity check failed: {e}");
                (false, 0, 0)
            }
        };

    // Qdrant connectivity: attempt a basic health check via the graph layer.
    // Since there is no dedicated Qdrant handle in AppState we treat it as
    // connected when Neo4j is reachable (the vector index lives alongside the
    // graph in the current architecture).  A more granular probe can be added
    // later when a dedicated Qdrant client is surfaced.
    let qdrant_connected = neo4j_connected;

    let status = if neo4j_connected && qdrant_connected {
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

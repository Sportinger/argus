use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use tracing::{error, info};
use uuid::Uuid;

use argus_core::api_types::{
    EntityDetailResponse, EntityTypeStat, GraphQueryRequest, GraphQueryResponse,
    GraphStatsResponse,
};
use argus_core::{EntityType, GraphQuery, GraphStore};

use crate::state::AppState;

pub async fn query_graph(
    State(state): State<AppState>,
    Json(request): Json<GraphQueryRequest>,
) -> impl IntoResponse {
    info!(cypher = %request.cypher, "Executing graph query");

    let query = GraphQuery {
        cypher: request.cypher,
        params: request.params,
    };

    match state.graph.execute_cypher(&query).await {
        Ok(result) => {
            let response = GraphQueryResponse { result };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            error!("Graph query failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Query failed: {e}") })),
            )
                .into_response()
        }
    }
}

pub async fn graph_stats(State(state): State<AppState>) -> impl IntoResponse {
    info!("Fetching graph statistics");

    let entity_count = match state.graph.entity_count().await {
        Ok(count) => count,
        Err(e) => {
            error!("Failed to get entity count: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to get entity count: {e}") })),
            )
                .into_response();
        }
    };

    let relationship_count = match state.graph.relationship_count().await {
        Ok(count) => count,
        Err(e) => {
            error!("Failed to get relationship count: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to get relationship count: {e}") })),
            )
                .into_response();
        }
    };

    // Query per-type entity counts via Cypher
    let entity_types = fetch_entity_type_stats(&state).await;

    let response = GraphStatsResponse {
        entity_count,
        relationship_count,
        entity_types,
    };

    (StatusCode::OK, Json(response)).into_response()
}

async fn fetch_entity_type_stats(state: &AppState) -> Vec<EntityTypeStat> {
    let query = GraphQuery {
        cypher: "MATCH (e:Entity) RETURN e.entity_type AS entity_type, count(e) AS count"
            .to_string(),
        params: serde_json::json!({}),
    };

    let result = match state.graph.execute_cypher(&query).await {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("Failed to fetch entity type stats: {e}");
            return Vec::new();
        }
    };

    let mut stats = Vec::new();

    if let Some(rows) = result.as_array() {
        for row in rows {
            let type_str = match row.get("entity_type").and_then(|v| v.as_str()) {
                Some(s) => s,
                None => continue,
            };

            let count = row
                .get("count")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            let entity_type = match type_str {
                "person" => EntityType::Person,
                "organization" => EntityType::Organization,
                "vessel" => EntityType::Vessel,
                "aircraft" => EntityType::Aircraft,
                "location" => EntityType::Location,
                "event" => EntityType::Event,
                "document" => EntityType::Document,
                "transaction" => EntityType::Transaction,
                "sanction" => EntityType::Sanction,
                _ => continue,
            };

            stats.push(EntityTypeStat { entity_type, count });
        }
    }

    stats
}

pub async fn get_neighbors(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    info!(%id, "Fetching neighbors");

    // First retrieve the entity itself
    let entity = match state.graph.get_entity(id).await {
        Ok(Some(entity)) => entity,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": format!("Entity {id} not found") })),
            )
                .into_response();
        }
        Err(e) => {
            error!("Failed to fetch entity {id}: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to fetch entity: {e}") })),
            )
                .into_response();
        }
    };

    match state.graph.get_neighbors(id, 1).await {
        Ok(neighbors_result) => {
            let response = EntityDetailResponse {
                entity,
                relationships: neighbors_result.relationships,
                neighbors: neighbors_result.neighbors,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            error!("Failed to fetch neighbors for entity {id}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Failed to fetch neighbors: {e}") })),
            )
                .into_response()
        }
    }
}

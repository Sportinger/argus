use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use tracing::{error, info};
use uuid::Uuid;

use argus_core::api_types::{
    EntityDetailResponse, EntitySearchRequest, EntitySearchResponse, TimelineEvent,
    TimelineRequest, TimelineResponse,
};
use argus_core::{GraphQuery, GraphStore};

use crate::state::AppState;

pub async fn search_entities(
    State(state): State<AppState>,
    Json(request): Json<EntitySearchRequest>,
) -> impl IntoResponse {
    info!(query = %request.query, limit = request.limit, "Searching entities");

    match state
        .graph
        .search_entities(&request.query, request.limit)
        .await
    {
        Ok(mut entities) => {
            // Filter by entity type if specified
            if let Some(ref et) = request.entity_type {
                entities.retain(|e| &e.entity_type == et);
            }

            let total = entities.len();
            let response = EntitySearchResponse { entities, total };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            error!("Entity search failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Search failed: {e}") })),
            )
                .into_response()
        }
    }
}

pub async fn get_entity(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    info!(%id, "Fetching entity");

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
            // Return the entity even if neighbors fail
            let response = EntityDetailResponse {
                entity,
                relationships: Vec::new(),
                neighbors: Vec::new(),
            };
            (StatusCode::OK, Json(response)).into_response()
        }
    }
}

pub async fn get_timeline(
    State(state): State<AppState>,
    Json(request): Json<TimelineRequest>,
) -> impl IntoResponse {
    info!(
        entity_id = ?request.entity_id,
        start = ?request.start,
        end = ?request.end,
        limit = request.limit,
        "Fetching timeline"
    );

    // Build a Cypher query for time-ordered events.
    // If an entity_id is provided, filter to events connected to that entity.
    let (cypher, params) = if let Some(entity_id) = request.entity_id {
        let mut conditions = vec!["(e)-[]->(ev)".to_string(), format!("e.id = '{entity_id}'")];

        if let Some(ref start) = request.start {
            conditions.push(format!("ev.timestamp >= datetime('{}')", start.to_rfc3339()));
        }
        if let Some(ref end) = request.end {
            conditions.push(format!("ev.timestamp <= datetime('{}')", end.to_rfc3339()));
        }

        let cypher = format!(
            "MATCH (e:Entity)-[r]->(ev:Entity) \
             WHERE e.id = $entity_id \
             {} \
             RETURN ev, type(r) as event_type, e \
             ORDER BY ev.last_seen DESC \
             LIMIT $limit",
            if request.start.is_some() || request.end.is_some() {
                let mut time_filter = String::new();
                if let Some(ref start) = request.start {
                    time_filter
                        .push_str(&format!("AND ev.last_seen >= datetime('{}')", start.to_rfc3339()));
                }
                if let Some(ref end) = request.end {
                    if !time_filter.is_empty() {
                        time_filter.push(' ');
                    }
                    time_filter
                        .push_str(&format!("AND ev.last_seen <= datetime('{}')", end.to_rfc3339()));
                }
                time_filter
            } else {
                String::new()
            }
        );

        let params = serde_json::json!({
            "entity_id": entity_id.to_string(),
            "limit": request.limit,
        });

        (cypher, params)
    } else {
        let mut time_filter = String::new();
        if let Some(ref start) = request.start {
            time_filter.push_str(&format!(
                "WHERE e.last_seen >= datetime('{}')",
                start.to_rfc3339()
            ));
        }
        if let Some(ref end) = request.end {
            if time_filter.is_empty() {
                time_filter.push_str(&format!(
                    "WHERE e.last_seen <= datetime('{}')",
                    end.to_rfc3339()
                ));
            } else {
                time_filter.push_str(&format!(
                    " AND e.last_seen <= datetime('{}')",
                    end.to_rfc3339()
                ));
            }
        }

        let cypher = format!(
            "MATCH (e:Entity) \
             {time_filter} \
             RETURN e \
             ORDER BY e.last_seen DESC \
             LIMIT $limit"
        );

        let params = serde_json::json!({
            "limit": request.limit,
        });

        (cypher, params)
    };

    let query = GraphQuery { cypher, params };

    match state.graph.execute_cypher(&query).await {
        Ok(result) => {
            // Parse the Cypher result into TimelineEvent structs.
            // The result format depends on the Neo4j driver; we do a
            // best-effort conversion here.
            let events = parse_timeline_events(&result);
            let response = TimelineResponse { events };
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            error!("Timeline query failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("Timeline query failed: {e}") })),
            )
                .into_response()
        }
    }
}

/// Best-effort parse of Cypher result JSON into timeline events.
fn parse_timeline_events(result: &serde_json::Value) -> Vec<TimelineEvent> {
    let mut events = Vec::new();

    let rows = match result.as_array() {
        Some(arr) => arr,
        None => return events,
    };

    for row in rows {
        // Try to extract entity data from the result row
        let entity_value = row.get("e").or_else(|| row.get("ev")).unwrap_or(row);

        let entity: argus_core::Entity = match serde_json::from_value(entity_value.clone()) {
            Ok(e) => e,
            Err(_) => continue,
        };

        let event_type = row
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or("observation")
            .to_string();

        let description = format!("{} — {}", entity.name, entity.source);

        let event = TimelineEvent {
            timestamp: entity.last_seen,
            entity,
            event_type,
            description,
            source: "graph".to_string(),
        };

        events.push(event);
    }

    events
}

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use tracing::{info, warn, error};

use argus_core::api_types::{AgentListResponse, AgentTriggerRequest, AgentTriggerResponse};
use argus_core::{ExtractionPipeline, GraphStore};

use crate::state::AppState;

/// GET /api/agents — list all registered agents with their current status.
pub async fn list_agents(State(state): State<AppState>) -> impl IntoResponse {
    info!("Listing all agents");

    let mut statuses = Vec::with_capacity(state.agents.len());

    for (_name, agent) in &state.agents {
        match agent.status().await {
            status => statuses.push(status),
        }
    }

    (StatusCode::OK, Json(AgentListResponse { agents: statuses }))
}

/// POST /api/agents/trigger — trigger a named agent to collect, extract, and store data.
pub async fn trigger_agent(
    State(state): State<AppState>,
    Json(req): Json<AgentTriggerRequest>,
) -> impl IntoResponse {
    info!(agent_name = %req.agent_name, "Triggering agent");

    // Look up the agent by name
    let agent = match state.agents.get(&req.agent_name) {
        Some(agent) => agent,
        None => {
            warn!(agent_name = %req.agent_name, "Agent not found");
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("Agent '{}' not found", req.agent_name)
                })),
            )
                .into_response();
        }
    };

    // Collect documents from the agent's data source
    let documents = match agent.collect().await {
        Ok(docs) => {
            info!(
                agent_name = %req.agent_name,
                count = docs.len(),
                "Agent collected documents"
            );
            docs
        }
        Err(e) => {
            error!(
                agent_name = %req.agent_name,
                error = %e,
                "Agent collection failed"
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Collection failed: {}", e)
                })),
            )
                .into_response();
        }
    };

    let doc_count = documents.len() as u64;

    // Run extraction pipeline on collected documents
    let extraction_results = match state.extraction.extract_batch(&documents).await {
        Ok(results) => {
            info!(
                agent_name = %req.agent_name,
                extractions = results.len(),
                "Extraction pipeline completed"
            );
            results
        }
        Err(e) => {
            error!(
                agent_name = %req.agent_name,
                error = %e,
                "Extraction pipeline failed"
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Extraction failed: {}", e)
                })),
            )
                .into_response();
        }
    };

    // Store each extraction result in the graph database
    for result in &extraction_results {
        if let Err(e) = state.graph.store_extraction(result).await {
            error!(
                agent_name = %req.agent_name,
                error = %e,
                "Failed to store extraction result in graph"
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "error": format!("Graph storage failed: {}", e)
                })),
            )
                .into_response();
        }
    }

    info!(
        agent_name = %req.agent_name,
        documents_collected = doc_count,
        "Agent trigger completed successfully"
    );

    (
        StatusCode::OK,
        Json(AgentTriggerResponse {
            agent_name: req.agent_name,
            documents_collected: doc_count,
            message: format!(
                "Successfully collected {} documents and extracted entities",
                doc_count
            ),
        }),
    )
        .into_response()
}

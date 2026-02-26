use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use tracing::{info, warn, error};
use uuid::Uuid;

use argus_core::api_types::{
    AgentListResponse, AgentRunState, AgentRunStatus, AgentRunsResponse,
    AgentTriggerRequest, AgentTriggerResponse,
};
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

/// POST /api/agents/trigger — trigger a named agent asynchronously.
/// Returns 202 Accepted immediately with a run_id to track progress.
pub async fn trigger_agent(
    State(state): State<AppState>,
    Json(req): Json<AgentTriggerRequest>,
) -> impl IntoResponse {
    let agent_name = req.agent_name.clone();
    info!(agent_name = %agent_name, "Triggering agent (async)");

    // Look up the agent by name
    let agent = match state.agents.get(&agent_name) {
        Some(agent) => agent.clone(),
        None => {
            warn!(agent_name = %agent_name, "Agent not found");
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({
                    "error": format!("Agent '{}' not found", agent_name)
                })),
            )
                .into_response();
        }
    };

    let run_id = Uuid::new_v4().to_string();
    let run_status = AgentRunStatus {
        run_id: run_id.clone(),
        agent_name: agent_name.clone(),
        status: AgentRunState::Running,
        started_at: Utc::now(),
        finished_at: None,
        documents_collected: 0,
        entities_extracted: 0,
        error: None,
    };

    // Register the run
    {
        let mut runs = state.runs.write().await;
        runs.push(run_status);
    }

    // Spawn the pipeline in the background
    let run_id_clone = run_id.clone();
    let runs = state.runs.clone();
    let extraction = state.extraction.clone();
    let graph = state.graph.clone();

    tokio::spawn(async move {
        let result = run_agent_pipeline(
            &agent_name,
            agent,
            extraction,
            graph,
        )
        .await;

        let mut runs_lock = runs.write().await;
        if let Some(run) = runs_lock.iter_mut().find(|r| r.run_id == run_id_clone) {
            run.finished_at = Some(Utc::now());
            match result {
                Ok((docs, entities)) => {
                    run.status = AgentRunState::Completed;
                    run.documents_collected = docs;
                    run.entities_extracted = entities;
                    info!(
                        run_id = %run_id_clone,
                        agent_name = %run.agent_name,
                        documents = docs,
                        entities = entities,
                        "Agent run completed"
                    );
                }
                Err(e) => {
                    run.status = AgentRunState::Failed;
                    run.error = Some(e.clone());
                    error!(
                        run_id = %run_id_clone,
                        error = %e,
                        "Agent run failed"
                    );
                }
            }
        }
    });

    (
        StatusCode::ACCEPTED,
        Json(AgentTriggerResponse {
            run_id,
            agent_name: req.agent_name,
            status: "running".to_string(),
            message: "Agent triggered, pipeline running in background".to_string(),
        }),
    )
        .into_response()
}

/// Run the full agent pipeline: collect → extract → store.
/// Returns (documents_collected, entities_extracted) on success.
async fn run_agent_pipeline(
    agent_name: &str,
    agent: std::sync::Arc<dyn argus_core::Agent>,
    extraction: std::sync::Arc<argus_extraction::LlmExtractionPipeline>,
    graph: std::sync::Arc<argus_graph::Neo4jGraphStore>,
) -> std::result::Result<(u64, u64), String> {
    // Collect
    let documents = agent.collect().await.map_err(|e| {
        format!("Collection failed: {}", e)
    })?;
    let doc_count = documents.len() as u64;
    info!(agent_name = %agent_name, documents = doc_count, "Collection complete");

    if documents.is_empty() {
        return Ok((0, 0));
    }

    // Extract
    let extraction_results = extraction.extract_batch(&documents).await.map_err(|e| {
        format!("Extraction failed: {}", e)
    })?;
    let entity_count: u64 = extraction_results
        .iter()
        .map(|r| r.entities.len() as u64)
        .sum();
    info!(agent_name = %agent_name, extractions = extraction_results.len(), entities = entity_count, "Extraction complete");

    // Store
    for result in &extraction_results {
        graph.store_extraction(result).await.map_err(|e| {
            format!("Graph storage failed: {}", e)
        })?;
    }
    info!(agent_name = %agent_name, "Stored all extraction results");

    Ok((doc_count, entity_count))
}

/// GET /api/agents/runs — list all agent runs (active and completed).
pub async fn list_runs(State(state): State<AppState>) -> impl IntoResponse {
    let runs = state.runs.read().await;
    let runs_vec: Vec<AgentRunStatus> = runs.iter().rev().cloned().collect();
    (
        StatusCode::OK,
        Json(AgentRunsResponse { runs: runs_vec }),
    )
}

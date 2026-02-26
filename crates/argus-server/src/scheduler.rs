use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tracing::{error, info, warn};
use uuid::Uuid;

use argus_core::api_types::{AgentRunState, AgentRunStatus};
use argus_core::{Agent, ExtractionPipeline, GraphStore};

use crate::state::AppState;

/// Schedule configuration for each agent.
struct AgentSchedule {
    name: &'static str,
    interval: Duration,
    requires_env: Option<&'static str>,
}

const SCHEDULES: &[AgentSchedule] = &[
    AgentSchedule {
        name: "gdelt",
        interval: Duration::from_secs(15 * 60), // 15 min
        requires_env: None,
    },
    AgentSchedule {
        name: "adsb",
        interval: Duration::from_secs(5 * 60), // 5 min
        requires_env: None,
    },
    AgentSchedule {
        name: "opencorporates",
        interval: Duration::from_secs(60 * 60), // 1 hour
        requires_env: None,
    },
    AgentSchedule {
        name: "opensanctions",
        interval: Duration::from_secs(6 * 60 * 60), // 6 hours
        requires_env: None,
    },
    AgentSchedule {
        name: "eu_transparency",
        interval: Duration::from_secs(24 * 60 * 60), // 24 hours
        requires_env: None,
    },
    AgentSchedule {
        name: "ais",
        interval: Duration::from_secs(5 * 60), // 5 min
        requires_env: Some("AISHUB_API_KEY"),
    },
];

/// Main scheduler loop. Spawns one task per agent, each running on its own interval.
pub async fn run_scheduler(state: AppState) {
    info!("Starting background scheduler");

    // Give the server a moment to start up before first collection
    tokio::time::sleep(Duration::from_secs(10)).await;

    for schedule in SCHEDULES {
        // Skip agents that require an env var that isn't set
        if let Some(env_var) = schedule.requires_env {
            if std::env::var(env_var).is_err() {
                info!(
                    agent = schedule.name,
                    env_var = env_var,
                    "Skipping scheduled agent (env var not set)"
                );
                continue;
            }
        }

        let agent = match state.agents.get(schedule.name) {
            Some(a) => a.clone(),
            None => {
                warn!(agent = schedule.name, "Scheduled agent not found in registry");
                continue;
            }
        };

        let interval = schedule.interval;
        let agent_name = schedule.name.to_string();
        let extraction = state.extraction.clone();
        let graph = state.graph.clone();
        let runs = state.runs.clone();
        let all_agents: Vec<(String, Arc<dyn Agent>)> = state
            .agents
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        tokio::spawn(async move {
            agent_loop(
                agent_name,
                agent,
                interval,
                extraction,
                graph,
                runs,
                all_agents,
            )
            .await;
        });

        info!(
            agent = schedule.name,
            interval_secs = schedule.interval.as_secs(),
            "Scheduled agent"
        );
    }
}

/// Run a single agent in a loop at the given interval.
async fn agent_loop(
    agent_name: String,
    agent: Arc<dyn Agent>,
    interval: Duration,
    extraction: Arc<argus_extraction::LlmExtractionPipeline>,
    graph: Arc<argus_graph::Neo4jGraphStore>,
    runs: Arc<tokio::sync::RwLock<Vec<AgentRunStatus>>>,
    all_agents: Vec<(String, Arc<dyn Agent>)>,
) {
    loop {
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

        {
            let mut runs_lock = runs.write().await;
            runs_lock.push(run_status);
            // Keep only the last 100 runs to avoid unbounded growth
            if runs_lock.len() > 100 {
                let drain_count = runs_lock.len() - 100;
                runs_lock.drain(0..drain_count);
            }
        }

        info!(agent = %agent_name, run_id = %run_id, "Scheduled collection starting");

        // Step 1: Collect
        let documents = match agent.collect().await {
            Ok(docs) => {
                info!(agent = %agent_name, count = docs.len(), "Collection complete");
                docs
            }
            Err(e) => {
                error!(agent = %agent_name, error = %e, "Collection failed");
                update_run(&runs, &run_id, AgentRunState::Failed, 0, 0, Some(e.to_string())).await;
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        let doc_count = documents.len() as u64;

        if documents.is_empty() {
            update_run(&runs, &run_id, AgentRunState::Completed, 0, 0, None).await;
            tokio::time::sleep(interval).await;
            continue;
        }

        // Step 2: Extract
        let extraction_results = match extraction.extract_batch(&documents).await {
            Ok(results) => {
                info!(agent = %agent_name, results = results.len(), "Extraction complete");
                results
            }
            Err(e) => {
                error!(agent = %agent_name, error = %e, "Extraction failed");
                update_run(&runs, &run_id, AgentRunState::Failed, doc_count, 0, Some(e.to_string())).await;
                tokio::time::sleep(interval).await;
                continue;
            }
        };

        let entity_count: u64 = extraction_results
            .iter()
            .map(|r| r.entities.len() as u64)
            .sum();

        // Step 3: Store
        let mut store_errors = 0;
        for result in &extraction_results {
            if let Err(e) = graph.store_extraction(result).await {
                error!(agent = %agent_name, error = %e, "Failed to store extraction result");
                store_errors += 1;
            }
        }

        // Step 4: Cross-reference new entities against other agents
        cross_reference(
            &agent_name,
            &extraction_results,
            &all_agents,
            &extraction,
            &graph,
        )
        .await;

        if store_errors > 0 {
            update_run(
                &runs, &run_id, AgentRunState::Completed, doc_count, entity_count,
                Some(format!("{} storage errors", store_errors)),
            ).await;
        } else {
            update_run(&runs, &run_id, AgentRunState::Completed, doc_count, entity_count, None).await;
        }

        info!(
            agent = %agent_name,
            documents = doc_count,
            entities = entity_count,
            "Scheduled run complete, sleeping for {}s",
            interval.as_secs()
        );

        tokio::time::sleep(interval).await;
    }
}

/// Cross-reference newly extracted entities against other agents' lookup capabilities.
async fn cross_reference(
    source_agent: &str,
    extraction_results: &[argus_core::ExtractionResult],
    all_agents: &[(String, Arc<dyn Agent>)],
    extraction: &Arc<argus_extraction::LlmExtractionPipeline>,
    graph: &Arc<argus_graph::Neo4jGraphStore>,
) {
    use argus_core::agent::AgentLookup;

    for result in extraction_results {
        for entity in &result.entities {
            for (name, agent) in all_agents {
                // Don't look up against the same agent that produced the entity
                if name == source_agent {
                    continue;
                }

                // Check if this agent supports lookup for this entity type
                let lookup: &dyn AgentLookup = match agent.as_any().downcast_ref::<argus_agents::GdeltAgent>() {
                    Some(a) => a as &dyn AgentLookup,
                    None => match agent.as_any().downcast_ref::<argus_agents::OpenSanctionsAgent>() {
                        Some(a) => a as &dyn AgentLookup,
                        None => match agent.as_any().downcast_ref::<argus_agents::OpenCorporatesAgent>() {
                            Some(a) => a as &dyn AgentLookup,
                            None => match agent.as_any().downcast_ref::<argus_agents::AdsbAgent>() {
                                Some(a) => a as &dyn AgentLookup,
                                None => match agent.as_any().downcast_ref::<argus_agents::AisAgent>() {
                                    Some(a) => a as &dyn AgentLookup,
                                    None => match agent.as_any().downcast_ref::<argus_agents::EuTransparencyAgent>() {
                                        Some(a) => a as &dyn AgentLookup,
                                        None => continue,
                                    },
                                },
                            },
                        },
                    },
                };

                if !lookup.can_lookup(&entity.entity_type) {
                    continue;
                }

                info!(
                    entity = %entity.name,
                    entity_type = ?entity.entity_type,
                    lookup_agent = %name,
                    "Cross-referencing entity"
                );

                match lookup.lookup(&entity.name, &entity.entity_type).await {
                    Ok(docs) if !docs.is_empty() => {
                        info!(
                            entity = %entity.name,
                            lookup_agent = %name,
                            docs = docs.len(),
                            "Cross-reference found documents"
                        );

                        match extraction.extract_batch(&docs).await {
                            Ok(results) => {
                                for r in &results {
                                    if let Err(e) = graph.store_extraction(r).await {
                                        warn!(
                                            error = %e,
                                            "Failed to store cross-reference extraction"
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(
                                    error = %e,
                                    "Cross-reference extraction failed"
                                );
                            }
                        }
                    }
                    Ok(_) => {} // No docs found, that's fine
                    Err(e) => {
                        warn!(
                            entity = %entity.name,
                            lookup_agent = %name,
                            error = %e,
                            "Cross-reference lookup failed"
                        );
                    }
                }
            }
        }
    }
}

async fn update_run(
    runs: &Arc<tokio::sync::RwLock<Vec<AgentRunStatus>>>,
    run_id: &str,
    status: AgentRunState,
    docs: u64,
    entities: u64,
    error: Option<String>,
) {
    let mut runs_lock = runs.write().await;
    if let Some(run) = runs_lock.iter_mut().find(|r| r.run_id == run_id) {
        run.status = status;
        run.finished_at = Some(Utc::now());
        run.documents_collected = docs;
        run.entities_extracted = entities;
        run.error = error;
    }
}

use std::sync::Arc;

use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

mod handlers;
mod routes;
mod scheduler;
mod state;

use state::AppState;

#[tokio::main]
async fn main() {
    // Load .env file if present
    if let Ok(env_path) = std::fs::canonicalize(".env") {
        if env_path.exists() {
            for line in std::fs::read_to_string(&env_path).unwrap_or_default().lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();
                    if std::env::var(key).is_err() {
                        std::env::set_var(key, value);
                    }
                }
            }
        }
    }

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("argus=info".parse().unwrap()))
        .init();

    let config = argus_core::AppConfig::from_env();
    let host = config.server_host.clone();
    let port = config.server_port;

    if config.anthropic_api_key.is_empty() {
        tracing::warn!("ANTHROPIC_API_KEY not set — extraction and reasoning will fail");
    } else {
        tracing::info!("ANTHROPIC_API_KEY loaded ({} chars)", config.anthropic_api_key.len());
    }

    let graph = Arc::new(argus_graph::Neo4jGraphStore::new(&config).await);
    let extraction = Arc::new(argus_extraction::LlmExtractionPipeline::new(&config));
    let reasoning = Arc::new(argus_reasoning::LlmReasoningEngine::new(
        graph.clone() as Arc<dyn argus_core::graph::GraphStore>,
        &config,
    ));
    let agents = argus_agents::agent_registry();
    let runs = Arc::new(RwLock::new(Vec::new()));

    let state = AppState {
        config,
        agents,
        graph,
        extraction,
        reasoning,
        runs,
    };

    // Start background scheduler
    let scheduler_state = state.clone();
    tokio::spawn(async move {
        scheduler::run_scheduler(scheduler_state).await;
    });

    let app = routes::create_router()
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = format!("{host}:{port}");
    tracing::info!("ARGUS server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

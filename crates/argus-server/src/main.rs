use std::sync::Arc;

use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

mod handlers;
mod routes;
mod state;

use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("argus=info".parse().unwrap()))
        .init();

    let config = argus_core::AppConfig::from_env();
    let host = config.server_host.clone();
    let port = config.server_port;

    let graph = Arc::new(
        argus_graph::Neo4jGraphStore::new(&config)
            .await
            .expect("Failed to connect to Neo4j"),
    );
    let extraction = Arc::new(argus_extraction::LlmExtractionPipeline::new(&config));
    let reasoning = Arc::new(argus_reasoning::LlmReasoningEngine::new(
        graph.clone() as Arc<dyn argus_core::graph::GraphStore>,
        &config,
    ));
    let agents = argus_agents::agent_registry();

    let state = AppState {
        config,
        agents,
        graph,
        extraction,
        reasoning,
    };

    let app = routes::create_router()
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = format!("{host}:{port}");
    tracing::info!("ARGUS server listening on {addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

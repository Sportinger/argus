use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    pub name: String,
    pub source_type: String,
    pub enabled: bool,
    pub url: Option<String>,
    pub api_key: Option<String>,
    pub interval_seconds: u64,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub neo4j_uri: String,
    pub neo4j_user: String,
    pub neo4j_password: String,
    pub qdrant_url: String,
    pub anthropic_api_key: String,
    pub server_host: String,
    pub server_port: u16,
    pub sources: Vec<SourceConfig>,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            neo4j_uri: std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".into()),
            neo4j_user: std::env::var("NEO4J_USER").unwrap_or_else(|_| "neo4j".into()),
            neo4j_password: std::env::var("NEO4J_PASSWORD").unwrap_or_else(|_| "argus".into()),
            qdrant_url: std::env::var("QDRANT_URL")
                .unwrap_or_else(|_| "http://localhost:6333".into()),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            server_host: std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            server_port: std::env::var("SERVER_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            sources: Vec::new(),
        }
    }
}

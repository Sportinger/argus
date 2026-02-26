use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use argus_core::agent::{Agent, AgentStatus, RawDocument};
use argus_core::error::{ArgusError, Result};

const OPENSANCTIONS_API_URL: &str = "https://api.opensanctions.org/entities";
const DEFAULT_DATASET: &str = "default";
const PAGE_LIMIT: u32 = 100;

#[derive(Debug, Deserialize)]
struct OpenSanctionsResponse {
    results: Vec<SanctionEntity>,
    #[serde(default)]
    total: Option<u64>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    offset: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct SanctionEntity {
    id: String,
    #[serde(default)]
    caption: Option<String>,
    #[serde(default)]
    schema_: Option<String>,
    #[serde(rename = "schema")]
    schema_name: Option<String>,
    #[serde(default)]
    properties: Option<serde_json::Value>,
    #[serde(default)]
    datasets: Option<Vec<String>>,
    #[serde(default)]
    referents: Option<Vec<String>>,
    #[serde(default)]
    first_seen: Option<String>,
    #[serde(default)]
    last_seen: Option<String>,
    #[serde(default)]
    last_change: Option<String>,
    #[serde(default)]
    target: Option<bool>,
}

#[derive(Debug)]
struct InternalState {
    enabled: bool,
    last_run: Option<DateTime<Utc>>,
    documents_collected: u64,
    last_error: Option<String>,
}

pub struct OpenSanctionsAgent {
    client: Client,
    state: RwLock<InternalState>,
}

impl OpenSanctionsAgent {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("argus-osint/0.1")
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .expect("failed to build HTTP client");

        Self {
            client,
            state: RwLock::new(InternalState {
                enabled: true,
                last_run: None,
                documents_collected: 0,
                last_error: None,
            }),
        }
    }

    fn entity_to_document(&self, entity: &SanctionEntity) -> RawDocument {
        let name = entity
            .caption
            .clone()
            .or_else(|| {
                entity
                    .properties
                    .as_ref()
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.as_str())
                    .map(String::from)
            })
            .unwrap_or_else(|| entity.id.clone());

        let schema = entity
            .schema_name
            .clone()
            .or_else(|| entity.schema_.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        let content = format!(
            "Sanctioned entity: {} (Schema: {}). ID: {}",
            name, schema, entity.id
        );

        let metadata = serde_json::json!({
            "schema": schema,
            "properties": entity.properties,
            "datasets": entity.datasets,
            "referents": entity.referents,
            "first_seen": entity.first_seen,
            "last_seen": entity.last_seen,
            "last_change": entity.last_change,
            "target": entity.target,
            "caption": entity.caption,
        });

        let url = format!(
            "https://api.opensanctions.org/entities/{}",
            entity.id
        );

        RawDocument {
            source: "opensanctions".to_string(),
            source_id: entity.id.clone(),
            title: Some(name),
            content,
            url: Some(url),
            collected_at: Utc::now(),
            metadata,
        }
    }

    async fn fetch_page(&self, offset: u32, limit: u32) -> Result<OpenSanctionsResponse> {
        let url = format!(
            "{}?dataset={}&limit={}&offset={}",
            OPENSANCTIONS_API_URL, DEFAULT_DATASET, limit, offset
        );

        debug!(url = %url, "Fetching OpenSanctions page");

        let response = self.client.get(&url).send().await.map_err(|e| {
            ArgusError::Agent {
                agent: "opensanctions".to_string(),
                message: format!("HTTP request failed: {}", e),
            }
        })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ArgusError::Agent {
                agent: "opensanctions".to_string(),
                message: format!(
                    "API returned HTTP {}: {}",
                    status,
                    body.chars().take(500).collect::<String>()
                ),
            });
        }

        let data: OpenSanctionsResponse = response.json().await.map_err(|e| {
            ArgusError::Agent {
                agent: "opensanctions".to_string(),
                message: format!("Failed to parse response JSON: {}", e),
            }
        })?;

        Ok(data)
    }
}

#[async_trait]
impl Agent for OpenSanctionsAgent {
    fn name(&self) -> &str {
        "opensanctions"
    }

    fn source_type(&self) -> &str {
        "sanctions"
    }

    async fn collect(&self) -> Result<Vec<RawDocument>> {
        info!("Starting OpenSanctions data collection");

        {
            let state = self.state.read().await;
            if !state.enabled {
                warn!("OpenSanctions agent is disabled, skipping collection");
                return Ok(Vec::new());
            }
        }

        let mut all_documents = Vec::new();
        let mut offset: u32 = 0;

        loop {
            let page = match self.fetch_page(offset, PAGE_LIMIT).await {
                Ok(page) => page,
                Err(e) => {
                    error!(error = %e, offset = offset, "Failed to fetch OpenSanctions page");
                    let mut state = self.state.write().await;
                    state.last_error = Some(e.to_string());
                    state.last_run = Some(Utc::now());
                    return Err(e);
                }
            };

            let result_count = page.results.len();
            debug!(
                offset = offset,
                results = result_count,
                total = ?page.total,
                "Fetched OpenSanctions page"
            );

            for entity in &page.results {
                let doc = self.entity_to_document(entity);
                all_documents.push(doc);
            }

            // If we got fewer results than the limit, we've reached the end
            if (result_count as u32) < PAGE_LIMIT {
                break;
            }

            // If the API reports a total and we've reached it, stop
            if let Some(total) = page.total {
                if (offset + result_count as u32) as u64 >= total {
                    break;
                }
            }

            offset += PAGE_LIMIT;

            // Safety limit to avoid runaway pagination
            if offset > 10_000 {
                warn!(
                    offset = offset,
                    "Reached safety pagination limit, stopping collection"
                );
                break;
            }
        }

        let doc_count = all_documents.len() as u64;

        {
            let mut state = self.state.write().await;
            state.last_run = Some(Utc::now());
            state.documents_collected += doc_count;
            state.last_error = None;
        }

        info!(
            documents = doc_count,
            "OpenSanctions collection complete"
        );

        Ok(all_documents)
    }

    async fn status(&self) -> AgentStatus {
        let state = self.state.read().await;
        AgentStatus {
            name: "opensanctions".to_string(),
            enabled: state.enabled,
            last_run: state.last_run,
            documents_collected: state.documents_collected,
            error: state.last_error.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_agent() {
        let agent = OpenSanctionsAgent::new();
        assert_eq!(agent.name(), "opensanctions");
        assert_eq!(agent.source_type(), "sanctions");
    }

    #[tokio::test]
    async fn test_initial_status() {
        let agent = OpenSanctionsAgent::new();
        let status = agent.status().await;
        assert_eq!(status.name, "opensanctions");
        assert!(status.enabled);
        assert!(status.last_run.is_none());
        assert_eq!(status.documents_collected, 0);
        assert!(status.error.is_none());
    }

    #[test]
    fn test_entity_to_document() {
        let agent = OpenSanctionsAgent::new();

        let entity = SanctionEntity {
            id: "Q123456".to_string(),
            caption: Some("John Doe".to_string()),
            schema_: None,
            schema_name: Some("Person".to_string()),
            properties: Some(serde_json::json!({
                "name": ["John Doe"],
                "nationality": ["us"]
            })),
            datasets: Some(vec!["us_ofac_sdn".to_string()]),
            referents: None,
            first_seen: Some("2023-01-01".to_string()),
            last_seen: Some("2024-06-15".to_string()),
            last_change: None,
            target: Some(true),
        };

        let doc = agent.entity_to_document(&entity);

        assert_eq!(doc.source, "opensanctions");
        assert_eq!(doc.source_id, "Q123456");
        assert_eq!(doc.title, Some("John Doe".to_string()));
        assert!(doc.content.contains("John Doe"));
        assert!(doc.content.contains("Person"));
        assert_eq!(
            doc.url,
            Some("https://api.opensanctions.org/entities/Q123456".to_string())
        );
        assert_eq!(doc.metadata["schema"], "Person");
        assert!(doc.metadata["datasets"].is_array());
    }

    #[test]
    fn test_entity_to_document_minimal() {
        let agent = OpenSanctionsAgent::new();

        let entity = SanctionEntity {
            id: "NK-001".to_string(),
            caption: None,
            schema_: None,
            schema_name: None,
            properties: None,
            datasets: None,
            referents: None,
            first_seen: None,
            last_seen: None,
            last_change: None,
            target: None,
        };

        let doc = agent.entity_to_document(&entity);

        assert_eq!(doc.source_id, "NK-001");
        // With no caption or properties, the title falls back to entity id
        assert_eq!(doc.title, Some("NK-001".to_string()));
        assert!(doc.content.contains("NK-001"));
        assert!(doc.content.contains("Unknown"));
    }

    #[test]
    fn test_entity_to_document_name_from_properties() {
        let agent = OpenSanctionsAgent::new();

        let entity = SanctionEntity {
            id: "entity-789".to_string(),
            caption: None,
            schema_: Some("LegalEntity".to_string()),
            schema_name: None,
            properties: Some(serde_json::json!({
                "name": ["ACME Corp"]
            })),
            datasets: None,
            referents: None,
            first_seen: None,
            last_seen: None,
            last_change: None,
            target: Some(false),
        };

        let doc = agent.entity_to_document(&entity);

        assert_eq!(doc.title, Some("ACME Corp".to_string()));
        assert!(doc.content.contains("ACME Corp"));
        // schema_ is used as fallback when schema_name is None
        assert!(doc.content.contains("LegalEntity"));
    }
}

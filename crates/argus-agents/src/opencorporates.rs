use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

use argus_core::agent::{Agent, AgentStatus, RawDocument};
use argus_core::error::{ArgusError, Result};

const OPENCORPORATES_API_BASE: &str = "https://api.opencorporates.com/v0.4";

#[derive(Debug, Deserialize)]
struct ApiResponse {
    results: ApiResults,
}

#[derive(Debug, Deserialize)]
struct ApiResults {
    companies: Vec<CompanyWrapper>,
    total_count: Option<u64>,
    page: Option<u64>,
    per_page: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct CompanyWrapper {
    company: Company,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct Company {
    name: Option<String>,
    company_number: Option<String>,
    jurisdiction_code: Option<String>,
    incorporation_date: Option<String>,
    dissolution_date: Option<String>,
    company_type: Option<String>,
    registry_url: Option<String>,
    branch: Option<String>,
    branch_status: Option<String>,
    inactive: Option<bool>,
    current_status: Option<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    retrieved_at: Option<String>,
    opencorporates_url: Option<String>,
    registered_address_in_full: Option<String>,
    source: Option<CompanySource>,
    #[serde(default)]
    previous_names: Vec<serde_json::Value>,
    #[serde(default)]
    alternative_names: Vec<serde_json::Value>,
    #[serde(default)]
    agent_name: Option<String>,
    #[serde(default)]
    agent_address: Option<String>,
    #[serde(default)]
    officers: Vec<serde_json::Value>,
    #[serde(default)]
    industry_codes: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct CompanySource {
    publisher: Option<String>,
    url: Option<String>,
    retrieved_at: Option<String>,
}

struct InternalState {
    last_run: Option<DateTime<Utc>>,
    documents_collected: u64,
    last_error: Option<String>,
}

pub struct OpenCorporatesAgent {
    client: Client,
    state: RwLock<InternalState>,
}

impl OpenCorporatesAgent {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("argus-intelligence-platform/0.1")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");

        Self {
            client,
            state: RwLock::new(InternalState {
                last_run: None,
                documents_collected: 0,
                last_error: None,
            }),
        }
    }

    fn build_search_url(&self) -> String {
        format!("{}/companies/search", OPENCORPORATES_API_BASE)
    }

    fn company_to_raw_document(&self, company: &Company, collected_at: DateTime<Utc>) -> RawDocument {
        let source_id = format!(
            "opencorporates:{}:{}",
            company.jurisdiction_code.as_deref().unwrap_or("unknown"),
            company.company_number.as_deref().unwrap_or("unknown")
        );

        let title = company.name.clone();

        let content = serde_json::to_string(company).unwrap_or_default();

        let url = company.opencorporates_url.clone();

        let metadata = serde_json::json!({
            "jurisdiction_code": company.jurisdiction_code,
            "company_number": company.company_number,
            "company_type": company.company_type,
            "incorporation_date": company.incorporation_date,
            "dissolution_date": company.dissolution_date,
            "current_status": company.current_status,
            "inactive": company.inactive,
            "registered_address": company.registered_address_in_full,
            "branch": company.branch,
            "branch_status": company.branch_status,
            "updated_at": company.updated_at,
            "retrieved_at": company.retrieved_at,
        });

        RawDocument {
            source: "opencorporates".to_string(),
            source_id,
            title,
            content,
            url,
            collected_at,
            metadata,
        }
    }
}

#[async_trait]
impl Agent for OpenCorporatesAgent {
    fn name(&self) -> &str {
        "opencorporates"
    }

    fn source_type(&self) -> &str {
        "corporate_registry"
    }

    #[instrument(skip(self), name = "opencorporates_collect")]
    async fn collect(&self) -> Result<Vec<RawDocument>> {
        info!("Starting OpenCorporates data collection");

        let url = self.build_search_url();
        let collected_at = Utc::now();

        // Search for recently updated companies using the updated_since parameter.
        // We look back 24 hours to capture recent updates.
        let since = (collected_at - chrono::Duration::hours(24))
            .format("%Y-%m-%dT%H:%M:%S+00:00")
            .to_string();

        debug!(
            url = %url,
            updated_since = %since,
            "Fetching companies from OpenCorporates API"
        );

        let response = self
            .client
            .get(&url)
            .query(&[
                ("q", "*"),
                ("order", "updated_at"),
                ("updated_since", &since),
                ("per_page", "100"),
            ])
            .send()
            .await
            .map_err(|e| {
                let msg = format!("HTTP request to OpenCorporates failed: {}", e);
                error!(%msg);
                ArgusError::Agent {
                    agent: "opencorporates".to_string(),
                    message: msg,
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let msg = format!(
                "OpenCorporates API returned HTTP {}: {}",
                status,
                body.chars().take(500).collect::<String>()
            );
            error!(%msg);

            let mut state = self.state.write().await;
            state.last_run = Some(Utc::now());
            state.last_error = Some(msg.clone());

            return Err(ArgusError::Agent {
                agent: "opencorporates".to_string(),
                message: msg,
            });
        }

        let api_response: ApiResponse = response.json().await.map_err(|e| {
            let msg = format!("Failed to parse OpenCorporates response: {}", e);
            error!(%msg);
            ArgusError::Agent {
                agent: "opencorporates".to_string(),
                message: msg,
            }
        })?;

        let total_count = api_response.results.total_count.unwrap_or(0);
        let companies = api_response.results.companies;

        info!(
            total_available = total_count,
            fetched = companies.len(),
            "Received companies from OpenCorporates"
        );

        let documents: Vec<RawDocument> = companies
            .iter()
            .filter_map(|wrapper| {
                let company = &wrapper.company;
                if company.company_number.is_none() && company.name.is_none() {
                    warn!("Skipping company with no number and no name");
                    return None;
                }
                Some(self.company_to_raw_document(company, collected_at))
            })
            .collect();

        debug!(
            document_count = documents.len(),
            "Converted companies to RawDocuments"
        );

        // Update internal state
        let mut state = self.state.write().await;
        state.last_run = Some(Utc::now());
        state.documents_collected += documents.len() as u64;
        state.last_error = None;

        info!(
            documents_collected = documents.len(),
            total_collected = state.documents_collected,
            "OpenCorporates collection complete"
        );

        Ok(documents)
    }

    async fn status(&self) -> AgentStatus {
        let state = self.state.read().await;
        AgentStatus {
            name: "opencorporates".to_string(),
            enabled: true,
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
        let agent = OpenCorporatesAgent::new();
        assert_eq!(agent.name(), "opencorporates");
        assert_eq!(agent.source_type(), "corporate_registry");
    }

    #[tokio::test]
    async fn test_initial_status() {
        let agent = OpenCorporatesAgent::new();
        let status = agent.status().await;
        assert_eq!(status.name, "opencorporates");
        assert!(status.enabled);
        assert!(status.last_run.is_none());
        assert_eq!(status.documents_collected, 0);
        assert!(status.error.is_none());
    }

    #[test]
    fn test_build_search_url() {
        let agent = OpenCorporatesAgent::new();
        let url = agent.build_search_url();
        assert_eq!(url, "https://api.opencorporates.com/v0.4/companies/search");
    }

    #[test]
    fn test_company_to_raw_document() {
        let agent = OpenCorporatesAgent::new();
        let company = Company {
            name: Some("Test Corp".to_string()),
            company_number: Some("12345".to_string()),
            jurisdiction_code: Some("us_de".to_string()),
            incorporation_date: Some("2020-01-01".to_string()),
            dissolution_date: None,
            company_type: Some("LLC".to_string()),
            registry_url: Some("https://example.com/registry".to_string()),
            branch: None,
            branch_status: None,
            inactive: Some(false),
            current_status: Some("Active".to_string()),
            created_at: Some("2020-01-01T00:00:00+00:00".to_string()),
            updated_at: Some("2025-01-01T00:00:00+00:00".to_string()),
            retrieved_at: Some("2025-06-01T00:00:00+00:00".to_string()),
            opencorporates_url: Some("https://opencorporates.com/companies/us_de/12345".to_string()),
            registered_address_in_full: Some("123 Main St, Dover, DE".to_string()),
            source: None,
            previous_names: vec![],
            alternative_names: vec![],
            agent_name: None,
            agent_address: None,
            officers: vec![],
            industry_codes: vec![],
        };

        let collected_at = Utc::now();
        let doc = agent.company_to_raw_document(&company, collected_at);

        assert_eq!(doc.source, "opencorporates");
        assert_eq!(doc.source_id, "opencorporates:us_de:12345");
        assert_eq!(doc.title, Some("Test Corp".to_string()));
        assert_eq!(
            doc.url,
            Some("https://opencorporates.com/companies/us_de/12345".to_string())
        );
        assert_eq!(doc.collected_at, collected_at);
        assert_eq!(doc.metadata["jurisdiction_code"], "us_de");
        assert_eq!(doc.metadata["company_number"], "12345");
        assert_eq!(doc.metadata["current_status"], "Active");
        assert_eq!(doc.metadata["inactive"], false);
    }
}

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use argus_core::agent::{Agent, AgentStatus, RawDocument};
use argus_core::error::{ArgusError, Result};

const EU_TRANSPARENCY_API_URL: &str =
    "https://ec.europa.eu/transparencyregister/public/consultation/statistics.do?action=getLobbyistsJson";

/// Internal mutable state for the EU Transparency Register agent.
struct EuTransparencyState {
    enabled: bool,
    last_run: Option<chrono::DateTime<Utc>>,
    documents_collected: u64,
    last_error: Option<String>,
}

/// A single lobbyist organization entry from the EU Transparency Register API.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LobbyistEntry {
    /// Unique registration identifier in the transparency register.
    #[serde(alias = "registrationId", alias = "id")]
    registration_id: Option<String>,

    /// Name of the registered organisation.
    #[serde(alias = "name", alias = "organisationName")]
    name: Option<String>,

    /// Category or section under which the organisation is registered.
    #[serde(alias = "category", alias = "section")]
    category: Option<String>,

    /// Sub-category of the registration.
    #[serde(alias = "subCategory")]
    sub_category: Option<String>,

    /// Country of the registered head office.
    #[serde(alias = "countryOfHeadOffice", alias = "country")]
    country: Option<String>,

    /// Number of accredited persons (lobbyists with EP access passes).
    #[serde(alias = "numberOfAccreditedPersons", alias = "accreditedPersons")]
    accredited_persons: Option<serde_json::Value>,

    /// Declared lobbying costs or financial range.
    #[serde(alias = "costs", alias = "lobbyingCosts")]
    lobbying_costs: Option<String>,

    /// Description of main activities or goals.
    #[serde(alias = "activities", alias = "goals")]
    activities: Option<String>,

    /// Registration date as a string.
    #[serde(alias = "registrationDate")]
    registration_date: Option<String>,

    /// Website URL of the organisation.
    #[serde(alias = "webSiteUrl", alias = "website")]
    website: Option<String>,
}

/// Wrapper for the API response which may be a direct array or nested under a key.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ApiResponse {
    Array(Vec<LobbyistEntry>),
    Wrapped { results: Vec<LobbyistEntry> },
    WrappedAlt { data: Vec<LobbyistEntry> },
}

impl ApiResponse {
    fn into_entries(self) -> Vec<LobbyistEntry> {
        match self {
            ApiResponse::Array(v) => v,
            ApiResponse::Wrapped { results } => results,
            ApiResponse::WrappedAlt { data } => data,
        }
    }
}

/// EU Transparency Register agent.
///
/// Fetches registered lobbyist organisations from the EU Transparency Register
/// public API and converts each entry into a `RawDocument`.
pub struct EuTransparencyAgent {
    client: reqwest::Client,
    state: RwLock<EuTransparencyState>,
}

impl EuTransparencyAgent {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("failed to build reqwest client"),
            state: RwLock::new(EuTransparencyState {
                enabled: true,
                last_run: None,
                documents_collected: 0,
                last_error: None,
            }),
        }
    }

    /// Convert a single lobbyist entry into a `RawDocument`.
    fn parse_entry(entry: &LobbyistEntry) -> Option<RawDocument> {
        let registration_id = entry.registration_id.as_deref()?.trim().to_string();
        if registration_id.is_empty() {
            return None;
        }

        let name = entry
            .name
            .as_deref()
            .unwrap_or("Unknown Organisation")
            .trim()
            .to_string();

        let category = entry
            .category
            .as_deref()
            .unwrap_or("Uncategorised")
            .trim()
            .to_string();

        let sub_category = entry
            .sub_category
            .as_deref()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        let country = entry
            .country
            .as_deref()
            .unwrap_or("unknown")
            .trim()
            .to_string();

        let activities = entry
            .activities
            .as_deref()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        let lobbying_costs = entry
            .lobbying_costs
            .as_deref()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        let registration_date = entry
            .registration_date
            .as_deref()
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        // Build human-readable content summary
        let mut content = format!(
            "Lobbyist organisation: {name} (ID: {registration_id}). \
             Category: {category}."
        );

        if !sub_category.is_empty() {
            content.push_str(&format!(" Sub-category: {sub_category}."));
        }

        content.push_str(&format!(" Country: {country}."));

        if !activities.is_empty() {
            content.push_str(&format!(" Activities: {activities}."));
        }

        if !lobbying_costs.is_empty() {
            content.push_str(&format!(" Lobbying costs: {lobbying_costs}."));
        }

        if !registration_date.is_empty() {
            content.push_str(&format!(" Registered: {registration_date}."));
        }

        let metadata = serde_json::json!({
            "registration_id": registration_id,
            "name": name,
            "category": category,
            "sub_category": sub_category,
            "country": country,
            "activities": activities,
            "lobbying_costs": lobbying_costs,
            "accredited_persons": entry.accredited_persons,
            "registration_date": registration_date,
            "website": entry.website,
        });

        let url = format!(
            "https://ec.europa.eu/transparencyregister/public/consultation/displaylobbyist.do?id={}",
            registration_id
        );

        Some(RawDocument {
            source: "eu_transparency".into(),
            source_id: registration_id,
            title: Some(name),
            content,
            url: Some(url),
            collected_at: Utc::now(),
            metadata,
        })
    }
}

#[async_trait]
impl Agent for EuTransparencyAgent {
    fn name(&self) -> &str {
        "eu_transparency"
    }

    fn source_type(&self) -> &str {
        "lobby_register"
    }

    async fn collect(&self) -> Result<Vec<RawDocument>> {
        info!("EU Transparency agent: starting collection from EU Transparency Register");

        let response = self
            .client
            .get(EU_TRANSPARENCY_API_URL)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| ArgusError::Agent {
                agent: "eu_transparency".into(),
                message: format!("HTTP request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let msg = format!(
                "EU Transparency Register API returned status {}: {}",
                status, body
            );
            warn!("EU Transparency agent: {}", msg);
            let mut state = self.state.write().await;
            state.last_run = Some(Utc::now());
            state.last_error = Some(msg.clone());
            return Err(ArgusError::Agent {
                agent: "eu_transparency".into(),
                message: msg,
            });
        }

        let body = response.text().await.map_err(|e| ArgusError::Agent {
            agent: "eu_transparency".into(),
            message: format!("failed to read response body: {}", e),
        })?;

        debug!(
            "EU Transparency agent: received response ({} bytes)",
            body.len()
        );

        let api_response: ApiResponse =
            serde_json::from_str(&body).map_err(|e| ArgusError::Agent {
                agent: "eu_transparency".into(),
                message: format!("failed to parse EU Transparency Register response: {}", e),
            })?;

        let entries = api_response.into_entries();

        debug!(
            "EU Transparency agent: parsed {} lobbyist entries",
            entries.len()
        );

        let documents: Vec<RawDocument> = entries
            .iter()
            .filter_map(Self::parse_entry)
            .collect();

        let count = documents.len() as u64;
        info!(
            "EU Transparency agent: collected {} lobbyist organisations",
            count
        );

        // Update internal state
        let mut state = self.state.write().await;
        state.last_run = Some(Utc::now());
        state.documents_collected += count;
        state.last_error = None;

        Ok(documents)
    }

    async fn status(&self) -> AgentStatus {
        let state = self.state.read().await;
        AgentStatus {
            name: "eu_transparency".into(),
            enabled: state.enabled,
            last_run: state.last_run,
            documents_collected: state.documents_collected,
            error: state.last_error.clone(),
        }
    }
}

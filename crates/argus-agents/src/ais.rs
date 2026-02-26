use async_trait::async_trait;
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use argus_core::agent::{Agent, AgentLookup, AgentStatus, RawDocument};
use argus_core::entity::EntityType;
use argus_core::error::{ArgusError, Result};

const AISHUB_API_URL: &str = "https://data.aishub.net/ws.php";

/// AIS vessel position record from the AISHub API response.
#[derive(Debug, Deserialize)]
struct AisVesselRecord {
    #[serde(rename = "MMSI")]
    mmsi: i64,
    #[serde(rename = "NAME", default)]
    name: Option<String>,
    #[serde(rename = "LATITUDE")]
    latitude: Option<f64>,
    #[serde(rename = "LONGITUDE")]
    longitude: Option<f64>,
    #[serde(rename = "SOG")]
    speed_over_ground: Option<f64>,
    #[serde(rename = "COG")]
    course_over_ground: Option<f64>,
    #[serde(rename = "HEADING")]
    heading: Option<f64>,
    #[serde(rename = "DESTINATION", default)]
    destination: Option<String>,
    #[serde(rename = "IMO", default)]
    imo: Option<i64>,
    #[serde(rename = "CALLSIGN", default)]
    callsign: Option<String>,
    #[serde(rename = "TYPE", default)]
    vessel_type: Option<i64>,
    #[serde(rename = "NAVSTAT", default)]
    nav_status: Option<i64>,
    #[serde(rename = "TIME", default)]
    timestamp: Option<String>,
}

/// AISHub API response envelope.
///
/// The API returns a JSON array where the first element is a metadata array
/// (containing error codes / record counts) and the second element is the
/// actual data array of vessel records.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum AisHubResponse {
    /// Successful response: `[ [{"ERROR": false, ...}], [ {vessel}, ... ] ]`
    Success(Vec<serde_json::Value>),
}

#[derive(Debug)]
struct AisAgentState {
    enabled: bool,
    last_run: Option<chrono::DateTime<Utc>>,
    documents_collected: u64,
    last_error: Option<String>,
}

impl Default for AisAgentState {
    fn default() -> Self {
        Self {
            enabled: true,
            last_run: None,
            documents_collected: 0,
            last_error: None,
        }
    }
}

/// AIS (Automatic Identification System) maritime vessel tracking agent.
///
/// Fetches real-time vessel position data from the AISHub API and produces
/// one `RawDocument` per vessel sighting.
pub struct AisAgent {
    client: Client,
    state: RwLock<AisAgentState>,
    api_key: Option<String>,
}

impl AisAgent {
    pub fn new() -> Self {
        let api_key = std::env::var("AISHUB_API_KEY").ok();
        if api_key.is_none() {
            warn!("AISHUB_API_KEY not set — AIS agent will return empty results");
        }

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("argus-intel/0.1")
            .build()
            .expect("failed to build reqwest client");

        Self {
            client,
            state: RwLock::new(AisAgentState::default()),
            api_key,
        }
    }

    /// Build the AISHub request URL with the required query parameters.
    fn build_url(&self, api_key: &str) -> String {
        format!(
            "{}?username={}&format=1&output=json&compress=0",
            AISHUB_API_URL, api_key
        )
    }

    /// Parse the raw API JSON into a vec of vessel records.
    fn parse_response(&self, body: &str) -> Result<Vec<AisVesselRecord>> {
        let envelope: Vec<serde_json::Value> =
            serde_json::from_str(body).map_err(|e| ArgusError::Agent {
                agent: self.name().into(),
                message: format!("failed to parse AISHub response envelope: {e}"),
            })?;

        // The first element contains metadata; check for errors.
        if let Some(meta_array) = envelope.first().and_then(|v| v.as_array()) {
            if let Some(meta) = meta_array.first() {
                if let Some(error_code) = meta.get("ERROR").and_then(|e| e.as_bool()) {
                    if error_code {
                        let error_msg = meta
                            .get("ERROR_MESSAGE")
                            .and_then(|m| m.as_str())
                            .unwrap_or("unknown API error");
                        return Err(ArgusError::Agent {
                            agent: self.name().into(),
                            message: format!("AISHub API error: {error_msg}"),
                        });
                    }
                }
            }
        }

        // The second element is the data array.
        let data_value = envelope.get(1).ok_or_else(|| ArgusError::Agent {
            agent: self.name().into(),
            message: "AISHub response missing data array".into(),
        })?;

        let vessels: Vec<AisVesselRecord> =
            serde_json::from_value(data_value.clone()).map_err(|e| ArgusError::Agent {
                agent: self.name().into(),
                message: format!("failed to parse AISHub vessel data: {e}"),
            })?;

        Ok(vessels)
    }

    /// Convert a single vessel record into a `RawDocument`.
    fn vessel_to_document(&self, vessel: &AisVesselRecord) -> RawDocument {
        let mmsi = vessel.mmsi.to_string();

        let vessel_name = vessel
            .name
            .as_deref()
            .unwrap_or("UNKNOWN")
            .trim()
            .to_string();

        let content = format!(
            "Vessel {} (MMSI: {}) at ({}, {}), SOG: {} kn, COG: {}°, destination: {}",
            vessel_name,
            mmsi,
            vessel.latitude.unwrap_or(0.0),
            vessel.longitude.unwrap_or(0.0),
            vessel.speed_over_ground.unwrap_or(0.0),
            vessel.course_over_ground.unwrap_or(0.0),
            vessel
                .destination
                .as_deref()
                .unwrap_or("N/A")
                .trim(),
        );

        let metadata = serde_json::json!({
            "mmsi": vessel.mmsi,
            "name": vessel_name,
            "latitude": vessel.latitude,
            "longitude": vessel.longitude,
            "speed_over_ground": vessel.speed_over_ground,
            "course_over_ground": vessel.course_over_ground,
            "heading": vessel.heading,
            "destination": vessel.destination.as_deref().map(|s| s.trim()),
            "imo": vessel.imo,
            "callsign": vessel.callsign,
            "vessel_type": vessel.vessel_type,
            "nav_status": vessel.nav_status,
            "timestamp": vessel.timestamp,
        });

        RawDocument {
            source: "ais".into(),
            source_id: mmsi,
            title: Some(vessel_name),
            content,
            url: None,
            collected_at: Utc::now(),
            metadata,
        }
    }
}

#[async_trait]
impl Agent for AisAgent {
    fn name(&self) -> &str {
        "ais"
    }

    fn source_type(&self) -> &str {
        "maritime_tracking"
    }

    async fn collect(&self) -> Result<Vec<RawDocument>> {
        let api_key = match &self.api_key {
            Some(key) => key.clone(),
            None => {
                let msg = "AISHUB_API_KEY not configured";
                error!(msg);
                let mut state = self.state.write().await;
                state.last_run = Some(Utc::now());
                state.last_error = Some(msg.into());
                return Err(ArgusError::Agent {
                    agent: self.name().into(),
                    message: msg.into(),
                });
            }
        };

        let url = self.build_url(&api_key);
        info!("AIS agent collecting vessel positions from AISHub");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| {
                error!(error = %e, "AIS HTTP request failed");
                ArgusError::Agent {
                    agent: self.name().into(),
                    message: format!("HTTP request failed: {e}"),
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let msg = format!("AISHub API returned HTTP {status}");
            error!(msg);
            let mut state = self.state.write().await;
            state.last_run = Some(Utc::now());
            state.last_error = Some(msg.clone());
            return Err(ArgusError::Agent {
                agent: self.name().into(),
                message: msg,
            });
        }

        let body = response.text().await.map_err(|e| {
            error!(error = %e, "failed to read AISHub response body");
            ArgusError::Agent {
                agent: self.name().into(),
                message: format!("failed to read response body: {e}"),
            }
        })?;

        debug!(
            body_length = body.len(),
            "received AISHub response"
        );

        let vessels = self.parse_response(&body)?;
        info!(count = vessels.len(), "parsed AIS vessel records");

        let documents: Vec<RawDocument> = vessels
            .iter()
            .map(|v| self.vessel_to_document(v))
            .collect();

        let doc_count = documents.len() as u64;

        // Update internal state.
        let mut state = self.state.write().await;
        state.last_run = Some(Utc::now());
        state.documents_collected += doc_count;
        state.last_error = None;

        info!(
            documents = doc_count,
            total = state.documents_collected,
            "AIS collection complete"
        );

        Ok(documents)
    }

    async fn status(&self) -> AgentStatus {
        let state = self.state.read().await;
        AgentStatus {
            name: self.name().into(),
            enabled: state.enabled,
            last_run: state.last_run,
            documents_collected: state.documents_collected,
            error: state.last_error.clone(),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[async_trait]
impl AgentLookup for AisAgent {
    fn can_lookup(&self, entity_type: &EntityType) -> bool {
        matches!(entity_type, EntityType::Vessel)
    }

    async fn lookup(&self, _name: &str, _entity_type: &EntityType) -> Result<Vec<RawDocument>> {
        // AIS lookup requires API key and doesn't support name-based search
        // AISHub API only returns bulk data, not individual vessel queries
        if self.api_key.is_none() {
            return Ok(Vec::new());
        }
        // Would need MMSI for targeted lookup; name search not directly supported
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_name() {
        let agent = AisAgent::new();
        assert_eq!(agent.name(), "ais");
    }

    #[test]
    fn test_source_type() {
        let agent = AisAgent::new();
        assert_eq!(agent.source_type(), "maritime_tracking");
    }

    #[tokio::test]
    async fn test_status_defaults() {
        let agent = AisAgent::new();
        let status = agent.status().await;
        assert_eq!(status.name, "ais");
        assert!(status.enabled);
        assert!(status.last_run.is_none());
        assert_eq!(status.documents_collected, 0);
        assert!(status.error.is_none());
    }

    #[test]
    fn test_parse_valid_response() {
        let agent = AisAgent::new();
        let body = r#"[
            [{"ERROR": false, "RECORDS": 2}],
            [
                {
                    "MMSI": 211234567,
                    "NAME": "TESTSHIP ONE",
                    "LATITUDE": 51.9,
                    "LONGITUDE": 4.5,
                    "SOG": 12.3,
                    "COG": 180.0,
                    "HEADING": 179,
                    "DESTINATION": "ROTTERDAM",
                    "IMO": 9123456,
                    "CALLSIGN": "DABC",
                    "TYPE": 70,
                    "NAVSTAT": 0,
                    "TIME": "2026-01-15T12:00:00Z"
                },
                {
                    "MMSI": 311999888,
                    "NAME": "CARGO EXPRESS",
                    "LATITUDE": -33.8,
                    "LONGITUDE": 151.2,
                    "SOG": 0.0,
                    "COG": 0.0,
                    "HEADING": 45,
                    "DESTINATION": "SYDNEY",
                    "IMO": 9654321,
                    "CALLSIGN": "VXYZ",
                    "TYPE": 80,
                    "NAVSTAT": 5,
                    "TIME": "2026-01-15T12:05:00Z"
                }
            ]
        ]"#;

        let vessels = agent.parse_response(body).unwrap();
        assert_eq!(vessels.len(), 2);
        assert_eq!(vessels[0].mmsi, 211234567);
        assert_eq!(vessels[0].name.as_deref(), Some("TESTSHIP ONE"));
        assert_eq!(vessels[1].mmsi, 311999888);
    }

    #[test]
    fn test_parse_error_response() {
        let agent = AisAgent::new();
        let body = r#"[
            [{"ERROR": true, "ERROR_MESSAGE": "Invalid API key"}],
            []
        ]"#;

        let result = agent.parse_response(body);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            ArgusError::Agent { agent, message } => {
                assert_eq!(agent, "ais");
                assert!(message.contains("Invalid API key"));
            }
            _ => panic!("expected ArgusError::Agent"),
        }
    }

    #[test]
    fn test_vessel_to_document() {
        let agent = AisAgent::new();
        let vessel = AisVesselRecord {
            mmsi: 123456789,
            name: Some("MY VESSEL".into()),
            latitude: Some(52.0),
            longitude: Some(4.0),
            speed_over_ground: Some(10.5),
            course_over_ground: Some(270.0),
            heading: Some(268.0),
            destination: Some("HAMBURG".into()),
            imo: Some(9111111),
            callsign: Some("ABCD".into()),
            vessel_type: Some(70),
            nav_status: Some(0),
            timestamp: Some("2026-01-15T10:00:00Z".into()),
        };

        let doc = agent.vessel_to_document(&vessel);
        assert_eq!(doc.source, "ais");
        assert_eq!(doc.source_id, "123456789");
        assert_eq!(doc.title.as_deref(), Some("MY VESSEL"));
        assert!(doc.content.contains("MY VESSEL"));
        assert!(doc.content.contains("123456789"));
        assert!(doc.content.contains("HAMBURG"));
        assert_eq!(doc.metadata["mmsi"], 123456789);
        assert_eq!(doc.metadata["latitude"], 52.0);
        assert_eq!(doc.metadata["longitude"], 4.0);
        assert_eq!(doc.metadata["speed_over_ground"], 10.5);
    }

    #[test]
    fn test_vessel_to_document_missing_fields() {
        let agent = AisAgent::new();
        let vessel = AisVesselRecord {
            mmsi: 999999999,
            name: None,
            latitude: None,
            longitude: None,
            speed_over_ground: None,
            course_over_ground: None,
            heading: None,
            destination: None,
            imo: None,
            callsign: None,
            vessel_type: None,
            nav_status: None,
            timestamp: None,
        };

        let doc = agent.vessel_to_document(&vessel);
        assert_eq!(doc.source_id, "999999999");
        assert_eq!(doc.title.as_deref(), Some("UNKNOWN"));
        assert!(doc.content.contains("UNKNOWN"));
        assert!(doc.content.contains("N/A"));
    }

    #[test]
    fn test_build_url() {
        let agent = AisAgent::new();
        let url = agent.build_url("test_key_123");
        assert!(url.starts_with(AISHUB_API_URL));
        assert!(url.contains("username=test_key_123"));
        assert!(url.contains("format=1"));
        assert!(url.contains("output=json"));
    }

    #[tokio::test]
    async fn test_collect_without_api_key() {
        // Ensure the env var is not set for this test.
        std::env::remove_var("AISHUB_API_KEY");
        let agent = AisAgent {
            client: Client::new(),
            state: RwLock::new(AisAgentState::default()),
            api_key: None,
        };

        let result = agent.collect().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ArgusError::Agent { agent, message } => {
                assert_eq!(agent, "ais");
                assert!(message.contains("not configured"));
            }
            _ => panic!("expected ArgusError::Agent"),
        }

        let status = agent.status().await;
        assert!(status.error.is_some());
    }
}

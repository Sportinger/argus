use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use argus_core::agent::{Agent, AgentStatus, RawDocument};
use argus_core::error::{ArgusError, Result};

const OPENSKY_API_URL: &str = "https://opensky-network.org/api/states/all";

/// Internal mutable state for the ADS-B agent.
struct AdsbState {
    enabled: bool,
    last_run: Option<chrono::DateTime<Utc>>,
    documents_collected: u64,
    last_error: Option<String>,
}

/// Raw response from the OpenSky Network REST API.
#[derive(Debug, Deserialize)]
struct OpenSkyResponse {
    time: i64,
    states: Option<Vec<Vec<serde_json::Value>>>,
}

/// ADS-B aircraft tracking agent.
///
/// Fetches real-time aircraft positions from the OpenSky Network REST API
/// and converts each aircraft state vector into a `RawDocument`.
pub struct AdsbAgent {
    client: reqwest::Client,
    state: RwLock<AdsbState>,
}

impl AdsbAgent {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .expect("failed to build reqwest client"),
            state: RwLock::new(AdsbState {
                enabled: true,
                last_run: None,
                documents_collected: 0,
                last_error: None,
            }),
        }
    }

    /// Parse a single OpenSky state vector array into a `RawDocument`.
    ///
    /// OpenSky state vector indices:
    ///  0 - icao24 (hex string)
    ///  1 - callsign
    ///  2 - origin_country
    ///  3 - time_position
    ///  4 - last_contact
    ///  5 - longitude
    ///  6 - latitude
    ///  7 - baro_altitude (meters)
    ///  8 - on_ground
    ///  9 - velocity (m/s)
    /// 10 - true_track (degrees clockwise from north)
    /// 11 - vertical_rate (m/s)
    /// 12 - sensors
    /// 13 - geo_altitude (meters)
    /// 14 - squawk
    /// 15 - spi
    /// 16 - position_source
    fn parse_state_vector(sv: &[serde_json::Value]) -> Option<RawDocument> {
        let icao24 = sv.first()?.as_str()?.trim().to_string();
        if icao24.is_empty() {
            return None;
        }

        let callsign = sv
            .get(1)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();

        let origin_country = sv
            .get(2)
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let longitude = sv.get(5).and_then(|v| v.as_f64());
        let latitude = sv.get(6).and_then(|v| v.as_f64());
        let baro_altitude = sv.get(7).and_then(|v| v.as_f64());
        let on_ground = sv.get(8).and_then(|v| v.as_bool()).unwrap_or(false);
        let velocity = sv.get(9).and_then(|v| v.as_f64());
        let true_track = sv.get(10).and_then(|v| v.as_f64());
        let vertical_rate = sv.get(11).and_then(|v| v.as_f64());
        let geo_altitude = sv.get(13).and_then(|v| v.as_f64());
        let squawk = sv
            .get(14)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Build a human-readable content summary
        let alt_str = baro_altitude
            .map(|a| format!("{:.0}m", a))
            .unwrap_or_else(|| "unknown alt".into());
        let vel_str = velocity
            .map(|v| format!("{:.1}m/s", v))
            .unwrap_or_else(|| "unknown vel".into());
        let pos_str = match (latitude, longitude) {
            (Some(lat), Some(lon)) => format!("({:.4}, {:.4})", lat, lon),
            _ => "unknown position".into(),
        };

        let content = format!(
            "Aircraft {icao24} (callsign: {callsign}) from {origin_country} \
             at {pos_str}, altitude {alt_str}, velocity {vel_str}, on_ground={on_ground}"
        );

        let metadata = serde_json::json!({
            "icao24": icao24,
            "callsign": callsign,
            "origin_country": origin_country,
            "latitude": latitude,
            "longitude": longitude,
            "baro_altitude": baro_altitude,
            "geo_altitude": geo_altitude,
            "on_ground": on_ground,
            "velocity": velocity,
            "true_track": true_track,
            "vertical_rate": vertical_rate,
            "squawk": squawk,
        });

        let title = if callsign.is_empty() {
            format!("Aircraft {}", icao24)
        } else {
            format!("{} ({})", callsign, icao24)
        };

        Some(RawDocument {
            source: "adsb".into(),
            source_id: icao24,
            title: Some(title),
            content,
            url: Some(format!(
                "https://opensky-network.org/network/explorer?icao24={}",
                sv.first().and_then(|v| v.as_str()).unwrap_or_default().trim()
            )),
            collected_at: Utc::now(),
            metadata,
        })
    }
}

#[async_trait]
impl Agent for AdsbAgent {
    fn name(&self) -> &str {
        "adsb"
    }

    fn source_type(&self) -> &str {
        "aircraft_tracking"
    }

    async fn collect(&self) -> Result<Vec<RawDocument>> {
        info!("ADS-B agent: starting collection from OpenSky Network");

        let response = self
            .client
            .get(OPENSKY_API_URL)
            .send()
            .await
            .map_err(|e| ArgusError::Agent {
                agent: "adsb".into(),
                message: format!("HTTP request failed: {}", e),
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            let msg = format!("OpenSky API returned status {}: {}", status, body);
            warn!("ADS-B agent: {}", msg);
            let mut state = self.state.write().await;
            state.last_run = Some(Utc::now());
            state.last_error = Some(msg.clone());
            return Err(ArgusError::Agent {
                agent: "adsb".into(),
                message: msg,
            });
        }

        let opensky: OpenSkyResponse =
            response.json().await.map_err(|e| ArgusError::Agent {
                agent: "adsb".into(),
                message: format!("failed to parse OpenSky response: {}", e),
            })?;

        debug!(
            "ADS-B agent: received response with timestamp {}",
            opensky.time
        );

        let states = opensky.states.unwrap_or_default();
        let documents: Vec<RawDocument> = states
            .iter()
            .filter_map(|sv| Self::parse_state_vector(sv))
            .collect();

        let count = documents.len() as u64;
        info!("ADS-B agent: collected {} aircraft positions", count);

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
            name: "adsb".into(),
            enabled: state.enabled,
            last_run: state.last_run,
            documents_collected: state.documents_collected,
            error: state.last_error.clone(),
        }
    }
}

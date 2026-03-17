use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Service {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub probe_path: String,
    pub timeout_ms: u64,
    pub enabled: bool,
}

impl Service {
    pub fn new<S: Into<String>>(
        id: S,
        name: S,
        base_url: S,
        probe_path: S,
        timeout_ms: u64,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            base_url: base_url.into(),
            probe_path: probe_path.into(),
            timeout_ms,
            enabled: true,
        }
    }

    pub fn url(&self) -> String {
        format!("{}{}", self.base_url, self.probe_path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeResult {
    pub id: Option<i64>,
    pub timestamp: i64,
    pub service_id: String,
    pub reachable: bool,
    pub status_code: Option<u16>,
    pub latency_ms: Option<i64>,
    pub error_type: Option<String>,
    pub estimated_bytes: i64,
}

impl ProbeResult {
    pub fn new_failure<S: Into<String>>(service_id: S, error_type: &str) -> Self {
        Self {
            id: None,
            timestamp: Utc::now().timestamp_millis(),
            service_id: service_id.into(),
            reachable: false,
            status_code: None,
            latency_ms: None,
            error_type: Some(error_type.to_string()),
            estimated_bytes: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub probe_interval_ms: u64,
    pub daily_traffic_budget_kb: u64,
    #[serde(default)]
    pub traffic_used_today_kb: u64,
    #[serde(default)]
    pub traffic_day_start_ms: i64,
}


use crate::models::{ProbeResult, Service};
use chrono::Utc;
use reqwest::Client;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProbeError {
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
}

fn estimate_bytes(status: Option<u16>) -> i64 {
    // 粗略估算每次探测的流量，上限若干 KB
    match status {
        Some(_) => 4 * 1024,
        None => 1 * 1024,
    }
}

pub async fn run_probe_once(service: &Service, timestamp: i64) -> Result<ProbeResult, ProbeError> {
    let client = Client::builder()
        .use_rustls_tls()
        .build()?;

    let url = service.url();
    let start = Utc::now();

    let resp = client
        .head(&url)
        .timeout(std::time::Duration::from_millis(service.timeout_ms))
        .send()
        .await;

    match resp {
        Ok(r) => {
            let end = Utc::now();
            let latency = (end - start).num_milliseconds();
            let status = r.status().as_u16();
            let bytes = estimate_bytes(Some(status));

            Ok(ProbeResult {
                id: None,
                timestamp,
                service_id: service.id.clone(),
                reachable: true,
                status_code: Some(status),
                latency_ms: Some(latency),
                error_type: None,
                estimated_bytes: bytes,
            })
        }
        Err(err) => {
            let mut result = ProbeResult::new_failure(&service.id, "unknown");
            if err.is_timeout() {
                result.error_type = Some("timeout".to_string());
            } else if err.is_connect() {
                result.error_type = Some("network".to_string());
            }
            result.timestamp = timestamp;
            result.estimated_bytes = estimate_bytes(None);
            Ok(result)
        }
    }
}


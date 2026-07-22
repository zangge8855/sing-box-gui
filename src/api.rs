use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::mpsc::Sender;

fn get_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default()
    })
}

fn get_streaming_client() -> &'static reqwest::Client {
    static STREAM_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    STREAM_CLIENT.get_or_init(|| reqwest::Client::builder().build().unwrap_or_default())
}

/// Look up `experimental.clash_api.secret` from the on-disk run configuration
/// and attach `Authorization: Bearer <secret>` to the request when present.
/// Native sing-box profiles often run with a non-empty secret; the generated
/// Clash profile keeps it empty so requests succeed either way.
fn with_secret(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    let secret = crate::config::get_clash_api_secret();
    if secret.is_empty() {
        return builder;
    }
    let Ok(value) = HeaderValue::from_str(&format!("Bearer {}", secret)) else {
        return builder;
    };
    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("authorization"), value);
    builder.headers(headers)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrafficInfo {
    pub up: u64,
    pub down: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ProxyInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "type")]
    pub proxy_type: String,
    pub udp: Option<bool>,
    pub history: Option<Vec<serde_json::Value>>,
    pub now: Option<String>,      // only present for selectors
    pub all: Option<Vec<String>>, // list of sub-nodes for selectors
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProxiesResponse {
    pub proxies: HashMap<String, ProxyInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DelayResponse {
    pub delay: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ConnectionMetadata {
    pub network: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub conn_type: String,
    #[serde(rename = "sourceIP")]
    #[allow(dead_code)]
    pub source_ip: String,
    #[serde(rename = "destinationIP")]
    pub destination_ip: String,
    #[serde(rename = "sourcePort")]
    #[allow(dead_code)]
    pub source_port: String,
    #[serde(rename = "destinationPort")]
    #[allow(dead_code)]
    pub destination_port: String,
    pub host: String,
    /// Process path when provided by clash_api (optional).
    #[serde(default, rename = "processPath")]
    pub process_path: Option<String>,
    #[serde(default, rename = "sourceProcess")]
    pub source_process: Option<String>,
}

impl ConnectionMetadata {
    /// Best-effort display name for the originating process (file name only).
    pub fn process_display(&self) -> Option<String> {
        let p = self
            .process_path
            .as_deref()
            .filter(|s| !s.is_empty())
            .or_else(|| self.source_process.as_deref().filter(|s| !s.is_empty()))?;
        Some(p.rsplit(['\\', '/']).next().unwrap_or(p).to_string())
    }
}

#[cfg(test)]
mod traffic_tests {
    use super::*;

    #[test]
    fn process_display_strips_path() {
        let m = ConnectionMetadata {
            network: "tcp".into(),
            conn_type: "HTTP".into(),
            source_ip: String::new(),
            destination_ip: String::new(),
            source_port: String::new(),
            destination_port: String::new(),
            host: String::new(),
            process_path: Some(r"C:\Program Files\App\chrome.exe".into()),
            source_process: None,
        };
        assert_eq!(m.process_display().as_deref(), Some("chrome.exe"));
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Connection {
    pub id: String,
    pub metadata: ConnectionMetadata,
    pub upload: u64,
    pub download: u64,
    pub start: String,
    pub chains: Vec<String>,
    pub rule: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionsResponse {
    #[serde(default)]
    pub connections: Option<Vec<Connection>>,
    #[serde(rename = "downloadTotal")]
    pub download_total: u64,
    #[serde(rename = "uploadTotal")]
    pub upload_total: u64,
}

pub async fn fetch_proxies(api_port: u16) -> Result<ProxiesResponse, String> {
    let url = format!("http://127.0.0.1:{}/proxies", api_port);
    let client = get_client();
    let res = with_secret(client.get(&url))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch proxies: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Proxy API returned an error: {e}"))?;

    let body = res
        .json::<ProxiesResponse>()
        .await
        .map_err(|e| format!("Failed to parse proxies response: {}", e))?;

    Ok(body)
}

pub async fn select_proxy(api_port: u16, selector: &str, node_tag: &str) -> Result<(), String> {
    let url = format!(
        "http://127.0.0.1:{}/proxies/{}",
        api_port,
        urlencoding::encode(selector)
    );
    let client = get_client();

    let body = serde_json::json!({
        "name": node_tag
    });

    let res = with_secret(client.put(&url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to select proxy: {}", e))?;

    if res.status().is_success() {
        Ok(())
    } else {
        Err(format!("Server returned error code: {}", res.status()))
    }
}

pub async fn test_node_latency(
    api_port: u16,
    node_tag: &str,
    test_url: &str,
    timeout_ms: u32,
) -> Result<u64, String> {
    let test_url = if test_url.trim().is_empty() {
        "http://cp.cloudflare.com/generate_204"
    } else {
        test_url.trim()
    };
    let timeout_ms = timeout_ms.clamp(500, 30_000);
    let url = format!(
        "http://127.0.0.1:{}/proxies/{}/delay?url={}&timeout={}",
        api_port,
        urlencoding::encode(node_tag),
        urlencoding::encode(test_url),
        timeout_ms
    );
    let client = get_client();
    let res = with_secret(
        client
            .get(&url)
            .timeout(std::time::Duration::from_millis(timeout_ms as u64 + 1000)),
    )
    .send()
    .await
    .map_err(|e| format!("Latency test request failed: {}", e))?;

    if res.status().is_success() {
        let delay_res = res
            .json::<DelayResponse>()
            .await
            .map_err(|e| format!("Failed to parse delay response: {}", e))?;
        Ok(delay_res.delay)
    } else {
        Err(format!("Server returned status code: {}", res.status()))
    }
}

pub async fn fetch_connections(api_port: u16) -> Result<ConnectionsResponse, String> {
    let url = format!("http://127.0.0.1:{}/connections", api_port);
    let client = get_client();
    let res = with_secret(client.get(&url))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch connections: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Connections API returned an error: {e}"))?;

    let body = res
        .json::<ConnectionsResponse>()
        .await
        .map_err(|e| format!("Failed to parse connections response: {}", e))?;

    Ok(body)
}

pub async fn close_connection(api_port: u16, id: &str) -> Result<(), String> {
    let url = format!(
        "http://127.0.0.1:{}/connections/{}",
        api_port,
        urlencoding::encode(id)
    );
    let client = get_client();
    let res = with_secret(client.delete(&url))
        .send()
        .await
        .map_err(|e| format!("Failed to close connection: {}", e))?;

    if res.status().is_success() {
        Ok(())
    } else {
        Err(format!("Server returned status code: {}", res.status()))
    }
}

/// Switch Clash-compatible routing mode (Rule / Global / Direct).
pub async fn set_mode(api_port: u16, mode: &str) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{}/configs", api_port);
    let client = get_client();
    let body = serde_json::json!({ "mode": mode });
    let res = with_secret(client.patch(&url))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to set mode: {}", e))?;

    if res.status().is_success() {
        Ok(())
    } else {
        Err(format!("Server returned status code: {}", res.status()))
    }
}

/// Close all active connections via Clash API (DELETE /connections).
pub async fn close_all_connections(api_port: u16) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{}/connections", api_port);
    let client = get_client();
    let res = with_secret(client.delete(&url))
        .send()
        .await
        .map_err(|e| format!("Failed to close all connections: {}", e))?;

    if res.status().is_success() {
        Ok(())
    } else {
        // Fallback: close one by one
        match fetch_connections(api_port).await {
            Ok(resp) => {
                let conns = resp.connections.unwrap_or_default();
                let mut failures = Vec::new();
                for conn in &conns {
                    if let Err(error) = close_connection(api_port, &conn.id).await {
                        failures.push(format!("{}: {}", conn.id, error));
                    }
                }
                if failures.is_empty() {
                    Ok(())
                } else {
                    let total = failures.len();
                    failures.truncate(5);
                    Err(format!(
                        "Close-all fallback partially failed for {total} connection(s): {}",
                        failures.join("; ")
                    ))
                }
            }
            Err(e) => Err(format!(
                "Close-all failed (status {}) and fallback failed: {}",
                res.status(),
                e
            )),
        }
    }
}

const MAX_TRAFFIC_LINE_BYTES: usize = 64 * 1024;

fn append_traffic_chunk(buffer: &mut String, chunk: &str) -> bool {
    buffer.push_str(chunk);
    if buffer.len() <= MAX_TRAFFIC_LINE_BYTES {
        return false;
    }
    while buffer.len() > MAX_TRAFFIC_LINE_BYTES {
        if let Some(first_newline) = buffer.find('\n') {
            buffer.drain(..=first_newline);
        } else {
            buffer.clear();
            break;
        }
    }
    true
}

pub fn spawn_traffic_monitor(
    api_port: u16,
    sender: Sender<TrafficInfo>,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
) {
    tokio::spawn(async move {
        let url = format!("http://127.0.0.1:{}/traffic", api_port);
        let client = get_streaming_client();

        loop {
            tokio::select! {
                _ = &mut cancel_rx => {
                    break;
                }
                res_future = with_secret(client.get(&url)).send() => {
                    if let Ok(mut res) = res_future {
                        let mut line_buffer = String::new();
                        loop {
                            tokio::select! {
                                _ = &mut cancel_rx => {
                                    return;
                                }
                                chunk_res = res.chunk() => {
                                    match chunk_res {
                                        Ok(Some(chunk)) => {
                                            let chunk_str = String::from_utf8_lossy(&chunk);
                                            append_traffic_chunk(&mut line_buffer, &chunk_str);

                                            while let Some(pos) = line_buffer.find('\n') {
                                                let line = line_buffer[..pos].trim();
                                                if !line.is_empty()
                                                    && let Ok(info) = serde_json::from_str::<TrafficInfo>(line) {
                                                        let _ = sender.try_send(info);
                                                    }
                                                line_buffer.drain(..=pos);
                                            }
                                        }
                                        _ => break,
                                    }
                                }
                            }
                        }
                    }
                }
            }
            tokio::select! {
                _ = &mut cancel_rx => {
                    break;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {}
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traffic_buffer_drops_unbounded_partial_lines() {
        let mut buffer = String::new();
        assert!(append_traffic_chunk(
            &mut buffer,
            &"x".repeat(MAX_TRAFFIC_LINE_BYTES + 1)
        ));
        assert!(buffer.len() <= MAX_TRAFFIC_LINE_BYTES);
    }

    #[test]
    fn traffic_buffer_keeps_complete_lines_after_overflow() {
        let mut buffer = String::new();
        let chunk = format!("{}\n{{\"up\":1}}\n", "x".repeat(MAX_TRAFFIC_LINE_BYTES));
        append_traffic_chunk(&mut buffer, &chunk);
        assert!(buffer.len() <= MAX_TRAFFIC_LINE_BYTES);
        assert!(buffer.contains("{\"up\":1}"));
    }
}

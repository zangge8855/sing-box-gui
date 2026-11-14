use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::mpsc::Sender;

const API_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const API_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const STREAM_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const STREAM_IDLE_CHUNK_TIMEOUT: Duration = Duration::from_secs(15);

fn build_no_proxy_client(
    connect_timeout: Duration,
    request_timeout: Option<Duration>,
) -> reqwest::Client {
    let configure = || {
        let mut builder = reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(connect_timeout);
        if let Some(timeout) = request_timeout {
            builder = builder.timeout(timeout);
        }
        builder
    };

    configure()
        .build()
        .or_else(|_| configure().no_gzip().build())
        .expect("failed to build no_proxy Clash API HTTP client")
}

fn get_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| build_no_proxy_client(API_CONNECT_TIMEOUT, Some(API_REQUEST_TIMEOUT)))
}

fn get_streaming_client() -> &'static reqwest::Client {
    static STREAM_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    STREAM_CLIENT.get_or_init(|| build_no_proxy_client(STREAM_CONNECT_TIMEOUT, None))
}

fn encode_path_segment(segment: &str) -> Cow<'_, str> {
    match segment {
        "." => Cow::Borrowed("%2E"),
        ".." => Cow::Borrowed("%2E%2E"),
        _ => urlencoding::encode(segment),
    }
}

fn build_select_proxy_url(api_port: u16, selector: &str) -> String {
    format!(
        "http://127.0.0.1:{}/proxies/{}",
        api_port,
        encode_path_segment(selector)
    )
}

fn build_delay_test_url(
    api_port: u16,
    node_tag: &str,
    test_url: &str,
    timeout_ms: u32,
) -> (String, u32) {
    let normalized_url = if test_url.trim().is_empty() {
        "http://cp.cloudflare.com/generate_204"
    } else {
        test_url.trim()
    };
    let clamped_timeout = timeout_ms.clamp(500, 30_000);
    let url = format!(
        "http://127.0.0.1:{}/proxies/{}/delay?url={}&timeout={}",
        api_port,
        encode_path_segment(node_tag),
        urlencoding::encode(normalized_url),
        clamped_timeout
    );
    (url, clamped_timeout)
}

fn build_close_connection_url(api_port: u16, id: &str) -> String {
    format!(
        "http://127.0.0.1:{}/connections/{}",
        api_port,
        encode_path_segment(id)
    )
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
    builder.bearer_auth(secret)
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrafficInfo {
    #[serde(default)]
    pub up: u64,
    #[serde(default)]
    pub down: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ProxyInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "type")]
    pub proxy_type: String,
    #[serde(default)]
    pub udp: Option<bool>,
    #[serde(default)]
    pub history: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub now: Option<String>, // only present for selectors
    #[serde(default)]
    pub all: Option<Vec<String>>, // list of sub-nodes for selectors
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProxiesResponse {
    #[serde(default)]
    pub proxies: HashMap<String, ProxyInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DelayResponse {
    #[serde(default)]
    pub delay: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Default)]
pub struct ConnectionMetadata {
    #[serde(default)]
    pub network: String,
    #[serde(default, rename = "type")]
    #[allow(dead_code)]
    pub conn_type: String,
    #[serde(default, rename = "sourceIP")]
    #[allow(dead_code)]
    pub source_ip: String,
    #[serde(default, rename = "destinationIP")]
    pub destination_ip: String,
    #[serde(default, rename = "sourcePort")]
    #[allow(dead_code)]
    pub source_port: String,
    #[serde(default, rename = "destinationPort")]
    #[allow(dead_code)]
    pub destination_port: String,
    #[serde(default)]
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
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub metadata: ConnectionMetadata,
    #[serde(default)]
    pub upload: u64,
    #[serde(default)]
    pub download: u64,
    #[serde(default)]
    pub start: String,
    #[serde(default)]
    pub chains: Vec<String>,
    #[serde(default)]
    pub rule: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionsResponse {
    #[serde(default)]
    pub connections: Option<Vec<Connection>>,
    #[serde(default, rename = "downloadTotal")]
    pub download_total: u64,
    #[serde(default, rename = "uploadTotal")]
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
    let url = build_select_proxy_url(api_port, selector);
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
    let (url, timeout_ms) = build_delay_test_url(api_port, node_tag, test_url, timeout_ms);
    let client = get_client();
    let res = with_secret(
        client
            .get(&url)
            .timeout(Duration::from_millis(timeout_ms as u64 + 1000)),
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
    let url = build_close_connection_url(api_port, id);
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
        // Fallback: close remaining connections concurrently with bounded fan-out.
        match fetch_connections(api_port).await {
            Ok(resp) => {
                let conns = resp.connections.unwrap_or_default();
                let results: Vec<(String, Result<(), String>)> = stream::iter(conns)
                    .map(|conn| async move {
                        let res = close_connection(api_port, &conn.id).await;
                        (conn.id, res)
                    })
                    .buffer_unordered(8)
                    .collect()
                    .await;
                let mut failures: Vec<String> = results
                    .into_iter()
                    .filter_map(|(id, res)| res.err().map(|e| format!("{id}: {e}")))
                    .collect();
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
            if sender.is_closed() {
                break;
            }
            tokio::select! {
                _ = &mut cancel_rx => {
                    break;
                }
                send_res = tokio::time::timeout(
                    STREAM_CONNECT_TIMEOUT,
                    with_secret(client.get(&url)).send(),
                ) => {
                    let mut backoff = Duration::from_secs(1);
                    if let Ok(Ok(mut res)) = send_res {
                        if !res.status().is_success() {
                            backoff = Duration::from_secs(2);
                        } else {
                            let mut line_buffer = String::new();
                            loop {
                                if sender.is_closed() {
                                    return;
                                }
                                tokio::select! {
                                    _ = &mut cancel_rx => {
                                        return;
                                    }
                                    chunk_res = tokio::time::timeout(
                                        STREAM_IDLE_CHUNK_TIMEOUT,
                                        res.chunk(),
                                    ) => {
                                        match chunk_res {
                                            Ok(Ok(Some(chunk))) => {
                                                let chunk_str = String::from_utf8_lossy(&chunk);
                                                append_traffic_chunk(&mut line_buffer, &chunk_str);

                                                while let Some(pos) = line_buffer.find('\n') {
                                                    let line = line_buffer[..pos].trim();
                                                    if !line.is_empty()
                                                        && let Ok(info) = serde_json::from_str::<TrafficInfo>(line)
                                                        && sender.try_send(info).is_err()
                                                        && sender.is_closed()
                                                    {
                                                        return;
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
                    tokio::select! {
                        _ = &mut cancel_rx => {
                            break;
                        }
                        _ = tokio::time::sleep(backoff) => {}
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

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

    #[test]
    fn encodes_proxy_group_and_node_names_with_special_chars() {
        assert_eq!(
            build_select_proxy_url(9090, "🚀 节点 / Hong Kong #1?"),
            "http://127.0.0.1:9090/proxies/%F0%9F%9A%80%20%E8%8A%82%E7%82%B9%20%2F%20Hong%20Kong%20%231%3F"
        );
        assert_eq!(
            build_close_connection_url(9090, ".."),
            "http://127.0.0.1:9090/connections/%2E%2E"
        );
        let (delay_url, timeout) = build_delay_test_url(9090, "香港/专线 🇯🇵", "  ", 99_999);
        assert_eq!(timeout, 30_000);
        assert!(delay_url.starts_with(
            "http://127.0.0.1:9090/proxies/%E9%A6%99%E6%B8%AF%2F%E4%B8%93%E7%BA%BF%20%F0%9F%87%AF%F0%9F%87%B5/delay?"
        ));
        assert!(delay_url.contains("timeout=30000"));
        assert!(delay_url.contains("url=http%3A%2F%2Fcp.cloudflare.com%2Fgenerate_204"));
    }

    #[test]
    fn api_clients_construct_without_fallback() {
        let _ = get_client();
        let _ = get_streaming_client();
    }

    #[tokio::test]
    async fn close_all_connections_fallback_deletes_in_parallel() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let closed = Arc::new(AtomicUsize::new(0));
        let closed_srv = Arc::clone(&closed);

        let srv = tokio::spawn(async move {
            loop {
                let Ok((mut socket, _)) = listener.accept().await else {
                    break;
                };
                let closed_req = Arc::clone(&closed_srv);
                tokio::spawn(async move {
                    let mut buf = [0u8; 2048];
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    let req = String::from_utf8_lossy(&buf[..n]);
                    let first_line = req.lines().next().unwrap_or("");
                    if first_line.starts_with("DELETE /connections HTTP/") {
                        let resp = "HTTP/1.1 501 Not Implemented\r\nContent-Length: 0\r\n\r\n";
                        let _ = socket.write_all(resp.as_bytes()).await;
                    } else if first_line.starts_with("GET /connections HTTP/") {
                        let body = r#"{"connections":[{"id":"id/1","upload":0,"download":0,"chains":[],"rule":""},{"id":"id 2","upload":0,"download":0,"chains":[],"rule":""},{"id":"id3","upload":0,"download":0,"chains":[],"rule":""}]}"#;
                        let resp = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                            body.len(),
                            body
                        );
                        let _ = socket.write_all(resp.as_bytes()).await;
                    } else if first_line.starts_with("DELETE /connections/") {
                        closed_req.fetch_add(1, Ordering::SeqCst);
                        let resp = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n";
                        let _ = socket.write_all(resp.as_bytes()).await;
                    }
                });
            }
        });

        let res = close_all_connections(port).await;
        assert!(res.is_ok(), "unexpected error: {res:?}");
        assert_eq!(closed.load(Ordering::SeqCst), 3);
        srv.abort();
    }
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;
use std::sync::OnceLock;

fn get_client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default()
    })
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrafficInfo {
    pub up: u64,
    pub down: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ProxyInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub proxy_type: String,
    pub udp: Option<bool>,
    pub history: Option<Vec<serde_json::Value>>,
    pub now: Option<String>, // only present for selectors
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

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionMetadata {
    pub network: String,
    #[serde(rename = "type")]
    pub conn_type: String,
    #[serde(rename = "sourceIP")]
    pub source_ip: String,
    #[serde(rename = "destinationIP")]
    pub destination_ip: String,
    #[serde(rename = "sourcePort")]
    pub source_port: String,
    #[serde(rename = "destinationPort")]
    pub destination_port: String,
    pub host: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
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
    let res = client.get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch proxies: {}", e))?;
        
    let body = res.json::<ProxiesResponse>()
        .await
        .map_err(|e| format!("Failed to parse proxies response: {}", e))?;
        
    Ok(body)
}

pub async fn select_proxy(api_port: u16, selector: &str, node_tag: &str) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{}/proxies/{}", api_port, urlencoding::encode(selector));
    let client = get_client();
    
    let body = serde_json::json!({
        "name": node_tag
    });
    
    let res = client.put(&url)
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

pub async fn test_node_latency(api_port: u16, node_tag: &str) -> Result<u64, String> {
    let url = format!(
        "http://127.0.0.1:{}/proxies/{}/delay?url=http://cp.cloudflare.com/generate_204&timeout=2000",
        api_port,
        urlencoding::encode(node_tag)
    );
    let client = get_client();
    let res = client.get(&url)
        .send()
        .await
        .map_err(|e| format!("Latency test request failed: {}", e))?;
        
    if res.status().is_success() {
        let delay_res = res.json::<DelayResponse>()
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
    let res = client.get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch connections: {}", e))?;
        
    let body = res.json::<ConnectionsResponse>()
        .await
        .map_err(|e| format!("Failed to parse connections response: {}", e))?;
        
    Ok(body)
}

pub async fn close_connection(api_port: u16, id: &str) -> Result<(), String> {
    let url = format!("http://127.0.0.1:{}/connections/{}", api_port, id);
    let client = get_client();
    let res = client.delete(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to close connection: {}", e))?;
        
    if res.status().is_success() {
        Ok(())
    } else {
        Err(format!("Server returned status code: {}", res.status()))
    }
}

pub fn spawn_traffic_monitor(
    api_port: u16,
    sender: UnboundedSender<TrafficInfo>,
    mut cancel_rx: tokio::sync::oneshot::Receiver<()>,
) {
    tokio::spawn(async move {
        let url = format!("http://127.0.0.1:{}/traffic", api_port);
        let client = get_client();
        
        loop {
            tokio::select! {
                _ = &mut cancel_rx => {
                    break;
                }
                res_future = client.get(&url).send() => {
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
                                            line_buffer.push_str(&chunk_str);
                                            
                                            while let Some(pos) = line_buffer.find('\n') {
                                                let line = line_buffer[..pos].trim();
                                                if !line.is_empty() {
                                                    if let Ok(info) = serde_json::from_str::<TrafficInfo>(line) {
                                                        let _ = sender.send(info);
                                                    }
                                                }
                                                line_buffer = line_buffer[pos + 1..].to_string();
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

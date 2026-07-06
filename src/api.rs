use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc::UnboundedSender;


#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TrafficInfo {
    pub up: u64,
    pub down: u64,
}

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

pub async fn fetch_proxies(api_port: u16) -> Result<ProxiesResponse, String> {
    let url = format!("http://127.0.0.1:{}/proxies", api_port);
    let client = reqwest::Client::new();
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
    let client = reqwest::Client::new();
    
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
    let client = reqwest::Client::new();
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

pub fn spawn_traffic_monitor(api_port: u16, sender: UnboundedSender<TrafficInfo>) {
    tokio::spawn(async move {
        let url = format!("http://127.0.0.1:{}/traffic", api_port);
        let client = reqwest::Client::new();
        
        loop {
            // Re-attempt connection to traffic stream if disconnected
            if let Ok(mut res) = client.get(&url).send().await {
                while let Ok(Some(chunk)) = res.chunk().await {
                    // The stream returns JSON lines
                    let chunk_str = String::from_utf8_lossy(&chunk);
                    for line in chunk_str.lines() {
                        if let Ok(info) = serde_json::from_str::<TrafficInfo>(line) {
                            let _ = sender.send(info);
                        }
                    }
                }
            }
            // Sleep and retry if the connection falls
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    });
}

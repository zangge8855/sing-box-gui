use crate::state::{Tab, RoutingMode};

#[derive(Debug, Clone)]
pub enum Message {
    TabChanged(Tab),
    ToggleCore,
    CoreStatusChanged(bool),
    NewLogLine(String),
    TrafficUpdated { up: u64, down: u64 },
    ToggleSystemProxy,
    #[allow(dead_code)]
    SystemProxyStatusChanged(bool),
    SubscriptionInputChanged(String),
    DownloadSubscription,
    SubscriptionDownloaded { id: String, error: Option<String> },
    SelectProfile(String),
    DeleteProfile(String),
    UpdateSubscription(String),
    SelectNode(String),
    NodeSelected { tag: String, error: Option<String> },
    StartLatencyTest,
    NodeLatencyTested { tag: String, latency: Option<u64> },
    Tick,
    ErrorOccurred(String),
    RoutingModeChanged(RoutingMode),
    PortInputChanged(String),
    SaveSettings,
    FetchConnections,
    ConnectionsFetched(Result<crate::api::ConnectionsResponse, String>),
    CloseConnection(String),
    ConnectionClosed(Result<String, String>),
}

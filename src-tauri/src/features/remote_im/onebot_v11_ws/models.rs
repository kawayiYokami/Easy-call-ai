// OneBot v11 反向 WebSocket 服务器
// 实现 OneBot v11 协议的反向 WebSocket 连接（基于 axum WebSocket）

use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::ws::{Message as AxumWsMessage, WebSocket};
use chrono::{DateTime, Utc};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::SinkExt;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, oneshot, watch, RwLock};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

// 以下 import 供 ide_context.rs 等其他 include! 文件使用（它们仍用 tokio-tungstenite 做独立 WS 服务器）
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

const CHANNEL_LOG_LIMIT: usize = 300;

/// 渠道日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelLogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String, // "info", "warn", "error"
    pub message: String,
}

/// 渠道连接状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelConnectionStatus {
    pub channel_id: String,
    pub connected: bool,
    pub peer_addr: Option<String>,
    pub connected_at: Option<DateTime<Utc>>,
    pub listen_addr: String,
    #[serde(default)]
    pub status_text: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub login_session_key: Option<String>,
    #[serde(default)]
    pub qrcode_url: Option<String>,
}

/// OneBot v11 凭证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnebotV11WsCredentials {
    #[serde(default = "default_ws_host")]
    pub ws_host: String,
    #[serde(default = "default_ws_port")]
    pub ws_port: u16,
    #[serde(default)]
    pub ws_token: Option<String>,
}

fn default_ws_host() -> String {
    "0.0.0.0".to_string()
}

fn default_ws_port() -> u16 {
    6199
}

const NAPCAT_RECONNECT_INTERVAL_SECS: u64 = 30;
const NAPCAT_MAX_MEDIA_DOWNLOAD_SIZE_BYTES: u64 = 20 * 1024 * 1024;
const NAPCAT_ACTIVE_CONNECTION_REPLACE_TIMEOUT_MS: u64 = 1500;

impl OnebotV11WsCredentials {
    pub fn from_credentials(credentials: &Value) -> Self {
        serde_json::from_value(credentials.clone()).unwrap_or_default()
    }
}

impl Default for OnebotV11WsCredentials {
    fn default() -> Self {
        Self {
            ws_host: default_ws_host(),
            ws_port: default_ws_port(),
            ws_token: None,
        }
    }
}

/// OneBot v11 API 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OneBotApiRequest {
    action: String,
    params: Value,
    #[serde(default)]
    echo: Option<Value>,
}

/// OneBot v11 API 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OneBotApiResponse {
    status: String,
    retcode: i64,
    data: Value,
    #[serde(default)]
    echo: Option<Value>,
}

/// WebSocket 连接信息
struct WsConnection {
    /// 发送请求的通道
    tx: broadcast::Sender<String>,
    /// 等待响应的 oneshot 映射: echo -> sender
    pending_responses: Arc<RwLock<HashMap<String, oneshot::Sender<OneBotApiResponse>>>>,
    /// 连接的对端地址
    peer_addr: Option<String>,
    /// 连接时间
    connected_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
struct OnebotChannelRuntime {
    #[allow(dead_code)] // 用于测试中的 runtime 匹配验证
    id: String,
    cancel: CancellationToken,
    tasks: TaskTracker,
}

/// axum WebSocket handler 所需的共享状态
#[derive(Clone)]
struct OnebotAxumState {
    channel_id: String,
    expected_token: Option<String>,
    conn_tx: broadcast::Sender<String>,
    pending_responses: Arc<RwLock<HashMap<String, oneshot::Sender<OneBotApiResponse>>>>,
    event_tx: broadcast::Sender<Value>,
    connections: Arc<RwLock<HashMap<String, WsConnection>>>,
    connection_stop_senders: Arc<RwLock<HashMap<String, watch::Sender<bool>>>>,
    channel_logs: Arc<RwLock<HashMap<String, Vec<ChannelLogEntry>>>>,
    active_connection_gate: Arc<std::sync::atomic::AtomicBool>,
    cancel: CancellationToken,
}

/// OneBot v11 WebSocket 服务器管理器
#[derive(Clone)]
pub struct OnebotV11WsManager {
    /// 活跃连接: channel_id -> 连接信息
    connections: Arc<RwLock<HashMap<String, WsConnection>>>,
    /// 活跃连接停止信号: channel_id -> stop sender
    connection_stop_senders: Arc<RwLock<HashMap<String, watch::Sender<bool>>>>,
    /// 每个渠道独立的事件总线: channel_id -> event sender
    channel_event_senders: Arc<RwLock<HashMap<String, broadcast::Sender<Value>>>>,
    /// 渠道日志: channel_id -> 日志条目列表
    channel_logs: Arc<RwLock<HashMap<String, Vec<ChannelLogEntry>>>>,
    /// 渠道监听地址: channel_id -> listen_addr
    listen_addrs: Arc<RwLock<HashMap<String, String>>>,
    /// 渠道状态文本: channel_id -> status_text
    channel_status_texts: Arc<RwLock<HashMap<String, String>>>,
    /// 渠道最近错误: channel_id -> last_error
    channel_last_errors: Arc<RwLock<HashMap<String, String>>>,
    /// 渠道 axum serve 任务的 JoinHandle
    channel_tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
    /// 渠道派生任务组，用于 stop 时收割所有连接任务
    channel_runtimes: Arc<RwLock<HashMap<String, OnebotChannelRuntime>>>,
    /// OneBot 事件消费器停止信号: channel_id -> stop sender
    event_consumer_stop_senders: Arc<RwLock<HashMap<String, watch::Sender<bool>>>>,
    /// OneBot 事件消费器任务: channel_id -> JoinHandle
    event_consumer_tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
    /// 渠道生命周期锁，确保同一 channel 的 start/stop/reconcile 串行化
    lifecycle_locks: Arc<tokio::sync::Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
}

type AxumWsSender = SplitSink<WebSocket, AxumWsMessage>;
type AxumWsReceiver = SplitStream<WebSocket>;

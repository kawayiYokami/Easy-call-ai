const WEIXIN_OC_DEFAULT_BASE_URL: &str = "https://ilinkai.weixin.qq.com";
const WEIXIN_OC_DEFAULT_CDN_BASE_URL: &str = "https://novac2c.cdn.weixin.qq.com/c2c";
const WEIXIN_OC_DEFAULT_LONG_POLL_TIMEOUT_MS: u64 = 35_000;
const WEIXIN_OC_DEFAULT_API_TIMEOUT_MS: u64 = 15_000;
const WEIXIN_OC_DEFAULT_BOT_TYPE: &str = "3";
const WEIXIN_OC_LOGIN_TTL_SECS: i64 = 5 * 60;
const WEIXIN_OC_TEXT_ITEM_TYPE: i64 = 1;
const WEIXIN_OC_IMAGE_ITEM_TYPE: i64 = 2;
const WEIXIN_OC_VOICE_ITEM_TYPE: i64 = 3;
const WEIXIN_OC_FILE_ITEM_TYPE: i64 = 4;
const WEIXIN_OC_VIDEO_ITEM_TYPE: i64 = 5;
const WEIXIN_OC_IMAGE_UPLOAD_TYPE: i64 = 1;
const WEIXIN_OC_FILE_UPLOAD_TYPE: i64 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WeixinOcLoginSession {
    session_key: String,
    qrcode: String,
    qrcode_img_content: String,
    started_at: String,
    status: String,
    #[serde(default)]
    error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WeixinOcLoginStartInput {
    channel_id: String,
    #[serde(default)]
    force_refresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WeixinOcLoginStatusInput {
    channel_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WeixinOcLoginStartResult {
    channel_id: String,
    session_key: String,
    qrcode: String,
    qrcode_img_content: String,
    status: String,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WeixinOcLoginStatusResult {
    channel_id: String,
    connected: bool,
    status: String,
    message: String,
    #[serde(default)]
    session_key: String,
    #[serde(default)]
    qrcode: String,
    #[serde(default)]
    qrcode_img_content: String,
    #[serde(default)]
    account_id: String,
    #[serde(default)]
    user_id: String,
    #[serde(default)]
    base_url: String,
    #[serde(default)]
    last_error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WeixinOcSyncContactsResult {
    channel_id: String,
    synced_count: usize,
    message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct WeixinOcCredentials {
    #[serde(default)]
    base_url: String,
    #[serde(default)]
    cdn_base_url: String,
    #[serde(default)]
    bot_type: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    qr_poll_interval: Option<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    long_poll_timeout_ms: Option<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    api_timeout_ms: Option<u64>,
    #[serde(default)]
    token: String,
    #[serde(default)]
    account_id: String,
    #[serde(default)]
    user_id: String,
    #[serde(default)]
    sync_buf: String,
}

impl WeixinOcCredentials {
    fn from_value(value: &Value) -> Self {
        serde_json::from_value(value.clone()).unwrap_or_default()
    }

    fn normalized_base_url(&self) -> String {
        let base = self.base_url.trim().trim_end_matches('/');
        if base.is_empty() {
            WEIXIN_OC_DEFAULT_BASE_URL.to_string()
        } else {
            base.to_string()
        }
    }

    fn normalized_cdn_base_url(&self) -> String {
        let base = self.cdn_base_url.trim().trim_end_matches('/');
        if base.is_empty() {
            WEIXIN_OC_DEFAULT_CDN_BASE_URL.to_string()
        } else {
            base.to_string()
        }
    }

    fn normalized_bot_type(&self) -> String {
        let out = self.bot_type.trim();
        if out.is_empty() {
            WEIXIN_OC_DEFAULT_BOT_TYPE.to_string()
        } else {
            out.to_string()
        }
    }

    fn normalized_long_poll_timeout_ms(&self) -> u64 {
        self.long_poll_timeout_ms
            .unwrap_or(WEIXIN_OC_DEFAULT_LONG_POLL_TIMEOUT_MS)
            .clamp(5_000, 60_000)
    }

    fn normalized_api_timeout_ms(&self) -> u64 {
        self.api_timeout_ms
            .unwrap_or(WEIXIN_OC_DEFAULT_API_TIMEOUT_MS)
            .clamp(5_000, 60_000)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct WeixinOcGetBotQrCodeResp {
    qrcode: Option<String>,
    qrcode_img_content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct WeixinOcQrStatusResp {
    ret: Option<i64>,
    errcode: Option<i64>,
    errmsg: Option<String>,
    status: Option<String>,
    #[serde(alias = "botToken")]
    bot_token: Option<String>,
    #[serde(alias = "ilinkBotId")]
    ilink_bot_id: Option<String>,
    #[serde(alias = "ilinkUserId")]
    ilink_user_id: Option<String>,
    #[serde(alias = "baseUrl")]
    baseurl: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct WeixinOcGetUpdatesResp {
    ret: Option<i64>,
    errcode: Option<i64>,
    errmsg: Option<String>,
    msgs: Option<Vec<WeixinOcInboundMessage>>,
    get_updates_buf: Option<String>,
}

/// 入站消息结构，按官方 WeixinMessage 全字段对齐。
/// 部分字段当前仅解析保留（seq/session_id/message_type/message_state/create_time_ms/run_id/client_id/to_user_id），
/// 用于协议兼容与后续流式/生命周期功能，尚未参与业务分支。
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct WeixinOcInboundMessage {
    #[serde(default)]
    seq: Option<i64>,
    message_id: Option<Value>,
    #[serde(default)]
    msg_id: Option<Value>,
    #[serde(default)]
    from_user_id: Option<String>,
    #[serde(default)]
    to_user_id: Option<String>,
    #[serde(default)]
    context_token: Option<String>,
    #[serde(default)]
    session_id: Option<String>,
    #[serde(default)]
    group_id: Option<String>,
    #[serde(default)]
    message_type: Option<i64>,
    #[serde(default)]
    message_state: Option<i64>,
    #[serde(default)]
    create_time_ms: Option<i64>,
    #[serde(default)]
    run_id: Option<String>,
    #[serde(default)]
    client_id: Option<String>,
    item_list: Option<Vec<WeixinOcMessageItem>>,
}

#[derive(Debug, Clone, Deserialize)]
struct WeixinOcMessageItem {
    #[serde(rename = "type")]
    item_type: Option<i64>,
    text_item: Option<WeixinOcTextItem>,
    image_item: Option<WeixinOcImageItem>,
    voice_item: Option<WeixinOcVoiceItem>,
    file_item: Option<WeixinOcFileItem>,
    video_item: Option<WeixinOcVideoItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct WeixinOcTextItem {
    text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct WeixinOcImageItem {
    media: Option<WeixinOcMediaPayload>,
    aeskey: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct WeixinOcMediaPayload {
    encrypt_query_param: Option<String>,
    aes_key: Option<String>,
    /// 加密类型：0=只加密 fileid，1=打包缩略图/中图
    #[serde(default)]
    encrypt_type: Option<i64>,
    /// 完整下载 URL（服务端直接返回，无需客户端拼接）
    #[serde(default)]
    full_url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct WeixinOcVoiceItem {
    media: Option<WeixinOcMediaPayload>,
    /// 语音编码类型：1=pcm 2=adpcm 3=feature 4=speex 5=amr 6=silk 7=mp3 8=ogg-speex
    #[serde(default)]
    encode_type: Option<i64>,
    #[serde(default)]
    sample_rate: Option<i64>,
    #[serde(default)]
    playtime: Option<i64>,
    /// 语音转文字内容
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct WeixinOcFileItem {
    media: Option<WeixinOcMediaPayload>,
    file_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct WeixinOcVideoItem {
    media: Option<WeixinOcMediaPayload>,
}

#[derive(Debug, Clone)]
struct WeixinOcCollectedMedia {
    parts: Vec<ChatIngressPart>,
}

#[derive(Debug, Clone)]
struct WeixinOcRuntimeState {
    connected: bool,
    connected_at: Option<chrono::DateTime<chrono::Utc>>,
    base_url: String,
    account_id: String,
    user_id: String,
    session_key: String,
    qrcode: String,
    qrcode_img_content: String,
    login_status: String,
    last_error: String,
}

impl Default for WeixinOcRuntimeState {
    fn default() -> Self {
        Self {
            connected: false,
            connected_at: None,
            base_url: WEIXIN_OC_DEFAULT_BASE_URL.to_string(),
            account_id: String::new(),
            user_id: String::new(),
            session_key: String::new(),
            qrcode: String::new(),
            qrcode_img_content: String::new(),
            login_status: "idle".to_string(),
            last_error: String::new(),
        }
    }
}

const WEIXIN_OC_TYPING_TICKET_TTL_SECS: u64 = 60;

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct WeixinOcTypingTicketState {
    ilink_user_id: String,
    typing_ticket: String,
    ticket_context_token: String,
    ticket_refresh_after: std::time::Instant,
}

#[derive(Debug)]
struct WeixinOcTypingState {
    ticket_state: WeixinOcTypingTicketState,
    cancel_tx: tokio::sync::oneshot::Sender<()>,
}

pub struct WeixinOcManager {
    states: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, WeixinOcRuntimeState>>,
    >,
    login_sessions: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, WeixinOcLoginSession>>,
    >,
    stop_senders: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, tokio::sync::watch::Sender<bool>>>,
    >,
    tasks: std::sync::Arc<
        tokio::sync::RwLock<
            std::collections::HashMap<String, tauri::async_runtime::JoinHandle<()>>,
        >,
    >,
    context_tokens: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, String>>,
    >,
    typing_states: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, WeixinOcTypingState>>,
    >,
    port_service: std::sync::Arc<LocalPortServiceCore>,
}

impl WeixinOcManager {
    pub fn new() -> Self {
        Self {
            states: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            login_sessions: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            stop_senders: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            tasks: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            context_tokens: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            typing_states: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            port_service: std::sync::Arc::new(LocalPortServiceCore::new()),
        }
    }
}

impl Default for WeixinOcManager {
    fn default() -> Self {
        Self::new()
    }
}

static WEIXIN_OC_MANAGER: once_cell::sync::Lazy<std::sync::Arc<WeixinOcManager>> =
    once_cell::sync::Lazy::new(|| std::sync::Arc::new(WeixinOcManager::new()));

fn weixin_oc_manager() -> std::sync::Arc<WeixinOcManager> {
    WEIXIN_OC_MANAGER.clone()
}

fn login_session_is_fresh(login: &WeixinOcLoginSession) -> bool {
    chrono::DateTime::parse_from_rfc3339(login.started_at.trim())
        .map(|ts| {
            chrono::Utc::now()
                .signed_duration_since(ts.with_timezone(&chrono::Utc))
                .num_seconds()
                < WEIXIN_OC_LOGIN_TTL_SECS
        })
        .unwrap_or(false)
}

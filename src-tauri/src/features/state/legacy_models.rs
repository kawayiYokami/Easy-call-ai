// ==================== state 旧格式模型（仅迁移服务内部使用） ====================
// V4 迁移前，旧数据以 JSON 文件形式存在 state 目录：
//   runtime_state.json / window_layouts.json / git_panel_repo_history.json
// 本模块是唯一允许定义旧 JSON 结构的地方。迁移完成后业务代码不感知这些结构。

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyWindowLayout {
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
    #[serde(default)]
    x: Option<i32>,
    #[serde(default)]
    y: Option<i32>,
    #[serde(default)]
    maximized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LegacyWindowLayouts {
    #[serde(default)]
    windows: std::collections::HashMap<String, LegacyWindowLayout>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyRemoteImGroupMemberInfo {
    user_id: String,
    #[serde(default)]
    nickname: String,
    #[serde(default)]
    card: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyRemoteImContact {
    id: String,
    channel_id: String,
    platform: String,
    remote_contact_type: String,
    remote_contact_id: String,
    #[serde(default)]
    remote_contact_name: String,
    #[serde(default)]
    avatar_url: String,
    #[serde(default)]
    remark_name: String,
    #[serde(default)]
    allow_send: bool,
    #[serde(default)]
    allow_send_files: bool,
    #[serde(default)]
    allow_receive: bool,
    #[serde(default)]
    activation_mode: String,
    #[serde(default)]
    activation_keywords: Vec<String>,
    #[serde(default)]
    mute_keywords: Vec<String>,
    #[serde(default)]
    unmute_keywords: Vec<String>,
    #[serde(default)]
    patience_seconds: u64,
    #[serde(default)]
    mute_duration_seconds: u64,
    #[serde(default)]
    activation_cooldown_seconds: u64,
    #[serde(default)]
    route_mode: String,
    #[serde(default)]
    bound_department_id: Option<String>,
    #[serde(default)]
    bound_agent_id: Option<String>,
    #[serde(default)]
    bound_conversation_id: Option<String>,
    #[serde(default)]
    processing_mode: String,
    #[serde(default)]
    response_strategy: String,
    #[serde(default)]
    blocked_message_prefixes: Vec<String>,
    #[serde(default)]
    group_reply_pacing: Option<serde_json::Value>,
    #[serde(default)]
    last_activated_at: Option<String>,
    #[serde(default)]
    last_message_at: Option<String>,
    #[serde(default)]
    dingtalk_session_webhook: Option<String>,
    #[serde(default)]
    dingtalk_session_webhook_expired_time: Option<i64>,
    #[serde(default)]
    onebot_group_members: Vec<LegacyRemoteImGroupMemberInfo>,
    #[serde(default)]
    shell_workspaces: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyRemoteImContactCheckpoint {
    contact_id: String,
    #[serde(default)]
    atomic_revision: u64,
    #[serde(default)]
    latest_seen_message_id: Option<String>,
    #[serde(default)]
    last_boundary_message_id: Option<String>,
    #[serde(default)]
    last_boundary_covers_message_id: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    energy: Option<f64>,
    #[serde(default)]
    energy_updated_at: Option<String>,
    #[serde(default)]
    last_success_reply_at: Option<String>,
    #[serde(default)]
    group_reply_delivery: Option<serde_json::Value>,
}

// runtime_state.json 的旧结构：只需读取迁移需要的字段，
// 未列出的字段（缓存、配置等）以 Value 兜底保留，不在本模块展开。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyRuntimeStateFile {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    runtime_revision: u64,
    #[serde(default)]
    data_migration_version: u32,
    #[serde(default)]
    message_store_migration_version: u32,
    #[serde(default)]
    assistant_department_agent_id: String,
    #[serde(default)]
    response_style_id: String,
    #[serde(default)]
    pdf_read_mode: String,
    #[serde(default)]
    background_voice_screenshot_keywords: String,
    #[serde(default)]
    background_voice_screenshot_mode: String,
    #[serde(default)]
    instruction_presets: Vec<serde_json::Value>,
    #[serde(default)]
    system_notification_conversation_id: Option<String>,
    #[serde(default)]
    main_conversation_id: Option<String>,
    #[serde(default)]
    pinned_conversation_ids: Vec<String>,
    #[serde(default)]
    remote_im_contacts: Vec<LegacyRemoteImContact>,
    #[serde(default)]
    remote_im_contact_checkpoints: Vec<LegacyRemoteImContactCheckpoint>,
    #[serde(default)]
    image_text_cache: Vec<serde_json::Value>,
    #[serde(default)]
    pdf_text_cache: Vec<serde_json::Value>,
    #[serde(default)]
    pdf_image_cache: Vec<serde_json::Value>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

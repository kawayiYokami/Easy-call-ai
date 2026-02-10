
use std::{fs, io::Cursor, path::PathBuf, sync::Mutex};

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use directories::ProjectDirs;
use image::ImageFormat;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use rig::{
  OneOrMany,
  completion::{
    Message as RigMessage,
    Prompt,
    message::{AudioMediaType, ImageDetail, ImageMediaType, UserContent},
  },
  prelude::CompletionClient,
  providers::openai,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{
  AppHandle, Emitter, Manager, PhysicalPosition, Position, State,
  menu::{Menu, MenuItem},
  tray::TrayIconBuilder,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

const APP_DATA_SCHEMA_VERSION: u32 = 1;
const ARCHIVE_IDLE_SECONDS: i64 = 30 * 60;
const MAX_MULTIMODAL_BYTES: usize = 10 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiConfig {
  id: String,
  name: String,
  request_format: String,
  #[serde(default = "default_true")]
  enable_text: bool,
  #[serde(default = "default_true")]
  enable_image: bool,
  #[serde(default = "default_true")]
  enable_audio: bool,
  base_url: String,
  api_key: String,
  model: String,
}

fn default_true() -> bool {
  true
}

impl Default for ApiConfig {
  fn default() -> Self {
    Self {
      id: "default-openai".to_string(),
      name: "Default OpenAI".to_string(),
      request_format: "openai".to_string(),
      enable_text: true,
      enable_image: true,
      enable_audio: true,
      base_url: "https://api.openai.com/v1".to_string(),
      api_key: String::new(),
      model: "gpt-4o-mini".to_string(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppConfig {
  hotkey: String,
  selected_api_config_id: String,
  api_configs: Vec<ApiConfig>,
}

impl Default for AppConfig {
  fn default() -> Self {
    let api_config = ApiConfig::default();
    Self {
      hotkey: "Alt+C".to_string(),
      selected_api_config_id: api_config.id.clone(),
      api_configs: vec![api_config],
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DebugApiConfig {
  request_format: Option<String>,
  base_url: String,
  api_key: String,
  model: String,
  fixed_test_prompt: Option<String>,
  enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinaryPart {
  mime: String,
  bytes_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatInputPayload {
  text: Option<String>,
  images: Option<Vec<BinaryPart>>,
  audios: Option<Vec<BinaryPart>>,
  model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendChatRequest {
  api_config_id: Option<String>,
  agent_id: String,
  payload: ChatInputPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendChatResult {
  conversation_id: String,
  latest_user_text: String,
  assistant_text: String,
  archived_before_send: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionSelector {
  api_config_id: Option<String>,
  agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatSnapshot {
  conversation_id: String,
  latest_user: Option<ChatMessage>,
  latest_assistant: Option<ChatMessage>,
  active_message_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RefreshModelsInput {
  base_url: String,
  api_key: String,
  request_format: String,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAIModelListItem {
  id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAIModelListResponse {
  data: Vec<OpenAIModelListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentProfile {
  id: String,
  name: String,
  system_prompt: String,
  created_at: String,
  updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveAgentsInput {
  agents: Vec<AgentProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
enum MessagePart {
  Text { text: String },
  Image {
    mime: String,
    bytes_base64: String,
    name: Option<String>,
    compressed: bool,
  },
  Audio {
    mime: String,
    bytes_base64: String,
    name: Option<String>,
    compressed: bool,
  },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatMessage {
  id: String,
  role: String,
  created_at: String,
  parts: Vec<MessagePart>,
  provider_meta: Option<Value>,
  tool_call: Option<Vec<Value>>,
  mcp_call: Option<Vec<Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Conversation {
  id: String,
  title: String,
  api_config_id: String,
  agent_id: String,
  created_at: String,
  updated_at: String,
  last_assistant_at: Option<String>,
  status: String,
  messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationArchive {
  archive_id: String,
  archived_at: String,
  reason: String,
  source_conversation: Conversation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveSummary {
  archive_id: String,
  archived_at: String,
  title: String,
  message_count: usize,
  api_config_id: String,
  agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AppData {
  version: u32,
  agents: Vec<AgentProfile>,
  #[serde(default = "default_selected_agent_id")]
  selected_agent_id: String,
  #[serde(default = "default_user_alias")]
  user_alias: String,
  conversations: Vec<Conversation>,
  archived_conversations: Vec<ConversationArchive>,
}

impl Default for AppData {
  fn default() -> Self {
    Self {
      version: APP_DATA_SCHEMA_VERSION,
      agents: vec![default_agent()],
      selected_agent_id: default_selected_agent_id(),
      user_alias: default_user_alias(),
      conversations: Vec::new(),
      archived_conversations: Vec::new(),
    }
  }
}

fn default_selected_agent_id() -> String {
  "default-agent".to_string()
}

fn default_user_alias() -> String {
  "用户".to_string()
}

#[derive(Debug, Clone)]
struct ResolvedApiConfig {
  request_format: String,
  base_url: String,
  api_key: String,
  model: String,
  fixed_test_prompt: String,
}

#[derive(Debug, Clone)]
struct PreparedPrompt {
  preamble: String,
  latest_user_text: String,
  latest_images: Vec<(String, String)>,
  latest_audios: Vec<(String, String)>,
}

struct AppState {
  config_path: PathBuf,
  data_path: PathBuf,
  state_lock: Mutex<()>,
}

impl AppState {
  fn new() -> Result<Self, String> {
    let project_dirs = ProjectDirs::from("ai", "easycall", "easy-call-ai")
      .ok_or_else(|| "Failed to resolve config directory".to_string())?;
    let config_dir = project_dirs.config_dir().to_path_buf();

    Ok(Self {
      config_path: config_dir.join("config.toml"),
      data_path: config_dir.join("app_data.json"),
      state_lock: Mutex::new(()),
    })
  }
}
fn now_utc() -> OffsetDateTime {
  OffsetDateTime::now_utc()
}

fn now_iso() -> String {
  now_utc()
    .format(&Rfc3339)
    .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn parse_iso(value: &str) -> Option<OffsetDateTime> {
  OffsetDateTime::parse(value, &Rfc3339).ok()
}

fn default_agent() -> AgentProfile {
  let now = now_iso();
  AgentProfile {
    id: "default-agent".to_string(),
    name: "助理".to_string(),
    system_prompt: "你是一个耐心、友善的助理。请用短信聊天的口吻与用户交流，优先自然、简短、有人味的表达。除非用户明确要求，否则不要使用结构化输出（如分点、表格、章节）和过度正式语气。面对截图相关问题时，先结合用户上下文给出直接可执行的建议，再补充必要说明。".to_string(),
    created_at: now.clone(),
    updated_at: now,
  }
}

fn ensure_default_agent(data: &mut AppData) {
  let old_prompt = "You are a concise and helpful assistant.";
  for agent in &mut data.agents {
    if agent.id == "default-agent" {
      if agent.name == "Default Agent" {
        agent.name = "助理".to_string();
      }
      if agent.system_prompt == old_prompt {
        agent.system_prompt = "你是一个耐心、友善的助理。请用短信聊天的口吻与用户交流，优先自然、简短、有人味的表达。除非用户明确要求，否则不要使用结构化输出（如分点、表格、章节）和过度正式语气。面对截图相关问题时，先结合用户上下文给出直接可执行的建议，再补充必要说明。".to_string();
      }
    }
  }
  if data.agents.is_empty() {
    data.agents.push(default_agent());
  }
  if data.selected_agent_id.trim().is_empty()
    || !data.agents.iter().any(|a| a.id == data.selected_agent_id)
  {
    data.selected_agent_id = default_selected_agent_id();
  }
  if data.user_alias.trim().is_empty() {
    data.user_alias = default_user_alias();
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatSettings {
  selected_agent_id: String,
  user_alias: String,
}

fn ensure_parent_dir(path: &PathBuf) -> Result<(), String> {
  let parent = path
    .parent()
    .ok_or_else(|| "Config path has no parent directory".to_string())?;
  fs::create_dir_all(parent).map_err(|err| format!("Create config directory failed: {err}"))
}

fn read_config(path: &PathBuf) -> Result<AppConfig, String> {
  if !path.exists() {
    return Ok(AppConfig::default());
  }

  let content = fs::read_to_string(path).map_err(|err| format!("Read config failed: {err}"))?;
  let parsed = toml::from_str::<AppConfig>(&content).unwrap_or_default();
  if parsed.api_configs.is_empty() {
    Ok(AppConfig::default())
  } else {
    Ok(parsed)
  }
}

fn write_config(path: &PathBuf, config: &AppConfig) -> Result<(), String> {
  ensure_parent_dir(path)?;
  let toml_str =
    toml::to_string_pretty(config).map_err(|err| format!("Serialize config failed: {err}"))?;
  fs::write(path, toml_str).map_err(|err| format!("Write config failed: {err}"))
}

fn read_app_data(path: &PathBuf) -> Result<AppData, String> {
  if !path.exists() {
    return Ok(AppData::default());
  }

  let content = fs::read_to_string(path).map_err(|err| format!("Read app_data failed: {err}"))?;
  let mut parsed = serde_json::from_str::<AppData>(&content).unwrap_or_default();
  parsed.version = APP_DATA_SCHEMA_VERSION;
  ensure_default_agent(&mut parsed);
  Ok(parsed)
}

fn write_app_data(path: &PathBuf, data: &AppData) -> Result<(), String> {
  ensure_parent_dir(path)?;
  let body = serde_json::to_string_pretty(data)
    .map_err(|err| format!("Serialize app_data failed: {err}"))?;
  fs::write(path, body).map_err(|err| format!("Write app_data failed: {err}"))
}

fn candidate_debug_config_paths() -> Vec<PathBuf> {
  vec![PathBuf::from(".debug").join("api-key.json")]
}

fn read_debug_api_config() -> Result<Option<DebugApiConfig>, String> {
  for path in candidate_debug_config_paths() {
    if !path.exists() {
      continue;
    }

    let content = fs::read_to_string(&path)
      .map_err(|err| format!("Read debug config failed ({}): {err}", path.display()))?;
    let parsed = serde_json::from_str::<DebugApiConfig>(&content)
      .map_err(|err| format!("Parse debug config failed ({}): {err}", path.display()))?;
    return Ok(Some(parsed));
  }
  Ok(None)
}

fn resolve_selected_api_config(app_config: &AppConfig, requested_id: Option<&str>) -> Option<ApiConfig> {
  if app_config.api_configs.is_empty() {
    return None;
  }

  let target_id = requested_id
    .map(str::trim)
    .filter(|v| !v.is_empty())
    .unwrap_or(app_config.selected_api_config_id.as_str());

  if let Some(found) = app_config.api_configs.iter().find(|p| p.id == target_id) {
    return Some(found.clone());
  }

  app_config.api_configs.first().cloned()
}

fn resolve_api_config(app_config: &AppConfig, requested_id: Option<&str>) -> Result<ResolvedApiConfig, String> {
  if let Some(debug_cfg) = read_debug_api_config()? {
    let enabled = debug_cfg.enabled.unwrap_or(true);
    let request_format_ok = debug_cfg
      .request_format
      .as_deref()
      .map(str::trim)
      .unwrap_or("openai")
      .eq_ignore_ascii_case("openai");

    if enabled && request_format_ok {
      if debug_cfg.api_key.trim().is_empty() {
        return Err(".debug/api-key.json exists but apiKey is empty.".to_string());
      }
      return Ok(ResolvedApiConfig {
        request_format: "openai".to_string(),
        base_url: debug_cfg.base_url.trim().to_string(),
        api_key: debug_cfg.api_key.trim().to_string(),
        model: debug_cfg.model.trim().to_string(),
        fixed_test_prompt: debug_cfg
          .fixed_test_prompt
          .unwrap_or_else(|| "EASY_CALL_AI_CACHE_TEST_V1".to_string()),
      });
    }
  }

  let selected = resolve_selected_api_config(app_config, requested_id)
    .ok_or_else(|| "No API config configured. Please add at least one API config.".to_string())?;

  if selected.api_key.trim().is_empty() {
    return Err("Selected API config API key is empty. Please fill it in settings.".to_string());
  }

  Ok(ResolvedApiConfig {
    request_format: selected.request_format.trim().to_string(),
    base_url: selected.base_url.trim().to_string(),
    api_key: selected.api_key.trim().to_string(),
    model: selected.model.trim().to_string(),
    fixed_test_prompt: "EASY_CALL_AI_CACHE_TEST_V1".to_string(),
  })
}

fn is_openai_style_request_format(request_format: &str) -> bool {
  matches!(request_format.trim(), "openai" | "deepseek/kimi")
}
fn ensure_active_conversation_index(data: &mut AppData, api_config_id: &str, agent_id: &str) -> usize {
  if let Some((idx, _)) = data
    .conversations
    .iter()
    .enumerate()
    .find(|(_, c)| c.status == "active" && c.api_config_id == api_config_id && c.agent_id == agent_id)
  {
    return idx;
  }

  let now = now_iso();
  let conversation = Conversation {
    id: Uuid::new_v4().to_string(),
    title: format!("Chat {}", &now.chars().take(16).collect::<String>()),
    api_config_id: api_config_id.to_string(),
    agent_id: agent_id.to_string(),
    created_at: now.clone(),
    updated_at: now,
    last_assistant_at: None,
    status: "active".to_string(),
    messages: Vec::new(),
  };

  data.conversations.push(conversation);
  data.conversations.len() - 1
}

fn archive_if_idle(data: &mut AppData, api_config_id: &str, agent_id: &str) -> bool {
  let Some((idx, _)) = data
    .conversations
    .iter()
    .enumerate()
    .find(|(_, c)| c.status == "active" && c.api_config_id == api_config_id && c.agent_id == agent_id)
  else {
    return false;
  };

  let Some(last_assistant_at) = data.conversations[idx].last_assistant_at.as_deref().and_then(parse_iso)
  else {
    return false;
  };

  let now = now_utc();
  if now.unix_timestamp() - last_assistant_at.unix_timestamp() < ARCHIVE_IDLE_SECONDS {
    return false;
  }

  let mut source = data.conversations.remove(idx);
  source.status = "archived".to_string();
  source.updated_at = now_iso();
  data.archived_conversations.push(ConversationArchive {
    archive_id: Uuid::new_v4().to_string(),
    archived_at: now_iso(),
    reason: "idle_timeout_30m".to_string(),
    source_conversation: source,
  });

  true
}

fn compress_image_to_webp(bytes: &[u8]) -> Result<Vec<u8>, String> {
  let image = image::load_from_memory(bytes).map_err(|err| format!("Decode image failed: {err}"))?;
  let mut cursor = Cursor::new(Vec::<u8>::new());
  image
    .write_to(&mut cursor, ImageFormat::WebP)
    .map_err(|err| format!("Encode image to WebP failed: {err}"))?;
  Ok(cursor.into_inner())
}

fn build_user_parts(payload: &ChatInputPayload, api_config: &ApiConfig) -> Result<Vec<MessagePart>, String> {
  let mut parts = Vec::<MessagePart>::new();
  let mut total_binary = 0usize;

  if let Some(text) = payload.text.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
    if !api_config.enable_text {
      return Err("Current API config has text disabled.".to_string());
    }
    parts.push(MessagePart::Text {
      text: text.to_string(),
    });
  }

  if let Some(images) = &payload.images {
    if !images.is_empty() && !api_config.enable_image {
      return Err("Current API config has image disabled.".to_string());
    }

    for image in images {
      let raw = B64
        .decode(image.bytes_base64.trim())
        .map_err(|err| format!("Decode image base64 failed: {err}"))?;
      let webp = compress_image_to_webp(&raw)?;
      total_binary += webp.len();
      parts.push(MessagePart::Image {
        mime: "image/webp".to_string(),
        bytes_base64: B64.encode(webp),
        name: None,
        compressed: true,
      });
    }
  }

  if let Some(audios) = &payload.audios {
    if !audios.is_empty() && !api_config.enable_audio {
      return Err("Current API config has audio disabled.".to_string());
    }

    for audio in audios {
      let raw = B64
        .decode(audio.bytes_base64.trim())
        .map_err(|err| format!("Decode audio base64 failed: {err}"))?;
      total_binary += raw.len();
      parts.push(MessagePart::Audio {
        mime: audio.mime.trim().to_string(),
        bytes_base64: B64.encode(raw),
        name: None,
        compressed: false,
      });
    }
  }

  if total_binary > MAX_MULTIMODAL_BYTES {
    return Err(format!(
      "Multimodal payload exceeds 10MB limit ({} bytes).",
      total_binary
    ));
  }

  if parts.is_empty() {
    return Err("Request payload is empty. Provide text, image, or audio.".to_string());
  }

  Ok(parts)
}

fn render_message_for_context(message: &ChatMessage) -> String {
  let mut chunks = Vec::<String>::new();
  for part in &message.parts {
    match part {
      MessagePart::Text { text } => chunks.push(text.clone()),
      MessagePart::Image { .. } => chunks.push("[image attached]".to_string()),
      MessagePart::Audio { .. } => chunks.push("[audio attached]".to_string()),
    }
  }
  format!("{}: {}", message.role.to_uppercase(), chunks.join(" | "))
}
fn build_prompt(
  conversation: &Conversation,
  agent: &AgentProfile,
  user_alias: &str,
  current_time: &str,
) -> PreparedPrompt {
  let mut history_lines = Vec::<String>::new();
  for message in &conversation.messages {
    history_lines.push(render_message_for_context(message));
  }

  let preamble = format!(
    "[SYSTEM PROMPT]\n{}\n\n[ROLE MAPPING]\nAssistant name: {}\nUser name: {}\nRules:\n- You are the assistant named '{}'.\n- The human user is named '{}'.\n- Never treat yourself as the user.\n\n[TIME]\nCurrent UTC time: {}\n\n[CONVERSATION HISTORY]\n{}\n",
    agent.system_prompt,
    agent.name,
    user_alias,
    agent.name,
    user_alias,
    current_time,
    history_lines.join("\n")
  );

  let latest_user = conversation
    .messages
    .iter()
    .rev()
    .find(|m| m.role == "user")
    .cloned();

  let mut latest_user_text = String::new();
  let mut latest_images = Vec::<(String, String)>::new();
  let mut latest_audios = Vec::<(String, String)>::new();

  if let Some(msg) = latest_user {
    for part in msg.parts {
      match part {
        MessagePart::Text { text } => {
          if !latest_user_text.is_empty() {
            latest_user_text.push('\n');
          }
          latest_user_text.push_str(&text);
        }
        MessagePart::Image { mime, bytes_base64, .. } => latest_images.push((mime, bytes_base64)),
        MessagePart::Audio { mime, bytes_base64, .. } => latest_audios.push((mime, bytes_base64)),
      }
    }
  }

  PreparedPrompt {
    preamble,
    latest_user_text,
    latest_images,
    latest_audios,
  }
}

fn image_media_type_from_mime(mime: &str) -> Option<ImageMediaType> {
  match mime.trim().to_ascii_lowercase().as_str() {
    "image/jpeg" | "image/jpg" => Some(ImageMediaType::JPEG),
    "image/png" => Some(ImageMediaType::PNG),
    "image/gif" => Some(ImageMediaType::GIF),
    "image/webp" => Some(ImageMediaType::WEBP),
    "image/heic" => Some(ImageMediaType::HEIC),
    "image/heif" => Some(ImageMediaType::HEIF),
    "image/svg+xml" => Some(ImageMediaType::SVG),
    _ => None,
  }
}

fn audio_media_type_from_mime(mime: &str) -> Option<AudioMediaType> {
  match mime.trim().to_ascii_lowercase().as_str() {
    "audio/wav" | "audio/wave" => Some(AudioMediaType::WAV),
    "audio/mp3" | "audio/mpeg" => Some(AudioMediaType::MP3),
    "audio/aiff" => Some(AudioMediaType::AIFF),
    "audio/aac" => Some(AudioMediaType::AAC),
    "audio/ogg" => Some(AudioMediaType::OGG),
    "audio/flac" => Some(AudioMediaType::FLAC),
    _ => None,
  }
}

async fn call_model_openai_style(api_config: &ResolvedApiConfig, model_name: &str, prepared: PreparedPrompt) -> Result<String, String> {
  let mut content_items: Vec<UserContent> = vec![UserContent::text(prepared.preamble)];

  if !prepared.latest_user_text.trim().is_empty() {
    content_items.push(UserContent::text(prepared.latest_user_text));
  }

  for (mime, bytes) in prepared.latest_images {
    content_items.push(UserContent::image_base64(
      bytes,
      image_media_type_from_mime(&mime),
      Some(ImageDetail::Auto),
    ));
  }

  for (mime, bytes) in prepared.latest_audios {
    content_items.push(UserContent::audio(bytes, audio_media_type_from_mime(&mime)));
  }

  let prompt_content = OneOrMany::many(content_items)
    .map_err(|_| "Request payload is empty. Provide text, image, or audio.".to_string())?;

  let mut client_builder: openai::ClientBuilder = openai::Client::builder().api_key(&api_config.api_key);
  if !api_config.base_url.is_empty() {
    client_builder = client_builder.base_url(&api_config.base_url);
  }
  let client = client_builder
    .build()
    .map_err(|err| format!("Failed to create OpenAI client via rig: {err}"))?;

  let agent = client.completions_api().agent(model_name).build();
  let prompt_message = RigMessage::User { content: prompt_content };

  agent
    .prompt(prompt_message)
    .await
    .map_err(|err| format!("rig prompt failed: {err}"))
}

fn show_window(app: &AppHandle, label: &str) -> Result<(), String> {
  let window = app
    .get_webview_window(label)
    .ok_or_else(|| format!("Window '{label}' not found"))?;

  if let Ok(Some(monitor)) = window.current_monitor() {
    if let Ok(window_size) = window.outer_size() {
      let margin = 24_i32;
      let x = monitor.position().x + monitor.size().width as i32 - window_size.width as i32 - margin;
      let y = monitor.position().y + margin;
      let _ = window.set_position(Position::Physical(PhysicalPosition::new(x, y)));
    }
  }

  let _ = window.unminimize();
  let _ = window.show();
  let _ = window.set_focus();
  let _ = window.emit("easy-call:refresh", ());
  Ok(())
}

fn toggle_window(app: &AppHandle, label: &str) -> Result<(), String> {
  let window = app
    .get_webview_window(label)
    .ok_or_else(|| format!("Window '{label}' not found"))?;
  let visible = window
    .is_visible()
    .map_err(|err| format!("Check window visibility failed: {err}"))?;
  if visible {
    window
      .hide()
      .map_err(|err| format!("Hide window failed: {err}"))?;
    return Ok(());
  }
  show_window(app, label)
}

fn register_default_hotkey(app: &AppHandle) -> Result<(), String> {
  let shortcut = Shortcut::new(Some(Modifiers::ALT), Code::KeyC);
  app
    .global_shortcut()
    .register(shortcut)
    .map_err(|err| format!("Register hotkey failed: {err}"))
}

fn build_tray(app: &AppHandle) -> Result<(), String> {
  let config = MenuItem::with_id(app, "config", "配置", true, None::<&str>)
    .map_err(|err| format!("Create tray menu item failed: {err}"))?;
  let chat = MenuItem::with_id(app, "chat", "对话", true, None::<&str>)
    .map_err(|err| format!("Create tray menu item failed: {err}"))?;
  let archives = MenuItem::with_id(app, "archives", "归档", true, None::<&str>)
    .map_err(|err| format!("Create tray menu item failed: {err}"))?;
  let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)
    .map_err(|err| format!("Create tray menu item failed: {err}"))?;

  let menu = Menu::with_items(app, &[&config, &chat, &archives, &quit])
    .map_err(|err| format!("Create tray menu failed: {err}"))?;

  let mut tray = TrayIconBuilder::new().menu(&menu);
  if let Some(icon) = app.default_window_icon() {
    tray = tray.icon(icon.clone());
  }

  tray
    .tooltip("Easy Call AI")
    .on_menu_event(|app, event| {
      let id = event.id().as_ref();
      if id == "config" {
        let _ = show_window(app, "main");
      } else if id == "chat" {
        let _ = show_window(app, "chat");
      } else if id == "archives" {
        let _ = show_window(app, "archives");
      } else if id == "quit" {
        app.exit(0);
      }
    })
    .build(app)
    .map_err(|err| format!("Build tray failed: {err}"))?;

  Ok(())
}

fn hide_on_close(app: &AppHandle) {
  for label in ["main", "chat", "archives"] {
    if let Some(window) = app.get_webview_window(label) {
      let cloned = window.clone();
      let _ = window.on_window_event(move |event| {
        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
          api.prevent_close();
          let _ = cloned.hide();
        }
      });
    }
  }
}
#[tauri::command]
fn load_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
  let guard = state
    .state_lock
    .lock()
    .map_err(|_| "Failed to lock state mutex".to_string())?;
  let result = read_config(&state.config_path);
  drop(guard);
  result
}

#[tauri::command]
fn save_config(config: AppConfig, state: State<'_, AppState>) -> Result<AppConfig, String> {
  if config.api_configs.is_empty() {
    return Err("At least one API config must be configured.".to_string());
  }

  let guard = state
    .state_lock
    .lock()
    .map_err(|_| "Failed to lock state mutex".to_string())?;

  write_config(&state.config_path, &config)?;
  drop(guard);
  Ok(config)
}

#[tauri::command]
fn load_agents(state: State<'_, AppState>) -> Result<Vec<AgentProfile>, String> {
  let guard = state
    .state_lock
    .lock()
    .map_err(|_| "Failed to lock state mutex".to_string())?;

  let mut data = read_app_data(&state.data_path)?;
  ensure_default_agent(&mut data);
  write_app_data(&state.data_path, &data)?;
  drop(guard);
  Ok(data.agents)
}

#[tauri::command]
fn save_agents(input: SaveAgentsInput, state: State<'_, AppState>) -> Result<Vec<AgentProfile>, String> {
  if input.agents.is_empty() {
    return Err("At least one agent is required.".to_string());
  }

  let guard = state
    .state_lock
    .lock()
    .map_err(|_| "Failed to lock state mutex".to_string())?;

  let mut data = read_app_data(&state.data_path)?;
  data.agents = input.agents;
  ensure_default_agent(&mut data);
  write_app_data(&state.data_path, &data)?;
  drop(guard);
  Ok(data.agents)
}

#[tauri::command]
fn load_chat_settings(state: State<'_, AppState>) -> Result<ChatSettings, String> {
  let guard = state
    .state_lock
    .lock()
    .map_err(|_| "Failed to lock state mutex".to_string())?;

  let mut data = read_app_data(&state.data_path)?;
  ensure_default_agent(&mut data);
  write_app_data(&state.data_path, &data)?;
  drop(guard);

  Ok(ChatSettings {
    selected_agent_id: data.selected_agent_id,
    user_alias: data.user_alias,
  })
}

#[tauri::command]
fn save_chat_settings(input: ChatSettings, state: State<'_, AppState>) -> Result<ChatSettings, String> {
  let guard = state
    .state_lock
    .lock()
    .map_err(|_| "Failed to lock state mutex".to_string())?;

  let mut data = read_app_data(&state.data_path)?;
  ensure_default_agent(&mut data);
  if !data.agents.iter().any(|a| a.id == input.selected_agent_id) {
    return Err("Selected agent not found.".to_string());
  }
  data.selected_agent_id = input.selected_agent_id.clone();
  data.user_alias = if input.user_alias.trim().is_empty() {
    default_user_alias()
  } else {
    input.user_alias.trim().to_string()
  };
  write_app_data(&state.data_path, &data)?;
  drop(guard);

  Ok(input)
}

#[tauri::command]
fn get_chat_snapshot(input: SessionSelector, state: State<'_, AppState>) -> Result<ChatSnapshot, String> {
  let guard = state
    .state_lock
    .lock()
    .map_err(|_| "Failed to lock state mutex".to_string())?;

  let app_config = read_config(&state.config_path)?;
  let api_config = resolve_selected_api_config(&app_config, input.api_config_id.as_deref())
    .ok_or_else(|| "No API config available".to_string())?;

  let mut data = read_app_data(&state.data_path)?;
  ensure_default_agent(&mut data);
  if !data.agents.iter().any(|a| a.id == input.agent_id) {
    return Err("Selected agent not found.".to_string());
  }

  archive_if_idle(&mut data, &api_config.id, &input.agent_id);
  let idx = ensure_active_conversation_index(&mut data, &api_config.id, &input.agent_id);
  let conversation = &data.conversations[idx];

  let latest_user = conversation
    .messages
    .iter()
    .rev()
    .find(|m| m.role == "user")
    .cloned();
  let latest_assistant = conversation
    .messages
    .iter()
    .rev()
    .find(|m| m.role == "assistant")
    .cloned();

  write_app_data(&state.data_path, &data)?;
  drop(guard);

  Ok(ChatSnapshot {
    conversation_id: conversation.id.clone(),
    latest_user,
    latest_assistant,
    active_message_count: conversation.messages.len(),
  })
}

#[tauri::command]
fn get_active_conversation_messages(
  input: SessionSelector,
  state: State<'_, AppState>,
) -> Result<Vec<ChatMessage>, String> {
  let guard = state
    .state_lock
    .lock()
    .map_err(|_| "Failed to lock state mutex".to_string())?;

  let app_config = read_config(&state.config_path)?;
  let api_config = resolve_selected_api_config(&app_config, input.api_config_id.as_deref())
    .ok_or_else(|| "No API config available".to_string())?;

  let mut data = read_app_data(&state.data_path)?;
  ensure_default_agent(&mut data);

  archive_if_idle(&mut data, &api_config.id, &input.agent_id);
  let idx = ensure_active_conversation_index(&mut data, &api_config.id, &input.agent_id);
  let messages = data.conversations[idx].messages.clone();

  write_app_data(&state.data_path, &data)?;
  drop(guard);
  Ok(messages)
}

#[tauri::command]
fn list_archives(state: State<'_, AppState>) -> Result<Vec<ArchiveSummary>, String> {
  let guard = state
    .state_lock
    .lock()
    .map_err(|_| "Failed to lock state mutex".to_string())?;

  let data = read_app_data(&state.data_path)?;
  drop(guard);

  let mut summaries = data
    .archived_conversations
    .iter()
    .map(|archive| ArchiveSummary {
      archive_id: archive.archive_id.clone(),
      archived_at: archive.archived_at.clone(),
      title: archive.source_conversation.title.clone(),
      message_count: archive.source_conversation.messages.len(),
      api_config_id: archive.source_conversation.api_config_id.clone(),
      agent_id: archive.source_conversation.agent_id.clone(),
    })
    .collect::<Vec<_>>();
  summaries.sort_by(|a, b| b.archived_at.cmp(&a.archived_at));
  Ok(summaries)
}

#[tauri::command]
fn get_archive_messages(archive_id: String, state: State<'_, AppState>) -> Result<Vec<ChatMessage>, String> {
  let guard = state
    .state_lock
    .lock()
    .map_err(|_| "Failed to lock state mutex".to_string())?;

  let data = read_app_data(&state.data_path)?;
  drop(guard);

  let archive = data
    .archived_conversations
    .iter()
    .find(|a| a.archive_id == archive_id)
    .ok_or_else(|| "Archive not found".to_string())?;

  Ok(archive.source_conversation.messages.clone())
}
#[tauri::command]
async fn send_chat_message(input: SendChatRequest, state: State<'_, AppState>) -> Result<SendChatResult, String> {
  let (resolved_api, model_name, prepared_prompt, conversation_id, latest_user_text, archived_before_send) = {
    let guard = state
      .state_lock
      .lock()
      .map_err(|_| "Failed to lock state mutex".to_string())?;

    let app_config = read_config(&state.config_path)?;
    let api_config = resolve_selected_api_config(&app_config, input.api_config_id.as_deref())
      .ok_or_else(|| "No API config configured. Please add one.".to_string())?;
    let resolved_api = resolve_api_config(&app_config, Some(api_config.id.as_str()))?;

    if !is_openai_style_request_format(&resolved_api.request_format) {
      return Err(format!(
        "Request format '{}' is not implemented in chat router yet.",
        resolved_api.request_format
      ));
    }

    let mut data = read_app_data(&state.data_path)?;
    ensure_default_agent(&mut data);
    let agent = data
      .agents
      .iter()
      .find(|a| a.id == input.agent_id)
      .cloned()
      .ok_or_else(|| "Selected agent not found.".to_string())?;

    let archived_before_send = archive_if_idle(&mut data, &api_config.id, &input.agent_id);
    let idx = ensure_active_conversation_index(&mut data, &api_config.id, &input.agent_id);

    let user_parts = build_user_parts(&input.payload, &api_config)?;
    let latest_user_text = user_parts
      .iter()
      .map(|part| match part {
        MessagePart::Text { text } => text.clone(),
        MessagePart::Image { .. } => "[image]".to_string(),
        MessagePart::Audio { .. } => "[audio]".to_string(),
      })
      .collect::<Vec<_>>()
      .join("\n");

    let now = now_iso();
    let user_message = ChatMessage {
      id: Uuid::new_v4().to_string(),
      role: "user".to_string(),
      created_at: now.clone(),
      parts: user_parts,
      provider_meta: None,
      tool_call: None,
      mcp_call: None,
    };

    data.conversations[idx].messages.push(user_message);
    data.conversations[idx].updated_at = now;

    let conversation = data.conversations[idx].clone();
    let prepared = build_prompt(&conversation, &agent, &data.user_alias, &now_iso());

    let model_name = input
      .payload
      .model
      .as_deref()
      .map(str::trim)
      .filter(|v| !v.is_empty())
      .map(ToOwned::to_owned)
      .unwrap_or_else(|| resolved_api.model.clone());
    let conversation_id = conversation.id.clone();

    write_app_data(&state.data_path, &data)?;
    drop(guard);

    (
      resolved_api,
      model_name,
      prepared,
      conversation_id,
      latest_user_text,
      archived_before_send,
    )
  };

  let assistant_text = call_model_openai_style(&resolved_api, &model_name, prepared_prompt).await?;

  {
    let guard = state
      .state_lock
      .lock()
      .map_err(|_| "Failed to lock state mutex".to_string())?;

    let mut data = read_app_data(&state.data_path)?;
    if let Some(conversation) = data
      .conversations
      .iter_mut()
      .find(|c| c.id == conversation_id && c.status == "active")
    {
      let now = now_iso();
      conversation.messages.push(ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        created_at: now.clone(),
        parts: vec![MessagePart::Text { text: assistant_text.clone() }],
        provider_meta: None,
        tool_call: None,
        mcp_call: None,
      });
      conversation.updated_at = now.clone();
      conversation.last_assistant_at = Some(now);
      write_app_data(&state.data_path, &data)?;
    }
    drop(guard);
  }

  Ok(SendChatResult {
    conversation_id,
    latest_user_text,
    assistant_text,
    archived_before_send,
  })
}

async fn fetch_models_openai(input: &RefreshModelsInput) -> Result<Vec<String>, String> {
  let base = input.base_url.trim().trim_end_matches('/');

  let mut headers = HeaderMap::new();
  headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
  let auth = format!("Bearer {}", input.api_key.trim());
  let auth_value = HeaderValue::from_str(&auth)
    .map_err(|err| format!("Build authorization header failed: {err}"))?;
  headers.insert(AUTHORIZATION, auth_value);

  let client = reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(20))
    .default_headers(headers)
    .build()
    .map_err(|err| format!("Build HTTP client failed: {err}"))?;

  let mut candidate_urls = vec![format!("{base}/models")];
  if !base.to_ascii_lowercase().contains("/v1") {
    candidate_urls.push(format!("{base}/v1/models"));
  }
  candidate_urls.sort();
  candidate_urls.dedup();

  let mut errors = Vec::new();
  for url in candidate_urls {
    let resp = client
      .get(&url)
      .send()
      .await
      .map_err(|err| format!("Fetch model list failed ({url}): {err}"))?;

    if !resp.status().is_success() {
      let status = resp.status();
      let raw = resp.text().await.unwrap_or_default();
      let snippet = raw.chars().take(300).collect::<String>();
      errors.push(format!("{url} -> {status} | {snippet}"));
      continue;
    }

    let body = resp
      .json::<OpenAIModelListResponse>()
      .await
      .map_err(|err| format!("Parse model list failed ({url}): {err}"))?;

    let mut models = body.data.into_iter().map(|item| item.id).collect::<Vec<_>>();
    models.sort();
    models.dedup();
    return Ok(models);
  }

  if errors.is_empty() {
    Err("Fetch model list failed: no candidate URL attempted".to_string())
  } else {
    Err(format!("Fetch model list failed. Tried: {}", errors.join(" || ")))
  }
}

#[tauri::command]
async fn refresh_models(input: RefreshModelsInput) -> Result<Vec<String>, String> {
  if input.api_key.trim().is_empty() {
    return Err("API key is empty.".to_string());
  }
  if input.base_url.trim().is_empty() {
    return Err("Base URL is empty.".to_string());
  }

  match input.request_format.trim() {
    "openai" | "deepseek/kimi" => fetch_models_openai(&input).await,
    other => Err(format!("Request format '{other}' model refresh is not implemented yet.")),
  }
}

#[tauri::command]
async fn send_debug_probe(state: State<'_, AppState>) -> Result<String, String> {
  let app_config = {
    let guard = state
      .state_lock
      .lock()
      .map_err(|_| "Failed to lock state mutex".to_string())?;
    let cfg = read_config(&state.config_path)?;
    drop(guard);
    cfg
  };

  let api_config = resolve_api_config(&app_config, None)?;
  if !is_openai_style_request_format(&api_config.request_format) {
    return Err(format!(
      "Request format '{}' is not implemented in probe router yet.",
      api_config.request_format
    ));
  }

  let prepared = PreparedPrompt {
    preamble: format!("[TIME]\nCurrent UTC time: {}", now_iso()),
    latest_user_text: api_config.fixed_test_prompt.clone(),
    latest_images: Vec::new(),
    latest_audios: Vec::new(),
  };

  call_model_openai_style(&api_config, &api_config.model, prepared).await
}

fn main() {
  let state = match AppState::new() {
    Ok(state) => state,
    Err(err) => {
      eprintln!("Failed to initialize application state: {err}");
      return;
    }
  };

  tauri::Builder::default()
    .plugin(
      tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _shortcut, event| {
          if event.state() == ShortcutState::Pressed {
            let _ = toggle_window(app, "chat");
          }
        })
        .build(),
    )
    .manage(state)
    .setup(|app| {
      let app_handle = app.handle().clone();
      register_default_hotkey(&app_handle)?;
      build_tray(&app_handle)?;
      hide_on_close(&app_handle);
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      load_config,
      save_config,
      load_agents,
      save_agents,
      load_chat_settings,
      save_chat_settings,
      get_chat_snapshot,
      get_active_conversation_messages,
      list_archives,
      get_archive_messages,
      send_chat_message,
      refresh_models,
      send_debug_probe
    ])
    .run(tauri::generate_context!())
    .unwrap_or_else(|err| {
      eprintln!("error while running tauri application: {err}");
    });
}

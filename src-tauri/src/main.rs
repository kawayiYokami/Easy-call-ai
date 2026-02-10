use std::{
  fs,
  path::PathBuf,
  sync::Mutex,
};

use directories::ProjectDirs;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use rig::{
  OneOrMany,
  completion::{
    Message as RigMessage,
    Prompt,
    message::{AudioMediaType, ImageMediaType, UserContent},
  },
  prelude::CompletionClient,
  providers::openai,
};
use serde::{Deserialize, Serialize};
use tauri::{
  AppHandle, Manager, State,
  PhysicalPosition, Position,
  menu::{Menu, MenuItem},
  tray::TrayIconBuilder,
};
use tauri_plugin_global_shortcut::{
  Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState,
};

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

#[derive(Debug, Clone)]
struct ResolvedApiConfig {
  request_format: String,
  base_url: String,
  api_key: String,
  model: String,
  fixed_test_prompt: String,
}

struct AppState {
  config_path: PathBuf,
  config_lock: Mutex<()>,
}

impl AppState {
  fn new() -> Result<Self, String> {
    let project_dirs = ProjectDirs::from("ai", "easycall", "easy-call-ai")
      .ok_or_else(|| "Failed to resolve config directory".to_string())?;
    let config_dir = project_dirs.config_dir().to_path_buf();
    let config_path = config_dir.join("config.toml");

    Ok(Self {
      config_path,
      config_lock: Mutex::new(()),
    })
  }
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

fn read_app_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
  let guard = state
    .config_lock
    .lock()
    .map_err(|_| "Failed to lock config mutex".to_string())?;
  let cfg = read_config(&state.config_path)?;
  drop(guard);
  Ok(cfg)
}

fn resolve_selected_api_config(app_config: &AppConfig) -> Option<ApiConfig> {
  if app_config.api_configs.is_empty() {
    return None;
  }

  if let Some(found) = app_config
    .api_configs
    .iter()
    .find(|p| p.id == app_config.selected_api_config_id)
  {
    return Some(found.clone());
  }

  app_config.api_configs.first().cloned()
}

fn resolve_api_config(app_config: &AppConfig) -> Result<ResolvedApiConfig, String> {
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

  let selected = resolve_selected_api_config(app_config)
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
    return Ok(AppConfig::default());
  }

  Ok(parsed)
}

fn write_config(path: &PathBuf, config: &AppConfig) -> Result<(), String> {
  ensure_parent_dir(path)?;
  let toml_str =
    toml::to_string_pretty(config).map_err(|err| format!("Serialize config failed: {err}"))?;
  fs::write(path, toml_str).map_err(|err| format!("Write config failed: {err}"))
}

fn show_main_window(app: &AppHandle) -> Result<(), String> {
  let window = app
    .get_webview_window("main")
    .ok_or_else(|| "Main window not found".to_string())?;

  if let Ok(Some(monitor)) = window.current_monitor() {
    if let Ok(window_size) = window.outer_size() {
      let margin = 24_i32;
      let x = monitor.position().x + monitor.size().width as i32 - window_size.width as i32 - margin;
      let y = monitor.position().y + margin;
      let _ = window.set_position(Position::Physical(PhysicalPosition::new(x, y)));
    }
  }

  if let Err(err) = window.unminimize() {
    return Err(format!("Failed to unminimize window: {err}"));
  }
  if let Err(err) = window.show() {
    return Err(format!("Failed to show window: {err}"));
  }
  if let Err(err) = window.set_focus() {
    return Err(format!("Failed to focus window: {err}"));
  }

  Ok(())
}

fn register_default_hotkey(app: &AppHandle) -> Result<(), String> {
  let shortcut = Shortcut::new(Some(Modifiers::ALT), Code::KeyC);
  app
    .global_shortcut()
    .register(shortcut)
    .map_err(|err| format!("Register hotkey failed: {err}"))
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
    Err(format!(
      "Fetch model list failed. Tried: {}",
      errors.join(" || ")
    ))
  }
}

fn is_openai_style_request_format(request_format: &str) -> bool {
  matches!(request_format.trim(), "openai" | "deepseek/kimi")
}

#[tauri::command]
fn load_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
  let guard = state
    .config_lock
    .lock()
    .map_err(|_| "Failed to lock config mutex".to_string())?;

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
    .config_lock
    .lock()
    .map_err(|_| "Failed to lock config mutex".to_string())?;

  write_config(&state.config_path, &config)?;
  drop(guard);

  Ok(config)
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
async fn chat_with_rig(payload: ChatInputPayload, state: State<'_, AppState>) -> Result<String, String> {
  let app_config = read_app_config(state)?;
  let api_config = resolve_api_config(&app_config)?;

  if !is_openai_style_request_format(&api_config.request_format) {
    return Err(format!(
      "Request format '{}' is not implemented in chat router yet.",
      api_config.request_format
    ));
  }

  let model_name = payload
    .model
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(ToOwned::to_owned)
    .unwrap_or_else(|| api_config.model.clone());

  let mut content_items: Vec<UserContent> = Vec::new();

  if let Some(text) = payload.text {
    let trimmed = text.trim();
    if !trimmed.is_empty() {
      content_items.push(UserContent::text(trimmed));
    }
  }

  if let Some(images) = payload.images {
    for image in images {
      if image.bytes_base64.trim().is_empty() {
        continue;
      }
      content_items.push(UserContent::image_base64(
        image.bytes_base64,
        image_media_type_from_mime(&image.mime),
        None,
      ));
    }
  }

  if let Some(audios) = payload.audios {
    for audio in audios {
      if audio.bytes_base64.trim().is_empty() {
        continue;
      }
      content_items.push(UserContent::audio(
        audio.bytes_base64,
        audio_media_type_from_mime(&audio.mime),
      ));
    }
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
  let prompt_message = RigMessage::User {
    content: prompt_content,
  };

  agent
    .prompt(prompt_message)
    .await
    .map_err(|err| format!("rig prompt failed: {err}"))
}

#[tauri::command]
async fn send_debug_probe(state: State<'_, AppState>) -> Result<String, String> {
  let app_config = read_app_config(state)?;
  let api_config = resolve_api_config(&app_config)?;

  if !is_openai_style_request_format(&api_config.request_format) {
    return Err(format!(
      "Request format '{}' is not implemented in probe router yet.",
      api_config.request_format
    ));
  }

  let prompt_message = RigMessage::User {
    content: OneOrMany::one(UserContent::text(api_config.fixed_test_prompt.clone())),
  };

  let mut client_builder: openai::ClientBuilder = openai::Client::builder().api_key(&api_config.api_key);
  if !api_config.base_url.is_empty() {
    client_builder = client_builder.base_url(&api_config.base_url);
  }
  let client = client_builder
    .build()
    .map_err(|err| format!("Failed to create OpenAI client via rig: {err}"))?;

  let agent = client.completions_api().agent(api_config.model).build();
  agent
    .prompt(prompt_message)
    .await
    .map_err(|err| format!("rig probe failed: {err}"))
}

fn build_tray(app: &AppHandle) -> Result<(), String> {
  let config = MenuItem::with_id(app, "config", "配置", true, None::<&str>)
    .map_err(|err| format!("Create tray menu item failed: {err}"))?;
  let chat = MenuItem::with_id(app, "chat", "对话", true, None::<&str>)
    .map_err(|err| format!("Create tray menu item failed: {err}"))?;
  let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)
    .map_err(|err| format!("Create tray menu item failed: {err}"))?;

  let menu = Menu::with_items(app, &[&config, &chat, &quit])
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
        let _ = show_main_window(app);
      } else if id == "chat" {
        eprintln!("Tray menu '对话' clicked (not implemented yet).");
      } else if id == "quit" {
        app.exit(0);
      }
    })
    .build(app)
    .map_err(|err| format!("Build tray failed: {err}"))?;

  Ok(())
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
            let _ = show_main_window(app);
          }
        })
        .build(),
    )
    .manage(state)
    .setup(|app| {
      let app_handle = app.handle().clone();
      register_default_hotkey(&app_handle)?;
      build_tray(&app_handle)?;
      if let Some(main_window) = app_handle.get_webview_window("main") {
        let window_for_events = main_window.clone();
        let _ = main_window.on_window_event(move |event| {
          if let tauri::WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = window_for_events.hide();
          }
        });
      }
      Ok(())
    })
    .invoke_handler(tauri::generate_handler![
      load_config,
      save_config,
      refresh_models,
      chat_with_rig,
      send_debug_probe
    ])
    .run(tauri::generate_context!())
    .unwrap_or_else(|err| {
      eprintln!("error while running tauri application: {err}");
    });
}




const RUNTIME_LOG_MAX_BYTES: usize = 10 * 1024 * 1024;
const BACKEND_LOG_MAX_BYTES: u64 = 5 * 1024 * 1024;
const BACKEND_LOG_MAX_FILES: usize = 20;
const BACKEND_LOG_ARCHIVE_MAX_BYTES: u64 = 25 * 1024 * 1024;
const BACKEND_LOG_FILE_NAME: &str = "backend.log";
const DEFAULT_LLM_ROUND_LOG_CAPACITY: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LlmRoundLogHeader {
    name: String,
    value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LlmRoundLogStage {
    stage: String,
    elapsed_ms: u64,
    since_prev_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LlmRoundLogEntry {
    id: String,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    trace_id: Option<String>,
    scene: String,
    request_format: String,
    provider: String,
    model: String,
    base_url: String,
    headers: Vec<LlmRoundLogHeader>,
    tools: Option<Value>,
    response: Option<Value>,
    error: Option<String>,
    elapsed_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeline: Option<Vec<LlmRoundLogStage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    round_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rounds: Option<Vec<LlmRoundLogEntry>>,
    success: bool,
}

#[derive(Debug, Default)]
struct PendingChatRoundBuffer {
    rounds_by_chat_session: std::collections::HashMap<String, Vec<LlmRoundLogEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeLogEntry {
    id: String,
    created_at: String,
    level: String,
    message: String,
    repeat: usize,
}

#[derive(Debug, Default)]
struct RuntimeLogBuffer {
    entries: std::collections::VecDeque<RuntimeLogEntry>,
    total_bytes: usize,
}

fn runtime_log_buffer() -> &'static Mutex<RuntimeLogBuffer> {
    static RUNTIME_LOGS: OnceLock<Mutex<RuntimeLogBuffer>> = OnceLock::new();
    RUNTIME_LOGS.get_or_init(|| Mutex::new(RuntimeLogBuffer::default()))
}

fn pending_chat_round_buffer() -> &'static Mutex<PendingChatRoundBuffer> {
    static PENDING_CHAT_ROUNDS: OnceLock<Mutex<PendingChatRoundBuffer>> = OnceLock::new();
    PENDING_CHAT_ROUNDS.get_or_init(|| Mutex::new(PendingChatRoundBuffer::default()))
}

fn backend_log_write_lock() -> &'static Mutex<()> {
    static BACKEND_LOG_WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    BACKEND_LOG_WRITE_LOCK.get_or_init(|| Mutex::new(()))
}

fn backend_log_path() -> &'static Option<PathBuf> {
    static BACKEND_LOG_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();
    BACKEND_LOG_PATH.get_or_init(resolve_backend_log_path)
}

fn resolve_backend_log_path() -> Option<PathBuf> {
    let log_dir = detect_portable_runtime_root()
        .or_else(|| {
            ProjectDirs::from("ai", "easycall", "p-ai")
                .map(|dirs| dirs.config_dir().to_path_buf())
        })
        .or_else(|| current_exe_dir())
        .unwrap_or_else(std::env::temp_dir)
        .join("logs");
    if fs::create_dir_all(&log_dir).is_err() {
        return None;
    }
    Some(log_dir.join(BACKEND_LOG_FILE_NAME))
}

fn backend_log_archive_path(path: &PathBuf) -> PathBuf {
    let ts = now_utc().unix_timestamp();
    let pid = std::process::id();
    for index in 0..100_u32 {
        let suffix = if index == 0 {
            String::new()
        } else {
            format!(".{index}")
        };
        let candidate = path.with_file_name(format!("backend.{ts}.{pid}{suffix}.log"));
        if !candidate.exists() {
            return candidate;
        }
    }
    path.with_file_name(format!("backend.{ts}.{pid}.fallback.log"))
}

fn is_backend_log_archive(path: &PathBuf) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    name.starts_with("backend.") && name.ends_with(".log") && name != BACKEND_LOG_FILE_NAME
}

fn prune_backend_log_archives(path: &PathBuf) {
    let Some(dir) = path.parent() else {
        return;
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut archives = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(is_backend_log_archive)
        .filter_map(|archive_path| {
            let metadata = fs::metadata(&archive_path).ok()?;
            let modified = metadata.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            Some((archive_path, modified, metadata.len()))
        })
        .collect::<Vec<_>>();
    archives.sort_by(|a, b| b.1.cmp(&a.1));

    let mut kept_files = 0_usize;
    let mut kept_bytes = 0_u64;
    for (archive_path, _, bytes) in archives {
        let next_files = kept_files.saturating_add(1);
        let next_bytes = kept_bytes.saturating_add(bytes);
        if next_files > BACKEND_LOG_MAX_FILES || next_bytes > BACKEND_LOG_ARCHIVE_MAX_BYTES {
            let _ = fs::remove_file(archive_path);
        } else {
            kept_files = next_files;
            kept_bytes = next_bytes;
        }
    }
}

fn archive_backend_log(path: &PathBuf) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    if metadata.len() == 0 {
        return;
    }
    let target = backend_log_archive_path(path);
    if fs::rename(path, &target).is_err() {
        if fs::copy(path, &target).is_ok() {
            let _ = fs::remove_file(path);
        }
    }
    prune_backend_log_archives(path);
}

fn rotate_backend_log_if_needed(path: &PathBuf, pending_bytes: u64) {
    let current_bytes = fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    if current_bytes.saturating_add(pending_bytes) <= BACKEND_LOG_MAX_BYTES {
        return;
    }
    archive_backend_log(path);
}

fn now_log_local_rfc3339() -> String {
    let now = now_utc();
    UtcOffset::current_local_offset()
        .ok()
        .map(|offset| now.to_offset(offset))
        .unwrap_or(now)
        .replace_nanosecond(0)
        .ok()
        .and_then(|value| value.format(&Rfc3339).ok())
        .unwrap_or_else(now_utc_rfc3339)
}

fn append_backend_log_line(level: &str, message: &str) {
    let Some(path) = backend_log_path().as_ref() else {
        return;
    };
    let Ok(_guard) = backend_log_write_lock().lock() else {
        return;
    };
    let line = format!(
        "{} {:<5} {}\n",
        now_log_local_rfc3339(),
        level.to_uppercase(),
        message
    );
    rotate_backend_log_if_needed(path, line.len() as u64);
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = std::io::Write::write_all(&mut file, line.as_bytes());
    }
}

fn init_backend_file_logging() {
    let Some(path) = backend_log_path().as_ref() else {
        return;
    };
    if let Ok(_guard) = backend_log_write_lock().lock() {
        archive_backend_log(path);
    }
    append_backend_log_line("info", "========== 本次启动开始 ==========");
}

fn install_backend_file_panic_hook() {
    static BACKEND_FILE_PANIC_HOOK_INSTALLED: OnceLock<()> = OnceLock::new();
    if BACKEND_FILE_PANIC_HOOK_INSTALLED.set(()).is_err() {
        return;
    }
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .map(|value| (*value).to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic payload".to_string());
        let location = info
            .location()
            .map(|loc| format!("{}:{}", loc.file(), loc.line()))
            .unwrap_or_else(|| "unknown location".to_string());
        let thread_name = std::thread::current()
            .name()
            .unwrap_or("unnamed")
            .to_string();
        append_backend_log_line(
            "error",
            &format!(
                "[panic] location={} thread={} payload={}",
                location.trim(),
                thread_name.trim(),
                payload.trim()
            ),
        );
        previous_hook(info);
    }));
}

fn normalize_runtime_log(level: &str, message: String) -> (String, String) {
    let mut current_level = level.to_string();
    let mut text = message.trim().to_string();
    let mappings = [
        ("[ERROR]", "error"),
        ("[WARN]", "warn"),
        ("[WARNING]", "warn"),
        ("[INFO]", "info"),
        ("[DEBUG]", "debug"),
        ("[TRACE]", "trace"),
    ];
    loop {
        let mut matched = false;
        for (prefix, mapped_level) in mappings {
            if let Some(rest) = text.strip_prefix(prefix) {
                current_level = mapped_level.to_string();
                text = rest.trim_start().to_string();
                matched = true;
                break;
            }
        }
        if !matched {
            break;
        }
    }
    (current_level, text)
}

fn runtime_log_push(level: &str, message: String) {
    let _ = std::io::Write::write_all(&mut std::io::stderr(), format!("{message}\n").as_bytes());
    let (normalized_level, normalized_message) = normalize_runtime_log(level, message);
    append_backend_log_line(&normalized_level, &normalized_message);
    let created_at = now_log_local_rfc3339();
    let Ok(mut buf) = runtime_log_buffer().lock() else {
        return;
    };
    if let Some(last) = buf.entries.back_mut() {
        if last.level == normalized_level && last.message == normalized_message {
            last.repeat = last.repeat.saturating_add(1);
            last.created_at = created_at;
            return;
        }
    }
    let entry = RuntimeLogEntry {
        id: Uuid::new_v4().to_string(),
        created_at,
        level: normalized_level,
        message: normalized_message,
        repeat: 1,
    };
    let entry_bytes = entry.created_at.len() + entry.level.len() + entry.message.len();
    buf.total_bytes = buf.total_bytes.saturating_add(entry_bytes);
    buf.entries.push_back(entry);
    while buf.total_bytes > RUNTIME_LOG_MAX_BYTES {
        let Some(old) = buf.entries.pop_front() else {
            break;
        };
        let old_bytes = old.created_at.len() + old.level.len() + old.message.len();
        buf.total_bytes = buf.total_bytes.saturating_sub(old_bytes);
    }
}

fn runtime_log_info(message: String) {
    runtime_log_push("info", message);
}

fn runtime_log_warn(message: String) {
    runtime_log_push("warn", message);
}

fn runtime_log_error(message: String) {
    runtime_log_push("error", message);
}

fn runtime_log_debug(message: String) {
    runtime_log_push("debug", message);
}

fn mask_secret_keep_edges(value: &str) -> String {
    let trimmed = value.trim();
    let chars = trimmed.chars().collect::<Vec<_>>();
    if chars.len() <= 4 {
        return "****".to_string();
    }
    let head = chars.iter().take(2).collect::<String>();
    let tail = chars
        .iter()
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{head}***{tail}")
}

fn masked_auth_headers(api_key: &str) -> Vec<LlmRoundLogHeader> {
    let masked = mask_secret_keep_edges(api_key);
    vec![
        LlmRoundLogHeader {
            name: "authorization".to_string(),
            value: format!("Bearer {masked}"),
        },
        LlmRoundLogHeader {
            name: "x-api-key".to_string(),
            value: masked,
        },
    ]
}

fn openai_input_audio_format_from_mime(mime: &str) -> String {
    let normalized = mime.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "audio/wav" | "audio/wave" | "audio/x-wav" => "wav".to_string(),
        "audio/mp3" | "audio/mpeg" => "mp3".to_string(),
        _ => normalized
            .split('/')
            .nth(1)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("wav")
            .to_string(),
    }
}

fn normalize_user_content(content: &Value) -> Value {
    let Value::Array(items) = content else {
        return content.clone();
    };
    if items.is_empty() {
        return Value::String(String::new());
    }
    let mut texts = Vec::<String>::new();
    for item in items {
        let Value::Object(obj) = item else {
            return content.clone();
        };
        if obj.get("type").and_then(Value::as_str) != Some("text") {
            return content.clone();
        }
        texts.push(
            obj.get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        );
    }
    if texts.len() == 1 {
        return Value::String(texts.remove(0));
    }
    content.clone()
}

fn normalize_prepared_prompt_messages(messages: &mut [Value]) {
    for msg in messages.iter_mut() {
        let Value::Object(obj) = msg else {
            continue;
        };
        if obj.get("role").and_then(Value::as_str) != Some("user") {
            continue;
        }
        let Some(content) = obj.get("content").cloned() else {
            continue;
        };
        obj.insert("content".to_string(), normalize_user_content(&content));
    }
}

fn prepared_prompt_latest_user_text_blocks_for_json(prepared: &PreparedPrompt) -> Vec<String> {
    let mut blocks = Vec::<String>::new();
    for text in [
        prepared.latest_user_text.trim(),
        prepared.latest_user_meta_text.trim(),
    ] {
        if !text.is_empty() {
            blocks.push(text.to_string());
        }
    }
    blocks.extend(prepared_prompt_latest_user_extra_blocks(prepared));
    blocks
}

fn prepared_prompt_to_messages_json(prepared: &PreparedPrompt) -> Vec<Value> {
    let mut messages = Vec::<Value>::new();
    if !prepared.preamble.trim().is_empty() {
        messages.push(serde_json::json!({
            "role": "system",
            "content": prepared.preamble
        }));
    }

    let normalized_history_messages = normalized_prepared_history_messages(&prepared.history_messages);
    for hm in &normalized_history_messages {
        if hm.role == "assistant" {
            let mut msg = serde_json::Map::new();
            msg.insert("role".to_string(), Value::String("assistant".to_string()));
            if hm.text.trim().is_empty() {
                msg.insert("content".to_string(), Value::Null);
            } else {
                msg.insert("content".to_string(), Value::String(hm.text.clone()));
            }
            if let Some(reasoning) = &hm.reasoning_content {
                msg.insert("reasoning_content".to_string(), Value::String(reasoning.clone()));
            }
            if let Some(calls) = &hm.tool_calls {
                msg.insert(
                    "tool_calls".to_string(),
                    Value::Array(
                        normalize_prompt_tool_calls(calls)
                            .iter()
                            .filter_map(normalized_tool_call_to_history_value)
                            .collect(),
                    ),
                );
            }
            messages.push(Value::Object(msg));
            continue;
        }

        if hm.role == "tool" {
            let mut msg = serde_json::Map::new();
            msg.insert("role".to_string(), Value::String("tool".to_string()));
            msg.insert("content".to_string(), Value::String(hm.text.clone()));
            if let Some(call_id) = &hm.tool_call_id {
                msg.insert("tool_call_id".to_string(), Value::String(call_id.clone()));
            }
            messages.push(Value::Object(msg));
            continue;
        }

        if hm.role == "user" {
            let mut content = Vec::<Value>::new();
            if !hm.text.trim().is_empty() {
                content.push(serde_json::json!({
                    "type": "text",
                    "text": hm.text,
                }));
            }
            for block in &hm.extra_text_blocks {
                if block.trim().is_empty() {
                    continue;
                }
                content.push(serde_json::json!({
                    "type": "text",
                    "text": block,
                }));
            }
            if let Some(time_text) = &hm.user_time_text {
                if !time_text.trim().is_empty() {
                    content.push(serde_json::json!({
                        "type": "text",
                        "text": time_text,
                    }));
                }
            }
            for image in &hm.images {
                if image.mime.trim().eq_ignore_ascii_case("application/pdf") {
                    content.push(serde_json::json!({
                        "type": "file",
                        "mime": image.mime,
                        "bytesBase64": image.content
                    }));
                } else {
                    let image_url = if is_remote_binary_url(&image.content) {
                        image.content.clone()
                    } else {
                        format!("data:{};base64,{}", image.mime, image.content)
                    };
                    content.push(serde_json::json!({
                        "type": "image_url",
                        "image_url": {
                            "url": image_url,
                            "detail": "auto"
                        }
                    }));
                }
            }
            for audio in &hm.audios {
                content.push(serde_json::json!({
                    "type": "input_audio",
                    "input_audio": {
                        "data": audio.content.clone(),
                        "format": openai_input_audio_format_from_mime(&audio.mime)
                    }
                }));
            }
            messages.push(serde_json::json!({
                "role": "user",
                "content": content,
            }));
            continue;
        }

        messages.push(serde_json::json!({
            "role": hm.role,
            "content": hm.text
        }));
    }

    let mut latest_user_content = Vec::<Value>::new();
    for text_block in prepared_prompt_latest_user_text_blocks_for_json(prepared) {
        latest_user_content.push(serde_json::json!({
            "type": "text",
            "text": text_block
        }));
    }
    for image in &prepared.latest_images {
        if image.mime.trim().eq_ignore_ascii_case("application/pdf") {
            latest_user_content.push(serde_json::json!({
                "type": "file",
                "mime": image.mime,
                "bytesBase64": image.content
            }));
        } else {
            let image_url = if is_remote_binary_url(&image.content) {
                image.content.clone()
            } else {
                format!("data:{};base64,{}", image.mime, image.content)
            };
            latest_user_content.push(serde_json::json!({
                "type": "image_url",
                "image_url": {
                    "url": image_url,
                    "detail": "auto"
                }
            }));
        }
    }
    for audio in &prepared.latest_audios {
        latest_user_content.push(serde_json::json!({
            "type": "input_audio",
            "input_audio": {
                "data": audio.content,
                "format": openai_input_audio_format_from_mime(&audio.mime)
            }
        }));
    }
    if !latest_user_content.is_empty() {
        messages.push(serde_json::json!({
            "role": "user",
            "content": latest_user_content
        }));
    }
    normalize_prepared_prompt_messages(&mut messages);
    messages
}

fn model_reply_to_log_value(reply: &ModelReply) -> Value {
    serde_json::json!({
        "assistantText": reply.assistant_text,
        "reasoningStandard": reply.reasoning_standard,
        "reasoningInline": reply.reasoning_inline,
        "toolHistoryEvents": reply.tool_history_events
    })
}

fn build_llm_round_log_entry(
    trace_id: Option<String>,
    scene: &str,
    request_format: RequestFormat,
    provider_name: &str,
    model_name: &str,
    base_url: &str,
    headers: Vec<LlmRoundLogHeader>,
    tools: Option<Value>,
    response: Option<Value>,
    error: Option<String>,
    elapsed_ms: u64,
    timeline: Option<Vec<LlmRoundLogStage>>,
) -> LlmRoundLogEntry {
    let success = error.as_ref().map(|value| value.trim().is_empty()).unwrap_or(true);
    LlmRoundLogEntry {
        id: Uuid::new_v4().to_string(),
        created_at: now_log_local_rfc3339(),
        trace_id,
        scene: scene.to_string(),
        request_format: request_format.as_str().to_string(),
        provider: provider_name.to_string(),
        model: model_name.to_string(),
        base_url: base_url.to_string(),
        headers,
        tools,
        response,
        error: error.filter(|v| !v.trim().is_empty()),
        elapsed_ms,
        timeline,
        round_count: None,
        tool_call_count: None,
        rounds: None,
        success,
    }
}

fn llm_round_log_group_key(
    scene: &str,
    trace_id: Option<&str>,
    group_key: Option<&str>,
) -> Option<String> {
    match scene {
        "chat" => group_key
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| {
                trace_id
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| value.strip_prefix("round-").unwrap_or(value).to_string())
            }),
        "chat_pipeline" => group_key
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| {
                trace_id
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
            }),
        _ => None,
    }
}

fn log_entry_tool_call_count(entry: &LlmRoundLogEntry) -> usize {
    let Some(response) = entry.response.as_ref() else {
        return 0;
    };
    if let Some(tool_calls) = response.get("toolCalls").and_then(Value::as_array) {
        return tool_calls.len();
    }
    let Some(events) = response
        .get("toolHistoryEvents")
        .and_then(Value::as_array)
    else {
        return 0;
    };
    events
        .iter()
        .filter_map(|event| event.get("tool_calls").and_then(Value::as_array))
        .map(|calls| calls.len())
        .sum()
}

fn normalize_llm_round_log_capacity(value: u32) -> usize {
    match value {
        1 => 1,
        3 => 3,
        10 => 10,
        0 => DEFAULT_LLM_ROUND_LOG_CAPACITY,
        value if value < 3 => 1,
        value if value < 10 => 3,
        _ => 10,
    }
}

fn llm_round_log_capacity_for_state(state: &AppState) -> usize {
    state_read_config_cached(state)
        .map(|config| normalize_llm_round_log_capacity(config.llm_round_log_capacity))
        .unwrap_or(DEFAULT_LLM_ROUND_LOG_CAPACITY)
}

fn trim_display_llm_logs(
    logs: &mut std::collections::VecDeque<LlmRoundLogEntry>,
    capacity: usize,
) {
    while logs.len() > capacity {
        let _ = logs.pop_front();
    }
}

fn push_display_llm_log(
    logs: &mut std::collections::VecDeque<LlmRoundLogEntry>,
    entry: LlmRoundLogEntry,
    capacity: usize,
) {
    logs.push_back(entry);
    trim_display_llm_logs(logs, capacity);
}

fn push_llm_round_log(
    state: Option<&AppState>,
    trace_id: Option<String>,
    group_key: Option<String>,
    scene: &str,
    request_format: RequestFormat,
    provider_name: &str,
    model_name: &str,
    base_url: &str,
    headers: Vec<LlmRoundLogHeader>,
    tools: Option<Value>,
    response: Option<Value>,
    error: Option<String>,
    elapsed_ms: u64,
    timeline: Option<Vec<LlmRoundLogStage>>,
) {
    let Some(app_state) = state else {
        return;
    };
    let entry = build_llm_round_log_entry(
        trace_id.clone(),
        scene,
        request_format,
        provider_name,
        model_name,
        base_url,
        headers,
        tools,
        response,
        error,
        elapsed_ms,
        timeline,
    );
    if scene == "chat" {
        let Some(group_key) =
            llm_round_log_group_key(scene, trace_id.as_deref(), group_key.as_deref())
        else {
            return;
        };
        let Ok(mut pending) = pending_chat_round_buffer().lock() else {
            return;
        };
        pending
            .rounds_by_chat_session
            .entry(group_key)
            .or_default()
            .push(entry);
        return;
    }
    if scene == "chat_pipeline" {
        let rounds = llm_round_log_group_key(scene, trace_id.as_deref(), group_key.as_deref())
            .and_then(|group_key| {
                pending_chat_round_buffer()
                    .lock()
                    .ok()
                    .and_then(|mut pending| pending.rounds_by_chat_session.remove(&group_key))
            })
            .unwrap_or_default();
        let round_count = rounds.len();
        let tool_call_count = rounds.iter().map(log_entry_tool_call_count).sum();
        let mut pipeline_entry = entry;
        pipeline_entry.round_count = Some(round_count);
        pipeline_entry.tool_call_count = Some(tool_call_count);
        if !rounds.is_empty() {
            pipeline_entry.rounds = Some(rounds);
        }
        let capacity = llm_round_log_capacity_for_state(app_state);
        let Ok(mut logs) = app_state.llm_round_logs.lock() else {
            return;
        };
        push_display_llm_log(&mut logs, pipeline_entry, capacity);
        return;
    }
    let capacity = llm_round_log_capacity_for_state(app_state);
    let Ok(mut logs) = app_state.llm_round_logs.lock() else {
        return;
    };
    push_display_llm_log(&mut logs, entry, capacity);
}

fn latest_chat_round_headers_and_tools(
    state: &AppState,
    chat_session_key: Option<&str>,
    request_format: RequestFormat,
    provider_name: &str,
    model_name: &str,
    base_url: &str,
) -> (Vec<LlmRoundLogHeader>, Option<Value>) {
    if let Some(group_key) = chat_session_key
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if let Ok(pending) = pending_chat_round_buffer().lock() {
            if let Some(rounds) = pending.rounds_by_chat_session.get(group_key) {
                if let Some(entry) = rounds.iter().rev().find(|entry| {
                    entry.scene == "chat"
                        && entry.request_format == request_format.as_str()
                        && entry.provider == provider_name
                        && entry.model == model_name
                        && entry.base_url == base_url
                }) {
                    return (entry.headers.clone(), entry.tools.clone());
                }
            }
        }
    }
    let Ok(logs) = state.llm_round_logs.lock() else {
        return (Vec::new(), None);
    };
    let Some(entry) = logs.iter().rev().find(|entry| {
        entry.scene == "chat"
            && entry.request_format == request_format.as_str()
            && entry.provider == provider_name
            && entry.model == model_name
            && entry.base_url == base_url
    }) else {
        return (Vec::new(), None);
    };
    (entry.headers.clone(), entry.tools.clone())
}

#[tauri::command]
fn list_recent_llm_round_logs(state: State<'_, AppState>) -> Result<Vec<LlmRoundLogEntry>, String> {
    let capacity = llm_round_log_capacity_for_state(&state);
    let logs = state
        .llm_round_logs
        .lock()
        .map_err(|_| "Failed to lock llm round logs".to_string())?;
    Ok(logs
        .iter()
        .skip(logs.len().saturating_sub(capacity))
        .cloned()
        .collect::<Vec<_>>())
}

#[tauri::command]
fn clear_recent_llm_round_logs(state: State<'_, AppState>) -> Result<bool, String> {
    let mut logs = state
        .llm_round_logs
        .lock()
        .map_err(|_| "Failed to lock llm round logs".to_string())?;
    logs.clear();
    pending_chat_round_buffer()
        .lock()
        .map_err(|_| "Failed to lock pending chat round logs".to_string())?
        .rounds_by_chat_session
        .clear();
    Ok(true)
}

#[tauri::command]
fn list_recent_runtime_logs() -> Result<Vec<RuntimeLogEntry>, String> {
    let logs = runtime_log_buffer()
        .lock()
        .map_err(|_| "Failed to lock runtime logs".to_string())?;
    Ok(logs.entries.iter().cloned().collect::<Vec<_>>())
}

#[tauri::command]
fn list_runtime_logs_since(since_created_at: Option<String>) -> Result<Vec<RuntimeLogEntry>, String> {
    let logs = runtime_log_buffer()
        .lock()
        .map_err(|_| "Failed to lock runtime logs".to_string())?;
    let anchor = since_created_at
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("");
    if anchor.is_empty() {
        return Ok(logs.entries.iter().cloned().collect());
    }
    Ok(logs
        .entries
        .iter()
        .filter(|entry| entry.created_at.as_str() > anchor)
        .cloned()
        .collect())
}

#[tauri::command]
fn clear_recent_runtime_logs() -> Result<bool, String> {
    let mut logs = runtime_log_buffer()
        .lock()
        .map_err(|_| "Failed to lock runtime logs".to_string())?;
    logs.entries.clear();
    logs.total_bytes = 0;
    Ok(true)
}

#[tauri::command]
fn append_runtime_log_probe(message: Option<String>) -> Result<bool, String> {
    let msg = message
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("运行日志窗口已打开");
    runtime_log_info(format!("[运行日志] {}", msg));
    Ok(true)
}

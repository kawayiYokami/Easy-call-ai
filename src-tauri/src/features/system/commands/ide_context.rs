#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextReferenceInput {
    id: String,
    file_path: String,
    #[serde(default)]
    start_line: Option<u32>,
    #[serde(default)]
    end_line: Option<u32>,
    #[serde(default)]
    content: String,
    #[serde(default)]
    language_id: Option<String>,
    source: String,
    captured_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpsertIdeContextSnapshotInput {
    client_id: String,
    #[serde(default)]
    auth_token: Option<String>,
    #[serde(default)]
    editor: String,
    #[serde(default)]
    workspace_roots: Vec<String>,
    #[serde(default)]
    references: Vec<IdeContextReferenceInput>,
    #[serde(default)]
    updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextWorkspaceQueryInput {
    #[serde(default)]
    workspaces: Vec<IdeContextWorkspaceInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextWorkspaceInput {
    path: String,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextReferenceItemOutput {
    id: String,
    workspace_path: String,
    workspace_name: String,
    file_path: String,
    file_name: String,
    relative_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_line: Option<u32>,
    display_label: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    language_id: Option<String>,
    source: String,
    captured_at: String,
    text_block: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextWorkspaceGroupOutput {
    workspace_path: String,
    workspace_name: String,
    references: Vec<IdeContextReferenceItemOutput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextQueryResultOutput {
    groups: Vec<IdeContextWorkspaceGroupOutput>,
    updated_at: String,
}

const IDE_CONTEXT_BRIDGE_HOST: &str = "127.0.0.1";
const IDE_CONTEXT_BRIDGE_BIND_HOST: &str = "0.0.0.0";
const IDE_CONTEXT_BRIDGE_BASE_PORT: u16 = 43129;
const IDE_CONTEXT_BRIDGE_MAX_PORT: u16 = 43139;
const IDE_CONTEXT_BRIDGE_PATH: &str = "/ide-context";
const IDE_CONTEXT_CHAT_BRIDGE_PATH: &str = "/chat";
const IDE_CONTEXT_BRIDGE_DISCOVERY_FILE: &str = "p-ai-ide-context-bridge.json";
const IDE_CONTEXT_SNAPSHOT_TTL_SECS: i64 = 30;
const IDE_CONTEXT_AUTH_TOKEN_TTL_SECS: i64 = 7 * 24 * 60 * 60;
static IDE_CONTEXT_BRIDGE_STARTED: AtomicBool = AtomicBool::new(false);
static IDE_CONTEXT_BRIDGE_SHUTDOWN: OnceLock<
    Arc<Mutex<Option<tokio_util::sync::CancellationToken>>>,
> = OnceLock::new();
static IDE_CONTEXT_BRIDGE_SERVER_TASK: OnceLock<
    Arc<Mutex<Option<tauri::async_runtime::JoinHandle<()>>>>,
> = OnceLock::new();
static IDE_CONTEXT_CHAT_CLIENTS: OnceLock<
    Arc<Mutex<std::collections::HashMap<String, tokio::sync::mpsc::UnboundedSender<serde_json::Value>>>>,
> = OnceLock::new();

#[derive(Debug, Clone)]
struct IdeContextRuntime {
    snapshots: Arc<Mutex<std::collections::HashMap<String, IdeContextSnapshot>>>,
    bridge_auth: Arc<Mutex<IdeContextBridgeAuthRuntime>>,
    current_port: Arc<Mutex<Option<u16>>>,
}

#[derive(Debug, Default)]
struct IdeContextBridgeAuthRuntime {
    valid_tokens: std::collections::HashMap<String, OffsetDateTime>,
    remote_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextPersistedBridgeToken {
    token: String,
    expires_at: String,
}

impl IdeContextRuntime {
    fn new() -> Self {
        let mut bridge_auth = IdeContextBridgeAuthRuntime::default();
        bridge_auth.remote_password = ide_context_generate_remote_password();
        Self {
            snapshots: Arc::new(Mutex::new(std::collections::HashMap::new())),
            bridge_auth: Arc::new(Mutex::new(bridge_auth)),
            current_port: Arc::new(Mutex::new(None)),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextUpdatedEvent {
    client_id: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextBridgeDiscovery {
    url: String,
    bridge_url: String,
    chat_url: String,
    host: String,
    bind_host: String,
    port: u16,
    path: String,
    chat_path: String,
    pid: u32,
    updated_at: String,
    #[serde(default)]
    token: String,
    remote_password: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebAccessInfoOutput {
    running: bool,
    enabled: bool,
    configured_port: u16,
    port: u16,
    local_url: String,
    remote_urls: Vec<String>,
    remote_password: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatJsonRpcRequest {
    #[serde(default)]
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatJsonRpcError {
    code: i32,
    message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatAuthLoginInput {
    #[serde(default)]
    password: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatConversationInput {
    conversation_id: String,
    workspace_path: Option<String>,
    workspace_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatConversationBlockPageInput {
    conversation_id: String,
    #[serde(default)]
    block_id: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatCreateConversationInput {
    #[serde(default)]
    department_id: Option<String>,
    #[serde(default)]
    agent_id: Option<String>,
    #[serde(default)]
    title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatSendInput {
    conversation_id: String,
    text: String,
    #[serde(default)]
    extra_text_blocks: Vec<String>,
    #[serde(default)]
    images: Vec<IdeChatImageInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatImageInput {
    mime: String,
    bytes_base64: String,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatStopInput {
    conversation_id: String,
    #[serde(default)]
    partial_assistant_text: String,
    #[serde(default)]
    partial_stream_blocks: Vec<AssistantStreamBlock>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatSelectModelInput {
    conversation_id: String,
    #[serde(default)]
    api_config_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatResolveTerminalApprovalInput {
    request_id: String,
    approved: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatWorkspacePermissionInput {
    conversation_id: String,
    access: String,
    #[serde(default)]
    workspace_path: Option<String>,
    #[serde(default)]
    workspace_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatRewindInput {
    conversation_id: String,
    message_id: String,
    #[serde(default, rename = "agentId")]
    _agent_id: Option<String>,
    #[serde(default)]
    undo_apply_patch: bool,
}

fn ide_chat_avatar_data_url(state: &AppState, path: Option<&str>) -> String {
    let Some(path) = path.map(str::trim).filter(|value| !value.is_empty()) else {
        return String::new();
    };
    let Ok(avatars_dir) = avatar_storage_dir(state) else {
        return String::new();
    };
    let Ok(root) = fs::canonicalize(&avatars_dir) else {
        return String::new();
    };
    let Ok(target) = fs::canonicalize(path) else {
        return String::new();
    };
    if !target.starts_with(&root) {
        return String::new();
    }
    let Ok(metadata) = fs::metadata(&target) else {
        return String::new();
    };
    if !metadata.is_file() {
        return String::new();
    }
    let ext = target
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    let mime = match ext.as_str() {
        "webp" => "image/webp",
        "png" => "image/png",
        _ => return String::new(),
    };
    let Ok(bytes) = fs::read(&target) else {
        return String::new();
    };
    format!("data:{mime};base64,{}", B64.encode(bytes))
}

fn ide_chat_persona_payload(state: &AppState, active_agent_id: Option<&str>) -> Result<Value, String> {
    let runtime = state_read_runtime_state_cached(state)?;
    let agents = state_read_agents_cached(state)?;
    let user_alias = runtime.user_alias.trim();
    let active_agent_id = active_agent_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| runtime.assistant_department_agent_id.trim());
    let mut persona_name_map = serde_json::Map::new();
    let mut persona_avatar_url_map = serde_json::Map::new();
    let mut assistant_name = String::new();
    let mut assistant_avatar_url = String::new();
    let mut user_avatar_url = String::new();
    for agent in &agents {
        let id = agent.id.trim();
        if id.is_empty() {
            continue;
        }
        let name = agent.name.trim();
        persona_name_map.insert(
            id.to_string(),
            serde_json::json!(if name.is_empty() { id } else { name }),
        );
        let avatar_url = ide_chat_avatar_data_url(state, agent.avatar_path.as_deref());
        if !avatar_url.is_empty() {
            persona_avatar_url_map.insert(id.to_string(), serde_json::json!(avatar_url.clone()));
        }
        if id == USER_PERSONA_ID || agent.is_built_in_user {
            if !avatar_url.is_empty() {
                user_avatar_url = avatar_url.clone();
            }
        }
        if id == active_agent_id {
            assistant_name = if name.is_empty() { id.to_string() } else { name.to_string() };
            assistant_avatar_url = avatar_url;
        }
    }
    if assistant_name.is_empty() {
        assistant_name = active_agent_id.to_string();
    }
    Ok(serde_json::json!({
        "userAlias": if user_alias.is_empty() { default_user_alias() } else { user_alias.to_string() },
        "userAvatarUrl": user_avatar_url,
        "assistantName": assistant_name,
        "assistantAvatarUrl": assistant_avatar_url,
        "personaNameMap": persona_name_map,
        "personaAvatarUrlMap": persona_avatar_url_map,
    }))
}

fn ide_chat_model_payload_for_conversation(state: &AppState, conversation: &Conversation) -> Result<Value, String> {
    let config = state_read_config_cached(state)?;
    let department_primary_id = config
        .departments
        .iter()
        .find(|department| department.id.trim() == conversation.department_id.trim())
        .map(department_primary_api_config_id)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| config.assistant_department_api_config_id.trim().to_string());
    let resolved_department_primary_id = resolve_model_role_api_config_id(&config, &department_primary_id)
        .unwrap_or_else(|| department_primary_id.clone());
    let preferred_id = repair_conversation_preferred_model_for_snapshot(state, conversation)?;
    let conversation_call_primary_id = preferred_id
        .as_deref()
        .unwrap_or(resolved_department_primary_id.as_str())
        .to_string();
    let options = config
        .api_configs
        .iter()
        .filter(|api| is_text_chat_api(api))
        .map(|api| {
            serde_json::json!({
                "id": api.id,
                "name": api.name,
                "requestFormat": api.request_format,
                "model": api.model,
                "enableText": api.enable_text,
                "enableImage": api.enable_image,
            })
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "conversationCallPrimaryApiConfigId": conversation_call_primary_id,
        "preferredChatModelId": preferred_id,
        "chatModelOptions": options,
    }))
}

fn ide_chat_workspace_permission_payload(
    state: &AppState,
    conversation: &Conversation,
) -> Result<Value, String> {
    let workspaces = terminal_allowed_workspaces_for_conversation_canonical(state, Some(conversation))?;
    let main = workspaces
        .iter()
        .find(|workspace| workspace.level == SHELL_WORKSPACE_LEVEL_MAIN)
        .or_else(|| workspaces.iter().find(|workspace| workspace.level == SHELL_WORKSPACE_LEVEL_SYSTEM));
    let access = main
        .map(|workspace| workspace.access.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| SHELL_WORKSPACE_ACCESS_APPROVAL.to_string());
    Ok(serde_json::json!({
        "access": access,
        "workspaceName": main.map(|workspace| workspace.name.clone()).unwrap_or_default(),
        "rootPath": main.map(|workspace| workspace.path.to_string_lossy().to_string()).unwrap_or_default(),
    }))
}

fn ide_chat_workspace_permission(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatConversationInput>(params)?;
    let conversation = state_read_conversation_cached(state, input.conversation_id.trim())?;
    ide_chat_workspace_permission_payload(state, &conversation)
}

fn ide_chat_select_workspace_permission(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatWorkspacePermissionInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let access = match input.access.trim() {
        SHELL_WORKSPACE_ACCESS_READ_ONLY => SHELL_WORKSPACE_ACCESS_READ_ONLY.to_string(),
        SHELL_WORKSPACE_ACCESS_APPROVAL => SHELL_WORKSPACE_ACCESS_APPROVAL.to_string(),
        SHELL_WORKSPACE_ACCESS_FULL_ACCESS => SHELL_WORKSPACE_ACCESS_FULL_ACCESS.to_string(),
        _ => return Err("Unsupported workspace access".to_string()),
    };
    let conversation = state_read_conversation_cached(state, conversation_id)?;
    let mut workspaces = conversation.shell_workspaces.clone();
    let mut changed = false;
    for workspace in workspaces.iter_mut() {
        if normalize_shell_workspace_level_text(&workspace.level) == SHELL_WORKSPACE_LEVEL_MAIN {
            workspace.access = access.clone();
            changed = true;
        }
    }
    if !changed {
        let workspace_path = input.workspace_path.as_deref().map(str::trim).unwrap_or_default();
        if workspace_path.is_empty() {
            return Err("当前会话没有主工作目录，无法设置权限。".to_string());
        }
        let fallback_name = workspace_path
            .replace('\\', "/")
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("VS Code")
            .to_string();
        let name = input
            .workspace_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(fallback_name.as_str())
            .to_string();
        workspaces.push(ShellWorkspaceConfig {
            id: "vscode-sidebar-main-workspace".to_string(),
            name,
            path: workspace_path.to_string(),
            level: SHELL_WORKSPACE_LEVEL_MAIN.to_string(),
            access: access.clone(),
            built_in: false,
        });
    }
    let normalized_workspaces = normalize_conversation_shell_workspaces(state, &workspaces);
    let updated = apply_conversation_chat_workspace_changes(
        state,
        conversation_id,
        Some(None),
        Some(normalized_workspaces),
        None,
    )?;
    ide_chat_workspace_permission_payload(state, &updated)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatWorkspaceListInput {
    conversation_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatWorkspaceDirectoryListInput {
    path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatFileReaderReadInput {
    path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatReadPlanFileInput {
    conversation_id: String,
    path: String,
}

fn ide_chat_workspace_list(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatWorkspaceListInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let conversation = state_read_conversation_cached(state, conversation_id)?;
    let workspaces = terminal_allowed_workspaces_for_conversation_canonical(state, Some(&conversation))?;
    let main = workspaces
        .iter()
        .find(|ws| ws.level == SHELL_WORKSPACE_LEVEL_MAIN)
        .or_else(|| workspaces.iter().find(|ws| ws.level == SHELL_WORKSPACE_LEVEL_SYSTEM));
    let root_path = main
        .map(|ws| ws.path.to_string_lossy().to_string())
        .unwrap_or_default();
    let workspace_name = main
        .map(|ws| ws.name.clone())
        .unwrap_or_default();
    let autonomous_mode = conversation.shell_autonomous_mode;
    let workspace_values: Vec<Value> = workspaces
        .iter()
        .map(|ws| {
            serde_json::json!({
                "id": ws.id,
                "name": ws.name,
                "level": ws.level,
                "access": ws.access,
                "builtIn": ws.built_in,
                "path": ws.path.to_string_lossy().to_string(),
            })
        })
        .collect();
    Ok(serde_json::json!({
        "workspaces": workspace_values,
        "rootPath": root_path,
        "workspaceName": workspace_name,
        "autonomousMode": autonomous_mode,
    }))
}

fn ide_chat_workspace_directory_list(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatWorkspaceDirectoryListInput>(params)?;
    let payload = list_file_reader_directory(input.path)?;
    let directories: Vec<Value> = payload
        .entries
        .into_iter()
        .filter(|entry| entry.is_directory)
        .map(|entry| {
            serde_json::json!({
                "path": entry.path,
                "name": entry.name,
            })
        })
        .collect();
    Ok(serde_json::json!({
        "path": payload.path,
        "name": payload.name,
        "directories": directories,
    }))
}

fn ide_chat_file_reader_directory_list(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatWorkspaceDirectoryListInput>(params)?;
    serde_json::to_value(list_file_reader_directory(input.path)?)
        .map_err(|err| format!("serialize file reader directory failed: {err}"))
}

fn ide_chat_file_reader_read(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatFileReaderReadInput>(params)?;
    let path = input.path.trim();
    if path.is_empty() {
        return Err("path is required".to_string());
    }
    let file_path = PathBuf::from(path);
    if !file_path.exists() {
        return Err(format!("文件不存在：{path}"));
    }
    if !file_path.is_file() {
        return Err(format!("目标不是文件：{path}"));
    }
    let metadata = fs::metadata(&file_path).map_err(|err| format!("读取文件信息失败：{err}"))?;
    let file_size = metadata.len();
    let force_plain = file_size > FILE_READER_PLAIN_TEXT_THRESHOLD;
    let content = match decode_text_file_from_path(&file_path) {
        Ok(decoded) => {
            if force_plain {
                truncate_long_lines(&decoded.text, FILE_READER_LINE_TRUNCATE_CHARS)
            } else {
                decoded.text
            }
        }
        Err(_) => {
            let bytes = fs::read(&file_path).map_err(|err| format!("读取文件失败：{err}"))?;
            format_hex_dump(&bytes)
        }
    };
    let resolved_path = file_path.canonicalize().unwrap_or_else(|_| file_path.clone());
    let extension = file_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let name = file_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(path)
        .to_string();
    let file_key = if extension.is_empty() {
        name.trim().to_ascii_lowercase()
    } else {
        extension.clone()
    };
    serde_json::to_value(FileReaderFilePayload {
        path: resolved_path.to_string_lossy().replace('\\', "/"),
        name,
        extension: file_key.clone(),
        kind: file_reader_file_kind(&file_key).to_string(),
        content,
        force_plain,
    })
    .map_err(|err| format!("serialize file reader payload failed: {err}"))
}

fn ide_chat_delegate_statuses(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ListConversationDelegateStatusesInput>(params)?;
    serde_json::to_value(list_conversation_delegate_statuses_inner(input, state)?)
        .map_err(|err| format!("Serialize delegate statuses failed: {err}"))
}

fn ide_chat_delegate_abort(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<AbortDelegateConversationInput>(params)?;
    serde_json::to_value(abort_delegate_conversation_inner(input, state)?)
        .map_err(|err| format!("Serialize delegate abort result failed: {err}"))
}

fn ide_chat_delegate_block_page(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<GetConversationBlockPageInput>(params)?;
    serde_json::to_value(get_delegate_conversation_block_page_inner(input, state)?)
        .map_err(|err| format!("Serialize delegate block page failed: {err}"))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatWorkspaceLayoutSaveInput {
    conversation_id: String,
    #[serde(default)]
    workspaces: Vec<ShellWorkspaceConfig>,
    #[serde(default)]
    autonomous_mode: Option<bool>,
}

fn ide_chat_workspace_layout_save(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatWorkspaceLayoutSaveInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let normalized_workspaces = normalize_conversation_shell_workspaces(state, &input.workspaces);
    let updated = apply_conversation_chat_workspace_changes(
        state,
        conversation_id,
        Some(None),
        Some(normalized_workspaces),
        input.autonomous_mode,
    )?;
    ide_chat_workspace_permission_payload(state, &updated)
}

fn ide_chat_create_conversation_options(state: &AppState) -> Result<Value, String> {
    let runtime_org = load_runtime_organization_snapshot(state)?;
    let config = runtime_org.config;
    let agents = runtime_org.agents;
    let options = config
        .departments
        .iter()
        .flat_map(|department| {
            let department_id = department.id.trim();
            if department_id.is_empty() {
                return Vec::new();
            }
            let Some(api_config_id) = department_primary_chat_api_config_id(&config, department) else {
                return Vec::new();
            };
            let Some(api_config) = config
                .api_configs
                .iter()
                .find(|api| api.id.trim() == api_config_id && is_text_chat_api(api)) else {
                    return Vec::new();
                };
            let department_name = if department.name.trim().is_empty() {
                department_id
            } else {
                department.name.trim()
            };
            department
                .agent_ids
                .iter()
                .map(|value| value.trim())
                .filter(|agent_id| !agent_id.is_empty())
                .filter_map(|agent_id| {
                    let agent = agents
                        .iter()
                        .find(|agent| agent.id.trim() == agent_id && !agent.is_built_in_user)?;
                    let agent_name = if agent.name.trim().is_empty() {
                        agent_id
                    } else {
                        agent.name.trim()
                    };
                    Some(serde_json::json!({
                        "id": format!("{department_id}::{agent_id}"),
                        "departmentId": department_id,
                        "agentId": agent_id,
                        "departmentName": department_name,
                        "agentName": agent_name,
                        "label": format!("{department_name} / {agent_name}"),
                        "name": department_name,
                        "ownerAgentId": agent_id,
                        "ownerName": agent_name,
                        "providerName": if api_config.name.trim().is_empty() { api_config.id.trim() } else { api_config.name.trim() },
                        "modelName": api_config.model.trim(),
                        "apiConfigId": api_config_id,
                        "childDepartmentIds": &department.child_department_ids,
                    }))
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let default_agent_id = assistant_department_agent_id(&config).unwrap_or_else(default_assistant_department_agent_id);
    Ok(serde_json::json!({
        "departments": options,
        "defaultDepartmentId": ASSISTANT_DEPARTMENT_ID,
        "defaultAgentId": default_agent_id,
    }))
}

fn ide_context_generate_bridge_token() -> String {
    Uuid::new_v4().to_string()
}

fn ide_context_generate_remote_password() -> String {
    generate_web_access_password()
}

fn ide_context_normalize_remote_password(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(|ch| ch.to_uppercase())
        .collect()
}

fn ide_context_remote_password(runtime: &IdeContextRuntime) -> Result<String, String> {
    let auth = runtime
        .bridge_auth
        .lock()
        .map_err(|_| "Failed to lock ide context bridge auth".to_string())?;
    Ok(auth.remote_password.clone())
}

fn ide_context_effective_remote_password(
    state: &AppState,
    runtime: &IdeContextRuntime,
) -> Result<String, String> {
    let config = state_read_config_cached(state)?;
    let password = normalize_web_access_password(&config.web_access_password);
    if !password.trim().is_empty() {
        return Ok(password);
    }
    ide_context_remote_password(runtime)
}

fn ide_context_current_port(runtime: &IdeContextRuntime) -> Option<u16> {
    runtime.current_port.lock().ok().and_then(|guard| *guard)
}

fn ide_context_set_current_port(runtime: &IdeContextRuntime, port: Option<u16>) {
    if let Ok(mut slot) = runtime.current_port.lock() {
        *slot = port;
    }
}

fn ide_context_verify_remote_password(
    runtime: &IdeContextRuntime,
    state: Option<&AppState>,
    provided: &str,
) -> Result<bool, String> {
    let expected = match state {
        Some(state) => ide_context_effective_remote_password(state, runtime)?,
        None => ide_context_remote_password(runtime)?,
    };
    let provided = ide_context_normalize_remote_password(provided);
    if provided.is_empty() {
        return Ok(false);
    }
    Ok(provided == ide_context_normalize_remote_password(&expected))
}

fn ide_context_peer_is_local(peer_addr: &std::net::SocketAddr) -> bool {
    peer_addr.ip().is_loopback()
}

fn ide_context_ws_header_value(request: &Request, name: &str) -> Option<String> {
    request
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn ide_context_ws_request_host_matches(request: &Request, origin_host: &str, port: u16) -> bool {
    let Some(raw_host) = ide_context_ws_header_value(request, "host") else {
        return false;
    };
    let host_url = format!("http://{raw_host}");
    let Ok(parsed) = reqwest::Url::parse(&host_url) else {
        return false;
    };
    if parsed.port_or_known_default() != Some(port) {
        return false;
    }
    parsed
        .host_str()
        .map(|host| host.eq_ignore_ascii_case(origin_host))
        .unwrap_or(false)
}

fn ide_context_ws_origin_allowed(request: &Request, port: u16) -> bool {
    let Some(origin) = ide_context_ws_header_value(request, "origin") else {
        return true;
    };
    if origin.starts_with("vscode-webview://") {
        return true;
    }
    let Ok(parsed) = reqwest::Url::parse(&origin) else {
        return false;
    };
    if parsed.scheme() != "http" || parsed.port_or_known_default() != Some(port) {
        return false;
    }
    parsed
        .host_str()
        .map(|host| ide_context_ws_request_host_matches(request, host, port))
        .unwrap_or(false)
}

fn ide_context_ws_forbidden_response(message: &str) -> tokio_tungstenite::tungstenite::handshake::server::ErrorResponse {
    let mut response =
        tokio_tungstenite::tungstenite::handshake::server::ErrorResponse::new(Some(message.to_string()));
    *response.status_mut() = tokio_tungstenite::tungstenite::http::StatusCode::FORBIDDEN;
    response
}

#[derive(Debug, Clone)]
struct IdeContextLanHostCandidate {
    ip: std::net::Ipv4Addr,
    adapter_name: String,
    adapter_description: String,
    has_gateway: bool,
    active: bool,
}

fn ide_context_ipv4_in_cidr(ip: std::net::Ipv4Addr, network: [u8; 4], prefix_len: u8) -> bool {
    let ip_num = u32::from(ip);
    let network_num = u32::from(std::net::Ipv4Addr::from(network));
    let mask = if prefix_len == 0 {
        0
    } else {
        u32::MAX << (32 - u32::from(prefix_len))
    };
    (ip_num & mask) == (network_num & mask)
}

fn ide_context_ipv4_is_private_lan(ip: std::net::Ipv4Addr) -> bool {
    ip.is_private()
}

fn ide_context_ipv4_is_remote_link_candidate(ip: std::net::Ipv4Addr) -> bool {
    !ip.is_loopback()
        && !ip.is_unspecified()
        && !ip.is_link_local()
        && !ip.is_multicast()
        && !ip.is_broadcast()
        && !ide_context_ipv4_in_cidr(ip, [198, 18, 0, 0], 15)
        && !ide_context_ipv4_in_cidr(ip, [100, 64, 0, 0], 10)
        && !ide_context_ipv4_in_cidr(ip, [192, 0, 2, 0], 24)
        && !ide_context_ipv4_in_cidr(ip, [198, 51, 100, 0], 24)
        && !ide_context_ipv4_in_cidr(ip, [203, 0, 113, 0], 24)
        && ide_context_ipv4_is_private_lan(ip)
}

fn ide_context_adapter_name_is_virtual(name: &str, description: &str) -> bool {
    let text = format!("{name} {description}").to_ascii_lowercase();
    [
        "mihomo",
        "clash",
        "tun",
        "tap",
        "wintun",
        "wireguard",
        "tailscale",
        "zerotier",
        "vethernet",
        "hyper-v",
        "wsl",
        "vmware",
        "virtualbox",
        "docker",
        "loopback",
        "bluetooth",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn ide_context_lan_host_rank(candidate: &IdeContextLanHostCandidate) -> (u8, u8, u8, u32) {
    let virtual_adapter = ide_context_adapter_name_is_virtual(
        &candidate.adapter_name,
        &candidate.adapter_description,
    );
    (
        if virtual_adapter { 1 } else { 0 },
        if candidate.has_gateway { 0 } else { 1 },
        if candidate.active { 0 } else { 1 },
        u32::from(candidate.ip),
    )
}

fn ide_context_collect_default_route_lan_host() -> Vec<IdeContextLanHostCandidate> {
    let mut hosts = Vec::new();
    if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
        let _ = socket.connect("8.8.8.8:80");
        if let Ok(addr) = socket.local_addr() {
            if let std::net::IpAddr::V4(ip) = addr.ip() {
                hosts.push(IdeContextLanHostCandidate {
                    ip,
                    adapter_name: "default-route".to_string(),
                    adapter_description: String::new(),
                    has_gateway: true,
                    active: true,
                });
            }
        }
    }
    hosts
}

fn ide_context_json_strings(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::String(text)) => vec![text.trim().to_string()],
        Some(serde_json::Value::Array(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(|text| text.trim().to_string()))
            .filter(|text| !text.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn ide_context_parse_windows_lan_host_candidates(
    value: serde_json::Value,
) -> Vec<IdeContextLanHostCandidate> {
    let entries = match value {
        serde_json::Value::Array(items) => items,
        serde_json::Value::Object(_) => vec![value],
        _ => Vec::new(),
    };
    let mut candidates = Vec::new();
    for entry in entries {
        let Some(object) = entry.as_object() else {
            continue;
        };
        let adapter_name = object
            .get("InterfaceAlias")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let adapter_description = object
            .get("InterfaceDescription")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let active = object
            .get("Status")
            .and_then(|value| value.as_str())
            .map(|value| value.eq_ignore_ascii_case("up"))
            .unwrap_or(true);
        let has_gateway = !ide_context_json_strings(object.get("IPv4DefaultGateway")).is_empty();
        for ip_text in ide_context_json_strings(object.get("IPv4Address")) {
            if let Ok(ip) = ip_text.parse::<std::net::Ipv4Addr>() {
                candidates.push(IdeContextLanHostCandidate {
                    ip,
                    adapter_name: adapter_name.clone(),
                    adapter_description: adapter_description.clone(),
                    has_gateway,
                    active,
                });
            }
        }
    }
    candidates
}

#[cfg(target_os = "windows")]
fn ide_context_collect_windows_lan_hosts() -> Vec<IdeContextLanHostCandidate> {
    let script = r#"
$ErrorActionPreference = 'SilentlyContinue'
Get-NetIPConfiguration | ForEach-Object {
  [pscustomobject]@{
    InterfaceAlias = $_.InterfaceAlias
    InterfaceDescription = $_.InterfaceDescription
    Status = $_.NetAdapter.Status
    IPv4Address = @($_.IPv4Address | ForEach-Object { $_.IPAddress })
    IPv4DefaultGateway = @($_.IPv4DefaultGateway | ForEach-Object { $_.NextHop })
  }
} | ConvertTo-Json -Depth 5 -Compress
"#;
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", script])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(value) => ide_context_parse_windows_lan_host_candidates(value),
        Err(_) => Vec::new(),
    }
}

#[cfg(not(target_os = "windows"))]
fn ide_context_collect_windows_lan_hosts() -> Vec<IdeContextLanHostCandidate> {
    Vec::new()
}

fn ide_context_lan_hosts() -> Vec<String> {
    let mut candidates = ide_context_collect_windows_lan_hosts();
    if candidates.is_empty() {
        candidates = ide_context_collect_default_route_lan_host();
    }
    candidates.retain(|candidate| ide_context_ipv4_is_remote_link_candidate(candidate.ip));
    candidates.sort_by_key(ide_context_lan_host_rank);
    let mut seen = std::collections::HashSet::<String>::new();
    candidates
        .into_iter()
        .map(|candidate| candidate.ip.to_string())
        .filter(|host| seen.insert(host.clone()))
        .collect::<Vec<_>>()
}

fn ide_context_chat_clients() -> Arc<Mutex<std::collections::HashMap<String, tokio::sync::mpsc::UnboundedSender<serde_json::Value>>>> {
    IDE_CONTEXT_CHAT_CLIENTS
        .get_or_init(|| Arc::new(Mutex::new(std::collections::HashMap::new())))
        .clone()
}

fn ide_chat_broadcast_notification(method: &str, params: serde_json::Value) {
    let clients = ide_context_chat_clients();
    let message = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    });
    let mut stale_ids = Vec::<String>::new();
    if let Ok(clients_guard) = clients.lock() {
        for (client_id, sender) in clients_guard.iter() {
            if sender.send(message.clone()).is_err() {
                stale_ids.push(client_id.clone());
            }
        }
    }
    if !stale_ids.is_empty() {
        if let Ok(mut clients_guard) = clients.lock() {
            for client_id in stale_ids {
                clients_guard.remove(&client_id);
            }
        }
    }
}

fn ide_context_prune_expired_bridge_tokens(auth: &mut IdeContextBridgeAuthRuntime, now: OffsetDateTime) {
    auth.valid_tokens.retain(|_, expires_at| *expires_at > now);
}

fn ide_context_bridge_token_store_path(state: &AppState) -> PathBuf {
    app_root_from_data_path(&state.data_path)
        .join("web-access")
        .join("bridge-auth-token.json")
}

fn ide_context_clear_persisted_bridge_token(state: &AppState) -> Result<(), String> {
    let path = ide_context_bridge_token_store_path(state);
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(&path)
        .map_err(|err| format!("删除 Web 访问令牌失败，path={}，error={err}", path.display()))
}

fn ide_context_persist_bridge_token(
    state: &AppState,
    token: &str,
    expires_at: OffsetDateTime,
) -> Result<(), String> {
    let path = ide_context_bridge_token_store_path(state);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("创建 Web 访问令牌目录失败，path={}，error={err}", parent.display()))?;
    }
    let payload = IdeContextPersistedBridgeToken {
        token: token.trim().to_string(),
        expires_at: expires_at
            .format(&Rfc3339)
            .map_err(|err| format!("格式化 Web 访问令牌过期时间失败: {err}"))?,
    };
    let text = serde_json::to_string_pretty(&payload)
        .map_err(|err| format!("序列化 Web 访问令牌失败: {err}"))?;
    fs::write(&path, text)
        .map_err(|err| format!("写入 Web 访问令牌失败，path={}，error={err}", path.display()))
}

fn ide_context_try_restore_persisted_bridge_token(
    state: &AppState,
    runtime: &IdeContextRuntime,
) -> Result<(), String> {
    let path = ide_context_bridge_token_store_path(state);
    if !path.exists() {
        return Ok(());
    }
    let text = fs::read_to_string(&path)
        .map_err(|err| format!("读取 Web 访问令牌失败，path={}，error={err}", path.display()))?;
    let payload: IdeContextPersistedBridgeToken = serde_json::from_str(&text)
        .map_err(|err| format!("解析 Web 访问令牌失败，path={}，error={err}", path.display()))?;
    let token = payload.token.trim().to_string();
    if token.is_empty() {
        let _ = ide_context_clear_persisted_bridge_token(state);
        return Ok(());
    }
    let Some(expires_at) = parse_iso(&payload.expires_at) else {
        let _ = ide_context_clear_persisted_bridge_token(state);
        return Ok(());
    };
    let now = now_utc();
    if expires_at <= now {
        let _ = ide_context_clear_persisted_bridge_token(state);
        return Ok(());
    }
    let mut auth = runtime
        .bridge_auth
        .lock()
        .map_err(|_| "Failed to lock ide context bridge auth".to_string())?;
    auth.valid_tokens.clear();
    auth.valid_tokens.insert(token, expires_at);
    Ok(())
}

fn ide_context_store_bridge_token(
    runtime: &IdeContextRuntime,
    state: Option<&AppState>,
    token: &str,
    expires_at: OffsetDateTime,
) -> Result<(), String> {
    let normalized_token = token.trim().to_string();
    if normalized_token.is_empty() {
        return Err("Web 访问令牌为空，无法保存".to_string());
    }
    {
        let mut auth = runtime
            .bridge_auth
            .lock()
            .map_err(|_| "Failed to lock ide context bridge auth".to_string())?;
        auth.valid_tokens.clear();
        auth.valid_tokens.insert(normalized_token.clone(), expires_at);
    }
    if let Some(state) = state {
        ide_context_persist_bridge_token(state, &normalized_token, expires_at)?;
    }
    Ok(())
}

fn ide_context_issue_bridge_token_with_state(
    runtime: &IdeContextRuntime,
    state: Option<&AppState>,
) -> Result<String, String> {
    let token = ide_context_generate_bridge_token();
    let now = now_utc();
    let expires_at = now + time::Duration::seconds(IDE_CONTEXT_AUTH_TOKEN_TTL_SECS);
    ide_context_store_bridge_token(runtime, state, &token, expires_at)?;
    Ok(token)
}

fn ide_context_consume_bridge_token_with_state(
    runtime: &IdeContextRuntime,
    state: Option<&AppState>,
    provided: &str,
) -> Result<String, (String, Option<String>)> {
    let provided = provided.trim();
    if provided.is_empty() {
        return Err(("authToken is required".to_string(), None));
    }
    if let Some(state) = state {
        let should_restore = runtime
            .bridge_auth
            .lock()
            .map(|auth| auth.valid_tokens.is_empty())
            .unwrap_or(false);
        if should_restore {
            if let Err(err) = ide_context_try_restore_persisted_bridge_token(state, runtime) {
                eprintln!("[IDE 上下文桥] 恢复持久化 Web 访问令牌失败: {}", err);
            }
        }
    }
    let mut auth = runtime
        .bridge_auth
        .lock()
        .map_err(|_| ("Failed to lock ide context bridge auth".to_string(), None))?;
    let now = now_utc();
    ide_context_prune_expired_bridge_tokens(&mut auth, now);
    if auth.valid_tokens.is_empty() {
        drop(auth);
        if let Some(state) = state {
            let _ = ide_context_clear_persisted_bridge_token(state);
        }
        let refreshed_token = ide_context_issue_bridge_token_with_state(runtime, state)
            .map_err(|err| (err, None))?;
        return Err((
            "IDE context bridge token expired, discovery refreshed".to_string(),
            Some(refreshed_token),
        ));
    }
    if !auth.valid_tokens.contains_key(provided) {
        return Err(("invalid authToken".to_string(), None));
    }
    let expires_at = now + time::Duration::seconds(IDE_CONTEXT_AUTH_TOKEN_TTL_SECS);
    drop(auth);
    ide_context_store_bridge_token(runtime, state, provided, expires_at)
        .map_err(|err| (err, None))?;
    Ok(provided.to_string())
}

fn ide_context_normalize_time_or_now(field_name: &str, raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return now_iso();
    }
    match normalize_rfc3339_to_utc_storage(field_name, trimmed) {
        Ok(value) => value,
        Err(err) => {
            eprintln!(
                "[IDE 上下文桥] 时间字段非法，回退当前时间: field={}, value={}, error={}",
                field_name, trimmed, err
            );
            now_iso()
        }
    }
}

fn ide_context_timestamp_compare_desc(left: &str, right: &str) -> std::cmp::Ordering {
    match (parse_iso(left), parse_iso(right)) {
        (Some(left_time), Some(right_time)) => right_time.cmp(&left_time),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => right.cmp(left),
    }
}

fn ide_context_timestamp_is_newer(candidate: &str, current: &str) -> bool {
    if current.trim().is_empty() {
        return !candidate.trim().is_empty();
    }
    ide_context_timestamp_compare_desc(candidate, current) == std::cmp::Ordering::Less
}

fn ide_context_reference_dedup_key(item: &IdeContextReferenceItemOutput) -> String {
    let file_key = ide_context_compare_key(&item.file_path);
    let source_key = item.source.trim();
    if file_key.is_empty() && source_key.is_empty() {
        item.id.clone()
    } else if file_key.is_empty() {
        format!("{}|{}", item.id, source_key)
    } else if source_key.is_empty() {
        file_key
    } else {
        format!("{}|{}", file_key, source_key)
    }
}

fn ide_context_reference_source_priority(source: &str) -> u8 {
    match source.trim() {
        "selection" => 3,
        "visible_range" => 2,
        "active_file" => 1,
        _ => 0,
    }
}

fn ide_context_should_replace_reference(
    candidate: &IdeContextReferenceItemOutput,
    existing: &IdeContextReferenceItemOutput,
) -> bool {
    if ide_context_timestamp_is_newer(&candidate.captured_at, &existing.captured_at) {
        return true;
    }
    if ide_context_timestamp_is_newer(&existing.captured_at, &candidate.captured_at) {
        return false;
    }

    let candidate_priority = ide_context_reference_source_priority(&candidate.source);
    let existing_priority = ide_context_reference_source_priority(&existing.source);
    if candidate_priority != existing_priority {
        return candidate_priority > existing_priority;
    }

    let candidate_content_len = candidate.content.trim().chars().count();
    let existing_content_len = existing.content.trim().chars().count();
    if candidate_content_len != existing_content_len {
        return candidate_content_len > existing_content_len;
    }

    candidate.display_label < existing.display_label
}

fn ide_context_snapshot_is_expired(snapshot: &IdeContextSnapshot, now: &OffsetDateTime) -> bool {
    match parse_iso(&snapshot.updated_at) {
        Some(updated_at) => updated_at < (*now - time::Duration::seconds(IDE_CONTEXT_SNAPSHOT_TTL_SECS)),
        None => true,
    }
}

fn ide_context_prune_expired_snapshots(
    snapshots: &mut std::collections::HashMap<String, IdeContextSnapshot>,
) {
    let now = now_utc();
    snapshots.retain(|client_id, snapshot| {
        if ide_context_snapshot_is_expired(snapshot, &now) {
            eprintln!(
                "[IDE 上下文桥] 快照过期已清理: client_id={}, updated_at={}",
                client_id, snapshot.updated_at
            );
            false
        } else {
            true
        }
    });
}

fn emit_ide_context_updated(state: &AppState, client_id: &str, updated_at: &str) {
    let app_handle = match state.app_handle.lock() {
        Ok(slot) => slot.clone(),
        Err(_) => None,
    };
    if let Some(app_handle) = app_handle {
        let _ = app_handle.emit(
            "ide-context-updated",
            IdeContextUpdatedEvent {
                client_id: client_id.to_string(),
                updated_at: updated_at.to_string(),
            },
        );
    }
    ide_chat_broadcast_notification(
        "ideContext.updated",
        serde_json::json!({
            "clientId": client_id,
            "updatedAt": updated_at,
        }),
    );
}

fn ide_context_compare_key(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let normalized = normalize_terminal_path_input_for_current_platform(trimmed);
    let path = std::path::PathBuf::from(if normalized.is_empty() { trimmed } else { &normalized });
    shell_workspace_display_path(&path)
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn ide_context_display_path(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let normalized = normalize_terminal_path_input_for_current_platform(trimmed);
    let path = std::path::PathBuf::from(if normalized.is_empty() { trimmed } else { &normalized });
    let resolved = path.canonicalize().unwrap_or(path);
    shell_workspace_display_path(&resolved).replace('\\', "/")
}

fn ide_context_workspace_name(input: &IdeContextWorkspaceInput) -> String {
    let explicit = input.name.as_deref().map(str::trim).unwrap_or("");
    if !explicit.is_empty() {
        return explicit.to_string();
    }
    let display_path = ide_context_display_path(&input.path);
    std::path::Path::new(&display_path)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or(display_path)
}

fn ide_context_path_is_within_workspace(file_path: &str, workspace_path: &str) -> bool {
    let file_key = ide_context_compare_key(file_path);
    let workspace_key = ide_context_compare_key(workspace_path);
    if file_key.is_empty() || workspace_key.is_empty() {
        return false;
    }
    file_key == workspace_key || file_key.starts_with(&(workspace_key + "/"))
}

fn ide_context_relative_display_path(file_path: &str, workspace_path: &str) -> String {
    let file_display = ide_context_display_path(file_path);
    let workspace_display = ide_context_display_path(workspace_path);
    let file_key = ide_context_compare_key(&file_display);
    let workspace_key = ide_context_compare_key(&workspace_display);
    if file_key == workspace_key {
        return std::path::Path::new(&file_display)
            .file_name()
            .and_then(|value| value.to_str())
            .map(ToOwned::to_owned)
            .unwrap_or(file_display);
    }
    let prefix = format!("{}/", workspace_key);
    if let Some(relative_key) = file_key.strip_prefix(&prefix) {
        let relative = relative_key.replace('/', std::path::MAIN_SEPARATOR_STR);
        return relative.replace('\\', "/");
    }
    file_display
}

fn ide_context_line_suffix(start_line: Option<u32>, end_line: Option<u32>) -> String {
    match (start_line, end_line) {
        (Some(start), Some(end)) if end > start => format!(":{start}-{end}"),
        (Some(start), _) => format!(":{start}"),
        _ => String::new(),
    }
}

fn ide_context_text_block(file_path: &str, reference: &IdeContextReference) -> String {
    if reference.source.trim() == "active_file" {
        return ["[IDE 上下文引用]".to_string(), format!("文件: {file_path}")].join("\n");
    }
    let mut lines = vec!["[IDE 上下文引用]".to_string(), format!("文件: {file_path}")];
    if reference.start_line.is_some() || reference.end_line.is_some() {
        let line_text = match (reference.start_line, reference.end_line) {
            (Some(start), Some(end)) if end > start => format!("{start}-{end}"),
            (Some(start), _) => start.to_string(),
            (_, Some(end)) => end.to_string(),
            _ => String::new(),
        };
        if !line_text.is_empty() {
            lines.push(format!("行号: {line_text}"));
        }
    }
    if let Some(language_id) = reference
        .language_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("语言: {language_id}"));
    }
    let source = reference.source.trim();
    if !source.is_empty() {
        lines.push(format!("来源: {source}"));
    }
    let captured_at = reference.captured_at.trim();
    if !captured_at.is_empty() {
        lines.push(format!("采集时间: {captured_at}"));
    }
    lines.push("内容:".to_string());
    lines.push(reference.content.clone());
    lines.join("\n")
}

fn ide_context_bridge_url_for_host(host: &str, port: u16) -> String {
    format!("ws://{}:{}{}", host, port, IDE_CONTEXT_BRIDGE_PATH)
}

fn ide_context_chat_bridge_url_for_host(host: &str, port: u16) -> String {
    format!("ws://{}:{}{}", host, port, IDE_CONTEXT_CHAT_BRIDGE_PATH)
}

fn ide_context_bridge_url(port: u16) -> String {
    ide_context_bridge_url_for_host(IDE_CONTEXT_BRIDGE_HOST, port)
}

fn ide_context_chat_bridge_url(port: u16) -> String {
    ide_context_chat_bridge_url_for_host(IDE_CONTEXT_BRIDGE_HOST, port)
}

fn ide_context_sidebar_url_for_host(host: &str, port: u16) -> String {
    format!("http://{}:{}/sidebar", host, port)
}

fn ide_context_bridge_discovery_path() -> std::path::PathBuf {
    std::env::temp_dir().join(IDE_CONTEXT_BRIDGE_DISCOVERY_FILE)
}

fn ide_context_bridge_shutdown_slot() -> Arc<Mutex<Option<tokio_util::sync::CancellationToken>>> {
    IDE_CONTEXT_BRIDGE_SHUTDOWN
        .get_or_init(|| Arc::new(Mutex::new(None)))
        .clone()
}

fn ide_context_bridge_create_shutdown_token() -> tokio_util::sync::CancellationToken {
    let token = tokio_util::sync::CancellationToken::new();
    if let Ok(mut slot) = ide_context_bridge_shutdown_slot().lock() {
        *slot = Some(token.clone());
    }
    token
}

fn ide_context_bridge_server_task_slot() -> Arc<Mutex<Option<tauri::async_runtime::JoinHandle<()>>>> {
    IDE_CONTEXT_BRIDGE_SERVER_TASK
        .get_or_init(|| Arc::new(Mutex::new(None)))
        .clone()
}

fn ide_context_bridge_set_server_task(handle: tauri::async_runtime::JoinHandle<()>) {
    if let Ok(mut slot) = ide_context_bridge_server_task_slot().lock() {
        *slot = Some(handle);
    }
}

fn ide_context_bridge_take_server_task() -> Option<tauri::async_runtime::JoinHandle<()>> {
    ide_context_bridge_server_task_slot()
        .lock()
        .ok()
        .and_then(|mut slot| slot.take())
}

fn publish_ide_context_bridge_discovery(port: u16, remote_password: &str) -> Result<(), String> {
    let url = ide_context_bridge_url(port);
    let chat_url = ide_context_chat_bridge_url(port);
    let payload = IdeContextBridgeDiscovery {
        url: url.clone(),
        bridge_url: url,
        chat_url,
        host: IDE_CONTEXT_BRIDGE_HOST.to_string(),
        bind_host: IDE_CONTEXT_BRIDGE_BIND_HOST.to_string(),
        port,
        path: IDE_CONTEXT_BRIDGE_PATH.to_string(),
        chat_path: IDE_CONTEXT_CHAT_BRIDGE_PATH.to_string(),
        pid: std::process::id(),
        updated_at: now_iso(),
        token: String::new(),
        remote_password: remote_password.to_string(),
    };
    let path = ide_context_bridge_discovery_path();
    let text = serde_json::to_string_pretty(&payload)
        .map_err(|err| format!("Serialize IDE context bridge discovery failed: {err}"))?;
    fs::write(&path, text).map_err(|err| {
        format!(
            "Write IDE context bridge discovery failed ({}): {err}",
            path.display()
        )
    })?;
    Ok(())
}

fn clear_ide_context_bridge_discovery() {
    let path = ide_context_bridge_discovery_path();
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

async fn bind_ide_context_bridge_listener(
    preferred_port: u16,
) -> Result<(tokio::net::TcpListener, u16), String> {
    let mut errors = Vec::new();
    let mut candidate_ports = Vec::new();
    if preferred_port >= 1024 {
        candidate_ports.push(preferred_port);
    }
    for port in IDE_CONTEXT_BRIDGE_BASE_PORT..=IDE_CONTEXT_BRIDGE_MAX_PORT {
        if !candidate_ports.contains(&port) {
            candidate_ports.push(port);
        }
    }
    for port in candidate_ports {
        let addr = format!("{}:{}", IDE_CONTEXT_BRIDGE_BIND_HOST, port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => return Ok((listener, port)),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::AddrInUse {
                    eprintln!("[IDE 上下文桥] 端口占用，尝试顺延: {}", addr);
                } else {
                    eprintln!("[IDE 上下文桥] 监听失败，尝试下一个端口 {}: {}", addr, err);
                }
                errors.push(format!("{addr}: {err}"));
            }
        }
    }
    Err(format!(
        "No available IDE context bridge port in {}:{}-{} ({})",
        IDE_CONTEXT_BRIDGE_BIND_HOST,
        IDE_CONTEXT_BRIDGE_BASE_PORT,
        IDE_CONTEXT_BRIDGE_MAX_PORT,
        errors.join("; ")
    ))
}

async fn ide_context_stream_is_websocket(stream: &tokio::net::TcpStream) -> bool {
    let mut buffer = [0_u8; 1024];
    match tokio::time::timeout(std::time::Duration::from_millis(500), stream.peek(&mut buffer)).await
    {
        Ok(Ok(count)) if count > 0 => {
            String::from_utf8_lossy(&buffer[..count])
                .to_ascii_lowercase()
                .contains("upgrade: websocket")
        }
        _ => false,
    }
}

fn ide_context_http_status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        404 => "Not Found",
        405 => "Method Not Allowed",
        426 => "Upgrade Required",
        _ => "Internal Server Error",
    }
}

fn ide_context_http_header_value<'a>(headers: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{}:", name.to_ascii_lowercase());
    headers.lines().skip(1).find_map(|line| {
        let trimmed = line.trim();
        if trimmed.to_ascii_lowercase().starts_with(&prefix) {
            trimmed.split_once(':').map(|(_, value)| value.trim())
        } else {
            None
        }
    })
}

fn ide_context_http_path_from_request(headers: &str) -> (&str, &str) {
    let first_line = headers.lines().next().unwrap_or_default();
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let uri = parts.next().unwrap_or("/");
    let path = uri.split('?').next().unwrap_or("/");
    (method, path)
}

fn ide_context_web_asset_path(path: &str) -> Option<String> {
    let path = path.trim();
    match path {
        "/" | "/sidebar" | "/sidebar.html" => Some("sidebar.html".to_string()),
        "/settings" | "/settings.html" => Some("settings.html".to_string()),
        _ if path.starts_with("/assets/") && !path.contains("..") => {
            Some(path.trim_start_matches('/').to_string())
        }
        _ => None,
    }
}

fn ide_context_web_icon_bytes(path: &str) -> Option<&'static [u8]> {
    match path.trim() {
        "/favicon.ico" | "/favicon.png" => {
            Some(include_bytes!("../../../../icons/32x32.png").as_slice())
        }
        _ => None,
    }
}

fn ide_context_web_html_with_bridge(asset_bytes: &[u8], host: &str) -> Vec<u8> {
    let chat_url = format!("ws://{}{}", host, IDE_CONTEXT_CHAT_BRIDGE_PATH);
    let injected = serde_json::json!({
        "chatUrl": chat_url,
        "workspaceRoots": [],
    });
    let script = format!(
        "<script>window.__PAI_SIDEBAR_BRIDGE__ = {}; window.__PAI_SETTINGS_BRIDGE__ = window.__PAI_SIDEBAR_BRIDGE__;</script>",
        injected
    );
    let icon_links = r#"<link rel="icon" type="image/png" href="/favicon.png">
  <link rel="shortcut icon" type="image/png" href="/favicon.png">"#;
    let injection = format!("{}\n  {}", icon_links, script);
    let html = String::from_utf8_lossy(asset_bytes);
    if html.contains("</head>") {
        html.replacen("</head>", &format!("  {}\n  </head>", injection), 1)
            .into_bytes()
    } else {
        format!("{}\n{}", injection, html).into_bytes()
    }
}

async fn ide_context_http_write_response(
    stream: &mut tokio::net::TcpStream,
    status: u16,
    content_type: &str,
    body: Vec<u8>,
) {
    use tokio::io::AsyncWriteExt;

    let headers = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n",
        status,
        ide_context_http_status_text(status),
        content_type,
        body.len()
    );
    let _ = stream.write_all(headers.as_bytes()).await;
    let _ = stream.write_all(&body).await;
    let _ = stream.shutdown().await;
}

async fn ide_context_http_handle_connection(
    mut stream: tokio::net::TcpStream,
    app: AppHandle,
) {
    use tokio::io::AsyncReadExt;

    let mut buffer = vec![0_u8; 8192];
    let count = match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        stream.read(&mut buffer),
    )
    .await
    {
        Ok(Ok(count)) => count,
        _ => 0,
    };
    let headers = String::from_utf8_lossy(&buffer[..count]);
    let (method, path) = ide_context_http_path_from_request(&headers);
    if method != "GET" {
        ide_context_http_write_response(
            &mut stream,
            405,
            "text/plain; charset=utf-8",
            b"Method Not Allowed".to_vec(),
        )
        .await;
        return;
    }
    if path == IDE_CONTEXT_BRIDGE_PATH || path == IDE_CONTEXT_CHAT_BRIDGE_PATH {
        ide_context_http_write_response(
            &mut stream,
            426,
            "text/plain; charset=utf-8",
            b"WebSocket upgrade required".to_vec(),
        )
        .await;
        return;
    }
    if let Some(icon) = ide_context_web_icon_bytes(path) {
        ide_context_http_write_response(
            &mut stream,
            200,
            "image/png",
            icon.to_vec(),
        )
        .await;
        return;
    }
    let Some(asset_path) = ide_context_web_asset_path(path) else {
        ide_context_http_write_response(
            &mut stream,
            404,
            "text/plain; charset=utf-8",
            b"Not Found".to_vec(),
        )
        .await;
        return;
    };
    let Some(asset) = app.asset_resolver().get(asset_path.clone()) else {
        ide_context_http_write_response(
            &mut stream,
            404,
            "text/plain; charset=utf-8",
            b"Asset Not Found".to_vec(),
        )
        .await;
        return;
    };
    let host = ide_context_http_header_value(&headers, "host")
        .filter(|value| !value.is_empty())
        .unwrap_or("127.0.0.1");
    let body = if asset_path == "sidebar.html" || asset_path == "settings.html" {
        ide_context_web_html_with_bridge(asset.bytes(), host)
    } else {
        asset.bytes().to_vec()
    };
    ide_context_http_write_response(
        &mut stream,
        200,
        asset.mime_type(),
        body,
    )
    .await;
}

fn upsert_ide_context_snapshot_internal(
    input: UpsertIdeContextSnapshotInput,
    runtime: &IdeContextRuntime,
) -> Result<(String, String), String> {
    let client_id = input.client_id.trim().to_string();
    if client_id.is_empty() {
        return Err("clientId is required".to_string());
    }
    let updated_at = input
        .updated_at
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| ide_context_normalize_time_or_now("updatedAt", value))
        .unwrap_or_else(now_iso);
    let snapshot = IdeContextSnapshot {
        client_id: client_id.clone(),
        editor: {
            let editor = input.editor.trim();
            if editor.is_empty() {
                "vscode".to_string()
            } else {
                editor.to_string()
            }
        },
        workspace_roots: input
            .workspace_roots
            .into_iter()
            .map(|path| ide_context_display_path(&path))
            .filter(|path| !path.trim().is_empty())
            .collect(),
        references: input
            .references
            .into_iter()
            .filter_map(|reference| {
                let id = reference.id.trim().to_string();
                let file_path = ide_context_display_path(&reference.file_path);
                let content = reference.content.trim().to_string();
                let source = reference.source.trim().to_string();
                let allow_empty_content = source == "active_file";
                if id.is_empty() || file_path.is_empty() || (!allow_empty_content && content.is_empty()) {
                    return None;
                }
                Some(IdeContextReference {
                    id,
                    file_path,
                    start_line: reference.start_line,
                    end_line: reference.end_line,
                    content,
                    language_id: reference
                        .language_id
                        .map(|value| value.trim().to_string())
                        .filter(|value| !value.is_empty()),
                    source,
                    captured_at: ide_context_normalize_time_or_now(
                        "references[].capturedAt",
                        &reference.captured_at,
                    ),
                })
            })
            .collect(),
        updated_at: updated_at.clone(),
    };
    let mut snapshots = runtime
        .snapshots
        .lock()
        .map_err(|_| "Failed to lock ide context snapshots".to_string())?;
    snapshots.insert(client_id.clone(), snapshot);
    Ok((client_id, updated_at))
}

#[tauri::command]
fn upsert_ide_context_snapshot(
    input: UpsertIdeContextSnapshotInput,
    state: State<'_, AppState>,
    ide_context_runtime: State<'_, IdeContextRuntime>,
) -> Result<(), String> {
    let (client_id, updated_at) =
        upsert_ide_context_snapshot_internal(input, ide_context_runtime.inner())?;
    emit_ide_context_updated(&state, &client_id, &updated_at);
    Ok(())
}

#[tauri::command]
fn query_ide_context_references(
    input: IdeContextWorkspaceQueryInput,
    ide_context_runtime: State<'_, IdeContextRuntime>,
) -> Result<IdeContextQueryResultOutput, String> {
    query_ide_context_references_internal(input, ide_context_runtime.inner())
}

#[tauri::command]
fn get_web_access_info(
    app: AppHandle,
    state: State<'_, AppState>,
    ide_context_runtime: State<'_, IdeContextRuntime>,
) -> Result<WebAccessInfoOutput, String> {
    get_web_access_info_inner(&app, &state, &ide_context_runtime)
}

fn get_web_access_info_inner(
    app: &AppHandle,
    state: &AppState,
    ide_context_runtime: &IdeContextRuntime,
) -> Result<WebAccessInfoOutput, String> {
    let config = state_read_config_cached(&state)?;
    let configured_port = normalize_web_access_port(config.web_access_port);
    if !config.web_access_enabled {
        return Ok(WebAccessInfoOutput {
            running: false,
            enabled: false,
            configured_port,
            port: configured_port,
            local_url: String::new(),
            remote_urls: Vec::new(),
            remote_password: String::new(),
        });
    }
    if !IDE_CONTEXT_BRIDGE_STARTED.load(Ordering::SeqCst) {
        start_ide_context_bridge_server(
            app.clone(),
            state.clone(),
            ide_context_runtime.clone(),
        );
    }
    let running = IDE_CONTEXT_BRIDGE_STARTED.load(Ordering::SeqCst);
    let port = ide_context_current_port(ide_context_runtime).unwrap_or(configured_port);
    let local_url = ide_context_sidebar_url_for_host(IDE_CONTEXT_BRIDGE_HOST, port);
    let remote_urls = ide_context_lan_hosts()
        .into_iter()
        .map(|host| ide_context_sidebar_url_for_host(&host, port))
        .collect::<Vec<_>>();
    Ok(WebAccessInfoOutput {
        running,
        enabled: true,
        configured_port,
        port,
        local_url,
        remote_urls,
        remote_password: ide_context_effective_remote_password(state, ide_context_runtime)?,
    })
}

fn query_ide_context_references_internal(
    input: IdeContextWorkspaceQueryInput,
    ide_context_runtime: &IdeContextRuntime,
) -> Result<IdeContextQueryResultOutput, String> {
    let workspaces: Vec<IdeContextWorkspaceInput> = input
        .workspaces
        .into_iter()
        .filter(|workspace| !workspace.path.trim().is_empty())
        .collect();
    if workspaces.is_empty() {
        return Ok(IdeContextQueryResultOutput {
            groups: Vec::new(),
            updated_at: String::new(),
        });
    }

    let mut snapshots = ide_context_runtime
        .snapshots
        .lock()
        .map_err(|_| "Failed to lock ide context snapshots".to_string())?;
    ide_context_prune_expired_snapshots(&mut snapshots);

    let mut groups = workspaces
        .iter()
        .map(|workspace| IdeContextWorkspaceGroupOutput {
            workspace_path: ide_context_display_path(&workspace.path),
            workspace_name: ide_context_workspace_name(workspace),
            references: Vec::new(),
        })
        .collect::<Vec<_>>();
    let mut latest_updated_at = String::new();

    for snapshot in snapshots.values() {
        if ide_context_timestamp_is_newer(&snapshot.updated_at, &latest_updated_at) {
            latest_updated_at = snapshot.updated_at.clone();
        }
        for reference in &snapshot.references {
            for group in &mut groups {
                if !ide_context_path_is_within_workspace(&reference.file_path, &group.workspace_path) {
                    continue;
                }
                let file_path = ide_context_display_path(&reference.file_path);
                let file_name = std::path::Path::new(&file_path)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| file_path.clone());
                let relative_path = ide_context_relative_display_path(&file_path, &group.workspace_path);
                let display_label = format!(
                    "{}{}",
                    file_name,
                    ide_context_line_suffix(reference.start_line, reference.end_line)
                );
                let text_block = ide_context_text_block(&file_path, reference);
                group.references.push(IdeContextReferenceItemOutput {
                    id: format!("{}:{}:{}", snapshot.client_id, reference.id, reference.captured_at),
                    workspace_path: group.workspace_path.clone(),
                    workspace_name: group.workspace_name.clone(),
                    file_path,
                    file_name,
                    relative_path,
                    start_line: reference.start_line,
                    end_line: reference.end_line,
                    display_label,
                    content: reference.content.clone(),
                    language_id: reference.language_id.clone(),
                    source: reference.source.clone(),
                    captured_at: reference.captured_at.clone(),
                    text_block,
                });
                break;
            }
        }
    }

    for group in &mut groups {
        let mut latest_by_file = std::collections::HashMap::<String, IdeContextReferenceItemOutput>::new();
        for item in group.references.drain(..) {
            let key = ide_context_reference_dedup_key(&item);
            let should_replace = latest_by_file
                .get(&key)
                .map(|existing| ide_context_should_replace_reference(&item, existing))
                .unwrap_or(true);
            if should_replace {
                latest_by_file.insert(key, item);
            }
        }
        group.references = latest_by_file.into_values().collect();
        group.references.sort_by(|left, right| {
            ide_context_timestamp_compare_desc(&left.captured_at, &right.captured_at)
                .then_with(|| left.display_label.cmp(&right.display_label))
        });
    }
    groups.retain(|group| !group.references.is_empty());

    Ok(IdeContextQueryResultOutput {
        groups,
        updated_at: latest_updated_at,
    })
}

fn ide_chat_jsonrpc_success(id: Option<Value>, result: Value) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

fn ide_chat_jsonrpc_error(id: Option<Value>, code: i32, message: impl Into<String>) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": IdeChatJsonRpcError {
            code,
            message: message.into(),
        },
    })
}

fn ide_chat_parse_params<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, String> {
    serde_json::from_value::<T>(params).map_err(|err| format!("invalid params: {err}"))
}

fn ide_chat_parse_param_field<T: serde::de::DeserializeOwned>(
    params: Value,
    field: &str,
) -> Result<T, String> {
    match params {
        Value::Object(mut map) => map
            .remove(field)
            .ok_or_else(|| format!("{field} is required"))
            .and_then(ide_chat_parse_params::<T>),
        _ => Err(format!("{field} is required")),
    }
}

fn ide_chat_parse_optional_param_field<T: serde::de::DeserializeOwned>(
    params: Value,
    field: &str,
) -> Result<Option<T>, String> {
    match params {
        Value::Object(mut map) => map
            .remove(field)
            .map(ide_chat_parse_params::<T>)
            .transpose(),
        _ => Ok(None),
    }
}

fn ide_chat_serialize<T: Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|err| format!("serialize result failed: {err}"))
}

fn ide_chat_load_config_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(load_config_inner(state)?)
}

fn ide_chat_load_app_bootstrap_snapshot_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(read_app_bootstrap_snapshot(state)?)
}

fn ide_chat_save_config_for_web_settings(
    state: &AppState,
    app: &AppHandle,
    ide_context_runtime: &IdeContextRuntime,
    params: Value,
) -> Result<Value, String> {
    let config = ide_chat_parse_param_field::<AppConfig>(params, "config")?;
    ide_chat_serialize(save_config_inner(config, app, state, ide_context_runtime)?)
}

fn ide_chat_load_agents_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(load_agents_inner(state)?)
}

fn ide_chat_save_agents_for_web_settings(
    state: &AppState,
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SaveAgentsInput>(params, "input")?;
    ide_chat_serialize(save_agents_inner(input, app, state)?)
}

fn ide_chat_load_chat_settings_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(load_chat_settings_inner(state)?)
}

fn ide_chat_save_chat_settings_for_web_settings(
    state: &AppState,
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ChatSettings>(params, "input")?;
    let patch = ChatSettingsPatch {
        assistant_department_agent_id: Some(input.assistant_department_agent_id),
        user_alias: Some(input.user_alias),
        response_style_id: Some(input.response_style_id),
        pdf_read_mode: Some(input.pdf_read_mode),
        background_voice_screenshot_keywords: Some(input.background_voice_screenshot_keywords),
        background_voice_screenshot_mode: Some(input.background_voice_screenshot_mode),
        instruction_presets: Some(input.instruction_presets),
    };
    ide_chat_serialize(patch_chat_settings_inner(patch, app, state)?)
}

fn ide_chat_patch_chat_settings_for_web_settings(
    state: &AppState,
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ChatSettingsPatch>(params, "input")?;
    ide_chat_serialize(patch_chat_settings_inner(input, app, state)?)
}

fn ide_chat_patch_conversation_api_settings_for_web_settings(
    state: &AppState,
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ConversationApiSettingsPatch>(params, "input")?;
    ide_chat_serialize(patch_conversation_api_settings_inner(input, app, state)?)
}

fn ide_chat_save_conversation_api_settings_for_web_settings(
    state: &AppState,
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ConversationApiSettings>(params, "input")?;
    let patch = ConversationApiSettingsPatch {
        assistant_department_api_config_id: Some(input.assistant_department_api_config_id),
        vision_api_config_id: Some(input.vision_api_config_id),
        tool_review_api_config_id: Some(input.tool_review_api_config_id),
        stt_api_config_id: Some(input.stt_api_config_id),
        stt_auto_send: Some(input.stt_auto_send),
    };
    ide_chat_serialize(patch_conversation_api_settings_inner(patch, app, state)?)
}

fn ide_chat_avatar_data_url_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<AvatarDataPathInput>(params, "input")?;
    ide_chat_serialize(read_avatar_data_url_inner(input, state)?)
}

fn ide_chat_save_agent_avatar_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SaveAgentAvatarInput>(params, "input")?;
    ide_chat_serialize(save_agent_avatar_inner(input, state)?)
}

fn ide_chat_clear_agent_avatar_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ClearAgentAvatarInput>(params, "input")?;
    clear_agent_avatar_inner(input, state)?;
    Ok(serde_json::json!(null))
}

fn ide_chat_sync_tray_icon_for_web_settings(app: &AppHandle) -> Result<Value, String> {
    sync_default_tray_icon(app)?;
    Ok(serde_json::json!(null))
}

async fn ide_chat_refresh_models_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RefreshModelsInput>(params, "input")?;
    ide_chat_serialize(refresh_models_inner(state, input).await?)
}

async fn ide_chat_quick_genai_chat_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<QuickGenaiChatInput>(params, "input")?;
    ide_chat_serialize(quick_genai_chat_inner(state, input).await?)
}

async fn ide_chat_fetch_model_metadata_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<FetchModelMetadataInput>(params, "input")?;
    ide_chat_serialize(fetch_model_metadata_inner(state, input).await?)
}

async fn ide_chat_test_embedding_connection_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TestEmbeddingConnectionInput>(params, "input")?;
    ide_chat_serialize(test_embedding_connection_inner(input).await?)
}

async fn ide_chat_test_rerank_connection_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TestRerankConnectionInput>(params, "input")?;
    ide_chat_serialize(test_rerank_connection_inner(input).await?)
}

async fn ide_chat_test_voice_connection_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TestVoiceConnectionInput>(params, "input")?;
    ide_chat_serialize(test_voice_connection_inner(input).await?)
}

fn ide_chat_resolve_model_adapter_kind_for_web_settings(params: Value) -> Result<Value, String> {
    let model_name = match params {
        Value::Object(mut map) => map
            .remove("modelName")
            .or_else(|| map.remove("model_name"))
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_default(),
        _ => String::new(),
    };
    ide_chat_serialize(resolve_model_adapter_kind_label(&model_name))
}

fn ide_chat_check_tools_status_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<CheckToolsStatusInput>(params, "input")?;
    ide_chat_serialize(check_tools_status_inner(input, state)?)
}

fn ide_chat_get_image_text_cache_stats_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(get_image_text_cache_stats_inner(state)?)
}

fn ide_chat_clear_image_text_cache_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(clear_image_text_cache_inner(state)?)
}

async fn ide_chat_list_tool_catalog_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(list_tool_catalog_inner(state).await?)
}

async fn ide_chat_list_department_permission_catalog_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    ide_chat_serialize(list_department_permission_catalog_inner(state).await?)
}

fn ide_chat_open_external_url_for_web_settings(params: Value) -> Result<Value, String> {
    let url = match params {
        Value::Object(mut map) => map
            .remove("url")
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_default(),
        _ => String::new(),
    };
    open_external_url(url)?;
    Ok(serde_json::json!(null))
}

fn ide_chat_web_access_info_for_web_settings(
    app: &AppHandle,
    state: &AppState,
    ide_context_runtime: &IdeContextRuntime,
) -> Result<Value, String> {
    ide_chat_serialize(get_web_access_info_inner(app, state, ide_context_runtime)?)
}

fn ide_chat_list_memories_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(memory_store_list_memories(&state.data_path)?)
}

fn ide_chat_delete_memory_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<DeleteMemoryInput>(params, "input")?;
    memory_store_delete_memory(&state.data_path, &input.memory_id)?;
    ide_chat_serialize(DeleteMemoryResult {
        status: "deleted".to_string(),
    })
}

fn ide_chat_preview_export_memories_for_web_settings(state: &AppState) -> Result<Value, String> {
    let owner_scope_by_agent = load_importable_agent_scope_labels(state)?;
    let scopes = build_export_scope_items(&state.data_path, &owner_scope_by_agent)?;
    ide_chat_serialize(PreviewExportMemoriesResult {
        total_count: scopes.iter().map(|item| item.count).sum(),
        scopes,
    })
}

fn ide_chat_export_memories_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let selected_scopes = match params {
        Value::Object(mut map) => map
            .remove("input")
            .and_then(|value| {
                value
                    .get("scopes")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .map(|item| item.as_str().unwrap_or_default().to_string())
                            .collect::<Vec<_>>()
                    })
            })
            .map(|scopes| normalize_selected_export_scopes(&scopes))
            .transpose()?,
        _ => None,
    };
    let owner_scope_by_agent = load_importable_agent_scope_labels(state)?;
    ide_chat_serialize(build_memory_exchange_payload(
        &state.data_path,
        &owner_scope_by_agent,
        selected_scopes.as_ref(),
    )?)
}

fn ide_chat_export_memories_to_path_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ExportMemoriesToPathInput>(params, "input")?;
    let target = PathBuf::from(input.path.trim());
    if input.path.trim().is_empty() {
        return Err("导出路径不能为空".to_string());
    }
    let parent = target
        .parent()
        .ok_or_else(|| "导出路径缺少父目录".to_string())?;
    fs::create_dir_all(parent).map_err(|err| format!("创建导出目录失败: {err}"))?;
    let selected_scopes = normalize_selected_export_scopes(&input.scopes)?;
    let owner_scope_by_agent = load_importable_agent_scope_labels(state)?;
    let payload = build_memory_exchange_payload(
        &state.data_path,
        &owner_scope_by_agent,
        Some(&selected_scopes),
    )?;
    let body = serde_json::to_string_pretty(&payload)
        .map_err(|err| format!("序列化导出记忆备份失败: {err}"))?;
    fs::write(&target, body).map_err(|err| format!("写入导出记忆备份失败: {err}"))?;
    ide_chat_serialize(ExportMemoriesFileResult {
        path: target.to_string_lossy().to_string(),
        count: payload.records.len(),
    })
}

fn ide_chat_import_memories_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ImportMemoriesInput>(params, "input")?;
    let stats = memory_store_import_memories(&state.data_path, &input.memories)?;
    ide_chat_serialize(ImportMemoriesResult {
        imported_count: stats.imported_count,
        created_count: stats.created_count,
        merged_count: stats.merged_count,
        total_count: stats.total_count,
    })
}

fn ide_chat_preview_import_angel_memories_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<PreviewImportAngelMemoriesInput>(params, "input")?;
    let parsed = parse_angel_memory_payload(&input.payload)?;
    ide_chat_serialize(PreviewImportAngelMemoriesResult {
        total_count: parsed.len(),
        scopes: build_preview_scope_items(&parsed),
        samples: sampled_angel_memory_preview_items(&parsed, 10),
    })
}

fn ide_chat_import_angel_memories_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ImportAngelMemoriesInput>(params, "input")?;
    let parsed = parse_angel_memory_payload(&input.payload)?;
    let scope_targets = resolve_import_scope_targets(state, &parsed, &input.scope_agent_mappings)?;
    let stats = import_angel_memories_by_scope(&state.data_path, &parsed, &scope_targets)?;
    ide_chat_serialize(ImportMemoriesResult {
        imported_count: stats.imported_count,
        created_count: stats.created_count,
        merged_count: stats.merged_count,
        total_count: stats.total_count,
    })
}

fn ide_chat_search_memories_mixed_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SearchMemoriesMixedInput>(params, "input")?;
    let started = std::time::Instant::now();
    let query = input.query.trim();
    if query.is_empty() {
        return ide_chat_serialize(SearchMemoriesMixedResult {
            memories: memory_store_list_memories(&state.data_path)?
                .into_iter()
                .map(|memory| SearchMemoriesMixedHit {
                    memory,
                    bm25_score: 0.0,
                    bm25_raw_score: 0.0,
                    vector_score: 0.0,
                    final_score: 0.0,
                })
                .collect::<Vec<_>>(),
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    let memories = memory_store_list_memories(&state.data_path)?;
    let ranked = memory_mixed_ranked_items(
        &state.data_path,
        &memories,
        query,
        MEMORY_MATCH_MAX_ITEMS * MEMORY_CANDIDATE_MULTIPLIER,
    );
    if ranked.is_empty() {
        return ide_chat_serialize(SearchMemoriesMixedResult {
            memories: Vec::new(),
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    let memory_map = memories
        .into_iter()
        .map(|memory| (memory.id.clone(), memory))
        .collect::<std::collections::HashMap<_, _>>();
    let mut out = Vec::<SearchMemoriesMixedHit>::new();
    for item in ranked {
        if let Some(memory) = memory_map.get(&item.memory_id) {
            out.push(SearchMemoriesMixedHit {
                memory: memory.clone(),
                bm25_score: item.bm25_score,
                bm25_raw_score: item.bm25_raw_score,
                vector_score: item.vector_score,
                final_score: item.final_score,
            });
        }
    }
    ide_chat_serialize(SearchMemoriesMixedResult {
        memories: out,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

fn ide_chat_search_chat_history_slices_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ChatHistorySearchInput>(params, "input")?;
    ide_chat_serialize(chat_history_search_for_agent(state, &input)?)
}

fn ide_chat_get_memory_provider_bindings_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    let conn = memory_store_open(&state.data_path)?;
    ide_chat_serialize(MemoryProviderBindings {
        embedding_api_config_id: memory_store_get_runtime_state(
            &conn,
            KB_STATE_EMBEDDING_API_CONFIG_ID,
        )?,
        rerank_api_config_id: memory_store_get_runtime_state(
            &conn,
            KB_STATE_RERANK_API_CONFIG_ID,
        )?,
    })
}

fn ide_chat_get_memory_embedding_sync_progress_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    let conn = memory_store_open(&state.data_path)?;
    ide_chat_serialize(MemoryEmbeddingSyncProgress {
        status: memory_store_get_runtime_state(&conn, KB_STATE_REBUILD_STATUS)?
            .unwrap_or_else(|| "idle".to_string()),
        done_batches: memory_store_get_runtime_state(&conn, KB_STATE_REBUILD_DONE_BATCHES)?
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0),
        total_batches: memory_store_get_runtime_state(&conn, KB_STATE_REBUILD_TOTAL_BATCHES)?
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0),
        trace_id: memory_store_get_runtime_state(&conn, KB_STATE_REBUILD_TRACE_ID)?,
        error: memory_store_get_runtime_state(&conn, KB_STATE_REBUILD_ERROR)?,
    })
}

fn ide_chat_test_memory_embedding_provider_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TestMemoryEmbeddingProviderInput>(params, "input")?;
    let started = std::time::Instant::now();
    let provider_id = input.provider_id.as_deref().unwrap_or("openai_embedding");
    let provider_kind = memory_provider_kind_from_id(provider_id);
    if matches!(provider_kind, MemoryProviderKind::VllmRerank) {
        return Err("rerank provider cannot be used as embedding provider.".to_string());
    }
    let app_config = read_config(&state.config_path)?;
    let provider_cfg = memory_resolve_provider_api_config(
        &app_config,
        provider_kind,
        input.api_config_id.as_deref(),
        provider_id,
    )
    .ok_or_else(|| "No matching API config for embedding test.".to_string())?;
    let model_name = input
        .model_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let provider = memory_create_embedding_provider(provider_kind, &provider_cfg, model_name)?;
    let text = input
        .text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("memory embedding connectivity test")
        .to_string();
    let vectors = provider.embed_batch(&vec![text])?;
    let first = vectors
        .first()
        .ok_or_else(|| "embedding test returned empty vectors".to_string())?;
    let dim = first.len();
    if dim == 0 {
        return Err("embedding test returned zero-dim vector".to_string());
    }
    ide_chat_serialize(TestMemoryEmbeddingProviderResult {
        provider_kind: format!("{provider_kind:?}"),
        model_name: model_name.unwrap_or(provider_cfg.model.trim()).to_string(),
        vector_dim: dim,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

fn ide_chat_test_memory_rerank_provider_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TestMemoryRerankProviderInput>(params, "input")?;
    let started = std::time::Instant::now();
    let app_config = read_config(&state.config_path)?;
    let provider_kind = MemoryProviderKind::VllmRerank;
    let provider_cfg = memory_resolve_provider_api_config(
        &app_config,
        provider_kind,
        input.api_config_id.as_deref(),
        "vllm_rerank",
    )
    .ok_or_else(|| "No matching API config for rerank test.".to_string())?;
    let model_name = input
        .model_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let provider = memory_create_rerank_provider(provider_kind, &provider_cfg, model_name)?;
    let query = input
        .query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("用户偏好什么风格？")
        .to_string();
    let documents = input.documents.unwrap_or_else(|| {
        vec![
            "用户偏好简洁回答，尽量直接结论。".to_string(),
            "用户喜欢复杂铺垫和长篇解释。".to_string(),
            "今天主要讨论了记忆系统检索。".to_string(),
        ]
    });
    let results = provider.rerank(&query, &documents, Some(3))?;
    let top = results.iter().max_by(|a, b| {
        a.relevance_score
            .partial_cmp(&b.relevance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ide_chat_serialize(TestMemoryRerankProviderResult {
        provider_kind: format!("{provider_kind:?}"),
        model_name: model_name.unwrap_or(provider_cfg.model.trim()).to_string(),
        elapsed_ms: started.elapsed().as_millis(),
        result_count: results.len(),
        top_index: top.map(|item| item.index),
        top_score: top.map(|item| item.relevance_score),
    })
}

fn ide_chat_save_memory_embedding_binding_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SaveMemoryEmbeddingBindingInput>(params, "input")?;
    let api_id = input.api_config_id.trim();
    if api_id.is_empty() {
        let conn = memory_store_open(&state.data_path)?;
        let old_provider_id =
            memory_store_get_runtime_state(&conn, KB_STATE_ACTIVE_INDEX_PROVIDER_ID)?;
        memory_store_set_runtime_state(&conn, KB_STATE_EMBEDDING_API_CONFIG_ID, "")?;
        memory_store_set_runtime_state(&conn, KB_STATE_ACTIVE_INDEX_PROVIDER_ID, "")?;
        return ide_chat_serialize(MemoryStoreProviderSyncReport {
            status: "disabled".to_string(),
            old_provider_id,
            new_provider_id: String::new(),
            deleted: 0,
            added: 0,
            batch_count: 0,
        });
    }

    let app_config = read_config(&state.config_path)?;
    let api = app_config
        .api_configs
        .iter()
        .find(|item| item.id == api_id)
        .cloned()
        .ok_or_else(|| "Selected embedding API config not found.".to_string())?;
    let provider_kind = match api.request_format {
        RequestFormat::OpenAIEmbedding => MemoryProviderKind::OpenAIEmbedding,
        RequestFormat::GeminiEmbedding => MemoryProviderKind::GeminiEmbedding,
        _ => {
            return Err(format!(
                "request_format '{}' is not embedding protocol.",
                api.request_format
            ))
        }
    };
    let model_name = input
        .model_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(api.model.trim());
    if model_name.is_empty() {
        return Err("Embedding model is empty.".to_string());
    }
    let provider_cfg = MemoryProviderApiConfig {
        base_url: api.base_url.clone(),
        api_key: api.api_key.clone(),
        model: api.model.clone(),
    };
    let provider = memory_create_embedding_provider(provider_kind, &provider_cfg, Some(model_name))?;
    let provider_id = memory_binding_provider_id(&api.id, api.request_format.as_str(), model_name);
    let batch_size = input.batch_size.unwrap_or(64).max(1);
    let report = memory_store_sync_provider_index(
        &state.data_path,
        &provider_id,
        model_name,
        batch_size,
        |texts| provider.embed_batch(texts),
    )?;

    let conn = memory_store_open(&state.data_path)?;
    memory_store_set_runtime_state(&conn, KB_STATE_EMBEDDING_API_CONFIG_ID, &api.id)?;
    ide_chat_serialize(report)
}

fn ide_chat_save_memory_rerank_binding_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SaveMemoryRerankBindingInput>(params, "input")?;
    let api_id = input.api_config_id.trim();
    if api_id.is_empty() {
        let conn = memory_store_open(&state.data_path)?;
        memory_store_set_runtime_state(&conn, KB_STATE_RERANK_API_CONFIG_ID, "")?;
        return ide_chat_serialize(SaveMemoryRerankBindingResult {
            status: "disabled".to_string(),
            rerank_api_config_id: None,
            model_name: String::new(),
        });
    }
    let app_config = read_config(&state.config_path)?;
    let api = app_config
        .api_configs
        .iter()
        .find(|item| item.id == api_id)
        .cloned()
        .ok_or_else(|| "Selected rerank API config not found.".to_string())?;
    if !matches!(api.request_format, RequestFormat::OpenAIRerank) {
        return Err(format!(
            "request_format '{}' is not rerank protocol.",
            api.request_format
        ));
    }
    let model_name = input
        .model_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(api.model.trim());
    if model_name.is_empty() {
        return Err("Rerank model is empty.".to_string());
    }

    let conn = memory_store_open(&state.data_path)?;
    memory_store_set_runtime_state(&conn, KB_STATE_RERANK_API_CONFIG_ID, &api.id)?;
    ide_chat_serialize(SaveMemoryRerankBindingResult {
        status: "saved".to_string(),
        rerank_api_config_id: Some(api.id),
        model_name: model_name.to_string(),
    })
}

fn ide_chat_get_agent_private_memory_count_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<AgentPrivateMemoryCountInput>(params, "input")?;
    let agent_id = input.agent_id.trim();
    if agent_id.is_empty() {
        return Err("agentId is required".to_string());
    }
    let config = read_config(&state.config_path)?;
    let agents = state_read_agents_cached(state)?;
    let (private_agent_ids, _) = runtime_private_organization_ids(&state.data_path, &config, &agents)?;
    if private_agent_ids.contains(agent_id) {
        return Err(private_agent_operation_error(agent_id));
    }
    ide_chat_serialize(AgentPrivateMemoryCountResult {
        count: memory_store_count_private_memories_by_agent(&state.data_path, agent_id)?,
    })
}

fn ide_chat_set_agent_memory_recall_mode_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SetAgentMemoryRecallModeInput>(params, "input")?;
    let agent_id = input.agent_id.trim();
    if agent_id.is_empty() {
        return Err("agentId is required".to_string());
    }
    let mode = match input.mode.trim().to_ascii_lowercase().as_str() {
        MEMORY_RECALL_MODE_AUTO => MEMORY_RECALL_MODE_AUTO.to_string(),
        MEMORY_RECALL_MODE_MANUAL => MEMORY_RECALL_MODE_MANUAL.to_string(),
        MEMORY_RECALL_MODE_OFF => MEMORY_RECALL_MODE_OFF.to_string(),
        _ => return Err("memoryRecallMode must be auto, manual, or off".to_string()),
    };

    let mut agents = state_read_agents_cached(state)?;
    let base_config = read_config(&state.config_path)?;
    let (private_agent_ids, _) =
        runtime_private_organization_ids(&state.data_path, &base_config, &agents)?;
    if private_agent_ids.contains(agent_id) {
        return Err(private_agent_operation_error(agent_id));
    }

    let agent_idx = agents
        .iter()
        .position(|agent| agent.id == agent_id && !agent.is_built_in_user)
        .ok_or_else(|| format!("Agent '{}' not found.", agent_id))?;
    if normalize_agent_memory_recall_mode(&agents[agent_idx].memory_recall_mode) != mode {
        agents[agent_idx].memory_recall_mode = mode.clone();
        state_write_agents_cached(state, &agents)?;
        runtime_log_info(format!(
            "[记忆] 完成，任务=切换人格回忆模式，agent_id={}，mode={}",
            agent_id, mode
        ));
    }
    ide_chat_serialize(SetAgentMemoryRecallModeResult {
        agent_id: agent_id.to_string(),
        mode,
    })
}

fn ide_chat_set_agent_private_memory_enabled_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SetAgentPrivateMemoryEnabledInput>(params, "input")?;
    let agent_id = input.agent_id.trim();
    if agent_id.is_empty() {
        return Err("agentId is required".to_string());
    }

    let mut agents = state_read_agents_cached(state)?;
    let base_config = read_config(&state.config_path)?;
    let (private_agent_ids, _) =
        runtime_private_organization_ids(&state.data_path, &base_config, &agents)?;
    if private_agent_ids.contains(agent_id) {
        return Err(private_agent_operation_error(agent_id));
    }

    let agent_idx = agents
        .iter()
        .position(|agent| agent.id == agent_id && !agent.is_built_in_user)
        .ok_or_else(|| format!("Agent '{}' not found.", agent_id))?;
    let current = agents[agent_idx].private_memory_enabled;
    if current == input.enabled {
        return ide_chat_serialize(SetAgentPrivateMemoryEnabledResult {
            agent_id: agent_id.to_string(),
            enabled: current,
            exported_count: 0,
            deleted_count: 0,
            export_path: None,
        });
    }

    if input.enabled {
        agents[agent_idx].private_memory_enabled = true;
        state_write_agents_cached(state, &agents)?;
        return ide_chat_serialize(SetAgentPrivateMemoryEnabledResult {
            agent_id: agent_id.to_string(),
            enabled: true,
            exported_count: 0,
            deleted_count: 0,
            export_path: None,
        });
    }

    let export = memory_store_export_agent_private_memories(&state.data_path, agent_id)?;
    let deleted = memory_store_delete_memories_by_owner_agent_id(&state.data_path, agent_id)?;
    agents[agent_idx].private_memory_enabled = false;
    state_write_agents_cached(state, &agents)?;
    ide_chat_serialize(SetAgentPrivateMemoryEnabledResult {
        agent_id: agent_id.to_string(),
        enabled: false,
        exported_count: export.count,
        deleted_count: deleted,
        export_path: Some(export.path),
    })
}

fn ide_chat_export_agent_private_memories_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ExportAgentPrivateMemoriesInput>(params, "input")?;
    let agent_id = input.agent_id.trim();
    if agent_id.is_empty() {
        return Err("agentId is required".to_string());
    }
    let config = read_config(&state.config_path)?;
    let agents = state_read_agents_cached(state)?;
    let (private_agent_ids, _) = runtime_private_organization_ids(&state.data_path, &config, &agents)?;
    if private_agent_ids.contains(agent_id) {
        return Err(private_agent_operation_error(agent_id));
    }
    let export = memory_store_export_agent_private_memories(&state.data_path, agent_id)?;
    ide_chat_serialize(ExportAgentPrivateMemoriesResult {
        count: export.count,
        path: export.path,
    })
}

fn ide_chat_disable_agent_private_memory_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<DisableAgentPrivateMemoryInput>(params, "input")?;
    let agent_id = input.agent_id.trim();
    if agent_id.is_empty() {
        return Err("agentId is required".to_string());
    }

    let mut agents = state_read_agents_cached(state)?;
    let base_config = read_config(&state.config_path)?;
    let (private_agent_ids, _) =
        runtime_private_organization_ids(&state.data_path, &base_config, &agents)?;
    if private_agent_ids.contains(agent_id) {
        return Err(private_agent_operation_error(agent_id));
    }

    let agent_idx = agents
        .iter()
        .position(|agent| agent.id == agent_id && !agent.is_built_in_user)
        .ok_or_else(|| format!("Agent '{}' not found.", agent_id))?;
    if !agents[agent_idx].private_memory_enabled {
        return ide_chat_serialize(DisableAgentPrivateMemoryResult {
            agent_id: agent_id.to_string(),
            enabled: false,
            deleted_count: 0,
        });
    }

    let deleted = memory_store_delete_memories_by_owner_agent_id(&state.data_path, agent_id)?;
    agents[agent_idx].private_memory_enabled = false;
    state_write_agents_cached(state, &agents)?;
    ide_chat_serialize(DisableAgentPrivateMemoryResult {
        agent_id: agent_id.to_string(),
        enabled: false,
        deleted_count: deleted,
    })
}

fn ide_chat_task_list_tasks_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(task_store_list_tasks(&state.data_path)?)
}

fn ide_chat_task_get_task_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TaskGetInput>(params, "input")?;
    ide_chat_serialize(task_store_get_task(&state.data_path, input.task_id.trim())?)
}

fn ide_chat_task_create_task_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TaskCreateInput>(params, "input")?;
    let input = task_create_input_for_write(state, &input)?;
    ide_chat_serialize(task_store_create_task(&state.data_path, &input)?)
}

fn ide_chat_task_update_task_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TaskUpdateInput>(params, "input")?;
    let input = task_update_input_for_write(state, &input)?;
    ide_chat_serialize(task_store_update_task(&state.data_path, &input)?)
}

fn ide_chat_task_complete_task_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TaskCompleteInput>(params, "input")?;
    ide_chat_serialize(task_store_complete_task(&state.data_path, &input)?)
}

fn ide_chat_task_delete_task_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TaskDeleteInput>(params, "input")?;
    task_store_delete_task(&state.data_path, input.task_id.trim())?;
    Ok(serde_json::json!(null))
}

fn ide_chat_task_list_run_logs_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TaskRunLogListInput>(params, "input")?;
    ide_chat_serialize(task_store_list_run_logs(
        &state.data_path,
        input.task_id.as_deref(),
        input.limit.unwrap_or(50),
    )?)
}

async fn ide_chat_task_optimize_draft_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TaskOptimizeDraftInput>(params, "input")?;
    ide_chat_serialize(task_optimize_draft_internal(input, state).await?)
}

fn ide_chat_mcp_list_servers_for_web_settings(state: &AppState) -> Result<Value, String> {
    let mut out = load_workspace_mcp_servers(state)?;
    for item in &mut out {
        *item = overlay_runtime_state_on_server(item.clone());
    }
    ide_chat_serialize(out)
}

fn ide_chat_mcp_validate_definition_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpDefinitionValidateInput>(params, "input")?;
    let _schema = mcp_definition_json_schema();
    let result = match normalize_mcp_definition_for_validation(&input.definition_json) {
        Ok((normalized_value, migrated)) => {
            let normalized_text = serde_json::to_string(&normalized_value)
                .map_err(|err| format!("序列化标准化 MCP 定义失败：{err}"))?;
            let (name, parsed) = parse_mcp_server_definition(&normalized_text)?;
            let _ = migrated;
            McpDefinitionValidateResult {
                ok: true,
                transport: Some(parsed.transport.as_str().to_string()),
                server_name: Some(name),
                message: "MCP definition is valid".to_string(),
                schema_version: None,
                error_code: None,
                details: Vec::new(),
                migrated_definition_json: None,
            }
        }
        Err(err) => McpDefinitionValidateResult {
            ok: false,
            transport: None,
            server_name: None,
            message: err.message,
            schema_version: None,
            error_code: Some(err.code),
            details: err.details,
            migrated_definition_json: None,
        },
    };
    ide_chat_serialize(result)
}

fn ide_chat_mcp_save_server_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerInput>(params, "input")?;
    let next = normalize_mcp_server_input(input)?;
    save_workspace_mcp_server(state, &next)?;
    let mut saved = load_server_by_id(state, &next.id)?;
    saved = overlay_runtime_state_on_server(saved);
    ide_chat_serialize(saved)
}

async fn ide_chat_mcp_remove_server_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerIdInput>(params, "input")?;
    let server_id = input.server_id.trim();
    if server_id.is_empty() {
        return Err("serverId is required".to_string());
    }
    let removed = remove_workspace_mcp_server(state, server_id)?;
    if removed {
        mcp_disconnect_cached_client(server_id).await;
        mcp_runtime_state_remove(server_id);
    }
    ide_chat_serialize(removed)
}

async fn ide_chat_mcp_list_server_tools_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerIdInput>(params, "input")?;
    let server_id = input.server_id.trim();
    if server_id.is_empty() {
        return Err("serverId is required".to_string());
    }
    let server = load_server_by_id(state, server_id)?;
    let started = std::time::Instant::now();
    mcp_runtime_state_mark_starting(&server);
    let tools = match mcp_list_server_tools_runtime(&server).await {
        Ok(tools) => tools,
        Err(err) => {
            let status = mcp_status_from_runtime_error(&err);
            mcp_runtime_state_mark_probe_failure(&server, status, &err);
            return Err(err);
        }
    };
    let discovered_names = tools
        .iter()
        .map(|t| t.tool_name.clone())
        .collect::<Vec<_>>();
    let merged_policies =
        merge_workspace_mcp_tool_policies_with_new_tools(state, &server.id, &discovered_names)?;
    let mut server_with_policies = server.clone();
    server_with_policies.tool_policies = merged_policies;
    let final_tools = tools
        .into_iter()
        .map(|tool| {
            let enabled = mcp_policy_enabled_for_tool(&server_with_policies, &tool.tool_name)
                && mcp_tool_allowed_by_definition(&server_with_policies, &tool.tool_name);
            McpToolDescriptor { enabled, ..tool }
        })
        .collect::<Vec<_>>();
    mcp_runtime_state_set(&server.id, true, "ready", "", final_tools.clone());
    refresh_global_tool_schema_cache(state);
    mark_prompt_cache_rebuild_for_all_final_system_sources(state);
    ide_chat_serialize(McpListServerToolsResult {
        server_id: server.id,
        tools: final_tools,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

fn ide_chat_mcp_list_server_tools_cached_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerIdInput>(params, "input")?;
    let server_id = input.server_id.trim();
    if server_id.is_empty() {
        return Err("serverId is required".to_string());
    }
    let server = load_server_by_id(state, server_id)?;
    let started = std::time::Instant::now();
    let tools = list_tools_from_runtime_or_policy(&server);
    ide_chat_serialize(McpListServerToolsResult {
        server_id: server.id,
        tools,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

fn ide_chat_mcp_deploy_server_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerIdInput>(params, "input")?;
    let server_id = input.server_id.trim();
    if server_id.is_empty() {
        return Err("serverId is required".to_string());
    }
    let server = {
        let server = load_server_by_id(state, server_id)?;
        set_workspace_mcp_policy_enabled(state, server_id, true)?;
        server
    };
    let started = std::time::Instant::now();
    mcp_runtime_state_mark_starting(&server);
    mcp_start_supervisor_probe_for_server(state.clone(), server.clone(), "manual_deploy");
    let server_id = server.id.clone();
    let tools = list_tools_from_runtime_or_policy(&server);
    ide_chat_serialize(McpListServerToolsResult {
        server_id,
        tools,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

async fn ide_chat_mcp_undeploy_server_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerIdInput>(params, "input")?;
    let server_id = input.server_id.trim();
    if server_id.is_empty() {
        return Err("serverId is required".to_string());
    }
    {
        let _ = load_server_by_id(state, server_id)?;
        set_workspace_mcp_policy_enabled(state, server_id, false)?;
    }
    mcp_disconnect_cached_client(server_id).await;
    mcp_runtime_state_set(server_id, false, "stopped", "", Vec::new());
    let mut out = load_server_by_id(state, server_id)?;
    out = overlay_runtime_state_on_server(out);
    ide_chat_serialize(out)
}

fn ide_chat_mcp_set_tool_enabled_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpSetToolEnabledInput>(params, "input")?;
    let server_id = input.server_id.trim();
    let tool_name = input.tool_name.trim();
    if server_id.is_empty() {
        return Err("serverId is required".to_string());
    }
    if tool_name.is_empty() {
        return Err("toolName is required".to_string());
    }
    let policies = {
        let _ = load_server_by_id(state, server_id)?;
        let mut policies = load_workspace_mcp_tool_policies(state, server_id)?;
        if let Some(policy) = policies.iter_mut().find(|p| p.tool_name == tool_name) {
            policy.enabled = input.enabled;
        } else {
            policies.push(McpToolPolicy {
                tool_name: tool_name.to_string(),
                enabled: input.enabled,
            });
        }
        save_workspace_mcp_tool_policies(state, server_id, &policies)?;
        policies
    };
    mcp_runtime_state_set_tool_enabled(server_id, tool_name, input.enabled);
    let mut server = load_server_by_id(state, server_id)?;
    server.tool_policies = policies;
    server = overlay_runtime_state_on_server(server);
    ide_chat_serialize(server)
}

fn ide_chat_mcp_open_workspace_dir_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(open_mcp_workspace_dir(state)?)
}

async fn ide_chat_mcp_refresh_mcp_and_skills_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(reload_workspace(state).await?)
}

fn ide_chat_mcp_list_skills_for_web_settings(state: &AppState) -> Result<Value, String> {
    let (skills, errors) = load_workspace_skill_summaries_with_errors(state)?;
    let _ = update_hidden_skill_snapshot_cache(state, &skills, None);
    ide_chat_serialize(SkillListResult { skills, errors })
}

fn ide_chat_skill_open_workspace_dir_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(open_skills_workspace_dir(state)?)
}

fn ide_chat_get_storage_usage_overview_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(build_storage_usage_overview(state)?)
}

fn ide_chat_open_storage_usage_item_directory_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<OpenStorageUsageItemDirectoryInput>(params, "input")?;
    let item_id = input.item_id.trim();
    let target = storage_usage_target_path(state, item_id)
        .ok_or_else(|| format!("未知存储分类：{item_id}"))?;
    let app_root = app_root_from_data_path(&state.data_path);
    let open_dir = storage_existing_directory_for_open(&target)?;
    let canonical_root = app_root.canonicalize().unwrap_or(app_root);
    let canonical_open_dir = open_dir.canonicalize().unwrap_or(open_dir.clone());
    if !canonical_open_dir.starts_with(&canonical_root) {
        return Err(format!(
            "拒绝打开应用私有目录之外的路径，path={}",
            canonical_open_dir.display()
        ));
    }
    open_shell_path_in_file_manager(&canonical_open_dir)?;
    Ok(serde_json::json!(null))
}

fn ide_chat_cleanup_storage_legacy_items_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<CleanupStorageLegacyItemsInput>(params, "input")?;
    let cleanup_kind = input.cleanup_kind.trim();
    let (scope, label) = match cleanup_kind {
        STORAGE_CLEANUP_LEGACY_CONVERSATIONS => (
            StorageLegacyConversationScope::Normal,
            "旧普通会话 JSON",
        ),
        STORAGE_CLEANUP_LEGACY_DELEGATE_CONVERSATIONS => (
            StorageLegacyConversationScope::Delegate,
            "旧委托会话 JSON",
        ),
        _ => return Err(format!("未知存储清理类型：{cleanup_kind}")),
    };
    let _migration_guard = lock_message_store_migration();
    eprintln!(
        "[存储] 开始，任务=清理{}，cleanup_kind={}",
        label,
        cleanup_kind
    );
    let started_at = std::time::Instant::now();
    let result = cleanup_storage_legacy_scope(state, scope);
    match &result {
        Ok(report) => eprintln!(
            "[存储] 完成，任务=清理{}，cleanup_kind={}，删除文件数={}，跳过文件数={}，释放字节={}，耗时毫秒={}",
            label,
            cleanup_kind,
            report.deleted_file_count,
            report.skipped_file_count,
            report.freed_bytes,
            started_at.elapsed().as_millis()
        ),
        Err(err) => eprintln!(
            "[存储] 失败，任务=清理{}，cleanup_kind={}，error={}，耗时毫秒={}",
            label,
            cleanup_kind,
            err,
            started_at.elapsed().as_millis()
        ),
    }
    ide_chat_serialize(result?)
}

fn ide_chat_migration_error_message(err: MigrationCommandError) -> String {
    match err.code {
        Some(code) if !code.trim().is_empty() => format!("{}: {}", code, err.message),
        _ => err.message,
    }
}

fn ide_chat_uploaded_migration_package_path(
    state: &AppState,
    input: &PreviewImportConfigMigrationPackageInput,
) -> Result<Option<PathBuf>, String> {
    let bytes_base64 = input
        .package_bytes_base64
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(bytes_base64) = bytes_base64 else {
        return Ok(None);
    };
    let bytes = B64
        .decode(bytes_base64)
        .map_err(|err| format!("解析迁移包上传内容失败: {err}"))?;
    let upload_dir = migration_temp_root(state).join("uploads");
    fs::create_dir_all(&upload_dir)
        .map_err(|err| format!("创建迁移包上传临时目录失败: {err}"))?;
    let extension = input
        .package_file_name
        .as_deref()
        .and_then(|name| Path::new(name).extension())
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| value.eq_ignore_ascii_case("zip"))
        .unwrap_or("zip");
    let path = upload_dir.join(format!("{}.{}", Uuid::new_v4(), extension));
    fs::write(&path, bytes).map_err(|err| format!("写入迁移包上传临时文件失败: {err}"))?;
    Ok(Some(path))
}

fn ide_chat_export_config_migration_package_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ExportConfigMigrationPackageInput>(params, "input")?;
    validate_export_migration_password(&input.password)?;
    let total_started_at = std::time::Instant::now();
    runtime_log_info(format!(
        "[迁移包导出] 开始 task=export_config_migration_package trigger=web_settings password_present={} password_len={}",
        !input.password.trim().is_empty(),
        input.password.chars().count()
    ));
    let payload_started_at = std::time::Instant::now();
    let payload = build_export_payload(state)?;
    runtime_log_info(format!(
        "[迁移包导出] 完成 task=export_config_migration_package trigger=web_settings stage=build_export_payload provider_count={} api_config_count={} memory_count={} duration_ms={}",
        payload.config.api_providers.len(),
        payload.config.api_configs.len(),
        payload.memories.len(),
        payload_started_at.elapsed().as_millis()
    ));
    let manifest = MigrationManifest {
        schema_version: MIGRATION_SCHEMA_VERSION,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        exported_at: now_iso(),
    };
    let exports_dir = app_root_from_data_path(&state.data_path).join("exports");
    fs::create_dir_all(&exports_dir).map_err(|err| format!("创建导出目录失败: {err}"))?;
    let file_name = format!(
        "p-ai-migration-{}.zip",
        now_iso()
            .replace(':', "-")
            .replace('/', "-")
            .replace('\\', "-")
    );
    let path = exports_dir.join(&file_name);
    write_migration_package(&path, input.password.trim(), &manifest, &payload)?;
    let bytes = fs::read(&path).map_err(|err| format!("读取迁移包失败: {err}"))?;
    runtime_log_info(format!(
        "[迁移包导出] 完成 task=export_config_migration_package trigger=web_settings stage=write_migration_package path={} provider_count={} api_config_count={} memory_count={} total_duration_ms={}",
        path.to_string_lossy(),
        payload.config.api_providers.len(),
        payload.config.api_configs.len(),
        payload.memories.len(),
        total_started_at.elapsed().as_millis()
    ));
    ide_chat_serialize(ExportConfigMigrationPackageResult {
        path: path.to_string_lossy().to_string(),
        provider_count: payload.config.api_providers.len(),
        api_config_count: payload.config.api_configs.len(),
        memory_count: payload.memories.len(),
        file_name,
        bytes_base64: B64.encode(bytes),
    })
}

fn ide_chat_preview_import_config_migration_package_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let mut input =
        ide_chat_parse_param_field::<PreviewImportConfigMigrationPackageInput>(params, "input")?;
    if let Some(uploaded_path) = ide_chat_uploaded_migration_package_path(state, &input)? {
        input.package_path = Some(uploaded_path.to_string_lossy().to_string());
    }
    let package_path = input
        .package_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| "迁移包路径不能为空".to_string())?;
    let preview_id = Uuid::new_v4().to_string();
    let preview_dir = migration_preview_dir(state, &preview_id);
    unzip_migration_package_to_dir(&package_path, input.password.trim(), &preview_dir)
        .map_err(ide_chat_migration_error_message)?;
    let (manifest, payload) = read_preview_payload(&preview_dir)?;
    assert_manifest_version(&manifest)?;

    let current_config = state_read_config_cached(state)?;
    let memory_preview = preview_memory_import(state, &preview_dir, &payload.memories)?;
    let (_, provider_added_count, provider_updated_count) =
        merge_api_providers(&current_config.api_providers, &payload.config.api_providers);
    let (_, api_config_added_count, api_config_updated_count) =
        merge_api_configs(&current_config.api_configs, &payload.config.api_configs);
    state
        .migration_preview_dirs
        .lock()
        .map_err(|err| format!("锁定迁移预检目录失败: {err}"))?
        .insert(preview_id.clone(), preview_dir.to_string_lossy().to_string());

    ide_chat_serialize(PreviewImportConfigMigrationPackageResult {
        preview_id,
        package_version: manifest.app_version,
        memory_added_count: memory_preview.created_count,
        memory_merged_count: memory_preview.merged_count,
        provider_added_count,
        provider_updated_count,
        api_config_added_count,
        api_config_updated_count,
        oauth_file_count: payload.oauth_files.len(),
        avatar_file_count: payload.avatar_files.len(),
    })
}

fn ide_chat_apply_import_config_migration_package_for_web_settings(
    state: &AppState,
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ApplyImportConfigMigrationPackageInput>(params, "input")?;
    let preview_dir = state
        .migration_preview_dirs
        .lock()
        .map_err(|err| format!("锁定迁移预检目录失败: {err}"))?
        .remove(input.preview_id.trim())
        .ok_or_else(|| "迁移预检已失效，请重新选择迁移包。".to_string())?;
    let preview_dir = PathBuf::from(preview_dir);
    let (manifest, payload) = read_preview_payload(&preview_dir)?;
    assert_manifest_version(&manifest)?;
    let backup_dir = backup_current_migration_targets(state)?;
    let current_config = state_read_config_cached(state)?;
    let current_data = state_read_agents_runtime_snapshot(state)?;
    let (
        final_config,
        provider_added_count,
        provider_updated_count,
        api_config_added_count,
        api_config_updated_count,
    ) = build_imported_config(&current_config, &payload.config);
    let avatar_path_map = write_avatar_files(state, &payload.avatar_files)?;
    write_oauth_files(&final_config, &payload.oauth_files)?;
    let final_data = build_imported_runtime(&current_data, &payload.runtime_data, &avatar_path_map);
    let memory_stats = memory_store_import_memories(&state.data_path, &payload.memories)?;
    state_write_config_cached(state, &final_config)?;
    state_write_agents_cached(state, &final_data.agents)?;
    state_write_runtime_state_cached(state, &build_runtime_state_file(&final_data))?;
    if let Err(err) = fs::remove_dir_all(&preview_dir) {
        runtime_log_warn(format!(
            "[迁移包导入] 失败 task=apply_import_config_migration_package stage=remove_preview_dir path={} err={:?}",
            preview_dir.display(),
            err
        ));
    }
    let result = ApplyImportConfigMigrationPackageResult {
        imported_memory_count: memory_stats.imported_count,
        created_memory_count: memory_stats.created_count,
        merged_memory_count: memory_stats.merged_count,
        provider_added_count,
        provider_updated_count,
        api_config_added_count,
        api_config_updated_count,
        backup_dir: backup_dir.to_string_lossy().to_string(),
    };
    let app_handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        graceful_restart_app(&app_handle);
    });
    ide_chat_serialize(result)
}

fn ide_chat_list_recent_llm_round_logs_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    let capacity = llm_round_log_capacity_for_state(state);
    let logs = state
        .llm_round_logs
        .lock()
        .map_err(|_| "Failed to lock llm round logs".to_string())?;
    ide_chat_serialize(
        logs.iter()
            .skip(logs.len().saturating_sub(capacity))
            .map(compact_llm_round_log_entry_for_ui)
            .collect::<Vec<_>>(),
    )
}

fn ide_chat_get_recent_llm_round_log_section_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let (id, section) = match params {
        Value::Object(mut map) => {
            let id = map
                .remove("id")
                .and_then(|value| value.as_str().map(ToOwned::to_owned))
                .unwrap_or_default();
            let section = map
                .remove("section")
                .and_then(|value| value.as_str().map(ToOwned::to_owned))
                .unwrap_or_default();
            (id, section)
        }
        _ => (String::new(), String::new()),
    };
    let id = id.trim().to_string();
    if id.is_empty() {
        return Ok(serde_json::json!(null));
    }
    let logs = state
        .llm_round_logs
        .lock()
        .map_err(|_| "Failed to lock llm round logs".to_string())?;
    ide_chat_serialize(logs.iter().rev().find_map(|entry| {
        find_llm_round_log_entry_by_id(entry, &id)
            .and_then(|entry| llm_round_log_section_value(entry, &section))
    }))
}

fn ide_chat_clear_recent_llm_round_logs_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
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
    ide_chat_serialize(true)
}

fn ide_chat_list_terminal_shell_candidates_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    let (preferred_kind, current, options) = terminal_shell_candidates_for_ui(state);
    Ok(serde_json::json!({
        "preferredKind": preferred_kind,
        "currentKind": current.kind,
        "currentPath": current.path,
        "options": options,
    }))
}

fn ide_chat_open_chat_shell_workspace_dir_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_optional_param_field::<ShellWorkspacePathInput>(params, "input")?;
    let root = resolve_requested_shell_workspace_root(
        state,
        input.as_ref().and_then(|value| value.workspace_path.as_deref()),
        true,
    )?;
    open_shell_path_in_file_manager(&root)?;
    ide_chat_serialize(shell_workspace_display_path(&root))
}

fn ide_chat_reset_chat_shell_workspace_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_optional_param_field::<ShellWorkspacePathInput>(params, "input")?;
    let root = resolve_requested_shell_workspace_root(
        state,
        input.as_ref().and_then(|value| value.workspace_path.as_deref()),
        true,
    )?;
    ensure_workspace_mcp_layout_at_root(&root)?;
    ensure_workspace_skills_layout_at_root(&root)?;
    ensure_workspace_private_organization_layout_at_root(&root)?;
    ide_chat_serialize(shell_workspace_display_path(&root))
}

fn ide_chat_get_default_chat_shell_workspace_path_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    let root = terminal_default_session_root_canonical(state)?;
    ide_chat_serialize(shell_workspace_display_path(&root))
}

async fn ide_chat_migrate_shell_workspace_directory_for_web_settings(
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<MigrateWorkspaceDirectoryInput>(params, "input")?;
    ide_chat_serialize(migrate_shell_workspace_directory(input, app.clone()).await?)
}

async fn ide_chat_install_host_runtime_prerequisite_for_web_settings(
    params: Value,
) -> Result<Value, String> {
    let kind = match params {
        Value::Object(mut map) => map
            .remove("kind")
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_default(),
        _ => String::new(),
    };
    ide_chat_serialize(install_host_runtime_prerequisite(kind).await?)
}

fn ide_chat_get_host_runtime_prerequisites_for_web_settings() -> Result<Value, String> {
    ide_chat_serialize(get_host_runtime_prerequisites())
}

fn ide_chat_show_window_for_web_settings(app: &AppHandle, label: &str) -> Result<Value, String> {
    show_window(app, label)?;
    Ok(serde_json::json!(null))
}

fn ide_chat_open_runtime_logs_window_for_web_settings(app: &AppHandle) -> Result<Value, String> {
    show_runtime_logs_window(app)?;
    Ok(serde_json::json!(null))
}

fn ide_chat_set_webview_zoom_percent_for_web_settings(
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let percent = match params {
        Value::Object(mut map) => map
            .remove("percent")
            .and_then(|value| value.as_u64())
            .unwrap_or(100),
        _ => 100,
    };
    let normalized = apply_webview_zoom_percent(app, percent as u32)?;
    emit_webview_zoom_percent_updated(app, normalized);
    ide_chat_serialize(normalized)
}

fn ide_chat_set_github_update_method_for_web_settings(
    state: &AppState,
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let update_method = match params {
        Value::Object(mut map) => map
            .remove("updateMethod")
            .or_else(|| map.remove("update_method"))
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_default(),
        _ => String::new(),
    };
    let normalized = normalize_github_update_method(&update_method);
    let mut config = state_read_config_cached(state)?;
    normalize_app_config(&mut config);
    if config.github_update_method != normalized {
        config.github_update_method = normalized.clone();
        state_write_config_cached(state, &config)?;
        eprintln!("[自动更新] 更新方式偏好已保存：method={normalized}");
    }
    let data = state_read_agents_runtime_snapshot(state)?;
    let runtime_config = runtime_config_with_private_organization(state, &config, &data)?;
    let _ = app.emit("easy-call:config-updated", &runtime_config);
    ide_chat_serialize(runtime_config)
}

async fn ide_chat_check_github_update_for_web_settings(params: Value) -> Result<Value, String> {
    let update_method = match params {
        Value::Object(mut map) => map
            .remove("updateMethod")
            .or_else(|| map.remove("update_method"))
            .and_then(|value| value.as_str().map(ToOwned::to_owned)),
        _ => None,
    };
    ide_chat_serialize(check_github_update(update_method).await?)
}

async fn ide_chat_start_github_update_for_web_settings(
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let (force, update_method) = match params {
        Value::Object(mut map) => {
            let force = map
                .remove("force")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let update_method = map
                .remove("updateMethod")
                .or_else(|| map.remove("update_method"))
                .and_then(|value| value.as_str().map(ToOwned::to_owned));
            (force, update_method)
        }
        _ => (false, None),
    };
    start_github_update(app.clone(), force, update_method).await?;
    Ok(serde_json::json!(null))
}

async fn ide_chat_apply_prepared_github_update_for_web_settings(
    app: &AppHandle,
) -> Result<Value, String> {
    apply_prepared_github_update(app.clone()).await?;
    Ok(serde_json::json!(null))
}

async fn ide_chat_codex_get_auth_status_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<CodexAuthStatusInput>(params, "input")?;
    ide_chat_serialize(codex_get_auth_status(input).await?)
}

async fn ide_chat_codex_start_oauth_login_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<CodexStartOAuthLoginInput>(params, "input")?;
    ide_chat_serialize(codex_start_oauth_login(input).await?)
}

async fn ide_chat_codex_get_rate_limits_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<CodexGetRateLimitsInput>(params, "input")?;
    ide_chat_serialize(codex_get_rate_limits(input).await?)
}

fn ide_chat_codex_logout_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<CodexLogoutInput>(params, "input")?;
    ide_chat_serialize(codex_logout(input)?)
}

async fn ide_chat_remote_im_get_channel_status_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let channel_id = match params {
        Value::Object(mut map) => map
            .remove("channelId")
            .or_else(|| map.remove("channel_id"))
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_default(),
        _ => String::new(),
    };
    let config = state_read_config_cached(state).map_err(|e| format!("{e:?}"))?;
    if let Some(channel) = config
        .remote_im_channels
        .iter()
        .find(|ch| ch.id == channel_id)
    {
        let status = match channel.platform {
            RemoteImPlatform::OnebotV11 => get_channel_connection_status(channel_id).await?,
            RemoteImPlatform::Dingtalk => dingtalk_stream_manager()
                .get_channel_status(&channel.id)
                .await,
            RemoteImPlatform::Feishu => ChannelConnectionStatus {
                channel_id: channel.id.clone(),
                connected: false,
                peer_addr: None,
                connected_at: None,
                listen_addr: String::new(),
                status_text: None,
                last_error: None,
                account_id: None,
                base_url: None,
                login_session_key: None,
                qrcode_url: None,
            },
            RemoteImPlatform::WeixinOc => weixin_oc_manager().build_status(&channel.id).await,
        };
        return ide_chat_serialize(status);
    }
    ide_chat_serialize(get_channel_connection_status(channel_id).await?)
}

async fn remote_im_restart_channel_inner(
    channel_id: String,
    state: &AppState,
) -> Result<ChannelConnectionStatus, String> {
    let channel_id = channel_id.trim().to_string();
    if channel_id.is_empty() {
        return Err("channelId 为必填项。".to_string());
    }
    eprintln!("[远程IM] 重启渠道: {}", channel_id);
    onebot_v11_ws_manager()
        .add_log(&channel_id, "info", "[远程IM] 收到渠道重启请求")
        .await;
    let config = state_read_config_cached(state)?;
    let channel = config
        .remote_im_channels
        .iter()
        .find(|ch| ch.id == channel_id)
        .ok_or_else(|| format!("渠道 {} 未找到", channel_id))?
        .clone();
    onebot_v11_ws_manager()
        .add_log(
            &channel_id,
            "info",
            &format!(
                "[远程IM] 当前渠道配置: enabled={}, platform={:?}",
                channel.enabled, channel.platform
            ),
        )
        .await;

    let effective_channel = remote_im_channel_with_effective_credentials(state, &channel)?;
    let manager = onebot_v11_ws_manager();
    manager
        .reconcile_channel_runtime(&effective_channel)
        .await
        .map_err(|err| format!("重启渠道失败: {}", err))?;
    eprintln!(
        "[远程IM] 渠道 {} 已按配置收敛: enabled={}, platform={:?}",
        channel_id, channel.enabled, channel.platform
    );

    if channel.enabled && channel.platform == RemoteImPlatform::OnebotV11 {
        manager
            .start_event_consumer(channel_id.clone(), state.clone())
            .await
            .map_err(|err| format!("重启事件消费器失败: {}", err))?;
    } else if channel.enabled && channel.platform == RemoteImPlatform::Dingtalk {
        let state_clone = state.clone();
        let manager = dingtalk_stream_manager();
        let channel_clone = remote_im_channel_with_effective_credentials(&state_clone, &channel)?;
        tauri::async_runtime::spawn(async move {
            if let Err(err) = manager
                .reconcile_channel_runtime(&channel_clone, state_clone)
                .await
            {
                eprintln!(
                    "[远程IM] 钉钉渠道收敛失败: channel_id={}, platform={:?}, error={}",
                    channel_clone.id, channel_clone.platform, err
                );
            }
        });
    } else if channel.platform == RemoteImPlatform::WeixinOc {
        weixin_oc_manager()
            .reconcile_channel_runtime(&effective_channel, state.clone())
            .await?;
    }

    if channel.platform == RemoteImPlatform::Dingtalk {
        Ok(dingtalk_stream_manager()
            .get_channel_status(&channel_id)
            .await)
    } else if channel.platform == RemoteImPlatform::WeixinOc {
        Ok(weixin_oc_manager().build_status(&channel_id).await)
    } else {
        Ok(manager.get_connection_status(&channel_id).await)
    }
}

async fn ide_chat_remote_im_restart_channel_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let channel_id = match params {
        Value::Object(mut map) => map
            .remove("channelId")
            .or_else(|| map.remove("channel_id"))
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_default(),
        _ => String::new(),
    };
    ide_chat_serialize(remote_im_restart_channel_inner(channel_id, state).await?)
}

async fn ide_chat_remote_im_get_channel_logs_for_web_settings(params: Value) -> Result<Value, String> {
    let channel_id = match params {
        Value::Object(mut map) => map
            .remove("channelId")
            .or_else(|| map.remove("channel_id"))
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_default(),
        _ => String::new(),
    };
    ide_chat_serialize(get_channel_logs(channel_id).await?)
}

async fn ide_chat_remote_im_get_contact_logs_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactLogsInput>(params, "input")?;
    let (channel_id, contact_marker) =
        remote_im_resolve_contact_log_query(state, &input.contact_id)?;
    let logs = get_channel_logs(channel_id).await?;
    ide_chat_serialize(remote_im_filter_channel_logs_for_contact(logs, &contact_marker))
}

fn ide_chat_remote_im_list_channels_for_web_settings(state: &AppState) -> Result<Value, String> {
    let config = state_read_config_cached(state)?;
    ide_chat_serialize(config.remote_im_channels)
}

fn ide_chat_remote_im_list_contacts_for_web_settings(state: &AppState) -> Result<Value, String> {
    let runtime = state_read_runtime_state_cached(state)?;
    let mut contacts = runtime.remote_im_contacts;
    contacts.sort_by(|a, b| {
        a.channel_id
            .cmp(&b.channel_id)
            .then_with(|| b.last_message_at.cmp(&a.last_message_at))
            .then_with(|| a.id.cmp(&b.id))
    });
    ide_chat_serialize(contacts)
}

fn ide_chat_remote_im_update_contact_allow_send_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactAllowSendUpdateInput>(params, "input")?;
    let mut runtime = state_read_runtime_state_cached(state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.allow_send = input.allow_send;
    contact.allow_receive = input.allow_send;
    let output = contact.clone();
    state_write_runtime_state_cached(state, &runtime)?;
    ide_chat_serialize(output)
}

fn ide_chat_remote_im_update_contact_allow_send_files_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactAllowSendFilesUpdateInput>(params, "input")?;
    let mut runtime = state_read_runtime_state_cached(state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.allow_send_files = input.allow_send_files;
    let output = contact.clone();
    state_write_runtime_state_cached(state, &runtime)?;
    ide_chat_serialize(output)
}

fn ide_chat_remote_im_update_contact_activation_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactActivationUpdateInput>(params, "input")?;
    let mut runtime = state_read_runtime_state_cached(state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.activation_mode = normalize_contact_activation_mode(&input.activation_mode);
    contact.activation_keywords = normalize_contact_activation_keywords(&input.activation_keywords);
    contact.mute_keywords = normalize_contact_keyword_list(&input.mute_keywords);
    contact.unmute_keywords = normalize_contact_keyword_list(&input.unmute_keywords);
    contact.patience_seconds = input.patience_seconds;
    contact.mute_duration_seconds = input.mute_duration_seconds;
    contact.activation_cooldown_seconds = input.activation_cooldown_seconds;
    contact.response_strategy = normalize_contact_response_strategy(&input.response_strategy);
    contact.response_guidance = normalize_contact_response_guidance(&input.response_guidance);
    let output = contact.clone();
    state_write_runtime_state_cached(state, &runtime)?;
    ide_chat_serialize(output)
}

fn ide_chat_remote_im_update_contact_department_binding_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactDepartmentBindingUpdateInput>(params, "input")?;
    let runtime_snapshot = load_runtime_organization_snapshot(state)?;
    let mut runtime = state_read_runtime_state_cached(state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    let next_department_id = input
        .department_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let next_agent_id = input
        .agent_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    if next_department_id.is_some() != next_agent_id.is_some() {
        return Err("远程IM绑定部门和人格必须同时提供".to_string());
    }
    let next_pair = if let Some(department_id) = next_department_id.as_deref() {
        let pair = resolve_department_agent_pair(
            Some(department_id),
            next_agent_id.as_deref(),
            &runtime_snapshot.config,
        )?;
        if !runtime_snapshot
            .agents
            .iter()
            .any(|agent| agent.id == pair.1 && !agent.is_built_in_user)
        {
            return Err(format!("路由人格不存在或不可用: {}", pair.1));
        }
        Some(pair)
    } else {
        None
    };
    contact.bound_department_id = next_pair
        .as_ref()
        .map(|(department_id, _)| department_id.clone());
    contact.bound_agent_id = next_pair.as_ref().map(|(_, agent_id)| agent_id.clone());
    contact.route_mode =
        remote_im_resolve_effective_route_mode(&runtime_snapshot.config, contact);
    let conversation_id = ensure_remote_im_contact_conversation_id(state, contact)?;
    let output = contact.clone();
    state_write_runtime_state_cached(state, &runtime)?;
    eprintln!(
        "[远程IM] 完成，任务=更新联系人处理部门，contact_id={}，conversation_id={}",
        output.id,
        conversation_id
    );
    ide_chat_serialize(output)
}

fn ide_chat_remote_im_update_contact_processing_mode_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactProcessingModeUpdateInput>(params, "input")?;
    let mut runtime = state_read_runtime_state_cached(state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.processing_mode = normalize_contact_processing_mode(&input.processing_mode);
    let output = contact.clone();
    state_write_runtime_state_cached(state, &runtime)?;
    ide_chat_serialize(output)
}

fn ide_chat_remote_im_update_contact_workspace_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactWorkspaceUpdateInput>(params, "input")?;
    let mut runtime = state_read_runtime_state_cached(state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.shell_workspaces = input.shell_workspaces;
    let output = contact.clone();
    state_write_runtime_state_cached(state, &runtime)?;
    ide_chat_serialize(output)
}

fn ide_chat_remote_im_delete_contact_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactDeleteInput>(params, "input")?;
    let contact_id = input.contact_id.trim();
    if contact_id.is_empty() {
        return Err("contact_id 为必填项。".to_string());
    }
    let mut runtime = state_read_runtime_state_cached(state)?;
    let before_contacts = runtime.remote_im_contacts.len();
    runtime.remote_im_contacts.retain(|item| item.id != contact_id);
    let removed = runtime.remote_im_contacts.len() != before_contacts;
    if removed {
        state_write_runtime_state_cached(state, &runtime)?;
    }
    ide_chat_serialize(removed)
}

async fn ide_chat_remote_im_weixin_oc_start_login_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<WeixinOcLoginStartInput>(params, "input")?;
    ide_chat_serialize(weixin_oc_manager().start_login(state, input).await?)
}

async fn ide_chat_remote_im_weixin_oc_get_login_status_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<WeixinOcLoginStatusInput>(params, "input")?;
    ide_chat_serialize(weixin_oc_manager().poll_login_status(state, input).await?)
}

async fn ide_chat_remote_im_weixin_oc_logout_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<WeixinOcLoginStatusInput>(params, "input")?;
    weixin_oc_manager()
        .logout(state, input.channel_id.as_str())
        .await?;
    ide_chat_serialize(true)
}

async fn ide_chat_remote_im_weixin_oc_sync_contacts_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<WeixinOcLoginStatusInput>(params, "input")?;
    let config = state_read_config_cached(state)?;
    let channel = remote_im_channel_by_id(&config, &input.channel_id)
        .ok_or_else(|| format!("渠道不存在: {}", input.channel_id))?;
    if channel.platform != RemoteImPlatform::WeixinOc {
        return Err("该渠道不是个人微信渠道".to_string());
    }
    let credentials = remote_im_effective_credentials(state, channel)?;
    let creds = WeixinOcCredentials::from_value(&credentials);
    if creds.account_id.trim().is_empty() || creds.token.trim().is_empty() {
        return ide_chat_serialize(WeixinOcSyncContactsResult {
            channel_id: input.channel_id,
            synced_count: 0,
            message: "当前还没有完成扫码登录，请先登录后再同步联系人。".to_string(),
        });
    }
    let user_id = creds.user_id.trim().to_string();
    let (_, created) = sync_weixin_oc_contact_from_user_id(state, &channel, &user_id)?;
    ide_chat_serialize(WeixinOcSyncContactsResult {
        channel_id: input.channel_id,
        synced_count: 1,
        message: if created {
            format!("已同步个人微信联系人：{}", user_id)
        } else {
            format!("联系人已存在，无需重复同步：{}", user_id)
        },
    })
}

fn ide_chat_runtime_for_conversation(
    state: &AppState,
    conversation_id: &str,
) -> Option<ConversationRuntimeSnapshot> {
    read_conversation_runtime_snapshot(state, conversation_id).ok()
}

fn ide_chat_sidebar_window_label(client_id: &str) -> String {
    format!("vscode-sidebar:{}", client_id.trim())
}

fn ide_chat_emit_overview_updated(state: &AppState) -> Result<(), String> {
    let overview_payload = conversation_service().refresh_unarchived_conversation_overview_payload(state)?;
    emit_unarchived_conversation_overview_updated_payload(state, &overview_payload);
    Ok(())
}

fn ide_chat_release_sidebar_conversation(
    state: &AppState,
    sidebar_label: &str,
) -> Result<(), String> {
    if unregister_detached_chat_window_by_label(sidebar_label).is_some() {
        ide_chat_emit_overview_updated(state)?;
    }
    Ok(())
}

fn ide_chat_register_sidebar_conversation(
    state: &AppState,
    conversation_id: &str,
    sidebar_label: &str,
    opened_conversation_id: &mut Option<String>,
) -> Result<(), String> {
    let conversation_id = conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let conversation = state_read_conversation_cached(state, conversation_id)?;
    if conversation_is_system_notification(&conversation) {
        if opened_conversation_id.as_deref() != Some(conversation_id) {
            ide_chat_release_sidebar_conversation(state, sidebar_label)?;
        }
        *opened_conversation_id = Some(conversation_id.to_string());
        return Ok(());
    }
    if let Some(existing_label) = detached_chat_window_for_conversation(conversation_id) {
        if existing_label != sidebar_label {
            return Err("会话已在其他窗口打开。".to_string());
        }
    }
    if opened_conversation_id.as_deref() != Some(conversation_id) {
        ide_chat_release_sidebar_conversation(state, sidebar_label)?;
    }
    register_detached_chat_window(conversation_id, sidebar_label)?;
    *opened_conversation_id = Some(conversation_id.to_string());
    ide_chat_emit_overview_updated(state)?;
    Ok(())
}

fn ide_chat_conversation_open_result(state: &AppState, conversation_id: &str) -> Result<Value, String> {
    let conversation_id = conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let conversation = state_read_conversation_cached(state, conversation_id)?;
    if !conversation.summary.trim().is_empty() {
        return Err("conversation is archived".to_string());
    }
    let messages = conversation_service().read_recent_unarchived_block_messages(state, conversation_id)?;
    let runtime = ide_chat_runtime_for_conversation(state, conversation_id);
    let persona = ide_chat_persona_payload(state, Some(conversation.agent_id.as_str()))?;
    let model = ide_chat_model_payload_for_conversation(state, &conversation)?;
    Ok(serde_json::json!({
        "conversationId": conversation.id,
        "title": conversation.title,
        "agentId": conversation.agent_id,
        "departmentId": conversation.department_id,
        "updatedAt": conversation.updated_at,
        "messages": messages,
        "runtime": runtime,
        "persona": persona,
        "model": model,
        "currentTodos": conversation.current_todos,
        "activeGoal": goal_active_goal_from_conversation(&conversation),
    }))
}

fn ide_chat_ensure_sidebar_workspace(
    state: &AppState,
    conversation_id: &str,
    workspace_path: &str,
    workspace_name: Option<&str>,
) -> Result<(), String> {
    let conversation = state_read_conversation_cached(state, conversation_id)?;
    let mut workspaces = conversation.shell_workspaces.clone();
    let has_main = workspaces.iter().any(|ws| {
        normalize_shell_workspace_level_text(&ws.level) == SHELL_WORKSPACE_LEVEL_MAIN
    });
    if has_main {
        return Ok(());
    }
    let name = workspace_name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(String::from)
        .unwrap_or_else(|| {
            std::path::Path::new(workspace_path)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| workspace_path.to_string())
        });
    workspaces.push(ShellWorkspaceConfig {
        id: "vscode-sidebar-main-workspace".to_string(),
        name: name.to_string(),
        path: workspace_path.to_string(),
        level: SHELL_WORKSPACE_LEVEL_MAIN.to_string(),
        access: SHELL_WORKSPACE_ACCESS_APPROVAL.to_string(),
        built_in: false,
    });
    let normalized_workspaces = normalize_conversation_shell_workspaces(state, &workspaces);
    apply_conversation_chat_workspace_changes(
        state,
        conversation_id,
        Some(None),
        Some(normalized_workspaces),
        None,
    )?;
    Ok(())
}

fn ide_chat_conversation_list(state: &AppState, current_viewer_id: &str) -> Result<Value, String> {
    let viewer_id = current_viewer_id.trim();
    let summaries = conversation_service()
        .list_unarchived_conversation_summaries(state)?
        .summaries
        .into_iter()
        .map(|mut item| {
            item.runtime_state = ide_chat_runtime_for_conversation(state, &item.conversation_id)
                .map(|snapshot| snapshot.runtime_state);
            item.state.current_viewer_id = Some(viewer_id.to_string());
            item
        })
        .collect::<Vec<_>>();
    let remote_im_contact_conversations = conversation_service().list_remote_im_contact_conversations(state)?;
    let persona = ide_chat_persona_payload(state, None)?;
    Ok(serde_json::json!({
        "conversations": summaries,
        "unarchivedConversations": summaries,
        "remoteImContactConversations": remote_im_contact_conversations,
        "persona": persona,
        "viewerId": viewer_id,
    }))
}

fn ide_chat_conversation_block_page(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatConversationBlockPageInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let page = conversation_service().read_unarchived_block_page(state, conversation_id, input.block_id)?;
    Ok(serde_json::json!({
        "blocks": page.blocks.into_iter().map(|item| {
            serde_json::json!({
                "blockId": item.block_id,
                "messageCount": item.message_count,
                "firstMessageId": item.first_message_id,
                "lastMessageId": item.last_message_id,
                "firstCreatedAt": item.first_created_at,
                "lastCreatedAt": item.last_created_at,
                "isLatest": item.is_latest,
            })
        }).collect::<Vec<_>>(),
        "selectedBlockId": page.selected_block_id,
        "messages": page.messages,
        "hasPrevBlock": page.has_prev_block,
        "hasNextBlock": page.has_next_block,
    }))
}

fn ide_chat_create_conversation(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatCreateConversationInput>(params)?;
    let result = conversation_service().create_unarchived_conversation(
        state,
        &CreateUnarchivedConversationInput {
            api_config_id: None,
            agent_id: input.agent_id,
            department_id: input.department_id,
            title: input.title,
            copy_source_conversation_id: None,
            shell_workspaces: None,
            shell_autonomous_mode: None,
        },
    )?;
    emit_unarchived_conversation_overview_updated_payload(state, &result.overview_payload);
    let conversation = ide_chat_conversation_open_result(state, &result.conversation_id)?;
    Ok(serde_json::json!({
        "conversationId": result.conversation_id,
        "unarchivedConversations": result.overview_payload.unarchived_conversations,
        "conversation": conversation,
    }))
}

fn ide_chat_delete_conversation(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatConversationInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let result = conversation_service().delete_unarchived_conversation(state, conversation_id)?;
    let _ = delegate_runtime_thread_conversation_delete_by_root(state, conversation_id);
    let overview_payload = conversation_service().refresh_unarchived_conversation_overview_payload(state)?;
    emit_unarchived_conversation_overview_updated_payload(state, &overview_payload);
    Ok(serde_json::json!({
        "deletedConversationId": result.deleted_conversation_id,
        "preferredConversationId": overview_payload.preferred_conversation_id,
        "unarchivedConversations": overview_payload.unarchived_conversations,
    }))
}

fn ide_chat_send_message(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatSendInput>(params)?;
    let conversation_id = input.conversation_id.trim().to_string();
    let text = input.text.trim().to_string();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    if text.is_empty()
        && input.extra_text_blocks.iter().all(|item| item.trim().is_empty())
        && input
            .images
            .iter()
            .all(|item| item.bytes_base64.trim().is_empty())
    {
        return Err("消息内容为空".to_string());
    }
    let conversation = state_read_conversation_cached(state, &conversation_id)?;
    let agent_id = conversation.agent_id.trim().to_string();
    if agent_id.is_empty() {
        return Err("会话信息不完整".to_string());
    }
    let department_id = conversation.department_id.trim().to_string();
    if department_id.is_empty() {
        return Err("会话部门为空，无法从侧边栏发送。".to_string());
    }
    let request_id = runtime_context_request_id_or_new(None, None, "vscode-sidebar");
    let mut parts = if text.is_empty() {
        Vec::new()
    } else {
        vec![MessagePart::Text { text: text.clone(),
                reasoning_content: None,
            }]
    };
    for image in input.images {
        let mime = image.mime.trim().to_ascii_lowercase();
        let bytes_base64 = image.bytes_base64.trim().to_string();
        if !mime.starts_with("image/") || bytes_base64.is_empty() {
            continue;
        }
        parts.push(MessagePart::Image {
            mime,
            bytes_base64,
            name: image.name.and_then(|value| {
                let trimmed = value.trim().to_string();
                if trimmed.is_empty() { None } else { Some(trimmed) }
            }),
            compressed: false,
        });
    }
    if parts.is_empty() && input.extra_text_blocks.iter().all(|item| item.trim().is_empty()) {
        return Err("消息内容为空".to_string());
    }
    let user_message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "user".to_string(),
        created_at: now_iso(),
        speaker_agent_id: None,
        parts,
        extra_text_blocks: input
            .extra_text_blocks
            .into_iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect(),
        provider_meta: Some(serde_json::json!({
            "requestId": request_id,
            "source": "vscode_sidebar",
        })),
        tool_call: None,
        mcp_call: None,
        meme_annotations: None,
    };
    let event_id = Uuid::new_v4().to_string();
    let mut runtime_context = runtime_context_new("user_message", "user_send");
    runtime_context.request_id = Some(request_id.clone());
    runtime_context.dispatch_id = Some(event_id.clone());
    runtime_context.origin_conversation_id = Some(conversation_id.clone());
    runtime_context.target_conversation_id = Some(conversation_id.clone());
    runtime_context.root_conversation_id = Some(conversation_id.clone());
    runtime_context.executor_agent_id = Some(agent_id.clone());
    runtime_context.executor_department_id = Some(department_id.clone());
    let event = ChatPendingEvent {
        id: event_id.clone(),
        conversation_id: conversation_id.clone(),
        created_at: now_iso(),
        source: ChatEventSource::User,
        queue_mode: ChatQueueMode::Normal,
        messages: vec![user_message],
        activate_assistant: true,
        session_info: ChatSessionInfo {
            department_id,
            agent_id,
        },
        runtime_context: Some(runtime_context),
        sender_info: None,
    };
    let ingress = ingress_chat_event(state, event)?;
    let queued = matches!(ingress, ChatEventIngress::Queued { .. });
    trigger_chat_event_after_ingress(state, ingress);
    Ok(serde_json::json!({
        "conversationId": conversation_id,
        "eventId": event_id,
        "requestId": request_id,
        "queued": queued,
    }))
}

fn ide_chat_stop_conversation(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatStopInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let conversation = state_read_conversation_cached(state, conversation_id)?;
    let (department_id, _agent_id) = resolve_runtime_control_department_and_agent(
        state,
        Some(conversation.department_id.as_str()),
        Some(conversation.agent_id.as_str()),
        Some(conversation_id),
    )?;
    let chat_key = inflight_chat_key(&department_id, Some(conversation_id));
    let aborted_chat = {
        let mut inflight = state
            .inflight_chat_abort_handles
            .lock()
            .map_err(|_| "Failed to lock inflight chat abort handles".to_string())?;
        inflight.remove(&chat_key).map(|handle| {
            handle.abort();
            true
        }).unwrap_or(false)
    };
    let aborted_tool = abort_inflight_tool_abort_handle(state, &chat_key)?;
    let aborted_delegate_children =
        abort_delegate_runtime_descendants_by_parent_context(state, &chat_key, Some(conversation_id))?;
    let cleared_queue_count = clear_conversation_queue(
        state,
        conversation_id,
        "消息已因 VS Code 侧边栏中断被清出队列",
    )?;
    let _ = release_conversation_processing_claim(state, conversation_id);
    let _ = set_conversation_runtime_state(state, conversation_id, MainSessionState::Idle);
    let _ = set_conversation_remote_im_activation_sources(state, conversation_id, Vec::new());
    let partial_stream_text = assistant_text_from_stream_blocks(&input.partial_stream_blocks);
    let partial_assistant_text = input.partial_assistant_text.trim().to_string();
    let partial_assistant_text = if partial_assistant_text.is_empty() {
        partial_stream_text.trim().to_string()
    } else {
        partial_assistant_text
    };
    let partial_activity_text = reasoning_text_from_stream_blocks(&input.partial_stream_blocks);
    let completed_tool_history = inflight_completed_tool_history(state, &chat_key)?;
    let partial_tool_history =
        merge_stream_block_tool_history(&completed_tool_history, &input.partial_stream_blocks);
    runtime_log_info(format!(
        "[聊天流式块][侧边栏停止] 停止请求完成 session={} conversation_id={} partial_text_len={} partial_reasoning_len={} partial_block_count={} partial_tool_history_count={} completed_tool_history_count={}",
        chat_key,
        conversation_id,
        partial_assistant_text.chars().count(),
        partial_activity_text.chars().count(),
        input.partial_stream_blocks.len(),
        partial_tool_history.len(),
        completed_tool_history.len(),
    ));
    clear_inflight_completed_tool_history(state, &chat_key)?;
    let stop_result = StopChatResult {
        aborted: aborted_chat || aborted_tool || aborted_delegate_children > 0,
        persisted: false,
        conversation_id: Some(conversation_id.to_string()),
        assistant_text: partial_assistant_text,
        assistant_message: None,
    };
    let payload = serde_json::json!({
        "conversationId": conversation_id,
        "status": "stopped",
        "aborted": stop_result.aborted,
        "persisted": stop_result.persisted,
        "clearedQueueCount": cleared_queue_count,
        "assistantText": stop_result.assistant_text,
        "assistantMessage": stop_result.assistant_message,
    });
    if !stop_result.persisted {
        ide_chat_broadcast_notification("chat.roundFinished", payload.clone());
    }
    Ok(payload)
}

fn ide_chat_session_for_conversation(state: &AppState, conversation_id: &str) -> Result<SessionSelector, String> {
    let conversation_id = conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let conversation = state_read_conversation_cached(state, conversation_id)?;
    let agent_id = conversation.agent_id.trim().to_string();
    if agent_id.is_empty() {
        return Err("会话信息不完整".to_string());
    }
    let department_id = conversation.department_id.trim().to_string();
    Ok(SessionSelector {
        api_config_id: None,
        department_id: (!department_id.is_empty()).then_some(department_id),
        agent_id,
        conversation_id: Some(conversation_id.to_string()),
    })
}

async fn ide_chat_rewind_conversation(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatRewindInput>(params)?;
    let conversation_id = input.conversation_id.trim().to_string();
    let message_id = input.message_id.trim().to_string();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    if message_id.is_empty() {
        return Err("messageId is required".to_string());
    }

    let started_at = std::time::Instant::now();
    let session = ide_chat_session_for_conversation(state, &conversation_id)?;
    let request = RewindConversationInput {
        session,
        message_id: message_id.clone(),
        undo_apply_patch: input.undo_apply_patch,
    };
    let result = conversation_service().rewind_conversation_from_message(
        state,
        &request,
        &message_id,
        &started_at,
    )?;
    if result.removed_count > 0 {
        emit_conversation_todos_updated_payload(
            state,
            &ConversationTodosUpdatedPayload {
                conversation_id: result.conversation_id.clone(),
                current_todo: result.current_todo.clone(),
                current_todos: result.current_todos.clone(),
            },
        );
        ide_chat_emit_overview_updated(state)?;
    }
    let mut recalled_user_message = result.recalled_user_message;
    if let Some(message) = recalled_user_message.as_mut() {
        materialize_message_parts_from_media_refs(&mut message.parts, &state.data_path);
    }
    let conversation = ide_chat_conversation_open_result(state, &conversation_id)?;
    Ok(serde_json::json!({
        "conversationId": conversation_id,
        "removedCount": result.removed_count,
        "remainingCount": result.remaining_count,
        "recalledUserMessage": recalled_user_message,
        "conversation": conversation,
    }))
}

async fn ide_chat_rewind_preview(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatRewindInput>(params)?;
    let conversation_id = input.conversation_id.trim().to_string();
    let message_id = input.message_id.trim().to_string();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    if message_id.is_empty() {
        return Err("messageId is required".to_string());
    }

    let started_at = std::time::Instant::now();
    runtime_log_info(format!(
        "[会话撤回] 开始，任务=ide_chat_rewind_preview，conversation_id={}，message_id={}",
        conversation_id,
        message_id
    ));
    let session = ide_chat_session_for_conversation(state, &conversation_id)?;
    let request = RewindConversationInput {
        session,
        message_id: message_id.clone(),
        undo_apply_patch: false,
    };
    let result = conversation_service().preview_rewind_conversation_from_message(
        state,
        &request,
        &message_id,
    )?;
    runtime_log_info(format!(
        "[会话撤回] 完成，任务=ide_chat_rewind_preview，conversation_id={}，can_undo_patch={}，duration_ms={}",
        result.conversation_id,
        result.can_undo_patch,
        started_at.elapsed().as_millis()
    ));
    Ok(serde_json::json!({
        "conversationId": result.conversation_id,
        "canUndoPatch": result.can_undo_patch,
        "hint": result.hint,
    }))
}

fn ide_chat_compact_preview(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatConversationInput>(params)?;
    let session = ide_chat_session_for_conversation(state, &input.conversation_id)?;
    let (selected_api, _resolved_api, source, _effective_agent_id) =
        resolve_archive_target_conversation(state, &session)?;
    let preview = build_trim_compaction_preview_result(state, &selected_api, &source)?;
    Ok(serde_json::to_value(preview).map_err(|err| format!("serialize compact preview failed: {err}"))?)
}

async fn ide_chat_compact_conversation(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatConversationInput>(params)?;
    let session = ide_chat_session_for_conversation(state, &input.conversation_id)?;
    let (selected_api, resolved_api, source, effective_agent_id) =
        resolve_archive_target_conversation(state, &session)?;
    let preview = build_trim_compaction_preview_result(state, &selected_api, &source)?;
    if !preview.can_compact {
        return Err(preview
            .compaction_disabled_reason
            .unwrap_or_else(|| "当前会话暂时不能压缩。".to_string()));
    }
    let result = run_context_compaction_pipeline(
        state,
        &selected_api,
        &resolved_api,
        &source,
        &effective_agent_id,
        "manual_trim_compaction",
        "COMPACTION-FORCE",
        &[],
        false,
    )
    .await?;
    trigger_chat_queue_processing(state);
    let overview_payload = conversation_service().refresh_unarchived_conversation_overview_payload(state)?;
    emit_unarchived_conversation_overview_updated_payload(state, &overview_payload);
    if let Some(compaction_message) = result.compaction_message.clone() {
        ide_chat_broadcast_notification(
            "conversation.messageAppended",
            serde_json::json!({
                "conversationId": source.id,
                "message": compaction_message,
            }),
        );
    }
    Ok(serde_json::to_value(result).map_err(|err| format!("serialize compact result failed: {err}"))?)
}

fn ide_chat_model_list(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatConversationInput>(params)?;
    let conversation = state_read_conversation_cached(state, input.conversation_id.trim())?;
    ide_chat_model_payload_for_conversation(state, &conversation)
}

fn ide_chat_select_model(state: &AppState, _app: &AppHandle, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatSelectModelInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let api_config_id = input.api_config_id.trim();
    runtime_log_info(format!(
        "[会话模型] 开始，任务=切换会话首选模型，入口=vscode_sidebar，会话ID={}，api_config_id={}",
        conversation_id,
        if api_config_id.is_empty() { "部门模型" } else { api_config_id }
    ));
    let preferred_api_config_id = if api_config_id.is_empty() {
        None
    } else {
        let config = state_read_config_cached(state)?;
        let resolved_api_config_id = resolve_model_role_api_config_id(&config, api_config_id)
            .ok_or_else(|| format!("Model role '{api_config_id}' is not configured."))?;
        let selected_api = config
            .api_configs
            .iter()
            .find(|item| item.id.trim() == resolved_api_config_id)
            .ok_or_else(|| format!("API config '{api_config_id}' not found."))?;
        if !is_text_chat_api(selected_api) {
            return Err(format!("API config '{api_config_id}' does not support chat text."));
        }
        Some(resolved_api_config_id)
    };
    conversation_service().set_conversation_preferred_api_config_id(
        state,
        conversation_id,
        preferred_api_config_id,
    )?;
    let overview_payload = conversation_service().refresh_unarchived_conversation_overview_payload(state)?;
    emit_unarchived_conversation_overview_updated_payload(state, &overview_payload);
    let updated_conversation = state_read_conversation_cached(state, conversation_id)?;
    runtime_log_info(format!(
        "[会话模型] 完成，任务=切换会话首选模型，入口=vscode_sidebar，会话ID={}，api_config_id={}",
        conversation_id,
        updated_conversation
            .preferred_api_config_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("部门模型")
    ));
    ide_chat_model_payload_for_conversation(state, &updated_conversation)
}

fn ide_chat_open_settings(app: &AppHandle) -> Result<Value, String> {
    show_window(app, "main")?;
    Ok(serde_json::json!({ "opened": true }))
}

fn ide_chat_resolve_terminal_approval(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatResolveTerminalApprovalInput>(params)?;
    let resolved = resolve_terminal_approval_request(
        state,
        input.request_id.trim(),
        input.approved,
    )?;
    Ok(serde_json::json!({ "resolved": resolved }))
}

fn ide_chat_set_conversation_plan_mode(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<SetConversationPlanModeInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId 不能为空".to_string());
    }
    let current_enabled =
        get_conversation_plan_mode_enabled(state, conversation_id).unwrap_or(false);
    if current_enabled != input.plan_mode_enabled {
        set_conversation_plan_mode_enabled(state, conversation_id, input.plan_mode_enabled)?;
        runtime_log_info(format!(
            "[计划模式] 完成，任务=VSCode边栏切换会话运行时计划模式，会话ID={}，状态={}",
            conversation_id,
            if input.plan_mode_enabled { "开启" } else { "关闭" }
        ));
    }
    Ok(serde_json::json!({
        "conversationId": conversation_id,
        "planModeEnabled": input.plan_mode_enabled,
    }))
}

async fn ide_chat_confirm_plan(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ConfirmPlanAndContinueInput>(params)?;
    let continued = confirm_plan_and_continue_inner(state, &input).await?;
    Ok(serde_json::json!({ "continued": continued }))
}

fn ide_chat_read_plan_file(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatReadPlanFileInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required.".to_string());
    }
    let resolved = resolve_plan_file_for_conversation_id(state, conversation_id, input.path.trim())?;
    let content = read_plan_markdown_file(&resolved.canonical_path)?;
    Ok(serde_json::json!({ "content": content }))
}

fn ide_chat_tool_review_reports(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ToolReviewConversationInput>(params)?;
    serde_json::to_value(list_tool_review_reports_internal(input, state)?)
        .map_err(|err| format!("Serialize tool review reports failed: {err}"))
}

fn ide_chat_tool_review_delete_report(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<DeleteToolReviewReportInput>(params)?;
    delete_tool_review_report_internal(input, state)?;
    Ok(serde_json::json!({ "deleted": true }))
}

async fn ide_chat_tool_review_commit_options(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ToolReviewCommitPageInput>(params)?;
    serde_json::to_value(list_tool_review_commit_options_internal_command(input, state).await?)
        .map_err(|err| format!("Serialize tool review commit options failed: {err}"))
}

async fn ide_chat_tool_review_submit_code(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ToolReviewCodeReviewInput>(params)?;
    serde_json::to_value(submit_tool_review_code_internal(input, state).await?)
        .map_err(|err| format!("Serialize tool review submit result failed: {err}"))
}

fn ide_chat_tool_review_batches(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ToolReviewConversationInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return serde_json::to_value(ListToolReviewBatchesOutput {
            batches: Vec::new(),
            current_batch_key: None,
        })
        .map_err(|err| format!("Serialize tool review batches failed: {err}"));
    }
    let (batches, current_batch_key) = with_tool_review_conversation(state, conversation_id, |conversation| {
        let batches = collect_tool_review_batches_internal(conversation);
        let current_batch_key = conversation
            .messages
            .iter()
            .rev()
            .find(|message| message.role.trim().eq_ignore_ascii_case("user"))
            .map(|message| message.id.clone());
        Ok((batches, current_batch_key))
    })?;
    serde_json::to_value(ListToolReviewBatchesOutput {
        current_batch_key,
        batches: batches
            .iter()
            .map(tool_review_batch_summary_from_collected)
            .collect(),
    })
    .map_err(|err| format!("Serialize tool review batches failed: {err}"))
}

fn ide_chat_tool_review_item_detail(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ToolReviewCallInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    let call_id = input.call_id.trim();
    if conversation_id.is_empty() || call_id.is_empty() {
        return Err("conversationId 和 callId 不能为空。".to_string());
    }
    let detail = with_tool_review_conversation(state, conversation_id, |conversation| {
        let item = tool_review_find_item(conversation, call_id)?;
        Ok(tool_review_item_detail_from_collected(&item))
    })?;
    serde_json::to_value(detail)
        .map_err(|err| format!("Serialize tool review item detail failed: {err}"))
}

async fn ide_chat_tool_review_item_review(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ToolReviewCallInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    let call_id = input.call_id.trim();
    if conversation_id.is_empty() || call_id.is_empty() {
        return Err("conversationId 和 callId 不能为空。".to_string());
    }
    serde_json::to_value(tool_review_run_for_call_internal(state, conversation_id, call_id).await?)
        .map_err(|err| format!("Serialize tool review item result failed: {err}"))
}

fn ide_chat_tool_review_item_decision(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ToolReviewSetUserDecisionInput>(params)?;
    let conversation_id = input.conversation_id.trim().to_string();
    let call_id = input.call_id.trim().to_string();
    if conversation_id.is_empty() || call_id.is_empty() {
        return Err("conversationId 和 callId 不能为空。".to_string());
    }
    let opinion = input.opinion.trim().to_string();
    let user_decision_review = serde_json::json!({
        "kind": "user_decision",
        "allow": input.allow,
        "reviewOpinion": if opinion.is_empty() {
            if input.allow { "用户已批准本次工具执行" } else { "用户已否决本次工具执行" }
        } else {
            opinion.as_str()
        },
        "userOpinion": opinion,
    });
    let detail = conversation_service().update_unarchived_conversation_by_id(
        state,
        &conversation_id,
        |conversation| {
            tool_review_write_call_review(conversation, &call_id, &user_decision_review)?;
            let refreshed = tool_review_find_item(conversation, &call_id)?;
            Ok(tool_review_item_detail_from_collected(&refreshed))
        },
    )?;
    serde_json::to_value(detail)
        .map_err(|err| format!("Serialize tool review decision result failed: {err}"))
}

async fn ide_chat_tool_review_batch_review(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ToolReviewBatchActionInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId 不能为空。".to_string());
    }
    let conversation = with_tool_review_conversation(state, conversation_id, |conversation| {
        Ok(conversation.clone())
    })?;
    let (_batch_number, batch) = tool_review_find_batch_by_index(&conversation, input.batch_index)?;
    let reviewed_call_ids = tool_review_run_missing_reviews_for_batch(state, conversation_id, &batch).await?;
    serde_json::to_value(RunToolReviewBatchOutput {
        batch_key: batch.batch_key,
        reviewed_call_ids,
    })
    .map_err(|err| format!("Serialize tool review batch result failed: {err}"))
}

async fn ide_chat_branch_conversation(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<BranchUnarchivedConversationFromSelectionInput>(params)?;
    serde_json::to_value(branch_unarchived_conversation_from_selection_internal(input, state).await?)
        .map_err(|err| format!("Serialize branch conversation result failed: {err}"))
}

async fn ide_chat_submit_delegate(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<SubmitUserAsyncDelegateInput>(params)?;
    serde_json::to_value(submit_user_async_delegate_internal(input, state).await?)
        .map_err(|err| format!("Serialize delegate submit result failed: {err}"))
}

fn ide_chat_task_create(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<TaskCreateInput>(params)?;
    let input = task_create_input_for_write(state, &input)?;
    serde_json::to_value(task_store_create_task(&state.data_path, &input)?)
        .map_err(|err| format!("Serialize task create result failed: {err}"))
}

fn ide_chat_task_update(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<TaskUpdateInput>(params)?;
    let input = task_update_input_for_write(state, &input)?;
    serde_json::to_value(task_store_update_task(&state.data_path, &input)?)
        .map_err(|err| format!("Serialize task update result failed: {err}"))
}

fn ide_chat_task_delete(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<TaskDeleteInput>(params)?;
    task_store_delete_task(&state.data_path, input.task_id.trim())?;
    Ok(serde_json::json!(true))
}

fn ide_chat_task_list(state: &AppState) -> Result<Value, String> {
    serde_json::to_value(task_store_list_tasks(&state.data_path)?)
        .map_err(|err| format!("Serialize task list result failed: {err}"))
}

async fn ide_chat_task_optimize_draft(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<TaskOptimizeDraftInput>(params)?;
    serde_json::to_value(task_optimize_draft_internal(input, state).await?)
        .map_err(|err| format!("Serialize task optimize result failed: {err}"))
}

async fn ide_chat_task_dispatch_now(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<TaskDispatchNowInput>(params)?;
    let task = task_store_get_task_record(&state.data_path, input.task_id.trim())?;
    let Some(session) = task_resolve_dispatch_session(state, &task)? else {
        task_fail_missing_bound_conversation(state, &task)?;
        return Ok(serde_json::json!(false));
    };
    task_dispatch_due_task(state, &task, &session).await?;
    Ok(serde_json::json!(true))
}

fn ide_chat_goal_current(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<GoalCancelInput>(params)?;
    serde_json::to_value(goal_get_current_inner(state, &input.conversation_id)?)
        .map_err(|err| format!("Serialize goal current result failed: {err}"))
}

fn ide_chat_goal_create(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<GoalCreateInput>(params)?;
    serde_json::to_value(goal_create_goal_inner(
        state,
        &input.conversation_id,
        &input.objective,
    )?)
    .map_err(|err| format!("Serialize goal create result failed: {err}"))
}

fn ide_chat_goal_cancel(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<GoalCancelInput>(params)?;
    serde_json::to_value(goal_cancel_goal_inner(state, &input.conversation_id)?)
        .map_err(|err| format!("Serialize goal cancel result failed: {err}"))
}

async fn ide_chat_handle_jsonrpc_request(
    request: IdeChatJsonRpcRequest,
    state: &AppState,
    app: &AppHandle,
    ide_context_runtime: &IdeContextRuntime,
    client_id: &str,
    opened_conversation_id: &mut Option<String>,
) -> Value {
    if request.jsonrpc.trim() != "2.0" {
        return ide_chat_jsonrpc_error(request.id, -32600, "jsonrpc must be 2.0");
    }
    let sidebar_label = ide_chat_sidebar_window_label(client_id);
    let sidebar_viewer_id = chat_viewer_id_for_window_label(&sidebar_label)
        .unwrap_or_else(|| format!("web:{}", client_id.trim()));
    let result = match request.method.as_str() {
        "conversation.list" => ide_chat_conversation_list(state, &sidebar_viewer_id),
        "conversation.open" => ide_chat_parse_params::<IdeChatConversationInput>(request.params)
            .and_then(|input| {
                let result = ide_chat_conversation_open_result(state, &input.conversation_id)?;
                ide_chat_register_sidebar_conversation(
                    state,
                    &input.conversation_id,
                    &sidebar_label,
                    opened_conversation_id,
                )?;
                if let Some(workspace_path) = input.workspace_path.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
                    let _ = ide_chat_ensure_sidebar_workspace(state, &input.conversation_id, workspace_path, input.workspace_name.as_deref());
                }
                Ok(result)
            }),
        "conversation.blockPage" => ide_chat_conversation_block_page(state, request.params),
        "conversation.create" => (|| {
            let result = ide_chat_create_conversation(state, request.params)?;
            if let Some(conversation_id) = result
                .get("conversationId")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                ide_chat_register_sidebar_conversation(
                    state,
                    conversation_id,
                    &sidebar_label,
                    opened_conversation_id,
                )?;
            }
            Ok(result)
        })(),
        "conversation.createOptions" => ide_chat_create_conversation_options(state),
        "conversation.delete" => ide_chat_delete_conversation(state, request.params),
        "conversation.rewindPreview" => ide_chat_rewind_preview(state, request.params).await,
        "conversation.rewind" => ide_chat_rewind_conversation(state, request.params).await,
        "conversation.branchFromSelection" => ide_chat_branch_conversation(state, request.params).await,
        "delegate.statuses" => ide_chat_delegate_statuses(state, request.params),
        "delegate.abort" => ide_chat_delegate_abort(state, request.params),
        "delegate.blockPage" => ide_chat_delegate_block_page(state, request.params),
        "delegate.submit" => ide_chat_submit_delegate(state, request.params).await,
        "task.list" => ide_chat_task_list(state),
        "task.create" => ide_chat_task_create(state, request.params),
        "task.update" => ide_chat_task_update(state, request.params),
        "task.delete" => ide_chat_task_delete(state, request.params),
        "task.optimizeDraft" => ide_chat_task_optimize_draft(state, request.params).await,
        "task.dispatchNow" => ide_chat_task_dispatch_now(state, request.params).await,
        "goal.current" => ide_chat_goal_current(state, request.params),
        "goal.create" => ide_chat_goal_create(state, request.params),
        "goal.cancel" => ide_chat_goal_cancel(state, request.params),
        "conversation.compactPreview" => ide_chat_compact_preview(state, request.params),
        "conversation.compact" => ide_chat_compact_conversation(state, request.params).await,
        "model.list" => ide_chat_model_list(state, request.params),
        "model.select" => ide_chat_select_model(state, app, request.params),
        "workspace.permission" => ide_chat_workspace_permission(state, request.params),
        "workspace.permission.select" => ide_chat_select_workspace_permission(state, request.params),
        "workspace.list" => ide_chat_workspace_list(state, request.params),
        "workspace.directory.list" => ide_chat_workspace_directory_list(request.params),
        "fileReader.directory.list" => ide_chat_file_reader_directory_list(request.params),
        "fileReader.readFile" => ide_chat_file_reader_read(request.params),
        "ideContext.query" => ide_chat_parse_params::<IdeContextWorkspaceQueryInput>(request.params)
            .and_then(|input| serde_json::to_value(query_ide_context_references_internal(input, ide_context_runtime)?)
                .map_err(|err| format!("serialize IDE context query result failed: {err}"))),
        "workspace.layout.save" => ide_chat_workspace_layout_save(state, request.params),
        "terminalApproval.resolve" => ide_chat_resolve_terminal_approval(state, request.params),
        "conversation.planMode.set" => ide_chat_set_conversation_plan_mode(state, request.params),
        "conversation.plan.confirm" => ide_chat_confirm_plan(state, request.params).await,
        "conversation.plan.readFile" => ide_chat_read_plan_file(state, request.params),
        "settings.open" => ide_chat_open_settings(app),
        "is_backend_ready" => Ok(serde_json::json!(state.backend_ready.load(std::sync::atomic::Ordering::Acquire))),
        "load_config" => ide_chat_load_config_for_web_settings(state),
        "load_app_bootstrap_snapshot" => ide_chat_load_app_bootstrap_snapshot_for_web_settings(state),
        "save_config" => ide_chat_save_config_for_web_settings(state, app, ide_context_runtime, request.params),
        "load_agents" => ide_chat_load_agents_for_web_settings(state),
        "save_agents" => ide_chat_save_agents_for_web_settings(state, app, request.params),
        "load_chat_settings" => ide_chat_load_chat_settings_for_web_settings(state),
        "save_chat_settings" => ide_chat_save_chat_settings_for_web_settings(state, app, request.params),
        "patch_chat_settings" => ide_chat_patch_chat_settings_for_web_settings(state, app, request.params),
        "save_conversation_api_settings" => ide_chat_save_conversation_api_settings_for_web_settings(state, app, request.params),
        "patch_conversation_api_settings" => ide_chat_patch_conversation_api_settings_for_web_settings(state, app, request.params),
        "read_avatar_data_url" => ide_chat_avatar_data_url_for_web_settings(state, request.params),
        "save_agent_avatar" => ide_chat_save_agent_avatar_for_web_settings(state, request.params),
        "clear_agent_avatar" => ide_chat_clear_agent_avatar_for_web_settings(state, request.params),
        "sync_tray_icon" => ide_chat_sync_tray_icon_for_web_settings(app),
        "refresh_models" => ide_chat_refresh_models_for_web_settings(state, request.params).await,
        "quick_genai_chat" => ide_chat_quick_genai_chat_for_web_settings(state, request.params).await,
        "fetch_model_metadata" => ide_chat_fetch_model_metadata_for_web_settings(state, request.params).await,
        "resolve_model_adapter_kind" => ide_chat_resolve_model_adapter_kind_for_web_settings(request.params),
        "test_embedding_connection" => ide_chat_test_embedding_connection_for_web_settings(request.params).await,
        "test_rerank_connection" => ide_chat_test_rerank_connection_for_web_settings(request.params).await,
        "test_voice_connection" => ide_chat_test_voice_connection_for_web_settings(request.params).await,
        "test_memory_embedding_provider" => ide_chat_test_memory_embedding_provider_for_web_settings(state, request.params),
        "test_memory_rerank_provider" => ide_chat_test_memory_rerank_provider_for_web_settings(state, request.params),
        "check_tools_status" => ide_chat_check_tools_status_for_web_settings(state, request.params),
        "get_image_text_cache_stats" => ide_chat_get_image_text_cache_stats_for_web_settings(state),
        "clear_image_text_cache" => ide_chat_clear_image_text_cache_for_web_settings(state),
        "list_tool_catalog" => ide_chat_list_tool_catalog_for_web_settings(state).await,
        "list_department_permission_catalog" => ide_chat_list_department_permission_catalog_for_web_settings(state).await,
        "get_app_version" => Ok(serde_json::json!(env!("CARGO_PKG_VERSION").to_string())),
        "get_project_repository_url" => Ok(serde_json::json!(GITHUB_REPO_PAGE.to_string())),
        "fetch_project_changelog_markdown" => fetch_project_changelog_markdown().await.and_then(ide_chat_serialize),
        "get_web_access_info" => ide_chat_web_access_info_for_web_settings(app, state, ide_context_runtime),
        "open_external_url" => ide_chat_open_external_url_for_web_settings(request.params),
        "show_main_window" => ide_chat_show_window_for_web_settings(app, "main"),
        "show_chat_window" => ide_chat_show_window_for_web_settings(app, "chat"),
        "show_archives_window" => ide_chat_show_window_for_web_settings(app, "archives"),
        "show_quick_setup_window" => ide_chat_show_window_for_web_settings(app, "quick-setup"),
        "complete_quick_setup_and_open_chat" => (|| {
            complete_quick_setup_and_open_chat(app.clone())?;
            Ok(serde_json::json!(null))
        })(),
        "open_runtime_logs_window" => ide_chat_open_runtime_logs_window_for_web_settings(app),
        "list_recent_runtime_logs" => list_recent_runtime_logs().and_then(ide_chat_serialize),
        "clear_recent_runtime_logs" => clear_recent_runtime_logs().and_then(ide_chat_serialize),
        "demo_send_native_notification" => demo_send_native_notification(app.clone()).and_then(ide_chat_serialize),
        "demo_restart_app" => (|| {
            demo_restart_app(app.clone())?;
            Ok(serde_json::json!(null))
        })(),
        "set_webview_zoom_percent" => ide_chat_set_webview_zoom_percent_for_web_settings(app, request.params),
        "set_github_update_method" => ide_chat_set_github_update_method_for_web_settings(state, app, request.params),
        "check_github_update" => ide_chat_check_github_update_for_web_settings(request.params).await,
        "start_github_update" => ide_chat_start_github_update_for_web_settings(app, request.params).await,
        "apply_prepared_github_update" => ide_chat_apply_prepared_github_update_for_web_settings(app).await,
        "codex_get_auth_status" => ide_chat_codex_get_auth_status_for_web_settings(request.params).await,
        "codex_start_oauth_login" => ide_chat_codex_start_oauth_login_for_web_settings(request.params).await,
        "codex_get_rate_limits" => ide_chat_codex_get_rate_limits_for_web_settings(request.params).await,
        "codex_logout" => ide_chat_codex_logout_for_web_settings(request.params),
        "list_memories" => ide_chat_list_memories_for_web_settings(state),
        "delete_memory" => ide_chat_delete_memory_for_web_settings(state, request.params),
        "search_memories_mixed" => ide_chat_search_memories_mixed_for_web_settings(state, request.params),
        "search_chat_history_slices" => ide_chat_search_chat_history_slices_for_web_settings(state, request.params),
        "get_memory_provider_bindings" => ide_chat_get_memory_provider_bindings_for_web_settings(state),
        "get_memory_embedding_sync_progress" => ide_chat_get_memory_embedding_sync_progress_for_web_settings(state),
        "save_memory_embedding_binding" => ide_chat_save_memory_embedding_binding_for_web_settings(state, request.params),
        "save_memory_rerank_binding" => ide_chat_save_memory_rerank_binding_for_web_settings(state, request.params),
        "get_agent_private_memory_count" => ide_chat_get_agent_private_memory_count_for_web_settings(state, request.params),
        "set_agent_memory_recall_mode" => ide_chat_set_agent_memory_recall_mode_for_web_settings(state, request.params),
        "set_agent_private_memory_enabled" => ide_chat_set_agent_private_memory_enabled_for_web_settings(state, request.params),
        "export_agent_private_memories" => ide_chat_export_agent_private_memories_for_web_settings(state, request.params),
        "disable_agent_private_memory" => ide_chat_disable_agent_private_memory_for_web_settings(state, request.params),
        "export_memories" => ide_chat_export_memories_for_web_settings(state, request.params),
        "preview_export_memories" => ide_chat_preview_export_memories_for_web_settings(state),
        "export_memories_to_path" => ide_chat_export_memories_to_path_for_web_settings(state, request.params),
        "import_memories" => ide_chat_import_memories_for_web_settings(state, request.params),
        "preview_import_angel_memories" => ide_chat_preview_import_angel_memories_for_web_settings(request.params),
        "import_angel_memories" => ide_chat_import_angel_memories_for_web_settings(state, request.params),
        "task_list_tasks" => ide_chat_task_list_tasks_for_web_settings(state),
        "task_get_task" => ide_chat_task_get_task_for_web_settings(state, request.params),
        "task_create_task" => ide_chat_task_create_task_for_web_settings(state, request.params),
        "task_update_task" => ide_chat_task_update_task_for_web_settings(state, request.params),
        "task_complete_task" => ide_chat_task_complete_task_for_web_settings(state, request.params),
        "task_delete_task" => ide_chat_task_delete_task_for_web_settings(state, request.params),
        "task_list_run_logs" => ide_chat_task_list_run_logs_for_web_settings(state, request.params),
        "task_optimize_draft" => ide_chat_task_optimize_draft_for_web_settings(state, request.params).await,
        "mcp_list_servers" => ide_chat_mcp_list_servers_for_web_settings(state),
        "mcp_validate_definition" => ide_chat_mcp_validate_definition_for_web_settings(request.params),
        "mcp_save_server" => ide_chat_mcp_save_server_for_web_settings(state, request.params),
        "mcp_remove_server" => ide_chat_mcp_remove_server_for_web_settings(state, request.params).await,
        "mcp_list_server_tools" => ide_chat_mcp_list_server_tools_for_web_settings(state, request.params).await,
        "mcp_list_server_tools_cached" => ide_chat_mcp_list_server_tools_cached_for_web_settings(state, request.params),
        "mcp_deploy_server" => ide_chat_mcp_deploy_server_for_web_settings(state, request.params),
        "mcp_undeploy_server" => ide_chat_mcp_undeploy_server_for_web_settings(state, request.params).await,
        "mcp_set_tool_enabled" => ide_chat_mcp_set_tool_enabled_for_web_settings(state, request.params),
        "mcp_open_workspace_dir" => ide_chat_mcp_open_workspace_dir_for_web_settings(state),
        "mcp_list_skills" => ide_chat_mcp_list_skills_for_web_settings(state),
        "mcp_refresh_mcp_and_skills" => ide_chat_mcp_refresh_mcp_and_skills_for_web_settings(state).await,
        "skill_open_workspace_dir" => ide_chat_skill_open_workspace_dir_for_web_settings(state),
        "get_storage_usage_overview" => ide_chat_get_storage_usage_overview_for_web_settings(state),
        "open_storage_usage_item_directory" => ide_chat_open_storage_usage_item_directory_for_web_settings(state, request.params),
        "cleanup_storage_legacy_items" => ide_chat_cleanup_storage_legacy_items_for_web_settings(state, request.params),
        "export_config_migration_package" => ide_chat_export_config_migration_package_for_web_settings(state, request.params),
        "preview_import_config_migration_package" => ide_chat_preview_import_config_migration_package_for_web_settings(state, request.params),
        "apply_import_config_migration_package" => ide_chat_apply_import_config_migration_package_for_web_settings(state, app, request.params),
        "list_recent_llm_round_logs" => ide_chat_list_recent_llm_round_logs_for_web_settings(state),
        "get_recent_llm_round_log_section" => ide_chat_get_recent_llm_round_log_section_for_web_settings(state, request.params),
        "clear_recent_llm_round_logs" => ide_chat_clear_recent_llm_round_logs_for_web_settings(state),
        "list_terminal_shell_candidates" => ide_chat_list_terminal_shell_candidates_for_web_settings(state),
        "open_chat_shell_workspace_dir" => ide_chat_open_chat_shell_workspace_dir_for_web_settings(state, request.params),
        "reset_chat_shell_workspace" => ide_chat_reset_chat_shell_workspace_for_web_settings(state, request.params),
        "get_default_chat_shell_workspace_path" => ide_chat_get_default_chat_shell_workspace_path_for_web_settings(state),
        "migrate_shell_workspace_directory" => ide_chat_migrate_shell_workspace_directory_for_web_settings(app, request.params).await,
        "get_host_runtime_prerequisites" => ide_chat_get_host_runtime_prerequisites_for_web_settings(),
        "install_host_runtime_prerequisite" => ide_chat_install_host_runtime_prerequisite_for_web_settings(request.params).await,
        "remote_im_get_channel_status" => ide_chat_remote_im_get_channel_status_for_web_settings(state, request.params).await,
        "remote_im_restart_channel" => ide_chat_remote_im_restart_channel_for_web_settings(state, request.params).await,
        "remote_im_get_channel_logs" => ide_chat_remote_im_get_channel_logs_for_web_settings(request.params).await,
        "remote_im_get_contact_logs" => ide_chat_remote_im_get_contact_logs_for_web_settings(state, request.params).await,
        "remote_im_list_channels" => ide_chat_remote_im_list_channels_for_web_settings(state),
        "remote_im_list_contacts" => ide_chat_remote_im_list_contacts_for_web_settings(state),
        "remote_im_update_contact_allow_send" => ide_chat_remote_im_update_contact_allow_send_for_web_settings(state, request.params),
        "remote_im_update_contact_allow_send_files" => ide_chat_remote_im_update_contact_allow_send_files_for_web_settings(state, request.params),
        "remote_im_update_contact_activation" => ide_chat_remote_im_update_contact_activation_for_web_settings(state, request.params),
        "remote_im_update_contact_department_binding" => ide_chat_remote_im_update_contact_department_binding_for_web_settings(state, request.params),
        "remote_im_update_contact_processing_mode" => ide_chat_remote_im_update_contact_processing_mode_for_web_settings(state, request.params),
        "remote_im_update_contact_workspace" => ide_chat_remote_im_update_contact_workspace_for_web_settings(state, request.params),
        "remote_im_delete_contact" => ide_chat_remote_im_delete_contact_for_web_settings(state, request.params),
        "remote_im_weixin_oc_start_login" => ide_chat_remote_im_weixin_oc_start_login_for_web_settings(state, request.params).await,
        "remote_im_weixin_oc_get_login_status" => ide_chat_remote_im_weixin_oc_get_login_status_for_web_settings(state, request.params).await,
        "remote_im_weixin_oc_sync_contacts" => ide_chat_remote_im_weixin_oc_sync_contacts_for_web_settings(state, request.params).await,
        "remote_im_weixin_oc_logout" => ide_chat_remote_im_weixin_oc_logout_for_web_settings(state, request.params).await,
        "chat.send" => ide_chat_send_message(state, request.params),
        "chat.stop" => ide_chat_stop_conversation(state, request.params),
        "toolReview.reports.list" => ide_chat_tool_review_reports(state, request.params),
        "toolReview.report.delete" => ide_chat_tool_review_delete_report(state, request.params),
        "toolReview.commitOptions.list" => ide_chat_tool_review_commit_options(state, request.params).await,
        "toolReview.code.submit" => ide_chat_tool_review_submit_code(state, request.params).await,
        "toolReview.batches.list" => ide_chat_tool_review_batches(state, request.params),
        "toolReview.item.detail" => ide_chat_tool_review_item_detail(state, request.params),
        "toolReview.item.review" => ide_chat_tool_review_item_review(state, request.params).await,
        "toolReview.batch.review" => ide_chat_tool_review_batch_review(state, request.params).await,
        "toolReview.item.decision" => ide_chat_tool_review_item_decision(state, request.params),
        _ => return ide_chat_jsonrpc_error(request.id, -32601, "method not found"),
    };
    match result {
        Ok(value) => ide_chat_jsonrpc_success(request.id, value),
        Err(err) => ide_chat_jsonrpc_error(request.id, -32000, err),
    }
}

fn start_ide_context_bridge_server(app: AppHandle, state: AppState, ide_context_runtime: IdeContextRuntime) {
    if IDE_CONTEXT_BRIDGE_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    let shutdown_token = ide_context_bridge_create_shutdown_token();
    let server_task = tauri::async_runtime::spawn(async move {
        let config = match state_read_config_cached(&state) {
            Ok(config) => config,
            Err(err) => {
                eprintln!(
                    "[网络访问] 读取配置失败，使用默认端口: {}",
                    err
                );
                AppConfig::default()
            }
        };
        if !config.web_access_enabled {
            IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
            ide_context_set_current_port(&ide_context_runtime, None);
            clear_ide_context_bridge_discovery();
            eprintln!("[网络访问] 跳过启动：网络访问已关闭");
            return;
        }
        let preferred_port = normalize_web_access_port(config.web_access_port);
        let (listener, port) = match bind_ide_context_bridge_listener(preferred_port).await {
            Ok(result) => result,
            Err(err) => {
                IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
                ide_context_set_current_port(&ide_context_runtime, None);
                clear_ide_context_bridge_discovery();
                eprintln!("[IDE 上下文桥] 监听失败: {}", err);
                return;
            }
        };
        ide_context_set_current_port(&ide_context_runtime, Some(port));
        let bridge_url = ide_context_bridge_url(port);
        let remote_password = match ide_context_effective_remote_password(&state, &ide_context_runtime) {
            Ok(password) => password,
            Err(err) => {
                IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
                ide_context_set_current_port(&ide_context_runtime, None);
                clear_ide_context_bridge_discovery();
                eprintln!("[IDE 上下文桥] 初始化远程访问密码失败: {}", err);
                return;
            }
        };
        if let Err(err) = publish_ide_context_bridge_discovery(port, &remote_password) {
            IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
            ide_context_set_current_port(&ide_context_runtime, None);
            clear_ide_context_bridge_discovery();
            eprintln!("[IDE 上下文桥] 写入发现文件失败: {}", err);
            return;
        }
        eprintln!("[IDE 上下文桥] 已监听 {}", bridge_url);
        loop {
            let (stream, peer_addr) = tokio::select! {
                _ = shutdown_token.cancelled() => {
                    clear_ide_context_bridge_discovery();
                    IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
                    ide_context_set_current_port(&ide_context_runtime, None);
                    if let Ok(mut slot) = ide_context_bridge_shutdown_slot().lock() {
                        slot.take();
                    }
                    eprintln!("[IDE 上下文桥] 收到停机信号，停止监听 {}", bridge_url);
                    break;
                }
                result = listener.accept() => match result {
                    Ok(result) => result,
                    Err(err) => {
                        eprintln!("[IDE 上下文桥] 接收连接失败: {}", err);
                        continue;
                    }
                },
            };
            let state_clone = state.clone();
            let app_clone = app.clone();
            let ide_context_runtime_clone = ide_context_runtime.clone();
            tauri::async_runtime::spawn(async move {
                ide_context_ws_handle_connection(
                    stream,
                    peer_addr,
                    port,
                    app_clone,
                    state_clone,
                    ide_context_runtime_clone,
                )
                .await;
            });
        }
    });
    ide_context_bridge_set_server_task(server_task);
}

pub(crate) async fn shutdown_ide_context_bridge_server() {
    if !IDE_CONTEXT_BRIDGE_STARTED.load(Ordering::SeqCst) {
        clear_ide_context_bridge_discovery();
        let _ = ide_context_bridge_take_server_task();
        return;
    }
    if let Ok(slot) = ide_context_bridge_shutdown_slot().lock() {
        if let Some(token) = slot.as_ref() {
            token.cancel();
        }
    }
    clear_ide_context_bridge_discovery();
    if let Some(clients) = IDE_CONTEXT_CHAT_CLIENTS.get() {
        if let Ok(mut clients) = clients.lock() {
            clients.clear();
        }
    }
    let task = ide_context_bridge_take_server_task();
    match task {
        Some(handle) => match tokio::time::timeout(std::time::Duration::from_secs(3), handle).await {
            Ok(Ok(())) => {
                eprintln!("[IDE 上下文桥] 已停止");
            }
            Ok(Err(err)) => {
                IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
                eprintln!("[IDE 上下文桥] 等待服务任务退出失败: {}", err);
            }
            Err(_) => {
                IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
                eprintln!("[IDE 上下文桥] 等待服务任务退出超时，已强制清理状态");
            }
        },
        None => {
            IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
            eprintln!("[IDE 上下文桥] 停机时未找到服务任务句柄，已清理状态");
        }
    }
    if let Ok(mut slot) = ide_context_bridge_shutdown_slot().lock() {
        slot.take();
    }
}

fn restart_ide_context_bridge_server(app: AppHandle, state: AppState, ide_context_runtime: IdeContextRuntime) {
    tauri::async_runtime::spawn(async move {
        shutdown_ide_context_bridge_server().await;
        start_ide_context_bridge_server(app, state, ide_context_runtime);
    });
}

async fn ide_context_ws_handle_connection(
    stream: tokio::net::TcpStream,
    peer_addr: std::net::SocketAddr,
    port: u16,
    app: AppHandle,
    state: AppState,
    ide_context_runtime: IdeContextRuntime,
) {
    if !ide_context_stream_is_websocket(&stream).await {
        ide_context_http_handle_connection(stream, app).await;
        return;
    }
    let path_holder = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let path_holder_clone = path_holder.clone();
    let ws_stream = match accept_hdr_async(stream, move |request: &Request, response: Response| {
        if let Ok(mut slot) = path_holder_clone.lock() {
            *slot = request.uri().path().to_string();
        }
        if !ide_context_ws_origin_allowed(request, port) {
            return Err(ide_context_ws_forbidden_response("Forbidden origin"));
        }
        Ok(response)
    })
    .await
    {
        Ok(ws_stream) => ws_stream,
        Err(err) => {
            eprintln!("[IDE 上下文桥] WebSocket 握手失败 {}: {}", peer_addr, err);
            return;
        }
    };
    let path = path_holder.lock().map(|value| value.clone()).unwrap_or_default();
    if path == IDE_CONTEXT_CHAT_BRIDGE_PATH {
        ide_context_chat_ws_handle_connection(
            ws_stream,
            peer_addr,
            app,
            state,
            ide_context_runtime,
        )
        .await;
        return;
    }
    if path != IDE_CONTEXT_BRIDGE_PATH {
        eprintln!("[IDE 上下文桥] 非法路径 {} from {}", path, peer_addr);
        return;
    }
    eprintln!("[IDE 上下文桥] 客户端已连接: {}", peer_addr);
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut connected_client_id = String::new();
    let mut authenticated = ide_context_peer_is_local(&peer_addr);
    let _ = ws_sender
        .send(tokio_tungstenite::tungstenite::Message::Text(
            serde_json::json!({
                "type": "ready",
                "path": IDE_CONTEXT_BRIDGE_PATH,
                "authRequired": !authenticated,
            })
                .to_string()
                .into(),
        ))
        .await;
    while let Some(message) = ws_receiver.next().await {
        match message {
            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                match serde_json::from_str::<UpsertIdeContextSnapshotInput>(&text) {
                    Ok(input) => {
                        if !authenticated {
                            match ide_context_consume_bridge_token_with_state(
                                &ide_context_runtime,
                                Some(&state),
                                input.auth_token.as_deref().unwrap_or(""),
                            ) {
                                Ok(_token) => {
                                    authenticated = true;
                                }
                                Err((err, refreshed_token)) => {
                                    if let Some(_refreshed_token) = refreshed_token.as_deref() {
                                        if let Ok(remote_password) = ide_context_effective_remote_password(&state, &ide_context_runtime) {
                                            if let Err(publish_err) =
                                                publish_ide_context_bridge_discovery(port, &remote_password)
                                            {
                                                eprintln!(
                                                    "[IDE 上下文桥] 过期后重写发现文件失败: {}",
                                                    publish_err
                                                );
                                            }
                                        }
                                    }
                                    let _ = ws_sender
                                        .send(tokio_tungstenite::tungstenite::Message::Text(
                                            serde_json::json!({"type": "ack", "ok": false, "error": err})
                                                .to_string()
                                                .into(),
                                        ))
                                        .await;
                                    break;
                                }
                            }
                        }
                        match upsert_ide_context_snapshot_internal(input, &ide_context_runtime) {
                            Ok((client_id, updated_at)) => {
                                connected_client_id = client_id.clone();
                                emit_ide_context_updated(&state, &client_id, &updated_at);
                                let _ = ws_sender
                                    .send(tokio_tungstenite::tungstenite::Message::Text(
                                        serde_json::json!({"type": "ack", "ok": true}).to_string().into(),
                                    ))
                                    .await;
                            }
                            Err(err) => {
                                let _ = ws_sender
                                    .send(tokio_tungstenite::tungstenite::Message::Text(
                                        serde_json::json!({"type": "ack", "ok": false, "error": err})
                                            .to_string()
                                            .into(),
                                    ))
                                    .await;
                            }
                        }
                    }
                    Err(err) => {
                        let _ = ws_sender
                            .send(tokio_tungstenite::tungstenite::Message::Text(
                                serde_json::json!({"type": "ack", "ok": false, "error": format!("invalid json: {err}")}).to_string().into(),
                            ))
                            .await;
                    }
                }
            }
            Ok(tokio_tungstenite::tungstenite::Message::Ping(payload)) => {
                let _ = ws_sender.send(tokio_tungstenite::tungstenite::Message::Pong(payload)).await;
            }
            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
            Ok(_) => {}
            Err(err) => {
                eprintln!("[IDE 上下文桥] 客户端消息错误 {}: {}", peer_addr, err);
                break;
            }
        }
    }
    if !connected_client_id.is_empty() {
        match ide_context_runtime.snapshots.lock() {
            Ok(mut snapshots) => {
                snapshots.remove(&connected_client_id);
            }
            Err(_) => {
                eprintln!("[IDE 上下文桥] 清理客户端缓存失败: {}", connected_client_id);
            }
        }
    }
    eprintln!("[IDE 上下文桥] 客户端已断开: {}", peer_addr);
}

async fn ide_context_chat_ws_handle_connection(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    peer_addr: std::net::SocketAddr,
    app: AppHandle,
    state: AppState,
    ide_context_runtime: IdeContextRuntime,
) {
    eprintln!("[VSCode 侧边栏] 客户端已连接: {}", peer_addr);
    let client_id = Uuid::new_v4().to_string();
    let mut authenticated = ide_context_peer_is_local(&peer_addr);
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let (outbound_tx, mut outbound_rx) = tokio::sync::mpsc::unbounded_channel::<serde_json::Value>();
    let writer_client_id = client_id.clone();
    let writer = tauri::async_runtime::spawn(async move {
        while let Some(message) = outbound_rx.recv().await {
            if ws_sender
                .send(tokio_tungstenite::tungstenite::Message::Text(message.to_string().into()))
                .await
                .is_err()
            {
                break;
            }
        }
        if let Ok(mut clients) = ide_context_chat_clients().lock() {
            clients.remove(&writer_client_id);
        }
    });
    let _ = outbound_tx.send(serde_json::json!({
        "jsonrpc": "2.0",
        "method": "bridge.ready",
        "params": {
            "path": IDE_CONTEXT_CHAT_BRIDGE_PATH,
            "authRequired": !authenticated,
            "authMode": if authenticated { "none" } else { "password" },
        },
    }));
    let mut registered_client = false;
    if authenticated {
        if let Ok(mut clients) = ide_context_chat_clients().lock() {
            clients.insert(client_id.clone(), outbound_tx.clone());
            registered_client = true;
        }
    }
    let mut opened_conversation_id: Option<String> = None;
    while let Some(message) = ws_receiver.next().await {
        match message {
            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                let response = match serde_json::from_str::<IdeChatJsonRpcRequest>(&text) {
                    Ok(request) => {
                        if !authenticated {
                            if request.jsonrpc.trim() != "2.0" {
                                ide_chat_jsonrpc_error(request.id, -32600, "jsonrpc must be 2.0")
                            } else if request.method.as_str() == "auth.login" {
                                match ide_chat_parse_params::<IdeChatAuthLoginInput>(request.params) {
                                    Ok(input) => match ide_context_verify_remote_password(
                                        &ide_context_runtime,
                                        Some(&state),
                                        &input.password,
                                    ) {
                                        Ok(true) => match ide_context_issue_bridge_token_with_state(
                                            &ide_context_runtime,
                                            Some(&state),
                                        ) {
                                            Ok(auth_token) => {
                                                authenticated = true;
                                                if !registered_client {
                                                    if let Ok(mut clients) = ide_context_chat_clients().lock() {
                                                        clients.insert(client_id.clone(), outbound_tx.clone());
                                                        registered_client = true;
                                                    }
                                                }
                                                ide_chat_jsonrpc_success(request.id, serde_json::json!({
                                                    "authenticated": true,
                                                    "authToken": auth_token,
                                                }))
                                            }
                                            Err(err) => ide_chat_jsonrpc_error(request.id, -32000, err),
                                        },
                                        Ok(false) => ide_chat_jsonrpc_error(request.id, -32001, "远程访问密码错误"),
                                        Err(err) => ide_chat_jsonrpc_error(request.id, -32000, err),
                                    },
                                    Err(err) => ide_chat_jsonrpc_error(request.id, -32602, err),
                                }
                            } else {
                                let provided_auth_token = request
                                    .params
                                    .as_object()
                                    .and_then(|params| params.get("authToken"))
                                    .and_then(Value::as_str)
                                    .unwrap_or("");
                                match ide_context_consume_bridge_token_with_state(
                                    &ide_context_runtime,
                                    Some(&state),
                                    provided_auth_token,
                                ) {
                                    Ok(_token) => {
                                        authenticated = true;
                                        if !registered_client {
                                            if let Ok(mut clients) = ide_context_chat_clients().lock() {
                                                clients.insert(client_id.clone(), outbound_tx.clone());
                                                registered_client = true;
                                            }
                                        }
                                        ide_chat_handle_jsonrpc_request(
                                            request,
                                            &state,
                                            &app,
                                            &ide_context_runtime,
                                            &client_id,
                                            &mut opened_conversation_id,
                                        )
                                        .await
                                    }
                                    Err((err, refreshed_token)) => {
                                        if let Some(_refreshed_token) = refreshed_token.as_deref() {
                                            if let Some(current_port) = ide_context_current_port(&ide_context_runtime) {
                                                if let Ok(remote_password) =
                                                    ide_context_effective_remote_password(&state, &ide_context_runtime)
                                                {
                                                    if let Err(publish_err) =
                                                        publish_ide_context_bridge_discovery(current_port, &remote_password)
                                                    {
                                                        eprintln!(
                                                            "[VSCode 侧边栏] 过期后重写发现文件失败: {}",
                                                            publish_err
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        ide_chat_jsonrpc_error(request.id, -32001, err)
                                    }
                                }
                            }
                        } else {
                            ide_chat_handle_jsonrpc_request(
                                request,
                                &state,
                                &app,
                                &ide_context_runtime,
                                &client_id,
                                &mut opened_conversation_id,
                            )
                            .await
                        }
                    }
                    Err(err) => ide_chat_jsonrpc_error(None, -32700, format!("invalid json: {err}")),
                };
                let _ = outbound_tx.send(response);
            }
            Ok(tokio_tungstenite::tungstenite::Message::Ping(payload)) => {
                let _ = outbound_tx.send(serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "bridge.ping",
                    "params": { "bytes": payload.len() },
                }));
            }
            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
            Ok(_) => {}
            Err(err) => {
                eprintln!("[VSCode 侧边栏] 客户端消息错误 {}: {}", peer_addr, err);
                break;
            }
        }
    }
    if let Ok(mut clients) = ide_context_chat_clients().lock() {
        clients.remove(&client_id);
    }
    if opened_conversation_id.is_some() {
        let sidebar_label = ide_chat_sidebar_window_label(&client_id);
        if let Err(err) = ide_chat_release_sidebar_conversation(&state, &sidebar_label) {
            eprintln!("[VSCode 侧边栏] 释放会话占用失败: {}", err);
        }
    }
    writer.abort();
    eprintln!("[VSCode 侧边栏] 客户端已断开: {}", peer_addr);
}

#[cfg(test)]
mod ide_context_tests {
    use super::*;

    #[test]
    fn ide_context_remote_password_accepts_human_input_format() {
        let runtime = IdeContextRuntime::new();
        let password = ide_context_remote_password(&runtime).expect("remote password");
        let compact_lowercase = password.replace('-', "").to_ascii_lowercase();

        assert!(ide_context_verify_remote_password(&runtime, None, &password).expect("verify password"));
        assert!(
            ide_context_verify_remote_password(&runtime, None, &compact_lowercase)
                .expect("verify compact password")
        );
        assert!(!ide_context_verify_remote_password(&runtime, None, "").expect("reject empty"));
        assert!(!ide_context_verify_remote_password(&runtime, None, "wrong-password").expect("reject wrong"));
    }

    #[test]
    fn ide_context_peer_is_local_only_allows_loopback() {
        let ipv4_local: std::net::SocketAddr = "127.0.0.1:43129".parse().expect("ipv4 local");
        let ipv6_local: std::net::SocketAddr = "[::1]:43129".parse().expect("ipv6 local");
        let remote: std::net::SocketAddr = "192.168.1.10:43129".parse().expect("remote");

        assert!(ide_context_peer_is_local(&ipv4_local));
        assert!(ide_context_peer_is_local(&ipv6_local));
        assert!(!ide_context_peer_is_local(&remote));
    }

    fn ide_context_ws_test_request(origin: Option<&str>) -> Request {
        ide_context_ws_test_request_with_host(origin, "127.0.0.1:43129")
    }

    fn ide_context_ws_test_request_with_host(origin: Option<&str>, host: &str) -> Request {
        let mut builder = Request::builder()
            .uri(IDE_CONTEXT_CHAT_BRIDGE_PATH)
            .header("host", host);
        if let Some(origin) = origin {
            builder = builder.header("origin", origin);
        }
        builder.body(()).expect("build websocket request")
    }

    #[test]
    fn ide_context_ws_origin_allows_owned_pages_and_vscode_webview() {
        assert!(ide_context_ws_origin_allowed(
            &ide_context_ws_test_request(None),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
        assert!(ide_context_ws_origin_allowed(
            &ide_context_ws_test_request(Some("vscode-webview://abc123")),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
        assert!(ide_context_ws_origin_allowed(
            &ide_context_ws_test_request(Some("http://127.0.0.1:43129")),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
        assert!(ide_context_ws_origin_allowed(
            &ide_context_ws_test_request_with_host(Some("http://192.168.1.20:43129"), "192.168.1.20:43129"),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
    }

    #[test]
    fn ide_context_ws_origin_rejects_external_pages() {
        assert!(!ide_context_ws_origin_allowed(
            &ide_context_ws_test_request(Some("https://example.com")),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
        assert!(!ide_context_ws_origin_allowed(
            &ide_context_ws_test_request(Some("http://example.com:43129")),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
        assert!(!ide_context_ws_origin_allowed(
            &ide_context_ws_test_request(Some("http://127.0.0.1:43130")),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
        assert!(!ide_context_ws_origin_allowed(
            &ide_context_ws_test_request_with_host(Some("http://192.168.1.50:43129"), "127.0.0.1:43129"),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
        assert!(!ide_context_ws_origin_allowed(
            &ide_context_ws_test_request(Some("null")),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
    }

    #[test]
    fn ide_context_lan_host_filter_rejects_reserved_and_accepts_private_lan() {
        let mihomo: std::net::Ipv4Addr = "198.18.0.1".parse().expect("mihomo ip");
        let hyperv: std::net::Ipv4Addr = "192.168.240.1".parse().expect("hyperv ip");
        let ethernet: std::net::Ipv4Addr = "192.168.5.23".parse().expect("ethernet ip");
        let cgnat: std::net::Ipv4Addr = "100.64.1.2".parse().expect("cgnat ip");

        assert!(!ide_context_ipv4_is_remote_link_candidate(mihomo));
        assert!(!ide_context_ipv4_is_remote_link_candidate(cgnat));
        assert!(ide_context_ipv4_is_remote_link_candidate(hyperv));
        assert!(ide_context_ipv4_is_remote_link_candidate(ethernet));
    }

    #[test]
    fn ide_context_lan_host_rank_prefers_real_gateway_adapter() {
        let ethernet = IdeContextLanHostCandidate {
            ip: "192.168.5.23".parse().expect("ethernet ip"),
            adapter_name: "以太网".to_string(),
            adapter_description: "Realtek PCIe GbE Family Controller".to_string(),
            has_gateway: true,
            active: true,
        };
        let hyperv = IdeContextLanHostCandidate {
            ip: "192.168.240.1".parse().expect("hyperv ip"),
            adapter_name: "vEthernet (Default Switch)".to_string(),
            adapter_description: "Hyper-V Virtual Ethernet Adapter".to_string(),
            has_gateway: false,
            active: true,
        };

        assert!(ide_context_lan_host_rank(&ethernet) < ide_context_lan_host_rank(&hyperv));
    }

    #[test]
    fn ide_context_bridge_tokens_allow_concurrent_consumers_until_expiry() {
        let runtime = IdeContextRuntime::new();
        let token = ide_context_issue_bridge_token_with_state(&runtime, None).expect("issue token");

        let next_token = ide_context_consume_bridge_token_with_state(&runtime, None, &token)
            .expect("first consume");
        assert_eq!(next_token, token);

        let second_next = ide_context_consume_bridge_token_with_state(&runtime, None, &token)
            .expect("second consume with same token");
        assert_eq!(second_next, token);
    }

    #[test]
    fn ide_context_bridge_tokens_reject_unknown_token() {
        let runtime = IdeContextRuntime::new();
        let _ = ide_context_issue_bridge_token_with_state(&runtime, None).expect("issue token");
        let err = ide_context_consume_bridge_token_with_state(&runtime, None, "bad-token")
            .expect_err("invalid token");
        assert!(err.0.contains("invalid authToken"));
    }

    #[test]
    fn ide_context_bridge_tokens_reissue_when_cache_expired() {
        let runtime = IdeContextRuntime::new();
        {
            let mut auth = runtime.bridge_auth.lock().expect("lock auth");
            auth.valid_tokens.insert(
                "expired-token".to_string(),
                time::OffsetDateTime::now_utc() - time::Duration::seconds(1),
            );
        }

        let err = ide_context_consume_bridge_token_with_state(&runtime, None, "expired-token")
            .expect_err("expired token should refresh discovery");
        assert!(err.0.contains("expired"));
        let refreshed = err.1.expect("should issue refreshed token");
        let auth = runtime.bridge_auth.lock().expect("lock auth");
        assert!(auth.valid_tokens.contains_key(&refreshed));
    }
}

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
const IDE_CONTEXT_BRIDGE_BASE_PORT: u16 = 43129;
const IDE_CONTEXT_BRIDGE_MAX_PORT: u16 = 43139;
const IDE_CONTEXT_BRIDGE_PATH: &str = "/ide-context";
const IDE_CONTEXT_CHAT_BRIDGE_PATH: &str = "/chat";
const IDE_CONTEXT_BRIDGE_DISCOVERY_FILE: &str = "p-ai-ide-context-bridge.json";
const IDE_CONTEXT_SNAPSHOT_TTL_SECS: i64 = 30;
const IDE_CONTEXT_AUTH_TOKEN_TTL_SECS: i64 = 24 * 60 * 60;
static IDE_CONTEXT_BRIDGE_STARTED: AtomicBool = AtomicBool::new(false);
static IDE_CONTEXT_BRIDGE_SHUTDOWN: OnceLock<tokio_util::sync::CancellationToken> = OnceLock::new();
static IDE_CONTEXT_CHAT_CLIENTS: OnceLock<
    Arc<Mutex<std::collections::HashMap<String, tokio::sync::mpsc::UnboundedSender<serde_json::Value>>>>,
> = OnceLock::new();

#[derive(Debug, Clone)]
struct IdeContextRuntime {
    snapshots: Arc<Mutex<std::collections::HashMap<String, IdeContextSnapshot>>>,
    bridge_auth: Arc<Mutex<IdeContextBridgeAuthRuntime>>,
}

#[derive(Debug, Default)]
struct IdeContextBridgeAuthRuntime {
    valid_tokens: std::collections::HashMap<String, OffsetDateTime>,
}

impl IdeContextRuntime {
    fn new() -> Self {
        Self {
            snapshots: Arc::new(Mutex::new(std::collections::HashMap::new())),
            bridge_auth: Arc::new(Mutex::new(IdeContextBridgeAuthRuntime::default())),
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
    port: u16,
    path: String,
    chat_path: String,
    pid: u32,
    updated_at: String,
    token: String,
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
    partial_reasoning_standard: String,
    #[serde(default)]
    partial_reasoning_inline: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatSelectModelInput {
    conversation_id: String,
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
    agent_id: String,
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
    let selected_id = config
        .departments
        .iter()
        .find(|department| department.id.trim() == conversation.department_id.trim())
        .map(department_primary_api_config_id)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| config.assistant_department_api_config_id.trim().to_string());
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
        "selectedChatModelId": selected_id,
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
    let config = state_read_config_cached(state)?;
    let agents = state_read_agents_cached(state)?;
    let options = config
        .departments
        .iter()
        .filter_map(|department| {
            let department_id = department.id.trim();
            if department_id.is_empty() {
                return None;
            }
            let api_config_id = department_primary_api_config_id(department);
            if api_config_id.trim().is_empty() {
                return None;
            }
            let api_config = config
                .api_configs
                .iter()
                .find(|api| api.id.trim() == api_config_id.trim() && is_text_chat_api(api))?;
            let owner_id = department
                .agent_ids
                .first()
                .map(|value| value.trim())
                .unwrap_or_default();
            let owner_name = agents
                .iter()
                .find(|agent| agent.id.trim() == owner_id)
                .map(|agent| agent.name.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| {
                    if owner_id.is_empty() {
                        "未设置负责人".to_string()
                    } else {
                        owner_id.to_string()
                    }
                });
            Some(serde_json::json!({
                "id": department_id,
                "name": if department.name.trim().is_empty() { department_id } else { department.name.trim() },
                "ownerAgentId": owner_id,
                "ownerName": owner_name,
                "providerName": if api_config.name.trim().is_empty() { api_config.id.trim() } else { api_config.name.trim() },
                "modelName": api_config.model.trim(),
                "childDepartmentIds": &department.child_department_ids,
            }))
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "departments": options,
        "defaultDepartmentId": ASSISTANT_DEPARTMENT_ID,
    }))
}

fn ide_context_generate_bridge_token() -> String {
    Uuid::new_v4().to_string()
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

fn ide_context_issue_bridge_token(runtime: &IdeContextRuntime) -> Result<String, String> {
    let token = ide_context_generate_bridge_token();
    let mut auth = runtime
        .bridge_auth
        .lock()
        .map_err(|_| "Failed to lock ide context bridge auth".to_string())?;
    let now = now_utc();
    ide_context_prune_expired_bridge_tokens(&mut auth, now);
    auth.valid_tokens.insert(
        token.clone(),
        now + time::Duration::seconds(IDE_CONTEXT_AUTH_TOKEN_TTL_SECS),
    );
    Ok(token)
}

fn ide_context_consume_bridge_token(
    runtime: &IdeContextRuntime,
    provided: &str,
) -> Result<String, (String, Option<String>)> {
    let provided = provided.trim();
    if provided.is_empty() {
        return Err(("authToken is required".to_string(), None));
    }
    let mut auth = runtime
        .bridge_auth
        .lock()
        .map_err(|_| ("Failed to lock ide context bridge auth".to_string(), None))?;
    let now = now_utc();
    ide_context_prune_expired_bridge_tokens(&mut auth, now);
    if auth.valid_tokens.is_empty() {
        let refreshed_token = ide_context_generate_bridge_token();
        auth.valid_tokens.insert(
            refreshed_token.clone(),
            now + time::Duration::seconds(IDE_CONTEXT_AUTH_TOKEN_TTL_SECS),
        );
        return Err((
            "IDE context bridge token expired, discovery refreshed".to_string(),
            Some(refreshed_token),
        ));
    }
    if !auth.valid_tokens.contains_key(provided) {
        return Err(("invalid authToken".to_string(), None));
    }
    auth.valid_tokens.insert(
        provided.to_string(),
        now + time::Duration::seconds(IDE_CONTEXT_AUTH_TOKEN_TTL_SECS),
    );
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

fn ide_context_bridge_url(port: u16) -> String {
    format!("ws://{}:{}{}", IDE_CONTEXT_BRIDGE_HOST, port, IDE_CONTEXT_BRIDGE_PATH)
}

fn ide_context_chat_bridge_url(port: u16) -> String {
    format!("ws://{}:{}{}", IDE_CONTEXT_BRIDGE_HOST, port, IDE_CONTEXT_CHAT_BRIDGE_PATH)
}

fn ide_context_bridge_discovery_path() -> std::path::PathBuf {
    std::env::temp_dir().join(IDE_CONTEXT_BRIDGE_DISCOVERY_FILE)
}

fn ide_context_bridge_shutdown_token() -> &'static tokio_util::sync::CancellationToken {
    IDE_CONTEXT_BRIDGE_SHUTDOWN.get_or_init(tokio_util::sync::CancellationToken::new)
}

fn publish_ide_context_bridge_discovery(port: u16, token: &str) -> Result<(), String> {
    let url = ide_context_bridge_url(port);
    let chat_url = ide_context_chat_bridge_url(port);
    let payload = IdeContextBridgeDiscovery {
        url: url.clone(),
        bridge_url: url,
        chat_url,
        host: IDE_CONTEXT_BRIDGE_HOST.to_string(),
        port,
        path: IDE_CONTEXT_BRIDGE_PATH.to_string(),
        chat_path: IDE_CONTEXT_CHAT_BRIDGE_PATH.to_string(),
        pid: std::process::id(),
        updated_at: now_iso(),
        token: token.to_string(),
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

async fn bind_ide_context_bridge_listener() -> Result<(tokio::net::TcpListener, u16), String> {
    let mut errors = Vec::new();
    for port in IDE_CONTEXT_BRIDGE_BASE_PORT..=IDE_CONTEXT_BRIDGE_MAX_PORT {
        let addr = format!("{}:{}", IDE_CONTEXT_BRIDGE_HOST, port);
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
        IDE_CONTEXT_BRIDGE_HOST,
        IDE_CONTEXT_BRIDGE_BASE_PORT,
        IDE_CONTEXT_BRIDGE_MAX_PORT,
        errors.join("; ")
    ))
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

fn ide_chat_conversation_list(state: &AppState) -> Result<Value, String> {
    let summaries = conversation_service()
        .list_unarchived_conversation_summaries(state)?
        .summaries
        .into_iter()
        .map(|mut item| {
            item.runtime_state = ide_chat_runtime_for_conversation(state, &item.conversation_id)
                .map(|snapshot| snapshot.runtime_state);
            item
        })
        .collect::<Vec<_>>();
    let persona = ide_chat_persona_payload(state, None)?;
    Ok(serde_json::json!({ "conversations": summaries, "persona": persona }))
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
        vec![MessagePart::Text { text: text.clone() }]
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
    let agent_id = conversation.agent_id.trim().to_string();
    if agent_id.is_empty() {
        return Err("Missing session.agentId".to_string());
    }
    let chat_key = inflight_chat_key(&agent_id, Some(conversation_id));
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
    let aborted_delegate_children = abort_delegate_runtime_descendants_by_parent_session(state, &chat_key)?;
    let cleared_queue_count = clear_conversation_queue(
        state,
        conversation_id,
        "消息已因 VS Code 侧边栏中断被清出队列",
    )?;
    let _ = release_conversation_processing_claim(state, conversation_id);
    let _ = set_conversation_runtime_state(state, conversation_id, MainSessionState::Idle);
    let _ = set_conversation_remote_im_activation_sources(state, conversation_id, Vec::new());
    let partial_assistant_text = input.partial_assistant_text.trim().to_string();
    let partial_reasoning_standard = input.partial_reasoning_standard.trim().to_string();
    let partial_reasoning_inline = input.partial_reasoning_inline.trim().to_string();
    let completed_tool_history = inflight_completed_tool_history(state, &chat_key)?;
    let should_persist = !partial_assistant_text.is_empty()
        || !partial_reasoning_standard.is_empty()
        || !partial_reasoning_inline.is_empty()
        || !completed_tool_history.is_empty();
    let mut persisted = false;
    let mut assistant_message = None::<ChatMessage>;
    if should_persist {
        let result = conversation_service().persist_stop_chat_partial_message(
            state,
            Some(conversation_id),
            Some(conversation.department_id.as_str()),
            &agent_id,
            &partial_assistant_text,
            &partial_reasoning_standard,
            &partial_reasoning_inline,
            &completed_tool_history,
        )?;
        persisted = result.persisted;
        assistant_message = result.assistant_message;
    }
    clear_inflight_completed_tool_history(state, &chat_key)?;
    let stop_result = StopChatResult {
        aborted: aborted_chat || aborted_tool || aborted_delegate_children > 0,
        persisted,
        conversation_id: Some(conversation_id.to_string()),
        assistant_text: partial_assistant_text,
        reasoning_standard: partial_reasoning_standard,
        reasoning_inline: partial_reasoning_inline,
        assistant_message,
    };
    if stop_result.persisted {
        emit_stop_chat_round_completed_event(state, conversation_id, &stop_result);
    }
    let payload = serde_json::json!({
        "conversationId": conversation_id,
        "status": "stopped",
        "aborted": stop_result.aborted,
        "persisted": stop_result.persisted,
        "clearedQueueCount": cleared_queue_count,
        "assistantText": stop_result.assistant_text,
        "reasoningStandard": stop_result.reasoning_standard,
        "reasoningInline": stop_result.reasoning_inline,
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
    let agent_id = input.agent_id.trim().to_string();
    let message_id = input.message_id.trim().to_string();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    if agent_id.is_empty() {
        return Err("agentId is required".to_string());
    }
    if message_id.is_empty() {
        return Err("messageId is required".to_string());
    }

    let started_at = std::time::Instant::now();
    let request = RewindConversationInput {
        session: SessionSelector {
            api_config_id: None,
            department_id: None,
            agent_id,
            conversation_id: Some(conversation_id.clone()),
        },
        message_id: message_id.clone(),
        undo_apply_patch: input.undo_apply_patch,
    };
    let result = conversation_service().rewind_conversation_from_message(
        state,
        &request,
        &request.session.agent_id,
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

fn ide_chat_select_model(state: &AppState, app: &AppHandle, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatSelectModelInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let api_config_id = input.api_config_id.trim();
    if api_config_id.is_empty() {
        return Err("apiConfigId is required".to_string());
    }
    let conversation = state_read_conversation_cached(state, conversation_id)?;
    let department_id = conversation.department_id.trim();
    if department_id.is_empty() {
        return Err("当前会话没有所属部门，无法切换模型。".to_string());
    }
    let mut config = state_read_config_cached(state)?;
    let selected_api = config
        .api_configs
        .iter()
        .find(|item| item.id.trim() == api_config_id)
        .ok_or_else(|| format!("API config '{api_config_id}' not found."))?;
    if !is_text_chat_api(selected_api) {
        return Err(format!("API config '{api_config_id}' does not support chat text."));
    }
    let (department_primary_api_config_id, assistant_department_changed) = {
        let Some(target_department) = config
            .departments
            .iter_mut()
            .find(|item| item.id.trim() == department_id)
        else {
            return Err(format!("Department '{department_id}' not found."));
        };
        let mut next_ids = department_api_config_ids(target_department);
        if next_ids.first().map(|item| item.trim()) != Some(api_config_id) {
            next_ids.retain(|item| !item.trim().eq_ignore_ascii_case(api_config_id));
            if next_ids.is_empty() {
                next_ids.push(api_config_id.to_string());
            } else {
                next_ids[0] = api_config_id.to_string();
            }
        }
        let mut seen = std::collections::HashSet::<String>::new();
        target_department.api_config_ids = next_ids
            .into_iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .filter(|item| seen.insert(item.to_ascii_lowercase()))
            .collect::<Vec<_>>();
        target_department.api_config_id = target_department
            .api_config_ids
            .first()
            .cloned()
            .unwrap_or_default();
        target_department.updated_at = now_iso();
        (
            target_department.api_config_id.clone(),
            target_department.id == ASSISTANT_DEPARTMENT_ID || target_department.is_built_in_assistant,
        )
    };
    if assistant_department_changed {
        config.assistant_department_api_config_id = department_primary_api_config_id;
    }
    config.selected_api_config_id = api_config_id.to_string();
    state_write_config_cached(state, &config)?;
    let data = state_read_agents_runtime_snapshot(state)?;
    let runtime_config = runtime_config_with_private_organization(state, &config, &data)?;
    let _ = app.emit("easy-call:config-updated", &runtime_config);
    let updated_conversation = state_read_conversation_cached(state, conversation_id)?;
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
    serde_json::to_value(task_store_create_task(&state.data_path, &input)?)
        .map_err(|err| format!("Serialize task create result failed: {err}"))
}

async fn ide_chat_task_dispatch_now(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<TaskDispatchNowInput>(params)?;
    let task = task_store_get_task_record(&state.data_path, input.task_id.trim())?;
    let Some(session) = task_resolve_dispatch_session(state, &task)? else {
        return Ok(serde_json::json!(false));
    };
    task_dispatch_due_task(state, &task, &session).await?;
    Ok(serde_json::json!(true))
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
    let result = match request.method.as_str() {
        "conversation.list" => ide_chat_conversation_list(state),
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
        "conversation.rewind" => ide_chat_rewind_conversation(state, request.params).await,
        "conversation.branchFromSelection" => ide_chat_branch_conversation(state, request.params).await,
        "delegate.submit" => ide_chat_submit_delegate(state, request.params).await,
        "task.create" => ide_chat_task_create(state, request.params),
        "task.dispatchNow" => ide_chat_task_dispatch_now(state, request.params).await,
        "conversation.compactPreview" => ide_chat_compact_preview(state, request.params),
        "conversation.compact" => ide_chat_compact_conversation(state, request.params).await,
        "model.list" => ide_chat_model_list(state, request.params),
        "model.select" => ide_chat_select_model(state, app, request.params),
        "workspace.permission" => ide_chat_workspace_permission(state, request.params),
        "workspace.permission.select" => ide_chat_select_workspace_permission(state, request.params),
        "workspace.list" => ide_chat_workspace_list(state, request.params),
        "ideContext.query" => ide_chat_parse_params::<IdeContextWorkspaceQueryInput>(request.params)
            .and_then(|input| serde_json::to_value(query_ide_context_references_internal(input, ide_context_runtime)?)
                .map_err(|err| format!("serialize IDE context query result failed: {err}"))),
        "workspace.layout.save" => ide_chat_workspace_layout_save(state, request.params),
        "terminalApproval.resolve" => ide_chat_resolve_terminal_approval(state, request.params),
        "conversation.planMode.set" => ide_chat_set_conversation_plan_mode(state, request.params),
        "conversation.plan.confirm" => ide_chat_confirm_plan(state, request.params).await,
        "conversation.plan.readFile" => ide_chat_read_plan_file(state, request.params),
        "settings.open" => ide_chat_open_settings(app),
        "chat.send" => ide_chat_send_message(state, request.params),
        "chat.stop" => ide_chat_stop_conversation(state, request.params),
        "toolReview.reports.list" => ide_chat_tool_review_reports(state, request.params),
        "toolReview.report.delete" => ide_chat_tool_review_delete_report(state, request.params),
        "toolReview.commitOptions.list" => ide_chat_tool_review_commit_options(state, request.params).await,
        "toolReview.code.submit" => ide_chat_tool_review_submit_code(state, request.params).await,
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
    let shutdown_token = ide_context_bridge_shutdown_token().clone();
    tauri::async_runtime::spawn(async move {
        let (listener, port) = match bind_ide_context_bridge_listener().await {
            Ok(result) => result,
            Err(err) => {
                IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
                clear_ide_context_bridge_discovery();
                eprintln!("[IDE 上下文桥] 监听失败: {}", err);
                return;
            }
        };
        let bridge_url = ide_context_bridge_url(port);
        let token = match ide_context_issue_bridge_token(&ide_context_runtime) {
            Ok(token) => token,
            Err(err) => {
                IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
                clear_ide_context_bridge_discovery();
                eprintln!("[IDE 上下文桥] 初始化鉴权 token 失败: {}", err);
                return;
            }
        };
        if let Err(err) = publish_ide_context_bridge_discovery(port, &token) {
            IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
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
}

pub(crate) async fn shutdown_ide_context_bridge_server() {
    if !IDE_CONTEXT_BRIDGE_STARTED.load(Ordering::SeqCst) {
        clear_ide_context_bridge_discovery();
        return;
    }
    ide_context_bridge_shutdown_token().cancel();
    clear_ide_context_bridge_discovery();
    if let Some(clients) = IDE_CONTEXT_CHAT_CLIENTS.get() {
        if let Ok(mut clients) = clients.lock() {
            clients.clear();
        }
    }
    for _ in 0..40 {
        if !IDE_CONTEXT_BRIDGE_STARTED.load(Ordering::SeqCst) {
            eprintln!("[IDE 上下文桥] 已停止");
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
    eprintln!("[IDE 上下文桥] 停止等待超时，已清理发现文件");
}

async fn ide_context_ws_handle_connection(
    stream: tokio::net::TcpStream,
    peer_addr: std::net::SocketAddr,
    port: u16,
    app: AppHandle,
    state: AppState,
    ide_context_runtime: IdeContextRuntime,
) {
    let path_holder = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let path_holder_clone = path_holder.clone();
    let ws_stream = match accept_hdr_async(stream, move |request: &Request, response: Response| {
        if let Ok(mut slot) = path_holder_clone.lock() {
            *slot = request.uri().path().to_string();
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
    let mut authenticated = false;
    let _ = ws_sender
        .send(tokio_tungstenite::tungstenite::Message::Text(
            serde_json::json!({"type": "ready", "path": IDE_CONTEXT_BRIDGE_PATH, "authRequired": true})
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
                            match ide_context_consume_bridge_token(
                                &ide_context_runtime,
                                input.auth_token.as_deref().unwrap_or(""),
                            ) {
                                Ok(_token) => {
                                    authenticated = true;
                                }
                                Err((err, refreshed_token)) => {
                                    if let Some(refreshed_token) = refreshed_token.as_deref() {
                                        if let Err(publish_err) =
                                            publish_ide_context_bridge_discovery(port, refreshed_token)
                                        {
                                            eprintln!(
                                                "[IDE 上下文桥] 过期后重写发现 token 失败: {}",
                                                publish_err
                                            );
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
            "authRequired": false,
        },
    }));
    if let Ok(mut clients) = ide_context_chat_clients().lock() {
        clients.insert(client_id.clone(), outbound_tx.clone());
    }
    let mut opened_conversation_id: Option<String> = None;
    while let Some(message) = ws_receiver.next().await {
        match message {
            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                let response = match serde_json::from_str::<IdeChatJsonRpcRequest>(&text) {
                    Ok(request) => ide_chat_handle_jsonrpc_request(
                        request,
                        &state,
                        &app,
                        &ide_context_runtime,
                        &client_id,
                        &mut opened_conversation_id,
                    )
                    .await,
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
    fn ide_context_bridge_tokens_allow_concurrent_consumers_until_expiry() {
        let runtime = IdeContextRuntime::new();
        let token = ide_context_issue_bridge_token(&runtime).expect("issue token");

        let next_token = ide_context_consume_bridge_token(&runtime, &token).expect("first consume");
        assert_eq!(next_token, token);

        let second_next =
            ide_context_consume_bridge_token(&runtime, &token).expect("second consume with same token");
        assert_eq!(second_next, token);
    }

    #[test]
    fn ide_context_bridge_tokens_reject_unknown_token() {
        let runtime = IdeContextRuntime::new();
        let _ = ide_context_issue_bridge_token(&runtime).expect("issue token");
        let err = ide_context_consume_bridge_token(&runtime, "bad-token").expect_err("invalid token");
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

        let err = ide_context_consume_bridge_token(&runtime, "expired-token")
            .expect_err("expired token should refresh discovery");
        assert!(err.0.contains("expired"));
        let refreshed = err.1.expect("should issue refreshed token");
        let auth = runtime.bridge_auth.lock().expect("lock auth");
        assert!(auth.valid_tokens.contains_key(&refreshed));
    }
}

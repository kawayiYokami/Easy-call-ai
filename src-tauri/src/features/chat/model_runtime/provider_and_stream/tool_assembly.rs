fn tool_manifest_item(
    source: &str,
    name: &str,
    enabled: bool,
    attached: bool,
    reason: Option<String>,
) -> Value {
    serde_json::json!({
        "source": source,
        "name": name,
        "enabled": enabled,
        "attached": attached,
        "reason": reason
    })
}

fn tool_schema_cache_store() -> &'static Mutex<Option<Vec<ProviderToolDefinition>>> {
    static STORE: OnceLock<Mutex<Option<Vec<ProviderToolDefinition>>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(None))
}

fn tool_schema_definition_to_manifest_item(definition: &ProviderToolDefinition) -> Value {
    tool_manifest_item("schema_cache", &definition.name, true, true, None)
}

fn runtime_tool_names_for_log(tool_assembly: &RuntimeToolAssembly) -> Option<Value> {
    let mut names = Vec::<String>::new();
    for item in &tool_assembly.tool_manifest {
        let Some(name) = item
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let enabled = item.get("enabled").and_then(Value::as_bool).unwrap_or(true);
        let attached = item.get("attached").and_then(Value::as_bool).unwrap_or(true);
        if enabled && attached && !names.iter().any(|existing| existing == name) {
            names.push(name.to_string());
        }
    }
    if names.is_empty() {
        return None;
    }
    Some(Value::Array(
        names
            .into_iter()
            .map(|name| serde_json::json!({ "name": name }))
            .collect(),
    ))
}

fn operate_provider_tool_definition() -> ProviderToolDefinition {
    ProviderToolDefinition::new(
        MCP_OPERATE_TOOL_NAME,
        "统一桌面脚本工具。入参只有 script:string，一行一个动作。\n可用语法：\nmouse <button> click @x,y [repeat=n] [delay=s] [pre_delay=s] [press=s]\nmouse scroll_up [repeat=n] [delay=s] [pre_delay=s]\nmouse scroll_down [repeat=n] [delay=s] [pre_delay=s]\nkey <combo> [repeat=n] [delay=s] [pre_delay=s] [press=s]\ntext \"内容\" [repeat=n] [delay=s] [pre_delay=s]\nwait <seconds>\nscreenshot [focused_window] [region=@x,y,w,h] [save=\"绝对路径\"] [quality=1..100]\n参数说明：button=left|right|middle|back|forward；combo 用 + 连接按键，如 Control+L、Control+Shift+P、Enter；x/y/w/h 为 0~1 百分比坐标；repeat=1~100；delay/pre_delay/press=0~300 秒；save 必须是绝对路径；quality 默认 75。规则：screenshot 对模型只保留最新一张，旧画面视为已经离去。",
        serde_json::json!({
            "type": "object",
            "properties": {
                "script": {
                    "type": "string",
                    "description": "桌面脚本文本，一行一个动作。"
                },
                "timeout_ms": {
                    "type": "integer",
                    "minimum": 1,
                    "default": 300000,
                    "description": "本次桌面脚本工具调用的超时时间，单位毫秒；未指定时默认 300000ms。长时间 wait 或自动化脚本应显式传入足够大的值。"
                }
            },
            "required": ["script"]
        }),
    )
}

fn read_provider_tool_definition() -> ProviderToolDefinition {
    ProviderToolDefinition::new(
        READ_TOOL_NAME,
        "读取本地文档内容。支持文本、代码、PDF 与 Office 文件；path 必须是绝对路径；对文本、代码、Office 等非 PDF 内容，offset 表示跳过行，limit 表示返回行数；对 PDF 则代表页。图片、音频、视频请改用 read_media。",
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "要读取的本地文件绝对路径。"
                },
                "offset": {
                    "type": "integer",
                    "minimum": 0,
                    "description": "跳过数，默认从 0 开始。"
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "返回数。"
                }
            },
            "required": ["path"]
        }),
    )
}

fn read_media_provider_tool_definition() -> ProviderToolDefinition {
    ProviderToolDefinition::new(
        READ_MEDIA_TOOL_NAME,
        "解析本地图片、音频或视频。path 必须是绝对路径；description 用于告诉多模态分析模型重点关注什么。",
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "要解析的本地媒体文件绝对路径。"
                },
                "description": {
                    "type": "string",
                    "description": "解析侧重点，例如要看什么、听什么、提取什么。"
                }
            },
            "required": ["path"]
        }),
    )
}

const OPERATE_TOOL_DEFAULT_TIMEOUT_MS: u64 = 300_000;

fn operate_tool_timeout_override(args_json: &str) -> std::time::Duration {
    let timeout_ms = parse_runtime_tool_args::<OperateRequest>(args_json)
        .ok()
        .and_then(|args| args.timeout_ms)
        .unwrap_or(OPERATE_TOOL_DEFAULT_TIMEOUT_MS)
        .max(1);
    std::time::Duration::from_millis(timeout_ms)
}

fn build_global_tool_schema_cache(state: &AppState) -> Vec<ProviderToolDefinition> {
    let preview_session_id = "__tool_schema_cache__".to_string();
    let _preview_api_id = "__tool_schema_cache__".to_string();
    let preview_agent_id = DEFAULT_AGENT_ID.to_string();
    let preview_memory_context = build_memory_agent_context(&preview_agent_id, false, true)
        .unwrap_or(MemoryAgentContext {
            owner_agent_id: None,
            effective_agent_id: preview_agent_id.clone(),
            private_memory_enabled: false,
            recall_enabled: true,
        });
    let mut definitions = vec![
        BuiltinFetchTool { app_state: state.clone() }.provider_tool_definition(),
        BuiltinBingSearchTool { app_state: state.clone() }.provider_tool_definition(),
        BuiltinRememberTool {
            app_state: state.clone(),
            memory_context: preview_memory_context.clone(),
        }
        .provider_tool_definition(),
        BuiltinRecallTool {
            app_state: state.clone(),
            memory_context: preview_memory_context,
        }
        .provider_tool_definition(),
        operate_provider_tool_definition(),
        BuiltinReloadTool { app_state: state.clone() }.provider_tool_definition(),
        read_provider_tool_definition(),
        read_media_provider_tool_definition(),
        BuiltinTerminalExecTool {
            app_state: state.clone(),
            session_id: preview_session_id.clone(),
        }
        .provider_tool_definition(),
        BuiltinWriteFileTool {
            app_state: state.clone(),
            session_id: preview_session_id.clone(),
        }
        .provider_tool_definition(),
        BuiltinDeleteFileTool {
            app_state: state.clone(),
            session_id: preview_session_id.clone(),
        }
        .provider_tool_definition(),
        BuiltinUpdateFileTool {
            app_state: state.clone(),
            session_id: preview_session_id.clone(),
        }
        .provider_tool_definition(),
        BuiltinMoveFileTool {
            app_state: state.clone(),
            session_id: preview_session_id.clone(),
        }
        .provider_tool_definition(),
        BuiltinApplyPatchTool {
            app_state: state.clone(),
            session_id: preview_session_id.clone(),
        }
        .provider_tool_definition(),
        BuiltinPlanTool {
            app_state: state.clone(),
            session_id: preview_session_id.clone(),
        }
        .provider_tool_definition(),
        BuiltinTodoTool {
            app_state: state.clone(),
            session_id: preview_session_id.clone(),
        }
        .provider_tool_definition(),
        BuiltinCreateGoalTool {
            app_state: state.clone(),
            session_id: preview_session_id.clone(),
        }
        .provider_tool_definition(),
        BuiltinUpdateGoalTool {
            app_state: state.clone(),
            session_id: preview_session_id.clone(),
        }
        .provider_tool_definition(),
        BuiltinGetSessionTool {
            app_state: state.clone(),
            session_id: preview_session_id.clone(),
        }
        .provider_tool_definition(),
        BuiltinInformSessionTool {
            app_state: state.clone(),
            session_id: preview_session_id.clone(),
        }
        .provider_tool_definition(),
        BuiltinTaskTool {
            app_state: state.clone(),
            session_id: preview_session_id.clone(),
            api_config_id: String::new(),
            executor_department_id: String::new(),
            executor_agent_id: preview_agent_id.clone(),
        }
        .provider_tool_definition(),
        BuiltinDelegateTool {
            app_state: state.clone(),
            session_id: preview_session_id.clone(),
            source_agent_id: preview_agent_id,
            source_department_id: String::new(),
        }
        .provider_tool_definition(),
        BuiltinMemeTool { app_state: state.clone() }.provider_tool_definition(),
        BuiltinContactReplyTool {
            app_state: state.clone(),
            session_id: preview_session_id.clone(),
        }
        .provider_tool_definition(),
        BuiltinContactSendFilesTool {
            app_state: state.clone(),
            session_id: preview_session_id,
        }
        .provider_tool_definition(),
        BuiltinContactNoReplyTool.provider_tool_definition(),
    ];

    match load_workspace_mcp_servers(state) {
        Ok(servers) => {
            for server in servers.into_iter().filter(|server| server.enabled) {
                for tool in list_tools_from_runtime(&server) {
                    definitions.push(ProviderToolDefinition::new(
                        tool.tool_name,
                        tool.description,
                        tool.parameters,
                    ));
                }
            }
        }
        Err(err) => runtime_log_warn(format!("[工具Schema缓存] 加载 MCP 配置失败: {err}")),
    }

    definitions.sort_by(|a, b| a.name.cmp(&b.name));
    definitions.dedup_by(|a, b| a.name == b.name);
    definitions
}

fn refresh_global_tool_schema_cache(state: &AppState) -> Vec<ProviderToolDefinition> {
    let definitions = build_global_tool_schema_cache(state);
    match tool_schema_cache_store().lock() {
        Ok(mut guard) => {
            *guard = Some(definitions.clone());
        }
        Err(err) => runtime_log_warn(format!("[工具Schema缓存] 刷新失败，缓存锁已损坏: {err}")),
    }
    definitions
}

fn clear_global_tool_schema_cache() {
    match tool_schema_cache_store().lock() {
        Ok(mut guard) => {
            *guard = None;
        }
        Err(err) => runtime_log_warn(format!("[工具Schema缓存] 清空失败，缓存锁已损坏: {err}")),
    }
}

fn read_global_tool_schema_cache(_state: Option<&AppState>) -> Vec<ProviderToolDefinition> {
    match tool_schema_cache_store().lock() {
        Ok(guard) => {
            if let Some(definitions) = guard.as_ref() {
                return definitions.clone();
            }
        }
        Err(err) => runtime_log_warn(format!("[工具Schema缓存] 读取失败，缓存锁已损坏: {err}")),
    }
    Vec::new()
}

fn resolve_runtime_tool_current_department<'a>(
    app_config: &'a AppConfig,
    executor_department_id: Option<&str>,
) -> Option<&'a DepartmentConfig> {
    executor_department_id
        .map(str::trim)
        .filter(|department_id| !department_id.is_empty())
        .and_then(|department_id| department_by_id(app_config, department_id))
}

#[derive(Debug, Clone, Copy)]
struct RuntimeToolPolicy {
    remote_im_contact_conversation: bool,
}

impl RuntimeToolPolicy {
    fn from_conversation(conversation: Option<&Conversation>) -> Self {
        Self {
            remote_im_contact_conversation: conversation
                .map(conversation_is_remote_im_contact)
                .unwrap_or(false),
        }
    }

    fn tool_allowed(self, tool_name: &str) -> bool {
        match tool_name.trim() {
            "contact_reply" | "contact_send_files" | "contact_no_reply" => {
                self.remote_im_contact_conversation
            }
            _ => true,
        }
    }
}

fn runtime_tool_policy_from_session(
    app_state: Option<&AppState>,
    tool_session_id: &str,
) -> RuntimeToolPolicy {
    let Some(state) = app_state else {
        return RuntimeToolPolicy::from_conversation(None);
    };
    let Ok(conversation_id) = goal_tool_conversation_id(tool_session_id) else {
        return RuntimeToolPolicy::from_conversation(None);
    };
    let conversation = state_read_conversation_cached(state, &conversation_id)
        .ok()
        .or_else(|| {
            delegate_runtime_thread_conversation_get(state, &conversation_id)
                .ok()
                .flatten()
        });
    RuntimeToolPolicy::from_conversation(conversation.as_ref())
}

async fn assemble_runtime_tools(
    app_config: &AppConfig,
    selected_api: &ApiConfig,
    agent: &AgentProfile,
    app_state: Option<&AppState>,
    tool_session_id: &str,
    executor_department_id: Option<&str>,
) -> Result<RuntimeToolAssembly, String> {
    let current_department =
        resolve_runtime_tool_current_department(app_config, executor_department_id);
    let delegate_unavailable_reason =
        delegate_builtin_tool_unavailable_reason(app_config, current_department);
    let runtime_tool_policy = runtime_tool_policy_from_session(app_state, tool_session_id);
    let base_tool_definitions = read_global_tool_schema_cache(app_state)
        .into_iter()
        .filter(|definition| {
            definition.name != "delegate" || delegate_unavailable_reason.is_none()
        })
        .collect::<Vec<_>>();
    let active_tool_definitions = base_tool_definitions
        .iter()
        .filter(|definition| runtime_tool_policy.tool_allowed(&definition.name))
        .cloned()
        .collect::<Vec<_>>();
    let mut tool_manifest = active_tool_definitions
        .iter()
        .map(tool_schema_definition_to_manifest_item)
        .collect::<Vec<_>>();
    if let Some(reason) = delegate_unavailable_reason.clone() {
        tool_manifest.push(tool_manifest_item(
            "runtime_policy",
            "delegate",
            false,
            false,
            Some(reason),
        ));
    }
    let mut tools: Vec<Box<dyn RuntimeToolDyn>> = Vec::new();
    if selected_api.enable_tools {
        push_runtime_tool_executors(
            &mut tools,
            app_state,
            selected_api.id.as_str(),
            agent,
            tool_session_id,
            delegate_unavailable_reason.is_none(),
            selected_api.enable_image,
            executor_department_id,
            runtime_tool_policy,
        )?;
    }
    Ok(RuntimeToolAssembly {
        tools,
        tool_definitions: active_tool_definitions,
        tool_manifest,
        unavailable_tool_notices: Vec::new(),
    })
}

fn push_runtime_tool_executors(
    tools: &mut Vec<Box<dyn RuntimeToolDyn>>,
    app_state: Option<&AppState>,
    api_config_id: &str,
    agent: &AgentProfile,
    tool_session_id: &str,
    enable_delegate: bool,
    model_supports_image: bool,
    executor_department_id: Option<&str>,
    runtime_tool_policy: RuntimeToolPolicy,
) -> Result<(), String> {
    let state = app_state
        .ok_or_else(|| "runtime tool execution requires app state".to_string())?
        .clone();
    let memory_context = memory_agent_context_from_agent(agent)?;
    tools.push(Box::new(BuiltinFetchTool { app_state: state.clone() }));
    tools.push(Box::new(BuiltinBingSearchTool { app_state: state.clone() }));
    tools.push(Box::new(BuiltinRememberTool {
        app_state: state.clone(),
        memory_context: memory_context.clone(),
    }));
    if memory_context.recall_enabled {
        tools.push(Box::new(BuiltinRecallTool {
            app_state: state.clone(),
            memory_context,
        }));
    }
    tools.push(Box::new(BuiltinOperateTool { model_supports_image }));
    tools.push(Box::new(BuiltinReloadTool { app_state: state.clone() }));
    tools.push(Box::new(BuiltinReadFileTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
        api_config_id: api_config_id.to_string(),
    }));
    tools.push(Box::new(BuiltinReadMediaTool {
        app_state: state.clone(),
    }));
    tools.push(Box::new(BuiltinTerminalExecTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
    }));
    tools.push(Box::new(BuiltinWriteFileTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
    }));
    tools.push(Box::new(BuiltinDeleteFileTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
    }));
    tools.push(Box::new(BuiltinUpdateFileTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
    }));
    tools.push(Box::new(BuiltinMoveFileTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
    }));
    tools.push(Box::new(BuiltinApplyPatchTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
    }));
    tools.push(Box::new(BuiltinPlanTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
    }));
    tools.push(Box::new(BuiltinTodoTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
    }));
    tools.push(Box::new(BuiltinCreateGoalTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
    }));
    tools.push(Box::new(BuiltinUpdateGoalTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
    }));
    tools.push(Box::new(BuiltinGetSessionTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
    }));
    tools.push(Box::new(BuiltinInformSessionTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
    }));
    tools.push(Box::new(BuiltinTaskTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
        api_config_id: api_config_id.to_string(),
        executor_department_id: executor_department_id
            .map(str::trim)
            .filter(|department_id| !department_id.is_empty())
            .unwrap_or_default()
            .to_string(),
        executor_agent_id: agent.id.trim().to_string(),
    }));
    if enable_delegate {
        tools.push(Box::new(BuiltinDelegateTool {
            app_state: state.clone(),
            session_id: tool_session_id.to_string(),
            source_agent_id: agent.id.trim().to_string(),
            source_department_id: executor_department_id
                .map(str::trim)
                .filter(|department_id| !department_id.is_empty())
                .unwrap_or_default()
                .to_string(),
        }));
    }
    tools.push(Box::new(BuiltinMemeTool { app_state: state.clone() }));
    if runtime_tool_policy.tool_allowed("contact_reply") {
        tools.push(Box::new(BuiltinContactReplyTool {
            app_state: state.clone(),
            session_id: tool_session_id.to_string(),
        }));
        tools.push(Box::new(BuiltinContactSendFilesTool {
            app_state: state.clone(),
            session_id: tool_session_id.to_string(),
        }));
        tools.push(Box::new(BuiltinContactNoReplyTool));
    }
    push_cached_mcp_runtime_tools(tools, &state, runtime_tool_policy);
    Ok(())
}

fn push_cached_mcp_runtime_tools(
    tools: &mut Vec<Box<dyn RuntimeToolDyn>>,
    state: &AppState,
    runtime_tool_policy: RuntimeToolPolicy,
) {
    let servers = match load_workspace_mcp_servers(state) {
        Ok(servers) => servers,
        Err(err) => {
            runtime_log_warn(format!("[MCP] 装配 MCP 工具执行器失败，加载配置失败: {err}"));
            return;
        }
    };
    let existing_names = tools.iter().map(|tool| tool.name()).collect::<HashSet<_>>();
    let mut added_names = HashSet::<String>::new();
    for server in servers.into_iter().filter(|server| server.enabled) {
        for descriptor in list_tools_from_runtime(&server)
            .into_iter()
            .filter(|tool| tool.enabled && runtime_tool_policy.tool_allowed(&tool.tool_name))
        {
            if existing_names.contains(&descriptor.tool_name) || !added_names.insert(descriptor.tool_name.clone()) {
                continue;
            }
            let input_schema = Arc::new(match descriptor.parameters {
                Value::Object(map) => map,
                _ => serde_json::Map::new(),
            });
            let definition = rmcp::model::Tool::new(
                descriptor.tool_name.clone(),
                descriptor.description,
                input_schema,
            );
            tools.push(Box::new(CachedMcpRuntimeTool {
                server: server.clone(),
                definition,
            }));
        }
    }
}

#[derive(Debug, Clone)]
struct BuiltinOperateTool {
    model_supports_image: bool,
}

#[derive(Debug, Clone)]
struct BuiltinReadFileTool {
    app_state: AppState,
    session_id: String,
    api_config_id: String,
}

#[derive(Debug, Clone)]
struct BuiltinReadMediaTool {
    app_state: AppState,
}

impl RuntimeToolMetadata for BuiltinOperateTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        operate_provider_tool_definition()
    }
}

impl RuntimeJsonTool for BuiltinOperateTool {
    const NAME: &'static str = MCP_OPERATE_TOOL_NAME;
    type Args = OperateRequest;
    type Error = ToolInvokeError;

    fn timeout_override(args_json: &str) -> Option<std::time::Duration> {
        Some(operate_tool_timeout_override(args_json))
    }

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        let model_supports_image = self.model_supports_image;
        Box::pin(async move {
            // 如果模型不支持图片，检查脚本中是否包含 screenshot 动作
            if !model_supports_image && script_contains_screenshot(&args.script) {
                return Err(ToolInvokeError::from(
                    "你的驱动模型并不支持图片，请放弃该功能".to_string(),
                ));
            }
            let args_value = serde_json::to_value(&args).unwrap_or(Value::Null);
            runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.start name=operate args={}",
                debug_value_snippet(&args_value, 240)
            ));
            let result = run_operate_tool(args)
                .await
                .map_err(|err| ToolInvokeError::from(err.message))
                .and_then(|output| {
                    serde_json::to_value(output)
                        .map_err(|err| ToolInvokeError::from(format!("Serialize operate output failed: {err}")))
                });
            match &result {
                Ok(v) => runtime_log_debug(format!(
                    "[TOOL-DEBUG] execute_builtin_tool.ok name=operate result={}",
                    debug_value_snippet(v, 240)
                )),
                Err(err) => eprintln!("[工具执行] 内置工具 operate 执行失败: 错误={err}"),
            }
            result
        })
    }
}

impl RuntimeToolMetadata for BuiltinReadFileTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        read_provider_tool_definition()
    }
}

impl RuntimeJsonTool for BuiltinReadFileTool {
    const NAME: &'static str = READ_TOOL_NAME;
    type Args = ReadFileRequest;
    type Error = ToolInvokeError;

    fn timeout_override(_args_json: &str) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(300))
    }

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
            let args_value = serde_json::to_value(&args).unwrap_or(Value::Null);
            runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.start name=read args={}",
                debug_value_snippet(&args_value, 240)
            ));
            let result = builtin_read_file(
                &self.app_state,
                &self.session_id,
                &self.api_config_id,
                args,
            )
            .await
            .map_err(ToolInvokeError::from);
            match &result {
                Ok(v) => runtime_log_debug(format!(
                    "[TOOL-DEBUG] execute_builtin_tool.ok name=read result={}",
                    debug_value_snippet(v, 240)
                )),
                Err(err) => eprintln!("[工具执行] 内置工具 read 执行失败: 错误={err}"),
            }
            result
        })
    }
}

impl RuntimeToolMetadata for BuiltinReadMediaTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        read_media_provider_tool_definition()
    }
}

impl RuntimeJsonTool for BuiltinReadMediaTool {
    const NAME: &'static str = READ_MEDIA_TOOL_NAME;
    type Args = ReadMediaToolArgs;
    type Error = ToolInvokeError;

    fn timeout_override(_args_json: &str) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(60 * 60))
    }

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
            let args_value = serde_json::to_value(&args).unwrap_or(Value::Null);
            runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.start name=read_media args={}",
                debug_value_snippet(&args_value, 240)
            ));
            let result = builtin_read_media(
                &self.app_state,
                ReadMediaRequest {
                    path: args.path,
                    description: args.description,
                },
            )
            .await
            .map_err(ToolInvokeError::from);
            match &result {
                Ok(v) => runtime_log_debug(format!(
                    "[TOOL-DEBUG] execute_builtin_tool.ok name=read_media result={}",
                    debug_value_snippet(v, 240)
                )),
                Err(err) => eprintln!("[工具执行] 内置工具 read_media 执行失败: 错误={err}"),
            }
            result
        })
    }
}

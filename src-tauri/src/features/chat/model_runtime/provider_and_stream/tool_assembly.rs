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
        "读取本地文件内容。自动识别文本、图片、PDF 与 Office 文件；path 必须是绝对路径；对文本、代码、Office 等非 PDF 内容，offset 表示跳过行，limit 表示返回行数；对 PDF 则代表页。",
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
    let preview_memory_context = build_memory_agent_context(&preview_agent_id, false)
        .unwrap_or(MemoryAgentContext {
            owner_agent_id: None,
            effective_agent_id: preview_agent_id.clone(),
            private_memory_enabled: false,
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
        BuiltinTerminalExecTool {
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
        BuiltinTaskTool {
            app_state: state.clone(),
            session_id: preview_session_id.clone(),
        }
        .provider_tool_definition(),
        BuiltinDelegateTool {
            app_state: state.clone(),
            session_id: preview_session_id.clone(),
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
    app_state: Option<&AppState>,
    agent: &AgentProfile,
    tool_session_id: &str,
) -> Option<&'a DepartmentConfig> {
    let conversation_department_id = app_state.and_then(|state| {
        let (_, _, conversation_id) = delegate_parse_session_parts(tool_session_id);
        let conversation_id = conversation_id?;
        if let Ok(Some(conversation)) = delegate_runtime_thread_conversation_get(&state, &conversation_id) {
            let department_id = conversation.department_id.trim();
            if !department_id.is_empty() {
                return Some(department_id.to_string());
            }
        }
        let conversation = state_read_conversation_cached(state, &conversation_id).ok()?;
        let department_id = conversation.department_id.trim();
        if department_id.is_empty() {
            None
        } else {
            Some(department_id.to_string())
        }
    });
    conversation_department_id
        .as_deref()
        .and_then(|department_id| department_by_id(app_config, department_id))
        .or_else(|| department_for_agent_id(app_config, &agent.id))
}

async fn assemble_runtime_tools(
    app_config: &AppConfig,
    selected_api: &ApiConfig,
    agent: &AgentProfile,
    app_state: Option<&AppState>,
    tool_session_id: &str,
) -> Result<RuntimeToolAssembly, String> {
    let current_department =
        resolve_runtime_tool_current_department(app_config, app_state, agent, tool_session_id);
    let delegate_unavailable_reason =
        delegate_builtin_tool_unavailable_reason(app_config, current_department);
    let tool_definitions = read_global_tool_schema_cache(app_state)
        .into_iter()
        .filter(|definition| {
            definition.name != "delegate" || delegate_unavailable_reason.is_none()
        })
        .collect::<Vec<_>>();
    let mut tool_manifest = tool_definitions
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
        )?;
    }
    Ok(RuntimeToolAssembly {
        tools,
        tool_definitions,
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
    tools.push(Box::new(BuiltinRecallTool {
        app_state: state.clone(),
        memory_context,
    }));
    tools.push(Box::new(BuiltinOperateTool { model_supports_image }));
    tools.push(Box::new(BuiltinReloadTool { app_state: state.clone() }));
    tools.push(Box::new(BuiltinReadFileTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
        api_config_id: api_config_id.to_string(),
    }));
    tools.push(Box::new(BuiltinTerminalExecTool {
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
    tools.push(Box::new(BuiltinTaskTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
    }));
    if enable_delegate {
        tools.push(Box::new(BuiltinDelegateTool {
            app_state: state.clone(),
            session_id: tool_session_id.to_string(),
        }));
    }
    tools.push(Box::new(BuiltinMemeTool { app_state: state.clone() }));
    tools.push(Box::new(BuiltinContactReplyTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
    }));
    tools.push(Box::new(BuiltinContactSendFilesTool {
        app_state: state.clone(),
        session_id: tool_session_id.to_string(),
    }));
    tools.push(Box::new(BuiltinContactNoReplyTool));
    push_cached_mcp_runtime_tools(tools, &state);
    Ok(())
}

fn push_cached_mcp_runtime_tools(tools: &mut Vec<Box<dyn RuntimeToolDyn>>, state: &AppState) {
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
        for descriptor in list_tools_from_runtime(&server).into_iter().filter(|tool| tool.enabled) {
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

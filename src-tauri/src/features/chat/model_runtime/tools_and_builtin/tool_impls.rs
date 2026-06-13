#[derive(Debug, Clone)]
struct BuiltinFetchTool {
    app_state: AppState,
}

impl RuntimeToolMetadata for BuiltinFetchTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "fetch",
            "静态网页抓取工具。抓取网页内容并提取正文。",
            serde_json::json!({
              "type": "object",
              "properties": {
                "url": { "type": "string", "description": "要抓取的网页地址" },
                "max_length": { "type": "integer", "description": "返回内容的最大字符数", "default": 1800 }
              },
              "required": ["url"]
            }),
        )
    }
}

impl RuntimeJsonTool for BuiltinFetchTool {
    const NAME: &'static str = "fetch";
    type Args = FetchToolArgs;
    type Error = ToolInvokeError;

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
        runtime_log_debug(format!(
            "[TOOL-DEBUG] execute_builtin_tool.start name=fetch args={}",
            debug_value_snippet(&serde_json::to_value(&args).unwrap_or(Value::Null), 240)
        ));
        let result = builtin_fetch(&self.app_state, &args.url, args.max_length.unwrap_or(1800))
            .await
            .map_err(ToolInvokeError::from);
        match &result {
            Ok(v) => runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.ok name=fetch result={}",
                debug_value_snippet(v, 240)
            )),
            Err(err) => eprintln!("[工具执行] 内置工具 fetch 执行失败: 错误={err}"),
        }
        result
        })
    }
}

#[derive(Debug, Clone)]
struct BuiltinBingSearchTool {
    app_state: AppState,
}

impl RuntimeToolMetadata for BuiltinBingSearchTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "websearch",
            "搜索互联网内容。优先使用其他可用的联网搜索或抓取工具；仅在没有其他网络搜索或抓取能力时再使用。",
            serde_json::json!({
              "type": "object",
              "properties": {
                "query": { "type": "string", "description": "搜索关键词或问题" }
              },
              "required": ["query"]
            }),
        )
    }
}

impl RuntimeJsonTool for BuiltinBingSearchTool {
    const NAME: &'static str = "websearch";
    type Args = BingSearchToolArgs;
    type Error = ToolInvokeError;

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
        runtime_log_debug(format!(
            "[TOOL-DEBUG] execute_builtin_tool.start name=websearch args={}",
            debug_value_snippet(&serde_json::to_value(&args).unwrap_or(Value::Null), 240)
        ));
        let result = builtin_bing_search(&self.app_state, &args.query)
            .await
            .map_err(ToolInvokeError::from);
        match &result {
            Ok(v) => runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.ok name=websearch result={}",
                debug_value_snippet(v, 240)
            )),
            Err(err) => {
                eprintln!("[工具执行] 内置工具 websearch 执行失败: 错误={err}")
            }
        }
        result
        })
    }
}

#[derive(Debug, Clone)]
struct BuiltinRememberTool {
    app_state: AppState,
    memory_context: MemoryAgentContext,
}

impl RuntimeToolMetadata for BuiltinRememberTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "remember",
            "保存与用户相关、长期有价值的记忆。禁止保存密码、密钥等敏感信息。",
            serde_json::json!({
              "type": "object",
              "properties": {
                "action": {
                  "type": "string",
                  "enum": ["create", "update", "merge"],
                  "description": "记忆动作。create=新增一条记忆；update=更新一条已有记忆；merge=把多条旧记忆合并为一条新记忆。"
                },
                "sourceMemoryIds": {
                  "type": "array",
                  "items": { "type": "string" },
                  "description": "源记忆 ID，使用 recall 记忆板里的短编号。create 必须传空数组或省略；update 必须正好 1 个；merge 至少 2 个。"
                },
                "memory": {
                  "type": "object",
                  "description": "目标记忆内容。create 时是新记忆；update 时是 sourceMemoryIds[0] 的新版本；merge 时是多条源记忆合并后的结果。",
                  "properties": {
                    "memoryType": {
                      "type": "string",
                      "enum": ["knowledge", "skill", "emotion", "event"],
                      "description": "记忆类型。knowledge=稳定认知或事实，skill=做事方法或能力，emotion=稳定情绪偏好或态度，event=发生过的事件。"
                    },
                    "judgment": {
                      "type": "string",
                      "description": "记忆本体。用一句独立、清楚、可检索的判断句写出真正要记住的内容。"
                    },
                    "reasoning": {
                      "type": "string",
                      "description": "支撑 judgment 的依据或背景，可为空。只写理由、证据、来源，不要写流程话术。"
                    },
                    "tags": {
                      "type": "array",
                      "items": { "type": "string" },
                      "description": "检索锚点列表，用于后续命中提示板。每一项都必须是独立、紧凑、稳定、可检索的词元；不要写整句，不要写短语拼接，不要把多个语义塞进同一个 tag。"
                    }
                  },
                  "required": ["memoryType", "judgment", "tags"]
                }
              },
              "required": ["action", "memory"]
            }),
        )
    }
}

impl RuntimeJsonTool for BuiltinRememberTool {
    const NAME: &'static str = "remember";
    type Args = MemorySaveToolArgs;
    type Error = ToolInvokeError;

    fn timeout_override(_args_json: &str) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(3))
    }

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
        let args_json = serde_json::json!({
            "action": args.action,
            "sourceMemoryIds": args.source_memory_ids,
            "memory": args.memory,
        });
        runtime_log_debug(format!(
            "[TOOL-DEBUG] execute_builtin_tool.start name=remember args={}",
            debug_value_snippet(&args_json, 240)
        ));
        let result = builtin_memory_save(&self.app_state, &self.memory_context, args_json)
            .map_err(ToolInvokeError::from);
        match &result {
            Ok(v) => runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.ok name=remember result={}",
                debug_value_snippet(v, 240)
            )),
            Err(err) => {
                eprintln!("[工具执行] 内置工具 remember 执行失败: 错误={err}")
            }
        }
        result
        })
    }
}

#[derive(Debug, Clone)]
struct BuiltinRecallTool {
    app_state: AppState,
    memory_context: MemoryAgentContext,
}

impl RuntimeToolMetadata for BuiltinRecallTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "recall",
            "回忆记忆，并返回可直接注入提示词的记忆板。query 和 time 可选；结果应用 offset/limit 分页。",
            serde_json::json!({
              "type": "object",
              "properties": {
                "query": { "type": "string", "description": "可选的回忆查询文本；不传或为空时返回全部可见记忆。传入时先按 query 过滤相关记忆。" },
                "time": { "type": "string", "description": "可选的时间过滤。传 YYYY 表示该年，传 YYYY-MM 表示该月，传 YYYY-MM-DD 表示该日。" },
                "offset": { "type": "integer", "minimum": 0, "description": "跳过多少条结果。默认 0。" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 50, "description": "返回多少条结果。默认 7，最大 50。" }
              }
            }),
        )
    }
}

impl RuntimeJsonTool for BuiltinRecallTool {
    const NAME: &'static str = "recall";
    type Args = RecallToolArgs;
    type Error = ToolInvokeError;

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
        let args_json = serde_json::to_value(&args).unwrap_or(Value::Null);
        runtime_log_debug(format!(
            "[TOOL-DEBUG] execute_builtin_tool.start name=recall args={}",
            debug_value_snippet(&args_json, 240)
        ));
        let result = builtin_recall(
            &self.app_state,
            &self.memory_context,
            args.query.as_deref().unwrap_or(""),
            args.time.as_deref(),
            args.offset,
            args.limit,
        )
            .map_err(ToolInvokeError::from);
        match &result {
            Ok(v) => runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.ok name=recall result={}",
                debug_value_snippet(v, 240)
            )),
            Err(err) => eprintln!("[工具执行] 内置工具 recall 执行失败: 错误={err}"),
        }
        result
        })
    }
}

#[derive(Debug, Clone)]
struct BuiltinReloadTool {
    app_state: AppState,
}

#[derive(Debug, Clone)]
struct BuiltinOrganizeContextTool {
    app_state: AppState,
    session_id: String,
    api_config_id: String,
    agent_id: String,
}

impl RuntimeJsonTool for BuiltinReloadTool {
    const NAME: &'static str = "reload";
    type Args = EmptyToolArgs;
    type Error = ToolInvokeError;

    fn call_typed(&self, _args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
            runtime_log_debug("[TOOL-DEBUG] execute_builtin_tool.start name=reload".to_string());
            let result = builtin_reload(&self.app_state)
                .await
                .map_err(ToolInvokeError::from);
            match &result {
                Ok(v) => runtime_log_debug(format!(
                    "[TOOL-DEBUG] execute_builtin_tool.ok name=reload result={}",
                    debug_value_snippet(v, 240)
                )),
                Err(err) => runtime_log_debug(format!(
                    "[TOOL-DEBUG] execute_builtin_tool.err name=reload err={err}"
                )),
            }
            result
        })
    }
}

impl RuntimeToolMetadata for BuiltinReloadTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "reload",
            "清空工作区运行态缓存后，重新加载 MCP、技能、私有人格与私有部门。不合法配置会被跳过，并在 repairSummary/repairItems 中返回路径、错误和修复建议。",
            serde_json::json!({
              "type": "object",
              "properties": {},
              "additionalProperties": false
            }),
        )
    }
}

impl RuntimeJsonTool for BuiltinOrganizeContextTool {
    const NAME: &'static str = "organize_context";
    type Args = EmptyToolArgs;
    type Error = ToolInvokeError;

    fn call_typed(&self, _args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
            runtime_log_debug(
                "[TOOL-DEBUG] execute_builtin_tool.start name=organize_context".to_string(),
            );
            let result = builtin_organize_context(
                &self.app_state,
                &self.session_id,
                &self.api_config_id,
                &self.agent_id,
            )
            .await
            .map_err(ToolInvokeError::from);
            match &result {
                Ok(v) => runtime_log_debug(format!(
                    "[TOOL-DEBUG] execute_builtin_tool.ok name=organize_context result={}",
                    debug_value_snippet(v, 240)
                )),
                Err(err) => runtime_log_debug(format!(
                    "[TOOL-DEBUG] execute_builtin_tool.err name=organize_context err={err}"
                )),
            }
            result
        })
    }
}

impl RuntimeToolMetadata for BuiltinOrganizeContextTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "organize_context",
            "整理当前活跃对话上下文。",
            serde_json::json!({
              "type": "object",
              "properties": {},
              "additionalProperties": false
            }),
        )
    }
}

#[derive(Debug, Clone)]
struct BuiltinTerminalExecTool {
    app_state: AppState,
    session_id: String,
}

impl RuntimeToolMetadata for BuiltinTerminalExecTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "exec",
            terminal_exec_tool_description(&terminal_shell_for_state(&self.app_state)),
            serde_json::json!({
              "type": "object",
              "properties": {
                "command": { "type": "string", "description": "要执行的一次性 shell 命令。" },
                "timeout_ms": { "type": "integer", "minimum": 1, "default": 300000, "description": "命令超时时间，单位毫秒；未指定时默认 300000ms，超时后回收本次进程树。长耗时检查/构建应显式传入足够大的值。" }
              },
              "required": ["command"],
              "additionalProperties": false
            }),
        )
    }
}

impl RuntimeJsonTool for BuiltinTerminalExecTool {
    const NAME: &'static str = "exec";
    type Args = TerminalExecToolArgs;
    type Error = ToolInvokeError;

    fn timeout_override(args_json: &str) -> Option<std::time::Duration> {
        parse_runtime_tool_args::<TerminalExecToolArgs>(args_json)
            .ok()
            .and_then(|args| args.timeout_ms)
            .map(std::time::Duration::from_millis)
    }

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
        let args_json = serde_json::to_value(&args).unwrap_or(Value::Null);
        runtime_log_debug(format!(
            "[TOOL-DEBUG] execute_builtin_tool.start name=exec args={}",
            debug_value_snippet(&args_json, 240)
        ));
        let resolved_action = args
            .action
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("run");
        let resolved_command = args.command.as_deref().map(str::trim).unwrap_or("");
        if resolved_action == "run" && resolved_command.is_empty() {
            return Err(ToolInvokeError::from("exec.command is required".to_string()));
        }
        let result = builtin_shell_exec(
            &self.app_state,
            &self.session_id,
            resolved_action,
            resolved_command,
            args.timeout_ms,
        )
        .await
        .map_err(ToolInvokeError::from);
        match &result {
            Ok(v) => {
                runtime_log_debug(format!(
                    "[TOOL-DEBUG] execute_builtin_tool.ok name=exec result={}",
                    debug_value_snippet(v, 240)
                ));
                runtime_log_debug(format!(
                    "[TOOL-DEBUG] execute_builtin_tool.ok name=exec summary={}",
                    debug_exec_result_summary(v)
                ));
            }
            Err(err) => {
                eprintln!("[工具执行] 内置工具 exec 执行失败: 错误={err}")
            }
        }
        result
        })
    }
}

#[derive(Debug, Clone)]
struct BuiltinApplyPatchTool {
    app_state: AppState,
    session_id: String,
}

#[derive(Debug, Clone)]
struct BuiltinWriteFileTool {
    app_state: AppState,
    session_id: String,
}

#[derive(Debug, Clone)]
struct BuiltinDeleteFileTool {
    app_state: AppState,
    session_id: String,
}

#[derive(Debug, Clone)]
struct BuiltinUpdateFileTool {
    app_state: AppState,
    session_id: String,
}

#[derive(Debug, Clone)]
struct BuiltinMoveFileTool {
    app_state: AppState,
    session_id: String,
}

impl RuntimeToolMetadata for BuiltinApplyPatchTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "apply_patch",
            [
                "编辑文件的 JSON 结构化修改工具。",
                "参数顶层固定为 {\"operations\": [...]}。",
                "只支持四种 action：add、delete、update、move。",
                "add 需要 path 和 content。",
                "delete 需要 path。",
                "update 需要 path、old_string、new_string；可选 replace_all。它通过 old_string 在原文件中做精确子串替换，不使用 diff hunk。",
                "move 需要 path 和 to；语义是重命名或移动文件。",
                "路径可以是绝对路径，也可以是相对当前工作目录的路径；最终仍受工作区权限校验。",
                "如果 old_string 命中 0 处会失败；命中多处且 replace_all=false 也会失败。此时应扩大 old_string 上下文，或明确设置 replace_all=true。",
                "最小示例：{\"operations\":[{\"action\":\"update\",\"path\":\"src/example.ts\",\"old_string\":\"before\",\"new_string\":\"after\"}]}",
            ]
            .join("\n"),
            serde_json::json!({
              "type": "object",
              "properties": {
                "operations": {
                  "type": "array",
                  "description": "结构化修改操作列表",
                  "items": {
                    "type": "object",
                    "properties": {
                      "action": { "type": "string", "enum": ["add", "delete", "update", "move"] },
                      "path": { "type": "string" },
                      "content": { "type": "string" },
                      "oldString": { "type": "string" },
                      "newString": { "type": "string" },
                      "replaceAll": { "type": "boolean" },
                      "to": { "type": "string" }
                    },
                    "required": ["action", "path"],
                    "additionalProperties": false
                  }
                },
              },
              "required": ["operations"],
              "additionalProperties": false
            }),
        )
    }
}

impl RuntimeJsonTool for BuiltinApplyPatchTool {
    const NAME: &'static str = "apply_patch";
    type Args = ApplyPatchToolArgs;
    type Error = ToolInvokeError;

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
        let args_json = serde_json::to_value(&args).unwrap_or(Value::Null);
        runtime_log_debug(format!(
            "[TOOL-DEBUG] execute_builtin_tool.start name=apply_patch args={}",
            debug_value_snippet(&args_json, 240)
        ));
        let result = builtin_apply_patch(&self.app_state, &self.session_id, args)
            .await
            .map_err(ToolInvokeError::from);
        match &result {
            Ok(v) => runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.ok name=apply_patch result={}",
                debug_value_snippet(v, 240)
            )),
            Err(err) => eprintln!("[工具执行] 内置工具 apply_patch 执行失败: 错误={err}"),
        }
        result
        })
    }
}

impl RuntimeToolMetadata for BuiltinWriteFileTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "write",
            "新增文件或直接写入一个完整文件。必须提供 path 和 content。用于创建新文件，或在你明确要用完整内容覆盖目标文件时使用。",
            serde_json::json!({
              "type": "object",
              "properties": {
                "path": { "type": "string", "description": "目标文件路径。可以是绝对路径，也可以是相对当前工作目录的路径；最终仍受工作区权限校验。" },
                "content": { "type": "string", "description": "要写入文件的完整内容。" }
              },
              "required": ["path", "content"],
              "additionalProperties": false
            }),
        )
    }
}

impl RuntimeJsonTool for BuiltinWriteFileTool {
    const NAME: &'static str = "write";
    type Args = WriteFileToolArgs;
    type Error = ToolInvokeError;

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
        runtime_log_debug(format!(
            "[TOOL-DEBUG] execute_builtin_tool.start name=write args={}",
            debug_value_snippet(&serde_json::to_value(&args).unwrap_or(Value::Null), 240)
        ));
        let result = builtin_write_file(&self.app_state, &self.session_id, args)
            .await
            .map_err(ToolInvokeError::from);
        match &result {
            Ok(v) => runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.ok name=write result={}",
                debug_value_snippet(v, 240)
            )),
            Err(err) => eprintln!("[工具执行] 内置工具 write 执行失败: 错误={err}"),
        }
        result
        })
    }
}

impl RuntimeToolMetadata for BuiltinDeleteFileTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "delete",
            "删除整个文件。只删除 path 指向的完整文件，不删除文件中的局部内容；如果你想删除文件中的一段内容，请改用 update。",
            serde_json::json!({
              "type": "object",
              "properties": {
                "path": { "type": "string", "description": "要删除的目标文件路径。可以是绝对路径，也可以是相对当前工作目录的路径；最终仍受工作区权限校验。" }
              },
              "required": ["path"],
              "additionalProperties": false
            }),
        )
    }
}

impl RuntimeJsonTool for BuiltinDeleteFileTool {
    const NAME: &'static str = "delete";
    type Args = DeleteFileToolArgs;
    type Error = ToolInvokeError;

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
        runtime_log_debug(format!(
            "[TOOL-DEBUG] execute_builtin_tool.start name=delete args={}",
            debug_value_snippet(&serde_json::to_value(&args).unwrap_or(Value::Null), 240)
        ));
        let result = builtin_delete_file(&self.app_state, &self.session_id, args)
            .await
            .map_err(ToolInvokeError::from);
        match &result {
            Ok(v) => runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.ok name=delete result={}",
                debug_value_snippet(v, 240)
            )),
            Err(err) => eprintln!("[工具执行] 内置工具 delete 执行失败: 错误={err}"),
        }
        result
        })
    }
}

impl RuntimeToolMetadata for BuiltinUpdateFileTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "update",
            "修改已有文件中的局部内容。必须提供 path、old_string、new_string；它通过 old_string 在原文件中做精确子串替换，不使用 diff hunk。若要删除局部内容，请让 new_string 设为空字符串。",
            serde_json::json!({
              "type": "object",
              "properties": {
                "path": { "type": "string", "description": "目标文件路径。可以是绝对路径，也可以是相对当前工作目录的路径；最终仍受工作区权限校验。" },
                "oldString": { "type": "string", "description": "必须和原文件中的一段内容精确匹配。" },
                "newString": { "type": "string", "description": "替换后的内容；如需删除旧内容，可传空字符串。" },
                "replaceAll": { "type": "boolean", "description": "是否替换全部命中项；默认 false。" }
              },
              "required": ["path", "oldString", "newString"],
              "additionalProperties": false
            }),
        )
    }
}

impl RuntimeJsonTool for BuiltinUpdateFileTool {
    const NAME: &'static str = "update";
    type Args = UpdateFileToolArgs;
    type Error = ToolInvokeError;

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
        runtime_log_debug(format!(
            "[TOOL-DEBUG] execute_builtin_tool.start name=update args={}",
            debug_value_snippet(&serde_json::to_value(&args).unwrap_or(Value::Null), 240)
        ));
        let result = builtin_update_file(&self.app_state, &self.session_id, args)
            .await
            .map_err(ToolInvokeError::from);
        match &result {
            Ok(v) => runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.ok name=update result={}",
                debug_value_snippet(v, 240)
            )),
            Err(err) => eprintln!("[工具执行] 内置工具 update 执行失败: 错误={err}"),
        }
        result
        })
    }
}

impl RuntimeToolMetadata for BuiltinMoveFileTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "move",
            "移动或重命名整个文件。必须提供 path 和 to。",
            serde_json::json!({
              "type": "object",
              "properties": {
                "path": { "type": "string", "description": "原始文件路径。可以是绝对路径，也可以是相对当前工作目录的路径；最终仍受工作区权限校验。" },
                "to": { "type": "string", "description": "目标文件路径。可以是绝对路径，也可以是相对当前工作目录的路径；最终仍受工作区权限校验。" }
              },
              "required": ["path", "to"],
              "additionalProperties": false
            }),
        )
    }
}

impl RuntimeJsonTool for BuiltinMoveFileTool {
    const NAME: &'static str = "move";
    type Args = MoveFileToolArgs;
    type Error = ToolInvokeError;

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
        runtime_log_debug(format!(
            "[TOOL-DEBUG] execute_builtin_tool.start name=move args={}",
            debug_value_snippet(&serde_json::to_value(&args).unwrap_or(Value::Null), 240)
        ));
        let result = builtin_move_file(&self.app_state, &self.session_id, args)
            .await
            .map_err(ToolInvokeError::from);
        match &result {
            Ok(v) => runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.ok name=move result={}",
                debug_value_snippet(v, 240)
            )),
            Err(err) => eprintln!("[工具执行] 内置工具 move 执行失败: 错误={err}"),
        }
        result
        })
    }
}




#[derive(Debug, Clone)]
struct BuiltinPlanTool {
    app_state: AppState,
    session_id: String,
}

#[derive(Debug, Clone)]
struct BuiltinTaskTool {
    app_state: AppState,
    session_id: String,
    api_config_id: String,
    executor_department_id: String,
    executor_agent_id: String,
}

#[derive(Debug, Clone)]
struct BuiltinTodoTool {
    app_state: AppState,
    session_id: String,
}

#[derive(Debug, Clone)]
struct BuiltinCreateGoalTool {
    app_state: AppState,
    session_id: String,
}

#[derive(Debug, Clone)]
struct BuiltinUpdateGoalTool {
    app_state: AppState,
    session_id: String,
}

#[derive(Debug, Clone)]
struct BuiltinGetSessionTool {
    app_state: AppState,
    session_id: String,
}

#[derive(Debug, Clone)]
struct BuiltinInformSessionTool {
    app_state: AppState,
    session_id: String,
}

impl RuntimeToolMetadata for BuiltinTodoTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        todo_provider_tool_definition()
    }
}

impl RuntimeToolDyn for BuiltinTodoTool {
    fn name(&self) -> String {
        TODO_TOOL_NAME.to_string()
    }

    fn call_json(&self, args_json: String) -> RuntimeToolCallFuture<'_> {
        Box::pin(async move {
            let args = parse_runtime_tool_args::<TodoWriteRequest>(&args_json)?;
            let args_value = serde_json::to_value(&args).unwrap_or(Value::Null);
            runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.start name=todo args={}",
                debug_value_snippet(&args_value, 240)
            ));
            let result = builtin_todo(&self.app_state, &self.session_id, args)
                .map(ProviderToolResult::text);
            match &result {
                Ok(v) => runtime_log_debug(format!(
                    "[TOOL-DEBUG] execute_builtin_tool.ok name=todo result={}",
                    debug_text_snippet(&v.display_text, 240)
                )),
                Err(err) => eprintln!("[工具执行] 内置工具 todo 执行失败: 错误={err}"),
            }
            result
        })
    }
}

impl RuntimeToolMetadata for BuiltinPlanTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "plan",
            plan_tool_description(),
            serde_json::json!({
              "type": "object",
              "properties": {
                "action": {
                  "type": "string",
                  "enum": ["present", "complete"],
                  "description": "present 表示提交计划；complete 表示标记该计划已完成"
                },
                "path": {
                  "type": "string",
                  "description": "计划 Markdown 文件路径"
                }
              },
              "required": ["action", "path"],
              "additionalProperties": false
            }),
        )
    }
}

impl RuntimeJsonTool for BuiltinPlanTool {
    const NAME: &'static str = "plan";
    type Args = PlanToolArgs;
    type Error = ToolInvokeError;

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
        runtime_log_debug(format!(
            "[TOOL-DEBUG] execute_builtin_tool.start name=plan args={}",
            debug_value_snippet(&serde_json::to_value(&args).unwrap_or(Value::Null), 240)
        ));
        let result = builtin_plan(&self.app_state, &self.session_id, args)
            .map_err(ToolInvokeError::from);
        match &result {
            Ok(v) => runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.ok name=plan result={}",
                debug_value_snippet(v, 240)
            )),
            Err(err) => eprintln!("[工具执行] 内置工具 plan 执行失败: 错误={err}"),
        }
        result
        })
    }
}

impl RuntimeToolMetadata for BuiltinTaskTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "task",
            "创建和管理会在未来自动触发委托的持久化任务。任务到点后会启动一个委托，并在委托会话中绑定 goal；委托完成后结果回到来源会话。",
            serde_json::json!({
              "type": "object",
              "properties": {
                "action": { "type": "string", "enum": ["list", "get", "create", "complete"], "description": "要执行的动作。" },
                "task_id": { "type": "string", "description": "任务 ID。get、complete 时必填。" },
                "goal": { "type": "string", "description": "任务到点后要交给委托完成的目标，也是列表标题。" },
                "why": { "type": "string", "description": "为什么要做这件事，用来避免后续推进走偏。" },
                "todo": { "type": "string", "description": "委托启动时要关注的下一步、范围边界或交付要求。" },
                "completion_state": { "type": "string", "enum": ["completed", "failed_completed"], "description": "complete 时必填。completed 表示完成，failed_completed 表示结束但失败。" },
                "completion_conclusion": { "type": "string", "description": "complete 时填写最终结果、失败原因或阻塞点。" },
                "trigger": {
                  "type": "object",
                  "description": "任务触发时间设置。",
                  "properties": {
                    "run_at": { "type": "string", "description": "必填。首次触发时间。RFC3339，保留时区和秒，不要毫秒，例如 2026-05-07T20:00:00+08:00。" },
                    "cron_expression": { "type": "string", "description": "可选。标准 Linux/Unix 5 段 cron。留空表示只触发一次，例如 * * * * * 表示每分钟一次。" },
                    "end_at": { "type": "string", "description": "可选。停止时间。RFC3339，保留时区和秒，不要毫秒，例如 2026-05-08T08:00:00+08:00；必须晚于 run_at。" }
                  }
                }
              },
              "required": ["action"]
            }),
        )
    }
}

impl RuntimeToolMetadata for BuiltinCreateGoalTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "create_goal",
            "在当前会话启动一个持续执行的 goal。只在用户明确要求启动目标时使用；已有 active goal 时会失败。",
            serde_json::json!({
              "type": "object",
              "properties": {
                "objective": { "type": "string", "description": "用户提供的完整目标内容。" }
              },
              "required": ["objective"],
              "additionalProperties": false
            }),
        )
    }
}

impl RuntimeJsonTool for BuiltinCreateGoalTool {
    const NAME: &'static str = "create_goal";
    type Args = CreateGoalToolArgs;
    type Error = ToolInvokeError;

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
            runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.start name=create_goal args={}",
                debug_value_snippet(&serde_json::to_value(&args).unwrap_or(Value::Null), 240)
            ));
            let result = goal_create_for_session(&self.app_state, &self.session_id, args)
                .map_err(ToolInvokeError::from);
            match &result {
                Ok(v) => runtime_log_debug(format!(
                    "[TOOL-DEBUG] execute_builtin_tool.ok name=create_goal result={}",
                    debug_value_snippet(v, 240)
                )),
                Err(err) => eprintln!("[工具执行] 内置工具 create_goal 执行失败: 错误={err}"),
            }
            result
        })
    }
}

impl RuntimeToolMetadata for BuiltinUpdateGoalTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "update_goal",
            "将当前会话 active goal 标记为终态。只允许 complete 或 blocked；不要用它记录普通进度。",
            serde_json::json!({
              "type": "object",
              "properties": {
                "status": { "type": "string", "enum": ["complete", "blocked"], "description": "goal 终态。" },
                "evidence": { "type": "string", "description": "status=complete 时必填，说明证明目标已完成的证据。" },
                "blocking_condition": { "type": "string", "description": "status=blocked 时必填，说明连续阻塞的同一条件。" }
              },
              "required": ["status"],
              "additionalProperties": false
            }),
        )
    }
}

impl RuntimeJsonTool for BuiltinUpdateGoalTool {
    const NAME: &'static str = "update_goal";
    type Args = UpdateGoalToolArgs;
    type Error = ToolInvokeError;

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
            runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.start name=update_goal args={}",
                debug_value_snippet(&serde_json::to_value(&args).unwrap_or(Value::Null), 240)
            ));
            let result = goal_update_for_session(&self.app_state, &self.session_id, args)
                .map_err(ToolInvokeError::from);
            match &result {
                Ok(v) => runtime_log_debug(format!(
                    "[TOOL-DEBUG] execute_builtin_tool.ok name=update_goal result={}",
                    debug_value_snippet(v, 240)
                )),
                Err(err) => eprintln!("[工具执行] 内置工具 update_goal 执行失败: 错误={err}"),
            }
            result
        })
    }
}

impl RuntimeToolMetadata for BuiltinGetSessionTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "get_session",
            "查询可投递的会话。默认返回本地普通未归档会话和远程联系人会话；可用 keyword 按标题、联系人、部门、人格筛选。",
            serde_json::json!({
              "type": "object",
              "properties": {
                "keyword": { "type": "string", "description": "可选，会话检索关键字。" }
              },
              "additionalProperties": false
            }),
        )
    }
}

impl RuntimeJsonTool for BuiltinGetSessionTool {
    const NAME: &'static str = "get_session";
    type Args = GetSessionToolArgs;
    type Error = ToolInvokeError;

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
            runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.start name=get_session args={}",
                debug_value_snippet(&serde_json::to_value(&args).unwrap_or(Value::Null), 240)
            ));
            let result =
                builtin_get_session(&self.app_state, &self.session_id, args).map_err(ToolInvokeError::from);
            match &result {
                Ok(v) => runtime_log_debug(format!(
                    "[TOOL-DEBUG] execute_builtin_tool.ok name=get_session result={}",
                    debug_value_snippet(v, 240)
                )),
                Err(err) => eprintln!("[工具执行] 内置工具 get_session 执行失败: 错误={err}"),
            }
            result
        })
    }
}

impl RuntimeToolMetadata for BuiltinInformSessionTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "inform_session",
            "向指定会话投递一条系统助理通知。目标为远程联系人时，会同时写入联系人会话并外发到远端。",
            serde_json::json!({
              "type": "object",
              "properties": {
                "session_id": { "type": "string", "description": "目标会话 ID。" },
                "content": { "type": "string", "description": "要通知的正文。" }
              },
              "required": ["session_id", "content"],
              "additionalProperties": false
            }),
        )
    }
}

impl RuntimeJsonTool for BuiltinInformSessionTool {
    const NAME: &'static str = "inform_session";
    type Args = InformSessionToolArgs;
    type Error = ToolInvokeError;

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
            runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.start name=inform_session args={}",
                debug_value_snippet(&serde_json::to_value(&args).unwrap_or(Value::Null), 240)
            ));
            let result = builtin_inform_session(&self.app_state, &self.session_id, args)
                .map_err(ToolInvokeError::from);
            match &result {
                Ok(v) => runtime_log_debug(format!(
                    "[TOOL-DEBUG] execute_builtin_tool.ok name=inform_session result={}",
                    debug_value_snippet(v, 240)
                )),
                Err(err) => eprintln!("[工具执行] 内置工具 inform_session 执行失败: 错误={err}"),
            }
            result
        })
    }
}

impl RuntimeJsonTool for BuiltinTaskTool {
    const NAME: &'static str = "task";
    type Args = TaskToolArgsWire;
    type Error = ToolInvokeError;

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
        runtime_log_debug(format!(
            "[TOOL-DEBUG] execute_builtin_tool.start name=task args={}",
            debug_value_snippet(&serde_json::to_value(&args).unwrap_or(Value::Null), 240)
        ));
        let result = builtin_task(
            &self.app_state,
            &self.session_id,
            &self.api_config_id,
            &self.executor_department_id,
            &self.executor_agent_id,
            args,
        )
            .await
            .map_err(ToolInvokeError::from);
        match &result {
            Ok(v) => runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.ok name=task result={}",
                debug_value_snippet(v, 240)
            )),
            Err(err) => eprintln!("[工具执行] 内置工具 task 执行失败: 错误={err}"),
        }
        result
        })
    }
}

#[derive(Debug, Clone)]
struct BuiltinDelegateTool {
    app_state: AppState,
    session_id: String,
    source_agent_id: String,
    source_department_id: String,
}

impl RuntimeToolMetadata for BuiltinDelegateTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "delegate",
            "在下级部门开启一个子代理，协助处理当前工作。当当前工作有更匹配的直属下级部门，或子任务能用简明背景独立说明清楚时，应优先发起委托。",
            serde_json::json!({
              "type": "object",
              "properties": {
                "department_id": { "type": "string", "description": "要委托给哪个下级部门的 ID。应选择与当前任务最匹配的直接下级部门。" },
                "mode": { "type": "string", "enum": ["wait", "background"], "description": "委托方式。mode 只表示父调度是否等待结果，不表示是否并发。除非用户明确要求后台运行，否则一律使用 wait。wait 会等待子代理返回结果，多个 wait 委托可以同时发出并等待全部返回；background 会后台运行并稍后写回当前来源会话。", "default": "wait" },
                "why": { "type": "string", "description": "为什么要做、背景材料、已知事实、已有线索或必要上下文。" },
                "goal": { "type": "string", "description": "这次委托要达成的目标，写成明确可执行、可判断完成的任务。" },
                "todo": { "type": "string", "description": "优先关注点、范围边界、交付要求、下一步待办或需要避免的方向。" }
              },
              "required": ["department_id", "why", "goal", "todo"]
            }),
        )
    }
}

impl RuntimeJsonTool for BuiltinDelegateTool {
    const NAME: &'static str = "delegate";
    type Args = DelegateToolArgs;
    type Error = ToolInvokeError;

    fn call_typed(&self, args: Self::Args) -> RuntimeJsonValueFuture<'_, Self::Error> {
        Box::pin(async move {
        runtime_log_debug(format!(
            "[TOOL-DEBUG] execute_builtin_tool.start name=delegate args={}",
            debug_value_snippet(&serde_json::to_value(&args).unwrap_or(Value::Null), 240)
        ));
        let result = builtin_delegate(
            &self.app_state,
            &self.session_id,
            Some(self.source_agent_id.as_str()),
            Some(self.source_department_id.as_str()),
            args,
        )
            .await
            .map_err(ToolInvokeError::from);
        match &result {
            Ok(v) => runtime_log_debug(format!(
                "[TOOL-DEBUG] execute_builtin_tool.ok name=delegate result={}",
                debug_value_snippet(v, 240)
            )),
            Err(err) => eprintln!("[工具执行] 内置工具 delegate 执行失败: 错误={err}"),
        }
        result
        })
    }
}

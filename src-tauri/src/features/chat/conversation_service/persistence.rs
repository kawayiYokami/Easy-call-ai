#[cfg(test)]
fn resolve_unarchived_conversation_index_with_fallback(
    data: &mut AppData,
    app_config: &AppConfig,
    effective_agent_id: &str,
    requested_conversation_id: Option<&str>,
) -> Result<usize, String> {
    if let Some(conversation_id) = requested_conversation_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if let Some(idx) = data.conversations.iter().position(|item| {
            item.id == conversation_id
                && item.summary.trim().is_empty()
                && conversation_visible_in_foreground_lists(item)
        }) {
            return Ok(idx);
        }
        runtime_log_warn(format!(
            "[解析对话索引] 请求的conversation_id不存在，终止本次读取: '{}' (agent_id: '{}')",
            conversation_id, effective_agent_id
        ));
        return Err(format!(
            "Requested conversation not found: {conversation_id}"
        ));
    }

    if let Some(existing_idx) = main_conversation_index(data, effective_agent_id) {
        return Ok(existing_idx);
    }

    if let Some(existing_idx) = latest_active_conversation_index(data, "", effective_agent_id) {
        return Ok(existing_idx);
    }

    let api_config = resolve_selected_api_config(app_config, None)
        .ok_or_else(|| "No API config available".to_string())?;
    Ok(ensure_active_conversation_index(
        data,
        &api_config.id,
        effective_agent_id,
    ))
}

fn build_foreground_conversation_snapshot_from_conversation(
    state: &AppState,
    conversation: &Conversation,
    recent_limit: usize,
) -> Result<ForegroundConversationSnapshotCore, String> {
    let (messages, has_more_history) = build_foreground_snapshot_recent_messages(
        state,
        conversation,
        recent_limit,
    )?;
    Ok(ForegroundConversationSnapshotCore {
        conversation_id: conversation.id.clone(),
        messages,
        has_more_history,
        runtime_state: unarchived_conversation_runtime_state(state, &conversation.id),
        current_todo: conversation_current_todo_text(conversation),
        current_todos: conversation.current_todos.clone(),
        preferred_api_config_id: conversation.preferred_api_config_id.clone(),
    })
}

fn build_foreground_snapshot_recent_messages(
    state: &AppState,
    conversation: &Conversation,
    recent_limit: usize,
) -> Result<(Vec<ChatMessage>, bool), String> {
    let paths = message_store::message_store_paths(&state.data_path, &conversation.id)?;
    if let Some(page) =
        message_store::read_ready_message_store_recent_messages_page_cached(&paths, recent_limit)?
    {
        if let Err(err) = conversation_service().retain_message_store_block_cache_whitelist(state) {
            runtime_log_warn(format!(
                "[消息存储] 警告，任务=retain_message_store_block_cache_whitelist，conversation_id={}，error={}",
                conversation.id, err
            ));
        }
        return Ok((page.messages, page.has_more));
    }
    let total_messages = conversation.messages.len();
    let start = total_messages.saturating_sub(recent_limit);
    Ok((conversation.messages[start..].to_vec(), start > 0))
}

fn provider_usage_prompt_tokens(usage: &Value) -> u64 {
    usage
        .get("promptTokens")
        .or_else(|| usage.get("prompt_tokens"))
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_i64().and_then(|item| u64::try_from(item).ok()))
        })
        .unwrap_or(0)
}

fn emit_provider_context_usage_update_from_conversation(
    state: &AppState,
    conversation: &Conversation,
    usage: &Value,
) {
    let prompt_tokens = provider_usage_prompt_tokens(usage);
    if prompt_tokens == 0 {
        return;
    }
    let app_config = match state_read_config_cached(state) {
        Ok(value) => value,
        Err(err) => {
            runtime_log_warn(format!(
                "[聊天用量] 跳过，任务=推送真实上下文用量，conversation_id={}，error={}",
                conversation.id, err
            ));
            return;
        }
    };
    let Some(api_config) =
        resolve_selected_api_config(&app_config, conversation.preferred_api_config_id.as_deref())
    else {
        return;
    };
    let context_window_tokens = api_config.context_window_tokens.max(1);
    let context_usage_ratio = prompt_tokens as f64 / f64::from(context_window_tokens);
    let context_usage_percent = context_usage_ratio
        .mul_add(100.0, 0.0)
        .round()
        .clamp(0.0, 100.0) as u32;
    let message = serde_json::json!({
        "conversationId": conversation.id,
        "providerPromptTokens": prompt_tokens,
        "contextUsagePercent": context_usage_percent,
        "contextUsageRatio": context_usage_ratio,
        "contextWindowTokens": context_window_tokens,
        "source": "provider_prompt_tokens",
        "eventReason": "provider_request_usage",
    })
    .to_string();
    emit_assistant_delta_app_event(
        state,
        &conversation.id,
        &AssistantDeltaEvent {
            delta: String::new(),
            kind: Some("context_usage_update".to_string()),
            request_id: None,
            activation_id: None,
            phase_id: None,
            reason: Some("provider_request_usage".to_string()),
            tool_name: None,
            tool_call_id: None,
            tool_status: None,
            tool_args: None,
            message: Some(message),
            stream_cache: None,
        },
    );
}

impl ConversationService {
    fn persist_conversation(
        &self,
        state: &AppState,
        conversation: &Conversation,
    ) -> Result<(), String> {
        state_schedule_conversation_persist(state, conversation).map(|_| ())
    }

    fn sync_conversation_metadata_from_snapshot(
        &self,
        state: &AppState,
        conversation: &Conversation,
    ) -> Result<Conversation, String> {
        let (conversation, (), _) =
            state_update_conversation_metadata_cached(state, &conversation.id, |cached| {
                preserve_field_level_conversation_metadata(cached, conversation);
                Ok(())
            })?;
        Ok(conversation)
    }

    fn add_conversation_cumulative_usage_delta(
        &self,
        state: &AppState,
        conversation_id: &str,
        usage: &Value,
    ) -> Result<bool, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Ok(false);
        }
        let mut probe = ConversationCumulativeUsage::default();
        if !conversation_cumulative_usage_add_provider_usage(&mut probe, usage) {
            return Ok(false);
        }
        let _guard = lock_conversation_with_metrics(state, "add_conversation_cumulative_usage")?;
        let (conversation, changed, _) = state_update_conversation_metadata_cached(
            state,
            normalized_conversation_id,
            |conversation| {
                Ok(conversation_cumulative_usage_add_provider_usage(
                    &mut conversation.cumulative_usage,
                    usage,
                ))
            },
        )?;
        if changed {
            emit_provider_context_usage_update_from_conversation(state, &conversation, usage);
        }
        Ok(changed)
    }

    fn set_conversation_preferred_api_config_id(
        &self,
        state: &AppState,
        conversation_id: &str,
        preferred_api_config_id: Option<String>,
    ) -> Result<Conversation, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let (conversation, (), _) = state_update_conversation_metadata_cached(
            state,
            normalized_conversation_id,
            |conversation| {
                conversation.preferred_api_config_id = preferred_api_config_id.clone();
                Ok(())
            },
        )?;
        drop(guard);
        Ok(conversation)
    }

    fn set_conversation_title_metadata(
        &self,
        state: &AppState,
        conversation_id: &str,
        next_title: &str,
    ) -> Result<Conversation, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let normalized_title = next_title.trim().to_string();
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let (conversation, (), _) = state_update_conversation_metadata_cached(
            state,
            normalized_conversation_id,
            |conversation| {
                conversation.title = normalized_title;
                Ok(())
            },
        )?;
        drop(guard);
        Ok(conversation)
    }

    fn set_conversation_plan_mode_enabled_metadata(
        &self,
        state: &AppState,
        conversation_id: &str,
        enabled: bool,
    ) -> Result<Conversation, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let (conversation, (), _) = state_update_conversation_metadata_cached(
            state,
            normalized_conversation_id,
            |conversation| {
                conversation.plan_mode_enabled = enabled;
                Ok(())
            },
        )?;
        drop(guard);
        Ok(conversation)
    }

    fn set_conversation_unread_count_metadata(
        &self,
        state: &AppState,
        conversation_id: &str,
        unread_count: usize,
    ) -> Result<Conversation, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let (conversation, (), _) = state_update_conversation_metadata_cached(
            state,
            normalized_conversation_id,
            |conversation| {
                conversation.unread_count = unread_count;
                Ok(())
            },
        )?;
        drop(guard);
        Ok(conversation)
    }

    fn set_conversation_current_todos_metadata(
        &self,
        state: &AppState,
        conversation_id: &str,
        current_todos: Vec<ConversationTodoItem>,
    ) -> Result<Conversation, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let (conversation, (), _) = state_update_conversation_metadata_cached(
            state,
            normalized_conversation_id,
            |conversation| {
                conversation.current_todos = current_todos;
                Ok(())
            },
        )?;
        drop(guard);
        Ok(conversation)
    }

    fn set_conversation_shell_workspace_metadata(
        &self,
        state: &AppState,
        conversation_id: &str,
        shell_workspace_path: Option<Option<String>>,
        shell_workspaces: Option<Vec<ShellWorkspaceConfig>>,
        shell_autonomous_mode: Option<bool>,
    ) -> Result<Conversation, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let (conversation, (), _) = state_update_conversation_metadata_cached(
            state,
            normalized_conversation_id,
            |conversation| {
                if let Some(value) = shell_workspace_path {
                    conversation.shell_workspace_path = value;
                }
                if let Some(value) = shell_workspaces {
                    conversation.shell_workspaces = value;
                }
                if let Some(value) = shell_autonomous_mode {
                    conversation.shell_autonomous_mode = value;
                }
                if conversation
                    .shell_workspace_path
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .is_some()
                    && terminal_workspace_path_from_conversation(state, conversation).is_none()
                {
                    conversation.shell_workspace_path = None;
                }
                Ok(())
            },
        )?;
        drop(guard);
        Ok(conversation)
    }

    #[allow(dead_code)]
    fn set_conversation_lifecycle_metadata(
        &self,
        state: &AppState,
        conversation_id: &str,
        status: Option<&str>,
        summary: Option<&str>,
        archived_at: Option<Option<String>>,
        updated_at: Option<String>,
    ) -> Result<Conversation, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let normalized_status = status.map(|value| value.trim().to_string());
        let next_summary = summary.map(ToOwned::to_owned);
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let (conversation, (), _) = state_update_conversation_metadata_cached(
            state,
            normalized_conversation_id,
            |conversation| {
                if let Some(value) = normalized_status {
                    conversation.status = value;
                }
                if let Some(value) = next_summary {
                    conversation.summary = value;
                }
                if let Some(value) = archived_at {
                    conversation.archived_at = value;
                }
                if let Some(value) = updated_at {
                    conversation.updated_at = value;
                }
                Ok(())
            },
        )?;
        drop(guard);
        Ok(conversation)
    }

    fn set_conversation_routing_metadata(
        &self,
        state: &AppState,
        conversation_id: &str,
        department_id: Option<&str>,
        agent_id: Option<&str>,
        root_conversation_id: Option<Option<String>>,
        conversation_kind: Option<&str>,
    ) -> Result<Conversation, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let next_department_id = department_id.map(|value| value.trim().to_string());
        let next_agent_id = agent_id.map(|value| value.trim().to_string());
        let next_conversation_kind = conversation_kind.map(|value| value.trim().to_string());
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let (conversation, (), _) = state_update_conversation_metadata_cached(
            state,
            normalized_conversation_id,
            |conversation| {
                if let Some(value) = next_department_id {
                    conversation.department_id = value;
                }
                if let Some(value) = next_agent_id {
                    conversation.agent_id = value;
                }
                if let Some(value) = root_conversation_id {
                    conversation.root_conversation_id = value;
                }
                if let Some(value) = next_conversation_kind {
                    conversation.conversation_kind = value;
                }
                Ok(())
            },
        )?;
        drop(guard);
        Ok(conversation)
    }

    fn append_tool_group_result(
        &self,
        state: &AppState,
        conversation_id: &str,
        agent_id: &str,
        assistant_tool_call_event: Value,
        tool_result_event: Value,
        provider_meta_patch: Option<Value>,
    ) -> Result<message_store::MessageStoreToolCallResultAppend, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let guard = lock_conversation_with_metrics(state, "append_tool_group_result")?;
        let conversation = state_read_conversation_cached(state, normalized_conversation_id)?;
        self.ensure_unarchived_conversation(&conversation, normalized_conversation_id)?;
        let append = message_store::apply_message_store_tool_group_result(
            &conversation,
            agent_id,
            assistant_tool_call_event,
            tool_result_event,
            provider_meta_patch,
        )?;
        state_schedule_conversation_persist(state, &append.conversation)?;
        drop(guard);
        Ok(append)
    }

    fn update_conversation_todos(
        &self,
        state: &AppState,
        conversation_id: &str,
        stored_todos: &[ConversationTodoItem],
    ) -> Result<Option<ConversationTodosUpdateResult>, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Ok(None);
        }
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let conversation = match state_read_conversation_cached(state, normalized_conversation_id) {
            Ok(conversation) => conversation,
            Err(err) => {
                runtime_log_debug(format!(
                    "[Todo] 读取会话失败，函数=update_conversation_todos，conversation_id={}，error={}",
                    normalized_conversation_id, err
                ));
                drop(guard);
                return Ok(None);
            }
        };
        if !conversation.summary.trim().is_empty() {
            drop(guard);
            return Ok(None);
        }
        if conversation.current_todos == stored_todos {
            drop(guard);
            return Ok(None);
        }
        drop(guard);
        let updated = self.set_conversation_current_todos_metadata(
            state,
            normalized_conversation_id,
            stored_todos.to_vec(),
        )?;
        let current_todo = conversation_current_todo_text(&updated);
        Ok(Some(ConversationTodosUpdateResult { current_todo }))
    }
}

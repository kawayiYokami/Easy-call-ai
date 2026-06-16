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

fn ensure_ready_message_store_from_legacy_conversation(
    state: &AppState,
    conversation_id: &str,
    store_paths: &message_store::MessageStorePaths,
) -> Result<(), String> {
    let normalized_conversation_id = conversation_id.trim();
    if normalized_conversation_id.is_empty() {
        return Err("conversationId is required.".to_string());
    }
    if message_store::read_ready_message_store_status(store_paths)?.is_some() {
        return Ok(());
    }
    let conversation =
        read_legacy_conversation_snapshot_for_ready_store_recovery(state, normalized_conversation_id)?;
    let recovery_job_id = format!("runtime-ready-store-recover-{normalized_conversation_id}");
    let recovery_reason =
        format!("运行时补建 ready message store，conversation_id={normalized_conversation_id}");
    conversation_service_v2().recover_conversation_snapshot(
        state,
        &recovery_job_id,
        "runtime_ready_store_recover",
        &recovery_reason,
        &conversation,
    )?;
    flush_pending_persists_blocking(state)?;
    Ok(())
}

// 这里是普通业务路径之外的唯一旧快照白名单读取口：
// 当 ready message store 尚未建立时，只能先读取历史 conversation 分片快照，
// 再立即通过 V2 特权恢复入口补建 store。其他业务代码禁止复用这条路径。
fn read_legacy_conversation_snapshot_for_ready_store_recovery(
    state: &AppState,
    conversation_id: &str,
) -> Result<Conversation, String> {
    state_read_conversation_cached(state, conversation_id)
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
        active_goal: goal_active_goal_from_conversation(conversation),
    })
}

fn build_foreground_conversation_snapshot_from_meta_view(
    state: &AppState,
    conversation_meta: &ConversationMetaView,
    recent_limit: usize,
) -> Result<ForegroundConversationSnapshotCore, String> {
    let conversation_id = conversation_meta.id.to_string();
    let paths = message_store::message_store_paths(&state.data_path, &conversation_id)?;
    ensure_ready_message_store_from_legacy_conversation(state, &conversation_id, &paths)?;
    let (messages, has_more_history) = if let Some(page) =
        message_store::read_ready_message_store_recent_messages_page_cached(&paths, recent_limit)?
    {
        if let Err(err) = conversation_service_v2().retain_message_store_block_cache_whitelist(state) {
            runtime_log_warn(format!(
                "[消息存储] 警告，任务=retain_message_store_block_cache_whitelist，conversation_id={}，error={}",
                conversation_id, err
            ));
        }
        (page.messages, page.has_more)
    } else {
        let messages = message_store::read_ready_message_store_all_messages(&paths)?.unwrap_or_default();
        let total_messages = messages.len();
        let start = total_messages.saturating_sub(recent_limit);
        (messages[start..].to_vec(), start > 0)
    };
    Ok(ForegroundConversationSnapshotCore {
        conversation_id: conversation_id.clone(),
        messages,
        has_more_history,
        runtime_state: unarchived_conversation_runtime_state(state, &conversation_id),
        current_todo: conversation_current_todo_text_from_items(&conversation_meta.current_todos),
        current_todos: conversation_meta.current_todos.clone(),
        preferred_api_config_id: conversation_meta.preferred_api_config_id.clone(),
        active_goal: conversation_meta.active_goal.clone(),
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
        if let Err(err) = conversation_service_v2().retain_message_store_block_cache_whitelist(state) {
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

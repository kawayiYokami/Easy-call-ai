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

impl ConversationService {
    fn persist_conversation_with_chat_index(
        &self,
        state: &AppState,
        conversation: &Conversation,
    ) -> Result<(), String> {
        state_schedule_conversation_persist(state, conversation, true).map(|_| ())
    }

    fn append_tool_call_result_pair(
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
        let _guard = lock_conversation_with_metrics(state, "append_tool_call_result_pair")?;
        let conversation = state_read_conversation_cached(state, normalized_conversation_id)?;
        self.ensure_unarchived_conversation(&conversation, normalized_conversation_id)?;
        let paths = message_store::message_store_paths(&state.data_path, normalized_conversation_id)?;
        let append = message_store::append_message_store_tool_call_result_pair(
            &paths,
            &conversation,
            agent_id,
            assistant_tool_call_event,
            tool_result_event,
            provider_meta_patch,
        )?;
        state_mark_conversation_direct_persisted(state, &append.conversation, true)?;
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
        let mut conversation = match state_read_conversation_cached(state, normalized_conversation_id) {
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
        conversation.current_todos = stored_todos.to_vec();
        let current_todo = conversation_current_todo_text(&conversation);
        state_schedule_conversation_persist(state, &conversation, false)?;
        drop(guard);
        Ok(Some(ConversationTodosUpdateResult { current_todo }))
    }
}

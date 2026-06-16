enum StopChatConversationTarget {
    Runtime(Conversation),
    PersistedRef {
        conversation_id: String,
        last_message: Option<ChatMessage>,
    },
}

impl StopChatConversationTarget {
    fn conversation_id(&self) -> &str {
        match self {
            Self::Runtime(conversation) => conversation.id.as_str(),
            Self::PersistedRef {
                conversation_id, ..
            } => conversation_id.as_str(),
        }
    }

    fn last_message(&self) -> Option<&ChatMessage> {
        match self {
            Self::Runtime(conversation) => conversation.messages.last(),
            Self::PersistedRef { last_message, .. } => last_message.as_ref(),
        }
    }
}

fn maybe_undo_rewind_apply_patch(
    state: &AppState,
    input: &RewindConversationInput,
    removed_messages: &[ChatMessage],
    message_id: &str,
    started_at: &std::time::Instant,
) -> Result<(), String> {
    if !input.undo_apply_patch {
        return Ok(());
    }
    runtime_log_info(format!(
        "[会话撤回] 开始工具逆向，任务=rewind_conversation_from_message，removed_messages={}，message_id={}",
        removed_messages.len(),
        message_id
    ));
    let (undone_patch_count, overwritten_files) = match try_undo_apply_patch_from_removed_messages(state, removed_messages) {
        Ok(value) => value,
        Err(err) => {
            let elapsed_ms = started_at.elapsed().as_millis();
            runtime_log_error(format!(
                "[会话撤回] 失败，任务=rewind_conversation_from_message，stage=undo_apply_patch，message_id={}，duration_ms={}，error={}",
                message_id, elapsed_ms, err
            ));
            return Err(err);
        }
    };
    runtime_log_info(format!(
        "[会话撤回] 工具逆向处理，任务=rewind_conversation_from_message，patches={}，overwritten={}，message_id={}",
        undone_patch_count, overwritten_files.len(), message_id
    ));
    if undone_patch_count > 0 {
        eprintln!(
            "[会话撤回] 已执行 apply_patch 反向撤回: patches={}, message_id={}",
            undone_patch_count,
            message_id
        );
    }
    if !overwritten_files.is_empty() {
        runtime_log_warn(format!(
            "[会话撤回] 有 {} 个文件存在非LLM修改被覆盖: {:?}",
            overwritten_files.len(),
            overwritten_files
        ));
    }
    Ok(())
}

fn resolve_stop_chat_api_config_id(
    app_config: &AppConfig,
    requested_department_id: Option<&str>,
    agent_id: &str,
) -> Result<String, String> {
    let raw_api_config_id = requested_department_id
        .and_then(|id| department_by_id(app_config, id))
        .map(department_primary_api_config_id)
        .or_else(|| department_for_agent_id(app_config, agent_id).map(department_primary_api_config_id))
        .or_else(|| resolve_selected_api_config(app_config, None).map(|api| api.id.clone()))
        .ok_or_else(|| "Missing available API config for stop request".to_string())?;
    resolve_model_role_api_config_id(app_config, &raw_api_config_id)
        .ok_or_else(|| format!("Model role '{raw_api_config_id}' is not configured."))
}

fn resolve_stop_chat_target(
    state: &AppState,
    requested_conversation_id: Option<&str>,
    agent_id: &str,
) -> Result<Option<StopChatConversationTarget>, String> {
    let runtime_requested = requested_conversation_id
        .filter(|conversation_id| {
            delegate_runtime_thread_conversation_get(state, conversation_id)
                .ok()
                .flatten()
                .is_some()
        })
        .map(ToOwned::to_owned);
    if let Some(conversation_id) = runtime_requested.as_deref() {
        let runtime_conversation = delegate_runtime_thread_conversation_get(state, conversation_id)?;
        return Ok(runtime_conversation.map(StopChatConversationTarget::Runtime));
    }
    let conversation_id = if let Some(conversation_id) = requested_conversation_id {
        Some(conversation_id.to_string())
    } else {
        conversation_service_v2().resolve_latest_foreground_conversation_id(state, agent_id)?
    };
    Ok(conversation_id.and_then(|conversation_id| {
        let conversation_meta = conversation_service_v2()
            .get_conversation_meta(state, &conversation_id)
            .ok()?;
        if !conversation_service_v2().conversation_meta_is_unarchived_meta_view(&conversation_meta)
            || !conversation_meta.visible_in_foreground_lists
        {
            return None;
        }
        let paths = message_store::message_store_paths(&state.data_path, &conversation_id).ok()?;
        let last_message = message_store::read_ready_message_store_recent_messages_page_cached(
            &paths,
            1,
        )
        .ok()
        .flatten()
        .and_then(|page| page.messages.into_iter().last());
        Some(StopChatConversationTarget::PersistedRef {
            conversation_id,
            last_message,
        })
    }))
}

fn build_stop_chat_skip_result(target: &StopChatConversationTarget) -> Option<StopChatPersistResult> {
    let last_message = target.last_message()?;
    if last_message.role == "assistant" {
        return Some(StopChatPersistResult {
            persisted: false,
            conversation_id: Some(target.conversation_id().to_string()),
            assistant_message: Some(last_message.clone()),
        });
    }
    None
}

fn build_stop_chat_partial_assistant_message(
    agent_id: &str,
    partial_assistant_text: &str,
    partial_activity_reasoning_text: &str,
    _partial_inline_activity_text: &str,
    completed_tool_history: &[Value],
) -> ChatMessage {
    let now = now_iso();
    let request_messages = assistant_request_sequence_from_tool_history(
        completed_tool_history,
        partial_assistant_text,
        partial_activity_reasoning_text,
    );
    build_assistant_message_from_request_sequence(
        Uuid::new_v4().to_string(),
        agent_id,
        now,
        &request_messages,
        None,
    )
}

fn apply_stop_chat_partial_message(
    conversation: &mut Conversation,
    assistant_message: &ChatMessage,
) -> String {
    conversation.messages.push(assistant_message.clone());
    conversation.updated_at = assistant_message.created_at.clone();
    conversation.last_assistant_at = Some(assistant_message.created_at.clone());
    conversation.id.clone()
}

fn read_latest_archive_summary_from_chat_index(state: &AppState) -> Result<Option<String>, String> {
    let chat_index = state_read_chat_index_cached(state)?;
    Ok(chat_index
        .conversations
        .iter()
        .rev()
        .filter_map(|item| conversation_service_v2().get_conversation_meta(state, item.id.as_str()).ok())
        .find(|conversation_meta| {
            !conversation_meta.is_delegate
                && !conversation_meta.summary.trim().is_empty()
        })
        .map(|conversation_meta| conversation_meta.summary.to_string()))
}

fn create_unarchived_conversation_shared(
    state: &AppState,
    input: &CreateUnarchivedConversationInput,
) -> Result<CreateUnarchivedConversationMutationResult, String> {
    let started_at = std::time::Instant::now();
    let guard = state
        .conversation_lock
        .lock()
        .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
    let runtime_snapshot = load_runtime_organization_snapshot(state)?;
    let app_config = runtime_snapshot.config.clone();
    let runtime = state_read_runtime_state_cached(state)?;
    let agents = runtime_snapshot.agents.clone();
    let requested_department_id = input
        .department_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let department_id = requested_department_id
        .ok_or_else(|| "新建会话必须选择部门。".to_string())?;
    let department = runtime_department_by_id(&runtime_snapshot, department_id)
        .ok_or_else(|| format!("Department '{department_id}' not found."))?;
    let api_config_id = input
        .api_config_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| department_primary_api_config_id(department));
    let agent_id = input
        .agent_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("新建会话必须选择人格，department_id={}", department.id))?
        .to_string();
    if !department.agent_ids.iter().any(|id| id.trim() == agent_id) {
        return Err(format!(
            "新建会话的人格不属于所选部门: department_id={}，agent_id={}",
            department.id, agent_id
        ));
    }
    if !agents
        .iter()
        .any(|agent| agent.id == agent_id && !agent.is_built_in_user)
    {
        return Err(format!("新建会话的人格不存在或不可用: agent_id={agent_id}"));
    }
    let conversation_title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_default();
    let copy_source_conversation_id = trimmed_option(input.copy_source_conversation_id.as_deref());
    let mut conversation = if let Some(source_conversation_id) = copy_source_conversation_id.as_deref() {
        let source_conversation = conversation_service_v2()
            .try_get_conversation_snapshot(state, source_conversation_id)?
            .filter(|conversation| {
                conversation.summary.trim().is_empty()
                    && conversation_visible_in_foreground_lists(conversation)
                    && conversation_is_local_normal_chat(conversation)
            })
            .ok_or_else(|| "要复制的当前会话不存在或已归档".to_string())?;
        clone_foreground_conversation_for_copy(
            &source_conversation,
            &agent_id,
            &department.id,
            conversation_title,
        )
    } else {
        build_unarchived_conversation_record_from_runtime(
            &state.data_path,
            &agents,
            &runtime.assistant_department_agent_id,
            read_latest_archive_summary_from_chat_index(state)?,
            &api_config_id,
            &agent_id,
            &department.id,
            conversation_title,
        )
    };
    conversation.preferred_api_config_id = resolve_model_role_api_config_id(&app_config, &api_config_id)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty() && !is_model_role_api_config_id(value))
        .filter(|value| {
            app_config
                .api_configs
                .iter()
                .any(|api| api.id == *value && is_text_chat_api(api))
        });
    if let Some(shell_workspaces) = input.shell_workspaces.as_ref() {
        conversation.shell_workspaces =
            normalize_conversation_shell_workspaces(state, shell_workspaces);
        conversation.shell_workspace_path = None;
    }
    if let Some(shell_autonomous_mode) = input.shell_autonomous_mode {
        conversation.shell_autonomous_mode = shell_autonomous_mode;
    }
    let conversation_id = conversation.id.clone();
    let persist_seq = state_schedule_conversation_persist(state, &conversation)?;
    runtime_log_info(format!(
        "[会话] 完成，任务=新建未归档会话，阶段=调度持久化，conversation_id={}，persist_seq={}，department_id={}，agent_id={}，preferred_api_config_id={}，message_count={}，duration_ms={}",
        conversation_id,
        persist_seq,
        conversation.department_id,
        conversation.agent_id,
        conversation.preferred_api_config_id.as_deref().unwrap_or(""),
        conversation.messages.len(),
        started_at.elapsed().as_millis()
    ));
    let overview_payload = UnarchivedConversationOverviewUpdatedPayload {
        preferred_conversation_id: Some(conversation_id.clone()),
        unarchived_conversations: conversation_service_v2()
            .collect_unarchived_conversation_summaries_cached(state, &app_config)?,
    };
    runtime_log_info(format!(
        "[会话] 完成，任务=新建未归档会话，阶段=构建概览，conversation_id={}，overview_count={}，duration_ms={}",
        conversation_id,
        overview_payload.unarchived_conversations.len(),
        started_at.elapsed().as_millis()
    ));
    drop(guard);
    Ok(CreateUnarchivedConversationMutationResult {
        conversation_id,
        overview_payload,
    })
}

fn build_unarchived_conversation_record_from_runtime(
    data_path: &PathBuf,
    agents: &[AgentProfile],
    assistant_department_agent_id: &str,
    _last_archive_summary: Option<String>,
    api_config_id: &str,
    agent_id: &str,
    department_id: &str,
    title: &str,
) -> Conversation {
    let mut conversation = build_conversation_record(
        api_config_id,
        agent_id,
        department_id,
        title,
        CONVERSATION_KIND_CHAT,
        None,
        None,
    );
    let snapshot_agent_id = if agent_id.trim().is_empty() {
        assistant_department_agent_id.trim().to_string()
    } else {
        agent_id.trim().to_string()
    };
    let user_profile_snapshot = agents
        .iter()
        .find(|item| item.id == snapshot_agent_id)
        .and_then(|agent| match build_user_profile_snapshot_block(data_path, agent, 12) {
            Ok(snapshot) => snapshot,
            Err(err) => {
                runtime_log_error(format!(
                    "[用户画像] 失败，任务=build_unarchived_conversation_record_from_runtime，agent_id={}，error={}",
                    agent.id, err
                ));
                None
            }
        });
    if let Some(snapshot) = user_profile_snapshot.clone() {
        conversation.user_profile_snapshot = snapshot;
    }
    let summary_message = build_initial_summary_context_message(
        user_profile_snapshot.as_deref(),
        Some(&conversation.current_todos),
        None,
    );
    conversation.last_user_at = Some(summary_message.created_at.clone());
    conversation.updated_at = summary_message.created_at.clone();
    conversation.messages.push(summary_message);
    conversation
}

fn branch_conversation_settings_agent_id_runtime(
    agents: &[AgentProfile],
    department: &DepartmentConfig,
    requested_agent_id: &str,
) -> Result<String, String> {
    let normalized_requested_agent_id = requested_agent_id.trim();
    if normalized_requested_agent_id.is_empty() {
        return first_available_department_agent(department, agents)
            .map(|agent| agent.id.clone())
            .ok_or_else(|| format!("源会话绑定部门没有可用人格，无法创建分支: department_id={}", department.id));
    }
    if available_non_user_agent(agents, normalized_requested_agent_id).is_some() {
        return Ok(normalized_requested_agent_id.to_string());
    }
    Err(format!(
        "源会话绑定人格不存在或不可用，无法创建分支: agent_id={normalized_requested_agent_id}"
    ))
}

fn build_branch_conversation_record_from_selection_runtime_meta_view(
    data_path: &PathBuf,
    agents: &[AgentProfile],
    source_meta: &ConversationMetaView,
    department: &DepartmentConfig,
    title: &str,
    latest_compaction_message: Option<&ChatMessage>,
    selected_messages: &[ChatMessage],
) -> Result<Conversation, String> {
    let agent_id =
        branch_conversation_settings_agent_id_runtime(agents, department, &source_meta.agent_id)?;
    let mut conversation = build_conversation_record(
        &department_primary_api_config_id(department),
        &agent_id,
        &department.id,
        title,
        CONVERSATION_KIND_CHAT,
        None,
        None,
    );
    conversation.parent_conversation_id = Some(source_meta.id.clone());
    conversation.plan_mode_enabled = source_meta.plan_mode_enabled;
    conversation.shell_workspace_path = source_meta.shell_workspace_path.clone();
    conversation.shell_workspaces = source_meta.shell_workspaces.clone();
    conversation.shell_autonomous_mode = source_meta.shell_autonomous_mode;
    conversation.current_todos = source_meta.current_todos.clone();
    let user_profile_snapshot = agents
        .iter()
        .find(|item| item.id == agent_id)
        .and_then(|agent| match build_user_profile_snapshot_block(data_path, agent, 12) {
            Ok(snapshot) => snapshot,
            Err(err) => {
                runtime_log_warn(format!(
                    "[会话分支] 跳过，任务=构建用户画像快照，agent_id={}，error={}",
                    agent.id, err
                ));
                None
            }
        })
        .or_else(|| {
            let snapshot = source_meta.user_profile_snapshot.trim();
            if snapshot.is_empty() {
                None
            } else {
                Some(snapshot.to_string())
            }
        });
    if let Some(snapshot) = user_profile_snapshot.clone() {
        conversation.user_profile_snapshot = snapshot;
    }
    if let Some(message) = latest_compaction_message {
        conversation
            .messages
            .push(clone_chat_message_for_copied_conversation(message));
    } else {
        conversation.messages.push(build_initial_summary_context_message(
            user_profile_snapshot.as_deref(),
            Some(&conversation.current_todos),
            None,
        ));
    }
    conversation.messages.extend(
        selected_messages
            .iter()
            .map(clone_chat_message_for_copied_conversation),
    );
    if let Some(last_message) = conversation.messages.last() {
        conversation.unread_count = 0;
        conversation.updated_at = last_message.created_at.clone();
        conversation.last_user_at = Some(last_message.created_at.clone());
    }
    Ok(conversation)
}

fn read_latest_visible_foreground_conversation_metadata(
    state: &AppState,
) -> Result<Option<ConversationMetaView>, String> {
    let chat_index = state_read_chat_index_cached(state)?;
    Ok(chat_index
        .conversations
        .iter()
        .filter_map(|item| conversation_service_v2().get_conversation_meta(state, item.id.as_str()).ok())
        .filter(|conversation_meta| {
            conversation_service_v2().conversation_meta_is_unarchived_meta_view(conversation_meta)
                && conversation_meta.visible_in_foreground_lists
                && conversation_service_v2().conversation_meta_is_local_normal_chat_meta_view(conversation_meta)
        })
        .max_by(|a, b| {
            a.updated_at
                .trim()
                .cmp(b.updated_at.trim())
                .then_with(|| a.id.cmp(&b.id))
        }))
}

fn read_branch_selection_or_pending_conversation(
    state: &AppState,
    conversation_id: &str,
    selected_message_ids: &[String],
) -> Result<message_store::MessageStoreBranchSelection, String> {
    let store_paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
    ensure_ready_message_store_from_legacy_conversation(state, conversation_id, &store_paths)?;
    message_store::read_ready_message_store_branch_selection(&store_paths, selected_message_ids)?
        .ok_or_else(|| "源会话消息尚未就绪".to_string())
}

struct ReadyStoreRewindState {
    keep_count: usize,
    removed_messages: Vec<ChatMessage>,
    recalled_user_message: ChatMessage,
    remaining_last_message_at: Option<String>,
    remaining_last_user_at: Option<String>,
    remaining_last_assistant_at: Option<String>,
    remaining_todos: Vec<ConversationTodoItem>,
    remaining_body_message_count: usize,
    remaining_body_text_length: usize,
    remaining_has_context_compaction_message: bool,
    remaining_latest_summary_title: Option<String>,
    remaining_preview_messages: Vec<message_store::ConversationShardPreviewMessage>,
}

fn read_ready_store_rewind_state_meta_view(
    _state: &AppState,
    store_paths: &message_store::MessageStorePaths,
    conversation_meta: &ConversationMetaView,
    message_id: &str,
) -> Result<ReadyStoreRewindState, String> {
    let rewind_slice = message_store::read_ready_message_store_rewind_slice(store_paths, message_id)?
        .ok_or_else(|| "Target user message not found in active conversation.".to_string())?;
    let removed_body_message_count = rewind_slice
        .removed_messages
        .iter()
        .filter(|message| {
            matches!(
                message.role.trim().to_ascii_lowercase().as_str(),
                "user" | "assistant"
            )
        })
        .count();
    let removed_body_text_length = rewind_slice
        .removed_messages
        .iter()
        .flat_map(|message| message.parts.iter())
        .filter_map(|part| match part {
            MessagePart::Text { text, .. } => Some(text.trim().chars().count()),
            _ => None,
        })
        .sum::<usize>();
    let removed_has_context_compaction = rewind_slice
        .removed_messages
        .iter()
        .any(|message| is_context_compaction_message(message, message.role.trim()));
    let removed_latest_summary_title = rewind_slice
        .removed_messages
        .iter()
        .rev()
        .find_map(summary_context_message_title);
    let removed_may_clear_latest_summary = removed_latest_summary_title.as_deref()
        == conversation_meta.latest_summary_title.as_deref();

    let mut remaining_last_message_at = None::<String>;
    let mut remaining_last_user_at = None::<String>;
    let mut remaining_last_assistant_at = None::<String>;
    let mut remaining_todos = None::<Vec<ConversationTodoItem>>;
    let mut remaining_has_context_compaction_message =
        conversation_meta.has_context_compaction_message && !removed_has_context_compaction;
    let mut remaining_latest_summary_title = if removed_may_clear_latest_summary {
        None
    } else {
        conversation_meta.latest_summary_title.clone()
    };
    let mut preview_messages_latest_first =
        Vec::<message_store::ConversationShardPreviewMessage>::new();
    let mut before_anchor = message_id.trim().to_string();

    while !before_anchor.trim().is_empty() {
        let Some(page) = message_store::read_ready_message_store_messages_before(
            store_paths,
            &before_anchor,
            100,
        )? else {
            break;
        };
        if page.messages.is_empty() {
            break;
        }
        if remaining_last_message_at.is_none() {
            remaining_last_message_at = page.messages.last().map(|message| message.created_at.clone());
        }
        for message in page.messages.iter().rev() {
            if remaining_last_user_at.is_none() && message.role.trim().eq_ignore_ascii_case("user") {
                remaining_last_user_at = Some(message.created_at.clone());
            }
            if remaining_last_assistant_at.is_none()
                && message.role.trim().eq_ignore_ascii_case("assistant")
            {
                remaining_last_assistant_at = Some(message.created_at.clone());
            }
            if remaining_todos.is_none() {
                if let Some(todos) = latest_todos_from_message_tool_history(message)? {
                    remaining_todos = Some(todos);
                }
            }
            if !remaining_has_context_compaction_message
                && is_context_compaction_message(message, message.role.trim())
            {
                remaining_has_context_compaction_message = true;
            }
            if remaining_latest_summary_title.is_none() {
                remaining_latest_summary_title = summary_context_message_title(message);
            }
            if preview_messages_latest_first.len() < 2
                && matches!(
                    message.role.trim().to_ascii_lowercase().as_str(),
                    "user" | "assistant" | "tool"
                )
            {
                preview_messages_latest_first.push(message_store::ConversationShardPreviewMessage {
                    message_id: message.id.clone(),
                    role: message.role.clone(),
                    speaker_agent_id: message.speaker_agent_id.clone(),
                    created_at: Some(message.created_at.clone())
                        .filter(|value| !value.trim().is_empty()),
                    text_preview: build_conversation_preview_text(message),
                    has_image: message.parts.iter().any(|part| {
                        matches!(part, MessagePart::Image { mime, .. } if !mime.trim().eq_ignore_ascii_case("application/pdf"))
                    }),
                    has_pdf: message.parts.iter().any(|part| {
                        matches!(part, MessagePart::Image { mime, .. } if mime.trim().eq_ignore_ascii_case("application/pdf"))
                    }),
                    has_audio: message
                        .parts
                        .iter()
                        .any(|part| matches!(part, MessagePart::Audio { .. })),
                    has_attachment: conversation_message_has_attachment(message),
                });
            }
        }
        let done = remaining_last_user_at.is_some()
            && remaining_last_assistant_at.is_some()
            && remaining_todos.is_some()
            && remaining_latest_summary_title.is_some()
            && remaining_has_context_compaction_message
            && preview_messages_latest_first.len() >= 2;
        if done || !page.has_more {
            break;
        }
        before_anchor = page
            .messages
            .first()
            .map(|message| message.id.clone())
            .unwrap_or_default();
    }

    preview_messages_latest_first.reverse();
    let remaining_todos =
        remaining_todos.unwrap_or_else(|| conversation_meta.current_todos.clone());

    Ok(ReadyStoreRewindState {
        keep_count: rewind_slice.keep_count,
        removed_messages: rewind_slice.removed_messages,
        recalled_user_message: rewind_slice.recalled_user_message,
        remaining_last_message_at,
        remaining_last_user_at,
        remaining_last_assistant_at,
        remaining_todos,
        remaining_body_message_count: conversation_meta
            .body_message_count
            .saturating_sub(removed_body_message_count),
        remaining_body_text_length: conversation_meta
            .body_text_length
            .saturating_sub(removed_body_text_length),
        remaining_has_context_compaction_message,
        remaining_latest_summary_title,
        remaining_preview_messages: preview_messages_latest_first,
    })
}

fn read_conversation_for_backup_cleanup(
    state: &AppState,
    conversation_id: &str,
) -> Result<Conversation, String> {
    let conversation_meta = conversation_service_v2().get_conversation_meta(state, conversation_id)?;
    let store_paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
    ensure_ready_message_store_from_legacy_conversation(state, conversation_id, &store_paths)?;
    let messages = message_store::read_ready_message_store_all_messages(&store_paths)?
        .unwrap_or_default();
    Ok(Conversation {
        id: conversation_meta.id,
        title: conversation_meta.title,
        agent_id: conversation_meta.agent_id,
        department_id: conversation_meta.department_id,
        bound_conversation_id: None,
        parent_conversation_id: None,
        child_conversation_ids: Vec::new(),
        fork_message_cursor: None,
        unread_count: conversation_meta.unread_count,
        conversation_kind: conversation_meta.conversation_kind,
        root_conversation_id: conversation_meta.root_conversation_id,
        delegate_id: conversation_meta.delegate_id,
        created_at: conversation_meta.created_at,
        updated_at: conversation_meta.updated_at,
        last_user_at: None,
        last_assistant_at: None,
        status: conversation_meta.status,
        summary: conversation_meta.summary,
        user_profile_snapshot: conversation_meta.user_profile_snapshot,
        shell_workspace_path: conversation_meta.shell_workspace_path,
        shell_workspaces: conversation_meta.shell_workspaces,
        shell_autonomous_mode: conversation_meta.shell_autonomous_mode,
        archived_at: conversation_meta.archived_at,
        messages,
        current_todos: conversation_meta.current_todos,
        memory_recall_table: Vec::new(),
        plan_mode_enabled: conversation_meta.plan_mode_enabled,
        preferred_api_config_id: conversation_meta.preferred_api_config_id,
        auto_push_remote_contact_id: conversation_meta.auto_push_remote_contact_id,
        cumulative_usage: conversation_meta.cumulative_usage,
        active_goal: conversation_meta.active_goal,
    })
}

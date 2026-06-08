impl ConversationService {
    fn list_archives(&self, state: &AppState) -> Result<Vec<ArchiveSummary>, String> {
        let guard = state.conversation_lock.lock().map_err(|err| {
            named_lock_error("conversation_lock", file!(), line!(), module_path!(), &err)
        })?;

        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let chat_index = state_read_chat_index_cached(state)?;
        let mut summaries = chat_index
            .conversations
            .iter()
            .filter(|item| chat_index_item_is_archived(item))
            .filter_map(|item| match state_read_conversation_cached(state, item.id.as_str()) {
                Ok(conversation) => Some(conversation),
                Err(err) => {
                    eprintln!(
                        "[会话索引读取] 状态=失败，任务=list_archives，conversation_id={}，error={}",
                        item.id, err
                    );
                    None
                }
            })
            .filter(|conversation| conversation_is_archived(conversation))
            .map(|archive| {
                let api_config_id = runtime_department_by_id(
                    &runtime_snapshot,
                    archive.department_id.trim(),
                )
                .or_else(|| runtime_department_for_agent(&runtime_snapshot, &archive.agent_id))
                    .map(department_primary_api_config_id)
                    .unwrap_or_default();
                ArchiveSummary {
                    archive_id: archive.id.clone(),
                    archived_at: archive
                        .archived_at
                        .clone()
                        .unwrap_or_else(|| archive.updated_at.clone()),
                    title: if archive.title.trim().is_empty() {
                        archive_first_user_preview(&archive, &runtime_snapshot.config.ui_language)
                    } else {
                        archive.title.trim().to_string()
                    },
                    message_count: archive.messages.len(),
                    api_config_id,
                    agent_id: archive.agent_id.clone(),
                }
            })
            .collect::<Vec<_>>();
        summaries.sort_by(|a, b| b.archived_at.cmp(&a.archived_at));
        drop(guard);
        Ok(summaries)
    }

    fn read_archive_messages(
        &self,
        state: &AppState,
        archive_id: &str,
    ) -> Result<Vec<ChatMessage>, String> {
        let normalized_archive_id = archive_id.trim();
        if normalized_archive_id.is_empty() {
            return Err("archiveId is required".to_string());
        }
        let guard = state.conversation_lock.lock().map_err(|err| {
            named_lock_error("conversation_lock", file!(), line!(), module_path!(), &err)
        })?;
        let mut messages = state_read_conversation_cached(state, normalized_archive_id)
            .map_err(|_| "Archive not found".to_string())
            .and_then(|conversation| {
                if !conversation_is_archived(&conversation) {
                    Err("Archive not found".to_string())
                } else {
                    Ok(conversation.messages)
                }
            })?;
        drop(guard);
        materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
        Ok(messages)
    }

    fn read_archive_block_page(
        &self,
        state: &AppState,
        archive_id: &str,
        requested_block_id: Option<u32>,
    ) -> Result<ConversationBlockPageResult, String> {
        let normalized_archive_id = archive_id.trim();
        if normalized_archive_id.is_empty() {
            return Err("archiveId is required".to_string());
        }
        let store_paths = message_store::message_store_paths(&state.data_path, normalized_archive_id)?;
        if let Some(page) =
            message_store::read_ready_message_store_block_page(&store_paths, requested_block_id)?
        {
            let mut messages = page.messages;
            materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
            return Ok(ConversationBlockPageResult {
                blocks: page
                    .blocks
                    .into_iter()
                    .map(|item| ConversationBlockSummaryResult {
                        block_id: item.block_id,
                        message_count: item.message_count,
                        first_message_id: item.first_message_id,
                        last_message_id: item.last_message_id,
                        first_created_at: item.first_created_at,
                        last_created_at: item.last_created_at,
                        is_latest: item.is_latest,
                    })
                    .collect(),
                selected_block_id: page.selected_block_id,
                messages,
                has_prev_block: page.has_prev_block,
                has_next_block: page.has_next_block,
            });
        }

        let guard = state.conversation_lock.lock().map_err(|err| {
            named_lock_error("conversation_lock", file!(), line!(), module_path!(), &err)
        })?;
        let conversation = state_read_conversation_cached(state, normalized_archive_id)
            .map_err(|_| "Archive not found".to_string())?;
        if !conversation_is_archived(&conversation) {
            drop(guard);
            return Err("Archive not found".to_string());
        }
        let mut page = ConversationBlockPageResult {
            blocks: vec![ConversationBlockSummaryResult {
                block_id: 0,
                message_count: conversation.messages.len(),
                first_message_id: conversation
                    .messages
                    .first()
                    .map(|message| message.id.clone())
                    .unwrap_or_default(),
                last_message_id: conversation
                    .messages
                    .last()
                    .map(|message| message.id.clone())
                    .unwrap_or_default(),
                first_created_at: conversation
                    .messages
                    .first()
                    .map(|message| message.created_at.clone()),
                last_created_at: conversation
                    .messages
                    .last()
                    .map(|message| message.created_at.clone()),
                is_latest: true,
            }],
            selected_block_id: 0,
            messages: conversation.messages,
            has_prev_block: false,
            has_next_block: false,
        };
        drop(guard);
        materialize_chat_message_parts_from_media_refs(&mut page.messages, &state.data_path);
        Ok(page)
    }

    fn read_archive_summary(&self, state: &AppState, archive_id: &str) -> Result<String, String> {
        let normalized_archive_id = archive_id.trim();
        if normalized_archive_id.is_empty() {
            return Err("archiveId is required".to_string());
        }
        let guard = state.conversation_lock.lock().map_err(|err| {
            named_lock_error("conversation_lock", file!(), line!(), module_path!(), &err)
        })?;
        let summary = state_read_conversation_cached(state, normalized_archive_id)
            .map_err(|_| "Archive not found".to_string())
            .and_then(|conversation| {
                if !conversation_is_archived(&conversation) {
                    Err("Archive not found".to_string())
                } else {
                    Ok(conversation.summary)
                }
            })?;
        drop(guard);
        Ok(summary)
    }

    fn resolve_archive_target_conversation(
        &self,
        state: &AppState,
        input: &SessionSelector,
    ) -> Result<(ApiConfig, ResolvedApiConfig, Conversation, String), String> {
        let guard = state.conversation_lock.lock().map_err(|err| {
            format!(
                "Failed to lock state mutex at {}:{} {}: {err}",
                file!(),
                line!(),
                module_path!()
            )
        })?;
        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let app_config = runtime_snapshot.config;
        let runtime = state_read_runtime_state_cached(state)?;
        let runtime_agents = runtime_snapshot.agents;
        let selected_api = resolve_selected_api_config(&app_config, input.api_config_id.as_deref())
            .ok_or_else(|| "No API config configured. Please add one.".to_string())?;
        let resolved_api = resolve_api_config(&app_config, Some(selected_api.id.as_str()))?;
        let requested_agent_id = input.agent_id.trim();
        let effective_agent_id = if runtime_agents
            .iter()
            .any(|agent| agent.id == requested_agent_id && !agent.is_built_in_user)
        {
            requested_agent_id.to_string()
        } else if runtime_agents.iter().any(|agent| {
            agent.id == runtime.assistant_department_agent_id && !agent.is_built_in_user
        }) {
            runtime.assistant_department_agent_id.clone()
        } else {
            runtime_agents
                .iter()
                .find(|agent| !agent.is_built_in_user)
                .map(|agent| agent.id.clone())
                .ok_or_else(|| "Selected agent not found.".to_string())?
        };
        let source_conversation_id = input
            .conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let source_conversation_id = if let Some(conversation_id) = source_conversation_id {
            let conversation = state_read_conversation_cached(state, conversation_id)
                .map_err(|_| "当前没有可归档的活动对话。".to_string())?;
            if conversation_is_unarchived_foreground(&conversation) {
                Some(conversation.id)
            } else {
                None
            }
        } else {
            self.resolve_latest_foreground_conversation_id(state, &effective_agent_id)?
        }
        .ok_or_else(|| "当前没有可归档的活动对话。".to_string())?;
        let source = state_read_conversation_cached(state, &source_conversation_id)
            .map_err(|_| "当前没有可归档的活动对话。".to_string())?;
        if !conversation_is_unarchived_foreground(&source) {
            drop(guard);
            return Err("当前没有可归档的活动对话。".to_string());
        }
        drop(guard);
        Ok((selected_api, resolved_api, source, effective_agent_id))
    }

    fn resolve_archive_request_conversation(
        &self,
        state: &AppState,
        input: &SessionSelector,
    ) -> Result<(ApiConfig, ResolvedApiConfig, Conversation, String), String> {
        let guard = state.conversation_lock.lock().map_err(|err| {
            format!(
                "Failed to lock state mutex at {}:{} {}: {err}",
                file!(),
                line!(),
                module_path!()
            )
        })?;
        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let app_config = runtime_snapshot.config;
        let runtime = state_read_runtime_state_cached(state)?;
        let runtime_agents = runtime_snapshot.agents;
        let selected_api = resolve_selected_api_config(&app_config, input.api_config_id.as_deref())
            .ok_or_else(|| "No API config configured. Please add one.".to_string())?;
        let resolved_api = resolve_api_config(&app_config, Some(selected_api.id.as_str()))?;
        let requested_agent_id = input.agent_id.trim();
        let effective_agent_id = if runtime_agents
            .iter()
            .any(|agent| agent.id == requested_agent_id && !agent.is_built_in_user)
        {
            requested_agent_id.to_string()
        } else if runtime_agents.iter().any(|agent| {
            agent.id == runtime.assistant_department_agent_id && !agent.is_built_in_user
        }) {
            runtime.assistant_department_agent_id.clone()
        } else {
            runtime_agents
                .iter()
                .find(|agent| !agent.is_built_in_user)
                .map(|agent| agent.id.clone())
                .ok_or_else(|| "Selected agent not found.".to_string())?
        };
        let source_conversation_id = input
            .conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let source_conversation_id = if let Some(conversation_id) = source_conversation_id {
            let conversation = state_read_conversation_cached(state, conversation_id)
                .map_err(|_| "当前没有可归档的活动对话。".to_string())?;
            if conversation_is_unarchived_foreground(&conversation)
                || conversation_is_archived(&conversation)
            {
                Some(conversation.id)
            } else {
                None
            }
        } else {
            self.resolve_latest_foreground_conversation_id(state, &effective_agent_id)?
        }
        .ok_or_else(|| "当前没有可归档的活动对话。".to_string())?;
        let source = state_read_conversation_cached(state, &source_conversation_id)
            .map_err(|_| "当前没有可归档的活动对话。".to_string())?;
        if !conversation_is_unarchived_foreground(&source) && !conversation_is_archived(&source) {
            drop(guard);
            return Err("当前没有可归档的活动对话。".to_string());
        }
        drop(guard);
        Ok((selected_api, resolved_api, source, effective_agent_id))
    }

    fn delete_archive(&self, state: &AppState, archive_id: &str) -> Result<(), String> {
        let normalized_archive_id = archive_id.trim();
        if normalized_archive_id.is_empty() {
            return Err("archiveId is required".to_string());
        }
        let guard = state.conversation_lock.lock().map_err(|err| {
            named_lock_error("conversation_lock", file!(), line!(), module_path!(), &err)
        })?;
        let conversation = state_read_conversation_cached(state, normalized_archive_id)
            .map_err(|_| "Archive not found".to_string())?;
        if !conversation_is_archived(&conversation) {
            drop(guard);
            return Err("Archive not found".to_string());
        }
        state_schedule_conversation_delete(state, normalized_archive_id)?;
        drop(guard);
        Ok(())
    }

    fn instant_archive_conversation(
        &self,
        state: &AppState,
        selected_api: &ApiConfig,
        source: &Conversation,
        archive_reason: &str,
    ) -> Result<InstantArchiveConversationMutationResult, String> {
        let guard = state.conversation_lock.lock().map_err(|err| {
            format!(
                "Failed to lock state mutex at {}:{} {}: {err}",
                file!(),
                line!(),
                module_path!()
            )
        })?;
        let source_conversation = state_read_conversation_cached(state, &source.id)
            .map_err(|err| format!("当前没有可归档的活动对话：{}", err))?;
        let already_archived = conversation_is_archived(&source_conversation);
        if !already_archived && !conversation_is_unarchived_foreground(&source_conversation) {
            drop(guard);
            return Err("当前没有可归档的活动对话。".to_string());
        }

        let runtime = state_read_runtime_state_cached(state)?;
        let agents = state_read_agents_cached(state)?;
        let chat_index = state_read_chat_index_cached(state)?;
        let active_conversation_id = if let Some(conversation_id) =
            chat_index
                .conversations
                .iter()
                .enumerate()
                .filter_map(|(idx, item)| {
                    let conversation = state_read_conversation_cached(state, item.id.as_str()).ok()?;
                    Some((idx, conversation))
                })
                .filter(|(_, conversation)| {
                    conversation.id != source.id
                        && conversation_is_unarchived_foreground(conversation)
                })
                .max_by(|(idx_a, a), (idx_b, b)| {
                    let a_updated = a.updated_at.trim();
                    let b_updated = b.updated_at.trim();
                    let a_created = a.created_at.trim();
                    let b_created = b.created_at.trim();
                    a_updated
                        .cmp(b_updated)
                        .then_with(|| a_created.cmp(b_created))
                        .then_with(|| idx_a.cmp(idx_b))
                })
                .map(|(_, conversation)| conversation.id)
        {
            conversation_id
        } else {
            let conversation =
                build_archive_replacement_conversation(
                    state,
                    &agents,
                    &runtime.assistant_department_agent_id,
                    selected_api,
                    source,
                )?;
            let conversation_id = conversation.id.clone();
            state_schedule_conversation_persist(state, &conversation)?;
            conversation_id
        };

        let archived_conversation = if already_archived {
            source_conversation
        } else {
            let previous_status = source_conversation.status.clone();
            let now = now_iso();
            let (conversation, (), _) = state_update_conversation_metadata_cached(
                state,
                &source.id,
                |conversation| {
                    conversation.status = "archived".to_string();
                    conversation.summary.clear();
                    conversation.archived_at = Some(now.clone());
                    conversation.updated_at = now.clone();
                    Ok(())
                },
            )?;
            runtime_log_info(format!(
                "[归档] 完成，任务=即时标记归档，conversation_id={}，previous_status={}，reason={}，archived_at={}",
                conversation.id,
                previous_status,
                archive_reason,
                conversation.archived_at.as_deref().unwrap_or("")
            ));
            conversation
        };
        let app_config = state_read_config_cached(state)?;
        let unarchived_conversations =
            self.collect_unarchived_conversation_summaries_cached(state, &app_config)?;
        let overview_payload = UnarchivedConversationOverviewUpdatedPayload {
            preferred_conversation_id: Some(active_conversation_id.clone()),
            unarchived_conversations,
        };
        drop(guard);
        Ok(InstantArchiveConversationMutationResult {
            archived_conversation,
            active_conversation_id,
            overview_payload,
            already_archived,
        })
    }

    fn import_archives(
        &self,
        state: &AppState,
        incoming_archives: &mut Vec<ConversationArchive>,
    ) -> Result<ImportArchivesMutationResult, String> {
        let guard = state.conversation_lock.lock().map_err(|err| {
            format!(
                "Failed to lock state mutex at {}:{} {}: {err}",
                file!(),
                line!(),
                module_path!()
            )
        })?;
        let chat_index = state_read_chat_index_cached(state)?;
        let existing_archive_ids = chat_index
            .conversations
            .iter()
            .filter(|item| chat_index_item_is_archived(item))
            .map(|item| item.id.clone())
            .collect::<std::collections::HashSet<_>>();

        let mut imported_count = 0usize;
        let mut replaced_count = 0usize;
        let mut skipped_count = 0usize;
        let mut selected_archive_id: Option<String> = None;
        let mut seen_conversation_ids = std::collections::HashSet::<String>::new();

        for archive in incoming_archives.iter_mut() {
            normalize_archive_for_import(archive, &state.data_path);
        }

        for archive in incoming_archives.drain(..) {
            let archive_id = archive.archive_id.clone();
            let conversation = archive_to_conversation(archive);
            let conversation_id = conversation.id.clone();
            if !seen_conversation_ids.insert(conversation_id.clone()) {
                skipped_count += 1;
                continue;
            }
            if existing_archive_ids.contains(&conversation_id) {
                state_update_conversation_metadata_cached(state, &conversation_id, |cached| {
                    preserve_field_level_conversation_metadata(cached, &conversation);
                    Ok(())
                })?;
                state_schedule_conversation_persist(state, &conversation)?;
                replaced_count += 1;
            } else {
                state_schedule_conversation_persist(state, &conversation)?;
                imported_count += 1;
            }
            if selected_archive_id.is_none() {
                selected_archive_id = Some(archive_id);
            }
        }
        drop(guard);
        let total_count = state_read_chat_index_cached(state)?
            .conversations
            .iter()
            .filter(|item| chat_index_item_is_archived(item))
            .count();

        Ok(ImportArchivesMutationResult {
            imported_count,
            replaced_count,
            skipped_count,
            total_count,
            selected_archive_id,
        })
    }

    fn persist_compaction_message(
        &self,
        state: &AppState,
        source: &Conversation,
        compression_message: &ChatMessage,
        user_profile_snapshot: Option<String>,
    ) -> Result<CompactionMessagePersistResult, String> {
        let guard = state.conversation_lock.lock().map_err(|err| {
            format!(
                "Failed to lock state mutex at {}:{} {}: {err}",
                file!(),
                line!(),
                module_path!()
            )
        })?;
        let mut conversation = state_read_conversation_cached(state, &source.id)
            .map_err(|_| "活动对话已变化，请重试上下文整理。".to_string())?;
        if conversation_is_archived(&conversation) {
            drop(guard);
            return Err("活动对话已变化，请重试上下文整理。".to_string());
        }
        let compression_message_id = compression_message.id.clone();
        conversation.messages.push(compression_message.clone());
        conversation.user_profile_snapshot = user_profile_snapshot.unwrap_or_default();
        let now = now_iso();
        conversation.updated_at = now.clone();
        conversation.last_user_at = Some(now);
        state_update_conversation_metadata_cached(
            state,
            &conversation.id,
            |cached| {
                cached.user_profile_snapshot = conversation.user_profile_snapshot.clone();
                cached.updated_at = conversation.updated_at.clone();
                cached.last_user_at = conversation.last_user_at.clone();
                Ok(())
            },
        )?;
        let active_conversation_id = Some(conversation.id.clone());
        state_schedule_conversation_persist(state, &conversation)?;

        drop(guard);

        {
            let verify_guard = state.conversation_lock.lock().map_err(|err| {
                format!(
                    "Failed to lock state mutex at {}:{} {}: {err}",
                    file!(),
                    line!(),
                    module_path!()
                )
            })?;
            let persisted = state_read_conversation_cached(state, &source.id)
                .ok()
                .filter(|conversation| !conversation_is_archived(conversation))
                .map(|conversation| {
                    conversation
                        .messages
                        .iter()
                        .any(|message| message.id == compression_message_id)
                })
                .unwrap_or(false);
            drop(verify_guard);
            if !persisted {
                return Err(
                    "上下文整理消息写入校验失败：已执行整理但未找到落盘消息，请重试。".to_string(),
                );
            }
        }

        Ok(CompactionMessagePersistResult {
            active_conversation_id,
            compression_message_id,
        })
    }
}

fn build_archive_replacement_conversation(
    state: &AppState,
    agents: &[AgentProfile],
    assistant_department_agent_id: &str,
    selected_api: &ApiConfig,
    _source: &Conversation,
) -> Result<Conversation, String> {
    let mut conversation = build_conversation_record(
        &selected_api.id,
        "",
        ASSISTANT_DEPARTMENT_ID,
        "",
        CONVERSATION_KIND_CHAT,
        None,
        None,
    );
    let profile_snapshot = agents
        .iter()
        .find(|item| item.id == assistant_department_agent_id)
        .and_then(|agent| match build_user_profile_snapshot_block(&state.data_path, agent, 12) {
            Ok(snapshot) => snapshot,
            Err(err) => {
                runtime_log_error(format!(
                    "[用户画像] 失败，任务=prepare_archive_active_conversation_seed_snapshot，agent_id={}，error={}",
                    agent.id,
                    err
                ));
                None
            }
        });
    if let Some(snapshot) = profile_snapshot {
        conversation.user_profile_snapshot = snapshot;
    }
    let summary_message = build_initial_summary_context_message(
        option_str_or_none(conversation.user_profile_snapshot.as_str()),
        Some(&conversation.current_todos),
        None,
    );
    conversation.last_user_at = Some(summary_message.created_at.clone());
    conversation.updated_at = summary_message.created_at.clone();
    conversation.messages.push(summary_message);
    Ok(conversation)
}

fn option_str_or_none(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

impl ConversationService {
    fn collect_unarchived_conversation_summaries_cached(
        &self,
        state: &AppState,
        app_config: &AppConfig,
    ) -> Result<Vec<UnarchivedConversationSummary>, String> {
        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let runtime_app_config = if runtime_snapshot.config.departments.is_empty() {
            app_config.clone()
        } else {
            runtime_snapshot.config
        };
        let runtime = state_read_runtime_state_cached(state)?;
        let main_conversation_id = runtime
            .main_conversation_id
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_string();
        let chat_index = state_read_chat_index_cached(state)?;
        let visible_conversations = chat_index
            .conversations
            .iter()
            .filter(|item| !chat_index_item_is_archived(item))
            .filter_map(|item| {
                let conversation = match state_read_conversation_cached(state, item.id.as_str()) {
                    Ok(conversation) => conversation,
                    Err(err) => {
                        eprintln!(
                            "[会话索引读取] 状态=失败，任务=collect_unarchived_conversation_summaries_cached，conversation_id={}，error={}",
                            item.id, err
                        );
                        return None;
                    }
                };
                conversation_is_unarchived_foreground(&conversation).then_some(conversation)
            })
            .collect::<Vec<_>>();
        let visible_ids = visible_conversations
            .iter()
            .map(|conversation| conversation.id.trim().to_string())
            .filter(|conversation_id| !conversation_id.is_empty())
            .collect::<std::collections::HashSet<_>>();
        let mut seen_pins = std::collections::HashSet::<String>::new();
        let pinned_conversation_ids = runtime
            .pinned_conversation_ids
            .iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .filter(|item| *item != main_conversation_id)
            .filter(|item| visible_ids.contains(item))
            .filter(|item| seen_pins.insert(item.clone()))
            .collect::<Vec<_>>();
        let summaries = visible_conversations
            .iter()
            .map(|conversation| {
                build_unarchived_conversation_summary(
                    state,
                    &runtime_app_config,
                    &main_conversation_id,
                    &pinned_conversation_ids,
                    conversation,
                    Some(DESKTOP_CHAT_VIEWER_ID),
                )
            })
            .collect::<Vec<_>>();
        Ok(sort_unarchived_conversation_summaries(summaries))
    }

    fn resolve_session_conversation_id_fast(
        &self,
        state: &AppState,
        session: &SessionSelector,
    ) -> Result<Option<String>, String> {
        if let Some(conversation_id) = session
            .conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Ok(Some(conversation_id.to_string()));
        }
        if !session.agent_id.trim().is_empty() {
            return Ok(None);
        }
        let runtime = state_read_runtime_state_cached(state)?;
        let Some(main_conversation_id) = runtime
            .main_conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return Ok(None);
        };
        Ok(self
            .try_read_unarchived_conversation(state, main_conversation_id)?
            .filter(|conversation| conversation_visible_in_foreground_lists(conversation))
            .map(|conversation| conversation.id))
    }

    fn resolve_effective_agent_id_for_read(
        &self,
        state: &AppState,
        app_config: &mut AppConfig,
        runtime_agents: &[AgentProfile],
        assistant_department_agent_id: &str,
        requested_agent_id: &str,
    ) -> Result<String, String> {
        let runtime_snapshot =
            build_runtime_organization_snapshot_from_parts(&state.data_path, app_config, runtime_agents)?;
        *app_config = runtime_snapshot.config.clone();
        let runtime_agents = runtime_snapshot.agents;
        let requested_agent_id = requested_agent_id.trim();
        if !requested_agent_id.is_empty() {
            if runtime_agents
                .iter()
                .any(|agent| agent.id == requested_agent_id && !agent.is_built_in_user)
            {
                return Ok(requested_agent_id.to_string());
            }
            return Err(format!("Selected agent '{requested_agent_id}' not found."));
        }
        if runtime_agents
            .iter()
            .any(|agent| {
                agent.id == assistant_department_agent_id && !agent.is_built_in_user
            })
        {
            return Ok(assistant_department_agent_id.to_string());
        }
        runtime_agents
            .iter()
            .find(|agent| !agent.is_built_in_user)
            .map(|agent| agent.id.clone())
            .ok_or_else(|| "Selected agent not found.".to_string())
    }

    fn read_foreground_snapshot(
        &self,
        state: &AppState,
        conversation_id: Option<&str>,
        agent_id: Option<&str>,
        recent_limit: usize,
    ) -> Result<ForegroundConversationSnapshotCore, String> {
        let direct_conversation_id = conversation_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);

        let mut snapshot = if let Some(conversation_id) = direct_conversation_id {
            self.with_chat_view_conversation_by_id_fast(state, &conversation_id, |conversation| {
                ensure_unarchived_conversation_not_organizing(state, &conversation.id)?;
                build_foreground_conversation_snapshot_from_conversation(
                    state,
                    conversation,
                    recent_limit,
                )
            })?
        } else if let Some(main_conversation_id) = state_read_runtime_state_cached(state)?
            .main_conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
        {
            self.with_unarchived_conversation_by_id_fast(state, &main_conversation_id, |conversation| {
                ensure_unarchived_conversation_not_organizing(state, &conversation.id)?;
                build_foreground_conversation_snapshot_from_conversation(
                    state,
                    conversation,
                    recent_limit,
                )
            })?
        } else {
            let mut app_config = state_read_config_cached(state)?;
            let runtime = state_read_runtime_state_cached(state)?;
            let agents = state_read_agents_cached(state)?;
            let effective_agent_id = self.resolve_effective_agent_id_for_read(
                state,
                &mut app_config,
                &agents,
                &runtime.assistant_department_agent_id,
                agent_id.unwrap_or_default(),
            )?;
            if let Some(target_conversation_id) =
                self.resolve_latest_foreground_conversation_id(state, &effective_agent_id)?
            {
                ensure_unarchived_conversation_not_organizing(state, &target_conversation_id)?;
                self.with_unarchived_conversation_by_id_fast(
                    state,
                    &target_conversation_id,
                    |conversation| {
                        build_foreground_conversation_snapshot_from_conversation(
                            state,
                            conversation,
                            recent_limit,
                        )
                    },
                )?
            } else {
                ForegroundConversationSnapshotCore {
                    conversation_id: String::new(),
                    messages: Vec::new(),
                    has_more_history: false,
                    runtime_state: None,
                    current_todo: None,
                    current_todos: Vec::new(),
                    preferred_api_config_id: None,
                    active_goal: None,
                }
            }
        };

        materialize_chat_message_parts_from_media_refs(&mut snapshot.messages, &state.data_path);
        Ok(snapshot)
    }

    fn read_chat_snapshot(
        &self,
        state: &AppState,
        input: &SessionSelector,
    ) -> Result<ChatSnapshot, String> {
        let requested_conversation_id = input
            .conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        if let Some(conversation_id) = requested_conversation_id
            .clone()
            .or_else(|| {
                state_read_runtime_state_cached(state)
                    .ok()
                    .and_then(|runtime| runtime.main_conversation_id)
                    .and_then(|value| {
                        let trimmed = value.trim();
                        (!trimmed.is_empty()).then(|| trimmed.to_string())
                    })
            })
        {
            let store_paths = message_store::message_store_paths(&state.data_path, &conversation_id)?;
            let snapshot = if let Some(snapshot) =
                message_store::read_ready_message_store_chat_snapshot(&store_paths)?
            {
                let mut latest_user = snapshot.latest_user;
                let mut latest_assistant = snapshot.latest_assistant;
                if let Some(message) = latest_user.as_mut() {
                    materialize_message_parts_from_media_refs(&mut message.parts, &state.data_path);
                }
                if let Some(message) = latest_assistant.as_mut() {
                    materialize_message_parts_from_media_refs(&mut message.parts, &state.data_path);
                }
                Some(ChatSnapshot {
                    conversation_id: conversation_id.clone(),
                    latest_user,
                    latest_assistant,
                    active_message_count: snapshot.active_message_count,
                })
            } else {
                self.try_read_unarchived_conversation(state, &conversation_id)?
                    .filter(|conversation| conversation_visible_in_foreground_lists(conversation))
                    .map(|conversation| {
                        let mut latest_user = conversation
                            .messages
                            .iter()
                            .rev()
                            .find(|message| message.role == "user")
                            .cloned();
                        let mut latest_assistant = conversation
                            .messages
                            .iter()
                            .rev()
                            .find(|message| message.role == "assistant")
                            .cloned();
                        if let Some(message) = latest_user.as_mut() {
                            materialize_message_parts_from_media_refs(
                                &mut message.parts,
                                &state.data_path,
                            );
                        }
                        if let Some(message) = latest_assistant.as_mut() {
                            materialize_message_parts_from_media_refs(
                                &mut message.parts,
                                &state.data_path,
                            );
                        }
                        ChatSnapshot {
                            conversation_id: conversation.id.clone(),
                            latest_user,
                            latest_assistant,
                            active_message_count: conversation.messages.len(),
                        }
                    })
            };
            if let Some(snapshot) = snapshot {
                return Ok(snapshot);
            }
        }

        let guard = state.conversation_lock.lock().map_err(|err| {
            format!(
                "Failed to lock state mutex at {}:{} {}: {err}",
                file!(),
                line!(),
                module_path!()
            )
        })?;

        let mut app_config = state_read_config_cached(state)?;
        let runtime = state_read_runtime_state_cached(state)?;
        let agents = state_read_agents_cached(state)?;
        let effective_agent_id = self.resolve_effective_agent_id_for_read(
            state,
            &mut app_config,
            &agents,
            &runtime.assistant_department_agent_id,
            &input.agent_id,
        )?;
        if let Some(conversation_id) =
            self.resolve_latest_foreground_conversation_id(state, &effective_agent_id)?
        {
            let store_paths = message_store::message_store_paths(&state.data_path, &conversation_id)?;
            if let Some(snapshot) = message_store::read_ready_message_store_chat_snapshot(&store_paths)? {
                let mut latest_user = snapshot.latest_user;
                let mut latest_assistant = snapshot.latest_assistant;
                if let Some(message) = latest_user.as_mut() {
                    materialize_message_parts_from_media_refs(&mut message.parts, &state.data_path);
                }
                if let Some(message) = latest_assistant.as_mut() {
                    materialize_message_parts_from_media_refs(&mut message.parts, &state.data_path);
                }
                return Ok(ChatSnapshot {
                    conversation_id,
                    latest_user,
                    latest_assistant,
                    active_message_count: snapshot.active_message_count,
                });
            }
        }
        drop(guard);
        Ok(ChatSnapshot {
            conversation_id: String::new(),
            latest_user: None,
            latest_assistant: None,
            active_message_count: 0,
        })
    }

    fn refresh_unarchived_conversation_overview_payload(
        &self,
        state: &AppState,
    ) -> Result<UnarchivedConversationOverviewUpdatedPayload, String> {
        let guard = state.conversation_lock.lock().map_err(|err| {
            format!(
                "Failed to lock state mutex at {}:{} {}: {err}",
                file!(),
                line!(),
                module_path!()
            )
        })?;
        let app_config = state_read_config_cached(state)?;
        let unarchived_conversations =
            self.collect_unarchived_conversation_summaries_cached(state, &app_config)?;
        drop(guard);
        Ok(UnarchivedConversationOverviewUpdatedPayload {
            preferred_conversation_id: unarchived_conversations
                .first()
                .map(|item| item.conversation_id.clone()),
            unarchived_conversations,
        })
    }

    fn list_unarchived_conversation_summaries(
        &self,
        state: &AppState,
    ) -> Result<ListUnarchivedConversationsMutationResult, String> {
        let guard = state.conversation_lock.lock().map_err(|err| {
            format!(
                "Failed to lock state mutex at {}:{} {}: {err}",
                file!(),
                line!(),
                module_path!()
            )
        })?;
        let app_config = state_read_config_cached(state)?;
        let summaries = self.collect_unarchived_conversation_summaries_cached(state, &app_config)?;
        drop(guard);
        Ok(ListUnarchivedConversationsMutationResult { summaries })
    }

    fn read_active_conversation_messages(
        &self,
        state: &AppState,
        session: &SessionSelector,
    ) -> Result<Vec<ChatMessage>, String> {
        if let Some(conversation_id) = session
            .conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                state_read_runtime_state_cached(state)
                    .ok()
                    .and_then(|runtime| runtime.main_conversation_id)
                    .and_then(|value| {
                        let trimmed = value.trim();
                        (!trimmed.is_empty()).then(|| trimmed.to_string())
                    })
            })
        {
            let store_paths = message_store::message_store_paths(&state.data_path, &conversation_id)?;
            let mut messages = if let Some(page) =
                message_store::read_ready_message_store_recent_blocks_page_cached(&store_paths, 2)?
            {
                let _ = self.retain_message_store_block_cache_whitelist(state);
                page.messages
            } else {
                self.with_unarchived_conversation_by_id_fast(state, &conversation_id, |conversation| {
                    Ok(conversation.messages.clone())
                })?
            };
            materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
            return Ok(messages);
        }

        let mut app_config = state_read_config_cached(state)?;
        let runtime = state_read_runtime_state_cached(state)?;
        let agents = state_read_agents_cached(state)?;
        let effective_agent_id = self.resolve_effective_agent_id_for_read(
            state,
            &mut app_config,
            &agents,
            &runtime.assistant_department_agent_id,
            &session.agent_id,
        )?;
        if let Some(conversation_id) =
            self.resolve_latest_foreground_conversation_id(state, &effective_agent_id)?
        {
            return self.read_unarchived_messages(state, &conversation_id);
        }
        Ok(Vec::new())
    }

    fn read_unarchived_messages(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Vec<ChatMessage>, String> {
        let mut messages =
            self.with_unarchived_conversation_by_id(state, conversation_id, |conversation| {
                Ok(conversation.messages.clone())
            })?;
        materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
        Ok(messages)
    }

    fn read_recent_unarchived_block_messages(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Vec<ChatMessage>, String> {
        let store_paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
        let mut messages = if let Some(page) = message_store::read_ready_message_store_recent_messages_page_cached(
            &store_paths,
            DEFAULT_FOREGROUND_SNAPSHOT_RECENT_LIMIT,
        )?
        {
            let _ = self.retain_message_store_block_cache_whitelist(state);
            page.messages
        } else {
            self.with_unarchived_conversation_by_id(state, conversation_id, |conversation| {
                let total = conversation.messages.len();
                let start = total.saturating_sub(DEFAULT_FOREGROUND_SNAPSHOT_RECENT_LIMIT);
                Ok(conversation.messages[start..].to_vec())
            })?
        };
        materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
        Ok(messages)
    }

    fn read_unarchived_block_page(
        &self,
        state: &AppState,
        conversation_id: &str,
        requested_block_id: Option<u32>,
    ) -> Result<ConversationBlockPageResult, String> {
        self.with_unarchived_conversation_by_id_fast(state, conversation_id, |conversation| {
            let store_paths = message_store::message_store_paths(&state.data_path, &conversation.id)?;
            if let Some(page) =
                message_store::read_ready_message_store_block_page(&store_paths, requested_block_id)?
            {
                let _ = self.retain_message_store_block_cache_whitelist(state);
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

            let mut messages = conversation.messages.clone();
            materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
            Ok(ConversationBlockPageResult {
                blocks: vec![ConversationBlockSummaryResult {
                    block_id: 0,
                    message_count: messages.len(),
                    first_message_id: messages
                        .first()
                        .map(|message| message.id.clone())
                        .unwrap_or_default(),
                    last_message_id: messages
                        .last()
                        .map(|message| message.id.clone())
                        .unwrap_or_default(),
                    first_created_at: messages.first().map(|message| message.created_at.clone()),
                    last_created_at: messages.last().map(|message| message.created_at.clone()),
                    is_latest: true,
                }],
                selected_block_id: 0,
                messages,
                has_prev_block: false,
                has_next_block: false,
            })
        })
    }

    fn read_recent_unarchived_messages(
        &self,
        state: &AppState,
        conversation_id: &str,
        limit: usize,
    ) -> Result<Vec<ChatMessage>, String> {
        let normalized_limit = limit.clamp(1, 50);
        let store_paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
        let mut messages = if let Some(page) =
            message_store::read_ready_message_store_recent_messages_page_cached(
                &store_paths,
                normalized_limit,
            )?
        {
            let _ = self.retain_message_store_block_cache_whitelist(state);
            page.messages
        } else {
            self.with_unarchived_conversation_by_id(state, conversation_id, |conversation| {
                let total = conversation.messages.len();
                let start = total.saturating_sub(normalized_limit);
                Ok(conversation.messages[start..].to_vec())
            })?
        };
        materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
        Ok(messages)
    }

    fn read_message_by_id(
        &self,
        state: &AppState,
        conversation_id: &str,
        message_id: &str,
    ) -> Result<ChatMessage, String> {
        let normalized_message_id = message_id.trim();
        if normalized_message_id.is_empty() {
            return Err("messageId is required.".to_string());
        }
        let store_paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
        let mut message = if let Some(message) =
            message_store::read_ready_message_store_message_by_id(&store_paths, normalized_message_id)?
        {
            message
        } else {
            self.with_unarchived_conversation_by_id(state, conversation_id, |conversation| {
                conversation
                    .messages
                    .iter()
                    .find(|item| item.id.trim() == normalized_message_id)
                    .cloned()
                    .ok_or_else(|| format!("Message not found: {normalized_message_id}"))
            })?
        };
        materialize_chat_message_parts_from_media_refs(
            std::slice::from_mut(&mut message),
            &state.data_path,
        );
        Ok(message)
    }

    fn read_messages_before(
        &self,
        state: &AppState,
        session: &SessionSelector,
        before_message_id: &str,
        limit: usize,
    ) -> Result<(Vec<ChatMessage>, bool), String> {
        let normalized_before_message_id = before_message_id.trim();
        if normalized_before_message_id.is_empty() {
            return Err("beforeMessageId is required.".to_string());
        }
        let normalized_limit = limit.clamp(1, 100);
        let direct_conversation_id = self.resolve_session_conversation_id_fast(state, session)?;

        let (mut page, has_more) = if let Some(conversation_id) = direct_conversation_id {
            let store_paths = message_store::message_store_paths(&state.data_path, &conversation_id)?;
            if let Some(page) = message_store::read_ready_message_store_messages_before(
                &store_paths,
                normalized_before_message_id,
                normalized_limit,
            )? {
                (page.messages, page.has_more)
            } else {
                self.with_unarchived_conversation_by_id_fast(state, &conversation_id, |conversation| {
                    clone_messages_before_page(
                        &conversation.messages,
                        normalized_before_message_id,
                        normalized_limit,
                    )
                })?
            }
        } else {
            let mut app_config = state_read_config_cached(state)?;
            let runtime = state_read_runtime_state_cached(state)?;
            let agents = state_read_agents_cached(state)?;
            let effective_agent_id = self.resolve_effective_agent_id_for_read(
                state,
                &mut app_config,
                &agents,
                &runtime.assistant_department_agent_id,
                &session.agent_id,
            )?;
            if let Some(conversation_id) =
                self.resolve_latest_foreground_conversation_id(state, &effective_agent_id)?
            {
                let store_paths =
                    message_store::message_store_paths(&state.data_path, &conversation_id)?;
                if let Some(page) = message_store::read_ready_message_store_messages_before(
                    &store_paths,
                    normalized_before_message_id,
                    normalized_limit,
                )? {
                    (page.messages, page.has_more)
                } else {
                    self.with_unarchived_conversation_by_id_fast(
                        state,
                        &conversation_id,
                        |conversation| {
                            clone_messages_before_page(
                                &conversation.messages,
                                normalized_before_message_id,
                                normalized_limit,
                            )
                        },
                    )?
                }
            } else {
                return Err("当前前台会话不存在，无法加载更早消息。".to_string());
            }
        };

        materialize_chat_message_parts_from_media_refs(&mut page, &state.data_path);
        Ok((page, has_more))
    }

    fn read_messages_after(
        &self,
        state: &AppState,
        session: &SessionSelector,
        after_message_id: &str,
        limit: usize,
    ) -> Result<Vec<ChatMessage>, String> {
        let normalized_after_message_id = after_message_id.trim();
        if normalized_after_message_id.is_empty() {
            return Err("afterMessageId is required.".to_string());
        }
        let normalized_limit = limit.clamp(1, 100);
        let direct_conversation_id = self.resolve_session_conversation_id_fast(state, session)?;

        let mut page = if let Some(conversation_id) = direct_conversation_id {
            let store_paths = message_store::message_store_paths(&state.data_path, &conversation_id)?;
            if let Some(page) = message_store::read_ready_message_store_messages_after(
                &store_paths,
                normalized_after_message_id,
                normalized_limit,
            )? {
                page.messages
            } else {
                self.with_unarchived_conversation_by_id_fast(state, &conversation_id, |conversation| {
                    clone_messages_after_page(
                        &conversation.messages,
                        normalized_after_message_id,
                        normalized_limit,
                    )
                })?
            }
        } else {
            let mut app_config = state_read_config_cached(state)?;
            let runtime = state_read_runtime_state_cached(state)?;
            let agents = state_read_agents_cached(state)?;
            let effective_agent_id = self.resolve_effective_agent_id_for_read(
                state,
                &mut app_config,
                &agents,
                &runtime.assistant_department_agent_id,
                &session.agent_id,
            )?;
            if let Some(conversation_id) =
                self.resolve_latest_foreground_conversation_id(state, &effective_agent_id)?
            {
                let store_paths =
                    message_store::message_store_paths(&state.data_path, &conversation_id)?;
                if let Some(page) = message_store::read_ready_message_store_messages_after(
                    &store_paths,
                    normalized_after_message_id,
                    normalized_limit,
                )? {
                    page.messages
                } else {
                    self.with_unarchived_conversation_by_id_fast(
                        state,
                        &conversation_id,
                        |conversation| {
                            clone_messages_after_page(
                                &conversation.messages,
                                normalized_after_message_id,
                                normalized_limit,
                            )
                        },
                    )?
                }
            } else {
                return Err("当前前台会话不存在，无法加载后续消息。".to_string());
            }
        };

        materialize_chat_message_parts_from_media_refs(&mut page, &state.data_path);
        Ok(page)
    }

    fn read_messages_after_with_fallback(
        &self,
        state: &AppState,
        conversation_id: &str,
        after_message_id: Option<&str>,
        fallback_limit: usize,
    ) -> Result<(Vec<ChatMessage>, Option<String>), String> {
        let trimmed_after = after_message_id
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let store_paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
        let (mut page, fallback_mode) = if let Some(after_id) = trimmed_after {
            if let Some(after_page) =
                message_store::read_ready_message_store_messages_after(&store_paths, after_id, 100)?
            {
                (after_page.messages, None)
            } else {
                self.with_unarchived_conversation_by_id_fast(state, conversation_id, |conversation| {
                    let messages = &conversation.messages;
                    let page_result = if let Some(after_idx) = messages.iter().position(|item| item.id == after_id) {
                        (messages[(after_idx + 1)..].to_vec(), None)
                    } else {
                        let start = messages.len().saturating_sub(fallback_limit);
                        (messages[start..].to_vec(), Some("recent_limit".to_string()))
                    };
                    Ok(page_result)
                })?
            }
        } else if let Some(page) =
            message_store::read_ready_message_store_recent_messages_page_cached(
                &store_paths,
                fallback_limit,
            )?
        {
            let _ = self.retain_message_store_block_cache_whitelist(state);
            (
                page.messages,
                Some("recent_limit_in_latest_block".to_string()),
            )
        } else {
            self.with_unarchived_conversation_by_id_fast(state, conversation_id, |conversation| {
                let messages = &conversation.messages;
                let start = messages.len().saturating_sub(fallback_limit);
                Ok((messages[start..].to_vec(), Some("recent_limit".to_string())))
            })?
        };
        materialize_chat_message_parts_from_media_refs(&mut page, &state.data_path);
        Ok((page, fallback_mode))
    }
}

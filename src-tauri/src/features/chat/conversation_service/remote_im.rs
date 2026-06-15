impl ConversationService {
    fn list_remote_im_contact_conversations(
        &self,
        state: &AppState,
    ) -> Result<Vec<RemoteImContactConversationSummary>, String> {
        let mut runtime = state_read_runtime_state_cached(state)?;
        let config = load_runtime_organization_snapshot(state)?.config;
        let mut resolved_pairs = Vec::<(RemoteImContact, String)>::new();
        let mut runtime_changed = false;
        for contact in runtime.remote_im_contacts.iter_mut() {
            let previous_bound_conversation_id = contact
                .bound_conversation_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned);
            let previous_bound_department_id = contact
                .bound_department_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned);
            let previous_bound_agent_id = contact
                .bound_agent_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned);
            let conversation_id = ensure_remote_im_contact_conversation_id(state, contact)?;
            let binding_changed = previous_bound_conversation_id.as_deref() != Some(conversation_id.as_str())
                || previous_bound_department_id.as_deref()
                    != contact.bound_department_id.as_deref().map(str::trim)
                || previous_bound_agent_id.as_deref()
                    != contact.bound_agent_id.as_deref().map(str::trim);
            if binding_changed {
                runtime_changed = true;
            }
            resolved_pairs.push((contact.clone(), conversation_id));
        }
        if runtime_changed {
            state_write_runtime_state_cached(state, &runtime)?;
        }
        let mut items = Vec::<RemoteImContactConversationSummary>::new();
        for (contact, conversation_id) in resolved_pairs {
            let store_paths = message_store::message_store_paths(&state.data_path, &conversation_id)?;
            let summary = if let Some(meta) = message_store::read_ready_message_store_meta(&store_paths)? {
                let manifest_status = message_store::read_message_store_manifest_status(&store_paths)?
                    .ok_or_else(|| format!("联系人会话缺少消息存储 manifest：{conversation_id}"))?;
                let preview_messages = self
                    .read_remote_im_contact_preview_messages(state, &conversation_id, 2)
                    .unwrap_or_default();
                Some(RemoteImContactConversationSummary {
                    contact_id: contact.id.clone(),
                    conversation_id: conversation_id.clone(),
                    title: remote_im_contact_conversation_title(&contact),
                    updated_at: meta.updated_at().to_string(),
                    last_message_at: meta
                        .last_assistant_at()
                        .map(ToOwned::to_owned)
                        .or_else(|| meta.last_user_at().map(ToOwned::to_owned))
                        .or_else(|| Some(meta.updated_at().to_string())),
                    message_count: manifest_status.source_message_count,
                    channel_id: contact.channel_id.clone(),
                    channel_name: remote_im_channel_by_id(&config, &contact.channel_id)
                        .map(|channel| channel.name.trim().to_string())
                        .filter(|value| !value.is_empty()),
                    platform: contact.platform.clone(),
                    contact_display_name: remote_im_contact_display_name(&contact),
                    bound_department_id: contact.bound_department_id.clone(),
                    bound_agent_id: contact.bound_agent_id.clone(),
                    processing_mode: normalize_contact_processing_mode(&contact.processing_mode),
                    preview_messages,
                })
            } else {
                let conversation = match self.try_read_unarchived_conversation(state, &conversation_id)? {
                    Some(conversation) if conversation_is_remote_im_contact(&conversation) => conversation,
                    _ => continue,
                };
                Some(RemoteImContactConversationSummary {
                    contact_id: contact.id.clone(),
                    conversation_id: conversation.id.clone(),
                    title: remote_im_contact_conversation_title(&contact),
                    updated_at: conversation.updated_at.clone(),
                    last_message_at: conversation
                        .messages
                        .last()
                        .map(|message| message.created_at.clone()),
                    message_count: conversation.messages.len(),
                    channel_id: contact.channel_id.clone(),
                    channel_name: remote_im_channel_by_id(&config, &contact.channel_id)
                        .map(|channel| channel.name.trim().to_string())
                        .filter(|value| !value.is_empty()),
                    platform: contact.platform.clone(),
                    contact_display_name: remote_im_contact_display_name(&contact),
                    bound_department_id: contact.bound_department_id.clone(),
                    bound_agent_id: contact.bound_agent_id.clone(),
                    processing_mode: normalize_contact_processing_mode(&contact.processing_mode),
                    preview_messages: build_conversation_preview_messages(&conversation, 2),
                })
            };
            if let Some(item) = summary {
                items.push(item);
            }
        }
        items.sort_by(|a, b| {
            let bk = b
                .last_message_at
                .as_deref()
                .unwrap_or(b.updated_at.as_str());
            let ak = a
                .last_message_at
                .as_deref()
                .unwrap_or(a.updated_at.as_str());
            bk.cmp(ak).then_with(|| b.updated_at.cmp(&a.updated_at))
        });
        Ok(items)
    }

    fn read_remote_im_contact_preview_messages(
        &self,
        state: &AppState,
        conversation_id: &str,
        limit: usize,
    ) -> Result<Vec<ConversationPreviewMessage>, String> {
        let normalized_limit = limit.max(1);
        let store_paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
        if let Some(page) = message_store::read_ready_message_store_recent_messages_page_cached(
            &store_paths,
            normalized_limit,
        )? {
            let mut messages = page.messages;
            materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
            return Ok(build_preview_messages_from_chat_messages(&messages, normalized_limit));
        }
        self.with_unarchived_conversation_by_id_fast(state, conversation_id, |conversation| {
            Ok(build_conversation_preview_messages(conversation, normalized_limit))
        })
    }

    fn read_remote_im_contact_conversation_messages(
        &self,
        state: &AppState,
        contact_id: &str,
    ) -> Result<Vec<ChatMessage>, String> {
        let normalized_contact_id = contact_id.trim();
        if normalized_contact_id.is_empty() {
            return Err("contact_id 为必填项。".to_string());
        }
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| state_lock_error_with_panic(file!(), line!(), module_path!(), &err))?;
        let runtime = state_read_runtime_state_cached(state)?;
        let runtime_contact = runtime
            .remote_im_contacts
            .iter()
            .find(|item| item.id == normalized_contact_id)
            .cloned()
            .ok_or_else(|| format!("未找到远程联系人：{normalized_contact_id}"))?;
        let conversation_id = if let Some(conversation_id) = runtime_contact
            .bound_conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            conversation_id.to_string()
        } else {
            let target_key = remote_im_contact_conversation_key(&runtime_contact);
            let chat_index = state_read_chat_index_cached(state)?;
            chat_index
                .conversations
                .iter()
                .filter_map(|item| state_read_conversation_cached(state, item.id.as_str()).ok())
                .find(|conversation| {
                    conversation.summary.trim().is_empty()
                        && conversation_is_remote_im_contact(conversation)
                        && conversation.root_conversation_id.as_deref().map(str::trim)
                            == Some(target_key.as_str())
                })
                .map(|conversation| conversation.id)
                .ok_or_else(|| format!("联系人未绑定联系人会话：{normalized_contact_id}"))?
        };
        drop(guard);
        let store_paths = message_store::message_store_paths(&state.data_path, &conversation_id)?;
        let mut messages = if let Some(page) =
            message_store::read_ready_message_store_recent_messages_page_cached(
                &store_paths,
                DEFAULT_FOREGROUND_SNAPSHOT_RECENT_LIMIT,
            )?
        {
                let _ = self.retain_message_store_block_cache_whitelist(state);
                page.messages
            } else {
                self.with_unarchived_conversation_by_id_fast(
                    state,
                    &conversation_id,
                    |conversation| {
                        let total = conversation.messages.len();
                        let start = total.saturating_sub(DEFAULT_FOREGROUND_SNAPSHOT_RECENT_LIMIT);
                        Ok(conversation.messages[start..].to_vec())
                    },
                )?
            };
        materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
        Ok(frontend_project_messages(messages))
    }

    fn read_remote_im_contact_conversation_block_page(
        &self,
        state: &AppState,
        contact_id: &str,
        requested_block_id: Option<u32>,
    ) -> Result<ConversationBlockPageResult, String> {
        let normalized_contact_id = contact_id.trim();
        if normalized_contact_id.is_empty() {
            return Err("contact_id 为必填项。".to_string());
        }
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| state_lock_error_with_panic(file!(), line!(), module_path!(), &err))?;
        let runtime = state_read_runtime_state_cached(state)?;
        let runtime_contact = runtime
            .remote_im_contacts
            .iter()
            .find(|item| item.id == normalized_contact_id)
            .cloned()
            .ok_or_else(|| format!("未找到远程联系人：{normalized_contact_id}"))?;
        let conversation_id = if let Some(conversation_id) = runtime_contact
            .bound_conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            conversation_id.to_string()
        } else {
            let target_key = remote_im_contact_conversation_key(&runtime_contact);
            let chat_index = state_read_chat_index_cached(state)?;
            chat_index
                .conversations
                .iter()
                .filter_map(|item| state_read_conversation_cached(state, item.id.as_str()).ok())
                .find(|conversation| {
                    conversation.summary.trim().is_empty()
                        && conversation_is_remote_im_contact(conversation)
                        && conversation.root_conversation_id.as_deref().map(str::trim)
                            == Some(target_key.as_str())
                })
                .map(|conversation| conversation.id)
                .ok_or_else(|| format!("联系人未绑定联系人会话：{normalized_contact_id}"))?
        };
        let conversation = state_read_conversation_cached(state, &conversation_id)?;
        if !conversation.summary.trim().is_empty() || !conversation_is_remote_im_contact(&conversation) {
            drop(guard);
            return Err(format!("联系人未绑定联系人会话：{normalized_contact_id}"));
        }
        drop(guard);

        let store_paths = message_store::message_store_paths(&state.data_path, &conversation_id)?;
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
                    messages: frontend_project_messages(messages),
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
                messages: frontend_project_messages(messages),
                has_prev_block: false,
                has_next_block: false,
            })
    }

    fn clear_remote_im_contact_conversation(
        &self,
        state: &AppState,
        contact_id: &str,
    ) -> Result<bool, String> {
        let normalized_contact_id = contact_id.trim();
        if normalized_contact_id.is_empty() {
            return Err("contact_id 为必填项。".to_string());
        }
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| state_lock_error_with_panic(file!(), line!(), module_path!(), &err))?;
        let mut runtime = state_read_runtime_state_cached(state)?;
        let Some(contact_index) = runtime
            .remote_im_contacts
            .iter()
            .position(|item| item.id == normalized_contact_id)
        else {
            drop(guard);
            return Err(format!("未找到远程联系人：{normalized_contact_id}"));
        };
        let contact = runtime.remote_im_contacts[contact_index].clone();
        let conversation_id = contact
            .bound_conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .or_else(|| {
                let target_key = remote_im_contact_conversation_key(&contact);
                match state_read_chat_index_cached(state) {
                    Ok(chat_index) => chat_index
                        .conversations
                        .iter()
                        .filter_map(|item| match state_read_conversation_cached(state, item.id.as_str()) {
                            Ok(conversation) => Some(conversation),
                            Err(err) => {
                                runtime_log_warn(format!(
                                    "[联系人会话] 警告，任务=clear_remote_im_contact_conversation_lookup，conversation_id={}，contact_id={}，error={}",
                                    item.id,
                                    normalized_contact_id,
                                    err
                                ));
                                None
                            }
                        })
                        .find(|conversation| {
                            conversation.summary.trim().is_empty()
                                && conversation_is_remote_im_contact(conversation)
                                && conversation.root_conversation_id.as_deref().map(str::trim)
                                    == Some(target_key.as_str())
                        })
                        .map(|conversation| conversation.id),
                    Err(err) => {
                        runtime_log_warn(format!(
                            "[联系人会话] 警告，任务=clear_remote_im_contact_read_chat_index，contact_id={}，error={}",
                            normalized_contact_id, err
                        ));
                        None
                    }
                }
            });
        let Some(conversation_id) = conversation_id else {
            drop(guard);
            return Ok(false);
        };
        let conversation = match state_read_conversation_cached(state, &conversation_id) {
            Ok(conversation)
                if conversation.summary.trim().is_empty()
                    && conversation_is_remote_im_contact(&conversation) =>
            {
                conversation
            }
            _ => {
                drop(guard);
                return Ok(false);
            }
        };

        runtime.remote_im_contacts[contact_index].bound_conversation_id = None;
        runtime
            .remote_im_contact_checkpoints
            .retain(|item| item.contact_id.trim() != normalized_contact_id);
        state_write_runtime_state_cached(state, &runtime)?;
        state_schedule_conversation_delete(state, &conversation.id)?;
        drop(guard);
        Ok(true)
    }
}

impl ConversationService {
    fn resolve_delegate_result_target_conversation(
        &self,
        state: &AppState,
        root_conversation_id: &str,
    ) -> Result<DelegateResultTargetConversationResolution, String> {
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| state_lock_error_with_panic(file!(), line!(), module_path!(), &err))?;
        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let assistant_agent_id = assistant_department_agent_id(&runtime_snapshot.config)
            .ok_or_else(|| "未找到助理部门委任人".to_string())?;
        let department_id = runtime_department_for_agent(&runtime_snapshot, &assistant_agent_id)
            .map(|item| item.id.clone())
            .unwrap_or_else(|| ASSISTANT_DEPARTMENT_ID.to_string());
        let normalized_root_conversation_id = root_conversation_id.trim();
        let target_conversation_id =
            if task_conversation_id_is_system_notification(normalized_root_conversation_id) {
                let conversation =
                    state_read_conversation_cached(state, SYSTEM_NOTIFICATION_CONVERSATION_ID)
                        .ok()
                        .filter(|conversation| {
                            conversation.summary.trim().is_empty()
                                && conversation_visible_in_foreground_lists(conversation)
                                && conversation_is_system_notification(conversation)
                        })
                        .unwrap_or_else(build_system_notification_conversation_record);
                let conversation_id = conversation.id.clone();
                state_schedule_conversation_persist(state, &conversation)?;
                conversation_id
            } else if state_read_conversation_cached(state, normalized_root_conversation_id)
                .ok()
                .filter(|conversation| {
                    conversation.summary.trim().is_empty()
                        && !conversation_is_delegate(conversation)
                        && !conversation_is_system_notification(conversation)
                })
                .is_some()
            {
                normalized_root_conversation_id.to_string()
            } else {
                return Err(format!(
                    "委托绑定会话不存在，无法写回结果，conversationId={normalized_root_conversation_id}"
                ));
            };
        drop(guard);
        Ok(DelegateResultTargetConversationResolution {
            department_id,
            agent_id: assistant_agent_id,
            target_conversation_id,
        })
    }

    fn resolve_delegate_context(
        &self,
        app_state: &AppState,
        source_agent_id: &str,
        source_department_id: Option<&str>,
        source_conversation_id: Option<&str>,
        target_department_id: &str,
        target_agent_id: Option<&str>,
    ) -> Result<DelegateContextResolution, String> {
        let guard = app_state
            .conversation_lock
            .lock()
            .map_err(|err| state_lock_error_with_panic(file!(), line!(), module_path!(), &err))?;
        let runtime_snapshot = load_runtime_organization_snapshot(app_state)?;
        let requested_source_conversation_id = source_conversation_id
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let thread_context = if let Some(conversation_id) = requested_source_conversation_id {
            delegate_runtime_thread_get(app_state, conversation_id)?
        } else {
            None
        };
        let source_conversation = if let Some(thread) = thread_context.as_ref() {
            Some(thread.conversation.clone())
        } else if let Some(conversation_id) = requested_source_conversation_id {
            Some(
                state_read_conversation_cached(app_state, conversation_id)
                    .ok()
                    .filter(|conversation| {
                        conversation.summary.trim().is_empty()
                            && !conversation_is_delegate(conversation)
                    })
                    .ok_or_else(|| {
                        format!("未找到指定来源会话，conversationId={conversation_id}")
                    })?,
            )
        } else {
            None
        };
        let requested_source_department_id = source_department_id
            .map(str::trim)
            .filter(|department_id| !department_id.is_empty());
        let source_department = if let Some(department_id) = requested_source_department_id {
            runtime_department_by_id(&runtime_snapshot, department_id)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "未找到发起部门，departmentId={}，agentId={}",
                        department_id, source_agent_id
                    )
                })?
        } else {
            source_conversation
                .as_ref()
                .and_then(|conversation| {
                    let department_id = conversation.department_id.trim();
                    if department_id.is_empty() {
                        None
                    } else {
                        runtime_department_by_id(&runtime_snapshot, department_id).cloned()
                    }
                })
                .ok_or_else(|| format!("未找到发起部门，agentId={source_agent_id}"))?
        };
        let target_department = runtime_department_by_id(&runtime_snapshot, target_department_id)
            .cloned()
            .ok_or_else(|| format!("目标部门不存在，departmentId={target_department_id}"))?;
        let target_agent_id = if let Some(requested_agent_id) = target_agent_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            if !target_department
                .agent_ids
                .iter()
                .any(|id| id.trim() == requested_agent_id)
            {
                drop(guard);
                return Err(format!(
                    "目标委任人不属于目标部门，departmentId={}，agentId={}",
                    target_department_id, requested_agent_id
                ));
            }
            if available_non_user_agent(&runtime_snapshot.agents, requested_agent_id).is_none() {
                drop(guard);
                return Err(format!("目标委任人不存在，agentId={requested_agent_id}"));
            }
            requested_agent_id.to_string()
        } else if let Some(agent) =
            first_available_department_agent(&target_department, &runtime_snapshot.agents)
        {
            agent.id.clone()
        } else {
            available_non_user_agent(&runtime_snapshot.agents, DEPUTY_AGENT_ID)
                .map(|agent| agent.id.clone())
                .ok_or_else(|| {
                    format!(
                        "目标部门没有可用委任人，且副手人格不可用，departmentId={target_department_id}"
                    )
                })?
        };
        let source_conversation_id = if let Some(thread) = thread_context.as_ref() {
            thread.root_conversation_id.clone()
        } else {
            source_conversation
                .as_ref()
                .map(|conversation| conversation.id.clone())
                .ok_or_else(|| "主代理缺少当前会话 ID，无法发起委托".to_string())?
        };
        drop(guard);
        Ok(DelegateContextResolution {
            config: runtime_snapshot.config,
            agents: runtime_snapshot.agents,
            source_department,
            target_department,
            target_agent_id,
            source_conversation_id,
            thread_context,
        })
    }


    fn resolve_prompt_prepare_conversation_from_data_read_only(
        &self,
        data: &AppData,
        data_path: &PathBuf,
        runtime_conversation_id: Option<&str>,
        runtime_conversation: &Conversation,
        selected_api: &ApiConfig,
        effective_agent_id: &str,
        requested_conversation_id: Option<&str>,
    ) -> Result<Option<PromptPrepareConversationResolution>, String> {
        let mut cloned = data.clone();
        self.resolve_prompt_prepare_conversation_core(
            &mut cloned,
            data_path,
            runtime_conversation_id,
            runtime_conversation,
            selected_api,
            effective_agent_id,
            requested_conversation_id,
            true,
        )
    }

    fn resolve_prompt_prepare_conversation_core(
        &self,
        data: &mut AppData,
        data_path: &PathBuf,
        runtime_conversation_id: Option<&str>,
        runtime_conversation: &Conversation,
        selected_api: &ApiConfig,
        effective_agent_id: &str,
        requested_conversation_id: Option<&str>,
        read_only: bool,
    ) -> Result<Option<PromptPrepareConversationResolution>, String> {
        let Some((idx, is_runtime_conversation)) = resolve_prompt_prepare_target(
            data,
            data_path,
            runtime_conversation_id,
            selected_api,
            effective_agent_id,
            requested_conversation_id,
            read_only,
        )? else {
            return Ok(None);
        };

        if idx.is_some() && !read_only {
            for conversation in &mut data.conversations {
                if conversation_is_delegate(conversation) || !conversation.summary.trim().is_empty()
                {
                    continue;
                }
                conversation.status = "active".to_string();
            }
        }

        let conversation_before = if let Some(actual_idx) = idx {
            data.conversations
                .get(actual_idx)
                .cloned()
                .ok_or_else(|| "前台会话索引无效".to_string())?
        } else {
            runtime_conversation.clone()
        };
        Ok(Some(build_prompt_prepare_resolution(
            data,
            &conversation_before,
            selected_api,
            is_runtime_conversation,
        )))
    }

}

fn resolve_prompt_prepare_target(
    data: &mut AppData,
    data_path: &PathBuf,
    runtime_conversation_id: Option<&str>,
    selected_api: &ApiConfig,
    effective_agent_id: &str,
    requested_conversation_id: Option<&str>,
    read_only: bool,
) -> Result<Option<(Option<usize>, bool)>, String> {
    let requested_conversation_idx = requested_conversation_id.and_then(|conversation_id| {
        data.conversations
            .iter()
            .position(|item| item.id == conversation_id && item.summary.trim().is_empty())
    });
    let is_runtime_conversation = requested_conversation_id.is_some()
        && requested_conversation_idx.is_none()
        && runtime_conversation_id.is_some();
    let idx = if let Some(requested_idx) = requested_conversation_idx {
        Some(requested_idx)
    } else if is_runtime_conversation {
        None
    } else if let Some(conversation_id) = requested_conversation_id {
        if read_only {
            return Ok(None);
        }
        Some(
            data.conversations
                .iter()
                .position(|item| item.id == conversation_id && item.summary.trim().is_empty())
                .ok_or_else(|| format!("指定会话不存在或不可用：{conversation_id}"))?,
        )
    } else if read_only {
        active_foreground_conversation_index_read_only(data, effective_agent_id)
    } else {
        Some(ensure_active_foreground_conversation_index_atomic(
            data,
            data_path,
            &selected_api.id,
            effective_agent_id,
        ))
    };
    Ok(Some((idx, is_runtime_conversation)))
}

fn build_prompt_prepare_resolution(
    data: &AppData,
    conversation_before: &Conversation,
    selected_api: &ApiConfig,
    is_runtime_conversation: bool,
) -> PromptPrepareConversationResolution {
    let is_remote_im_contact_conversation = conversation_is_remote_im_contact(conversation_before);
    let remote_im_contact_processing_mode = if is_remote_im_contact_conversation {
        remote_im_find_contact_by_conversation(data, &conversation_before.id)
            .map(|contact| normalize_contact_processing_mode(&contact.processing_mode))
            .unwrap_or_else(|| "continuous".to_string())
    } else {
        "continuous".to_string()
    };
    PromptPrepareConversationResolution {
        conversation_before: build_prompt_prepare_conversation_before(
            conversation_before,
            is_remote_im_contact_conversation,
            &remote_im_contact_processing_mode,
        ),
        last_archive_summary: prompt_prepare_last_archive_summary(
            data,
            conversation_before,
            is_runtime_conversation,
        ),
        is_remote_im_contact_conversation,
        remote_im_contact_processing_mode,
        response_style_id: data.response_style_id.clone(),
        user_name: user_persona_name(data),
        user_intro: user_persona_intro(data),
        enable_pdf_images: data.pdf_read_mode == "image" && selected_api.enable_image,
        is_runtime_conversation,
    }
}

fn build_prompt_prepare_conversation_before(
    conversation_before: &Conversation,
    is_remote_im_contact_conversation: bool,
    remote_im_contact_processing_mode: &str,
) -> Conversation {
    if is_remote_im_contact_conversation && remote_im_contact_processing_mode == "qa" {
        let trimmed = remote_im_trim_conversation_for_qa_mode(conversation_before);
        eprintln!(
            "[远程IM] 问答模式裁剪会话上下文: conversation_id={}, original_messages={}, trimmed_messages={}",
            conversation_before.id,
            conversation_before.messages.len(),
            trimmed.messages.len()
        );
        return trimmed;
    }
    conversation_before.clone()
}

fn prompt_prepare_last_archive_summary(
    _data: &AppData,
    _conversation_before: &Conversation,
    _is_runtime_conversation: bool,
) -> Option<String> {
    None
}

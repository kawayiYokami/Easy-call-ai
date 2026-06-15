fn normalized_session_search_keyword(keyword: Option<&str>) -> Option<String> {
    keyword
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase())
}

fn session_search_hit(haystacks: &[String], keyword: Option<&str>) -> bool {
    let Some(keyword) = normalized_session_search_keyword(keyword) else {
        return true;
    };
    haystacks.iter().any(|value| {
        let normalized = value.trim().to_lowercase();
        !normalized.is_empty() && normalized.contains(&keyword)
    })
}

fn conversation_bound_persona_name(
    agents: &[AgentProfile],
    conversation: &Conversation,
) -> Option<String> {
    let agent_id = conversation.agent_id.trim();
    if agent_id.is_empty() {
        return None;
    }
    agents
        .iter()
        .find(|agent| agent.id.trim() == agent_id)
        .map(|agent| agent.name.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn conversation_bound_department_name(
    config: &AppConfig,
    conversation: &Conversation,
) -> Option<String> {
    let department_id = conversation.department_id.trim();
    if department_id.is_empty() {
        return None;
    }
    config
        .departments
        .iter()
        .find(|department| department.id.trim() == department_id)
        .map(|department| department.name.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn session_notification_source_label(
    title: &str,
    department_name: Option<&str>,
    persona_name: Option<&str>,
) -> String {
    let title = title.trim();
    let department_name = department_name.unwrap_or("").trim();
    let persona_name = persona_name.unwrap_or("").trim();
    let left = if title.is_empty() { "未命名会话" } else { title };
    let middle = if department_name.is_empty() { "未绑定部门" } else { department_name };
    let right = if persona_name.is_empty() { "未绑定人格" } else { persona_name };
    format!("[{}·{}·{}]", left, middle, right)
}

fn delegate_completion_notification_label(
    department_name: Option<&str>,
    persona_name: Option<&str>,
) -> String {
    let department_name = department_name.unwrap_or("").trim();
    let persona_name = persona_name.unwrap_or("").trim();
    let left = if department_name.is_empty() { "未绑定部门" } else { department_name };
    let right = if persona_name.is_empty() { "未绑定人格" } else { persona_name };
    format!("[{}·{}]", left, right)
}

fn build_delegate_completion_notification_body(
    state: &AppState,
    target_department_id: &str,
    target_agent_id: &str,
    content: &str,
) -> Result<String, String> {
    let normalized_content = content.trim();
    if normalized_content.is_empty() {
        return Err("通知正文不能为空".to_string());
    }
    let runtime_snapshot = load_runtime_organization_snapshot(state)?;
    let department_name = runtime_snapshot
        .config
        .departments
        .iter()
        .find(|department| department.id.trim() == target_department_id.trim())
        .map(|department| department.name.trim().to_string());
    let persona_name = runtime_snapshot
        .agents
        .iter()
        .find(|agent| agent.id.trim() == target_agent_id.trim())
        .map(|agent| agent.name.trim().to_string());
    let label = delegate_completion_notification_label(
        department_name.as_deref(),
        persona_name.as_deref(),
    );
    Ok(format!("{label}:{normalized_content}"))
}

fn build_session_notification_body(
    state: &AppState,
    source_conversation_id: &str,
    content: &str,
) -> Result<String, String> {
    let normalized_conversation_id = source_conversation_id.trim();
    if normalized_conversation_id.is_empty() {
        return Err("sourceConversationId 不能为空".to_string());
    }
    let normalized_content = content.trim();
    if normalized_content.is_empty() {
        return Err("通知正文不能为空".to_string());
    }
    let runtime_snapshot = load_runtime_organization_snapshot(state)?;
    let conversation = state_read_conversation_cached(state, normalized_conversation_id)
        .map_err(|_| "来源会话不存在".to_string())?;
    let label = session_notification_source_label(
        &conversation.title,
        conversation_bound_department_name(&runtime_snapshot.config, &conversation).as_deref(),
        conversation_bound_persona_name(&runtime_snapshot.agents, &conversation).as_deref(),
    );
    Ok(format!("{label}:{normalized_content}"))
}

fn build_session_notification_message(text: &str) -> ChatMessage {
    ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        created_at: now_iso(),
        speaker_agent_id: Some(SYSTEM_PERSONA_ID.to_string()),
        parts: vec![MessagePart::Text {
            text: text.to_string(),
            reasoning_content: None,
        }],
        extra_text_blocks: Vec::new(),
        provider_meta: Some(serde_json::json!({
            "messageKind": "session_notification",
            "sessionNotification": true,
        })),
        tool_call: None,
        mcp_call: None,
        meme_annotations: None,
    }
}

fn selected_messages_notification_content(selected_messages: &[ChatMessage]) -> String {
    selected_messages
        .iter()
        .map(|message| {
            let speaker = match message.role.trim().to_ascii_lowercase().as_str() {
                "user" => "用户",
                "assistant" => "助手",
                "system" => "系统",
                "tool" => "工具",
                _ => "消息",
            };
            let text = message
                .parts
                .iter()
                .filter_map(|part| match part {
                    MessagePart::Text { text, .. } => Some(text.trim()),
                    _ => None,
                })
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n");
            if text.is_empty() {
                format!("[{speaker}]: [空消息]")
            } else {
                format!("[{speaker}]: {text}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[derive(Debug, Clone)]
struct SessionNotificationDispatchRequest {
    state: AppState,
    target_conversation_id: String,
    body: String,
    message: ChatMessage,
    action: String,
}

fn session_notification_body_preview(body: &str) -> String {
    let preview = body.trim().chars().take(60).collect::<String>();
    if preview.is_empty() {
        "[空正文]".to_string()
    } else {
        preview
    }
}

fn session_notification_dispatch_sender(
) -> Result<&'static std::sync::mpsc::Sender<SessionNotificationDispatchRequest>, String> {
    static SENDER: OnceLock<std::sync::mpsc::Sender<SessionNotificationDispatchRequest>> =
        OnceLock::new();
    if let Some(sender) = SENDER.get() {
        return Ok(sender);
    }

    let (tx, rx) = std::sync::mpsc::channel::<SessionNotificationDispatchRequest>();
    std::thread::Builder::new()
        .name("session-notification-worker".to_string())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("session-notification-async")
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(err) => {
                    runtime_log_error(format!(
                        "[会话通知] 失败，任务=启动后台投递 worker，error={err}"
                    ));
                    return;
                }
            };
            runtime_log_info("[会话通知] 完成，任务=启动后台投递 worker".to_string());
            for request in rx {
                runtime_log_info(format!(
                    "[会话通知] 开始，任务=接收投递请求，action={}，target_conversation_id={}，message_id={}，body_preview={}",
                    request.action,
                    request.target_conversation_id,
                    request.message.id,
                    session_notification_body_preview(&request.body)
                ));
                runtime.spawn(async move {
                    if let Err(err) = process_session_notification_dispatch_request(request).await {
                        runtime_log_error(format!(
                            "[会话通知] 失败，任务=执行会话投递，error={err}"
                        ));
                    }
                });
            }
            runtime_log_warn("[会话通知] 跳过，任务=后台投递 worker，原因=请求通道已关闭".to_string());
        })
        .map_err(|err| format!("启动会话通知 worker 失败: {err}"))?;

    let _ = SENDER.set(tx);
    SENDER
        .get()
        .ok_or_else(|| "会话通知 worker 启动后未注册 sender".to_string())
}

fn enqueue_session_notification_dispatch(
    state: &AppState,
    target_conversation_id: &str,
    body: &str,
    message: &ChatMessage,
    action: &str,
) -> Result<(), String> {
    let normalized_target_conversation_id = target_conversation_id.trim();
    let sender = session_notification_dispatch_sender()?;
    runtime_log_info(format!(
        "[会话通知] 开始，任务=投递请求入队，action={}，target_conversation_id={}，message_id={}，body_preview={}",
        action,
        normalized_target_conversation_id,
        message.id,
        session_notification_body_preview(body)
    ));
    sender
        .send(SessionNotificationDispatchRequest {
            state: state.clone(),
            target_conversation_id: normalized_target_conversation_id.to_string(),
            body: body.to_string(),
            message: message.clone(),
            action: action.to_string(),
        })
        .map_err(|err| format!("投递会话通知入队失败: {err}"))?;
    runtime_log_info(format!(
        "[会话通知] 完成，任务=投递请求入队，action={}，target_conversation_id={}，message_id={}",
        action,
        normalized_target_conversation_id,
        message.id
    ));
    Ok(())
}

fn session_notification_wait_reason(state: MainSessionState) -> Option<&'static str> {
    match state {
        MainSessionState::AssistantStreaming => Some("目标会话正在流式输出"),
        MainSessionState::OrganizingContext => Some("目标会话正在整理上下文"),
        MainSessionState::Idle => None,
    }
}

async fn process_session_notification_dispatch_request(
    request: SessionNotificationDispatchRequest,
) -> Result<(), String> {
    const RETRY_DELAY_MS: u64 = 350;
    let started_at = std::time::Instant::now();
    let mut wait_round = 0u32;
    runtime_log_info(format!(
        "[会话通知] 开始，任务=执行会话投递，action={}，target_conversation_id={}，message_id={}",
        request.action,
        request.target_conversation_id,
        request.message.id
    ));
    loop {
        let state = get_conversation_runtime_state(&request.state, &request.target_conversation_id)?;
        if let Some(reason) = session_notification_wait_reason(state.clone()) {
            wait_round += 1;
            if wait_round == 1 || wait_round % 10 == 0 {
                runtime_log_info(format!(
                    "[会话通知] 等待，任务=会话间投递，action={}，target_conversation_id={}，reason={}，wait_round={}，duration_ms={}",
                    request.action,
                    request.target_conversation_id,
                    reason,
                    wait_round,
                    started_at.elapsed().as_millis()
                ));
            }
            tokio::time::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS)).await;
            continue;
        }
        break;
    }

    conversation_service()
        .deliver_session_notification(
            &request.state,
            &request.target_conversation_id,
            &request.body,
            &request.message,
            &request.action,
        )
        .await?;
    runtime_log_info(format!(
        "[会话通知] 完成，任务=会话间投递，action={}，target_conversation_id={}，message_id={}，wait_round={}，duration_ms={}",
        request.action,
        request.target_conversation_id,
        request.message.id,
        wait_round,
        started_at.elapsed().as_millis()
    ));
    Ok(())
}

impl ConversationService {
    fn list_tool_session_targets(
        &self,
        state: &AppState,
        keyword: Option<&str>,
    ) -> Result<Vec<ToolSessionTargetSummary>, String> {
        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let config = runtime_snapshot.config;
        let agents = runtime_snapshot.agents;
        let local_items = self
            .collect_unarchived_conversation_summaries_cached(state, &config)?
            .into_iter()
            .filter(|item| !item.is_system_notification_conversation)
            .filter_map(|item| {
                let conversation =
                    state_read_conversation_cached(state, &item.conversation_id).ok()?;
                if !conversation_is_local_normal_chat(&conversation) {
                    return None;
                }
                let persona_name = conversation_bound_persona_name(&agents, &conversation);
                let department_name = conversation_bound_department_name(&config, &conversation)
                    .or_else(|| {
                        let name = item.department_name.trim();
                        if name.is_empty() {
                            None
                        } else {
                            Some(name.to_string())
                        }
                    });
                let title = if !item.title.trim().is_empty() {
                    item.title.trim().to_string()
                } else if let Some(summary_title) = item.summary_title.as_deref().map(str::trim) {
                    summary_title.to_string()
                } else {
                    item.conversation_id.clone()
                };
                let haystacks = vec![
                    title.clone(),
                    item.summary_title.unwrap_or_default(),
                    department_name.clone().unwrap_or_default(),
                    persona_name.clone().unwrap_or_default(),
                ];
                if !session_search_hit(&haystacks, keyword) {
                    return None;
                }
                Some(ToolSessionTargetSummary {
                    session_id: item.conversation_id.clone(),
                    kind: "local_unarchived".to_string(),
                    title,
                    department_name,
                    persona_name,
                    remote_contact_id: None,
                    remote_contact_name: None,
                    channel_name: None,
                    updated_at: item.updated_at,
                })
            })
            .collect::<Vec<_>>();

        let remote_items = self
            .list_remote_im_contact_conversations(state)?
            .into_iter()
            .filter_map(|item| {
                let department_name = item
                    .bound_department_id
                    .as_deref()
                    .and_then(|department_id| {
                        config
                            .departments
                            .iter()
                            .find(|department| department.id.trim() == department_id.trim())
                            .map(|department| department.name.trim().to_string())
                    })
                    .filter(|value| !value.is_empty());
                let persona_name = item
                    .bound_agent_id
                    .as_deref()
                    .and_then(|agent_id| {
                        agents
                            .iter()
                            .find(|agent| agent.id.trim() == agent_id.trim())
                            .map(|agent| agent.name.trim().to_string())
                    })
                    .filter(|value| !value.is_empty());
                let haystacks = vec![
                    item.title.clone(),
                    item.contact_display_name.clone(),
                    item.channel_name.clone().unwrap_or_default(),
                    department_name.clone().unwrap_or_default(),
                    persona_name.clone().unwrap_or_default(),
                ];
                if !session_search_hit(&haystacks, keyword) {
                    return None;
                }
                Some(ToolSessionTargetSummary {
                    session_id: item.conversation_id,
                    kind: "remote_im_contact".to_string(),
                    title: item.title,
                    department_name,
                    persona_name,
                    remote_contact_id: Some(item.contact_id),
                    remote_contact_name: Some(item.contact_display_name),
                    channel_name: item.channel_name,
                    updated_at: item.updated_at,
                })
            })
            .collect::<Vec<_>>();

        let mut items = Vec::<ToolSessionTargetSummary>::new();
        items.extend(local_items);
        items.extend(remote_items);
        items.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then_with(|| a.title.cmp(&b.title)));
        Ok(items)
    }

    async fn deliver_session_notification(
        &self,
        state: &AppState,
        target_session_id: &str,
        body: &str,
        message: &ChatMessage,
        action: &str,
    ) -> Result<(), String> {
        let normalized_target_session_id = target_session_id.trim();
        let app_config = state_read_config_cached(state)?;
        let target_conversation =
            state_read_conversation_cached(state, normalized_target_session_id)
            .map_err(|_| "目标会话不存在".to_string())?;
        if !conversation_is_unarchived(&target_conversation) {
            return Err("目标会话不存在".to_string());
        }

        if conversation_is_remote_im_contact(&target_conversation) {
            let runtime = state_read_runtime_state_cached(state)?;
            let contact = self
                .find_remote_im_contact_by_conversation_in_runtime(
                    &runtime,
                    normalized_target_session_id,
                )
                .cloned()
                .ok_or_else(|| "目标远程联系人不存在".to_string())?;
            let channel = remote_im_channel_by_id(&app_config, &contact.channel_id)
                .cloned()
                .ok_or_else(|| format!("远程 IM 渠道不存在: {}", contact.channel_id))?;
            if !channel.enabled {
                return Err(format!("远程 IM 渠道未启用: {}", contact.channel_id));
            }
            if !contact.allow_send {
                return Err("当前联系人不允许发送消息".to_string());
            }
            runtime_log_info(format!(
                "[会话通知] 开始，任务=远程联系人投递，action={}，target_conversation_id={}，remote_contact_id={}，channel_id={}，message_id={}",
                action,
                normalized_target_session_id,
                contact.id,
                contact.channel_id,
                message.id
            ));
            remote_im_send_content_payload(
                state,
                &channel,
                &contact,
                vec![serde_json::json!({
                    "type": "text",
                    "text": body,
                })],
                false,
                action,
            ).await?;
            runtime_log_info(format!(
                "[会话通知] 完成，任务=远程联系人投递，action={}，target_conversation_id={}，remote_contact_id={}，channel_id={}，message_id={}",
                action,
                normalized_target_session_id,
                contact.id,
                contact.channel_id,
                message.id
            ));
        } else {
            if !conversation_visible_in_foreground_lists(&target_conversation)
                || !conversation_is_local_normal_chat(&target_conversation)
            {
                return Err("目标会话不存在".to_string());
            }
        }
        self.append_message_to_unarchived_conversation(
            state,
            normalized_target_session_id,
            message,
        )?;
        emit_conversation_message_appended_event(state, normalized_target_session_id, message);
        match self.collect_unarchived_conversation_summaries_cached(state, &app_config) {
            Ok(unarchived_conversations) => {
                emit_unarchived_conversation_overview_updated_payload(
                    state,
                    &UnarchivedConversationOverviewUpdatedPayload {
                        preferred_conversation_id: Some(normalized_target_session_id.to_string()),
                        unarchived_conversations,
                    },
                );
            }
            Err(err) => runtime_log_warn(format!(
                "[会话通知] 警告，任务=刷新会话概览，target_conversation_id={}，error={}",
                normalized_target_session_id, err
            )),
        }
        Ok(())
    }

    fn inform_session(
        &self,
        state: &AppState,
        source_conversation_id: &str,
        target_session_id: &str,
        content: &str,
    ) -> Result<InformSessionMutationResult, String> {
        let normalized_target_session_id = target_session_id.trim();
        if normalized_target_session_id.is_empty() {
            return Err("session_id 不能为空".to_string());
        }
        let body = build_session_notification_body(state, source_conversation_id, content)?;
        let message = build_session_notification_message(&body);
        enqueue_session_notification_dispatch(
            state,
            normalized_target_session_id,
            &body,
            &message,
            "inform_session",
        )?;
        Ok(InformSessionMutationResult {
            target_conversation_id: normalized_target_session_id.to_string(),
            target_kind: "queued".to_string(),
            remote_contact_id: None,
            pushed_to_remote: false,
            message,
        })
    }

    fn enqueue_delegate_completion_session_notification(
        &self,
        state: &AppState,
        root_conversation_id: &str,
        target_department_id: &str,
        target_agent_id: &str,
        content: &str,
        action: &str,
    ) -> Result<(), String> {
        let resolved_target =
            self.resolve_delegate_result_target_conversation(state, root_conversation_id)?;
        let body = build_delegate_completion_notification_body(
            state,
            target_department_id,
            target_agent_id,
            content,
        )?;
        let message = build_session_notification_message(&body);
        enqueue_session_notification_dispatch(
            state,
            &resolved_target.target_conversation_id,
            &body,
            &message,
            action,
        )
    }

    fn resolve_remote_im_contact_conversation_id_for_notification(
        &self,
        state: &AppState,
        remote_contact_id: &str,
    ) -> Result<String, String> {
        let normalized_remote_contact_id = remote_contact_id.trim();
        if normalized_remote_contact_id.is_empty() {
            return Err("remoteContactId 不能为空".to_string());
        }
        let mut runtime = state_read_runtime_state_cached(state)?;
        let contact = runtime
            .remote_im_contacts
            .iter_mut()
            .find(|item| item.id.trim() == normalized_remote_contact_id)
            .ok_or_else(|| format!("未找到远程联系人：{normalized_remote_contact_id}"))?;
        let config = state_read_config_cached(state)?;
        let channel = remote_im_channel_by_id(&config, &contact.channel_id)
            .ok_or_else(|| format!("远程联系人所属渠道不存在：{}", contact.channel_id))?;
        if !channel.enabled {
            return Err(format!("远程联系人所属渠道未启用：{}", contact.channel_id));
        }
        let previous_bound_conversation_id = contact
            .bound_conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let conversation_id = ensure_remote_im_contact_conversation_id(state, contact)?;
        if previous_bound_conversation_id.as_deref() != Some(conversation_id.as_str()) {
            state_write_runtime_state_cached(state, &runtime)?;
            runtime_log_info(format!(
                "[自动推送] 完成，任务=修复远程联系人绑定会话，remote_contact_id={}，conversation_id={}，previous_conversation_id={}",
                normalized_remote_contact_id,
                conversation_id,
                previous_bound_conversation_id.as_deref().unwrap_or("")
            ));
        } else {
            runtime_log_info(format!(
                "[自动推送] 完成，任务=解析远程联系人绑定会话，remote_contact_id={}，conversation_id={}",
                normalized_remote_contact_id, conversation_id
            ));
        }
        Ok(conversation_id)
    }

    fn enqueue_auto_push_remote_contact_message(
        &self,
        state: &AppState,
        source_conversation_id: &str,
        remote_contact_id: &str,
        content: &str,
    ) -> Result<(), String> {
        let normalized_source_conversation_id = source_conversation_id.trim();
        let normalized_remote_contact_id = remote_contact_id.trim();
        if normalized_source_conversation_id.is_empty() || normalized_remote_contact_id.is_empty() {
            return Ok(());
        }
        let source_conversation =
            state_read_conversation_cached(state, normalized_source_conversation_id)?;
        if !conversation_is_local_normal_chat(&source_conversation)
            || !conversation_visible_in_foreground_lists(&source_conversation)
            || conversation_is_system_notification(&source_conversation)
        {
            runtime_log_info(format!(
                "[自动推送] 跳过，任务=解析推送源会话，source_conversation_id={}，remote_contact_id={}，reason=source_conversation_not_eligible",
                normalized_source_conversation_id,
                normalized_remote_contact_id
            ));
            return Ok(());
        }
        runtime_log_info(format!(
            "[自动推送] 开始，任务=解析远程联系人通知目标，source_conversation_id={}，remote_contact_id={}",
            normalized_source_conversation_id,
            normalized_remote_contact_id
        ));
        let target_conversation_id = self.resolve_remote_im_contact_conversation_id_for_notification(
            state,
            normalized_remote_contact_id,
        )?;
        let body = build_session_notification_body(state, normalized_source_conversation_id, content)?;
        let message = build_session_notification_message(&body);
        runtime_log_info(format!(
            "[自动推送] 开始，任务=通知转发入队，source_conversation_id={}，target_conversation_id={}，remote_contact_id={}，message_id={}",
            normalized_source_conversation_id,
            target_conversation_id,
            normalized_remote_contact_id,
            message.id
        ));
        enqueue_session_notification_dispatch(
            state,
            &target_conversation_id,
            &body,
            &message,
            "auto_push_session",
        )?;
        runtime_log_info(format!(
            "[自动推送] 完成，任务=通知转发入队，source_conversation_id={}，target_conversation_id={}，remote_contact_id={}，message_id={}",
            normalized_source_conversation_id,
            target_conversation_id,
            normalized_remote_contact_id,
            message.id
        ));
        Ok(())
    }
}

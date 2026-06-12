fn normalize_payload_image_attachments(
    raw: Option<&Vec<BinaryPart>>,
) -> Vec<serde_json::Value> {
    let mut out = Vec::<serde_json::Value>::new();
    let Some(images) = raw else {
        return out;
    };
    let mut seen = std::collections::HashSet::<String>::new();
    for image in images {
        let relative_path = image
            .saved_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.replace('\\', "/"));
        let Some(relative_path) = relative_path else {
            continue;
        };
        let file_name = std::path::Path::new(&relative_path)
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("attachment")
            .to_string();
        let mime = image.mime.trim().to_string();
        let dedup_key = format!("{}::{}", relative_path, mime);
        if !seen.insert(dedup_key) {
            continue;
        }
        out.push(serde_json::json!({
            "fileName": file_name,
            "relativePath": relative_path,
            "mime": mime,
        }));
    }
    out
}

fn normalize_payload_mentions(
    raw: Option<&Vec<UserMentionTargetInput>>,
) -> Vec<serde_json::Value> {
    let Some(items) = raw else {
        return Vec::new();
    };
    let mut seen = std::collections::HashSet::<String>::new();
    let mut out = Vec::<serde_json::Value>::new();
    for item in items.iter().take(3) {
        let agent_id = item.agent_id.trim();
        let department_id = item.department_id.trim();
        if agent_id.is_empty() || department_id.is_empty() {
            continue;
        }
        let dedup_key = format!("{agent_id}::{department_id}");
        if !seen.insert(dedup_key) {
            continue;
        }
        out.push(serde_json::json!({
            "agentId": agent_id,
            "agentName": item.agent_name.as_deref().map(str::trim).filter(|value| !value.is_empty()).unwrap_or(agent_id),
            "departmentId": department_id,
            "departmentName": item.department_name.as_deref().map(str::trim).filter(|value| !value.is_empty()).unwrap_or(department_id),
        }));
    }
    out
}

fn build_user_message_provider_meta(
    input_provider_meta: Option<Value>,
    attachments: &[serde_json::Value],
    mentions: &[serde_json::Value],
    request_id: Option<&str>,
) -> Option<Value> {
    let merged = merge_provider_meta_with_attachments(input_provider_meta, attachments);
    let mut root = match merged {
        Some(Value::Object(map)) => map,
        Some(other) => {
            let mut map = serde_json::Map::new();
            map.insert("_raw".to_string(), other);
            map
        }
        None => serde_json::Map::new(),
    };

    let message_meta_value = root
        .remove("message_meta")
        .or_else(|| root.remove("messageMeta"))
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
    let mut message_meta = match message_meta_value {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    message_meta.insert("kind".to_string(), Value::String("user_message".to_string()));
    if !mentions.is_empty() {
        message_meta.insert("mentions".to_string(), Value::Array(mentions.to_vec()));
    }
    root.insert("message_meta".to_string(), Value::Object(message_meta));
    if let Some(request_id) = request_id.map(str::trim).filter(|value| !value.is_empty()) {
        root.insert("requestId".to_string(), Value::String(request_id.to_string()));
    }
    Some(Value::Object(root))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfirmPlanAndContinueInput {
    conversation_id: String,
    plan_message_id: String,
    #[serde(default)]
    department_id: Option<String>,
    #[serde(default)]
    agent_id: Option<String>,
}

fn plan_path_from_message_provider_meta(message: &ChatMessage) -> Option<String> {
    message
        .provider_meta
        .as_ref()
        .and_then(|meta| meta.get("planCard"))
        .and_then(Value::as_object)
        .and_then(|card| card.get("path"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn resolve_plan_file_for_conversation_id(
    state: &AppState,
    conversation_id: &str,
    raw_path: &str,
) -> Result<ResolvedPlanFilePath, String> {
    let conversation = state_read_conversation_cached(state, conversation_id)?;
    let base_root = terminal_default_workspace_for_conversation_resolved(state, Some(&conversation))
        .map(|workspace| workspace.path)
        .or_else(|_| plan_self_directory_canonical(state))?;
    resolve_plan_file_for_conversation(&base_root, raw_path)
}

fn plan_continue_confirmation_message() -> ChatMessage {
    ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "user".to_string(),
        created_at: now_iso(),
        speaker_agent_id: None,
        parts: vec![MessagePart::Text {
            text: "我同意，请执行。".to_string(),
                reasoning_content: None,
            }],
        extra_text_blocks: Vec::new(),
        provider_meta: Some(serde_json::json!({
            "messageKind": "plan_confirm_continue",
            "message_meta": {
                "kind": "plan_confirm_continue"
            }
        })),
        tool_call: None,
        mcp_call: None,
        meme_annotations: None,
    }
}

fn resolve_runtime_control_department_and_agent(
    state: &AppState,
    requested_department_id: Option<&str>,
    requested_agent_id: Option<&str>,
    requested_conversation_id: Option<&str>,
) -> Result<(String, String), String> {
    let bound_conversation = requested_conversation_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|conversation_id| state_read_conversation_cached(state, conversation_id).ok());
    if let Some(conversation) = bound_conversation.as_ref() {
        let department_id = conversation.department_id.trim();
        if department_id.is_empty() {
            return Err(format!("会话缺少绑定部门：conversationId={}", conversation.id));
        }
        let runtime_org = load_runtime_organization_snapshot(state)?;
        let department = runtime_department_by_id(&runtime_org, department_id)
            .ok_or_else(|| format!("部门已经消失：{department_id}"))?;
        let agent_id = conversation.agent_id.trim();
        let agent_id = if agent_id.is_empty() {
            first_available_department_agent(department, &runtime_org.agents)
                .map(|agent| agent.id.clone())
                .ok_or_else(|| format!("会话绑定部门没有可用人格：conversationId={}", conversation.id))?
        } else {
            if available_non_user_agent(&runtime_org.agents, agent_id).is_none() {
                return Err(format!("会话人格已经消失或不可用：{agent_id}"));
            }
            agent_id.to_string()
        };
        return Ok((department.id.clone(), agent_id.to_string()));
    }
    let department_id = requested_department_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| "Missing session.departmentId".to_string())?;
    let runtime_org = load_runtime_organization_snapshot(state)?;
    let department = runtime_department_by_id(&runtime_org, &department_id)
        .ok_or_else(|| format!("部门已经消失：{department_id}"))?;
    let agent_id = requested_agent_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            first_available_department_agent(department, &runtime_org.agents)
                .map(|agent| agent.id.clone())
        })
        .ok_or_else(|| "Missing session.agentId".to_string())?;
    if available_non_user_agent(&runtime_org.agents, &agent_id).is_none() {
        return Err(format!("会话人格已经消失或不可用：{agent_id}"));
    }
    Ok((department.id.clone(), agent_id))
}

fn plan_confirm_context_usage_ratio(source: &Conversation, selected_api: &ApiConfig) -> f64 {
    conversation_prompt_service()
        .latest_real_prompt_usage(source, selected_api)
        .map(|usage| usage.usage_ratio.max(0.0))
        .unwrap_or(0.0)
}

async fn confirm_plan_and_continue_inner(
    state: &AppState,
    input: &ConfirmPlanAndContinueInput,
) -> Result<bool, String> {
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required.".to_string());
    }
    let plan_message_id = input.plan_message_id.trim();
    if plan_message_id.is_empty() {
        return Err("planMessageId is required.".to_string());
    }
    let plan_message = conversation_service().read_message_by_id(
        state,
        conversation_id,
        plan_message_id,
    )?;
    let plan_path = plan_path_from_message_provider_meta(&plan_message)
        .ok_or_else(|| "指定消息不是可执行计划。".to_string())?;
    let resolved_plan_path =
        resolve_plan_file_for_conversation_id(state, conversation_id, &plan_path)?;
    message_store::active_plan_append_in_progress(
        &state.data_path,
        conversation_id,
        plan_message_id,
        &resolved_plan_path.display_path,
    )?;
    let conversation = state_read_conversation_cached(state, conversation_id)?;
    let requested_department_id = input
        .department_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            let department_id = conversation.department_id.trim();
            (!department_id.is_empty()).then(|| department_id.to_string())
        });
    let requested_agent_id = input
        .agent_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            let agent_id = conversation.agent_id.trim();
            (!agent_id.is_empty()).then(|| agent_id.to_string())
        });
    let (selected_api, resolved_api, department_id, agent_id) = {
        let runtime_org = load_runtime_organization_snapshot(state)?;
        let app_config = &runtime_org.config;
        let department = requested_department_id
            .as_deref()
            .and_then(|department_id| runtime_department_by_id(&runtime_org, department_id))
            .ok_or_else(|| "找不到可用于继续执行计划的部门。".to_string())?;
        let agent_id = requested_agent_id
            .as_deref()
            .ok_or_else(|| format!("缺少可用于继续执行计划的人格: department_id={}", department.id))?
            .to_string();
        if !runtime_org
            .agents
            .iter()
            .any(|agent| agent.id == agent_id && !agent.is_built_in_user)
        {
            return Err(format!("计划继续执行的人格不存在或不可用: agent_id={agent_id}"));
        }
        let api_config_id = department_primary_chat_api_config_id(app_config, department)
            .ok_or_else(|| format!("部门模型未配置或不可用于聊天: {}", department.id))?;
        let selected_api = app_config
            .api_configs
            .iter()
            .find(|api| api.id == api_config_id)
            .cloned()
            .ok_or_else(|| format!("模型配置不存在: {api_config_id}"))?;
        let resolved_api = resolve_api_config(&app_config, Some(selected_api.id.as_str()))?;
        (
            selected_api,
            resolved_api,
            department.id.clone(),
            agent_id,
        )
    };
    let continue_event_id = format!("confirm-plan-continue-{}", Uuid::new_v4());
    let preview = build_trim_compaction_preview_result(state, &selected_api, &conversation)?;
    let should_compact_before_continue =
        preview.can_compact && plan_confirm_context_usage_ratio(&conversation, &selected_api) >= 0.60;
    if should_compact_before_continue {
        let compaction_result = run_context_compaction_pipeline(
            state,
            &selected_api,
            &resolved_api,
            &conversation,
            &agent_id,
            "confirm_plan_before_continue",
            "COMPACTION-CONFIRM-PLAN",
            &[],
            false,
        )
        .await;
        match compaction_result {
            Ok(result) => {
                runtime_log_info(format!(
                    "[上下文整理] 计划确认前压缩完成 conversation_id={} merged_memories={} warning={}",
                    conversation_id,
                    result.merged_memories,
                    result.warning.as_deref().unwrap_or("")
                ));
            }
            Err(err) => {
                return Err(err);
            }
        }
    }
    let mut runtime_context = runtime_context_new("plan_confirm", "context_compaction_followup");
    runtime_context.request_id = Some(continue_event_id.clone());
    runtime_context.dispatch_id = Some(continue_event_id.clone());
    runtime_context.origin_conversation_id = Some(conversation_id.to_string());
    runtime_context.target_conversation_id = Some(conversation_id.to_string());
    runtime_context.root_conversation_id = Some(conversation_id.to_string());
    runtime_context.executor_agent_id = Some(agent_id.clone());
    runtime_context.executor_department_id = Some(department_id.clone());
    runtime_context.model_config_id = Some(selected_api.id.clone());
    let event = ChatPendingEvent {
        id: continue_event_id,
        conversation_id: conversation_id.to_string(),
        created_at: now_iso(),
        source: ChatEventSource::System,
        queue_mode: ChatQueueMode::Normal,
        messages: vec![plan_continue_confirmation_message()],
        activate_assistant: true,
        session_info: ChatSessionInfo {
            department_id,
            agent_id,
        },
        runtime_context: Some(runtime_context),
        sender_info: None,
    };
    match ingress_chat_event(state, event)? {
        ChatEventIngress::Direct(event) => {
            trigger_chat_event_after_ingress(state, ChatEventIngress::Direct(event));
        }
        ChatEventIngress::Queued { event_id } => {
            runtime_log_info(format!(
                "[计划] 确认后继续执行已入队 conversation_id={} event_id={}",
                conversation_id, event_id
            ));
        }
        ChatEventIngress::Duplicate { event_id } => {
            runtime_log_info(format!(
                "[计划] 确认后继续执行重复，已忽略 conversation_id={} event_id={}",
                conversation_id, event_id
            ));
        }
    }
    trigger_chat_queue_processing(state);
    Ok(true)
}

#[tauri::command]
async fn confirm_plan_and_continue(
    input: ConfirmPlanAndContinueInput,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    confirm_plan_and_continue_inner(state.inner(), &input).await
}

#[tauri::command]
fn read_plan_file_content(
    conversation_id: String,
    path: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let normalized_conversation_id = conversation_id.trim();
    if normalized_conversation_id.is_empty() {
        return Err("conversationId is required.".to_string());
    }
    let resolved = resolve_plan_file_for_conversation_id(
        state.inner(),
        normalized_conversation_id,
        path.trim(),
    )?;
    read_plan_markdown_file(&resolved.canonical_path)
}

#[derive(Debug, Clone)]
struct UserMentionPlan {
    root_conversation_id: String,
    source_department_id: String,
    source_agent_id: String,
    target_department_id: String,
    target_agent_id: String,
    target_agent_name: String,
    instruction: String,
    background: String,
    target_api_config_ids: Vec<String>,
}

#[derive(Debug, Clone)]
struct UserMentionFailurePlan {
    root_conversation_id: String,
    source_agent_id: String,
    target_department_id: String,
    target_agent_id: String,
    target_agent_name: String,
    reason: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubmitUserAsyncDelegateInput {
    conversation_id: String,
    target_department_id: String,
    #[serde(default)]
    target_agent_id: Option<String>,
    #[serde(default)]
    preset_id: Option<String>,
    #[serde(default)]
    why: Option<String>,
    #[serde(default)]
    goal: Option<String>,
    #[serde(default)]
    todo: Option<String>,
    #[serde(default)]
    background: Option<String>,
    #[serde(default)]
    question: Option<String>,
    #[serde(default)]
    focus: Option<String>,
    #[serde(default)]
    selected_message_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SubmitUserAsyncDelegateOutput {
    delegate_id: String,
    conversation_id: String,
    target_agent_id: String,
    target_agent_name: String,
    selected_message_count: usize,
}

#[derive(Debug, Clone)]
struct UserAsyncDelegatePlan {
    root_conversation_id: String,
    source_department_id: String,
    source_agent_id: String,
    target_department_id: String,
    target_agent_id: String,
    target_agent_name: String,
    title: String,
    goal: String,
    why: String,
    todo: String,
    target_api_config_ids: Vec<String>,
}

fn build_user_mention_context_snapshot_with_agents(
    conversation: &Conversation,
    agents: &[AgentProfile],
    latest_user_text: &str,
) -> String {
    let mut lines = Vec::<String>::new();
    let recent_messages = conversation
        .messages
        .iter()
        .rev()
        .filter_map(|message| {
            let text = render_prompt_message_text(message);
            if text.trim().is_empty() {
                return None;
            }
            let speaker_name = match message.role.trim() {
                "user" => "用户".to_string(),
                "assistant" | "tool" => {
                    let speaker_agent_id = message
                        .speaker_agent_id
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .unwrap_or("");
                    agents
                        .iter()
                        .find(|agent| agent.id == speaker_agent_id)
                        .map(|agent| agent.name.trim().to_string())
                        .filter(|value| !value.is_empty())
                        .unwrap_or_else(|| "助理".to_string())
                }
                other => other.to_string(),
            };
            Some(format!("[{}] {}", speaker_name, text.trim()))
        })
        .take(12)
        .collect::<Vec<_>>();
    for line in recent_messages.into_iter().rev() {
        lines.push(line);
    }
    if !latest_user_text.trim().is_empty() {
        lines.push(format!("[当前用户问题] {}", latest_user_text.trim()));
    }
    lines.join("\n")
}

fn build_user_mention_dispatch_plans(
    app_config: &AppConfig,
    conversation: &Conversation,
    agents: &[AgentProfile],
    source_department_id: &str,
    source_agent_id: &str,
    latest_user_text: &str,
    mentions: Option<&Vec<UserMentionTargetInput>>,
) -> Result<(Vec<UserMentionPlan>, Vec<UserMentionFailurePlan>), String> {
    let Some(items) = mentions.filter(|items| !items.is_empty()) else {
        return Ok((Vec::new(), Vec::new()));
    };
    let mention_background =
        build_user_mention_context_snapshot_with_agents(conversation, agents, latest_user_text);
    let mut mention_plans = Vec::<UserMentionPlan>::new();
    let mut mention_failures = Vec::<UserMentionFailurePlan>::new();
    let mut seen_mentions = std::collections::HashSet::<String>::new();
    for mention in items.iter().take(3) {
        let target_department_id = mention.department_id.trim().to_string();
        let target_agent_id = mention.agent_id.trim().to_string();
        let target_department_name = mention
            .department_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(target_department_id.as_str())
            .to_string();
        if target_department_id.is_empty() || target_agent_id.is_empty() {
            continue;
        }
        let mention_key = format!("{target_department_id}::{target_agent_id}");
        if !seen_mentions.insert(mention_key) {
            continue;
        }
        let Some(target_department) = department_by_id(app_config, &target_department_id) else {
            mention_failures.push(UserMentionFailurePlan {
                root_conversation_id: conversation.id.clone(),
                source_agent_id: source_agent_id.to_string(),
                target_department_id: target_department_id.clone(),
                target_agent_id: String::new(),
                target_agent_name: target_department_name.clone(),
                reason: format!("目标部门不存在，departmentId={target_department_id}"),
            });
            continue;
        };
        if !target_department
            .agent_ids
            .iter()
            .any(|agent_id| agent_id.trim() == target_agent_id)
        {
            mention_failures.push(UserMentionFailurePlan {
                root_conversation_id: conversation.id.clone(),
                source_agent_id: source_agent_id.to_string(),
                target_department_id: target_department_id.clone(),
                target_agent_id: target_agent_id.clone(),
                target_agent_name: target_department_name.clone(),
                reason: format!(
                    "目标人格不属于目标部门，departmentId={}，agentId={}",
                    target_department_id, target_agent_id
                ),
            });
            continue;
        }
        let Some(target_agent) = agents
            .iter()
            .find(|agent| agent.id == target_agent_id && !agent.is_built_in_user)
        else {
            mention_failures.push(UserMentionFailurePlan {
                root_conversation_id: conversation.id.clone(),
                source_agent_id: source_agent_id.to_string(),
                target_department_id: target_department_id.clone(),
                target_agent_id: target_agent_id.clone(),
                target_agent_name: target_department_name.clone(),
                reason: format!("目标人格不存在或不可用，agentId={target_agent_id}"),
            });
            continue;
        };
        let target_agent_name = if target_agent.name.trim().is_empty() {
            target_agent_id.clone()
        } else {
            target_agent.name.trim().to_string()
        };
        if target_agent_id == source_agent_id {
            mention_failures.push(UserMentionFailurePlan {
                root_conversation_id: conversation.id.clone(),
                source_agent_id: source_agent_id.to_string(),
                target_department_id: target_department_id.clone(),
                target_agent_id: target_agent_id.clone(),
                target_agent_name: target_agent_name.clone(),
                reason: SAME_PERSONA_BACKGROUND_DELEGATE_REASON.to_string(),
            });
            continue;
        }
        let target_api_config_ids = delegate_target_chat_api_config_ids(app_config, target_department);
        if target_api_config_ids.is_empty() {
            mention_failures.push(UserMentionFailurePlan {
                root_conversation_id: conversation.id.clone(),
                source_agent_id: source_agent_id.to_string(),
                target_department_id: target_department_id.clone(),
                target_agent_id: target_agent_id.clone(),
                target_agent_name: target_agent_name.clone(),
                reason: format!("目标部门未配置可用模型，departmentId={target_department_id}"),
            });
            continue;
        }
        mention_plans.push(UserMentionPlan {
            root_conversation_id: conversation.id.clone(),
            source_department_id: source_department_id.to_string(),
            source_agent_id: source_agent_id.to_string(),
            target_department_id,
            target_agent_id,
            target_agent_name,
            instruction: latest_user_text.to_string(),
            background: mention_background.clone(),
            target_api_config_ids,
        });
    }
    Ok((mention_plans, mention_failures))
}

fn user_async_delegate_message_text(message: &ChatMessage) -> String {
    if message.role.trim() != "user" && message.role.trim() != "assistant" {
        return String::new();
    }
    let mut chunks = Vec::<String>::new();
    for part in &message.parts {
        if let MessagePart::Text { text, .. } = part {
            let text = text.trim();
            if !text.is_empty() {
                chunks.push(text.to_string());
            }
        }
    }
    chunks.join("\n").trim().to_string()
}

fn user_async_delegate_speaker_name(message: &ChatMessage, agents: &[AgentProfile]) -> String {
    if message.role.trim() == "user" {
        return "用户".to_string();
    }
    let speaker_agent_id = message
        .speaker_agent_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("");
    agents
        .iter()
        .find(|agent| agent.id == speaker_agent_id)
        .map(|agent| agent.name.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "助理".to_string())
}

fn user_async_delegate_truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let head = text.chars().take(max_chars).collect::<String>();
    format!("{head}\n[内容过长，已截断]")
}

fn build_user_async_delegate_selected_context(
    conversation: &Conversation,
    agents: &[AgentProfile],
    selected_message_ids: &[String],
) -> (String, usize) {
    let (selected_messages, _) =
        collect_selected_messages_for_branch(conversation, selected_message_ids);
    let mut lines = Vec::<String>::new();
    let mut count = 0usize;
    for message in selected_messages {
        let text = user_async_delegate_message_text(&message);
        if text.trim().is_empty() {
            continue;
        }
        count += 1;
        let speaker = user_async_delegate_speaker_name(&message, agents);
        lines.push(format!(
            "[{}] {}",
            speaker,
            user_async_delegate_truncate_chars(text.trim(), 4000)
        ));
    }
    let joined = lines.join("\n\n");
    (user_async_delegate_truncate_chars(joined.trim(), 30000), count)
}

fn normalize_user_async_delegate_why(raw_why: &str) -> String {
    let trimmed = raw_why.trim();
    if trimmed.eq_ignore_ascii_case("请使用review skill")
        || trimmed.eq_ignore_ascii_case("请使用 review skill")
    {
        return String::new();
    }
    trimmed.to_string()
}

fn build_user_async_delegate_why(
    raw_why: &str,
    selected_context: &str,
) -> String {
    let mut parts = Vec::<String>::new();
    let raw_why = normalize_user_async_delegate_why(raw_why);
    if !raw_why.is_empty() {
        parts.push(format!("用户补充背景：\n{raw_why}"));
    }
    let selected_context = selected_context.trim();
    if !selected_context.is_empty() {
        parts.push(format!("当前会话选中消息纯文本：\n{selected_context}"));
    }
    parts.join("\n\n")
}

fn user_async_delegate_title(goal: &str, preset_id: Option<&str>) -> String {
    let prefix = match preset_id.map(str::trim).filter(|value| !value.is_empty()) {
        Some("review") => "审查委托",
        Some(_) => "异步委托",
        None => "异步委托",
    };
    let first_line = goal
        .trim()
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    let suffix = first_line.chars().take(24).collect::<String>();
    if suffix.trim().is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}：{suffix}")
    }
}

fn resolve_user_async_delegate_plan(
    app_state: &AppState,
    input: &SubmitUserAsyncDelegateInput,
) -> Result<(UserAsyncDelegatePlan, usize), String> {
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let target_department_id = input.target_department_id.trim();
    if target_department_id.is_empty() {
        return Err("targetDepartmentId is required".to_string());
    }
    let target_agent_id = input
        .target_agent_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "targetAgentId is required".to_string())?;
    let goal = delegate_arg_new_or_legacy(&input.goal, &input.question);
    if goal.trim().is_empty() {
        return Err("goal is required".to_string());
    }
    let todo = delegate_arg_new_or_legacy(&input.todo, &input.focus);
    let raw_why = delegate_arg_new_or_legacy(&input.why, &input.background);
    let selected_message_ids = input
        .selected_message_ids
        .iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();

    let runtime_org = load_runtime_organization_snapshot(app_state)?;
    let app_config = &runtime_org.config;
    let agents = &runtime_org.agents;
    let conversation = state_read_conversation_cached(app_state, conversation_id)
        .ok()
        .filter(|conversation| {
            conversation.summary.trim().is_empty()
                && conversation_visible_in_foreground_lists(conversation)
                && !conversation_is_delegate(conversation)
        })
        .ok_or_else(|| "当前会话不存在或已归档".to_string())?;
    let source_department_id = conversation.department_id.trim();
    let source_department = runtime_department_by_id(&runtime_org, source_department_id)
        .ok_or_else(|| format!("当前会话所属部门不存在，departmentId={source_department_id}"))?;
    let conversation_agent_id = conversation.agent_id.trim();
    let source_agent_id = if conversation_agent_id.is_empty() {
        first_available_department_agent(source_department, agents)
            .map(|agent| agent.id.clone())
            .ok_or_else(|| format!("当前会话所属部门没有可用人格，departmentId={source_department_id}"))?
    } else {
        if available_non_user_agent(agents, conversation_agent_id).is_none() {
            return Err(format!(
                "当前会话绑定人格不存在或不可用，agentId={conversation_agent_id}"
            ));
        }
        conversation_agent_id.to_string()
    };
    let target_department = runtime_department_by_id(&runtime_org, target_department_id)
        .ok_or_else(|| format!("目标部门不存在，departmentId={target_department_id}"))?;
    if !target_department
        .agent_ids
        .iter()
        .any(|id| id.trim() == target_agent_id)
    {
        return Err(format!(
            "目标人格不属于目标部门，departmentId={}，agentId={}",
            target_department_id, target_agent_id
        ));
    }
    if target_agent_id == source_agent_id {
        return Err(SAME_PERSONA_BACKGROUND_DELEGATE_REASON.to_string());
    }
    let target_agent = agents
        .iter()
        .find(|agent| agent.id == target_agent_id && !agent.is_built_in_user)
        .ok_or_else(|| format!("目标委任人不存在，agentId={target_agent_id}"))?;
    let target_api_config_ids = delegate_target_chat_api_config_ids(app_config, target_department);
    if target_api_config_ids.is_empty() {
        return Err(format!("目标部门没有可用模型，departmentId={target_department_id}"));
    }

    let (selected_context, selected_count) =
        build_user_async_delegate_selected_context(&conversation, agents, &selected_message_ids);
    let why = build_user_async_delegate_why(&raw_why, &selected_context);
    let title = user_async_delegate_title(&goal, input.preset_id.as_deref());
    Ok((
        UserAsyncDelegatePlan {
            root_conversation_id: conversation.id.clone(),
            source_department_id: source_department.id.clone(),
            source_agent_id,
            target_department_id: target_department.id.clone(),
            target_agent_id: target_agent_id.to_string(),
            target_agent_name: target_agent.name.trim().to_string(),
            title,
            goal,
            why,
            todo,
            target_api_config_ids,
        },
        selected_count,
    ))
}

fn enqueue_user_mention_result_message(
    app_state: &AppState,
    root_conversation_id: &str,
    source_agent_id: &str,
    target_department_id: &str,
    target_agent_id: &str,
    text: &str,
    provider_meta: Value,
) -> Result<(), String> {
    let delegate_message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "assistant".to_string(),
        created_at: now_iso(),
        speaker_agent_id: Some(target_agent_id.to_string()),
        parts: vec![MessagePart::Text {
            text: text.to_string(),
                reasoning_content: None,
            }],
        extra_text_blocks: Vec::new(),
        provider_meta: Some(provider_meta),
        tool_call: None,
        mcp_call: None,
        meme_annotations: None,
    };
    append_delegate_result_message_and_emit(
        app_state,
        root_conversation_id,
        &delegate_message,
        false,
        Some(ChatSessionInfo {
            department_id: target_department_id.to_string(),
            agent_id: source_agent_id.to_string(),
        }),
    )
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConversationMessageAppendedPayload {
    conversation_id: String,
    message: ChatMessage,
}

fn emit_conversation_message_appended_event(
    app_state: &AppState,
    conversation_id: &str,
    message: &ChatMessage,
) {
    let app_handle = match app_state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(err) => {
            eprintln!(
                "[聊天推送] append 消息事件发送失败：锁已损坏，conversation_id={}, error={:?}",
                conversation_id, err
            );
            None
        }
    };
    let Some(app_handle) = app_handle else {
        eprintln!(
            "[聊天推送] append 消息事件发送失败：app_handle 不可用，conversation_id={}",
            conversation_id
        );
        return;
    };
    let payload = ConversationMessageAppendedPayload {
        conversation_id: conversation_id.to_string(),
        message: message.clone(),
    };
    ide_chat_broadcast_notification("conversation.messageAppended", serde_json::json!(&payload));
    if let Err(err) = app_handle.emit(CHAT_CONVERSATION_MESSAGE_APPENDED_EVENT, payload) {
        eprintln!(
            "[聊天推送] append 消息事件发送失败：conversation_id={}, message_id={}, error={}",
            conversation_id,
            message.id,
            err
        );
    }
}

fn append_delegate_result_message_and_emit(
    app_state: &AppState,
    conversation_id: &str,
    message: &ChatMessage,
    continue_main_assistant: bool,
    session_info: Option<ChatSessionInfo>,
) -> Result<(), String> {
    let conversation_id = conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("缺少 conversation_id，无法写回委托结果".to_string());
    }
    conversation_service().append_message_to_unarchived_conversation(
        app_state,
        conversation_id,
        message,
    )?;
    emit_conversation_message_appended_event(app_state, conversation_id, message);

    if continue_main_assistant {
        let session_info = session_info.ok_or_else(|| "缺少 session_info，无法继续主助理".to_string())?;
        tauri::async_runtime::spawn({
            let state = app_state.clone();
            let conversation_id = conversation_id.to_string();
            async move {
                let oldest_queue_created_at = now_iso();
                let mut runtime_context = runtime_context_new("delegate_result", "delegate_continue");
                runtime_context.request_id = Some(format!("delegate-continue-{}", Uuid::new_v4()));
                if let Err(err) = activate_main_assistant(
                    &state,
                    &session_info,
                    &conversation_id,
                    None,
                    None,
                    Some(runtime_context),
                    Vec::new(),
                    &oldest_queue_created_at,
                )
                .await
                {
                    eprintln!(
                        "[委托结果] 追加后继续主助理失败: conversation_id={}, department_id={}, agent_id={}, error={}",
                        conversation_id,
                        session_info.department_id,
                        session_info.agent_id,
                        err
                    );
                }
            }
        });
    }

    Ok(())
}

fn spawn_user_mention_failure_message(app_state: AppState, failure: UserMentionFailurePlan) {
    tokio::spawn(async move {
        let text = format!("《用户@委托：{}》执行失败：{}", failure.target_agent_name.trim(), failure.reason.trim());
        if let Err(err) = enqueue_user_mention_result_message(
            &app_state,
            &failure.root_conversation_id,
            &failure.source_agent_id,
            &failure.target_department_id,
            &failure.target_agent_id,
            &text,
            serde_json::json!({
                "messageKind": "delegate_result",
                "delegateKind": DELEGATE_TOOL_KIND_USER_MENTION,
                "resultStatus": "failed",
                "speakerAgentId": failure.target_agent_id,
                "sourceAgentId": failure.source_agent_id,
                "targetAgentId": failure.target_agent_id,
                "error": failure.reason,
            }),
        ) {
            eprintln!(
                "[用户@委托] 写回失败结果消息失败: conversation_id={}, target_agent_id={}, error={}",
                failure.root_conversation_id,
                failure.target_agent_id,
                err
            );
        }
    });
}

fn spawn_user_async_delegate(app_state: AppState, plan: UserAsyncDelegatePlan) -> Result<String, String> {
    let delegate = delegate_create_record(
        &app_state,
        DELEGATE_TOOL_KIND_USER_MENTION,
        &plan.root_conversation_id,
        None,
        &plan.source_department_id,
        &plan.target_department_id,
        &plan.source_agent_id,
        &plan.target_agent_id,
        &plan.title,
        plan.why.clone(),
        plan.goal.clone(),
        plan.todo.clone(),
        false,
        vec![
            plan.source_department_id.clone(),
            plan.target_department_id.clone(),
        ],
    )?;
    let delegate_id = delegate.delegate_id.clone();
    tokio::spawn(async move {
        let target_agent_name = plan.target_agent_name.clone();
        let run_result = delegate_run_thread_to_completion(
            app_state.clone(),
            delegate.clone(),
            plan.target_api_config_ids.clone(),
            None,
        )
        .await;
        match run_result {
            Ok(run) => {
                let text = if run.assistant_text.trim().is_empty() {
                    format!("《{}》已处理完成。", delegate.title.trim())
                } else {
                    run.assistant_text.clone()
                };
                if let Err(err) = enqueue_user_mention_result_message(
                    &app_state,
                    &plan.root_conversation_id,
                    &plan.source_agent_id,
                    &plan.target_department_id,
                    &plan.target_agent_id,
                    &text,
                    serde_json::json!({
                        "messageKind": "delegate_result",
                        "delegateId": delegate.delegate_id,
                        "delegateKind": DELEGATE_TOOL_KIND_USER_MENTION,
                        "resultStatus": "completed",
                        "speakerAgentId": plan.target_agent_id,
                        "sourceAgentId": plan.source_agent_id,
                        "targetAgentId": plan.target_agent_id,
                    }),
                ) {
                    eprintln!(
                        "[用户异步委托] 写回完成结果消息失败: conversation_id={}, target_agent_id={}, error={}",
                        plan.root_conversation_id,
                        plan.target_agent_id,
                        err
                    );
                }
            }
            Err(err) => {
                let text = format!("《{}》执行失败：{}", delegate.title.trim(), err);
                if let Err(enqueue_err) = enqueue_user_mention_result_message(
                    &app_state,
                    &plan.root_conversation_id,
                    &plan.source_agent_id,
                    &plan.target_department_id,
                    &plan.target_agent_id,
                    &text,
                    serde_json::json!({
                        "messageKind": "delegate_result",
                        "delegateId": delegate.delegate_id,
                        "delegateKind": DELEGATE_TOOL_KIND_USER_MENTION,
                        "resultStatus": "failed",
                        "speakerAgentId": plan.target_agent_id,
                        "sourceAgentId": plan.source_agent_id,
                        "targetAgentId": plan.target_agent_id,
                        "error": err,
                    }),
                ) {
                    eprintln!(
                        "[用户异步委托] 写回失败结果消息失败: conversation_id={}, target_agent_id={}, error={}",
                        plan.root_conversation_id,
                        plan.target_agent_id,
                        enqueue_err
                    );
                }
            }
        }
        runtime_log_info(format!(
            "[用户异步委托] 后台执行 完成 delegate_id={} target_agent_name={}",
            delegate.delegate_id,
            target_agent_name
        ));
    });
    Ok(delegate_id)
}

fn spawn_user_mention_delegate(app_state: AppState, plan: UserMentionPlan) {
    tokio::spawn(async move {
        let delegate = match delegate_create_record(
            &app_state,
            DELEGATE_TOOL_KIND_USER_MENTION,
            &plan.root_conversation_id,
            None,
            &plan.source_department_id,
            &plan.target_department_id,
            &plan.source_agent_id,
            &plan.target_agent_id,
            &format!("用户@委托：{}", plan.target_agent_name.trim()),
            plan.background.clone(),
            plan.instruction.clone(),
            "请直接基于当前上下文作答，不要复述委托框架。".to_string(),
            false,
            vec![
                plan.source_department_id.clone(),
                plan.target_department_id.clone(),
            ],
        ) {
            Ok(value) => value,
            Err(err) => {
                spawn_user_mention_failure_message(
                    app_state,
                    UserMentionFailurePlan {
                        root_conversation_id: plan.root_conversation_id,
                        source_agent_id: plan.source_agent_id,
                        target_department_id: plan.target_department_id,
                        target_agent_id: plan.target_agent_id,
                        target_agent_name: plan.target_agent_name,
                        reason: err,
                    },
                );
                return;
            }
        };
        let target_agent_name = plan.target_agent_name.clone();
        let run_result = delegate_run_thread_to_completion(
            app_state.clone(),
            delegate.clone(),
            plan.target_api_config_ids.clone(),
            None,
        )
        .await;
        match run_result {
            Ok(run) => {
                let text = if run.assistant_text.trim().is_empty() {
                    format!("《用户@委托：{}》已处理完成。", target_agent_name.trim())
                } else {
                    run.assistant_text.clone()
                };
                if let Err(err) = enqueue_user_mention_result_message(
                    &app_state,
                    &plan.root_conversation_id,
                    &plan.source_agent_id,
                    &plan.target_department_id,
                    &plan.target_agent_id,
                    &text,
                    serde_json::json!({
                        "messageKind": "delegate_result",
                        "delegateId": delegate.delegate_id,
                        "delegateKind": DELEGATE_TOOL_KIND_USER_MENTION,
                        "resultStatus": "completed",
                        "speakerAgentId": plan.target_agent_id,
                        "sourceAgentId": plan.source_agent_id,
                        "targetAgentId": plan.target_agent_id,
                    }),
                ) {
                    eprintln!(
                        "[用户@委托] 写回完成结果消息失败: conversation_id={}, target_agent_id={}, error={}",
                        plan.root_conversation_id,
                        plan.target_agent_id,
                        err
                    );
                }
            }
            Err(err) => {
                let fail_text = format!("《用户@委托：{}》执行失败：{}", target_agent_name.trim(), err.trim());
                if let Err(enqueue_err) = enqueue_user_mention_result_message(
                    &app_state,
                    &plan.root_conversation_id,
                    &plan.source_agent_id,
                    &plan.target_department_id,
                    &plan.target_agent_id,
                    &fail_text,
                    serde_json::json!({
                        "messageKind": "delegate_result",
                        "delegateId": delegate.delegate_id,
                        "delegateKind": DELEGATE_TOOL_KIND_USER_MENTION,
                        "resultStatus": "failed",
                        "speakerAgentId": plan.target_agent_id,
                        "sourceAgentId": plan.source_agent_id,
                        "targetAgentId": plan.target_agent_id,
                        "error": err,
                    }),
                ) {
                    eprintln!(
                        "[用户@委托] 写回失败结果消息失败: conversation_id={}, target_agent_id={}, error={}",
                        plan.root_conversation_id,
                        plan.target_agent_id,
                        enqueue_err
                    );
                }
            }
        }
    });
}

fn spawn_user_mention_after_message_flushed(
    app_state: AppState,
    event_id: String,
    result_rx: tokio::sync::oneshot::Receiver<Result<SendChatResult, String>>,
    mention_failures: Vec<UserMentionFailurePlan>,
    mention_plans: Vec<UserMentionPlan>,
) {
    tokio::spawn(async move {
        match result_rx.await {
            Ok(Ok(_)) => {
                for failure in mention_failures {
                    spawn_user_mention_failure_message(app_state.clone(), failure);
                }
                for plan in mention_plans {
                    spawn_user_mention_delegate(app_state.clone(), plan);
                }
            }
            Ok(Err(err)) => {
                eprintln!(
                    "[用户@委托] 用户消息落库前调度失败，跳过委托: event_id={}, error={}",
                    event_id, err
                );
            }
            Err(_) => {
                eprintln!(
                    "[用户@委托] 用户消息落库结果丢失，跳过委托: event_id={}",
                    event_id
                );
            }
        }
    });
}

const ACCEPTED_SUBMIT_TRACE_ID_LIMIT: usize = 5000;

fn claim_submit_trace_id(state: &AppState, trace_id: &str) -> Result<bool, String> {
    let trace_id = trace_id.trim();
    if trace_id.is_empty() {
        return Ok(true);
    }
    let mut accepted = state
        .accepted_submit_trace_ids
        .lock()
        .map_err(|_| "Failed to lock accepted submit trace ids".to_string())?;
    if accepted.iter().any(|value| value == trace_id) {
        return Ok(false);
    }
    accepted.push_back(trace_id.to_string());
    while accepted.len() > ACCEPTED_SUBMIT_TRACE_ID_LIMIT {
        accepted.pop_front();
    }
    Ok(true)
}

fn release_submit_trace_id(state: &AppState, trace_id: &str) {
    let trace_id = trace_id.trim();
    if trace_id.is_empty() {
        return;
    }
    if let Ok(mut accepted) = state.accepted_submit_trace_ids.lock() {
        if let Some(index) = accepted.iter().position(|value| value == trace_id) {
            accepted.remove(index);
        }
    }
}

async fn submit_chat_message_inner(
    input: SendChatRequest,
    state: &AppState,
    on_delta: tauri::ipc::Channel<AssistantDeltaEvent>,
) -> Result<SubmitChatResult, String> {
    if input.trigger_only {
        return Err("submit_chat_message 不支持 trigger_only".to_string());
    }

    let text = input.payload.text.as_deref().unwrap_or("").trim();
    let images = input.payload.images.as_ref().map(|v| v.as_slice()).unwrap_or(&[]);
    let attachments = input
        .payload
        .attachments
        .as_ref()
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    if text.is_empty() && images.is_empty() && attachments.is_empty() {
        return Err("消息内容为空".to_string());
    }

    let display_text = input
        .payload
        .display_text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(text)
        .to_string();
    let normalized_mentions = normalize_payload_mentions(input.payload.mentions.as_ref());
    let has_user_mentions = !normalized_mentions.is_empty();

    let mut message_parts = Vec::new();
    if !text.is_empty() {
        message_parts.push(MessagePart::Text { text: text.to_string(),
                reasoning_content: None,
            });
    }
    for img in images {
        message_parts.push(MessagePart::Image {
            mime: img.mime.clone(),
            bytes_base64: img.bytes_base64.clone(),
            name: None,
            compressed: false,
        });
    }

    let request_id = runtime_context_request_id_or_new(None, input.trace_id.as_deref(), "chat");
    if !claim_submit_trace_id(state, &request_id)? {
        let session = input.session.as_ref().ok_or_else(|| "缺少会话信息".to_string())?;
        let conversation_id = session
            .conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("")
            .to_string();
        return Ok(SubmitChatResult {
            accepted: false,
            duplicate: true,
            event_id: String::new(),
            conversation_id,
            trace_id: request_id,
            ingress: "duplicate".to_string(),
        });
    }
    let user_message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "user".to_string(),
        created_at: now_iso(),
        speaker_agent_id: None,
        parts: message_parts,
        extra_text_blocks: input.payload.extra_text_blocks.clone().unwrap_or_default(),
        provider_meta: {
            let mut attachment_entries =
                normalize_payload_attachments(input.payload.attachments.as_ref());
            attachment_entries.extend(normalize_payload_image_attachments(
                input.payload.images.as_ref(),
            ));
            build_user_message_provider_meta(
                input.payload.provider_meta.clone(),
                &attachment_entries,
                &normalized_mentions,
                Some(request_id.as_str()),
            )
        },
        tool_call: None,
        mcp_call: None,
        meme_annotations: None,
    };

    let session = input.session.as_ref().ok_or_else(|| "缺少会话信息".to_string())?;
    let requested_department_id = session
        .department_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            eprintln!("[聊天发送] 缺少 department_id，拒绝提交用户消息");
            "缺少 department_id".to_string()
        })?;
    let conversation_id = session
        .conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            eprintln!("[聊天发送] 缺少 conversation_id，拒绝提交用户消息");
            "缺少 conversation_id".to_string()
        })?;

    let prepare_started_at = std::time::Instant::now();
    let (department_id, agent_id, model_config_id, mention_plans, mention_failures) = {
        let config_started_at = std::time::Instant::now();
        let runtime_org = load_runtime_organization_snapshot(state)?;
        let app_config = runtime_org.config.clone();
        let config_elapsed_ms = config_started_at.elapsed().as_millis();
        let agents = runtime_org.agents.clone();
        let app_data_elapsed_ms = 0u128;
        let department_started_at = std::time::Instant::now();
        let department = runtime_department_by_id(&runtime_org, requested_department_id.as_str())
            .ok_or_else(|| format!("部门已经消失：{}", requested_department_id))?;
        let agent_id = session.agent_id.trim().to_string();
        if agent_id.is_empty() {
            return Err("缺少执行人格：session.agentId 为空".to_string());
        }
        if !agents
            .iter()
            .any(|agent| agent.id == agent_id && !agent.is_built_in_user)
        {
            return Err(format!("执行人格不存在或不可用: agentId={agent_id}"));
        }
        let department_elapsed_ms = department_started_at.elapsed().as_millis();
        let api_config_id = department_primary_chat_api_config_id(&app_config, department)
            .ok_or_else(|| format!("部门模型未配置或不可用于聊天: {}", department.id))?;

        let conversation_started_at = std::time::Instant::now();
        let conversation = state_read_conversation_cached(state, &conversation_id)?;
        let conversation_elapsed_ms = conversation_started_at.elapsed().as_millis();
        let (mention_plans, mention_failures) = build_user_mention_dispatch_plans(
            &app_config,
            &conversation,
            &agents,
            &department.id,
            &agent_id,
            &display_text,
            input.payload.mentions.as_ref(),
        )?;

        eprintln!(
            "[聊天发送] 提交前准备耗时：总计={}ms，读取配置={}ms，读取应用数据={}ms，解析部门={}ms，会话解析={}ms，conversation_id={}，department_id={}，agent_id={}",
            prepare_started_at.elapsed().as_millis(),
            config_elapsed_ms,
            app_data_elapsed_ms,
            department_elapsed_ms,
            conversation_elapsed_ms,
            conversation_id,
            department.id,
            agent_id
        );

        (
            department.id.clone(),
            agent_id,
            api_config_id,
            mention_plans,
            mention_failures,
        )
    };

    let event_id = Uuid::new_v4().to_string();
    let mut runtime_context = runtime_context_new(
        "user_message",
        if has_user_mentions { "user_mention_send" } else { "user_send" },
    );
    runtime_context.request_id = Some(request_id.clone());
    runtime_context.dispatch_id = Some(event_id.clone());
    runtime_context.origin_conversation_id = Some(conversation_id.clone());
    runtime_context.target_conversation_id = Some(conversation_id.clone());
    runtime_context.root_conversation_id = Some(conversation_id.clone());
    runtime_context.executor_agent_id = Some(agent_id.clone());
    runtime_context.executor_department_id = Some(department_id.clone());
    runtime_context.model_config_id = Some(model_config_id.clone());
    let event = ChatPendingEvent {
        id: event_id.clone(),
        conversation_id: conversation_id.clone(),
        created_at: now_iso(),
        source: ChatEventSource::User,
        queue_mode: ChatQueueMode::Normal,
        messages: vec![user_message],
        activate_assistant: !has_user_mentions,
        session_info: ChatSessionInfo {
            department_id: department_id.clone(),
            agent_id: agent_id.clone(),
        },
        runtime_context: Some(runtime_context),
        sender_info: None,
    };

    let mention_result_rx = if has_user_mentions {
        let (result_tx, result_rx) = tokio::sync::oneshot::channel();
        register_chat_event_runtime(state, &event_id, on_delta, result_tx)?;
        Some(result_rx)
    } else {
        register_chat_event_delta_channel(state, &event_id, on_delta)?;
        None
    };

    let ingress = match ingress_chat_event(state, event) {
        Ok(value) => value,
        Err(err) => {
            let _ = state
                .pending_chat_delta_channels
                .lock()
                .map(|mut map| map.remove(&event_id));
            let _ = state
                .pending_chat_result_senders
                .lock()
                .map(|mut map| map.remove(&event_id));
            release_submit_trace_id(state, &request_id);
            return Err(err);
        }
    };

    let (accepted, duplicate, ingress_label) = match &ingress {
        ChatEventIngress::Direct(_) => (true, false, "direct"),
        ChatEventIngress::Queued { .. } => (true, false, "queued"),
        ChatEventIngress::Duplicate { .. } => (false, true, "duplicate"),
    };

    trigger_chat_event_after_ingress(state, ingress);

    if accepted {
        if let Some(result_rx) = mention_result_rx {
            spawn_user_mention_after_message_flushed(
                state.clone(),
                event_id.clone(),
                result_rx,
                mention_failures,
                mention_plans,
            );
        }
    }

    Ok(SubmitChatResult {
        accepted,
        duplicate,
        event_id,
        conversation_id,
        trace_id: request_id,
        ingress: ingress_label.to_string(),
    })
}

#[tauri::command]
async fn submit_chat_message(
    input: SendChatRequest,
    state: State<'_, AppState>,
    on_delta: tauri::ipc::Channel<AssistantDeltaEvent>,
) -> Result<SubmitChatResult, String> {
    submit_chat_message_inner(input, state.inner(), on_delta).await
}

#[tauri::command]
async fn send_chat_message(
    input: SendChatRequest,
    state: State<'_, AppState>,
    on_delta: tauri::ipc::Channel<AssistantDeltaEvent>,
) -> Result<SendChatResult, String> {
    if input
        .payload
        .mentions
        .as_ref()
        .map(|items| !items.is_empty())
        .unwrap_or(false)
    {
        return send_user_mention_message_inner(input, state.inner(), &on_delta).await;
    }

    // 如果是 trigger_only 模式（由调度器调用），直接执行
    if input.trigger_only {
        return send_chat_message_inner(input, state.inner(), &on_delta).await;
    }

    // 用户发言：构造消息并入队
    let text = input.payload.text.as_deref().unwrap_or("").trim();
    let images = input.payload.images.as_ref().map(|v| v.as_slice()).unwrap_or(&[]);
    let attachments = input
        .payload
        .attachments
        .as_ref()
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    if text.is_empty() && images.is_empty() && attachments.is_empty() {
        return Err("消息内容为空".to_string());
    }

    let display_text = input
        .payload
        .display_text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(text)
        .to_string();
    let normalized_mentions = normalize_payload_mentions(input.payload.mentions.as_ref());

    let mut message_parts = Vec::new();
    if !text.is_empty() {
        message_parts.push(MessagePart::Text { text: text.to_string(),
                reasoning_content: None,
            });
    }
    for img in images {
        message_parts.push(MessagePart::Image {
            mime: img.mime.clone(),
            bytes_base64: img.bytes_base64.clone(),
            name: None,
            compressed: false,
        });
    }
    // 先确定 requestId，再写入用户消息 provider_meta，保证重复发送可按已落地消息幂等识别。
    let request_id = runtime_context_request_id_or_new(None, input.trace_id.as_deref(), "chat");
    let user_message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "user".to_string(),
        created_at: now_iso(),
        speaker_agent_id: None,
        parts: message_parts,
        extra_text_blocks: input.payload.extra_text_blocks.clone().unwrap_or_default(),
        provider_meta: {
            let mut attachment_entries =
                normalize_payload_attachments(input.payload.attachments.as_ref());
            attachment_entries.extend(normalize_payload_image_attachments(
                input.payload.images.as_ref(),
            ));
            build_user_message_provider_meta(
                input.payload.provider_meta.clone(),
                &attachment_entries,
                &normalized_mentions,
                Some(request_id.as_str()),
            )
        },
        tool_call: None,
        mcp_call: None,
        meme_annotations: None,
    };

    // 获取会话信息
    let session = input.session.as_ref().ok_or_else(|| "缺少会话信息".to_string())?;
    let requested_department_id = session
        .department_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            eprintln!("[聊天发送] 缺少 department_id，拒绝发送用户消息");
            "缺少 department_id".to_string()
        })?;
    let conversation_id = session
        .conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            eprintln!("[聊天发送] 缺少 conversation_id，拒绝发送用户消息");
            "缺少 conversation_id".to_string()
        })?;

    let prepare_started_at = std::time::Instant::now();
    let (department_id, agent_id, model_config_id, mention_plans, mention_failures) = {
        let config_started_at = std::time::Instant::now();
        let runtime_org = load_runtime_organization_snapshot(&state)?;
        let app_config = runtime_org.config.clone();
        let config_elapsed_ms = config_started_at.elapsed().as_millis();
        let agents = runtime_org.agents.clone();
        let app_data_elapsed_ms = 0u128;
        let department_started_at = std::time::Instant::now();
        let department = runtime_department_by_id(&runtime_org, requested_department_id.as_str())
            .ok_or_else(|| format!("部门已经消失：{}", requested_department_id))?;
        let agent_id = session.agent_id.trim().to_string();
        if agent_id.is_empty() {
            return Err("缺少执行人格：session.agentId 为空".to_string());
        }
        if !agents
            .iter()
            .any(|agent| agent.id == agent_id && !agent.is_built_in_user)
        {
            return Err(format!("执行人格不存在或不可用: agentId={agent_id}"));
        }
        let department_elapsed_ms = department_started_at.elapsed().as_millis();
        let api_config_id = department_primary_chat_api_config_id(&app_config, department)
            .ok_or_else(|| format!("部门模型未配置或不可用于聊天: {}", department.id))?;

        let conversation_started_at = std::time::Instant::now();
        let conversation = {
            state_read_conversation_cached(&state, &conversation_id)?
        };
        let conversation_elapsed_ms = conversation_started_at.elapsed().as_millis();
        let (mention_plans, mention_failures) = build_user_mention_dispatch_plans(
            &app_config,
            &conversation,
            &agents,
            &department.id,
            &agent_id,
            &display_text,
            input.payload.mentions.as_ref(),
        )?;

        eprintln!(
            "[聊天发送] 发送前准备耗时：总计={}ms，读取配置={}ms，读取应用数据={}ms，解析部门={}ms，会话解析={}ms，conversation_id={}，department_id={}，agent_id={}",
            prepare_started_at.elapsed().as_millis(),
            config_elapsed_ms,
            app_data_elapsed_ms,
            department_elapsed_ms,
            conversation_elapsed_ms,
            conversation_id,
            department.id,
            agent_id
        );

        (
            department.id.clone(),
            agent_id,
            api_config_id,
            mention_plans,
            mention_failures,
        )
    };

    // 构造队列事件
    let event_id = Uuid::new_v4().to_string();
    let has_user_mentions = input
        .payload
        .mentions
        .as_ref()
        .map(|items| !items.is_empty())
        .unwrap_or(false);
    let mut runtime_context = runtime_context_new("user_message", "user_send");
    runtime_context.request_id = Some(request_id.clone());
    runtime_context.dispatch_id = Some(event_id.clone());
    runtime_context.origin_conversation_id = Some(conversation_id.clone());
    runtime_context.target_conversation_id = Some(conversation_id.clone());
    runtime_context.root_conversation_id = Some(conversation_id.clone());
    runtime_context.executor_agent_id = Some(agent_id.clone());
    runtime_context.executor_department_id = Some(department_id.clone());
    runtime_context.model_config_id = Some(model_config_id.clone());
    let event = ChatPendingEvent {
        id: event_id.clone(),
        conversation_id: conversation_id.clone(),
        created_at: now_iso(),
        source: ChatEventSource::User,
        queue_mode: ChatQueueMode::Normal,
        messages: vec![user_message],
        activate_assistant: !has_user_mentions,
        session_info: ChatSessionInfo {
            department_id: department_id.clone(),
            agent_id: agent_id.clone(),
        },
        runtime_context: Some(runtime_context.clone()),
        sender_info: None,
    };

    let (result_tx, result_rx) = tokio::sync::oneshot::channel();
    register_chat_event_runtime(state.inner(), &event_id, on_delta.clone(), result_tx)?;

    // 入队前先做阻塞判定：空闲且无排队则直写历史；否则入队。
    let ingress = match ingress_chat_event(state.inner(), event) {
        Ok(value) => value,
        Err(err) => {
            let _ = state
                .pending_chat_delta_channels
                .lock()
                .map(|mut map| map.remove(&event_id));
            let _ = state
                .pending_chat_result_senders
                .lock()
                .map(|mut map| map.remove(&event_id));
            return Err(err);
        }
    };

    // 根据 ingress 结果执行：直写或排队；排队仅在事件仍滞留时才通知前端。
    trigger_chat_event_after_ingress(state.inner(), ingress);

    let send_result = result_rx
        .await
        .map_err(|_| "聊天请求已取消或调度结果丢失".to_string())?;
    let send_result = send_result?;

    for failure in mention_failures {
        spawn_user_mention_failure_message(state.inner().clone(), failure);
    }
    for plan in mention_plans {
        spawn_user_mention_delegate(state.inner().clone(), plan);
    }

    Ok(send_result)
}

async fn send_user_mention_message_inner(
    input: SendChatRequest,
    state: &AppState,
    on_delta: &tauri::ipc::Channel<AssistantDeltaEvent>,
) -> Result<SendChatResult, String> {
    let mention_count = input
        .payload
        .mentions
        .as_ref()
        .map(|items| items.iter().filter(|item| !item.agent_id.trim().is_empty()).count())
        .unwrap_or(0);
    if mention_count == 0 {
        return Err("缺少有效的@目标".to_string());
    }

    let text = input.payload.text.as_deref().unwrap_or("").trim();
    let images = input.payload.images.as_ref().map(|v| v.as_slice()).unwrap_or(&[]);
    let attachments = input
        .payload
        .attachments
        .as_ref()
        .map(|v| v.as_slice())
        .unwrap_or(&[]);

    if text.is_empty() && images.is_empty() && attachments.is_empty() {
        return Err("消息内容为空".to_string());
    }

    let display_text = input
        .payload
        .display_text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(text)
        .to_string();
    let normalized_mentions = normalize_payload_mentions(input.payload.mentions.as_ref());

    let mut message_parts = Vec::new();
    if !text.is_empty() {
        message_parts.push(MessagePart::Text { text: text.to_string(),
                reasoning_content: None,
            });
    }
    for img in images {
        message_parts.push(MessagePart::Image {
            mime: img.mime.clone(),
            bytes_base64: img.bytes_base64.clone(),
            name: None,
            compressed: false,
        });
    }
    // 先确定 requestId，再写入用户消息 provider_meta，保证重复发送可按已落地消息幂等识别。
    let request_id = runtime_context_request_id_or_new(None, input.trace_id.as_deref(), "chat");
    let user_message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "user".to_string(),
        created_at: now_iso(),
        speaker_agent_id: None,
        parts: message_parts,
        extra_text_blocks: input.payload.extra_text_blocks.clone().unwrap_or_default(),
        provider_meta: {
            let mut attachment_entries =
                normalize_payload_attachments(input.payload.attachments.as_ref());
            attachment_entries.extend(normalize_payload_image_attachments(
                input.payload.images.as_ref(),
            ));
            build_user_message_provider_meta(
                input.payload.provider_meta.clone(),
                &attachment_entries,
                &normalized_mentions,
                Some(request_id.as_str()),
            )
        },
        tool_call: None,
        mcp_call: None,
        meme_annotations: None,
    };

    let session = input.session.as_ref().ok_or_else(|| "缺少会话信息".to_string())?;
    let requested_department_id = session
        .department_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            eprintln!("[聊天发送] 缺少 department_id，拒绝发送用户@委托消息");
            "缺少 department_id".to_string()
        })?;
    let conversation_id = session
        .conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            eprintln!("[聊天发送] 缺少 conversation_id，拒绝发送用户@委托消息");
            "缺少 conversation_id".to_string()
        })?;

    let prepare_started_at = std::time::Instant::now();
    let (department_id, agent_id, model_config_id, mention_plans, mention_failures) = {
        let config_started_at = std::time::Instant::now();
        let runtime_org = load_runtime_organization_snapshot(state)?;
        let app_config = runtime_org.config.clone();
        let config_elapsed_ms = config_started_at.elapsed().as_millis();
        let agents = runtime_org.agents.clone();
        let app_data_elapsed_ms = 0u128;
        let department_started_at = std::time::Instant::now();
        let department = runtime_department_by_id(&runtime_org, requested_department_id.as_str())
            .ok_or_else(|| format!("部门已经消失：{}", requested_department_id))?;
        let agent_id = session.agent_id.trim().to_string();
        if agent_id.is_empty() {
            return Err("缺少执行人格：session.agentId 为空".to_string());
        }
        if !agents
            .iter()
            .any(|agent| agent.id == agent_id && !agent.is_built_in_user)
        {
            return Err(format!("执行人格不存在或不可用: agentId={agent_id}"));
        }
        let department_elapsed_ms = department_started_at.elapsed().as_millis();
        let api_config_id = department_primary_chat_api_config_id(&app_config, department)
            .ok_or_else(|| format!("部门模型未配置或不可用于聊天: {}", department.id))?;

        let conversation_started_at = std::time::Instant::now();
        let conversation = {
            state_read_conversation_cached(state, &conversation_id)?
        };
        let conversation_elapsed_ms = conversation_started_at.elapsed().as_millis();
        let (mention_plans, mention_failures) = build_user_mention_dispatch_plans(
            &app_config,
            &conversation,
            &agents,
            &department.id,
            &agent_id,
            &display_text,
            input.payload.mentions.as_ref(),
        )?;

        eprintln!(
            "[聊天发送] 用户@委托发送前准备耗时：总计={}ms，读取配置={}ms，读取应用数据={}ms，解析部门={}ms，会话解析={}ms，conversation_id={}，department_id={}，agent_id={}，mention_count={}",
            prepare_started_at.elapsed().as_millis(),
            config_elapsed_ms,
            app_data_elapsed_ms,
            department_elapsed_ms,
            conversation_elapsed_ms,
            conversation_id,
            department.id,
            agent_id,
            mention_count
        );

        (
            department.id.clone(),
            agent_id,
            api_config_id,
            mention_plans,
            mention_failures,
        )
    };

    let event_id = Uuid::new_v4().to_string();
    let mut runtime_context = runtime_context_new("user_message", "user_mention_send");
    runtime_context.request_id = Some(request_id.clone());
    runtime_context.dispatch_id = Some(event_id.clone());
    runtime_context.origin_conversation_id = Some(conversation_id.clone());
    runtime_context.target_conversation_id = Some(conversation_id.clone());
    runtime_context.root_conversation_id = Some(conversation_id.clone());
    runtime_context.executor_agent_id = Some(agent_id.clone());
    runtime_context.executor_department_id = Some(department_id.clone());
    runtime_context.model_config_id = Some(model_config_id.clone());
    let event = ChatPendingEvent {
        id: event_id.clone(),
        conversation_id: conversation_id.clone(),
        created_at: now_iso(),
        source: ChatEventSource::User,
        queue_mode: ChatQueueMode::Normal,
        messages: vec![user_message],
        activate_assistant: false,
        session_info: ChatSessionInfo {
            department_id: department_id.clone(),
            agent_id: agent_id.clone(),
        },
        runtime_context: Some(runtime_context.clone()),
        sender_info: None,
    };

    let (result_tx, result_rx) = tokio::sync::oneshot::channel();
    register_chat_event_runtime(state, &event_id, on_delta.clone(), result_tx)?;

    let ingress = match ingress_chat_event(state, event) {
        Ok(value) => value,
        Err(err) => {
            let _ = state
                .pending_chat_delta_channels
                .lock()
                .map(|mut map| map.remove(&event_id));
            let _ = state
                .pending_chat_result_senders
                .lock()
                .map(|mut map| map.remove(&event_id));
            return Err(err);
        }
    };

    trigger_chat_event_after_ingress(state, ingress);

    let send_result = result_rx
        .await
        .map_err(|_| "聊天请求已取消或调度结果丢失".to_string())?;
    let send_result = send_result?;

    for failure in mention_failures {
        spawn_user_mention_failure_message(state.clone(), failure);
    }
    for plan in mention_plans {
        spawn_user_mention_delegate(state.clone(), plan);
    }

    Ok(send_result)
}

#[tauri::command]
async fn send_user_mention_message(
    input: SendChatRequest,
    state: State<'_, AppState>,
    on_delta: tauri::ipc::Channel<AssistantDeltaEvent>,
) -> Result<SendChatResult, String> {
    send_user_mention_message_inner(input, state.inner(), &on_delta).await
}

#[tauri::command]
async fn submit_user_async_delegate(
    input: SubmitUserAsyncDelegateInput,
    state: State<'_, AppState>,
) -> Result<SubmitUserAsyncDelegateOutput, String> {
    submit_user_async_delegate_internal(input, state.inner()).await
}

async fn submit_user_async_delegate_internal(
    input: SubmitUserAsyncDelegateInput,
    state: &AppState,
) -> Result<SubmitUserAsyncDelegateOutput, String> {
    let (plan, selected_message_count) = resolve_user_async_delegate_plan(state, &input)?;
    let output = SubmitUserAsyncDelegateOutput {
        delegate_id: String::new(),
        conversation_id: plan.root_conversation_id.clone(),
        target_agent_id: plan.target_agent_id.clone(),
        target_agent_name: plan.target_agent_name.clone(),
        selected_message_count,
    };
    let delegate_id = spawn_user_async_delegate(state.clone(), plan)?;
    runtime_log_info(format!(
        "[用户异步委托] 发起 完成 conversation_id={} delegate_id={} target_agent_id={} selected_message_count={}",
        output.conversation_id,
        delegate_id,
        output.target_agent_id,
        output.selected_message_count
    ));
    Ok(SubmitUserAsyncDelegateOutput {
        delegate_id,
        ..output
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BindActiveChatViewStreamInput {
    #[serde(default)]
    conversation_id: Option<String>,
}

#[tauri::command]
async fn bind_active_chat_view_stream(
    input: BindActiveChatViewStreamInput,
    state: State<'_, AppState>,
    window: tauri::Window,
    on_delta: tauri::ipc::Channel<AssistantDeltaEvent>,
) -> Result<(), String> {
    let window_label = window.label().to_string();
    let conversation_id = input
        .conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(conversation_id) = conversation_id {
        set_active_chat_view_stream_binding(
            state.inner(),
            &window_label,
            Some(conversation_id),
            on_delta.clone(),
        )?;
        runtime_log_debug(format!(
            "[聊天] 已绑定活动聊天流: window={}, conversation_id={}",
            window_label, conversation_id
        ));
    } else {
        // 空会话视图仍保留绑定，作为单窗口通配接收端，避免远程消息落地后前端无推送。
        set_active_chat_view_stream_binding(
            state.inner(),
            &window_label,
            Some("*"),
            on_delta,
        )?;
    }
    Ok(())
}

#[tauri::command]
async fn stop_chat_message(
    input: StopChatRequest,
    state: State<'_, AppState>,
) -> Result<StopChatResult, String> {
    let requested_conversation_id = input
        .session
        .conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let requested_department_id = input
        .session
        .department_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let (department_id, _agent_id) = resolve_runtime_control_department_and_agent(
        state.inner(),
        requested_department_id.as_deref(),
        Some(input.session.agent_id.as_str()),
        requested_conversation_id.as_deref(),
    )?;

    let chat_key = inflight_chat_key(
        &department_id,
        requested_conversation_id.as_deref(),
    );
    let aborted_chat = {
        let mut inflight = state
            .inflight_chat_abort_handles
            .lock()
            .map_err(|_| "Failed to lock inflight chat abort handles".to_string())?;
        if let Some(handle) = inflight.remove(&chat_key) {
            handle.abort();
            true
        } else {
            false
        }
    };
    let aborted_tool = abort_inflight_tool_abort_handle(state.inner(), &chat_key)?;
    let aborted_delegate_children =
        abort_delegate_runtime_descendants_by_parent_context(
            state.inner(),
            &chat_key,
            requested_conversation_id.as_deref(),
        )?;
    let aborted = aborted_chat || aborted_tool || aborted_delegate_children > 0;
    if aborted {
        if let Some(conversation_id) = requested_conversation_id.as_deref() {
            mark_goal_continue_suppressed_by_user_interrupt(
                state.inner(),
                conversation_id,
                "stop_chat_message",
            )?;
        }
    }
    if aborted_delegate_children > 0 {
        eprintln!(
            "[聊天] 停止请求已级联到同步委托子会话: session={}, child_count={}",
            chat_key,
            aborted_delegate_children
        );
    }

    let partial_stream_text = assistant_text_from_stream_blocks(&input.partial_stream_blocks);
    let partial_assistant_text = input.partial_assistant_text.trim().to_string();
    let partial_assistant_text = if partial_assistant_text.is_empty() {
        partial_stream_text.trim().to_string()
    } else {
        partial_assistant_text
    };
    let partial_activity_text = reasoning_text_from_stream_blocks(&input.partial_stream_blocks);
    let completed_tool_history = inflight_completed_tool_history(state.inner(), &chat_key)?;
    let partial_tool_history =
        merge_stream_block_tool_history(&completed_tool_history, &input.partial_stream_blocks);
    let build_stop_result =
        |conversation_id: Option<String>| -> StopChatResult {
            StopChatResult {
                aborted,
                persisted: false,
                conversation_id,
                assistant_text: partial_assistant_text.clone(),
                assistant_message: None,
            }
        };
    runtime_log_info(format!(
        "[聊天流式块][后端停止] 停止请求完成 session={} conversation_id={} aborted={} partial_text_len={} partial_reasoning_len={} partial_block_count={} partial_tool_history_count={} completed_tool_history_count={}",
        chat_key,
        requested_conversation_id.as_deref().unwrap_or(""),
        aborted,
        partial_assistant_text.chars().count(),
        partial_activity_text.chars().count(),
        input.partial_stream_blocks.len(),
        partial_tool_history.len(),
        completed_tool_history.len(),
    ));
    clear_inflight_completed_tool_history(state.inner(), &chat_key)?;
    Ok(build_stop_result(requested_conversation_id))
}

#[tauri::command]
async fn get_chat_queue_snapshot(
    state: State<'_, AppState>,
) -> Result<Vec<ChatQueueEventSummary>, String> {
    get_queue_snapshot(state.inner())
}

#[tauri::command]
async fn recall_chat_queue_event(
    event_id: String,
    state: State<'_, AppState>,
) -> Result<ChatQueueRecallResult, String> {
    let removed = recall_queue_event(state.inner(), &event_id)?;
    let message_text = removed
        .as_ref()
        .and_then(|event| {
            event.messages.first().and_then(|msg| {
                msg.parts.iter().find_map(|part| match part {
                    MessagePart::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
            })
        })
        .unwrap_or_default();
    Ok(ChatQueueRecallResult {
        removed: removed.is_some(),
        message_text,
    })
}

#[tauri::command]
async fn mark_chat_queue_event_guided(
    event_id: String,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let conversation_id = mark_queue_event_guided(state.inner(), &event_id)?;
    if let Some(conversation_id) = conversation_id {
        trigger_guided_queue_processing(state.inner(), &conversation_id);
        return Ok(true);
    }
    Ok(false)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InterruptConversationRuntimeResult {
    aborted: bool,
    cleared_queue_count: usize,
}

#[tauri::command]
async fn interrupt_conversation_runtime(
    session: SessionSelector,
    state: State<'_, AppState>,
) -> Result<InterruptConversationRuntimeResult, String> {
    let conversation_id = session
        .conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| "Missing session.conversationId".to_string())?;
    let requested_department_id = session
        .department_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let (department_id, _) = resolve_runtime_control_department_and_agent(
        state.inner(),
        requested_department_id,
        Some(session.agent_id.as_str()),
        Some(&conversation_id),
    )?;

    let chat_key = inflight_chat_key(&department_id, Some(&conversation_id));
    let aborted_chat = {
        let mut inflight = state
            .inflight_chat_abort_handles
            .lock()
            .map_err(|_| "Failed to lock inflight chat abort handles".to_string())?;
        if let Some(handle) = inflight.remove(&chat_key) {
            handle.abort();
            true
        } else {
            false
        }
    };
    let aborted_tool = abort_inflight_tool_abort_handle(state.inner(), &chat_key)?;
    let aborted_delegate_children =
        abort_delegate_runtime_descendants_by_parent_context(
            state.inner(),
            &chat_key,
            Some(&conversation_id),
        )?;
    let cleared_queue_count = clear_conversation_queue(
        state.inner(),
        &conversation_id,
        "消息已因会话撤回被清出队列",
    )?;
    let _ = release_conversation_processing_claim(state.inner(), &conversation_id);
    let _ = set_conversation_runtime_state(state.inner(), &conversation_id, MainSessionState::Idle);
    let _ = set_conversation_remote_im_activation_sources(state.inner(), &conversation_id, Vec::new());

    let aborted = aborted_chat || aborted_tool || aborted_delegate_children > 0;
    if aborted || cleared_queue_count > 0 {
        mark_goal_continue_suppressed_by_user_interrupt(
            state.inner(),
            &conversation_id,
            "interrupt_conversation_runtime",
        )?;
    }
    eprintln!(
        "[聊天调度] 会话运行已中断: conversation_id={}, aborted={}, cleared_queue_count={}, child_abort_count={}",
        conversation_id,
        aborted,
        cleared_queue_count,
        aborted_delegate_children
    );
    Ok(InterruptConversationRuntimeResult {
        aborted,
        cleared_queue_count,
    })
}

#[tauri::command]
async fn get_main_session_state_snapshot(
    state: State<'_, AppState>,
) -> Result<MainSessionState, String> {
    get_main_session_state(state.inner())
}

fn assistant_text_from_stream_blocks(blocks: &[AssistantStreamBlock]) -> String {
    blocks
        .iter()
        .map(|block| block.text.as_str())
        .collect::<Vec<_>>()
        .join("")
}

fn reasoning_text_from_stream_blocks(blocks: &[AssistantStreamBlock]) -> String {
    blocks
        .iter()
        .map(|block| block.reasoning.trim())
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn tool_call_ids_from_history(events: &[Value]) -> std::collections::HashSet<String> {
    events
        .iter()
        .filter_map(|event| event.get("tool_calls").and_then(Value::as_array))
        .flat_map(|calls| calls.iter())
        .filter_map(|call| call.get("id").and_then(Value::as_str))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn stream_blocks_to_tool_history_events(blocks: &[AssistantStreamBlock]) -> Vec<Value> {
    let mut events = Vec::<Value>::new();
    for block in blocks {
        let tools = block
            .tools
            .iter()
            .filter(|tool| !tool.tool_call_id.trim().is_empty() && !tool.name.trim().is_empty())
            .collect::<Vec<_>>();
        if tools.is_empty() {
            continue;
        }
        let mut assistant_event = serde_json::Map::new();
        assistant_event.insert("role".to_string(), Value::String("assistant".to_string()));
        if block.text.trim().is_empty() {
            assistant_event.insert("content".to_string(), Value::Null);
        } else {
            assistant_event.insert("content".to_string(), Value::String(block.text.clone()));
        }
        if !block.reasoning.trim().is_empty() {
            assistant_event.insert(
                "reasoning_content".to_string(),
                Value::String(block.reasoning.trim().to_string()),
            );
        }
        assistant_event.insert(
            "tool_calls".to_string(),
            Value::Array(
                tools
                    .iter()
                    .map(|tool| {
                        serde_json::json!({
                            "id": tool.tool_call_id.trim(),
                            "type": "function",
                            "function": {
                                "name": tool.name.trim(),
                                "arguments": if tool.args_text.trim().is_empty() { "{}" } else { tool.args_text.trim() },
                            }
                        })
                    })
                    .collect(),
            ),
        );
        events.push(Value::Object(assistant_event));
        for tool in tools {
            let result_text = tool.result_text.trim();
            if result_text.is_empty() || tool.status.trim() == "doing" {
                continue;
            }
            events.push(serde_json::json!({
                "role": "tool",
                "tool_call_id": tool.tool_call_id.trim(),
                "content": result_text,
            }));
        }
    }
    events
}

fn merge_stream_block_tool_history(
    completed_tool_history: &[Value],
    partial_stream_blocks: &[AssistantStreamBlock],
) -> Vec<Value> {
    let mut merged = completed_tool_history.to_vec();
    let existing_ids = tool_call_ids_from_history(&merged);
    for event in stream_blocks_to_tool_history_events(partial_stream_blocks) {
        if event
            .get("role")
            .and_then(Value::as_str)
            .is_some_and(|role| role.eq_ignore_ascii_case("assistant"))
        {
            let has_new_tool = event
                .get("tool_calls")
                .and_then(Value::as_array)
                .map(|calls| {
                    calls.iter().any(|call| {
                        call.get("id")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .filter(|id| !id.is_empty())
                            .is_some_and(|id| !existing_ids.contains(id))
                    })
                })
                .unwrap_or(false);
            if !has_new_tool {
                continue;
            }
        }
        merged.push(event);
    }
    merged
}

#[cfg(test)]
mod stop_stream_block_tool_history_tests {
    use super::*;

    #[test]
    fn stream_blocks_to_tool_history_should_keep_running_tool_call_for_interrupted_message() {
        let events = stream_blocks_to_tool_history_events(&[AssistantStreamBlock {
            reasoning: "打算用 operate 工具等待 3 秒。".to_string(),
            text: String::new(),
            tools: vec![AssistantStreamToolBlock {
                tool_call_id: "tool-1".to_string(),
                name: "operate".to_string(),
                args_text: "{\"action\":\"wait\",\"seconds\":3}".to_string(),
                result_text: String::new(),
                status: "doing".to_string(),
            }],
        }]);

        assert_eq!(events.len(), 1);
        assert_eq!(events[0]["role"].as_str(), Some("assistant"));
        assert_eq!(
            events[0]["reasoning_content"].as_str(),
            Some("打算用 operate 工具等待 3 秒。")
        );
        assert_eq!(
            events[0]["tool_calls"][0]["function"]["name"].as_str(),
            Some("operate")
        );
    }
}

#[tauri::command]
async fn get_conversation_runtime_snapshot(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<ConversationRuntimeSnapshot, String> {
    let normalized_conversation_id = conversation_id.trim();
    if normalized_conversation_id.is_empty() {
        return Err("conversationId 不能为空".to_string());
    }
    let snapshot = read_conversation_runtime_snapshot(state.inner(), normalized_conversation_id)?;
    runtime_log_info(format!(
        "[聊天运行态恢复] 完成，任务=读取会话运行态快照，conversation_id={}，runtime_state={:?}，is_processing={}，pending_queue_count={}，has_visible_progress={}，assistant_text_len={}，stream_block_count={}",
        snapshot.conversation_id,
        snapshot.runtime_state,
        snapshot.is_processing,
        snapshot.pending_queue_count,
        snapshot.stream_cache.has_visible_progress,
        snapshot.stream_cache.assistant_text.chars().count(),
        snapshot.stream_cache.stream_blocks.len()
    ));
    Ok(snapshot)
}

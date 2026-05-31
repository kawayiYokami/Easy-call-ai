fn mark_tasks_as_session_lost(data_path: &PathBuf, conversation_id: &str) {
    let Ok(tasks) = task_store_list_task_records(data_path) else {
        eprintln!(
            "[TASK-CLEANUP] 查询任务列表失败: conversation_id={}",
            conversation_id
        );
        return;
    };
    for task in &tasks {
        if task.completion_state != TASK_STATE_ACTIVE {
            continue;
        }
        if task.conversation_id.as_deref() != Some(conversation_id) {
            continue;
        }
        if let Err(err) = task_store_complete_task(
            data_path,
            &TaskCompleteInput {
                task_id: task.task_id.clone(),
                completion_state: TASK_STATE_FAILED_COMPLETED.to_string(),
                completion_conclusion: "会话丢失".to_string(),
            },
        ) {
            eprintln!(
                "[TASK-CLEANUP] 标记任务失败: task_id={}, conversation_id={}, error={}",
                task.task_id, conversation_id, err
            );
        }
    }
}

#[derive(Debug, Clone, Default)]
struct PreparedArchiveMemoryDraft {
    input: Option<MemoryDraftInput>,
    is_profile: bool,
    skipped_profile: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct AppliedArchiveMemoryStats {
    merged_memories: usize,
    merged_groups: usize,
    applied_profile_memories: usize,
    skipped_profile_memories: usize,
}

fn archive_memory_draft_is_profile_candidate(tags: &[String]) -> bool {
    tags.iter().any(|tag| memory_tag_is_user_profile_category_tag(tag))
}

fn prepare_archive_memory_draft(
    draft: &ArchiveMemoryDraft,
    owner_agent_id: Option<&str>,
) -> PreparedArchiveMemoryDraft {
    let judgment = clean_text(draft.judgment.trim());
    if judgment.is_empty() {
        return PreparedArchiveMemoryDraft::default();
    }
    let tags = normalize_memory_keywords(&draft.tags);
    if tags.is_empty() {
        return PreparedArchiveMemoryDraft::default();
    }
    let is_profile_candidate = archive_memory_draft_is_profile_candidate(&tags);
    if is_profile_candidate {
        if tags.len() < 3 || !archive_profile_memory_type_allowed(&draft.memory_type)
        {
            return PreparedArchiveMemoryDraft {
                input: None,
                is_profile: false,
                skipped_profile: true,
            };
        }
    }
    PreparedArchiveMemoryDraft {
        input: Some(MemoryDraftInput {
            memory_type: draft.memory_type.clone(),
            judgment,
            reasoning: clean_text(draft.reasoning.trim()),
            tags,
            owner_agent_id: owner_agent_id
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToOwned::to_owned),
        }),
        is_profile: is_profile_candidate,
        skipped_profile: false,
    }
}

fn upsert_archive_memory_draft_with_ids(
    data_path: &PathBuf,
    draft: &ArchiveMemoryDraft,
    owner_agent_id: Option<&str>,
) -> Result<(Vec<String>, bool, bool), String> {
    let prepared = prepare_archive_memory_draft(draft, owner_agent_id);
    if prepared.skipped_profile {
        return Ok((Vec::new(), false, true));
    }
    let Some(input) = prepared.input else {
        return Ok((Vec::new(), false, false));
    };
    let (results, _) = memory_store_upsert_drafts(data_path, &[input])?;
    Ok((
        results.into_iter().filter_map(|r| r.id).collect::<Vec<_>>(),
        prepared.is_profile,
        false,
    ))
}

fn apply_memory_actions_into_store(
    data_path: &PathBuf,
    actions: &[ArchiveMemoryActionDraft],
    owner_agent_id: Option<&str>,
) -> Result<AppliedArchiveMemoryStats, String> {
    let mut stats = AppliedArchiveMemoryStats::default();
    let mut applied_memories = 0usize;
    for action in actions {
        match action.action {
            ArchiveMemoryActionKind::Create => {
                let (upserted_ids, is_profile, skipped_profile) =
                    upsert_archive_memory_draft_with_ids(data_path, &action.memory, owner_agent_id)?;
                if skipped_profile {
                    stats.skipped_profile_memories += 1;
                    continue;
                }
                applied_memories += upserted_ids.len();
                if is_profile {
                    stats.applied_profile_memories += upserted_ids.len();
                }
            }
            ArchiveMemoryActionKind::Update | ArchiveMemoryActionKind::Merge => {
                let source_ids = action
                    .source_memory_ids
                    .iter()
                    .map(|id| id.trim())
                    .filter(|id| !id.is_empty())
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>();
                if source_ids.is_empty() {
                    continue;
                }
                let (upserted_ids, is_profile, skipped_profile) =
                    upsert_archive_memory_draft_with_ids(data_path, &action.memory, owner_agent_id)?;
                if skipped_profile {
                    stats.skipped_profile_memories += 1;
                    continue;
                }
                if upserted_ids.is_empty() {
                    continue;
                }
                applied_memories += upserted_ids.len();
                if is_profile {
                    stats.applied_profile_memories += upserted_ids.len();
                }
                let retained = upserted_ids
                    .iter()
                    .map(|id| id.as_str())
                    .collect::<HashSet<_>>();
                for source_id in source_ids {
                    if retained.contains(source_id.as_str()) {
                        continue;
                    }
                    if let Err(err) = memory_store_delete_memory(data_path, &source_id) {
                        eprintln!(
                            "[ARCHIVE-PIPELINE] delete merged source memory failed: id={}, err={}",
                            source_id, err
                        );
                    }
                }
                stats.merged_groups += 1;
            }
        }
    }
    stats.merged_memories = applied_memories;
    Ok(stats)
}

fn resolve_archive_owner_context(
    state: &AppState,
    source: &Conversation,
) -> Result<(AgentProfile, String, String), String> {
    let mut config = state_read_config_cached(state)?;
    let runtime = state_read_runtime_state_cached(state)?;
    let user_alias = runtime.user_alias.clone();
    let mut agents = state_read_agents_cached(state)?;
    merge_private_organization_into_runtime(&state.data_path, &mut config, &mut agents)?;

    let owner_agent_id = resolve_archive_owner_agent_id(&config, &agents, source)?;
    let owner_agent = agents
        .iter()
        .find(|agent| agent.id == owner_agent_id)
        .cloned()
        .ok_or_else(|| {
            format!(
                "归档记忆归属人格不存在: conversation_id={}, agent_id={}",
                source.id, owner_agent_id
            )
        })?;

    Ok((owner_agent, owner_agent_id, user_alias))
}

fn archive_profile_memory_type_allowed(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "knowledge" | "skill" | "event"
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForceArchiveResult {
    archived: bool,
    archive_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active_conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    compaction_message: Option<ChatMessage>,
    summary: String,
    merged_memories: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elapsed_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    memory_feedback: Option<MemoryArchiveFeedbackReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    merge_groups: Option<usize>,
}

fn build_context_compaction_followup_runtime_context(source: &Conversation) -> RuntimeContext {
    let mut runtime_context = runtime_context_new(
        "context_compaction",
        "context_compaction_followup",
    );
    runtime_context.request_id = Some(format!(
        "context-compaction-followup-{}",
        Uuid::new_v4()
    ));
    runtime_context.origin_conversation_id = Some(source.id.clone());
    runtime_context.target_conversation_id = Some(source.id.clone());
    runtime_context.root_conversation_id = Some(source.id.clone());
    runtime_context
}

fn context_compaction_followup_department_id(
    state: &AppState,
    source: &Conversation,
    effective_agent_id: &str,
) -> Result<String, String> {
    let department_id = source.department_id.trim();
    if !department_id.is_empty() {
        return Ok(department_id.to_string());
    }
    Ok(department_for_agent_id(&state_read_config_cached(state)?, effective_agent_id)
        .map(|department| department.id.clone())
        .unwrap_or_else(|| ASSISTANT_DEPARTMENT_ID.to_string()))
}

fn enqueue_context_compaction_followup(
    state: &AppState,
    source: &Conversation,
    effective_agent_id: &str,
) -> Result<(), String> {
    if conversation_has_guided_queue_events(state, &source.id).unwrap_or(false) {
        return Ok(());
    }
    let followup_event = ChatPendingEvent {
        id: format!("context-compaction-followup-{}", Uuid::new_v4()),
        conversation_id: source.id.clone(),
        created_at: now_iso(),
        source: ChatEventSource::System,
        queue_mode: ChatQueueMode::Normal,
        messages: Vec::new(),
        activate_assistant: true,
        session_info: ChatSessionInfo {
            department_id: context_compaction_followup_department_id(state, source, effective_agent_id)?,
            agent_id: effective_agent_id.to_string(),
        },
        runtime_context: Some(build_context_compaction_followup_runtime_context(source)),
        sender_info: None,
    };
    match ingress_chat_event(state, followup_event)? {
        ChatEventIngress::Direct(event) => {
            trigger_chat_event_after_ingress(state, ChatEventIngress::Direct(event));
        }
        ChatEventIngress::Queued { event_id } => {
            runtime_log_info(format!(
                "[上下文整理] 完成后续激活已入队 conversation_id={} event_id={}",
                source.id, event_id
            ));
        }
        ChatEventIngress::Duplicate { event_id } => {
            runtime_log_info(format!(
                "[上下文整理] 完成后续激活重复，已忽略 conversation_id={} event_id={}",
                source.id, event_id
            ));
        }
    }
    trigger_chat_queue_processing(state);
    Ok(())
}

fn spawn_organize_context_auto_compaction(
    state: &AppState,
    selected_api: ApiConfig,
    resolved_api: ResolvedApiConfig,
    source: Conversation,
    effective_agent_id: String,
) {
    let state = state.clone();
    tauri::async_runtime::spawn(async move {
        let conversation_id = source.id.clone();
        let result = run_context_compaction_pipeline(
            &state,
            &selected_api,
            &resolved_api,
            &source,
            &effective_agent_id,
            "organize_context",
            "ORGANIZE-CONTEXT-AUTO",
            true,
        )
        .await;
        match result {
            Ok(_) => {
                if let Err(err) = enqueue_context_compaction_followup(
                    &state,
                    &source,
                    &effective_agent_id,
                ) {
                    runtime_log_warn(format!(
                        "[上下文整理] 自动压缩完成后续激活失败 conversation_id={} error={}",
                        conversation_id, err
                    ));
                }
            }
            Err(err) => {
                runtime_log_warn(format!(
                    "[上下文整理] 自动压缩失败 conversation_id={} error={}",
                    conversation_id, err
                ));
            }
        }
    });
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForceArchivePreviewResult {
    conversation_id: String,
    can_archive: bool,
    can_drop_conversation: bool,
    message_count: usize,
    has_assistant_reply: bool,
    is_empty: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    archive_disabled_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForceArchiveCurrentInput {
    session: SessionSelector,
    #[serde(default)]
    target_conversation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForceCompactionPreviewResult {
    conversation_id: String,
    can_compact: bool,
    message_count: usize,
    has_assistant_reply: bool,
    is_empty: bool,
    context_usage_percent: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    compaction_disabled_reason: Option<String>,
}

const SHORT_CONVERSATION_DELETE_THRESHOLD: usize = 3;

fn build_archive_reporting_conversation(
    source: &Conversation,
) -> std::borrow::Cow<'_, Conversation> {
    let Some(fork_cursor) = source
        .fork_message_cursor
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return std::borrow::Cow::Borrowed(source);
    };
    let Some(fork_index) = source
        .messages
        .iter()
        .position(|message| message.id.trim() == fork_cursor)
    else {
        return std::borrow::Cow::Borrowed(source);
    };
    let mut reporting = source.clone();
    reporting.messages = source
        .messages
        .iter()
        .skip(fork_index + 1)
        .cloned()
        .collect();
    std::borrow::Cow::Owned(reporting)
}

fn resolve_archive_target_conversation(
    state: &AppState,
    input: &SessionSelector,
) -> Result<(ApiConfig, ResolvedApiConfig, Conversation, String), String> {
    conversation_service().resolve_archive_target_conversation(state, input)
}

fn prepare_background_archive_active_conversation(
    state: &AppState,
    selected_api: &ApiConfig,
    source: &Conversation,
) -> Result<String, String> {
    conversation_service().prepare_background_archive_active_conversation(
        state,
        selected_api,
        source,
    )
}

fn build_force_compaction_preview_result(
    state: &AppState,
    selected_api: &ApiConfig,
    source: &Conversation,
) -> Result<ForceCompactionPreviewResult, String> {
    let message_count = archive_pipeline_message_count_for_delete(source);
    let has_assistant_reply = archive_pipeline_has_assistant_reply(source);
    let is_empty = source.messages.is_empty();
    let usage_ratio = conversation_prompt_service()
        .latest_real_prompt_usage(source, selected_api)
        .map(|usage| usage.usage_ratio.max(0.0))
        .unwrap_or(0.0);
    let context_usage_percent = usage_ratio.mul_add(100.0, 0.0).round().clamp(0.0, 100.0) as u32;
    let compaction_disabled_reason = if get_conversation_runtime_state(state, &source.id)?
        == MainSessionState::OrganizingContext
    {
        Some("当前会话正在整理上下文或归档处理中，请稍候。".to_string())
    } else if is_empty {
        Some("当前会话为空，无需整理。".to_string())
    } else if !has_assistant_reply {
        Some("当前会话还没有助理回复，暂不建议压缩。".to_string())
    } else {
        None
    };
    Ok(ForceCompactionPreviewResult {
        conversation_id: source.id.clone(),
        can_compact: compaction_disabled_reason.is_none(),
        message_count,
        has_assistant_reply,
        is_empty,
        context_usage_percent,
        compaction_disabled_reason,
    })
}

fn archive_pipeline_message_count_for_delete(source: &Conversation) -> usize {
    source
        .messages
        .iter()
        .filter(|message| {
            matches!(
                message.role.trim().to_ascii_lowercase().as_str(),
                "user" | "assistant"
            )
        })
        .count()
}

fn archive_pipeline_has_assistant_reply(source: &Conversation) -> bool {
    source
        .messages
        .iter()
        .any(|message| message.role.trim().eq_ignore_ascii_case("assistant"))
}

#[derive(Debug, Clone)]
enum SummaryContextModelError {
    EmptyReply(String),
    NonRetryable(String),
}

impl SummaryContextModelError {
    fn is_empty_reply(&self) -> bool {
        matches!(self, SummaryContextModelError::EmptyReply(_))
    }
}

impl std::fmt::Display for SummaryContextModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SummaryContextModelError::EmptyReply(message)
            | SummaryContextModelError::NonRetryable(message) => f.write_str(message),
        }
    }
}

impl From<String> for SummaryContextModelError {
    fn from(value: String) -> Self {
        SummaryContextModelError::NonRetryable(value)
    }
}

async fn summarize_archived_conversation_with_model_v2(
    state: &AppState,
    resolved_api: &ResolvedApiConfig,
    selected_api: &ApiConfig,
    agent: &AgentProfile,
    user_alias: &str,
    source_conversation: &Conversation,
    scene: SummaryContextScene,
    memories: &[MemoryEntry],
    _recall_table: &[String],
) -> Result<MemoryCurationDraft, SummaryContextModelError> {
    let app_config = state_read_config_cached(state)?;
    let agents = state_read_agents_cached(state)?;
    let mut prepared = build_prepared_prompt_for_mode(
        PromptBuildMode::SummaryContext,
        source_conversation,
        agent,
        &agents,
        &app_config.departments,
        user_alias,
        "",
        "concise",
        "zh-CN",
        None,
        None,
        None,
        Some(ChatPromptOverrides {
            latest_user_intent: Some(LatestUserPayloadIntent::SummaryContext {
                scene,
                user_alias: user_alias.to_string(),
            }),
            latest_images: Some(Vec::new()),
            latest_audios: Some(Vec::new()),
            ..ChatPromptOverrides::default()
        }),
        Some(state),
        Some(selected_api),
        Some(resolved_api),
        None,
    );
    prepared.latest_images.clear();
    prepared.latest_audios.clear();
    let timeout_secs = 360u64;
    let archive_summary_execution = call_archive_summary_model_with_timeout(
        state,
        resolved_api,
        selected_api,
        prepared,
        timeout_secs,
    )
    .await;
    push_model_call_log_parts(Some(state), &archive_summary_execution);
    let reply = archive_summary_execution.result?;
    match model_reply_content_state(&reply) {
        ModelReplyContentState::Visible => {}
        ModelReplyContentState::ReasoningOnly => {
            return Err(SummaryContextModelError::EmptyReply(
                "SummaryContext 模型只返回 reasoning，没有最终 JSON".to_string(),
            ));
        }
        ModelReplyContentState::Empty => {
            return Err(SummaryContextModelError::EmptyReply(
                "SummaryContext 模型返回空内容".to_string(),
            ));
        }
    }
    let parsed = parse_memory_curation_draft(&reply.assistant_text).ok_or_else(|| {
        SummaryContextModelError::NonRetryable(format!(
            "SummaryContext JSON 解析失败，raw={}",
            reply.assistant_text.chars().take(240).collect::<String>()
        ))
    })?;
    let open_loops = parsed
        .open_loops
        .iter()
        .map(|item| clean_text(item.trim()))
        .filter(|item| !item.is_empty())
        .take(7)
        .collect::<Vec<_>>();
    let summary = if matches!(scene, SummaryContextScene::Archive) {
        String::new()
    } else {
        compose_summary_context_summary(&parsed.summary, &open_loops, scene)
    };
    if !matches!(scene, SummaryContextScene::Archive) && summary.is_empty() {
        return Err(SummaryContextModelError::NonRetryable(
            "SummaryContext summary is empty".to_string(),
        ));
    }
    let id_alias_map = memory_curation_id_alias_map(memories);
    Ok(MemoryCurationDraft {
        title: if matches!(scene, SummaryContextScene::Archive) {
            String::new()
        } else {
            normalize_summary_context_title(&parsed.title).unwrap_or_default()
        },
        summary,
        open_loops: if matches!(scene, SummaryContextScene::Archive) {
            Vec::new()
        } else {
            open_loops
        },
        useful_memory_ids: resolve_memory_curation_ids(&parsed.useful_memory_ids, &id_alias_map),
        memory_actions: resolve_memory_action_drafts(
            &parsed.memory_actions.into_iter().take(7).collect::<Vec<_>>(),
            &id_alias_map,
        ),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SummaryContextScene {
    Compaction,
    Archive,
}

fn summary_context_prompt_template(scene: SummaryContextScene) -> &'static str {
    match scene {
        SummaryContextScene::Compaction => {
            include_str!("../../../../resources/prompts/summary-context.md")
        }
        SummaryContextScene::Archive => {
            include_str!("../../../../resources/prompts/archive-reflection.md")
        }
    }
}

fn extract_prompt_xml_block(raw: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = raw.find(&open)?;
    let body_start = start + open.len();
    let end = raw[body_start..].find(&close)? + body_start;
    Some(raw[start..end + close.len()].trim().to_string())
}

fn build_summary_context_requirement_block(scene: SummaryContextScene) -> String {
    extract_prompt_xml_block(summary_context_prompt_template(scene), "summary_requirement")
        .unwrap_or_default()
}

fn build_summary_context_system_remind_block(scene: SummaryContextScene) -> String {
    extract_prompt_xml_block(summary_context_prompt_template(scene), "system_remind")
        .unwrap_or_default()
}

fn build_summary_context_memory_block(
    scene: SummaryContextScene,
    agent: &AgentProfile,
    user_alias: &str,
) -> String {
    extract_prompt_xml_block(summary_context_prompt_template(scene), "memory_curation_context")
        .unwrap_or_default()
        .replace("{{assistant_name}}", agent.name.trim())
        .replace("{{user_name}}", user_alias.trim())
        .replace("{{memory_generation_rules}}", memory_generation_rules_body())
}

fn build_summary_context_json_contract_block(scene: SummaryContextScene) -> String {
    let json_example = match scene {
        SummaryContextScene::Compaction => memory_curation_example_output_block(),
        SummaryContextScene::Archive => archive_reflection_example_output_block(),
    };
    extract_prompt_xml_block(summary_context_prompt_template(scene), "json_contract")
        .unwrap_or_default()
        .replace("{{json_example}}", json_example)
}

fn archive_pipeline_message_plain_text(message: &ChatMessage) -> String {
    let mut blocks = Vec::<String>::new();
    for part in &message.parts {
        if let MessagePart::Text { text } = part {
            let cleaned = clean_text(text.trim());
            if !cleaned.is_empty() {
                blocks.push(cleaned);
            }
        }
    }
    for block in &message.extra_text_blocks {
        let cleaned = clean_text(block.trim());
        if !cleaned.is_empty() {
            blocks.push(cleaned);
        }
    }
    clean_text(blocks.join("\n").trim())
}

fn archive_pipeline_recent_user_assistant_messages_by_token_budget(
    source: &Conversation,
    max_tokens: usize,
) -> Vec<(String, String)> {
    if max_tokens == 0 {
        return Vec::new();
    }
    let mut recent = Vec::<(String, String)>::new();
    let mut consumed_tokens = 0.0f64;
    let token_budget = max_tokens as f64;
    for message in source.messages.iter().rev() {
        let role = message.role.trim();
        if role != "user" && role != "assistant" {
            continue;
        }
        if archive_pipeline_is_context_compaction_message(message) {
            continue;
        }
        let text = archive_pipeline_message_plain_text(message);
        if text.is_empty() {
            continue;
        }
        let next_tokens = estimated_tokens_for_text(&text);
        if !recent.is_empty() && consumed_tokens + next_tokens > token_budget {
            break;
        }
        consumed_tokens += next_tokens;
        recent.push((role.to_string(), text));
        if consumed_tokens >= token_budget {
            break;
        }
    }
    recent.reverse();
    recent
}

fn archive_pipeline_is_context_compaction_message(message: &ChatMessage) -> bool {
    if message.role.trim() != "user" {
        return false;
    }
    matches!(
        message
            .provider_meta
            .as_ref()
            .and_then(|meta| meta.get("message_meta"))
            .and_then(|meta| meta.get("kind"))
            .and_then(Value::as_str)
            .map(str::trim),
        Some("context_compaction") | Some("summary_context_seed")
    )
}

fn archive_pipeline_dedup_recall_table(recall_table: &[String]) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    let mut deduped = Vec::<String>::new();
    for id in recall_table
        .iter()
        .map(|id| id.trim())
        .filter(|id| !id.is_empty())
    {
        if seen.insert(id.to_string()) {
            deduped.push(id.to_string());
        }
    }
    deduped
}

fn memory_curation_id_alias_map(memories: &[MemoryEntry]) -> HashMap<String, String> {
    let mut map = HashMap::<String, String>::new();
    for memory in memories {
        let canonical_id = memory.id.trim();
        if canonical_id.is_empty() {
            continue;
        }
        map.insert(canonical_id.to_string(), canonical_id.to_string());
        let display_id = memory.display_id();
        let short_id = display_id.trim();
        if !short_id.is_empty() {
            map.insert(short_id.to_string(), canonical_id.to_string());
        }
    }
    map
}

fn resolve_memory_curation_ids(
    items: &[String],
    id_alias_map: &HashMap<String, String>,
) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::<String>::new();
    for raw in items {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let resolved = id_alias_map
            .get(trimmed)
            .cloned()
            .unwrap_or_else(|| trimmed.to_string());
        if seen.insert(resolved.clone()) {
            out.push(resolved);
        }
    }
    out
}

fn resolve_memory_action_drafts(
    drafts: &[ArchiveMemoryActionDraft],
    id_alias_map: &HashMap<String, String>,
) -> Vec<ArchiveMemoryActionDraft> {
    drafts
        .iter()
        .map(|item| ArchiveMemoryActionDraft {
            action: item.action.clone(),
            source_memory_ids: resolve_memory_curation_ids(&item.source_memory_ids, id_alias_map),
            memory: item.memory.clone(),
        })
        .collect::<Vec<_>>()
}

fn compose_summary_context_summary(
    summary: &str,
    open_loops: &[String],
    scene: SummaryContextScene,
) -> String {
    let summary = clean_text(summary.trim());
    if open_loops.is_empty() {
        return summary;
    }
    let open_loop_lines = open_loops
        .iter()
        .enumerate()
        .map(|(idx, item)| format!("{}. {}", idx + 1, item))
        .collect::<Vec<_>>()
        .join("\n");
    let _ = scene;
    let section_title = "未完事项";
    if summary.is_empty() {
        format!("{}：\n{}", section_title, open_loop_lines)
    } else {
        format!("{}\n\n{}：\n{}", summary, section_title, open_loop_lines)
    }
}

fn emit_archive_history_flushed_event(
    state: &AppState,
    source_conversation_id: &str,
    active_conversation_id: &str,
    archive_id: &str,
    archive_reason: &str,
) {
    let app_handle = match state.app_handle.lock().ok().and_then(|guard| guard.clone()) {
        Some(handle) => handle,
        None => {
            eprintln!(
                "[ARCHIVE-PIPELINE] history_flushed emit skipped: app_handle unavailable, source_conversation_id={}, active_conversation_id={}",
                source_conversation_id, active_conversation_id
            );
            return;
        }
    };
    let payload = serde_json::json!({
        "conversationId": active_conversation_id,
        "messageCount": 0,
        "messages": [],
        "activateAssistant": false,
        "archiveApplied": true,
        "archiveId": archive_id,
        "archiveReason": archive_reason,
        "sourceConversationId": source_conversation_id,
    });
    if let Err(err) = app_handle.emit(CHAT_HISTORY_FLUSHED_EVENT, payload) {
        eprintln!(
            "[ARCHIVE-PIPELINE] history_flushed emit failed: source_conversation_id={}, active_conversation_id={}, archive_id={}, error={}",
            source_conversation_id, active_conversation_id, archive_id, err
        );
    } else {
        eprintln!(
            "[ARCHIVE-PIPELINE] history_flushed emitted: source_conversation_id={}, active_conversation_id={}, archive_id={}",
            source_conversation_id, active_conversation_id, archive_id
        );
    }
    if let Err(err) = emit_unarchived_conversation_overview_updated_from_state(state) {
        eprintln!(
            "[会话概览] archive_history_flushed 后推送失败: source_conversation_id={}, error={}",
            source_conversation_id, err
        );
    }
}

fn emit_compaction_history_flushed_event(
    state: &AppState,
    conversation_id: &str,
    compression_message: &ChatMessage,
    activate_after_flush: bool,
) {
    let app_handle = match state.app_handle.lock().ok().and_then(|guard| guard.clone()) {
        Some(handle) => handle,
        None => {
            eprintln!(
                "[ARCHIVE-PIPELINE] 上下文整理 history_flushed 发送跳过: app_handle 不可用, conversation_id={}",
                conversation_id
            );
            return;
        }
    };
    let payload = serde_json::json!({
        "conversationId": conversation_id,
        "messageCount": 1,
        "messages": [compression_message],
        "activateAssistant": activate_after_flush,
        "compactionApplied": true,
    });
    if let Err(err) = app_handle.emit(CHAT_HISTORY_FLUSHED_EVENT, payload) {
        eprintln!(
            "[ARCHIVE-PIPELINE] 上下文整理 history_flushed 发送失败: conversation_id={}, error={}",
            conversation_id, err
        );
    } else {
        eprintln!(
            "[ARCHIVE-PIPELINE] 上下文整理 history_flushed 已发送: conversation_id={}",
            conversation_id
        );
    }
    if let Err(err) = emit_unarchived_conversation_overview_updated_from_state(state) {
        eprintln!(
            "[会话概览] compaction_history_flushed 后推送失败: conversation_id={}, error={}",
            conversation_id, err
        );
    }
}

fn emit_deleted_history_flushed_event(
    state: &AppState,
    deleted_conversation_id: &str,
    active_conversation_id: &str,
    delete_reason: &str,
) {
    let app_handle = match state.app_handle.lock().ok().and_then(|guard| guard.clone()) {
        Some(handle) => handle,
        None => {
            eprintln!(
                "[ARCHIVE-PIPELINE] 删除 history_flushed 发送跳过: app_handle 不可用, deleted_conversation_id={}, active_conversation_id={}",
                deleted_conversation_id, active_conversation_id
            );
            return;
        }
    };
    let payload = serde_json::json!({
        "conversationId": active_conversation_id,
        "messageCount": 0,
        "messages": [],
        "activateAssistant": false,
        "archiveApplied": false,
        "deletedConversationId": deleted_conversation_id,
        "deleteReason": delete_reason,
    });
    if let Err(err) = app_handle.emit(CHAT_HISTORY_FLUSHED_EVENT, payload) {
        eprintln!(
            "[ARCHIVE-PIPELINE] 删除 history_flushed 发送失败: deleted_conversation_id={}, active_conversation_id={}, error={}",
            deleted_conversation_id, active_conversation_id, err
        );
    } else {
        eprintln!(
            "[ARCHIVE-PIPELINE] 删除 history_flushed 已发送: deleted_conversation_id={}, active_conversation_id={}",
            deleted_conversation_id, active_conversation_id
        );
    }
}

fn delete_main_conversation_and_activate_latest(
    state: &AppState,
    selected_api: &ApiConfig,
    source: &Conversation,
) -> Result<String, String> {
    conversation_service().delete_main_conversation_and_activate_latest(state, selected_api, source)
}

fn build_compaction_message(
    summary: &str,
    title: Option<&str>,
    compaction_reason: &str,
    user_profile_snapshot: Option<&str>,
    current_todos: Option<&[ConversationTodoItem]>,
    preserved_dialogue: Option<&str>,
) -> ChatMessage {
    let now = now_iso();
    let reason = compaction_reason.trim();
    let profile_snapshot = user_profile_snapshot
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(clean_text)
        .unwrap_or_else(|| "（暂无用户画像）".to_string());
    let todo_snapshot = current_todos
        .and_then(todo_markdown_block)
        .map(|value| clean_text(&value))
        .unwrap_or_default();
    let user_profile_block = if todo_snapshot.is_empty() {
        profile_snapshot
    } else {
        format!("{}\n\n{}", profile_snapshot, todo_snapshot)
    };
    let summary_note = if reason.is_empty() {
        "以下内容为当前会话中较早历史对话的整理结果。\n\
         为保证连续性，后文按约 10K token 的预算保留了最近的原始对话，不包含在本段摘要中。\n\
         摘要中的助手发言统一使用当前人格昵称表示。"
            .to_string()
    } else {
        "以下内容为当前会话中较早历史对话的整理结果。\n\
         为保证连续性，后文按约 10K token 的预算保留了最近的原始对话，不包含在本段摘要中。\n\
         摘要中的助手发言统一使用当前人格昵称表示。"
            .to_string()
    };
    let preserved_dialogue_text = preserved_dialogue
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(normalize_multiline_block)
        .unwrap_or_else(|| "（暂无保留对话）".to_string());
    let text = format!(
        "[上下文整理]\n\n用户画像：\n{}\n\n摘要说明：\n{}\n\n摘要正文：\n{}\n\n保留对话：\n{}",
        user_profile_block,
        clean_text(summary_note.trim()),
        clean_compaction_summary_text(summary),
        preserved_dialogue_text
    );
    let normalized_title = title.and_then(normalize_summary_context_title);
    ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "user".to_string(),
        created_at: now,
        speaker_agent_id: Some(SYSTEM_PERSONA_ID.to_string()),
        parts: vec![MessagePart::Text { text }],
        extra_text_blocks: Vec::new(),
        provider_meta: Some(serde_json::json!({
            "message_meta": {
                "kind": "context_compaction",
                "scene": "compaction",
                "schemaVersion": SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION,
                "reason": reason,
                "title": normalized_title,
            }
        })),
        tool_call: None,
        mcp_call: None,
    }
}

fn normalize_multiline_block(input: &str) -> String {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn clean_compaction_summary_text(input: &str) -> String {
    let trimmed = input.trim();
    if let Some((summary, active_plans)) = trimmed.split_once("<active_plans>") {
        let cleaned_summary = clean_text(summary.trim());
        let cleaned_active_plans = normalize_multiline_block(&format!("<active_plans>{active_plans}"));
        if cleaned_summary.is_empty() {
            return cleaned_active_plans;
        }
        return format!("{}\n\n{}", cleaned_summary, cleaned_active_plans);
    }
    clean_text(trimmed)
}

fn build_initial_summary_context_message(
    user_profile_snapshot: Option<&str>,
    current_todos: Option<&[ConversationTodoItem]>,
    title: Option<&str>,
) -> ChatMessage {
    let now = now_iso();
    let profile_snapshot = user_profile_snapshot
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(clean_text)
        .unwrap_or_else(|| "（暂无用户画像）".to_string());
    let todo_snapshot = current_todos
        .and_then(todo_markdown_block)
        .map(|value| clean_text(&value))
        .unwrap_or_default();
    let user_profile_block = if todo_snapshot.is_empty() {
        profile_snapshot
    } else {
        format!("{}\n\n{}", profile_snapshot, todo_snapshot)
    };
    let normalized_title = title.and_then(normalize_summary_context_title);
    let text = format!(
        "[上下文整理]\n\n用户画像：\n{}\n\n摘要说明：\n这是新会话的初始背景，不包含历史对话摘要。",
        user_profile_block
    );
    ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "user".to_string(),
        created_at: now,
        speaker_agent_id: Some(SYSTEM_PERSONA_ID.to_string()),
        parts: vec![MessagePart::Text { text }],
        extra_text_blocks: Vec::new(),
        provider_meta: Some(serde_json::json!({
            "message_meta": {
                "kind": "summary_context_seed",
                "scene": "seed",
                "schemaVersion": SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION,
                "title": normalized_title,
            }
        })),
        tool_call: None,
        mcp_call: None,
    }
}

#[derive(Debug, Clone, Default)]
struct SummaryContextApplyReport {
    merged_memories: usize,
    merged_groups: usize,
    applied_profile_memories: usize,
    skipped_profile_memories: usize,
    memory_feedback: MemoryArchiveFeedbackReport,
}

fn apply_summary_context_result(
    data_path: &PathBuf,
    host_agent: &AgentProfile,
    recall_ids: &[String],
    draft: &MemoryCurationDraft,
) -> Result<SummaryContextApplyReport, String> {
    let owner_agent_id = if host_agent.private_memory_enabled && !host_agent.is_built_in_user {
        Some(host_agent.id.as_str())
    } else {
        None
    };
    let memory_feedback =
        memory_store_apply_archive_feedback(data_path, recall_ids, &draft.useful_memory_ids)?;
    let memory_stats =
        apply_memory_actions_into_store(data_path, &draft.memory_actions, owner_agent_id)?;
    Ok(SummaryContextApplyReport {
        merged_memories: memory_stats.merged_memories,
        merged_groups: memory_stats.merged_groups,
        applied_profile_memories: memory_stats.applied_profile_memories,
        skipped_profile_memories: memory_stats.skipped_profile_memories,
        memory_feedback,
    })
}

fn build_compaction_preserved_dialogue_block(
    source: &Conversation,
    user_alias: &str,
    assistant_name: &str,
    max_tokens: usize,
) -> String {
    archive_pipeline_recent_user_assistant_messages_by_token_budget(source, max_tokens)
        .into_iter()
        .map(|(role, text)| {
            let speaker = if role.eq_ignore_ascii_case("assistant") {
                assistant_name.trim()
            } else {
                user_alias.trim()
            };
            let speaker = if speaker.is_empty() {
                if role.eq_ignore_ascii_case("assistant") {
                    "助手"
                } else {
                    "用户"
                }
            } else {
                speaker
            };
            format!("{}：{}", speaker, text)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn summarize_archive_summary_with_fallback(
    state: &AppState,
    resolved_api: &ResolvedApiConfig,
    selected_api: &ApiConfig,
    host_agent: &AgentProfile,
    user_alias: &str,
    reporting_source: &Conversation,
    memories: &[MemoryEntry],
) -> (MemoryCurationDraft, Option<String>) {
    let deduped_recall = archive_pipeline_dedup_recall_table(&reporting_source.memory_recall_table);
    match summarize_archived_conversation_with_model_v2(
        state,
        resolved_api,
        selected_api,
        host_agent,
        user_alias,
        reporting_source,
        SummaryContextScene::Archive,
        memories,
        &deduped_recall,
    )
    .await
    {
        Ok(mut draft) => {
            draft.title.clear();
            draft.summary.clear();
            draft.open_loops.clear();
            (draft, None)
        }
        Err(err) => (
            MemoryCurationDraft {
                title: String::new(),
                summary: String::new(),
                open_loops: Vec::new(),
                useful_memory_ids: Vec::new(),
                memory_actions: Vec::new(),
            },
            Some(format!(
                "SummaryContext 归档反思失败，已跳过本轮记忆整理：{}",
                err
            )),
        ),
    }
}

async fn summarize_compaction_with_fallback(
    state: &AppState,
    selected_api: &ApiConfig,
    resolved_api: &ResolvedApiConfig,
    host_agent: &AgentProfile,
    user_alias: &str,
    source: &Conversation,
    trace_id: &str,
) -> (MemoryCurationDraft, Option<String>) {
    const MAX_ATTEMPTS: usize = 3;
    const RETRY_DELAY_SECS: u64 = 5;

    let mut last_err = String::new();
    let empty_draft = || MemoryCurationDraft {
        title: String::new(),
        summary: String::new(),
        open_loops: Vec::new(),
        useful_memory_ids: Vec::new(),
        memory_actions: Vec::new(),
    };
    for attempt in 1..=MAX_ATTEMPTS {
        let visible_memories = match memory_store_list_memories_visible_for_agent(
            &state.data_path,
            &host_agent.id,
            host_agent.private_memory_enabled,
        ) {
            Ok(items) => items,
            Err(err) => {
                return (
                    empty_draft(),
                    Some(format!(
                        "SummaryContext 读取可见记忆失败，压缩摘要留空：{}",
                        err
                    )),
                )
            }
        };
        let deduped_recall = archive_pipeline_dedup_recall_table(&source.memory_recall_table);
        match summarize_archived_conversation_with_model_v2(
            state,
            resolved_api,
            selected_api,
            host_agent,
            user_alias,
            source,
            SummaryContextScene::Compaction,
            &visible_memories,
            &deduped_recall,
        )
        .await
        {
            Ok(summary) => return (summary, None),
            Err(err) => {
                let is_empty_reply = err.is_empty_reply();
                last_err = err.to_string();
                if !is_empty_reply {
                    eprintln!(
                        "[SummaryContext] 上下文整理失败，不重试非空回错误: trace_id={}, conversation_id={}, attempt={}，err={}",
                        trace_id,
                        source.id,
                        attempt,
                        last_err
                    );
                    return (
                        empty_draft(),
                        Some(format!(
                            "SummaryContext 上下文整理失败（非空回不重试），压缩摘要留空：{}",
                            last_err
                        )),
                    );
                }
                if attempt < MAX_ATTEMPTS {
                    eprintln!(
                        "[ARCHIVE-PIPELINE] 上下文整理模型空回，准备重试: trace_id={}, conversation_id={}, attempt={}，next_retry_secs={}，error={}",
                        trace_id,
                        source.id,
                        attempt,
                        RETRY_DELAY_SECS,
                        last_err
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(RETRY_DELAY_SECS)).await;
                }
            }
        }
    }

    eprintln!(
        "[SummaryContext] 上下文整理失败，压缩摘要留空继续主流程: trace_id={}, conversation_id={}, attempts={}, err={}",
        trace_id, source.id, MAX_ATTEMPTS, last_err
    );
    (
        empty_draft(),
        Some(format!(
            "SummaryContext 上下文整理失败（空回已尝试{}次），压缩摘要留空：{}",
            MAX_ATTEMPTS, last_err
        )),
    )
}

#[tauri::command]
async fn force_archive_current(
    input: ForceArchiveCurrentInput,
    state: State<'_, AppState>,
) -> Result<ForceArchiveResult, String> {
    if input
        .target_conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
    {
        return Err("归档目标投放已停用：归档只保留原会话正文，并在归档反思中处理记忆。".to_string());
    }
    let (selected_api, resolved_api, source, effective_agent_id) =
        resolve_archive_target_conversation(state.inner(), &input.session)?;
    let runtime = state_read_runtime_state_cached(state.inner())?;
    let main_conversation_id = runtime
        .main_conversation_id
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    if source.id.trim() == main_conversation_id {
        return Err("主会话暂不支持归档。".to_string());
    }
    if get_conversation_runtime_state(state.inner(), &source.id)?
        == MainSessionState::OrganizingContext
    {
        return Err("强制归档正在进行中，请稍候。".to_string());
    }
    let active_conversation_id =
        prepare_background_archive_active_conversation(state.inner(), &selected_api, &source)?;

    let state_cloned = state.inner().clone();
    let selected_api_cloned = selected_api.clone();
    let resolved_api_cloned = resolved_api.clone();
    let source_cloned = source.clone();
    let effective_agent_id_cloned = effective_agent_id.clone();
    let active_conversation_id_for_background = active_conversation_id.clone();
    tauri::async_runtime::spawn(async move {
        let panic_safe_task = std::panic::AssertUnwindSafe(async {
            let result = run_archive_pipeline(
                &state_cloned,
                &selected_api_cloned,
                &resolved_api_cloned,
                &source_cloned,
                &effective_agent_id_cloned,
                Some(active_conversation_id_for_background.as_str()),
                None,
                "manual_force_archive",
                "ARCHIVE-FORCE",
            )
            .await;
            if let Err(err) = result {
                eprintln!(
                    "[ARCHIVE-FORCE] 失败，任务=background_force_archive，conversation_id={}，error={}",
                    source_cloned.id, err
                );
            }
            trigger_chat_queue_processing(&state_cloned);
        });
        if futures_util::FutureExt::catch_unwind(panic_safe_task)
            .await
            .is_err()
        {
            eprintln!(
                "[ARCHIVE-FORCE] 失败，任务=background_force_archive，conversation_id={}，error=panic",
                source_cloned.id
            );
            if let Err(err) = set_conversation_runtime_state(
                &state_cloned,
                &source_cloned.id,
                MainSessionState::Idle,
            ) {
                eprintln!(
                    "[ARCHIVE-FORCE] 警告，任务=background_force_archive_reset_state，conversation_id={}，error={}",
                    source_cloned.id, err
                );
            } else {
                emit_conversation_runtime_state_updated_payload(
                    &state_cloned,
                    &ConversationRuntimeStateUpdatedPayload {
                        conversation_id: source_cloned.id.clone(),
                        runtime_state: MainSessionState::Idle,
                    },
                );
            }
            trigger_chat_queue_processing(&state_cloned);
        }
    });

    Ok(ForceArchiveResult {
        archived: false,
        archive_id: None,
        active_conversation_id: Some(active_conversation_id),
        compaction_message: None,
        summary: String::new(),
        merged_memories: 0,
        warning: None,
        reason_code: Some("background_started".to_string()),
        elapsed_ms: None,
        memory_feedback: None,
        merge_groups: Some(0),
    })
}

#[tauri::command]
async fn force_compact_current(
    input: SessionSelector,
    state: State<'_, AppState>,
) -> Result<ForceArchiveResult, String> {
    let (selected_api, resolved_api, source, effective_agent_id) =
        resolve_archive_target_conversation(state.inner(), &input)?;
    let preview = build_force_compaction_preview_result(state.inner(), &selected_api, &source)?;
    if !preview.can_compact {
        return Err(preview
            .compaction_disabled_reason
            .unwrap_or_else(|| "当前会话暂时不能压缩。".to_string()));
    }
    let result = run_context_compaction_pipeline(
        state.inner(),
        &selected_api,
        &resolved_api,
        &source,
        &effective_agent_id,
        "manual_force_compaction",
        "COMPACTION-FORCE",
        false,
    )
    .await?;
    trigger_chat_queue_processing(state.inner());
    Ok(result)
}

#[tauri::command]
fn preview_force_archive_current(
    input: SessionSelector,
    state: State<'_, AppState>,
) -> Result<ForceArchivePreviewResult, String> {
    let (_selected_api, _resolved_api, source, _effective_agent_id) =
        resolve_archive_target_conversation(state.inner(), &input)?;
    let runtime = state_read_runtime_state_cached(state.inner())?;
    let main_conversation_id = runtime
        .main_conversation_id
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    let is_main_conversation = source.id.trim() == main_conversation_id;
    let message_count = archive_pipeline_message_count_for_delete(&source);
    let has_assistant_reply = archive_pipeline_has_assistant_reply(&source);
    let is_empty = source.messages.is_empty();
    let archive_disabled_reason = if is_main_conversation {
        Some("主会话暂不支持归档。".to_string())
    } else if get_conversation_runtime_state(state.inner(), &source.id)?
        == MainSessionState::OrganizingContext
    {
        Some("当前会话正在后台归档或整理上下文，请稍候。".to_string())
    } else if is_empty {
        Some("当前会话为空，不能归档。".to_string())
    } else if !has_assistant_reply {
        Some("当前会话还没有助理回复，不能归档。".to_string())
    } else if message_count <= SHORT_CONVERSATION_DELETE_THRESHOLD {
        Some(format!(
            "当前会话过短（仅 {} 条用户/助理消息），暂不建议归档。",
            message_count
        ))
    } else {
        None
    };
    Ok(ForceArchivePreviewResult {
        conversation_id: source.id,
        can_archive: archive_disabled_reason.is_none(),
        can_drop_conversation: !is_main_conversation,
        message_count,
        has_assistant_reply,
        is_empty,
        archive_disabled_reason,
    })
}

#[tauri::command]
fn preview_force_compact_current(
    input: SessionSelector,
    state: State<'_, AppState>,
) -> Result<ForceCompactionPreviewResult, String> {
    let (selected_api, _resolved_api, source, _effective_agent_id) =
        resolve_archive_target_conversation(state.inner(), &input)?;
    build_force_compaction_preview_result(state.inner(), &selected_api, &source)
}

pub(crate) async fn run_archive_pipeline(
    state: &AppState,
    selected_api: &ApiConfig,
    resolved_api: &ResolvedApiConfig,
    source: &Conversation,
    effective_agent_id: &str,
    prepared_active_conversation_id: Option<&str>,
    _target_conversation_id: Option<&str>,
    archive_reason: &str,
    trace_tag: &str,
) -> Result<ForceArchiveResult, String> {
    let started_at = std::time::Instant::now();
    let trace_id = Uuid::new_v4().to_string();

    // 设置状态为 OrganizingContext（仅影响所属会话）
    set_conversation_runtime_state(state, &source.id, MainSessionState::OrganizingContext)?;
    emit_conversation_runtime_state_updated_payload(
        state,
        &ConversationRuntimeStateUpdatedPayload {
            conversation_id: source.id.clone(),
            runtime_state: MainSessionState::OrganizingContext,
        },
    );
    eprintln!(
        "[ARCHIVE-PIPELINE] 开始: task=archive_pipeline, trace_id={}, agent_id={}, api_id={}, started_at={}",
        trace_id, effective_agent_id, selected_api.id, started_at.elapsed().as_millis()
    );

    // 确保在所有退出路径都恢复状态
    let result = run_archive_pipeline_inner(
        state,
        selected_api,
        resolved_api,
        source,
        effective_agent_id,
        prepared_active_conversation_id,
        None,
        archive_reason,
        trace_tag,
        started_at,
        &trace_id,
    )
    .await;

    // 归档完成，切换回 Idle（即使内部失败也要恢复状态）
    let elapsed_ms = started_at.elapsed().as_millis();
    if let Err(state_err) =
        set_conversation_runtime_state(state, &source.id, MainSessionState::Idle)
    {
        eprintln!(
            "[ARCHIVE-PIPELINE] 警告: 状态恢复失败, trace_id={}, elapsed_ms={}, error={}",
            trace_id, elapsed_ms, state_err
        );
    } else {
        emit_conversation_runtime_state_updated_payload(
            state,
            &ConversationRuntimeStateUpdatedPayload {
                conversation_id: source.id.clone(),
                runtime_state: MainSessionState::Idle,
            },
        );
        eprintln!(
            "[ARCHIVE-PIPELINE] 完成: task=archive_pipeline, trace_id={}, agent_id={}, api_id={}, elapsed_ms={}",
            trace_id, effective_agent_id, selected_api.id, elapsed_ms
        );
    }

    // 注意：不在这里触发 process_chat_queue，由调用方负责

    result
}

pub(crate) async fn run_context_compaction_pipeline(
    state: &AppState,
    selected_api: &ApiConfig,
    resolved_api: &ResolvedApiConfig,
    source: &Conversation,
    effective_agent_id: &str,
    compaction_reason: &str,
    trace_tag: &str,
    activate_after_flush: bool,
) -> Result<ForceArchiveResult, String> {
    let started_at = std::time::Instant::now();
    let trace_id = Uuid::new_v4().to_string();

    set_conversation_runtime_state(state, &source.id, MainSessionState::OrganizingContext)?;
    eprintln!(
        "[ARCHIVE-PIPELINE] 开始: task=context_compaction, trace_id={}, agent_id={}, api_id={}, started_at={}",
        trace_id, effective_agent_id, selected_api.id, started_at.elapsed().as_millis()
    );

    let result = run_context_compaction_pipeline_inner(
        state,
        selected_api,
        resolved_api,
        source,
        effective_agent_id,
        compaction_reason,
        trace_tag,
        activate_after_flush,
        started_at,
        &trace_id,
    )
    .await;

    let elapsed_ms = started_at.elapsed().as_millis();
    if let Err(state_err) =
        set_conversation_runtime_state(state, &source.id, MainSessionState::Idle)
    {
        eprintln!(
            "[ARCHIVE-PIPELINE] 警告: 状态恢复失败, trace_id={}, elapsed_ms={}, error={}",
            trace_id, elapsed_ms, state_err
        );
    } else {
        eprintln!(
            "[ARCHIVE-PIPELINE] 完成: task=context_compaction, trace_id={}, agent_id={}, api_id={}, elapsed_ms={}",
            trace_id, effective_agent_id, selected_api.id, elapsed_ms
        );
    }

    result
}

async fn run_context_compaction_pipeline_inner(
    state: &AppState,
    selected_api: &ApiConfig,
    resolved_api: &ResolvedApiConfig,
    source: &Conversation,
    _effective_agent_id: &str,
    compaction_reason: &str,
    trace_tag: &str,
    activate_after_flush: bool,
    started_at: std::time::Instant,
    trace_id: &str,
) -> Result<ForceArchiveResult, String> {
    if source.messages.is_empty() {
        return Ok(ForceArchiveResult {
            archived: false,
            archive_id: None,
            active_conversation_id: Some(source.id.clone()),
            compaction_message: None,
            summary: "当前对话为空，无需整理。".to_string(),
            merged_memories: 0,
            warning: None,
            reason_code: Some("empty_conversation".to_string()),
            elapsed_ms: Some(started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64),
            memory_feedback: None,
            merge_groups: None,
        });
    }

    if !archive_pipeline_has_assistant_reply(source) {
        let active_conversation_id =
            delete_main_conversation_and_activate_latest(state, selected_api, source)?;
        emit_deleted_history_flushed_event(
            state,
            &source.id,
            &active_conversation_id,
            "no_assistant_reply_deleted",
        );
        eprintln!(
            "[ARCHIVE-PIPELINE] 整理前直接删除：conversation_id={}, reason=no_assistant_reply_deleted, next_conversation_id={}",
            source.id, active_conversation_id
        );
        return Ok(ForceArchiveResult {
            archived: false,
            archive_id: None,
            active_conversation_id: Some(active_conversation_id),
            compaction_message: None,
            summary: String::new(),
            merged_memories: 0,
            warning: None,
            reason_code: Some("no_assistant_reply_deleted".to_string()),
            elapsed_ms: Some(started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64),
            memory_feedback: None,
            merge_groups: None,
        });
    }

    let (owner_agent, owner_agent_id, user_alias) = resolve_archive_owner_context(state, source)?;

    eprintln!(
        "[{}] trace={} begin api={} model={} format={} conversation={} ownerAgent={}",
        trace_tag,
        trace_id,
        selected_api.id,
        selected_api.model,
        resolved_api.request_format,
        source.id,
        owner_agent_id
    );

    let (summary_draft, compaction_warning) = summarize_compaction_with_fallback(
        state,
        selected_api,
        resolved_api,
        &owner_agent,
        &user_alias,
        source,
        trace_id,
    )
    .await;
    let deduped_recall = archive_pipeline_dedup_recall_table(&source.memory_recall_table);
    let applied_report = apply_summary_context_result(
        &state.data_path,
        &owner_agent,
        &deduped_recall,
        &summary_draft,
    )?;
    let summary_with_pending_plan = match message_store::active_plan_prompt_block(
        &state.data_path,
        &source.id,
    )? {
        Some(plan_block) if summary_draft.summary.trim().is_empty() => {
            format!("\n{}", plan_block.trim())
        }
        Some(plan_block) => format!("{}\n\n{}", summary_draft.summary.trim(), plan_block.trim()),
        None => summary_draft.summary.clone(),
    };
    let user_profile_snapshot =
        if conversation_is_delegate(source) || conversation_is_remote_im_contact(source) {
            None
        } else {
            build_user_profile_snapshot_block(&state.data_path, &owner_agent, 12)?
        };

    let compression_message = build_compaction_message(
        &summary_with_pending_plan,
        Some(summary_draft.title.as_str()),
        compaction_reason,
        user_profile_snapshot.as_deref(),
        Some(&source.current_todos),
        Some(&build_compaction_preserved_dialogue_block(
            source,
            &user_alias,
            &owner_agent.name,
            10_000,
        )),
    );
    let persist_result = conversation_service().persist_compaction_message(
        state,
        source,
        &compression_message,
        user_profile_snapshot.clone(),
    )?;
    let active_conversation_id = persist_result.active_conversation_id;
    let compression_message_id = persist_result.compression_message_id;
    eprintln!(
        "[ARCHIVE-PIPELINE] 上下文整理消息写入校验通过: conversation_id={}, message_id={}",
        source.id, compression_message_id
    );
    match clear_apply_patch_temp(&state.data_path) {
        Ok((record_count, blob_count)) => {
            eprintln!(
                "[apply_patch缓存] 完成，任务=clear_temp_on_compaction，conversation_id={}，记录条数={}，备份条数={}",
                source.id, record_count, blob_count
            );
        }
        Err(err) => {
            eprintln!(
                "[apply_patch缓存] 失败，任务=clear_temp_on_compaction，conversation_id={}，error={}",
                source.id, err
            );
        }
    }
    emit_compaction_history_flushed_event(state, &source.id, &compression_message, activate_after_flush);

    eprintln!(
        "[SummaryContext] 完成，场景=compaction，trace_id={}，conversation_id={}，merged_memories={}，merged_groups={}，profile_applied={}，profile_skipped={}，useful_accept={}，penalized={}，natural_decay={}",
        trace_id,
        source.id,
        applied_report.merged_memories,
        applied_report.merged_groups,
        applied_report.applied_profile_memories,
        applied_report.skipped_profile_memories,
        applied_report.memory_feedback.useful_accepted_count,
        applied_report.memory_feedback.penalized_count,
        applied_report.memory_feedback.natural_decay_count
    );

    Ok(ForceArchiveResult {
        archived: false,
        archive_id: None,
        active_conversation_id,
        compaction_message: Some(compression_message),
        summary: summary_draft.summary,
        merged_memories: applied_report.merged_memories,
        warning: compaction_warning,
        reason_code: None,
        elapsed_ms: Some(started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64),
        memory_feedback: Some(applied_report.memory_feedback),
        merge_groups: Some(applied_report.merged_groups),
    })
}

async fn run_archive_pipeline_inner(
    state: &AppState,
    selected_api: &ApiConfig,
    resolved_api: &ResolvedApiConfig,
    source: &Conversation,
    _effective_agent_id: &str,
    prepared_active_conversation_id: Option<&str>,
    _target_conversation_id: Option<&str>,
    archive_reason: &str,
    trace_tag: &str,
    started_at: std::time::Instant,
    trace_id: &str,
) -> Result<ForceArchiveResult, String> {
    let runtime = state_read_runtime_state_cached(state)?;
    let is_main_conversation = runtime
        .main_conversation_id
        .as_deref()
        .map(str::trim)
        == Some(source.id.as_str());

    if source.messages.is_empty() {
        if !is_main_conversation {
            mark_tasks_as_session_lost(&state.data_path, &source.id);
        }
        let active_conversation_id =
            delete_main_conversation_and_activate_latest(state, selected_api, source)?;
        emit_deleted_history_flushed_event(
            state,
            &source.id,
            &active_conversation_id,
            "empty_conversation_deleted",
        );
        return Ok(ForceArchiveResult {
            archived: false,
            archive_id: None,
            active_conversation_id: Some(active_conversation_id),
            compaction_message: None,
            summary: String::new(),
            merged_memories: 0,
            warning: None,
            reason_code: Some("empty_conversation_deleted".to_string()),
            elapsed_ms: Some(started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64),
            memory_feedback: None,
            merge_groups: None,
        });
    }

    if !archive_pipeline_has_assistant_reply(source) {
        if !is_main_conversation {
            mark_tasks_as_session_lost(&state.data_path, &source.id);
        }
        let active_conversation_id =
            delete_main_conversation_and_activate_latest(state, selected_api, source)?;
        emit_deleted_history_flushed_event(
            state,
            &source.id,
            &active_conversation_id,
            "no_assistant_reply_deleted",
        );
        eprintln!(
            "[ARCHIVE-PIPELINE] 无助理回复会话直接删除: conversation_id={}, next_conversation_id={}",
            source.id, active_conversation_id
        );
        return Ok(ForceArchiveResult {
            archived: false,
            archive_id: None,
            active_conversation_id: Some(active_conversation_id),
            compaction_message: None,
            summary: String::new(),
            merged_memories: 0,
            warning: None,
            reason_code: Some("no_assistant_reply_deleted".to_string()),
            elapsed_ms: Some(started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64),
            memory_feedback: None,
            merge_groups: None,
        });
    }

    let message_count = archive_pipeline_message_count_for_delete(source);
    if message_count <= SHORT_CONVERSATION_DELETE_THRESHOLD {
        if !is_main_conversation {
            mark_tasks_as_session_lost(&state.data_path, &source.id);
        }
        let active_conversation_id =
            delete_main_conversation_and_activate_latest(state, selected_api, source)?;
        emit_deleted_history_flushed_event(
            state,
            &source.id,
            &active_conversation_id,
            "short_conversation_deleted",
        );
        eprintln!(
            "[ARCHIVE-PIPELINE] 短对话直接删除: conversation_id={}, message_count={}, next_conversation_id={}",
            source.id, message_count, active_conversation_id
        );

        return Ok(ForceArchiveResult {
            archived: false,
            archive_id: None,
            active_conversation_id: Some(active_conversation_id),
            compaction_message: None,
            summary: String::new(),
            merged_memories: 0,
            warning: None,
            reason_code: Some("short_conversation_deleted".to_string()),
            elapsed_ms: Some(started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64),
            memory_feedback: None,
            merge_groups: None,
        });
    }

    let (owner_agent, owner_agent_id, user_alias) = resolve_archive_owner_context(state, source)?;
    let memories = memory_store_list_memories_visible_for_agent(
        &state.data_path,
        &owner_agent_id,
        owner_agent.private_memory_enabled,
    )?;

    eprintln!(
        "[{}] trace={} begin api={} model={} format={} conversation={} ownerAgent={}",
        trace_tag,
        trace_id,
        selected_api.id,
        selected_api.model,
        resolved_api.request_format,
        source.id,
        owner_agent_id
    );

    let reporting_source = build_archive_reporting_conversation(source);
    let (summary_draft, archive_warning) = summarize_archive_summary_with_fallback(
        state,
        resolved_api,
        selected_api,
        &owner_agent,
        &user_alias,
        reporting_source.as_ref(),
        &memories,
    )
    .await;
    let deduped_recall = archive_pipeline_dedup_recall_table(&source.memory_recall_table);
    let applied_report = apply_summary_context_result(
        &state.data_path,
        &owner_agent,
        &deduped_recall,
        &summary_draft,
    )?;

    let mut archived_conversation = state_read_conversation_cached(state, &source.id)
        .map_err(|_| "归档前会话已变化，请重试归档。".to_string())?;
    if !archived_conversation.summary.trim().is_empty() {
        return Err("归档前会话已变化，请重试归档。".to_string());
    }
    let previous_status = archived_conversation.status.clone();
    let now = now_iso();
    archived_conversation.status = "archived".to_string();
    archived_conversation.summary.clear();
    archived_conversation.archived_at = Some(now.clone());
    archived_conversation.updated_at = now;
    let archive_id = archived_conversation.id.clone();
    eprintln!(
        "[会话] 已归档: conversation_id={}, previous_status={}, reason=\"{}\"",
        archived_conversation.id,
        previous_status,
        archive_reason
    );
    clear_screenshot_artifact_cache();
    if !is_main_conversation {
        mark_tasks_as_session_lost(&state.data_path, &source.id);
    }
    let active_conversation_id = prepared_active_conversation_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            match conversation_service().resolve_latest_foreground_conversation_id(state, "") {
                Ok(value) => value,
                Err(err) => {
                    runtime_log_warn(format!(
                        "[归档] 警告，任务=resolve_latest_foreground_conversation_id_after_archive，source_conversation_id={}，error={}",
                        source.id, err
                    ));
                    None
                }
            }
        })
        .ok_or_else(|| "归档后未能确定当前前台会话。".to_string())?;
    state_schedule_conversation_persist(state, &archived_conversation, true)?;
    match delegate_runtime_thread_conversation_delete_by_root(state, &source.id) {
        Ok(deleted_count) => runtime_log_info(format!(
            "[委托会话] 完成，任务=随会话归档级联清理，root_conversation_id={}，deleted_count={}",
            source.id, deleted_count
        )),
        Err(err) => runtime_log_warn(format!(
            "[委托会话] 失败，任务=随会话归档级联清理，root_conversation_id={}，error={}",
            source.id, err
        )),
    }

    // 清理PDF缓存
    if let Err(e) = cleanup_pdf_cache_for_conversation(&state, &source.id) {
        eprintln!(
            "[归档] 清理 PDF 缓存失败: conversation={}, error={}",
            source.id, e
        );
    }

    emit_archive_history_flushed_event(
        state,
        &source.id,
        &active_conversation_id,
        &archive_id,
        archive_reason,
    );

    eprintln!(
        "[SummaryContext] 完成，场景=archive，trace_id={}，conversation_id={}，merged_memories={}，merged_groups={}，profile_applied={}，profile_skipped={}，useful_accept={}，penalized={}，natural_decay={}",
        trace_id,
        source.id,
        applied_report.merged_memories,
        applied_report.merged_groups,
        applied_report.applied_profile_memories,
        applied_report.skipped_profile_memories,
        applied_report.memory_feedback.useful_accepted_count,
        applied_report.memory_feedback.penalized_count,
        applied_report.memory_feedback.natural_decay_count
    );

    Ok(ForceArchiveResult {
        archived: true,
        archive_id: Some(archive_id),
        active_conversation_id: Some(active_conversation_id),
        compaction_message: None,
        summary: String::new(),
        merged_memories: applied_report.merged_memories,
        warning: archive_warning,
        reason_code: None,
        elapsed_ms: Some(started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64),
        memory_feedback: Some(applied_report.memory_feedback),
        merge_groups: Some(applied_report.merged_groups),
    })
}

#[cfg(test)]
mod archive_pipeline_tests {
    use super::*;

    fn test_message(id: &str, role: &str, text: &str) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            role: role.to_string(),
            created_at: "2026-04-18T10:00:00Z".to_string(),
            speaker_agent_id: Some("agent-a".to_string()),
            parts: vec![MessagePart::Text {
                text: text.to_string(),
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: None,
            tool_call: None,
            mcp_call: None,
        }
    }

    fn test_conversation() -> Conversation {
        Conversation {
            id: "conversation-a".to_string(),
            title: "测试会话".to_string(),
            agent_id: "agent-a".to_string(),
            department_id: "dept-a".to_string(),
            bound_conversation_id: None,
            parent_conversation_id: Some("parent-a".to_string()),
            child_conversation_ids: Vec::new(),
            fork_message_cursor: Some("m2".to_string()),
            unread_count: 0,
            conversation_kind: CONVERSATION_KIND_CHAT.to_string(),
            root_conversation_id: None,
            delegate_id: None,
            created_at: "2026-04-18T10:00:00Z".to_string(),
            updated_at: "2026-04-18T10:03:00Z".to_string(),
            last_user_at: Some("2026-04-18T10:02:00Z".to_string()),
            last_assistant_at: Some("2026-04-18T10:03:00Z".to_string()),
            status: "active".to_string(),
            summary: String::new(),
            user_profile_snapshot: String::new(),
            shell_workspace_path: None,
            shell_workspaces: Vec::new(),
            shell_autonomous_mode: false,
            archived_at: None,
            messages: vec![
                test_message("m1", "user", "前置问题"),
                test_message("m2", "assistant", "分叉点回答"),
                test_message("m3", "user", "分叉后的新问题"),
                test_message("m4", "assistant", "分叉后的最终结论"),
            ],
            current_todos: Vec::new(),
            memory_recall_table: Vec::new(),
            plan_mode_enabled: false,
        relationship_state: None,
        }
    }

    #[test]
    fn build_archive_reporting_conversation_should_only_keep_post_fork_messages() {
        let source = test_conversation();
        let reporting = build_archive_reporting_conversation(&source);
        let ids = reporting
            .messages
            .iter()
            .map(|message| message.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["m3", "m4"]);
    }

    #[test]
    fn compose_summary_context_summary_should_append_open_loops() {
        let summary = compose_summary_context_summary(
            "已完成前端任务编辑器重构",
            &vec!["继续改 archive pipeline".to_string(), "补充 JSON 契约测试".to_string()],
            SummaryContextScene::Compaction,
        );
        assert!(summary.contains("已完成前端任务编辑器重构"));
        assert!(summary.contains("未完事项"));
        assert!(summary.contains("1. 继续改 archive pipeline"));
        assert!(summary.contains("2. 补充 JSON 契约测试"));
    }
}

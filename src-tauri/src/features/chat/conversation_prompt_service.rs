#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationPromptRevisions {
    conversation_revision: u64,
    prompt_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AbstractConversationMessageProjection {
    stable_message_id: String,
    created_at: String,
    role: String,
    #[serde(default)]
    prompt_role: Option<String>,
    semantic_kind: String,
    #[serde(default)]
    speaker_agent_id: Option<String>,
    text_part_count: usize,
    extra_text_block_count: usize,
    image_part_count: usize,
    audio_part_count: usize,
    attachment_refs: Vec<String>,
    tool_call_count: usize,
    mcp_call_count: usize,
    has_provider_meta: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationPromptSnapshot {
    conversation_id: String,
    agent_id: String,
    revisions: ConversationPromptRevisions,
    department_prompt: String,
    environment_prompt: String,
    abstract_messages: Vec<AbstractConversationMessageProjection>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PromptUsageResolution {
    effective_prompt_tokens: u64,
    usage_ratio: f64,
    estimated_prompt_tokens: Option<u64>,
    source: &'static str,
}

#[derive(Debug, Clone)]
struct AbstractMessageProjectionCacheEntry {
    revision: u64,
    messages: Vec<AbstractConversationMessageProjection>,
}

#[derive(Debug, Default)]
struct ConversationPromptService;

fn conversation_prompt_service() -> &'static ConversationPromptService {
    static SERVICE: OnceLock<ConversationPromptService> = OnceLock::new();
    SERVICE.get_or_init(ConversationPromptService::default)
}

fn prompt_usage_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|item| u64::try_from(item).ok()))
}

fn prompt_usage_f64(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_i64().map(|item| item as f64))
        .or_else(|| value.as_u64().map(|item| item as f64))
        .filter(|item| item.is_finite())
}

fn prompt_usage_meta_u64(provider_meta: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter()
        .find_map(|key| provider_meta.get(*key).and_then(prompt_usage_u64))
        .or_else(|| {
            provider_meta.get("usage").and_then(|usage| {
                keys.iter()
                    .find_map(|key| usage.get(*key).and_then(prompt_usage_u64))
            })
        })
        .filter(|value| *value > 0)
}

fn prompt_usage_meta_f64(provider_meta: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .find_map(|key| provider_meta.get(*key).and_then(prompt_usage_f64))
        .or_else(|| {
            provider_meta.get("usage").and_then(|usage| {
                keys.iter()
                    .find_map(|key| usage.get(*key).and_then(prompt_usage_f64))
            })
        })
        .filter(|value| *value > 0.0)
}

fn prompt_usage_tokens_from_provider_meta(provider_meta: &Value) -> Option<(u64, &'static str)> {
    if let Some(value) = prompt_usage_meta_u64(provider_meta, &["effectivePromptTokens"]) {
        return Some((value, "assistant_message_effective_prompt_tokens"));
    }
    if let Some(value) = prompt_usage_meta_u64(provider_meta, &["providerPromptTokens"]) {
        return Some((value, "assistant_message_provider_prompt_tokens"));
    }
    prompt_usage_meta_u64(
        provider_meta,
        &["promptTokens", "prompt_tokens", "inputTokens", "input_tokens"],
    )
    .map(|value| (value, "assistant_message_usage_prompt_tokens"))
}

fn prompt_usage_ratio_from_provider_meta(provider_meta: &Value) -> Option<(f64, &'static str)> {
    if let Some(value) = prompt_usage_meta_f64(provider_meta, &["contextUsageRatio"]) {
        return Some((value, "assistant_message_context_usage_ratio"));
    }
    prompt_usage_meta_f64(provider_meta, &["contextUsagePercent"])
        .map(|value| (value / 100.0, "assistant_message_context_usage_percent"))
}

fn prompt_usage_resolution_from_provider_meta(
    provider_meta: &Value,
    selected_api: &ApiConfig,
) -> Option<PromptUsageResolution> {
    let context_window = f64::from(selected_api.context_window_tokens.max(1));
    if let Some((value, source)) = prompt_usage_tokens_from_provider_meta(provider_meta) {
        let usage_ratio = prompt_usage_ratio_from_provider_meta(provider_meta)
            .map(|(ratio, _)| ratio)
            .unwrap_or_else(|| value as f64 / context_window);
        return Some(PromptUsageResolution {
            effective_prompt_tokens: value,
            usage_ratio,
            estimated_prompt_tokens: None,
            source,
        });
    }
    prompt_usage_ratio_from_provider_meta(provider_meta).map(|(value, source)| {
        PromptUsageResolution {
            effective_prompt_tokens: (value * context_window)
                .round()
                .clamp(0.0, u64::MAX as f64) as u64,
            usage_ratio: value,
            estimated_prompt_tokens: None,
            source,
        }
    })
}

fn abstract_message_projection_cache(
) -> &'static Mutex<std::collections::HashMap<String, AbstractMessageProjectionCacheEntry>> {
    static CACHE: OnceLock<
        Mutex<std::collections::HashMap<String, AbstractMessageProjectionCacheEntry>>,
    > = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn stable_revision_hash(parts: &[&str]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for part in parts {
        part.hash(&mut hasher);
    }
    hasher.finish()
}

fn stable_revision_hash_json<T: Serialize>(value: &T) -> u64 {
    match serde_json::to_string(value) {
        Ok(text) => stable_revision_hash(&[text.as_str()]),
        Err(err) => stable_revision_hash(&[format!("serde_error:{err}").as_str()]),
    }
}

fn flatten_system_prompt_blocks(blocks: &[String]) -> String {
    let normalized = blocks
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    build_system_prompt_text_uncached(&normalized)
}

fn abstract_message_projection_semantic_kind(message: &ChatMessage, agent_id: &str) -> String {
    if is_context_compaction_message(message, &message.role) {
        let kind = message
            .provider_meta
            .as_ref()
            .and_then(|meta| meta.get("message_meta").or_else(|| meta.get("messageMeta")))
            .and_then(|meta| meta.get("kind"))
            .and_then(Value::as_str)
            .unwrap_or("context_compaction");
        return kind.to_string();
    }
    if remote_im_contact_key_from_message(message).is_some() {
        return "remote_im".to_string();
    }
    if message.tool_call.as_ref().map(|items| !items.is_empty()).unwrap_or(false) {
        return "tool_carrier".to_string();
    }
    let Some(prompt_role) = prompt_role_for_message(message, agent_id) else {
        return "non_prompt".to_string();
    };
    if prompt_role == "assistant" && message.role == "user" {
        return "system_persona_message".to_string();
    }
    "standard".to_string()
}

fn build_abstract_message_projection(
    message: &ChatMessage,
    agent_id: &str,
) -> AbstractConversationMessageProjection {
    let mut text_part_count = 0usize;
    let mut image_part_count = 0usize;
    let mut audio_part_count = 0usize;
    let mut attachment_refs = Vec::<String>::new();
    for part in &message.parts {
        match part {
            MessagePart::Text { .. } => text_part_count += 1,
            MessagePart::Image { name, .. } => {
                image_part_count += 1;
                if let Some(name) = name.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()) {
                    attachment_refs.push(name.to_string());
                }
            }
            MessagePart::Audio { name, .. } => {
                audio_part_count += 1;
                if let Some(name) = name.as_ref().map(|value| value.trim()).filter(|value| !value.is_empty()) {
                    attachment_refs.push(name.to_string());
                }
            }
        }
    }
    if let Some(meta) = message.provider_meta.as_ref() {
        if let Some(attachments) = meta.get("attachments").and_then(Value::as_array) {
            for item in attachments {
                let relative_path = item
                    .get("relativePath")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim();
                if !relative_path.is_empty() {
                    attachment_refs.push(relative_path.to_string());
                }
            }
        }
    }
    AbstractConversationMessageProjection {
        stable_message_id: message.id.clone(),
        created_at: message.created_at.clone(),
        role: message.role.clone(),
        prompt_role: prompt_role_for_message(message, agent_id),
        semantic_kind: abstract_message_projection_semantic_kind(message, agent_id),
        speaker_agent_id: message.speaker_agent_id.clone(),
        text_part_count,
        extra_text_block_count: message.extra_text_blocks.len(),
        image_part_count,
        audio_part_count,
        attachment_refs,
        tool_call_count: message.tool_call.as_ref().map(|items| items.len()).unwrap_or(0),
        mcp_call_count: message.mcp_call.as_ref().map(|items| items.len()).unwrap_or(0),
        has_provider_meta: message.provider_meta.is_some(),
    }
}

impl ConversationPromptService {
    fn trusted_prompt_usage_from_real_usage(
        &self,
        usage: PromptUsageResolution,
    ) -> Option<TrustedPromptUsage> {
        if usage.estimated_prompt_tokens.is_some() {
            return None;
        }
        Some(TrustedPromptUsage {
            effective_prompt_tokens: usage.effective_prompt_tokens,
            context_usage_ratio: usage.usage_ratio,
            estimated: false,
        })
    }

    fn trusted_prompt_usage_from_tokens(
        &self,
        effective_prompt_tokens: u64,
        selected_api: &ApiConfig,
        estimated: bool,
    ) -> Option<TrustedPromptUsage> {
        if effective_prompt_tokens == 0 {
            return None;
        }
        Some(TrustedPromptUsage {
            effective_prompt_tokens,
            context_usage_ratio: effective_prompt_tokens as f64
                / f64::from(selected_api.context_window_tokens.max(1)),
            estimated,
        })
    }

    fn prompt_usage_from_runtime_usage(
        &self,
        usage: TrustedPromptUsage,
        trusted_source: &'static str,
    ) -> PromptUsageResolution {
        PromptUsageResolution {
            effective_prompt_tokens: usage.effective_prompt_tokens,
            usage_ratio: usage.context_usage_ratio,
            estimated_prompt_tokens: usage.estimated.then_some(usage.effective_prompt_tokens),
            source: if usage.estimated {
                "estimated_prompt_tokens"
            } else {
                trusted_source
            },
        }
    }

    fn prime_runtime_trusted_prompt_usage(
        &self,
        runtime_context: &mut RuntimeContext,
        conversation: &Conversation,
        prepared: &PreparedPrompt,
        selected_api: &ApiConfig,
        current_agent: &AgentProfile,
    ) -> PromptUsageResolution {
        if let Some(usage) = runtime_context.trusted_prompt_usage {
            return self.prompt_usage_from_runtime_usage(usage, "trusted_prompt_usage");
        }
        if let Some(usage) = self.latest_real_prompt_usage(conversation, selected_api) {
            runtime_context.trusted_prompt_usage =
                self.trusted_prompt_usage_from_real_usage(usage);
            return usage;
        }
        let usage = self.resolve_prompt_usage_from_estimate(prepared, selected_api, current_agent);
        runtime_context.trusted_prompt_usage = self.trusted_prompt_usage_from_tokens(
            usage.effective_prompt_tokens,
            selected_api,
            true,
        );
        usage
    }

    fn resolve_runtime_trusted_prompt_usage_or_estimate(
        &self,
        runtime_context: &RuntimeContext,
        prepared: &PreparedPrompt,
        selected_api: &ApiConfig,
        current_agent: &AgentProfile,
    ) -> PromptUsageResolution {
        if let Some(usage) = runtime_context.trusted_prompt_usage {
            return self.prompt_usage_from_runtime_usage(usage, "trusted_prompt_usage");
        }
        self.resolve_prompt_usage_from_estimate(prepared, selected_api, current_agent)
    }

    fn resolve_shared_trusted_prompt_usage_or_estimate(
        &self,
        trusted_prompt_usage: &std::sync::Mutex<Option<TrustedPromptUsage>>,
        prepared: &PreparedPrompt,
        selected_api: &ApiConfig,
        current_agent: &AgentProfile,
    ) -> PromptUsageResolution {
        let guard = cache_lock_recover("trusted_prompt_usage", trusted_prompt_usage);
        if let Some(usage) = *guard {
            return self.prompt_usage_from_runtime_usage(usage, "trusted_prompt_usage");
        }
        drop(guard);
        self.resolve_prompt_usage_from_estimate(prepared, selected_api, current_agent)
    }

    fn refresh_shared_trusted_prompt_usage(
        &self,
        trusted_prompt_usage: &std::sync::Mutex<Option<TrustedPromptUsage>>,
        provider_prompt_tokens: Option<u64>,
        selected_api: &ApiConfig,
    ) {
        let Some(next) = provider_prompt_tokens
            .filter(|value| *value > 0)
            .and_then(|value| self.trusted_prompt_usage_from_tokens(value, selected_api, false))
        else {
            return;
        };
        let mut guard = cache_lock_recover("trusted_prompt_usage", trusted_prompt_usage);
        *guard = Some(next);
    }

    fn update_runtime_trusted_prompt_usage_from_request(
        &self,
        runtime_context: &mut RuntimeContext,
        provider_prompt_tokens: Option<u64>,
        estimated_prompt_tokens: Option<u64>,
        selected_api: &ApiConfig,
    ) {
        let next = provider_prompt_tokens
            .filter(|value| *value > 0)
            .and_then(|value| self.trusted_prompt_usage_from_tokens(value, selected_api, false))
            .or_else(|| {
                estimated_prompt_tokens
                    .filter(|value| *value > 0)
                    .and_then(|value| self.trusted_prompt_usage_from_tokens(value, selected_api, true))
            });
        if let Some(next) = next {
            runtime_context.trusted_prompt_usage = Some(next);
        }
    }

    fn store_shared_prompt_usage_resolution(
        &self,
        trusted_prompt_usage: &std::sync::Mutex<Option<TrustedPromptUsage>>,
        usage: &PromptUsageResolution,
        selected_api: &ApiConfig,
    ) {
        let Some(next) = self.trusted_prompt_usage_from_tokens(
            usage.effective_prompt_tokens,
            selected_api,
            usage.estimated_prompt_tokens.is_some(),
        ) else {
            return;
        };
        let mut guard = cache_lock_recover("trusted_prompt_usage", trusted_prompt_usage);
        *guard = Some(next);
    }

    fn add_estimated_prompt_tokens_to_usage(
        &self,
        usage: PromptUsageResolution,
        additional_tokens: u64,
        selected_api: &ApiConfig,
        source: &'static str,
    ) -> PromptUsageResolution {
        if additional_tokens == 0 {
            return usage;
        }
        let effective_prompt_tokens = usage
            .effective_prompt_tokens
            .saturating_add(additional_tokens);
        PromptUsageResolution {
            effective_prompt_tokens,
            usage_ratio: effective_prompt_tokens as f64
                / f64::from(selected_api.context_window_tokens.max(1)),
            estimated_prompt_tokens: Some(effective_prompt_tokens),
            source,
        }
    }

    fn latest_real_prompt_usage(
        &self,
        conversation: &Conversation,
        selected_api: &ApiConfig,
    ) -> Option<PromptUsageResolution> {
        for message in conversation.messages.iter().rev() {
            if is_context_compaction_message(message, &message.role) {
                return None;
            }
            if message.role.trim() != "assistant" {
                continue;
            }
            let Some(provider_meta) = message.provider_meta.as_ref() else {
                continue;
            };
            if let Some(usage) =
                prompt_usage_resolution_from_provider_meta(provider_meta, selected_api)
            {
                return Some(usage);
            }
        }
        None
    }

    fn estimate_prepared_prompt_tokens(
        &self,
        prepared: &PreparedPrompt,
        selected_api: &ApiConfig,
        current_agent: &AgentProfile,
    ) -> u64 {
        let mut total = 0.0f64;
        let preamble_tokens = estimated_tokens_for_text(&prepared.preamble);
        total += preamble_tokens;
        let mut history_overhead = 0.0f64;
        let mut history_text = 0.0f64;
        let mut history_tool_calls = 0.0f64;
        for hm in &prepared.history_messages {
            history_overhead += 4.0;  // 每条消息格式化开销
            history_text += estimated_tokens_for_text(&hm.text);
            for block in &hm.extra_text_blocks {
                history_text += estimated_tokens_for_text(block);
            }
            if let Some(v) = hm.user_time_text.as_deref() {
                history_text += estimated_tokens_for_text(v);
            }
            // reasoning_content 是输出，不算输入 token
            if let Some(calls) = hm.tool_calls.as_ref() {
                // 只算 function.name + function.arguments，不算 call_id/id/type
                for call in calls {
                    if let Some(function) = call.get("function") {
                        if let Some(name) = function.get("name").and_then(|v| v.as_str()) {
                            history_tool_calls += estimated_tokens_for_text(name);
                        }
                        if let Some(arguments) = function.get("arguments").and_then(|v| v.as_str()) {
                            history_tool_calls += estimated_tokens_for_text(arguments);
                        }
                    }
                }
            }
        }
        total += history_overhead + history_text + history_tool_calls;
        runtime_log_debug(format!(
            "[估算Token] preamble={:.0} history_count={} overhead={:.0} text={:.0} tool_calls={:.0} total_so_far={:.0}",
            preamble_tokens,
            prepared.history_messages.len(),
            history_overhead,
            history_text,
            history_tool_calls,
            total,
        ));
        for text_block in prepared_prompt_latest_user_text_blocks(prepared) {
            total += estimated_tokens_for_text(&text_block);
        }
        total += prepared.latest_images.len() as f64 * 280.0;
        total += prepared.latest_audios.len() as f64 * 320.0;

        if selected_api.enable_tools {
            let mut seen = std::collections::HashSet::<String>::new();
            for tool in selected_api
                .tools
                .iter()
                .chain(current_agent.tools.iter())
                .filter(|tool| tool.enabled)
            {
                let key = tool.id.trim().to_ascii_lowercase();
                if key.is_empty() || !seen.insert(key) {
                    continue;
                }
                let text = serde_json::to_string(tool).unwrap_or_default();
                total += estimated_tokens_for_text(&text);
            }
        }

        total.ceil().max(0.0).min(u64::MAX as f64) as u64
    }

    fn resolve_prompt_usage_from_estimate(
        &self,
        prepared: &PreparedPrompt,
        selected_api: &ApiConfig,
        current_agent: &AgentProfile,
    ) -> PromptUsageResolution {
        let context_window = f64::from(selected_api.context_window_tokens.max(1));
        let estimated_prompt_tokens =
            self.estimate_prepared_prompt_tokens(prepared, selected_api, current_agent);
        PromptUsageResolution {
            effective_prompt_tokens: estimated_prompt_tokens,
            usage_ratio: estimated_prompt_tokens as f64 / context_window,
            estimated_prompt_tokens: Some(estimated_prompt_tokens),
            source: "estimated_prompt_tokens",
        }
    }

    fn resolve_prompt_usage(
        &self,
        prepared: &PreparedPrompt,
        selected_api: &ApiConfig,
        current_agent: &AgentProfile,
        conversation: &Conversation,
    ) -> PromptUsageResolution {
        if let Some(usage) = self.latest_real_prompt_usage(conversation, selected_api) {
            return usage;
        }
        self.resolve_prompt_usage_from_estimate(prepared, selected_api, current_agent)
    }

    fn build_prompt_revisions(
        &self,
        conversation: &Conversation,
        department_prompt: &str,
        environment_prompt: &str,
        abstract_messages: &[AbstractConversationMessageProjection],
    ) -> ConversationPromptRevisions {
        let conversation_revision = stable_revision_hash_json(&serde_json::json!({
            "conversation_id": conversation.id,
            "updated_at": conversation.updated_at,
            "message_count": conversation.messages.len(),
            "abstract_messages": abstract_messages,
        }));
        let prompt_revision = stable_revision_hash(&[department_prompt, environment_prompt]);
        ConversationPromptRevisions {
            conversation_revision,
            prompt_revision,
        }
    }

    fn get_or_build_abstract_message_projection(
        &self,
        state: Option<&AppState>,
        conversation: &Conversation,
        agent: &AgentProfile,
    ) -> Vec<AbstractConversationMessageProjection> {
        let cache_key = format!(
            "scope={}|conversation_id={}|agent_id={}",
            prompt_cache_scope_key(state),
            conversation.id.trim(),
            agent.id.trim()
        );
        let source_messages = match find_last_context_compaction_index(&conversation.messages, &agent.id)
        {
            Some(boundary) => &conversation.messages[boundary..],
            None => conversation.messages.as_slice(),
        };
        let projection_revision = stable_revision_hash_json(&serde_json::json!({
            "updated_at": conversation.updated_at,
            "messages": source_messages,
        }));
        {
            let cache = cache_lock_recover(
                "abstract_message_projection_cache",
                abstract_message_projection_cache(),
            );
            if let Some(entry) = cache.get(&cache_key) {
                if entry.revision == projection_revision {
                    return entry.messages.clone();
                }
            }
        }
        let messages = source_messages
            .iter()
            .map(|message| build_abstract_message_projection(message, &agent.id))
            .collect::<Vec<_>>();
        let mut cache = cache_lock_recover(
            "abstract_message_projection_cache",
            abstract_message_projection_cache(),
        );
        cache.insert(
            cache_key,
            AbstractMessageProjectionCacheEntry {
                revision: projection_revision,
                messages: messages.clone(),
            },
        );
        messages
    }

    fn build_prompt_snapshot(
        &self,
        state: Option<&AppState>,
        mode_label: &str,
        conversation: &Conversation,
        agent: &AgentProfile,
        departments: &[DepartmentConfig],
        ui_language: &str,
        selected_api: Option<&ApiConfig>,
        fixed_system_prompt_text: &str,
        user_profile_memory_block: Option<&str>,
        terminal_block: Option<&str>,
        system_preamble_blocks: &[String],
        chat_overrides: Option<&ChatPromptOverrides>,
    ) -> ConversationPromptSnapshot {
        let department_snapshot = get_or_build_department_system_prompt_snapshot(
            state,
            conversation,
            agent,
            departments,
            ui_language,
        );
        let department_config = departments_only_config(departments);
        let current_department = department_for_agent_id(&department_config, &agent.id);
        let mut tool_rule_blocks = Vec::<String>::new();
        tool_rule_blocks.push(build_memory_rag_rule_block());
        let mut deferred_tool_blocks = Vec::<String>::new();
        let mut task_block = None;
        for block in department_snapshot.department_tool_rule_blocks.iter().cloned() {
            if block.contains("<task tool rule>") {
                task_block = Some(block);
            } else {
                deferred_tool_blocks.push(block);
            }
        }
        if let Some(task_block) = task_block {
            tool_rule_blocks.push(task_block);
        }
        tool_rule_blocks.push(build_question_and_planning_rule_block(state, conversation));
        if let Some(todo_block) = build_builtin_tool_rule_block("todo") {
            tool_rule_blocks.push(todo_block);
        }
        tool_rule_blocks.extend(deferred_tool_blocks);
        if department_builtin_tool_enabled(&department_config, current_department, "meme") {
            if let Some(meme_block) = meme_prompt_rule_block(state).as_deref() {
                tool_rule_blocks.push(meme_block.trim().to_string());
            }
        }
        if let Some(image_ref_block) = build_builtin_tool_rule_block("image_reference") {
            tool_rule_blocks.push(image_ref_block);
        }
        if conversation_is_remote_im_contact(conversation)
            || chat_overrides
                .and_then(|overrides| {
                    resolve_bound_remote_im_activation_source(&overrides.remote_im_activation_sources)
                })
                .is_some()
        {
            tool_rule_blocks.push(prompt_xml_block(
                "contact tools rule",
                "联系人专用工具仅对本轮绑定联系人生效。\n\
                 1. 若需要先回应一句“收到、我先看一下、稍后回复”，请使用 `contact_reply`。\n\
                 2. 若需要发送图片或文件，请使用 `contact_send_files`。发送文件时，应把真实本地文件路径放进 `contact_send_files.file_paths`，不要把本地路径或文件链接直接写进正文。\n\
                 3. 若判断本轮结束时不应自动向联系人发送最终回复，请使用 `contact_no_reply`，并在 `reason` 中简要记录原因。\n\
                 4. `contact_reply` 与 `contact_send_files` 是中途动作，不会取消本轮结束后的自动最终回复。\n\
                 5. 如果你没有调用 `contact_no_reply`，系统会在本轮结束后，自动把最终 assistant 回复发给本轮绑定联系人。",
            ));
        }
        let (tool_rule_extra_blocks, runtime_extra_blocks, im_extra_blocks) =
            split_system_preamble_blocks(system_preamble_blocks);
        tool_rule_blocks.extend(tool_rule_extra_blocks);
        if !tool_rule_blocks
            .iter()
            .any(|block| block.contains("<builtin tool general rule>"))
            && !tool_rule_blocks.is_empty()
        {
            tool_rule_blocks.insert(0, build_builtin_tool_general_rule_block());
        }
        let environment_snapshot = get_or_build_conversation_environment_prompt_snapshot(
            state,
            conversation,
            mode_label,
            terminal_block,
            &runtime_extra_blocks,
            &im_extra_blocks,
        );

        let mut department_blocks = Vec::<String>::new();
        let fixed = fixed_system_prompt_text.trim();
        if !fixed.is_empty() {
            department_blocks.push(fixed.to_string());
        }
        let department_prompt_block = department_snapshot.department_prompt_block.trim();
        if !department_prompt_block.is_empty() {
            department_blocks.push(department_prompt_block.to_string());
        }
        department_blocks.extend(
            tool_rule_blocks
                .into_iter()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
        );
        if mode_label.trim() != "summary_context" {
            if let Some(profile_block) = user_profile_memory_block
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                department_blocks.push(profile_block.to_string());
            }
        }
        let department_prompt = flatten_system_prompt_blocks(&department_blocks);
        let environment_prompt = flatten_system_prompt_blocks(
            &environment_snapshot
                .runtime_blocks
                .into_iter()
                .chain(environment_snapshot.im_rule_blocks)
                .collect::<Vec<_>>(),
        );
        let abstract_messages =
            self.get_or_build_abstract_message_projection(state, conversation, agent);
        let revisions = self.build_prompt_revisions(
            conversation,
            &department_prompt,
            &environment_prompt,
            &abstract_messages,
        );
        let _ = selected_api;
        ConversationPromptSnapshot {
            conversation_id: conversation.id.clone(),
            agent_id: agent.id.clone(),
            revisions,
            department_prompt,
            environment_prompt,
            abstract_messages,
        }
    }

    fn resolve_terminal_block(
        &self,
        state: Option<&AppState>,
        conversation: &Conversation,
        selected_api: Option<&ApiConfig>,
        terminal_block_override: Option<&str>,
        stage_logger: Option<&dyn Fn(&str)>,
    ) -> Option<String> {
        if let Some(block) = terminal_block_override
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(block.to_string());
        }
        let block = match (state, selected_api) {
            (Some(state), Some(selected_api)) => {
                terminal_prompt_trusted_roots_block(state, selected_api, Some(conversation))
            }
            _ => None,
        };
        if block.is_some() {
            if let Some(log_stage) = stage_logger {
                log_stage("prepare_context.terminal_block_ready");
            }
        }
        block
    }

    fn build_internal_system_preamble_blocks(
        &self,
        mode: PromptBuildMode,
        state: Option<&AppState>,
        conversation: &Conversation,
        agent: &AgentProfile,
        departments: &[DepartmentConfig],
        ui_language: &str,
        overrides: &ChatPromptOverrides,
        stage_logger: Option<&dyn Fn(&str)>,
    ) -> Vec<String> {
        let mut blocks = Vec::<String>::new();
        if let Some(state) = state {
            let department_config = departments_only_config(departments);
            let current_department = department_for_agent_id(&department_config, &agent.id);
            blocks.push(build_hidden_skill_snapshot_block_for_department(
                state,
                current_department,
            ));
            if let Some(log_stage) = stage_logger {
                log_stage("prepare_context.skill_snapshot_ready");
            }
            if let Some(workspace_agents_block) =
                build_workspace_agents_md_block(conversation, state)
            {
                blocks.push(workspace_agents_block);
            }
            if let Some(log_stage) = stage_logger {
                log_stage("prepare_context.workspace_agents_ready");
            }
            if mode != PromptBuildMode::SummaryContext && overrides.todo_tool_enabled {
                blocks.push(build_todo_guide_block());
            }
            if let Some(log_stage) = stage_logger {
                log_stage("prepare_context.todo_guide_ready");
            }
            if mode != PromptBuildMode::SummaryContext {
                if let Some(runtime_block) = build_remote_im_activation_runtime_block(
                    &overrides.remote_im_activation_sources,
                    ui_language,
                ) {
                    blocks.push(runtime_block);
                }
            }
            if let Some(log_stage) = stage_logger {
                log_stage("prepare_context.im_runtime_ready");
            }
        }
        blocks
    }

    fn build_remote_im_transient_profile_blocks(
        &self,
        state: Option<&AppState>,
        conversation: &Conversation,
        agent: &AgentProfile,
    ) -> Vec<String> {
        const PROFILE_USER_LIMIT: usize = 3;
        const PROFILE_MEMORY_LIMIT_PER_USER: usize = 4;

        if !conversation_is_remote_im_contact(conversation) {
            return Vec::new();
        }
        let Some(state) = state else {
            return Vec::new();
        };

        let mut targets = Vec::<(String, String)>::new();
        let mut seen_user_ids = std::collections::HashSet::<String>::new();
        for message in conversation.messages.iter().rev() {
            let Some(role) = prompt_role_for_message(message, &agent.id) else {
                break;
            };
            if is_context_compaction_message(message, role.as_str())
                || is_tool_review_report_message(message)
            {
                continue;
            }
            if role != "user" {
                break;
            }
            let Some(user_id) = remote_im_message_canonical_user_id(message) else {
                continue;
            };
            if !seen_user_ids.insert(user_id.clone()) {
                continue;
            }
            let display_name = prompt_speaker_label(message, &Vec::new(), "");
            targets.push((user_id, display_name));
            if targets.len() >= PROFILE_USER_LIMIT {
                break;
            }
        }

        let mut blocks = Vec::<String>::new();
        for (user_id, display_name) in targets {
            match build_transient_user_profile_snapshot_block_for_user(
                &state.data_path,
                agent,
                &user_id,
                &display_name,
                PROFILE_MEMORY_LIMIT_PER_USER,
            ) {
                Ok(Some(block)) => blocks.push(block),
                Ok(None) => {}
                Err(err) => {
                    runtime_log_error(format!(
                        "[用户画像] 失败，任务=build_remote_im_transient_profile_blocks，conversation_id={}，user_id={}，error={}",
                        conversation.id, user_id, err
                    ));
                }
            }
        }
        blocks
    }

    fn build_latest_user_payload(
        &self,
        _mode: PromptBuildMode,
        state: Option<&AppState>,
        conversation: &Conversation,
        agent: &AgentProfile,
        overrides: &ChatPromptOverrides,
        prepared: &PreparedPrompt,
        stage_logger: Option<&dyn Fn(&str)>,
    ) -> (String, String, Vec<String>, LatestUserExtraBlocksMode) {
        let Some(intent) = overrides.latest_user_intent.as_ref() else {
            return (
                prepared.latest_user_text.clone(),
                prepared.latest_user_meta_text.clone(),
                prepared.latest_user_extra_blocks.clone(),
                LatestUserExtraBlocksMode::ReplaceIfNonEmpty,
            );
        };
        match intent {
            LatestUserPayloadIntent::ChatRequest {
                trigger_only,
                submitted_user_text,
                include_task_board,
                include_todo_board,
                attachment_relative_paths,
            } => {
                let latest_user_text = if *trigger_only {
                    conversation
                        .messages
                        .iter()
                        .rev()
                        .find(|message| {
                            prompt_role_for_message(message, &agent.id).as_deref()
                                == Some("user")
                        })
                        .map(render_message_content_for_model)
                        .unwrap_or_default()
                } else {
                    submitted_user_text.clone()
                };
                let mut extra_blocks = Vec::<String>::new();
                if *include_task_board {
                    if let Some(state) = state {
                        if let Some(task_board) = build_hidden_task_board_block(state) {
                            extra_blocks.push(task_board);
                        }
                    }
                }
                if let Some(log_stage) = stage_logger {
                    log_stage("prepare_context.task_board_ready");
                }
                if *include_todo_board {
                    if let Some(todo_board) = build_conversation_todo_board_block(conversation) {
                        extra_blocks.push(todo_board);
                    }
                }
                if let Some(log_stage) = stage_logger {
                    log_stage("prepare_context.todo_board_ready");
                }
                for relative_path in attachment_relative_paths {
                    let trimmed = relative_path.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    extra_blocks.push(build_attachment_notice_text("", trimmed));
                }
                if let Some(log_stage) = stage_logger {
                    log_stage("prepare_context.attachment_hints_ready");
                }
                extra_blocks.extend(self.build_remote_im_transient_profile_blocks(
                    state,
                    conversation,
                    agent,
                ));
                (
                    latest_user_text,
                    prepared.latest_user_meta_text.clone(),
                    extra_blocks,
                    LatestUserExtraBlocksMode::Append,
                )
            }
            LatestUserPayloadIntent::SummaryContext {
                scene,
                user_alias,
            } => {
                let mut prompt_blocks = vec![
                    build_summary_context_system_remind_block(*scene),
                    build_summary_context_requirement_block(*scene),
                    build_summary_context_memory_block(*scene, agent, user_alias),
                ];
                prompt_blocks.push(build_summary_context_json_contract_block(*scene));
                (
                    prompt_blocks
                        .into_iter()
                        .map(|block| block.trim().to_string())
                        .filter(|block| !block.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n\n"),
                    String::new(),
                    Vec::new(),
                    LatestUserExtraBlocksMode::ReplaceIfNonEmpty,
                )
            }
            LatestUserPayloadIntent::Explicit {
                text,
                meta_text,
                extra_blocks,
            } => (
                text.clone(),
                meta_text.clone(),
                extra_blocks.clone(),
                LatestUserExtraBlocksMode::ReplaceIfNonEmpty,
            ),
        }
    }

    fn finalize_system_prompt(
        &self,
        state: Option<&AppState>,
        mode: PromptBuildMode,
        mode_label: &str,
        conversation: &Conversation,
        agent: &AgentProfile,
        departments: &[DepartmentConfig],
        selected_api: Option<&ApiConfig>,
        ui_language: &str,
        fixed_system_prompt_text: &str,
        user_profile_memory_block: Option<&str>,
        terminal_block_override: Option<&str>,
        overrides: &ChatPromptOverrides,
        stage_logger: Option<&dyn Fn(&str)>,
    ) -> String {
        let final_cache_key = format!(
            "scope={}|conversation_id={}|agent={}",
            prompt_cache_scope_key(state),
            conversation.id.trim(),
            agent.id.trim(),
        );
        let mut rebuild_reason = "cache_miss";
        {
            let cache = cache_lock_recover("system_prompt_text_cache", system_prompt_text_cache());
            if let Some(entry) = cache.get(&final_cache_key) {
                if entry.dirty_state.is_clean() {
                    if let Some(log_stage) = stage_logger {
                        log_stage("prepare_context.prompt_system_cache_hit");
                    }
                    return entry.text.clone();
                }
                rebuild_reason = entry.dirty_state.rebuild_reason();
            }
        }
        let department_id = department_id_for_agent(departments, &agent.id);
        runtime_log_info(format!(
            "[系统提示词] 开始重建 conversation_id={} agent_id={} department_id={} reason={}",
            conversation.id.trim(),
            agent.id.trim(),
            department_id,
            rebuild_reason
        ));
        let terminal_block = self.resolve_terminal_block(
            state,
            conversation,
            selected_api,
            terminal_block_override,
            stage_logger,
        );
        let system_preamble_blocks = self.build_internal_system_preamble_blocks(
            mode,
            state,
            conversation,
            agent,
            departments,
            ui_language,
            overrides,
            stage_logger,
        );
        let snapshot = self.build_prompt_snapshot(
            state,
            mode_label,
            conversation,
            agent,
            departments,
            ui_language,
            selected_api,
            fixed_system_prompt_text,
            user_profile_memory_block,
            terminal_block.as_deref(),
            &system_preamble_blocks,
            Some(overrides),
        );
        let prompt_text = flatten_system_prompt_blocks(&vec![
            snapshot.department_prompt.clone(),
            snapshot.environment_prompt.clone(),
        ]);
        let mut cache = cache_lock_recover("system_prompt_text_cache", system_prompt_text_cache());
        cache.insert(
            final_cache_key,
            FinalSystemPromptCacheEntry {
                conversation_id: conversation.id.trim().to_string(),
                agent_id: agent.id.trim().to_string(),
                department_id,
                text: prompt_text.clone(),
                dirty_state: FinalSystemPromptDirtyState::default(),
            },
        );
        if let Some(log_stage) = stage_logger {
            log_stage("prepare_context.prompt_system_cache_rebuilt");
        }
        prompt_text
    }

    fn build_conversation_payload(
        &self,
        enriched_conversation: &Conversation,
        source_conversation: &Conversation,
        agent: &AgentProfile,
        agents: &[AgentProfile],
        state: Option<&AppState>,
        data_path: Option<&PathBuf>,
        recall_memories: Option<&[MemoryEntry]>,
        prompt_user_name: &str,
        ui_language: &str,
        latest_user_index: Option<usize>,
    ) -> PreparedConversationPromptPayload {
        let _ = self.get_or_build_abstract_message_projection(
            state,
            enriched_conversation,
            agent,
        );
        build_conversation_prompt_payload(
            enriched_conversation,
            source_conversation,
            agent,
            agents,
            state,
            data_path,
            recall_memories,
            prompt_user_name,
            ui_language,
            latest_user_index,
        )
    }

    fn build_prepared_prompt_for_mode(
        &self,
        mode: PromptBuildMode,
        conversation: &Conversation,
        agent: &AgentProfile,
        agents: &[AgentProfile],
        departments: &[DepartmentConfig],
        user_name: &str,
        user_intro: &str,
        response_style_id: &str,
        ui_language: &str,
        data_path: Option<&PathBuf>,
        _last_archive_summary: Option<&str>,
        terminal_block_override: Option<String>,
        chat_overrides: Option<ChatPromptOverrides>,
        state: Option<&AppState>,
        stage_logger: Option<&dyn Fn(&str)>,
        selected_api: Option<&ApiConfig>,
        resolved_api: Option<&ResolvedApiConfig>,
        enable_pdf_images: Option<bool>,
    ) -> PreparedPrompt {
        match mode {
            PromptBuildMode::Chat => {
                let mut prepared = build_prompt_with_stage_logger(
                    conversation,
                    agent,
                    agents,
                    departments,
                    user_name,
                    user_intro,
                    response_style_id,
                    ui_language,
                    data_path,
                    state,
                    stage_logger,
                    resolved_api,
                    enable_pdf_images.unwrap_or(false),
                );
                let overrides = chat_overrides.unwrap_or_default();
                prepared.preamble = self.finalize_system_prompt(
                    state,
                    PromptBuildMode::Chat,
                    "chat",
                    conversation,
                    agent,
                    departments,
                    selected_api,
                    ui_language,
                    &prepared.preamble,
                    None,
                    terminal_block_override.as_deref(),
                    &overrides,
                    stage_logger,
                );
                if let Some(log_stage) = stage_logger {
                    log_stage("prepare_context.prompt_system_finalize_ready");
                }
                let (
                    latest_user_text,
                    latest_user_meta_text,
                    latest_user_extra_blocks,
                    latest_user_extra_blocks_mode,
                ) =
                    self.build_latest_user_payload(
                        PromptBuildMode::Chat,
                        state,
                        conversation,
                        agent,
                        &overrides,
                        &prepared,
                        stage_logger,
                    );
                apply_chat_latest_user_payload(
                    &mut prepared,
                    latest_user_text,
                    latest_user_meta_text,
                    &latest_user_extra_blocks,
                    latest_user_extra_blocks_mode,
                    overrides.latest_images,
                    overrides.latest_audios,
                );
                prepared
            }
            PromptBuildMode::Delegate => {
                let mut prepared = build_delegate_prompt_with_stage_logger(
                    conversation,
                    agent,
                    agents,
                    departments,
                    response_style_id,
                    ui_language,
                    data_path,
                    state,
                    stage_logger,
                    resolved_api,
                    enable_pdf_images.unwrap_or(false),
                );
                let overrides = chat_overrides.unwrap_or_default();
                prepared.preamble = self.finalize_system_prompt(
                    state,
                    PromptBuildMode::Delegate,
                    "delegate",
                    conversation,
                    agent,
                    departments,
                    selected_api,
                    ui_language,
                    &prepared.preamble,
                    None,
                    terminal_block_override.as_deref(),
                    &overrides,
                    stage_logger,
                );
                if let Some(log_stage) = stage_logger {
                    log_stage("prepare_context.prompt_system_finalize_ready");
                }
                let (
                    latest_user_text,
                    latest_user_meta_text,
                    latest_user_extra_blocks,
                    latest_user_extra_blocks_mode,
                ) =
                    self.build_latest_user_payload(
                        PromptBuildMode::Delegate,
                        state,
                        conversation,
                        agent,
                        &overrides,
                        &prepared,
                        stage_logger,
                    );
                apply_chat_latest_user_payload(
                    &mut prepared,
                    latest_user_text,
                    latest_user_meta_text,
                    &latest_user_extra_blocks,
                    latest_user_extra_blocks_mode,
                    overrides.latest_images,
                    overrides.latest_audios,
                );
                prepared
            }
            PromptBuildMode::SummaryContext => {
                let mut prepared = build_prompt_with_stage_logger(
                    conversation,
                    agent,
                    agents,
                    departments,
                    user_name,
                    user_intro,
                    response_style_id,
                    ui_language,
                    data_path,
                    state,
                    stage_logger,
                    resolved_api,
                    enable_pdf_images.unwrap_or(false),
                );
                for message in &mut prepared.history_messages {
                    message.images.clear();
                    message.audios.clear();
                }
                prepared.latest_user_meta_text.clear();
                prepared.latest_user_extra_text.clear();
                prepared.latest_user_extra_blocks.clear();
                prepared.latest_images.clear();
                prepared.latest_audios.clear();
                let overrides = chat_overrides.unwrap_or_default();
                prepared.preamble = self.finalize_system_prompt(
                    state,
                    PromptBuildMode::SummaryContext,
                    "summary_context",
                    conversation,
                    agent,
                    departments,
                    selected_api,
                    ui_language,
                    &prepared.preamble,
                    None,
                    terminal_block_override.as_deref(),
                    &overrides,
                    stage_logger,
                );
                if let Some(log_stage) = stage_logger {
                    log_stage("prepare_context.prompt_system_finalize_ready");
                }
                let (
                    latest_user_text,
                    latest_user_meta_text,
                    latest_user_extra_blocks,
                    latest_user_extra_blocks_mode,
                ) =
                    self.build_latest_user_payload(
                        PromptBuildMode::SummaryContext,
                        state,
                        conversation,
                        agent,
                        &overrides,
                        &prepared,
                        stage_logger,
                    );
                apply_chat_latest_user_payload(
                    &mut prepared,
                    latest_user_text,
                    latest_user_meta_text,
                    &latest_user_extra_blocks,
                    latest_user_extra_blocks_mode,
                    overrides.latest_images,
                    overrides.latest_audios,
                );
                prepared
            }
        }
    }

    fn build_tool_safety_review_prepared_prompt(
        &self,
        language: &str,
        tool_name: &str,
        context: &Value,
    ) -> PreparedPrompt {
        PreparedPrompt {
            preamble: tool_safety_review_system_prompt(language),
            history_messages: Vec::new(),
            latest_user_text: build_tool_safety_review_user_prompt(tool_name, context),
            latest_user_meta_text: String::new(),
            latest_user_extra_text: String::new(),
            latest_user_extra_blocks: Vec::new(),
            latest_images: Vec::new(),
            latest_audios: Vec::new(),
        }
    }

    fn build_vision_description_prepared_prompt(
        &self,
        image: &BinaryPart,
    ) -> PreparedPrompt {
        let mime = image.mime.trim();
        PreparedPrompt {
            preamble: "[SYSTEM PROMPT]\n你是图像理解助手。请读取图片中的关键信息并输出简洁中文描述，保留有价值的文本、数字、UI元素与上下文。".to_string(),
            history_messages: Vec::new(),
            latest_user_text: "请识别这张图片并给出可用于后续对话的文本描述。".to_string(),
            latest_user_meta_text: String::new(),
            latest_user_extra_text: String::new(),
            latest_user_extra_blocks: Vec::new(),
            latest_images: vec![PreparedBinaryPayload {
                mime: if mime.is_empty() {
                    "image/png".to_string()
                } else {
                    mime.to_string()
                },
                content: image.bytes_base64.clone(),
                saved_path: image.saved_path.clone(),
            }],
            latest_audios: Vec::new(),
        }
    }
}

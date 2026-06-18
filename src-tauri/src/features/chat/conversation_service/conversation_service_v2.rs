#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ConversationServiceV2ErrorCode {
    ConversationNotFound,
    ConversationBusy,
    ConversationReadOnly,
    InvalidOverwriteSource,
    OverwriteForbidden,
    MessageNotFound,
    AnchorNotFound,
    ToolAppendClosed,
    FinalTextAlreadyCommitted,
    MessageNotWritable,
    IntegrityCheckFailed,
    StorageCorrupted,
}

impl ConversationServiceV2ErrorCode {
    fn as_str(self) -> &'static str {
        match self {
            Self::ConversationNotFound => "CONV_NOT_FOUND",
            Self::ConversationBusy => "CONV_BUSY",
            Self::ConversationReadOnly => "CONV_READ_ONLY",
            Self::InvalidOverwriteSource => "CONV_INVALID_OVERWRITE_SOURCE",
            Self::OverwriteForbidden => "CONV_OVERWRITE_FORBIDDEN",
            Self::MessageNotFound => "MSG_NOT_FOUND",
            Self::AnchorNotFound => "MSG_ANCHOR_NOT_FOUND",
            Self::ToolAppendClosed => "MSG_TOOL_APPEND_CLOSED",
            Self::FinalTextAlreadyCommitted => "MSG_FINAL_TEXT_ALREADY_COMMITTED",
            Self::MessageNotWritable => "MSG_NOT_WRITABLE",
            Self::IntegrityCheckFailed => "STORE_INTEGRITY_FAILED",
            Self::StorageCorrupted => "STORE_CORRUPTED",
        }
    }
}

#[derive(Debug, Clone)]
struct ConversationServiceV2Error {
    code: ConversationServiceV2ErrorCode,
    message: String,
}

impl ConversationServiceV2Error {
    fn new(code: ConversationServiceV2ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn into_string(self) -> String {
        format!("{}: {}", self.code.as_str(), self.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ConversationOverwriteSource {
    Import,
    ExportSync,
    MigrationRecovery,
}

impl ConversationOverwriteSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::ExportSync => "export_sync",
            Self::MigrationRecovery => "migration_recovery",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationOverwriteAudit {
    job_id: String,
    source: ConversationOverwriteSource,
    operator: String,
    reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationMetaView {
    id: String,
    title: String,
    latest_summary_title: Option<String>,
    status: String,
    summary: String,
    conversation_kind: String,
    visible_in_foreground_lists: bool,
    is_remote_im_contact: bool,
    is_delegate: bool,
    agent_id: String,
    delegate_id: Option<String>,
    department_id: String,
    root_conversation_id: Option<String>,
    unread_count: usize,
    updated_at: String,
    created_at: String,
    archived_at: Option<String>,
    last_user_at: Option<String>,
    last_assistant_at: Option<String>,
    message_count: usize,
    body_message_count: usize,
    body_text_length: usize,
    has_assistant_reply: bool,
    has_context_compaction_message: bool,
    last_message_at: Option<String>,
    parent_conversation_id: Option<String>,
    fork_message_cursor: Option<String>,
    user_profile_snapshot: String,
    preferred_api_config_id: Option<String>,
    auto_push_remote_contact_id: Option<String>,
    cumulative_usage: ConversationCumulativeUsage,
    plan_mode_enabled: bool,
    shell_workspace_path: Option<String>,
    shell_workspaces: Vec<ShellWorkspaceConfig>,
    shell_autonomous_mode: bool,
    current_todos: Vec<ConversationTodoItem>,
    active_goal: Option<ConversationGoalState>,
    preview_messages: Vec<ConversationMetaPreviewMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationMetaPreviewMessage {
    message_id: String,
    role: String,
    speaker_agent_id: Option<String>,
    created_at: Option<String>,
    text_preview: String,
    has_image: bool,
    has_pdf: bool,
    has_audio: bool,
    has_attachment: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationMessagePageView {
    messages: Vec<ChatMessage>,
    has_more: bool,
    has_more_before: bool,
    has_more_after: bool,
    first_message_id: Option<String>,
    last_message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssistantMessageToolAppendInput {
    conversation_id: String,
    assistant_message_id: String,
    assistant_tool_event: Value,
    tool_result_event: Value,
    provider_meta_patch: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssistantMessageToolAppendResult {
    conversation_id: String,
    assistant_message_id: String,
    tool_event_count: usize,
    tool_append_closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssistantMessageFinalTextAppendInput {
    conversation_id: String,
    assistant_message_id: String,
    final_text: String,
    reasoning_text: Option<String>,
    provider_meta_patch: Option<Value>,
    meme_annotations: Option<Vec<MemeAnnotation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssistantMessageFinalTextAppendResult {
    conversation_id: String,
    assistant_message_id: String,
    final_text_committed: bool,
    tool_append_closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssistantMessageBootstrapInput {
    conversation_id: String,
    assistant_message_id: String,
    speaker_agent_id: String,
    created_at: Option<String>,
    provider_meta_patch: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssistantMessageBootstrapResult {
    conversation_id: String,
    assistant_message_id: String,
    created: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssistantMessageProviderMetaPatchInput {
    conversation_id: String,
    assistant_message_id: String,
    provider_meta_patch: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssistantMessageProviderMetaPatchResult {
    conversation_id: String,
    assistant_message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserMessageAppendInput {
    conversation_id: String,
    message: ChatMessage,
    #[serde(default)]
    memory_recall_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessageProviderMetaPatchItem {
    message_id: String,
    provider_meta: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MessageProviderMetaBatchPatchInput {
    conversation_id: String,
    items: Vec<MessageProviderMetaPatchItem>,
}

impl ConversationMetaView {
    fn from_meta(meta: &message_store::ConversationShardMeta) -> Self {
        Self {
            id: meta.id().to_string(),
            title: meta.title().to_string(),
            latest_summary_title: meta.latest_summary_title().map(ToOwned::to_owned),
            status: meta.status().to_string(),
            summary: meta.summary().to_string(),
            conversation_kind: meta.conversation_kind().to_string(),
            visible_in_foreground_lists: conversation_service_v2()
                .conversation_meta_visible_in_foreground_lists(meta),
            is_remote_im_contact: conversation_service_v2()
                .conversation_meta_is_remote_im_contact(meta),
            is_delegate: conversation_service_v2().conversation_meta_is_delegate(meta),
            agent_id: meta.agent_id().to_string(),
            delegate_id: meta.delegate_id().map(ToOwned::to_owned),
            department_id: meta.department_id().to_string(),
            root_conversation_id: meta.root_conversation_id_text().map(ToOwned::to_owned),
            unread_count: meta.unread_count(),
            updated_at: meta.updated_at().to_string(),
            created_at: meta.created_at().to_string(),
            archived_at: meta.archived_at().map(ToOwned::to_owned),
            last_user_at: meta.last_user_at().map(ToOwned::to_owned),
            last_assistant_at: meta.last_assistant_at().map(ToOwned::to_owned),
            message_count: meta.message_count(),
            body_message_count: meta.body_message_count(),
            body_text_length: meta.body_text_length(),
            has_assistant_reply: meta.has_assistant_reply(),
            has_context_compaction_message: meta.has_context_compaction_message(),
            last_message_at: meta.last_message_at().map(ToOwned::to_owned),
            parent_conversation_id: meta.parent_conversation_id().map(ToOwned::to_owned),
            fork_message_cursor: meta.fork_message_cursor().map(ToOwned::to_owned),
            user_profile_snapshot: meta.user_profile_snapshot().to_string(),
            preferred_api_config_id: meta.preferred_api_config_id().map(ToOwned::to_owned),
            auto_push_remote_contact_id: meta.auto_push_remote_contact_id().map(ToOwned::to_owned),
            cumulative_usage: meta.cumulative_usage().clone(),
            plan_mode_enabled: meta.plan_mode_enabled(),
            shell_workspace_path: meta.shell_workspace_path().map(ToOwned::to_owned),
            shell_workspaces: meta.shell_workspaces().to_vec(),
            shell_autonomous_mode: meta.shell_autonomous_mode(),
            current_todos: meta.current_todos().to_vec(),
            active_goal: meta.active_goal().cloned(),
            preview_messages: meta
                .preview_messages()
                .iter()
                .map(|item| ConversationMetaPreviewMessage {
                    message_id: item.message_id.clone(),
                    role: item.role.clone(),
                    speaker_agent_id: item.speaker_agent_id.clone(),
                    created_at: item.created_at.clone(),
                    text_preview: item.text_preview.clone(),
                    has_image: item.has_image,
                    has_pdf: item.has_pdf,
                    has_audio: item.has_audio,
                    has_attachment: item.has_attachment,
                })
                .collect(),
        }
    }
}

const FRONTEND_TOOL_RESULT_PLACEHOLDER_TEXT: &str = "工具已执行，结果已省略。";

fn frontend_sanitize_tool_history_event(event: &Value) -> Value {
    let Some(object) = event.as_object() else {
        return event.clone();
    };
    let role = object
        .get("role")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if role != "tool" {
        return event.clone();
    }
    let tool_call_id = object
        .get("tool_call_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    let mut sanitized = object.clone();
    sanitized.insert(
        "content".to_string(),
        Value::String(FRONTEND_TOOL_RESULT_PLACEHOLDER_TEXT.to_string()),
    );
    sanitized.insert("contentOmitted".to_string(), Value::Bool(true));
    if !tool_call_id.is_empty() {
        sanitized.insert(
            "tool_call_id".to_string(),
            Value::String(tool_call_id.to_string()),
        );
    }
    Value::Object(sanitized)
}

fn frontend_project_message(mut message: ChatMessage) -> ChatMessage {
    if let Some(events) = message.tool_call.take() {
        let projected = events
            .into_iter()
            .map(|event| frontend_sanitize_tool_history_event(&event))
            .collect::<Vec<_>>();
        message.tool_call = if projected.is_empty() {
            None
        } else {
            Some(projected)
        };
    }
    message
}

fn frontend_project_messages(messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
    messages.into_iter().map(frontend_project_message).collect()
}

fn assistant_message_has_final_text(message: &ChatMessage) -> bool {
    message.parts.iter().any(|part| match part {
        MessagePart::Text { text, .. } => !text.trim().is_empty(),
        _ => false,
    }) || message
        .extra_text_blocks
        .iter()
        .any(|block| !block.trim().is_empty())
}

fn build_message_page_view_v2(
    messages: Vec<ChatMessage>,
    has_more_before: bool,
    has_more_after: bool,
) -> ConversationMessagePageView {
    let first_message_id = messages
        .first()
        .map(|message| message.id.trim().to_string())
        .filter(|value| !value.is_empty());
    let last_message_id = messages
        .last()
        .map(|message| message.id.trim().to_string())
        .filter(|value| !value.is_empty());
    ConversationMessagePageView {
        has_more: has_more_before || has_more_after,
        has_more_before,
        has_more_after,
        first_message_id,
        last_message_id,
        messages,
    }
}

fn dedup_memory_recall_ids_v2(ids: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::<String>::new();
    ids.iter()
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
        .filter(|item| seen.insert(item.clone()))
        .collect()
}

fn assistant_message_tool_append_closed(message: &ChatMessage) -> bool {
    assistant_message_has_final_text(message)
        || message
            .provider_meta
            .as_ref()
            .and_then(|meta| meta.get("streamFinalCommitted"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn merge_optional_text_block_v2(current: &mut Option<String>, next: Option<String>) {
    let Some(next) = next.map(|value| value.trim().to_string()) else {
        return;
    };
    if next.is_empty() {
        return;
    }
    match current {
        Some(current) if !current.trim().is_empty() => {
            current.push_str("\n\n");
            current.push_str(&next);
        }
        _ => *current = Some(next),
    }
}

fn merge_provider_meta_patch_v2(target: &mut Option<Value>, patch: Option<Value>) {
    let Some(patch) = patch else {
        return;
    };
    let Some(patch_obj) = patch.as_object() else {
        return;
    };
    if patch_obj.is_empty() {
        return;
    }
    let mut current = target.take().unwrap_or_else(|| serde_json::json!({}));
    if !current.is_object() {
        current = serde_json::json!({
            "_raw_provider_meta": current,
        });
    }
    if let Some(current_obj) = current.as_object_mut() {
        for (key, value) in patch_obj {
            current_obj.insert(key.clone(), value.clone());
        }
    }
    *target = Some(current);
}

fn mark_stream_final_committed_v2(target: &mut Option<Value>) {
    let mut current = target.take().unwrap_or_else(|| serde_json::json!({}));
    if !current.is_object() {
        current = serde_json::json!({
            "_raw_provider_meta": current,
        });
    }
    if let Some(current_obj) = current.as_object_mut() {
        current_obj.insert("streamFinalCommitted".to_string(), Value::Bool(true));
    }
    *target = Some(current);
}

fn tool_call_ids_from_assistant_tool_event_v2(event: &Value) -> Vec<String> {
    event
        .get("tool_calls")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            item.get("id")
                .or_else(|| item.get("call_id"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn tool_history_contains_assistant_tool_group_v2(
    events: &[Value],
    group_call_ids: &[String],
) -> bool {
    if group_call_ids.is_empty() {
        return false;
    }
    events.iter().any(|event| {
        event
            .get("role")
            .and_then(Value::as_str)
            .is_some_and(|role| role.trim().eq_ignore_ascii_case("assistant"))
            && event
                .get("tool_calls")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .any(|call| {
                    call.get("id")
                        .or_else(|| call.get("call_id"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .is_some_and(|id| group_call_ids.iter().any(|candidate| candidate == id))
                })
    })
}

fn tool_history_contains_tool_result_id_v2(events: &[Value], tool_call_id: &str) -> bool {
    events.iter().any(|event| {
        event
            .get("role")
            .and_then(Value::as_str)
            .is_some_and(|role| role.trim().eq_ignore_ascii_case("tool"))
            && event
                .get("tool_call_id")
                .and_then(Value::as_str)
                .map(str::trim)
                == Some(tool_call_id)
    })
}

fn validate_tool_group_result_append_v2(
    assistant_tool_call_event: &Value,
    tool_result_event: &Value,
) -> Result<String, String> {
    let assistant_role = assistant_tool_call_event
        .get("role")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if assistant_role != "assistant" {
        return Err("追加工具结果失败：第一条事件必须是 assistant tool_call".to_string());
    }
    let group_call_ids = tool_call_ids_from_assistant_tool_event_v2(assistant_tool_call_event);
    if group_call_ids.is_empty() {
        return Err("追加工具结果失败：assistant 事件缺少 tool_calls".to_string());
    }
    let tool_role = tool_result_event
        .get("role")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if tool_role != "tool" {
        return Err("追加工具结果失败：第二条事件必须是 tool result".to_string());
    }
    let result_call_id = tool_result_event
        .get("tool_call_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "追加工具结果失败：tool result 缺少 tool_call_id".to_string())?;
    if !group_call_ids.iter().any(|tool_call_id| tool_call_id == result_call_id) {
        return Err(format!(
            "追加工具结果失败：tool_call_id 不在工具组内，group_tool_call_ids={}，result_tool_call_id={}",
            group_call_ids.join(","),
            result_call_id
        ));
    }
    Ok(result_call_id.to_string())
}

#[derive(Debug, Default)]
struct ConversationServiceV2;

#[derive(Debug, Default, Clone)]
struct ConversationExternalMetadataPatch {
    title: Option<String>,
    unread_count: Option<usize>,
    preferred_api_config_id: Option<Option<String>>,
    auto_push_remote_contact_id: Option<Option<String>>,
    current_todos: Option<Vec<ConversationTodoItem>>,
    plan_mode_enabled: Option<bool>,
    shell_workspace_path: Option<Option<String>>,
    shell_workspaces: Option<Vec<ShellWorkspaceConfig>>,
    shell_autonomous_mode: Option<bool>,
    lifecycle_status: Option<String>,
    lifecycle_summary: Option<String>,
    lifecycle_archived_at: Option<Option<String>>,
    lifecycle_updated_at: Option<String>,
    routing_department_id: Option<String>,
    routing_agent_id: Option<String>,
    routing_root_conversation_id: Option<Option<String>>,
    routing_conversation_kind: Option<String>,
}

fn conversation_service_v2() -> &'static ConversationServiceV2 {
    static SERVICE: OnceLock<ConversationServiceV2> = OnceLock::new();
    SERVICE.get_or_init(ConversationServiceV2::default)
}

#[cfg(test)]
fn conversation_service() -> &'static ConversationServiceV2 {
    conversation_service_v2()
}

impl ConversationServiceV2 {
    fn ensure_appendable_ready_message_store(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<message_store::ConversationShardMeta, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let store_paths =
            message_store::message_store_paths(&state.data_path, normalized_conversation_id)?;
        ensure_ready_message_store_from_legacy_conversation(
            state,
            normalized_conversation_id,
            &store_paths,
        )?;
        message_store::read_ready_message_store_meta(&store_paths)?
            .ok_or_else(|| {
                format!(
                    "会话消息仓库不可追加：缺少 ready 消息元数据，conversation_id={normalized_conversation_id}"
                )
            })
    }

    fn fill_summary_preview_messages_fallback(
        &self,
        state: &AppState,
        conversation_meta: &ConversationMetaView,
    ) -> ConversationMetaView {
        if !conversation_meta.preview_messages.is_empty() {
            return conversation_meta.clone();
        }
        let fallback_messages = match self.get_recent_messages(state, &conversation_meta.id, 2) {
            Ok(messages) => messages,
            Err(_) => return conversation_meta.clone(),
        };
        if fallback_messages.is_empty() {
            return conversation_meta.clone();
        }
        let mut hydrated = conversation_meta.clone();
        hydrated.preview_messages = build_preview_messages_from_chat_messages(&fallback_messages, 2)
            .into_iter()
            .map(|message| ConversationMetaPreviewMessage {
                message_id: message.message_id,
                role: message.role,
                speaker_agent_id: message.speaker_agent_id,
                created_at: message.created_at,
                text_preview: message.text_preview,
                has_image: message.has_image,
                has_pdf: message.has_pdf,
                has_audio: message.has_audio,
                has_attachment: message.has_attachment,
            })
            .collect();
        if hydrated.message_count == 0 {
            hydrated.message_count = fallback_messages.len();
        }
        if hydrated.body_message_count == 0 {
            hydrated.body_message_count = fallback_messages
                .iter()
                .filter(|message| {
                    matches!(
                        message.role.trim().to_ascii_lowercase().as_str(),
                        "user" | "assistant"
                    )
                })
                .count();
        }
        if hydrated.body_text_length == 0 {
            hydrated.body_text_length = fallback_messages
                .iter()
                .flat_map(|message| message.parts.iter())
                .filter_map(|part| match part {
                    MessagePart::Text { text, .. } => Some(text.trim().chars().count()),
                    _ => None,
                })
                .sum();
        }
        if !hydrated.has_assistant_reply {
            hydrated.has_assistant_reply = fallback_messages
                .iter()
                .any(|message| message.role.trim().eq_ignore_ascii_case("assistant"));
        }
        hydrated
    }

    fn validate_overwrite_audit(
        &self,
        audit: &ConversationOverwriteAudit,
    ) -> Result<(), String> {
        if audit.job_id.trim().is_empty() {
            return Err("overwrite audit jobId is required.".to_string());
        }
        if audit.operator.trim().is_empty() {
            return Err("overwrite audit operator is required.".to_string());
        }
        if audit.reason.trim().is_empty() {
            return Err("overwrite audit reason is required.".to_string());
        }
        Ok(())
    }

    // 危险：这是遗留的整会话覆写入口，会直接按传入 snapshot 全量重写消息仓库。
    // 除导入/迁移这类“向空会话灌入外部快照”的过渡场景外，项目内一律禁止调用。
    // 普通聊天、补写、恢复、自愈、同步都必须使用原子增量接口，不能复用这个方法。
    // 后续目标：彻底移除此能力，连导入也改为按消息/按 block 增量写入。
    fn apply_privileged_snapshot_overwrite(
        &self,
        state: &AppState,
        audit: &ConversationOverwriteAudit,
        snapshot: &Conversation,
    ) -> Result<(), String> {
        self.validate_overwrite_audit(audit)?;
        if snapshot.id.trim().is_empty() {
            return Err("overwrite snapshot conversation.id is required.".to_string());
        }
        let _guard =
            lock_conversation_with_metrics(state, "conversation_v2_privileged_overwrite")?;
        runtime_log_info(format!(
            "[会话V2] 开始，任务=特批覆写会话，conversation_id={}，source={}，job_id={}，operator={}，reason={}，message_count={}",
            snapshot.id,
            audit.source.as_str(),
            audit.job_id,
            audit.operator,
            audit.reason,
            snapshot.messages.len()
        ));
        state_schedule_conversation_persist(state, snapshot)?;
        runtime_log_info(format!(
            "[会话V2] 完成，任务=特批覆写会话，conversation_id={}，source={}，job_id={}，operator={}，message_count={}",
            snapshot.id,
            audit.source.as_str(),
            audit.job_id,
            audit.operator,
            snapshot.messages.len()
        ));
        Ok(())
    }

    fn read_current_writable_assistant_message(
        &self,
        state: &AppState,
        conversation_id: &str,
        assistant_message_id: &str,
    ) -> Result<ChatMessage, String> {
        let target_message = self.get_message_by_id(state, conversation_id, assistant_message_id)?;
        if target_message.role.trim() != "assistant" {
            return Err(
                ConversationServiceV2Error::new(
                    ConversationServiceV2ErrorCode::MessageNotWritable,
                    format!(
                        "目标消息不是 assistant，conversationId={}，assistantMessageId={}",
                        conversation_id, assistant_message_id
                    ),
                )
                .into_string(),
            );
        }
        let recent_messages = self.get_recent_messages(state, conversation_id, 1)?;
        let Some(last_message) = recent_messages.last() else {
            return Err(
                ConversationServiceV2Error::new(
                    ConversationServiceV2ErrorCode::MessageNotFound,
                    format!(
                        "会话缺少可写尾消息，conversationId={}，assistantMessageId={}",
                        conversation_id, assistant_message_id
                    ),
                )
                .into_string(),
            );
        };
        if last_message.id.trim() != assistant_message_id {
            return Err(
                ConversationServiceV2Error::new(
                    ConversationServiceV2ErrorCode::MessageNotWritable,
                    format!(
                        "目标消息不是最后一个可写 assistant message，conversationId={}，assistantMessageId={}，tailMessageId={}",
                        conversation_id,
                        assistant_message_id,
                        last_message.id.trim()
                    ),
                )
                .into_string(),
            );
        }
        Ok(target_message)
    }

    fn conversation_has_active_chat_view(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> bool {
        let target = conversation_id.trim();
        if target.is_empty() {
            return false;
        }
        state
            .active_chat_view_bindings
            .lock()
            .map(|bindings| {
                bindings.values().any(|binding| {
                    let bound = binding.conversation_id.trim();
                    !bound.is_empty() && bound != "*" && bound == target
                })
            })
            .unwrap_or(false)
    }

    fn increment_conversation_unread_count_if_background(
        &self,
        state: &AppState,
        conversation: &mut Conversation,
        count: usize,
    ) {
        if count == 0 {
            return;
        }
        if self.conversation_has_active_chat_view(state, &conversation.id) {
            clear_conversation_unread_count(conversation);
        } else {
            increment_conversation_unread_count(conversation, count);
        }
        if let Err(err) = state_update_conversation_metadata_cached(
            state,
            &conversation.id,
            |cached| {
                cached.unread_count = conversation.unread_count;
                cached.updated_at = conversation.updated_at.clone();
                cached.last_user_at = conversation.last_user_at.clone();
                cached.last_assistant_at = conversation.last_assistant_at.clone();
                Ok(())
            },
        ) {
            runtime_log_warn(format!(
                "[会话未读] 警告，任务=同步未读数metadata缓存，会话ID={}，unread_count={}，error={}",
                conversation.id, conversation.unread_count, err
            ));
        }
    }

    fn persist_replaced_ready_message(
        &self,
        state: &AppState,
        conversation_id: &str,
        updated_message: &ChatMessage,
    ) -> Result<(), String> {
        let paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
        let _existing_shard_meta = message_store::read_ready_message_store_meta(&paths)?
            .ok_or_else(|| {
                ConversationServiceV2Error::new(
                    ConversationServiceV2ErrorCode::StorageCorrupted,
                    format!(
                        "ready 消息元数据缺失，conversationId={}，assistantMessageId={}",
                        conversation_id,
                        updated_message.id.trim()
                    ),
                )
                .into_string()
            })?;
        let updated_at = updated_message.created_at.clone();
        let last_assistant_at = Some(updated_at.clone());
        let (updated_meta_conversation, (), _) = state_update_conversation_metadata_cached(
            state,
            conversation_id,
            |cached| {
                cached.updated_at = updated_at.clone();
                cached.last_assistant_at = last_assistant_at.clone();
                Ok(())
            },
        )?;
        let current = self.get_conversation_snapshot(state, conversation_id)?;
        let mut updated_messages = current.messages.clone();
        let target_message_id = updated_message.id.trim();
        let Some(target_index) = updated_messages
            .iter()
            .position(|message| message.id.trim() == target_message_id)
        else {
            return Err(
                ConversationServiceV2Error::new(
                    ConversationServiceV2ErrorCode::StorageCorrupted,
                    format!(
                        "ready 消息替换失败：目标消息不存在，conversationId={}，assistantMessageId={}",
                        conversation_id,
                        updated_message.id.trim()
                    ),
                )
                .into_string(),
            );
        };
        updated_messages[target_index] = updated_message.clone();
        let rebuilt_conversation = Conversation {
            messages: updated_messages,
            ..updated_meta_conversation.clone()
        };
        let persist_meta = message_store::ConversationShardMeta::from_conversation(&rebuilt_conversation)
            .to_persist_meta();
        message_store::write_jsonl_snapshot_replaced_message_shard(
            &paths,
            &persist_meta,
            updated_message,
        )?;
        state_mark_conversation_metadata_direct_persisted(state, conversation_id)?;
        Ok(())
    }

    fn resolve_latest_foreground_conversation_id(
        &self,
        state: &AppState,
        agent_id: &str,
    ) -> Result<Option<String>, String> {
        let normalized_agent_id = agent_id.trim();
        let chat_index = state_read_chat_index_cached(state)?;
        Ok(chat_index
            .conversations
            .iter()
            .rev()
            .find_map(|item| {
                if !item.summary.trim().is_empty() {
                    return None;
                }
                let conversation_meta = self.get_conversation_meta(state, &item.id).ok()?;
                if !conversation_meta.visible_in_foreground_lists
                    || !self.conversation_meta_is_local_normal_chat_meta_view(&conversation_meta)
                {
                    return None;
                }
                if !normalized_agent_id.is_empty()
                    && conversation_meta.agent_id.trim() != normalized_agent_id
                {
                    return None;
                }
                Some(conversation_meta.id.to_string())
            }))
    }

    fn with_unarchived_conversation_by_id_fast<T>(
        &self,
        state: &AppState,
        conversation_id: &str,
        reader: impl FnOnce(&Conversation) -> Result<T, String>,
    ) -> Result<T, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let conversation = self.read_persisted_conversation(state, normalized_conversation_id)
            .map_err(|err| {
                format!(
                    "Unarchived conversation not found: {normalized_conversation_id}: {err}"
                )
            })?;
        self.ensure_unarchived_conversation(&conversation, normalized_conversation_id)?;
        let result = reader(&conversation)?;
        drop(guard);
        Ok(result)
    }

    fn apply_external_metadata_patch(
        &self,
        state: &AppState,
        conversation_id: &str,
        task_name: &str,
        patch: ConversationExternalMetadataPatch,
    ) -> Result<Conversation, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let _guard = lock_conversation_with_metrics(state, task_name)?;
        let (conversation, (), _) = state_update_conversation_metadata_cached(
            state,
            normalized_conversation_id,
            |conversation| {
                if let Some(value) = patch.title {
                    conversation.title = value;
                }
                if let Some(value) = patch.unread_count {
                    conversation.unread_count = value;
                }
                if let Some(value) = patch.preferred_api_config_id {
                    conversation.preferred_api_config_id = value;
                }
                if let Some(value) = patch.auto_push_remote_contact_id {
                    conversation.auto_push_remote_contact_id = value;
                }
                if let Some(value) = patch.current_todos {
                    conversation.current_todos = value;
                }
                if let Some(value) = patch.plan_mode_enabled {
                    conversation.plan_mode_enabled = value;
                }
                if let Some(value) = patch.shell_workspace_path {
                    conversation.shell_workspace_path = value;
                }
                if let Some(value) = patch.shell_workspaces {
                    conversation.shell_workspaces = value;
                }
                if let Some(value) = patch.shell_autonomous_mode {
                    conversation.shell_autonomous_mode = value;
                }
                if let Some(value) = patch.lifecycle_status {
                    conversation.status = value;
                }
                if let Some(value) = patch.lifecycle_summary {
                    conversation.summary = value;
                }
                if let Some(value) = patch.lifecycle_archived_at {
                    conversation.archived_at = value;
                }
                if let Some(value) = patch.lifecycle_updated_at {
                    conversation.updated_at = value;
                }
                if let Some(value) = patch.routing_department_id {
                    conversation.department_id = value;
                }
                if let Some(value) = patch.routing_agent_id {
                    conversation.agent_id = value;
                }
                if let Some(value) = patch.routing_root_conversation_id {
                    conversation.root_conversation_id = value;
                }
                if let Some(value) = patch.routing_conversation_kind {
                    conversation.conversation_kind = value;
                }
                if conversation
                    .shell_workspace_path
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .is_some()
                    && terminal_workspace_path_from_conversation(state, conversation).is_none()
                {
                    conversation.shell_workspace_path = None;
                }
                Ok(())
            },
        )?;
        Ok(conversation)
    }

    fn get_conversation_meta(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<ConversationMetaView, String> {
        let meta = state_read_conversation_metadata_cached(state, conversation_id).map_err(|_| {
            ConversationServiceV2Error::new(
                ConversationServiceV2ErrorCode::ConversationNotFound,
                format!("conversationId={}", conversation_id.trim()),
            )
            .into_string()
        })?;
        Ok(ConversationMetaView::from_meta(&meta))
    }

    fn ensure_system_notification_conversation(
        &self,
        state: &AppState,
    ) -> Result<(), String> {
        let exists = self
            .get_conversation_meta(state, SYSTEM_NOTIFICATION_CONVERSATION_ID)
            .ok()
            .filter(|conversation_meta| {
                self.conversation_meta_is_unarchived_meta_view(conversation_meta)
                    && self.conversation_meta_is_system_notification_meta_view(conversation_meta)
            })
            .is_some();
        if exists {
            return Ok(());
        }
        let conversation = build_system_notification_conversation_record();
        state_schedule_conversation_persist(state, &conversation)?;
        Ok(())
    }

    fn conversation_meta_is_delegate(
        &self,
        conversation_meta: &message_store::ConversationShardMeta,
    ) -> bool {
        conversation_meta.conversation_kind().trim() == CONVERSATION_KIND_DELEGATE
    }

    fn conversation_meta_is_system_notification_meta_view(
        &self,
        conversation_meta: &ConversationMetaView,
    ) -> bool {
        conversation_meta.id.trim() == SYSTEM_NOTIFICATION_CONVERSATION_ID
            || conversation_meta.conversation_kind.trim()
                == CONVERSATION_KIND_SYSTEM_NOTIFICATION
    }

    fn conversation_meta_is_unarchived_meta_view(
        &self,
        conversation_meta: &ConversationMetaView,
    ) -> bool {
        conversation_meta.status.trim() != "archived"
            && conversation_meta
                .archived_at
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
            && conversation_meta.summary.trim().is_empty()
    }

    fn conversation_meta_visible_in_foreground_lists(
        &self,
        conversation_meta: &message_store::ConversationShardMeta,
    ) -> bool {
        !self.conversation_meta_is_delegate(conversation_meta)
            && !self.conversation_meta_is_remote_im_contact(conversation_meta)
    }

    fn conversation_meta_is_remote_im_contact(
        &self,
        conversation_meta: &message_store::ConversationShardMeta,
    ) -> bool {
        conversation_meta.conversation_kind().trim() == CONVERSATION_KIND_REMOTE_IM_CONTACT
    }

    fn conversation_meta_is_local_normal_chat_meta_view(
        &self,
        conversation_meta: &ConversationMetaView,
    ) -> bool {
        self.conversation_meta_is_unarchived_meta_view(conversation_meta)
            && conversation_meta.visible_in_foreground_lists
            && conversation_meta.conversation_kind.trim() == CONVERSATION_KIND_CHAT
            && conversation_meta.conversation_kind.trim()
                != CONVERSATION_KIND_SYSTEM_NOTIFICATION
    }

    fn build_conversation_snapshot_from_meta(
        &self,
        conversation_meta: &message_store::ConversationShardMeta,
        messages: Vec<ChatMessage>,
    ) -> Conversation {
        let mut conversation = build_conversation_record("", "", "", "", "", None, None);
        conversation.id = conversation_meta.id().to_string();
        conversation_meta.apply_to_conversation(&mut conversation);
        conversation.messages = messages;
        conversation
    }

    fn build_conversation_record_from_meta_view(
        &self,
        conversation_meta: &ConversationMetaView,
    ) -> Conversation {
        let mut conversation = build_conversation_record("", "", "", "", "", None, None);
        conversation.id = conversation_meta.id.clone();
        conversation.title = conversation_meta.title.clone();
        conversation.agent_id = conversation_meta.agent_id.clone();
        conversation.department_id = conversation_meta.department_id.clone();
        conversation.unread_count = conversation_meta.unread_count;
        conversation.conversation_kind = conversation_meta.conversation_kind.clone();
        conversation.root_conversation_id = conversation_meta.root_conversation_id.clone();
        conversation.delegate_id = conversation_meta.delegate_id.clone();
        conversation.created_at = conversation_meta.created_at.clone();
        conversation.updated_at = conversation_meta.updated_at.clone();
        conversation.last_user_at = conversation_meta.last_user_at.clone();
        conversation.last_assistant_at = conversation_meta.last_assistant_at.clone();
        conversation.status = conversation_meta.status.clone();
        conversation.summary = conversation_meta.summary.clone();
        conversation.user_profile_snapshot = conversation_meta.user_profile_snapshot.clone();
        conversation.shell_workspace_path = conversation_meta.shell_workspace_path.clone();
        conversation.shell_workspaces = conversation_meta.shell_workspaces.clone();
        conversation.shell_autonomous_mode = conversation_meta.shell_autonomous_mode;
        conversation.archived_at = conversation_meta.archived_at.clone();
        conversation.current_todos = conversation_meta.current_todos.clone();
        conversation.plan_mode_enabled = conversation_meta.plan_mode_enabled;
        conversation.preferred_api_config_id =
            conversation_meta.preferred_api_config_id.clone();
        conversation.auto_push_remote_contact_id =
            conversation_meta.auto_push_remote_contact_id.clone();
        conversation.cumulative_usage = conversation_meta.cumulative_usage.clone();
        conversation.active_goal = conversation_meta.active_goal.clone();
        conversation
    }

    fn ensure_unarchived_conversation(
        &self,
        conversation: &Conversation,
        conversation_id: &str,
    ) -> Result<(), String> {
        if !conversation_is_unarchived(conversation) {
            return Err(format!(
                "Unarchived conversation not found: {}",
                conversation_id.trim()
            ));
        }
        Ok(())
    }

    fn read_persisted_conversation(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Conversation, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let conversation_meta =
            self.get_conversation_meta(state, normalized_conversation_id)?;
        let store_paths =
            message_store::message_store_paths(&state.data_path, normalized_conversation_id)?;
        ensure_ready_message_store_from_legacy_conversation(
            state,
            normalized_conversation_id,
            &store_paths,
        )?;
        let messages =
            message_store::read_ready_message_store_all_messages(&store_paths)?.unwrap_or_default();
        let mut conversation = self.build_conversation_record_from_meta_view(&conversation_meta);
        conversation.messages = messages;
        Ok(conversation)
    }

    fn read_archive_pipeline_source_conversation(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Conversation, String> {
        self.read_persisted_conversation(state, conversation_id)
    }

    fn read_archive_pipeline_last_block_conversation(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Conversation, String> {
        let source = self.read_persisted_conversation(state, conversation_id)?;
        let store_paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
        let mut block_messages =
            if let Some(page) = message_store::read_ready_message_store_block_page(&store_paths, None)? {
                page.messages
            } else {
                source.messages.clone()
            };
        materialize_chat_message_parts_from_media_refs(&mut block_messages, &state.data_path);
        let mut last_block = source.clone();
        last_block.messages = block_messages;
        Ok(last_block)
    }

    fn try_read_persisted_conversation(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Option<Conversation>, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Ok(None);
        }
        match self.read_persisted_conversation(state, normalized_conversation_id) {
            Ok(conversation) => Ok(Some(conversation)),
            Err(err) if err.contains("not found") || err.contains("不存在") => Ok(None),
            Err(err) => Err(err),
        }
    }

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
                let conversation_meta = match self.get_conversation_meta(state, item.id.as_str()) {
                    Ok(conversation_meta) => conversation_meta,
                    Err(err) => {
                        eprintln!(
                            "[会话索引读取] 状态=失败，任务=collect_unarchived_conversation_summaries_cached，conversation_id={}，error={}",
                            item.id, err
                        );
                        return None;
                    }
                };
                (self.conversation_meta_is_unarchived_meta_view(&conversation_meta)
                    && conversation_meta.visible_in_foreground_lists)
                    .then_some(conversation_meta)
            })
            .collect::<Vec<_>>();
        let visible_ids = visible_conversations
            .iter()
            .map(|conversation_meta| conversation_meta.id.trim().to_string())
            .filter(|conversation_id: &String| !conversation_id.is_empty())
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
            .map(|conversation_meta| {
                let hydrated_conversation_meta =
                    self.fill_summary_preview_messages_fallback(state, conversation_meta);
                build_unarchived_conversation_summary_from_meta_view(
                    state,
                    &runtime_app_config,
                    &main_conversation_id,
                    &pinned_conversation_ids,
                    &hydrated_conversation_meta,
                    Some(DESKTOP_CHAT_VIEWER_ID),
                )
            })
            .collect::<Vec<_>>();
        Ok(sort_unarchived_conversation_summaries(summaries))
    }

    fn set_conversation_unread_count_metadata(
        &self,
        state: &AppState,
        conversation_id: &str,
        unread_count: usize,
    ) -> Result<Conversation, String> {
        self.apply_external_metadata_patch(
            state,
            conversation_id,
            "conversation_v2_set_conversation_unread_count_metadata",
            ConversationExternalMetadataPatch {
                unread_count: Some(unread_count),
                ..Default::default()
            },
        )
    }

    fn get_chat_snapshot(
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
                    latest_user: latest_user.map(frontend_project_message),
                    latest_assistant: latest_assistant.map(frontend_project_message),
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
                            latest_user: latest_user.map(frontend_project_message),
                            latest_assistant: latest_assistant.map(frontend_project_message),
                            active_message_count: conversation.messages.len(),
                        }
                    })
            };
            if let Some(snapshot) = snapshot {
                return Ok(snapshot);
            }
        }

        let _guard = state.conversation_lock.lock().map_err(|err| {
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
        let runtime_snapshot = build_runtime_organization_snapshot_from_parts(
            &state.data_path,
            &mut app_config,
            &agents,
        )?;
        let runtime_agents = runtime_snapshot.agents;
        let requested_agent_id = input.agent_id.trim();
        let effective_agent_id = if !requested_agent_id.is_empty() {
            if runtime_agents
                .iter()
                .any(|agent| agent.id == requested_agent_id && !agent.is_built_in_user)
            {
                requested_agent_id.to_string()
            } else {
                return Err(format!("Selected agent '{requested_agent_id}' not found."));
            }
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
                    latest_user: latest_user.map(frontend_project_message),
                    latest_assistant: latest_assistant.map(frontend_project_message),
                    active_message_count: snapshot.active_message_count,
                });
            }
        }

        Ok(ChatSnapshot {
            conversation_id: String::new(),
            latest_user: None,
            latest_assistant: None,
            active_message_count: 0,
        })
    }

    fn get_conversation_snapshot(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Conversation, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let conversation_meta =
            self.get_conversation_meta(state, normalized_conversation_id)?;
        let store_paths =
            message_store::message_store_paths(&state.data_path, normalized_conversation_id)?;
        ensure_ready_message_store_from_legacy_conversation(
            state,
            normalized_conversation_id,
            &store_paths,
        )?;
        let messages =
            message_store::read_ready_message_store_all_messages(&store_paths)?.unwrap_or_default();
        let mut conversation = self.build_conversation_record_from_meta_view(&conversation_meta);
        conversation.messages = messages;
        Ok(conversation)
    }

    fn try_get_conversation_snapshot(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Option<Conversation>, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Ok(None);
        }
        match self.get_conversation_snapshot(state, normalized_conversation_id) {
            Ok(conversation) => Ok(Some(conversation)),
            Err(err) if err.contains("not found") || err.contains("不存在") => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn get_conversation_last_block(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<ConversationBlockPageResult, String> {
        self.get_conversation_block(state, conversation_id, 0)
    }

    fn get_conversation_block(
        &self,
        state: &AppState,
        conversation_id: &str,
        block_id: u32,
    ) -> Result<ConversationBlockPageResult, String> {
        self.with_unarchived_conversation_by_id_fast(state, conversation_id, |conversation| {
            let store_paths = message_store::message_store_paths(&state.data_path, &conversation.id)?;
            if let Some(page) =
                message_store::read_ready_message_store_block_page(&store_paths, Some(block_id))?
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
        })
    }

    fn get_recent_messages(
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
            page.messages
        } else {
            self.with_unarchived_conversation_by_id_fast(state, conversation_id, |conversation| {
                let total = conversation.messages.len();
                let start = total.saturating_sub(normalized_limit);
                Ok(conversation.messages[start..].to_vec())
            })?
        };
        materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
        Ok(frontend_project_messages(messages))
    }

    fn get_all_messages(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Vec<ChatMessage>, String> {
        let mut messages =
            self.with_unarchived_conversation_by_id_fast(state, conversation_id, |conversation| {
                Ok(conversation.messages.clone())
            })?;
        materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
        Ok(frontend_project_messages(messages))
    }

    fn get_recent_block_messages(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Vec<ChatMessage>, String> {
        let store_paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
        let mut messages = if let Some(page) =
            message_store::read_ready_message_store_recent_messages_page_cached(
                &store_paths,
                DEFAULT_FOREGROUND_SNAPSHOT_RECENT_LIMIT,
            )?
        {
            page.messages
        } else {
            self.with_unarchived_conversation_by_id_fast(state, conversation_id, |conversation| {
                let total = conversation.messages.len();
                let start = total.saturating_sub(DEFAULT_FOREGROUND_SNAPSHOT_RECENT_LIMIT);
                Ok(conversation.messages[start..].to_vec())
            })?
        };
        materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
        Ok(frontend_project_messages(messages))
    }

    fn get_active_conversation_messages(
        &self,
        state: &AppState,
        input: &SessionSelector,
    ) -> Result<Vec<ChatMessage>, String> {
        let Some(conversation_id) = self.resolve_session_conversation_id_fast(state, input)? else {
            return Ok(Vec::new());
        };
        self.get_all_messages(state, &conversation_id)
    }

    fn get_message_by_id(
        &self,
        state: &AppState,
        conversation_id: &str,
        message_id: &str,
    ) -> Result<ChatMessage, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let normalized_message_id = message_id.trim();
        if normalized_message_id.is_empty() {
            return Err("messageId is required.".to_string());
        }
        let store_paths =
            message_store::message_store_paths(&state.data_path, normalized_conversation_id)?;
        ensure_ready_message_store_from_legacy_conversation(
            state,
            normalized_conversation_id,
            &store_paths,
        )?;
        let mut message =
            message_store::read_ready_message_store_message_by_id(&store_paths, normalized_message_id)?
                .ok_or_else(|| format!("Message not found: {normalized_message_id}"))?;
        materialize_chat_message_parts_from_media_refs(
            std::slice::from_mut(&mut message),
            &state.data_path,
        );
        Ok(frontend_project_message(message))
    }

    fn read_messages_before_internal(
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
        Ok((frontend_project_messages(page), has_more))
    }

    fn read_messages_after_internal(
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
        Ok(frontend_project_messages(page))
    }

    fn get_messages_before(
        &self,
        state: &AppState,
        conversation_id: &str,
        anchor_message_id: &str,
        limit: usize,
    ) -> Result<ConversationMessagePageView, String> {
        let session = SessionSelector {
            api_config_id: None,
            department_id: None,
            agent_id: String::new(),
            conversation_id: Some(conversation_id.trim().to_string()),
        };
        let (messages, has_more) = self.read_messages_before_internal(
            state,
            &session,
            anchor_message_id,
            limit,
        )?;
        Ok(build_message_page_view_v2(messages, has_more, false))
    }

    fn get_messages_before_from_session(
        &self,
        state: &AppState,
        session: &SessionSelector,
        before_message_id: &str,
        limit: usize,
    ) -> Result<ConversationMessagePageView, String> {
        let (messages, has_more) =
            self.read_messages_before_internal(state, session, before_message_id, limit)?;
        Ok(build_message_page_view_v2(messages, has_more, false))
    }

    fn get_messages_after(
        &self,
        state: &AppState,
        conversation_id: &str,
        anchor_message_id: &str,
        limit: usize,
    ) -> Result<ConversationMessagePageView, String> {
        let session = SessionSelector {
            api_config_id: None,
            department_id: None,
            agent_id: String::new(),
            conversation_id: Some(conversation_id.trim().to_string()),
        };
        let messages = self.read_messages_after_internal(
            state,
            &session,
            anchor_message_id,
            limit,
        )?;
        let has_more_after = messages.len() >= limit.clamp(1, 100);
        Ok(build_message_page_view_v2(
            messages,
            false,
            has_more_after,
        ))
    }

    fn get_messages_after_from_session(
        &self,
        state: &AppState,
        session: &SessionSelector,
        after_message_id: &str,
        limit: usize,
    ) -> Result<ConversationMessagePageView, String> {
        let messages =
            self.read_messages_after_internal(state, session, after_message_id, limit)?;
        let has_more_after = messages.len() >= limit.clamp(1, 100);
        Ok(build_message_page_view_v2(messages, false, has_more_after))
    }

    fn get_messages_after_with_fallback(
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
                    let page_result = if let Some(after_idx) =
                        messages.iter().position(|item| item.id == after_id)
                    {
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
        Ok((frontend_project_messages(page), fallback_mode))
    }

    fn append_message(
        &self,
        state: &AppState,
        conversation_id: &str,
        message: &ChatMessage,
    ) -> Result<(), String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let conversation_meta =
            self.get_conversation_meta(state, normalized_conversation_id)?;
        if !self.conversation_meta_is_unarchived_meta_view(&conversation_meta) {
            return Err(format!(
                "Unarchived conversation not found: {}",
                normalized_conversation_id
            ));
        }
        let store_paths =
            message_store::message_store_paths(&state.data_path, normalized_conversation_id)?;
        let updated_at = message.created_at.clone();
        let last_user_at = if message.role.trim() == "user" {
            Some(message.created_at.clone())
        } else {
            conversation_meta.last_user_at.clone()
        };
        let last_assistant_at = if message.role.trim() == "assistant" {
            Some(message.created_at.clone())
        } else {
            conversation_meta.last_assistant_at.clone()
        };
        let unread_count = if self.conversation_has_active_chat_view(state, normalized_conversation_id)
            || conversation_meta.conversation_kind.trim() == CONVERSATION_KIND_REMOTE_IM_CONTACT
        {
            0
        } else {
            conversation_meta.unread_count.saturating_add(1)
        };
        let (_, (), _) = state_update_conversation_metadata_cached(
            state,
            normalized_conversation_id,
            |cached| {
                cached.unread_count = unread_count;
                cached.updated_at = updated_at.clone();
                cached.last_user_at = last_user_at.clone();
                cached.last_assistant_at = last_assistant_at.clone();
                Ok(())
            },
        )?;
        let mut updated_meta_view = conversation_meta.clone();
        updated_meta_view.unread_count = unread_count;
        updated_meta_view.updated_at = updated_at.clone();
        updated_meta_view.last_user_at = last_user_at.clone();
        updated_meta_view.last_assistant_at = last_assistant_at.clone();
        let mut ready_meta = self
            .ensure_appendable_ready_message_store(state, normalized_conversation_id)?;
        ready_meta.apply_metadata_fields_from_meta_view(&updated_meta_view);
        ready_meta.apply_appended_messages(std::slice::from_ref(message));
        message_store::write_jsonl_snapshot_appended_messages_shard_from_meta(
            &store_paths,
            &ready_meta,
            std::slice::from_ref(message),
        )?;
        state_mark_conversation_metadata_direct_persisted(state, normalized_conversation_id)?;
        drop(guard);
        Ok(())
    }

    fn append_messages(
        &self,
        state: &AppState,
        conversation_id: &str,
        messages: &[ChatMessage],
    ) -> Result<(), String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        if messages.is_empty() {
            return Ok(());
        }
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let conversation_meta =
            self.get_conversation_meta(state, normalized_conversation_id)?;
        if !self.conversation_meta_is_unarchived_meta_view(&conversation_meta) {
            return Err(format!(
                "Unarchived conversation not found: {}",
                normalized_conversation_id
            ));
        }
        let store_paths =
            message_store::message_store_paths(&state.data_path, normalized_conversation_id)?;
        let last_message = messages
            .last()
            .ok_or_else(|| "messages is empty".to_string())?;
        let updated_at = last_message.created_at.clone();
        let last_user_at = if last_message.role.trim() == "user" {
            Some(last_message.created_at.clone())
        } else {
            conversation_meta.last_user_at.clone()
        };
        let last_assistant_at = if last_message.role.trim() == "assistant" {
            Some(last_message.created_at.clone())
        } else {
            conversation_meta.last_assistant_at.clone()
        };
        let unread_count = if self.conversation_has_active_chat_view(state, normalized_conversation_id)
            || conversation_meta.conversation_kind.trim() == CONVERSATION_KIND_REMOTE_IM_CONTACT
        {
            0
        } else {
            conversation_meta.unread_count.saturating_add(messages.len())
        };
        let (_, (), _) = state_update_conversation_metadata_cached(
            state,
            normalized_conversation_id,
            |cached| {
                cached.unread_count = unread_count;
                cached.updated_at = updated_at.clone();
                cached.last_user_at = last_user_at.clone();
                cached.last_assistant_at = last_assistant_at.clone();
                Ok(())
            },
        )?;
        let mut updated_meta_view = conversation_meta.clone();
        updated_meta_view.unread_count = unread_count;
        updated_meta_view.updated_at = updated_at.clone();
        updated_meta_view.last_user_at = last_user_at.clone();
        updated_meta_view.last_assistant_at = last_assistant_at.clone();
        let mut ready_meta = self
            .ensure_appendable_ready_message_store(state, normalized_conversation_id)?;
        ready_meta.apply_metadata_fields_from_meta_view(&updated_meta_view);
        ready_meta.apply_appended_messages(messages);
        message_store::write_jsonl_snapshot_appended_messages_shard_from_meta(
            &store_paths,
            &ready_meta,
            messages,
        )?;
        state_mark_conversation_metadata_direct_persisted(state, normalized_conversation_id)?;
        drop(guard);
        Ok(())
    }

    fn build_forward_selection_notification_message(
        &self,
        state: &AppState,
        source_conversation_id: &str,
        selected_messages: &[ChatMessage],
    ) -> Result<ChatMessage, String> {
        let content = selected_messages_notification_content(selected_messages);
        let body = build_session_notification_body(state, source_conversation_id, &content)?;
        Ok(build_session_notification_message(&body))
    }

    fn append_user_message(
        &self,
        state: &AppState,
        input: &UserMessageAppendInput,
    ) -> Result<(), String> {
        let conversation_id = input.conversation_id.trim();
        if conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        if input.message.role.trim() != "user" {
            return Err("append_user_message 只允许 user message".to_string());
        }
        let memory_recall_ids = dedup_memory_recall_ids_v2(&input.memory_recall_ids);
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let conversation_meta = self.get_conversation_meta(state, conversation_id)?;
        if !self.conversation_meta_is_unarchived_meta_view(&conversation_meta) {
            drop(guard);
            return Err(format!("Unarchived conversation not found: {conversation_id}"));
        }
        let store_paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
        let updated_at = input.message.created_at.clone();
        let unread_count = if self.conversation_has_active_chat_view(state, conversation_id)
            || conversation_meta.conversation_kind.trim() == CONVERSATION_KIND_REMOTE_IM_CONTACT
        {
            0
        } else {
            conversation_meta.unread_count.saturating_add(1)
        };
        let (updated_meta_conversation, (), _) = state_update_conversation_metadata_cached(
            state,
            conversation_id,
            |cached| {
                cached.unread_count = unread_count;
                cached.updated_at = updated_at.clone();
                cached.last_user_at = Some(updated_at.clone());
                if !memory_recall_ids.is_empty() {
                    for memory_id in &memory_recall_ids {
                        if !cached.memory_recall_table.iter().any(|item| item == memory_id) {
                            cached.memory_recall_table.push(memory_id.clone());
                        }
                    }
                }
                Ok(())
            },
        )?;
        let mut ready_meta = self.ensure_appendable_ready_message_store(state, conversation_id)?;
        ready_meta.apply_metadata_fields_from_conversation(&updated_meta_conversation);
        ready_meta.apply_appended_messages(std::slice::from_ref(&input.message));
        message_store::write_jsonl_snapshot_appended_messages_shard_from_meta(
            &store_paths,
            &ready_meta,
            std::slice::from_ref(&input.message),
        )?;
        state_mark_conversation_metadata_direct_persisted(state, conversation_id)?;
        drop(guard);
        Ok(())
    }

    fn increment_unread_count_if_background(
        &self,
        state: &AppState,
        conversation: &mut Conversation,
        count: usize,
    ) {
        self.increment_conversation_unread_count_if_background(state, conversation, count)
    }

    fn create_remote_im_contact_conversation(
        &self,
        state: &AppState,
        title: &str,
        department_id: &str,
        agent_id: &str,
        root_conversation_id: &str,
    ) -> Result<Conversation, String> {
        let normalized_title = title.trim();
        let normalized_department_id = department_id.trim();
        let normalized_agent_id = agent_id.trim();
        let normalized_root_conversation_id = root_conversation_id.trim();
        if normalized_title.is_empty() {
            return Err("title is required.".to_string());
        }
        if normalized_department_id.is_empty() {
            return Err("departmentId is required.".to_string());
        }
        if normalized_agent_id.is_empty() {
            return Err("agentId is required.".to_string());
        }
        if normalized_root_conversation_id.is_empty() {
            return Err("rootConversationId is required.".to_string());
        }
        let _guard = lock_conversation_with_metrics(
            state,
            "conversation_v2_create_remote_im_contact_conversation",
        )?;
        let mut conversation = build_conversation_record(
            "",
            normalized_agent_id,
            normalized_department_id,
            normalized_title,
            CONVERSATION_KIND_REMOTE_IM_CONTACT,
            Some(normalized_root_conversation_id.to_string()),
            None,
        );
        conversation.status = "inactive".to_string();
        state_schedule_conversation_persist(state, &conversation)?;
        Ok(conversation)
    }

    fn create_conversation(
        &self,
        state: &AppState,
        input: &CreateUnarchivedConversationInput,
    ) -> Result<CreateUnarchivedConversationMutationResult, String> {
        create_unarchived_conversation_shared(state, input)
    }

    fn switch_active_conversation_snapshot(
        &self,
        state: &AppState,
        input: &SwitchActiveConversationSnapshotInput,
    ) -> Result<SwitchActiveConversationSnapshotMutationResult, String> {
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let mut app_config = state_read_config_cached(state)?;
        let agents = state_read_agents_cached(state)?;
        let mut runtime = state_read_runtime_state_cached(state)?;
        let _effective_agent_id = self.resolve_effective_agent_id_for_read(
            state,
            &mut app_config,
            &agents,
            &runtime.assistant_department_agent_id,
            input.agent_id.as_deref().unwrap_or_default(),
        )?;
        let requested_conversation_id = input
            .conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let (target_conversation_meta, target_conversation, created_new_conversation) =
            if let Some(conversation_id) = requested_conversation_id {
                let conversation_meta = self.get_conversation_meta(state, conversation_id)
                    .ok()
                    .filter(|conversation_meta| {
                        self.conversation_meta_is_unarchived_meta_view(conversation_meta)
                            && conversation_meta.visible_in_foreground_lists
                    })
                    .ok_or_else(|| {
                        format!("Requested conversation not found: {conversation_id}")
                    })?;
                (Some(conversation_meta), None, false)
            } else if let Some(conversation_meta) = runtime
                .main_conversation_id
                .as_deref()
                .and_then(|conversation_id| {
                    self.get_conversation_meta(state, conversation_id.trim()).ok()
                })
                .filter(|conversation_meta| {
                    self.conversation_meta_is_unarchived_meta_view(conversation_meta)
                        && conversation_meta.visible_in_foreground_lists
                })
            {
                (Some(conversation_meta), None, false)
            } else if runtime.main_conversation_id.as_deref().map(str::trim)
                == Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
            {
                let conversation = build_system_notification_conversation_record();
                (None, Some(conversation), true)
            } else if let Some(conversation_meta) =
                read_latest_visible_foreground_conversation_metadata(state)?
            {
                (Some(conversation_meta), None, false)
            } else {
                let conversation = build_system_notification_conversation_record();
                (None, Some(conversation), true)
            };
        let target_conversation_id = target_conversation_meta
            .as_ref()
            .map(|conversation_meta| conversation_meta.id.to_string())
            .or_else(|| target_conversation.as_ref().map(|conversation| conversation.id.clone()))
            .ok_or_else(|| "Requested conversation not found.".to_string())?;
        ensure_unarchived_conversation_not_organizing(state, &target_conversation_id)?;
        let unread_changed = target_conversation_meta
            .as_ref()
            .map(|conversation_meta| conversation_meta.unread_count > 0)
            .unwrap_or(false);
        clear_conversation_list_activity_mark(state, &target_conversation_id);
        if unread_changed && !created_new_conversation {
            state_update_conversation_metadata_cached(
                state,
                &target_conversation_id,
                |conversation| {
                    conversation.unread_count = 0;
                    Ok(())
                },
            )?;
        }
        if created_new_conversation {
            let conversation = target_conversation
                .as_ref()
                .ok_or_else(|| "Requested conversation not found.".to_string())?;
            state_schedule_conversation_persist(state, conversation)?;
        }
        if target_conversation_meta
            .as_ref()
            .map(|conversation_meta| self.conversation_meta_is_system_notification_meta_view(conversation_meta))
            .or_else(|| {
                target_conversation
                    .as_ref()
                    .map(conversation_is_system_notification)
            })
            .unwrap_or(false)
            && runtime.main_conversation_id.as_deref().map(str::trim)
                != Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
        {
            runtime.main_conversation_id = Some(SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string());
            state_write_runtime_state_cached(state, &runtime)?;
        }
        let snapshot = if let Some(conversation_meta) = target_conversation_meta.as_ref() {
            build_foreground_conversation_snapshot_from_meta_view(
                state,
                conversation_meta,
                DEFAULT_FOREGROUND_SNAPSHOT_RECENT_LIMIT,
            )?
        } else {
            build_foreground_conversation_snapshot_from_conversation(
                state,
                target_conversation
                    .as_ref()
                    .ok_or_else(|| "Requested conversation not found.".to_string())?,
                DEFAULT_FOREGROUND_SNAPSHOT_RECENT_LIMIT,
            )?
        };
        let unarchived_conversations =
            self.collect_unarchived_conversation_summaries_cached(state, &app_config)?;
        drop(guard);

        let mut materialized_snapshot = snapshot;
        materialize_chat_message_parts_from_media_refs(
            &mut materialized_snapshot.messages,
            &state.data_path,
        );
        Ok(SwitchActiveConversationSnapshotMutationResult {
            snapshot: materialized_snapshot,
            unarchived_conversations,
        })
    }

    fn get_foreground_snapshot(
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
            let conversation_meta = self.get_conversation_meta(state, &conversation_id)?;
            if self.conversation_meta_is_unarchived_meta_view(&conversation_meta)
                && (conversation_meta.visible_in_foreground_lists
                    || conversation_meta.is_remote_im_contact)
            {
                ensure_unarchived_conversation_not_organizing(state, &conversation_meta.id)?;
                build_foreground_conversation_snapshot_from_meta_view(
                    state,
                    &conversation_meta,
                    recent_limit,
                )?
            } else {
                return Err(format!(
                    "Conversation not available for chat view: {}",
                    conversation_id
                ));
            }
        } else if let Some(main_conversation_id) = state_read_runtime_state_cached(state)?
            .main_conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
        {
            let conversation_meta = self.get_conversation_meta(state, &main_conversation_id)?;
            ensure_unarchived_conversation_not_organizing(state, &conversation_meta.id)?;
            if !self.conversation_meta_is_unarchived_meta_view(&conversation_meta) {
                return Err(format!(
                    "Unarchived conversation not found: {}",
                    main_conversation_id
                ));
            }
            build_foreground_conversation_snapshot_from_meta_view(
                state,
                &conversation_meta,
                recent_limit,
            )?
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
                let conversation_meta = self.get_conversation_meta(state, &target_conversation_id)?;
                build_foreground_conversation_snapshot_from_meta_view(
                    state,
                    &conversation_meta,
                    recent_limit,
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
        snapshot.messages = frontend_project_messages(snapshot.messages);
        Ok(snapshot)
    }

    fn mark_conversation_read(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<MarkConversationReadResult, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Ok(MarkConversationReadResult {
                conversation: None,
            });
        }
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let conversation_meta = match self.get_conversation_meta(state, normalized_conversation_id) {
            Ok(conversation_meta) => conversation_meta,
            Err(err) => {
                runtime_log_debug(format!(
                    "[会话已读] 读取会话失败，conversation_id={}，error={}",
                    normalized_conversation_id, err
                ));
                drop(guard);
                return Ok(MarkConversationReadResult {
                    conversation: None,
                });
            }
        };
        if conversation_meta.unread_count == 0 {
            drop(guard);
            return Ok(MarkConversationReadResult {
                conversation: Some(self.build_conversation_record_from_meta_view(
                    &conversation_meta,
                )),
            });
        }
        drop(guard);
        let result_conversation =
            self.set_conversation_unread_count_metadata(state, normalized_conversation_id, 0)?;
        Ok(MarkConversationReadResult {
            conversation: Some(result_conversation),
        })
    }

    fn delete_conversation(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<DeleteUnarchivedConversationMutationResult, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let app_config = state_read_config_cached(state)?;
        let runtime = state_read_runtime_state_cached(state)?;
        let main_conversation_id = runtime
            .main_conversation_id
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_string();
        if normalized_conversation_id == main_conversation_id {
            drop(guard);
            return Err("系统通知会话暂不支持删除".to_string());
        }
        let conversation = self.get_conversation_meta(state, normalized_conversation_id).ok();
        if conversation
            .as_ref()
            .map(|conversation| self.conversation_meta_is_system_notification_meta_view(conversation))
            .unwrap_or(false)
        {
            drop(guard);
            return Err("系统通知会话暂不支持删除".to_string());
        }
        let chat_index = state_read_chat_index_cached(state)?;
        let active_conversation_id = chat_index
            .conversations
            .iter()
            .filter_map(|item| self.get_conversation_meta(state, item.id.as_str()).ok())
            .find(|conversation_meta| {
                conversation_meta.id != normalized_conversation_id
                    && self.conversation_meta_is_unarchived_meta_view(conversation_meta)
                    && conversation_meta.visible_in_foreground_lists
                    && conversation_meta.status.trim() == "active"
            })
            .map(|conversation_meta| conversation_meta.id.to_string())
            .or_else(|| {
                chat_index
                    .conversations
                    .iter()
                    .filter_map(|item| self.get_conversation_meta(state, item.id.as_str()).ok())
                    .find(|conversation_meta| {
                        conversation_meta.id != normalized_conversation_id
                            && self.conversation_meta_is_unarchived_meta_view(conversation_meta)
                            && conversation_meta.visible_in_foreground_lists
                    })
                    .map(|conversation_meta| conversation_meta.id.to_string())
            })
            .unwrap_or_default();
        mark_tasks_as_session_lost(&state.data_path, normalized_conversation_id);
        let active_conversation_id = if active_conversation_id.trim().is_empty() {
            let system_notification_exists = self
                .get_conversation_meta(state, SYSTEM_NOTIFICATION_CONVERSATION_ID)
                .ok()
                .filter(|conversation_meta| {
                    self.conversation_meta_is_unarchived_meta_view(conversation_meta)
                        && conversation_meta.visible_in_foreground_lists
                        && self.conversation_meta_is_system_notification_meta_view(conversation_meta)
                })
                .is_some();
            if !system_notification_exists {
                let system_notification = build_system_notification_conversation_record();
                state_schedule_conversation_persist(state, &system_notification)?;
            }
            let mut next_runtime = runtime.clone();
            if next_runtime.main_conversation_id.as_deref().map(str::trim)
                != Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
            {
                next_runtime.main_conversation_id = Some(SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string());
                state_write_runtime_state_cached(state, &next_runtime)?;
            }
            SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string()
        } else {
            active_conversation_id
        };
        if let Ok(cleanup_conversation) =
            read_conversation_for_backup_cleanup(state, normalized_conversation_id)
        {
            match cleanup_backup_records_from_messages(&state.data_path, &cleanup_conversation.messages) {
                Ok(cleaned) if cleaned > 0 => {
                    eprintln!(
                        "[会话删除] apply_patch 备份清理完成: conversation={}, cleaned={}",
                        normalized_conversation_id, cleaned
                    );
                }
                Err(err) => {
                    eprintln!(
                        "[会话删除] apply_patch 备份清理失败: conversation={}, error={}",
                        normalized_conversation_id, err
                    );
                }
                _ => {}
            }
        }
        state_schedule_conversation_delete(state, normalized_conversation_id)?;
        clear_conversation_list_activity_mark(state, normalized_conversation_id);
        let unarchived_conversations =
            self.collect_unarchived_conversation_summaries_cached(state, &app_config)?;
        drop(guard);
        Ok(DeleteUnarchivedConversationMutationResult {
            deleted_conversation_id: normalized_conversation_id.to_string(),
            active_conversation_id,
            overview_payload: UnarchivedConversationOverviewUpdatedPayload {
                preferred_conversation_id: unarchived_conversations
                    .first()
                    .map(|item| item.conversation_id.clone()),
                unarchived_conversations,
            },
        })
    }

    fn rewind_conversation(
        &self,
        state: &AppState,
        input: &RewindConversationInput,
        message_id: &str,
        started_at: &std::time::Instant,
    ) -> Result<RewindConversationMutationResult, String> {
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| {
                format!(
                    "Failed to lock state mutex at {}:{} {}: {err}",
                    file!(),
                    line!(),
                    module_path!()
                )
            })?;

        let requested_conversation_id = trimmed_option(input.session.conversation_id.as_deref());
        let Some(requested_conversation_id) = requested_conversation_id.as_deref() else {
            drop(guard);
            return Err("conversationId is required.".to_string());
        };
        let conversation_meta = self
            .get_conversation_meta(state, requested_conversation_id)
            .map_err(|_| "Target user message not found in active conversation.".to_string())?;
        let conversation_id = if self.conversation_meta_is_unarchived_meta_view(&conversation_meta)
            && (conversation_meta.visible_in_foreground_lists
                || conversation_meta.is_remote_im_contact)
        {
            conversation_meta.id.to_string()
        } else {
            drop(guard);
            return Err("Target user message not found in active conversation.".to_string());
        };
        let runtime_state = get_conversation_runtime_state(state, &conversation_id)?;
        if runtime_state != MainSessionState::Idle {
            let runtime_state_text = match runtime_state {
                MainSessionState::Idle => "空闲",
                MainSessionState::AssistantStreaming => "助理流式输出",
                MainSessionState::OrganizingContext => "整理上下文",
            };
            drop(guard);
            runtime_log_info(format!(
                "[会话撤回] 失败，任务=rewind_conversation_from_message，conversation_id={}，原因=会话运行中，runtime_state={}",
                conversation_id, runtime_state_text
            ));
            return Err("当前会话正在运行或整理上下文，完成后再撤回。".to_string());
        }
        let store_paths = message_store::message_store_paths(&state.data_path, &conversation_id)?;
        ensure_ready_message_store_from_legacy_conversation(state, &conversation_id, &store_paths)?;
        let rewind_state =
            read_ready_store_rewind_state_meta_view(state, &store_paths, &conversation_meta, message_id)?;
        let git_snapshot = read_git_snapshot_record_from_provider_meta(
            rewind_state.recalled_user_message.provider_meta.as_ref(),
        );
        maybe_undo_rewind_apply_patch(
            state,
            input,
            &rewind_state.removed_messages,
            message_id,
            started_at,
        )?;
        let updated_at = now_iso();
        let mut updated_meta = message_store::read_ready_message_store_meta(&store_paths)?
            .ok_or_else(|| {
                format!(
                    "撤回会话消息失败：缺少 ready 消息元数据，conversation_id={conversation_id}"
                )
            })?;
        updated_meta.apply_metadata_fields_from_meta_view(&conversation_meta);
        updated_meta.apply_truncated_rewind_state(
            rewind_state.keep_count,
            rewind_state.remaining_todos.clone(),
            updated_at,
            rewind_state.remaining_last_user_at.clone(),
            rewind_state.remaining_last_assistant_at.clone(),
            rewind_state.remaining_last_message_at.clone(),
            rewind_state.remaining_body_message_count,
            rewind_state.remaining_body_text_length,
            rewind_state.remaining_last_assistant_at.is_some(),
            rewind_state.remaining_has_context_compaction_message,
            rewind_state.remaining_latest_summary_title.clone(),
            rewind_state.remaining_preview_messages.clone(),
        );
        let current_todo = conversation_current_todo_text_from_items(&rewind_state.remaining_todos);
        let _write_guard = state
            .app_data_persist_write_lock
            .lock()
            .map_err(|err| {
                named_lock_error(
                    "app_data_persist_write_lock",
                    file!(),
                    line!(),
                    module_path!(),
                    &err,
                )
            })?;
        message_store::write_jsonl_snapshot_truncated_messages_shard_from_meta(
            &store_paths,
            &updated_meta,
            rewind_state.keep_count,
        )?;
        state_mark_conversation_metadata_direct_persisted(state, &conversation_id)?;
        drop(guard);
        Ok(RewindConversationMutationResult {
            conversation_id,
            removed_count: rewind_state.removed_messages.len(),
            remaining_count: rewind_state.keep_count,
            current_todo,
            current_todos: rewind_state.remaining_todos,
            recalled_user_message: Some(rewind_state.recalled_user_message),
            git_snapshot,
        })
    }

    fn preview_rewind_conversation(
        &self,
        state: &AppState,
        input: &RewindConversationInput,
        message_id: &str,
    ) -> Result<RewindConversationPreviewResult, String> {
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| {
                format!(
                    "Failed to lock state mutex at {}:{} {}: {err}",
                    file!(),
                    line!(),
                    module_path!()
                )
            })?;

        let requested_conversation_id = trimmed_option(input.session.conversation_id.as_deref());
        let Some(requested_conversation_id) = requested_conversation_id.as_deref() else {
            drop(guard);
            return Err("conversationId is required.".to_string());
        };
        let conversation_meta = self
            .get_conversation_meta(state, requested_conversation_id)
            .map_err(|_| "Target user message not found in active conversation.".to_string())?;
        let conversation_id = if self.conversation_meta_is_unarchived_meta_view(&conversation_meta)
            && (conversation_meta.visible_in_foreground_lists
                || conversation_meta.is_remote_im_contact)
        {
            conversation_meta.id.to_string()
        } else {
            drop(guard);
            return Err("Target user message not found in active conversation.".to_string());
        };
        let runtime_state = get_conversation_runtime_state(state, &conversation_id)?;
        if runtime_state != MainSessionState::Idle {
            drop(guard);
            return Ok(RewindConversationPreviewResult {
                conversation_id,
                can_undo_patch: false,
                hint: "当前会话正在运行或整理上下文，完成后再撤回。".to_string(),
            });
        }
        let store_paths = message_store::message_store_paths(&state.data_path, &conversation_id)?;
        ensure_ready_message_store_from_legacy_conversation(state, &conversation_id, &store_paths)?;
        let rewind_state =
            read_ready_store_rewind_state_meta_view(state, &store_paths, &conversation_meta, message_id)?;
        let backup_record_ids = collect_backup_record_ids_from_messages(&rewind_state.removed_messages);
        let existing_backup_count = backup_record_ids
            .iter()
            .filter(|record_id| apply_patch_record_path(&state.data_path, record_id).exists())
            .count();
        drop(guard);

        if existing_backup_count > 0 {
            return Ok(RewindConversationPreviewResult {
                conversation_id,
                can_undo_patch: true,
                hint: String::new(),
            });
        }
        let hint = if backup_record_ids.is_empty() {
            "该范围内没有检测到可撤回的工具修改。"
        } else {
            "检测到工具修改记录，但对应备份已不存在，无法撤回文件修改。"
        };
        Ok(RewindConversationPreviewResult {
            conversation_id,
            can_undo_patch: false,
            hint: hint.to_string(),
        })
    }

    fn branch_conversation_from_selection(
        &self,
        state: &AppState,
        source_conversation_id: &str,
        selected_message_ids: &[String],
    ) -> Result<BranchUnarchivedConversationMutationResult, String> {
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let app_config = runtime_snapshot.config.clone();
        let runtime = state_read_runtime_state_cached(state)?;
        let agents = runtime_snapshot.agents.clone();
        let source_conversation_meta = self
            .get_conversation_meta(state, source_conversation_id)
            .ok()
            .filter(|conversation_meta| {
                self.conversation_meta_is_local_normal_chat_meta_view(conversation_meta)
            })
            .ok_or_else(|| "源会话不存在或已归档".to_string())?;
        let selection =
            read_branch_selection_or_pending_conversation(state, source_conversation_id, selected_message_ids)?;
        let selected_messages = selection.selected_messages;
        let first_selected_ordinal = selection.first_selected_ordinal;
        if selected_messages.is_empty() {
            drop(guard);
            return Err("未找到可创建会话分支的已选消息".to_string());
        }
        let department = runtime_department_by_id(
            &runtime_snapshot,
            source_conversation_meta.department_id.trim(),
        )
        .cloned()
        .ok_or_else(|| "源会话所属部门不存在".to_string())?;
        let branched_title = build_branch_conversation_title(
            &source_conversation_meta.title,
            first_selected_ordinal.max(1),
            runtime.main_conversation_id.as_deref().map(str::trim)
                == Some(source_conversation_meta.id.as_str()),
        );
        let latest_compaction_message = selection.latest_compaction_message;
        let conversation = build_branch_conversation_record_from_selection_runtime_meta_view(
            &state.data_path,
            &agents,
            &source_conversation_meta,
            &department,
            &branched_title,
            latest_compaction_message.as_ref(),
            &selected_messages,
        )?;
        let conversation_id = conversation.id.clone();
        state_schedule_conversation_persist(state, &conversation)?;
        let overview_payload = UnarchivedConversationOverviewUpdatedPayload {
            preferred_conversation_id: Some(conversation_id.clone()),
            unarchived_conversations: self.collect_unarchived_conversation_summaries_cached(
                state,
                &app_config,
            )?,
        };
        drop(guard);
        Ok(BranchUnarchivedConversationMutationResult {
            conversation_id,
            title: branched_title,
            selected_count: selected_messages.len(),
            has_compaction_seed: latest_compaction_message.is_some(),
            overview_payload,
        })
    }

    fn forward_conversation_selection(
        &self,
        state: &AppState,
        source_conversation_id: &str,
        target_conversation_id: &str,
        selected_message_ids: &[String],
    ) -> Result<ForwardUnarchivedConversationMutationResult, String> {
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let target_runtime_state = {
            let runtime_slots = lock_conversation_runtime_slots(state)?;
            runtime_slots
                .get(target_conversation_id)
                .map(|slot| slot.state.clone())
                .unwrap_or(MainSessionState::Idle)
        };
        if target_runtime_state == MainSessionState::AssistantStreaming {
            drop(guard);
            return Err("目标会话正在流式输出中，暂时无法转发到会话".to_string());
        }
        if target_runtime_state == MainSessionState::OrganizingContext {
            drop(guard);
            return Err("目标会话正在整理上下文，暂时无法转发到会话".to_string());
        }
        let app_config = state_read_config_cached(state)?;
        let _source_conversation = self
            .get_conversation_meta(state, source_conversation_id)
            .ok()
            .filter(|conversation_meta| {
                self.conversation_meta_is_local_normal_chat_meta_view(conversation_meta)
            })
            .ok_or_else(|| "源会话不存在或已归档".to_string())?;
        let selection =
            read_branch_selection_or_pending_conversation(state, source_conversation_id, selected_message_ids)?;
        let selected_messages = selection.selected_messages;
        if selected_messages.is_empty() {
            drop(guard);
            return Err("未找到可转发到会话的已选消息".to_string());
        }
        let _target_conversation = self
            .get_conversation_meta(state, target_conversation_id)
            .ok()
            .filter(|conversation_meta| {
                self.conversation_meta_is_local_normal_chat_meta_view(conversation_meta)
            })
            .ok_or_else(|| "目标会话不存在或已归档".to_string())?;
        drop(guard);
        let copied_messages = selected_messages
            .iter()
            .map(clone_chat_message_for_copied_conversation)
            .collect::<Vec<_>>();
        conversation_service_v2().append_messages(
            state,
            target_conversation_id,
            &copied_messages,
        )?;
        let overview_payload = UnarchivedConversationOverviewUpdatedPayload {
            preferred_conversation_id: Some(target_conversation_id.to_string()),
            unarchived_conversations: self.collect_unarchived_conversation_summaries_cached(
                state,
                &app_config,
            )?,
        };
        Ok(ForwardUnarchivedConversationMutationResult {
            target_conversation_id: target_conversation_id.to_string(),
            forwarded_count: selected_messages.len(),
            overview_payload,
        })
    }

    fn forward_selection_to_remote_im_contact(
        &self,
        state: &AppState,
        source_conversation_id: &str,
        target_conversation_id: &str,
        remote_contact_id: &str,
        selected_message_ids: &[String],
    ) -> Result<ForwardSelectionToRemoteImContactMutationResult, String> {
        let normalized_remote_contact_id = remote_contact_id.trim();
        if normalized_remote_contact_id.is_empty() {
            return Err("remoteContactId 不能为空".to_string());
        }
        let app_config = state_read_config_cached(state)?;
        let _source_conversation = self
            .get_conversation_meta(state, source_conversation_id)
            .ok()
            .filter(|conversation_meta| {
                self.conversation_meta_is_local_normal_chat_meta_view(conversation_meta)
            })
            .ok_or_else(|| "源会话不存在或已归档".to_string())?;
        let selection =
            read_branch_selection_or_pending_conversation(state, source_conversation_id, selected_message_ids)?;
        let selected_messages = selection.selected_messages;
        if selected_messages.is_empty() {
            return Err("未找到可推送到远程联系人的已选消息".to_string());
        }

        let _target_conversation = self
            .get_conversation_meta(state, target_conversation_id)
            .ok()
            .filter(|conversation_meta| conversation_meta.is_remote_im_contact)
            .ok_or_else(|| "目标远程联系人会话不存在".to_string())?;
        let runtime = state_read_runtime_state_cached(state)?;
        let contact = runtime
            .remote_im_contacts
            .iter()
            .find(|item| item.id.trim() == normalized_remote_contact_id)
            .cloned()
            .ok_or_else(|| "目标远程联系人不存在".to_string())?;
        if contact.bound_conversation_id.as_deref().map(str::trim) != Some(target_conversation_id) {
            return Err("远程联系人与目标会话不匹配".to_string());
        }
        if !contact.allow_send {
            return Err("当前联系人不允许发送消息".to_string());
        }
        let channel = remote_im_channel_by_id(&app_config, &contact.channel_id)
            .cloned()
            .ok_or_else(|| format!("远程 IM 渠道不存在: {}", contact.channel_id))?;
        if !channel.enabled {
            return Err(format!("远程 IM 渠道未启用: {}", contact.channel_id));
        }

        let notification_message = self.build_forward_selection_notification_message(
            state,
            source_conversation_id,
            &selected_messages,
        )?;
        let notification_body = notification_message
            .parts
            .iter()
            .filter_map(|part| match part {
                MessagePart::Text { text, .. } => Some(text.trim().to_string()),
                _ => None,
            })
            .filter(|text: &String| !text.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        enqueue_session_notification_dispatch(
            state,
            target_conversation_id,
            &notification_body,
            &notification_message,
            "forward_selection_to_remote_im_contact",
        )?;
        let overview_payload = UnarchivedConversationOverviewUpdatedPayload {
            preferred_conversation_id: Some(target_conversation_id.to_string()),
            unarchived_conversations: self.collect_unarchived_conversation_summaries_cached(
                state,
                &app_config,
            )?,
        };
        Ok(ForwardSelectionToRemoteImContactMutationResult {
            target_conversation_id: target_conversation_id.to_string(),
            remote_contact_id: normalized_remote_contact_id.to_string(),
            forwarded_count: selected_messages.len(),
            overview_payload,
        })
    }

    fn rename_conversation(
        &self,
        state: &AppState,
        conversation_id: &str,
        next_title: &str,
    ) -> Result<String, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let normalized_title = next_title.trim();
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;

        ensure_unarchived_conversation_not_organizing(state, normalized_conversation_id)?;

        let conversation_meta =
            self.get_conversation_meta(state, normalized_conversation_id)?;
        if !self.conversation_meta_is_unarchived_meta_view(&conversation_meta) {
            drop(guard);
            return Err("未找到可改名的会话".to_string());
        }
        if self.conversation_meta_is_system_notification_meta_view(&conversation_meta) {
            drop(guard);
            return Err("系统通知会话不支持改名".to_string());
        }
        if conversation_meta.title.trim() == normalized_title {
            drop(guard);
            return Ok(normalized_title.to_string());
        }

        drop(guard);
        self.set_title(state, normalized_conversation_id, normalized_title)?;
        Ok(normalized_title.to_string())
    }

    fn update_latest_summary_title(
        &self,
        state: &AppState,
        conversation_id: &str,
        next_title: &str,
    ) -> Result<bool, String> {
        self.update_unarchived_conversation_by_id(state, conversation_id, |conversation| {
            Ok(conversation_update_latest_summary_title(
                conversation,
                Some(next_title),
            ))
        })
    }

    fn toggle_conversation_pin(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<ToggleUnarchivedConversationPinMutationResult, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId 不能为空".to_string());
        }
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;

        let mut runtime = state_read_runtime_state_cached(state)?;
        let main_conversation_id = runtime
            .main_conversation_id
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_string();
        if normalized_conversation_id == main_conversation_id {
            drop(guard);
            return Err("系统通知会话始终置顶".to_string());
        }
        let conversation = match self.get_conversation_meta(state, normalized_conversation_id) {
            Ok(conversation_meta) => conversation_meta,
            Err(_) => {
                drop(guard);
                return Err("未找到可置顶的会话".to_string());
            }
        };
        if self.conversation_meta_is_system_notification_meta_view(&conversation) {
            drop(guard);
            return Err("系统通知会话始终置顶".to_string());
        }
        if !self.conversation_meta_is_local_normal_chat_meta_view(&conversation) {
            drop(guard);
            return Err("未找到可置顶的会话".to_string());
        }

        let visible_ids = state_read_chat_index_cached(state)?
            .conversations
            .iter()
            .filter_map(|item| self.get_conversation_meta(state, item.id.as_str()).ok())
            .filter(|conversation_meta| {
                self.conversation_meta_is_local_normal_chat_meta_view(conversation_meta)
            })
            .map(|conversation_meta| conversation_meta.id.trim().to_string())
            .filter(|conversation_id| !conversation_id.is_empty())
            .collect::<std::collections::HashSet<_>>();
        let mut seen = std::collections::HashSet::<String>::new();
        let previous_pinned = runtime
            .pinned_conversation_ids
            .iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .filter(|item| *item != main_conversation_id)
            .filter(|item| visible_ids.contains(item))
            .filter(|item| seen.insert(item.clone()))
            .collect::<Vec<_>>();
        let mut next_pinned = previous_pinned.clone();
        if let Some(index) = next_pinned
            .iter()
            .position(|item| item.trim() == normalized_conversation_id)
        {
            next_pinned.remove(index);
        } else {
            next_pinned.insert(0, normalized_conversation_id.to_string());
        }
        runtime.pinned_conversation_ids = next_pinned.clone();
        state_write_runtime_state_cached(state, &runtime)?;
        drop(guard);

        let is_pinned = next_pinned
            .iter()
            .any(|item| item.trim() == normalized_conversation_id);
        let pin_index = next_pinned
            .iter()
            .position(|item| item.trim() == normalized_conversation_id);
        Ok(ToggleUnarchivedConversationPinMutationResult {
            conversation_id: normalized_conversation_id.to_string(),
            is_pinned,
            pin_index,
        })
    }

    fn persist_stop_chat_partial_message(
        &self,
        state: &AppState,
        requested_conversation_id: Option<&str>,
        requested_department_id: Option<&str>,
        agent_id: &str,
        partial_assistant_text: &str,
        partial_activity_reasoning_text: &str,
        partial_inline_activity_text: &str,
        completed_tool_history: &[Value],
    ) -> Result<StopChatPersistResult, String> {
        let should_persist = !partial_assistant_text.trim().is_empty()
            || !partial_activity_reasoning_text.trim().is_empty()
            || !completed_tool_history.is_empty();
        if !should_persist {
            return Ok(StopChatPersistResult {
                persisted: false,
                conversation_id: None,
                assistant_message: None,
            });
        }

        let _guard = lock_conversation_with_metrics(state, "stop_chat_generation_persist_partial")?;
        let app_config = load_runtime_organization_snapshot(state)?.config;
        let api_config_id =
            resolve_stop_chat_api_config_id(&app_config, requested_department_id, agent_id)?;
        if !app_config.api_configs.iter().any(|api| api.id == api_config_id) {
            return Err(format!("Selected API config '{api_config_id}' not found."));
        }
        let Some(target) = resolve_stop_chat_target(state, requested_conversation_id, agent_id)? else {
            return Ok(StopChatPersistResult {
                persisted: false,
                conversation_id: None,
                assistant_message: None,
            });
        };
        if let Some(result) = build_stop_chat_skip_result(&target) {
            return Ok(result);
        }

        let mut assistant_message = build_stop_chat_partial_assistant_message(
            agent_id,
            partial_assistant_text,
            partial_activity_reasoning_text,
            partial_inline_activity_text,
            completed_tool_history,
        );
        let assistant_message_seed = assistant_message.id.clone();
        populate_assistant_meme_annotations(
            state,
            &assistant_message_seed,
            &mut assistant_message,
        )?;
        let conversation_id = match target {
            StopChatConversationTarget::Runtime(mut conversation) => {
                let target_id = apply_stop_chat_partial_message(&mut conversation, &assistant_message);
                delegate_runtime_thread_conversation_update(state, &target_id, conversation)
                    .map(|_| target_id.to_string())?
            }
            StopChatConversationTarget::PersistedRef { conversation_id, .. } => {
                let target_id = conversation_id.to_string();
                self.append_message(state, &target_id, &assistant_message)?;
                target_id
            }
        };

        Ok(StopChatPersistResult {
            persisted: true,
            conversation_id: Some(conversation_id),
            assistant_message: Some(assistant_message),
        })
    }

    fn archive_conversation(
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
        let source_conversation_meta = self
            .get_conversation_meta(state, &source.id)
            .map_err(|err| format!("当前没有可归档的活动对话：{}", err))?;
        let source_conversation =
            self.build_conversation_record_from_meta_view(&source_conversation_meta);
        let already_archived = source_conversation_meta.status.trim() == "archived";
        if !already_archived
            && !self.conversation_meta_is_local_normal_chat_meta_view(&source_conversation_meta)
        {
            drop(guard);
            return Err("当前没有可归档的活动对话。".to_string());
        }

        let runtime = state_read_runtime_state_cached(state)?;
        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let agents = runtime_snapshot.agents;
        let chat_index = state_read_chat_index_cached(state)?;
        let active_conversation_id = if let Some(conversation_id) = chat_index
            .conversations
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                let conversation_meta = self.get_conversation_meta(state, item.id.as_str()).ok()?;
                Some((idx, conversation_meta))
            })
            .filter(|(_, conversation_meta)| {
                conversation_meta.id != source.id
                    && self.conversation_meta_is_local_normal_chat_meta_view(conversation_meta)
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
            .map(|(_, conversation_meta)| conversation_meta.id.to_string())
        {
            conversation_id
        } else {
            let conversation = build_archive_replacement_conversation(
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
        let app_config = runtime_snapshot.config;
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

    fn set_preferred_api_config_id(
        &self,
        state: &AppState,
        conversation_id: &str,
        preferred_api_config_id: Option<String>,
    ) -> Result<Conversation, String> {
        self.apply_external_metadata_patch(
            state,
            conversation_id,
            "conversation_v2_set_preferred_api_config_id",
            ConversationExternalMetadataPatch {
                preferred_api_config_id: Some(preferred_api_config_id),
                ..Default::default()
            },
        )
    }

    fn set_auto_push_remote_contact_id(
        &self,
        state: &AppState,
        conversation_id: &str,
        auto_push_remote_contact_id: Option<String>,
    ) -> Result<Conversation, String> {
        self.apply_external_metadata_patch(
            state,
            conversation_id,
            "conversation_v2_set_auto_push_remote_contact_id",
            ConversationExternalMetadataPatch {
                auto_push_remote_contact_id: Some(auto_push_remote_contact_id),
                ..Default::default()
            },
        )
    }

    fn set_title(
        &self,
        state: &AppState,
        conversation_id: &str,
        next_title: &str,
    ) -> Result<Conversation, String> {
        self.apply_external_metadata_patch(
            state,
            conversation_id,
            "conversation_v2_set_title",
            ConversationExternalMetadataPatch {
                title: Some(next_title.trim().to_string()),
                ..Default::default()
            },
        )
    }

    fn set_plan_mode_enabled(
        &self,
        state: &AppState,
        conversation_id: &str,
        enabled: bool,
    ) -> Result<Conversation, String> {
        self.apply_external_metadata_patch(
            state,
            conversation_id,
            "conversation_v2_set_plan_mode_enabled",
            ConversationExternalMetadataPatch {
                plan_mode_enabled: Some(enabled),
                ..Default::default()
            },
        )
    }

    fn refresh_unarchived_conversation_overview(
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

    fn set_active_conversation(
        &self,
        state: &AppState,
        input: &SetActiveUnarchivedConversationInput,
    ) -> Result<String, String> {
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let mut app_config = state_read_config_cached(state)?;
        let agents = state_read_agents_cached(state)?;
        let mut runtime = state_read_runtime_state_cached(state)?;
        let _effective_agent_id = self.resolve_effective_agent_id_for_read(
            state,
            &mut app_config,
            &agents,
            &runtime.assistant_department_agent_id,
            input.agent_id.as_deref().unwrap_or_default(),
        )?;
        let requested_conversation_id = input
            .conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let (target_conversation_meta, target_conversation, created_new_conversation) =
            if let Some(conversation_id) = requested_conversation_id {
                if let Some(conversation_meta) = self.get_conversation_meta(state, conversation_id)
                    .ok()
                    .filter(|conversation_meta| {
                        self.conversation_meta_is_unarchived_meta_view(conversation_meta)
                            && conversation_meta.visible_in_foreground_lists
                    })
                {
                    (Some(conversation_meta), None, false)
                } else if let Some(conversation_meta) = runtime
                    .main_conversation_id
                    .as_deref()
                    .and_then(|current_main| {
                        self.get_conversation_meta(state, current_main.trim()).ok()
                    })
                    .filter(|conversation_meta| {
                        self.conversation_meta_is_unarchived_meta_view(conversation_meta)
                            && conversation_meta.visible_in_foreground_lists
                    })
                {
                    (Some(conversation_meta), None, false)
                } else if runtime.main_conversation_id.as_deref().map(str::trim)
                    == Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
                {
                    let conversation = build_system_notification_conversation_record();
                    (None, Some(conversation), true)
                } else if let Some(conversation_meta) =
                    read_latest_visible_foreground_conversation_metadata(state)?
                {
                    (Some(conversation_meta), None, false)
                } else {
                    let conversation = build_system_notification_conversation_record();
                    (None, Some(conversation), true)
                }
            } else if let Some(conversation_meta) = runtime
                .main_conversation_id
                .as_deref()
                .and_then(|conversation_id| {
                    self.get_conversation_meta(state, conversation_id.trim()).ok()
                })
                .filter(|conversation_meta| {
                    self.conversation_meta_is_unarchived_meta_view(conversation_meta)
                        && conversation_meta.visible_in_foreground_lists
                })
            {
                (Some(conversation_meta), None, false)
            } else if runtime.main_conversation_id.as_deref().map(str::trim)
                == Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
            {
                let conversation = build_system_notification_conversation_record();
                (None, Some(conversation), true)
            } else if let Some(conversation_meta) =
                read_latest_visible_foreground_conversation_metadata(state)?
            {
                (Some(conversation_meta), None, false)
            } else {
                let conversation = build_system_notification_conversation_record();
                (None, Some(conversation), true)
            };
        let conversation_id = target_conversation_meta
            .as_ref()
            .map(|conversation_meta| conversation_meta.id.to_string())
            .or_else(|| target_conversation.as_ref().map(|conversation| conversation.id.clone()))
            .ok_or_else(|| "Requested conversation not found.".to_string())?;
        ensure_unarchived_conversation_not_organizing(state, &conversation_id)?;
        clear_conversation_list_activity_mark(state, &conversation_id);
        if created_new_conversation {
            let conversation = target_conversation
                .as_ref()
                .ok_or_else(|| "Requested conversation not found.".to_string())?;
            state_schedule_conversation_persist(state, conversation)?;
        }
        if target_conversation_meta
            .as_ref()
            .map(|conversation_meta| self.conversation_meta_is_system_notification_meta_view(conversation_meta))
            .or_else(|| {
                target_conversation
                    .as_ref()
                    .map(conversation_is_system_notification)
            })
            .unwrap_or(false)
            && runtime.main_conversation_id.as_deref().map(str::trim)
                != Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
        {
            runtime.main_conversation_id = Some(SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string());
            state_write_runtime_state_cached(state, &runtime)?;
        }
        drop(guard);
        Ok(conversation_id)
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
        let conversation_meta = match self.get_conversation_meta(state, normalized_conversation_id) {
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
        if !conversation_meta.summary.trim().is_empty() {
            drop(guard);
            return Ok(None);
        }
        if conversation_meta.current_todos == stored_todos {
            drop(guard);
            return Ok(None);
        }
        drop(guard);
        let updated = self.set_current_todos(
            state,
            normalized_conversation_id,
            stored_todos.to_vec(),
        )?;
        let current_todo = conversation_current_todo_text_from_items(&updated.current_todos);
        Ok(Some(ConversationTodosUpdateResult { current_todo }))
    }

    fn read_unarchived_conversation_summary(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Option<UnarchivedConversationSummary>, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Ok(None);
        }
        let guard = state.conversation_lock.lock().map_err(|err| {
            format!(
                "Failed to lock state mutex at {}:{} {}: {err}",
                file!(),
                line!(),
                module_path!()
            )
        })?;
        let app_config = state_read_config_cached(state)?;
        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let runtime_app_config = if runtime_snapshot.config.departments.is_empty() {
            app_config
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
        let visible_ids = chat_index
            .conversations
            .iter()
            .filter(|item| !chat_index_item_is_archived(item))
            .map(|item| item.id.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect::<std::collections::HashSet<_>>();
        let conversation_meta = match self.get_conversation_meta(state, normalized_conversation_id) {
                Ok(conversation_meta) => conversation_meta,
                Err(err) => {
                    drop(guard);
                    eprintln!(
                        "[会话索引读取] 状态=失败，任务=read_unarchived_conversation_summary，conversation_id={}，error={}",
                        normalized_conversation_id, err
                    );
                    return Ok(None);
                }
            };
        if !visible_ids.contains(normalized_conversation_id)
            || !self.conversation_meta_is_unarchived_meta_view(&conversation_meta)
            || !conversation_meta.visible_in_foreground_lists
        {
            drop(guard);
            return Ok(None);
        }
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
        let summary = build_unarchived_conversation_summary_from_meta_view(
            state,
            &runtime_app_config,
            &main_conversation_id,
            &pinned_conversation_ids,
            &conversation_meta,
            Some(DESKTOP_CHAT_VIEWER_ID),
        );
        drop(guard);
        Ok(Some(summary))
    }

    fn set_current_todos(
        &self,
        state: &AppState,
        conversation_id: &str,
        current_todos: Vec<ConversationTodoItem>,
    ) -> Result<Conversation, String> {
        self.apply_external_metadata_patch(
            state,
            conversation_id,
            "conversation_v2_set_current_todos",
            ConversationExternalMetadataPatch {
                current_todos: Some(current_todos),
                ..Default::default()
            },
        )
    }

    fn set_shell_workspace(
        &self,
        state: &AppState,
        conversation_id: &str,
        shell_workspace_path: Option<Option<String>>,
        shell_workspaces: Option<Vec<ShellWorkspaceConfig>>,
        shell_autonomous_mode: Option<bool>,
    ) -> Result<Conversation, String> {
        self.apply_external_metadata_patch(
            state,
            conversation_id,
            "conversation_v2_set_shell_workspace",
            ConversationExternalMetadataPatch {
                shell_workspace_path,
                shell_workspaces,
                shell_autonomous_mode,
                ..Default::default()
            },
        )
    }

    fn update_shell_workspace(
        &self,
        state: &AppState,
        conversation_id: &str,
        shell_workspace_path: Option<Option<String>>,
        shell_workspaces: Option<Vec<ShellWorkspaceConfig>>,
        shell_autonomous_mode: Option<bool>,
    ) -> Result<Conversation, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("指定会话不存在：".to_string());
        }
        let conversation = self
            .read_persisted_conversation(state, normalized_conversation_id)
            .map_err(|_| format!("指定会话不存在：{normalized_conversation_id}"))?;
        let original_path = conversation.shell_workspace_path.clone();
        let original_workspaces = conversation.shell_workspaces.clone();
        let original_autonomous_mode = conversation.shell_autonomous_mode;
        let updated = self.set_shell_workspace(
            state,
            normalized_conversation_id,
            shell_workspace_path,
            shell_workspaces,
            shell_autonomous_mode,
        )?;
        if updated.shell_workspace_path == original_path
            && updated.shell_workspaces == original_workspaces
            && updated.shell_autonomous_mode == original_autonomous_mode
        {
            return Ok(updated);
        }
        Ok(updated)
    }

    fn add_conversation_cumulative_usage_delta(
        &self,
        state: &AppState,
        conversation_id: &str,
        usage: &Value,
    ) -> Result<bool, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Ok(false);
        }
        let mut probe = ConversationCumulativeUsage::default();
        if !conversation_cumulative_usage_add_provider_usage(&mut probe, usage) {
            return Ok(false);
        }
        let _guard = lock_conversation_with_metrics(state, "add_conversation_cumulative_usage")?;
        let (conversation, changed, _) = state_update_conversation_metadata_cached(
            state,
            normalized_conversation_id,
            |conversation| {
                Ok(conversation_cumulative_usage_add_provider_usage(
                    &mut conversation.cumulative_usage,
                    usage,
                ))
            },
        )?;
        if changed {
            emit_provider_context_usage_update_from_conversation(state, &conversation, usage);
        }
        Ok(changed)
    }

    fn enqueue_delegate_completion_notification(
        &self,
        state: &AppState,
        root_conversation_id: &str,
        target_department_id: &str,
        target_agent_id: &str,
        delegate_title: &str,
        content: &str,
        action: &str,
    ) -> Result<(), String> {
        let resolved_target =
            self.resolve_delegate_result_target_conversation(state, root_conversation_id)?;
        let body = build_delegate_completion_notification_body(
            state,
            target_department_id,
            target_agent_id,
            delegate_title,
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
        let source_conversation_meta = self
            .get_conversation_meta(state, normalized_source_conversation_id)?;
        if !self.conversation_meta_is_local_normal_chat_meta_view(&source_conversation_meta)
            || !source_conversation_meta.visible_in_foreground_lists
            || self.conversation_meta_is_system_notification_meta_view(&source_conversation_meta)
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
        let target_conversation_id =
            self.resolve_remote_im_contact_conversation_id_for_notification(
                state,
                normalized_remote_contact_id,
            )?;
        let body =
            build_session_notification_body(state, normalized_source_conversation_id, content)?;
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

    fn resolve_prompt_prepare_conversation_read_only(
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
        self.resolve_prompt_prepare_conversation_core_v2(
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

    fn resolve_prompt_prepare_conversation_core_v2(
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
        Ok(Some(self.build_prompt_prepare_resolution_v2(
            data,
            &conversation_before,
            selected_api,
            is_runtime_conversation,
        )))
    }

    fn build_prompt_prepare_resolution_v2(
        &self,
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
            conversation_before: self.build_prompt_prepare_conversation_before_v2(
                conversation_before,
                is_remote_im_contact_conversation,
                &remote_im_contact_processing_mode,
            ),
            last_archive_summary: None,
            is_remote_im_contact_conversation,
            remote_im_contact_processing_mode,
            response_style_id: data.response_style_id.clone(),
            user_name: user_persona_name(data),
            user_intro: user_persona_intro(data),
            enable_pdf_images: data.pdf_read_mode == "image" && selected_api.enable_image,
            is_runtime_conversation,
        }
    }

    fn build_prompt_prepare_conversation_before_v2(
        &self,
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

    fn find_remote_im_contact_by_conversation_in_data<'a>(
        &self,
        data: &'a AppData,
        conversation_id: &str,
    ) -> Option<&'a RemoteImContact> {
        let conversation = data
            .conversations
            .iter()
            .find(|item| item.id == conversation_id)?;
        let contact_conversation_key = conversation
            .root_conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(key) = contact_conversation_key {
            return data
                .remote_im_contacts
                .iter()
                .find(|contact| remote_im_contact_conversation_key(contact) == key);
        }
        data.remote_im_contacts.iter().find(|contact| {
            contact
                .bound_conversation_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                == Some(conversation_id)
        })
    }

    fn find_remote_im_contact_by_conversation_in_runtime<'a>(
        &self,
        runtime: &'a RuntimeStateFile,
        conversation_id: &str,
    ) -> Option<&'a RemoteImContact> {
        runtime.remote_im_contacts.iter().find(|contact| {
            contact
                .bound_conversation_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                == Some(conversation_id)
        })
    }

    fn try_get_conversation_snapshot_fast(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Option<Conversation>, String> {
        self.try_read_persisted_conversation(state, conversation_id)
    }

    fn try_read_unarchived_conversation(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Option<Conversation>, String> {
        Ok(self
            .try_get_conversation_snapshot_fast(state, conversation_id)?
            .filter(conversation_is_unarchived))
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
        let meta = match self.get_conversation_meta(state, main_conversation_id) {
            Ok(meta) => meta,
            Err(_) => return Ok(None),
        };
        Ok(meta
            .visible_in_foreground_lists
            .then(|| meta.id.to_string()))
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
        if runtime_agents.iter().any(|agent| {
            agent.id == assistant_department_agent_id && !agent.is_built_in_user
        }) {
            return Ok(assistant_department_agent_id.to_string());
        }
        runtime_agents
            .iter()
            .find(|agent| !agent.is_built_in_user)
            .map(|agent| agent.id.clone())
            .ok_or_else(|| "Selected agent not found.".to_string())
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
                self.get_conversation_meta(app_state, conversation_id)
                    .ok()
                    .filter(|conversation_meta| {
                        conversation_meta.summary.trim().is_empty()
                            && conversation_meta.conversation_kind.trim()
                                != CONVERSATION_KIND_DELEGATE
                    })
                    .map(|conversation_meta| {
                        self.build_conversation_record_from_meta_view(&conversation_meta)
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
                if let Some(conversation_meta) = self
                    .get_conversation_meta(state, SYSTEM_NOTIFICATION_CONVERSATION_ID)
                    .ok()
                    .filter(|conversation_meta| {
                        self.conversation_meta_is_unarchived_meta_view(conversation_meta)
                            && conversation_meta.visible_in_foreground_lists
                            && self.conversation_meta_is_system_notification_meta_view(
                                conversation_meta,
                            )
                    })
                {
                    conversation_meta.id
                } else {
                    let conversation = build_system_notification_conversation_record();
                    let conversation_id = conversation.id.clone();
                    state_schedule_conversation_persist(state, &conversation)?;
                    conversation_id
                }
            } else if self
                .get_conversation_meta(state, normalized_root_conversation_id)
                .ok()
                .filter(|conversation_meta| {
                    conversation_meta.summary.trim().is_empty()
                        && conversation_meta.conversation_kind.trim()
                            != CONVERSATION_KIND_DELEGATE
                        && conversation_meta.conversation_kind.trim()
                            != CONVERSATION_KIND_SYSTEM_NOTIFICATION
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
                let conversation_meta =
                    self.get_conversation_meta(state, &item.conversation_id).ok()?;
                if !self.conversation_meta_is_local_normal_chat_meta_view(&conversation_meta) {
                    return None;
                }
                let persona_name = agents
                    .iter()
                    .find(|agent| agent.id == conversation_meta.agent_id)
                    .map(|agent| agent.name.trim().to_string())
                    .filter(|name| !name.is_empty());
                let department_name = department_by_id(&config, &conversation_meta.department_id)
                    .map(|department| department.name.trim().to_string())
                    .filter(|name| !name.is_empty())
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

    fn list_remote_im_contact_conversations(
        &self,
        state: &AppState,
    ) -> Result<Vec<RemoteImContactConversationSummary>, String> {
        let mut runtime = state_read_runtime_state_cached(state)?;
        let config = load_runtime_organization_snapshot(state)?.config;
        let mut resolved_pairs = Vec::<(RemoteImContact, String)>::new();
        let mut runtime_changed = false;
        for contact in runtime.remote_im_contacts.iter_mut() {
            if remote_im_channel_by_id(&config, &contact.channel_id).is_none() {
                if contact
                    .bound_conversation_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .is_some()
                {
                    contact.bound_conversation_id = None;
                    runtime_changed = true;
                }
                continue;
            }
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
            let binding_pair = match resolve_department_agent_pair(
                contact.bound_department_id.as_deref(),
                contact.bound_agent_id.as_deref(),
                &config,
            ) {
                Ok(pair) => Some(pair),
                Err(err) => {
                    runtime_log_warn(format!(
                        "[远程IM] 跳过，任务=同步联系人会话绑定，contact_id={}，原因={}",
                        contact.id, err
                    ));
                    None
                }
            };
            if let Some((department_id, agent_id)) = binding_pair.as_ref() {
                contact.bound_department_id = Some(department_id.clone());
                contact.bound_agent_id = Some(agent_id.clone());
            }
            let target_key = remote_im_contact_conversation_key(contact);
            let conversation_id = previous_bound_conversation_id
                .as_deref()
                .and_then(|conversation_id| {
                    self.get_conversation_meta(state, conversation_id)
                        .ok()
                        .filter(|conversation_meta| {
                            conversation_meta.summary.trim().is_empty()
                                && conversation_meta.is_remote_im_contact
                        })
                        .map(|conversation_meta| conversation_meta.id.to_string())
                })
                .or_else(|| {
                    state_read_chat_index_cached(state)
                        .ok()?
                        .conversations
                        .iter()
                        .filter_map(|item| self.get_conversation_meta(state, item.id.as_str()).ok())
                        .find(|conversation_meta| {
                            conversation_meta.summary.trim().is_empty()
                                && conversation_meta.is_remote_im_contact
                                && conversation_meta.root_conversation_id.as_deref()
                                    == Some(target_key.as_str())
                        })
                        .map(|conversation_meta| conversation_meta.id.to_string())
                });
            let Some(conversation_id) = conversation_id else {
                if previous_bound_conversation_id.is_some() {
                    contact.bound_conversation_id = None;
                    runtime_changed = true;
                }
                continue;
            };
            contact.bound_conversation_id = Some(conversation_id.clone());
            if let Some((department_id, agent_id)) = binding_pair.as_ref() {
                sync_remote_im_contact_conversation_binding(
                    state,
                    contact,
                    &conversation_id,
                    department_id,
                    agent_id,
                )?;
            }
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
            let channel = remote_im_channel_by_id(&config, &contact.channel_id);
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
                    channel_name: channel
                        .as_ref()
                        .map(|item| item.name.trim().to_string())
                        .filter(|value| !value.is_empty()),
                    channel_enabled: channel.as_ref().map(|item| item.enabled).unwrap_or(false),
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
                    last_message_at: conversation.messages.last().map(|message| message.created_at.clone()),
                    message_count: conversation.messages.len(),
                    channel_id: contact.channel_id.clone(),
                    channel_name: channel
                        .as_ref()
                        .map(|item| item.name.trim().to_string())
                        .filter(|value| !value.is_empty()),
                    channel_enabled: channel.as_ref().map(|item| item.enabled).unwrap_or(false),
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
            let bk = b.last_message_at.as_deref().unwrap_or(b.updated_at.as_str());
            let ak = a.last_message_at.as_deref().unwrap_or(a.updated_at.as_str());
            bk.cmp(ak).then_with(|| b.updated_at.cmp(&a.updated_at))
        });
        Ok(items)
    }

    fn get_remote_im_contact_conversation_messages(
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
                .filter_map(|item| self.get_conversation_meta(state, item.id.as_str()).ok())
                .find(|conversation_meta| {
                    conversation_meta.summary.trim().is_empty()
                        && conversation_meta.is_remote_im_contact
                        && conversation_meta.root_conversation_id.as_deref()
                            == Some(target_key.as_str())
                })
                .map(|conversation_meta| conversation_meta.id.to_string())
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
            self.with_unarchived_conversation_by_id_fast(state, &conversation_id, |conversation| {
                let total = conversation.messages.len();
                let start = total.saturating_sub(DEFAULT_FOREGROUND_SNAPSHOT_RECENT_LIMIT);
                Ok(conversation.messages[start..].to_vec())
            })?
        };
        materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
        Ok(frontend_project_messages(messages))
    }

    fn get_remote_im_contact_conversation_block_page(
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
                .filter_map(|item| self.get_conversation_meta(state, item.id.as_str()).ok())
                .find(|conversation_meta| {
                    conversation_meta.summary.trim().is_empty()
                        && conversation_meta.is_remote_im_contact
                        && conversation_meta.root_conversation_id.as_deref()
                            == Some(target_key.as_str())
                })
                .map(|conversation_meta| conversation_meta.id.to_string())
                .ok_or_else(|| format!("联系人未绑定联系人会话：{normalized_contact_id}"))?
        };
        let conversation_meta = self.get_conversation_meta(state, &conversation_id)?;
        if !conversation_meta.summary.trim().is_empty()
            || !conversation_meta.is_remote_im_contact
        {
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

        let conversation = self.read_persisted_conversation(state, &conversation_id)?;
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
                        .filter_map(|item| match self.get_conversation_meta(state, item.id.as_str()) {
                            Ok(conversation_meta) => Some(conversation_meta),
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
                        .find(|conversation_meta| {
                            conversation_meta.summary.trim().is_empty()
                                && conversation_meta.is_remote_im_contact
                                && conversation_meta.root_conversation_id.as_deref()
                                    == Some(target_key.as_str())
                        })
                        .map(|conversation_meta| conversation_meta.id.to_string()),
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
        let conversation_meta = match self.get_conversation_meta(state, &conversation_id) {
            Ok(conversation_meta)
                if conversation_meta.summary.trim().is_empty()
                    && conversation_meta.is_remote_im_contact =>
            {
                conversation_meta
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
        state_schedule_conversation_delete(state, &conversation_meta.id)?;
        drop(guard);
        Ok(true)
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

    fn update_unarchived_conversation_by_id<T>(
        &self,
        state: &AppState,
        conversation_id: &str,
        updater: impl FnOnce(&mut Conversation) -> Result<T, String>,
    ) -> Result<T, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let mut conversation = self.read_persisted_conversation(state, normalized_conversation_id)?;
        self.ensure_unarchived_conversation(&conversation, normalized_conversation_id)?;
        let result = updater(&mut conversation)?;
        state_update_conversation_metadata_cached(state, normalized_conversation_id, |cached| {
            preserve_field_level_conversation_metadata(cached, &conversation);
            Ok(())
        })?;
        state_schedule_conversation_persist(state, &conversation)?;
        drop(guard);
        Ok(result)
    }

    fn get_active_goal(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Option<ConversationGoalState>, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required".to_string());
        }
        if let Ok(conversation_meta) = self.get_conversation_meta(state, normalized_conversation_id)
        {
            return Ok(conversation_meta
                .active_goal
                .as_ref()
                .filter(|goal| conversation_goal_is_active(goal))
                .cloned());
        }
        let conversation = delegate_runtime_thread_conversation_get(state, normalized_conversation_id)?
            .ok_or_else(|| format!("Conversation not found: {normalized_conversation_id}"))?;
        Ok(conversation
            .active_goal
            .as_ref()
            .filter(|goal| conversation_goal_is_active(goal))
            .cloned())
    }

    fn update_goal_conversation<T>(
        &self,
        state: &AppState,
        conversation_id: &str,
        task_name: &str,
        updater: impl FnOnce(&mut Conversation) -> Result<T, String>,
    ) -> Result<(Conversation, T), String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required".to_string());
        }
        let _guard = lock_conversation_with_metrics(state, task_name)?;
        if self
            .get_conversation_meta(state, normalized_conversation_id)
            .is_ok()
        {
            let (conversation, result, _) = state_update_conversation_metadata_cached(
                state,
                normalized_conversation_id,
                updater,
            )?;
            return Ok((conversation, result));
        }
        let mut conversation = delegate_runtime_thread_conversation_get(state, normalized_conversation_id)?
            .ok_or_else(|| format!("Conversation not found: {normalized_conversation_id}"))?;
        let result = updater(&mut conversation)?;
        delegate_runtime_thread_conversation_update(
            state,
            normalized_conversation_id,
            conversation.clone(),
        )?;
        Ok((conversation, result))
    }

    fn remote_im_runtime_state_should_cache_blocks(
        &self,
        runtime_state: &RemoteImContactRuntimeState,
    ) -> bool {
        runtime_state.presence_state == RemoteImPresenceState::Present
            || runtime_state.work_state == RemoteImWorkState::Busy
            || runtime_state.has_pending
    }

    fn collect_block_cache_whitelist_conversation_ids(
        &self,
        state: &AppState,
    ) -> Result<std::collections::HashSet<String>, String> {
        let mut ids = std::collections::HashSet::<String>::new();
        if let Ok(bindings) = state.active_chat_view_bindings.lock() {
            for binding in bindings.values() {
                let conversation_id = binding.conversation_id.trim();
                if !conversation_id.is_empty() {
                    ids.insert(conversation_id.to_string());
                }
            }
        }
        let active_contact_ids = state
            .remote_im_contact_runtime_states
            .lock()
            .map(|runtime_states| {
                runtime_states
                    .iter()
                    .filter(|(_, runtime_state)| {
                        self.remote_im_runtime_state_should_cache_blocks(runtime_state)
                    })
                    .map(|(contact_id, _)| contact_id.trim().to_string())
                    .filter(|contact_id| !contact_id.is_empty())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !active_contact_ids.is_empty() {
            let contact_ids = active_contact_ids
                .into_iter()
                .collect::<std::collections::HashSet<_>>();
            let runtime = state_read_runtime_state_cached(state)?;
            let mut unresolved_contact_ids = std::collections::HashSet::<String>::new();
            for contact in runtime
                .remote_im_contacts
                .iter()
                .filter(|contact| contact_ids.contains(contact.id.trim()))
            {
                if let Some(bound_conversation_id) = contact
                    .bound_conversation_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    ids.insert(bound_conversation_id.to_string());
                } else {
                    unresolved_contact_ids.insert(contact.id.trim().to_string());
                }
            }
            if !unresolved_contact_ids.is_empty() {
                let chat_index = state_read_chat_index_cached(state)?;
                let conversation_key_map = runtime
                    .remote_im_contacts
                    .iter()
                    .filter(|contact| unresolved_contact_ids.contains(contact.id.trim()))
                    .map(|contact| {
                        (
                            remote_im_contact_conversation_key(contact),
                            contact.id.trim().to_string(),
                        )
                    })
                    .collect::<std::collections::HashMap<_, _>>();
                let mapped_ids = chat_index
                    .conversations
                    .iter()
                    .filter_map(|item| {
                        let conversation_meta = match self.get_conversation_meta(
                            state,
                            item.id.as_str(),
                        ) {
                            Ok(conversation_meta) => conversation_meta,
                            Err(err) => {
                                eprintln!(
                                    "[会话索引读取] 状态=失败，任务=collect_block_cache_whitelist_conversation_ids，conversation_id={}，error={}",
                                    item.id, err
                                );
                                return None;
                            }
                        };
                        let root_key = conversation_meta.root_conversation_id.as_deref()?;
                        if !conversation_meta.summary.trim().is_empty()
                            || !conversation_meta.is_remote_im_contact
                            || !conversation_key_map.contains_key(root_key)
                        {
                            return None;
                        }
                        Some(conversation_meta.id.to_string())
                    })
                    .collect::<Vec<_>>();
                ids.extend(mapped_ids);
            }
        }
        Ok(ids)
    }

    fn retain_message_store_block_cache_whitelist(
        &self,
        state: &AppState,
    ) -> Result<(), String> {
        let conversation_ids = self.collect_block_cache_whitelist_conversation_ids(state)?;
        let mut allowed_paths = std::collections::HashSet::<PathBuf>::new();
        for conversation_id in conversation_ids {
            let paths = message_store::message_store_paths(&state.data_path, &conversation_id)?;
            if let Some(block_paths) =
                message_store::read_ready_message_store_latest_block_paths(&paths, 2)?
            {
                allowed_paths.extend(block_paths);
            }
        }
        message_store::retain_message_store_block_file_cache_paths(&allowed_paths);
        Ok(())
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
        let target_conversation_meta = self
            .get_conversation_meta(state, normalized_target_session_id)
            .map_err(|_| "目标会话不存在".to_string())?;
        if !self.conversation_meta_is_unarchived_meta_view(&target_conversation_meta) {
            return Err("目标会话不存在".to_string());
        }

        if target_conversation_meta.is_remote_im_contact {
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
            )
            .await?;
            runtime_log_info(format!(
                "[会话通知] 完成，任务=远程联系人投递，action={}，target_conversation_id={}，remote_contact_id={}，channel_id={}，message_id={}",
                action,
                normalized_target_session_id,
                contact.id,
                contact.channel_id,
                message.id
            ));
        } else if !target_conversation_meta.visible_in_foreground_lists
            || !self.conversation_meta_is_local_normal_chat_meta_view(&target_conversation_meta)
        {
            return Err("目标会话不存在".to_string());
        }

        self.append_message(state, normalized_target_session_id, message)?;
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

    fn list_archives(
        &self,
        state: &AppState,
    ) -> Result<Vec<ArchiveSummary>, String> {
        let guard = state.conversation_lock.lock().map_err(|err| {
            named_lock_error("conversation_lock", file!(), line!(), module_path!(), &err)
        })?;

        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let chat_index = state_read_chat_index_cached(state)?;
        let mut summaries = chat_index
            .conversations
            .iter()
            .filter(|item| chat_index_item_is_archived(item))
            .filter_map(|item| match self.get_conversation_meta(state, item.id.as_str()) {
                Ok(conversation_meta) => Some(conversation_meta),
                Err(err) => {
                    eprintln!(
                        "[会话索引读取] 状态=失败，任务=list_archives，conversation_id={}，error={}",
                        item.id, err
                    );
                    None
                }
            })
            .filter(|archive_meta| archive_meta.status.trim() == "archived")
            .map(|archive_meta| {
                let api_config_id = runtime_department_by_id(
                    &runtime_snapshot,
                    archive_meta.department_id.trim(),
                )
                .or_else(|| {
                    runtime_department_for_agent(&runtime_snapshot, archive_meta.agent_id.as_str())
                })
                .map(department_primary_api_config_id)
                .unwrap_or_default();
                let title = if archive_meta.title.trim().is_empty() {
                    let store_paths =
                        message_store::message_store_paths(&state.data_path, &archive_meta.id).ok();
                    store_paths
                        .and_then(|paths| {
                            message_store::read_ready_message_store_index_summary(&paths)
                                .ok()
                                .flatten()
                        })
                        .and_then(|summary| summary.first_user_text_preview)
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| "无内容".to_string())
                } else {
                    archive_meta.title.trim().to_string()
                };
                ArchiveSummary {
                    archive_id: archive_meta.id.to_string(),
                    archived_at: archive_meta
                        .archived_at
                        .clone()
                        .unwrap_or_else(|| archive_meta.updated_at.to_string()),
                    title,
                    message_count: archive_meta.message_count,
                    api_config_id,
                    agent_id: archive_meta.agent_id.to_string(),
                }
            })
            .collect::<Vec<_>>();
        summaries.sort_by(|a, b| b.archived_at.cmp(&a.archived_at));
        drop(guard);
        Ok(summaries)
    }

    fn get_archive_messages(
        &self,
        state: &AppState,
        archive_id: &str,
    ) -> Result<Vec<ChatMessage>, String> {
        let normalized_archive_id = archive_id.trim();
        if normalized_archive_id.is_empty() {
            return Err("archiveId is required".to_string());
        }
        let store_paths =
            message_store::message_store_paths(&state.data_path, normalized_archive_id)?;
        if let Some(mut messages) =
            message_store::read_ready_message_store_all_messages(&store_paths)?
        {
            materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
            return Ok(messages);
        }
        let guard = state.conversation_lock.lock().map_err(|err| {
            named_lock_error("conversation_lock", file!(), line!(), module_path!(), &err)
        })?;
        ensure_archive_ready_message_store_from_legacy(state, normalized_archive_id, &store_paths)?;
        drop(guard);
        let mut messages = message_store::read_ready_message_store_all_messages(&store_paths)?
            .ok_or_else(|| format!("归档消息仓库不可读，archive_id={normalized_archive_id}"))?;
        materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
        Ok(messages)
    }

    fn get_archive_block_page(
        &self,
        state: &AppState,
        archive_id: &str,
        block_id: Option<u32>,
    ) -> Result<ConversationBlockPageResult, String> {
        let normalized_archive_id = archive_id.trim();
        if normalized_archive_id.is_empty() {
            return Err("archiveId is required".to_string());
        }
        let store_paths = message_store::message_store_paths(&state.data_path, normalized_archive_id)?;
        if let Some(page) = message_store::read_ready_message_store_block_page(&store_paths, block_id)? {
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
        ensure_archive_ready_message_store_from_legacy(state, normalized_archive_id, &store_paths)?;
        drop(guard);
        let page = message_store::read_ready_message_store_block_page(&store_paths, block_id)?
            .ok_or_else(|| format!("归档块分页不可读，archive_id={normalized_archive_id}"))?;
        let mut messages = page.messages;
        materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
        Ok(ConversationBlockPageResult {
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
        })
    }

    fn get_archive_summary(
        &self,
        state: &AppState,
        archive_id: &str,
    ) -> Result<String, String> {
        let normalized_archive_id = archive_id.trim();
        if normalized_archive_id.is_empty() {
            return Err("archiveId is required".to_string());
        }
        let guard = state.conversation_lock.lock().map_err(|err| {
            named_lock_error("conversation_lock", file!(), line!(), module_path!(), &err)
        })?;
        let summary = self
            .get_conversation_meta(state, normalized_archive_id)
            .map_err(|_| "Archive not found".to_string())
            .and_then(|conversation_meta| {
                if conversation_meta.status.trim() != "archived" {
                    Err("Archive not found".to_string())
                } else {
                    Ok(conversation_meta.summary)
                }
            })?;
        drop(guard);
        Ok(summary)
    }

    fn delete_archive(
        &self,
        state: &AppState,
        archive_id: &str,
    ) -> Result<(), String> {
        let normalized_archive_id = archive_id.trim();
        if normalized_archive_id.is_empty() {
            return Err("archiveId is required".to_string());
        }
        let guard = state.conversation_lock.lock().map_err(|err| {
            named_lock_error("conversation_lock", file!(), line!(), module_path!(), &err)
        })?;
        let conversation_meta = self
            .get_conversation_meta(state, normalized_archive_id)
            .map_err(|_| "Archive not found".to_string())?;
        if conversation_meta.status.trim() != "archived" {
            drop(guard);
            return Err("Archive not found".to_string());
        }
        state_schedule_conversation_delete(state, normalized_archive_id)?;
        drop(guard);
        Ok(())
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
            let conversation_meta = self
                .get_conversation_meta(state, conversation_id)
                .map_err(|_| "当前没有可归档的活动对话。".to_string())?;
            if self.conversation_meta_is_local_normal_chat_meta_view(&conversation_meta) {
                Some(conversation_meta.id.to_string())
            } else {
                None
            }
        } else {
            self.resolve_latest_foreground_conversation_id(state, &effective_agent_id)?
        }
        .ok_or_else(|| "当前没有可归档的活动对话。".to_string())?;
        let source_meta = self
            .get_conversation_meta(state, &source_conversation_id)
            .map_err(|_| "当前没有可归档的活动对话。".to_string())?;
        if !self.conversation_meta_is_local_normal_chat_meta_view(&source_meta) {
            drop(guard);
            return Err("当前没有可归档的活动对话。".to_string());
        }
        let source_agent_id = source_meta.agent_id.trim();
        if source_agent_id.is_empty() {
            drop(guard);
            return Err(format!(
                "会话未绑定人格，无法归档: conversation_id={}",
                source_meta.id
            ));
        }
        if !runtime_agents
            .iter()
            .any(|agent| agent.id == source_agent_id && !agent.is_built_in_user)
        {
            drop(guard);
            return Err(format!(
                "会话绑定人格不存在或不可用，无法归档: conversation_id={}, agent_id={}",
                source_meta.id, source_agent_id
            ));
        }
        let source = self.get_conversation_snapshot(state, &source_meta.id)?;
        let effective_agent_id = source_agent_id.to_string();
        drop(guard);
        Ok((selected_api, resolved_api, source, effective_agent_id))
    }

    fn resolve_archive_request_conversation_by_id(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<(ApiConfig, ResolvedApiConfig, Conversation, String), String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required".to_string());
        }
        let guard = state.conversation_lock.lock().map_err(|err| {
            format!(
                "Failed to lock state mutex at {}:{} {}: {err}",
                file!(),
                line!(),
                module_path!()
            )
        })?;
        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let app_config = &runtime_snapshot.config;
        let source_meta = self
            .get_conversation_meta(state, normalized_conversation_id)
            .map_err(|_| "当前没有可归档的活动对话。".to_string())?;
        if !self.conversation_meta_is_local_normal_chat_meta_view(&source_meta)
            && source_meta.status.trim() != "archived"
        {
            drop(guard);
            return Err("当前没有可归档的活动对话。".to_string());
        }
        let department_id = source_meta.department_id.trim();
        let department = if department_id.is_empty() {
            runtime_log_warn(format!(
                "[归档] 跳过部门校验，任务=resolve_archive_request_conversation_by_id，conversation_id={}，原因=会话未绑定部门，改为直接归档并跳过归档反思",
                source_meta.id
            ));
            None
        } else {
            match runtime_department_by_id(&runtime_snapshot, department_id) {
                Some(department) => Some(department),
                None => {
                    runtime_log_warn(format!(
                        "[归档] 跳过部门校验，任务=resolve_archive_request_conversation_by_id，conversation_id={}，department_id={}，原因=会话绑定部门不存在，改为直接归档并跳过归档反思",
                        source_meta.id, department_id
                    ));
                    None
                }
            }
        };
        let effective_agent_id = source_meta.agent_id.trim();
        let effective_agent_id = if effective_agent_id.is_empty() {
            runtime_log_warn(format!(
                "[归档] 跳过人格校验，任务=resolve_archive_request_conversation_by_id，conversation_id={}，原因=会话未绑定人格，不再回退到其他人格；后续如无法确定归档归属，将直接跳过归档反思",
                source_meta.id
            ));
            String::new()
        } else if runtime_snapshot
            .agents
            .iter()
            .any(|agent| agent.id == effective_agent_id && !agent.is_built_in_user)
        {
            effective_agent_id.to_string()
        } else {
            runtime_log_warn(format!(
                "[归档] 跳过人格校验，任务=resolve_archive_request_conversation_by_id，conversation_id={}，agent_id={}，原因=会话绑定人格不存在或不可用，不再回退到其他人格；后续如无法确定归档归属，将直接跳过归档反思",
                source_meta.id, effective_agent_id
            ));
            effective_agent_id.to_string()
        };
        let preferred_api_id = source_meta
            .preferred_api_config_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .and_then(|api_id| resolve_department_chat_api_config_id(app_config, api_id));
        let selected_api_id = preferred_api_id.or_else(|| {
            department.and_then(|department| department_primary_chat_api_config_id(app_config, department))
        });
        let selected_api = resolve_selected_api_config(app_config, selected_api_id.as_deref())
            .ok_or_else(|| "No API config configured. Please add one.".to_string())?;
        let resolved_api = resolve_api_config(app_config, Some(selected_api.id.as_str()))?;
        let source = self.get_conversation_snapshot(state, &source_meta.id)?;
        drop(guard);
        Ok((selected_api, resolved_api, source, effective_agent_id))
    }

    fn delete_main_conversation_and_activate_latest(
        &self,
        state: &AppState,
        selected_api: &ApiConfig,
        source: &Conversation,
    ) -> Result<String, String> {
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let mut runtime = state_read_runtime_state_cached(state)?;
        let agents = state_read_agents_cached(state)?;
        let source_conversation = read_conversation_for_backup_cleanup(state, &source.id)
            .map_err(|_| "活动对话已变化，请重试归档。".to_string())?;
        if source_conversation.summary.trim().is_empty() || conversation_is_delegate(&source_conversation) {
            drop(guard);
            return Err("活动对话已变化，请重试归档。".to_string());
        }
        match cleanup_backup_records_from_messages(&state.data_path, &source_conversation.messages) {
            Ok(cleaned) if cleaned > 0 => {
                eprintln!(
                    "[会话删除] apply_patch 备份清理完成: conversation={}, cleaned={}",
                    source.id, cleaned
                );
            }
            Err(err) => {
                eprintln!(
                    "[会话删除] apply_patch 备份清理失败: conversation={}, error={}",
                    source.id, err
                );
            }
            _ => {}
        }
        state_schedule_conversation_delete(state, &source.id)?;
        let system_notification_exists = self
            .get_conversation_meta(state, SYSTEM_NOTIFICATION_CONVERSATION_ID)
            .ok()
            .filter(|conversation_meta| {
                self.conversation_meta_is_unarchived_meta_view(conversation_meta)
                    && conversation_meta.visible_in_foreground_lists
                    && self.conversation_meta_is_system_notification_meta_view(conversation_meta)
            })
            .is_some();
        if !system_notification_exists {
            let system_notification = build_system_notification_conversation_record();
            state_schedule_conversation_persist(state, &system_notification)?;
        }
        if runtime.main_conversation_id.as_deref().map(str::trim)
            != Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
        {
            runtime.main_conversation_id = Some(SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string());
            state_write_runtime_state_cached(state, &runtime)?;
        }
        let chat_index = state_read_chat_index_cached(state)?;
        let active_conversation_id = chat_index
            .conversations
            .iter()
            .filter_map(|item| self.get_conversation_meta(state, item.id.as_str()).ok())
            .find(|conversation_meta| {
                conversation_meta.id != source.id
                    && !conversation_meta.is_delegate
                    && self.conversation_meta_is_local_normal_chat_meta_view(conversation_meta)
            })
            .map(|conversation_meta| conversation_meta.id.to_string());
        let active_conversation_id = if let Some(active_conversation_id) = active_conversation_id {
            active_conversation_id
        } else {
            let replacement = build_archive_replacement_conversation(
                state,
                &agents,
                &runtime.assistant_department_agent_id,
                selected_api,
                &source_conversation,
            )?;
            let replacement_id = replacement.id.clone();
            state_schedule_conversation_persist(state, &replacement)?;
            replacement_id
        };
        drop(guard);

        cleanup_pdf_session_memory_cache_for_conversation(&source.id);
        Ok(active_conversation_id)
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
        let source_meta = self
            .get_conversation_meta(state, &source.id)
            .map_err(|_| "活动对话已变化，请重试上下文整理。".to_string())?;
        if !self.conversation_meta_is_unarchived_meta_view(&source_meta) {
            drop(guard);
            return Err("活动对话已变化，请重试上下文整理。".to_string());
        }
        let store_paths = message_store::message_store_paths(&state.data_path, &source.id)?;
        ensure_ready_message_store_from_legacy_conversation(state, &source.id, &store_paths)?;
        let previous_latest_block_id = message_store::read_ready_message_store_block_page(
            &store_paths,
            None,
        )?
        .map(|page| page.selected_block_id);
        let compression_message_id = compression_message.id.clone();
        let now = now_iso();
        let next_user_profile_snapshot = user_profile_snapshot.unwrap_or_default();
        let (conversation, (), _) = state_update_conversation_metadata_cached(
            state,
            &source.id,
            |cached| {
                cached.user_profile_snapshot = next_user_profile_snapshot.clone();
                cached.updated_at = now.clone();
                cached.last_user_at = Some(now.clone());
                Ok(())
            },
        )?;
        let active_conversation_id = Some(conversation.id.clone());
        let mut conversation_meta = message_store::read_ready_message_store_meta(&store_paths)?
            .ok_or_else(|| {
                format!(
                    "写入上下文整理消息失败：缺少 ready 消息元数据，conversation_id={}",
                    conversation.id
                )
            })?;
        conversation_meta.apply_metadata_fields_from_conversation(&conversation);
        conversation_meta.apply_appended_messages(std::slice::from_ref(compression_message));
        message_store::write_jsonl_snapshot_appended_messages_shard_from_meta(
            &store_paths,
            &conversation_meta,
            std::slice::from_ref(compression_message),
        )?;
        state_mark_conversation_metadata_direct_persisted(state, &conversation.id)?;

        drop(guard);

        let persisted = message_store::read_ready_message_store_message_by_id(
            &store_paths,
            &compression_message_id,
        )?
        .is_some();
        if !persisted {
            return Err(
                "上下文整理消息写入校验失败：已执行整理但未找到落盘消息，请重试。".to_string(),
            );
        }
        let latest_block = message_store::read_ready_message_store_block_page(&store_paths, None)?
            .ok_or_else(|| {
                format!(
                    "上下文整理消息写入校验失败：缺少最新块，conversation_id={}",
                    source.id
                )
            })?;
        if previous_latest_block_id.is_some()
            && Some(latest_block.selected_block_id) == previous_latest_block_id
        {
            return Err(format!(
                "上下文整理消息写入校验失败：未创建新的摘要块，conversation_id={}",
                source.id
            ));
        }
        let first_message_id = latest_block
            .blocks
            .iter()
            .find(|block| block.block_id == latest_block.selected_block_id)
            .map(|block| block.first_message_id.as_str())
            .unwrap_or_default();
        if first_message_id.trim() != compression_message_id {
            return Err(format!(
                "上下文整理消息写入校验失败：摘要消息不是新块首条消息，conversation_id={}",
                source.id
            ));
        }

        Ok(CompactionMessagePersistResult {
            active_conversation_id,
            compression_message_id,
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
            self.import_conversation_snapshot(
                state,
                &format!("archive_import_{}", archive_id),
                "archive_import",
                "archive_json_import",
                &conversation,
            )?;
            if existing_archive_ids.contains(&conversation_id) {
                replaced_count += 1;
            } else {
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

    fn set_routing(
        &self,
        state: &AppState,
        conversation_id: &str,
        department_id: Option<&str>,
        agent_id: Option<&str>,
        root_conversation_id: Option<Option<String>>,
        conversation_kind: Option<&str>,
    ) -> Result<Conversation, String> {
        self.apply_external_metadata_patch(
            state,
            conversation_id,
            "conversation_v2_set_routing",
            ConversationExternalMetadataPatch {
                routing_department_id: department_id.map(|value| value.trim().to_string()),
                routing_agent_id: agent_id.map(|value| value.trim().to_string()),
                routing_root_conversation_id: root_conversation_id,
                routing_conversation_kind: conversation_kind
                    .map(|value| value.trim().to_string()),
                ..Default::default()
            },
        )
    }

    fn commit_scheduler_history_flush(
        &self,
        state: &AppState,
        conversation_id: &str,
        events: &[ChatPendingEvent],
        prepared_batches: Vec<Vec<(ChatMessage, Vec<String>)>>,
        history_flush_time: &str,
        should_seed_summary_context: bool,
        seeded_profile_snapshot: Option<&str>,
    ) -> Result<SchedulerHistoryFlushCommitResult, String> {
        let _guard = lock_conversation_with_metrics(state, "scheduler_commit")?;
        let conversation_meta = match self.get_conversation_meta(state, conversation_id) {
            Ok(conversation_meta) if conversation_meta.summary.trim().is_empty() => {
                conversation_meta
            }
            _ => {
                let event_ids = events
                    .iter()
                    .map(|event| event.id.clone())
                    .collect::<Vec<_>>();
                complete_pending_chat_events_with_error(
                    state,
                    &event_ids,
                    &format!("目标会话不存在，conversationId={conversation_id}"),
                )?;
                return Err(format!("目标会话不存在，conversationId={conversation_id}"));
            }
        };
        let mut conversation = self.build_conversation_record_from_meta_view(&conversation_meta);
        let mut runtime = state_read_runtime_state_cached(state)?;
        let remote_im_runtime_before = serde_json::to_vec(&(
            runtime.remote_im_contacts.clone(),
            runtime.remote_im_contact_checkpoints.clone(),
        ))
        .ok();

        let persisted_batch_messages = self.write_scheduler_persisted_message_batch_v2(
            conversation_id,
            events,
            prepared_batches,
            history_flush_time,
            should_seed_summary_context,
            conversation_meta.message_count > 0,
            conversation_meta.has_context_compaction_message,
            seeded_profile_snapshot,
            state,
            &mut conversation,
        );
        let (event_activate_flags, _activated_contacts) =
            self.handle_scheduler_remote_im_activations_v2(
                state,
                &runtime.remote_im_contacts,
                &mut runtime.remote_im_contact_checkpoints,
                &mut conversation,
                events,
                history_flush_time,
            )?;
        conversation.updated_at = history_flush_time.to_string();
        let (metadata_conversation, (), _) = state_update_conversation_metadata_cached(
            state,
            &conversation.id,
            |cached| {
                cached.user_profile_snapshot = conversation.user_profile_snapshot.clone();
                cached.memory_recall_table = conversation.memory_recall_table.clone();
                cached.unread_count = conversation.unread_count;
                cached.updated_at = conversation.updated_at.clone();
                cached.last_user_at = conversation.last_user_at.clone();
                cached.last_assistant_at = conversation.last_assistant_at.clone();
                Ok(())
            },
        )?;
        self.persist_scheduler_flush_appended_messages_v2(
            state,
            &metadata_conversation,
            &persisted_batch_messages,
            &runtime,
            remote_im_runtime_before,
        )?;
        Ok(SchedulerHistoryFlushCommitResult {
            persisted_batch_messages,
            event_activate_flags,
        })
    }

    fn write_scheduler_persisted_message_batch_v2(
        &self,
        conversation_id: &str,
        events: &[ChatPendingEvent],
        prepared_batches: Vec<Vec<(ChatMessage, Vec<String>)>>,
        history_flush_time: &str,
        should_seed_summary_context: bool,
        has_existing_messages: bool,
        has_summary_context: bool,
        seeded_profile_snapshot: Option<&str>,
        state: &AppState,
        conversation: &mut Conversation,
    ) -> Vec<ChatMessage> {
        let mut persisted_batch_messages = Vec::<ChatMessage>::new();
        if should_seed_summary_context
            && !has_existing_messages
            && conversation.messages.is_empty()
            && !has_summary_context
            && !conversation_is_delegate(conversation)
            && !conversation_is_remote_im_contact(conversation)
        {
            if conversation.user_profile_snapshot.trim().is_empty() {
                if let Some(snapshot) = seeded_profile_snapshot {
                    conversation.user_profile_snapshot = snapshot.to_string();
                }
            }
            let summary_message = build_initial_summary_context_message(
                Some(conversation.user_profile_snapshot.as_str()),
                Some(&conversation.current_todos),
                None,
            );
            persisted_batch_messages.push(summary_message.clone());
            conversation.messages.push(summary_message);
        }

        for (event, prepared_messages) in events.iter().zip(prepared_batches.into_iter()) {
            self.append_scheduler_prepared_messages_to_conversation_v2(
                state,
                conversation,
                conversation_id,
                event,
                prepared_messages,
                history_flush_time,
                &mut persisted_batch_messages,
            );
        }
        persisted_batch_messages
    }

    fn append_scheduler_prepared_messages_to_conversation_v2(
        &self,
        state: &AppState,
        conversation: &mut Conversation,
        conversation_id: &str,
        event: &ChatPendingEvent,
        prepared_messages: Vec<(ChatMessage, Vec<String>)>,
        history_flush_time: &str,
        persisted_batch_messages: &mut Vec<ChatMessage>,
    ) {
        for (persisted, recall_ids) in prepared_messages {
            if persisted.role.trim() == "user" && !recall_ids.is_empty() {
                for memory_id in &recall_ids {
                    conversation.memory_recall_table.push(memory_id.clone());
                }
                eprintln!(
                    "[记忆RAG][出队消息写入] conversation_id={} user_message_id={} agent_id={} retrieved_memory_ids={:?}",
                    conversation_id,
                    persisted.id,
                    event.session_info.agent_id,
                    persisted
                        .provider_meta
                        .as_ref()
                        .and_then(|meta| meta.get("retrieved_memory_ids"))
                        .and_then(Value::as_array)
                        .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>())
                        .unwrap_or_default()
                );
            }
            let persisted_for_event = persisted.clone();
            match persisted.role.trim() {
                "user" => conversation.last_user_at = Some(history_flush_time.to_string()),
                "assistant" => {
                    conversation.last_assistant_at = Some(history_flush_time.to_string())
                }
                _ => {}
            }
            conversation.messages.push(persisted);
            self.increment_conversation_unread_count_if_background(state, conversation, 1);
            persisted_batch_messages.push(persisted_for_event);
        }
    }

    fn handle_scheduler_remote_im_activations_v2(
        &self,
        state: &AppState,
        contacts: &[RemoteImContact],
        checkpoints: &mut Vec<RemoteImContactCheckpoint>,
        conversation: &mut Conversation,
        events: &[ChatPendingEvent],
        history_flush_time: &str,
    ) -> Result<(Vec<bool>, std::collections::HashSet<String>), String> {
        let mut event_activate_flags = Vec::<bool>::with_capacity(events.len());
        let mut activated_contacts_in_batch = std::collections::HashSet::<String>::new();
        for event in events {
            let event_should_activate = if matches!(event.source, ChatEventSource::RemoteIm) {
                remote_im_handle_persisted_event_after_history_flush_runtime(
                    state,
                    contacts,
                    checkpoints,
                    conversation,
                    event,
                    history_flush_time,
                    &mut activated_contacts_in_batch,
                )?
            } else {
                event.activate_assistant
            };
            event_activate_flags.push(event_should_activate);
        }
        Ok((event_activate_flags, activated_contacts_in_batch))
    }

    fn persist_scheduler_flush_appended_messages_v2(
        &self,
        state: &AppState,
        conversation: &Conversation,
        appended_messages: &[ChatMessage],
        runtime: &RuntimeStateFile,
        remote_im_runtime_before: Option<Vec<u8>>,
    ) -> Result<(), String> {
        let remote_im_runtime_changed = remote_im_runtime_before
            != serde_json::to_vec(&(
                runtime.remote_im_contacts.clone(),
                runtime.remote_im_contact_checkpoints.clone(),
            ))
            .ok();
        let paths = message_store::message_store_paths(&state.data_path, &conversation.id)?;
        let mut conversation_meta = message_store::read_ready_message_store_meta(&paths)?
            .ok_or_else(|| {
                format!(
                    "历史回灌落盘失败：缺少 ready 消息元数据，conversation_id={}",
                    conversation.id
                )
            })?;
        conversation_meta.apply_metadata_fields_from_conversation(conversation);
        conversation_meta.apply_appended_messages(appended_messages);
        message_store::write_jsonl_snapshot_appended_messages_shard_from_meta(
            &paths,
            &conversation_meta,
            appended_messages,
        )?;
        state_mark_conversation_metadata_direct_persisted(state, &conversation.id)?;
        if remote_im_runtime_changed {
            state_write_runtime_state_cached(state, runtime)?;
        }
        Ok(())
    }

    fn import_conversation_snapshot(
        &self,
        state: &AppState,
        job_id: &str,
        operator: &str,
        reason: &str,
        snapshot: &Conversation,
    ) -> Result<(), String> {
        self.apply_privileged_snapshot_overwrite(
            state,
            &ConversationOverwriteAudit {
                job_id: job_id.trim().to_string(),
                source: ConversationOverwriteSource::Import,
                operator: operator.trim().to_string(),
                reason: reason.trim().to_string(),
            },
            snapshot,
        )
    }

    #[cfg(test)]
    fn sync_replace_conversation_snapshot(
        &self,
        state: &AppState,
        job_id: &str,
        operator: &str,
        reason: &str,
        snapshot: &Conversation,
    ) -> Result<(), String> {
        self.apply_privileged_snapshot_overwrite(
            state,
            &ConversationOverwriteAudit {
                job_id: job_id.trim().to_string(),
                source: ConversationOverwriteSource::ExportSync,
                operator: operator.trim().to_string(),
                reason: reason.trim().to_string(),
            },
            snapshot,
        )
    }

    fn recover_conversation_snapshot(
        &self,
        state: &AppState,
        job_id: &str,
        operator: &str,
        reason: &str,
        snapshot: &Conversation,
    ) -> Result<(), String> {
        self.apply_privileged_snapshot_overwrite(
            state,
            &ConversationOverwriteAudit {
                job_id: job_id.trim().to_string(),
                source: ConversationOverwriteSource::MigrationRecovery,
                operator: operator.trim().to_string(),
                reason: reason.trim().to_string(),
            },
            snapshot,
        )
    }

    #[cfg(test)]
    fn set_conversation_preferred_api_config_id(
        &self,
        state: &AppState,
        conversation_id: &str,
        preferred_api_config_id: Option<String>,
    ) -> Result<Conversation, String> {
        self.set_preferred_api_config_id(state, conversation_id, preferred_api_config_id)
    }

    #[cfg(test)]
    fn set_conversation_auto_push_remote_contact_id(
        &self,
        state: &AppState,
        conversation_id: &str,
        auto_push_remote_contact_id: Option<String>,
    ) -> Result<Conversation, String> {
        self.set_auto_push_remote_contact_id(state, conversation_id, auto_push_remote_contact_id)
    }

    #[cfg(test)]
    fn read_foreground_snapshot(
        &self,
        state: &AppState,
        conversation_id: Option<&str>,
        agent_id: Option<&str>,
        recent_limit: usize,
    ) -> Result<ForegroundConversationSnapshotCore, String> {
        self.get_foreground_snapshot(state, conversation_id, agent_id, recent_limit)
    }

    #[cfg(test)]
    fn set_conversation_title_metadata(
        &self,
        state: &AppState,
        conversation_id: &str,
        next_title: &str,
    ) -> Result<Conversation, String> {
        self.set_title(state, conversation_id, next_title)
    }

    #[cfg(test)]
    fn set_conversation_shell_workspace_metadata(
        &self,
        state: &AppState,
        conversation_id: &str,
        shell_workspace_path: Option<Option<String>>,
        shell_workspaces: Option<Vec<ShellWorkspaceConfig>>,
        shell_autonomous_mode: Option<bool>,
    ) -> Result<Conversation, String> {
        self.set_shell_workspace(
            state,
            conversation_id,
            shell_workspace_path,
            shell_workspaces,
            shell_autonomous_mode,
        )
    }

    #[cfg(test)]
    fn set_conversation_lifecycle_metadata(
        &self,
        state: &AppState,
        conversation_id: &str,
        status: Option<&str>,
        summary: Option<&str>,
        archived_at: Option<Option<String>>,
        updated_at: Option<String>,
    ) -> Result<Conversation, String> {
        self.apply_external_metadata_patch(
            state,
            conversation_id,
            "conversation_v2_test_set_lifecycle_metadata",
            ConversationExternalMetadataPatch {
                lifecycle_status: status.map(|value| value.trim().to_string()),
                lifecycle_summary: summary.map(ToOwned::to_owned),
                lifecycle_archived_at: archived_at,
                lifecycle_updated_at: updated_at,
                ..Default::default()
            },
        )
    }

    #[cfg(test)]
    fn set_conversation_current_todos_metadata(
        &self,
        state: &AppState,
        conversation_id: &str,
        current_todos: Vec<ConversationTodoItem>,
    ) -> Result<Conversation, String> {
        self.set_current_todos(state, conversation_id, current_todos)
    }

    #[cfg(test)]
    fn append_message_to_unarchived_conversation(
        &self,
        state: &AppState,
        conversation_id: &str,
        message: &ChatMessage,
    ) -> Result<(), String> {
        self.append_message(state, conversation_id, message)
    }

    #[cfg(test)]
    fn read_message_by_id(
        &self,
        state: &AppState,
        conversation_id: &str,
        message_id: &str,
    ) -> Result<ChatMessage, String> {
        self.get_message_by_id(state, conversation_id, message_id)
    }

    #[cfg(test)]
    fn read_archive_block_page(
        &self,
        state: &AppState,
        archive_id: &str,
        block_id: Option<u32>,
    ) -> Result<ConversationBlockPageResult, String> {
        self.get_archive_block_page(state, archive_id, block_id)
    }

    #[cfg(test)]
    fn read_unarchived_messages(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Vec<ChatMessage>, String> {
        self.get_all_messages(state, conversation_id)
    }

    #[cfg(test)]
    fn rewind_conversation_from_message(
        &self,
        state: &AppState,
        input: &RewindConversationInput,
        message_id: &str,
        started_at: &std::time::Instant,
    ) -> Result<RewindConversationMutationResult, String> {
        self.rewind_conversation(state, input, message_id, started_at)
    }

    fn append_tool_event_to_assistant_message(
        &self,
        state: &AppState,
        input: &AssistantMessageToolAppendInput,
    ) -> Result<AssistantMessageToolAppendResult, String> {
        let conversation_id = input.conversation_id.trim();
        let assistant_message_id = input.assistant_message_id.trim();
        if conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        if assistant_message_id.is_empty() {
            return Err("assistantMessageId is required.".to_string());
        }
        let target_message = self.read_current_writable_assistant_message(
            state,
            conversation_id,
            assistant_message_id,
        )?;
        if assistant_message_tool_append_closed(&target_message) {
            return Err(
                ConversationServiceV2Error::new(
                    ConversationServiceV2ErrorCode::ToolAppendClosed,
                    format!(
                        "目标 assistant message 已关闭 tool append，conversationId={}，assistantMessageId={}",
                        conversation_id, assistant_message_id
                    ),
                )
                .into_string(),
            );
        }
        let agent_id = target_message
            .speaker_agent_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                ConversationServiceV2Error::new(
                    ConversationServiceV2ErrorCode::MessageNotWritable,
                    format!(
                        "目标 assistant message 缺少 speaker_agent_id，conversationId={}，assistantMessageId={}",
                        conversation_id, assistant_message_id
                    ),
                )
                .into_string()
            })?;
        let _guard = lock_conversation_with_metrics(
            state,
            "conversation_v2_append_tool_event_to_assistant_message",
        )?;
        let mut target_message = self.read_current_writable_assistant_message(
            state,
            conversation_id,
            assistant_message_id,
        )?;
        let normalized_agent_id = agent_id.trim();
        let target_agent_id = target_message
            .speaker_agent_id
            .as_deref()
            .map(str::trim)
            .unwrap_or_default();
        if target_agent_id != normalized_agent_id {
            return Err(
                ConversationServiceV2Error::new(
                    ConversationServiceV2ErrorCode::MessageNotWritable,
                    format!(
                        "目标 assistant message 的 speaker_agent_id 不匹配，conversationId={}，assistantMessageId={}，expectedAgentId={}，actualAgentId={}",
                        conversation_id,
                        assistant_message_id,
                        normalized_agent_id,
                        target_agent_id
                    ),
                )
                .into_string(),
            );
        }
        let tool_call_id = validate_tool_group_result_append_v2(
            &input.assistant_tool_event,
            &input.tool_result_event,
        )?;
        let group_call_ids =
            tool_call_ids_from_assistant_tool_event_v2(&input.assistant_tool_event);
        let events = target_message.tool_call.get_or_insert_with(Vec::new);
        if !tool_history_contains_tool_result_id_v2(events, &tool_call_id) {
            if !tool_history_contains_assistant_tool_group_v2(events, &group_call_ids) {
                events.push(input.assistant_tool_event.clone());
            }
            events.push(input.tool_result_event.clone());
        }
        merge_provider_meta_patch_v2(
            &mut target_message.provider_meta,
            input.provider_meta_patch.clone(),
        );
        let tool_event_count = target_message.tool_call.as_ref().map(Vec::len).unwrap_or(0);
        self.persist_replaced_ready_message(state, conversation_id, &target_message)?;
        Ok(AssistantMessageToolAppendResult {
            conversation_id: conversation_id.to_string(),
            assistant_message_id: assistant_message_id.to_string(),
            tool_event_count,
            tool_append_closed: false,
        })
    }

    fn append_final_text_to_assistant_message(
        &self,
        state: &AppState,
        input: &AssistantMessageFinalTextAppendInput,
    ) -> Result<AssistantMessageFinalTextAppendResult, String> {
        let conversation_id = input.conversation_id.trim();
        let assistant_message_id = input.assistant_message_id.trim();
        if conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        if assistant_message_id.is_empty() {
            return Err("assistantMessageId is required.".to_string());
        }
        let final_text = input.final_text.trim();
        let _guard = lock_conversation_with_metrics(
            state,
            "conversation_v2_append_final_text_to_assistant_message",
        )?;
        let mut target_message = self.read_current_writable_assistant_message(
            state,
            conversation_id,
            assistant_message_id,
        )?;
        if assistant_message_tool_append_closed(&target_message) {
            return Err(
                ConversationServiceV2Error::new(
                    ConversationServiceV2ErrorCode::FinalTextAlreadyCommitted,
                    format!(
                        "目标 assistant message 已有 final 正文，禁止重复提交，conversationId={}，assistantMessageId={}",
                        conversation_id, assistant_message_id
                    ),
                )
                .into_string(),
            );
        }
        let has_reasoning_patch = input
            .reasoning_text
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some();
        let has_provider_meta_patch = input
            .provider_meta_patch
            .as_ref()
            .and_then(Value::as_object)
            .map(|value| !value.is_empty())
            .unwrap_or(false);
        if final_text.is_empty()
            && !has_reasoning_patch
            && !has_provider_meta_patch
        {
            return Err("finalText/reasoningText/providerMetaPatch 至少需要一个".to_string());
        }

        let mut text_part_updated = false;
        for part in &mut target_message.parts {
            if let MessagePart::Text {
                text,
                reasoning_content,
            } = part
            {
                if !final_text.is_empty() {
                    *text = final_text.to_string();
                }
                merge_optional_text_block_v2(reasoning_content, input.reasoning_text.clone());
                text_part_updated = true;
                break;
            }
        }
        if !text_part_updated && (!final_text.is_empty() || has_reasoning_patch) {
            target_message.parts.push(MessagePart::Text {
                text: final_text.to_string(),
                reasoning_content: input.reasoning_text.clone(),
            });
        }
        merge_provider_meta_patch_v2(
            &mut target_message.provider_meta,
            input.provider_meta_patch.clone(),
        );
        runtime_log_debug(format!(
            "[表情替换] FinalAppend开始，conversation_id={}，assistant_message_id={}，existing_annotation_count={}，incoming_annotation_count={}，incoming_tokens=[{}]",
            conversation_id,
            assistant_message_id,
            target_message
                .meme_annotations
                .as_ref()
                .map(Vec::len)
                .unwrap_or(0),
            input
                .meme_annotations
                .as_ref()
                .map(Vec::len)
                .unwrap_or(0),
            input
                .meme_annotations
                .as_ref()
                .map(|items| items.iter().map(|item| item.meme.trim().to_string()).collect::<Vec<_>>().join(","))
                .unwrap_or_default()
        ));
        target_message.meme_annotations = input.meme_annotations.clone();
        mark_stream_final_committed_v2(&mut target_message.provider_meta);

        self.persist_replaced_ready_message(state, conversation_id, &target_message)?;
        runtime_log_debug(format!(
            "[表情替换] FinalAppend完成，conversation_id={}，assistant_message_id={}，stored_annotation_count={}，stored_tokens=[{}]",
            conversation_id,
            assistant_message_id,
            target_message
                .meme_annotations
                .as_ref()
                .map(Vec::len)
                .unwrap_or(0),
            target_message
                .meme_annotations
                .as_ref()
                .map(|items| items.iter().map(|item| item.meme.trim().to_string()).collect::<Vec<_>>().join(","))
                .unwrap_or_default()
        ));
        Ok(AssistantMessageFinalTextAppendResult {
            conversation_id: conversation_id.to_string(),
            assistant_message_id: assistant_message_id.to_string(),
            final_text_committed: true,
            tool_append_closed: true,
        })
    }

    fn bootstrap_streaming_assistant_message(
        &self,
        state: &AppState,
        input: &AssistantMessageBootstrapInput,
    ) -> Result<AssistantMessageBootstrapResult, String> {
        let conversation_id = input.conversation_id.trim();
        let assistant_message_id = input.assistant_message_id.trim();
        let speaker_agent_id = input.speaker_agent_id.trim();
        if conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        if assistant_message_id.is_empty() {
            return Err("assistantMessageId is required.".to_string());
        }
        if speaker_agent_id.is_empty() {
            return Err("speakerAgentId is required.".to_string());
        }
        if self
            .get_message_by_id(state, conversation_id, assistant_message_id)
            .is_ok()
        {
            return Ok(AssistantMessageBootstrapResult {
                conversation_id: conversation_id.to_string(),
                assistant_message_id: assistant_message_id.to_string(),
                created: false,
            });
        }
        let created_at = input
            .created_at
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(now_iso);
        let mut message = ChatMessage {
            id: assistant_message_id.to_string(),
            role: "assistant".to_string(),
            created_at,
            speaker_agent_id: Some(speaker_agent_id.to_string()),
            parts: vec![MessagePart::Text {
                text: String::new(),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: None,
            tool_call: None,
            mcp_call: None,
            meme_annotations: None,
        };
        merge_provider_meta_patch_v2(&mut message.provider_meta, input.provider_meta_patch.clone());
        self.append_message(state, conversation_id, &message)?;
        Ok(AssistantMessageBootstrapResult {
            conversation_id: conversation_id.to_string(),
            assistant_message_id: assistant_message_id.to_string(),
            created: true,
        })
    }

    fn patch_provider_meta_on_assistant_message(
        &self,
        state: &AppState,
        input: &AssistantMessageProviderMetaPatchInput,
    ) -> Result<AssistantMessageProviderMetaPatchResult, String> {
        let conversation_id = input.conversation_id.trim();
        let assistant_message_id = input.assistant_message_id.trim();
        if conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        if assistant_message_id.is_empty() {
            return Err("assistantMessageId is required.".to_string());
        }
        if !input.provider_meta_patch.is_object() {
            return Err("providerMetaPatch must be an object.".to_string());
        }
        let _guard = lock_conversation_with_metrics(
            state,
            "conversation_v2_patch_provider_meta_on_assistant_message",
        )?;
        let mut target_message = self.read_current_writable_assistant_message(
            state,
            conversation_id,
            assistant_message_id,
        )?;
        merge_provider_meta_patch_v2(
            &mut target_message.provider_meta,
            Some(input.provider_meta_patch.clone()),
        );
        self.persist_replaced_ready_message(state, conversation_id, &target_message)?;
        Ok(AssistantMessageProviderMetaPatchResult {
            conversation_id: conversation_id.to_string(),
            assistant_message_id: assistant_message_id.to_string(),
        })
    }

    fn patch_message_provider_meta_batch(
        &self,
        state: &AppState,
        input: &MessageProviderMetaBatchPatchInput,
    ) -> Result<(), String> {
        let conversation_id = input.conversation_id.trim();
        if conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        if input.items.is_empty() {
            return Ok(());
        }
        let mut patch_by_id = std::collections::HashMap::<String, Option<Value>>::new();
        for item in &input.items {
            let message_id = item.message_id.trim();
            if message_id.is_empty() {
                return Err("messageId is required.".to_string());
            }
            if let Some(meta) = item.provider_meta.as_ref() {
                if !meta.is_object() {
                    return Err(format!(
                        "providerMeta 必须是 object 或 null，messageId={message_id}"
                    ));
                }
            }
            patch_by_id.insert(message_id.to_string(), item.provider_meta.clone());
        }
        let _guard = lock_conversation_with_metrics(
            state,
            "conversation_v2_patch_message_provider_meta_batch",
        )?;
        let conversation_meta = self.get_conversation_meta(state, conversation_id)?;
        if !self.conversation_meta_is_unarchived_meta_view(&conversation_meta) {
            return Err(format!("Unarchived conversation not found: {conversation_id}"));
        }
        let paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
        ensure_ready_message_store_from_legacy_conversation(state, conversation_id, &paths)?;
        let mut ready_meta = message_store::read_ready_message_store_meta(&paths)?
            .ok_or_else(|| {
                format!(
                    "批量更新消息 providerMeta 失败：缺少 ready 消息元数据，conversation_id={conversation_id}"
                )
            })?;
        ready_meta.apply_metadata_fields_from_meta_view(&conversation_meta);
        let current = self.get_conversation_snapshot(state, conversation_id)?;
        let mut updated_messages = current.messages.clone();
        let mut matched_ids = std::collections::HashSet::<String>::new();
        for message in &mut updated_messages {
            let message_id = message.id.trim();
            if let Some(provider_meta) = patch_by_id.get(message_id) {
                message.provider_meta = provider_meta.clone();
                matched_ids.insert(message_id.to_string());
            }
        }
        let missing_ids = patch_by_id
            .keys()
            .filter(|message_id| !matched_ids.contains(*message_id))
            .cloned()
            .collect::<Vec<_>>();
        if !missing_ids.is_empty() {
            return Err(format!(
                "批量更新消息 providerMeta 失败：消息不存在，conversation_id={}，message_ids={}",
                conversation_id,
                missing_ids.join(",")
            ));
        }
        message_store::write_jsonl_snapshot_spliced_messages_shard(
            &paths,
            &ready_meta.to_persist_meta(),
            0,
            updated_messages.len(),
            &updated_messages,
        )?;
        state_mark_conversation_metadata_direct_persisted(state, conversation_id)?;
        Ok(())
    }
}

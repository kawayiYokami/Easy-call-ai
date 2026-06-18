fn latest_active_conversation_index(
    data: &AppData,
    _api_config_id: &str,
    _agent_id: &str,
) -> Option<usize> {
    data.conversations
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.summary.trim().is_empty()
                && conversation_visible_in_foreground_lists(c)
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
        .map(|(idx, _)| idx)
}

fn latest_main_conversation_index(data: &AppData, _agent_id: &str) -> Option<usize> {
    data.conversations
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            c.summary.trim().is_empty()
                && conversation_visible_in_foreground_lists(c)
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
        .map(|(idx, _)| idx)
}

fn system_notification_conversation_title() -> String {
    "P-ai系统".to_string()
}

fn normalize_system_notification_conversation(conversation: &mut Conversation) -> bool {
    let mut changed = false;
    if conversation.id.trim() != SYSTEM_NOTIFICATION_CONVERSATION_ID {
        conversation.id = SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string();
        changed = true;
    }
    if conversation.conversation_kind.trim() != CONVERSATION_KIND_SYSTEM_NOTIFICATION {
        conversation.conversation_kind = CONVERSATION_KIND_SYSTEM_NOTIFICATION.to_string();
        changed = true;
    }
    let expected_title = system_notification_conversation_title();
    if conversation.title.trim() != expected_title {
        conversation.title = expected_title;
        changed = true;
    }
    if conversation.department_id.trim().is_empty() {
        conversation.department_id = ASSISTANT_DEPARTMENT_ID.to_string();
        changed = true;
    }
    if conversation.status.trim().is_empty() {
        conversation.status = "active".to_string();
        changed = true;
    }
    changed
}

fn conversation_is_system_notification(conversation: &Conversation) -> bool {
    conversation.id.trim() == SYSTEM_NOTIFICATION_CONVERSATION_ID
        || conversation.conversation_kind.trim() == CONVERSATION_KIND_SYSTEM_NOTIFICATION
}

fn available_non_user_agent<'a>(
    agents: &'a [AgentProfile],
    agent_id: &str,
) -> Option<&'a AgentProfile> {
    let agent_id = agent_id.trim();
    if agent_id.is_empty() {
        return None;
    }
    agents
        .iter()
        .find(|agent| agent.id == agent_id && !agent.is_built_in_user)
}

fn first_available_department_agent<'a>(
    department: &DepartmentConfig,
    agents: &'a [AgentProfile],
) -> Option<&'a AgentProfile> {
    department
        .agent_ids
        .iter()
        .map(|id| id.trim())
        .find_map(|agent_id| available_non_user_agent(agents, agent_id))
}

fn resolve_conversation_bound_agent<'a>(
    conversation: &Conversation,
    agents: &'a [AgentProfile],
    departments: &[DepartmentConfig],
) -> Result<&'a AgentProfile, String> {
    let conversation_id = conversation.id.trim();
    let department_id = conversation.department_id.trim();
    let bound_department = if department_id.is_empty() {
        None
    } else {
        Some(
            departments
                .iter()
                .find(|department| department.id.trim() == department_id)
                .ok_or_else(|| {
                    format!(
                        "会话绑定部门不存在: conversation_id={}, department_id={}",
                        conversation_id, department_id
                    )
                })?,
        )
    };
    let bound_agent_id = conversation.agent_id.trim();
    if !bound_agent_id.is_empty() {
        if let Some(agent) = available_non_user_agent(agents, bound_agent_id) {
            return Ok(agent);
        }
        return Err(format!(
            "会话绑定人格不存在或不可用: conversation_id={}, agent_id={}",
            conversation_id, bound_agent_id
        ));
    }

    if let Some(department) = bound_department {
        return first_available_department_agent(department, agents).ok_or_else(|| {
            format!(
                "会话绑定部门没有可用人格: conversation_id={}, department_id={}",
                conversation_id, department_id
            )
        });
    }
    Err(format!(
        "会话缺少有效人格绑定: conversation_id={}, department_id={}",
        conversation_id, department_id
    ))
}

fn main_conversation_index(data: &AppData, _agent_id: &str) -> Option<usize> {
    let target_id = data
        .main_conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    data.conversations.iter().position(|conversation| {
        conversation.id == target_id
            && conversation.summary.trim().is_empty()
            && conversation_visible_in_foreground_lists(conversation)
    })
}

fn normalize_main_conversation_marker(data: &mut AppData, _agent_id: &str) -> bool {
    let fixed_id = SYSTEM_NOTIFICATION_CONVERSATION_ID;
    if let Some(idx) = data.conversations.iter().position(|conversation| {
        conversation.id.trim() == fixed_id
            && conversation.summary.trim().is_empty()
            && conversation_visible_in_foreground_lists(conversation)
    }) {
        let mut changed = normalize_system_notification_conversation(&mut data.conversations[idx]);
        if data.main_conversation_id.as_deref().map(str::trim) != Some(fixed_id) {
            data.main_conversation_id = Some(fixed_id.to_string());
            changed = true;
        }
        return changed;
    }
    if let Some(idx) = data.conversations.iter().position(|conversation| {
        conversation.summary.trim().is_empty()
            && conversation_visible_in_foreground_lists(conversation)
            && conversation_is_system_notification(conversation)
    }) {
        let mut changed = normalize_system_notification_conversation(&mut data.conversations[idx]);
        if data.main_conversation_id.as_deref().map(str::trim) != Some(fixed_id) {
            data.main_conversation_id = Some(fixed_id.to_string());
            changed = true;
        }
        return changed;
    }
    data.conversations.push(build_system_notification_conversation_record());
    data.main_conversation_id = Some(fixed_id.to_string());
    true
}

fn normalize_single_active_main_conversation(data: &mut AppData) -> bool {
    let Some(keep_idx) = latest_active_conversation_index(data, "", "")
        .or_else(|| latest_main_conversation_index(data, ""))
    else {
        return false;
    };

    let mut changed = false;
    for (_idx, conversation) in data.conversations.iter_mut().enumerate() {
        if !conversation_visible_in_foreground_lists(conversation) || !conversation.summary.trim().is_empty() {
            continue;
        }
        let target_status = "active";
        if conversation.status.trim() != target_status {
            conversation.status = target_status.to_string();
            changed = true;
        }
    }
    if changed {
        let keep_id = data
            .conversations
            .get(keep_idx)
            .map(|item| item.id.clone())
            .unwrap_or_default();
        eprintln!(
            "[会话] 归一化未归档会话激活标记: active_conversation_id={}",
            keep_id
        );
    }
    changed
}

fn conversation_is_delegate(conversation: &Conversation) -> bool {
    conversation.conversation_kind.trim() == CONVERSATION_KIND_DELEGATE
}

fn conversation_is_remote_im_contact(conversation: &Conversation) -> bool {
    conversation.conversation_kind.trim() == CONVERSATION_KIND_REMOTE_IM_CONTACT
}

fn increment_conversation_unread_count(conversation: &mut Conversation, count: usize) {
    if count == 0 || conversation_is_remote_im_contact(conversation) {
        return;
    }
    conversation.unread_count = conversation.unread_count.saturating_add(count);
}

fn clear_conversation_unread_count(conversation: &mut Conversation) -> bool {
    if conversation.unread_count == 0 {
        return false;
    }
    conversation.unread_count = 0;
    true
}

fn conversation_visible_in_foreground_lists(conversation: &Conversation) -> bool {
    !conversation_is_delegate(conversation)
        && !conversation_is_remote_im_contact(conversation)
}

fn conversation_is_unarchived(conversation: &Conversation) -> bool {
    !conversation_is_archived(conversation)
}

fn conversation_is_archived(conversation: &Conversation) -> bool {
    if conversation.status.trim() == "archived" {
        return true;
    }
    if conversation
        .archived_at
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
    {
        return true;
    }
    !conversation.summary.trim().is_empty()
}

const SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION: u64 = 2;
const SUMMARY_CONTEXT_TITLE_MAX_CHARS: usize = 20;

fn conversation_is_local_normal_chat(conversation: &Conversation) -> bool {
    conversation.conversation_kind.trim() == CONVERSATION_KIND_CHAT
        && !conversation_is_system_notification(conversation)
        && !conversation_is_delegate(conversation)
        && !conversation_is_remote_im_contact(conversation)
}

fn summary_context_message_kind(message: &ChatMessage) -> Option<&str> {
    let meta = message.provider_meta.as_ref()?;
    meta.get("message_meta")
        .and_then(|value| value.get("kind"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            meta.get("messageMeta")
                .and_then(|value| value.get("kind"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .or_else(|| {
            meta.get("messageKind")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
}

fn is_summary_context_message_kind(kind: &str) -> bool {
    matches!(kind.trim(), "context_compaction" | "summary_context_seed")
}

fn normalize_summary_context_title(raw: &str) -> Option<String> {
    let first_line = raw
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    let stripped = first_line
        .trim_matches(|ch| {
            matches!(
                ch,
                '"' | '\''
                    | '`'
                    | '“'
                    | '”'
                    | '‘'
                    | '’'
                    | '《'
                    | '》'
                    | '「'
                    | '」'
                    | '『'
                    | '』'
            )
        })
        .trim_matches(|ch| matches!(ch, '。' | '！' | '？' | '!' | '?' | '，' | ',' | '；' | ';' | '：' | ':' | '、'))
        .trim();
    let cleaned = clean_text(stripped);
    if cleaned.is_empty() {
        return None;
    }
    Some(
        cleaned
            .chars()
            .take(SUMMARY_CONTEXT_TITLE_MAX_CHARS)
            .collect::<String>(),
    )
}

fn summary_context_message_title(message: &ChatMessage) -> Option<String> {
    let kind = summary_context_message_kind(message)?;
    if !is_summary_context_message_kind(kind) {
        return None;
    }
    let meta = message.provider_meta.as_ref()?;
    meta.get("message_meta")
        .and_then(|value| value.get("title"))
        .and_then(Value::as_str)
        .and_then(normalize_summary_context_title)
        .or_else(|| {
            meta.get("messageMeta")
                .and_then(|value| value.get("title"))
                .and_then(Value::as_str)
                .and_then(normalize_summary_context_title)
        })
}

fn ensure_summary_context_message_meta_object_mut(
    message: &mut ChatMessage,
) -> Option<&mut serde_json::Map<String, Value>> {
    let provider_meta = message
        .provider_meta
        .get_or_insert_with(|| Value::Object(serde_json::Map::new()));
    if !provider_meta.is_object() {
        *provider_meta = Value::Object(serde_json::Map::new());
    }
    let Some(root) = provider_meta.as_object_mut() else {
        return None;
    };
    let message_meta = root
        .entry("message_meta".to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    if !message_meta.is_object() {
        *message_meta = Value::Object(serde_json::Map::new());
    }
    message_meta.as_object_mut()
}

fn conversation_update_latest_summary_title(
    conversation: &mut Conversation,
    next_title: Option<&str>,
) -> bool {
    let normalized_title = next_title.and_then(normalize_summary_context_title);
    let Some(message) = conversation
        .messages
        .iter_mut()
        .rev()
        .find(|message| {
            summary_context_message_kind(message)
                .map(is_summary_context_message_kind)
                .unwrap_or(false)
        })
    else {
        return false;
    };
    let Some(message_meta) = ensure_summary_context_message_meta_object_mut(message) else {
        return false;
    };
    let mut changed = false;
    if message_meta.get("schemaVersion").and_then(Value::as_u64)
        != Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION)
    {
        message_meta.insert(
            "schemaVersion".to_string(),
            Value::Number(serde_json::Number::from(
                SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION,
            )),
        );
        changed = true;
    }
    let previous_title = message_meta
        .get("title")
        .and_then(Value::as_str)
        .and_then(normalize_summary_context_title);
    match normalized_title {
        Some(title) => {
            if previous_title.as_deref() != Some(title.as_str()) {
                message_meta.insert("title".to_string(), Value::String(title));
                changed = true;
            }
        }
        None => {
            if message_meta.remove("title").is_some() {
                changed = true;
            }
        }
    }
    changed
}

fn conversation_latest_summary_title(conversation: &Conversation) -> Option<String> {
    conversation
        .messages
        .iter()
        .rev()
        .find_map(summary_context_message_title)
}

fn cleanup_legacy_summary_context_messages(conversation: &mut Conversation) -> bool {
    let mut changed = false;
    for message in conversation.messages.iter_mut() {
        let Some(kind) = summary_context_message_kind(message) else {
            continue;
        };
        if !is_summary_context_message_kind(kind) {
            continue;
        }
        let Some(message_meta) = ensure_summary_context_message_meta_object_mut(message) else {
            continue;
        };
        let schema_backfilled = message_meta.get("schemaVersion").and_then(Value::as_u64)
            != Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION)
        ;
        if schema_backfilled {
            message_meta.insert(
                "schemaVersion".to_string(),
                Value::Number(serde_json::Number::from(
                    SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION,
                )),
            );
            changed = true;
        }
        if schema_backfilled {
            message_meta.insert("title".to_string(), Value::String(String::new()));
            changed = true;
        } else if !message_meta.contains_key("title") {
            message_meta.insert("title".to_string(), Value::String(String::new()));
            changed = true;
        }
    }
    changed
}

fn conversation_real_user_messages<'a>(conversation: &'a Conversation) -> Vec<&'a ChatMessage> {
    conversation
        .messages
        .iter()
        .filter(|message| {
            message.role.trim().eq_ignore_ascii_case("user")
                && !is_context_compaction_message(message, "user")
                && message
                    .speaker_agent_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    != Some(SYSTEM_PERSONA_ID)
        })
        .collect::<Vec<_>>()
}

fn conversation_real_user_message_texts(conversation: &Conversation) -> Vec<String> {
    conversation_real_user_messages(conversation)
        .into_iter()
        .map(render_message_content_for_model)
        .map(|text| clean_text(text.trim()))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
}

#[cfg(test)]
mod summary_context_title_tests {
    use super::*;

    fn test_chat_message(
        id: &str,
        role: &str,
        speaker_agent_id: Option<&str>,
        text: &str,
        provider_meta: Option<Value>,
    ) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            role: role.to_string(),
            created_at: "2026-05-06T10:00:00Z".to_string(),
            speaker_agent_id: speaker_agent_id.map(ToOwned::to_owned),
            parts: vec![MessagePart::Text {
                text: text.to_string(),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta,
            tool_call: None,
            mcp_call: None,
        meme_annotations: None,
        }
    }

    fn test_summary_meta(kind: &str, title: Option<&str>, schema_version: Option<u64>) -> Value {
        let mut message_meta = serde_json::Map::new();
        message_meta.insert("kind".to_string(), Value::String(kind.to_string()));
        message_meta.insert("scene".to_string(), Value::String("test".to_string()));
        if let Some(title) = title {
            message_meta.insert("title".to_string(), Value::String(title.to_string()));
        }
        if let Some(schema_version) = schema_version {
            message_meta.insert(
                "schemaVersion".to_string(),
                Value::Number(serde_json::Number::from(schema_version)),
            );
        }
        Value::Object(serde_json::Map::from_iter([(
            "message_meta".to_string(),
            Value::Object(message_meta),
        )]))
    }

    fn test_conversation(messages: Vec<ChatMessage>) -> Conversation {
        Conversation {
            id: "conversation-a".to_string(),
            title: String::new(),
            agent_id: "agent-a".to_string(),
            department_id: "dept-a".to_string(),
            bound_conversation_id: None,
            parent_conversation_id: None,
            child_conversation_ids: Vec::new(),
            fork_message_cursor: None,
            unread_count: 0,
            conversation_kind: CONVERSATION_KIND_CHAT.to_string(),
            root_conversation_id: None,
            delegate_id: None,
            created_at: "2026-05-06T10:00:00Z".to_string(),
            updated_at: "2026-05-06T10:00:00Z".to_string(),
            last_user_at: None,
            last_assistant_at: None,
            status: "active".to_string(),
            summary: String::new(),
            user_profile_snapshot: String::new(),
            shell_workspace_path: None,
            shell_workspaces: Vec::new(),
            shell_autonomous_mode: false,
            archived_at: None,
            messages,
            current_todos: Vec::new(),
            memory_recall_table: Vec::new(),
            plan_mode_enabled: false,
            preferred_api_config_id: None,
            auto_push_remote_contact_id: None,
            active_goal: None,
            cumulative_usage: ConversationCumulativeUsage::default(),
        }
    }

    #[test]
    fn cleanup_legacy_summary_context_messages_should_backfill_legacy_message_meta() {
        let mut conversation = test_conversation(vec![
            test_chat_message("u1", "user", Some(USER_PERSONA_ID), "正常消息", None),
            test_chat_message(
                "legacy",
                "user",
                Some(SYSTEM_PERSONA_ID),
                "旧压缩",
                Some(test_summary_meta("context_compaction", Some("旧标题"), None)),
            ),
            test_chat_message(
                "seed",
                "user",
                Some(SYSTEM_PERSONA_ID),
                "新摘要",
                Some(test_summary_meta(
                    "summary_context_seed",
                    Some("新标题"),
                    Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION),
                )),
            ),
            test_chat_message("a1", "assistant", Some("agent-a"), "回复", None),
        ]);

        assert!(cleanup_legacy_summary_context_messages(&mut conversation));
        let remaining_ids = conversation
            .messages
            .iter()
            .map(|message| message.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(remaining_ids, vec!["u1", "legacy", "seed", "a1"]);
        let legacy_meta = conversation
            .messages
            .iter()
            .find(|message| message.id == "legacy")
            .and_then(|message| message.provider_meta.as_ref())
            .and_then(|meta| meta.get("message_meta"))
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            legacy_meta.get("schemaVersion").and_then(Value::as_u64),
            Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION)
        );
        assert_eq!(
            legacy_meta.get("title").and_then(Value::as_str),
            Some("")
        );
        assert_eq!(
            conversation_latest_summary_title(&conversation).as_deref(),
            Some("新标题")
        );
    }

    #[test]
    fn conversation_real_user_message_texts_should_skip_summary_context_and_system_user_messages() {
        let conversation = test_conversation(vec![
            test_chat_message(
                "seed",
                "user",
                Some(SYSTEM_PERSONA_ID),
                "不要计入",
                Some(test_summary_meta(
                    "summary_context_seed",
                    Some("摘要标题"),
                    Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION),
                )),
            ),
            test_chat_message("u1", "user", Some(USER_PERSONA_ID), "第一问", None),
            test_chat_message("sys", "user", Some(SYSTEM_PERSONA_ID), "伪造系统用户", None),
            test_chat_message(
                "compact",
                "user",
                Some(SYSTEM_PERSONA_ID),
                "不要计入二",
                Some(test_summary_meta(
                    "context_compaction",
                    Some("压缩标题"),
                    Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION),
                )),
            ),
            test_chat_message("u2", "user", Some(USER_PERSONA_ID), "第二问", None),
            test_chat_message("a1", "assistant", Some("agent-a"), "回复", None),
        ]);

        assert_eq!(
            conversation_real_user_message_texts(&conversation),
            vec!["第一问".to_string(), "第二问".to_string()]
        );
    }

    #[test]
    fn conversation_update_latest_summary_title_should_update_latest_summary_message() {
        let mut conversation = test_conversation(vec![
            test_chat_message(
                "seed",
                "user",
                Some(SYSTEM_PERSONA_ID),
                "摘要",
                Some(test_summary_meta(
                    "summary_context_seed",
                    Some("旧标题"),
                    Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION),
                )),
            ),
            test_chat_message(
                "compact",
                "user",
                Some(SYSTEM_PERSONA_ID),
                "压缩",
                Some(test_summary_meta(
                    "context_compaction",
                    None,
                    Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION),
                )),
            ),
        ]);

        assert!(conversation_update_latest_summary_title(
            &mut conversation,
            Some(" “新的标题。” "),
        ));
        assert_eq!(
            conversation_latest_summary_title(&conversation).as_deref(),
            Some("新的标题")
        );
        let latest_meta = conversation
            .messages
            .last()
            .and_then(|message| message.provider_meta.as_ref())
            .and_then(|meta| meta.get("message_meta"))
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            latest_meta
                .get("schemaVersion")
                .and_then(Value::as_u64),
            Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION)
        );
        assert_eq!(
            latest_meta
                .get("title")
                .and_then(Value::as_str),
            Some("新的标题")
        );
    }

}

fn sanitize_tool_history_events(events: &[Value]) -> Vec<Value> {
    fn assistant_tool_call_ids(event: &Value) -> Vec<String> {
        event
            .get("tool_calls")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .flat_map(|item| {
                        ["id", "call_id"]
                            .into_iter()
                            .filter_map(|key| item.get(key).and_then(Value::as_str))
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(ToOwned::to_owned)
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    fn assistant_with_matched_tool_calls(event: &Value, matched_ids: &[String]) -> Value {
        let mut next = event.clone();
        let Some(object) = next.as_object_mut() else {
            return next;
        };
        let filtered = event
            .get("tool_calls")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter(|item| {
                        ["id", "call_id"].into_iter().any(|key| {
                            item.get(key)
                                .and_then(Value::as_str)
                                .map(str::trim)
                                .is_some_and(|id| matched_ids.iter().any(|matched| matched == id))
                        })
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        object.insert("tool_calls".to_string(), Value::Array(filtered));
        next
    }

    #[derive(Debug, Clone)]
    struct PendingAssistant {
        event: Value,
        allowed_ids: Vec<String>,
        matched_ids: Vec<String>,
        output_index: Option<usize>,
        legacy_without_ids: bool,
    }

    let mut sanitized = Vec::<Value>::new();
    let mut pending_assistant: Option<PendingAssistant> = None;
    for event in events {
        let role = event
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        match role.as_str() {
            "assistant" => {
                let has_tool_calls = event
                    .get("tool_calls")
                    .and_then(Value::as_array)
                    .map(|items| !items.is_empty())
                    .unwrap_or(false);
                let tool_call_ids = assistant_tool_call_ids(event);
                pending_assistant = if has_tool_calls {
                    Some(PendingAssistant {
                        event: event.clone(),
                        legacy_without_ids: tool_call_ids.is_empty(),
                        allowed_ids: tool_call_ids,
                        matched_ids: Vec::new(),
                        output_index: None,
                    })
                } else {
                    sanitized.push(event.clone());
                    None
                };
            }
            "tool" => {
                if let Some(pending) = pending_assistant.as_mut() {
                    let tool_call_id = event
                        .get("tool_call_id")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .unwrap_or_default();
                    let matched_index = pending
                        .allowed_ids
                        .iter()
                        .position(|id| id == tool_call_id);
                    let legacy_without_ids =
                        pending.legacy_without_ids && pending.output_index.is_none();
                    if legacy_without_ids || matched_index.is_some() {
                        if !pending.matched_ids.iter().any(|id| id == tool_call_id) {
                            pending.matched_ids.push(tool_call_id.to_string());
                        }
                        let assistant_event = if pending.legacy_without_ids {
                            pending.event.clone()
                        } else {
                            assistant_with_matched_tool_calls(&pending.event, &pending.matched_ids)
                        };
                        if let Some(index) = pending.output_index {
                            sanitized[index] = assistant_event;
                        } else {
                            pending.output_index = Some(sanitized.len());
                            sanitized.push(assistant_event);
                        }
                        sanitized.push(event.clone());
                        if let Some(index) = matched_index {
                            pending.allowed_ids.remove(index);
                            if pending.allowed_ids.is_empty() {
                                pending_assistant = None;
                            }
                        } else {
                            pending_assistant = None;
                        }
                    }
                }
            }
            _ => {
                pending_assistant = None;
                sanitized.push(event.clone());
            }
        }
    }
    sanitized
}

fn build_conversation_record(
    _api_config_id: &str,
    agent_id: &str,
    department_id: &str,
    title: &str,
    conversation_kind: &str,
    root_conversation_id: Option<String>,
    delegate_id: Option<String>,
) -> Conversation {
    let now = now_iso();
    Conversation {
        id: Uuid::new_v4().to_string(),
        title: title.trim().to_string(),
        agent_id: agent_id.to_string(),
        department_id: department_id.trim().to_string(),
        bound_conversation_id: None,
        parent_conversation_id: None,
        child_conversation_ids: Vec::new(),
        fork_message_cursor: None,
        unread_count: 0,
        conversation_kind: conversation_kind.trim().to_string(),
        root_conversation_id,
        delegate_id,
        created_at: now.clone(),
        updated_at: now,
        last_user_at: None,
        last_assistant_at: None,
        status: "active".to_string(),
        summary: String::new(),
        user_profile_snapshot: String::new(),
        shell_workspace_path: None,
        shell_workspaces: Vec::new(),
        shell_autonomous_mode: false,
        archived_at: None,
        messages: Vec::new(),
        current_todos: Vec::new(),
        memory_recall_table: Vec::new(),
        plan_mode_enabled: false,
        preferred_api_config_id: None,
        auto_push_remote_contact_id: None,
        cumulative_usage: ConversationCumulativeUsage::default(),
        active_goal: None,
    }
}

fn build_system_notification_conversation_record() -> Conversation {
    let mut conversation = build_conversation_record(
        "",
        DEFAULT_AGENT_ID,
        ASSISTANT_DEPARTMENT_ID,
        &system_notification_conversation_title(),
        CONVERSATION_KIND_SYSTEM_NOTIFICATION,
        None,
        None,
    );
    conversation.id = SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string();
    conversation.messages = Vec::new();
    conversation.last_user_at = None;
    conversation.last_assistant_at = None;
    conversation
}

#[cfg(test)]
fn ensure_active_conversation_index(
    data: &mut AppData,
    api_config_id: &str,
    agent_id: &str,
) -> usize {
    let _ = normalize_main_conversation_marker(data, agent_id);
    let _ = normalize_single_active_main_conversation(data);
    if let Some(idx) = latest_active_conversation_index(data, api_config_id, agent_id) {
        return idx;
    }

    if let Some(idx) = latest_main_conversation_index(data, agent_id) {
        for (_i, conversation) in data.conversations.iter_mut().enumerate() {
            if !conversation_visible_in_foreground_lists(conversation) || !conversation.summary.trim().is_empty() {
                continue;
            }
            conversation.status = "active".to_string();
        }
        return idx;
    }

    let conversation = build_system_notification_conversation_record();

    for item in &mut data.conversations {
        if !conversation_visible_in_foreground_lists(item) || !item.summary.trim().is_empty() {
            continue;
        }
        item.status = "active".to_string();
    }
    data.conversations.push(conversation);
    if data
        .main_conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        data.main_conversation_id = Some(SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string());
    }
    data.conversations.len() - 1
}

#[cfg(test)]
fn ensure_main_conversation_index(
    data: &mut AppData,
    _api_config_id: &str,
    agent_id: &str,
) -> usize {
    let _ = normalize_main_conversation_marker(data, agent_id);
    if let Some(idx) = main_conversation_index(data, agent_id) {
        return idx;
    }
    let conversation = build_system_notification_conversation_record();
    for item in &mut data.conversations {
        if !conversation_visible_in_foreground_lists(item) || !item.summary.trim().is_empty() {
            continue;
        }
        item.status = "active".to_string();
    }
    data.conversations.push(conversation);
    data.main_conversation_id = Some(SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string());
    data.conversations.len() - 1
}

fn ensure_active_foreground_conversation_index_atomic(
    data: &mut AppData,
    _data_path: &PathBuf,
    _api_config_id: &str,
    agent_id: &str,
) -> usize {
    let _ = normalize_main_conversation_marker(data, agent_id);
    let _ = normalize_single_active_main_conversation(data);
    if let Some(idx) = main_conversation_index(data, agent_id) {
        for conversation in &mut data.conversations {
            if !conversation_visible_in_foreground_lists(conversation)
                || !conversation.summary.trim().is_empty()
            {
                continue;
            }
            conversation.status = "active".to_string();
        }
        return idx;
    }

    let conversation = build_system_notification_conversation_record();
    for item in &mut data.conversations {
        if !conversation_visible_in_foreground_lists(item) || !item.summary.trim().is_empty() {
            continue;
        }
        item.status = "active".to_string();
    }
    data.conversations.push(conversation);
    if data
        .main_conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        data.main_conversation_id = Some(SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string());
    }
    data.conversations.len() - 1
}

fn active_foreground_conversation_index_read_only(
    data: &AppData,
    agent_id: &str,
) -> Option<usize> {
    main_conversation_index(data, agent_id)
        .or_else(|| latest_active_conversation_index(data, "", agent_id))
        .or_else(|| latest_main_conversation_index(data, agent_id))
}

#[derive(Debug, Clone)]
struct ArchiveDecision {
    should_archive: bool,
    forced: bool,
    reason: String,
    usage_ratio: f64,
}

fn estimated_tokens_for_text(text: &str) -> f64 {
    static TOKEN_BPE: std::sync::OnceLock<Option<tiktoken_rs::CoreBPE>> = std::sync::OnceLock::new();
    if let Some(bpe) = TOKEN_BPE
        .get_or_init(|| tiktoken_rs::cl100k_base().ok())
        .as_ref()
    {
        return bpe.encode_with_special_tokens(text).len() as f64;
    }

    // 极端情况下 tokenizer 初始化失败，回退到旧启发式，避免中断主流程。
    let mut zh_chars = 0usize;
    let mut other_chars = 0usize;
    for ch in text.chars() {
        if ch.is_whitespace() {
            continue;
        }
        if ('\u{4e00}'..='\u{9fff}').contains(&ch)
            || ('\u{3400}'..='\u{4dbf}').contains(&ch)
            || ('\u{f900}'..='\u{faff}').contains(&ch)
        {
            zh_chars += 1;
        } else {
            other_chars += 1;
        }
    }
    zh_chars as f64 * 0.6 + other_chars as f64 * 0.3
}

fn build_archive_decision_from_usage_ratio(
    usage_ratio: f64,
    _last_user_at: Option<&str>,
    has_assistant_reply: bool,
) -> ArchiveDecision {
    if !has_assistant_reply {
        return ArchiveDecision {
            should_archive: false,
            forced: false,
            reason: "no_assistant_reply".to_string(),
            usage_ratio,
        };
    }
    if usage_ratio >= 0.82 {
        return ArchiveDecision {
            should_archive: true,
            forced: true,
            reason: "force_context_usage_82".to_string(),
            usage_ratio,
        };
    }

    ArchiveDecision {
        should_archive: false,
        forced: false,
        reason: "context_usage_below_force_threshold".to_string(),
        usage_ratio,
    }
}

fn build_archive_decision_from_estimated_usage_ratio(
    usage_ratio: f64,
    _last_user_at: Option<&str>,
    has_assistant_reply: bool,
) -> ArchiveDecision {
    if !has_assistant_reply {
        return ArchiveDecision {
            should_archive: false,
            forced: false,
            reason: "no_assistant_reply".to_string(),
            usage_ratio,
        };
    }
    if usage_ratio >= 0.95 {
        return ArchiveDecision {
            should_archive: true,
            forced: true,
            reason: "force_estimated_context_usage_95".to_string(),
            usage_ratio,
        };
    }

    ArchiveDecision {
        should_archive: false,
        forced: false,
        reason: "estimated_context_usage_below_force_threshold".to_string(),
        usage_ratio,
    }
}

#[cfg(test)]
fn decide_archive_before_model_request(
    estimated_prompt_tokens: u64,
    context_window_tokens: u32,
    last_user_at: Option<&str>,
    has_assistant_reply: bool,
) -> ArchiveDecision {
    let max_tokens = context_window_tokens.max(1) as f64;
    let usage_ratio = (estimated_prompt_tokens as f64 / max_tokens).max(0.0);
    build_archive_decision_from_usage_ratio(usage_ratio, last_user_at, has_assistant_reply)
}

#[cfg(test)]
fn decide_archive_before_send_with_fallback(
    cached_effective_prompt_tokens: u64,
    cached_usage_ratio: f64,
    estimated_prompt_tokens: Option<u64>,
    context_window_tokens: u32,
    last_user_at: Option<&str>,
    has_assistant_reply: bool,
) -> (ArchiveDecision, &'static str) {
    if cached_effective_prompt_tokens > 0 {
        return (
            decide_archive_before_model_request(
                cached_effective_prompt_tokens,
                context_window_tokens,
                last_user_at,
                has_assistant_reply,
            ),
            "cached_effective_prompt_tokens",
        );
    }
    if cached_usage_ratio.is_finite() && cached_usage_ratio > 0.0 {
        return (
            build_archive_decision_from_usage_ratio(
                cached_usage_ratio.max(0.0),
                last_user_at,
                has_assistant_reply,
            ),
            "cached_usage_ratio",
        );
    }
    (
        build_archive_decision_from_estimated_usage_ratio(
            (estimated_prompt_tokens.unwrap_or(0) as f64
                / context_window_tokens.max(1) as f64)
                .max(0.0),
            last_user_at,
            has_assistant_reply,
        ),
        "estimated_prompt_tokens",
    )
}

fn decide_archive_before_send_from_usage(
    usage: &PromptUsageResolution,
    last_user_at: Option<&str>,
    has_assistant_reply: bool,
) -> (ArchiveDecision, &'static str) {
    let decision = match usage.source {
        "cached_effective_prompt_tokens"
        | "cached_usage_ratio"
        | "trusted_prompt_usage"
        | "assistant_message_effective_prompt_tokens"
        | "assistant_message_context_usage_ratio" => {
            build_archive_decision_from_usage_ratio(
                usage.usage_ratio,
                last_user_at,
                has_assistant_reply,
            )
        }
        _ => build_archive_decision_from_estimated_usage_ratio(
            usage.usage_ratio,
            last_user_at,
            has_assistant_reply,
        ),
    };
    (decision, usage.source)
}

#[cfg(test)]
fn archive_conversation_now(
    data: &mut AppData,
    conversation_id: &str,
    reason: &str,
    summary: &str,
) -> Option<String> {
    let idx = data
        .conversations
        .iter()
        .position(|c| c.id == conversation_id && c.summary.trim().is_empty())?;
    let conv = data.conversations.get_mut(idx)?;
    let previous_status = conv.status.clone();
    let now = now_iso();
    conv.status = "archived".to_string();
    conv.summary = summary.to_string();
    conv.archived_at = Some(now.clone());
    conv.updated_at = now;
    let archive_id = conv.id.clone();
    eprintln!(
        "[会话] 已归档: conversation_id={}, previous_status={}, reason=\"{}\", summary=\"{}\"",
        conv.id,
        previous_status,
        reason,
        summary
    );
    clear_screenshot_artifact_cache();
    Some(archive_id)
}

fn normalize_image_for_chat_upload(bytes: &[u8]) -> Result<LlmRequestNormalizedImage, String> {
    normalize_image_bytes_for_llm_request(bytes, None)
}

fn is_supported_image_upload_mime(mime: &str) -> bool {
    llm_request_image_supported_raster_mime(mime)
}

fn build_llm_image_input_fallback_notice(
    label: &str,
    reason: &str,
    saved_path: Option<&str>,
) -> String {
    let trimmed_reason = reason.trim();
    let trimmed_path = saved_path
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(path) = trimmed_path {
        return build_attachment_notice_text(label, path);
    }
    format!(
        "[系统提示] {label} 未能作为图片输入提供给模型，原因：{trimmed_reason}。\n请按普通附件处理该文件，必要时改用 shell 或 read 读取。"
    )
}

fn build_user_parts(
    payload: &ChatInputPayload,
    api_config: &ApiConfig,
) -> Result<Vec<MessagePart>, String> {
    let mut parts = Vec::<MessagePart>::new();
    let mut total_binary = 0usize;

    if let Some(text) = payload
        .text
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        if !api_config.enable_text {
            return Err("Current API config has text disabled.".to_string());
        }
        parts.push(MessagePart::Text {
            text: text.to_string(),
                reasoning_content: None,
            });
    }

    if let Some(images) = &payload.images {
        if !images.is_empty() && !api_config.enable_image {
            return Err("Current API config has image disabled.".to_string());
        }

        for (index, image) in images.iter().enumerate() {
            let mime = image.mime.trim().to_ascii_lowercase();
            if !is_supported_image_upload_mime(&mime) {
                parts.push(MessagePart::Text {
                    text: build_llm_image_input_fallback_notice(
                        &format!("第 {} 张图片", index + 1),
                        &format!("不支持的图片 MIME：{}", image.mime.trim()),
                        image.saved_path.as_deref(),
                    ),
                    reasoning_content: None,
                });
                continue;
            }
            let bytes_base64 = image.bytes_base64.trim();
            let raw = match B64.decode(bytes_base64) {
                Ok(value) => value,
                Err(err) => {
                    parts.push(MessagePart::Text {
                        text: build_llm_image_input_fallback_notice(
                            &format!("第 {} 张图片", index + 1),
                            &format!("图片内容解码失败：{err}"),
                            image.saved_path.as_deref(),
                        ),
                        reasoning_content: None,
                    });
                    continue;
                }
            };
            let normalized = match normalize_image_for_chat_upload(&raw) {
                Ok(value) => value,
                Err(err) => {
                    parts.push(MessagePart::Text {
                        text: build_llm_image_input_fallback_notice(
                            &format!("第 {} 张图片", index + 1),
                            &err,
                            image.saved_path.as_deref(),
                        ),
                        reasoning_content: None,
                    });
                    continue;
                }
            };
            total_binary += normalized.bytes.len();
            parts.push(MessagePart::Image {
                mime: normalized.mime,
                bytes_base64: B64.encode(normalized.bytes),
                name: None,
                compressed: !normalized.reused_original,
            });
        }
    }

    if let Some(audios) = &payload.audios {
        if !audios.is_empty() && !api_config.enable_audio {
            return Err("Current API config has audio disabled.".to_string());
        }

        for audio in audios {
            let bytes_base64 = audio.bytes_base64.trim();
            let raw = B64
                .decode(bytes_base64)
                .map_err(|err| format!("Decode audio base64 failed: {err}"))?;
            total_binary += raw.len();
            parts.push(MessagePart::Audio {
                mime: audio.mime.trim().to_string(),
                bytes_base64: bytes_base64.to_string(),
                name: None,
                compressed: false,
            });
        }
    }

    if total_binary > MAX_MULTIMODAL_BYTES {
        return Err(format!(
            "Multimodal payload exceeds 10MB limit ({} bytes).",
            total_binary
        ));
    }

    if parts.is_empty() {
        return Err("Request payload is empty. Provide text, image, or audio.".to_string());
    }

    Ok(parts)
}

fn build_prepared_binary_payloads_from_message_parts(
    parts: &[MessagePart],
    image_saved_paths: &[Option<String>],
    audio_saved_paths: &[Option<String>],
) -> (Vec<PreparedBinaryPayload>, Vec<PreparedBinaryPayload>) {
    let images = parts
        .iter()
        .filter_map(|part| match part {
            MessagePart::Image {
                mime, bytes_base64, ..
            } => Some((mime.clone(), bytes_base64.clone())),
            _ => None,
        })
        .enumerate()
        .map(|(index, (mime, bytes_base64))| PreparedBinaryPayload {
            mime,
            content: bytes_base64,
            saved_path: image_saved_paths.get(index).cloned().flatten(),
        })
        .collect::<Vec<_>>();
    let audios = parts
        .iter()
        .filter_map(|part| match part {
            MessagePart::Audio {
                mime, bytes_base64, ..
            } => Some((mime.clone(), bytes_base64.clone())),
            _ => None,
        })
        .enumerate()
        .map(|(index, (mime, bytes_base64))| PreparedBinaryPayload {
            mime,
            content: bytes_base64,
            saved_path: audio_saved_paths.get(index).cloned().flatten(),
        })
        .collect::<Vec<_>>();
    (images, audios)
}

fn build_effective_prompt_media_from_prepared(
    payload: &ChatInputPayload,
    api_config: &ApiConfig,
    prepared_images: &[PreparedBinaryPayload],
    prepared_audios: &[PreparedBinaryPayload],
) -> Result<(String, Vec<PreparedBinaryPayload>, Vec<PreparedBinaryPayload>), String> {
    let mut chunks = Vec::<String>::new();

    if let Some(text) = payload
        .text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !api_config.enable_text {
            return Err("Current API config has text disabled.".to_string());
        }
        chunks.push(text.to_string());
    }

    let mut images = Vec::<PreparedBinaryPayload>::new();
    if let Some(requested_images) = &payload.images {
        if !requested_images.is_empty() && !api_config.enable_image {
            return Err("Current API config has image disabled.".to_string());
        }
        if requested_images.len() > prepared_images.len() {
            return Err("Prepared image payload count is smaller than effective payload.".to_string());
        }
        for (index, requested) in requested_images.iter().enumerate() {
            let mut prepared = prepared_images
                .get(index)
                .cloned()
                .ok_or_else(|| "Prepared image payload is missing.".to_string())?;
            if prepared.saved_path.is_none() {
                prepared.saved_path = requested.saved_path.clone();
            }
            if prepared.mime.trim().eq_ignore_ascii_case("application/pdf") {
                chunks.push("[pdf]".to_string());
            } else {
                chunks.push("[image]".to_string());
            }
            images.push(prepared);
        }
    }

    let mut audios = Vec::<PreparedBinaryPayload>::new();
    if let Some(requested_audios) = &payload.audios {
        if !requested_audios.is_empty() && !api_config.enable_audio {
            return Err("Current API config has audio disabled.".to_string());
        }
        if requested_audios.len() > prepared_audios.len() {
            return Err("Prepared audio payload count is smaller than effective payload.".to_string());
        }
        for (index, requested) in requested_audios.iter().enumerate() {
            let mut prepared = prepared_audios
                .get(index)
                .cloned()
                .ok_or_else(|| "Prepared audio payload is missing.".to_string())?;
            if prepared.saved_path.is_none() {
                prepared.saved_path = requested.saved_path.clone();
            }
            chunks.push("[audio]".to_string());
            audios.push(prepared);
        }
    }

    if chunks.is_empty() {
        return Err("Request payload is empty. Provide text, image, or audio.".to_string());
    }

    Ok((chunks.join("\n"), images, audios))
}

fn render_message_content_for_model(message: &ChatMessage) -> String {
    let mut chunks = Vec::<String>::new();
    for part in &message.parts {
        match part {
            MessagePart::Text { text, .. } => chunks.push(text.clone()),
            MessagePart::Image { mime, .. } => {
                if mime.trim().eq_ignore_ascii_case("application/pdf") {
                    chunks.push("[pdf attached]".to_string());
                } else {
                    chunks.push("[image attached]".to_string());
                }
            }
            MessagePart::Audio { .. } => chunks.push("[audio attached]".to_string()),
        }
    }
    if let Some(meta) = &message.provider_meta {
        if let Some(hidden_prompt_text) = meta
            .get("hiddenPromptText")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            chunks.push(hidden_prompt_text.to_string());
        }
        if let Some(task_trigger) = meta
            .get("taskTrigger")
            .and_then(Value::as_object)
            .filter(|_| {
                meta.get("messageKind")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    == Some("task_trigger")
            })
        {
            let mut lines = Vec::<String>::new();
            if let Some(task_id) = task_trigger
                .get("taskId")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                lines.push(format!("taskId: {}", task_id));
            }
            if let Some(next_run_at_local) = task_trigger
                .get("next_run_at")
                .or_else(|| task_trigger.get("nextRunAt"))
                .or_else(|| task_trigger.get("nextRunAtLocal"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                lines.push(format!("next_run_at: {}", next_run_at_local));
            }
            if let Some(run_at_local) = task_trigger
                .get("run_at")
                .or_else(|| task_trigger.get("runAt"))
                .or_else(|| task_trigger.get("runAtLocal"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                lines.push(format!("run_at: {}", run_at_local));
            }
            if let Some(cron_expression) = task_trigger
                .get("cron_expression")
                .or_else(|| task_trigger.get("cronExpression"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                lines.push(format!("cron_expression: {}", cron_expression));
            }
            if let Some(end_at_local) = task_trigger
                .get("end_at")
                .or_else(|| task_trigger.get("endAt"))
                .or_else(|| task_trigger.get("endAtLocal"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                lines.push(format!("end_at: {}", end_at_local));
            }
            if !lines.is_empty() {
                chunks.push(lines.join("\n"));
            }
        }
        if let Some(attachments) = meta.get("attachments").and_then(Value::as_array) {
            for item in attachments {
                let relative_path = item
                    .get("relativePath")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .unwrap_or("");
                if relative_path.is_empty() {
                    continue;
                }
                chunks.push(build_attachment_notice_text("", relative_path));
            }
        }
    }
    chunks.join(" | ")
}

fn sanitize_memory_block_xml(raw: &str) -> String {
    if !raw.contains("<memory_board")
        && !raw.contains("[MemoryBoard]")
        && !raw.contains("<memory_context>")
    {
        return raw.to_string();
    }
    raw.lines()
        .filter(|line| {
            let t = line.trim();
            !(t.contains("<keywords>")
                || t.contains("</keywords>")
                || t.contains("<reason>")
                || t.contains("</reason>"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn xml_escape_prompt(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn prompt_role_for_message(message: &ChatMessage, current_agent_id: &str) -> Option<String> {
    let raw_role = message.role.trim().to_lowercase();
    let speaker_id = message
        .speaker_agent_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("");
    let message_kind = message
        .provider_meta
        .as_ref()
        .and_then(|meta| meta.get("messageKind"))
        .and_then(Value::as_str)
        .map(str::trim);
    if raw_role == "system"
        && speaker_id == SYSTEM_PERSONA_ID
        && (message_is_goal_continue(message) || message_kind == Some("task_trigger"))
    {
        return Some("user".to_string());
    }
    if raw_role != "user" && raw_role != "assistant" {
        return None;
    }
    if !speaker_id.is_empty() && speaker_id == current_agent_id {
        return Some("assistant".to_string());
    }
    Some("user".to_string())
}

fn prompt_speaker_label(
    message: &ChatMessage,
    agents: &[AgentProfile],
    user_name: &str,
) -> String {
    // 优先检查远程 IM 来源
    if let Some(meta) = &message.provider_meta {
        if let Some(origin) = meta.get("origin") {
            if origin.get("kind").and_then(|v| v.as_str()) == Some("remote_im") {
                let sender = remote_im_origin_string(origin, "sender_name").unwrap_or("");
                let contact = remote_im_origin_string(origin, "contact_name").unwrap_or("");
                let contact_type = remote_im_origin_string(origin, "contact_type").unwrap_or("");
                if contact_type == "group" && !contact.is_empty() && !sender.is_empty() {
                    return format!("{} ({})", sender, contact);
                }
                if !sender.is_empty() {
                    return sender.to_string();
                }
                if !contact.is_empty() {
                    return contact.to_string();
                }
            }
        }
    }

    let speaker_id = message
        .speaker_agent_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("");
    if speaker_id.is_empty() {
        return user_name.trim().to_string();
    }
    if speaker_id == USER_PERSONA_ID {
        let label = user_name.trim();
        if !label.is_empty() {
            return label.to_string();
        }
    }
    agents
        .iter()
        .find(|profile| profile.id == speaker_id)
        .map(|profile| profile.name.trim().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| speaker_id.to_string())
}

fn build_prompt_speaker_block(
    message: &ChatMessage,
    agents: &[AgentProfile],
    user_name: &str,
    _ui_language: &str,
) -> String {
    let speaker_name = prompt_speaker_label(message, agents, user_name);
    if speaker_name.trim().is_empty() {
        return String::new();
    }
    format!("[{}]", speaker_name)
}

fn build_prompt_user_meta_text(
    message: &ChatMessage,
    agents: &[AgentProfile],
    user_name: &str,
    ui_language: &str,
    include_remote_identity: bool,
) -> Option<String> {
    if is_context_compaction_message(message, "user") {
        return None;
    }
    let speaker_block = build_prompt_speaker_block(message, agents, user_name, ui_language);
    let time_text = format_message_time_rfc3339_local_to_minute(&message.created_at);
    let has_speaker = !speaker_block.trim().is_empty();
    let has_time = !time_text.trim().is_empty();
    let mut base = match (has_speaker, has_time) {
        (true, true) => format!("{} {}", speaker_block, time_text),
        (true, false) => speaker_block,
        (false, true) => format!("[{}]", time_text),
        (false, false) => String::new(),
    };
    let mut tags = Vec::<String>::new();
    if let Some(origin) = remote_im_origin_from_message(message) {
        if let Some(user_id) = remote_im_origin_canonical_user_id(origin) {
            tags.push(format!("user_id={}", user_id));
        }
        if include_remote_identity {
            let channel_id = remote_im_origin_string(origin, "channel_id").unwrap_or("");
            let contact_id = remote_im_origin_string(origin, "contact_id").unwrap_or("");
            if !channel_id.is_empty() {
                tags.push(format!("channel_id={}", channel_id));
            }
            if !contact_id.is_empty() {
                tags.push(format!("contact_id={}", contact_id));
            }
        }
    } else if message
        .speaker_agent_id
        .as_deref()
        .map(str::trim)
        == Some(USER_PERSONA_ID)
    {
        tags.push(format!("user_id={}", USER_PERSONA_ID));
    }
    if let Some(memory_ids) = message
        .provider_meta
        .as_ref()
        .and_then(|meta| meta.get("memoryIds"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|items| !items.is_empty())
    {
        tags.push(format!("memory={}", memory_ids.join(",")));
    }
    if !tags.is_empty() {
        if !base.trim().is_empty() {
            base.push_str(" | ");
        }
        base.push_str(&tags.join(" | "));
    }
    if base.trim().is_empty() {
        None
    } else {
        Some(base)
    }
}

fn format_message_time_rfc3339_local_to_minute(raw: &str) -> String {
    let full = format_message_time_rfc3339_local(raw);
    let trimmed = full.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let Some(t_idx) = trimmed.find('T') else {
        return format_message_time_text(raw)
            .chars()
            .take(16)
            .collect::<String>();
    };
    let date = &trimmed[..t_idx];
    let rest = &trimmed[t_idx + 1..];
    let tz_idx = rest
        .find(|ch: char| ch == '+' || ch == '-' || ch == 'Z')
        .unwrap_or(rest.len());
    let time = &rest[..tz_idx];
    let mut segs = time.split(':');
    let hh = segs.next().unwrap_or("");
    let mm = segs.next().unwrap_or("");
    if hh.len() == 2 && mm.len() == 2 {
        return format!("{date}T{hh}:{mm}");
    }
    trimmed.to_string()
}

fn prompt_current_date_timezone_line(_ui_language: &str) -> String {
    let tz = local_utc_offset()
        .map(|offset| {
            let seconds = offset.whole_seconds();
            let sign = if seconds < 0 { '-' } else { '+' };
            let abs = seconds.abs();
            let hours = abs / 3600;
            let minutes = (abs % 3600) / 60;
            format!("{sign}{hours:02}:{minutes:02}")
        })
        .unwrap_or_else(|| "local".to_string());
    format!("- 时区：{}", tz)
}

fn render_prompt_message_text(message: &ChatMessage) -> String {
    // 注意：这里是“通用消息渲染”层，只负责把一条消息已有内容转换成模型可读文本。
    // 不要在这里注入 latest user 专属策略，也不要在这里拼接用户 @ mention 前缀。
    //
    // 原因：
    // 1. 最终请求体分成“history_messages”和“latest_user_text”两块分别组装；
    // 2. 预览与实际发送都依赖这两块的最终组装结果，而不是单靠这里；
    // 3. 若把 @ mention、最新消息特判等逻辑塞进这里，容易出现：
    //    - 预览看不到
    //    - 只有部分消息生效
    //    - 历史消息 / 最新消息行为不一致
    //
    // 正确边界：
    // - 这里：只做通用内容渲染
    // - build_prompt_with_mode(...)：负责 history / latest user 的请求体组装策略
    render_message_content_for_model(message)
}

fn render_prompt_message_reasoning(message: &ChatMessage) -> Option<String> {
    let reasoning = message
        .parts
        .iter()
        .filter_map(|part| match part {
            MessagePart::Text {
                reasoning_content,
                ..
            } => reasoning_content.as_deref(),
            _ => None,
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if reasoning.is_empty() {
        None
    } else {
        Some(reasoning)
    }
}

fn render_prompt_user_text_only(message: &ChatMessage) -> String {
    let mut chunks = Vec::<String>::new();
    for part in &message.parts {
        if let MessagePart::Text { text, .. } = part {
            if !text.trim().is_empty() {
                chunks.push(text.clone());
            }
        }
    }
    chunks.join("\n")
}

fn render_prompt_user_mention_prefix(message: &ChatMessage) -> String {
    message
        .provider_meta
        .as_ref()
        .and_then(|meta| meta.get("message_meta").or_else(|| meta.get("messageMeta")))
        .and_then(Value::as_object)
        .and_then(|message_meta| message_meta.get("mentions"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    item.get("agentName")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(|value| format!("@{}", value))
                })
                .collect::<Vec<_>>()
                .join(",")
        })
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_default()
}

fn remote_im_origin_from_message(message: &ChatMessage) -> Option<&Value> {
    let meta = message.provider_meta.as_ref()?;
    let origin = meta.get("origin")?;
    if origin.get("kind").and_then(Value::as_str) != Some("remote_im") {
        return None;
    }
    Some(origin)
}

fn remote_im_origin_string<'a>(origin: &'a Value, key: &str) -> Option<&'a str> {
    origin
        .get(key)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn remote_im_origin_canonical_user_id<'a>(origin: &'a Value) -> Option<&'a str> {
    remote_im_origin_string(origin, "sender_id")
        .or_else(|| remote_im_origin_string(origin, "contact_id"))
}

fn remote_im_message_canonical_user_id(message: &ChatMessage) -> Option<String> {
    let origin = remote_im_origin_from_message(message)?;
    remote_im_origin_canonical_user_id(origin).map(ToOwned::to_owned)
}

fn remote_im_contact_key_from_message(message: &ChatMessage) -> Option<String> {
    let origin = remote_im_origin_from_message(message)?;
    let channel_id = remote_im_origin_string(origin, "channel_id").unwrap_or("");
    let contact_id = remote_im_origin_string(origin, "contact_id").unwrap_or("");
    if channel_id.is_empty() || contact_id.is_empty() {
        return None;
    }
    Some(format!("{}::{}", channel_id, contact_id))
}

fn prompt_retrieved_memory_ids_from_message(message: &ChatMessage) -> Vec<String> {
    let Some(meta) = message.provider_meta.as_ref() else {
        return Vec::new();
    };
    let Some(ids) = meta
        .get("retrieved_memory_ids")
        .or_else(|| meta.get("recallMemoryIds"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    let mut seen = HashSet::<String>::new();
    ids.iter()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .filter_map(|value| {
            let owned = value.to_string();
            if seen.insert(owned.clone()) {
                Some(owned)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}

fn collect_prompt_retrieved_memory_ids(messages: &[ChatMessage]) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    let mut collected = Vec::<String>::new();
    for message in messages {
        for memory_id in prompt_retrieved_memory_ids_from_message(message) {
            if seen.insert(memory_id.clone()) {
                collected.push(memory_id);
            }
        }
    }
    collected
}

fn prompt_recall_memory_block_for_message(
    message: &ChatMessage,
    recall_memories: Option<&[MemoryEntry]>,
    seen_memory_ids: &mut HashSet<String>,
) -> Option<String> {
    let Some(memories) = recall_memories else {
        return None;
    };
    let retrieved_ids = prompt_retrieved_memory_ids_from_message(message);
    if retrieved_ids.is_empty() {
        return None;
    }
    let inject_ids = retrieved_ids
        .into_iter()
        .filter(|memory_id| seen_memory_ids.insert(memory_id.clone()))
        .collect::<Vec<_>>();
    build_memory_board_xml_from_recall_ids(memories, &inject_ids)
}

fn prompt_user_extra_blocks_for_message(
    _state: Option<&AppState>,
    _conversation: Option<&Conversation>,
    message: &ChatMessage,
    _agents: &[AgentProfile],
    _prompt_user_name: &str,
    _ui_language: &str,
    _include_remote_identity: bool,
    recall_memories: Option<&[MemoryEntry]>,
    seen_memory_ids: &mut HashSet<String>,
    _include_conversation_workspace: bool,
) -> Vec<String> {
    let mut blocks = Vec::<String>::new();
    if let Some(recall_block) =
        prompt_recall_memory_block_for_message(message, recall_memories, seen_memory_ids)
    {
        blocks.push(recall_block);
    }
    for extra in &message.extra_text_blocks {
        if extra.trim().is_empty() {
            continue;
        }
        let trimmed = extra.trim();
        if trimmed.starts_with("[远程IM] 发送者:")
            || trimmed.starts_with("[RemoteIM] sender:")
        {
            continue;
        }
        let extra = sanitize_memory_block_xml(extra);
        if extra.trim().is_empty() {
            continue;
        }
        blocks.push(extra);
    }
    if let Some(meta) = message.provider_meta.as_ref() {
        if let Some(attachments) = meta.get("attachments").and_then(Value::as_array) {
            for item in attachments {
                let relative_path = item
                    .get("relativePath")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .unwrap_or("");
                if relative_path.is_empty() {
                    continue;
                }
                blocks.push(build_attachment_notice_text("", relative_path));
            }
        }
    }
    blocks
}

fn provider_meta_message_kind(message: &ChatMessage) -> Option<String> {
    message
        .provider_meta
        .as_ref()?
        .get("message_meta")
        .or_else(|| message.provider_meta.as_ref()?.get("messageMeta"))
        .and_then(Value::as_object)?
        .get("kind")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn is_context_compaction_message(message: &ChatMessage, role: &str) -> bool {
    if role != "user" {
        return false;
    }
    matches!(
        provider_meta_message_kind(message).as_deref(),
        Some("context_compaction") | Some("summary_context_seed")
    )
}

fn is_tool_review_report_message(message: &ChatMessage) -> bool {
    matches!(
        provider_meta_message_kind(message).as_deref(),
        Some("tool_review_report")
    )
}

fn message_attachment_paths_by_mime(
    message: &ChatMessage,
    prefix: &str,
) -> std::collections::HashMap<String, std::collections::VecDeque<String>> {
    let mut out =
        std::collections::HashMap::<String, std::collections::VecDeque<String>>::new();
    let Some(attachments) = message
        .provider_meta
        .as_ref()
        .and_then(|meta| meta.get("attachments"))
        .and_then(Value::as_array)
    else {
        return out;
    };
    for item in attachments {
        let mime = item
            .get("mime")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("")
            .to_ascii_lowercase();
        if !mime.starts_with(prefix) {
            continue;
        }
        let relative_path = item
            .get("relativePath")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let Some(relative_path) = relative_path else {
            continue;
        };
        out.entry(mime)
            .or_default()
            .push_back(relative_path.replace('\\', "/"));
    }
    out
}

fn resolve_media_from_message(
    message: &ChatMessage,
    data_path: Option<&PathBuf>,
    log_prefix: &str,
) -> (Vec<PreparedBinaryPayload>, Vec<PreparedBinaryPayload>) {
    let mut images = Vec::<PreparedBinaryPayload>::new();
    let mut audios = Vec::<PreparedBinaryPayload>::new();
    let mut image_paths = message_attachment_paths_by_mime(message, "image/");
    let mut audio_paths = message_attachment_paths_by_mime(message, "audio/");
    for part in &message.parts {
        match part {
            MessagePart::Image {
                mime, bytes_base64, ..
            } => {
                let stored_path = prompt_path_from_stored_binary_marker(bytes_base64, data_path);
                let resolved = if let Some(path) = data_path {
                    match resolve_stored_binary_base64(path, bytes_base64) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!(
                                "{} 解析图片附件失败，mime={}，data_path={}，bytes_base64_len={}，error={}",
                                log_prefix,
                                mime,
                                path.to_string_lossy(),
                                bytes_base64.len(),
                                err
                            );
                            bytes_base64.clone()
                        }
                    }
                } else {
                    bytes_base64.clone()
                };
                if !resolved.trim().is_empty() {
                    let saved_path = image_paths
                        .get_mut(&mime.trim().to_ascii_lowercase())
                        .and_then(|paths| paths.pop_front())
                        .or(stored_path);
                    images.push(PreparedBinaryPayload {
                        mime: mime.clone(),
                        content: resolved,
                        saved_path,
                    });
                }
            }
            MessagePart::Audio {
                mime, bytes_base64, ..
            } => {
                let stored_path = prompt_path_from_stored_binary_marker(bytes_base64, data_path);
                let resolved = if let Some(path) = data_path {
                    match resolve_stored_binary_base64(path, bytes_base64) {
                        Ok(value) => value,
                        Err(err) => {
                            eprintln!(
                                "{} 解析音频附件失败，mime={}，data_path={}，bytes_base64_len={}，error={}",
                                log_prefix,
                                mime,
                                path.to_string_lossy(),
                                bytes_base64.len(),
                                err
                            );
                            bytes_base64.clone()
                        }
                    }
                } else {
                    bytes_base64.clone()
                };
                if !resolved.trim().is_empty() {
                    let saved_path = audio_paths
                        .get_mut(&mime.trim().to_ascii_lowercase())
                        .and_then(|paths| paths.pop_front())
                        .or(stored_path);
                    audios.push(PreparedBinaryPayload {
                        mime: mime.clone(),
                        content: resolved,
                        saved_path,
                    });
                }
            }
            MessagePart::Text { .. } => {}
        }
    }
    (images, audios)
}

fn prompt_path_from_stored_binary_marker(
    value: &str,
    data_path: Option<&PathBuf>,
) -> Option<String> {
    let (kind, stored_id) = stored_binary_ref_from_marker(value.trim())?;
    let stored_id = stored_id.trim().replace('\\', "/");
    if stored_id.is_empty() {
        return None;
    }
    match kind {
        StoredBinaryRefKind::Download => Some(format!("downloads/{stored_id}")),
        StoredBinaryRefKind::Media => data_path
            .and_then(|path| media_storage_dir_from_data_path(path).ok())
            .map(|dir| dir.join(stored_id).to_string_lossy().replace('\\', "/")),
    }
}

#[cfg(test)]
mod prompt_media_path_tests {
    use super::*;

    #[test]
    fn prompt_path_from_stored_binary_marker_should_keep_download_refs_workspace_relative() {
        let data_path = PathBuf::from("C:/pai/config/app_data.json");

        let path = prompt_path_from_stored_binary_marker(
            &download_marker_from_id("conversation-a/image.png"),
            Some(&data_path),
        );

        assert_eq!(path.as_deref(), Some("downloads/conversation-a/image.png"));
    }

    #[test]
    fn prompt_path_from_stored_binary_marker_should_make_media_refs_reachable() {
        let data_path = PathBuf::from("C:/pai/config/app_data.json");

        let path = prompt_path_from_stored_binary_marker(
            &media_marker_from_id("image.png"),
            Some(&data_path),
        );

        assert_eq!(path.as_deref(), Some("C:/pai/media/image.png"));
    }
}

fn collect_prompt_media_parts(
    message: &ChatMessage,
    data_path: Option<&PathBuf>,
) -> (Vec<PreparedBinaryPayload>, Vec<PreparedBinaryPayload>) {
    resolve_media_from_message(message, data_path, "[提示词] 历史消息")
}

#[derive(Debug, Clone)]
struct PromptDepartmentCard {
    id: String,
    name: String,
    summary: String,
}

#[derive(Debug, Clone)]
struct PromptDepartmentContext {
    current: PromptDepartmentCard,
    available: Vec<PromptDepartmentCard>,
}

struct DepartmentPromptLabels {
    current_name_label: &'static str,
    current_guide_label: &'static str,
    available_title: &'static str,
    available_empty: &'static str,
    available_id_label: &'static str,
    available_summary_label: &'static str,
    empty_summary: &'static str,
    empty_guide: &'static str,
}

fn department_prompt_labels(_ui_language: &str) -> DepartmentPromptLabels {
    DepartmentPromptLabels {
        current_name_label: "部门",
        current_guide_label: "部门办事指南",
        available_title: "你的直属下级部门",
        available_empty: "当前没有可用的直属下级部门。",
        available_id_label: "部门 ID",
        available_summary_label: "概述",
        empty_summary: "未提供",
        empty_guide: "尚未配置办事指南。",
    }
}

fn prompt_department_card_from_config(
    department: &DepartmentConfig,
    empty_summary: &str,
) -> PromptDepartmentCard {
    PromptDepartmentCard {
        id: department.id.clone(),
        name: department.name.trim().to_string(),
        summary: if department.summary.trim().is_empty() {
            empty_summary.to_string()
        } else {
            department.summary.trim().to_string()
        },
    }
}

fn departments_only_config(departments: &[DepartmentConfig]) -> AppConfig {
    AppConfig {
        hotkey: String::new(),
        ui_language: String::new(),
        ui_font: String::new(),
        webview_zoom_percent: default_webview_zoom_percent(),
        web_access_port: default_web_access_port(),
        web_access_enabled: default_web_access_enabled(),
        web_access_password: default_web_access_password(),
        github_update_method: default_github_update_method(),
        record_hotkey: String::new(),
        record_background_wake_enabled: false,
        min_record_seconds: 0,
        max_record_seconds: 0,
        tool_max_iterations: 0,
        llm_round_log_capacity: default_llm_round_log_capacity(),
        message_notification_enabled: default_message_notification_enabled(),
        message_notification_sound_enabled: default_message_notification_sound_enabled(),
        selected_api_config_id: String::new(),
        assistant_department_api_config_id: String::new(),
        vision_api_config_id: None,
        tool_review_api_config_id: None,
        stt_api_config_id: None,
        stt_auto_send: false,
        terminal_shell_kind: default_terminal_shell_kind(),
        shell_workspaces: Vec::new(),
        mcp_servers: Vec::new(),
        remote_im_channels: Vec::new(),
        departments: departments.to_vec(),
        provider_non_stream_base_urls: Vec::new(),
        api_providers: Vec::new(),
        api_configs: Vec::new(),
    }
}

fn build_departments_prompt_block(
    _conversation: &Conversation,
    current_department_id: &str,
    departments: &[DepartmentConfig],
    ui_language: &str,
) -> String {
    if departments.is_empty() {
        return String::new();
    }
    let labels = department_prompt_labels(ui_language);
    let config = departments_only_config(departments);
    let current_department = department_by_id(&config, current_department_id);
    let prompt_context = current_department.map(|department| PromptDepartmentContext {
        current: prompt_department_card_from_config(department, labels.empty_summary),
        available: department_direct_child_departments(&config, department)
            .into_iter()
            .filter(|item| item.id != department.id)
            .map(|item| prompt_department_card_from_config(item, labels.empty_summary))
            .collect::<Vec<_>>(),
    });
    let Some(prompt_context) = prompt_context else {
        return String::new();
    };
    let guide = current_department
        .map(|department| department.guide.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| labels.empty_guide.to_string());
    let mut lines = vec![
        format!("{}：{}", labels.current_name_label, prompt_context.current.name),
        format!("{}：{}", labels.current_guide_label, guide),
        String::new(),
        format!("{}：", labels.available_title),
    ];
    if prompt_context.available.is_empty() {
        lines.push(labels.available_empty.to_string());
    } else {
        for department in prompt_context.available {
            lines.push(format!(
                "{}：{} | {}：{} | {}：{}",
                labels.current_name_label,
                department.name,
                labels.available_id_label,
                department.id,
                labels.available_summary_label,
                department.summary
            ));
        }
    }
    lines.push(String::new());
    prompt_xml_block("department context", lines.join("\n"))
}

fn build_memory_rag_rule_block() -> String {
    prompt_xml_block(
        "memory rag rule",
        "## 定位\n\
         - `<memory_context>` 是系统检索出的历史记忆背景，不是用户当前这条消息本身。\n\n\
         ## 使用原则\n\
         - 只有在确有帮助时才参考记忆。\n\
         - 自然融入理解与回复，不主动暴露检索、注入、档案读取等机制。\n\
         - 不要把记忆误当成用户此刻明确表达的立场、需求或情绪。\n\
         - 当前消息与记忆冲突时，一律以当前消息为准。\n\
         - 表现得像你正好理解用户，而不是像你参考了记忆。",
    )
}

fn build_builtin_tool_general_rule_block() -> String {
    prompt_xml_block(
        "builtin tool general rule",
        "仅在系统工具能真正帮助完成用户任务时才使用，不要为了显得主动而滥用工具。",
    )
}

fn build_builtin_tool_rule_block(tool_id: &str) -> Option<String> {
    let (block_name, body) = match tool_id.trim() {
        "todo" => (
            "todo tool rule",
            "## 何时使用\n\
             - 当前这轮已经明确要开始执行。\n\
             - 任务能拆成若干可完成、可验证的步骤。\n\
             - 正在执行的 plan 需要落到当前步骤时，用 todo 推进。\n\n\
             ## 不要使用\n\
             - 单步即可完成的简单问题。\n\
             - 纯解释性回答。\n\
             - 方案还没想清楚的复杂任务。\n\n\
             ## 使用要求\n\
             - 拆成 3~7 步。\n\
             - 每一步都必须有可验证、可完成的结果。\n\
             - 开始执行后及时更新状态。\n\
             - 任一时刻只允许一个 `in_progress`。\n\
             - 计划变化时同步修正 todo。\n\n\
             ## 定位\n\
             - todo 是当前会话内的执行步骤板。\n\
             - plan 负责阶段方案，todo 负责当前执行。\n\
             - todo 不是长期任务系统，也不是方案设计工具。",
        ),
        "goal" => (
            "goal tool rule",
            "## 定位\n\
             - goal 是绑定当前会话的持续执行状态，不是普通进度记录。\n\
             - `create_goal` 只在用户明确要求启动目标时调用；已有 active goal 时不要覆盖。\n\
             - `update_goal` 只允许在目标真正完成或严格阻塞审计成立时调用。\n\n\
             ## 完成\n\
             - 只有当前证据能证明原始目标的全部显式要求都满足，才调用 `update_goal` 并设置 `status: \"complete\"`。\n\
             - complete 必须提供 evidence，说明哪些当前状态、文件、命令输出、测试或运行结果证明目标完成。\n\
             - 不要把部分进展、当前轮能完成的小范围、测试暂时通过或最终答复本身当成完成证明。\n\n\
             ## 阻塞\n\
             - 不要在第一次遇到困难或不确定时标记 blocked。\n\
             - 只有同一阻塞条件连续至少三轮 goal 轮次重复，且没有用户输入或外部状态变化就无法继续推进时，才调用 `update_goal` 并设置 `status: \"blocked\"`。\n\
             - blocked 必须提供 `blocking_condition`，说明同一阻塞条件是什么。\n\n\
             ## 活跃状态\n\
             - 如果目标尚未被完整证明完成，也未满足严格阻塞条件，就保持 goal active，继续朝最终状态推进。",
        ),
        "get_session" | "inform_session" => (
            "session tool rule",
            "## 定位\n\
             - `get_session` 用来查询可投递的会话列表，只返回本地普通未归档会话和远程联系人会话。\n\
             - `inform_session` 用来向指定会话投递一条系统助理通知。\n\n\
             ## 使用原则\n\
             - 只有在确实需要跨会话同步信息、提醒、续接上下文或通知远程联系人时才使用。\n\
             - 先用 `get_session` 缩小目标范围，再调用 `inform_session`。\n\
             - `inform_session` 只负责投递，不会自动让目标会话继续推理。\n\
             - 通知正文应简洁、可执行、可读，不要把整轮冗长思考原样倾倒给别的会话。",
        ),
        "delegate" => (
            "delegate tool rule",
            "## 何时优先使用\n\
             - 当前工作有职责或能力更匹配的直属下级部门时，优先使用 delegate。\n\
             - 子任务不需要完整当前上下文，只要用 `why`、`goal`、`todo` 就能独立说明清楚时，优先委托。\n\
             - 简单但繁琐的搜索、排查、比对、整理、验证、资料收集、影响面摸底等工作，适合委托给下级部门完成。\n\
             - 主线程需要这个结果继续下一步时，也可以委托，但必须使用 `mode: \"wait\"` 等待结果。`wait` 可以并发发出多个委托，它只表示等待结果，不表示串行。\n\n\
             ## 何时不要使用\n\
             - 没有合适的直属下级部门。\n\
             - 子任务无法脱离完整当前上下文，压缩成背景后会丢失关键判断依据。\n\
             - 用户要求你本人直接完成，或任务需要你立即和用户连续澄清。\n\n\
             ## 使用要求\n\
             - 除非用户明确指示后台运行，否则一律使用 `mode: \"wait\"`。\n\
             - 需要并发委托时，也应使用 `wait`；一次发出多个 `wait` 委托后等待全部结果再整合。\n\
             - 只有用户明确要求后台运行、不等待结果时，才使用 `mode: \"background\"`。\n\
             - 当前已经在委托线程中再次委托时，只允许使用 `wait`。\n\
             - 若目标岗位由你本人兼任，只允许使用 `wait`。\n\
             - `department_id` 必须选择最匹配的直属下级部门。\n\
             - `why` 写清父任务、已知事实、必要上下文、约束和前序结果；不要只写一句空泛背景。\n\
             - `goal` 写清本次子任务要完成什么，目标应可判断是否完成。\n\
             - `todo` 写清优先关注点、范围边界、交付要求和需要避免的方向。\n\
             - 收到同步委托结果后，必须由你整合、判断、裁决并继续推进，不要机械转述。\n\
             - 对下级返回的关键结论、风险判断、文件定位、数据口径或会影响最终决策的发现，必须挑选重点亲自核验；可以信任下级完成繁琐工作，但不要盲目相信未经核验的关键结论。",
        ),
        "task" => (
            "task tool rule",
            "## 何时使用\n\
             - 事情已经超出当前这次会话。\n\
             - 后续需要自动触发、延后执行、循环提醒或跨会话续跑。\n\
             - 长期目标在推进中会持续产生多个阶段计划。\n\n\
             ## 不要使用\n\
             - 当前会话里就能直接推进并完成。\n\
             - 当前阶段还能靠 plan 和 todo 收口。\n\
             - 只是想记录一个临时执行步骤。\n\n\
             ## 使用要求\n\
             - 只在确实需要未来继续跑时创建。\n\
             - 写清长期目标、触发方式和续跑条件。\n\
             - 不要把 task 当成当前步骤板。\n\n\
             ## 定位\n\
             - task 管长期目标。\n\
             - plan 管阶段推进。\n\
             - todo 管当前执行步骤。",
        ),
        "exec" => (
            "exec tool rule",
            "## 何时使用\n\
             当需要搜索文件、读取代码/配置/日志、检查环境、查看 git 状态、运行验证、执行脚本或获取本地事实时，使用 `exec`。很多 skill 会要求执行脚本。\n\n\
             ## 探索策略\n\
             在不了解代码位置、调用链或现状时，先广撒网建立上下文，再收敛到具体文件和实现。不要只凭一个搜索结果就过早下结论。\n\n\
             - **并发搜索按独立线索方向拆分，不要机械拆词。** 每个搜索命令可以合并同族关键词、别名、字段名、函数名、日志前缀或中英文相关术语来提高召回，但它们必须共同指向同一个检索意图。\n\
             - **同一检索意图内用 `|` 连接别名。** 例如搜索工作目录字段时，可用 `rg -n \"workspaceLabel|workspaceRootPath|shellWorkspaces|工作目录\"`；搜索会话创建入口时，可用 `rg -n \"createConversation|newConversation|新建会话\"`。\n\
             - **彼此独立、互不依赖的线索方向应一次性并发发起。** 例如同时搜索入口组件、类型定义、后端命令、日志文案和测试位置；不要等一个无关搜索结束后才开始下一个。\n\
             - **依赖前序结果时必须串行。** 先用较宽的搜索找到候选文件，再根据命中结果读取具体片段、追调用链或运行针对性验证。\n\
             - **搜索文本优先 `rg`，找文件优先 `rg --files`。** 如果缺失，再使用当前 shell 的原生命令。\n\n\
             ## 使用边界\n\
             - `exec` 默认是一次性进程执行；命令结束、失败或超时后会回收本次进程树，不依赖持久 live shell 状态。\n\
             - 读取文件、列目录、搜索文本、查看 `git diff/status` 等低风险探索可以积极使用。\n\
             - 运行测试或构建时，只执行和本次改动直接相关的最小必要检查。\n\
             - 新增、删除、移动或改写文件必须优先使用文件编辑工具（`write` / `delete` / `update` / `move`）。不要用 `exec` 通过 `cat`、`echo`、重定向、heredoc、python 等方式直接写文件，除非用户明确要求或没有可行替代。\n\
             - 不要执行删除、移动、清空、重置仓库、切换分支等破坏性命令，除非用户明确要求。\n\
             - Windows 下不要跨 shell 拼接破坏性文件操作；如必须处理路径，优先使用当前 shell 的原生命令和明确路径。\n\
             - 若 `exec` 报告工作区、路径授权或 `shell_switch_workspace` 相关错误，应先解决工作区与授权前提，再继续当前工具链。\n\n\
             ## 为什么\n\
             `exec` 用来快速建立本地事实和验证判断。先按独立线索并发搜索，再基于结果收敛，可以减少误判、漏查和反复试错；文件修改仍由更可控的编辑工具承担。",
        ),
        "write" | "delete" | "update" | "move" | "file_edit" => (
            "file edit tool rule",
            "请优先使用文件编辑工具写文件，不要使用终端或者 python 写入。\n\
             - 新增完整文件或明确要写入完整内容时，使用 `write`。\n\
             - 删除整个文件时，使用 `delete`。\n\
             - 删除或修改文件中的局部内容时，使用 `update`；不要把局部内容删除写成 `delete`。\n\
             - 移动或重命名文件时，使用 `move`。\n\
             - 如果 `update` 的目标片段不唯一，应扩大 `old_string` 上下文，或明确设置 `replace_all: true`。 ",
        ),
        "file_reference" => (
            "file reference rule",
            "## 何时必须引用\n\
             - 回复里提到本地文件、目录、代码位置、配置、日志、计划文档或其他现有路径。\n\
             - 你知道该对象的绝对路径。\n\
             - 你在说明改了哪个文件、建议用户查看哪个文件、总结代码位置或列举相关文件。\n\n\
             ## 不要硬造引用\n\
             - 只是泛指文件类型。\n\
             - 还不知道具体绝对路径。\n\
             - 当前对象是远程联系人，且本地路径不应暴露。\n\n\
             ## 正确格式\n\
             - 本地文件：`[file.rs](/abs/path/file.rs)`\n\
             - 本地文件带行号：`[file.rs](/abs/path/file.rs:12)`\n\
             - 网页：`[文档](https://example.com)`\n\n\
             ## 注意\n\
             - 当前前端识别盘符开头的绝对本地路径；`file:///E:/...` 容易被当成普通网页链接或被错误解析。",
        ),
        "image_reference" => (
            "image reference rule",
            "## 在回复中展示本地图片\n\
             - 当本地已经有一张图片文件时（比如截图、脚本输出图表、分析结果图），在回复正文中使用标准 Markdown 图片语法 `![说明](路径)`，就能直接把本地图片展示在对话里。\n\
             - 示例：`![结果图](E:/path/to/result.png)`、`![结果图](outputs/result.png)`\n\
             - 路径可以是绝对路径或相对于工作区目录的相对路径。\n\
             - 只能引用已经存在的本地图片文件，不要编造路径。\n\
             - 支持的格式：png、jpg/jpeg、webp、gif、bmp。\n\n\
             ## 不要这样做\n\
             - 不要输出 `file://` 链接来引用图片；直接写本地路径或相对路径。\n\
             - 不要把本地图片写成普通文本链接 `[说明](路径)`；图片必须使用 `![说明](路径)`。",
        ),
        _ => return None,
    };
    Some(prompt_xml_block(block_name, body))
}

fn department_builtin_tool_enabled(
    department_config: &AppConfig,
    current_department: Option<&DepartmentConfig>,
    id: &str,
) -> bool {
    if !builtin_tool_is_department_controlled(id) {
        return builtin_tool_is_fixed_system(id);
    }
    if builtin_tool_unavailable_reason(department_config, current_department, id).is_some() {
        return false;
    }
    if tool_forced_by_department(current_department, id) {
        return true;
    }
    if !department_permission_allows_any_name(
        current_department,
        DepartmentPermissionCategory::BuiltinTool,
        &[id],
    ) {
        return false;
    }
    if let Some(tool) = default_agent_tools().iter().find(|tool| tool.id == id) {
        return tool.enabled;
    }
    true
}

fn build_system_tools_rule_blocks(
    current_department_id: &str,
    departments: &[DepartmentConfig],
) -> Vec<String> {
    let department_config = departments_only_config(departments);
    let current_department = department_by_id(&department_config, current_department_id);
    let mut blocks = Vec::<String>::new();
    let mut any_builtin_enabled = false;
    for tool_id in ["delegate", "task", "exec"] {
        if department_builtin_tool_enabled(&department_config, current_department, tool_id) {
            any_builtin_enabled = true;
            if let Some(block) = build_builtin_tool_rule_block(tool_id) {
                blocks.push(block);
            }
        }
    }
    if ["write", "delete", "update", "move"]
        .into_iter()
        .any(|tool_id| department_builtin_tool_enabled(&department_config, current_department, tool_id))
    {
        any_builtin_enabled = true;
        if let Some(block) = build_builtin_tool_rule_block("file_edit") {
            blocks.push(block);
        }
    }
    if ["exec", "read", "read_file", "write", "delete", "update", "move"]
        .into_iter()
        .any(|tool_id| department_builtin_tool_enabled(&department_config, current_department, tool_id))
    {
        any_builtin_enabled = true;
        if let Some(block) = build_builtin_tool_rule_block("file_reference") {
            blocks.push(block);
        }
    }
    if any_builtin_enabled {
        blocks.insert(0, build_builtin_tool_general_rule_block());
    }
    blocks
}

fn build_question_and_planning_rule_block(
    state: Option<&AppState>,
    conversation: &Conversation,
) -> String {
    let preferred_plan_dir = state
        .and_then(|app_state| {
            plan_preferred_directory_display_for_conversation(app_state, Some(conversation)).ok()
        })
        .unwrap_or_else(|| "{会话工作目录或助理空间}\\.pai\\plan".to_string());
    prompt_xml_block(
        "plan tool rule",
        &format!(
            "## 提问之法\n\
             - **价值锚定**：只有缺失信息会显著影响方向、风险、成本或产出时，才向用户提问。\n\
             - **前置分析**：提问前先检索上下文，形成初步逻辑模型。\n\
             - **低通量**：首轮提问必须精准，避免堆砌问题清单。\n\
             - **自主检索**：代码、配置、既有文档可自证时，先自己查。\n\
             - **默认推进**：存在高概率、低风险默认假设时，带着假设推进并明确告知。\n\
             - **拒绝外包**：自身职责内的分析、设计和决策，不转嫁给用户。\n\n\
             ## 何时使用 plan\n\
             - 任务复杂到不能直接靠 todo 执行。\n\
             - 需要先想清阶段目标、方案、边界、风险或取舍。\n\
             - 阶段方案需要和用户对齐，或后续会持续修正。\n\n\
             ## 层级关系\n\
             - task 管长期目标。\n\
             - plan 管当前阶段或某次推进方案。\n\
             - todo 管 plan 执行中的具体步骤。\n\
             - 正在执行的 plan 应继续拆成 todo 推进。\n\
             - 若后续需要跨会话长期续跑，再把更高层目标交给 task。\n\n\
             ## 计划文件\n\
             - 先把计划写成 Markdown 文件，再调用 plan。\n\
             - 优先写到：{preferred_plan_dir}\n\
             - `plan.present` 只提交 `path`。\n\
             - `plan.complete` 也只提交同一份 `path`。\n\
             - 只改计划局部时，优先补丁修改现有计划文件，不要整份重写。\n\n\
             ## 规划方式\n\
             - 遇到非平凡任务，先扫描核心文件，构建含主阶段与要点的骨架计划。\n\
             - 计划未定前，不做穷尽式本地或网络搜寻。\n\
             - 计划要驱动迭代，不写成冗长形式文档。\n\
             - 遇到非显性分叉或明显成本差异，先和用户同步，再做重度开发。\n\
             - 新信息进入后及时修正计划，不硬执行旧路线。\n\n\
             ## 核心逻辑\n\
             - 提问是为了破除当前计划的核心不确定性。\n\
             - 计划是把当前认知转化为可执行路径。\n\
             - 不要在无初步计划时盲目索取信息。\n\
             - 不要在关键因子未明时假装架构稳定。"
        ),
    )
}

#[allow(dead_code)]
fn build_prompt(
    conversation: &Conversation,
    agent: &AgentProfile,
    agents: &[AgentProfile],
    departments: &[DepartmentConfig],
    user_name: &str,
    user_intro: &str,
    response_style_id: &str,
    ui_language: &str,
    data_path: Option<&PathBuf>,
    state: Option<&AppState>,
    resolved_api: Option<&ResolvedApiConfig>,
    enable_pdf_images: bool,
) -> PreparedPrompt {
    build_prompt_with_stage_logger(
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
        None,
        resolved_api,
        enable_pdf_images,
    )
    .expect("build prompt")
}

fn build_prompt_with_stage_logger(
    conversation: &Conversation,
    agent: &AgentProfile,
    agents: &[AgentProfile],
    departments: &[DepartmentConfig],
    user_name: &str,
    user_intro: &str,
    response_style_id: &str,
    ui_language: &str,
    data_path: Option<&PathBuf>,
    state: Option<&AppState>,
    stage_logger: Option<&dyn Fn(&str)>,
    resolved_api: Option<&ResolvedApiConfig>,
    enable_pdf_images: bool,
) -> Result<PreparedPrompt, String> {
    build_prompt_with_mode(
        conversation,
        agent,
        agents,
        departments,
        Some((user_name, user_intro)),
        response_style_id,
        ui_language,
        data_path,
        state,
        stage_logger,
        resolved_api,
        enable_pdf_images,
    )
}

#[allow(dead_code)]
fn build_delegate_prompt(
    conversation: &Conversation,
    agent: &AgentProfile,
    agents: &[AgentProfile],
    departments: &[DepartmentConfig],
    response_style_id: &str,
    ui_language: &str,
    data_path: Option<&PathBuf>,
    state: Option<&AppState>,
    resolved_api: Option<&ResolvedApiConfig>,
    enable_pdf_images: bool,
) -> PreparedPrompt {
    build_delegate_prompt_with_stage_logger(
        conversation,
        agent,
        agents,
        departments,
        response_style_id,
        ui_language,
        data_path,
        state,
        None,
        resolved_api,
        enable_pdf_images,
    )
    .expect("build delegate prompt")
}

fn build_delegate_prompt_with_stage_logger(
    conversation: &Conversation,
    agent: &AgentProfile,
    agents: &[AgentProfile],
    departments: &[DepartmentConfig],
    response_style_id: &str,
    ui_language: &str,
    data_path: Option<&PathBuf>,
    state: Option<&AppState>,
    stage_logger: Option<&dyn Fn(&str)>,
    resolved_api: Option<&ResolvedApiConfig>,
    enable_pdf_images: bool,
) -> Result<PreparedPrompt, String> {
    build_prompt_with_mode(
        conversation,
        agent,
        agents,
        departments,
        None,
        response_style_id,
        ui_language,
        data_path,
        state,
        stage_logger,
        resolved_api,
        enable_pdf_images,
    )
}

fn find_last_context_compaction_index(
    messages: &[ChatMessage],
    agent_id: &str,
) -> Option<usize> {
    messages
        .iter()
        .enumerate()
        .filter_map(|(idx, message)| {
            let role = prompt_role_for_message(message, agent_id)?;
            if is_context_compaction_message(message, role.as_str()) {
                Some(idx)
            } else {
                None
            }
        })
        .last()
}

fn merge_optional_text_block(current: &mut Option<String>, next: Option<String>) {
    let Some(next_text) = next.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()) else {
        return;
    };
    match current {
        Some(existing) if !existing.trim().is_empty() => {
            existing.push_str("\n\n");
            existing.push_str(&next_text);
        }
        _ => {
            *current = Some(next_text);
        }
    }
}

fn merge_history_message_text(current: &mut String, next: String) {
    if current.trim().is_empty() {
        *current = next;
        return;
    }
    if next.trim().is_empty() {
        return;
    }
    current.push_str("\n\n");
    current.push_str(&next);
}

fn merge_adjacent_assistant_history_messages(
    messages: Vec<PreparedHistoryMessage>,
) -> Vec<PreparedHistoryMessage> {
    let mut merged = Vec::<PreparedHistoryMessage>::new();
    for message in messages {
        if message.role == "assistant" {
            if message.tool_calls.as_ref().is_some_and(|calls| !calls.is_empty()) {
                merged.push(message);
                continue;
            }
            if let Some(last) = merged.last_mut() {
                if last.role == "assistant"
                    && last
                        .tool_calls
                        .as_ref()
                        .map(|calls| calls.is_empty())
                        .unwrap_or(true)
                {
                    merge_history_message_text(&mut last.text, message.text);
                    last.extra_text_blocks.extend(message.extra_text_blocks);
                    last.images.extend(message.images);
                    last.audios.extend(message.audios);
                    if let Some(mut next_calls) = message.tool_calls {
                        if let Some(current_calls) = last.tool_calls.as_mut() {
                            current_calls.append(&mut next_calls);
                        } else {
                            last.tool_calls = Some(next_calls);
                        }
                    }
                    if last.tool_call_id.is_none() {
                        last.tool_call_id = message.tool_call_id;
                    }
                    merge_optional_text_block(&mut last.user_time_text, message.user_time_text);
                    merge_optional_text_block(&mut last.reasoning_content, message.reasoning_content);
                    continue;
                }
            }
        }
        merged.push(message);
    }
    merged
}

fn normalized_prepared_history_messages(
    messages: &[PreparedHistoryMessage],
) -> Vec<PreparedHistoryMessage> {
    merge_adjacent_assistant_history_messages(messages.to_vec())
}

fn normalize_prepared_history_messages_in_place(prepared: &mut PreparedPrompt) {
    prepared.history_messages =
        merge_adjacent_assistant_history_messages(std::mem::take(&mut prepared.history_messages));
}

fn build_conversation_prompt_payload(
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
    let mut seen_remote_contacts = std::collections::HashSet::<String>::new();
    let mut seen_prompt_memory_ids = HashSet::<String>::new();
    let mut history_messages = Vec::<PreparedHistoryMessage>::new();
    for (idx, message) in enriched_conversation.messages.iter().enumerate() {
        if is_tool_review_report_message(message) {
            continue;
        }
        let Some(role) = prompt_role_for_message(message, &agent.id) else {
            continue;
        };
        if Some(idx) == latest_user_index {
            continue;
        }
        let is_self_message = role == "assistant";
        if is_self_message {
            history_messages.extend(build_prepared_history_messages_from_tool_history(
                message,
                MessageToolHistoryView::PromptReplay,
            ));
        }
        let is_user = role == "user";
        let (history_user_meta_text, history_extra_blocks) = if is_user {
            let include_remote_identity = remote_im_contact_key_from_message(message)
                .map(|key| seen_remote_contacts.insert(key))
                .unwrap_or(false);
            (
                build_prompt_user_meta_text(
                    message,
                    agents,
                    prompt_user_name,
                    ui_language,
                    include_remote_identity,
                ),
                prompt_user_extra_blocks_for_message(
                    state,
                    Some(source_conversation),
                    message,
                    agents,
                    prompt_user_name,
                    ui_language,
                    include_remote_identity,
                    recall_memories,
                    &mut seen_prompt_memory_ids,
                    false,
                ),
            )
        } else {
            (None, Vec::new())
        };
        let mut text = if is_user {
            let rendered = render_prompt_user_text_only(message);
            let mention_prefix = render_prompt_user_mention_prefix(message);
            if mention_prefix.is_empty() {
                rendered
            } else if rendered.trim().is_empty() {
                mention_prefix
            } else {
                format!("{mention_prefix}\n{rendered}")
            }
        } else {
            render_prompt_message_text(message)
        };
        let (images, audios) = if is_user {
            collect_prompt_media_parts(message, data_path)
        } else {
            (Vec::new(), Vec::new())
        };
        if text.trim().is_empty() && images.is_empty() && audios.is_empty() {
            text = " ".to_string();
        }
        history_messages.push(PreparedHistoryMessage {
            role: role.clone(),
            text,
            extra_text_blocks: history_extra_blocks,
            user_time_text: history_user_meta_text,
            images,
            audios,
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: if is_self_message {
                render_prompt_message_reasoning(message)
            } else {
                None
            },
        });
    }

    let latest_user = latest_user_index
        .and_then(|idx| enriched_conversation.messages.get(idx).cloned());
    let mut latest_user_text = String::new();
    let mut latest_user_meta_text = String::new();
    let mut latest_user_extra_blocks = Vec::<String>::new();
    let mut latest_images = Vec::<PreparedBinaryPayload>::new();
    let mut latest_audios = Vec::<PreparedBinaryPayload>::new();

    if let Some(msg) = latest_user {
        let latest_user_text_rendered = {
            let rendered = render_prompt_user_text_only(&msg);
            let mention_prefix = render_prompt_user_mention_prefix(&msg);
            if mention_prefix.is_empty() {
                rendered
            } else if rendered.trim().is_empty() {
                mention_prefix
            } else {
                format!("{mention_prefix}\n{rendered}")
            }
        };
        let (resolved_images, resolved_audios) =
            resolve_media_from_message(&msg, data_path, "[提示词] 最新消息");
        let include_remote_identity = remote_im_contact_key_from_message(&msg)
            .map(|key| seen_remote_contacts.insert(key))
            .unwrap_or(false);
        latest_user_meta_text = build_prompt_user_meta_text(
            &msg,
            agents,
            prompt_user_name,
            ui_language,
            include_remote_identity,
        )
        .unwrap_or_default();
        let latest_extra_blocks = prompt_user_extra_blocks_for_message(
            state,
            Some(source_conversation),
            &msg,
            agents,
            prompt_user_name,
            ui_language,
            include_remote_identity,
            recall_memories,
            &mut seen_prompt_memory_ids,
            true,
        );
        latest_user_text = latest_user_text_rendered;
        latest_images = resolved_images;
        latest_audios = resolved_audios;
        for extra in latest_extra_blocks {
            let trimmed = extra.trim();
            if trimmed.is_empty() {
                continue;
            }
            latest_user_extra_blocks.push(trimmed.to_string());
        }
        if latest_user_text.trim().is_empty()
            && latest_user_meta_text.trim().is_empty()
            && latest_user_extra_blocks.is_empty()
            && latest_images.is_empty()
            && latest_audios.is_empty()
        {
            latest_user_text = " ".to_string();
        }
    }

    PreparedConversationPromptPayload {
        history_messages: merge_adjacent_assistant_history_messages(history_messages),
        latest_user_text,
        latest_user_meta_text,
        latest_user_extra_blocks,
        latest_images,
        latest_audios,
    }
}

fn build_prompt_with_mode(
    conversation: &Conversation,
    _agent: &AgentProfile,
    agents: &[AgentProfile],
    departments: &[DepartmentConfig],
    user_profile: Option<(&str, &str)>,
    response_style_id: &str,
    ui_language: &str,
    data_path: Option<&PathBuf>,
    state: Option<&AppState>,
    stage_logger: Option<&dyn Fn(&str)>,
    _resolved_api: Option<&ResolvedApiConfig>,
    enable_pdf_images: bool,
) -> Result<PreparedPrompt, String> {
    let prompt_agent = resolve_conversation_bound_agent(conversation, agents, departments)?;
    let source_messages = match find_last_context_compaction_index(
        &conversation.messages,
        &prompt_agent.id,
    ) {
        Some(boundary) => &conversation.messages[boundary..],
        None => conversation.messages.as_slice(),
    };

    let mut enriched_messages = source_messages.to_vec();

    if let Some(state) = state {
        for message in &mut enriched_messages {
            if let Some(meta) = message.provider_meta.as_ref() {
                if let Some(attachments) = meta.get("attachments").and_then(Value::as_array) {
                    for item in attachments {
                        let mime = item.get("mime").and_then(Value::as_str).unwrap_or("");
                        if mime != "application/pdf" {
                            continue;
                        }
                        let relative_path =
                            item.get("relativePath").and_then(Value::as_str).unwrap_or("");
                        if relative_path.is_empty() {
                            continue;
                        }
                        let file_name = item.get("fileName").and_then(Value::as_str).unwrap_or("");
                        let include_images = enable_pdf_images;
                        let workspace_root = configured_workspace_root_path(state)
                            .unwrap_or_else(|_| state.llm_workspace_path.clone());
                        let file_path = workspace_root.join(relative_path);
                        let Some(file_path_str) = file_path.to_str() else {
                            eprintln!(
                                "[PDF提取] 跳过 路径包含非UTF-8字符, conversation_id={}, relative_path={}",
                                conversation.id, relative_path
                            );
                            continue;
                        };
                        let conversation_id = conversation.id.clone();
                        match get_or_extract_pdf_structured(
                            state,
                            &conversation_id,
                            file_path_str,
                            include_images,
                        ) {
                            Ok(result) => {
                                if enable_pdf_images {
                                    for page in result.pages {
                                        for (img_idx, img) in page.images.iter().enumerate() {
                                            message.parts.push(MessagePart::Image {
                                                mime: img.mime.clone(),
                                                bytes_base64: img.bytes_base64.clone(),
                                                name: Some(format!(
                                                    "{}_p{}_img{}.webp",
                                                    result.file_name,
                                                    page.page_index + 1,
                                                    img_idx + 1,
                                                )),
                                                compressed: false,
                                            });
                                        }
                                    }
                                } else {
                                    for page in result.pages {
                                        let page_text = format!(
                                            "[PDF文档分页]\n文件名：{}\n页码：{}/{}\n\n{}",
                                            result.file_name,
                                            page.page_index + 1,
                                            result.total_pages,
                                            page.text
                                        );
                                        message.extra_text_blocks.push(page_text);
                                    }
                                }
                                message.extra_text_blocks.push(format!(
                                    "提示：如需阅读完整内容，请使用 shell 工具读取 {}",
                                    relative_path
                                ));
                            }
                            Err(e) => {
                                if is_pdf_page_limit_exceeded_error(&e) {
                                    message.extra_text_blocks.push(format!(
                                        "提示：PDF 页数超过 {} 页，已按普通文件处理，不进行自动提取。",
                                        100
                                    ));
                                    message.extra_text_blocks.push(format!(
                                        "提示：如需阅读完整内容，请使用 shell 工具读取 {}",
                                        relative_path
                                    ));
                                    continue;
                                }
                                eprintln!(
                                    "[PDF提取] 失败 conversation_id={}, file_name={}, error={:?}",
                                    conversation_id, file_name, e
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    let enriched_conversation = Conversation {
        id: conversation.id.clone(),
        title: conversation.title.clone(),
        agent_id: conversation.agent_id.clone(),
        department_id: conversation.department_id.clone(),
        bound_conversation_id: conversation.bound_conversation_id.clone(),
        parent_conversation_id: conversation.parent_conversation_id.clone(),
        child_conversation_ids: conversation.child_conversation_ids.clone(),
        fork_message_cursor: conversation.fork_message_cursor.clone(),
        unread_count: conversation.unread_count,
        conversation_kind: conversation.conversation_kind.clone(),
        root_conversation_id: conversation.root_conversation_id.clone(),
        delegate_id: conversation.delegate_id.clone(),
        created_at: conversation.created_at.clone(),
        updated_at: conversation.updated_at.clone(),
        last_user_at: conversation.last_user_at.clone(),
        last_assistant_at: conversation.last_assistant_at.clone(),
        status: conversation.status.clone(),
        summary: conversation.summary.clone(),
        user_profile_snapshot: conversation.user_profile_snapshot.clone(),
        shell_workspace_path: conversation.shell_workspace_path.clone(),
        shell_workspaces: conversation.shell_workspaces.clone(),
        shell_autonomous_mode: conversation.shell_autonomous_mode,
        archived_at: conversation.archived_at.clone(),
        messages: enriched_messages,
        current_todos: conversation.current_todos.clone(),
        memory_recall_table: conversation.memory_recall_table.clone(),
        plan_mode_enabled: conversation.plan_mode_enabled,
        preferred_api_config_id: conversation.preferred_api_config_id.clone(),
        auto_push_remote_contact_id: conversation.auto_push_remote_contact_id.clone(),
        cumulative_usage: conversation.cumulative_usage.clone(),
        active_goal: conversation.active_goal.clone(),
    };
    let recall_memory_ids = collect_prompt_retrieved_memory_ids(&enriched_conversation.messages);
    let recall_memories = if recall_memory_ids.is_empty() {
        None
    } else {
        data_path.and_then(|path| match memory_store_list_memories_by_ids_visible_for_agent(
            path,
            &recall_memory_ids,
            &prompt_agent.id,
            prompt_agent.private_memory_enabled,
        ) {
            Ok(memories) => Some(memories),
            Err(err) => {
                runtime_log_error(format!(
                    "[提示词] 读取召回记忆失败: agent_id={}, recall_ids={:?}, error={:?}",
                    prompt_agent.id, recall_memory_ids, err
                ));
                None
            }
        })
    };

    let prompt_user_name = user_profile.map(|(user_name, _)| user_name).unwrap_or("");
    let last_compaction_index =
        find_last_context_compaction_index(&enriched_conversation.messages, &prompt_agent.id);
    let mut latest_user_index = None;
    for (idx, message) in enriched_conversation.messages.iter().enumerate().rev() {
        if let Some(boundary) = last_compaction_index {
            if idx < boundary {
                break;
            }
        }
        if is_tool_review_report_message(message) {
            continue;
        }
        let Some(role) = prompt_role_for_message(message, &prompt_agent.id) else {
            continue;
        };
        if is_context_compaction_message(message, role.as_str()) {
            continue;
        }
        if role == "user" {
            latest_user_index = Some(idx);
        }
        break;
    }

    let preamble = build_core_system_prompt_text(
        &enriched_conversation,
        prompt_agent,
        departments,
        user_profile,
        response_style_id,
        ui_language,
        state,
    );
    if let Some(log_stage) = stage_logger {
        log_stage("prepare_context.prompt_fixed_system_ready");
    }
    let conversation_payload = conversation_prompt_service().build_conversation_payload(
        &enriched_conversation,
        conversation,
        prompt_agent,
        agents,
        state,
        data_path,
        recall_memories.as_deref(),
        prompt_user_name,
        ui_language,
        latest_user_index,
    );
    if let Some(log_stage) = stage_logger {
        log_stage("prepare_context.prompt_conversation_payload_ready");
    }

    let mut latest_user_extra_blocks = conversation_payload.latest_user_extra_blocks;
    if let Some(path) = data_path {
        match message_store::active_plan_prompt_block(path, &conversation.id) {
            Ok(Some(active_plan_block)) => {
                latest_user_extra_blocks.push(active_plan_block);
            }
            Ok(None) => {}
            Err(err) => {
                runtime_log_error(format!(
                    "[提示词] active_plan_prompt_block 读取失败: conversation_id={}, data_path={}, error={:?}",
                    conversation.id,
                    path.display(),
                    err
                ));
            }
        }
    }

    Ok(PreparedPrompt {
        preamble,
        history_messages: conversation_payload.history_messages,
        latest_user_text: conversation_payload.latest_user_text,
        latest_user_meta_text: conversation_payload.latest_user_meta_text,
        latest_user_extra_text: latest_user_extra_blocks.join("\n\n"),
        latest_user_extra_blocks,
        latest_images: conversation_payload.latest_images,
        latest_audios: conversation_payload.latest_audios,
    })
}

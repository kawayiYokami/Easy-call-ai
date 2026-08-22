fn send_text_delta_event(
    on_delta: &tauri::ipc::Channel<AssistantDeltaEvent>,
    text: &str,
) {
    if text.is_empty() {
        return;
    }
    let _ = on_delta.send(AssistantDeltaEvent {
        delta: text.to_string(),
        kind: None,
        request_id: None,
        activation_id: None,
        phase_id: None,
        reason: None,
        tool_name: None,
        tool_call_id: None,
        tool_status: None,
        tool_args: None,
        message: None,
        stream_cache: None,
    });
}

fn assistant_tool_group_history_event_value(
    turn_text: &str,
    tool_calls: &[genai::chat::ToolCall],
    turn_reasoning: &str,
    trusted_input_tokens: Option<u64>,
    context_window_tokens: u32,
) -> Value {
    let tool_call_values = tool_loop_round_tool_calls_json(tool_calls);
    let content = turn_text
        .trim()
        .is_empty()
        .then_some(Value::Null)
        .unwrap_or_else(|| Value::String(turn_text.to_string()));
    let mut assistant_tool_event = serde_json::json!({
        "role": "assistant",
        "content": content,
        "tool_calls": tool_call_values
    });
    if let Some(object) = assistant_tool_event.as_object_mut() {
        let reasoning = turn_reasoning.trim();
        if !reasoning.is_empty() {
            object.insert(
                "reasoning_content".to_string(),
                Value::String(reasoning.to_string()),
            );
        }
        // 本轮 LLM 响应的真实用量随工具调用事件落盘（与 metadata 先例同层），
        // 聚合侧直接取事件用量，不再另行在 provider_meta 补写。
        // context_window 归一化到 >=1，与 meta 写入口径一致，避免聚合侧把 0 当 1 造成不一致。
        if let Some(prompt_tokens) = trusted_input_tokens.filter(|value| *value > 0) {
            object.insert(
                "usage".to_string(),
                serde_json::json!({
                    "promptTokens": prompt_tokens,
                    "contextWindowTokens": context_window_tokens.max(1),
                }),
            );
        }
    }
    assistant_tool_event
}

fn assistant_tool_group_stream_event_value(
    turn_text: &str,
    tool_calls: &[genai::chat::ToolCall],
) -> Value {
    let tool_call_values = tool_loop_round_tool_calls_json(tool_calls);
    let content = turn_text
        .trim()
        .is_empty()
        .then_some(Value::Null)
        .unwrap_or_else(|| Value::String(turn_text.to_string()));
    serde_json::json!({
        "role": "assistant",
        "content": content,
        "tool_calls": tool_call_values
    })
}

fn insert_before_trailing_user_history_events(events: &mut Vec<Value>, event: Value) {
    let insert_at = events
        .iter()
        .rposition(|item| {
            !item
                .get("role")
                .and_then(Value::as_str)
                .is_some_and(|role| role.trim().eq_ignore_ascii_case("user"))
        })
        .map(|index| index + 1)
        .unwrap_or(0);
    events.insert(insert_at, event);
}

fn insert_before_trailing_user_messages(
    messages: &mut Vec<genai::chat::ChatMessage>,
    message: genai::chat::ChatMessage,
) {
    let insert_at = messages
        .iter()
        .rposition(|item| !matches!(item.role, genai::chat::ChatRole::User))
        .map(|index| index + 1)
        .unwrap_or(0);
    messages.insert(insert_at, message);
}

fn send_assistant_tool_event(
    on_delta: &tauri::ipc::Channel<AssistantDeltaEvent>,
    assistant_tool_event: &Value,
) {
    let _ = on_delta.send(AssistantDeltaEvent {
        delta: String::new(),
        kind: Some("assistant_tool_event".to_string()),
        request_id: None,
        activation_id: None,
        phase_id: None,
        reason: None,
        tool_name: None,
        tool_call_id: None,
        tool_status: None,
        tool_args: None,
        message: Some(assistant_tool_event.to_string()),
        stream_cache: None,
    });
}

fn send_assistant_tool_result_event(
    on_delta: &tauri::ipc::Channel<AssistantDeltaEvent>,
    tool_result_event: &Value,
) {
    let _ = on_delta.send(AssistantDeltaEvent {
        delta: String::new(),
        kind: Some("assistant_tool_result".to_string()),
        request_id: None,
        activation_id: None,
        phase_id: None,
        reason: None,
        tool_name: None,
        tool_call_id: None,
        tool_status: None,
        tool_args: None,
        message: Some(tool_result_event.to_string()),
        stream_cache: None,
    });
}

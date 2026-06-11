const GOAL_STATUS_ACTIVE: &str = "active";
const GOAL_STATUS_COMPLETE: &str = "complete";
const GOAL_STATUS_BLOCKED: &str = "blocked";
const GOAL_STATUS_CANCELLED_BY_USER: &str = "cancelled_by_user";
const GOAL_UPDATED_EVENT: &str = "easy-call:conversation-goal-updated";
const GOAL_CONTINUATION_PROMPT_TEMPLATE: &str =
    include_str!("../../resources/prompts/goal-continuation.md");

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct GoalCreateInput {
    conversation_id: String,
    objective: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct GoalCancelInput {
    conversation_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GoalUsageDelta {
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GoalMutationOutput {
    conversation_id: String,
    goal: ConversationGoalState,
    usage_delta: GoalUsageDelta,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateGoalToolArgs {
    objective: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct UpdateGoalToolArgs {
    status: String,
    #[serde(default)]
    evidence: Option<String>,
    #[serde(default, alias = "blockingCondition")]
    blocking_condition: Option<String>,
}

fn goal_usage_delta(
    start: &ConversationCumulativeUsage,
    end: &ConversationCumulativeUsage,
) -> GoalUsageDelta {
    GoalUsageDelta {
        input_tokens: end.input_tokens.saturating_sub(start.input_tokens),
        output_tokens: end.output_tokens.saturating_sub(start.output_tokens),
        cache_read_tokens: end.cache_read_tokens.saturating_sub(start.cache_read_tokens),
        cache_write_tokens: end.cache_write_tokens.saturating_sub(start.cache_write_tokens),
    }
}

fn goal_blocked_turn_threshold_met(
    conversation: &Conversation,
    goal: &ConversationGoalState,
) -> bool {
    goal_continue_turn_for_conversation(conversation, &goal.goal_id)
        .saturating_sub(1)
        >= 3
}

fn goal_output(conversation_id: &str, goal: ConversationGoalState) -> GoalMutationOutput {
    let usage_end = goal
        .usage_end
        .as_ref()
        .unwrap_or(&goal.usage_start)
        .clone();
    GoalMutationOutput {
        conversation_id: conversation_id.to_string(),
        usage_delta: goal_usage_delta(&goal.usage_start, &usage_end),
        goal,
    }
}

fn emit_goal_updated(state: &AppState, conversation_id: &str, goal: Option<&ConversationGoalState>) {
    let payload = serde_json::json!({
        "conversationId": conversation_id,
        "goal": goal,
    });
    ide_chat_broadcast_notification("conversation.goalUpdated", payload.clone());
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    if let Some(app_handle) = app_handle {
        let _ = app_handle.emit(GOAL_UPDATED_EVENT, payload);
    }
}

fn goal_active_goal_from_conversation(
    conversation: &Conversation,
) -> Option<ConversationGoalState> {
    conversation
        .active_goal
        .as_ref()
        .filter(|goal| conversation_goal_is_active(goal))
        .cloned()
}

fn goal_get_current_inner(
    state: &AppState,
    conversation_id: &str,
) -> Result<Option<ConversationGoalState>, String> {
    let normalized_conversation_id = conversation_id.trim();
    if normalized_conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let conversation = goal_read_conversation(state, normalized_conversation_id)?;
    Ok(goal_active_goal_from_conversation(&conversation))
}

fn goal_read_conversation(state: &AppState, conversation_id: &str) -> Result<Conversation, String> {
    match state_read_conversation_cached(state, conversation_id) {
        Ok(conversation) => Ok(conversation),
        Err(main_err) => match delegate_runtime_thread_conversation_get(state, conversation_id) {
            Ok(Some(conversation)) => Ok(conversation),
            Ok(None) => Err(main_err),
            Err(delegate_err) => Err(format!("{main_err}; delegate={delegate_err}")),
        },
    }
}

fn goal_update_conversation_metadata<T>(
    state: &AppState,
    conversation_id: &str,
    updater: impl FnOnce(&mut Conversation) -> Result<T, String>,
) -> Result<(Conversation, T), String> {
    if state_read_conversation_cached(state, conversation_id).is_ok() {
        let (conversation, result, _) =
            state_update_conversation_metadata_cached(state, conversation_id, updater)?;
        return Ok((conversation, result));
    }
    let mut conversation = delegate_runtime_thread_conversation_get(state, conversation_id)?
        .ok_or_else(|| format!("Conversation not found: {conversation_id}"))?;
    let result = updater(&mut conversation)?;
    delegate_runtime_thread_conversation_update(state, conversation_id, conversation.clone())?;
    Ok((conversation, result))
}

fn goal_create_goal_inner(
    state: &AppState,
    conversation_id: &str,
    objective: &str,
) -> Result<GoalMutationOutput, String> {
    let normalized_conversation_id = conversation_id.trim();
    if normalized_conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let objective = objective.trim();
    if objective.is_empty() {
        return Err("goal.objective is required".to_string());
    }
    let guard = state
        .conversation_lock
        .lock()
        .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
    let now = now_iso();
    let (conversation, goal) = goal_update_conversation_metadata(
        state,
        normalized_conversation_id,
        |conversation| {
            if conversation
                .active_goal
                .as_ref()
                .map(conversation_goal_is_active)
                .unwrap_or(false)
            {
                return Err("当前会话已有 active goal，不能覆盖。".to_string());
            }
            let goal = ConversationGoalState {
                goal_id: format!("goal-{}", Uuid::new_v4()),
                status: GOAL_STATUS_ACTIVE.to_string(),
                objective: objective.to_string(),
                started_at: now.clone(),
                ended_at: None,
                usage_start: conversation.cumulative_usage.clone(),
                usage_end: None,
            };
            conversation.active_goal = Some(goal.clone());
            conversation.updated_at = now.clone();
            Ok(goal)
        },
    )?;
    drop(guard);
    emit_goal_updated(state, normalized_conversation_id, conversation.active_goal.as_ref());
    clear_goal_continue_suppression(state, normalized_conversation_id, "goal_created")?;
    if let Err(err) = maybe_enqueue_goal_continue_after_idle(state, normalized_conversation_id) {
        runtime_log_warn(format!(
            "[目标续跑] 跳过，任务=创建目标后投递续跑，conversation_id={}，goal_id={}，error={}",
            normalized_conversation_id,
            goal.goal_id,
            err
        ));
    }
    Ok(goal_output(normalized_conversation_id, goal))
}

fn goal_update_terminal_inner(
    state: &AppState,
    conversation_id: &str,
    status: &str,
    evidence: Option<&str>,
    blocking_condition: Option<&str>,
) -> Result<GoalMutationOutput, String> {
    let normalized_conversation_id = conversation_id.trim();
    if normalized_conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let normalized_status = status.trim();
    match normalized_status {
        GOAL_STATUS_COMPLETE => {
            if evidence.map(str::trim).filter(|value| !value.is_empty()).is_none() {
                return Err("update_goal complete requires non-empty evidence".to_string());
            }
        }
        GOAL_STATUS_BLOCKED => {
            if blocking_condition
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_none()
            {
                return Err("update_goal blocked requires non-empty blocking_condition".to_string());
            }
        }
        _ => {
            return Err("update_goal.status must be complete or blocked".to_string());
        }
    }
    let guard = state
        .conversation_lock
        .lock()
        .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
    let now = now_iso();
    let (conversation, goal) = goal_update_conversation_metadata(
        state,
        normalized_conversation_id,
        |conversation| {
            let mut goal = conversation
                .active_goal
                .clone()
                .filter(conversation_goal_is_active)
                .ok_or_else(|| "当前会话没有 active goal。".to_string())?;
            if normalized_status == GOAL_STATUS_BLOCKED
                && !goal_blocked_turn_threshold_met(conversation, &goal)
            {
                return Err("update_goal blocked requires at least three goal continuation turns for the same active goal".to_string());
            }
            goal.status = normalized_status.to_string();
            goal.ended_at = Some(now.clone());
            goal.usage_end = Some(conversation.cumulative_usage.clone());
            conversation.active_goal = Some(goal.clone());
            conversation.updated_at = now.clone();
            Ok(goal)
        },
    )?;
    drop(guard);
    emit_goal_updated(state, normalized_conversation_id, conversation.active_goal.as_ref());
    Ok(goal_output(normalized_conversation_id, goal))
}

fn goal_cancel_goal_inner(
    state: &AppState,
    conversation_id: &str,
) -> Result<GoalMutationOutput, String> {
    let normalized_conversation_id = conversation_id.trim();
    if normalized_conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let guard = state
        .conversation_lock
        .lock()
        .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
    let now = now_iso();
    let (conversation, goal) = goal_update_conversation_metadata(
        state,
        normalized_conversation_id,
        |conversation| {
            let mut goal = conversation
                .active_goal
                .clone()
                .filter(conversation_goal_is_active)
                .ok_or_else(|| "当前会话没有 active goal。".to_string())?;
            goal.status = GOAL_STATUS_CANCELLED_BY_USER.to_string();
            goal.ended_at = Some(now.clone());
            goal.usage_end = Some(conversation.cumulative_usage.clone());
            conversation.active_goal = Some(goal.clone());
            conversation.updated_at = now.clone();
            Ok(goal)
        },
    )?;
    drop(guard);
    emit_goal_updated(state, normalized_conversation_id, conversation.active_goal.as_ref());
    Ok(goal_output(normalized_conversation_id, goal))
}

fn goal_tool_conversation_id(session_id: &str) -> Result<String, String> {
    let (_, _, conversation_id) = delegate_parse_session_parts(session_id);
    conversation_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "缺少当前工具调用会话 ID，无法操作 goal。".to_string())
}

fn goal_create_for_session(
    state: &AppState,
    session_id: &str,
    args: CreateGoalToolArgs,
) -> Result<Value, String> {
    let conversation_id = goal_tool_conversation_id(session_id)?;
    let output = goal_create_goal_inner(state, &conversation_id, &args.objective)?;
    serde_json::to_value(output).map_err(|err| format!("序列化 goal 创建结果失败: {err}"))
}

fn goal_update_for_session(
    state: &AppState,
    session_id: &str,
    args: UpdateGoalToolArgs,
) -> Result<Value, String> {
    let conversation_id = goal_tool_conversation_id(session_id)?;
    let output = goal_update_terminal_inner(
        state,
        &conversation_id,
        &args.status,
        args.evidence.as_deref(),
        args.blocking_condition.as_deref(),
    )?;
    serde_json::to_value(output).map_err(|err| format!("序列化 goal 更新结果失败: {err}"))
}

fn render_goal_continuation_prompt(objective: &str) -> String {
    GOAL_CONTINUATION_PROMPT_TEMPLATE.replace(
        "{{ objective }}",
        &xml_escape_prompt(objective.trim()),
    )
}

#[tauri::command]
fn goal_get_current(
    conversation_id: String,
    state: State<'_, AppState>,
) -> Result<Option<ConversationGoalState>, String> {
    goal_get_current_inner(&state, &conversation_id)
}

#[tauri::command]
fn goal_create_goal(
    input: GoalCreateInput,
    state: State<'_, AppState>,
) -> Result<GoalMutationOutput, String> {
    goal_create_goal_inner(&state, &input.conversation_id, &input.objective)
}

#[tauri::command]
fn goal_cancel_goal(
    input: GoalCancelInput,
    state: State<'_, AppState>,
) -> Result<GoalMutationOutput, String> {
    goal_cancel_goal_inner(&state, &input.conversation_id)
}

#[cfg(test)]
mod goal_tests {
    use super::*;

    fn goal_continue_test_message(goal_id: &str, turn: usize) -> ChatMessage {
        ChatMessage {
            id: format!("goal-message-{turn}"),
            role: "user".to_string(),
            created_at: "2026-06-11T00:00:00Z".to_string(),
            speaker_agent_id: None,
            parts: vec![MessagePart::Text {
                text: "继续推进当前目标。".to_string(),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: Some(serde_json::json!({
                "messageKind": "goal_continue",
                "goalId": goal_id,
                "goalTurn": turn,
            })),
            tool_call: None,
            mcp_call: None,
        }
    }

    #[test]
    fn goal_usage_delta_should_saturate() {
        let start = ConversationCumulativeUsage {
            input_tokens: 10,
            output_tokens: 20,
            cache_read_tokens: 30,
            cache_write_tokens: 40,
        };
        let end = ConversationCumulativeUsage {
            input_tokens: 15,
            output_tokens: 18,
            cache_read_tokens: 35,
            cache_write_tokens: 60,
        };
        let delta = goal_usage_delta(&start, &end);
        assert_eq!(delta.input_tokens, 5);
        assert_eq!(delta.output_tokens, 0);
        assert_eq!(delta.cache_read_tokens, 5);
        assert_eq!(delta.cache_write_tokens, 20);
    }

    #[test]
    fn render_goal_continuation_prompt_should_escape_objective() {
        let rendered = render_goal_continuation_prompt("完成 <tag> & \"quote\"");
        assert!(rendered.contains("&lt;tag&gt;"));
        assert!(rendered.contains("&amp;"));
        assert!(!rendered.contains("完成 <tag>"));
    }

    #[test]
    fn build_goal_continue_message_should_use_system_persona_and_hidden_prompt() {
        let goal = ConversationGoalState {
            goal_id: "goal-message-shape".to_string(),
            status: GOAL_STATUS_ACTIVE.to_string(),
            objective: "完成目标".to_string(),
            started_at: "2026-06-11T00:00:00Z".to_string(),
            ended_at: None,
            usage_start: ConversationCumulativeUsage::default(),
            usage_end: None,
        };
        let message = build_goal_continue_message(
            &goal,
            2,
            "隐藏的完整续跑提示".to_string(),
            "2026-06-11T00:00:00Z".to_string(),
        );

        assert_eq!(message.role, "system");
        assert_eq!(message.speaker_agent_id.as_deref(), Some(SYSTEM_PERSONA_ID));
        let first_text = match message.parts.first() {
            Some(MessagePart::Text { text, .. }) => text.as_str(),
            _ => "",
        };
        assert_eq!(first_text, GOAL_CONTINUE_DISPLAY_TEXT);
        let meta = message.provider_meta.as_ref().expect("provider meta");
        assert_eq!(meta.get("messageKind").and_then(Value::as_str), Some("goal_continue"));
        assert_eq!(
            meta.get("hiddenPromptText").and_then(Value::as_str),
            Some("隐藏的完整续跑提示")
        );
    }

    #[test]
    fn prompt_role_for_goal_continue_system_message_should_feed_model_as_user() {
        let goal = ConversationGoalState {
            goal_id: "goal-prompt-role".to_string(),
            status: GOAL_STATUS_ACTIVE.to_string(),
            objective: "完成目标".to_string(),
            started_at: "2026-06-11T00:00:00Z".to_string(),
            ended_at: None,
            usage_start: ConversationCumulativeUsage::default(),
            usage_end: None,
        };
        let message = build_goal_continue_message(
            &goal,
            1,
            "继续推进当前目标。".to_string(),
            "2026-06-11T00:00:00Z".to_string(),
        );

        assert_eq!(
            prompt_role_for_message(&message, DEFAULT_AGENT_ID).as_deref(),
            Some("user")
        );
    }

    #[test]
    fn goal_blocked_threshold_should_require_three_goal_continue_messages() {
        let goal = ConversationGoalState {
            goal_id: "goal-threshold".to_string(),
            status: GOAL_STATUS_ACTIVE.to_string(),
            objective: "验证 blocked 门槛".to_string(),
            started_at: "2026-06-11T00:00:00Z".to_string(),
            ended_at: None,
            usage_start: ConversationCumulativeUsage::default(),
            usage_end: None,
        };
        let mut conversation = build_conversation_record(
            "",
            DEFAULT_AGENT_ID,
            ASSISTANT_DEPARTMENT_ID,
            "blocked 门槛",
            CONVERSATION_KIND_CHAT,
            None,
            None,
        );
        conversation.active_goal = Some(goal.clone());
        conversation.messages = vec![
            goal_continue_test_message(&goal.goal_id, 1),
            goal_continue_test_message(&goal.goal_id, 2),
        ];

        assert!(!goal_blocked_turn_threshold_met(&conversation, &goal));
        conversation
            .messages
            .push(goal_continue_test_message(&goal.goal_id, 3));
        assert!(goal_blocked_turn_threshold_met(&conversation, &goal));
    }
}

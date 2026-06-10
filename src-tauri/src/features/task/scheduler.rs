fn task_conversation_available_for_dispatch(conversation: &Conversation) -> bool {
    conversation.summary.trim().is_empty()
        && !conversation_is_delegate(conversation)
        && !conversation_is_system_notification(conversation)
}

#[derive(Debug, Clone)]
struct TaskResolvedConversation {
    conversation_id: String,
    target_scope: String,
    system_task: bool,
}

#[derive(Debug, Clone)]
struct TaskDispatchSessionResolved {
    model_config_id: String,
    department_id: String,
    agent_id: String,
    conversation_id: String,
    target_scope: String,
    system_task: bool,
}

#[derive(Debug, Clone)]
struct TaskDispatchCandidate {
    task: TaskRecordStored,
    session: TaskDispatchSessionResolved,
}

#[derive(Debug, Clone)]
struct TaskDispatchSkipContext {
    request_id: String,
    dispatch_id: String,
    task_goal: String,
    conversation_id: String,
    trigger_label: String,
    todo_count: usize,
    has_run_at: bool,
    cron_expression: String,
    duration_ms: u128,
    target_scope: String,
    system_task: bool,
}

fn task_scope_for_conversation(conversation: &Conversation) -> &'static str {
    if conversation_is_remote_im_contact(conversation) {
        TASK_TARGET_SCOPE_CONTACT
    } else {
        TASK_TARGET_SCOPE_DESKTOP
    }
}

fn task_resolve_dispatch_conversation(
    state: &AppState,
    requested_conversation_id: Option<&str>,
) -> Result<Option<TaskResolvedConversation>, String> {
    let requested = task_normalize_bound_conversation_id(requested_conversation_id);
    if task_conversation_id_is_system_notification(&requested) {
        task_ensure_system_notification_conversation(state)?;
        return Ok(Some(TaskResolvedConversation {
            conversation_id: SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string(),
            target_scope: TASK_TARGET_SCOPE_DESKTOP.to_string(),
            system_task: true,
        }));
    }

    if let Ok(conversation) = state_read_conversation_cached(state, &requested) {
        if task_conversation_available_for_dispatch(&conversation) {
            return Ok(Some(TaskResolvedConversation {
                conversation_id: conversation.id.clone(),
                target_scope: task_scope_for_conversation(&conversation).to_string(),
                system_task: false,
            }));
        }
    }
    Ok(None)
}

fn task_resolve_dispatch_session(
    state: &AppState,
    task: &TaskRecordStored,
) -> Result<Option<TaskDispatchSessionResolved>, String> {
    let requested_conversation_id = task
        .conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let resolved = task_resolve_dispatch_conversation(state, requested_conversation_id)?;
    let Some(resolved) = resolved else {
        return Ok(None);
    };
    let existing_department_id = task
        .department_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let existing_agent_id = task
        .agent_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let (department_id, agent_id) = if let Some(department_id) = existing_department_id {
        task_resolve_stored_department_agent_for_dispatch(state, department_id, existing_agent_id)?
    } else if let Some(agent_id) = existing_agent_id {
        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let department = runtime_department_for_agent(&runtime_snapshot, agent_id)
            .ok_or_else(|| format!("任务绑定人格缺少所属部门：agentId={agent_id}"))?;
        task_validate_department_agent_for_write(state, &department.id, agent_id)?
    } else if resolved.system_task {
        task_default_department_agent_for_write(state)?
    } else if let Some(pair) =
        task_department_agent_from_conversation_for_write(state, &resolved.conversation_id)?
    {
        pair
    } else {
        task_default_department_agent_for_write(state)?
    };
    let runtime_snapshot = load_runtime_organization_snapshot(state)?;
    let app_config = runtime_snapshot.config.clone();
    let department = runtime_department_by_id(&runtime_snapshot, &department_id)
        .ok_or_else(|| format!("任务绑定部门不存在：{department_id}"))?;
    let model_config_id = department_primary_chat_api_config_id(&app_config, department)
        .ok_or_else(|| format!("任务绑定部门没有可用模型：{department_id}"))?;
    Ok(Some(TaskDispatchSessionResolved {
        model_config_id,
        department_id,
        agent_id,
        conversation_id: resolved.conversation_id,
        target_scope: resolved.target_scope,
        system_task: resolved.system_task,
    }))
}

fn task_dispatch_block_reason(
    state: &AppState,
    conversation_id: &str,
) -> Result<Option<&'static str>, String> {
    let _dequeue_guard = state
        .dequeue_lock
        .lock()
        .map_err(|_| "Failed to lock dequeue lock".to_string())?;
    let claims = lock_conversation_processing_claims(state)?;
    let slots = lock_conversation_runtime_slots(state)?;
    let running_count = claims.len();
    let slot = slots.get(conversation_id);
    if slot.map(|item| item.state != MainSessionState::Idle).unwrap_or(false) {
        return Ok(Some("conversation_busy"));
    }
    if slot
        .map(|item| !item.pending_queue.is_empty())
        .unwrap_or(false)
    {
        return Ok(Some("conversation_queue_not_empty"));
    }
    if conversation_running_slot_count(&claims, conversation_id) > 0 {
        return Ok(Some("conversation_busy"));
    }
    if running_count >= CHAT_CONCURRENCY_LIMIT {
        return Ok(Some("chat_concurrency_limit"));
    }
    Ok(None)
}

fn task_trigger_label(task: &TaskRecordStored) -> &'static str {
    if task
        .trigger
        .cron_expression
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
    {
        "cron"
    } else if task.trigger.legacy_every_minutes.is_some() {
        "legacy_every_minutes"
    } else {
        "once"
    }
}

fn task_dispatch_todo_count(task: &TaskRecordStored) -> usize {
    task_legacy_todos_from_todo(&task_todo_from_legacy_fields(&task.status_summary, &task.todos)).len()
}

fn task_complete_one_time_dispatch_if_needed(
    state: &AppState,
    task: &TaskRecordStored,
) -> Result<(), String> {
    if !task_record_is_one_time(task) {
        return Ok(());
    }
    let changed = task_store_complete_one_time_dispatch(
        &state.data_path,
        &task.task_id,
    )?;
    if changed {
        runtime_log_info(format!(
            "[任务调度] 完成，任务=一次性任务已发起调度，task_id={}",
            task.task_id
        ));
    }
    Ok(())
}

fn task_mark_dispatch_sent(state: &AppState, task: &TaskRecordStored) -> Result<(), String> {
    if task_record_is_one_time(task) {
        return Ok(());
    }
    task_store_mark_triggered(&state.data_path, &task.task_id)
}

fn task_mark_dispatch_skipped(
    state: &AppState,
    task: &TaskRecordStored,
    reason: &str,
    context: &TaskDispatchSkipContext,
) -> Result<(), String> {
    task_store_mark_skipped(
        &state.data_path,
        &task.task_id,
        "skipped",
        &format!(
            "任务已跳过，requestId={}，dispatchId={}，goal={}，conversationId={}，trigger={}，todoCount={}，hasRunAt={}，cronExpression={}，durationMs={}，targetScope={}，systemTask={}，reason={}",
            context.request_id,
            context.dispatch_id,
            context.task_goal.trim(),
            context.conversation_id,
            context.trigger_label,
            context.todo_count,
            context.has_run_at,
            context.cron_expression,
            context.duration_ms,
            context.target_scope,
            context.system_task,
            reason
        ),
    )?;
    task_complete_one_time_dispatch_if_needed(state, task)
}

fn task_fail_missing_bound_conversation(
    state: &AppState,
    task: &TaskRecordStored,
) -> Result<(), String> {
    let conversation_id = task
        .conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(SYSTEM_NOTIFICATION_CONVERSATION_ID);
    let changed = task_store_fail_active_task(
        &state.data_path,
        &task.task_id,
        TASK_BOUND_CONVERSATION_MISSING_CONCLUSION,
        &format!(
            "绑定会话不存在，任务已失败，taskId={}，conversationId={}",
            task.task_id,
            conversation_id
        ),
    )?;
    if changed {
        runtime_log_info(format!(
            "[任务调度] 失败，任务=绑定会话丢失，task_id={}，conversation_id={}",
            task.task_id,
            conversation_id
        ));
    }
    Ok(())
}

fn task_fail_unavailable_owner(
    state: &AppState,
    task: &TaskRecordStored,
    reason: &str,
) -> Result<(), String> {
    let changed = task_store_fail_active_task(
        &state.data_path,
        &task.task_id,
        TASK_BOUND_OWNER_UNAVAILABLE_CONCLUSION,
        &format!(
            "任务负责人不可用，任务已失败，taskId={}，reason={}",
            task.task_id,
            reason.trim()
        ),
    )?;
    if changed {
        runtime_log_info(format!(
            "[任务调度] 失败，任务=负责人不可用，task_id={}，reason={}",
            task.task_id,
            reason.trim()
        ));
    }
    Ok(())
}

fn task_try_ingress_chat_event_direct(
    state: &AppState,
    event: ChatPendingEvent,
) -> Result<Result<ChatPendingEvent, &'static str>, String> {
    let _dequeue_guard = state
        .dequeue_lock
        .lock()
        .map_err(|_| "Failed to lock dequeue lock".to_string())?;
    let mut claims = lock_conversation_processing_claims(state)?;
    let mut slots = lock_conversation_runtime_slots(state)?;
    let running_count = claims.len();
    let slot = conversation_slot_mut(&mut slots, &event.conversation_id);
    if slot.state != MainSessionState::Idle {
        return Ok(Err("conversation_busy"));
    }
    if !slot.pending_queue.is_empty() {
        return Ok(Err("conversation_queue_not_empty"));
    }
    if conversation_running_slot_count(&claims, &event.conversation_id) > 0 {
        return Ok(Err("conversation_busy"));
    }
    if running_count >= CHAT_CONCURRENCY_LIMIT {
        return Ok(Err("chat_concurrency_limit"));
    }
    slot.last_activity_at = now_iso();
    claims.insert(event.conversation_id.clone());
    Ok(Ok(event))
}

fn task_conversation_last_message_is_system_persona(
    state: &AppState,
    conversation_id: &str,
) -> Result<bool, String> {
    let conversation = state_read_conversation_cached(state, conversation_id)?;
    Ok(conversation
        .messages
        .last()
        .and_then(|message| message.speaker_agent_id.as_deref())
        .map(str::trim)
        == Some(SYSTEM_PERSONA_ID))
}

fn task_is_due(entry: &TaskRecordStored, now: OffsetDateTime) -> bool {
    if entry.completion_state != TASK_STATE_ACTIVE {
        return false;
    }
    entry
        .trigger
        .next_run_at_utc
        .as_deref()
        .and_then(parse_rfc3339_time)
        .map(|next_run_at| now >= next_run_at)
        .unwrap_or(false)
}

fn task_build_board_snapshot(data_path: &PathBuf) -> Result<TaskBoardSnapshot, String> {
    let tasks = task_store_list_tasks(data_path)?;
    Ok(TaskBoardSnapshot {
        tasks: tasks
            .into_iter()
            .filter(|item| {
                item.completion_state == TASK_STATE_ACTIVE
                    && item
                        .conversation_id
                        .as_deref()
                        .map(task_conversation_id_is_system_notification)
                        != Some(true)
            })
            .take(TASK_MAX_BOARD_ITEMS)
            .collect(),
    })
}

fn build_hidden_task_board_block(state: &AppState) -> Option<String> {
    let snapshot = task_build_board_snapshot(&state.data_path).ok()?;
    if snapshot.tasks.is_empty() {
        return None;
    }
    let mut lines = Vec::<String>::new();
    lines.push(format!("currentLocalTime: {}", now_local_rfc3339()));
    lines.push("timeFormatNote: all task times below use local RFC3339 with timezone offset; copy the same format directly when writing run_at or end_at".to_string());
    lines.push(format!("activeTaskCount: {}", snapshot.tasks.len()));
    for (idx, task) in snapshot.tasks.iter().enumerate() {
        let task_no = idx + 1;
        lines.push(format!("task[{task_no}].id: {}", task.task_id));
        lines.push(format!("task[{task_no}].goal: {}", task.goal.trim()));
        if !task.todo.trim().is_empty() {
            lines.push(format!("task[{task_no}].how: {}", task.todo.trim()));
        }
        if !task.why.trim().is_empty() {
            lines.push(format!("task[{task_no}].why: {}", task.why.trim()));
        }
        if let Some(run_at) = task.trigger.run_at.as_deref() {
            lines.push(format!("task[{task_no}].run_at: {}", run_at));
        }
        if let Some(cron_expression) = task.trigger.cron_expression.as_deref() {
            lines.push(format!("task[{task_no}].cron_expression: {}", cron_expression));
        }
        if let Some(end_at) = task.trigger.end_at.as_deref() {
            lines.push(format!("task[{task_no}].end_at: {}", end_at));
        }
        if let Some(next_run_at) = task.trigger.next_run_at.as_deref() {
            lines.push(format!("task[{task_no}].next_run_at: {}", next_run_at));
        }
    }
    Some(prompt_xml_block("task board", lines.join("\n")))
}

fn build_task_trigger_hidden_prompt(task: &TaskRecordStored) -> String {
    let goal = task_goal_from_legacy_fields(&task.title, &task.goal);
    let why = task_why_from_legacy_record(task);
    let todo = task_todo_from_legacy_fields(&task.status_summary, &task.todos);
    let lines = if why.trim().is_empty() {
        vec![
            "背景：用户希望你能独立完成任务达成目标".to_string(),
            format!("目标：{}", goal.trim()),
            "要求：一直持续工作，直到达成目标，最后在当前会话进行工作汇报。".to_string(),
        ]
    } else {
        vec![
            format!("背景：{}", why.trim()),
            format!("目标：{}", goal.trim()),
            format!("要求：{}", todo.trim()),
        ]
    };
    format!("<task_remind>\n{}\n</task_remind>", lines.join("\n"))
}

fn build_task_trigger_provider_meta(task: &TaskRecordStored) -> Value {
    let trigger_view = task_trigger_view_from_stored(&task.trigger);
    let goal = task_goal_from_legacy_fields(&task.title, &task.goal);
    let why = task_why_from_legacy_record(task);
    let todo = task_todo_from_legacy_fields(&task.status_summary, &task.todos);
    serde_json::json!({
        "messageKind": "task_trigger",
        "hiddenPromptText": build_task_trigger_hidden_prompt(task),
        "taskTrigger": {
            "taskId": task.task_id,
            "goal": goal.trim(),
            "how": todo.trim(),
            "why": why.trim(),
            "run_at": trigger_view.run_at,
            "cron_expression": trigger_view.cron_expression,
            "end_at": trigger_view.end_at,
            "next_run_at": trigger_view.next_run_at,
        }
    })
}

async fn task_dispatch_due_task(
    state: &AppState,
    task: &TaskRecordStored,
    session: &TaskDispatchSessionResolved,
) -> Result<(), String> {
    let started_at = std::time::Instant::now();
    task_complete_one_time_dispatch_if_needed(state, task)?;

    if let Some(requested) = task
        .conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        eprintln!(
            "[任务调度] 会话{}的任务{}，投递中，requested_conversation_id={}",
            session.conversation_id,
            task.task_id,
            requested
        );
    }

    // 构造任务消息
    let task_message = ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "user".to_string(),
        created_at: now_iso(),
        speaker_agent_id: Some(SYSTEM_PERSONA_ID.to_string()),
        parts: vec![MessagePart::Text {
            text: build_task_trigger_hidden_prompt(task),
            reasoning_content: None,
        }],
        extra_text_blocks: Vec::new(),
        provider_meta: Some(build_task_trigger_provider_meta(task)),
        tool_call: None,
        mcp_call: None,
    };

    // 创建事件并入队
    let event_id = Uuid::new_v4().to_string();
    let request_id = format!("task-dispatch-{}", Uuid::new_v4());
    let mut runtime_context = runtime_context_new("task_trigger", "task_due");
    runtime_context.request_id = Some(request_id.clone());
    runtime_context.dispatch_id = Some(event_id.clone());
    runtime_context.origin_conversation_id = task
        .conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    runtime_context.target_conversation_id = Some(session.conversation_id.clone());
    runtime_context.root_conversation_id = runtime_context
        .origin_conversation_id
        .clone()
        .or_else(|| Some(session.conversation_id.clone()));
    runtime_context.executor_agent_id = Some(session.agent_id.clone());
    runtime_context.executor_department_id = Some(session.department_id.clone());
    runtime_context.model_config_id = Some(session.model_config_id.clone());
    let event = ChatPendingEvent {
        id: event_id.clone(),
        conversation_id: session.conversation_id.clone(),
        created_at: now_iso(),
        source: ChatEventSource::Task,
        queue_mode: ChatQueueMode::Normal,
        messages: vec![task_message],
        activate_assistant: true,
        session_info: ChatSessionInfo {
            department_id: session.department_id.clone(),
            agent_id: session.agent_id.clone(),
        },
        runtime_context: Some(runtime_context.clone()),
        sender_info: None,
    };

    let trigger_label = task_trigger_label(task);
    let todo_count = task_dispatch_todo_count(task);
    let task_goal = task_goal_from_legacy_fields(&task.title, &task.goal);

    match task_try_ingress_chat_event_direct(state, event)? {
        Ok(event) => {
            task_mark_dispatch_sent(state, task)?;
            trigger_chat_event_after_ingress(state, ChatEventIngress::Direct(event));

            let duration_ms = started_at.elapsed().as_millis();
            task_store_insert_run_log(
                &state.data_path,
                &task.task_id,
                "sent",
                &format!(
                    "{}，requestId={}，dispatchId={}，goal={}，conversationId={}，trigger={}，todoCount={}，hasRunAt={}，cronExpression={}，durationMs={}，targetScope={}，systemTask={}",
                    "任务已发送",
                    request_id,
                    event_id,
                    task_goal.trim(),
                    session.conversation_id,
                    trigger_label,
                    todo_count,
                    task.trigger.run_at_utc.is_some(),
                    task.trigger.cron_expression.as_deref().unwrap_or(""),
                    duration_ms,
                    session.target_scope,
                    session.system_task
                ),
            )?;
            Ok(())
        }
        Err(reason) => {
            let duration_ms = started_at.elapsed().as_millis();
            task_mark_dispatch_skipped(state, task, reason, &TaskDispatchSkipContext {
                request_id,
                dispatch_id: event_id,
                task_goal,
                conversation_id: session.conversation_id.clone(),
                trigger_label: trigger_label.to_string(),
                todo_count,
                has_run_at: task.trigger.run_at_utc.is_some(),
                cron_expression: task.trigger.cron_expression.clone().unwrap_or_default(),
                duration_ms,
                target_scope: session.target_scope.clone(),
                system_task: session.system_task,
            })?;
            Ok(())
        }
    }
}

fn task_skip_context_for_candidate_filter(
    task: &TaskRecordStored,
    session: &TaskDispatchSessionResolved,
) -> TaskDispatchSkipContext {
    TaskDispatchSkipContext {
        request_id: "task-candidate-skip".to_string(),
        dispatch_id: format!("task-skip-{}", Uuid::new_v4()),
        task_goal: task_goal_from_legacy_fields(&task.title, &task.goal),
        conversation_id: session.conversation_id.clone(),
        trigger_label: task_trigger_label(task).to_string(),
        todo_count: task_dispatch_todo_count(task),
        has_run_at: task.trigger.run_at_utc.is_some(),
        cron_expression: task.trigger.cron_expression.clone().unwrap_or_default(),
        duration_ms: 0,
        target_scope: session.target_scope.clone(),
        system_task: session.system_task,
    }
}

fn task_build_dispatch_candidates(
    state: &AppState,
    tasks: Vec<TaskRecordStored>,
    now: OffsetDateTime,
) -> Result<Vec<TaskDispatchCandidate>, String> {
    let mut due_tasks = tasks
        .into_iter()
        .filter(|item| task_is_due(item, now))
        .collect::<Vec<_>>();
    due_tasks.sort_by_key(|item| item.order_index);

    let mut used_conversation_ids = std::collections::HashSet::<String>::new();
    let mut candidates = Vec::<TaskDispatchCandidate>::new();
    for task in due_tasks {
        let session = match task_resolve_dispatch_session(state, &task) {
            Ok(Some(session)) => session,
            Ok(None) => {
                task_fail_missing_bound_conversation(state, &task)?;
                continue;
            }
            Err(err) => {
                task_fail_unavailable_owner(state, &task, &err)?;
                continue;
            }
        };
        if let Some(reason) = task_dispatch_block_reason(state, &session.conversation_id)? {
            let context = task_skip_context_for_candidate_filter(&task, &session);
            task_mark_dispatch_skipped(state, &task, reason, &context)?;
            continue;
        }
        if task_conversation_last_message_is_system_persona(state, &session.conversation_id)? {
            let context = task_skip_context_for_candidate_filter(&task, &session);
            task_mark_dispatch_skipped(state, &task, "last_message_is_task_trigger", &context)?;
            continue;
        }
        if !used_conversation_ids.insert(session.conversation_id.clone()) {
            let context = task_skip_context_for_candidate_filter(&task, &session);
            task_mark_dispatch_skipped(state, &task, "same_conversation_already_selected", &context)?;
            continue;
        }
        candidates.push(TaskDispatchCandidate { task, session });
    }
    Ok(candidates)
}

async fn task_scheduler_tick(state: &AppState) -> Result<(), String> {
    let tasks = task_store_list_task_records(&state.data_path)?;
    let now = now_utc();
    let candidates = task_build_dispatch_candidates(state, tasks, now)?;
    for candidate in candidates {
        task_dispatch_due_task(state, &candidate.task, &candidate.session).await?;
    }
    Ok(())
}

fn start_task_scheduler(state: AppState) {
    tauri::async_runtime::spawn(async move {
        loop {
            let tick_started_at = std::time::Instant::now();
            if let Err(err) = task_scheduler_tick(&state).await {
                eprintln!(
                    "[任务调度] 调度轮询失败，error={}，durationMs={}，dataPath={}",
                    err,
                    tick_started_at.elapsed().as_millis(),
                    state.data_path.to_string_lossy()
                );
            }
            tokio::time::sleep(std::time::Duration::from_secs(TASK_SCHEDULER_INTERVAL_SECONDS)).await;
        }
    });
}

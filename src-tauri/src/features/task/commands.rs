#[tauri::command]
fn task_list_tasks(state: State<'_, AppState>) -> Result<Vec<TaskEntry>, String> {
    task_store_list_tasks(&state.data_path)
}

fn task_ensure_system_notification_conversation(state: &AppState) -> Result<(), String> {
    if state_read_conversation_cached(state, SYSTEM_NOTIFICATION_CONVERSATION_ID)
        .ok()
        .filter(|conversation| conversation_is_system_notification(conversation))
        .is_some()
    {
        return Ok(());
    }
    let conversation = build_system_notification_conversation_record();
    state_schedule_conversation_persist(state, &conversation)?;
    Ok(())
}

fn task_normalize_conversation_for_write(
    state: &AppState,
    conversation_id: Option<&str>,
) -> Result<String, String> {
    let normalized = task_normalize_bound_conversation_id(conversation_id);
    if task_conversation_id_is_system_notification(&normalized) {
        task_ensure_system_notification_conversation(state)?;
        return Ok(normalized);
    }
    let conversation = state_read_conversation_cached(state, &normalized)
        .map_err(|_| format!("绑定会话不存在：{normalized}"))?;
    if !conversation.summary.trim().is_empty() || conversation_is_delegate(&conversation) {
        return Err(format!("绑定会话不可用：{normalized}"));
    }
    Ok(normalized)
}

fn task_create_input_for_write(
    state: &AppState,
    input: &TaskCreateInput,
) -> Result<TaskCreateInput, String> {
    let mut next = input.clone();
    let conversation_id =
        task_normalize_conversation_for_write(state, input.conversation_id.as_deref())?;
    next.conversation_id = Some(conversation_id);
    Ok(next)
}

fn task_update_input_for_write(
    state: &AppState,
    input: &TaskUpdateInput,
) -> Result<TaskUpdateInput, String> {
    let existing = task_store_get_task_record(&state.data_path, input.task_id.trim())?;
    let mut next = input.clone();
    let conversation_id = task_normalize_conversation_for_write(
        state,
        input
            .conversation_id
            .as_deref()
            .or(existing.conversation_id.as_deref()),
    )?;
    next.conversation_id = Some(conversation_id);
    Ok(next)
}

#[tauri::command]
fn task_get_task(input: TaskGetInput, state: State<'_, AppState>) -> Result<TaskEntry, String> {
    task_store_get_task(&state.data_path, input.task_id.trim())
}

#[tauri::command]
fn task_create_task(input: TaskCreateInput, state: State<'_, AppState>) -> Result<TaskEntry, String> {
    let input = task_create_input_for_write(state.inner(), &input)?;
    task_store_create_task(&state.data_path, &input)
}

#[tauri::command]
async fn task_dispatch_task_now(input: TaskDispatchNowInput, state: State<'_, AppState>) -> Result<bool, String> {
    let task = task_store_get_task_record(&state.data_path, input.task_id.trim())?;
    let Some(session) = task_resolve_dispatch_session(&state, &task)? else {
        task_fail_missing_bound_conversation(state.inner(), &task)?;
        return Ok(false);
    };
    task_dispatch_due_task(&state, &task, &session).await?;
    Ok(true)
}

#[tauri::command]
fn task_update_task(input: TaskUpdateInput, state: State<'_, AppState>) -> Result<TaskEntry, String> {
    let input = task_update_input_for_write(state.inner(), &input)?;
    task_store_update_task(&state.data_path, &input)
}

#[tauri::command]
fn task_complete_task(input: TaskCompleteInput, state: State<'_, AppState>) -> Result<TaskEntry, String> {
    task_store_complete_task(&state.data_path, &input)
}

#[tauri::command]
fn task_delete_task(input: TaskDeleteInput, state: State<'_, AppState>) -> Result<(), String> {
    task_store_delete_task(&state.data_path, input.task_id.trim())
}

#[tauri::command]
fn task_list_run_logs(
    input: Option<TaskRunLogListInput>,
    state: State<'_, AppState>,
) -> Result<Vec<TaskRunLogEntry>, String> {
    let payload = input.unwrap_or(TaskRunLogListInput {
        task_id: None,
        limit: Some(50),
    });
    task_store_list_run_logs(
        &state.data_path,
        payload.task_id.as_deref(),
        payload.limit.unwrap_or(50),
    )
}

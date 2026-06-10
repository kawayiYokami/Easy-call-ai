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

fn task_optimize_draft_prompt(input: &TaskOptimizeDraftInput) -> Result<String, String> {
    let content = input.content.trim();
    if content.is_empty() {
        return Err("任务内容不能为空。".to_string());
    }
    let title = input.title.trim();
    let schedule_mode = task_optimize_schedule_mode(&input.schedule_mode);
    let repeat_unit = task_optimize_repeat_unit(&input.repeat_unit);
    let repeat_every = task_optimize_repeat_every(&input.repeat_every, repeat_unit, "1");
    Ok(format!(
        "你是任务草稿结构化助手。请根据用户草稿和当前界面设置，整理出可以直接保存的任务草稿。\n\
当前本地时间：{}\n\n\
当前界面设置：\n\
- title: {}\n\
- content: {}\n\
- scheduleMode: {}\n\
- runAt: {}\n\
- repeatEvery: {}\n\
- repeatUnit: {}\n\
- endAt: {}\n\n\
规则：\n\
1. 输出只允许是 JSON 对象，不要 Markdown，不要解释。\n\
2. 字段固定为 title, content, scheduleMode, runAt, repeatEvery, repeatUnit, endAt。\n\
3. scheduleMode 只能是 once 或 interval。\n\
4. runAt 和 endAt 必须是带时区偏移的本地 RFC3339，例如 2026-06-10T17:00:00+08:00；没有结束时间时 endAt 返回空字符串。\n\
5. repeatUnit 只能是 minutes, hours, days, weeks, months；repeatEvery 是正整数字符串。\n\
6. 如果 content 里明确说了时间或频率，以 content 为准并回填调度字段。比如“明天下午5点叫我起床”应返回 scheduleMode=once，runAt=明天 17:00 的本地 RFC3339。\n\
7. 如果 content 里没有明确时间或频率，保留当前界面设置里的 scheduleMode/runAt/repeatEvery/repeatUnit/endAt。\n\
8. 如果是每天、每周、每月、每隔 N 小时/分钟/天等重复意图，返回 scheduleMode=interval，并设置首次触发 runAt 与 repeatEvery/repeatUnit。\n\
9. 如果只说日期没有时间，沿用当前 runAt 的时分；如果只说时间没有日期，取未来最近一次该时间。\n\
10. title 简短清楚，最多 30 个汉字或 60 个英文字符。\n\
11. content 写成到点触发后要执行/提醒的具体内容，去掉已经被结构化到调度字段里的时间口令，不改变用户意图，不添加新目标。\n\n\
返回示例：{{\"title\":\"叫我起床\",\"content\":\"提醒用户起床。\",\"scheduleMode\":\"once\",\"runAt\":\"2026-06-10T17:00:00+08:00\",\"repeatEvery\":\"1\",\"repeatUnit\":\"hours\",\"endAt\":\"\"}}\n\n\
用户草稿：\n{}",
        now_local_rfc3339(),
        if title.is_empty() { "（空）" } else { title },
        content,
        schedule_mode,
        input.run_at.trim(),
        repeat_every,
        repeat_unit,
        input.end_at.trim(),
        content
    ))
}

fn task_trim_chars(value: &str, limit: usize) -> String {
    let normalized = value.trim().replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::new();
    for (index, ch) in normalized.chars().enumerate() {
        if index >= limit {
            break;
        }
        out.push(ch);
    }
    out.trim().to_string()
}

fn task_optimize_draft_output_from_value(
    value: &Value,
    input: &TaskOptimizeDraftInput,
) -> Result<TaskOptimizeDraftOutput, String> {
    let title = value
        .get("title")
        .and_then(Value::as_str)
        .map(|item| task_trim_chars(item, 80))
        .filter(|item| !item.is_empty())
        .unwrap_or_else(|| task_trim_chars(&input.title, 80));
    let content = value
        .get("content")
        .and_then(Value::as_str)
        .map(|item| task_trim_chars(item, 4000))
        .filter(|item| !item.is_empty())
        .ok_or_else(|| "快速模型未返回有效任务内容".to_string())?;
    let schedule_mode = task_value_string_any(value, &["scheduleMode", "schedule_mode"])
        .map(|item| task_optimize_schedule_mode(&item))
        .unwrap_or_else(|| task_optimize_schedule_mode(&input.schedule_mode))
        .to_string();
    let run_at = task_optimize_run_at(
        task_value_string_any(value, &["runAt", "run_at"]).as_deref(),
        &input.run_at,
    );
    let repeat_unit = task_value_string_any(value, &["repeatUnit", "repeat_unit"])
        .map(|item| task_optimize_repeat_unit(&item))
        .unwrap_or_else(|| task_optimize_repeat_unit(&input.repeat_unit))
        .to_string();
    let repeat_every = task_optimize_repeat_every(
        task_value_string_any(value, &["repeatEvery", "repeat_every"]).as_deref().unwrap_or(""),
        &repeat_unit,
        &input.repeat_every,
    );
    let end_at = if schedule_mode == "interval" {
        task_optimize_end_at(
            task_value_string_any(value, &["endAt", "end_at"]).as_deref(),
            &input.end_at,
            &run_at,
        )
    } else {
        String::new()
    };
    Ok(TaskOptimizeDraftOutput {
        title,
        content,
        schedule_mode,
        run_at,
        repeat_every,
        repeat_unit,
        end_at,
    })
}

fn task_value_string_any(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
}

fn task_optimize_schedule_mode(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "interval" | "recurring" | "repeat" | "repeating" | "定时" | "重复" => "interval",
        _ => "once",
    }
}

fn task_optimize_repeat_unit(value: &str) -> &'static str {
    match value.trim().to_ascii_lowercase().as_str() {
        "minute" | "minutes" | "分钟" => "minutes",
        "day" | "days" | "天" | "日" => "days",
        "week" | "weeks" | "周" | "星期" => "weeks",
        "month" | "months" | "月" => "months",
        _ => "hours",
    }
}

fn task_optimize_repeat_every(value: &str, unit: &str, fallback: &str) -> String {
    let fallback_number = fallback
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|item| *item > 0)
        .unwrap_or(1);
    let mut number = value
        .trim()
        .parse::<i64>()
        .ok()
        .filter(|item| *item > 0)
        .unwrap_or(fallback_number);
    if unit == "months" && !matches!(number, 1 | 2 | 3 | 4 | 6 | 12) {
        number = if matches!(fallback_number, 1 | 2 | 3 | 4 | 6 | 12) {
            fallback_number
        } else {
            1
        };
    }
    number.to_string()
}

fn task_optimize_rfc3339_local(value: &str) -> Option<String> {
    parse_rfc3339_time(value.trim()).map(format_offset_datetime_to_local_rfc3339)
}

fn task_optimize_run_at(value: Option<&str>, fallback: &str) -> String {
    value
        .and_then(task_optimize_rfc3339_local)
        .or_else(|| task_optimize_rfc3339_local(fallback))
        .unwrap_or_else(now_local_rfc3339)
}

fn task_optimize_end_at(value: Option<&str>, fallback: &str, run_at: &str) -> String {
    let candidate = value
        .and_then(task_optimize_rfc3339_local)
        .or_else(|| task_optimize_rfc3339_local(fallback));
    let Some(candidate) = candidate else {
        return String::new();
    };
    let Some(end_at_dt) = parse_rfc3339_time(&candidate) else {
        return String::new();
    };
    let Some(run_at_dt) = parse_rfc3339_time(run_at) else {
        return candidate;
    };
    if end_at_dt > run_at_dt {
        candidate
    } else {
        String::new()
    }
}

#[tauri::command]
fn task_get_task(input: TaskGetInput, state: State<'_, AppState>) -> Result<TaskEntry, String> {
    task_store_get_task(&state.data_path, input.task_id.trim())
}

#[tauri::command]
async fn task_optimize_draft(
    input: TaskOptimizeDraftInput,
    state: State<'_, AppState>,
) -> Result<TaskOptimizeDraftOutput, String> {
    task_optimize_draft_internal(input, state.inner()).await
}

async fn task_optimize_draft_internal(
    input: TaskOptimizeDraftInput,
    state: &AppState,
) -> Result<TaskOptimizeDraftOutput, String> {
    let started_at = std::time::Instant::now();
    runtime_log_info(format!(
        "[任务草稿优化] 开始，任务=结构化任务草稿，标题字符数={}，正文字符数={}，schedule_mode={}，run_at={}，repeat_every={}，repeat_unit={}，end_at={}",
        input.title.trim().chars().count(),
        input.content.trim().chars().count(),
        task_optimize_schedule_mode(&input.schedule_mode),
        input.run_at.trim(),
        input.repeat_every.trim(),
        task_optimize_repeat_unit(&input.repeat_unit),
        input.end_at.trim()
    ));
    let result = async {
        let prompt = task_optimize_draft_prompt(&input)?;
        let value = invoke_quick_model_json(
            state,
            "Task draft optimization",
            &prompt,
            None,
            &["content"],
            &["title"],
        )
        .await?;
        task_optimize_draft_output_from_value(&value, &input)
    }
    .await;
    match result {
        Ok(output) => {
            runtime_log_info(format!(
                "[任务草稿优化] 完成，任务=结构化任务草稿，耗时毫秒={}，schedule_mode={}，run_at={}，repeat_every={}，repeat_unit={}，end_at={}，标题字符数={}，正文字符数={}",
                started_at.elapsed().as_millis(),
                output.schedule_mode,
                output.run_at,
                output.repeat_every,
                output.repeat_unit,
                output.end_at,
                output.title.chars().count(),
                output.content.chars().count()
            ));
            Ok(output)
        }
        Err(err) => {
            runtime_log_warn(format!(
                "[任务草稿优化] 失败，任务=结构化任务草稿，耗时毫秒={}，error={}",
                started_at.elapsed().as_millis(),
                err
            ));
            Err(err)
        }
    }
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

// ==================== 调度事件系统（事件式溯源） ====================
//
// 以「一轮调度」为边界的事件流，首阶段仅委托接入，调试页 LogTab 不动。
// 一轮调度 = 一次 send_chat_message_inner（chat_pipeline 生命周期，含多轮工具调用直到最终回答）。
// 这是面向全项目的事件系统首个域（调度域），后续扩展到任务、记忆等域共用同一事件基座。
// 缓存语义：按 conversation_id 分组，每会话保留最近 N 个 Run（普通 3，委托 10），进程退出清空。



const SCHEDULE_EVENT_CAPACITY_NORMAL: usize = 3;
const SCHEDULE_EVENT_CAPACITY_DELEGATE: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScheduleEvent {
    id: String,
    run_id: String,
    conversation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    delegate_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    root_conversation_id: Option<String>,
    phase: String,
    created_at: String,
    elapsed_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    success: Option<bool>,
    #[serde(default)]
    detail: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScheduleRun {
    run_id: String,
    conversation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    delegate_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    root_conversation_id: Option<String>,
    status: String,
    started_at: String,
    updated_at: String,
    elapsed_ms: u64,
    request_count: usize,
    tool_call_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_model_name: Option<String>,
    events: Vec<ScheduleEvent>,
    #[serde(skip)]
    started_instant: Option<std::time::Instant>,
}

#[derive(Debug, Default)]
struct ScheduleEventStore {
    runs_by_conversation: std::collections::HashMap<String, std::collections::VecDeque<ScheduleRun>>,
}

fn schedule_event_capacity(is_delegate: bool) -> usize {
    if is_delegate {
        SCHEDULE_EVENT_CAPACITY_DELEGATE
    } else {
        SCHEDULE_EVENT_CAPACITY_NORMAL
    }
}

fn schedule_event_is_delegate_schedule(
    state: &AppState,
    runtime_context: &RuntimeContext,
    conversation_id: &str,
) -> bool {
    if runtime_context
        .remote_im_reply_delegate_id
        .as_deref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
    {
        return true;
    }
    if runtime_context
        .dispatch_id
        .as_deref()
        .map(|value| value.trim().starts_with("delegate-"))
        .unwrap_or(false)
    {
        return true;
    }
    let normalized_conversation_id = conversation_id.trim();
    if normalized_conversation_id.is_empty() {
        return false;
    }
    if let Ok(Some(thread)) = delegate_runtime_thread_get(state, normalized_conversation_id) {
        let _ = thread;
        return true;
    }
    if let Ok(meta) = conversation_service_v2().get_conversation_meta(state, normalized_conversation_id) {
        return meta.conversation_kind.trim() == CONVERSATION_KIND_DELEGATE;
    }
    false
}

fn schedule_event_ensure_run(
    store: &mut ScheduleEventStore,
    conversation_id: &str,
    run_id: &str,
    delegate_id: Option<String>,
    root_conversation_id: Option<String>,
    started_at: &str,
    is_delegate: bool,
) {
    schedule_event_ensure_run_with_instant(
        store,
        conversation_id,
        run_id,
        delegate_id,
        root_conversation_id,
        started_at,
        None,
        is_delegate,
    );
}

fn schedule_event_ensure_run_with_instant(
    store: &mut ScheduleEventStore,
    conversation_id: &str,
    run_id: &str,
    delegate_id: Option<String>,
    root_conversation_id: Option<String>,
    started_at: &str,
    started_instant: Option<std::time::Instant>,
    is_delegate: bool,
) {
    let key = conversation_id.trim().to_string();
    if key.is_empty() || run_id.trim().is_empty() {
        return;
    }
    let deque = store.runs_by_conversation.entry(key.clone()).or_default();
    if deque.iter().any(|run| run.run_id == run_id) {
        return;
    }
    let now = started_at.to_string();
    deque.push_back(ScheduleRun {
        run_id: run_id.to_string(),
        conversation_id: key.clone(),
        delegate_id: delegate_id.filter(|value| !value.trim().is_empty()),
        root_conversation_id: root_conversation_id.filter(|value| !value.trim().is_empty()),
        status: "running".to_string(),
        started_at: now.clone(),
        updated_at: now,
        elapsed_ms: 0,
        request_count: 0,
        tool_call_count: 0,
        last_tool_name: None,
        last_model_name: None,
        events: Vec::new(),
        started_instant,
    });
    let capacity = schedule_event_capacity(is_delegate);
    while deque.len() > capacity {
        deque.pop_front();
    }
}

fn schedule_event_push_inner(
    state: &AppState,
    conversation_id: &str,
    run_id: &str,
    phase: &str,
    elapsed_ms: u64,
    success: Option<bool>,
    detail: Value,
) -> Result<(), String> {
    let key = conversation_id.trim();
    let normalized_run_id = run_id.trim();
    if key.is_empty() || normalized_run_id.is_empty() || phase.trim().is_empty() {
        return Ok(());
    }
    let Ok(mut store) = state.schedule_events.lock() else {
        return Err("Failed to lock schedule events".to_string());
    };
    let Some(deque) = store.runs_by_conversation.get_mut(key) else {
        return Ok(());
    };
    let Some(run) = deque.iter_mut().find(|item| item.run_id == normalized_run_id) else {
        return Ok(());
    };
    let normalized_phase = phase.trim();
    if normalized_phase == "tool_call" {
        run.tool_call_count = run.tool_call_count.saturating_add(1);
        if let Some(name) = detail.get("toolName").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()) {
            run.last_tool_name = Some(name.to_string());
        }
    }
    if normalized_phase == "model_round_start" {
        run.request_count = run.request_count.saturating_add(1);
    }
    if normalized_phase == "model_round_start" || normalized_phase == "model_round_end" {
        if let Some(name) = detail.get("modelName").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()) {
            run.last_model_name = Some(name.to_string());
        }
    }
    // 兼容 fallback: 若 model_round 阶段漏打点，dispatch 维度也可在 compaction 阶段感知
    let event = ScheduleEvent {
        id: Uuid::new_v4().to_string(),
        run_id: normalized_run_id.to_string(),
        conversation_id: key.to_string(),
        delegate_id: run.delegate_id.clone(),
        root_conversation_id: run.root_conversation_id.clone(),
        phase: normalized_phase.to_string(),
        created_at: now_log_local_rfc3339(),
        elapsed_ms,
        success,
        detail: detail.clone(),
    };
    if normalized_phase == "dispatch_end" {
        if let Some(flag) = success {
            run.status = if flag { "success".to_string() } else { "error".to_string() };
        }
    }
    if normalized_phase == "model_round_end" {
        if let Some(flag) = success {
            if !flag {
                // 失败模型轮次不翻转整体 Run 状态，仅记录事件
            }
        }
    }
    run.elapsed_ms = elapsed_ms;
    run.updated_at = event.created_at.clone();
    run.events.push(event);
    Ok(())
}

fn schedule_event_resolve_target_run(
    store: &ScheduleEventStore,
    key: &str,
) -> Option<(String, usize)> {
    if let Some(deque) = store.runs_by_conversation.get(key) {
        let idx = deque
            .iter()
            .rposition(|run| run.status == "running")
            .or_else(|| if deque.is_empty() { None } else { Some(deque.len() - 1) })?;
        return Some((key.to_string(), idx));
    }
    let key_suffix = key.rsplit("::").next().unwrap_or(key).trim();
    let key_suffix_core = key_suffix.split(':').next_back().unwrap_or(key_suffix).trim();
    for (candidate_key, deque) in store.runs_by_conversation.iter() {
        let candidate_suffix = candidate_key
            .rsplit("::")
            .next()
            .unwrap_or(candidate_key.as_str())
            .trim();
        let suffix_match = !key_suffix.is_empty()
            && (key_suffix == candidate_suffix
                || key_suffix_core == candidate_suffix
                || key_suffix == candidate_key.as_str()
                || key_suffix_core == candidate_key.as_str());
        if suffix_match || key.contains(candidate_key.as_str()) || candidate_key.contains(key) {
            if let Some(idx) = deque.iter().rposition(|run| run.status == "running") {
                return Some((candidate_key.clone(), idx));
            }
            if !deque.is_empty() {
                return Some((candidate_key.clone(), deque.len() - 1));
            }
        }
    }
    for (candidate_key, deque) in store.runs_by_conversation.iter() {
        let has_match = deque
            .iter()
            .any(|run| run.delegate_id.as_deref() == Some(key) || key == run.conversation_id);
        if has_match && !deque.is_empty() {
            let idx = deque
                .iter()
                .rposition(|run| run.status == "running")
                .unwrap_or(deque.len().saturating_sub(1));
            return Some((candidate_key.clone(), idx));
        }
    }
    let mut running_candidates: Vec<(String, usize)> = Vec::new();
    for (candidate_key, deque) in store.runs_by_conversation.iter() {
        if let Some(idx) = deque.iter().rposition(|run| run.status == "running") {
            running_candidates.push((candidate_key.clone(), idx));
        }
    }
    if running_candidates.len() == 1 {
        return running_candidates.into_iter().next();
    }
    None
}

fn schedule_event_push_to_latest_run(
    state: &AppState,
    conversation_id: &str,
    phase: &str,
    elapsed_ms: u64,
    success: Option<bool>,
    detail: Value,
) -> Result<bool, String> {
    let key = conversation_id.trim();
    let normalized_phase = phase.trim();
    if key.is_empty() || normalized_phase.is_empty() {
        return Ok(false);
    }
    let Ok(mut store) = state.schedule_events.lock() else {
        return Err("Failed to lock schedule events".to_string());
    };
    let Some((deque_key, idx)) = schedule_event_resolve_target_run(&store, key) else {
        return Ok(false);
    };
    let Some(deque) = store.runs_by_conversation.get_mut(deque_key.as_str()) else {
        return Ok(false);
    };
    // 计算 elapsed：若调用方传入了 elapsed 则直接使用，否则用 Instant 推导
    let computed_elapsed = if elapsed_ms > 0 {
        elapsed_ms
    } else if let Some(instant) = deque[idx].started_instant {
        instant.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
    } else {
        deque[idx].elapsed_ms
    };
    let run_id = deque[idx].run_id.clone();
    let delegate_id = deque[idx].delegate_id.clone();
    let root_conversation_id = deque[idx].root_conversation_id.clone();
    let conversation_id_owned = deque[idx].conversation_id.clone();
    if normalized_phase == "tool_call" {
        deque[idx].tool_call_count = deque[idx].tool_call_count.saturating_add(1);
        if let Some(name) = detail.get("toolName").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()) {
            deque[idx].last_tool_name = Some(name.to_string());
        }
    }
    if normalized_phase == "model_round_start" {
        deque[idx].request_count = deque[idx].request_count.saturating_add(1);
    }
    if normalized_phase == "model_round_start" || normalized_phase == "model_round_end" {
        if let Some(name) = detail.get("modelName").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()) {
            deque[idx].last_model_name = Some(name.to_string());
        }
    }
    let event = ScheduleEvent {
        id: Uuid::new_v4().to_string(),
        run_id: run_id.clone(),
        conversation_id: conversation_id_owned,
        delegate_id,
        root_conversation_id,
        phase: normalized_phase.to_string(),
        created_at: now_log_local_rfc3339(),
        elapsed_ms: computed_elapsed,
        success,
        detail: detail.clone(),
    };
    deque[idx].elapsed_ms = computed_elapsed;
    deque[idx].updated_at = event.created_at.clone();
    deque[idx].events.push(event);
    Ok(true)
}

fn schedule_event_elapsed_from_run(
    store: &ScheduleEventStore,
    conversation_id: &str,
    run_id: &str,
) -> u64 {
    let key = conversation_id.trim();
    let normalized_run_id = run_id.trim();
    if key.is_empty() || normalized_run_id.is_empty() {
        return 0;
    }
    let Some(deque) = store.runs_by_conversation.get(key) else {
        return 0;
    };
    let Some(run) = deque.iter().find(|item| item.run_id == normalized_run_id) else {
        return 0;
    };
    if let Some(instant) = run.started_instant {
        instant.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
    } else {
        run.elapsed_ms
    }
}

fn schedule_event_run_start(
    state: &AppState,
    runtime_context: &RuntimeContext,
    conversation_id: &str,
    run_id: &str,
    started_at: &str,
    elapsed_ms: u64,
    detail: Value,
) -> Result<bool, String> {
    schedule_event_run_start_with_instant(
        state,
        runtime_context,
        conversation_id,
        run_id,
        started_at,
        None,
        elapsed_ms,
        detail,
    )
}

fn schedule_event_run_start_with_instant(
    state: &AppState,
    runtime_context: &RuntimeContext,
    conversation_id: &str,
    run_id: &str,
    started_at: &str,
    started_instant: Option<std::time::Instant>,
    elapsed_ms: u64,
    detail: Value,
) -> Result<bool, String> {
    if !schedule_event_is_delegate_schedule(state, runtime_context, conversation_id) {
        return Ok(false);
    }
    let delegate_id = runtime_context
        .remote_im_reply_delegate_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            let trimmed = conversation_id.trim();
            if trimmed.is_empty() {
                None
            } else if let Ok(Some(_)) = delegate_runtime_thread_get(state, trimmed) {
                Some(trimmed.to_string())
            } else {
                None
            }
        });
    let root_conversation_id = runtime_context
        .root_conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            runtime_context
                .origin_conversation_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        });
    let is_delegate = true;
    {
        let Ok(mut store) = state.schedule_events.lock() else {
            return Err("Failed to lock schedule events".to_string());
        };
        schedule_event_ensure_run_with_instant(
            &mut store,
            conversation_id,
            run_id,
            delegate_id.clone(),
            root_conversation_id.clone(),
            started_at,
            started_instant,
            is_delegate,
        );
    }
    schedule_event_push_inner(state, conversation_id, run_id, "dispatch_start", elapsed_ms, None, detail)?;
    Ok(true)
}

fn schedule_event_push_if_delegate(
    state: &AppState,
    runtime_context: &RuntimeContext,
    conversation_id: &str,
    run_id: &str,
    phase: &str,
    elapsed_ms: u64,
    success: Option<bool>,
    detail: Value,
) -> Result<bool, String> {
    if !schedule_event_is_delegate_schedule(state, runtime_context, conversation_id) {
        return Ok(false);
    }
    schedule_event_push_inner(state, conversation_id, run_id, phase, elapsed_ms, success, detail)?;
    Ok(true)
}

fn schedule_event_list_runs_inner(
    state: &AppState,
    conversation_id: &str,
) -> Result<Vec<ScheduleRun>, String> {
    let key = conversation_id.trim();
    if key.is_empty() {
        return Ok(Vec::new());
    }
    let store = state
        .schedule_events
        .lock()
        .map_err(|_| "Failed to lock schedule events".to_string())?;
    let Some(deque) = store.runs_by_conversation.get(key) else {
        return Ok(Vec::new());
    };
    Ok(deque.iter().cloned().collect())
}

#[tauri::command]
fn list_schedule_runs(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<ScheduleRun>, String> {
    schedule_event_list_runs_inner(state.inner(), &conversation_id)
}

fn schedule_event_estimated_json_bytes(store: &ScheduleEventStore) -> usize {
    estimate_json_bytes(&store.runs_by_conversation)
}

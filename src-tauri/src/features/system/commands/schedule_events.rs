// ==================== 调度事件系统（事件式溯源） ====================
//
// 以「一轮调度」为边界的事件流，首阶段仅委托接入，调试页 LogTab 不动。
// 一轮调度 = 一次 send_chat_message_inner（chat_pipeline 生命周期，含多轮工具调用直到最终回答）。
// 这是面向全项目的事件系统首个域（调度域），后续扩展到任务、记忆等域共用同一事件基座。
// 缓存语义：按 conversation_id 分组，每会话保留最近 N 个 Run（普通 3，委托 10），进程退出清空。



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
    #[serde(skip_serializing_if = "Option::is_none")]
    trace_id: Option<String>,
    #[serde(default)]
    scene: String,
    #[serde(default)]
    request_format: String,
    #[serde(default)]
    provider: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    base_url: String,
    #[serde(default)]
    headers: Vec<LlmRoundLogHeader>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Value>,
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

fn schedule_event_capacity_for_state(state: &AppState) -> usize {
    llm_round_log_capacity_for_state(state)
}

fn schedule_event_apply_run_metadata_from_detail(run: &mut ScheduleRun, detail: &Value) {
    if run.trace_id.is_none() {
        if let Some(value) = detail.get("traceId").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()) {
            run.trace_id = Some(value.to_string());
        }
    }
    if run.scene.trim().is_empty() {
        if let Some(value) = detail.get("scene").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()) {
            run.scene = value.to_string();
        }
    }
    if run.request_format.trim().is_empty() {
        if let Some(value) = detail.get("requestFormat").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()) {
            run.request_format = value.to_string();
        }
    }
    if run.provider.trim().is_empty() {
        if let Some(value) = detail.get("provider").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()) {
            run.provider = value.to_string();
        } else if let Some(value) = detail.get("providerName").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()) {
            run.provider = value.to_string();
        }
    }
    if run.model.trim().is_empty() {
        if let Some(value) = detail.get("model").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()) {
            run.model = value.to_string();
        } else if let Some(value) = detail.get("modelName").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()) {
            run.model = value.to_string();
        }
    }
    if run.base_url.trim().is_empty() {
        if let Some(value) = detail.get("baseUrl").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty()) {
            run.base_url = value.to_string();
        }
    }
    if run.headers.is_empty() {
        if let Some(value) = detail.get("headers").and_then(Value::as_array) {
            let mut headers = Vec::new();
            for item in value {
                let name = item.get("name").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty());
                let header_value = item.get("value").and_then(Value::as_str).map(str::trim).filter(|value| !value.is_empty());
                if let (Some(name), Some(header_value)) = (name, header_value) {
                    headers.push(LlmRoundLogHeader { name: name.to_string(), value: header_value.to_string() });
                }
            }
            if !headers.is_empty() {
                run.headers = headers;
            }
        }
    }
    if run.tools.is_none() {
        if let Some(value) = detail.get("tools").cloned() {
            if !value.is_null() {
                run.tools = Some(value);
            }
        }
    }
    if run.tools.is_none() {
        if let Some(value) = detail.get("availableTools").cloned() {
            if !value.is_null() {
                run.tools = Some(value);
            }
        }
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
fn schedule_event_ensure_run(
    state: &AppState,
    store: &mut ScheduleEventStore,
    conversation_id: &str,
    run_id: &str,
    delegate_id: Option<String>,
    root_conversation_id: Option<String>,
    started_at: &str,
) {
    schedule_event_ensure_run_with_instant(
        state,
        store,
        conversation_id,
        run_id,
        delegate_id,
        root_conversation_id,
        started_at,
        None,
    );
}

fn schedule_event_ensure_run_with_instant(
    state: &AppState,
    store: &mut ScheduleEventStore,
    conversation_id: &str,
    run_id: &str,
    delegate_id: Option<String>,
    root_conversation_id: Option<String>,
    started_at: &str,
    started_instant: Option<std::time::Instant>,
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
        trace_id: None,
        scene: String::new(),
        request_format: String::new(),
        provider: String::new(),
        model: String::new(),
        base_url: String::new(),
        headers: Vec::new(),
        tools: None,
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
    let capacity = schedule_event_capacity_for_state(state);
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
    // 去重：同一 Run 内 headers/baseUrl/tools 等不变元数据仅在 dispatch_start 存一次，后续增量不再重复写入 Run 头
    if normalized_phase == "dispatch_start" || normalized_phase == "dispatch_end" || normalized_phase == "model_round_start" || normalized_phase == "model_round_end" {
        schedule_event_apply_run_metadata_from_detail(run, &detail);
    }
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
    // 去重：Run 头元数据仅首次写入，后续增量不再重复覆盖 headers/baseUrl/tools
    if normalized_phase == "dispatch_start"
        || normalized_phase == "dispatch_end"
        || normalized_phase == "model_round_start"
        || normalized_phase == "model_round_end"
    {
        schedule_event_apply_run_metadata_from_detail(&mut deque[idx], &detail);
    }
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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
    // 已全量接入：不再按 is_delegate 过滤，普通会话与委托会话均落库，容量由 llmRoundLogCapacity 统一控制
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
    {
        let Ok(mut store) = state.schedule_events.lock() else {
            return Err("Failed to lock schedule events".to_string());
        };
        schedule_event_ensure_run_with_instant(
            state,
            &mut store,
            conversation_id,
            run_id,
            delegate_id.clone(),
            root_conversation_id.clone(),
            started_at,
            started_instant,
        );
    }
    schedule_event_push_inner(state, conversation_id, run_id, "dispatch_start", elapsed_ms, None, detail)?;
    Ok(true)
}

fn schedule_event_push_if_delegate(
    state: &AppState,
    _runtime_context: &RuntimeContext,
    conversation_id: &str,
    run_id: &str,
    phase: &str,
    elapsed_ms: u64,
    success: Option<bool>,
    detail: Value,
) -> Result<bool, String> {
    // 已全量接入：保持兼容名但不再过滤，直接落库
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

fn schedule_event_update_run_metadata(
    state: &AppState,
    conversation_id: &str,
    run_id: &str,
    detail: Value,
) -> Result<bool, String> {
    let key = conversation_id.trim();
    let normalized_run_id = run_id.trim();
    if key.is_empty() || normalized_run_id.is_empty() {
        return Ok(false);
    }
    let Ok(mut store) = state.schedule_events.lock() else {
        return Err("Failed to lock schedule events".to_string());
    };
    let Some(deque) = store.runs_by_conversation.get_mut(key) else {
        return Ok(false);
    };
    let Some(run) = deque.iter_mut().find(|item| item.run_id == normalized_run_id) else {
        return Ok(false);
    };
    schedule_event_apply_run_metadata_from_detail(run, &detail);
    Ok(true)
}

fn schedule_run_to_llm_entry(run: &ScheduleRun) -> LlmRoundLogEntry {
    let scene = if run.scene.trim().is_empty() { "chat_pipeline".to_string() } else { run.scene.clone() };
    let request_format = if run.request_format.trim().is_empty() { "openai".to_string() } else { run.request_format.clone() };
    let provider = run.provider.clone();
    let model = run.model.clone();
    let base_url = run.base_url.clone();
    let headers = run.headers.clone();
    let tools = run.tools.clone();
    let success = run.status != "error" && run.status != "failed";
    let mut error: Option<String> = None;
    let mut response: Option<Value> = None;
    let mut timeline: Option<Vec<LlmRoundLogStage>> = None;
    if !run.events.is_empty() {
        let mut stages = Vec::new();
        let mut prev_elapsed = 0_u64;
        for event in &run.events {
            let since = event.elapsed_ms.saturating_sub(prev_elapsed);
            let detail = if event.detail.is_null() { None } else { Some(event.detail.clone()) };
            stages.push(LlmRoundLogStage { stage: event.phase.clone(), elapsed_ms: event.elapsed_ms, since_prev_ms: since, detail });
            prev_elapsed = event.elapsed_ms;
            if event.phase == "dispatch_end" {
                if let Some(flag) = event.success {
                    if !flag {
                        error = event.detail.get("error").and_then(Value::as_str).map(ToOwned::to_owned).or_else(|| Some("dispatch failed".to_string()));
                    }
                }
                // 合成 pipeline response：取 dispatch_end 的文本与 usage
                if error.is_none() {
                    let mut obj = serde_json::Map::new();
                    if let Some(v) = event.detail.get("assistantTextLength") { obj.insert("assistantTextLength".to_string(), v.clone()); }
                    if let Some(v) = event.detail.get("reasoningLength") { obj.insert("reasoningLength".to_string(), v.clone()); }
                    if let Some(v) = event.detail.get("textPreview").and_then(Value::as_str) {
                        obj.insert("assistantText".to_string(), Value::String(v.to_string()));
                    }
                    if let Some(v) = event.detail.get("reasoningPreview").and_then(Value::as_str) {
                        obj.insert("reasoningContent".to_string(), Value::String(v.to_string()));
                        obj.insert("activityReasoningText".to_string(), Value::String(v.to_string()));
                    }
                    if let Some(v) = event.detail.get("usage") { obj.insert("usage".to_string(), v.clone()); }
                    if let Some(v) = event.detail.get("toolCallCount") { obj.insert("toolCallCount".to_string(), v.clone()); }
                    if !obj.is_empty() { response = Some(Value::Object(obj)); }
                } else {
                    // 失败时也保留 error 字段，response 置空
                }
            }
        }
        timeline = Some(stages);
    }
    if response.is_none() && run.events.iter().any(|e| e.phase == "model_round_end") {
        // 回退：取最后一次 model_round_end 的预览作为 pipeline response
        if let Some(ev) = run.events.iter().rev().find(|e| e.phase == "model_round_end") {
            let mut obj = serde_json::Map::new();
            if let Some(v) = ev.detail.get("assistantTextLength") { obj.insert("assistantTextLength".to_string(), v.clone()); }
            if let Some(v) = ev.detail.get("reasoningLength") { obj.insert("reasoningLength".to_string(), v.clone()); }
            if let Some(v) = ev.detail.get("textPreview").and_then(Value::as_str) {
                obj.insert("assistantText".to_string(), Value::String(v.to_string()));
            }
            if let Some(v) = ev.detail.get("reasoningPreview").and_then(Value::as_str) {
                obj.insert("reasoningContent".to_string(), Value::String(v.to_string()));
                obj.insert("activityReasoningText".to_string(), Value::String(v.to_string()));
            }
            if let Some(v) = ev.detail.get("toolCallCount") { obj.insert("toolCallCount".to_string(), v.clone()); }
            if let Some(v) = ev.detail.get("usage") { obj.insert("usage".to_string(), v.clone()); }
            if !obj.is_empty() { response = Some(Value::Object(obj)); }
        }
    }
    // 合成 rounds：以 model_round_start/end 配对
    let mut rounds: Option<Vec<LlmRoundLogEntry>> = None;
    let mut round_entries = Vec::new();
    let mut idx = 0_usize;
    while idx < run.events.len() {
        let ev = &run.events[idx];
        if ev.phase == "model_round_start" {
            let start = ev;
            // 寻找对应的 end
            let mut end_opt: Option<&ScheduleEvent> = None;
            let mut tool_names: Vec<String> = Vec::new();
            let mut tool_count = 0_usize;
            let mut j = idx + 1;
            while j < run.events.len() {
                let nxt = &run.events[j];
                if nxt.phase == "tool_call" {
                    if let Some(name) = nxt.detail.get("toolName").and_then(Value::as_str) { push_unique_log_name(&mut tool_names, name); }
                    tool_count = tool_count.saturating_add(1);
                }
                if nxt.phase == "model_round_end" {
                    end_opt = Some(nxt);
                    break;
                }
                if nxt.phase == "model_round_start" {
                    break;
                }
                j += 1;
            }
            if let Some(end) = end_opt {
                let round_id = format!("{}-round-{}", run.run_id, round_entries.len() + 1);
                let assistant_text = end.detail.get("textPreview").and_then(Value::as_str).unwrap_or("").to_string();
                let reasoning_text = end.detail.get("reasoningPreview").and_then(Value::as_str).unwrap_or("").to_string();
                let assistant_len = end.detail.get("assistantTextLength").and_then(Value::as_u64).unwrap_or(assistant_text.chars().count() as u64);
                let reasoning_len = end.detail.get("reasoningLength").and_then(Value::as_u64).unwrap_or(reasoning_text.chars().count() as u64);
                let success = end.success.unwrap_or(true);
                let err = end.detail.get("error").and_then(Value::as_str).map(ToOwned::to_owned);
                let mut resp_obj = serde_json::Map::new();
                resp_obj.insert("assistantText".to_string(), Value::String(assistant_text.clone()));
                if !reasoning_text.is_empty() {
                    resp_obj.insert("reasoningContent".to_string(), Value::String(reasoning_text.clone()));
                    resp_obj.insert("activityReasoningText".to_string(), Value::String(reasoning_text));
                }
                resp_obj.insert("assistantTextLength".to_string(), serde_json::json!(assistant_len));
                resp_obj.insert("reasoningContentLength".to_string(), serde_json::json!(reasoning_len));
                resp_obj.insert("toolCallCount".to_string(), serde_json::json!(tool_count));
                if !tool_names.is_empty() { resp_obj.insert("toolCallNames".to_string(), log_tool_call_names_value(tool_names.clone())); }
                let round = LlmRoundLogEntry {
                    id: round_id,
                    created_at: start.created_at.clone(),
                    trace_id: run.trace_id.clone(),
                    scene: "chat".to_string(),
                    request_format: request_format.clone(),
                    provider: if provider.is_empty() { start.detail.get("providerName").and_then(Value::as_str).unwrap_or("").to_string() } else { provider.clone() },
                    model: if model.is_empty() { start.detail.get("modelName").and_then(Value::as_str).unwrap_or("").to_string() } else { model.clone() },
                    base_url: base_url.clone(),
                    headers: headers.clone(),
                    tools: tools.clone(),
                    response: Some(Value::Object(resp_obj)),
                    error: err.clone(),
                    elapsed_ms: end.elapsed_ms.saturating_sub(start.elapsed_ms),
                    timeline: None,
                    round_count: None,
                    tool_call_count: Some(tool_count),
                    rounds: None,
                    success,
                };
                round_entries.push(round);
                idx = j + 1;
                continue;
            }
        }
        idx += 1;
    }
    if !round_entries.is_empty() { rounds = Some(round_entries); }
    // 若无 rounds 但有工具调用，仍提供空 rounds 占位以免前端误判
    let round_count_val = rounds.as_ref().map(|v| v.len()).unwrap_or(0);
    let tool_call_count_val = run.tool_call_count;
    // 未设置 provider/model 时回退到 last_model_name
    let final_provider = if provider.is_empty() { run.last_model_name.clone().unwrap_or_default() } else { provider };
    let final_model = if model.is_empty() { run.last_model_name.clone().unwrap_or_default() } else { model };
    LlmRoundLogEntry {
        id: run.run_id.clone(),
        created_at: run.started_at.clone(),
        trace_id: run.trace_id.clone(),
        scene,
        request_format,
        provider: final_provider,
        model: final_model,
        base_url,
        headers,
        tools,
        response,
        error,
        elapsed_ms: run.elapsed_ms,
        timeline,
        round_count: Some(round_count_val),
        tool_call_count: Some(tool_call_count_val),
        rounds,
        success,
    }
}

fn schedule_event_collect_llm_entries(state: &AppState, capacity: usize) -> Vec<LlmRoundLogEntry> {
    let Ok(store) = state.schedule_events.lock() else { return Vec::new(); };
    let mut all: Vec<LlmRoundLogEntry> = Vec::new();
    for deque in store.runs_by_conversation.values() {
        for run in deque.iter() {
            all.push(schedule_run_to_llm_entry(run));
        }
    }
    // 按创建时间排序，最新的在后，再按 capacity 截断，每会话窗口已由 Run 容量控制，这里再全局按 capacity 过滤以兼容旧语义（pipeline_logs + other_logs 各 capacity）
    all.sort_by(|a, b| a.created_at.cmp(&b.created_at));
    if all.len() > capacity.saturating_mul(2) {
        let skip = all.len().saturating_sub(capacity.saturating_mul(2));
        all.drain(0..skip);
    }
    // 映射为 UI 兼容的紧凑结构（tools 压缩等由 compact_llm_round_log_entry_for_ui 处理，上层会再处理）
    all.into_iter().map(|entry| compact_llm_round_log_entry_for_ui(&entry)).collect()
}

fn schedule_event_find_entry_by_id(state: &AppState, id: &str) -> Option<LlmRoundLogEntry> {
    let trimmed = id.trim();
    if trimmed.is_empty() { return None; }
    let Ok(store) = state.schedule_events.lock() else { return None; };
    for deque in store.runs_by_conversation.values() {
        for run in deque.iter() {
            let entry = schedule_run_to_llm_entry(run);
            if let Some(found) = find_llm_round_log_entry_by_id(&entry, trimmed) { return Some(found.clone()); }
        }
    }
    None
}

fn schedule_event_clear_all(state: &AppState) -> Result<bool, String> {
    let mut store = state.schedule_events.lock().map_err(|_| "Failed to lock schedule events".to_string())?;
    store.runs_by_conversation.clear();
    Ok(true)
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

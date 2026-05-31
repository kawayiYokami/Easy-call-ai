// ==================== 群聊消息调度与主助理串行系统 ====================
//
// 这是当前项目最核心、也最容易被误改的业务边界之一。
//
// 这里必须先分清两个概念：
//
// 1. 队列（queue）
//    语义是：等当前这一轮 LLM 调度完整结束后，再把下一批消息送进去。
//    它不会改变当前轮次的收口时机，只会决定“下一轮什么时候开始”。
//
// 2. 引导（guided）
//    语义是：当前这一轮只要完成一次工具执行，就允许立刻截断当前调度，
//    把引导消息插进去，然后重新发起一轮新的调度。
//    它不是普通队列换个标签，而是会改变“当前轮次什么时候被截断”。
//
// 因此，这里实现的不是传统“用户发一句 -> 助理立刻回一句”的线性聊天，
// 而是一个面向未来跨进程协作的“单主助理消息流”：
//
// 1. 新消息先进入调度层
//    所有新消息，无论来自用户、任务、委托、系统，未来甚至来自其他进程，
//    都必须先进入调度层，不能直接写进正式历史。
//
// 2. 正式历史是唯一生效层
//    一条消息只有在批量写入 conversation.messages 之后，才算正式生效。
//    因此消息的业务时间应以“刷入历史的时间”为准，而不是入队时间。
//
// 3. 主助理永远只有一个前台轮次
//    当主助理正在流式，或者正在整理上下文时，普通队列消息不能插入当前轮次。
//    只有引导才允许在“工具执行完成”这个切点提前截断并重启调度。
//
// 4. 调度器负责两种不同切点
//    - 队列：本轮完美结束 -> 下一轮开始
//    - 引导：一次工具执行完成 -> 立刻截断 -> 插入消息 -> 重启调度
//
// 这套设计保证了：
// - 无论同一时刻涌入多少消息、来自多少个来源，都能稳定收敛；
// - 正式历史和运行态切点分离，不把“已持久化”误当成“已完成调度”；
// - 前后端都围绕同一套“何时落历史、何时截断、何时重启”的语义工作。

// ==================== 数据结构定义 ====================

/// 主会话状态机
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MainSessionState {
    /// 空闲，可以出队
    Idle,
    /// 主助理正在流式输出
    AssistantStreaming,
    /// 正在整理上下文
    OrganizingContext,
}

/// 消息来源类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ChatEventSource {
    /// 用户发言
    User,
    /// 任务触发
    Task,
    /// 委托回报
    Delegate,
    /// 系统事件
    System,
    /// 远程 IM 渠道消息
    #[serde(rename = "remote_im")]
    RemoteIm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ChatQueueMode {
    Normal,
    Guided,
}

fn default_chat_queue_mode() -> ChatQueueMode {
    ChatQueueMode::Normal
}

/// 会话信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatSessionInfo {
    pub department_id: String,
    pub agent_id: String,
}

/// 待处理事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatPendingEvent {
    /// 事件唯一ID
    pub id: String,
    /// 目标会话ID
    pub conversation_id: String,
    /// 入队时间，仅用于排队观测，不代表正式生效时间
    pub created_at: String,
    /// 来源类型
    pub source: ChatEventSource,
    /// 队列模式
    #[serde(default = "default_chat_queue_mode")]
    pub queue_mode: ChatQueueMode,
    /// 要写入的消息集合
    pub messages: Vec<ChatMessage>,
    /// 是否在本批消息写入历史后激活主助理
    pub activate_assistant: bool,
    /// 会话信息
    pub session_info: ChatSessionInfo,
    /// 运行上下文（渐进接入）
    #[serde(default)]
    pub runtime_context: Option<RuntimeContext>,
    /// 远程消息来源（仅 source=RemoteIm 时使用）
    #[serde(default)]
    pub sender_info: Option<RemoteImMessageSource>,
}

#[derive(Clone)]
struct QueuedChatActivation {
    event_id: String,
}

#[derive(Debug, Clone)]
struct ActivatedAssistantResult {
    result: SendChatResult,
    activation_id: String,
    request_id: String,
}

pub(crate) enum ChatEventIngress {
    Direct(ChatPendingEvent),
    Queued { event_id: String },
    Duplicate { event_id: String },
}

// ==================== 队列查询和管理 ====================

/// 队列事件摘要（用于前端显示）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatQueueEventSummary {
    pub id: String,
    pub source: ChatEventSource,
    pub queue_mode: ChatQueueMode,
    pub created_at: String,
    pub message_preview: String,
    pub message_text: String,
    pub conversation_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatQueueRecallResult {
    pub removed: bool,
    pub message_text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatQueueSnapshotPush {
    queue_events: Vec<ChatQueueEventSummary>,
    session_state: MainSessionState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConversationRuntimeSnapshot {
    pub conversation_id: String,
    pub runtime_state: MainSessionState,
    pub is_processing: bool,
    pub has_pending_queue: bool,
    pub pending_queue_count: usize,
    pub stream_cache: ConversationStreamRuntimeCacheSnapshot,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConversationStreamRuntimeCacheSnapshot {
    pub activation_id: String,
    pub request_id: String,
    pub assistant_text: String,
    pub reasoning_standard: String,
    pub reasoning_inline: String,
    pub tool_status_text: String,
    pub tool_status_state: String,
    pub stream_tool_calls: Vec<ConversationStreamToolCallRuntimeCache>,
    pub stream_tool_call_count: usize,
    pub stream_last_tool_name: String,
    pub started_at: String,
    pub started_at_ms: u64,
    pub updated_at: String,
    pub has_visible_progress: bool,
    pub persisted_assistant_message_id: String,
}

const CHAT_QUEUE_SNAPSHOT_EVENT: &str = "easy-call:chat-queue-snapshot";
const CHAT_HISTORY_FLUSHED_EVENT: &str = "easy-call:history-flushed";
const CHAT_ROUND_STARTED_EVENT: &str = "easy-call:round-started";
const CHAT_ROUND_COMPLETED_EVENT: &str = "easy-call:round-completed";
const CHAT_ROUND_FAILED_EVENT: &str = "easy-call:round-failed";
const CHAT_ASSISTANT_DELTA_EVENT: &str = "easy-call:assistant-delta";
const CHAT_STREAM_REBIND_REQUIRED_EVENT: &str = "easy-call:stream-rebind-required";
const CHAT_CONVERSATION_MESSAGE_APPENDED_EVENT: &str = "easy-call:conversation-message-appended";
const CHAT_CONVERSATION_OVERVIEW_UPDATED_EVENT: &str = "easy-call:conversation-overview-updated";
const CHAT_CONCURRENCY_LIMIT: usize = 8;

fn lock_conversation_runtime_slots(
    state: &AppState,
) -> Result<
    std::sync::MutexGuard<'_, std::collections::HashMap<String, ConversationRuntimeSlot>>,
    String,
> {
    match state.conversation_runtime_slots.lock() {
        Ok(guard) => Ok(guard),
        Err(poisoned) => {
            eprintln!("[聊天调度] 警告: conversation_runtime_slots 锁已 poison，正在继续恢复使用");
            Ok(poisoned.into_inner())
        }
    }
}

fn lock_conversation_processing_claims(
    state: &AppState,
) -> Result<std::sync::MutexGuard<'_, std::collections::HashSet<String>>, String> {
    match state.conversation_processing_claims.lock() {
        Ok(guard) => Ok(guard),
        Err(poisoned) => {
            eprintln!(
                "[聊天调度] 警告: conversation_processing_claims 锁已 poison，正在继续恢复使用"
            );
            Ok(poisoned.into_inner())
        }
    }
}

fn conversation_slot_mut<'a>(
    slots: &'a mut std::collections::HashMap<String, ConversationRuntimeSlot>,
    conversation_id: &str,
) -> &'a mut ConversationRuntimeSlot {
    slots.entry(conversation_id.to_string()).or_insert_with(|| {
        let mut slot = ConversationRuntimeSlot::default();
        slot.last_activity_at = now_iso();
        slot
    })
}

fn conversation_running_slot_count(
    claims: &std::collections::HashSet<String>,
    conversation_id: &str,
) -> usize {
    usize::from(claims.contains(conversation_id))
}

fn chat_pending_event_request_id(event: &ChatPendingEvent) -> Option<&str> {
    event
        .runtime_context
        .as_ref()
        .and_then(|context| context.request_id.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn chat_pending_event_duplicates_existing_message(
    state: &AppState,
    event: &ChatPendingEvent,
) -> Result<bool, String> {
    let Some(request_id) = chat_pending_event_request_id(event) else {
        return Ok(false);
    };
    let conversation = match state_read_conversation_cached(state, &event.conversation_id) {
        Ok(conversation) if conversation.summary.trim().is_empty() => conversation,
        _ => return Ok(false),
    };
    Ok(conversation.messages.iter().any(|message| {
        message
            .provider_meta
            .as_ref()
            .and_then(|meta| meta.get("requestId").or_else(|| meta.get("request_id")))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value == request_id)
            .unwrap_or(false)
    }))
}

fn pending_queue_has_same_request_id(
    slot: &ConversationRuntimeSlot,
    event: &ChatPendingEvent,
) -> bool {
    let Some(request_id) = chat_pending_event_request_id(event) else {
        return false;
    };
    slot.pending_queue
        .iter()
        .filter_map(chat_pending_event_request_id)
        .any(|value| value == request_id)
}

/// 获取队列状态
pub(crate) fn get_queue_snapshot(state: &AppState) -> Result<Vec<ChatQueueEventSummary>, String> {
    let slots = lock_conversation_runtime_slots(state)?;
    let mut summaries = Vec::<ChatQueueEventSummary>::new();
    for slot in slots.values() {
        for event in &slot.pending_queue {
            let message_preview = event
                .messages
                .first()
                .and_then(|msg| {
                    msg.parts.iter().find_map(|part| match part {
                        MessagePart::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                })
                .unwrap_or_default();
            let preview = if message_preview.chars().count() > 50 {
                format!(
                    "{}...",
                    message_preview.chars().take(50).collect::<String>()
                )
            } else {
                message_preview.clone()
            };
            summaries.push(ChatQueueEventSummary {
                id: event.id.clone(),
                source: event.source.clone(),
                queue_mode: event.queue_mode.clone(),
                created_at: event.created_at.clone(),
                message_preview: preview,
                message_text: message_preview,
                conversation_id: event.conversation_id.clone(),
            });
        }
    }
    summaries.sort_by(|left, right| {
        left.created_at
            .cmp(&right.created_at)
            .then_with(|| left.id.cmp(&right.id))
    });

    Ok(summaries)
}

pub(crate) fn emit_chat_queue_snapshot(state: &AppState) {
    let queue_events = get_queue_snapshot(state).unwrap_or_default();
    let session_state = get_main_session_state(state).unwrap_or(MainSessionState::Idle);
    let payload = ChatQueueSnapshotPush {
        queue_events,
        session_state,
    };
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    if let Some(app_handle) = app_handle {
        let _ = app_handle.emit(CHAT_QUEUE_SNAPSHOT_EVENT, payload);
    }
}

/// 将普通队列消息退回输入框
pub(crate) fn recall_queue_event(
    state: &AppState,
    event_id: &str,
) -> Result<Option<ChatPendingEvent>, String> {
    // 队列修改统一走 dequeue_lock -> queue_lock，保证进出队原子顺序一致。
    let _dequeue_guard = state
        .dequeue_lock
        .lock()
        .map_err(|_| "Failed to lock dequeue lock".to_string())?;
    let mut slots = lock_conversation_runtime_slots(state)?;
    let mut removed = None;
    let mut remaining_queue_len = 0usize;
    for slot in slots.values_mut() {
        if let Some(pos) = slot.pending_queue.iter().position(|e| e.id == event_id) {
            if slot.pending_queue[pos].queue_mode == ChatQueueMode::Guided {
                return Err("引导中的消息不能移出队列".to_string());
            }
            removed = slot.pending_queue.remove(pos);
            remaining_queue_len = slot.pending_queue.len();
            break;
        }
    }
    drop(slots);
    if removed.is_some() {
        eprintln!(
            "[聊天调度] 队列消息退回输入框: id={}, queue_len={}",
            event_id, remaining_queue_len
        );
        emit_chat_queue_snapshot(state);
        complete_pending_chat_events_with_error(
            state,
            &[event_id.to_string()],
            "消息已退回输入框",
        )?;
    }
    Ok(removed)
}

pub(crate) fn mark_queue_event_guided(
    state: &AppState,
    event_id: &str,
) -> Result<Option<String>, String> {
    let _dequeue_guard = state
        .dequeue_lock
        .lock()
        .map_err(|_| "Failed to lock dequeue lock".to_string())?;
    let mut slots = lock_conversation_runtime_slots(state)?;
    let mut updated_conversation_id = None::<String>;
    for slot in slots.values_mut() {
        if let Some(event) = slot.pending_queue.iter_mut().find(|item| item.id == event_id) {
            if !matches!(event.source, ChatEventSource::User) {
                return Err("只有用户队列消息可以设置为引导".to_string());
            }
            if event.queue_mode == ChatQueueMode::Guided {
                updated_conversation_id = Some(event.conversation_id.clone());
                break;
            }
            event.queue_mode = ChatQueueMode::Guided;
            event.activate_assistant = true;
            if let Some(runtime_context) = event.runtime_context.as_mut() {
                runtime_context.dispatch_reason = Some("guided_queue".to_string());
            } else {
                event.runtime_context = Some(runtime_context_new("user_message", "guided_queue"));
            }
            updated_conversation_id = Some(event.conversation_id.clone());
            break;
        }
    }
    drop(slots);
    if let Some(conversation_id) = updated_conversation_id.as_deref() {
        runtime_log_info(format!(
            "[引导投送] 开始，任务=mark_queue_event_guided，conversation_id={}，event_id={}",
            conversation_id, event_id
        ));
        emit_chat_queue_snapshot(state);
    }
    Ok(updated_conversation_id)
}

pub(crate) fn clear_conversation_queue(
    state: &AppState,
    conversation_id: &str,
    error_message: &str,
) -> Result<usize, String> {
    let trimmed_conversation_id = conversation_id.trim();
    if trimmed_conversation_id.is_empty() {
        return Ok(0);
    }
    let _dequeue_guard = state
        .dequeue_lock
        .lock()
        .map_err(|_| "Failed to lock dequeue lock".to_string())?;
    let mut slots = lock_conversation_runtime_slots(state)?;
    let Some(slot) = slots.get_mut(trimmed_conversation_id) else {
        return Ok(0);
    };
    let removed_events = slot.pending_queue.drain(..).collect::<Vec<_>>();
    slot.last_activity_at = now_iso();
    let removed_count = removed_events.len();
    let removed_event_ids = removed_events
        .iter()
        .map(|event| event.id.clone())
        .collect::<Vec<_>>();
    drop(slots);
    if removed_count > 0 {
        eprintln!(
            "[聊天调度] 清空会话队列: conversation_id={}, removed_count={}",
            trimmed_conversation_id, removed_count
        );
        emit_chat_queue_snapshot(state);
        complete_pending_chat_events_with_error(state, &removed_event_ids, error_message)?;
    }
    Ok(removed_count)
}

fn claim_guided_queue_events_for_conversation(
    state: &AppState,
    conversation_id: &str,
) -> Result<Vec<ChatPendingEvent>, String> {
    let _dequeue_guard = state
        .dequeue_lock
        .lock()
        .map_err(|_| "Failed to lock dequeue lock".to_string())?;
    let mut claims = lock_conversation_processing_claims(state)?;
    let mut slots = lock_conversation_runtime_slots(state)?;
    let trimmed_conversation_id = conversation_id.trim();
    let slot = conversation_slot_mut(&mut slots, trimmed_conversation_id);
    let has_guided = slot
        .pending_queue
        .iter()
        .any(|event| event.queue_mode == ChatQueueMode::Guided);
    if (slot.state != MainSessionState::Idle && slot.state != MainSessionState::OrganizingContext)
        || !has_guided
        || claims.contains(trimmed_conversation_id)
        || claims.len() >= CHAT_CONCURRENCY_LIMIT
    {
        return Ok(Vec::new());
    }

    claims.insert(trimmed_conversation_id.to_string());
    slot.state = MainSessionState::OrganizingContext;
    slot.last_activity_at = now_iso();

    let mut guided_events = Vec::<ChatPendingEvent>::new();
    let mut remaining_queue = std::collections::VecDeque::<ChatPendingEvent>::new();
    while let Some(mut event) = slot.pending_queue.pop_front() {
        if event.queue_mode == ChatQueueMode::Guided {
            event.activate_assistant = true;
            if let Some(runtime_context) = event.runtime_context.as_mut() {
                runtime_context.dispatch_reason = Some("guided_queue".to_string());
            } else {
                event.runtime_context = Some(runtime_context_new("user_message", "guided_queue"));
            }
            guided_events.push(event);
        } else {
            remaining_queue.push_back(event);
        }
    }
    slot.pending_queue = remaining_queue;

    if guided_events.is_empty() {
        claims.remove(trimmed_conversation_id);
        if slot.pending_queue.is_empty() {
            slot.state = MainSessionState::Idle;
        }
    }

    Ok(guided_events)
}

fn remove_queue_events_by_ids(
    state: &AppState,
    conversation_id: &str,
    event_ids: &[String],
) -> Result<usize, String> {
    if event_ids.is_empty() {
        return Ok(0);
    }
    let id_set = event_ids.iter().map(|item| item.as_str()).collect::<std::collections::HashSet<_>>();
    let _dequeue_guard = state
        .dequeue_lock
        .lock()
        .map_err(|_| "Failed to lock dequeue lock".to_string())?;
    let mut slots = lock_conversation_runtime_slots(state)?;
    let Some(slot) = slots.get_mut(conversation_id) else {
        return Ok(0);
    };
    let before = slot.pending_queue.len();
    slot.pending_queue.retain(|event| !id_set.contains(event.id.as_str()));
    let removed = before.saturating_sub(slot.pending_queue.len());
    if removed > 0 {
        slot.last_activity_at = now_iso();
    }
    Ok(removed)
}

fn conversation_has_guided_queue_events(
    state: &AppState,
    conversation_id: &str,
) -> Result<bool, String> {
    let slots = lock_conversation_runtime_slots(state)?;
    Ok(slots
        .get(conversation_id)
        .map(|slot| {
            slot.pending_queue
                .iter()
                .any(|event| event.queue_mode == ChatQueueMode::Guided)
        })
        .unwrap_or(false))
}

fn tool_loop_should_close_for_guided_queue(
    state: Option<&AppState>,
    context: Option<&ToolLoopAutoCompactionContext>,
) -> bool {
    // 注意：这里判断的不是“当前轮次是否已经完整结束”，
    // 而是“在一次工具执行完成之后，是否存在待插入的引导消息”。
    // 一旦为 true，当前调度应在这个工具切点收口，并把后续回复让位给引导重启的新轮次。
    let Some(state) = state else {
        return false;
    };
    let Some(context) = context else {
        return false;
    };
    conversation_has_guided_queue_events(state, &context.conversation_id).unwrap_or(false)
}

// ==================== 队列管理函数 ====================

pub(crate) fn ingress_chat_event(
    state: &AppState,
    event: ChatPendingEvent,
) -> Result<ChatEventIngress, String> {
    // 原子区间：阻塞判定 +（可选）入队，在同一把流程锁内完成。
    let _dequeue_guard = state
        .dequeue_lock
        .lock()
        .map_err(|_| "Failed to lock dequeue lock".to_string())?;
    let mut claims = lock_conversation_processing_claims(state)?;
    let mut slots = lock_conversation_runtime_slots(state)?;
    let running_count = claims.len();
    let slot = conversation_slot_mut(&mut slots, &event.conversation_id);
    if pending_queue_has_same_request_id(slot, &event) {
        return Ok(ChatEventIngress::Duplicate { event_id: event.id });
    }
    let blocked = slot.state != MainSessionState::Idle
        || !slot.pending_queue.is_empty()
        || conversation_running_slot_count(&claims, &event.conversation_id) > 0
        || running_count >= CHAT_CONCURRENCY_LIMIT;
    if blocked && chat_pending_event_duplicates_existing_message(state, &event)? {
        return Ok(ChatEventIngress::Duplicate { event_id: event.id });
    }
    slot.last_activity_at = now_iso();
    if blocked {
        let event_id = event.id.clone();
        slot.pending_queue.push_back(event);
        return Ok(ChatEventIngress::Queued { event_id });
    }
    claims.insert(event.conversation_id.clone());
    Ok(ChatEventIngress::Direct(event))
}

pub(crate) fn register_chat_event_delta_channel(
    state: &AppState,
    event_id: &str,
    on_delta: tauri::ipc::Channel<AssistantDeltaEvent>,
) -> Result<(), String> {
    state
        .pending_chat_delta_channels
        .lock()
        .map_err(|_| "Failed to lock pending chat delta channels".to_string())?
        .insert(event_id.to_string(), on_delta);
    Ok(())
}

pub(crate) fn register_chat_event_runtime(
    state: &AppState,
    event_id: &str,
    on_delta: tauri::ipc::Channel<AssistantDeltaEvent>,
    sender: tokio::sync::oneshot::Sender<Result<SendChatResult, String>>,
) -> Result<(), String> {
    register_chat_event_delta_channel(state, event_id, on_delta)?;
    state
        .pending_chat_result_senders
        .lock()
        .map_err(|_| "Failed to lock pending chat result senders".to_string())?
        .insert(event_id.to_string(), sender);
    Ok(())
}

pub(crate) fn set_active_chat_view_stream_binding(
    state: &AppState,
    window_label: &str,
    conversation_id: Option<&str>,
    on_delta: tauri::ipc::Channel<AssistantDeltaEvent>,
) -> Result<(), String> {
    let mut bindings = state
        .active_chat_view_bindings
        .lock()
        .map_err(|_| "Failed to lock active chat view bindings".to_string())?;
    let trimmed_window_label = window_label.trim();
    if trimmed_window_label.is_empty() {
        return Err("Missing window label when binding active chat stream".to_string());
    }
    let trimmed_conversation_id = conversation_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    if let Some(conversation_id) = trimmed_conversation_id {
        bindings.insert(
            trimmed_window_label.to_string(),
            ActiveChatViewBinding {
                conversation_id,
                delta_channel: on_delta,
            },
        );
    } else {
        bindings.remove(trimmed_window_label);
    }
    Ok(())
}

fn collect_active_chat_view_delta_channels(
    state: &AppState,
    conversation_id: &str,
) -> Result<Vec<(String, tauri::ipc::Channel<AssistantDeltaEvent>)>, String> {
    let bindings = state
        .active_chat_view_bindings
        .lock()
        .map_err(|_| "Failed to lock active chat view bindings".to_string())?;
    let conversation_id = conversation_id.trim();

    let exact = bindings
        .iter()
        .filter_map(|(window_label, binding)| {
            if binding.conversation_id != conversation_id {
                return None;
            }
            Some((window_label.clone(), binding.delta_channel.clone()))
        })
        .collect::<Vec<_>>();
    if !exact.is_empty() {
        return Ok(exact);
    }

    Ok(bindings
        .iter()
        .filter_map(|(window_label, binding)| {
            if binding.conversation_id != "*" {
                return None;
            }
            Some((window_label.clone(), binding.delta_channel.clone()))
        })
        .collect::<Vec<_>>())
}

fn prune_failed_active_chat_view_bindings(state: &AppState, window_labels: &[String]) {
    if window_labels.is_empty() {
        return;
    }
    if let Ok(mut bindings) = state.active_chat_view_bindings.lock() {
        for window_label in window_labels {
            bindings.remove(window_label);
        }
    }
}

fn conversation_has_focused_chat_view(state: &AppState, conversation_id: &str) -> bool {
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    let Some(app_handle) = app_handle else {
        return false;
    };
    let focused_window_labels = match state.active_chat_view_bindings.lock() {
        Ok(bindings) => bindings
            .iter()
            .filter_map(|(window_label, binding)| {
                if binding.conversation_id.trim() != conversation_id.trim() {
                    return None;
                }
                Some(window_label.clone())
            })
            .collect::<Vec<_>>(),
        Err(_) => return false,
    };
    if focused_window_labels.is_empty() {
        return false;
    }
    if focused_window_labels.iter().any(|window_label| {
        let Some(window) = app_handle.get_webview_window(window_label) else {
            return false;
        };
        let is_visible = window.is_visible().unwrap_or(false);
        let is_focused = window.is_focused().unwrap_or(false);
        is_visible && is_focused
    }) {
        return true;
    }
    // VS Code 侧边栏通过 WebSocket 连接，不在 active_chat_view_bindings 中，
    // 但会注册到 detached_chat_windows；只要会话已打开就应跳过通知。
    detached_chat_window_for_conversation(conversation_id).is_some()
}

fn emit_assistant_delta_app_event(
    state: &AppState,
    conversation_id: &str,
    event: &AssistantDeltaEvent,
) {
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    let Some(app_handle) = app_handle else {
        return;
    };
    let payload = serde_json::json!({
        "conversationId": conversation_id,
        "event": event,
    });
    ide_chat_broadcast_notification("chat.assistantDelta", payload.clone());
    let _ = app_handle.emit(CHAT_ASSISTANT_DELTA_EVENT, payload);
}

fn should_emit_assistant_delta_via_app_event_only(event: &AssistantDeltaEvent) -> bool {
    matches!(
        event.kind.as_deref(),
        Some("tool_status") | Some("stream_rebind_required")
    )
}

fn is_visible_stream_progress_event(event: &AssistantDeltaEvent) -> bool {
    if !event.delta.trim().is_empty() {
        return true;
    }
    matches!(
        event.kind.as_deref(),
        Some("reasoning_standard") | Some("reasoning_inline")
    )
}

fn stream_cache_has_visible_progress(cache: &ConversationStreamRuntimeCache) -> bool {
    !cache.assistant_text.trim().is_empty()
        || !cache.reasoning_standard.trim().is_empty()
        || !cache.reasoning_inline.trim().is_empty()
        || !cache.tool_status_text.trim().is_empty()
        || !cache.tool_status_state.trim().is_empty()
        || !cache.stream_tool_calls.is_empty()
}

fn now_unix_ms() -> u64 {
    let millis = now_utc().unix_timestamp_nanos() / 1_000_000;
    if millis <= 0 {
        0
    } else {
        millis.min(i128::from(u64::MAX)) as u64
    }
}

fn event_tool_call_id(event: &AssistantDeltaEvent) -> Option<&str> {
    event
        .tool_call_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn find_stream_tool_call_cache_index(
    calls: &[ConversationStreamToolCallRuntimeCache],
    tool_call_id: &str,
) -> Option<usize> {
    calls.iter().position(|call| {
        call.tool_call_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            == Some(tool_call_id)
    })
}

fn reset_conversation_stream_runtime_cache(
    state: &AppState,
    conversation_id: &str,
    activation_id: &str,
    request_id: &str,
    started_at: &str,
    started_at_ms: u64,
) -> Result<(), String> {
    let mut slots = lock_conversation_runtime_slots(state)?;
    let slot = conversation_slot_mut(&mut slots, conversation_id);
    slot.stream_cache = ConversationStreamRuntimeCache {
        activation_id: activation_id.trim().to_string(),
        request_id: request_id.trim().to_string(),
        started_at: started_at.trim().to_string(),
        started_at_ms,
        updated_at: started_at.trim().to_string(),
        ..ConversationStreamRuntimeCache::default()
    };
    Ok(())
}

fn set_stream_cache_persisted_assistant_message_id(
    state: &AppState,
    conversation_id: &str,
    assistant_message_id: &str,
) {
    let mut slots = match lock_conversation_runtime_slots(state) {
        Ok(slots) => slots,
        Err(err) => {
            eprintln!("[聊天流式缓存] 更新 persisted_assistant_message_id 失败，锁错误: {err}");
            return;
        }
    };
    let cid = conversation_id.trim();
    if cid.is_empty() {
        return;
    }
    let slot = conversation_slot_mut(&mut slots, cid);
    slot.stream_cache.persisted_assistant_message_id = assistant_message_id.trim().to_string();
}

fn clear_conversation_stream_runtime_cache(
    state: &AppState,
    conversation_id: &str,
) -> Result<(), String> {
    let mut slots = lock_conversation_runtime_slots(state)?;
    if let Some(slot) = slots.get_mut(conversation_id.trim()) {
        slot.stream_cache = ConversationStreamRuntimeCache::default();
    }
    Ok(())
}

fn update_conversation_stream_runtime_cache(
    state: &AppState,
    conversation_id: &str,
    event: &AssistantDeltaEvent,
) -> Result<(), String> {
    let cid = conversation_id.trim();
    if cid.is_empty() {
        return Ok(());
    }
    if event.kind.as_deref() == Some("round_completed")
        || event.kind.as_deref() == Some("round_failed")
        || event.kind.as_deref() == Some("history_flushed")
        || event.kind.as_deref() == Some("stream_rebind_required")
    {
        return Ok(());
    }
    let has_progress = is_visible_stream_progress_event(event)
        || event.kind.as_deref() == Some("tool_status");
    if !has_progress {
        return Ok(());
    }

    let mut slots = lock_conversation_runtime_slots(state)?;
    let slot = conversation_slot_mut(&mut slots, cid);
    let cache = &mut slot.stream_cache;
    if let Some(value) = event
        .activation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        cache.activation_id = value.to_string();
    }
    if let Some(value) = event
        .request_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        cache.request_id = value.to_string();
    }
    match event.kind.as_deref() {
        Some("tool_status") => {
            let tool_name = event.tool_name.as_deref().map(str::trim).unwrap_or("");
            let tool_status = event.tool_status.as_deref().unwrap_or("").trim();
            let tool_args = event.tool_args.clone().unwrap_or_default();
            let tool_call_id = event_tool_call_id(event);
            if let Some(tool_call_id) = tool_call_id {
                if tool_status == "running" && !tool_name.is_empty() {
                    cache.stream_last_tool_name = tool_name.to_string();
                    if let Some(index) =
                        find_stream_tool_call_cache_index(&cache.stream_tool_calls, tool_call_id)
                    {
                        if let Some(call) = cache.stream_tool_calls.get_mut(index) {
                            call.name = tool_name.to_string();
                            call.args_text = tool_args.clone();
                            call.tool_call_id = Some(tool_call_id.to_string());
                            call.status = Some("doing".to_string());
                        }
                    } else {
                        cache.stream_tool_call_count += 1;
                        cache
                            .stream_tool_calls
                            .push(ConversationStreamToolCallRuntimeCache {
                                name: tool_name.to_string(),
                                args_text: tool_args.clone(),
                                tool_call_id: Some(tool_call_id.to_string()),
                                status: Some("doing".to_string()),
                            });
                    }
                } else if (tool_status == "done" || tool_status == "failed")
                    && !tool_name.is_empty()
                {
                    if let Some(index) =
                        find_stream_tool_call_cache_index(&cache.stream_tool_calls, tool_call_id)
                    {
                        if let Some(call) = cache.stream_tool_calls.get_mut(index) {
                            call.name = tool_name.to_string();
                            if !tool_args.is_empty() {
                                call.args_text = tool_args.clone();
                            }
                            call.tool_call_id = Some(tool_call_id.to_string());
                            call.status = Some("done".to_string());
                        }
                    }
                }
            }
            cache.tool_status_text = event.message.clone().unwrap_or_default();
            cache.tool_status_state = match tool_status {
                "running" | "done" | "failed" => tool_status.to_string(),
                _ => String::new(),
            };
        }
        Some("reasoning_standard") => {
            cache.reasoning_standard.push_str(&event.delta);
        }
        Some("reasoning_inline") => {
            cache.reasoning_inline.push_str(&event.delta);
        }
        _ => {
            cache.assistant_text.push_str(&event.delta);
        }
    }
    cache.updated_at = now_iso();
    Ok(())
}

fn dispatch_assistant_delta_to_active_view(
    state: &AppState,
    conversation_id: &str,
    event: &AssistantDeltaEvent,
) {
    emit_assistant_delta_app_event(state, conversation_id, event);

    if should_emit_assistant_delta_via_app_event_only(event) {
        return;
    }

    let targets =
        collect_active_chat_view_delta_channels(state, conversation_id).unwrap_or_default();
    if targets.is_empty() {
        return;
    }

    let mut delivered = false;
    let mut failed_labels = Vec::<String>::new();
    for (window_label, channel) in targets {
        match channel.send(event.clone()) {
            Ok(_) => {
                delivered = true;
            }
            Err(_) => {
                failed_labels.push(window_label);
            }
        }
    }
    prune_failed_active_chat_view_bindings(state, &failed_labels);
    let _ = delivered;
}

fn emit_stream_rebind_required_event(
    state: &AppState,
    conversation_id: &str,
    request_id: Option<&str>,
    phase_id: Option<&str>,
    reason: &str,
) {
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    let Some(app_handle) = app_handle else {
        return;
    };
    let payload = serde_json::json!({
        "conversationId": conversation_id,
        "requestId": request_id.map(str::trim).filter(|value| !value.is_empty()),
        "phaseId": phase_id.map(str::trim).filter(|value| !value.is_empty()),
        "reason": reason.trim(),
    });
    ide_chat_broadcast_notification("chat.streamRebindRequired", payload.clone());
    runtime_log_info(format!(
        "[聊天流式重绑] 发送普通事件 conversation_id={} request_id={} phase_id={} reason={}",
        conversation_id.trim(),
        request_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(""),
        phase_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(""),
        reason.trim(),
    ));
    match app_handle.emit(CHAT_STREAM_REBIND_REQUIRED_EVENT, payload) {
        Ok(_) => {
            runtime_log_info(format!(
                "[聊天流式重绑] 普通事件发送成功 conversation_id={} request_id={} phase_id={} reason={}",
                conversation_id.trim(),
                request_id.map(str::trim).filter(|value| !value.is_empty()).unwrap_or(""),
                phase_id.map(str::trim).filter(|value| !value.is_empty()).unwrap_or(""),
                reason.trim(),
            ));
        }
        Err(err) => {
            runtime_log_error(format!(
                "[聊天流式重绑] 普通事件发送失败 conversation_id={} request_id={} phase_id={} reason={} error={}",
                conversation_id.trim(),
                request_id.map(str::trim).filter(|value| !value.is_empty()).unwrap_or(""),
                phase_id.map(str::trim).filter(|value| !value.is_empty()).unwrap_or(""),
                reason.trim(),
                err
            ));
        }
    }
}

#[allow(dead_code)]
pub(crate) fn clear_active_chat_view_stream_binding(
    state: &AppState,
    window_label: &str,
) -> Result<(), String> {
    let mut bindings = state
        .active_chat_view_bindings
        .lock()
        .map_err(|_| "Failed to lock active chat view bindings".to_string())?;
    bindings.remove(window_label.trim());
    Ok(())
}

pub(crate) fn trigger_chat_queue_processing(state: &AppState) {
    let state_clone = state.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(err) = process_chat_queue(&state_clone).await {
            eprintln!("[聊天调度] process_chat_queue 失败: {}", err);
        }
    });
}

pub(crate) fn is_chat_event_queued(state: &AppState, event_id: &str) -> Result<bool, String> {
    let slots = lock_conversation_runtime_slots(state)?;
    Ok(slots
        .values()
        .any(|slot| slot.pending_queue.iter().any(|item| item.id == event_id)))
}

pub(crate) async fn process_chat_queue_for_event(state: &AppState, event_id: &str) {
    if let Err(err) = process_chat_queue(state).await {
        eprintln!("[聊天调度] process_chat_queue 失败: {}", err);
    }
    if is_chat_event_queued(state, event_id).unwrap_or(false) {
        emit_chat_queue_snapshot(state);
    }
}

pub(crate) async fn process_chat_event_after_ingress(state: &AppState, ingress: ChatEventIngress) {
    match ingress {
        ChatEventIngress::Direct(event) => {
            let conversation_id = event.conversation_id.clone();
            if let Err(err) =
                process_claimed_conversation_batch(state, &conversation_id, vec![event]).await
            {
                eprintln!("[聊天调度] 处理直接事件失败: {}", err);
            }
        }
        ChatEventIngress::Queued { event_id } => {
            process_chat_queue_for_event(state, &event_id).await;
        }
        ChatEventIngress::Duplicate { event_id } => {
            let _ = complete_pending_chat_events_with_error(
                state,
                &[event_id],
                "重复消息已忽略",
            );
            emit_chat_queue_snapshot(state);
        }
    }
}

pub(crate) fn trigger_chat_event_after_ingress(state: &AppState, ingress: ChatEventIngress) {
    let state_clone = state.clone();
    tauri::async_runtime::spawn(async move {
        process_chat_event_after_ingress(&state_clone, ingress).await;
    });
}

async fn process_claimed_conversation_batch(
    state: &AppState,
    conversation_id: &str,
    events: Vec<ChatPendingEvent>,
) -> Result<(), String> {
    let result = process_conversation_batch(state, conversation_id, events).await;
    if let Err(release_err) = release_conversation_processing_claim(state, conversation_id) {
        eprintln!(
            "[聊天调度] 释放会话处理声明失败: conversation_id={}, error={}",
            conversation_id, release_err
        );
    }
    emit_chat_queue_snapshot(state);
    if conversation_has_guided_queue_events(state, conversation_id).unwrap_or(false) {
        trigger_guided_queue_processing(state, conversation_id);
    } else {
        trigger_chat_queue_processing(state);
    }
    result
}

fn resolve_activation_reason(runtime_context: &RuntimeContext) -> String {
    let dispatch_reason = runtime_context
        .dispatch_reason
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    match dispatch_reason {
        "user_send" => "user_send",
        "task_due" => "task_due",
        "remote_im_followup" => "remote_im_followup",
        "context_compaction_followup" => "context_compaction_followup",
        "guided_queue" => "guided_queue",
        "delegate_continue" => "delegate_continue",
        _ => "queue_dispatch",
    }
    .to_string()
}

async fn process_guided_queue_after_round(
    state: &AppState,
    conversation_id: &str,
    guided_events: Vec<ChatPendingEvent>,
) -> Result<bool, String> {
    if guided_events.is_empty() {
        return Ok(false);
    }
    let guided_event_count = guided_events.len();
    let started_at = std::time::Instant::now();
    runtime_log_info(format!(
        "[引导投送] 开始，任务=process_guided_queue_after_round，conversation_id={}，event_count={}",
        conversation_id,
        guided_event_count
    ));
    process_conversation_batch(state, conversation_id, guided_events).await?;
    runtime_log_info(format!(
        "[引导投送] 完成，任务=process_guided_queue_after_round，conversation_id={}，event_count={}，duration_ms={}",
        conversation_id,
        guided_event_count,
        started_at.elapsed().as_millis()
    ));
    Ok(true)
}

async fn process_guided_queue_when_idle(
    state: &AppState,
    conversation_id: &str,
) -> Result<(), String> {
    let guided_events = claim_guided_queue_events_for_conversation(state, conversation_id)?;
    if guided_events.is_empty() {
        return Ok(());
    }
    emit_chat_queue_snapshot(state);

    let result = process_guided_queue_after_round(state, conversation_id, guided_events).await;
    if let Err(release_err) = release_conversation_processing_claim(state, conversation_id) {
        runtime_log_warn(format!(
            "[引导投送] 失败，任务=release_guided_claim，conversation_id={}，error={}",
            conversation_id, release_err
        ));
    }
    let next_state = if conversation_has_guided_queue_events(state, conversation_id).unwrap_or(false)
    {
        MainSessionState::OrganizingContext
    } else {
        MainSessionState::Idle
    };
    if let Err(state_err) = set_conversation_runtime_state(state, conversation_id, next_state) {
        runtime_log_warn(format!(
            "[引导投送] 失败，任务=restore_guided_runtime_state，conversation_id={}，error={}",
            conversation_id, state_err
        ));
    }
    emit_chat_queue_snapshot(state);
    if conversation_has_guided_queue_events(state, conversation_id).unwrap_or(false) {
        trigger_guided_queue_processing(state, conversation_id);
    } else {
        trigger_chat_queue_processing(state);
    }
    result.map(|_| ())
}

pub(crate) fn trigger_guided_queue_processing(state: &AppState, conversation_id: &str) {
    let state_clone = state.clone();
    let conversation_id = conversation_id.to_string();
    tauri::async_runtime::spawn(async move {
        if let Err(err) = process_guided_queue_when_idle(&state_clone, &conversation_id).await {
            runtime_log_warn(format!(
                "[引导投送] 失败，任务=trigger_guided_queue_processing，conversation_id={}，error={}",
                conversation_id, err
            ));
        }
    });
}

// ==================== 状态机管理函数 ====================

/// 获取当前状态
pub(crate) fn get_main_session_state(state: &AppState) -> Result<MainSessionState, String> {
    let slots = lock_conversation_runtime_slots(state)?;
    if slots
        .values()
        .any(|slot| slot.state == MainSessionState::OrganizingContext)
    {
        return Ok(MainSessionState::OrganizingContext);
    }
    if slots
        .values()
        .any(|slot| slot.state == MainSessionState::AssistantStreaming)
    {
        return Ok(MainSessionState::AssistantStreaming);
    }
    Ok(MainSessionState::Idle)
}

pub(crate) fn get_conversation_runtime_state(
    state: &AppState,
    conversation_id: &str,
) -> Result<MainSessionState, String> {
    let slots = lock_conversation_runtime_slots(state)?;
    Ok(slots
        .get(conversation_id)
        .map(|slot| slot.state.clone())
        .unwrap_or(MainSessionState::Idle))
}

pub(crate) fn read_conversation_runtime_snapshot(
    state: &AppState,
    conversation_id: &str,
) -> Result<ConversationRuntimeSnapshot, String> {
    let cid = conversation_id.trim();
    let claims = lock_conversation_processing_claims(state)?;
    let is_processing = claims.contains(cid);
    drop(claims);
    let slots = lock_conversation_runtime_slots(state)?;
    let (runtime_state, pending_queue_count, stream_cache) = slots
        .get(cid)
        .map(|slot| {
            (
                slot.state.clone(),
                slot.pending_queue.len(),
                slot.stream_cache.clone(),
            )
        })
        .unwrap_or_else(|| {
            (
                MainSessionState::Idle,
                0,
                ConversationStreamRuntimeCache::default(),
            )
        });
    let has_visible_progress = stream_cache_has_visible_progress(&stream_cache);
    let stream_cache = ConversationStreamRuntimeCacheSnapshot {
        activation_id: stream_cache.activation_id,
        request_id: stream_cache.request_id,
        assistant_text: stream_cache.assistant_text,
        reasoning_standard: stream_cache.reasoning_standard,
        reasoning_inline: stream_cache.reasoning_inline,
        tool_status_text: stream_cache.tool_status_text,
        tool_status_state: stream_cache.tool_status_state,
        stream_tool_calls: stream_cache.stream_tool_calls,
        stream_tool_call_count: stream_cache.stream_tool_call_count,
        stream_last_tool_name: stream_cache.stream_last_tool_name,
        started_at: stream_cache.started_at,
        started_at_ms: stream_cache.started_at_ms,
        updated_at: stream_cache.updated_at,
        has_visible_progress,
        persisted_assistant_message_id: stream_cache.persisted_assistant_message_id,
    };
    Ok(ConversationRuntimeSnapshot {
        conversation_id: cid.to_string(),
        runtime_state,
        is_processing,
        has_pending_queue: pending_queue_count > 0,
        pending_queue_count,
        stream_cache,
    })
}

/// 设置会话状态并记录日志
pub(crate) fn set_conversation_runtime_state(
    state: &AppState,
    conversation_id: &str,
    new_state: MainSessionState,
) -> Result<(), String> {
    let (old_state_cn, new_state_cn) = {
        let mut slots = lock_conversation_runtime_slots(state)?;
        let slot = conversation_slot_mut(&mut slots, conversation_id);
        let old_state = slot.state.clone();
        slot.state = new_state.clone();
        slot.last_activity_at = now_iso();

        let old_state_cn = match old_state {
            MainSessionState::Idle => "空闲",
            MainSessionState::AssistantStreaming => "助理流式输出",
            MainSessionState::OrganizingContext => "整理上下文",
        };
        let new_state_cn = match new_state {
            MainSessionState::Idle => "空闲",
            MainSessionState::AssistantStreaming => "助理流式输出",
            MainSessionState::OrganizingContext => "整理上下文",
        };
        (old_state_cn, new_state_cn)
    };

    eprintln!(
        "[聊天调度] 会话状态转换: conversation_id={}, {} -> {}",
        conversation_id, old_state_cn, new_state_cn
    );

    emit_chat_queue_snapshot(state);
    Ok(())
}

fn remote_im_activation_source_key(source: &RemoteImActivationSource) -> String {
    format!(
        "{}::{}::{}",
        source.channel_id.trim(),
        source.remote_contact_type.trim(),
        source.remote_contact_id.trim()
    )
}

fn remote_im_activation_source_from_sender(
    sender: &RemoteImMessageSource,
) -> RemoteImActivationSource {
    RemoteImActivationSource {
        channel_id: sender.channel_id.trim().to_string(),
        platform: sender.platform.clone(),
        remote_contact_type: sender.remote_contact_type.trim().to_string(),
        remote_contact_id: sender.remote_contact_id.trim().to_string(),
        remote_contact_name: sender.remote_contact_name.trim().to_string(),
    }
}

pub(crate) fn resolve_bound_remote_im_activation_source(
    sources: &[RemoteImActivationSource],
) -> Option<RemoteImActivationSource> {
    if sources.len() == 1 {
        return sources.first().cloned();
    }
    None
}

pub(crate) fn set_conversation_remote_im_activation_sources(
    state: &AppState,
    conversation_id: &str,
    sources: Vec<RemoteImActivationSource>,
) -> Result<(), String> {
    let mut slots = lock_conversation_runtime_slots(state)?;
    let slot = conversation_slot_mut(&mut slots, conversation_id);
    slot.active_remote_im_activation_sources = sources;
    slot.last_activity_at = now_iso();
    Ok(())
}

pub(crate) fn get_conversation_remote_im_activation_sources(
    state: &AppState,
    conversation_id: &str,
) -> Result<Vec<RemoteImActivationSource>, String> {
    let normalized_conversation_id = conversation_id.trim();
    if normalized_conversation_id.is_empty() {
        return Ok(Vec::new());
    }
    let slots = lock_conversation_runtime_slots(state)?;
    Ok(slots
        .get(normalized_conversation_id)
        .map(|slot| slot.active_remote_im_activation_sources.clone())
        .unwrap_or_default())
}

pub(crate) fn set_conversation_remote_im_assistant_context(
    state: &AppState,
    conversation_id: &str,
    context: Option<RemoteImConversationAssistantContext>,
) -> Result<(), String> {
    let normalized_conversation_id = conversation_id.trim();
    let mut slots = lock_conversation_runtime_slots(state)?;
    let slot = conversation_slot_mut(&mut slots, normalized_conversation_id);
    slot.active_remote_im_assistant_context = context;
    slot.last_activity_at = now_iso();
    Ok(())
}

pub(crate) fn get_conversation_remote_im_assistant_context(
    state: &AppState,
    conversation_id: &str,
) -> Result<Option<RemoteImConversationAssistantContext>, String> {
    let normalized_conversation_id = conversation_id.trim();
    if normalized_conversation_id.is_empty() {
        return Ok(None);
    }
    let slots = lock_conversation_runtime_slots(state)?;
    Ok(slots
        .get(normalized_conversation_id)
        .and_then(|slot| slot.active_remote_im_assistant_context.clone()))
}

pub(crate) fn set_conversation_plan_mode_enabled(
    state: &AppState,
    conversation_id: &str,
    enabled: bool,
) -> Result<(), String> {
    let normalized_conversation_id = conversation_id.trim();
    let mut slots = lock_conversation_runtime_slots(state)?;
    let slot = conversation_slot_mut(&mut slots, normalized_conversation_id);
    slot.plan_mode_enabled = enabled;
    slot.last_activity_at = now_iso();
    Ok(())
}

pub(crate) fn get_conversation_plan_mode_enabled(
    state: &AppState,
    conversation_id: &str,
) -> Result<bool, String> {
    let normalized_conversation_id = conversation_id.trim();
    {
        let slots = lock_conversation_runtime_slots(state)?;
        if let Some(slot) = slots.get(normalized_conversation_id) {
            return Ok(slot.plan_mode_enabled);
        }
    }
    Ok(state_read_conversation_cached(state, normalized_conversation_id)
        .map(|conversation| conversation.plan_mode_enabled)
        .unwrap_or(false))
}

fn collect_activated_remote_im_sources(
    events: &[ChatPendingEvent],
    event_activate_flags: &[bool],
) -> Vec<RemoteImActivationSource> {
    let mut activated_remote_im_sources = Vec::<RemoteImActivationSource>::new();
    let mut activated_remote_im_source_keys = std::collections::HashSet::<String>::new();
    for (event, should_activate) in events.iter().zip(event_activate_flags.iter().copied()) {
        if !should_activate || !matches!(event.source, ChatEventSource::RemoteIm) {
            continue;
        }
        let Some(sender) = event.sender_info.as_ref() else {
            continue;
        };
        let source = remote_im_activation_source_from_sender(sender);
        let source_key = remote_im_activation_source_key(&source);
        if activated_remote_im_source_keys.insert(source_key) {
            activated_remote_im_sources.push(source);
        }
    }
    activated_remote_im_sources
}

fn remote_im_source_has_pending_queue_event(
    state: &AppState,
    conversation_id: &str,
    source: &RemoteImActivationSource,
) -> bool {
    let Ok(slots) = lock_conversation_runtime_slots(state) else {
        return false;
    };
    let Some(slot) = slots.get(conversation_id.trim()) else {
        return false;
    };
    let source_key = remote_im_activation_source_key(source);
    slot.pending_queue.iter().any(|event| {
        matches!(event.source, ChatEventSource::RemoteIm)
            && event
                .sender_info
                .as_ref()
                .map(|sender| {
                    remote_im_activation_source_key(&remote_im_activation_source_from_sender(sender))
                        == source_key
                })
                .unwrap_or(false)
    })
}

fn filter_remote_im_follow_up_sources_for_pending_queue(
    state: &AppState,
    conversation_id: &str,
    sources: Vec<RemoteImActivationSource>,
) -> Vec<RemoteImActivationSource> {
    sources
        .into_iter()
        .filter(|source| {
            let has_pending_queue =
                remote_im_source_has_pending_queue_event(state, conversation_id, source);
            if has_pending_queue {
                runtime_log_info(format!(
                    "[远程联系人状态机] 待办续跑跳过: conversation_id={}，remote_contact_id={}，reason=等待队列消息先写入历史",
                    conversation_id,
                    source.remote_contact_id
                ));
            }
            !has_pending_queue
        })
        .collect()
}

fn release_conversation_processing_claim(
    state: &AppState,
    conversation_id: &str,
) -> Result<(), String> {
    let mut claims = lock_conversation_processing_claims(state)?;
    claims.remove(conversation_id.trim());
    Ok(())
}

fn has_normal_user_queue_event(events: &std::collections::VecDeque<ChatPendingEvent>) -> bool {
    events.iter().any(|event| {
        event.queue_mode == ChatQueueMode::Normal && matches!(event.source, ChatEventSource::User)
    })
}

fn claim_queued_conversation_batches(
    state: &AppState,
) -> Result<Vec<(String, Vec<ChatPendingEvent>)>, String> {
    let _dequeue_guard = state
        .dequeue_lock
        .lock()
        .map_err(|_| "Failed to lock dequeue lock".to_string())?;
    let mut claims = lock_conversation_processing_claims(state)?;
    if claims.len() >= CHAT_CONCURRENCY_LIMIT {
        return Ok(Vec::new());
    }
    let available_slots = CHAT_CONCURRENCY_LIMIT.saturating_sub(claims.len());
    let mut slots = lock_conversation_runtime_slots(state)?;
    let mut eligible = slots
        .iter()
        .filter_map(|(conversation_id, slot)| {
            let has_guided = slot
                .pending_queue
                .iter()
                .any(|event| event.queue_mode == ChatQueueMode::Guided);
            if slot.state != MainSessionState::Idle
                || slot.pending_queue.is_empty()
                || (has_guided && !has_normal_user_queue_event(&slot.pending_queue))
                || claims.contains(conversation_id)
            {
                return None;
            }
            let created_at = slot
                .pending_queue
                .front()
                .map(|event| event.created_at.clone())
                .unwrap_or_default();
            Some((conversation_id.clone(), created_at))
        })
        .collect::<Vec<_>>();
    eligible.sort_by(|left, right| left.1.cmp(&right.1).then_with(|| left.0.cmp(&right.0)));

    let mut claimed_batches = Vec::<(String, Vec<ChatPendingEvent>)>::new();
    for (conversation_id, _) in eligible.into_iter().take(available_slots) {
        let slot = conversation_slot_mut(&mut slots, &conversation_id);
        let batch = slot.pending_queue.drain(..).collect::<Vec<_>>();
        if batch.is_empty() {
            continue;
        }
        slot.last_activity_at = now_iso();
        claims.insert(conversation_id.clone());
        claimed_batches.push((conversation_id, batch));
    }
    Ok(claimed_batches)
}

// ==================== 出队调度器 ====================

/// 主出队处理函数
///
/// 语义上，它是在做“下一轮候选输入结算”：
/// 1. 把当前门口的所有消息先收进来；
/// 2. 按会话分别批处理；
/// 3. 每个会话先批量写正式历史；
/// 4. 再决定该会话是否需要开启新的主助理轮次。
pub(crate) async fn process_chat_queue(state: &AppState) -> Result<(), String> {
    let claimed_batches = claim_queued_conversation_batches(state)?;
    if claimed_batches.is_empty() {
        return Ok(());
    }
    emit_chat_queue_snapshot(state);
    for (conversation_id, events) in claimed_batches {
        let state_clone = state.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(err) =
                process_claimed_conversation_batch(&state_clone, &conversation_id, events).await
            {
                eprintln!("[聊天调度] 处理会话失败 {}: {}", conversation_id, err);
            }
        });
    }
    Ok(())
}

/// 处理单个会话的批次
///
/// 这里严格遵守“先历史，后激活”的顺序：
/// 1. 不论是否需要激活主助理，先把整批消息写入正式历史；
/// 2. 写入时统一刷新 created_at，确保消息生效时间以入历史为准；
/// 3. 然后再判断 should_activate：
///    - false：只更新历史，不开启流式；
///    - true：先通知前端历史已落地，再开启新的主助理轮次。
async fn process_conversation_batch(
    state: &AppState,
    conversation_id: &str,
    events: Vec<ChatPendingEvent>,
) -> Result<(), String> {
    let event_ids = events
        .iter()
        .map(|event| event.id.clone())
        .collect::<Vec<_>>();
    let latest_user_text = latest_user_text_from_events(&events);
    let history_flush_time = now_iso();
    let oldest_queue_created_at = events
        .iter()
        .map(|event| event.created_at.trim())
        .find(|value| !value.is_empty())
        .unwrap_or("");
    fn defer_image_parts_for_history_flushed(
        message: &ChatMessage,
        data_path: &PathBuf,
    ) -> (ChatMessage, usize, usize) {
        let mut deferred_message = message.clone();
        let mut deferred_image_count = 0usize;
        let mut deferred_base64_chars = 0usize;
        let mut next_parts = Vec::<MessagePart>::with_capacity(deferred_message.parts.len());

        for part in deferred_message.parts.drain(..) {
            match part {
                MessagePart::Image {
                    mime,
                    bytes_base64,
                    name,
                    compressed,
                } => {
                    let next_ref =
                        externalize_stored_binary_base64(data_path, &mime, &bytes_base64)
                            .unwrap_or(bytes_base64.clone());
                    if next_ref != bytes_base64 {
                        deferred_image_count += 1;
                        deferred_base64_chars += bytes_base64.len();
                    }
                    next_parts.push(MessagePart::Image {
                        mime,
                        bytes_base64: next_ref,
                        name,
                        compressed,
                    });
                }
                other => next_parts.push(other),
            }
        }

        if deferred_image_count > 0 {
            let mut provider_meta = deferred_message
                .provider_meta
                .take()
                .unwrap_or_else(|| serde_json::json!({}));
            if !provider_meta.is_object() {
                provider_meta = serde_json::json!({});
            }
            if let Some(obj) = provider_meta.as_object_mut() {
                obj.insert(
                    "historyFlushedImageDeferred".to_string(),
                    serde_json::json!(true),
                );
                obj.insert(
                    "historyFlushedDeferredImageCount".to_string(),
                    serde_json::json!(deferred_image_count),
                );
            }
            deferred_message.provider_meta = Some(provider_meta);
        }

        deferred_message.parts = next_parts;
        (
            deferred_message,
            deferred_image_count,
            deferred_base64_chars,
        )
    }

    // 1. 先写入所有消息到会话记录。
    //
    // 这里统一覆盖 created_at 为 history_flush_time，
    // 目的是把“正式进入历史的时间”作为消息的业务生效时间。
    // 入队时间只用于队列观察，不用于正式会话排序和轮次判断。
    let conversation = match state_read_conversation_cached(state, conversation_id) {
        Ok(conversation) if conversation.summary.trim().is_empty() => conversation,
        _ => {
            complete_pending_chat_events_with_error(
                state,
                &event_ids,
                &format!("目标会话不存在，conversationId={conversation_id}"),
            )?;
            return Err(format!("目标会话不存在，conversationId={conversation_id}"));
        }
    };
    let scheduler_agents = state_read_agents_cached(state)?;
    let has_summary_context = conversation
        .messages
        .iter()
        .any(|message| is_context_compaction_message(message, message.role.trim()));
    let should_seed_summary_context = !has_summary_context
        && !conversation_is_delegate(&conversation)
        && !conversation_is_remote_im_contact(&conversation);
    let summary_seed_agent = if should_seed_summary_context
        && conversation.user_profile_snapshot.trim().is_empty()
    {
        scheduler_agents
            .iter()
            .find(|item| item.id == conversation.agent_id)
            .cloned()
    } else {
        None
    };
    let seeded_profile_snapshot = if let Some(agent) = summary_seed_agent.as_ref() {
        match with_memory_lock(state, "scheduler_profile_snapshot", || {
            build_user_profile_snapshot_block(&state.data_path, agent, 12)
        }) {
            Ok(snapshot) => snapshot,
            Err(err) => {
                runtime_log_error(format!(
                    "[用户画像] 失败，任务=seed_scheduler_profile_snapshot，conversation_id={}，agent_id={}，error={}",
                    conversation_id,
                    agent.id,
                    err
                ));
                None
            }
        }
    } else {
        None
    };
    let mut prepared_batches = Vec::<Vec<(ChatMessage, Vec<String>)>>::with_capacity(events.len());
    for event in &events {
        let mut prepared_messages =
            Vec::<(ChatMessage, Vec<String>)>::with_capacity(event.messages.len());
        for message in &event.messages {
            let mut persisted = message.clone();
            externalize_message_parts_to_media_refs(&mut persisted.parts, &state.data_path)?;
            persisted.created_at = history_flush_time.clone();
            let recall_payload = if persisted.role.trim() == "user" {
                with_memory_lock(state, "scheduler_user_message_recall", || {
                    collect_recall_payload_for_user_message(
                        &state.data_path,
                        &scheduler_agents,
                        &event.session_info.agent_id,
                        &persisted,
                    )
                })?
            } else {
                UserMessageRecallPayload::default()
            };
            if !recall_payload.stored_ids.is_empty() {
                write_retrieved_memory_ids_into_provider_meta(
                    &mut persisted.provider_meta,
                    &recall_payload.stored_ids,
                );
            }
            prepared_messages.push((persisted, recall_payload.raw_ids));
        }
        prepared_batches.push(prepared_messages);
    }
    let commit_result = conversation_service().commit_scheduler_history_flush(
        state,
        conversation_id,
        &events,
        prepared_batches,
        &history_flush_time,
        should_seed_summary_context,
        seeded_profile_snapshot.as_deref(),
    )?;
    let persisted_batch_messages = commit_result.persisted_batch_messages;
    let event_activate_flags = commit_result.event_activate_flags;

    // 2. 判断是否需要激活主助理。
    // 这一步故意放在“写历史之后”，避免出现前端先开流式、
    // 但本批消息还没正式落入历史的时序错乱。
    let activated_remote_im_sources =
        collect_activated_remote_im_sources(&events, &event_activate_flags);
    let mut should_activate = event_activate_flags.iter().copied().any(|v| v);
    let mut remote_im_skip_decision = None::<String>;
    let mut activating_session_info = events.first().map(|event| event.session_info.clone());
    if should_activate && !activated_remote_im_sources.is_empty() {
        if let Some(contact) = remote_im_resolve_secretary_contact(state, &activated_remote_im_sources)? {
            let scheduler_config = state_read_config_cached(state)?;
            match remote_im_resolve_contact_assistant_context(
                &scheduler_config,
                &scheduler_agents,
                &contact,
            ) {
                Ok(resolved_assistant) => {
                    set_conversation_remote_im_assistant_context(
                        state,
                        conversation_id,
                        Some(resolved_assistant),
                    )?;
                }
                Err(err) => {
                    should_activate = false;
                    remote_im_skip_decision = Some("no_reply".to_string());
                    set_conversation_remote_im_assistant_context(state, conversation_id, None)?;
                    runtime_log_warn(format!(
                        "[远程联系人入场] 当前助理上下文非法，跳过本轮激活: conversation_id={}, contact_id={}, error={}",
                        conversation_id, contact.id, err
                    ));
                    remote_im_append_channel_log(
                        &contact.channel_id,
                        "warn",
                        format!(
                            "[联系人入场] 跳过: contact={}, conversation_id={}, reason={}",
                            remote_im_contact_log_label(&contact),
                            conversation_id,
                            err
                        ),
                    );
                    let follow_up_sources = remote_im_finalize_round_completion(
                        state,
                        &activated_remote_im_sources,
                        Some("no_reply"),
                        None,
                        None,
                        &history_flush_time,
                    )?;
                    if !follow_up_sources.is_empty() {
                        runtime_log_warn(format!(
                            "[远程联系人入场] 跳过激活后仍出现待办续跑，当前先跳过: conversation_id={}, source_count={}",
                            conversation_id,
                            follow_up_sources.len()
                        ));
                    }
                }
            }
            if should_activate {
                let current_assistant =
                    remote_im_secretary_current_assistant_context(state, conversation_id)?;
                activating_session_info = Some(ChatSessionInfo {
                    department_id: current_assistant.department_id.clone(),
                    agent_id: current_assistant.agent_id.clone(),
                });
                if remote_im_contact_uses_smart_judge(&contact) {
                    let secretary_recent_history = remote_im_collect_secretary_recent_messages(
                        &conversation.messages,
                        7,
                        &contact,
                        &scheduler_agents,
                        &current_assistant,
                    );
                    let secretary_new_batch_messages = remote_im_collect_secretary_recent_messages(
                        &persisted_batch_messages,
                        persisted_batch_messages.len(),
                        &contact,
                        &scheduler_agents,
                        &current_assistant,
                    );
                    let decision = match run_remote_im_secretary_decision(
                        state,
                        &contact,
                        &current_assistant,
                        &secretary_recent_history,
                        &secretary_new_batch_messages,
                    )
                    .await
                    {
                        Ok(result) => result,
                        Err(err) => {
                            runtime_log_warn(format!(
                                "[远程联系人秘书] 判断失败，降级为始终回复: conversation_id={}, contact_id={}, error={}",
                                conversation_id, contact.id, err
                            ));
                            remote_im_append_channel_log(
                                &contact.channel_id,
                                "warn",
                                format!(
                                    "[联系人秘书] 智能判断失败: contact={}, conversation_id={}, strategy=smart_judge, fallback=always_reply, error={}",
                                    remote_im_contact_log_label(&contact),
                                    conversation_id,
                                    err
                                ),
                            );
                            RemoteImSecretaryDecision {
                                should_reply: true,
                                reason: format!("秘书判断失败，已降级为始终回复：{err}"),
                                model_name: String::new(),
                            }
                        }
                    };
                    eprintln!(
                        "[远程联系人秘书] 决策完成: conversation_id={}, contact_id={}, should_reply={}, model={}, reason={}",
                        conversation_id,
                        contact.id,
                        decision.should_reply,
                        if decision.model_name.trim().is_empty() {
                            "fallback"
                        } else {
                            decision.model_name.as_str()
                        },
                        decision.reason
                    );
                    remote_im_append_channel_log(
                        &contact.channel_id,
                        "info",
                        format!(
                            "[联系人秘书] 智能判断: contact={}, conversation_id={}, result={}, model={}, history_count={}, new_count={}, reason={}",
                            remote_im_contact_log_label(&contact),
                            conversation_id,
                            if decision.should_reply { "回复" } else { "不回复" },
                            if decision.model_name.trim().is_empty() {
                                "fallback"
                            } else {
                                decision.model_name.as_str()
                            },
                            secretary_recent_history.len(),
                            secretary_new_batch_messages.len(),
                            decision.reason
                        ),
                    );
                    if !decision.should_reply {
                        should_activate = false;
                        remote_im_skip_decision = Some("no_reply".to_string());
                        let follow_up_sources = remote_im_finalize_round_completion(
                            state,
                            &activated_remote_im_sources,
                            Some("no_reply"),
                            None,
                            None,
                            &history_flush_time,
                        )?;
                        if !follow_up_sources.is_empty() {
                            runtime_log_warn(format!(
                                "[远程联系人秘书] 判定不回复后仍出现待办续跑，当前先跳过: conversation_id={}, source_count={}",
                                conversation_id,
                                follow_up_sources.len()
                            ));
                        }
                    }
                }
            }
        } else {
            set_conversation_remote_im_assistant_context(state, conversation_id, None)?;
        }
    }
    let guided_event_ids = events
        .iter()
        .filter(|event| event.queue_mode == ChatQueueMode::Guided)
        .map(|event| event.id.clone())
        .collect::<Vec<_>>();
    let activating_runtime_context = events
        .iter()
        .zip(event_activate_flags.iter().copied())
        .rev()
        .find_map(|(event, should_activate)| {
            if should_activate {
                event.runtime_context.clone()
            } else {
                None
            }
        });

    let batch_message_count = events.iter().map(|e| e.messages.len()).sum::<usize>();
    let mut activations = take_queued_chat_activations(state, &event_ids)?;
    if activations.is_empty() {
        activations = collect_active_chat_view_activations(state, conversation_id)?;
    }
    let mut history_flushed_messages =
        Vec::<ChatMessage>::with_capacity(persisted_batch_messages.len());
    for message in &persisted_batch_messages {
        let (deferred_message, _, _) =
            defer_image_parts_for_history_flushed(message, &state.data_path);
        history_flushed_messages.push(deferred_message);
    }
    let history_flushed_payload = serde_json::json!({
        "conversationId": conversation_id,
        "messageCount": batch_message_count,
        "messages": history_flushed_messages,
        "activateAssistant": should_activate,
    });
    emit_history_flushed_event(state, &history_flushed_payload, conversation_id, &event_ids);

    // 3. 如果需要激活，调用主助理。
    if should_activate {
        if let Some(activating_session_info) = activating_session_info.as_ref() {
            // 同一批里可能有多个激活请求，但前台主助理轮次只能有一个。
            // 因此这里只保留最后一个激活请求作为实际流式绑定对象。
            let activation = activations.pop();
            let main_request_id = runtime_context_request_id_or_new(
                activating_runtime_context.as_ref(),
                activation
                    .as_ref()
                    .map(|item| format!("queue-{}", item.event_id))
                    .as_deref(),
                "queue",
            );
            match activate_main_assistant(
                state,
                activating_session_info,
                conversation_id,
                activation.clone(),
                (!guided_event_ids.is_empty()).then_some(guided_event_ids.as_slice()),
                activating_runtime_context.clone(),
                activated_remote_im_sources.clone(),
                oldest_queue_created_at,
            )
            .await
            {
                Ok(activated) => {
                    let result = activated.result;
                    let mut follow_up_sources = match remote_im_finalize_round_completion(
                        state,
                        &activated_remote_im_sources,
                        result.remote_im_reply_decision.as_deref(),
                        result.remote_im_reply_target.as_ref(),
                        None,
                        &history_flush_time,
                    ) {
                        Ok(sources) => sources,
                        Err(finalize_err) => {
                            runtime_log_warn(format!(
                            "[聊天调度] 远程联系人轮次收尾失败（完成分支），conversation_id={}，error={}",
                            conversation_id, finalize_err
                            ));
                            Vec::new()
                        }
                    };
                    follow_up_sources = filter_remote_im_follow_up_sources_for_pending_queue(
                        state,
                        conversation_id,
                        follow_up_sources,
                    );
                    emit_round_completed_event(
                        state,
                        conversation_id,
                        &result,
                        Some(activated.activation_id.as_str()),
                        Some(activated.request_id.as_str()),
                    );
                    complete_pending_chat_events_with_result(state, &event_ids, result)?;
                    while !follow_up_sources.is_empty() {
                        let follow_up_started_at = now_iso();
                        let mut follow_up_context =
                            runtime_context_new("remote_im", "remote_im_followup");
                        follow_up_context.request_id = Some(format!(
                            "remote-im-follow-up-{}",
                            Uuid::new_v4()
                        ));
                        let follow_up_request_id = runtime_context_request_id_or_new(
                            Some(&follow_up_context),
                            None,
                            "queue",
                        );
                        eprintln!(
                            "[远程联系人状态机] 待办续跑 开始: conversation_id={}, source_count={}",
                            conversation_id,
                            follow_up_sources.len()
                        );
                        match activate_main_assistant(
                            state,
                            activating_session_info,
                            conversation_id,
                            None,
                            None,
                            Some(follow_up_context),
                            follow_up_sources.clone(),
                            &follow_up_started_at,
                        )
                        .await
                        {
                            Ok(follow_up_activated) => {
                                let follow_up_result = follow_up_activated.result;
                                follow_up_sources = match remote_im_finalize_round_completion(
                                    state,
                                    &follow_up_sources,
                                    follow_up_result.remote_im_reply_decision.as_deref(),
                                    follow_up_result.remote_im_reply_target.as_ref(),
                                    None,
                                    &follow_up_started_at,
                                ) {
                                    Ok(sources) => sources,
                                    Err(finalize_err) => {
                                        runtime_log_warn(format!(
                                            "[聊天调度] 远程联系人待办续跑收尾失败，conversation_id={}，error={}",
                                            conversation_id, finalize_err
                                        ));
                                        Vec::new()
                                    }
                                };
                                follow_up_sources =
                                    filter_remote_im_follow_up_sources_for_pending_queue(
                                        state,
                                        conversation_id,
                                        follow_up_sources,
                                    );
                                emit_round_completed_event(
                                    state,
                                    conversation_id,
                                    &follow_up_result,
                                    Some(follow_up_activated.activation_id.as_str()),
                                    Some(follow_up_activated.request_id.as_str()),
                                );
                            }
                            Err(err) => {
                                emit_round_failed_event(
                                    state,
                                    conversation_id,
                                    &err,
                                    Some(follow_up_request_id.as_str()),
                                    Some(follow_up_request_id.as_str()),
                                );
                                if let Err(finalize_err) = remote_im_finalize_round_completion(
                                    state,
                                    &follow_up_sources,
                                    None,
                                    None,
                                    Some(&err),
                                    &follow_up_started_at,
                                ) {
                                    runtime_log_warn(format!(
                                        "[聊天调度] 远程联系人待办续跑收尾失败（失败分支），conversation_id={}，original_error={}，finalize_error={}",
                                        conversation_id, err, finalize_err
                                    ));
                                }
                                return Err(err);
                            }
                        }
                    }
                }
                Err(err) => {
                    if err == CHAT_ABORTED_BY_USER_ERROR {
                        complete_pending_chat_events_with_error(state, &event_ids, &err)?;
                        if let Err(finalize_err) = remote_im_finalize_round_completion(
                            state,
                            &activated_remote_im_sources,
                            None,
                            None,
                            None,
                            &history_flush_time,
                        ) {
                            runtime_log_warn(format!(
                                "[聊天调度] 远程联系人轮次收尾失败（停止分支），conversation_id={}，error={}",
                                conversation_id, finalize_err
                            ));
                        }
                        return Ok(());
                    }
                    emit_round_failed_event(
                        state,
                        conversation_id,
                        &err,
                        Some(main_request_id.as_str()),
                        Some(main_request_id.as_str()),
                    );
                    complete_pending_chat_events_with_error(state, &event_ids, &err)?;
                    if let Err(finalize_err) = remote_im_finalize_round_completion(
                        state,
                        &activated_remote_im_sources,
                        None,
                        None,
                        Some(&err),
                        &history_flush_time,
                    ) {
                        runtime_log_warn(format!(
                            "[聊天调度] 远程联系人轮次收尾失败（失败分支），conversation_id={}，original_error={}，finalize_error={}",
                            conversation_id, err, finalize_err
                        ));
                    }
                    return Err(err);
                }
            }
        }
    } else {
        set_conversation_remote_im_activation_sources(state, conversation_id, Vec::new())?;
        set_conversation_remote_im_assistant_context(state, conversation_id, None)?;
        if !guided_event_ids.is_empty() {
            let error_text = "引导消息未能触发助理回复";
            complete_pending_chat_events_with_error(state, &event_ids, error_text)?;
            return Err(error_text.to_string());
        }
        // 不激活时，本批消息依然已经是正式历史的一部分。
        // 这里只回传一个“已落地但未开启新轮次”的结果，前端应刷新历史，
        // 但不应启动新的主助理流式显示。
        complete_pending_chat_events_with_result(
            state,
            &event_ids,
            SendChatResult {
                conversation_id: conversation_id.to_string(),
                latest_user_text,
                assistant_text: String::new(),
                final_response_text: String::new(),
                reasoning_standard: String::new(),
                reasoning_inline: String::new(),
                archived_before_send: false,
                assistant_message: None,
                provider_prompt_tokens: None,
                estimated_prompt_tokens: None,
                effective_prompt_tokens: None,
                effective_prompt_source: None,
                context_window_tokens: None,
                max_output_tokens: None,
                context_usage_percent: None,
                remote_im_reply_decision: remote_im_skip_decision,
                remote_im_reply_target: None,
            },
        )?;
    }

    Ok(())
}

/// 激活主助理
///
/// 注意：这里只负责启动“下一轮主助理”，不负责把新消息写进历史。
/// 新消息进入历史的动作已经在 process_conversation_batch 中完成。
async fn activate_main_assistant(
    state: &AppState,
    session_info: &ChatSessionInfo,
    conversation_id: &str,
    activation: Option<QueuedChatActivation>,
    guided_event_ids: Option<&[String]>,
    runtime_context: Option<RuntimeContext>,
    remote_im_activation_sources: Vec<RemoteImActivationSource>,
    oldest_queue_created_at: &str,
) -> Result<ActivatedAssistantResult, String> {
    let mut runtime_context = runtime_context.unwrap_or_default();
    if runtime_context.bound_remote_im_activation_source.is_none() {
        runtime_context.bound_remote_im_activation_source =
            resolve_bound_remote_im_activation_source(&remote_im_activation_sources);
    }
    let activation_trace_id = activation
        .as_ref()
        .map(|item| format!("queue-{}", item.event_id));
    let trace_id = runtime_context_request_id_or_new(
        Some(&runtime_context),
        activation_trace_id.as_deref(),
        "queue",
    );
    if runtime_context.request_id.is_none() {
        runtime_context.request_id = Some(trace_id.clone());
    }
    if runtime_context.target_conversation_id.is_none() {
        runtime_context.target_conversation_id = Some(conversation_id.to_string());
    }
    if runtime_context.executor_agent_id.is_none() {
        runtime_context.executor_agent_id = Some(session_info.agent_id.clone());
    }
    if runtime_context.executor_department_id.is_none() {
        runtime_context.executor_department_id = Some(session_info.department_id.clone());
    }
    let activation_id = trace_id.clone();
    let activation_reason = resolve_activation_reason(&runtime_context);
    let stream_started_at = now_iso();
    let stream_started_at_ms = now_unix_ms();
    reset_conversation_stream_runtime_cache(
        state,
        conversation_id,
        activation_id.as_str(),
        trace_id.as_str(),
        stream_started_at.as_str(),
        stream_started_at_ms,
    )?;
    emit_round_started_event(
        state,
        conversation_id,
        activation_id.as_str(),
        trace_id.as_str(),
        activation_reason.as_str(),
        session_info.department_id.as_str(),
        session_info.agent_id.as_str(),
        stream_started_at.as_str(),
        stream_started_at_ms,
    );

    // 设置状态为 AssistantStreaming
    set_conversation_runtime_state(state, conversation_id, MainSessionState::AssistantStreaming)?;
    set_conversation_remote_im_activation_sources(
        state,
        conversation_id,
        remote_im_activation_sources.clone(),
    )?;
    if let Some(event_ids) = guided_event_ids.filter(|items| !items.is_empty()) {
        let removed = remove_queue_events_by_ids(state, conversation_id, event_ids)?;
        emit_chat_queue_snapshot(state);
        runtime_log_info(format!(
            "[引导投送] 完成，任务=remove_guided_queue_after_busy，conversation_id={}，event_count={}，removed_count={}",
            conversation_id,
            event_ids.len(),
            removed
        ));
    }

    // 对 WeixinOc 渠道启动 typing 状态（对方正在输入）
    let weixin_oc_typing_sources: Vec<(String, String, WeixinOcCredentials)> = {
        let config = state_read_config_cached(state);
        let config = match config {
            Ok(c) => c,
            Err(err) => {
                eprintln!("[聊天调度] 读取配置失败，跳过 typing 启动: error={}", err);
                AppConfig::default()
            }
        };
        remote_im_activation_sources
            .iter()
            .filter(|src| src.platform == RemoteImPlatform::WeixinOc)
            .filter_map(|src| {
                let channel = remote_im_channel_by_id(&config, &src.channel_id)?;
                let credentials = WeixinOcCredentials::from_value(&channel.credentials);
                if credentials.token.trim().is_empty() {
                    return None;
                }
                Some((
                    src.channel_id.clone(),
                    src.remote_contact_id.clone(),
                    credentials,
                ))
            })
            .collect()
    };
    for (ch_id, contact_id, credentials) in &weixin_oc_typing_sources {
        let ctx_token = weixin_oc_manager()
            .get_context_token(&ch_id, &contact_id)
            .await;
        weixin_oc_manager()
            .start_typing(&ch_id, credentials.clone(), &contact_id, ctx_token)
            .await;
    }

    // 构造 trigger_only 请求
    let request = SendChatRequest {
        trigger_only: true, // 不写入新消息，只触发助理回复
        session: Some(SessionSelector {
            api_config_id: None,
            department_id: Some(session_info.department_id.clone()),
            agent_id: session_info.agent_id.clone(),
            conversation_id: Some(conversation_id.to_string()),
        }),
        payload: ChatInputPayload {
            text: None,
            display_text: None,
            images: None,
            audios: None,
            attachments: None,
            model: None,
            extra_text_blocks: None,
            mentions: None,
            provider_meta: None,
        },
        speaker_agent_id: None,
        trace_id: Some(trace_id.clone()),
        oldest_queue_created_at: Some(oldest_queue_created_at.to_string()),
        remote_im_activation_sources,
        runtime_context: Some(runtime_context),
    };

    // 使用 emit 作为远程激活轮次的流式主通道，避免前端窗口重绑定造成 channel 失联。
    let state_for_delta = state.clone();
    let conversation_id_for_emit = conversation_id.to_string();
    let stream_start_rebind_emitted = std::sync::Arc::new(std::sync::Mutex::new(false));
    let stream_start_rebind_emitted_for_channel = stream_start_rebind_emitted.clone();
    let active_channel: tauri::ipc::Channel<AssistantDeltaEvent> = tauri::ipc::Channel::new(
        move |body| {
            let parsed_event = match body {
                tauri::ipc::InvokeResponseBody::Json(json) => {
                    serde_json::from_str::<AssistantDeltaEvent>(&json).ok()
                }
                tauri::ipc::InvokeResponseBody::Raw(bytes) => {
                    serde_json::from_slice::<AssistantDeltaEvent>(&bytes).ok()
                }
            };
            if let Some(event) = parsed_event {
                if let Err(err) = update_conversation_stream_runtime_cache(
                    &state_for_delta,
                    &conversation_id_for_emit,
                    &event,
                ) {
                    runtime_log_warn(format!(
                        "[聊天流式缓存] 更新失败，conversation_id={}，kind={}，error={}",
                        conversation_id_for_emit.trim(),
                        event.kind.as_deref().unwrap_or("delta"),
                        err
                    ));
                }
                let mut stream_start_rebind_guard =
                    stream_start_rebind_emitted_for_channel.lock().ok();
                if event.kind.as_deref() == Some("stream_rebind_required") {
                    if let Some(flag) = stream_start_rebind_guard.as_mut() {
                        **flag = false;
                    }
                } else if stream_start_rebind_guard
                    .as_ref()
                    .map(|flag| !**flag)
                    .unwrap_or(true)
                    && is_visible_stream_progress_event(&event)
                {
                    runtime_log_info(format!(
                        "[聊天流式重绑] 检测到首个可见流式包 conversation_id={} kind={} delta_len={}",
                        conversation_id_for_emit.trim(),
                        event.kind.as_deref().unwrap_or("delta"),
                        event.delta.chars().count(),
                    ));
                    emit_stream_rebind_required_event(
                        &state_for_delta,
                        &conversation_id_for_emit,
                        event.request_id.as_deref(),
                        event.phase_id.as_deref(),
                        "stream_start",
                    );
                    runtime_log_info(format!(
                        "[聊天流式重绑] 首个可见流式包触发重绑事件 conversation_id={} kind={} delta_len={}",
                        conversation_id_for_emit.trim(),
                        event.kind.as_deref().unwrap_or("delta"),
                        event.delta.chars().count(),
                    ));
                    if let Some(flag) = stream_start_rebind_guard.as_mut() {
                        **flag = true;
                    }
                }
                if should_emit_assistant_delta_via_app_event_only(&event) {
                    if event.kind.as_deref() == Some("stream_rebind_required") {
                        emit_stream_rebind_required_event(
                            &state_for_delta,
                            &conversation_id_for_emit,
                            event.request_id.as_deref(),
                            event.phase_id.as_deref(),
                            event.reason.as_deref().unwrap_or("tool_start"),
                        );
                    } else {
                        emit_assistant_delta_app_event(
                            &state_for_delta,
                            &conversation_id_for_emit,
                            &event,
                        );
                    }
                } else {
                    dispatch_assistant_delta_to_active_view(
                        &state_for_delta,
                        &conversation_id_for_emit,
                        &event,
                    );
                }
            }
            Ok(())
        },
    );

    // 调用 send_chat_message_inner
    let result = send_chat_message_inner(request, state, &active_channel).await;

    // WeixinOc 渠道：回复结束后停止 typing
    for (ch_id, contact_id, _) in &weixin_oc_typing_sources {
        weixin_oc_manager().stop_typing(ch_id, contact_id).await;
    }

    if let Err(err) =
        set_conversation_remote_im_activation_sources(state, conversation_id, Vec::new())
    {
        eprintln!(
            "[聊天调度] 清理远程IM激活来源失败: conversation_id={}, error={}",
            conversation_id, err
        );
    }
    if let Err(err) = set_conversation_remote_im_assistant_context(state, conversation_id, None) {
        eprintln!(
            "[聊天调度] 清理远程IM当前助理失败: conversation_id={}, error={}",
            conversation_id, err
        );
    }

    let next_state = if conversation_has_guided_queue_events(state, conversation_id).unwrap_or(false)
    {
        MainSessionState::OrganizingContext
    } else {
        MainSessionState::Idle
    };
    set_conversation_runtime_state(state, conversation_id, next_state)?;
    if let Err(err) = clear_conversation_stream_runtime_cache(state, conversation_id) {
        runtime_log_warn(format!(
            "[聊天流式缓存] 清理失败，conversation_id={}，error={}",
            conversation_id, err
        ));
    }

    // 后台会话活动标记：前台观看时不写 completed/failed，直接清标记
    let is_watched = state
        .active_chat_view_bindings
        .lock()
        .ok()
        .map(|bindings| {
            bindings.values().any(|b| b.conversation_id.trim() == conversation_id)
        })
        .unwrap_or(false)
        || detached_chat_window_for_conversation(conversation_id).is_some();
    if is_watched {
        clear_conversation_list_activity_mark(state, conversation_id);
    } else {
        match &result {
            Ok(_) => {
                set_conversation_list_activity_mark(
                    state,
                    conversation_id,
                    ConversationListActivityMark {
                        activity: "completed".to_string(),
                        failed_message: None,
                        completed_at: Some(now_iso()),
                    },
                );
            }
            Err(err) => {
                if err != CHAT_ABORTED_BY_USER_ERROR {
                    set_conversation_list_activity_mark(
                        state,
                        conversation_id,
                        ConversationListActivityMark {
                            activity: "failed".to_string(),
                            failed_message: Some(err.clone()),
                            completed_at: None,
                        },
                    );
                } else {
                    clear_conversation_list_activity_mark(state, conversation_id);
                }
            }
        }
    }

    // 活动标记变化后广播完整列表更新
    if let Err(err) = emit_unarchived_conversation_overview_updated_from_state(state) {
        runtime_log_warn(format!(
            "[会话概览] 跳过，任务=活动标记更新后推送，conversation_id={}，error={}",
            conversation_id, err
        ));
    }

    result.map(|result| ActivatedAssistantResult {
        result,
        activation_id,
        request_id: trace_id,
    })
}

fn latest_user_text_from_events(events: &[ChatPendingEvent]) -> String {
    events
        .iter()
        .flat_map(|event| event.messages.iter())
        .rev()
        .find_map(|message| {
            if message.role.trim() != "user" {
                return None;
            }
            message.parts.iter().find_map(|part| match part {
                MessagePart::Text { text } => Some(text.clone()),
                _ => None,
            })
        })
        .unwrap_or_default()
}

fn emit_history_flushed_event(
    state: &AppState,
    payload: &serde_json::Value,
    conversation_id: &str,
    event_ids: &[String],
) {
    ide_chat_broadcast_notification("chat.historyFlushed", payload.clone());
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    let Some(app_handle) = app_handle else {
        eprintln!(
            "[聊天调度] history_flushed emit 失败: app_handle unavailable, conversation_id={}, event_ids={:?}",
            conversation_id, event_ids
        );
        return;
    };
    match app_handle.emit(CHAT_HISTORY_FLUSHED_EVENT, payload) {
        Ok(_) => {}
        Err(err) => {
            eprintln!(
                "[聊天调度] history_flushed emit 失败: conversation_id={}, event_ids={:?}, error={}",
                conversation_id, event_ids, err
            );
        }
    }
}

fn emit_round_started_event(
    state: &AppState,
    conversation_id: &str,
    activation_id: &str,
    request_id: &str,
    reason: &str,
    department_id: &str,
    agent_id: &str,
    started_at: &str,
    started_at_ms: u64,
) {
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    let Some(app_handle) = app_handle else {
        eprintln!(
            "[聊天推送] emit round_started 失败: app_handle unavailable, conversation_id={}",
            conversation_id
        );
        return;
    };
    let payload = serde_json::json!({
        "conversationId": conversation_id,
        "activationId": activation_id,
        "requestId": request_id,
        "reason": reason,
        "departmentId": department_id,
        "agentId": agent_id,
        "startedAt": started_at,
        "startedAtMs": started_at_ms,
    });
    ide_chat_broadcast_notification("chat.roundStarted", payload.clone());
    match app_handle.emit(CHAT_ROUND_STARTED_EVENT, payload) {
        Ok(_) => {}
        Err(err) => eprintln!(
            "[聊天推送] emit round_started 失败: conversation_id={}, error={}",
            conversation_id, err
        ),
    }
}

fn emit_round_completed_event(
    state: &AppState,
    conversation_id: &str,
    result: &SendChatResult,
    activation_id: Option<&str>,
    request_id: Option<&str>,
) {
    notify_local_chat_round_completed(state, conversation_id, &result.assistant_text);
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    let Some(app_handle) = app_handle else {
        eprintln!(
            "[聊天推送] emit round_completed 失败: app_handle unavailable, conversation_id={}",
            conversation_id
        );
        return;
    };
    let payload = serde_json::json!({
        "conversationId": conversation_id,
        "activationId": activation_id.map(str::trim).filter(|value| !value.is_empty()),
        "requestId": request_id.map(str::trim).filter(|value| !value.is_empty()),
        "status": "completed",
        "assistantText": result.assistant_text,
        "reasoningStandard": result.reasoning_standard,
        "reasoningInline": result.reasoning_inline,
        "archivedBeforeSend": result.archived_before_send,
        "assistantMessage": result.assistant_message,
    });
    ide_chat_broadcast_notification("chat.roundFinished", payload.clone());
    match app_handle.emit(CHAT_ROUND_COMPLETED_EVENT, payload) {
        Ok(_) => {}
        Err(err) => eprintln!(
            "[聊天推送] emit round_completed 失败: conversation_id={}, error={}",
            conversation_id, err
        ),
    }
}

fn notify_local_chat_round_completed(
    state: &AppState,
    conversation_id: &str,
    assistant_text: &str,
) {
    let conversation = match state_read_conversation_cached(state, conversation_id) {
        Ok(conversation) => conversation,
        Err(err) => {
            runtime_log_warn(format!(
                "[通知] 跳过，任务=读取本地会话完成通知上下文，conversation_id={}，error={}",
                conversation_id, err
            ));
            return;
        }
    };
    if !conversation_is_local_normal_chat(&conversation) {
        return;
    }
    if conversation_has_focused_chat_view(state, conversation_id) {
        runtime_log_debug(format!(
            "[通知] 跳过，任务=本地会话完成通知，conversation_id={}，reason=chat_view_focused",
            conversation_id
        ));
        return;
    }
    let notification_settings = local_chat_notification_settings(state, conversation_id);
    if !notification_settings.enabled {
        runtime_log_debug(format!(
            "[通知] 跳过，任务=本地会话完成通知，conversation_id={}，reason=notification_disabled",
            conversation_id
        ));
        return;
    }
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    let Some(app_handle) = app_handle else {
        runtime_log_warn(format!(
            "[通知] 跳过，任务=发送本地会话完成通知，conversation_id={}，reason=app_handle_unavailable",
            conversation_id
        ));
        return;
    };
    let speaker_name = notification_speaker_name_for_conversation(
        state,
        &conversation,
        notification_settings.ui_language,
    );
    let body = native_notification_text_excerpt(
        assistant_text,
        NATIVE_NOTIFICATION_BODY_MAX_CHARS,
    );
    let final_body = if body.trim().is_empty() {
        local_chat_notification_text(
            notification_settings.ui_language,
            "已完成本轮回复。",
            "已完成本輪回覆。",
            "Finished this reply.",
        )
    } else {
        body
    };
    if let Err(err) = send_native_notification(
        &app_handle,
        &speaker_name,
        &final_body,
        notification_settings.sound_enabled,
    ) {
        runtime_log_warn(format!(
            "[通知] 失败，任务=发送本地会话完成通知，conversation_id={}，error={}",
            conversation_id, err
        ));
    }
}

fn local_chat_notification_text(
    ui_language: &str,
    zh_cn: &str,
    zh_tw: &str,
    en_us: &str,
) -> String {
    match ui_language.trim() {
        "en-US" => en_us.to_string(),
        "zh-TW" => zh_tw.to_string(),
        _ => zh_cn.to_string(),
    }
}

fn notification_speaker_name_for_conversation(
    state: &AppState,
    conversation: &Conversation,
    ui_language: &str,
) -> String {
    let agent_id = conversation.agent_id.trim();
    if agent_id.is_empty() {
        return local_chat_notification_text(
            ui_language,
            "当前人格",
            "當前人格",
            "Current persona",
        );
    }
    match state_read_agents_cached(state) {
        Ok(agents) => agents
            .iter()
            .find(|agent| agent.id.trim() == agent_id)
            .map(|agent| agent.name.trim())
            .filter(|name| !name.is_empty())
            .unwrap_or(agent_id)
            .to_string(),
        Err(err) => {
            runtime_log_warn(format!(
                "[通知] 跳过，任务=读取人格名称失败后回退ID，conversation_id={}，agent_id={}，error={}",
                conversation.id, agent_id, err
            ));
            agent_id.to_string()
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct LocalChatNotificationSettings {
    enabled: bool,
    sound_enabled: bool,
    ui_language: &'static str,
}

impl Default for LocalChatNotificationSettings {
    fn default() -> Self {
        Self {
            enabled: default_message_notification_enabled(),
            sound_enabled: default_message_notification_sound_enabled(),
            ui_language: "zh-CN",
        }
    }
}

fn local_chat_notification_settings(
    state: &AppState,
    conversation_id: &str,
) -> LocalChatNotificationSettings {
    match state_read_config_cached(state) {
        Ok(config) => LocalChatNotificationSettings {
            enabled: config.message_notification_enabled,
            sound_enabled: config.message_notification_sound_enabled,
            ui_language: match config.ui_language.trim() {
                "en-US" => "en-US",
                "zh-TW" => "zh-TW",
                _ => "zh-CN",
            },
        },
        Err(err) => {
            runtime_log_warn(format!(
                "[通知] 跳过，任务=读取通知设置失败后回退默认值，conversation_id={}，error={}",
                conversation_id, err
            ));
            LocalChatNotificationSettings::default()
        }
    }
}

fn emit_stop_chat_round_completed_event(
    state: &AppState,
    conversation_id: &str,
    result: &StopChatResult,
) {
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    let Some(app_handle) = app_handle else {
        eprintln!(
            "[聊天推送] emit stop round_completed 失败: app_handle unavailable, conversation_id={}",
            conversation_id
        );
        return;
    };
    let payload = serde_json::json!({
        "conversationId": conversation_id,
        "status": "stopped",
        "assistantText": result.assistant_text,
        "reasoningStandard": result.reasoning_standard,
        "reasoningInline": result.reasoning_inline,
        "archivedBeforeSend": false,
        "assistantMessage": result.assistant_message,
    });
    ide_chat_broadcast_notification("chat.roundFinished", payload.clone());
    match app_handle.emit(CHAT_ROUND_COMPLETED_EVENT, payload) {
        Ok(_) => {}
        Err(err) => eprintln!(
            "[聊天推送] emit stop round_completed 失败: conversation_id={}, error={}",
            conversation_id, err
        ),
    }
}

fn emit_round_failed_event(
    state: &AppState,
    conversation_id: &str,
    error_text: &str,
    activation_id: Option<&str>,
    request_id: Option<&str>,
) {
    notify_local_chat_round_failed(state, conversation_id, error_text);
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    let Some(app_handle) = app_handle else {
        eprintln!(
            "[聊天推送] emit round_failed 失败: app_handle unavailable, conversation_id={}",
            conversation_id
        );
        return;
    };
    let payload = serde_json::json!({
        "conversationId": conversation_id,
        "activationId": activation_id.map(str::trim).filter(|value| !value.is_empty()),
        "requestId": request_id.map(str::trim).filter(|value| !value.is_empty()),
        "status": "failed",
        "error": error_text,
    });
    ide_chat_broadcast_notification("chat.roundFinished", payload.clone());
    match app_handle.emit(CHAT_ROUND_FAILED_EVENT, payload) {
        Ok(_) => {}
        Err(err) => eprintln!(
            "[聊天推送] emit round_failed 失败: conversation_id={}, error={}",
            conversation_id, err
        ),
    }
}

fn notify_local_chat_round_failed(state: &AppState, conversation_id: &str, error_text: &str) {
    let conversation = match state_read_conversation_cached(state, conversation_id) {
        Ok(conversation) => conversation,
        Err(err) => {
            runtime_log_warn(format!(
                "[通知] 跳过，任务=读取本地会话失败通知上下文，conversation_id={}，error={}",
                conversation_id, err
            ));
            return;
        }
    };
    if !conversation_is_local_normal_chat(&conversation) {
        return;
    }
    if conversation_has_focused_chat_view(state, conversation_id) {
        runtime_log_debug(format!(
            "[通知] 跳过，任务=本地会话失败通知，conversation_id={}，reason=chat_view_focused",
            conversation_id
        ));
        return;
    }
    let notification_settings = local_chat_notification_settings(state, conversation_id);
    if !notification_settings.enabled {
        runtime_log_debug(format!(
            "[通知] 跳过，任务=本地会话失败通知，conversation_id={}，reason=notification_disabled",
            conversation_id
        ));
        return;
    }
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    let Some(app_handle) = app_handle else {
        runtime_log_warn(format!(
            "[通知] 跳过，任务=发送本地会话失败通知，conversation_id={}，reason=app_handle_unavailable",
            conversation_id
        ));
        return;
    };
    let speaker_name = notification_speaker_name_for_conversation(
        state,
        &conversation,
        notification_settings.ui_language,
    );
    let body = native_notification_text_excerpt(
        error_text,
        NATIVE_NOTIFICATION_BODY_MAX_CHARS,
    );
    let final_body = if body.trim().is_empty() {
        local_chat_notification_text(
            notification_settings.ui_language,
            "本轮调度失败。",
            "本輪調度失敗。",
            "This round failed.",
        )
    } else {
        body
    };
    let title = match notification_settings.ui_language {
        "en-US" => format!("{speaker_name} response failed"),
        "zh-TW" => format!("{speaker_name} 調度失敗"),
        _ => format!("{speaker_name} 调度失败"),
    };
    if let Err(err) = send_native_notification(
        &app_handle,
        &title,
        &final_body,
        notification_settings.sound_enabled,
    ) {
        runtime_log_warn(format!(
            "[通知] 失败，任务=发送本地会话失败通知，conversation_id={}，error={}",
            conversation_id, err
        ));
    }
}

fn collect_active_chat_view_activations(
    state: &AppState,
    conversation_id: &str,
) -> Result<Vec<QueuedChatActivation>, String> {
    let bindings = state
        .active_chat_view_bindings
        .lock()
        .map_err(|_| "Failed to lock active chat view bindings".to_string())?;
    let conversation_id = conversation_id.trim();
    let binding_snapshot = bindings
        .iter()
        .map(|(window_label, binding)| format!("{}=>{}", window_label, binding.conversation_id))
        .collect::<Vec<_>>();
    eprintln!(
        "[聊天调度] 绑定快照: conversation_id={}, bindings={:?}",
        conversation_id, binding_snapshot
    );
    let exact = bindings
        .iter()
        .filter_map(|(window_label, binding)| {
            if binding.conversation_id != conversation_id {
                return None;
            }
            Some(QueuedChatActivation {
                event_id: format!("active-view:{window_label}"),
            })
        })
        .collect::<Vec<_>>();
    if !exact.is_empty() {
        eprintln!(
            "[聊天调度] 绑定筛选命中(exact): conversation_id={}, hit={}",
            conversation_id,
            exact.len()
        );
        return Ok(exact);
    }

    let wildcard = bindings
        .iter()
        .filter_map(|(window_label, binding)| {
            if binding.conversation_id != "*" {
                return None;
            }
            Some(QueuedChatActivation {
                event_id: format!("active-view:{window_label}"),
            })
        })
        .collect::<Vec<_>>();
    if !wildcard.is_empty() {
        eprintln!(
            "[聊天调度] 使用通配前端绑定: conversation_id={}, binding_count={}",
            conversation_id,
            wildcard.len()
        );
        return Ok(wildcard);
    }

    eprintln!(
        "[聊天调度] 绑定筛选未命中: conversation_id={}, bindings_count={}",
        conversation_id,
        bindings.len()
    );
    Ok(Vec::new())
}

fn take_queued_chat_activations(
    state: &AppState,
    event_ids: &[String],
) -> Result<Vec<QueuedChatActivation>, String> {
    let mut channels = state
        .pending_chat_delta_channels
        .lock()
        .map_err(|_| "Failed to lock pending chat delta channels".to_string())?;
    let mut activations = Vec::<QueuedChatActivation>::new();
    for event_id in event_ids {
        if channels.remove(event_id).is_some() {
            activations.push(QueuedChatActivation {
                event_id: event_id.clone(),
            });
        }
    }
    Ok(activations)
}

fn complete_pending_chat_events_with_result(
    state: &AppState,
    event_ids: &[String],
    result: SendChatResult,
) -> Result<(), String> {
    let mut channels = state
        .pending_chat_delta_channels
        .lock()
        .map_err(|_| "Failed to lock pending chat delta channels".to_string())?;
    let mut senders = state
        .pending_chat_result_senders
        .lock()
        .map_err(|_| "Failed to lock pending chat result senders".to_string())?;
    for event_id in event_ids {
        channels.remove(event_id);
        if let Some(sender) = senders.remove(event_id) {
            let _ = sender.send(Ok(result.clone()));
        }
    }
    Ok(())
}

fn complete_pending_chat_events_with_error(
    state: &AppState,
    event_ids: &[String],
    error: &str,
) -> Result<(), String> {
    let mut channels = state
        .pending_chat_delta_channels
        .lock()
        .map_err(|_| "Failed to lock pending chat delta channels".to_string())?;
    let mut senders = state
        .pending_chat_result_senders
        .lock()
        .map_err(|_| "Failed to lock pending chat result senders".to_string())?;
    for event_id in event_ids {
        channels.remove(event_id);
        if let Some(sender) = senders.remove(event_id) {
            let _ = sender.send(Err(error.to_string()));
        }
    }
    Ok(())
}

fn lock_remote_im_contact_runtime_states(
    state: &AppState,
) -> Result<std::sync::MutexGuard<'_, std::collections::HashMap<String, RemoteImContactRuntimeState>>, String>
{
    match state.remote_im_contact_runtime_states.lock() {
        Ok(states) => Ok(states),
        Err(poisoned) => {
            runtime_log_warn(
                "[远程联系人状态机] 运行时锁中毒，已恢复并继续处理当前业务".to_string(),
            );
            Ok(poisoned.into_inner())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteImReplyInspectionPath {
    Mention,
    NonMention,
}

#[derive(Clone)]
struct RemoteImReplyDebounceReady {
    contact_id: String,
    generation: u64,
    start_message_id: String,
    end_message_id: String,
    focus: bool,
    max_chars: u32,
    path: RemoteImReplyInspectionPath,
    event: ChatPendingEvent,
}

fn remote_im_event_latest_user_message(event: &ChatPendingEvent) -> Option<&ChatMessage> {
    event
        .messages
        .iter()
        .rev()
        .find(|message| message.role.trim().eq_ignore_ascii_case("user"))
}

fn remote_im_event_hits_wake(contact: &RemoteImContact, event: &ChatPendingEvent) -> bool {
    let Some(message) = remote_im_event_latest_user_message(event) else {
        return false;
    };
    remote_im_keyword_matched(contact, &render_message_content_for_model(message))
}

fn remote_im_contact_is_muted(state: &AppState, contact_id: &str) -> Result<bool, String> {
    let mut runtime_states = lock_remote_im_contact_runtime_states(state)?;
    let Some(runtime) = runtime_states.get_mut(contact_id) else {
        return Ok(false);
    };
    let Some(mute_until) = runtime.mute_until.clone() else {
        return Ok(false);
    };
    if remote_im_is_mute_expired(&mute_until, now_utc()) {
        runtime.mute_until = None;
        return Ok(false);
    }
    Ok(true)
}

fn clear_remote_im_debounces_for_contact(
    state: &AppState,
    contact_id: &str,
) -> Result<(), String> {
    let key = remote_im_group_reply_state_key(state, contact_id);
    let mut store = lock_remote_im_group_reply_state_store();
    store.by_contact.remove(&key);
    Ok(())
}

fn remote_im_group_reply_reconfigure_contact(
    state: &AppState,
    contact: &RemoteImContact,
) -> Result<(), String> {
    let key = remote_im_group_reply_state_key(state, &contact.id);
    let pacing = effective_remote_im_group_reply_pacing(state, contact);
    let mut store = lock_remote_im_group_reply_state_store();
    let Some(existing) = store.by_contact.get(&key).cloned() else {
        return Ok(());
    };
    if existing.phase == RemoteImGroupReplyPhase::CommitPending {
        return Ok(());
    }
    let kind = if existing.phase == RemoteImGroupReplyPhase::MentionScheduled {
        RemoteImGroupReplyTimerKind::Mention
    } else {
        RemoteImGroupReplyTimerKind::NonMention
    };
    let delay = match kind {
        RemoteImGroupReplyTimerKind::Mention => {
            std::time::Duration::from_secs(pacing.assistant_debounce_seconds)
        }
        RemoteImGroupReplyTimerKind::NonMention => remote_im_group_reply_inspection_delay(
            &pacing,
            remote_im_group_reply_random_sample(),
        ),
        RemoteImGroupReplyTimerKind::Commit => return Ok(()),
    };
    let generation = remote_im_group_reply_next_generation(&mut store);
    store.by_contact.insert(
        key.clone(),
        RemoteImGroupReplyState {
            generation,
            phase: match kind {
                RemoteImGroupReplyTimerKind::Mention => RemoteImGroupReplyPhase::MentionScheduled,
                RemoteImGroupReplyTimerKind::NonMention => {
                    RemoteImGroupReplyPhase::NonMentionScheduled
                }
                RemoteImGroupReplyTimerKind::Commit => return Ok(()),
            },
            start_message_id: existing.start_message_id,
            decision_end_message_id: existing.decision_end_message_id,
            focus: existing.focus,
            energy_settled: existing.energy_settled,
            next_round_mention: existing.next_round_mention,
            event: existing.event,
            due_at: std::time::Instant::now() + delay,
            inspection_kind: kind,
            pending_settlement: None,
        },
    );
    drop(store);
    remote_im_group_reply_schedule_action(
        state,
        RemoteImGroupReplyTimerAction {
            state_key: key,
            contact_id: contact.id.clone(),
            generation,
            kind,
            delay,
        },
    );
    Ok(())
}

fn remote_im_enforce_mute_side_effects(
    state: &AppState,
    contact_id: &str,
    reason: &str,
) {
    if let Err(err) = clear_remote_im_debounces_for_contact(state, contact_id) {
        runtime_log_warn(format!(
            "[群聊巡检] 闭嘴清理降级，contact_id={}，error={}",
            contact_id, err
        ));
    }
    let aborted = match abort_remote_im_reply_delegates_for_contact(state, contact_id, reason) {
        Ok(aborted) => aborted,
        Err(err) => {
            runtime_log_warn(format!(
                "[远程联系人状态机] 闭嘴终止委托降级，contact_id={}，reason={}，error={}",
                contact_id, reason, err
            ));
            0
        }
    };
    runtime_log_info(format!(
        "[远程联系人状态机] 闭嘴善后 完成: contact_id={}, aborted_delegate_count={}, reason={}",
        contact_id, aborted, reason
    ));
}

fn remote_im_group_reply_schedule_action(
    state: &AppState,
    action: RemoteImGroupReplyTimerAction,
) {
    #[cfg(test)]
    if action.delay >= std::time::Duration::from_secs(300) {
        return;
    }
    let state = state.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(action.delay).await;
        remote_im_group_reply_handle_timer(&state, action).await;
    });
}

fn remote_im_group_reply_reschedule_non_mention(
    state: &AppState,
    contact: &RemoteImContact,
    generation: u64,
    reason: &str,
) {
    let key = remote_im_group_reply_state_key(state, &contact.id);
    let pacing = effective_remote_im_group_reply_pacing(state, contact);
    let delay = remote_im_group_reply_inspection_delay(
        &pacing,
        remote_im_group_reply_random_sample(),
    );
    let action = {
        let mut store = lock_remote_im_group_reply_state_store();
        let is_current = store
            .by_contact
            .get(&key)
            .map(|current| current.generation == generation)
            .unwrap_or(false);
        if !is_current {
            None
        } else {
            let next_generation = remote_im_group_reply_next_generation(&mut store);
            store.by_contact.get_mut(&key).map(|current| {
                current.generation = next_generation;
                current.phase = RemoteImGroupReplyPhase::NonMentionScheduled;
                current.inspection_kind = RemoteImGroupReplyTimerKind::NonMention;
                current.pending_settlement = None;
                current.due_at = std::time::Instant::now() + delay;
                RemoteImGroupReplyTimerAction {
                    state_key: key.clone(),
                    contact_id: contact.id.clone(),
                    generation: next_generation,
                    kind: RemoteImGroupReplyTimerKind::NonMention,
                    delay,
                }
            })
        }
    };
    if let Some(action) = action {
        runtime_log_warn(format!(
            "[群聊巡检] 降级重排，contact_id={}，generation={}，delay_ms={}，reason={}",
            contact.id,
            action.generation,
            action.delay.as_millis(),
            reason
        ));
        remote_im_group_reply_schedule_action(state, action);
    } else {
        runtime_log_warn(format!(
            "[群聊巡检] 重排跳过，contact_id={}，reason=状态已变化，original_reason={}",
            contact.id, reason
        ));
    }
}

fn remote_im_group_reply_retry_generation(
    state: &AppState,
    contact_id: &str,
    generation: u64,
    reason: &str,
) {
    const RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(5);
    let key = remote_im_group_reply_state_key(state, contact_id);
    let action = {
        let mut store = lock_remote_im_group_reply_state_store();
        let current = store.by_contact.get(&key).cloned();
        match current {
            Some(current)
                if current.generation == generation
                    && current.phase != RemoteImGroupReplyPhase::CommitPending =>
            {
                let next_generation = remote_im_group_reply_next_generation(&mut store);
                let kind = current.inspection_kind;
                let phase = match kind {
                    RemoteImGroupReplyTimerKind::Mention => {
                        RemoteImGroupReplyPhase::MentionScheduled
                    }
                    RemoteImGroupReplyTimerKind::NonMention => {
                        RemoteImGroupReplyPhase::NonMentionScheduled
                    }
                    RemoteImGroupReplyTimerKind::Commit => return,
                };
                if let Some(current) = store.by_contact.get_mut(&key) {
                    current.generation = next_generation;
                    current.phase = phase;
                    current.due_at = std::time::Instant::now() + RETRY_DELAY;
                }
                Some(RemoteImGroupReplyTimerAction {
                    state_key: key,
                    contact_id: contact_id.to_string(),
                    generation: next_generation,
                    kind,
                    delay: RETRY_DELAY,
                })
            }
            _ => None,
        }
    };
    if let Some(action) = action {
        runtime_log_warn(format!(
            "[群聊巡检] 故障重试已保留批次，contact_id={}，generation={}，delay_ms={}，reason={}",
            contact_id,
            action.generation,
            action.delay.as_millis(),
            reason
        ));
        remote_im_group_reply_schedule_action(state, action);
    }
}

fn remote_im_group_reply_advance_after_settlement(
    state: &AppState,
    contact: &RemoteImContact,
    generation: u64,
) {
    let key = remote_im_group_reply_state_key(state, &contact.id);
    let pacing = effective_remote_im_group_reply_pacing(state, contact);
    let current = lock_remote_im_group_reply_state_store()
        .by_contact
        .get(&key)
        .filter(|current| current.generation == generation)
        .cloned();
    let Some(current) = current else {
        return;
    };
    let Some(boundary_message_id) = current.decision_end_message_id.as_deref() else {
        return;
    };
    let next_start = match remote_im_group_reply_next_unsettled_start_message_id(
        state,
        contact,
        current.event.conversation_id.as_str(),
        boundary_message_id,
    ) {
        Ok(value) => value,
        Err(err) => {
            if err.contains("Message not found") || err.contains("不存在") {
                runtime_log_warn(format!(
                    "[群聊巡检] 已结算边界不存在，结束内存批次，contact_id={}，generation={}，error={}",
                    contact.id, generation, err
                ));
                lock_remote_im_group_reply_state_store().by_contact.remove(&key);
                return;
            }
            runtime_log_warn(format!(
                "[群聊巡检] 下一轮起点读取失败，已保留结算状态等待重试，contact_id={}，generation={}，error={}",
                contact.id, generation, err
            ));
            let state = state.clone();
            let contact = contact.clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                remote_im_group_reply_advance_after_settlement(
                    &state,
                    &contact,
                    generation,
                );
            });
            return;
        }
    };
    let Some(next_start) = next_start else {
        lock_remote_im_group_reply_state_store().by_contact.remove(&key);
        return;
    };
    let kind = if current.next_round_mention {
        RemoteImGroupReplyTimerKind::Mention
    } else {
        RemoteImGroupReplyTimerKind::NonMention
    };
    let delay = match kind {
        RemoteImGroupReplyTimerKind::Mention => {
            std::time::Duration::from_secs(pacing.assistant_debounce_seconds)
        }
        RemoteImGroupReplyTimerKind::NonMention => remote_im_group_reply_inspection_delay(
            &pacing,
            remote_im_group_reply_random_sample(),
        ),
        RemoteImGroupReplyTimerKind::Commit => return,
    };
    let action = {
        let mut store = lock_remote_im_group_reply_state_store();
        let Some(current) = store.by_contact.get(&key) else {
            return;
        };
        if current.generation != generation {
            return;
        }
        let mut next_event = current.event.clone();
        next_event.messages.clear();
        let next_generation = remote_im_group_reply_next_generation(&mut store);
        store.by_contact.insert(
            key.clone(),
            RemoteImGroupReplyState {
                generation: next_generation,
                phase: match kind {
                    RemoteImGroupReplyTimerKind::Mention => RemoteImGroupReplyPhase::MentionScheduled,
                    RemoteImGroupReplyTimerKind::NonMention => RemoteImGroupReplyPhase::NonMentionScheduled,
                    RemoteImGroupReplyTimerKind::Commit => return,
                },
                start_message_id: next_start.clone(),
                decision_end_message_id: None,
                focus: false,
                energy_settled: false,
                next_round_mention: false,
                event: next_event,
                due_at: std::time::Instant::now() + delay,
                inspection_kind: kind,
                pending_settlement: None,
            },
        );
        Some(RemoteImGroupReplyTimerAction {
            state_key: key,
            contact_id: contact.id.clone(),
            generation: next_generation,
            kind,
            delay,
        })
    };
    if let Some(action) = action {
        remote_im_group_reply_schedule_action(state, action);
    }
}

fn remote_im_group_reply_settle_generation(
    state: &AppState,
    contact: &RemoteImContact,
    generation: u64,
    settlement: RemoteImGroupReplySettlement,
) {
    match remote_im_persist_group_reply_settlement(state, contact, &settlement) {
        Ok(()) => remote_im_group_reply_advance_after_settlement(state, contact, generation),
        Err(err) => {
            const COMMIT_RETRY_DELAY: std::time::Duration = std::time::Duration::from_secs(5);
            let key = remote_im_group_reply_state_key(state, &contact.id);
            let action = {
                let mut store = lock_remote_im_group_reply_state_store();
                let current = store.by_contact.get_mut(&key);
                match current {
                    Some(current) if current.generation == generation => {
                        current.phase = RemoteImGroupReplyPhase::CommitPending;
                        current.pending_settlement = Some(settlement);
                        current.due_at = std::time::Instant::now() + COMMIT_RETRY_DELAY;
                        Some(RemoteImGroupReplyTimerAction {
                            state_key: key,
                            contact_id: contact.id.clone(),
                            generation,
                            kind: RemoteImGroupReplyTimerKind::Commit,
                            delay: COMMIT_RETRY_DELAY,
                        })
                    }
                    _ => None,
                }
            };
            runtime_log_warn(format!(
                "[群聊巡检] 结算落盘失败，已进入仅提交重试且禁止重复发送，contact_id={}，generation={}，error={}",
                contact.id, generation, err
            ));
            if let Some(action) = action {
                remote_im_group_reply_schedule_action(state, action);
            }
        }
    }
}

fn remote_im_group_reply_finish_generation(
    state: &AppState,
    contact: &RemoteImContact,
    generation: u64,
) {
    let key = remote_im_group_reply_state_key(state, &contact.id);
    let boundary_message_id = lock_remote_im_group_reply_state_store()
        .by_contact
        .get(&key)
        .filter(|current| current.generation == generation)
        .and_then(|current| current.decision_end_message_id.clone());
    let Some(boundary_message_id) = boundary_message_id else {
        return;
    };
    remote_im_group_reply_settle_generation(
        state,
        contact,
        generation,
        RemoteImGroupReplySettlement {
            boundary_message_id,
            final_text: None,
            outbound_key: None,
            platform_message_id: None,
            status: RemoteImGroupReplySettlementStatus::Delivered,
        },
    );
}

fn remote_im_group_reply_retry_after_dispatch_failure(
    state: &AppState,
    contact_id: &str,
    generation: u64,
    reason: &str,
) {
    match remote_im_group_reply_contact_latest(state, contact_id) {
        Ok(contact) => {
            let marker = state_service_get_remote_im_contact_checkpoint(state, contact_id).map(
                |checkpoint| {
                    checkpoint
                        .and_then(|checkpoint| checkpoint.group_reply_delivery)
                        .filter(|marker| {
                            marker.generation == generation && marker.status == "dispatching"
                        })
                },
            );
            match marker {
                Ok(Some(marker)) => {
                    runtime_log_warn(format!(
                        "[群聊巡检] 检测到外发已开始但回调超时，按结果不确定结算且禁止重发，contact_id={}，generation={}",
                        contact_id, generation
                    ));
                    remote_im_group_reply_complete_after_send(
                        state,
                        &contact,
                        generation,
                        marker,
                        None,
                        RemoteImGroupReplySettlementStatus::Uncertain,
                    );
                }
                Ok(None) => remote_im_group_reply_reschedule_non_mention(
                    state,
                    &contact,
                    generation,
                    reason,
                ),
                Err(err) => {
                    runtime_log_warn(format!(
                        "[群聊巡检] 外发标记读取失败，保持当前批次且暂不重发，contact_id={}，generation={}，error={}",
                        contact_id, generation, err
                    ));
                    let state = state.clone();
                    let contact_id = contact_id.to_string();
                    let reason = reason.to_string();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        let state = state.clone();
                        let contact_id = contact_id.clone();
                        let reason = reason.clone();
                        let _ = tokio::task::spawn_blocking(move || {
                            remote_im_group_reply_retry_after_dispatch_failure(
                                &state,
                                &contact_id,
                                generation,
                                &reason,
                            )
                        })
                        .await;
                    });
                }
            }
        }
        Err(err) => {
            if err.starts_with("群聊联系人已不存在") {
                runtime_log_info(format!(
                    "[群聊巡检] 联系人已删除，停止旧批次重试，contact_id={}，generation={}",
                    contact_id, generation
                ));
                let _ = clear_remote_im_debounces_for_contact(state, contact_id);
                return;
            }
            runtime_log_warn(format!(
                "[群聊巡检] 委托失败后联系人读取降级，contact_id={}，generation={}，error={}",
                contact_id, generation, err
            ));
            remote_im_group_reply_retry_generation(state, contact_id, generation, reason);
        }
    }
}

fn remote_im_group_reply_complete_after_send(
    state: &AppState,
    contact: &RemoteImContact,
    generation: u64,
    marker: RemoteImGroupReplyDeliveryMarker,
    platform_message_id: Option<String>,
    status: RemoteImGroupReplySettlementStatus,
) {
    remote_im_group_reply_settle_generation(
        state,
        contact,
        generation,
        RemoteImGroupReplySettlement {
            boundary_message_id: marker.boundary_message_id,
            final_text: Some(marker.final_text),
            outbound_key: Some(marker.outbound_key),
            platform_message_id,
            status,
        },
    );
}

fn remote_im_group_reply_contact_latest(
    state: &AppState,
    contact_id: &str,
) -> Result<RemoteImContact, String> {
    state_service_get_remote_im_contact(state, contact_id)?
        .ok_or_else(|| format!("群聊联系人已不存在：{contact_id}"))
}

fn remote_im_group_reply_message_matches_contact(
    message: &ChatMessage,
    contact: &RemoteImContact,
) -> bool {
    message.role.trim().eq_ignore_ascii_case("user")
        && message_origin_string(message, "kind") == Some("remote_im")
        && message_origin_string(message, "channel_id") == Some(contact.channel_id.trim())
        && message_origin_string(message, "contact_type")
            == Some(contact.remote_contact_type.trim())
        && message_origin_string(message, "contact_id") == Some(contact.remote_contact_id.trim())
}

fn read_remote_im_group_reply_range_to_latest(
    state: &AppState,
    contact: &RemoteImContact,
    conversation_id: &str,
    start_message_id: &str,
) -> Result<(Vec<ChatMessage>, String, bool), String> {
    const RANGE_PAGE_SIZE: usize = 100;
    let paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
    ensure_chat_store_conversation_readable(state, conversation_id, &paths)?;
    let mutation_gate = conversation_mutation_gate(&state.data_path, conversation_id)?;
    let _guard = mutation_gate.lock().map_err(|err| {
        named_lock_error(
            "conversation_mutation_gate",
            file!(),
            line!(),
            module_path!(),
            &err,
        )
    })?;
    let start = message_store::chat_store_read_message_by_id(&paths, start_message_id)?
        .ok_or_else(|| format!("群聊巡检起点消息不存在：{start_message_id}"))?;
    let mut range = vec![start];
    let mut after_message_id = start_message_id.to_string();
    loop {
        let Some(page) = message_store::chat_store_read_messages_after(
            &paths,
            &after_message_id,
            RANGE_PAGE_SIZE,
        )? else {
            break;
        };
        if page.messages.is_empty() {
            break;
        }
        let has_more = page.has_more;
        for message in page.messages {
            after_message_id = message.id.clone();
            range.push(message);
        }
        if !has_more {
            break;
        }
    }
    let end_message_id = range
        .iter()
        .rev()
        .find(|message| remote_im_group_reply_message_matches_contact(message, contact))
        .map(|message| message.id.clone())
        .ok_or_else(|| format!("群聊巡检范围没有联系人入站消息：{start_message_id}"))?;
    let focus = range
        .iter()
        .filter(|message| remote_im_group_reply_message_matches_contact(message, contact))
        .map(render_message_content_for_model)
        .any(|text| remote_im_group_reply_focus_matches(state, contact, &text));
    Ok((range, end_message_id, focus))
}

fn remote_im_group_reply_next_unsettled_start_message_id(
    state: &AppState,
    contact: &RemoteImContact,
    conversation_id: &str,
    boundary_message_id: &str,
) -> Result<Option<String>, String> {
    const PAGE_SIZE: usize = 100;
    let paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
    ensure_chat_store_conversation_readable(state, conversation_id, &paths)?;
    let mut after_message_id = boundary_message_id.to_string();
    loop {
        let Some(page) = message_store::chat_store_read_messages_after(
            &paths,
            &after_message_id,
            PAGE_SIZE,
        )? else {
            return Ok(None);
        };
        if page.messages.is_empty() {
            return Ok(None);
        }
        let has_more = page.has_more;
        for message in page.messages {
            after_message_id = message.id.clone();
            if remote_im_group_reply_message_matches_contact(&message, contact) {
                return Ok(Some(message.id));
            }
        }
        if !has_more {
            return Ok(None);
        }
    }
}

async fn remote_im_group_reply_handle_timer(
    state: &AppState,
    action: RemoteImGroupReplyTimerAction,
) {
    let initial_snapshot = {
        let store = lock_remote_im_group_reply_state_store();
        (|| {
            let current = store.by_contact.get(&action.state_key)?;
            if current.generation != action.generation
                || !remote_im_group_reply_phase_matches_timer(current.phase, action.kind)
            {
                return None;
            }
            Some(current.clone())
        })()
    };
    let Some(initial_snapshot) = initial_snapshot else {
        return;
    };
    let contact = match remote_im_group_reply_contact_latest(state, &action.contact_id) {
        Ok(contact) => contact,
        Err(err) => {
            if err.starts_with("群聊联系人已不存在") {
                runtime_log_info(format!(
                    "[群聊巡检] 联系人已删除，清理定时批次，contact_id={}，generation={}",
                    action.contact_id, action.generation
                ));
                let _ = clear_remote_im_debounces_for_contact(state, &action.contact_id);
                return;
            }
            runtime_log_warn(format!(
                "[群聊巡检] 定时读取联系人失败，已保留批次，contact_id={}，generation={}，error={}",
                action.contact_id, action.generation, err
            ));
            if action.kind == RemoteImGroupReplyTimerKind::Commit {
                remote_im_group_reply_schedule_action(
                    state,
                    RemoteImGroupReplyTimerAction {
                        delay: std::time::Duration::from_secs(5),
                        ..action
                    },
                );
            } else {
                remote_im_group_reply_retry_generation(
                    state,
                    &action.contact_id,
                    action.generation,
                    "定时读取联系人失败",
                );
            }
            return;
        }
    };
    if action.kind == RemoteImGroupReplyTimerKind::Commit {
        let Some(settlement) = initial_snapshot.pending_settlement else {
            runtime_log_warn(format!(
                "[群聊巡检] 提交重试缺少结算快照，保留状态等待后续恢复，contact_id={}，generation={}",
                contact.id, action.generation
            ));
            remote_im_group_reply_schedule_action(
                state,
                RemoteImGroupReplyTimerAction {
                    delay: std::time::Duration::from_secs(5),
                    ..action
                },
            );
            return;
        };
        remote_im_group_reply_settle_generation(state, &contact, action.generation, settlement);
        return;
    }
    let conversation_id = initial_snapshot.event.conversation_id.clone();
    let range_result = if let Some(decision_end_message_id) =
        initial_snapshot.decision_end_message_id.as_deref()
    {
        read_remote_im_debounce_secretary_messages(
            state,
            &conversation_id,
            &initial_snapshot.start_message_id,
            decision_end_message_id,
        )
        .map(|(_, batch)| {
            (
                batch,
                decision_end_message_id.to_string(),
                initial_snapshot.focus,
            )
        })
    } else {
        read_remote_im_group_reply_range_to_latest(
            state,
            &contact,
            &conversation_id,
            &initial_snapshot.start_message_id,
        )
    };
    let (range_messages, decision_end_message_id, focus) = match range_result {
        Ok(result) => result,
        Err(err) => {
            remote_im_group_reply_retry_generation(
                state,
                &contact.id,
                action.generation,
                &err,
            );
            return;
        }
    };
    let mut snapshot = {
        let mut store = lock_remote_im_group_reply_state_store();
        let Some(current) = store.by_contact.get_mut(&action.state_key) else {
            return;
        };
        if current.generation != action.generation
            || !remote_im_group_reply_phase_matches_timer(current.phase, action.kind)
        {
            return;
        }
        current.decision_end_message_id = Some(decision_end_message_id.clone());
        current.focus = focus;
        current.inspection_kind = action.kind;
        current.phase = match action.kind {
            RemoteImGroupReplyTimerKind::Mention => {
                RemoteImGroupReplyPhase::AssistantDispatching
            }
            RemoteImGroupReplyTimerKind::NonMention => {
                RemoteImGroupReplyPhase::SecretaryJudging
            }
            RemoteImGroupReplyTimerKind::Commit => RemoteImGroupReplyPhase::CommitPending,
        };
        current.clone()
    };
    if !snapshot.energy_settled {
        let inbound_messages = range_messages
            .iter()
            .filter(|message| remote_im_group_reply_message_matches_contact(message, &contact))
            .cloned()
            .collect::<Vec<_>>();
        if let Err(err) = remote_im_apply_group_energy_for_messages(
            state,
            &contact,
            &inbound_messages,
        ) {
            remote_im_group_reply_retry_generation(
                state,
                &contact.id,
                action.generation,
                &format!("巡检范围能量结算失败：{err}"),
            );
            return;
        }
        let mut store = lock_remote_im_group_reply_state_store();
        let Some(current) = store.by_contact.get_mut(&action.state_key) else {
            return;
        };
        if current.generation != action.generation {
            return;
        }
        current.energy_settled = true;
        snapshot = current.clone();
    }
    let gate = match {
        let state_for_blocking = state.clone();
        let contact_for_blocking = contact.clone();
        match tokio::task::spawn_blocking(move || {
            remote_im_group_reply_gate(&state_for_blocking, &contact_for_blocking, snapshot.focus)
        })
        .await
        {
            Ok(result) => result,
            Err(err) => {
                remote_im_group_reply_retry_generation(
                    state,
                    &contact.id,
                    action.generation,
                    &format!("能量门控任务失败：{err}"),
                );
                return;
            }
        }
    } {
        Ok(gate) => gate,
        Err(err) => {
            remote_im_group_reply_retry_generation(
                state,
                &contact.id,
                action.generation,
                &format!("读取能量或冷却状态失败：{err}"),
            );
            return;
        }
    };
    if !gate.allowed {
        remote_im_group_reply_reschedule_non_mention(
            state,
            &contact,
            action.generation,
            &gate.reason,
        );
        return;
    }
    let batch_preview = range_messages
        .iter()
        .rev()
        .find(|message| remote_im_group_reply_message_matches_contact(message, &contact))
        .map(render_message_content_for_model)
        .map(|text| remote_im_preview_text(&text, 100))
        .unwrap_or_else(|| "（无文本）".to_string());
    runtime_log_info(format!(
        "[群聊巡检] 开始：联系人={}，内容={}，轮次={}，路径={:?}，能量={:.2}，回复上限={}字",
        remote_im_contact_log_label(&contact),
        batch_preview,
        action.generation,
        action.kind,
        gate.energy,
        gate.max_chars
    ));
    let ready = RemoteImReplyDebounceReady {
        contact_id: contact.id.clone(),
        generation: action.generation,
        start_message_id: snapshot.start_message_id,
        end_message_id: snapshot
            .decision_end_message_id
            .unwrap_or_else(|| decision_end_message_id.clone()),
        focus: snapshot.focus,
        max_chars: gate.max_chars,
        path: match action.kind {
            RemoteImGroupReplyTimerKind::Mention => RemoteImReplyInspectionPath::Mention,
            RemoteImGroupReplyTimerKind::NonMention => RemoteImReplyInspectionPath::NonMention,
            RemoteImGroupReplyTimerKind::Commit => return,
        },
        event: snapshot.event,
    };
    match process_remote_im_reply_debounce(state, ready).await {
        Ok(RemoteImReplyDispatchOutcome::NoReply) => {
            remote_im_group_reply_finish_generation(state, &contact, action.generation);
        }
        Ok(RemoteImReplyDispatchOutcome::DispatchStarted) => {
            let state = state.clone();
            let contact_id = contact.id.clone();
            let generation = action.generation;
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(180)).await;
                if remote_im_group_reply_generation_is_current(&state, &contact_id, generation) {
                    let state = state.clone();
                    let contact_id = contact_id.clone();
                    let _ = tokio::task::spawn_blocking(move || {
                        remote_im_group_reply_retry_after_dispatch_failure(
                            &state,
                            &contact_id,
                            generation,
                            "远程应答等待发送回调超时",
                        )
                    })
                    .await;
                }
            });
        }
        Err(err) => remote_im_group_reply_retry_generation(
            state,
            &contact.id,
            action.generation,
            &err,
        ),
    }
}

fn observe_remote_im_persisted_event(
    state: &AppState,
    contact: &RemoteImContact,
    event: &ChatPendingEvent,
) {
    let Some(message) = remote_im_event_latest_user_message(event) else {
        return;
    };
    match remote_im_contact_is_muted(state, &contact.id) {
        Ok(true) => {
            let _ = clear_remote_im_debounces_for_contact(state, &contact.id);
            return;
        }
        Ok(false) => {}
        Err(err) => {
            runtime_log_warn(format!(
                "[群聊巡检] 闭嘴状态读取失败，已保留批次并在发送前再次校验，contact_id={}，message_id={}，error={}",
                contact.id, message.id, err
            ));
        }
    }
    let hits_mention = remote_im_event_hits_wake(contact, event);
    let pacing = effective_remote_im_group_reply_pacing(state, contact);
    let key = remote_im_group_reply_state_key(state, &contact.id);
    let mut action = None;
    {
        let mut store = lock_remote_im_group_reply_state_store();
            if let Some(phase) = store.by_contact.get(&key).map(|current| current.phase) {
                match phase {
                    RemoteImGroupReplyPhase::NonMentionScheduled if hits_mention => {
                        let generation = remote_im_group_reply_next_generation(&mut store);
                        let delay = std::time::Duration::from_secs(pacing.assistant_debounce_seconds);
                        let Some(current) = store.by_contact.get_mut(&key) else {
                            return;
                        };
                        current.generation = generation;
                        current.phase = RemoteImGroupReplyPhase::MentionScheduled;
                        current.inspection_kind = RemoteImGroupReplyTimerKind::Mention;
                        current.due_at = std::time::Instant::now() + delay;
                        action = Some(RemoteImGroupReplyTimerAction {
                            state_key: key.clone(),
                            contact_id: contact.id.clone(),
                            generation,
                            kind: RemoteImGroupReplyTimerKind::Mention,
                            delay,
                        });
                    }
                    RemoteImGroupReplyPhase::MentionScheduled => {}
                    RemoteImGroupReplyPhase::SecretaryJudging
                    | RemoteImGroupReplyPhase::AssistantDispatching
                    | RemoteImGroupReplyPhase::CommitPending if hits_mention => {
                        if let Some(current) = store.by_contact.get_mut(&key) {
                            current.next_round_mention = true;
                        }
                    }
                    _ => {}
                }
            } else {
                let generation = remote_im_group_reply_next_generation(&mut store);
                let start_message_id = message.id.clone();
                let kind = if hits_mention {
                    RemoteImGroupReplyTimerKind::Mention
                } else {
                    RemoteImGroupReplyTimerKind::NonMention
                };
                let delay = match kind {
                    RemoteImGroupReplyTimerKind::Mention => {
                        std::time::Duration::from_secs(pacing.assistant_debounce_seconds)
                    }
                    RemoteImGroupReplyTimerKind::NonMention => remote_im_group_reply_inspection_delay(
                        &pacing,
                        remote_im_group_reply_random_sample(),
                    ),
                    RemoteImGroupReplyTimerKind::Commit => return,
                };
                store.by_contact.insert(
                    key.clone(),
                    RemoteImGroupReplyState {
                        generation,
                        phase: match kind {
                            RemoteImGroupReplyTimerKind::Mention => {
                                RemoteImGroupReplyPhase::MentionScheduled
                            }
                            RemoteImGroupReplyTimerKind::NonMention => {
                                RemoteImGroupReplyPhase::NonMentionScheduled
                            }
                            RemoteImGroupReplyTimerKind::Commit => return,
                        },
                        start_message_id,
                        decision_end_message_id: None,
                        focus: false,
                        energy_settled: false,
                        next_round_mention: false,
                        event: {
                            let mut reference = event.clone();
                            reference.messages.clear();
                            reference
                        },
                        due_at: std::time::Instant::now() + delay,
                        inspection_kind: kind,
                        pending_settlement: None,
                    },
                );
                action = Some(RemoteImGroupReplyTimerAction {
                    state_key: key,
                    contact_id: contact.id.clone(),
                    generation,
                    kind,
                    delay,
                });
            }
    }
    if let Some(action) = action {
        runtime_log_info(format!(
            "[群聊巡检] 已安排：联系人={}，内容={}，轮次={}，路径={:?}，等待毫秒={}",
            remote_im_contact_log_label(contact),
            remote_im_preview_text(&render_message_content_for_model(message), 100),
            action.generation,
            action.kind,
            action.delay.as_millis()
        ));
        remote_im_group_reply_schedule_action(state, action);
    }
}

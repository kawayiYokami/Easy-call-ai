#[derive(Debug, Clone)]
struct RemoteImGroupReplyGate {
    allowed: bool,
    max_chars: u32,
    energy: f64,
    reason: String,
}

fn effective_remote_im_group_reply_pacing(
    state: &AppState,
    contact: &RemoteImContact,
) -> RemoteImGroupReplyPacing {
    let defaults = RemoteImGroupReplyPacing::default();
    let mut pacing = remote_im_channel_behavior_settings_for_contact(state, contact)
        .group_reply_pacing;
    pacing.assistant_debounce_seconds = pacing.assistant_debounce_seconds.max(1);
    pacing.secretary_inspection_seconds = pacing.secretary_inspection_seconds.max(1);
    pacing.inspection_jitter_ratio = if pacing.inspection_jitter_ratio.is_finite() {
        pacing.inspection_jitter_ratio.clamp(0.0, 1.0)
    } else {
        defaults.inspection_jitter_ratio
    };
    pacing.maximum_energy = if pacing.maximum_energy.is_finite() && pacing.maximum_energy > 0.0 {
        pacing.maximum_energy
    } else {
        defaults.maximum_energy
    };
    for value in [
        &mut pacing.base_reply_energy_cost,
        &mut pacing.energy_cost_per_character,
        &mut pacing.energy_recovery_per_second,
        &mut pacing.positive_energy_delta,
    ] {
        if !value.is_finite() || *value < 0.0 {
            *value = 0.0;
        }
    }
    if !pacing.negative_energy_delta.is_finite() || pacing.negative_energy_delta > 0.0 {
        pacing.negative_energy_delta = defaults.negative_energy_delta;
    }
    pacing.normal_reply_max_chars = pacing.normal_reply_max_chars.max(1);
    pacing.focus_reply_max_chars = pacing
        .focus_reply_max_chars
        .max(pacing.normal_reply_max_chars);
    pacing.positive_energy_phrases =
        normalize_contact_keyword_list(&pacing.positive_energy_phrases);
    pacing.negative_energy_phrases =
        normalize_contact_keyword_list(&pacing.negative_energy_phrases);
    pacing.focus_instructions = normalize_contact_keyword_list(&pacing.focus_instructions);
    pacing
}

fn remote_im_group_energy_at(
    checkpoint: Option<&RemoteImContactCheckpoint>,
    pacing: &RemoteImGroupReplyPacing,
    now: OffsetDateTime,
) -> f64 {
    let stored = checkpoint
        .and_then(|item| item.energy)
        .filter(|value| value.is_finite())
        .unwrap_or(pacing.maximum_energy)
        .clamp(-pacing.maximum_energy, pacing.maximum_energy);
    let elapsed = checkpoint
        .and_then(|item| item.energy_updated_at.as_deref())
        .and_then(parse_iso)
        .map(|updated_at| (now - updated_at).whole_seconds().max(0) as f64)
        .unwrap_or(0.0);
    (stored + pacing.energy_recovery_per_second * elapsed)
        .clamp(-pacing.maximum_energy, pacing.maximum_energy)
}

fn remote_im_bump_checkpoint_atomic_revision(checkpoint: &mut RemoteImContactCheckpoint) {
    checkpoint.atomic_revision = checkpoint.atomic_revision.saturating_add(1).max(1);
}

fn remote_im_group_energy_can_reply(energy: f64) -> bool {
    energy.is_finite() && energy > 0.0
}

fn remote_im_group_reply_gate(
    state: &AppState,
    contact: &RemoteImContact,
    focus: bool,
) -> Result<RemoteImGroupReplyGate, String> {
    if !contact.allow_receive || !contact.allow_send {
        return Ok(RemoteImGroupReplyGate {
            allowed: false,
            max_chars: 0,
            energy: 0.0,
            reason: "联系人未同时允许收发".to_string(),
        });
    }
    let config = state_read_config_cached(state)?;
    let Some(channel) = remote_im_channel_by_id(&config, &contact.channel_id) else {
        return Ok(RemoteImGroupReplyGate {
            allowed: false,
            max_chars: 0,
            energy: 0.0,
            reason: "联系人渠道不存在".to_string(),
        });
    };
    if !channel.enabled {
        return Ok(RemoteImGroupReplyGate {
            allowed: false,
            max_chars: 0,
            energy: 0.0,
            reason: "联系人渠道未启用".to_string(),
        });
    }
    if remote_im_contact_is_muted(state, &contact.id)? {
        return Ok(RemoteImGroupReplyGate {
            allowed: false,
            max_chars: 0,
            energy: 0.0,
            reason: "联系人处于闭嘴状态".to_string(),
        });
    }
    let checkpoint = state_service_get_remote_im_contact_checkpoint(state, &contact.id)?;
    let pacing = effective_remote_im_group_reply_pacing(state, contact);
    let now = now_utc();
    let energy = remote_im_group_energy_at(checkpoint.as_ref(), &pacing, now);
    if let Some(last_success_at) = checkpoint
        .as_ref()
        .and_then(|item| item.last_success_reply_at.as_deref())
        .and_then(parse_iso)
    {
        let elapsed = (now - last_success_at).whole_seconds().max(0) as u64;
        if elapsed < pacing.reply_cooldown_seconds {
            return Ok(RemoteImGroupReplyGate {
                allowed: false,
                max_chars: 0,
                energy,
                reason: format!(
                    "回复冷却中，剩余约 {} 秒",
                    pacing.reply_cooldown_seconds.saturating_sub(elapsed)
                ),
            });
        }
    }
    if !remote_im_group_energy_can_reply(energy) {
        return Ok(RemoteImGroupReplyGate {
            allowed: false,
            max_chars: 0,
            energy,
            reason: "当前能量小于或等于 0".to_string(),
        });
    }
    let configured_limit = if focus {
        pacing.focus_reply_max_chars
    } else {
        pacing.normal_reply_max_chars
    };
    Ok(RemoteImGroupReplyGate {
        allowed: true,
        max_chars: configured_limit,
        energy,
        reason: String::new(),
    })
}

fn remote_im_group_inbound_batch_delta(
    messages: &[ChatMessage],
    pacing: &RemoteImGroupReplyPacing,
) -> f64 {
    let mut positive_phrases = std::collections::HashSet::<String>::new();
    let mut negative_hits = 0usize;
    for message in messages {
        let text = render_message_content_for_model(message).to_lowercase();
        for phrase in &pacing.positive_energy_phrases {
            let phrase = phrase.trim().to_lowercase();
            if !phrase.is_empty() && text.contains(&phrase) {
                positive_phrases.insert(phrase);
            }
        }
        negative_hits = negative_hits.saturating_add(
            pacing
                .negative_energy_phrases
                .iter()
                .filter(|phrase| {
                    let phrase = phrase.trim().to_lowercase();
                    !phrase.is_empty() && text.contains(&phrase)
                })
                .count(),
        );
    }
    let cap = pacing.maximum_energy * 0.2;
    (positive_phrases.len() as f64 * pacing.positive_energy_delta).min(cap)
        + (negative_hits as f64 * pacing.negative_energy_delta).max(-cap)
}

fn remote_im_apply_group_energy_for_messages(
    state: &AppState,
    contact: &RemoteImContact,
    messages: &[ChatMessage],
) -> Result<(), String> {
    let pacing = effective_remote_im_group_reply_pacing(state, contact);
    let delta = remote_im_group_inbound_batch_delta(messages, &pacing);
    if delta.abs() <= f64::EPSILON {
        return Ok(());
    }
    let now = now_utc();
    let mut checkpoint = state_service_get_remote_im_contact_checkpoint(state, &contact.id)?.unwrap_or(
        RemoteImContactCheckpoint {
            contact_id: contact.id.clone(),
            ..RemoteImContactCheckpoint::default()
        },
    );
    let before = {
        let checkpoint_ref = &mut checkpoint;
        let before = remote_im_group_energy_at(Some(checkpoint_ref), &pacing, now);
        checkpoint_ref.energy = Some(
            (before + delta).clamp(-pacing.maximum_energy, pacing.maximum_energy),
        );
        checkpoint_ref.energy_updated_at = Some(now_iso());
        remote_im_bump_checkpoint_atomic_revision(checkpoint_ref);
        checkpoint_ref.updated_at = Some(now_iso());
        before
    };
    state_service_set_remote_im_contact_checkpoint(state, &checkpoint)?;
    runtime_log_debug(format!(
        "[群聊能量] 巡检范围词库结算：联系人={}，结算前={:.2}，变化={:.2}",
        remote_im_contact_log_label(contact), before, delta
    ));
    remote_im_emit_contact_dashboard_snapshot(state, &contact.id);
    Ok(())
}

#[cfg(test)]
fn remote_im_apply_inbound_group_energy(
    state: &AppState,
    contact: &RemoteImContact,
    _sender_id: &str,
    text: &str,
) -> Result<(), String> {
    let message = ChatMessage {
        id: String::new(),
        role: "user".to_string(),
        created_at: now_iso(),
        speaker_agent_id: None,
        parts: vec![MessagePart::Text {
            text: text.to_string(),
            reasoning_content: None,
        }],
        extra_text_blocks: Vec::new(),
        provider_meta: None,
        tool_call: None,
        mcp_call: None,
        meme_annotations: None,
    };
    remote_im_apply_group_energy_for_messages(state, contact, std::slice::from_ref(&message))
}

fn remote_im_prepare_group_reply_delivery(
    state: &AppState,
    contact: &RemoteImContact,
    generation: u64,
    final_text: &str,
) -> Result<RemoteImGroupReplyDeliveryMarker, String> {
    let key = remote_im_group_reply_state_key(state, &contact.id);
    let boundary_message_id = {
        let store = lock_remote_im_group_reply_state_store();
        let current = store
            .by_contact
            .get(&key)
            .ok_or_else(|| format!("群聊巡检批次已结束：{}", contact.id))?;
        if current.generation != generation
            || current.phase != RemoteImGroupReplyPhase::AssistantDispatching
        {
            return Err(format!(
                "群聊巡检批次已失效：contact_id={}，generation={generation}",
                contact.id
            ));
        }
        current
            .decision_end_message_id
            .clone()
            .ok_or_else(|| {
                format!(
                    "群聊巡检尚未冻结边界：contact_id={}，generation={generation}",
                    contact.id
                )
            })?
    };
    let outbound_key = format!(
        "group-reply::{}::{}::{}",
        contact.id, generation, boundary_message_id
    );
    let mut checkpoint = state_service_get_remote_im_contact_checkpoint(state, &contact.id)?.unwrap_or(
        RemoteImContactCheckpoint {
            contact_id: contact.id.clone(),
            ..RemoteImContactCheckpoint::default()
        },
    );
    if checkpoint
        .group_reply_delivery
        .as_ref()
        .map(|marker| {
            marker.outbound_key == outbound_key && marker.status != "preflight_failed"
        })
        .unwrap_or(false)
    {
        return Err(format!("群聊外发批次已有持久标记：{outbound_key}"));
    }
    let marker = RemoteImGroupReplyDeliveryMarker {
        generation,
        boundary_message_id,
        outbound_key,
        final_text: final_text.to_string(),
        status: "dispatching".to_string(),
        platform_message_id: None,
        energy_applied: false,
        updated_at: Some(now_iso()),
    };
    checkpoint.group_reply_delivery = Some(marker.clone());
    remote_im_bump_checkpoint_atomic_revision(&mut checkpoint);
    checkpoint.updated_at = Some(now_iso());
    state_service_set_remote_im_contact_checkpoint(state, &checkpoint)?;
    Ok(marker)
}

fn remote_im_cancel_prepared_group_reply_delivery(
    state: &AppState,
    contact_id: &str,
    marker: &RemoteImGroupReplyDeliveryMarker,
    reason: &str,
) -> Result<(), String> {
    let mut checkpoint = state_service_get_remote_im_contact_checkpoint(state, contact_id)?.unwrap_or(
        RemoteImContactCheckpoint {
            contact_id: contact_id.to_string(),
            ..RemoteImContactCheckpoint::default()
        },
    );
    let changed = {
        let checkpoint = &mut checkpoint;
        let Some(current) = checkpoint.group_reply_delivery.as_mut() else {
            return Ok(());
        };
        if current.outbound_key != marker.outbound_key || current.status != "dispatching" {
            false
        } else {
            current.status = "preflight_failed".to_string();
            current.updated_at = Some(now_iso());
            remote_im_bump_checkpoint_atomic_revision(checkpoint);
            checkpoint.updated_at = Some(now_iso());
            true
        }
    };
    if changed {
        state_service_set_remote_im_contact_checkpoint(state, &checkpoint)?;
    }
    if !changed {
        return Ok(());
    }
    runtime_log_warn(format!(
        "[群聊巡检] 外发前置检查失败，已撤销发送标记并保留批次，contact_id={}，outbound_key={}，reason={}",
        contact_id, marker.outbound_key, reason
    ));
    Ok(())
}

fn remote_im_persist_group_reply_settlement(
    state: &AppState,
    contact: &RemoteImContact,
    settlement: &RemoteImGroupReplySettlement,
) -> Result<(), String> {
    let now = now_utc();
    let settlement_status = match settlement.status {
        RemoteImGroupReplySettlementStatus::Delivered => "committed",
        RemoteImGroupReplySettlementStatus::Uncertain => "uncertain",
    };
    let mut checkpoint = state_service_get_remote_im_contact_checkpoint(state, &contact.id)?.unwrap_or(
        RemoteImContactCheckpoint {
            contact_id: contact.id.clone(),
            ..RemoteImContactCheckpoint::default()
        },
    );
    let applied = {
        let checkpoint = &mut checkpoint;
        if settlement.outbound_key.as_ref().is_some_and(|outbound_key| {
            checkpoint
                .group_reply_delivery
                .as_ref()
                .is_some_and(|marker| marker.outbound_key != *outbound_key)
        }) {
            false
        } else {
            let existing_marker = settlement
                .outbound_key
                .as_deref()
                .zip(checkpoint.group_reply_delivery.as_ref())
                .and_then(|(outbound_key, marker)| {
                    (marker.outbound_key == outbound_key).then(|| marker.clone())
                });
        let energy_already_applied = existing_marker
            .as_ref()
            .map(|marker| marker.energy_applied)
            .unwrap_or(false);
        let previous_status = existing_marker
            .as_ref()
            .map(|marker| marker.status.as_str())
            .unwrap_or_default();
        let mut energy_applied = energy_already_applied;
        if !energy_already_applied {
            if let Some(final_text) = settlement.final_text.as_deref() {
                let pacing = effective_remote_im_group_reply_pacing(state, contact);
                let before = remote_im_group_energy_at(Some(checkpoint), &pacing, now);
                let char_count = effective_remote_im_group_reply_char_count(final_text) as f64;
                let cost = pacing.base_reply_energy_cost
                    + char_count * pacing.energy_cost_per_character;
                let after = (before - cost).max(-pacing.maximum_energy);
                checkpoint.energy = Some(after);
                checkpoint.energy_updated_at = Some(now_iso());
                energy_applied = true;
                runtime_log_info(format!(
                    "[群聊能量] 完成，contact_id={}，before={:.2}，cost={:.2}，after={:.2}，chars={}，delivery_status={}",
                    contact.id,
                    before,
                    cost,
                    after,
                    char_count as usize,
                    settlement_status
                ));
            }
        }
        if settlement.status == RemoteImGroupReplySettlementStatus::Delivered
            && previous_status != "committed"
        {
            checkpoint.last_success_reply_at = Some(now_iso());
        }
        checkpoint.last_boundary_message_id = Some(settlement.boundary_message_id.clone());
        checkpoint.last_boundary_covers_message_id = Some(settlement.boundary_message_id.clone());
        checkpoint.updated_at = Some(now_iso());
        if let Some(outbound_key) = settlement.outbound_key.as_ref() {
            checkpoint.group_reply_delivery = Some(RemoteImGroupReplyDeliveryMarker {
                generation: existing_marker
                    .as_ref()
                    .map(|marker| marker.generation)
                    .unwrap_or_default(),
                boundary_message_id: settlement.boundary_message_id.clone(),
                outbound_key: outbound_key.clone(),
                final_text: settlement
                    .final_text
                    .clone()
                    .or_else(|| existing_marker.as_ref().map(|marker| marker.final_text.clone()))
                    .unwrap_or_default(),
                status: settlement_status.to_string(),
                platform_message_id: settlement
                    .platform_message_id
                    .clone()
                    .or_else(|| {
                        existing_marker
                            .as_ref()
                            .and_then(|marker| marker.platform_message_id.clone())
                    }),
                energy_applied,
                updated_at: Some(now_iso()),
            });
        }
        remote_im_bump_checkpoint_atomic_revision(checkpoint);
        true
        }
    };
    if applied {
        state_service_set_remote_im_contact_checkpoint(state, &checkpoint)?;
    }
    if !applied {
        runtime_log_warn(format!(
            "[群聊巡检] 跳过过期外发结算，避免覆盖较新的发送标记，contact_id={}，outbound_key={}",
            contact.id,
            settlement.outbound_key.as_deref().unwrap_or("")
        ));
    } else {
        remote_im_emit_contact_dashboard_snapshot(state, &contact.id);
    }
    Ok(())
}

fn remote_im_recover_group_reply_delivery_marker(
    state: &AppState,
    contact: &RemoteImContact,
) -> Result<(), String> {
    let marker = state_service_get_remote_im_contact_checkpoint(state, &contact.id)?
        .and_then(|checkpoint| checkpoint.group_reply_delivery)
        .filter(|marker| marker.status == "dispatching");
    let Some(marker) = marker else {
        return Ok(());
    };
    let state_key = remote_im_group_reply_state_key(state, &contact.id);
    let current_runtime = lock_remote_im_group_reply_state_store()
        .by_contact
        .get(&state_key)
        .map(|current| (current.generation, current.phase));
    if current_runtime
        .map(|(generation, _)| generation > marker.generation)
        .unwrap_or(false)
    {
        if let Err(err) = remote_im_cancel_prepared_group_reply_delivery(
            state,
            &contact.id,
            &marker,
            "内存状态已进入更新 generation，旧发送标记判定为前置失败遗留",
        ) {
            runtime_log_warn(format!(
                "[群聊巡检] 旧发送标记清理降级，暂不按不确定结果扣能，contact_id={}，outbound_key={}，error={}",
                contact.id, marker.outbound_key, err
            ));
        }
        return Ok(());
    }
    let active_delivery = current_runtime
        .map(|(generation, phase)| {
            generation == marker.generation
                && matches!(
                    phase,
                    RemoteImGroupReplyPhase::AssistantDispatching
                        | RemoteImGroupReplyPhase::CommitPending
                )
        })
        .unwrap_or(false);
    if active_delivery {
        return Ok(());
    }
    runtime_log_warn(format!(
        "[群聊巡检] 恢复未确认外发，contact_id={}，outbound_key={}，处理=按结果不确定消费批次且禁止重发",
        contact.id, marker.outbound_key
    ));
    remote_im_persist_group_reply_settlement(
        state,
        contact,
        &RemoteImGroupReplySettlement {
            boundary_message_id: marker.boundary_message_id,
            final_text: Some(marker.final_text),
            outbound_key: Some(marker.outbound_key),
            platform_message_id: marker.platform_message_id,
            status: RemoteImGroupReplySettlementStatus::Uncertain,
        },
    )
}

fn remote_im_recover_all_group_reply_delivery_markers(
    state: &AppState,
) -> Result<(usize, usize), String> {
    let pending_contact_ids = state_service_list_remote_im_contact_checkpoints(state)?
        .into_iter()
        .filter_map(|checkpoint| {
            checkpoint
                .group_reply_delivery
                .as_ref()
                .filter(|marker| marker.status == "dispatching")
                .map(|_| checkpoint.contact_id.clone())
        })
        .collect::<Vec<_>>();
    let mut recovered = 0usize;
    let mut failed = 0usize;
    for contact_id in &pending_contact_ids {
        let Some(contact) = state_service_get_remote_im_contact(state, contact_id)?
        else {
            runtime_log_warn(format!(
                "[群聊巡检] 启动恢复跳过孤立发送标记，contact_id={}，reason=联系人已不存在",
                contact_id
            ));
            continue;
        };
        match remote_im_recover_group_reply_delivery_marker(state, &contact) {
            Ok(()) => recovered = recovered.saturating_add(1),
            Err(err) => {
                failed = failed.saturating_add(1);
                runtime_log_warn(format!(
                    "[群聊巡检] 启动恢复失败，contact_id={}，error={}",
                    contact_id, err
                ));
            }
        }
    }
    Ok((recovered, failed))
}

#[cfg(test)]
mod remote_im_group_reply_energy_tests {
    use super::*;

    #[test]
    fn group_energy_should_recover_with_symmetric_bounds() {
        let pacing = RemoteImGroupReplyPacing {
            maximum_energy: 100.0,
            energy_recovery_per_second: 2.0,
            ..RemoteImGroupReplyPacing::default()
        };
        let now = now_utc();
        let checkpoint = RemoteImContactCheckpoint {
            energy: Some(40.0),
            energy_updated_at: Some(
                (now - time::Duration::seconds(20))
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default(),
            ),
            ..RemoteImContactCheckpoint::default()
        };
        assert!(
            (remote_im_group_energy_at(Some(&checkpoint), &pacing, now) - 80.0).abs() < 0.01
        );

        let negative_checkpoint = RemoteImContactCheckpoint {
            energy: Some(-50.0),
            energy_updated_at: Some(
                (now - time::Duration::seconds(20))
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default(),
            ),
            ..RemoteImContactCheckpoint::default()
        };
        assert!(
            (remote_im_group_energy_at(Some(&negative_checkpoint), &pacing, now) + 10.0).abs()
                < 0.01
        );

        let below_floor_checkpoint = RemoteImContactCheckpoint {
            energy: Some(-150.0),
            energy_updated_at: Some(now_iso()),
            ..RemoteImContactCheckpoint::default()
        };
        assert_eq!(
            remote_im_group_energy_at(Some(&below_floor_checkpoint), &pacing, now),
            -100.0
        );

        let capped_checkpoint = RemoteImContactCheckpoint {
            energy: Some(90.0),
            energy_updated_at: Some(
                (now - time::Duration::seconds(20))
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default(),
            ),
            ..RemoteImContactCheckpoint::default()
        };
        assert!(
            (remote_im_group_energy_at(Some(&capped_checkpoint), &pacing, now) - 100.0).abs()
                < 0.01
        );
    }

    #[test]
    fn group_energy_reply_gate_should_require_only_positive_energy() {
        assert!(!remote_im_group_energy_can_reply(-0.01));
        assert!(!remote_im_group_energy_can_reply(0.0));
        assert!(remote_im_group_energy_can_reply(0.01));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationIdOnlyInput {
    conversation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConversationCommandStatus {
    success: bool,
}

#[tauri::command]
async fn archive_conversation(
    input: ConversationIdOnlyInput,
    state: State<'_, AppState>,
) -> Result<ConversationCommandStatus, String> {
    let requested_conversation_id = input.conversation_id.trim();
    if requested_conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    runtime_log_info(format!(
        "[归档] 开始，任务=手动归档，conversation_id={}",
        requested_conversation_id
    ));
    let (selected_api, resolved_api, source, effective_agent_id) =
        match resolve_archive_request_conversation_by_id(state.inner(), requested_conversation_id) {
            Ok(resolved) => resolved,
            Err(err) => return Err(log_manual_archive_failure(requested_conversation_id, err)),
        };
    let already_archived = conversation_is_archived(&source);
    let runtime = state_read_runtime_state_cached(state.inner())
        .map_err(|err| log_manual_archive_failure(&source.id, err))?;
    let main_conversation_id = runtime
        .main_conversation_id
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    if !already_archived && source.id.trim() == main_conversation_id {
        return Err(log_manual_archive_failure(
            &source.id,
            "系统通知会话暂不支持归档。".to_string(),
        ));
    }
    let conversation_runtime_state = get_conversation_runtime_state(state.inner(), &source.id)
        .map_err(|err| log_manual_archive_failure(&source.id, err))?;
    if !already_archived && conversation_runtime_state == MainSessionState::OrganizingContext {
        return Err(log_manual_archive_failure(
            &source.id,
            "强制归档正在进行中，请稍候。".to_string(),
        ));
    }
    if !already_archived {
        verify_archive_source_message_integrity(state.inner(), &source)
            .map_err(|err| log_manual_archive_failure(&source.id, err))?;
    }
    let archive_result = instant_archive_conversation(state.inner(), &selected_api, &source)
        .map_err(|err| log_manual_archive_failure(&source.id, err))?;
    flush_pending_persists_blocking(state.inner()).map_err(|err| {
        log_manual_archive_failure(&source.id, format!("归档状态写入失败：{}", err))
    })?;
    emit_unarchived_conversation_overview_updated_payload(
        state.inner(),
        &archive_result.overview_payload,
    );
    let active_conversation_id = archive_result.active_conversation_id.clone();

    if !archive_result.already_archived {
        let state_cloned = state.inner().clone();
        let selected_api_cloned = selected_api.clone();
        let resolved_api_cloned = resolved_api.clone();
        let source_conversation_id = source.id.clone();
        let effective_agent_id_cloned = effective_agent_id.clone();
        let active_conversation_id_for_background = active_conversation_id.clone();
        tauri::async_runtime::spawn(async move {
            let panic_safe_task = std::panic::AssertUnwindSafe(async {
                let source_cloned = match conversation_service_v2()
                    .read_archive_pipeline_source_conversation(
                        &state_cloned,
                        &source_conversation_id,
                    ) {
                    Ok(conversation) => conversation,
                    Err(err) => {
                        runtime_log_warn(format!(
                            "[归档] 失败，任务=后台归档维护，conversation_id={}，error=读取归档流水线源会话失败：{}",
                            source_conversation_id, err
                        ));
                        trigger_chat_queue_processing(&state_cloned);
                        return;
                    }
                };
                let result = run_archive_pipeline(
                    &state_cloned,
                    &selected_api_cloned,
                    &resolved_api_cloned,
                    &source_cloned,
                    &effective_agent_id_cloned,
                    Some(active_conversation_id_for_background.as_str()),
                    None,
                    "archive_conversation",
                    "ARCHIVE-FORCE",
                )
                .await;
                if let Err(err) = result {
                    runtime_log_warn(format!(
                        "[归档] 失败，任务=后台归档维护，conversation_id={}，error={}",
                        source_cloned.id, err
                    ));
                }
                trigger_chat_queue_processing(&state_cloned);
            });
            if futures_util::FutureExt::catch_unwind(panic_safe_task)
                .await
                .is_err()
            {
                eprintln!(
                    "[归档] 失败，任务=后台归档维护，conversation_id={}，error=panic",
                    source_conversation_id
                );
                trigger_chat_queue_processing(&state_cloned);
            }
        });
    }

    runtime_log_info(format!(
        "[归档] 完成，任务=手动归档，conversation_id={}，already_archived={}",
        source.id, archive_result.already_archived
    ));
    Ok(ConversationCommandStatus { success: true })
}

pub(crate) async fn run_archive_pipeline(
    state: &AppState,
    selected_api: &ApiConfig,
    resolved_api: &ResolvedApiConfig,
    source: &Conversation,
    effective_agent_id: &str,
    prepared_active_conversation_id: Option<&str>,
    _target_conversation_id: Option<&str>,
    archive_reason: &str,
    trace_tag: &str,
) -> Result<ForceArchiveResult, String> {
    let started_at = std::time::Instant::now();
    let trace_id = Uuid::new_v4().to_string();
    let reflection_source = conversation_service_v2()
        .read_archive_pipeline_last_block_conversation(state, &source.id)
        .map_err(|err| format!("读取归档反思最后块失败：{}", err))?;

    eprintln!(
        "[ARCHIVE-PIPELINE] 开始: task=archive_maintenance, trace_id={}, agent_id={}, api_id={}, started_at={}",
        trace_id, effective_agent_id, selected_api.id, started_at.elapsed().as_millis()
    );

    let result = run_archive_pipeline_inner(
        state,
        selected_api,
        resolved_api,
        source,
        &reflection_source,
        effective_agent_id,
        prepared_active_conversation_id,
        None,
        archive_reason,
        trace_tag,
        started_at,
        &trace_id,
    )
    .await;

    let elapsed_ms = started_at.elapsed().as_millis();
    eprintln!(
        "[ARCHIVE-PIPELINE] 完成: task=archive_maintenance, trace_id={}, agent_id={}, api_id={}, elapsed_ms={}",
        trace_id, effective_agent_id, selected_api.id, elapsed_ms
    );

    result
}

async fn run_archive_pipeline_inner(
    state: &AppState,
    selected_api: &ApiConfig,
    resolved_api: &ResolvedApiConfig,
    source: &Conversation,
    reflection_source: &Conversation,
    _effective_agent_id: &str,
    prepared_active_conversation_id: Option<&str>,
    _target_conversation_id: Option<&str>,
    archive_reason: &str,
    trace_tag: &str,
    started_at: std::time::Instant,
    trace_id: &str,
) -> Result<ForceArchiveResult, String> {
    let reflection_skip_warning = archive_reflection_skip_reason(reflection_source);
    runtime_log_info(format!(
        "[归档反思] 开始，任务=最后块正文反思，conversation_id={}，full_body_message_count={}，last_block_body_message_count={}",
        source.id,
        archive_pipeline_message_count_for_delete(source),
        archive_pipeline_message_count_for_delete(reflection_source)
    ));
    if let Some(reason) = reflection_skip_warning.as_ref() {
        runtime_log_info(format!(
            "[归档] 跳过归档反思，任务=后台归档维护，conversation_id={}，原因={}，行为=直接完成归档，不阻塞主流程",
            source.id, reason
        ));
    }

    let reporting_source = build_archive_reporting_conversation(reflection_source);
    let (archive_warning, applied_report, archive_body_tokens) =
        if let Some(reason) = reflection_skip_warning.clone() {
            (
                Some(reason),
                None,
                archive_body_token_count(reporting_source.as_ref()),
            )
        } else {
            match resolve_archive_owner_context(state, source) {
                Ok((owner_agent, owner_agent_id, user_alias)) => {
                    let memories = memory_store_list_memories_visible_for_agent(
                        &state.data_path,
                        &owner_agent_id,
                        owner_agent.private_memory_enabled,
                    )?;

                    eprintln!(
                        "[{}] trace={} begin api={} model={} format={} conversation={} ownerAgent={}",
                        trace_tag,
                        trace_id,
                        selected_api.id,
                        selected_api.model,
                        resolved_api.request_format,
                        source.id,
                        owner_agent_id
                    );

                    let body_reporting_source = build_archive_body_reporting_conversation(
                        reporting_source.as_ref(),
                        &memories,
                    );
                    let archive_body_tokens = archive_body_token_count(&body_reporting_source);
                    if archive_body_tokens >= ARCHIVE_REFLECTION_MIN_BODY_TOKENS {
                        let (summary_draft, archive_warning) =
                            summarize_archive_summary_with_fallback(
                                state,
                                resolved_api,
                                selected_api,
                                &owner_agent,
                                &user_alias,
                                &body_reporting_source,
                                &memories,
                            )
                            .await;
                        let deduped_recall =
                            archive_pipeline_dedup_recall_table(&source.memory_recall_table);
                        let applied_report = apply_summary_context_result(
                            &state.data_path,
                            &owner_agent,
                            &deduped_recall,
                            &summary_draft,
                        )?;
                        (archive_warning, Some(applied_report), archive_body_tokens)
                    } else {
                        runtime_log_info(format!(
                            "[SummaryContext] 跳过，场景=archive，conversation_id={}，原因=正文不足1000token，body_tokens={:.0}，threshold={:.0}",
                            source.id,
                            archive_body_tokens,
                            ARCHIVE_REFLECTION_MIN_BODY_TOKENS
                        ));
                        (None, None, archive_body_tokens)
                    }
                }
                Err(err) => {
                    runtime_log_warn(format!(
                        "[归档] 跳过归档反思，任务=后台归档维护，conversation_id={}，原因={}，行为=直接完成归档，不阻塞主流程",
                        source.id, err
                    ));
                    (
                        Some(format!("归档反思已跳过：{}", err)),
                        None,
                        archive_body_token_count(reporting_source.as_ref()),
                    )
                }
            }
        };

    let archived_conversation = conversation_service_v2()
        .get_conversation_meta(state, &source.id)
        .map_err(|_| "归档后维护失败：会话已不存在。".to_string())?;
    if archived_conversation.status.trim() != "archived"
        && archived_conversation
            .archived_at
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
        && archived_conversation.summary.trim().is_empty()
    {
        return Err("归档后维护失败：会话尚未标记为已归档。".to_string());
    }
    let archive_id = archived_conversation.id.to_string();
    eprintln!(
        "[归档] 开始，任务=后台维护，conversation_id={}，reason=\"{}\"",
        archived_conversation.id, archive_reason
    );
    clear_screenshot_artifact_cache();
    mark_tasks_as_session_lost(&state.data_path, &source.id);
    let active_conversation_id = prepared_active_conversation_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            match conversation_service_v2().resolve_latest_foreground_conversation_id(state, "") {
                Ok(value) => value,
                Err(err) => {
                    runtime_log_warn(format!(
                        "[归档] 警告，任务=resolve_latest_foreground_conversation_id_after_archive，source_conversation_id={}，error={}",
                        source.id, err
                    ));
                    None
                }
            }
        })
        .ok_or_else(|| "归档后未能确定当前前台会话。".to_string())?;
    match delegate_runtime_thread_conversation_delete_by_root(state, &source.id) {
        Ok(deleted_count) => runtime_log_info(format!(
            "[委托会话] 完成，任务=随会话归档级联清理，root_conversation_id={}，deleted_count={}",
            source.id, deleted_count
        )),
        Err(err) => runtime_log_warn(format!(
            "[委托会话] 失败，任务=随会话归档级联清理，root_conversation_id={}，error={}",
            source.id, err
        )),
    }

    match cleanup_backup_records_from_messages(&state.data_path, &source.messages) {
        Ok(cleaned) if cleaned > 0 => {
            eprintln!(
                "[归档] apply_patch 备份清理完成: conversation={}, cleaned={}",
                source.id, cleaned
            );
        }
        Err(err) => {
            eprintln!(
                "[归档] apply_patch 备份清理失败: conversation={}, error={}",
                source.id, err
            );
        }
        _ => {}
    }

    if let Err(e) = cleanup_pdf_cache_for_conversation(&state, &source.id) {
        eprintln!(
            "[归档] 清理 PDF 缓存失败: conversation={}, error={}",
            source.id, e
        );
    }

    if let Some(applied_report) = applied_report.as_ref() {
        eprintln!(
            "[SummaryContext] 完成，场景=archive，trace_id={}，conversation_id={}，merged_memories={}，merged_groups={}，profile_applied={}，profile_skipped={}，useful_accept={}，penalized={}，natural_decay={}",
            trace_id,
            source.id,
            applied_report.merged_memories,
            applied_report.merged_groups,
            applied_report.applied_profile_memories,
            applied_report.skipped_profile_memories,
            applied_report.memory_feedback.useful_accepted_count,
            applied_report.memory_feedback.penalized_count,
            applied_report.memory_feedback.natural_decay_count
        );
    } else {
        eprintln!(
            "[SummaryContext] 跳过完成，场景=archive，trace_id={}，conversation_id={}，body_tokens={:.0}",
            trace_id,
            source.id,
            archive_body_tokens
        );
    }
    let merged_memories = applied_report
        .as_ref()
        .map(|report| report.merged_memories)
        .unwrap_or(0);
    let merge_groups = applied_report
        .as_ref()
        .map(|report| report.merged_groups);
    let memory_feedback = applied_report.map(|report| report.memory_feedback);

    Ok(ForceArchiveResult {
        archived: true,
        archive_id: Some(archive_id),
        active_conversation_id: Some(active_conversation_id),
        compaction_message: None,
        summary: String::new(),
        merged_memories,
        warning: archive_warning,
        reason_code: None,
        elapsed_ms: Some(started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64),
        memory_feedback,
        merge_groups,
    })
}

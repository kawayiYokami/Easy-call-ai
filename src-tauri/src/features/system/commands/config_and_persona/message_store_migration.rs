const MESSAGE_STORE_MIGRATION_PROGRESS_EVENT: &str = "easy-call:message-store-migration-progress";

fn message_store_migration_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

fn lock_message_store_migration() -> std::sync::MutexGuard<'static, ()> {
    message_store_migration_lock().lock().unwrap_or_else(|poison| {
        runtime_log_info(format!(
            "[消息存储迁移] 迁移锁已污染，继续串行执行恢复，error={:?}",
            poison
        ));
        poison.into_inner()
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MessageStoreMigrationPreflightReport {
    migration_required: bool,
    total_conversations: usize,
    ready_count: usize,
    legacy_count: usize,
    busy_count: usize,
    blocked_count: usize,
    can_auto_migrate: bool,
    items: Vec<MessageStoreMigrationPreflightItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MessageStoreMigrationPreflightItem {
    conversation_id: String,
    title: String,
    status: String,
    message_count: usize,
    reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunMessageStoreMigrationInput {}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MessageStoreMigrationRunReport {
    migrated_count: usize,
    skipped_ready_count: usize,
    discarded_count: usize,
    failed_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MessageStoreMigrationProgressPayload {
    current: usize,
    total: usize,
    conversation_id: String,
    title: String,
    status: String,
    detail: Option<String>,
}

fn emit_message_store_migration_progress(
    app: &AppHandle,
    payload: MessageStoreMigrationProgressPayload,
) {
    if let Err(err) = app.emit(MESSAGE_STORE_MIGRATION_PROGRESS_EVENT, payload) {
        runtime_log_error(format!(
            "[消息存储迁移] 进度事件发送失败：event={}，error={:?}",
            MESSAGE_STORE_MIGRATION_PROGRESS_EVENT, err
        ));
    }
}

fn message_store_migration_candidate_ids(data_path: &PathBuf) -> Vec<String> {
    let conversations_dir = app_layout_chat_conversations_dir(data_path);
    let mut ids = std::collections::BTreeSet::<String>::new();
    if let Ok(entries) = fs::read_dir(conversations_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) == Some("json") {
                if let Some(id) = path.file_stem().and_then(|value| value.to_str()) {
                    if !id.trim().is_empty() {
                        ids.insert(id.trim().to_string());
                    }
                }
                continue;
            }
            if path.is_dir() {
                if let Some(id) = path.file_name().and_then(|value| value.to_str()) {
                    if !id.trim().is_empty() {
                        ids.insert(id.trim().to_string());
                    }
                }
            }
        }
    }
    ids.into_iter().collect()
}

fn preflight_legacy_conversation(
    data_path: &PathBuf,
    conversation_id: &str,
) -> MessageStoreMigrationPreflightItem {
    match read_json_file::<Conversation>(
        &app_layout_chat_conversation_path(data_path, conversation_id),
        "conversation file",
    ) {
        Ok(conversation) => {
            if conversation.id.trim() != conversation_id {
                return MessageStoreMigrationPreflightItem {
                    conversation_id: conversation_id.to_string(),
                    title: conversation.title,
                    status: "discarded".to_string(),
                    message_count: conversation.messages.len(),
                    reason: Some(format!(
                        "会话文件名与内部 ID 不一致：file_id={}，conversation_id={}",
                        conversation_id, conversation.id
                    )),
                };
            }
            let paths = match message_store::message_store_paths(data_path, conversation_id) {
                Ok(paths) => paths,
                Err(err) => {
                    return MessageStoreMigrationPreflightItem {
                        conversation_id: conversation_id.to_string(),
                        title: conversation.title,
                        status: "discarded".to_string(),
                        message_count: conversation.messages.len(),
                        reason: Some(err),
                    };
                }
            };
            match message_store::run_jsonl_snapshot_migration(&paths, &conversation, true) {
                Ok(_) => MessageStoreMigrationPreflightItem {
                    conversation_id: conversation_id.to_string(),
                    title: conversation.title,
                    status: "legacyReadyToMigrate".to_string(),
                    message_count: conversation.messages.len(),
                    reason: None,
                },
                Err(err) => MessageStoreMigrationPreflightItem {
                    conversation_id: conversation_id.to_string(),
                    title: conversation.title,
                    status: "discarded".to_string(),
                    message_count: conversation.messages.len(),
                    reason: Some(err),
                },
            }
        }
        Err(err) => MessageStoreMigrationPreflightItem {
            conversation_id: conversation_id.to_string(),
            title: String::new(),
            status: "discarded".to_string(),
            message_count: 0,
            reason: Some(err),
        },
    }
}

fn preflight_ready_message_store_conversation(
    paths: &message_store::MessageStorePaths,
    conversation_id: &str,
    fallback_message_count: usize,
) -> MessageStoreMigrationPreflightItem {
    let ready_status = match message_store::read_ready_message_store_status(paths) {
        Ok(Some(ready_status)) => ready_status,
        Ok(None) => {
            return MessageStoreMigrationPreflightItem {
                conversation_id: conversation_id.to_string(),
                title: String::new(),
                status: "discarded".to_string(),
                message_count: fallback_message_count,
                reason: Some("ready JSONL 会话状态不可读".to_string()),
            };
        }
        Err(err) => {
            return MessageStoreMigrationPreflightItem {
                conversation_id: conversation_id.to_string(),
                title: String::new(),
                status: "discarded".to_string(),
                message_count: fallback_message_count,
                reason: Some(err),
            };
        }
    };
    match message_store::read_ready_message_store_meta(paths) {
        Ok(Some(meta)) => MessageStoreMigrationPreflightItem {
            conversation_id: conversation_id.to_string(),
            title: meta.title().to_string(),
            status: "ready".to_string(),
            message_count: ready_status.source_message_count,
            reason: None,
        },
        Ok(None) => MessageStoreMigrationPreflightItem {
            conversation_id: conversation_id.to_string(),
            title: String::new(),
            status: "discarded".to_string(),
            message_count: ready_status.source_message_count,
            reason: Some("ready JSONL 会话缺少 meta".to_string()),
        },
        Err(err) => MessageStoreMigrationPreflightItem {
            conversation_id: conversation_id.to_string(),
            title: String::new(),
            status: "discarded".to_string(),
            message_count: ready_status.source_message_count,
            reason: Some(err),
        },
    }
}

fn preflight_message_store_conversation(
    data_path: &PathBuf,
    conversation_id: &str,
) -> MessageStoreMigrationPreflightItem {
    let paths = match message_store::message_store_paths(data_path, conversation_id) {
        Ok(paths) => paths,
        Err(err) => {
            return MessageStoreMigrationPreflightItem {
                conversation_id: conversation_id.to_string(),
                title: String::new(),
                status: "discarded".to_string(),
                message_count: 0,
                reason: Some(err),
            };
        }
    };
    match message_store::read_message_store_manifest_status(&paths) {
        Ok(Some(status)) if status.ready_jsonl => {
            preflight_ready_message_store_conversation(
                &paths,
                conversation_id,
                status.source_message_count,
            )
        }
        Ok(Some(status)) => {
            let legacy_path = app_layout_chat_conversation_path(data_path, conversation_id);
            if legacy_path.exists() {
                let mut item = preflight_legacy_conversation(data_path, conversation_id);
                if item.status == "legacyReadyToMigrate" {
                    item.reason = Some(format!(
                        "检测到未完成的消息仓库迁移，将重试恢复：kind={}，state={}",
                        status.message_store_kind, status.migration_state
                    ));
                }
                return item;
            }
            if let Ok(Some(_)) = message_store::recover_ready_jsonl_snapshot_manifest_from_directory(&paths) {
                return preflight_ready_message_store_conversation(
                    &paths,
                    conversation_id,
                    status.source_message_count,
                );
            }
            MessageStoreMigrationPreflightItem {
                conversation_id: conversation_id.to_string(),
                title: String::new(),
                status: "ready".to_string(),
                message_count: status.source_message_count,
                reason: None,
            }
        }
        Ok(None) => preflight_legacy_conversation(data_path, conversation_id),
        Err(err) => MessageStoreMigrationPreflightItem {
            conversation_id: conversation_id.to_string(),
            title: String::new(),
            status: "discarded".to_string(),
            message_count: 0,
            reason: Some(err),
        },
    }
}

fn empty_message_store_migration_preflight_report() -> MessageStoreMigrationPreflightReport {
    MessageStoreMigrationPreflightReport {
        migration_required: false,
        total_conversations: 0,
        ready_count: 0,
        legacy_count: 0,
        busy_count: 0,
        blocked_count: 0,
        can_auto_migrate: true,
        items: Vec::new(),
    }
}

fn message_store_migration_current_version_recorded(state: &AppState) -> Result<bool, String> {
    Ok(state_service_get_message_store_migration_version(state)?
        >= DATA_MIGRATION_CURRENT_VERSION)
}

fn build_message_store_migration_preflight_report(
    state: &AppState,
) -> MessageStoreMigrationPreflightReport {
    let items = message_store_migration_candidate_ids(&state.data_path)
        .into_iter()
        .map(|conversation_id| preflight_message_store_conversation(&state.data_path, &conversation_id))
        .collect::<Vec<_>>();
    let ready_count = items.iter().filter(|item| item.status == "ready").count();
    let legacy_count = items
        .iter()
        .filter(|item| item.status == "legacyReadyToMigrate")
        .count();
    let busy_count = items.iter().filter(|item| item.status == "busy").count();
    let blocked_count = 0usize;
    MessageStoreMigrationPreflightReport {
        migration_required: true,
        total_conversations: items.len(),
        ready_count,
        legacy_count,
        busy_count,
        blocked_count,
        can_auto_migrate: true,
        items,
    }
}

fn message_store_migration_error_is_system_failure(error: &str) -> bool {
    [
        "创建",
        "写入",
        "替换",
        "清理",
        "删除",
        "备份",
        "锁定",
        "SQLite",
        "事务",
        "读取 JSONL 快照失败",
        "读取消息存储 manifest 失败",
    ]
    .iter()
    .any(|marker| error.contains(marker))
}

fn record_discarded_message_store_migration_item(
    app: &AppHandle,
    report: &mut MessageStoreMigrationRunReport,
    current: usize,
    total: usize,
    item: &MessageStoreMigrationPreflightItem,
    reason: impl Into<String>,
) {
    let reason = reason.into();
    report.discarded_count += 1;
    runtime_log_warn(format!(
        "[消息存储迁移] 放弃，任务=V1到V2会话迁移，conversation_id={}，title={}，reason={}",
        item.conversation_id, item.title, reason
    ));
    emit_message_store_migration_progress(
        app,
        MessageStoreMigrationProgressPayload {
            current,
            total,
            conversation_id: item.conversation_id.clone(),
            title: item.title.clone(),
            status: "discarded".to_string(),
            detail: Some(reason),
        },
    );
}

#[tauri::command]
fn check_message_store_migration(
    state: State<'_, AppState>,
) -> Result<MessageStoreMigrationPreflightReport, String> {
    check_message_store_migration_inner(&state)
}

fn check_message_store_migration_inner(
    state: &AppState,
) -> Result<MessageStoreMigrationPreflightReport, String> {
    let _migration_guard = lock_message_store_migration();
    if message_store_migration_current_version_recorded(state)? {
        return Ok(empty_message_store_migration_preflight_report());
    }
    Ok(build_message_store_migration_preflight_report(state))
}

fn refresh_message_store_migration_caches(state: &AppState) -> Result<(), String> {
    // 这里只刷新普通会话消息仓库迁移相关缓存。
    // 委托快照缓存只镜像当前委托目录型正文仓库；旧格式委托若需要升级，职责在迁移服务本身，
    // 绝不能让运行期委托列表/快照路径为了观察旧格式变化而额外承担兼容或补刷逻辑。
    *state
        .cached_app_data
        .lock()
        .map_err(|_| "Failed to lock cached app data".to_string())? = None;
    *state
        .cached_app_data_signature
        .lock()
        .map_err(|_| "Failed to lock cached app data signature".to_string())? = None;
    state
        .cached_conversation_metadata
        .lock()
        .map_err(|_| "Failed to lock cached conversation metadata".to_string())?
        .clear();
    state
        .cached_conversation_mtimes
        .lock()
        .map_err(|_| "Failed to lock cached conversation mtimes".to_string())?
        .clear();
    refresh_cached_app_data_dirty(state);
    Ok(())
}

#[tauri::command]
fn run_message_store_migration(
    app: AppHandle,
    state: State<'_, AppState>,
    input: RunMessageStoreMigrationInput,
) -> Result<MessageStoreMigrationRunReport, String> {
    run_message_store_migration_inner(&app, &state, input)
}

fn run_message_store_migration_inner(
    app: &AppHandle,
    state: &AppState,
    _input: RunMessageStoreMigrationInput,
) -> Result<MessageStoreMigrationRunReport, String> {
    let _migration_guard = lock_message_store_migration();
    let mut report = MessageStoreMigrationRunReport {
        migrated_count: 0,
        skipped_ready_count: 0,
        discarded_count: 0,
        failed_count: 0,
    };
    if message_store_migration_current_version_recorded(state)? {
        return Ok(report);
    }
    if state_service_get_message_store_migration_version(state)?
        >= DATA_MIGRATION_VERSION_V2_ASSISTANT_WORKSPACE_FOR_EMPTY_SHELL_WORKSPACES
    {
        message_store::chat_metadata_store_run_v3_migration(&state.data_path)?;
        let config = state_read_config_cached(state)?;
        message_store::chat_metadata_store_run_usage_trail_migration(&state.data_path, &config)?;
        state_service_set_message_store_migration_version(
            state,
            DATA_MIGRATION_VERSION_V3_CHAT_METADATA_SQLITE,
        )?;
        return Ok(report);
    }
    let preflight = build_message_store_migration_preflight_report(state);
    let discarded = preflight
        .items
        .iter()
        .filter(|item| item.status == "discarded")
        .cloned()
        .collect::<Vec<_>>();
    for (idx, item) in discarded.iter().enumerate() {
        record_discarded_message_store_migration_item(
            &app,
            &mut report,
            idx + 1,
            preflight.items.len(),
            item,
            item.reason.clone().unwrap_or_else(|| "V1 会话不可迁移".to_string()),
        );
    }

    let runnable_items = preflight
        .items
        .into_iter()
        .filter(|item| item.status != "discarded")
        .collect::<Vec<_>>();
    let total = runnable_items.len();
    for (idx, item) in runnable_items.iter().enumerate() {
        emit_message_store_migration_progress(
            &app,
            MessageStoreMigrationProgressPayload {
                current: idx + 1,
                total,
                conversation_id: item.conversation_id.clone(),
                title: item.title.clone(),
                status: "migrating".to_string(),
                detail: None,
            },
        );
        if item.status == "ready" {
            report.skipped_ready_count += 1;
            continue;
        }
        let conversation = match read_json_file::<Conversation>(
            &app_layout_chat_conversation_path(&state.data_path, &item.conversation_id),
            "conversation file",
        ) {
            Ok(conversation) => conversation,
            Err(err) => {
                record_discarded_message_store_migration_item(
                    &app,
                    &mut report,
                    idx + 1,
                    total,
                    item,
                    err,
                );
                continue;
            }
        };
        let paths = match message_store::message_store_paths(&state.data_path, &item.conversation_id) {
            Ok(paths) => paths,
            Err(err) => {
                record_discarded_message_store_migration_item(
                    &app,
                    &mut report,
                    idx + 1,
                    total,
                    item,
                    err,
                );
                continue;
            }
        };
        match message_store::resume_jsonl_snapshot_migration(&paths, &conversation) {
            Ok(_) => {
                let recovery_job_id =
                    format!("message-store-migration-recover-{}", item.conversation_id);
                let recovery_reason = format!(
                    "消息仓库迁移恢复，conversation_id={}，title={}",
                    item.conversation_id, item.title
                );
                conversation_service_v2().recover_conversation_snapshot(
                    state,
                    &recovery_job_id,
                    "message_store_migration",
                    &recovery_reason,
                    &conversation,
                )?;
                flush_pending_persists_blocking(state)?;
                report.migrated_count += 1;
                emit_message_store_migration_progress(
                    &app,
                    MessageStoreMigrationProgressPayload {
                        current: idx + 1,
                        total,
                        conversation_id: item.conversation_id.clone(),
                        title: item.title.clone(),
                        status: "completed".to_string(),
                        detail: None,
                    },
                );
            }
            Err(err) => {
                if !message_store_migration_error_is_system_failure(&err) {
                    record_discarded_message_store_migration_item(
                        &app,
                        &mut report,
                        idx + 1,
                        total,
                        item,
                        err,
                    );
                    continue;
                }
                emit_message_store_migration_progress(
                    &app,
                    MessageStoreMigrationProgressPayload {
                        current: idx + 1,
                        total,
                        conversation_id: item.conversation_id.clone(),
                        title: item.title.clone(),
                        status: "failed".to_string(),
                        detail: Some(err.clone()),
                    },
                );
                return Err(err);
            }
        }
    }
    refresh_message_store_migration_caches(state)?;
    state_service_set_message_store_migration_version(
        state,
        DATA_MIGRATION_VERSION_V2_ASSISTANT_WORKSPACE_FOR_EMPTY_SHELL_WORKSPACES,
    )?;
    message_store::chat_metadata_store_run_v3_migration(&state.data_path)?;
    let config = state_read_config_cached(state)?;
    message_store::chat_metadata_store_run_usage_trail_migration(&state.data_path, &config)?;
    state_service_set_message_store_migration_version(
        state,
        DATA_MIGRATION_VERSION_V3_CHAT_METADATA_SQLITE,
    )?;
    Ok(report)
}

#[cfg(test)]
mod message_store_migration_gate_tests {
    use super::*;

    fn test_message(id: &str, role: &str) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            role: role.to_string(),
            created_at: "2026-06-09T00:00:00Z".to_string(),
            speaker_agent_id: None,
            parts: vec![MessagePart::Text {
                text: format!("message {id}"),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: None,
            tool_call: None,
            mcp_call: None,
        meme_annotations: None,
        }
    }

    fn test_conversation(id: &str) -> Conversation {
        Conversation {
            id: id.to_string(),
            title: "迁移测试会话".to_string(),
            agent_id: DEFAULT_AGENT_ID.to_string(),
            department_id: ASSISTANT_DEPARTMENT_ID.to_string(),
            bound_conversation_id: None,
            parent_conversation_id: None,
            child_conversation_ids: Vec::new(),
            fork_message_cursor: None,
            unread_count: 0,
            conversation_kind: String::new(),
            root_conversation_id: None,
            delegate_id: None,
            created_at: now_iso(),
            updated_at: now_iso(),
            last_user_at: None,
            last_assistant_at: None,
            status: String::new(),
            user_profile_snapshot: String::new(),
            shell_workspace_path: None,
            shell_workspaces: Vec::new(),
            shell_autonomous_mode: false,
            shell_work_mode: default_shell_work_mode(),
            archived_at: None,
            messages: vec![test_message("m1", "user"), test_message("m2", "assistant")],
            fast_request_turns: Vec::new(),
            current_todos: Vec::new(),
            memory_recall_table: Vec::new(),
            plan_mode_enabled: false,
            preferred_api_config_id: None,
            auto_push_remote_contact_id: None,
            active_goal: None,
            cumulative_usage: ConversationCumulativeUsage::default(),
        }
    }

    fn temp_data_path(label: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "eca-message-store-migration-{label}-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("config").join("app_data.json");
        (root, data_path)
    }

    #[test]
    fn message_store_preflight_should_retry_legacy_when_manifest_building() {
        let (root, data_path) = temp_data_path("legacy-building");
        let conversation = test_conversation("conversation-building-legacy");
        let legacy_path = app_layout_chat_conversation_path(&data_path, &conversation.id);
        write_json_file_atomic(&legacy_path, &conversation, "conversation file")
            .expect("write legacy conversation");
        let manifest_file = app_layout_chat_conversations_dir(&data_path)
            .join(&conversation.id)
            .join(message_store::MESSAGE_STORE_MANIFEST_FILE_NAME);
        let building = message_store::MessageStoreManifest::jsonl_snapshot_building(&conversation);
        message_store::write_message_store_manifest_atomic(&manifest_file, &building)
            .expect("write building manifest");

        let item = preflight_message_store_conversation(&data_path, &conversation.id);

        assert_eq!(item.status, "legacyReadyToMigrate");
        assert!(item
            .reason
            .as_deref()
            .unwrap_or_default()
            .contains("未完成的消息仓库迁移"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_preflight_should_recover_complete_building_directory() {
        let (root, data_path) = temp_data_path("recover-building");
        let conversation = test_conversation("conversation-building-ready");
        let paths = message_store::message_store_paths(&data_path, &conversation.id)
            .expect("message store paths");
        message_store::run_jsonl_snapshot_migration(&paths, &conversation, false)
            .expect("seed ready message store");
        let manifest_file = app_layout_chat_conversations_dir(&data_path)
            .join(&conversation.id)
            .join(message_store::MESSAGE_STORE_MANIFEST_FILE_NAME);
        let building = message_store::MessageStoreManifest::jsonl_snapshot_building(&conversation);
        message_store::write_message_store_manifest_atomic(&manifest_file, &building)
            .expect("write building manifest");

        let item = preflight_message_store_conversation(&data_path, &conversation.id);
        let status = message_store::read_message_store_manifest_status(&paths)
            .expect("read manifest status")
            .expect("manifest exists");

        assert_eq!(item.status, "ready");
        assert!(status.ready_jsonl);
        assert_eq!(status.migration_state, "ready");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_read_shard_should_recover_complete_building_directory() {
        let (root, data_path) = temp_data_path("read-recover-building");
        let conversation = test_conversation("conversation-building-read");
        let paths = message_store::message_store_paths(&data_path, &conversation.id)
            .expect("message store paths");
        message_store::run_jsonl_snapshot_migration(&paths, &conversation, false)
            .expect("seed ready message store");
        let manifest_file = app_layout_chat_conversations_dir(&data_path)
            .join(&conversation.id)
            .join(message_store::MESSAGE_STORE_MANIFEST_FILE_NAME);
        let building = message_store::MessageStoreManifest::jsonl_snapshot_building(&conversation);
        message_store::write_message_store_manifest_atomic(&manifest_file, &building)
            .expect("write building manifest");

        let loaded = read_conversation_shard(&data_path, &conversation.id)
            .expect("read recovered conversation");
        let status = message_store::read_message_store_manifest_status(&paths)
            .expect("read manifest status")
            .expect("manifest exists");

        assert_eq!(loaded.id, conversation.id);
        assert_eq!(loaded.messages.len(), conversation.messages.len());
        assert!(status.ready_jsonl);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_gate_should_ignore_general_data_migration_version() {
        let (root, data_path) = temp_data_path("gate-ignores-general-version");
        let conversation = test_conversation("conversation-legacy-gate");
        let legacy_path = app_layout_chat_conversation_path(&data_path, &conversation.id);
        write_json_file_atomic(&legacy_path, &conversation, "conversation file")
            .expect("write legacy conversation");
        let state = AppState {
            app_handle: Arc::new(Mutex::new(None)),
            config_path: root.join("app_config.toml"),
            data_path: data_path.clone(),
            llm_workspace_path: root.join("llm-workspace"),
            shared_http_client: reqwest::Client::new(),
            terminal_shell: detect_default_terminal_shell(),
            terminal_shell_candidates: detect_terminal_shell_candidates(),
            conversation_lock: Arc::new(ConversationDomainLock::new()),
            memory_lock: Arc::new(Mutex::new(())),
            cached_config: Arc::new(Mutex::new(None)),
            cached_config_mtime: Arc::new(Mutex::new(None)),
            cached_agents: Arc::new(Mutex::new(None)),
            cached_agents_mtime: Arc::new(Mutex::new(None)),
            cached_chat_index: Arc::new(Mutex::new(None)),
            cached_conversation_metadata: Arc::new(Mutex::new(std::collections::HashMap::new())),
            cached_conversation_field_metadata_ids: Arc::new(Mutex::new(
                std::collections::HashSet::new(),
            )),
            cached_conversation_mtimes: Arc::new(Mutex::new(std::collections::HashMap::new())),
            cached_app_data: Arc::new(Mutex::new(None)),
            cached_app_data_signature: Arc::new(Mutex::new(None)),
            cached_app_data_dirty: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            app_data_persist_pending: Arc::new(Mutex::new(None)),
            app_data_persist_notify: Arc::new(tokio::sync::Notify::new()),
            app_data_persist_started: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            app_data_persist_latest_seq: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            conversation_persist_pending: Arc::new(Mutex::new(None)),
            conversation_persist_notify: Arc::new(tokio::sync::Notify::new()),
            conversation_persist_started: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            conversation_persist_latest_seq: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            cached_conversation_dirty_ids: Arc::new(Mutex::new(std::collections::HashSet::new())),
            cached_deleted_conversation_ids: Arc::new(Mutex::new(std::collections::HashSet::new())),
            app_data_persist_write_lock: Arc::new(Mutex::new(())),
            last_panic_snapshot: Arc::new(Mutex::new(None)),
            inflight_chat_abort_handles: Arc::new(Mutex::new(std::collections::HashMap::new())),
            inflight_tool_abort_handles: Arc::new(Mutex::new(std::collections::HashMap::new())),
            inflight_completed_tool_history: Arc::new(Mutex::new(std::collections::HashMap::new())),
            terminal_session_roots: Arc::new(Mutex::new(std::collections::HashMap::new())),
            terminal_live_sessions: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            terminal_pending_approvals: Arc::new(Mutex::new(std::collections::HashMap::new())),
            llm_round_logs: Arc::new(Mutex::new(RecentLlmRoundLogs::default())),
            conversation_runtime_slots: Arc::new(Mutex::new(std::collections::HashMap::new())),
            conversation_processing_claims: Arc::new(Mutex::new(std::collections::HashSet::new())),
            goal_continue_suppressed_conversation_ids: Arc::new(Mutex::new(std::collections::HashSet::new())),
            pending_chat_result_senders: Arc::new(Mutex::new(std::collections::HashMap::new())),
            pending_chat_delta_channels: Arc::new(Mutex::new(std::collections::HashMap::new())),
            accepted_submit_trace_ids: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            active_chat_view_bindings: Arc::new(Mutex::new(std::collections::HashMap::new())),
            conversation_list_activity_marks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            dequeue_lock: Arc::new(Mutex::new(())),
            task_scheduler_notify: Arc::new(tokio::sync::Notify::new()),
            delegate_runtime_threads: Arc::new(Mutex::new(std::collections::HashMap::new())),
            delegate_recent_threads: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            provider_streaming_disabled_keys: Arc::new(Mutex::new(std::collections::HashMap::new())),
            provider_system_message_user_fallback_keys: Arc::new(Mutex::new(std::collections::HashSet::new())),
            provider_request_gates: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            remote_im_contact_runtime_states: Arc::new(Mutex::new(std::collections::HashMap::new())),
            remote_im_reply_delegate_runtimes: Arc::new(Mutex::new(std::collections::HashMap::new())),
            remote_im_reply_delegate_semaphore: Arc::new(tokio::sync::Semaphore::new(8)),
            remote_im_channel_state_write_locks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            hidden_skill_snapshot_cache: Arc::new(Mutex::new(String::new())),
            preferred_release_source: Arc::new(Mutex::new(String::new())),
            migration_preview_dirs: Arc::new(Mutex::new(std::collections::HashMap::new())),
            delegate_active_ids: Arc::new(Mutex::new(std::collections::HashSet::new())),
            backend_ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        state_service_set_data_migration_version(&state, DATA_MIGRATION_CURRENT_VERSION)
            .expect("write data migration version");
        state_service_set_message_store_migration_version(&state, 0)
            .expect("write message store migration version");
        let report = build_message_store_migration_preflight_report(&state);

        assert_eq!(
            state_service_get_data_migration_version(&state).expect("read data migration version"),
            DATA_MIGRATION_CURRENT_VERSION
        );
        assert_eq!(
            state_service_get_message_store_migration_version(&state)
                .expect("read message store migration version"),
            0
        );
        assert!(!message_store_migration_current_version_recorded(&state).expect("read gate"));
        assert_eq!(report.legacy_count, 1);
        assert_eq!(report.items[0].status, "legacyReadyToMigrate");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_preflight_should_discard_broken_v1_conversation() {
        let (root, data_path) = temp_data_path("discard-broken-v1");
        let conversation_id = "conversation-broken-v1";
        let legacy_path = app_layout_chat_conversation_path(&data_path, conversation_id);
        if let Some(parent) = legacy_path.parent() {
            fs::create_dir_all(parent).expect("create legacy dir");
        }
        fs::write(&legacy_path, "{broken").expect("write broken legacy conversation");

        let item = preflight_message_store_conversation(&data_path, conversation_id);

        assert_eq!(item.status, "discarded");
        assert!(item.reason.as_deref().unwrap_or_default().contains("Parse conversation file failed"));
        let _ = fs::remove_dir_all(root);
    }
}

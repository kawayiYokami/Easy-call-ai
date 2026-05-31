const MESSAGE_STORE_MIGRATION_PROGRESS_EVENT: &str = "easy-call:message-store-migration-progress";
const CURRENT_MESSAGE_STORE_MIGRATION_VERSION: u32 = 1;
const ACTIVE_BUILDING_MANIFEST_GRACE_SECONDS: i64 = 300;

fn message_store_migration_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

fn lock_message_store_migration() -> std::sync::MutexGuard<'static, ()> {
    message_store_migration_lock().lock().unwrap_or_else(|poison| {
        eprintln!(
            "[消息存储迁移] 迁移锁已污染，继续串行执行恢复，error={:?}",
            poison
        );
        poison.into_inner()
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MessageStoreMigrationPreflightReport {
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
struct RunMessageStoreMigrationInput {
    #[serde(default)]
    discard_invalid: bool,
}

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
        eprintln!(
            "[消息存储迁移] 进度事件发送失败：event={}，error={:?}",
            MESSAGE_STORE_MIGRATION_PROGRESS_EVENT, err
        );
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
                    status: "blocked".to_string(),
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
                        status: "blocked".to_string(),
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
                    status: "blocked".to_string(),
                    message_count: conversation.messages.len(),
                    reason: Some(err),
                },
            }
        }
        Err(err) => MessageStoreMigrationPreflightItem {
            conversation_id: conversation_id.to_string(),
            title: String::new(),
            status: "blocked".to_string(),
            message_count: 0,
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
                status: "blocked".to_string(),
                message_count: 0,
                reason: Some(err),
            };
        }
    };
    match message_store::read_message_store_manifest_status(&paths) {
        Ok(Some(status)) if status.ready_jsonl => {
            let ready_status = match message_store::read_ready_message_store_status(&paths) {
                Ok(Some(ready_status)) => ready_status,
                Ok(None) => {
                    return MessageStoreMigrationPreflightItem {
                        conversation_id: conversation_id.to_string(),
                        title: String::new(),
                        status: "blocked".to_string(),
                        message_count: status.source_message_count,
                        reason: Some("ready JSONL 会话状态不可读".to_string()),
                    };
                }
                Err(err) => {
                    return MessageStoreMigrationPreflightItem {
                        conversation_id: conversation_id.to_string(),
                        title: String::new(),
                        status: "blocked".to_string(),
                        message_count: status.source_message_count,
                        reason: Some(err),
                    };
                }
            };
            match message_store::read_ready_message_store_meta(&paths) {
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
                    status: "blocked".to_string(),
                    message_count: ready_status.source_message_count,
                    reason: Some("ready JSONL 会话缺少 meta".to_string()),
                },
                Err(err) => MessageStoreMigrationPreflightItem {
                    conversation_id: conversation_id.to_string(),
                    title: String::new(),
                    status: "blocked".to_string(),
                    message_count: ready_status.source_message_count,
                    reason: Some(err),
                },
            }
        }
        Ok(Some(status)) => {
            if status.migration_state == "building" && message_store_manifest_recently_updated(&status.updated_at) {
                return MessageStoreMigrationPreflightItem {
                    conversation_id: conversation_id.to_string(),
                    title: String::new(),
                    status: "busy".to_string(),
                    message_count: status.source_message_count,
                    reason: Some(format!(
                        "消息仓库正在写入，暂不参与迁移预检：kind={}，state={}",
                        status.message_store_kind, status.migration_state
                    )),
                };
            }
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
            MessageStoreMigrationPreflightItem {
                conversation_id: conversation_id.to_string(),
                title: String::new(),
                status: "blocked".to_string(),
                message_count: status.source_message_count,
                reason: Some(format!(
                    "消息仓库 manifest 未处于 ready JSONL 状态：kind={}，state={}",
                    status.message_store_kind, status.migration_state
                )),
            }
        }
        Ok(None) => preflight_legacy_conversation(data_path, conversation_id),
        Err(err) => MessageStoreMigrationPreflightItem {
            conversation_id: conversation_id.to_string(),
            title: String::new(),
            status: "blocked".to_string(),
            message_count: 0,
            reason: Some(err),
        },
    }
}

fn empty_message_store_migration_preflight_report() -> MessageStoreMigrationPreflightReport {
    MessageStoreMigrationPreflightReport {
        total_conversations: 0,
        ready_count: 0,
        legacy_count: 0,
        busy_count: 0,
        blocked_count: 0,
        can_auto_migrate: true,
        items: Vec::new(),
    }
}

fn message_store_manifest_recently_updated(updated_at: &str) -> bool {
    let Some(updated_at) = parse_iso(updated_at) else {
        return false;
    };
    let elapsed = now_utc() - updated_at;
    elapsed.whole_seconds() >= 0
        && elapsed.whole_seconds() <= ACTIVE_BUILDING_MANIFEST_GRACE_SECONDS
}

fn message_store_migration_version_recorded(state: &AppState) -> Result<bool, String> {
    Ok(state_read_runtime_state_cached(state)?.message_store_migration_version
        >= CURRENT_MESSAGE_STORE_MIGRATION_VERSION)
}

fn record_message_store_migration_version(state: &AppState) -> Result<(), String> {
    let mut runtime = state_read_runtime_state_cached(state)?;
    if runtime.message_store_migration_version >= CURRENT_MESSAGE_STORE_MIGRATION_VERSION {
        return Ok(());
    }
    runtime.message_store_migration_version = CURRENT_MESSAGE_STORE_MIGRATION_VERSION;
    state_write_runtime_state_cached(state, &runtime)?;
    eprintln!(
        "[消息存储迁移] 完成 task=record_message_store_migration_version version={}",
        CURRENT_MESSAGE_STORE_MIGRATION_VERSION
    );
    Ok(())
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
    let blocked_count = items.iter().filter(|item| item.status == "blocked").count();
    MessageStoreMigrationPreflightReport {
        total_conversations: items.len(),
        ready_count,
        legacy_count,
        busy_count,
        blocked_count,
        can_auto_migrate: blocked_count == 0,
        items,
    }
}

#[tauri::command]
fn check_message_store_migration(
    state: State<'_, AppState>,
) -> Result<MessageStoreMigrationPreflightReport, String> {
    let _migration_guard = lock_message_store_migration();
    if message_store_migration_version_recorded(&state)? {
        return Ok(empty_message_store_migration_preflight_report());
    }
    let report = build_message_store_migration_preflight_report(&state);
    if report.blocked_count == 0 && report.legacy_count == 0 {
        record_message_store_migration_version(&state)?;
    }
    Ok(report)
}

fn message_store_discard_backup_root(state: &AppState) -> Result<PathBuf, String> {
    let root = app_layout_backups_dir(&state.data_path)
        .join("discarded-message-store-migration")
        .join(format!("{}", now_utc().unix_timestamp()));
    fs::create_dir_all(&root).map_err(|err| {
        format!(
            "创建消息仓库迁移异常备份目录失败，path={}，error={err}",
            root.display()
        )
    })?;
    Ok(root)
}

fn move_path_to_backup(path: &PathBuf, backup_path: &PathBuf, label: &str) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }
    if let Some(parent) = backup_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "创建{label}备份父目录失败，path={}，error={err}",
                parent.display()
            )
        })?;
    }
    fs::rename(path, backup_path).map_err(|err| {
        format!(
            "移动{label}到备份失败，source={}，target={}，error={err}",
            path.display(),
            backup_path.display()
        )
    })?;
    Ok(true)
}

fn discard_message_store_migration_item(
    state: &AppState,
    item: &MessageStoreMigrationPreflightItem,
    backup_root: &PathBuf,
) -> Result<bool, String> {
    let conversation_id = item.conversation_id.trim();
    if conversation_id.is_empty() {
        return Ok(false);
    }
    let _ = message_store::message_store_paths(&state.data_path, conversation_id)?;
    let conversation_file = app_layout_chat_conversation_path(&state.data_path, conversation_id);
    let directory_shard = app_layout_chat_conversations_dir(&state.data_path).join(conversation_id);
    let item_backup_root = backup_root.join(conversation_id);
    let mut changed = false;
    changed |= move_path_to_backup(
        &conversation_file,
        &item_backup_root.join(format!("{conversation_id}.json")),
        "异常旧会话文件",
    )?;
    changed |= move_path_to_backup(
        &directory_shard,
        &item_backup_root.join(conversation_id),
        "异常目录型会话",
    )?;
    eprintln!(
        "[消息存储迁移] 抛弃异常会话：conversation_id={}，backup={}，reason={}",
        conversation_id,
        item_backup_root.display(),
        item.reason.as_deref().unwrap_or("未提供原因")
    );
    Ok(changed)
}

fn refresh_message_store_migration_caches(state: &AppState) -> Result<(), String> {
    *state
        .cached_app_data
        .lock()
        .map_err(|_| "Failed to lock cached app data".to_string())? = None;
    *state
        .cached_app_data_signature
        .lock()
        .map_err(|_| "Failed to lock cached app data signature".to_string())? = None;
    state
        .cached_conversations
        .lock()
        .map_err(|_| "Failed to lock cached conversations".to_string())?
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
    let _migration_guard = lock_message_store_migration();
    let mut report = MessageStoreMigrationRunReport {
        migrated_count: 0,
        skipped_ready_count: 0,
        discarded_count: 0,
        failed_count: 0,
    };
    if message_store_migration_version_recorded(&state)? {
        return Ok(report);
    }
    let preflight = build_message_store_migration_preflight_report(&state);
    let blocked = preflight
        .items
        .iter()
        .filter(|item| item.status == "blocked")
        .cloned()
        .collect::<Vec<_>>();
    if !blocked.is_empty() && !input.discard_invalid {
        return Err(format!(
            "消息仓库迁移预验证失败：blocked_count={}，请确认是否抛弃异常会话后继续",
            blocked.len()
        ));
    }
    if !blocked.is_empty() {
        let backup_root = message_store_discard_backup_root(&state)?;
        for item in &blocked {
            if discard_message_store_migration_item(&state, item, &backup_root)? {
                report.discarded_count += 1;
            }
        }
    }

    let runnable_items = preflight
        .items
        .into_iter()
        .filter(|item| item.status != "blocked")
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
        let conversation = read_json_file::<Conversation>(
            &app_layout_chat_conversation_path(&state.data_path, &item.conversation_id),
            "conversation file",
        )?;
        let paths = message_store::message_store_paths(&state.data_path, &item.conversation_id)?;
        match message_store::run_jsonl_snapshot_migration(&paths, &conversation, false) {
            Ok(_) => {
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
    refresh_message_store_migration_caches(&state)?;
    record_message_store_migration_version(&state)?;
    Ok(report)
}

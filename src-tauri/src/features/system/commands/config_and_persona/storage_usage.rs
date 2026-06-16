const STORAGE_CLEANUP_LEGACY_CONVERSATIONS: &str = "legacyConversations";
const STORAGE_CLEANUP_LEGACY_DELEGATE_CONVERSATIONS: &str = "legacyDelegateConversations";

#[derive(Debug, Clone, Default)]
struct StorageSizeStats {
    bytes: u64,
    file_count: usize,
    directory_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageUsageOverview {
    root_path: String,
    total_bytes: u64,
    reclaimable_bytes: u64,
    items: Vec<StorageUsageItem>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct UsageOverviewTotals {
    conversation_count: usize,
    archived_conversation_count: usize,
    active_conversation_count: usize,
    delegate_conversation_count: usize,
    with_usage_conversation_count: usize,
    weighted_tokens: u64,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct UsageAggregateItem {
    key: String,
    label: String,
    conversation_count: usize,
    weighted_tokens: u64,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UsageConversationItem {
    conversation_id: String,
    title: String,
    updated_at: String,
    archived_at: Option<String>,
    agent_id: String,
    agent_name: String,
    department_id: String,
    department_name: String,
    api_config_id: String,
    api_config_name: String,
    model_name: String,
    conversation_kind: String,
    is_delegate: bool,
    is_system_notification_conversation: bool,
    message_count: usize,
    weighted_tokens: u64,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UsageOverview {
    generated_at: String,
    totals: UsageOverviewTotals,
    conversations: Vec<UsageConversationItem>,
    by_model: Vec<UsageAggregateItem>,
    by_api_config: Vec<UsageAggregateItem>,
    by_agent: Vec<UsageAggregateItem>,
    by_department: Vec<UsageAggregateItem>,
    by_kind: Vec<UsageAggregateItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageUsageItem {
    id: String,
    target_path: String,
    bytes: u64,
    file_count: usize,
    directory_count: usize,
    cleanable_bytes: u64,
    cleanable_file_count: usize,
    cleanup_kind: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CleanupStorageLegacyItemsInput {
    cleanup_kind: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenStorageUsageItemDirectoryInput {
    item_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StorageCleanupResult {
    deleted_file_count: usize,
    skipped_file_count: usize,
    freed_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StorageLegacyConversationScope {
    Normal,
    Delegate,
}

#[derive(Debug, Clone, Default)]
struct StorageLegacyCleanupScan {
    total_file_count: usize,
    cleanable_paths: Vec<PathBuf>,
    cleanable_bytes: u64,
}

fn storage_usage_item(
    id: &str,
    target_path: PathBuf,
    stats: StorageSizeStats,
    cleanup_kind: Option<&str>,
    cleanable_bytes: u64,
    cleanable_file_count: usize,
) -> StorageUsageItem {
    StorageUsageItem {
        id: id.to_string(),
        target_path: target_path.to_string_lossy().to_string(),
        bytes: stats.bytes,
        file_count: stats.file_count,
        directory_count: stats.directory_count,
        cleanable_bytes,
        cleanable_file_count,
        cleanup_kind: cleanup_kind.map(str::to_string),
    }
}

fn storage_add_path_stats(path: &PathBuf, stats: &mut StorageSizeStats) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let metadata = fs::metadata(path).map_err(|err| {
        format!("读取存储路径元数据失败，path={}，error={err}", path.display())
    })?;
    if metadata.is_file() {
        stats.bytes = stats.bytes.saturating_add(metadata.len());
        stats.file_count += 1;
        return Ok(());
    }
    if !metadata.is_dir() {
        return Ok(());
    }
    stats.directory_count += 1;
    let entries = fs::read_dir(path).map_err(|err| {
        format!("读取存储目录失败，path={}，error={err}", path.display())
    })?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            format!("读取存储目录项失败，path={}，error={err}", path.display())
        })?;
        storage_add_path_stats(&entry.path(), stats)?;
    }
    Ok(())
}

fn storage_stats_for_paths(paths: Vec<PathBuf>) -> Result<StorageSizeStats, String> {
    let mut stats = StorageSizeStats::default();
    for path in paths {
        storage_add_path_stats(&path, &mut stats)?;
    }
    Ok(stats)
}

fn storage_stats_for_directory_entries(
    dir: &PathBuf,
    filter: impl Fn(&PathBuf, &fs::FileType) -> bool,
) -> Result<StorageSizeStats, String> {
    let mut stats = StorageSizeStats::default();
    if !dir.exists() {
        return Ok(stats);
    }
    let entries = fs::read_dir(dir).map_err(|err| {
        format!("读取存储目录失败，path={}，error={err}", dir.display())
    })?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            format!("读取存储目录项失败，path={}，error={err}", dir.display())
        })?;
        let file_type = entry.file_type().map_err(|err| {
            format!(
                "读取存储目录项类型失败，path={}，error={err}",
                entry.path().display()
            )
        })?;
        let path = entry.path();
        if filter(&path, &file_type) {
            storage_add_path_stats(&path, &mut stats)?;
        }
    }
    Ok(stats)
}

fn storage_path_file_name_is(path: &PathBuf, name: &str) -> bool {
    path.file_name().and_then(|value| value.to_str()) == Some(name)
}

fn storage_path_extension_is(path: &PathBuf, extension: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
}

fn storage_conversation_dir(data_path: &PathBuf, scope: StorageLegacyConversationScope) -> PathBuf {
    match scope {
        StorageLegacyConversationScope::Normal => app_layout_chat_conversations_dir(data_path),
        StorageLegacyConversationScope::Delegate => delegate_conversation_store_dir(data_path),
    }
}

fn storage_legacy_file_ready_to_cleanup(
    data_path: &PathBuf,
    conversation_id: &str,
    scope: StorageLegacyConversationScope,
) -> Result<bool, String> {
    match scope {
        StorageLegacyConversationScope::Normal => {
            let paths = message_store::message_store_paths(data_path, conversation_id)?;
            message_store::read_ready_message_store_status(&paths).map(|status| status.is_some())
        }
        StorageLegacyConversationScope::Delegate => {
            let paths = delegate_conversation_message_store_paths(data_path, conversation_id)?;
            delegate_conversation_store_read_ready_meta(&paths, conversation_id)
                .map(|meta| meta.is_some())
        }
    }
}

fn storage_scan_legacy_cleanup_candidates(
    data_path: &PathBuf,
    scope: StorageLegacyConversationScope,
) -> Result<StorageLegacyCleanupScan, String> {
    let dir = storage_conversation_dir(data_path, scope);
    let mut scan = StorageLegacyCleanupScan::default();
    if !dir.exists() {
        return Ok(scan);
    }
    let entries = fs::read_dir(&dir).map_err(|err| {
        format!("读取旧会话目录失败，path={}，error={err}", dir.display())
    })?;
    for entry in entries {
        let entry = entry.map_err(|err| {
            format!("读取旧会话目录项失败，path={}，error={err}", dir.display())
        })?;
        let path = entry.path();
        if !path.is_file() || !storage_path_extension_is(&path, "json") {
            continue;
        }
        scan.total_file_count += 1;
        let conversation_id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .trim()
            .to_string();
        if conversation_id.is_empty() {
            continue;
        }
        match storage_legacy_file_ready_to_cleanup(data_path, &conversation_id, scope) {
            Ok(true) => {
                let bytes = fs::metadata(&path).map(|metadata| metadata.len()).unwrap_or(0);
                scan.cleanable_bytes = scan.cleanable_bytes.saturating_add(bytes);
                scan.cleanable_paths.push(path);
            }
            Ok(false) => {}
            Err(err) => {
                eprintln!(
                    "[存储] 跳过，任务=检查旧会话清理候选，conversation_id={}，reason={}",
                    conversation_id,
                    err
                );
            }
        }
    }
    Ok(scan)
}

fn storage_legacy_conversation_stats(
    data_path: &PathBuf,
    scope: StorageLegacyConversationScope,
) -> Result<(StorageSizeStats, StorageLegacyCleanupScan), String> {
    let dir = storage_conversation_dir(data_path, scope);
    let stats = storage_stats_for_directory_entries(&dir, |path, file_type| {
        file_type.is_file() && storage_path_extension_is(path, "json")
    })?;
    let scan = storage_scan_legacy_cleanup_candidates(data_path, scope)?;
    Ok((stats, scan))
}

fn storage_current_conversation_store_stats(
    data_path: &PathBuf,
    scope: StorageLegacyConversationScope,
) -> Result<StorageSizeStats, String> {
    let dir = storage_conversation_dir(data_path, scope);
    storage_stats_for_directory_entries(&dir, |_path, file_type| file_type.is_dir())
}

fn storage_conversation_other_stats(
    data_path: &PathBuf,
    scope: StorageLegacyConversationScope,
) -> Result<StorageSizeStats, String> {
    let dir = storage_conversation_dir(data_path, scope);
    storage_stats_for_directory_entries(&dir, |path, file_type| {
        !file_type.is_dir() && !(file_type.is_file() && storage_path_extension_is(path, "json"))
    })
}

fn storage_usage_target_path(state: &AppState, item_id: &str) -> Option<PathBuf> {
    let app_root = app_root_from_data_path(&state.data_path);
    let path = match item_id {
        "configuration" => state
            .config_path
            .parent()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| app_root.clone()),
        "runtimeState" => app_layout_state_dir(&state.data_path),
        "chatMetadata" => app_layout_chat_dir(&state.data_path),
        "conversationStores" | "legacyConversations" | "conversationOther" => {
            app_layout_chat_conversations_dir(&state.data_path)
        }
        "delegateRecords" => app_root.join("delegate"),
        "delegateConversationStores"
        | "legacyDelegateConversations"
        | "delegateConversationOther" => delegate_conversation_store_dir(&state.data_path),
        "legacyAppData" => app_root.clone(),
        "memory" => app_root.join("memory"),
        "task" => app_root.join("task"),
        "media" => app_root.join("media"),
        "avatars" => app_root.join("avatars"),
        "exports" => app_root.join("exports"),
        "backups" => app_layout_backups_dir(&state.data_path),
        "workspace" => state.llm_workspace_path.clone(),
        "toolReview" => app_root.join("tool-review-reports"),
        "temp" => app_root.join("temp"),
        "other" => app_root,
        _ => return None,
    };
    Some(path)
}

fn storage_chat_metadata_stats(data_path: &PathBuf) -> Result<StorageSizeStats, String> {
    let chat_dir = app_layout_chat_dir(data_path);
    storage_stats_for_directory_entries(&chat_dir, |path, _file_type| {
        !storage_path_file_name_is(path, LAYOUT_DIR_CHAT_CONVERSATIONS)
    })
}

fn storage_root_other_stats(state: &AppState) -> Result<StorageSizeStats, String> {
    let app_root = app_root_from_data_path(&state.data_path);
    let config_file_name = state
        .config_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string();
    let data_file_name = state
        .data_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_string();
    storage_stats_for_directory_entries(&app_root, |path, _file_type| {
        let name = path.file_name().and_then(|value| value.to_str()).unwrap_or_default();
        !matches!(
            name,
            LAYOUT_DIR_CONFIG
                | LAYOUT_DIR_STATE
                | LAYOUT_DIR_CHAT
                | LAYOUT_DIR_BACKUPS
                | DELEGATE_CONVERSATIONS_DIR_NAME
                | "delegate"
                | "memory"
                | "task"
                | "media"
                | "avatars"
                | "exports"
                | "llm-workspace"
                | "tool-review-reports"
                | "temp"
                | LEGACY_APP_DATA_SPLIT_DIR_NAME
        ) && name != config_file_name
            && name != data_file_name
    })
}

fn build_storage_usage_overview(state: &AppState) -> Result<StorageUsageOverview, String> {
    let app_root = app_root_from_data_path(&state.data_path);
    let (legacy_conversation_stats, legacy_conversation_scan) =
        storage_legacy_conversation_stats(&state.data_path, StorageLegacyConversationScope::Normal)?;
    let (legacy_delegate_stats, legacy_delegate_scan) =
        storage_legacy_conversation_stats(&state.data_path, StorageLegacyConversationScope::Delegate)?;

    let mut items = vec![
        storage_usage_item(
            "configuration",
            storage_usage_target_path(state, "configuration").unwrap_or_else(|| app_root.clone()),
            storage_stats_for_paths(vec![state.config_path.clone(), app_layout_config_dir(&state.data_path)])?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "runtimeState",
            storage_usage_target_path(state, "runtimeState").unwrap_or_else(|| app_root.clone()),
            storage_stats_for_paths(vec![app_layout_state_dir(&state.data_path)])?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "chatMetadata",
            storage_usage_target_path(state, "chatMetadata").unwrap_or_else(|| app_root.clone()),
            storage_chat_metadata_stats(&state.data_path)?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "conversationStores",
            storage_usage_target_path(state, "conversationStores").unwrap_or_else(|| app_root.clone()),
            storage_current_conversation_store_stats(
                &state.data_path,
                StorageLegacyConversationScope::Normal,
            )?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "legacyConversations",
            storage_usage_target_path(state, "legacyConversations").unwrap_or_else(|| app_root.clone()),
            legacy_conversation_stats,
            Some(STORAGE_CLEANUP_LEGACY_CONVERSATIONS),
            legacy_conversation_scan.cleanable_bytes,
            legacy_conversation_scan.cleanable_paths.len(),
        ),
        storage_usage_item(
            "conversationOther",
            storage_usage_target_path(state, "conversationOther").unwrap_or_else(|| app_root.clone()),
            storage_conversation_other_stats(
                &state.data_path,
                StorageLegacyConversationScope::Normal,
            )?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "delegateRecords",
            storage_usage_target_path(state, "delegateRecords").unwrap_or_else(|| app_root.clone()),
            storage_stats_for_paths(vec![app_root.join("delegate")])?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "delegateConversationStores",
            storage_usage_target_path(state, "delegateConversationStores").unwrap_or_else(|| app_root.clone()),
            storage_current_conversation_store_stats(
                &state.data_path,
                StorageLegacyConversationScope::Delegate,
            )?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "legacyDelegateConversations",
            storage_usage_target_path(state, "legacyDelegateConversations").unwrap_or_else(|| app_root.clone()),
            legacy_delegate_stats,
            Some(STORAGE_CLEANUP_LEGACY_DELEGATE_CONVERSATIONS),
            legacy_delegate_scan.cleanable_bytes,
            legacy_delegate_scan.cleanable_paths.len(),
        ),
        storage_usage_item(
            "delegateConversationOther",
            storage_usage_target_path(state, "delegateConversationOther").unwrap_or_else(|| app_root.clone()),
            storage_conversation_other_stats(
                &state.data_path,
                StorageLegacyConversationScope::Delegate,
            )?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "legacyAppData",
            storage_usage_target_path(state, "legacyAppData").unwrap_or_else(|| app_root.clone()),
            storage_stats_for_paths(vec![
                state.data_path.clone(),
                legacy_app_data_split_dir(&state.data_path),
            ])?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "memory",
            storage_usage_target_path(state, "memory").unwrap_or_else(|| app_root.clone()),
            storage_stats_for_paths(vec![app_root.join("memory")])?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "task",
            storage_usage_target_path(state, "task").unwrap_or_else(|| app_root.clone()),
            storage_stats_for_paths(vec![app_root.join("task")])?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "media",
            storage_usage_target_path(state, "media").unwrap_or_else(|| app_root.clone()),
            storage_stats_for_paths(vec![app_root.join("media")])?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "avatars",
            storage_usage_target_path(state, "avatars").unwrap_or_else(|| app_root.clone()),
            storage_stats_for_paths(vec![app_root.join("avatars")])?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "exports",
            storage_usage_target_path(state, "exports").unwrap_or_else(|| app_root.clone()),
            storage_stats_for_paths(vec![app_root.join("exports")])?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "backups",
            storage_usage_target_path(state, "backups").unwrap_or_else(|| app_root.clone()),
            storage_stats_for_paths(vec![app_layout_backups_dir(&state.data_path)])?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "workspace",
            storage_usage_target_path(state, "workspace").unwrap_or_else(|| app_root.clone()),
            storage_stats_for_paths(vec![state.llm_workspace_path.clone()])?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "toolReview",
            storage_usage_target_path(state, "toolReview").unwrap_or_else(|| app_root.clone()),
            storage_stats_for_paths(vec![app_root.join("tool-review-reports")])?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "temp",
            storage_usage_target_path(state, "temp").unwrap_or_else(|| app_root.clone()),
            storage_stats_for_paths(vec![app_root.join("temp")])?,
            None,
            0,
            0,
        ),
        storage_usage_item(
            "other",
            storage_usage_target_path(state, "other").unwrap_or_else(|| app_root.clone()),
            storage_root_other_stats(state)?,
            None,
            0,
            0,
        ),
    ];
    items.sort_by(|left, right| right.bytes.cmp(&left.bytes).then_with(|| left.id.cmp(&right.id)));
    let total_bytes = items
        .iter()
        .fold(0_u64, |total, item| total.saturating_add(item.bytes));
    let reclaimable_bytes = items
        .iter()
        .fold(0_u64, |total, item| total.saturating_add(item.cleanable_bytes));
    Ok(StorageUsageOverview {
        root_path: app_root.to_string_lossy().to_string(),
        total_bytes,
        reclaimable_bytes,
        items,
    })
}

#[tauri::command]
fn get_storage_usage_overview(
    state: State<'_, AppState>,
) -> Result<StorageUsageOverview, String> {
    build_storage_usage_overview(&state)
}

fn usage_resolve_api_config_id(conversation: &Conversation, config: &AppConfig) -> String {
    let preferred = conversation
        .preferred_api_config_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    if let Some(value) = preferred {
        return value;
    }
    let department_id = conversation.department_id.trim();
    if department_id.is_empty() {
        return String::new();
    }
    config
        .departments
        .iter()
        .find(|item| item.id.trim() == department_id)
        .map(|item| {
            let primary = item.api_config_id.trim();
            if !primary.is_empty() {
                return primary.to_string();
            }
            item.api_config_ids
                .iter()
                .find_map(|value| {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                })
                .unwrap_or_default()
        })
        .unwrap_or_default()
}

fn usage_kind_key_and_label(conversation: &Conversation) -> (String, String) {
    if conversation_is_system_notification(conversation) {
        return ("system_notification".to_string(), "系统通知".to_string());
    }
    if conversation
        .delegate_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
    {
        return ("delegate".to_string(), "委托".to_string());
    }
    let kind = conversation.conversation_kind.trim();
    if kind == "remote_im_contact" {
        return ("remote_im_contact".to_string(), "远程联系人".to_string());
    }
    if conversation.archived_at.is_some() {
        return ("archived".to_string(), "已归档".to_string());
    }
    ("normal".to_string(), "普通".to_string())
}

fn usage_aggregate_push(
    map: &mut std::collections::HashMap<String, UsageAggregateItem>,
    key: String,
    label: String,
    item: &UsageConversationItem,
) {
    let entry = map.entry(key.clone()).or_insert_with(|| UsageAggregateItem {
        key,
        label,
        ..UsageAggregateItem::default()
    });
    entry.conversation_count = entry.conversation_count.saturating_add(1);
    entry.weighted_tokens = entry.weighted_tokens.saturating_add(item.weighted_tokens);
    entry.input_tokens = entry.input_tokens.saturating_add(item.input_tokens);
    entry.output_tokens = entry.output_tokens.saturating_add(item.output_tokens);
    entry.cache_read_tokens = entry.cache_read_tokens.saturating_add(item.cache_read_tokens);
    entry.cache_write_tokens = entry.cache_write_tokens.saturating_add(item.cache_write_tokens);
}

fn usage_sort_aggregate_items(items: std::collections::HashMap<String, UsageAggregateItem>) -> Vec<UsageAggregateItem> {
    let mut out = items.into_values().collect::<Vec<_>>();
    out.sort_by(|left, right| {
        right
            .weighted_tokens
            .cmp(&left.weighted_tokens)
            .then_with(|| right.output_tokens.cmp(&left.output_tokens))
            .then_with(|| right.conversation_count.cmp(&left.conversation_count))
            .then_with(|| left.label.cmp(&right.label))
    });
    out
}

fn usage_cumulative_from_conversation(conversation: &Conversation) -> (ConversationCumulativeUsage, u64) {
    if conversation_is_delegate(conversation) && conversation.cumulative_usage.is_empty() {
        let stats = conversation_delegate_stats_from_conversation(conversation, &[]);
        let weighted = conversation_cumulative_usage_weighted_tokens(&stats.cumulative_usage);
        return (stats.cumulative_usage, weighted);
    }
    let cumulative = conversation.cumulative_usage.clone();
    let weighted = conversation_cumulative_usage_weighted_tokens(&cumulative);
    (cumulative, weighted)
}

fn usage_push_conversation(
    conversation: &Conversation,
    config: &AppConfig,
    api_config_name_map: &std::collections::HashMap<String, String>,
    api_config_model_map: &std::collections::HashMap<String, String>,
    agent_name_map: &std::collections::HashMap<String, String>,
    department_name_map: &std::collections::HashMap<String, String>,
    totals: &mut UsageOverviewTotals,
    conversations: &mut Vec<UsageConversationItem>,
    by_model: &mut std::collections::HashMap<String, UsageAggregateItem>,
    by_api_config: &mut std::collections::HashMap<String, UsageAggregateItem>,
    by_agent: &mut std::collections::HashMap<String, UsageAggregateItem>,
    by_department: &mut std::collections::HashMap<String, UsageAggregateItem>,
    by_kind: &mut std::collections::HashMap<String, UsageAggregateItem>,
) {
    totals.conversation_count = totals.conversation_count.saturating_add(1);
    if conversation.archived_at.is_some() {
        totals.archived_conversation_count = totals.archived_conversation_count.saturating_add(1);
    } else {
        totals.active_conversation_count = totals.active_conversation_count.saturating_add(1);
    }
    let is_delegate = conversation_is_delegate(conversation);
    if is_delegate {
        totals.delegate_conversation_count = totals.delegate_conversation_count.saturating_add(1);
    }

    let (cumulative, weighted) = usage_cumulative_from_conversation(conversation);
    if !cumulative.is_empty() || weighted > 0 {
        totals.with_usage_conversation_count = totals.with_usage_conversation_count.saturating_add(1);
    }
    totals.weighted_tokens = totals.weighted_tokens.saturating_add(weighted);
    totals.input_tokens = totals.input_tokens.saturating_add(cumulative.input_tokens);
    totals.output_tokens = totals.output_tokens.saturating_add(cumulative.output_tokens);
    totals.cache_read_tokens = totals.cache_read_tokens.saturating_add(cumulative.cache_read_tokens);
    totals.cache_write_tokens = totals.cache_write_tokens.saturating_add(cumulative.cache_write_tokens);

    let api_config_id = usage_resolve_api_config_id(conversation, config);
    let api_config_name = api_config_name_map
        .get(&api_config_id)
        .cloned()
        .unwrap_or_else(|| if api_config_id.is_empty() { "未绑定配置".to_string() } else { api_config_id.clone() });
    let model_name = api_config_model_map
        .get(&api_config_id)
        .cloned()
        .unwrap_or_else(|| "未绑定模型".to_string());
    let agent_name = agent_name_map
        .get(&conversation.agent_id)
        .cloned()
        .unwrap_or_else(|| if conversation.agent_id.trim().is_empty() { "未绑定人格".to_string() } else { conversation.agent_id.clone() });
    let department_name = department_name_map
        .get(&conversation.department_id)
        .cloned()
        .unwrap_or_else(|| if conversation.department_id.trim().is_empty() { "未绑定部门".to_string() } else { conversation.department_id.clone() });
    let (kind_key, kind_label) = usage_kind_key_and_label(conversation);
    let usage_item = UsageConversationItem {
        conversation_id: conversation.id.clone(),
        title: conversation.title.clone(),
        updated_at: conversation.updated_at.clone(),
        archived_at: conversation.archived_at.clone(),
        agent_id: conversation.agent_id.clone(),
        agent_name: agent_name.clone(),
        department_id: conversation.department_id.clone(),
        department_name: department_name.clone(),
        api_config_id: api_config_id.clone(),
        api_config_name: api_config_name.clone(),
        model_name: model_name.clone(),
        conversation_kind: conversation.conversation_kind.clone(),
        is_delegate,
        is_system_notification_conversation: conversation_is_system_notification(conversation),
        message_count: conversation.messages.len(),
        weighted_tokens: weighted,
        input_tokens: cumulative.input_tokens,
        output_tokens: cumulative.output_tokens,
        cache_read_tokens: cumulative.cache_read_tokens,
        cache_write_tokens: cumulative.cache_write_tokens,
    };
    usage_aggregate_push(
        by_model,
        if model_name == "未绑定模型" { "unbound_model".to_string() } else { model_name.clone() },
        model_name,
        &usage_item,
    );
    usage_aggregate_push(
        by_api_config,
        if api_config_id.trim().is_empty() { "unbound_api_config".to_string() } else { api_config_id.clone() },
        api_config_name,
        &usage_item,
    );
    usage_aggregate_push(by_agent, usage_item.agent_id.clone(), agent_name, &usage_item);
    usage_aggregate_push(
        by_department,
        if usage_item.department_id.trim().is_empty() { "unbound_department".to_string() } else { usage_item.department_id.clone() },
        department_name,
        &usage_item,
    );
    usage_aggregate_push(by_kind, kind_key, kind_label, &usage_item);
    conversations.push(usage_item);
}

fn build_usage_overview(state: &AppState) -> Result<UsageOverview, String> {
    let config = state_read_config_cached(state)?;
    let runtime = state_read_agents_runtime_snapshot(state)?;
    let chat_index = state_read_chat_index_cached(state)?;
    let mut api_config_name_map = std::collections::HashMap::<String, String>::new();
    let mut api_config_model_map = std::collections::HashMap::<String, String>::new();
    for item in &config.api_configs {
        api_config_name_map.insert(item.id.clone(), item.name.clone());
        api_config_model_map.insert(item.id.clone(), item.model.clone());
    }
    let mut agent_name_map = std::collections::HashMap::<String, String>::new();
    for agent in &runtime.agents {
        agent_name_map.insert(agent.id.clone(), agent.name.clone());
    }
    let mut department_name_map = std::collections::HashMap::<String, String>::new();
    for department in &config.departments {
        department_name_map.insert(department.id.clone(), department.name.clone());
    }

    let mut totals = UsageOverviewTotals::default();

    let mut conversations = Vec::<UsageConversationItem>::new();
    let mut by_model = std::collections::HashMap::<String, UsageAggregateItem>::new();
    let mut by_api_config = std::collections::HashMap::<String, UsageAggregateItem>::new();
    let mut by_agent = std::collections::HashMap::<String, UsageAggregateItem>::new();
    let mut by_department = std::collections::HashMap::<String, UsageAggregateItem>::new();
    let mut by_kind = std::collections::HashMap::<String, UsageAggregateItem>::new();

    for item in chat_index.conversations {
        let conversation = match state_read_conversation_cached(state, &item.id) {
            Ok(value) => value,
            Err(_) => continue,
        };
        usage_push_conversation(
            &conversation,
            &config,
            &api_config_name_map,
            &api_config_model_map,
            &agent_name_map,
            &department_name_map,
            &mut totals,
            &mut conversations,
            &mut by_model,
            &mut by_api_config,
            &mut by_agent,
            &mut by_department,
            &mut by_kind,
        );
    }

    let mut seen_delegate_ids = std::collections::HashSet::<String>::new();
    for thread in delegate_runtime_thread_list(state)? {
        if !seen_delegate_ids.insert(thread.delegate_id.clone()) {
            continue;
        }
        usage_push_conversation(
            &thread.conversation,
            &config,
            &api_config_name_map,
            &api_config_model_map,
            &agent_name_map,
            &department_name_map,
            &mut totals,
            &mut conversations,
            &mut by_model,
            &mut by_api_config,
            &mut by_agent,
            &mut by_department,
            &mut by_kind,
        );
    }
    for thread in delegate_recent_thread_list(state)? {
        if !seen_delegate_ids.insert(thread.delegate_id.clone()) {
            continue;
        }
        usage_push_conversation(
            &thread.conversation,
            &config,
            &api_config_name_map,
            &api_config_model_map,
            &agent_name_map,
            &department_name_map,
            &mut totals,
            &mut conversations,
            &mut by_model,
            &mut by_api_config,
            &mut by_agent,
            &mut by_department,
            &mut by_kind,
        );
    }
    for conversation in delegate_persisted_conversation_list(state)? {
        let delegate_id = conversation
            .delegate_id
            .clone()
            .unwrap_or_else(|| conversation.id.clone());
        if !seen_delegate_ids.insert(delegate_id) {
            continue;
        }
        usage_push_conversation(
            &conversation,
            &config,
            &api_config_name_map,
            &api_config_model_map,
            &agent_name_map,
            &department_name_map,
            &mut totals,
            &mut conversations,
            &mut by_model,
            &mut by_api_config,
            &mut by_agent,
            &mut by_department,
            &mut by_kind,
        );
    }

    conversations.sort_by(|left, right| {
        right
            .weighted_tokens
            .cmp(&left.weighted_tokens)
            .then_with(|| right.output_tokens.cmp(&left.output_tokens))
            .then_with(|| right.updated_at.cmp(&left.updated_at))
            .then_with(|| left.conversation_id.cmp(&right.conversation_id))
    });

    Ok(UsageOverview {
        generated_at: now_iso(),
        totals,
        conversations,
        by_model: usage_sort_aggregate_items(by_model),
        by_api_config: usage_sort_aggregate_items(by_api_config),
        by_agent: usage_sort_aggregate_items(by_agent),
        by_department: usage_sort_aggregate_items(by_department),
        by_kind: usage_sort_aggregate_items(by_kind),
    })
}

#[tauri::command]
fn get_usage_overview(state: State<'_, AppState>) -> Result<UsageOverview, String> {
    build_usage_overview(&state)
}

fn storage_existing_directory_for_open(path: &PathBuf) -> Result<PathBuf, String> {
    if path.exists() {
        if path.is_dir() {
            return Ok(path.clone());
        }
        if let Some(parent) = path.parent() {
            return Ok(parent.to_path_buf());
        }
    }
    let mut current = path.as_path();
    while let Some(parent) = current.parent() {
        if parent.exists() && parent.is_dir() {
            return Ok(parent.to_path_buf());
        }
        current = parent;
    }
    Err(format!("目标目录不存在，path={}", path.display()))
}

#[tauri::command]
fn open_storage_usage_item_directory(
    state: State<'_, AppState>,
    input: OpenStorageUsageItemDirectoryInput,
) -> Result<(), String> {
    let item_id = input.item_id.trim();
    let target = storage_usage_target_path(&state, item_id)
        .ok_or_else(|| format!("未知存储分类：{item_id}"))?;
    let app_root = app_root_from_data_path(&state.data_path);
    let open_dir = storage_existing_directory_for_open(&target)?;
    let canonical_root = app_root.canonicalize().unwrap_or(app_root);
    let canonical_open_dir = open_dir.canonicalize().unwrap_or(open_dir.clone());
    if !canonical_open_dir.starts_with(&canonical_root) {
        return Err(format!(
            "拒绝打开应用私有目录之外的路径，path={}",
            canonical_open_dir.display()
        ));
    }
    open_shell_path_in_file_manager(&canonical_open_dir)
}

fn cleanup_storage_legacy_scope(
    state: &AppState,
    scope: StorageLegacyConversationScope,
) -> Result<StorageCleanupResult, String> {
    let scan = storage_scan_legacy_cleanup_candidates(&state.data_path, scope)?;
    let skipped_file_count = scan
        .total_file_count
        .saturating_sub(scan.cleanable_paths.len());
    let expected_dir = storage_conversation_dir(&state.data_path, scope);
    let mut deleted_file_count = 0;
    let mut freed_bytes = 0_u64;
    for path in scan.cleanable_paths {
        if path.parent() != Some(expected_dir.as_path()) {
            return Err(format!(
                "拒绝清理旧会话文件：候选路径不在预期目录内，path={}，expected_dir={}",
                path.display(),
                expected_dir.display()
            ));
        }
        let bytes = fs::metadata(&path).map(|metadata| metadata.len()).unwrap_or(0);
        fs::remove_file(&path).map_err(|err| {
            format!("删除旧会话文件失败，path={}，error={err}", path.display())
        })?;
        deleted_file_count += 1;
        freed_bytes = freed_bytes.saturating_add(bytes);
    }
    if deleted_file_count > 0 {
        refresh_message_store_migration_caches(state)?;
    }
    Ok(StorageCleanupResult {
        deleted_file_count,
        skipped_file_count,
        freed_bytes,
    })
}

#[tauri::command]
fn cleanup_storage_legacy_items(
    state: State<'_, AppState>,
    input: CleanupStorageLegacyItemsInput,
) -> Result<StorageCleanupResult, String> {
    let cleanup_kind = input.cleanup_kind.trim();
    let (scope, label) = match cleanup_kind {
        STORAGE_CLEANUP_LEGACY_CONVERSATIONS => (
            StorageLegacyConversationScope::Normal,
            "旧普通会话 JSON",
        ),
        STORAGE_CLEANUP_LEGACY_DELEGATE_CONVERSATIONS => (
            StorageLegacyConversationScope::Delegate,
            "旧委托会话 JSON",
        ),
        _ => return Err(format!("未知存储清理类型：{cleanup_kind}")),
    };
    let _migration_guard = lock_message_store_migration();
    eprintln!(
        "[存储] 开始，任务=清理{}，cleanup_kind={}",
        label,
        cleanup_kind
    );
    let started_at = std::time::Instant::now();
    let result = cleanup_storage_legacy_scope(&state, scope);
    match &result {
        Ok(report) => eprintln!(
            "[存储] 完成，任务=清理{}，cleanup_kind={}，删除文件数={}，跳过文件数={}，释放字节={}，耗时毫秒={}",
            label,
            cleanup_kind,
            report.deleted_file_count,
            report.skipped_file_count,
            report.freed_bytes,
            started_at.elapsed().as_millis()
        ),
        Err(err) => eprintln!(
            "[存储] 失败，任务=清理{}，cleanup_kind={}，error={}，耗时毫秒={}",
            label,
            cleanup_kind,
            err,
            started_at.elapsed().as_millis()
        ),
    }
    result
}

#[cfg(test)]
mod storage_usage_tests {
    use super::*;

    fn storage_usage_test_message(id: &str, role: &str) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            role: role.to_string(),
            created_at: "2026-06-08T00:00:00Z".to_string(),
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

    fn storage_usage_test_conversation(conversation_id: &str, kind: &str) -> Conversation {
        Conversation {
            id: conversation_id.to_string(),
            title: "测试会话".to_string(),
            agent_id: DEFAULT_AGENT_ID.to_string(),
            department_id: ASSISTANT_DEPARTMENT_ID.to_string(),
            bound_conversation_id: None,
            parent_conversation_id: None,
            child_conversation_ids: Vec::new(),
            fork_message_cursor: None,
            unread_count: 0,
            conversation_kind: kind.to_string(),
            root_conversation_id: (kind == CONVERSATION_KIND_DELEGATE)
                .then_some("root-conversation".to_string()),
            delegate_id: (kind == CONVERSATION_KIND_DELEGATE)
                .then_some(conversation_id.to_string()),
            created_at: "2026-06-08T00:00:00Z".to_string(),
            updated_at: "2026-06-08T00:00:01Z".to_string(),
            last_user_at: Some("2026-06-08T00:00:00Z".to_string()),
            last_assistant_at: Some("2026-06-08T00:00:01Z".to_string()),
            status: String::new(),
            summary: String::new(),
            user_profile_snapshot: String::new(),
            shell_workspace_path: None,
            shell_workspaces: Vec::new(),
            shell_autonomous_mode: false,
            archived_at: None,
            messages: vec![
                storage_usage_test_message("m1", "user"),
                storage_usage_test_message("m2", "assistant"),
            ],
            current_todos: Vec::new(),
            memory_recall_table: Vec::new(),
            plan_mode_enabled: false,
            preferred_api_config_id: None,
            auto_push_remote_contact_id: None,
            active_goal: None,
            cumulative_usage: ConversationCumulativeUsage::default(),
        }
    }

    #[test]
    fn storage_cleanup_candidates_require_ready_normal_message_store() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-storage-cleanup-normal-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let ready = storage_usage_test_conversation("ready-conversation", "");
        let legacy_only = storage_usage_test_conversation("legacy-only", "");
        let ready_paths =
            message_store::message_store_paths(&data_path, &ready.id).expect("ready paths");
        message_store::write_jsonl_snapshot_directory_shard_if_changed(&ready_paths, &ready)
            .expect("write ready message store");
        write_json_file_atomic(
            &app_layout_chat_conversation_path(&data_path, &ready.id),
            &ready,
            "legacy ready conversation",
        )
        .expect("write ready legacy");
        write_json_file_atomic(
            &app_layout_chat_conversation_path(&data_path, &legacy_only.id),
            &legacy_only,
            "legacy only conversation",
        )
        .expect("write legacy only");

        let scan = storage_scan_legacy_cleanup_candidates(
            &data_path,
            StorageLegacyConversationScope::Normal,
        )
        .expect("scan cleanup candidates");

        assert_eq!(scan.total_file_count, 2);
        assert_eq!(scan.cleanable_paths.len(), 1);
        assert_eq!(
            scan.cleanable_paths[0],
            app_layout_chat_conversation_path(&data_path, &ready.id)
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn storage_cleanup_candidates_require_ready_delegate_message_store() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-storage-cleanup-delegate-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let ready = storage_usage_test_conversation("delegate-ready", CONVERSATION_KIND_DELEGATE);
        let legacy_only =
            storage_usage_test_conversation("delegate-legacy", CONVERSATION_KIND_DELEGATE);
        delegate_conversation_store_write(&data_path, &ready).expect("write ready delegate store");
        let ready_legacy_path =
            delegate_conversation_store_path(&data_path, &ready.id).expect("ready legacy path");
        let legacy_only_path = delegate_conversation_store_path(&data_path, &legacy_only.id)
            .expect("legacy only path");
        write_json_file_atomic(&ready_legacy_path, &ready, "legacy ready delegate")
            .expect("write ready legacy delegate");
        write_json_file_atomic(&legacy_only_path, &legacy_only, "legacy only delegate")
            .expect("write legacy only delegate");

        let scan = storage_scan_legacy_cleanup_candidates(
            &data_path,
            StorageLegacyConversationScope::Delegate,
        )
        .expect("scan delegate cleanup candidates");

        assert_eq!(scan.total_file_count, 2);
        assert_eq!(scan.cleanable_paths.len(), 1);
        assert_eq!(scan.cleanable_paths[0], ready_legacy_path);
        let _ = fs::remove_dir_all(root);
    }
}

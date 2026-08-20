#[derive(Debug, Clone)]
pub(super) struct MessageStoreLimitPage {
    pub(super) messages: Vec<ChatMessage>,
    pub(super) has_more: bool,
}

#[derive(Debug, Clone)]
pub(super) struct MessageStoreStatus {
    pub(super) manifest_exists: bool,
    pub(super) legacy_shard_exists: bool,
    pub(super) directory_shard_exists: bool,
    pub(super) message_store_kind: String,
    pub(super) migration_state: String,
    pub(super) source_message_count: usize,
    pub(super) last_message_id: String,
    pub(super) messages_jsonl_bytes: u64,
    pub(super) updated_at: String,
    pub(super) ready_jsonl: bool,
}

#[derive(Debug, Clone)]
pub(super) struct MessageStoreIndexSummary {
    pub(super) message_count: usize,
    pub(super) visible_message_count: usize,
    pub(super) last_message_id: String,
    pub(super) last_message_at: Option<String>,
    pub(super) first_user_text_preview: Option<String>,
    pub(super) preview_items: Vec<MessageStoreIndexPreviewItem>,
}

#[derive(Debug, Clone)]
pub(super) struct MessageStoreIndexPreviewItem {
    pub(super) message_id: String,
    pub(super) role: String,
    pub(super) speaker_agent_id: Option<String>,
    pub(super) created_at: Option<String>,
    pub(super) text_preview: String,
    pub(super) has_image: bool,
    pub(super) has_pdf: bool,
    pub(super) has_audio: bool,
    pub(super) has_attachment: bool,
}

#[derive(Debug, Clone)]
pub(super) struct MessageStoreChatSnapshot {
    pub(super) latest_user: Option<ChatMessage>,
    pub(super) latest_assistant: Option<ChatMessage>,
    pub(super) active_message_count: usize,
}

#[derive(Debug, Clone)]
pub(super) struct MessageStoreCompactionSegment {
    pub(super) messages: Vec<ChatMessage>,
    pub(super) boundary_message_id: Option<String>,
    pub(super) previous_boundary_message_id: Option<String>,
    pub(super) has_previous_segment: bool,
}

#[derive(Debug, Clone)]
pub(super) struct MessageStoreBlockSummary {
    pub(super) block_id: u32,
    pub(super) message_count: usize,
    pub(super) first_message_id: String,
    pub(super) last_message_id: String,
    pub(super) first_created_at: Option<String>,
    pub(super) last_created_at: Option<String>,
    pub(super) is_latest: bool,
}

#[derive(Debug, Clone)]
pub(super) struct MessageStoreBlockPage {
    pub(super) blocks: Vec<MessageStoreBlockSummary>,
    pub(super) selected_block_id: u32,
    pub(super) messages: Vec<ChatMessage>,
    pub(super) has_prev_block: bool,
    pub(super) has_next_block: bool,
}

#[derive(Debug, Clone)]
pub(super) struct MessageStoreBlockMessagePage {
    pub(super) selected_block_id: u32,
    pub(super) messages: Vec<ChatMessage>,
    pub(super) has_more: bool,
}

#[derive(Debug, Clone)]
pub(super) struct MessageStoreBranchSelection {
    pub(super) selected_messages: Vec<ChatMessage>,
    pub(super) first_selected_ordinal: usize,
    pub(super) latest_compaction_message: Option<ChatMessage>,
}

#[derive(Debug, Clone)]
pub(super) struct MessageStoreRewindSlice {
    pub(super) keep_count: usize,
    pub(super) removed_messages: Vec<ChatMessage>,
    pub(super) recalled_user_message: ChatMessage,
}

#[derive(Debug, Clone)]
pub(super) struct MessageStoreToolCallResultAppend {
    pub(super) conversation: Conversation,
    pub(super) assistant_message_id: String,
    pub(super) created: bool,
    pub(super) tool_event_count: usize,
}

trait MessageStore {
    fn read_all_messages(&self) -> Result<Vec<ChatMessage>, String>;
    fn read_recent_messages(&self, limit: usize) -> Result<Vec<ChatMessage>, String>;
    fn read_message_by_id(&self, message_id: &str) -> Result<ChatMessage, String>;
    fn read_messages_before(&self, before_message_id: &str, limit: usize) -> Result<MessageStoreLimitPage, String>;
    fn read_messages_after(&self, after_message_id: &str, limit: usize) -> Result<MessageStoreLimitPage, String>;
    fn read_current_compaction_segment(&self) -> Result<MessageStoreCompactionSegment, String>;
    fn read_compaction_segment_before(&self, boundary_message_id: &str) -> Result<MessageStoreCompactionSegment, String>;
}

struct ConversationJsonMessageStore<'a> {
    conversation: &'a Conversation,
}

struct JsonlSnapshotMessageStore {
    messages_file: PathBuf,
    index_file: Option<PathBuf>,
    sqlite_index: Option<Arc<MessageStoreIndexFile>>,
}

#[derive(Debug, Clone)]
struct CachedMessageStoreBlockFile {
    modified_at: Option<std::time::SystemTime>,
    len: u64,
    messages_by_id: Arc<std::collections::HashMap<String, ChatMessage>>,
}

enum MessageStoreBackend<'a> {
    ConversationJson(ConversationJsonMessageStore<'a>),
    JsonlSnapshot(JsonlSnapshotMessageStore),
}

static MESSAGE_STORE_BLOCK_FILE_CACHE: OnceLock<
    Mutex<std::collections::HashMap<PathBuf, CachedMessageStoreBlockFile>>,
> = OnceLock::new();

fn message_store_block_file_cache(
) -> &'static Mutex<std::collections::HashMap<PathBuf, CachedMessageStoreBlockFile>> {
    MESSAGE_STORE_BLOCK_FILE_CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn lock_message_store_block_file_cache(
) -> std::sync::MutexGuard<
    'static,
    std::collections::HashMap<PathBuf, CachedMessageStoreBlockFile>,
> {
    message_store_block_file_cache().lock().unwrap_or_else(|poison| {
        runtime_log_info(format!(
            "[消息存储] 会话块缓存锁已污染，继续使用内部状态，error={:?}",
            poison
        ));
        poison.into_inner()
    })
}

pub(super) fn retain_message_store_block_file_cache_paths(
    allowed_paths: &std::collections::HashSet<PathBuf>,
) {
    let mut cache = lock_message_store_block_file_cache();
    cache.retain(|path, _| allowed_paths.contains(path));
}

pub(super) fn forget_message_store_block_file_cache_paths(
    paths: &std::collections::HashSet<PathBuf>,
) {
    if paths.is_empty() {
        return;
    }
    let mut cache = lock_message_store_block_file_cache();
    for path in paths {
        cache.remove(path);
    }
}

pub(super) fn message_store_block_file_cache_stats() -> (usize, usize, usize) {
    let cache = lock_message_store_block_file_cache();
    let entry_count = cache.len();
    let message_count = cache
        .values()
        .map(|item| item.messages_by_id.len())
        .sum::<usize>();
    let estimated_json_bytes = cache
        .values()
        .map(|item| {
            item.messages_by_id
                .values()
                .map(|message| serde_json::to_vec(message).map(|raw| raw.len()).unwrap_or(0))
                .sum::<usize>()
        })
        .sum::<usize>();
    (entry_count, message_count, estimated_json_bytes)
}

impl<'a> ConversationJsonMessageStore<'a> {
    fn new(conversation: &'a Conversation) -> Self {
        Self { conversation }
    }

    fn messages(&self) -> &[ChatMessage] {
        &self.conversation.messages
    }
}

impl JsonlSnapshotMessageStore {
    fn new(messages_file: PathBuf) -> Self {
        Self {
            messages_file,
            index_file: None,
            sqlite_index: None,
        }
    }

    fn with_index(messages_file: PathBuf, index_file: PathBuf) -> Self {
        Self {
            messages_file,
            index_file: Some(index_file),
            sqlite_index: None,
        }
    }

    fn with_sqlite_index(messages_file: PathBuf, index: MessageStoreIndexFile) -> Self {
        Self {
            messages_file,
            index_file: None,
            sqlite_index: Some(Arc::new(index)),
        }
    }

    fn messages(&self) -> Result<Vec<ChatMessage>, String> {
        if let Some(index) = self.index()? {
            return read_jsonl_snapshot_messages_by_index_items(&self.messages_file, &index.items);
        }
        read_jsonl_snapshot_messages_file(&self.messages_file)
    }

    fn index(&self) -> Result<Option<Arc<MessageStoreIndexFile>>, String> {
        if let Some(index) = self.sqlite_index.as_ref() {
            return Ok(Some(Arc::clone(index)));
        }
        let Some(index_file) = self.index_file.as_ref() else {
            return Ok(None);
        };
        if !index_file.exists() {
            return Ok(None);
        }
        read_message_store_index_file(index_file).map(Some)
    }

    fn read_messages_after_all(&self, after_message_id: &str) -> Result<Vec<ChatMessage>, String> {
        if let Some(index) = self.index()? {
            return read_messages_after_all_from_index(&self.messages_file, &index, after_message_id);
        }
        let messages = self.messages()?;
        read_messages_after_all_from_slice(&messages, after_message_id)
    }

    fn read_recent_messages_page(&self, limit: usize) -> Result<MessageStoreLimitPage, String> {
        if let Some(index) = self.index()? {
            return read_recent_messages_page_from_index(&self.messages_file, &index, limit);
        }
        let messages = self.messages()?;
        read_recent_messages_page_from_slice(&messages, limit)
    }
}

impl MessageStore for ConversationJsonMessageStore<'_> {
    fn read_all_messages(&self) -> Result<Vec<ChatMessage>, String> {
        Ok(self.messages().to_vec())
    }

    fn read_recent_messages(&self, limit: usize) -> Result<Vec<ChatMessage>, String> {
        read_recent_messages_from_slice(self.messages(), limit)
    }

    fn read_message_by_id(&self, message_id: &str) -> Result<ChatMessage, String> {
        read_message_by_id_from_slice(self.messages(), message_id)
    }

    fn read_messages_before(&self, before_message_id: &str, limit: usize) -> Result<MessageStoreLimitPage, String> {
        read_messages_before_from_slice(self.messages(), before_message_id, limit)
    }

    fn read_messages_after(&self, after_message_id: &str, limit: usize) -> Result<MessageStoreLimitPage, String> {
        read_messages_after_from_slice(self.messages(), after_message_id, limit)
    }

    fn read_current_compaction_segment(&self) -> Result<MessageStoreCompactionSegment, String> {
        read_current_compaction_segment_from_slice(self.messages())
    }

    fn read_compaction_segment_before(&self, boundary_message_id: &str) -> Result<MessageStoreCompactionSegment, String> {
        read_compaction_segment_before_from_slice(self.messages(), boundary_message_id)
    }
}

impl MessageStore for JsonlSnapshotMessageStore {
    fn read_all_messages(&self) -> Result<Vec<ChatMessage>, String> {
        self.messages()
    }

    fn read_recent_messages(&self, limit: usize) -> Result<Vec<ChatMessage>, String> {
        if let Some(index) = self.index()? {
            return read_recent_messages_from_index(&self.messages_file, &index, limit);
        }
        let messages = self.messages()?;
        read_recent_messages_from_slice(&messages, limit)
    }

    fn read_message_by_id(&self, message_id: &str) -> Result<ChatMessage, String> {
        if let Some(index) = self.index()? {
            return read_message_by_id_from_index(&self.messages_file, &index, message_id);
        }
        let messages = self.messages()?;
        read_message_by_id_from_slice(&messages, message_id)
    }

    fn read_messages_before(&self, before_message_id: &str, limit: usize) -> Result<MessageStoreLimitPage, String> {
        if let Some(index) = self.index()? {
            return read_messages_before_from_index(&self.messages_file, &index, before_message_id, limit);
        }
        let messages = self.messages()?;
        read_messages_before_from_slice(&messages, before_message_id, limit)
    }

    fn read_messages_after(&self, after_message_id: &str, limit: usize) -> Result<MessageStoreLimitPage, String> {
        if let Some(index) = self.index()? {
            return read_messages_after_from_index(&self.messages_file, &index, after_message_id, limit);
        }
        let messages = self.messages()?;
        read_messages_after_from_slice(&messages, after_message_id, limit)
    }

    fn read_current_compaction_segment(&self) -> Result<MessageStoreCompactionSegment, String> {
        if let Some(index) = self.index()? {
            return read_current_compaction_segment_from_index(&self.messages_file, &index);
        }
        let messages = self.messages()?;
        read_current_compaction_segment_from_slice(&messages)
    }

    fn read_compaction_segment_before(&self, boundary_message_id: &str) -> Result<MessageStoreCompactionSegment, String> {
        if let Some(index) = self.index()? {
            return read_compaction_segment_before_from_index(&self.messages_file, &index, boundary_message_id);
        }
        let messages = self.messages()?;
        read_compaction_segment_before_from_slice(&messages, boundary_message_id)
    }
}

impl MessageStore for MessageStoreBackend<'_> {
    fn read_all_messages(&self) -> Result<Vec<ChatMessage>, String> {
        match self {
            MessageStoreBackend::ConversationJson(store) => store.read_all_messages(),
            MessageStoreBackend::JsonlSnapshot(store) => store.read_all_messages(),
        }
    }

    fn read_recent_messages(&self, limit: usize) -> Result<Vec<ChatMessage>, String> {
        match self {
            MessageStoreBackend::ConversationJson(store) => store.read_recent_messages(limit),
            MessageStoreBackend::JsonlSnapshot(store) => store.read_recent_messages(limit),
        }
    }

    fn read_message_by_id(&self, message_id: &str) -> Result<ChatMessage, String> {
        match self {
            MessageStoreBackend::ConversationJson(store) => store.read_message_by_id(message_id),
            MessageStoreBackend::JsonlSnapshot(store) => store.read_message_by_id(message_id),
        }
    }

    fn read_messages_before(&self, before_message_id: &str, limit: usize) -> Result<MessageStoreLimitPage, String> {
        match self {
            MessageStoreBackend::ConversationJson(store) => store.read_messages_before(before_message_id, limit),
            MessageStoreBackend::JsonlSnapshot(store) => store.read_messages_before(before_message_id, limit),
        }
    }

    fn read_messages_after(&self, after_message_id: &str, limit: usize) -> Result<MessageStoreLimitPage, String> {
        match self {
            MessageStoreBackend::ConversationJson(store) => store.read_messages_after(after_message_id, limit),
            MessageStoreBackend::JsonlSnapshot(store) => store.read_messages_after(after_message_id, limit),
        }
    }

    fn read_current_compaction_segment(&self) -> Result<MessageStoreCompactionSegment, String> {
        match self {
            MessageStoreBackend::ConversationJson(store) => store.read_current_compaction_segment(),
            MessageStoreBackend::JsonlSnapshot(store) => store.read_current_compaction_segment(),
        }
    }

    fn read_compaction_segment_before(&self, boundary_message_id: &str) -> Result<MessageStoreCompactionSegment, String> {
        match self {
            MessageStoreBackend::ConversationJson(store) => store.read_compaction_segment_before(boundary_message_id),
            MessageStoreBackend::JsonlSnapshot(store) => store.read_compaction_segment_before(boundary_message_id),
        }
    }
}

fn message_store_backend_for_conversation<'a>(
    paths: &MessageStorePaths,
    conversation: &'a Conversation,
) -> Result<MessageStoreBackend<'a>, String> {
    let manifest = read_message_store_manifest(&paths.manifest_file)?;
    if let Some(item) = manifest.as_ref() {
        if matches!(
            (item.message_store_kind, item.migration_state),
            (MessageStoreKind::JsonlSnapshot, MessageStoreMigrationState::Ready)
        ) {
            validate_ready_message_store_snapshot_integrity(paths, item)?;
            return Ok(MessageStoreBackend::JsonlSnapshot(
                JsonlSnapshotMessageStore::with_index(
                    paths.messages_file.clone(),
                    paths.index_file.clone(),
                ),
            ));
        }
        if matches!(
            (item.message_store_kind, item.migration_state),
            (MessageStoreKind::JsonlEventLog, MessageStoreMigrationState::Ready)
        ) {
            return Err(format!(
                "消息存储暂不支持读取 JSONL 事件日志，conversation_id={}",
                conversation.id
            ));
        }
    }
    if let Some(reason) = manifest.and_then(|item| item.stale_jsonl_reason()) {
        runtime_log_warn(format!(
            "[消息存储] 跳过目录型消息 store，conversation_id={}，reason={}",
            conversation.id, reason
        ));
    }
    Ok(MessageStoreBackend::ConversationJson(
        ConversationJsonMessageStore::new(conversation),
    ))
}

pub(super) fn read_ready_message_store_directory_conversation(
    paths: &MessageStorePaths,
) -> Result<Option<Conversation>, String> {
    if paths.is_v3_ready()? {
        return chat_metadata_store_read_conversation(paths);
    }
    let Some(manifest) = read_message_store_manifest(&paths.manifest_file)? else {
        return Ok(None);
    };
    if matches!(
        (manifest.message_store_kind, manifest.migration_state),
        (
            MessageStoreKind::JsonlSnapshot,
            MessageStoreMigrationState::Ready
        )
    ) {
        return read_message_store_directory_conversation_with_manifest(paths, manifest).map(Some);
    }
    if matches!(
        (manifest.message_store_kind, manifest.migration_state),
        (
            MessageStoreKind::JsonlEventLog,
            MessageStoreMigrationState::Ready
        )
    ) {
        return Err(format!(
            "目录型会话暂不支持读取 JSONL 事件日志，path={}",
            paths.manifest_file.display()
        ));
    }
    Ok(None)
}

pub(super) fn read_ready_message_store_meta(
    paths: &MessageStorePaths,
) -> Result<Option<ConversationShardMeta>, String> {
    if paths.is_v3_ready()? {
        return chat_metadata_store_read_meta(paths);
    }
    let Some(manifest) = read_message_store_manifest(&paths.manifest_file)? else {
        return Ok(None);
    };
    if matches!(
        (manifest.message_store_kind, manifest.migration_state),
        (
            MessageStoreKind::JsonlSnapshot,
            MessageStoreMigrationState::Ready
        )
    ) {
        let meta = read_conversation_shard_meta(&paths.meta_file)?;
        validate_conversation_shard_meta_id(paths, &meta)?;
        return Ok(Some(meta));
    }
    if matches!(
        (manifest.message_store_kind, manifest.migration_state),
        (
            MessageStoreKind::JsonlEventLog,
            MessageStoreMigrationState::Ready
        )
    ) {
        return Err(format!(
            "目录型会话暂不支持读取 JSONL 事件日志元数据，path={}",
            paths.manifest_file.display()
        ));
    }
    Ok(None)
}

/// 按替换后的消息集合重算最新摘要标题（统一派生规则）。
/// v3 走轻量摘要范围读取；非 v3 目录型会话按该后端既有整读成本读取并合并替换。
pub(super) fn recompute_latest_summary_title_after_replace(
    paths: &MessageStorePaths,
    updated_messages: &[ChatMessage],
) -> Result<Option<String>, String> {
    if paths.is_v3_ready()? {
        return chat_metadata_store_recompute_latest_summary_title(paths, updated_messages);
    }
    let mut current = read_message_store_directory_conversation(paths)?;
    for updated in updated_messages {
        if let Some(position) = current
            .messages
            .iter()
            .position(|message| message.id.trim() == updated.id.trim())
        {
            current.messages[position] = updated.clone();
        }
    }
    Ok(conversation_latest_summary_title(&current))
}

pub(super) fn read_ready_message_store_all_messages(
    paths: &MessageStorePaths,
) -> Result<Option<Vec<ChatMessage>>, String> {
    if paths.is_v3_ready()? {
        return chat_metadata_store_with_read_snapshot(paths, || {
            let index = chat_metadata_store_read_index(paths)?
                .ok_or_else(|| format!("读取 SQLite 会话索引失败，conversation_id={}", paths.conversation_id))?;
            JsonlSnapshotMessageStore::with_sqlite_index(paths.messages_file.clone(), index)
                .read_all_messages()
        })
        .map(Some);
    }
    let Some(store) = ready_jsonl_snapshot_store(paths)? else {
        return Ok(None);
    };
    store.read_all_messages().map(Some)
}

pub(super) fn read_ready_message_store_latest_compaction_message(
    paths: &MessageStorePaths,
) -> Result<Option<ChatMessage>, String> {
    if paths.is_v3_ready()? {
        return chat_metadata_store_read_latest_compaction_message(paths);
    }
    let Some(store) = ready_jsonl_snapshot_store(paths)? else {
        return Ok(None);
    };
    Ok(store
        .read_current_compaction_segment()?
        .messages
        .into_iter()
        .next())
}

pub(super) fn read_ready_message_store_recent_messages(
    paths: &MessageStorePaths,
    limit: usize,
) -> Result<Option<Vec<ChatMessage>>, String> {
    if paths.is_v3_ready()? {
        return Ok(Some(chat_metadata_store_read_recent_page(paths, limit, false)?.messages));
    }
    let Some(store) = ready_jsonl_snapshot_store(paths)? else {
        return Ok(None);
    };
    store.read_recent_messages(limit).map(Some)
}

pub(super) fn read_ready_message_store_recent_messages_page(
    paths: &MessageStorePaths,
    limit: usize,
) -> Result<Option<MessageStoreLimitPage>, String> {
    if paths.is_v3_ready()? {
        return Ok(Some(chat_metadata_store_read_recent_page(paths, limit, false)?));
    }
    let Some(store) = ready_jsonl_snapshot_store(paths)? else {
        return Ok(None);
    };
    store.read_recent_messages_page(limit).map(Some)
}

pub(super) fn read_ready_message_store_recent_messages_page_cached(
    paths: &MessageStorePaths,
    limit: usize,
) -> Result<Option<MessageStoreLimitPage>, String> {
    if paths.is_v3_ready()? {
        return Ok(Some(chat_metadata_store_read_recent_page(paths, limit, true)?));
    }
    let Some(manifest) = read_message_store_manifest(&paths.manifest_file)? else {
        return Ok(None);
    };
    if !manifest.should_read_jsonl() {
        return Ok(None);
    }
    validate_ready_message_store_snapshot_integrity(paths, &manifest)?;
    if !paths.index_file.exists() {
        return Ok(None);
    }
    let mut index = read_message_store_index_file(&paths.index_file)?;
    let manifest_last_message_id = manifest.last_message_id().trim().to_string();
    let manifest_message_count = manifest.source_message_count();
    let cached_index_stale =
        index.items.len() != manifest_message_count
            || index
                .items
                .last()
                .map(|item| item.message_id.trim())
                .unwrap_or_default()
                != manifest_last_message_id;
    if cached_index_stale {
        forget_message_store_index_cache(&paths.index_file);
        index = Arc::new(read_message_store_index_file_uncached(&paths.index_file)?);
    }
    let limit = normalized_message_limit(limit);
    let start = index.items.len().saturating_sub(limit);
    let mut messages =
        read_jsonl_snapshot_messages_by_index_items_cached(&paths.messages_file, &index.items[start..])?;
    let cached_page_stale = index.items.len() != messages.len().saturating_add(start)
        || messages
            .last()
            .map(|message| message.id.trim())
            .unwrap_or_default()
            != manifest_last_message_id;
    if cached_page_stale {
        let affected_paths = index.items[start..]
            .iter()
            .filter_map(|item| jsonl_snapshot_index_item_path(&paths.messages_file, item.block_id).ok())
            .collect::<std::collections::HashSet<_>>();
        forget_message_store_block_file_cache_paths(&affected_paths);
        messages = read_jsonl_snapshot_messages_by_index_items(&paths.messages_file, &index.items[start..])?;
    }
    Ok(Some(MessageStoreLimitPage {
        messages,
        has_more: start > 0,
    }))
}

fn read_message_store_index_file_uncached(path: &PathBuf) -> Result<MessageStoreIndexFile, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("读取消息索引失败，path={}，error={err}", path.display()))?;
    let index = serde_json::from_str::<MessageStoreIndexFile>(&raw)
        .map_err(|err| format!("解析消息索引失败，path={}，error={err}", path.display()))?;
    validate_message_store_index_file(path, &index)?;
    Ok(index.with_position_lookup())
}

pub(super) fn read_ready_message_store_recent_blocks_page(
    paths: &MessageStorePaths,
    block_limit: usize,
) -> Result<Option<MessageStoreLimitPage>, String> {
    read_ready_message_store_recent_blocks_page_with_cache(paths, block_limit, false)
}

pub(super) fn read_ready_message_store_recent_blocks_page_cached(
    paths: &MessageStorePaths,
    block_limit: usize,
) -> Result<Option<MessageStoreLimitPage>, String> {
    read_ready_message_store_recent_blocks_page_with_cache(paths, block_limit, true)
}

pub(super) fn read_ready_message_store_latest_block_paths(
    paths: &MessageStorePaths,
    block_limit: usize,
) -> Result<Option<Vec<PathBuf>>, String> {
    if paths.is_v3_ready()? {
        return Ok(Some(chat_metadata_store_latest_block_paths(paths, block_limit)?));
    }
    let Some(manifest) = read_message_store_manifest(&paths.manifest_file)? else {
        return Ok(None);
    };
    if !manifest.should_read_jsonl() {
        return Ok(None);
    }
    validate_ready_message_store_snapshot_integrity(paths, &manifest)?;
    if !paths.index_file.exists() {
        return Ok(None);
    }
    let index = read_message_store_index_file(&paths.index_file)?;
    let mut block_ids = ordered_message_store_index_block_ids(&index);
    if block_ids.is_empty() {
        return Ok(Some(Vec::new()));
    }
    let normalized_limit = block_limit.clamp(1, 8);
    let start = block_ids.len().saturating_sub(normalized_limit);
    block_ids = block_ids[start..].to_vec();
    let mut block_paths = Vec::<PathBuf>::with_capacity(block_ids.len());
    for block_id in block_ids {
        block_paths.push(jsonl_snapshot_index_item_path(
            &paths.messages_file,
            Some(block_id),
        )?);
    }
    Ok(Some(block_paths))
}

fn read_ready_message_store_recent_blocks_page_with_cache(
    paths: &MessageStorePaths,
    block_limit: usize,
    use_block_cache: bool,
) -> Result<Option<MessageStoreLimitPage>, String> {
    if paths.is_v3_ready()? {
        return Ok(Some(chat_metadata_store_read_recent_blocks_page(paths, block_limit, use_block_cache)?));
    }
    let Some(manifest) = read_message_store_manifest(&paths.manifest_file)? else {
        return Ok(None);
    };
    if !manifest.should_read_jsonl() {
        return Ok(None);
    }
    validate_ready_message_store_snapshot_integrity(paths, &manifest)?;
    if !paths.index_file.exists() {
        return Ok(None);
    }
    let index = read_message_store_index_file(&paths.index_file)?;
    let block_ids = ordered_message_store_index_block_ids(&index);
    if block_ids.is_empty() {
        return Ok(Some(MessageStoreLimitPage {
            messages: Vec::new(),
            has_more: false,
        }));
    }
    let normalized_limit = block_limit.clamp(1, 8);
    let selected_block_ids = block_ids
        .iter()
        .rev()
        .take(normalized_limit)
        .copied()
        .collect::<Vec<_>>();
    let mut selected_block_ids = selected_block_ids;
    selected_block_ids.reverse();
    let selected_block_ids = selected_block_ids
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    let selected_items = index
        .items
        .iter()
        .filter(|item| selected_block_ids.contains(&item.block_id.unwrap_or(0)))
        .cloned()
        .collect::<Vec<_>>();
    let messages = if use_block_cache {
        read_jsonl_snapshot_messages_by_index_items_cached(&paths.messages_file, &selected_items)?
    } else {
        read_jsonl_snapshot_messages_by_index_items(&paths.messages_file, &selected_items)?
    };
    Ok(Some(MessageStoreLimitPage {
        messages,
        has_more: block_ids.len() > normalized_limit,
    }))
}

pub(super) fn read_ready_message_store_message_by_id(
    paths: &MessageStorePaths,
    message_id: &str,
) -> Result<Option<ChatMessage>, String> {
    if paths.is_v3_ready()? {
        return chat_metadata_store_read_message_by_id(paths, message_id).map(Some);
    }
    let Some(store) = ready_jsonl_snapshot_store(paths)? else {
        return Ok(None);
    };
    store.read_message_by_id(message_id).map(Some)
}

pub(super) fn read_ready_message_store_message_sequence(
    paths: &MessageStorePaths,
    message_id: &str,
) -> Result<Option<usize>, String> {
    if paths.is_v3_ready()? {
        return chat_metadata_store_read_message_sequence(paths, message_id);
    }
    let Some(store) = ready_jsonl_snapshot_store(paths)? else {
        return Ok(None);
    };
    Ok(store
        .read_all_messages()?
        .iter()
        .position(|message| message.id.trim() == message_id.trim()))
}

pub(super) fn read_ready_message_store_messages_after_all(
    paths: &MessageStorePaths,
    after_message_id: &str,
) -> Result<Option<Vec<ChatMessage>>, String> {
    if paths.is_v3_ready()? {
        return chat_metadata_store_read_messages_after_all(paths, after_message_id).map(Some);
    }
    let Some(store) = ready_jsonl_snapshot_store(paths)? else {
        return Ok(None);
    };
    store.read_messages_after_all(after_message_id).map(Some)
}

pub(super) fn read_ready_message_store_rewind_slice(
    paths: &MessageStorePaths,
    message_id: &str,
) -> Result<Option<MessageStoreRewindSlice>, String> {
    if paths.is_v3_ready()? {
        return chat_metadata_store_with_read_snapshot(paths, || {
        let anchor = chat_metadata_store_read_locator_by_id(paths, message_id)?
            .ok_or_else(|| format!("Message not found: {}", message_id.trim()))?;
        let conn = chat_metadata_store_open(&paths.data_path)?;
        let locators = chat_metadata_store_query_locators(&conn, &paths.conversation_id, "AND sequence>=?2", &[&anchor.sequence])?;
        let removed_messages = chat_metadata_store_read_messages_for_locators(paths, &locators, false)?;
        let recalled_user_message = removed_messages.first().cloned()
            .ok_or_else(|| format!("Message not found: {}", message_id.trim()))?;
        Ok(Some(MessageStoreRewindSlice {
            keep_count: anchor.sequence as usize,
            removed_messages,
            recalled_user_message,
        }))
        });
    }
    let Some(manifest) = read_message_store_manifest(&paths.manifest_file)? else {
        return Ok(None);
    };
    if !manifest.should_read_jsonl() {
        return Ok(None);
    }
    validate_ready_message_store_snapshot_integrity(paths, &manifest)?;
    if !paths.index_file.exists() {
        return Ok(None);
    }
    let index = read_message_store_index_file(&paths.index_file)?;
    let message_idx = find_index_item_position(&index, message_id)
        .ok_or_else(|| format!("Message not found: {}", message_id.trim()))?;
    let removed_messages =
        read_jsonl_snapshot_messages_by_index_items(&paths.messages_file, &index.items[message_idx..])?;
    let recalled_user_message = removed_messages
        .first()
        .cloned()
        .ok_or_else(|| format!("Message not found: {}", message_id.trim()))?;
    Ok(Some(MessageStoreRewindSlice {
        keep_count: message_idx,
        removed_messages,
        recalled_user_message,
    }))
}

pub(super) fn read_message_store_status(
    paths: &MessageStorePaths,
    fallback_conversation: &Conversation,
) -> Result<MessageStoreStatus, String> {
    let manifest = read_message_store_manifest(&paths.manifest_file)?;
    let fallback_last_message_id = fallback_conversation
        .messages
        .last()
        .map(|message| message.id.trim().to_string())
        .unwrap_or_default();
    let (
        message_store_kind,
        migration_state,
        source_message_count,
        last_message_id,
        messages_jsonl_bytes,
        ready_jsonl,
    ) = if let Some(item) = manifest.as_ref() {
        (
            item.store_kind_label().to_string(),
            item.migration_state_label().to_string(),
            item.source_message_count(),
            item.last_message_id().to_string(),
            item.messages_jsonl_bytes(),
            item.should_read_jsonl(),
        )
    } else {
        (
            "conversationJson".to_string(),
            "none".to_string(),
            fallback_conversation.messages.len(),
            fallback_last_message_id,
            0,
            false,
        )
    };
    Ok(MessageStoreStatus {
        manifest_exists: manifest.is_some(),
        legacy_shard_exists: paths.legacy_conversation_file.exists(),
        directory_shard_exists: paths.shard_dir.exists(),
        message_store_kind,
        migration_state,
        source_message_count,
        last_message_id,
        messages_jsonl_bytes,
        updated_at: manifest
            .as_ref()
            .map(|item| item.updated_at().to_string())
            .unwrap_or_default(),
        ready_jsonl,
    })
}

pub(super) fn read_ready_message_store_status(
    paths: &MessageStorePaths,
) -> Result<Option<MessageStoreStatus>, String> {
    if paths.is_v3_ready()? {
        return chat_metadata_store_status(paths);
    }
    let Some(manifest) = read_message_store_manifest(&paths.manifest_file)? else {
        return Ok(None);
    };
    if !manifest.should_read_jsonl() {
        return Ok(None);
    }
    validate_ready_message_store_snapshot_integrity(paths, &manifest)?;
    let meta = read_conversation_shard_meta(&paths.meta_file)
        .map_err(|err| format!("ready JSONL 会话缺少可读 meta.json，无法读取消息存储状态: {err}"))?;
    validate_conversation_shard_meta_id(paths, &meta)?;
    Ok(Some(MessageStoreStatus {
        manifest_exists: true,
        legacy_shard_exists: paths.legacy_conversation_file.exists(),
        directory_shard_exists: paths.shard_dir.exists(),
        message_store_kind: manifest.store_kind_label().to_string(),
        migration_state: manifest.migration_state_label().to_string(),
        source_message_count: manifest.source_message_count(),
        last_message_id: manifest.last_message_id().to_string(),
        messages_jsonl_bytes: manifest.messages_jsonl_bytes(),
        updated_at: manifest.updated_at().to_string(),
        ready_jsonl: true,
    }))
}

pub(super) fn read_ready_message_store_index_summary(
    paths: &MessageStorePaths,
) -> Result<Option<MessageStoreIndexSummary>, String> {
    if paths.is_v3_ready()? {
        return Ok(Some(chat_metadata_store_index_summary(paths)?));
    }
    let Some(manifest) = read_message_store_manifest(&paths.manifest_file)? else {
        return Ok(None);
    };
    if !manifest.should_read_jsonl() {
        return Ok(None);
    }
    validate_ready_message_store_snapshot_integrity(paths, &manifest)?;
    if !paths.index_file.exists() {
        return Ok(None);
    }
    let index = read_message_store_index_file(&paths.index_file)?;
    let messages = read_jsonl_snapshot_messages_by_index_items(&paths.messages_file, &index.items)?;
    let last = messages.last();
    let visible_messages = messages
        .iter()
        .filter(|message| {
            matches!(
                message.role.trim().to_ascii_lowercase().as_str(),
                "user" | "assistant" | "tool"
            )
        })
        .collect::<Vec<_>>();
    let first_user_text_preview = messages
        .iter()
        .find(|message| {
            message.role.trim().eq_ignore_ascii_case("user")
                && message
                    .speaker_agent_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    != Some(SYSTEM_PERSONA_ID)
                && !build_conversation_preview_text(message).trim().is_empty()
        })
        .map(|message| build_conversation_preview_text(message).trim().to_string());
    let preview_start = visible_messages.len().saturating_sub(2);
    let preview_items = visible_messages[preview_start..]
        .iter()
        .map(|message| MessageStoreIndexPreviewItem {
            message_id: message.id.clone(),
            role: message.role.clone(),
            speaker_agent_id: message.speaker_agent_id.clone(),
            created_at: Some(message.created_at.clone()).filter(|value| !value.trim().is_empty()),
            text_preview: build_conversation_preview_text(message),
            has_image: message_store_message_has_image(message),
            has_pdf: message_store_message_has_pdf(message),
            has_audio: message_store_message_has_audio(message),
            has_attachment: conversation_message_has_attachment(message),
        })
        .collect::<Vec<_>>();
    Ok(Some(MessageStoreIndexSummary {
        message_count: index.items.len(),
        visible_message_count: visible_messages.len(),
        last_message_id: last
            .map(|message| message.id.trim().to_string())
            .unwrap_or_default(),
        last_message_at: last.map(|message| message.created_at.clone()),
        first_user_text_preview,
        preview_items,
    }))
}

pub(super) fn read_ready_message_store_chat_snapshot(
    paths: &MessageStorePaths,
) -> Result<Option<MessageStoreChatSnapshot>, String> {
    if paths.is_v3_ready()? {
        return Ok(Some(chat_metadata_store_chat_snapshot(paths)?));
    }
    let Some(manifest) = read_message_store_manifest(&paths.manifest_file)? else {
        return Ok(None);
    };
    if !manifest.should_read_jsonl() {
        return Ok(None);
    }
    validate_ready_message_store_snapshot_integrity(paths, &manifest)?;
    if !paths.index_file.exists() {
        return Ok(None);
    }
    let index = read_message_store_index_file(&paths.index_file)?;
    let messages = read_jsonl_snapshot_messages_by_index_items(&paths.messages_file, &index.items)?;
    let latest_user = messages
        .iter()
        .rev()
        .find(|message| message.role.trim().eq_ignore_ascii_case("user"))
        .cloned();
    let latest_assistant = messages
        .iter()
        .rev()
        .find(|message| message.role.trim().eq_ignore_ascii_case("assistant"))
        .cloned();
    let active_message_count = messages.len();
    Ok(Some(MessageStoreChatSnapshot {
        latest_user,
        latest_assistant,
        active_message_count,
    }))
}

pub(super) fn read_ready_message_store_branch_selection(
    paths: &MessageStorePaths,
    selected_message_ids: &[String],
) -> Result<Option<MessageStoreBranchSelection>, String> {
    if paths.is_v3_ready()? {
        return chat_metadata_store_with_read_snapshot(paths, || {
        let mut locators = selected_message_ids.iter()
            .map(|message_id| chat_metadata_store_read_locator_by_id(paths, message_id))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter().flatten().collect::<Vec<_>>();
        locators.sort_by_key(|locator| locator.sequence);
        let first_sequence = locators.first().map(|locator| locator.sequence);
        let conn = chat_metadata_store_open(&paths.data_path)?;
        let first_selected_ordinal = match first_sequence {
            Some(sequence) => conn.query_row(
                "SELECT COUNT(*) FROM message_locator WHERE conversation_id=?1 AND sequence<=?2 AND compaction_kind IS NULL",
                rusqlite::params![paths.conversation_id, sequence],
                |row| row.get::<_, i64>(0),
            ).map_err(|err| format!("统计 SQLite 分支选择位置失败: {err}"))? as usize,
            None => 0,
        };
        let latest_compaction_locator = conn.query_row(
            "SELECT sequence, message_id, block_id, byte_offset, byte_len, compaction_kind, role, created_at
             FROM message_locator WHERE conversation_id=?1 AND compaction_kind IS NOT NULL ORDER BY sequence DESC LIMIT 1",
            [&paths.conversation_id],
            |row| Ok(ChatMetadataLocator {
                sequence: row.get(0)?,
                item: MessageStoreIndexItem {
                    message_id: row.get(1)?, block_id: Some(row.get::<_, i64>(2)? as u32),
                    offset: row.get::<_, i64>(3)? as u64, byte_len: row.get::<_, i64>(4)? as u64,
                    compaction_kind: row.get(5)?, role: row.get(6)?, created_at: row.get(7)?,
                },
            }),
        ).optional().map_err(|err| format!("读取 SQLite 最新压缩消息失败: {err}"))?;
        let latest_compaction_message = latest_compaction_locator.map(|locator| {
            chat_metadata_store_read_messages_for_locators(paths, &[locator], false)?.pop()
                .ok_or_else(|| "读取 SQLite 最新压缩消息为空".to_string())
        }).transpose()?;
        Ok(Some(MessageStoreBranchSelection {
            selected_messages: chat_metadata_store_read_messages_for_locators(paths, &locators, false)?,
            first_selected_ordinal,
            latest_compaction_message,
        }))
        });
    }
    let Some(manifest) = read_message_store_manifest(&paths.manifest_file)? else {
        return Ok(None);
    };
    if !manifest.should_read_jsonl() {
        return Ok(None);
    }
    validate_ready_message_store_snapshot_integrity(paths, &manifest)?;
    if !paths.index_file.exists() {
        return Ok(None);
    }
    let selected_ids = selected_message_ids
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .collect::<std::collections::HashSet<_>>();
    let index = read_message_store_index_file(&paths.index_file)?;
    let mut selected_items = Vec::<MessageStoreIndexItem>::new();
    let mut visible_ordinal = 0usize;
    let mut first_selected_ordinal = 0usize;
    let mut latest_compaction_item: Option<MessageStoreIndexItem> = None;
    let boundaries = compaction_boundary_index_items(&index)
        .into_iter()
        .collect::<std::collections::HashSet<_>>();
    for (item_idx, item) in index.items.iter().enumerate() {
        if boundaries.contains(&item_idx) {
            latest_compaction_item = Some(item.clone());
            continue;
        }
        visible_ordinal += 1;
        if selected_ids.contains(item.message_id.trim()) {
            if first_selected_ordinal == 0 {
                first_selected_ordinal = visible_ordinal;
            }
            selected_items.push(item.clone());
        }
    }
    let selected_messages =
        read_jsonl_snapshot_messages_by_index_items(&paths.messages_file, &selected_items)?;
    let latest_compaction_message = if let Some(item) = latest_compaction_item {
        let mut messages =
            read_jsonl_snapshot_messages_by_index_items(&paths.messages_file, &[item])?;
        messages.pop()
    } else {
        None
    };
    Ok(Some(MessageStoreBranchSelection {
        selected_messages,
        first_selected_ordinal,
        latest_compaction_message,
    }))
}

pub(super) fn read_message_store_manifest_status(
    paths: &MessageStorePaths,
) -> Result<Option<MessageStoreStatus>, String> {
    if paths.is_v3_ready()? {
        return read_ready_message_store_status(paths);
    }
    let Some(manifest) = read_message_store_manifest(&paths.manifest_file)? else {
        return Ok(None);
    };
    Ok(Some(MessageStoreStatus {
        manifest_exists: true,
        legacy_shard_exists: paths.legacy_conversation_file.exists(),
        directory_shard_exists: paths.shard_dir.exists(),
        message_store_kind: manifest.store_kind_label().to_string(),
        migration_state: manifest.migration_state_label().to_string(),
        source_message_count: manifest.source_message_count(),
        last_message_id: manifest.last_message_id().to_string(),
        messages_jsonl_bytes: manifest.messages_jsonl_bytes(),
        updated_at: manifest.updated_at().to_string(),
        ready_jsonl: manifest.should_read_jsonl(),
    }))
}

pub(super) fn read_ready_message_store_messages_before(
    paths: &MessageStorePaths,
    before_message_id: &str,
    limit: usize,
) -> Result<Option<MessageStoreLimitPage>, String> {
    if paths.is_v3_ready()? {
        return Ok(Some(chat_metadata_store_read_messages_before(paths, before_message_id, limit)?));
    }
    let Some(store) = ready_jsonl_snapshot_store(paths)? else {
        return Ok(None);
    };
    store.read_messages_before(before_message_id, limit).map(Some)
}

pub(super) fn read_ready_message_store_messages_after(
    paths: &MessageStorePaths,
    after_message_id: &str,
    limit: usize,
) -> Result<Option<MessageStoreLimitPage>, String> {
    if paths.is_v3_ready()? {
        return Ok(Some(chat_metadata_store_read_messages_after(paths, after_message_id, limit)?));
    }
    let Some(store) = ready_jsonl_snapshot_store(paths)? else {
        return Ok(None);
    };
    store.read_messages_after(after_message_id, limit).map(Some)
}

pub(super) fn read_ready_message_store_current_compaction_segment(
    paths: &MessageStorePaths,
) -> Result<Option<MessageStoreCompactionSegment>, String> {
    if paths.is_v3_ready()? {
        return Ok(Some(chat_metadata_store_compaction_segment(paths, None)?));
    }
    let Some(store) = ready_jsonl_snapshot_store(paths)? else {
        return Ok(None);
    };
    store.read_current_compaction_segment().map(Some)
}

pub(super) fn read_ready_message_store_compaction_segment_before(
    paths: &MessageStorePaths,
    boundary_message_id: &str,
) -> Result<Option<MessageStoreCompactionSegment>, String> {
    if paths.is_v3_ready()? {
        return Ok(Some(chat_metadata_store_compaction_segment(paths, Some(boundary_message_id))?));
    }
    let Some(store) = ready_jsonl_snapshot_store(paths)? else {
        return Ok(None);
    };
    store
        .read_compaction_segment_before(boundary_message_id)
        .map(Some)
}

pub(super) fn read_ready_message_store_block_page(
    paths: &MessageStorePaths,
    requested_block_id: Option<u32>,
) -> Result<Option<MessageStoreBlockPage>, String> {
    if paths.is_v3_ready()? {
        return Ok(Some(chat_metadata_store_block_page(paths, requested_block_id)?));
    }
    let index = if let Some(index) = chat_metadata_store_read_index(paths)? {
        index
    } else {
        let Some(manifest) = read_message_store_manifest(&paths.manifest_file)? else {
            return Ok(None);
        };
        if !manifest.should_read_jsonl() {
            return Ok(None);
        }
        validate_ready_message_store_snapshot_integrity(paths, &manifest)?;
        if !paths.index_file.exists() {
            return Ok(None);
        }
        (*read_message_store_index_file(&paths.index_file)?).clone()
    };
    if index.items.is_empty() {
        return Ok(Some(MessageStoreBlockPage {
            blocks: Vec::new(),
            selected_block_id: requested_block_id.unwrap_or(0),
            messages: Vec::new(),
            has_prev_block: false,
            has_next_block: false,
        }));
    }
    let summaries = build_message_store_block_summaries(paths, &index)?;
    let selected_block_id = requested_block_id
        .filter(|block_id| summaries.iter().any(|item| item.block_id == *block_id))
        .or_else(|| summaries.last().map(|item| item.block_id))
        .unwrap_or(0);
    let selected_idx = summaries
        .iter()
        .position(|item| item.block_id == selected_block_id)
        .ok_or_else(|| {
            format!(
                "会话块不存在，conversation_id={}，block_id={selected_block_id}",
                paths.conversation_id
            )
        })?;
    let selected_items = index
        .items
        .iter()
        .filter(|item| item.block_id.unwrap_or(0) == selected_block_id)
        .cloned()
        .collect::<Vec<_>>();
    let messages =
        read_jsonl_snapshot_messages_by_index_items_cached(&paths.messages_file, &selected_items)?;
    let has_next_block = selected_idx + 1 < summaries.len();
    Ok(Some(MessageStoreBlockPage {
        blocks: summaries,
        selected_block_id,
        messages,
        has_prev_block: selected_idx > 0,
        has_next_block,
    }))
}

pub(super) fn read_ready_message_store_block_messages_before(
    paths: &MessageStorePaths,
    requested_block_id: Option<u32>,
    before_message_id: Option<&str>,
    limit: usize,
) -> Result<Option<MessageStoreBlockMessagePage>, String> {
    if paths.is_v3_ready()? {
        return Ok(Some(chat_metadata_store_read_block_messages_before(
            paths,
            requested_block_id,
            before_message_id,
            limit,
        )?));
    }
    let Some(store) = ready_jsonl_snapshot_store(paths)? else {
        return Ok(None);
    };
    let Some(index) = store.index()? else {
        return Err(format!(
            "读取 JSONL block 保留对话失败：缺少消息索引，conversation_id={}",
            paths.conversation_id
        ));
    };
    read_jsonl_block_messages_before(paths, &index, requested_block_id, before_message_id, limit)
        .map(Some)
}

/// 仅从索引计数触发消息所在同一当前 block 的总消息数。
///
/// 远程唤醒的低频判断要看当前 block 的完整消息规模，不能只看触发消息之前的数量。
pub(super) fn read_ready_message_store_block_message_count(
    paths: &MessageStorePaths,
    before_message_id: &str,
) -> Result<Option<usize>, String> {
    if paths.is_v3_ready()? {
        return Ok(Some(
            chat_metadata_store_count_block_messages(paths, before_message_id)?,
        ));
    }
    let Some(store) = ready_jsonl_snapshot_store(paths)? else {
        return Ok(None);
    };
    let Some(index) = store.index()? else {
        return Err(format!(
            "读取 JSONL block 群消息计数失败：缺少消息索引，conversation_id={}",
            paths.conversation_id
        ));
    };
    count_jsonl_block_messages(&index, before_message_id).map(Some)
}

pub(super) fn read_message_store_current_compaction_segment_for_conversation(
    paths: &MessageStorePaths,
    conversation: &Conversation,
) -> Result<MessageStoreCompactionSegment, String> {
    if paths.is_v3_ready()? {
        return chat_metadata_store_compaction_segment(paths, None);
    }
    message_store_backend_for_conversation(paths, conversation)?
        .read_current_compaction_segment()
}

pub(super) fn read_message_store_compaction_segment_before_for_conversation(
    paths: &MessageStorePaths,
    conversation: &Conversation,
    boundary_message_id: &str,
) -> Result<MessageStoreCompactionSegment, String> {
    if paths.is_v3_ready()? {
        return chat_metadata_store_compaction_segment(paths, Some(boundary_message_id));
    }
    message_store_backend_for_conversation(paths, conversation)?
        .read_compaction_segment_before(boundary_message_id)
}

pub(super) fn append_message_store_tool_group_result(
    paths: &MessageStorePaths,
    conversation: &Conversation,
    agent_id: &str,
    assistant_tool_call_event: Value,
    tool_result_event: Value,
    provider_meta_patch: Option<Value>,
    assistant_message_id: Option<&str>,
) -> Result<MessageStoreToolCallResultAppend, String> {
    if paths.is_v3_ready()? {
        let mut ready_meta = message_store::read_ready_message_store_meta(paths)?
            .ok_or_else(|| format!("追加工具结果失败：缺少 ready 消息元数据，conversation_id={}", paths.conversation_id))?;
        let tail_messages = message_store::read_ready_message_store_recent_messages_page_cached(paths, 1)?
            .map(|page| page.messages)
            .unwrap_or_default();
        let mut next = ready_meta.clone().into_conversation(tail_messages);
        let append = append_tool_group_result_to_conversation(
            &mut next,
            agent_id,
            assistant_tool_call_event,
            tool_result_event,
            provider_meta_patch,
            assistant_message_id,
        )?;
        ready_meta.apply_metadata_fields_from_conversation(&next);
        let updated_message = next.messages.last().ok_or_else(|| {
            format!("追加工具结果失败：缺少目标助理消息，conversation_id={}", paths.conversation_id)
        })?;
        if append.created {
            ready_meta.apply_appended_messages(std::slice::from_ref(updated_message));
            write_jsonl_snapshot_appended_messages_shard_from_meta(
                paths,
                &ready_meta,
                std::slice::from_ref(updated_message),
            )?;
        } else {
            write_jsonl_snapshot_replaced_message_shard(
                paths,
                &ready_meta.to_persist_meta(),
                updated_message,
            )?;
        }
        return Ok(MessageStoreToolCallResultAppend {
            conversation: next,
            assistant_message_id: append.assistant_message_id,
            created: append.created,
            tool_event_count: append.tool_event_count,
        });
    }
    let mut next = conversation.clone();
    let append = append_tool_group_result_to_conversation(
        &mut next,
        agent_id,
        assistant_tool_call_event,
        tool_result_event,
        provider_meta_patch,
        assistant_message_id,
    )?;
    write_jsonl_snapshot_directory_shard(paths, &next)?;
    Ok(MessageStoreToolCallResultAppend {
        conversation: next,
        assistant_message_id: append.assistant_message_id,
        created: append.created,
        tool_event_count: append.tool_event_count,
    })
}

pub(super) fn apply_message_store_tool_group_result(
    conversation: &Conversation,
    agent_id: &str,
    assistant_tool_call_event: Value,
    tool_result_event: Value,
    provider_meta_patch: Option<Value>,
    assistant_message_id: Option<&str>,
) -> Result<MessageStoreToolCallResultAppend, String> {
    let mut next = conversation.clone();
    let append = append_tool_group_result_to_conversation(
        &mut next,
        agent_id,
        assistant_tool_call_event,
        tool_result_event,
        provider_meta_patch,
        assistant_message_id,
    )?;
    Ok(MessageStoreToolCallResultAppend {
        conversation: next,
        assistant_message_id: append.assistant_message_id,
        created: append.created,
        tool_event_count: append.tool_event_count,
    })
}

#[derive(Debug, Clone)]
struct ToolGroupResultAppend {
    assistant_message_id: String,
    created: bool,
    tool_event_count: usize,
}

fn append_tool_group_result_to_conversation(
    conversation: &mut Conversation,
    agent_id: &str,
    assistant_tool_call_event: Value,
    tool_result_event: Value,
    provider_meta_patch: Option<Value>,
    assistant_message_id: Option<&str>,
) -> Result<ToolGroupResultAppend, String> {
    let tool_call_id =
        validate_tool_group_result_append(&assistant_tool_call_event, &tool_result_event)?;
    let group_call_ids = tool_call_ids_from_assistant_tool_event(&assistant_tool_call_event);
    let now = now_iso();
    let target_id = assistant_message_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "追加工具结果失败：缺少 assistantMessageId".to_string())?;
    let target_idx = conversation
        .messages
        .iter()
        .rposition(|message| message.id.trim() == target_id)
        .ok_or_else(|| {
            format!("追加工具结果失败：目标 assistant message 不存在，assistantMessageId={target_id}")
        })?;
    let message = conversation
        .messages
        .get_mut(target_idx)
        .ok_or_else(|| {
            format!("追加工具结果失败：目标 assistant message 不存在，assistantMessageId={target_id}")
        })?;
    if message.role.trim() != "assistant" {
        return Err(format!(
            "追加工具结果失败：目标消息不是 assistant，assistantMessageId={}",
            message.id
        ));
    }
    let target_agent_id = message
        .speaker_agent_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_default();
    if target_agent_id != agent_id.trim() {
        return Err(format!(
            "追加工具结果失败：目标 assistant message 的 speaker_agent_id 不匹配，assistantMessageId={}，expectedAgentId={}，actualAgentId={}",
            message.id,
            agent_id.trim(),
            target_agent_id
        ));
    }
    let events = message.tool_call.get_or_insert_with(Vec::new);
    if tool_history_contains_tool_result_id(events, &tool_call_id) {
        merge_provider_meta_patch(&mut message.provider_meta, provider_meta_patch);
        return Ok(ToolGroupResultAppend {
            assistant_message_id: message.id.clone(),
            created: false,
            tool_event_count: events.len(),
        });
    }
    if !tool_history_contains_assistant_tool_group(events, &group_call_ids) {
        events.push(assistant_tool_call_event);
    }
    events.push(tool_result_event);
    message.created_at = if message.created_at.trim().is_empty() {
        now.clone()
    } else {
        message.created_at.clone()
    };
    merge_provider_meta_patch(&mut message.provider_meta, provider_meta_patch);
    let assistant_message_id = message.id.clone();
    let tool_event_count = events.len();
    conversation.updated_at = now.clone();
    conversation.last_assistant_at = Some(now);
    Ok(ToolGroupResultAppend {
        assistant_message_id,
        created: false,
        tool_event_count,
    })
}

fn merge_provider_meta_patch(target: &mut Option<Value>, patch: Option<Value>) {
    let Some(patch) = patch else {
        return;
    };
    let Some(patch_obj) = patch.as_object() else {
        return;
    };
    if patch_obj.is_empty() {
        return;
    }
    let mut current = target.take().unwrap_or_else(|| serde_json::json!({}));
    if !current.is_object() {
        current = serde_json::json!({
            "_raw_provider_meta": current,
        });
    }
    if let Some(current_obj) = current.as_object_mut() {
        for (key, value) in patch_obj {
            current_obj.insert(key.clone(), value.clone());
        }
    }
    *target = Some(current);
}

fn tool_call_ids_from_assistant_tool_event(event: &Value) -> Vec<String> {
    event
        .get("tool_calls")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| {
            item.get("id")
                .or_else(|| item.get("call_id"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn tool_history_contains_assistant_tool_group(events: &[Value], group_call_ids: &[String]) -> bool {
    if group_call_ids.is_empty() {
        return false;
    }
    events.iter().any(|event| {
        event
            .get("role")
            .and_then(Value::as_str)
            .is_some_and(|role| role.trim().eq_ignore_ascii_case("assistant"))
            && event
                .get("tool_calls")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .any(|call| {
                    call.get("id")
                        .or_else(|| call.get("call_id"))
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .is_some_and(|id| group_call_ids.iter().any(|candidate| candidate == id))
                })
    })
}

fn tool_history_contains_tool_result_id(events: &[Value], tool_call_id: &str) -> bool {
    events.iter().any(|event| {
        event
            .get("role")
            .and_then(Value::as_str)
            .is_some_and(|role| role.trim().eq_ignore_ascii_case("tool"))
            && event
            .get("tool_call_id")
            .and_then(Value::as_str)
            .map(str::trim)
            == Some(tool_call_id)
    })
}

fn validate_tool_group_result_append(
    assistant_tool_call_event: &Value,
    tool_result_event: &Value,
) -> Result<String, String> {
    let assistant_role = assistant_tool_call_event
        .get("role")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if assistant_role != "assistant" {
        return Err("追加工具结果失败：第一条事件必须是 assistant tool_call".to_string());
    }
    let group_call_ids = tool_call_ids_from_assistant_tool_event(assistant_tool_call_event);
    if group_call_ids.is_empty() {
        return Err("追加工具结果失败：assistant 事件缺少 tool_calls".to_string());
    }
    let tool_role = tool_result_event
        .get("role")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if tool_role != "tool" {
        return Err("追加工具结果失败：第二条事件必须是 tool result".to_string());
    }
    let result_call_id = tool_result_event
        .get("tool_call_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "追加工具结果失败：tool result 缺少 tool_call_id".to_string())?;
    if !group_call_ids.iter().any(|tool_call_id| tool_call_id == result_call_id) {
        return Err(format!(
            "追加工具结果失败：tool_call_id 不在工具组内，group_tool_call_ids={}，result_tool_call_id={}",
            group_call_ids.join(","),
            result_call_id
        ));
    }
    Ok(result_call_id.to_string())
}

fn ready_jsonl_snapshot_store(
    paths: &MessageStorePaths,
) -> Result<Option<JsonlSnapshotMessageStore>, String> {
    if let Some(index) = chat_metadata_store_read_index(paths)? {
        return Ok(Some(JsonlSnapshotMessageStore::with_sqlite_index(
            paths.messages_file.clone(),
            index,
        )));
    }
    let Some(manifest) = read_message_store_manifest(&paths.manifest_file)? else {
        return Ok(None);
    };
    if matches!(
        (manifest.message_store_kind, manifest.migration_state),
        (
            MessageStoreKind::JsonlSnapshot,
            MessageStoreMigrationState::Ready
        )
    ) {
        validate_ready_message_store_snapshot_integrity(paths, &manifest)?;
        return Ok(Some(JsonlSnapshotMessageStore::with_index(
            paths.messages_file.clone(),
            paths.index_file.clone(),
        )));
    }
    if matches!(
        (manifest.message_store_kind, manifest.migration_state),
        (
            MessageStoreKind::JsonlEventLog,
            MessageStoreMigrationState::Ready
        )
    ) {
        return Err(format!(
            "目录型会话暂不支持读取 JSONL 事件日志，path={}",
            paths.manifest_file.display()
        ));
    }
    Ok(None)
}

pub(super) fn validate_ready_message_store_snapshot_integrity(
    paths: &MessageStorePaths,
    manifest: &MessageStoreManifest,
) -> Result<(), String> {
    if !manifest.should_read_jsonl() {
        return Ok(());
    }
    let rebuilt = rebuild_ready_message_store_snapshot_from_blocks(paths)?;
    let meta = read_conversation_shard_meta(&paths.meta_file)
        .map_err(|err| format!("ready JSONL 会话缺少可读 meta.json，无法校验消息存储状态: {err}"))?;
    validate_conversation_shard_meta_id(paths, &meta)?;
    let current_index = read_message_store_index_file(&paths.index_file).ok();
    let index_matches = current_index
        .as_ref()
        .map(|index| index.persistent_view().items == rebuilt.index.persistent_view().items)
        .unwrap_or(false);
    let manifest_matches = manifest.source_message_count() == rebuilt.message_count
        && manifest.last_message_id().trim() == rebuilt.last_message_id
        && manifest.messages_jsonl_bytes() == rebuilt.total_bytes;
    if index_matches && manifest_matches {
        return Ok(());
    }
    Err(format!(
        "ready JSONL 快照与 blocks 不一致，conversation_id={}，index_matched={}，manifest_matched={}",
        paths.conversation_id, index_matches, manifest_matches
    ))
}

#[derive(Debug, Clone)]
struct RebuiltReadyMessageStoreSnapshot {
    index: MessageStoreIndexFile,
    total_bytes: u64,
    message_count: usize,
    last_message_id: String,
}

fn rebuild_ready_message_store_snapshot_from_blocks(
    paths: &MessageStorePaths,
) -> Result<RebuiltReadyMessageStoreSnapshot, String> {
    rebuild_message_store_snapshot_from_blocks(paths, false)
}

fn repair_and_rebuild_ready_message_store_snapshot_from_blocks(
    paths: &MessageStorePaths,
) -> Result<RebuiltReadyMessageStoreSnapshot, String> {
    rebuild_message_store_snapshot_from_blocks(paths, true)
}

fn rebuild_message_store_snapshot_from_blocks(
    paths: &MessageStorePaths,
    repair_invalid_lines: bool,
) -> Result<RebuiltReadyMessageStoreSnapshot, String> {
    let mut block_entries = fs::read_dir(&paths.blocks_dir)
        .map_err(|err| {
            format!(
                "读取会话块目录失败，conversation_id={}，path={}，error={err}",
                paths.conversation_id,
                paths.blocks_dir.display()
            )
        })?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let block_id = name
                .strip_suffix(".jsonl")
                .and_then(|value| value.parse::<u32>().ok())?;
            Some((block_id, path))
        })
        .collect::<Vec<_>>();
    block_entries.sort_by_key(|(block_id, _)| *block_id);

    let mut items = Vec::<MessageStoreIndexItem>::new();
    let mut total_bytes = 0_u64;
    let mut last_message_id = String::new();
    let mut seen_message_ids = std::collections::HashSet::<String>::new();

    for (block_id, block_path) in block_entries {
        let (report, discarded_count, duplicate_count) = if repair_invalid_lines {
            repair_jsonl_snapshot_file(&block_path, &mut seen_message_ids)?
        } else {
            (verify_jsonl_snapshot_file(&block_path, usize::MAX, "")?, 0, 0)
        };
        if discarded_count > 0 || duplicate_count > 0 {
            runtime_log_warn(format!(
                "[消息存储迁移] 完成，任务=清理block异常行，conversation_id={}，block_id={}，损坏行数={}，重复行数={}",
                paths.conversation_id, block_id, discarded_count, duplicate_count
            ));
        }
        if report.message_count == 0 {
            if repair_invalid_lines {
                fs::remove_file(&block_path).map_err(|err| {
                    format!(
                        "删除修复后空 block 失败，conversation_id={}，block_id={}，path={}，error={err}",
                        paths.conversation_id,
                        block_id,
                        block_path.display()
                    )
                })?;
                continue;
            }
            return Err(format!(
                "校验会话块失败，conversation_id={}，block_id={}，path={}，error=空 block 文件不允许作为 ready 快照真相",
                paths.conversation_id,
                block_id,
                block_path.display()
            ));
        }
        let block_len = fs::metadata(&block_path)
            .map_err(|err| {
                format!(
                    "读取会话块元数据失败，conversation_id={}，path={}，error={err}",
                    paths.conversation_id,
                    block_path.display()
                )
            })?
            .len();
        total_bytes = total_bytes.checked_add(block_len).ok_or_else(|| {
            format!(
                "统计会话块字节数失败：总字节数溢出，conversation_id={}，path={}",
                paths.conversation_id,
                block_path.display()
            )
        })?;
        last_message_id = report.last_message_id;
        items.extend(report.index.items.into_iter().map(|mut item| {
            item.block_id = Some(block_id);
            item
        }));
    }

    Ok(RebuiltReadyMessageStoreSnapshot {
        message_count: items.len(),
        last_message_id,
        total_bytes,
        index: MessageStoreIndexFile::new(MESSAGE_STORE_MANIFEST_VERSION, items),
    })
}

fn message_store_index_total_bytes(
    paths: &MessageStorePaths,
    index: &MessageStoreIndexFile,
) -> Result<u64, String> {
    let mut block_ids = std::collections::BTreeSet::<Option<u32>>::new();
    for item in &index.items {
        block_ids.insert(item.block_id);
    }
    let mut total = 0_u64;
    for block_id in block_ids {
        let path = jsonl_snapshot_index_item_path(&paths.messages_file, block_id)?;
        let len = fs::metadata(&path)
            .map_err(|err| {
                format!(
                    "读取会话块元数据失败，conversation_id={}，path={}，error={err}",
                    paths.conversation_id,
                    path.display()
                )
            })?
            .len();
        total = total.checked_add(len).ok_or_else(|| {
            format!(
                "统计会话块字节数失败：总字节数溢出，conversation_id={}，path={}",
                paths.conversation_id,
                path.display()
            )
        })?;
    }
    Ok(total)
}

fn read_message_store_directory_conversation(paths: &MessageStorePaths) -> Result<Conversation, String> {
    let manifest = read_message_store_manifest(&paths.manifest_file)?
        .ok_or_else(|| format!("目录型会话缺少 manifest，path={}", paths.manifest_file.display()))?;
    read_message_store_directory_conversation_with_manifest(paths, manifest)
}

fn read_message_store_directory_conversation_with_manifest(
    paths: &MessageStorePaths,
    manifest: MessageStoreManifest,
) -> Result<Conversation, String> {
    if !matches!(
        (manifest.message_store_kind, manifest.migration_state),
        (MessageStoreKind::JsonlSnapshot, MessageStoreMigrationState::Ready)
    ) {
        return Err(format!(
            "目录型会话 manifest 未处于可读取快照状态: kind={:?}, state={:?}",
            manifest.message_store_kind, manifest.migration_state
        ));
    }
    validate_ready_message_store_snapshot_integrity(paths, &manifest)?;
    let manifest = read_message_store_manifest(&paths.manifest_file)?
        .ok_or_else(|| format!("目录型会话缺少 manifest，path={}", paths.manifest_file.display()))?;
    let meta = read_conversation_shard_meta(&paths.meta_file)?;
    validate_conversation_shard_meta_id(paths, &meta)?;
    let messages = JsonlSnapshotMessageStore::with_index(
        paths.messages_file.clone(),
        paths.index_file.clone(),
    )
    .read_all_messages()?;
    if manifest.source_message_count != messages.len() {
        return Err(format!(
            "目录型会话消息数量不一致，conversation_id={}，manifest={}，actual={}",
            meta.id,
            manifest.source_message_count,
            messages.len()
        ));
    }
    let actual_last_message_id = messages
        .last()
        .map(|message| message.id.trim().to_string())
        .unwrap_or_default();
    if manifest.last_message_id.trim() != actual_last_message_id {
        return Err(format!(
            "目录型会话最后消息不一致，conversation_id={}，manifest={}，actual={}",
            meta.id, manifest.last_message_id, actual_last_message_id
        ));
    }
    Ok(meta.into_conversation(messages))
}

fn validate_conversation_shard_meta_id(
    paths: &MessageStorePaths,
    meta: &ConversationShardMeta,
) -> Result<(), String> {
    if meta.id.trim() != paths.conversation_id {
        return Err(format!(
            "目录型会话元数据 ID 不一致，expected={}，actual={}，path={}",
            paths.conversation_id,
            meta.id,
            paths.meta_file.display()
        ));
    }
    Ok(())
}

pub(super) fn delete_message_store_shard_artifacts(
    paths: &MessageStorePaths,
) -> Result<bool, String> {
    let delete_artifacts = || {
        let sqlite_deleted = if paths.is_v3_ready()? {
            chat_metadata_store_delete_conversation_unlocked(paths)?
        } else {
            false
        };
        if paths.shard_dir.exists() {
            validate_message_store_shard_dir_for_delete(paths)?;
        }
        let mut changed = sqlite_deleted;
        if paths.legacy_conversation_file.exists() {
            fs::remove_file(&paths.legacy_conversation_file).map_err(|err| {
                format!(
                    "删除旧会话分片失败，path={}，error={err}",
                    paths.legacy_conversation_file.display()
                )
            })?;
            changed = true;
        }
        if paths.shard_dir.exists() {
            fs::remove_dir_all(&paths.shard_dir).map_err(|err| {
                format!(
                    "删除目录型会话分片失败，path={}，error={err}",
                    paths.shard_dir.display()
                )
            })?;
            forget_message_store_index_cache(&paths.index_file);
            changed = true;
        }
        Ok(changed)
    };
    if paths.is_v3_ready()? {
        return chat_metadata_store_with_delete_gate(paths, delete_artifacts);
    }
    delete_artifacts()
}

#[cfg(test)]
pub(super) fn message_store_shard_write_signature(
    paths: &MessageStorePaths,
) -> Vec<(PathBuf, u64, Option<std::time::SystemTime>)> {
    let mut signatures = Vec::new();
    collect_message_store_shard_write_signatures(&paths.legacy_conversation_file, &mut signatures);
    collect_message_store_shard_write_signatures(&paths.shard_dir, &mut signatures);
    signatures.sort_by(|left, right| left.0.cmp(&right.0));
    signatures
}

#[cfg(test)]
fn collect_message_store_shard_write_signatures(
    path: &PathBuf,
    signatures: &mut Vec<(PathBuf, u64, Option<std::time::SystemTime>)>,
) {
    let Ok(metadata) = fs::metadata(path) else {
        return;
    };
    if metadata.is_file() {
        signatures.push((path.clone(), metadata.len(), metadata.modified().ok()));
        return;
    }
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        collect_message_store_shard_write_signatures(&entry.path(), signatures);
    }
}

fn validate_message_store_shard_dir_for_delete(paths: &MessageStorePaths) -> Result<(), String> {
    let conversations_dir = paths
        .legacy_conversation_file
        .parent()
        .ok_or_else(|| "删除目录型会话分片失败：旧会话文件缺少父目录".to_string())?;
    let shard_parent = paths
        .shard_dir
        .parent()
        .ok_or_else(|| "删除目录型会话分片失败：目录型分片缺少父目录".to_string())?;
    if shard_parent != conversations_dir {
        return Err(format!(
            "删除目录型会话分片失败：分片目录不在 conversations 目录内，shard={}，conversations={}",
            paths.shard_dir.display(),
            conversations_dir.display()
        ));
    }
    if paths.shard_dir == conversations_dir {
        return Err(format!(
            "删除目录型会话分片失败：拒绝删除 conversations 根目录，path={}",
            paths.shard_dir.display()
        ));
    }
    if paths.shard_dir.file_name().is_none() {
        return Err(format!(
            "删除目录型会话分片失败：分片目录名为空，path={}",
            paths.shard_dir.display()
        ));
    }
    Ok(())
}

fn read_jsonl_snapshot_messages_file(path: &PathBuf) -> Result<Vec<ChatMessage>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("读取 JSONL 消息文件失败，path={}，error={err}", path.display()))?;
    read_jsonl_snapshot_messages_from_content(&raw)
}

fn read_jsonl_snapshot_messages_from_content(content: &str) -> Result<Vec<ChatMessage>, String> {
    let report = verify_jsonl_snapshot_content(content, usize::MAX, "")?;
    let mut messages = Vec::with_capacity(report.message_count);
    for (line_no, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let message = decode_jsonl_snapshot_message(line)
            .map_err(|err| format!("解析 JSONL 消息失败，line={}，error={err}", line_no + 1))?;
        messages.push(message);
    }
    Ok(messages)
}

fn find_index_item_position(index: &MessageStoreIndexFile, message_id: &str) -> Option<usize> {
    let message_id = message_id.trim();
    if message_id.is_empty() {
        return None;
    }
    if let Some(position) = index.positions_by_message_id.get(message_id) {
        return Some(*position);
    }
    index
        .items
        .iter()
        .position(|item| item.message_id.trim() == message_id)
}

fn read_jsonl_block_messages_before(
    paths: &MessageStorePaths,
    index: &MessageStoreIndexFile,
    requested_block_id: Option<u32>,
    before_message_id: Option<&str>,
    limit: usize,
) -> Result<MessageStoreBlockMessagePage, String> {
    let before_message_id = before_message_id
        .map(str::trim)
        .filter(|message_id| !message_id.is_empty());
    let before_position = before_message_id
        .map(|message_id| {
            find_index_item_position(index, message_id)
                .ok_or_else(|| format!("Message not found: {message_id}"))
        })
        .transpose()?;
    let anchor_block_id = before_position
        .and_then(|position| index.items.get(position))
        .map(|item| item.block_id.unwrap_or(0));
    if let (Some(requested), Some(anchor)) = (requested_block_id, anchor_block_id) {
        if requested != anchor {
            return Err(format!(
                "block 反向读取锚点不属于目标块：conversation_id={}，block_id={}，anchor_block_id={}",
                paths.conversation_id, requested, anchor
            ));
        }
    }
    let selected_block_id = requested_block_id
        .or(anchor_block_id)
        .or_else(|| index.items.last().map(|item| item.block_id.unwrap_or(0)))
        .unwrap_or(0);
    let block_positions = index
        .items
        .iter()
        .enumerate()
        .filter_map(|(position, item)| {
            (item.block_id.unwrap_or(0) == selected_block_id).then_some(position)
        })
        .collect::<Vec<_>>();
    if block_positions.is_empty() {
        if requested_block_id.is_some() {
            return Err(format!(
                "会话块不存在，conversation_id={}，block_id={selected_block_id}",
                paths.conversation_id
            ));
        }
        return Ok(MessageStoreBlockMessagePage {
            selected_block_id,
            messages: Vec::new(),
            has_more: false,
        });
    }
    let end = before_position
        .map(|position| {
            block_positions
                .iter()
                .position(|candidate| *candidate == position)
                .ok_or_else(|| {
                    format!(
                        "block 反向读取锚点未进入目标块索引：conversation_id={}，block_id={selected_block_id}",
                        paths.conversation_id
                    )
                })
        })
        .transpose()?
        .unwrap_or(block_positions.len());
    let limit = normalized_message_limit(limit);
    let start = end.saturating_sub(limit);
    let selected_items = block_positions[start..end]
        .iter()
        .filter_map(|position| index.items.get(*position).cloned())
        .collect::<Vec<_>>();
    Ok(MessageStoreBlockMessagePage {
        selected_block_id,
        messages: read_jsonl_snapshot_messages_by_index_items(
            &paths.messages_file,
            &selected_items,
        )?,
        has_more: start > 0,
    })
}

fn count_jsonl_block_messages(
    index: &MessageStoreIndexFile,
    before_message_id: &str,
) -> Result<usize, String> {
    let before_message_id = before_message_id.trim();
    let anchor_position = find_index_item_position(index, before_message_id)
        .ok_or_else(|| format!("Message not found: {before_message_id}"))?;
    let selected_block_id = index
        .items
        .get(anchor_position)
        .and_then(|item| item.block_id)
        .unwrap_or(0);
    Ok(index.items
        .iter()
        .filter(|item| item.block_id.unwrap_or(0) == selected_block_id)
        .count())
}

fn read_jsonl_snapshot_messages_by_index_items(
    path: &PathBuf,
    items: &[MessageStoreIndexItem],
) -> Result<Vec<ChatMessage>, String> {
    let mut messages = Vec::with_capacity(items.len());
    let mut current_file_path = PathBuf::new();
    let mut current_file: Option<fs::File> = None;
    for item in items {
        let item_path = jsonl_snapshot_index_item_path(path, item.block_id)?;
        if current_file_path != item_path {
            current_file = Some(fs::File::open(&item_path).map_err(|err| {
                format!(
                    "打开 JSONL 消息文件失败，path={}，message_id={}，error={err}",
                    item_path.display(),
                    item.message_id
                )
            })?);
            current_file_path = item_path;
        }
        let Some(file) = current_file.as_mut() else {
            return Err(format!("打开 JSONL 消息文件失败，message_id={}", item.message_id));
        };
        std::io::Seek::seek(file, std::io::SeekFrom::Start(item.offset)).map_err(|err| {
            format!(
                "定位 JSONL 消息失败，path={}，message_id={}，offset={}，error={err}",
                current_file_path.display(),
                item.message_id,
                item.offset
            )
        })?;
        let mut buffer = vec![0_u8; item.byte_len as usize];
        std::io::Read::read_exact(file, &mut buffer).map_err(|err| {
            format!(
                "读取 JSONL 消息失败，path={}，message_id={}，offset={}，byte_len={}，error={err}",
                current_file_path.display(),
                item.message_id,
                item.offset,
                item.byte_len
            )
        })?;
        let raw = String::from_utf8(buffer)
            .map_err(|err| format!("JSONL 消息不是 UTF-8，message_id={}，error={err}", item.message_id))?;
        let line = raw.trim_end_matches(['\r', '\n']);
        let message = decode_jsonl_snapshot_message(line)
            .map_err(|err| format!("解析 JSONL 消息失败，message_id={}，error={err}", item.message_id))?;
        if message.id.trim() != item.message_id.trim() {
            return Err(format!(
                "JSONL 索引与消息不一致，path={}，expected_message_id={}，actual_message_id={}，offset={}，byte_len={}",
                current_file_path.display(),
                item.message_id,
                message.id,
                item.offset,
                item.byte_len
            ));
        }
        messages.push(message);
    }
    Ok(messages)
}

fn read_jsonl_snapshot_messages_by_index_items_cached(
    path: &PathBuf,
    items: &[MessageStoreIndexItem],
) -> Result<Vec<ChatMessage>, String> {
    let mut messages = Vec::with_capacity(items.len());
    let mut current_file_path = PathBuf::new();
    let mut current_file: Option<fs::File> = None;
    let mut current_modified_at: Option<std::time::SystemTime> = None;
    let mut current_len = 0_u64;
    let mut current_messages_by_id = std::collections::HashMap::<String, ChatMessage>::new();
    let mut current_cache_dirty = false;

    let flush_current_cache = |
        current_file_path: &PathBuf,
        current_modified_at: &Option<std::time::SystemTime>,
        current_len: u64,
        current_messages_by_id: &std::collections::HashMap<String, ChatMessage>,
        current_cache_dirty: bool,
    | {
        if !current_cache_dirty || current_file_path.as_os_str().is_empty() {
            return;
        }
        lock_message_store_block_file_cache().insert(
            current_file_path.clone(),
            CachedMessageStoreBlockFile {
                modified_at: *current_modified_at,
                len: current_len,
                messages_by_id: Arc::new(current_messages_by_id.clone()),
            },
        );
    };

    for item in items {
        let item_path = jsonl_snapshot_index_item_path(path, item.block_id)?;
        if current_file_path != item_path {
            flush_current_cache(
                &current_file_path,
                &current_modified_at,
                current_len,
                &current_messages_by_id,
                current_cache_dirty,
            );
            current_cache_dirty = false;
            current_file = None;
            current_messages_by_id.clear();

            let metadata = fs::metadata(&item_path).map_err(|err| {
                format!(
                    "读取会话块元数据失败，path={}，message_id={}，error={err}",
                    item_path.display(),
                    item.message_id
                )
            })?;
            current_modified_at = metadata.modified().ok();
            current_len = metadata.len();
            {
                let cache = lock_message_store_block_file_cache();
                if let Some(cached) = cache.get(&item_path) {
                    if cached.modified_at == current_modified_at && cached.len == current_len {
                        current_messages_by_id = (*cached.messages_by_id).clone();
                    }
                }
            }
            current_file_path = item_path;
        }

        if let Some(message) = current_messages_by_id.get(item.message_id.trim()) {
            messages.push(message.clone());
            continue;
        }

        if current_file.is_none() {
            current_file = Some(fs::File::open(&current_file_path).map_err(|err| {
                format!(
                    "打开 JSONL 消息文件失败，path={}，message_id={}，error={err}",
                    current_file_path.display(),
                    item.message_id
                )
            })?);
        }
        let Some(file) = current_file.as_mut() else {
            return Err(format!("打开 JSONL 消息文件失败，message_id={}", item.message_id));
        };
        std::io::Seek::seek(file, std::io::SeekFrom::Start(item.offset)).map_err(|err| {
            format!(
                "定位 JSONL 消息失败，path={}，message_id={}，offset={}，error={err}",
                current_file_path.display(),
                item.message_id,
                item.offset
            )
        })?;
        let mut buffer = vec![0_u8; item.byte_len as usize];
        std::io::Read::read_exact(file, &mut buffer).map_err(|err| {
            format!(
                "读取 JSONL 消息失败，path={}，message_id={}，offset={}，byte_len={}，error={err}",
                current_file_path.display(),
                item.message_id,
                item.offset,
                item.byte_len
            )
        })?;
        let raw = String::from_utf8(buffer)
            .map_err(|err| format!("JSONL 消息不是 UTF-8，message_id={}，error={err}", item.message_id))?;
        let line = raw.trim_end_matches(['\r', '\n']);
        let message = decode_jsonl_snapshot_message(line)
            .map_err(|err| format!("解析 JSONL 消息失败，message_id={}，error={err}", item.message_id))?;
        if message.id.trim() != item.message_id.trim() {
            return Err(format!(
                "JSONL 索引与消息不一致，path={}，expected_message_id={}，actual_message_id={}，offset={}，byte_len={}",
                current_file_path.display(),
                item.message_id,
                message.id,
                item.offset,
                item.byte_len
            ));
        }
        current_messages_by_id.insert(message.id.clone(), message.clone());
        current_cache_dirty = true;
        messages.push(message);
    }

    flush_current_cache(
        &current_file_path,
        &current_modified_at,
        current_len,
        &current_messages_by_id,
        current_cache_dirty,
    );
    Ok(messages)
}

fn build_message_store_block_summaries(
    path: &MessageStorePaths,
    index: &MessageStoreIndexFile,
) -> Result<Vec<MessageStoreBlockSummary>, String> {
    let block_ids = ordered_message_store_index_block_ids(index);
    let latest_block_id = block_ids.last().copied().unwrap_or(0);
    let mut summaries = Vec::<MessageStoreBlockSummary>::with_capacity(block_ids.len());
    for block_id in block_ids {
        let block_items = index
            .items
            .iter()
            .filter(|item| item.block_id.unwrap_or(0) == block_id)
            .cloned()
            .collect::<Vec<_>>();
        if block_items.is_empty() {
            continue;
        }
        let first_message = read_jsonl_snapshot_messages_by_index_items(
            &path.messages_file,
            &block_items[0..1],
        )?
        .into_iter()
        .next();
        let last_message = read_jsonl_snapshot_messages_by_index_items(
            &path.messages_file,
            &block_items[(block_items.len() - 1)..],
        )?
        .into_iter()
        .next();
        summaries.push(MessageStoreBlockSummary {
            block_id,
            message_count: block_items.len(),
            first_message_id: block_items
                .first()
                .map(|item| item.message_id.clone())
                .unwrap_or_default(),
            last_message_id: block_items
                .last()
                .map(|item| item.message_id.clone())
                .unwrap_or_default(),
            first_created_at: first_message.map(|message| message.created_at),
            last_created_at: last_message.map(|message| message.created_at),
            is_latest: block_id == latest_block_id,
        });
    }
    Ok(summaries)
}

fn jsonl_snapshot_index_item_path(base_messages_file: &PathBuf, block_id: Option<u32>) -> Result<PathBuf, String> {
    let Some(block_id) = block_id else {
        return Err(format!(
            "会话块路径解析失败：索引缺少 block_id，path={}",
            base_messages_file.display()
        ));
    };
    let Some(shard_dir) = base_messages_file.parent() else {
        return Err(format!(
            "会话块路径解析失败：messages 文件缺少父目录，path={}",
            base_messages_file.display()
        ));
    };
    Ok(shard_dir
        .join(MESSAGE_STORE_BLOCKS_DIR_NAME)
        .join(format!("{block_id:06}.jsonl")))
}

fn message_store_message_has_image(message: &ChatMessage) -> bool {
    message.parts.iter().any(|part| {
        matches!(part, MessagePart::Image { mime, .. } if !mime.trim().eq_ignore_ascii_case("application/pdf"))
            || matches!(part, MessagePart::Attachment { mime, .. } if message_attachment_kind(mime) == "image")
    })
}

fn message_store_message_has_pdf(message: &ChatMessage) -> bool {
    message.parts.iter().any(|part| {
        matches!(part, MessagePart::Image { mime, .. } if mime.trim().eq_ignore_ascii_case("application/pdf"))
            || matches!(part, MessagePart::Attachment { mime, .. } if message_attachment_kind(mime) == "pdf")
    })
}

fn message_store_message_has_audio(message: &ChatMessage) -> bool {
    message
        .parts
        .iter()
        .any(|part| {
            matches!(part, MessagePart::Audio { .. })
                || matches!(part, MessagePart::Attachment { mime, .. } if message_attachment_kind(mime) == "audio")
        })
}

fn read_messages_before_from_index(
    path: &PathBuf,
    index: &MessageStoreIndexFile,
    before_message_id: &str,
    limit: usize,
) -> Result<MessageStoreLimitPage, String> {
    let before_idx = find_index_item_position(index, before_message_id)
        .ok_or_else(|| format!("Message not found: {}", before_message_id.trim()))?;
    let limit = normalized_message_limit(limit);
    let start = before_idx.saturating_sub(limit);
    Ok(MessageStoreLimitPage {
        messages: read_jsonl_snapshot_messages_by_index_items(path, &index.items[start..before_idx])?,
        has_more: start > 0,
    })
}

fn read_messages_after_from_index(
    path: &PathBuf,
    index: &MessageStoreIndexFile,
    after_message_id: &str,
    limit: usize,
) -> Result<MessageStoreLimitPage, String> {
    let after_idx = find_index_item_position(index, after_message_id)
        .ok_or_else(|| format!("Message not found: {}", after_message_id.trim()))?;
    let limit = normalized_message_limit(limit);
    let start = after_idx.saturating_add(1);
    let end = (start + limit).min(index.items.len());
    Ok(MessageStoreLimitPage {
        messages: read_jsonl_snapshot_messages_by_index_items(path, &index.items[start..end])?,
        has_more: end < index.items.len(),
    })
}

fn read_messages_after_all_from_index(
    path: &PathBuf,
    index: &MessageStoreIndexFile,
    after_message_id: &str,
) -> Result<Vec<ChatMessage>, String> {
    let after_idx = find_index_item_position(index, after_message_id)
        .ok_or_else(|| format!("Message not found: {}", after_message_id.trim()))?;
    let start = after_idx.saturating_add(1);
    read_jsonl_snapshot_messages_by_index_items(path, &index.items[start..])
}

fn compaction_boundary_index_items(index: &MessageStoreIndexFile) -> Vec<usize> {
    if !index.positions_by_message_id.is_empty() || index.items.is_empty() {
        return index.compaction_boundary_positions.clone();
    }
    index
        .items
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| {
            if item.compaction_kind.is_some() {
                Some(idx)
            } else {
                None
            }
        })
        .collect()
}

fn build_indexed_compaction_segment(
    path: &PathBuf,
    index: &MessageStoreIndexFile,
    start: usize,
    end: usize,
    previous_boundary_index: Option<usize>,
) -> Result<MessageStoreCompactionSegment, String> {
    let boundary_message_id = index
        .items
        .get(start)
        .filter(|_| is_index_compaction_boundary_position(index, start))
        .map(|item| item.message_id.trim().to_string());
    let previous_boundary_message_id = previous_boundary_index
        .and_then(|idx| index.items.get(idx))
        .map(|item| item.message_id.trim().to_string());
    Ok(MessageStoreCompactionSegment {
        messages: read_jsonl_snapshot_messages_by_index_items(path, &index.items[start..end])?,
        boundary_message_id,
        previous_boundary_message_id,
        has_previous_segment: start > 0,
    })
}

fn is_index_compaction_boundary_position(index: &MessageStoreIndexFile, idx: usize) -> bool {
    index.compaction_boundary_positions.contains(&idx)
        || index
            .items
            .get(idx)
            .is_some_and(|item| item.compaction_kind.is_some())
}

fn read_current_compaction_segment_from_index(
    path: &PathBuf,
    index: &MessageStoreIndexFile,
) -> Result<MessageStoreCompactionSegment, String> {
    if index.items.is_empty() {
        return Ok(MessageStoreCompactionSegment {
            messages: Vec::new(),
            boundary_message_id: None,
            previous_boundary_message_id: None,
            has_previous_segment: false,
        });
    }
    let boundaries = compaction_boundary_index_items(index);
    let start = boundaries.last().copied().unwrap_or(0);
    let previous_boundary_index = boundaries.iter().rev().nth(1).copied();
    build_indexed_compaction_segment(path, index, start, index.items.len(), previous_boundary_index)
}

fn read_compaction_segment_before_from_index(
    path: &PathBuf,
    index: &MessageStoreIndexFile,
    boundary_message_id: &str,
) -> Result<MessageStoreCompactionSegment, String> {
    let boundary_idx = find_index_item_position(index, boundary_message_id)
        .ok_or_else(|| format!("Compaction boundary not found: {}", boundary_message_id.trim()))?;
    let boundaries = compaction_boundary_index_items(index);
    let Some(boundary_pos) = boundaries.iter().position(|idx| *idx == boundary_idx) else {
        return Err(format!("Compaction boundary not indexed: {}", boundary_message_id.trim()));
    };
    let start = if boundary_pos == 0 {
        0
    } else {
        boundaries[boundary_pos - 1]
    };
    let previous_boundary_index = if boundary_pos >= 2 {
        Some(boundaries[boundary_pos - 2])
    } else {
        None
    };
    build_indexed_compaction_segment(path, index, start, boundary_idx, previous_boundary_index)
}

fn normalized_message_limit(limit: usize) -> usize {
    limit.clamp(1, 100)
}

fn find_message_index(messages: &[ChatMessage], message_id: &str) -> Option<usize> {
    let message_id = message_id.trim();
    if message_id.is_empty() {
        return None;
    }
    messages
        .iter()
        .position(|message| message.id.trim() == message_id)
}

fn read_recent_messages_from_index(
    path: &PathBuf,
    index: &MessageStoreIndexFile,
    limit: usize,
) -> Result<Vec<ChatMessage>, String> {
    let limit = normalized_message_limit(limit);
    let start = index.items.len().saturating_sub(limit);
    read_jsonl_snapshot_messages_by_index_items(path, &index.items[start..])
}

fn read_recent_messages_page_from_index(
    path: &PathBuf,
    index: &MessageStoreIndexFile,
    limit: usize,
) -> Result<MessageStoreLimitPage, String> {
    let limit = normalized_message_limit(limit);
    let start = index.items.len().saturating_sub(limit);
    let messages = read_jsonl_snapshot_messages_by_index_items(path, &index.items[start..])?;
    Ok(MessageStoreLimitPage {
        messages,
        has_more: start > 0,
    })
}

fn read_message_by_id_from_index(
    path: &PathBuf,
    index: &MessageStoreIndexFile,
    message_id: &str,
) -> Result<ChatMessage, String> {
    let idx = find_index_item_position(index, message_id)
        .ok_or_else(|| format!("Message not found: {}", message_id.trim()))?;
    let mut messages = read_jsonl_snapshot_messages_by_index_items(path, &index.items[idx..=idx])?;
    messages
        .pop()
        .ok_or_else(|| format!("Message not found: {}", message_id.trim()))
}

fn read_recent_messages_from_slice(
    messages: &[ChatMessage],
    limit: usize,
) -> Result<Vec<ChatMessage>, String> {
    let limit = normalized_message_limit(limit);
    let start = messages.len().saturating_sub(limit);
    Ok(messages[start..].to_vec())
}

fn read_recent_messages_page_from_slice(
    messages: &[ChatMessage],
    limit: usize,
) -> Result<MessageStoreLimitPage, String> {
    let limit = normalized_message_limit(limit);
    let start = messages.len().saturating_sub(limit);
    Ok(MessageStoreLimitPage {
        messages: messages[start..].to_vec(),
        has_more: start > 0,
    })
}

fn read_message_by_id_from_slice(
    messages: &[ChatMessage],
    message_id: &str,
) -> Result<ChatMessage, String> {
    let message_id = message_id.trim();
    if message_id.is_empty() {
        return Err("messageId is required.".to_string());
    }
    messages
        .iter()
        .find(|item| item.id.trim() == message_id)
        .cloned()
        .ok_or_else(|| format!("Message not found: {message_id}"))
}

fn read_messages_before_from_slice(
    messages: &[ChatMessage],
    before_message_id: &str,
    limit: usize,
) -> Result<MessageStoreLimitPage, String> {
    let before_idx = find_message_index(messages, before_message_id)
        .ok_or_else(|| format!("Message not found: {}", before_message_id.trim()))?;
    let limit = normalized_message_limit(limit);
    let start = before_idx.saturating_sub(limit);
    Ok(MessageStoreLimitPage {
        messages: messages[start..before_idx].to_vec(),
        has_more: start > 0,
    })
}

fn read_messages_after_from_slice(
    messages: &[ChatMessage],
    after_message_id: &str,
    limit: usize,
) -> Result<MessageStoreLimitPage, String> {
    let after_idx = find_message_index(messages, after_message_id)
        .ok_or_else(|| format!("Message not found: {}", after_message_id.trim()))?;
    let limit = normalized_message_limit(limit);
    let start = after_idx.saturating_add(1);
    let end = (start + limit).min(messages.len());
    Ok(MessageStoreLimitPage {
        messages: messages[start..end].to_vec(),
        has_more: end < messages.len(),
    })
}

fn read_messages_after_all_from_slice(
    messages: &[ChatMessage],
    after_message_id: &str,
) -> Result<Vec<ChatMessage>, String> {
    let after_idx = find_message_index(messages, after_message_id)
        .ok_or_else(|| format!("Message not found: {}", after_message_id.trim()))?;
    Ok(messages[(after_idx + 1)..].to_vec())
}

fn compaction_boundary_indexes(messages: &[ChatMessage]) -> Vec<usize> {
    messages
        .iter()
        .enumerate()
        .filter_map(|(idx, message)| {
            if message_store_compaction_kind(message).is_some() {
                Some(idx)
            } else {
                None
            }
        })
        .collect()
}

fn build_compaction_segment(
    messages: &[ChatMessage],
    start: usize,
    end: usize,
    previous_boundary_index: Option<usize>,
) -> MessageStoreCompactionSegment {
    let boundary_message_id = messages
        .get(start)
        .filter(|message| message_store_compaction_kind(message).is_some())
        .map(|message| message.id.trim().to_string());
    let previous_boundary_message_id = previous_boundary_index
        .and_then(|idx| messages.get(idx))
        .map(|message| message.id.trim().to_string());
    MessageStoreCompactionSegment {
        messages: messages[start..end].to_vec(),
        boundary_message_id,
        previous_boundary_message_id,
        has_previous_segment: start > 0,
    }
}

fn read_current_compaction_segment_from_slice(messages: &[ChatMessage]) -> Result<MessageStoreCompactionSegment, String> {
    if messages.is_empty() {
        return Ok(MessageStoreCompactionSegment {
            messages: Vec::new(),
            boundary_message_id: None,
            previous_boundary_message_id: None,
            has_previous_segment: false,
        });
    }
    let boundaries = compaction_boundary_indexes(messages);
    let start = boundaries.last().copied().unwrap_or(0);
    let previous_boundary_index = boundaries
        .iter()
        .rev()
        .nth(1)
        .copied();
    Ok(build_compaction_segment(
        messages,
        start,
        messages.len(),
        previous_boundary_index,
    ))
}

fn read_compaction_segment_before_from_slice(
    messages: &[ChatMessage],
    boundary_message_id: &str,
) -> Result<MessageStoreCompactionSegment, String> {
    let boundary_idx = find_message_index(messages, boundary_message_id)
        .ok_or_else(|| format!("Compaction boundary not found: {}", boundary_message_id.trim()))?;
    if message_store_compaction_kind(&messages[boundary_idx]).is_none() {
        return Err(format!(
            "Message is not a compaction boundary: {}",
            boundary_message_id.trim()
        ));
    }
    let boundaries = compaction_boundary_indexes(messages);
    let Some(boundary_pos) = boundaries.iter().position(|idx| *idx == boundary_idx) else {
        return Err(format!("Compaction boundary not indexed: {}", boundary_message_id.trim()));
    };
    let start = if boundary_pos == 0 {
        0
    } else {
        boundaries[boundary_pos - 1]
    };
    let previous_boundary_index = if boundary_pos >= 2 {
        Some(boundaries[boundary_pos - 2])
    } else {
        None
    };
    Ok(build_compaction_segment(
        messages,
        start,
        boundary_idx,
        previous_boundary_index,
    ))
}

#[cfg(test)]
mod message_store_reader_tests {
    use super::*;

    fn test_message(id: &str, role: &str) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            role: role.to_string(),
            created_at: format!("2026-04-24T00:00:0{}Z", id.len()),
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

    fn test_compaction_message(id: &str, kind: &str) -> ChatMessage {
        let mut message = test_message(id, "assistant");
        message.provider_meta = Some(serde_json::json!({
            "messageMeta": {
                "kind": kind
            }
        }));
        message
    }

    fn test_single_tool_group_result(call_id: &str, tool_name: &str) -> (Value, Value) {
        (
            serde_json::json!({
                "role": "assistant",
                "content": Value::Null,
                "tool_calls": [{
                    "id": call_id,
                    "type": "function",
                    "function": {
                        "name": tool_name,
                        "arguments": "{}"
                    }
                }]
            }),
            serde_json::json!({
                "role": "tool",
                "tool_call_id": call_id,
                "content": "{\"ok\":true}"
            }),
        )
    }

    fn test_tool_group_with_two_results() -> (Value, Value, Value) {
        (
            serde_json::json!({
                "role": "assistant",
                "content": Value::Null,
                "reasoning_content": "先同时读取两个文件",
                "tool_calls": [
                    {
                        "id": "call-a",
                        "call_id": "call-a",
                        "type": "function",
                        "function": {
                            "name": "read",
                            "arguments": "{\"path\":\"a.rs\"}"
                        }
                    },
                    {
                        "id": "call-b",
                        "call_id": "call-b",
                        "type": "function",
                        "function": {
                            "name": "read",
                            "arguments": "{\"path\":\"b.rs\"}"
                        }
                    }
                ]
            }),
            serde_json::json!({
                "role": "tool",
                "tool_call_id": "call-a",
                "content": "工具 A 结果"
            }),
            serde_json::json!({
                "role": "tool",
                "tool_call_id": "call-b",
                "content": "工具 B 结果"
            }),
        )
    }

    fn write_test_messages(messages: &[ChatMessage]) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-reader-{}",
            Uuid::new_v4()
        ));
        let messages_file = root.join("messages.jsonl");
        let content = encode_jsonl_snapshot_messages(messages).expect("encode messages");
        write_jsonl_snapshot_atomic(&messages_file, &content).expect("write messages");
        (root, messages_file)
    }

    fn write_test_messages_with_index(messages: &[ChatMessage]) -> (PathBuf, PathBuf, PathBuf) {
        let (root, messages_file) = write_test_messages(messages);
        let index_file = root.join("messages.idx.json");
        let index = rebuild_jsonl_snapshot_index_from_file(&messages_file).expect("rebuild index");
        write_message_store_index_atomic(&index_file, &index).expect("write index");
        (root, messages_file, index_file)
    }

    fn write_test_blocks_with_index(messages: &[ChatMessage]) -> (PathBuf, PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-reader-blocks-{}",
            Uuid::new_v4()
        ));
        let shard_dir = root.join("conversation-reader");
        let messages_file = shard_dir.join("messages.jsonl");
        let blocks = build_jsonl_snapshot_conversation_blocks(messages).expect("build blocks");
        fs::create_dir_all(shard_dir.join(MESSAGE_STORE_BLOCKS_DIR_NAME)).expect("create blocks dir");
        for block in &blocks.blocks {
            let block_path = shard_dir.join(&block.block_file);
            write_jsonl_snapshot_atomic(&block_path, &block.content).expect("write block");
        }
        let index_file = shard_dir.join("messages.idx.json");
        write_message_store_index_atomic(&index_file, &blocks.index).expect("write index");
        (root, messages_file, index_file)
    }

    fn test_conversation(messages: Vec<ChatMessage>) -> Conversation {
        Conversation {
            id: "conversation-reader".to_string(),
            title: "reader".to_string(),
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
            messages,
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

    #[test]
    fn append_tool_group_result_should_merge_into_target_assistant_by_id_even_if_not_tail() {
        let mut assistant = test_message("assistant-1", "assistant");
        assistant.speaker_agent_id = Some("agent-a".to_string());
        let mut conversation = test_conversation(vec![
            assistant,
            test_message("user-2", "user"),
        ]);
        let (call_event, result_event) = test_single_tool_group_result("call-by-id", "read_file");

        let append = append_tool_group_result_to_conversation(
            &mut conversation,
            "agent-a",
            call_event,
            result_event,
            None,
            Some("assistant-1"),
        )
        .expect("append group result by id");

        assert!(!append.created);
        assert_eq!(append.assistant_message_id, "assistant-1");
        assert_eq!(conversation.messages.len(), 2);
        assert_eq!(
            conversation.messages[0]
                .tool_call
                .as_ref()
                .map(Vec::len),
            Some(2)
        );
    }

    #[test]
    fn append_tool_group_result_should_merge_into_last_same_assistant() {
        let mut assistant = test_message("assistant-1", "assistant");
        assistant.speaker_agent_id = Some("agent-a".to_string());
        let mut conversation = test_conversation(vec![
            test_message("user-1", "user"),
            assistant,
        ]);
        let (call_event, result_event) = test_single_tool_group_result("call-1", "read_file");

        let append = append_tool_group_result_to_conversation(
            &mut conversation,
            "agent-a",
            call_event,
            result_event,
            None,
            Some("assistant-1"),
        )
        .expect("append group result");

        assert!(!append.created);
        assert_eq!(append.assistant_message_id, "assistant-1");
        assert_eq!(append.tool_event_count, 2);
        assert_eq!(conversation.messages.len(), 2);
        assert_eq!(
            conversation.messages[1]
                .tool_call
                .as_ref()
                .map(Vec::len),
            Some(2)
        );
    }

    #[test]
    fn append_tool_group_result_should_reject_missing_assistant_message_id() {
        let mut assistant = test_message("assistant-1", "assistant");
        assistant.speaker_agent_id = Some("agent-a".to_string());
        let mut conversation = test_conversation(vec![assistant]);
        let (call_event, result_event) = test_single_tool_group_result("call-1", "read_file");

        let err = append_tool_group_result_to_conversation(
            &mut conversation,
            "agent-a",
            call_event,
            result_event,
            None,
            None,
        )
        .expect_err("missing assistant message id should fail");

        assert!(err.contains("缺少 assistantMessageId"));
        assert_eq!(conversation.messages.len(), 1);
    }

    #[test]
    fn append_tool_group_result_should_reject_missing_target_assistant() {
        let mut conversation = test_conversation(vec![test_message("user-1", "user")]);
        let (call_event, result_event) = test_single_tool_group_result("call-1", "read_file");

        let err = append_tool_group_result_to_conversation(
            &mut conversation,
            "agent-a",
            call_event,
            result_event,
            None,
            Some("assistant-missing"),
        )
        .expect_err("missing target assistant should fail");

        assert!(err.contains("目标 assistant message 不存在"));
        assert_eq!(conversation.messages.len(), 1);
    }

    #[test]
    fn append_tool_group_result_should_reject_mismatched_result() {
        let mut conversation = test_conversation(vec![test_message("user-1", "user")]);
        let (call_event, mut result_event) = test_single_tool_group_result("call-1", "read_file");
        result_event["tool_call_id"] = Value::String("call-2".to_string());

        let err = append_tool_group_result_to_conversation(
            &mut conversation,
            "agent-a",
            call_event,
            result_event,
            None,
            Some("assistant-1"),
        )
        .expect_err("mismatch should fail");

        assert!(err.contains("tool_call_id 不在工具组内"));
    }

    #[test]
    fn append_tool_group_result_should_skip_existing_tool_call_id() {
        let mut assistant = test_message("assistant-1", "assistant");
        assistant.speaker_agent_id = Some("agent-a".to_string());
        let (call_event, result_event) = test_single_tool_group_result("call-1", "read_file");
        assistant.tool_call = Some(vec![call_event.clone(), result_event.clone()]);
        let mut conversation = test_conversation(vec![assistant]);

        let append = append_tool_group_result_to_conversation(
            &mut conversation,
            "agent-a",
            call_event,
            result_event,
            Some(serde_json::json!({
                "providerPromptTokens": 123_u64,
                "effectivePromptTokens": 123_u64,
                "contextUsageRatio": 0.5,
            })),
            Some("assistant-1"),
        )
        .expect("append group result");

        assert!(!append.created);
        assert_eq!(append.tool_event_count, 2);
        assert_eq!(
            conversation.messages[0]
                .tool_call
                .as_ref()
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(
            conversation.messages[0]
                .provider_meta
                .as_ref()
                .and_then(|meta| meta.get("providerPromptTokens"))
                .and_then(Value::as_u64),
            Some(123)
        );
    }

    #[test]
    fn append_tool_group_result_should_keep_tool_group_reasoning_once() {
        let mut assistant = test_message("assistant-1", "assistant");
        assistant.speaker_agent_id = Some("agent-a".to_string());
        let mut conversation = test_conversation(vec![assistant]);
        let (group_event, result_a, result_b) = test_tool_group_with_two_results();

        let first = append_tool_group_result_to_conversation(
            &mut conversation,
            "agent-a",
            group_event.clone(),
            result_a,
            None,
            Some("assistant-1"),
        )
        .expect("append first group result");
        let second = append_tool_group_result_to_conversation(
            &mut conversation,
            "agent-a",
            group_event,
            result_b,
            None,
            Some("assistant-1"),
        )
        .expect("append second group result");

        assert!(!first.created);
        assert!(!second.created);
        assert_eq!(conversation.messages.len(), 1);
        let events = conversation.messages[0].tool_call.as_ref().expect("tool history");
        assert_eq!(events.len(), 3);
        assert_eq!(events[0]["role"].as_str(), Some("assistant"));
        assert_eq!(events[0]["reasoning_content"].as_str(), Some("先同时读取两个文件"));
        assert_eq!(
            events[0]["tool_calls"].as_array().map(Vec::len),
            Some(2)
        );
        assert_eq!(events[1]["tool_call_id"].as_str(), Some("call-a"));
        assert_eq!(events[2]["tool_call_id"].as_str(), Some("call-b"));
        let reasoning_event_count = events
            .iter()
            .filter(|event| event.get("reasoning_content").and_then(Value::as_str).is_some())
            .count();
        assert_eq!(reasoning_event_count, 1);
    }

    #[test]
    fn message_store_jsonl_reader_should_match_before_after_limit_semantics() {
        let messages = vec![
            test_message("m1", "user"),
            test_message("m2", "assistant"),
            test_message("m3", "user"),
            test_message("m4", "assistant"),
        ];
        let (root, messages_file) = write_test_messages(&messages);
        let store = JsonlSnapshotMessageStore::new(messages_file);

        let before = store.read_messages_before("m4", 2).expect("before page");
        let after = store.read_messages_after("m1", 2).expect("after page");

        assert_eq!(
            before.messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            vec!["m2", "m3"]
        );
        assert!(before.has_more);
        assert_eq!(
            after.messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            vec!["m2", "m3"]
        );
        assert!(after.has_more);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_jsonl_reader_should_match_compaction_segment_semantics() {
        let messages = vec![
            test_message("m1", "user"),
            test_compaction_message("c1", "context_compaction"),
            test_message("m2", "assistant"),
            test_compaction_message("c2", "summary_context_seed"),
            test_message("m3", "assistant"),
        ];
        let (root, messages_file) = write_test_messages(&messages);
        let store = JsonlSnapshotMessageStore::new(messages_file);

        let current = store.read_current_compaction_segment().expect("current segment");
        let previous = store
            .read_compaction_segment_before("c2")
            .expect("previous segment");

        assert_eq!(
            current.messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            vec!["c2", "m3"]
        );
        assert_eq!(current.boundary_message_id.as_deref(), Some("c2"));
        assert_eq!(current.previous_boundary_message_id.as_deref(), Some("c1"));
        assert_eq!(
            previous.messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            vec!["c1", "m2"]
        );
        assert_eq!(previous.boundary_message_id.as_deref(), Some("c1"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_jsonl_indexed_reader_should_only_decode_requested_page() {
        let messages = vec![
            test_message("m1", "user"),
            test_message("m2", "assistant"),
            test_message("m3", "user"),
            test_message("m4", "assistant"),
            test_message("m5", "user"),
        ];
        let (root, messages_file, index_file) = write_test_blocks_with_index(&messages);
        let store = JsonlSnapshotMessageStore::with_index(messages_file, index_file);

        let before = store.read_messages_before("m5", 2).expect("indexed before page");
        let after = store.read_messages_after("m2", 2).expect("indexed after page");

        assert_eq!(
            before.messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            vec!["m3", "m4"]
        );
        assert!(before.has_more);
        assert_eq!(
            after.messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            vec!["m3", "m4"]
        );
        assert!(after.has_more);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_jsonl_indexed_reader_should_read_recent_and_single_message() {
        let messages = vec![
            test_message("m1", "user"),
            test_message("m2", "assistant"),
            test_message("m3", "user"),
            test_message("m4", "assistant"),
        ];
        let (root, messages_file, index_file) = write_test_blocks_with_index(&messages);
        let store = JsonlSnapshotMessageStore::with_index(messages_file, index_file);

        let recent = store.read_recent_messages(2).expect("recent messages");
        let message = store.read_message_by_id("m2").expect("message by id");
        let after_all = store.read_messages_after_all("m2").expect("after all");

        assert_eq!(
            recent.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            vec!["m3", "m4"]
        );
        assert_eq!(message.id, "m2");
        assert_eq!(
            after_all.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            vec!["m3", "m4"]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_jsonl_indexed_reader_should_read_recent_page_with_has_more() {
        let messages = vec![
            test_message("m1", "user"),
            test_message("m2", "assistant"),
            test_message("m3", "user"),
            test_message("m4", "assistant"),
        ];
        let (root, messages_file, index_file) = write_test_blocks_with_index(&messages);
        let store = JsonlSnapshotMessageStore::with_index(messages_file, index_file);

        let page = store
            .read_recent_messages_page(2)
            .expect("recent messages page");

        assert_eq!(
            page.messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            vec!["m3", "m4"]
        );
        assert!(page.has_more);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_jsonl_indexed_reader_should_page_compaction_segments() {
        let messages = vec![
            test_message("m1", "user"),
            test_compaction_message("c1", "context_compaction"),
            test_message("m2", "assistant"),
            test_compaction_message("c2", "summary_context_seed"),
            test_message("m3", "assistant"),
        ];
        let (root, messages_file, index_file) = write_test_blocks_with_index(&messages);
        let store = JsonlSnapshotMessageStore::with_index(messages_file, index_file);

        let current = store.read_current_compaction_segment().expect("indexed current segment");
        let previous = store
            .read_compaction_segment_before("c2")
            .expect("indexed previous segment");

        assert_eq!(
            current.messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            vec!["c2", "m3"]
        );
        assert_eq!(current.previous_boundary_message_id.as_deref(), Some("c1"));
        assert_eq!(
            previous.messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            vec!["c1", "m2"]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_jsonl_block_reader_should_stop_at_block_boundary() {
        let messages = vec![
            test_message("old", "user"),
            test_compaction_message("c1", "context_compaction"),
            test_message("current-1", "user"),
            test_message("current-2", "assistant"),
        ];
        let (root, messages_file, index_file) = write_test_blocks_with_index(&messages);
        let store = JsonlSnapshotMessageStore::with_index(messages_file.clone(), index_file);
        let index = store.index().expect("read index").expect("index");
        let paths = message_store_paths_for_shard_dir(
            &root,
            "conversation-reader",
            messages_file.parent().expect("shard dir").to_path_buf(),
            root.join("conversation.json"),
        )
        .expect("message store paths");

        let latest = read_jsonl_block_messages_before(&paths, &index, None, None, 10)
            .expect("read latest block");
        assert_eq!(
            latest
                .messages
                .iter()
                .map(|message| message.id.as_str())
                .collect::<Vec<_>>(),
            vec!["c1", "current-1", "current-2"]
        );
        assert!(!latest.has_more);

        let before = read_jsonl_block_messages_before(
            &paths,
            &index,
            None,
            Some("current-2"),
            10,
        )
        .expect("read block before anchor");
        assert_eq!(
            before
                .messages
                .iter()
                .map(|message| message.id.as_str())
                .collect::<Vec<_>>(),
            vec!["c1", "current-1"]
        );
        assert!(!before.has_more);

        let block_message_count = count_jsonl_block_messages(&index, "current-2")
            .expect("count current block messages");
        assert_eq!(block_message_count, 3);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_jsonl_indexed_reader_should_reject_unsupported_index_version() {
        let messages = vec![test_message("m1", "user"), test_message("m2", "assistant")];
        let (root, messages_file, index_file) = write_test_blocks_with_index(&messages);
        let mut index = (*read_message_store_index_file(&index_file).expect("read index")).clone();
        index.version = MESSAGE_STORE_MANIFEST_VERSION + 1;
        write_message_store_index_atomic(&index_file, &index).expect("write future index");
        let store = JsonlSnapshotMessageStore::with_index(messages_file, index_file);

        let err = store
            .read_recent_messages(1)
            .expect_err("future index version should fail");

        assert!(err.contains("消息索引版本不支持"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_jsonl_indexed_reader_should_reject_stale_message_id() {
        let messages = vec![test_message("m1", "user"), test_message("m2", "assistant")];
        let (root, messages_file, index_file) = write_test_blocks_with_index(&messages);
        let mut index = (*read_message_store_index_file(&index_file).expect("read index")).clone();
        index.items[1].message_id = "wrong-m2".to_string();
        write_message_store_index_atomic(&index_file, &index).expect("write stale index");
        let store = JsonlSnapshotMessageStore::with_index(messages_file, index_file);

        let err = store
            .read_message_by_id("wrong-m2")
            .expect_err("stale message id should fail");

        assert!(err.contains("JSONL 索引与消息不一致"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_jsonl_indexed_reader_should_reject_invalid_index_shape() {
        let messages = vec![test_message("m1", "user"), test_message("m2", "assistant")];
        let (root, _messages_file, index_file) = write_test_blocks_with_index(&messages);
        let mut index = (*read_message_store_index_file(&index_file).expect("read index")).clone();
        index.items[1].message_id = index.items[0].message_id.clone();
        write_message_store_index_atomic(&index_file, &index).expect("write duplicate index");

        let err = read_message_store_index_file(&index_file)
            .expect_err("duplicate index message id should fail");

        assert!(err.contains("消息索引包含重复消息 ID"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_backend_should_not_read_stale_jsonl_without_ready_manifest() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-backend-stale-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![test_message("old1", "user")]);
        let stale_messages = vec![test_message("jsonl1", "assistant")];
        let stale_content = encode_jsonl_snapshot_messages(&stale_messages).expect("encode stale");
        write_jsonl_snapshot_atomic(&paths.messages_file, &stale_content).expect("write stale");
        let building = MessageStoreManifest::jsonl_snapshot_building(&conversation);
        write_message_store_manifest_atomic(&paths.manifest_file, &building)
            .expect("write manifest");

        let store =
            message_store_backend_for_conversation(&paths, &conversation).expect("select store");
        let messages = store.read_all_messages().expect("read messages");

        assert_eq!(
            messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            vec!["old1"]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_backend_should_read_jsonl_only_when_manifest_ready() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-backend-ready-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![test_message("old1", "user")]);
        let jsonl_conversation = test_conversation(vec![
            test_message("jsonl1", "assistant"),
            test_message("jsonl2", "user"),
        ]);
        run_jsonl_snapshot_migration(&paths, &jsonl_conversation, false).expect("run migration");

        let store =
            message_store_backend_for_conversation(&paths, &conversation).expect("select store");
        let messages = store.read_all_messages().expect("read messages");

        assert_eq!(
            messages.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            vec!["jsonl1", "jsonl2"]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_ready_chat_snapshot_should_seek_latest_messages_from_index() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-chat-snapshot-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![
            test_message("u1", "user"),
            test_message("a1", "assistant"),
            test_message("a2", "assistant"),
            test_message("u2", "user"),
        ]);
        run_jsonl_snapshot_migration(&paths, &conversation, false).expect("run migration");

        let snapshot = read_ready_message_store_chat_snapshot(&paths)
            .expect("read chat snapshot")
            .expect("ready snapshot");

        assert_eq!(snapshot.latest_user.as_ref().map(|m| m.id.as_str()), Some("u2"));
        assert_eq!(
            snapshot.latest_assistant.as_ref().map(|m| m.id.as_str()),
            Some("a2")
        );
        assert_eq!(snapshot.active_message_count, 4);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn recent_messages_page_cached_should_refetch_when_block_cache_stale() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-recent-page-stale-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-recent-page").expect("paths");
        let mut conversation = test_conversation(vec![
            test_message("u1", "user"),
            test_message("a1", "assistant"),
        ]);
        conversation.id = "conversation-recent-page".to_string();
        run_jsonl_snapshot_migration(&paths, &conversation, false).expect("run migration");

        let seeded = read_ready_message_store_recent_messages_page_cached(&paths, 8)
            .expect("seed cached page")
            .expect("ready page");
        assert_eq!(
            seeded.messages.iter().map(|message| message.id.as_str()).collect::<Vec<_>>(),
            vec!["u1", "a1"]
        );

        let appended = test_message("a2", "assistant");
        conversation.messages.push(appended.clone());
        let meta = ConversationShardMeta::from_conversation(&conversation);
        write_jsonl_snapshot_appended_messages_shard_from_meta(
            &paths,
            &meta,
            std::slice::from_ref(&appended),
        )
        .expect("append latest message");

        let latest_block_path = read_ready_message_store_latest_block_paths(&paths, 1)
            .expect("read latest block path")
            .expect("latest block path")
            .into_iter()
            .next()
            .expect("one latest block");
        let latest_block_metadata = fs::metadata(&latest_block_path).expect("latest block metadata");
        let stale_messages = seeded
            .messages
            .iter()
            .map(|message| (message.id.clone(), message.clone()))
            .collect::<std::collections::HashMap<_, _>>();
        lock_message_store_block_file_cache().insert(
            latest_block_path,
            CachedMessageStoreBlockFile {
                modified_at: latest_block_metadata.modified().ok(),
                len: latest_block_metadata.len(),
                messages_by_id: Arc::new(stale_messages),
            },
        );

        let page = read_ready_message_store_recent_messages_page_cached(&paths, 8)
            .expect("read cached page after append")
            .expect("ready page");
        assert_eq!(
            page.messages.iter().map(|message| message.id.as_str()).collect::<Vec<_>>(),
            vec!["u1", "a1", "a2"]
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_branch_selection_should_keep_branch_semantics_from_index() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-branch-selection-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![
            test_message("m1", "user"),
            test_compaction_message("c1", "context_compaction"),
            test_message("m2", "assistant"),
            test_message("m3", "user"),
            test_compaction_message("c2", "summary_context_seed"),
            test_message("m4", "assistant"),
        ]);
        run_jsonl_snapshot_migration(&paths, &conversation, false).expect("run migration");
        let selected_ids = vec!["m3".to_string(), "m2".to_string()];

        let selection = read_ready_message_store_branch_selection(&paths, &selected_ids)
            .expect("read branch selection")
            .expect("ready branch selection");

        assert_eq!(
            selection
                .selected_messages
                .iter()
                .map(|message| message.id.as_str())
                .collect::<Vec<_>>(),
            vec!["m2", "m3"]
        );
        assert_eq!(selection.first_selected_ordinal, 2);
        assert_eq!(
            selection
                .latest_compaction_message
                .as_ref()
                .map(|message| message.id.as_str()),
            Some("c2")
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_directory_conversation_should_assemble_meta_and_messages() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-directory-read-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![
            test_message("jsonl1", "assistant"),
            test_message("jsonl2", "user"),
        ]);
        run_jsonl_snapshot_migration(&paths, &conversation, false).expect("run migration");

        let loaded = read_message_store_directory_conversation(&paths).expect("read directory");

        assert_eq!(loaded.id, conversation.id);
        assert_eq!(loaded.title, conversation.title);
        assert_eq!(
            loaded
                .messages
                .iter()
                .map(|message| message.id.as_str())
                .collect::<Vec<_>>(),
            vec!["jsonl1", "jsonl2"]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_ready_meta_should_not_decode_messages_jsonl() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-meta-ready-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![
            test_message("jsonl1", "assistant"),
            test_message("jsonl2", "user"),
        ]);
        run_jsonl_snapshot_migration(&paths, &conversation, false).expect("run migration");
        fs::write(&paths.messages_file, "{broken jsonl").expect("break messages jsonl");

        let meta = read_ready_message_store_meta(&paths)
            .expect("read ready meta")
            .expect("ready meta should exist");

        assert_eq!(meta.id, conversation.id);
        assert_eq!(meta.title, conversation.title);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_ready_status_should_fail_when_block_truth_is_corrupted() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-status-ready-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![
            test_message("jsonl1", "assistant"),
            test_message("jsonl2", "user"),
        ]);
        run_jsonl_snapshot_migration(&paths, &conversation, false).expect("run migration");
        let block_path = paths.blocks_dir.join("000000.jsonl");
        let original_content = fs::read_to_string(&block_path).expect("read messages");
        fs::write(&block_path, "x".repeat(original_content.len()))
            .expect("break messages jsonl without changing size");

        let err = read_ready_message_store_status(&paths)
            .expect_err("corrupted block truth should fail status read");

        assert!(err.contains("校验会话块失败") || err.contains("JSONL"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_ready_status_should_reject_stale_manifest_bytes_without_writing() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-status-size-mismatch-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![test_message("jsonl1", "assistant")]);
        run_jsonl_snapshot_migration(&paths, &conversation, false).expect("run migration");
        let mut manifest = read_message_store_manifest(&paths.manifest_file)
            .expect("read manifest")
            .expect("manifest exists");
        manifest.messages_jsonl_bytes += 1;
        write_message_store_manifest_atomic(&paths.manifest_file, &manifest)
            .expect("write stale manifest");

        let error = read_ready_message_store_status(&paths)
            .expect_err("stale manifest bytes must not self-heal during read");
        let stored_manifest = read_message_store_manifest(&paths.manifest_file)
            .expect("read manifest")
            .expect("manifest exists");

        assert!(error.contains("与 blocks 不一致"));
        assert_eq!(stored_manifest.messages_jsonl_bytes(), manifest.messages_jsonl_bytes());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_status_should_not_mark_event_log_ready_as_supported() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-event-log-status-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![test_message("jsonl1", "assistant")]);
        let mut manifest = MessageStoreManifest::jsonl_snapshot_building(&conversation);
        manifest.message_store_kind = MessageStoreKind::JsonlEventLog;
        manifest.migration_state = MessageStoreMigrationState::Ready;
        write_message_store_manifest_atomic(&paths.manifest_file, &manifest)
            .expect("write event log manifest");

        let ready_status = read_ready_message_store_status(&paths).expect("read ready status");
        let manifest_status = read_message_store_manifest_status(&paths)
            .expect("read manifest status")
            .expect("manifest status should exist");

        assert!(ready_status.is_none());
        assert_eq!(manifest_status.message_store_kind, "jsonlEventLog");
        assert_eq!(manifest_status.migration_state, "ready");
        assert!(!manifest_status.ready_jsonl);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_ready_meta_should_reject_mismatched_meta_id() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-meta-mismatch-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![test_message("jsonl1", "assistant")]);
        run_jsonl_snapshot_migration(&paths, &conversation, false).expect("run migration");
        let mut wrong_conversation = conversation.clone();
        wrong_conversation.id = "wrong-conversation".to_string();
        let wrong_meta = ConversationShardMeta::from_conversation(&wrong_conversation);
        write_conversation_shard_meta_atomic(&paths.meta_file, &wrong_meta)
            .expect("write wrong meta");

        let meta_err =
            read_ready_message_store_meta(&paths).expect_err("mismatched meta should fail");
        let status_err =
            read_ready_message_store_status(&paths).expect_err("mismatched status should fail");
        let directory_err = read_message_store_directory_conversation(&paths)
            .expect_err("mismatched directory should fail");

        assert!(meta_err.contains("元数据 ID 不一致"));
        assert!(status_err.contains("元数据 ID 不一致"));
        assert!(directory_err.contains("元数据 ID 不一致"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_directory_conversation_should_reject_stale_manifest_without_writing() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-directory-stale-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![test_message("m1", "user")]);
        run_jsonl_snapshot_migration(&paths, &conversation, false).expect("run migration");
        let mut manifest = read_message_store_manifest(&paths.manifest_file)
            .expect("read manifest")
            .expect("manifest exists");
        manifest.last_message_id = "wrong-last-id".to_string();
        write_message_store_manifest_atomic(&paths.manifest_file, &manifest)
            .expect("write stale manifest");

        let error = read_message_store_directory_conversation(&paths)
            .expect_err("stale manifest must not self-heal during read");
        let stored_manifest = read_message_store_manifest(&paths.manifest_file)
            .expect("read repaired manifest")
            .expect("manifest exists");

        assert!(error.contains("与 blocks 不一致"));
        assert_eq!(stored_manifest.last_message_id(), "wrong-last-id");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_ready_reader_should_reject_stale_manifest_bytes_without_writing() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-size-mismatch-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![test_message("m1", "user")]);
        run_jsonl_snapshot_migration(&paths, &conversation, false).expect("run migration");
        let mut manifest = read_message_store_manifest(&paths.manifest_file)
            .expect("read manifest")
            .expect("manifest exists");
        manifest.messages_jsonl_bytes += 1;
        write_message_store_manifest_atomic(&paths.manifest_file, &manifest)
            .expect("write stale manifest");

        let error = read_ready_message_store_recent_messages(&paths, 1)
            .expect_err("stale manifest bytes must not self-heal during read");
        let stored_manifest = read_message_store_manifest(&paths.manifest_file)
            .expect("read manifest")
            .expect("manifest exists");

        assert!(error.contains("与 blocks 不一致"));
        assert_eq!(stored_manifest.messages_jsonl_bytes(), manifest.messages_jsonl_bytes());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_backend_should_reject_stale_manifest_bytes_without_writing() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-backend-size-mismatch-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![test_message("m1", "user")]);
        run_jsonl_snapshot_migration(&paths, &conversation, false).expect("run migration");
        let mut manifest = read_message_store_manifest(&paths.manifest_file)
            .expect("read manifest")
            .expect("manifest exists");
        manifest.messages_jsonl_bytes += 1;
        write_message_store_manifest_atomic(&paths.manifest_file, &manifest)
            .expect("write stale manifest");

        let error = match message_store_backend_for_conversation(&paths, &conversation) {
            Ok(_) => panic!("stale manifest bytes must not self-heal during read"),
            Err(error) => error,
        };
        let stored_manifest = read_message_store_manifest(&paths.manifest_file)
            .expect("read manifest")
            .expect("manifest exists");

        assert!(error.contains("与 blocks 不一致"));
        assert_eq!(stored_manifest.messages_jsonl_bytes(), manifest.messages_jsonl_bytes());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_ready_reader_should_reject_index_manifest_count_mismatch_without_writing() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-index-count-mismatch-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![test_message("m1", "user")]);
        run_jsonl_snapshot_migration(&paths, &conversation, false).expect("run migration");
        let mut manifest = read_message_store_manifest(&paths.manifest_file)
            .expect("read manifest")
            .expect("manifest exists");
        manifest.source_message_count = 2;
        write_message_store_manifest_atomic(&paths.manifest_file, &manifest)
            .expect("write stale manifest");

        let error = read_ready_message_store_recent_messages(&paths, 1)
            .expect_err("stale manifest count must not self-heal during read");
        let stored_manifest = read_message_store_manifest(&paths.manifest_file)
            .expect("read stored manifest")
            .expect("manifest exists");

        assert!(error.contains("与 blocks 不一致"));
        assert_eq!(stored_manifest.source_message_count(), 2);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_ready_reader_should_reject_manifest_last_id_mismatch_without_writing() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-index-last-id-mismatch-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![test_message("m1", "user")]);
        run_jsonl_snapshot_migration(&paths, &conversation, false).expect("run migration");
        let mut manifest = read_message_store_manifest(&paths.manifest_file)
            .expect("read manifest")
            .expect("manifest exists");
        manifest.last_message_id = "wrong-last-id".to_string();
        write_message_store_manifest_atomic(&paths.manifest_file, &manifest)
            .expect("write stale manifest");

        let error = read_ready_message_store_recent_messages(&paths, 1)
            .expect_err("stale manifest last id must not self-heal during read");
        let stored_manifest = read_message_store_manifest(&paths.manifest_file)
            .expect("read stored manifest")
            .expect("manifest exists");

        assert!(error.contains("与 blocks 不一致"));
        assert_eq!(stored_manifest.last_message_id(), "wrong-last-id");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_ready_reader_should_reject_broken_block_without_writing() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-broken-block-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![test_message("m1", "user")]);
        run_jsonl_snapshot_migration(&paths, &conversation, false).expect("run migration");
        let block_path = paths.blocks_dir.join("000000.jsonl");
        let original = fs::read_to_string(&block_path).expect("read block");
        fs::write(&block_path, "x".repeat(original.len())).expect("corrupt block");

        let error = read_ready_message_store_recent_messages(&paths, 1)
            .expect_err("broken block content must not self-heal during read");

        assert!(error.contains("校验会话块失败") || error.contains("JSONL"));
        assert!(block_path.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_ready_reader_should_not_repair_index_for_compaction_kind_only_difference() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-compaction-kind-only-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![
            test_compaction_message("c1", "context_compaction"),
            test_message("m1", "assistant"),
        ]);
        run_jsonl_snapshot_migration(&paths, &conversation, false).expect("run migration");

        let mut index = (*read_message_store_index_file(&paths.index_file).expect("read index")).clone();
        for item in &mut index.items {
            item.compaction_kind = None;
        }
        write_message_store_index_atomic(&paths.index_file, &index).expect("write normalized index");
        let before = fs::read_to_string(&paths.index_file).expect("read index before");

        let messages = read_ready_message_store_recent_messages(&paths, 2)
            .expect("compaction kind difference should not trigger repair")
            .expect("ready messages should exist");
        let after = fs::read_to_string(&paths.index_file).expect("read index after");

        assert_eq!(messages.len(), 2);
        assert_eq!(before, after);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_delete_should_remove_legacy_file_and_directory_shard() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-delete-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![test_message("m1", "user")]);
        write_conversation_shard(&data_path, &conversation).expect("write legacy");
        run_jsonl_snapshot_migration(&paths, &conversation, false).expect("run migration");

        let changed = delete_message_store_shard_artifacts(&paths).expect("delete artifacts");

        assert!(changed);
        assert!(!paths.legacy_conversation_file.exists());
        assert!(!paths.shard_dir.exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_delete_should_validate_directory_before_removing_legacy_file() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-delete-guard-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let paths = message_store_paths(&data_path, "conversation-reader").expect("paths");
        let conversation = test_conversation(vec![test_message("m1", "user")]);
        write_conversation_shard(&data_path, &conversation).expect("write legacy");
        let outside_dir = root.join("outside-shard");
        fs::create_dir_all(&outside_dir).expect("create outside dir");
        let bad_paths = MessageStorePaths {
            data_path: paths.data_path.clone(),
            conversation_id: paths.conversation_id.clone(),
            legacy_conversation_file: paths.legacy_conversation_file.clone(),
            shard_dir: outside_dir.clone(),
            manifest_file: outside_dir.join("manifest.json"),
            meta_file: outside_dir.join("meta.json"),
            messages_file: outside_dir.join("messages.jsonl"),
            active_plans_file: outside_dir.join("active_plans.jsonl"),
            index_file: outside_dir.join("messages.idx.json"),
            blocks_dir: outside_dir.join("blocks"),
            blobs_dir: outside_dir.join("blobs"),
        };

        let err = delete_message_store_shard_artifacts(&bad_paths)
            .expect_err("unsafe shard dir should fail before deletion");

        assert!(err.contains("分片目录不在 conversations 目录内"));
        assert!(!paths.legacy_conversation_file.exists());
        assert!(outside_dir.exists());
        let _ = fs::remove_dir_all(root);
    }
}

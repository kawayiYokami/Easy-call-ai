const MESSAGE_STORE_MANIFEST_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum MessageStoreKind {
    ConversationJson,
    JsonlSnapshot,
    JsonlEventLog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum MessageStoreMigrationState {
    None,
    Building,
    Ready,
    Failed,
    Rollback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(super) struct MessageStoreManifest {
    version: u32,
    message_store_kind: MessageStoreKind,
    migration_state: MessageStoreMigrationState,
    #[serde(default)]
    source_conversation_revision: u64,
    #[serde(default)]
    source_message_count: usize,
    #[serde(default)]
    last_message_id: String,
    #[serde(default)]
    messages_jsonl_bytes: u64,
    #[serde(default)]
    messages_index_revision: u64,
    #[serde(default)]
    updated_at: String,
}

impl MessageStoreManifest {
    fn conversation_json_now(conversation: &Conversation) -> Self {
        Self {
            version: MESSAGE_STORE_MANIFEST_VERSION,
            message_store_kind: MessageStoreKind::ConversationJson,
            migration_state: MessageStoreMigrationState::None,
            source_conversation_revision: 0,
            source_message_count: conversation.messages.len(),
            last_message_id: conversation
                .messages
                .last()
                .map(|message| message.id.trim().to_string())
                .unwrap_or_default(),
            messages_jsonl_bytes: 0,
            messages_index_revision: 0,
            updated_at: now_iso(),
        }
    }

    pub(super) fn jsonl_snapshot_building(conversation: &Conversation) -> Self {
        Self {
            message_store_kind: MessageStoreKind::JsonlSnapshot,
            migration_state: MessageStoreMigrationState::Building,
            ..Self::conversation_json_now(conversation)
        }
    }

    fn jsonl_snapshot_ready(
        mut self,
        messages_jsonl_bytes: u64,
        messages_index_revision: u64,
    ) -> Self {
        self.message_store_kind = MessageStoreKind::JsonlSnapshot;
        self.migration_state = MessageStoreMigrationState::Ready;
        self.messages_jsonl_bytes = messages_jsonl_bytes;
        self.messages_index_revision = messages_index_revision;
        self.updated_at = now_iso();
        self
    }

    fn jsonl_snapshot_ready_for_messages(
        source_message_count: usize,
        last_message_id: String,
        messages_jsonl_bytes: u64,
        messages_index_revision: u64,
    ) -> Self {
        Self {
            version: MESSAGE_STORE_MANIFEST_VERSION,
            message_store_kind: MessageStoreKind::JsonlSnapshot,
            migration_state: MessageStoreMigrationState::Ready,
            source_conversation_revision: 0,
            source_message_count,
            last_message_id,
            messages_jsonl_bytes,
            messages_index_revision,
            updated_at: now_iso(),
        }
    }

    pub(super) fn should_read_jsonl(&self) -> bool {
        matches!(
            (self.message_store_kind, self.migration_state),
            (
                MessageStoreKind::JsonlSnapshot,
                MessageStoreMigrationState::Ready
            )
        )
    }

    pub(super) fn is_ready_directory_store(&self) -> bool {
        matches!(
            (self.message_store_kind, self.migration_state),
            (
                MessageStoreKind::JsonlSnapshot | MessageStoreKind::JsonlEventLog,
                MessageStoreMigrationState::Ready
            )
        )
    }

    fn should_write_jsonl_snapshot(&self) -> bool {
        matches!(
            (self.message_store_kind, self.migration_state),
            (
                MessageStoreKind::JsonlSnapshot,
                MessageStoreMigrationState::Ready
            )
        )
    }

    pub(super) fn store_kind_label(&self) -> &'static str {
        match self.message_store_kind {
            MessageStoreKind::ConversationJson => "conversationJson",
            MessageStoreKind::JsonlSnapshot => "jsonlSnapshot",
            MessageStoreKind::JsonlEventLog => "jsonlEventLog",
        }
    }

    pub(super) fn migration_state_label(&self) -> &'static str {
        match self.migration_state {
            MessageStoreMigrationState::None => "none",
            MessageStoreMigrationState::Building => "building",
            MessageStoreMigrationState::Ready => "ready",
            MessageStoreMigrationState::Failed => "failed",
            MessageStoreMigrationState::Rollback => "rollback",
        }
    }

    pub(super) fn source_message_count(&self) -> usize {
        self.source_message_count
    }

    pub(super) fn last_message_id(&self) -> &str {
        &self.last_message_id
    }

    pub(super) fn messages_jsonl_bytes(&self) -> u64 {
        self.messages_jsonl_bytes
    }

    pub(super) fn updated_at(&self) -> &str {
        &self.updated_at
    }

    fn stale_jsonl_reason(&self) -> Option<String> {
        if self.should_read_jsonl() {
            return None;
        }
        if self.is_ready_directory_store() {
            return Some(format!(
                "消息存储 manifest 处于 ready 状态，但当前版本暂不支持读取该目录型存储: kind={:?}, state={:?}",
                self.message_store_kind, self.migration_state
            ));
        }
        Some(format!(
            "消息存储 manifest 未处于 ready JSONL 状态: kind={:?}, state={:?}",
            self.message_store_kind, self.migration_state
        ))
    }

    fn validate_version(&self, path: &PathBuf) -> Result<(), String> {
        if self.version != MESSAGE_STORE_MANIFEST_VERSION {
            return Err(format!(
                "消息存储 manifest 版本不支持，path={}，expected={}，actual={}",
                path.display(),
                MESSAGE_STORE_MANIFEST_VERSION,
                self.version
            ));
        }
        Ok(())
    }
}

pub(super) fn read_message_store_manifest(path: &PathBuf) -> Result<Option<MessageStoreManifest>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("读取消息存储 manifest 失败，path={}，error={err}", path.display()))?;
    let manifest = serde_json::from_str::<MessageStoreManifest>(&raw)
        .map_err(|err| format!("解析消息存储 manifest 失败，path={}，error={err}", path.display()))?;
    manifest.validate_version(path)?;
    Ok(Some(manifest))
}

pub(super) fn read_message_store_manifest_for_paths(
    paths: &MessageStorePaths,
) -> Result<Option<MessageStoreManifest>, String> {
    read_message_store_manifest(&paths.manifest_file)
}

pub(super) fn write_message_store_manifest_atomic(
    path: &PathBuf,
    manifest: &MessageStoreManifest,
) -> Result<(), String> {
    let raw = serde_json::to_string_pretty(manifest)
        .map_err(|err| format!("序列化消息存储 manifest 失败: {err}"))?;
    write_message_store_text_atomic(path, "json.tmp", &raw, "消息存储 manifest")
}

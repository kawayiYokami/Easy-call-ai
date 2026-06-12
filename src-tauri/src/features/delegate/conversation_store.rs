const DELEGATE_CONVERSATIONS_DIR_NAME: &str = "delegate-conversations";

#[derive(Debug, Clone)]
struct DelegatePersistedConversationSummary {
    conversation_id: String,
    title: String,
    updated_at: String,
    last_message_at: Option<String>,
    message_count: usize,
    agent_id: String,
    delegate_id: Option<String>,
    root_conversation_id: Option<String>,
    archived_at: Option<String>,
}

fn delegate_conversation_store_dir(data_path: &PathBuf) -> PathBuf {
    app_root_from_data_path(data_path).join(DELEGATE_CONVERSATIONS_DIR_NAME)
}

fn validate_delegate_conversation_id(conversation_id: &str) -> Result<(), String> {
    if conversation_id.trim().is_empty() {
        return Err("委托会话 ID 不能为空".to_string());
    }
    if conversation_id != conversation_id.trim() {
        return Err(format!(
            "委托会话 ID 不能包含首尾空白，conversation_id={conversation_id}"
        ));
    }
    if conversation_id.contains('/') || conversation_id.contains('\\') {
        return Err(format!(
            "委托会话 ID 不能包含路径分隔符，conversation_id={conversation_id}"
        ));
    }
    if conversation_id
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
    {
        return Err(format!(
            "委托会话 ID 不能包含 Windows 文件名非法字符，conversation_id={conversation_id}"
        ));
    }
    if conversation_id.ends_with([' ', '.']) {
        return Err(format!(
            "委托会话 ID 不能以 Windows 不稳定文件名字符结尾，conversation_id={conversation_id}"
        ));
    }
    let mut components = std::path::Path::new(conversation_id).components();
    let Some(component) = components.next() else {
        return Err("委托会话 ID 不能为空".to_string());
    };
    if components.next().is_some() || !matches!(component, std::path::Component::Normal(_)) {
        return Err(format!(
            "委托会话 ID 不能包含路径组件，conversation_id={conversation_id}"
        ));
    }
    Ok(())
}

fn delegate_conversation_store_path(
    data_path: &PathBuf,
    conversation_id: &str,
) -> Result<PathBuf, String> {
    let conversation_id = conversation_id.trim();
    validate_delegate_conversation_id(conversation_id)?;
    Ok(delegate_conversation_store_dir(data_path).join(format!("{conversation_id}.json")))
}

fn delegate_conversation_message_store_paths(
    data_path: &PathBuf,
    conversation_id: &str,
) -> Result<message_store::MessageStorePaths, String> {
    let conversation_id = conversation_id.trim();
    validate_delegate_conversation_id(conversation_id)?;
    let store_dir = delegate_conversation_store_dir(data_path);
    message_store::message_store_paths_for_shard_dir(
        data_path,
        conversation_id,
        store_dir.join(conversation_id),
        store_dir.join(format!("{conversation_id}.json")),
    )
}

fn validate_delegate_conversation_record(
    conversation: &Conversation,
    requested_conversation_id: &str,
) -> Result<(), String> {
    if conversation.conversation_kind.trim() != CONVERSATION_KIND_DELEGATE {
        return Err(format!(
            "委托会话文件类型不正确，conversation_id={}，conversation_kind={}",
            conversation.id, conversation.conversation_kind
        ));
    }
    if conversation.id.trim() != requested_conversation_id.trim() {
        return Err(format!(
            "委托会话文件 ID 不匹配，requested={}，actual={}",
            requested_conversation_id.trim(), conversation.id
        ));
    }
    Ok(())
}

fn validate_delegate_conversation_for_write(conversation: &Conversation) -> Result<(), String> {
    validate_delegate_conversation_id(&conversation.id)?;
    validate_delegate_conversation_record(conversation, &conversation.id)?;
    if conversation
        .root_conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        return Err(format!(
            "委托会话缺少 root_conversation_id，conversation_id={}",
            conversation.id
        ));
    }
    Ok(())
}

fn validate_delegate_conversation_meta(
    meta: &message_store::ConversationShardMeta,
    requested_conversation_id: &str,
) -> Result<(), String> {
    if meta.conversation_kind().trim() != CONVERSATION_KIND_DELEGATE {
        return Err(format!(
            "委托会话元数据类型不正确，conversation_id={}，conversation_kind={}",
            meta.id(),
            meta.conversation_kind()
        ));
    }
    if meta.id().trim() != requested_conversation_id.trim() {
        return Err(format!(
            "委托会话元数据 ID 不匹配，requested={}，actual={}",
            requested_conversation_id.trim(),
            meta.id()
        ));
    }
    Ok(())
}

fn delegate_conversation_store_read_legacy(
    data_path: &PathBuf,
    conversation_id: &str,
) -> Result<Option<Conversation>, String> {
    let path = delegate_conversation_store_path(data_path, conversation_id)?;
    if !path.exists() {
        return Ok(None);
    }
    let conversation = read_json_file::<Conversation>(&path, "delegate conversation file")?;
    validate_delegate_conversation_record(&conversation, conversation_id)?;
    Ok(Some(conversation))
}

fn delegate_conversation_store_migrate_legacy(
    paths: &message_store::MessageStorePaths,
    conversation: &Conversation,
) -> Result<(), String> {
    validate_delegate_conversation_for_write(conversation)?;
    let outcome = message_store::resume_jsonl_snapshot_migration(paths, conversation)?;
    if outcome.wrote_files {
        runtime_log_info(format!(
            "[委托会话] 完成，任务=迁移委托会话正文，conversation_id={}，message_count={}",
            conversation.id,
            conversation.messages.len()
        ));
    }
    Ok(())
}

fn delegate_conversation_store_read_ready_meta(
    paths: &message_store::MessageStorePaths,
    conversation_id: &str,
) -> Result<Option<message_store::ConversationShardMeta>, String> {
    let Some(meta) = message_store::read_ready_message_store_meta(paths)? else {
        return Ok(None);
    };
    validate_delegate_conversation_meta(&meta, conversation_id)?;
    Ok(Some(meta))
}

fn delegate_conversation_store_read(
    data_path: &PathBuf,
    conversation_id: &str,
) -> Result<Option<Conversation>, String> {
    let conversation_id = conversation_id.trim();
    let paths = delegate_conversation_message_store_paths(data_path, conversation_id)?;
    match message_store::read_ready_message_store_directory_conversation(&paths) {
        Ok(Some(conversation)) => {
            validate_delegate_conversation_record(&conversation, conversation_id)?;
            return Ok(Some(conversation));
        }
        Ok(None) => {}
        Err(store_err) => {
            if let Some(legacy) = delegate_conversation_store_read_legacy(data_path, conversation_id)? {
                if let Err(migrate_err) =
                    delegate_conversation_store_migrate_legacy(&paths, &legacy)
                {
                    runtime_log_info(format!(
                        "[委托会话] 失败，任务=迁移委托会话正文，conversation_id={}，error={}",
                        conversation_id, migrate_err
                    ));
                    return Ok(Some(legacy));
                }
                match message_store::read_ready_message_store_directory_conversation(&paths) {
                    Ok(Some(conversation)) => {
                        validate_delegate_conversation_record(&conversation, conversation_id)?;
                        return Ok(Some(conversation));
                    }
                    Ok(None) => return Ok(Some(legacy)),
                    Err(read_err) => {
                        runtime_log_info(format!(
                            "[委托会话] 失败，任务=读取迁移后委托会话正文，conversation_id={}，error={}",
                            conversation_id, read_err
                        ));
                        return Ok(Some(legacy));
                    }
                }
            }
            return Err(store_err);
        }
    }

    if let Some(legacy) = delegate_conversation_store_read_legacy(data_path, conversation_id)? {
        if let Err(err) = delegate_conversation_store_migrate_legacy(&paths, &legacy) {
            runtime_log_info(format!(
                "[委托会话] 失败，任务=迁移委托会话正文，conversation_id={}，error={}",
                conversation_id, err
            ));
            return Ok(Some(legacy));
        }
        match message_store::read_ready_message_store_directory_conversation(&paths) {
            Ok(Some(conversation)) => {
                validate_delegate_conversation_record(&conversation, conversation_id)?;
                return Ok(Some(conversation));
            }
            Ok(None) => return Ok(Some(legacy)),
            Err(err) => {
                runtime_log_info(format!(
                    "[委托会话] 失败，任务=读取迁移后委托会话正文，conversation_id={}，error={}",
                    conversation_id, err
                ));
                return Ok(Some(legacy));
            }
        }
    }

    if let Some(status) = message_store::read_message_store_manifest_status(&paths)? {
        return Err(format!(
            "委托会话消息仓库未处于可读取状态，conversation_id={}，kind={}，state={}",
            conversation_id, status.message_store_kind, status.migration_state
        ));
    }
    Ok(None)
}

fn delegate_conversation_store_write(
    data_path: &PathBuf,
    conversation: &Conversation,
) -> Result<(), String> {
    validate_delegate_conversation_for_write(conversation)?;
    fs::create_dir_all(delegate_conversation_store_dir(data_path)).map_err(|err| {
        format!(
            "创建委托会话目录失败，path={}，error={err}",
            delegate_conversation_store_dir(data_path).display()
        )
    })?;
    let paths = delegate_conversation_message_store_paths(data_path, &conversation.id)?;
    message_store::write_jsonl_snapshot_directory_shard_if_changed(&paths, conversation)?;
    Ok(())
}

fn delegate_conversation_store_delete(
    data_path: &PathBuf,
    conversation_id: &str,
) -> Result<bool, String> {
    let paths = delegate_conversation_message_store_paths(data_path, conversation_id)?;
    message_store::delete_message_store_shard_artifacts(&paths)
}

fn delegate_conversation_store_collect_ids(
    data_path: &PathBuf,
) -> Result<std::collections::BTreeSet<String>, String> {
    let dir = delegate_conversation_store_dir(data_path);
    if !dir.exists() {
        return Ok(std::collections::BTreeSet::new());
    }
    let mut ids = std::collections::BTreeSet::<String>::new();
    for entry in fs::read_dir(&dir).map_err(|err| {
        format!("读取委托会话目录失败，path={}，error={err}", dir.display())
    })? {
        let entry = entry.map_err(|err| format!("读取委托会话目录项失败: {err}"))?;
        let path = entry.path();
        let conversation_id = if path.is_dir() {
            path.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .trim()
                .to_string()
        } else if path.extension().and_then(|value| value.to_str()) == Some("json") {
            path.file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .trim()
                .to_string()
        } else {
            continue;
        };
        if conversation_id.is_empty() || validate_delegate_conversation_id(&conversation_id).is_err()
        {
            continue;
        }
        ids.insert(conversation_id);
    }
    Ok(ids)
}

fn delegate_conversation_summary_last_message_at(
    meta: &message_store::ConversationShardMeta,
) -> Option<String> {
    match (meta.last_user_at(), meta.last_assistant_at()) {
        (Some(left), Some(right)) => Some(if left >= right { left } else { right }.to_string()),
        (Some(value), None) | (None, Some(value)) => Some(value.to_string()),
        (None, None) => None,
    }
}

fn delegate_conversation_store_summary_from_conversation(
    conversation: &Conversation,
) -> DelegatePersistedConversationSummary {
    DelegatePersistedConversationSummary {
        conversation_id: conversation.id.clone(),
        title: conversation.title.clone(),
        updated_at: conversation.updated_at.clone(),
        last_message_at: conversation.messages.last().map(|message| message.created_at.clone()),
        message_count: conversation.messages.len(),
        agent_id: conversation.agent_id.clone(),
        delegate_id: conversation.delegate_id.clone(),
        root_conversation_id: conversation.root_conversation_id.clone(),
        archived_at: conversation.archived_at.clone(),
    }
}

fn delegate_conversation_store_summary_from_meta(
    meta: &message_store::ConversationShardMeta,
    message_count: usize,
) -> DelegatePersistedConversationSummary {
    DelegatePersistedConversationSummary {
        conversation_id: meta.id().to_string(),
        title: meta.title().to_string(),
        updated_at: meta.updated_at().to_string(),
        last_message_at: delegate_conversation_summary_last_message_at(meta),
        message_count,
        agent_id: meta.agent_id().to_string(),
        delegate_id: meta.delegate_id().map(str::to_string),
        root_conversation_id: meta.root_conversation_id().map(str::to_string),
        archived_at: meta.archived_at().map(str::to_string),
    }
}

fn delegate_conversation_store_summary_read(
    data_path: &PathBuf,
    conversation_id: &str,
) -> Result<Option<DelegatePersistedConversationSummary>, String> {
    let conversation_id = conversation_id.trim();
    let paths = delegate_conversation_message_store_paths(data_path, conversation_id)?;
    match delegate_conversation_store_read_ready_meta(&paths, conversation_id) {
        Ok(Some(meta)) => {
            let status = message_store::read_ready_message_store_status(&paths)?
                .ok_or_else(|| format!("委托会话消息仓库状态缺失，conversation_id={conversation_id}"))?;
            return Ok(Some(delegate_conversation_store_summary_from_meta(
                &meta,
                status.source_message_count,
            )));
        }
        Ok(None) => {}
        Err(err) => {
            if delegate_conversation_store_path(data_path, conversation_id)?.exists() {
                runtime_log_info(format!(
                    "[委托会话] 跳过，任务=读取目录型委托会话摘要，conversation_id={}，reason={}",
                    conversation_id, err
                ));
            } else {
                return Err(err);
            }
        }
    }

    let Some(legacy) = delegate_conversation_store_read_legacy(data_path, conversation_id)? else {
        if let Some(status) = message_store::read_message_store_manifest_status(&paths)? {
            return Err(format!(
                "委托会话消息仓库未处于可读取状态，conversation_id={}，kind={}，state={}",
                conversation_id, status.message_store_kind, status.migration_state
            ));
        }
        return Ok(None);
    };
    if let Err(err) = delegate_conversation_store_migrate_legacy(&paths, &legacy) {
        runtime_log_info(format!(
            "[委托会话] 失败，任务=迁移委托会话摘要正文，conversation_id={}，error={}",
            conversation_id, err
        ));
        return Ok(Some(delegate_conversation_store_summary_from_conversation(
            &legacy,
        )));
    }
    match delegate_conversation_store_read_ready_meta(&paths, conversation_id)? {
        Some(meta) => {
            let status = message_store::read_ready_message_store_status(&paths)?
                .ok_or_else(|| format!("委托会话消息仓库状态缺失，conversation_id={conversation_id}"))?;
            Ok(Some(delegate_conversation_store_summary_from_meta(
                &meta,
                status.source_message_count,
            )))
        }
        None => Ok(Some(delegate_conversation_store_summary_from_conversation(
            &legacy,
        ))),
    }
}

fn delegate_conversation_store_summary_list(
    data_path: &PathBuf,
) -> Result<Vec<DelegatePersistedConversationSummary>, String> {
    let mut summaries = Vec::new();
    for conversation_id in delegate_conversation_store_collect_ids(data_path)? {
        if let Some(summary) = delegate_conversation_store_summary_read(data_path, &conversation_id)?
        {
            summaries.push(summary);
        }
    }
    Ok(summaries)
}

fn delegate_conversation_store_list(data_path: &PathBuf) -> Result<Vec<Conversation>, String> {
    let mut conversations = Vec::new();
    for conversation_id in delegate_conversation_store_collect_ids(data_path)? {
        if let Some(conversation) = delegate_conversation_store_read(data_path, &conversation_id)? {
            conversations.push(conversation);
        }
    }
    Ok(conversations)
}

fn delegate_conversation_store_block_page_from_legacy(
    conversation: &Conversation,
) -> message_store::MessageStoreBlockPage {
    message_store::MessageStoreBlockPage {
        blocks: vec![message_store::MessageStoreBlockSummary {
            block_id: 0,
            message_count: conversation.messages.len(),
            first_message_id: conversation
                .messages
                .first()
                .map(|message| message.id.clone())
                .unwrap_or_default(),
            last_message_id: conversation
                .messages
                .last()
                .map(|message| message.id.clone())
                .unwrap_or_default(),
            first_created_at: conversation.messages.first().map(|message| message.created_at.clone()),
            last_created_at: conversation.messages.last().map(|message| message.created_at.clone()),
            is_latest: true,
        }],
        selected_block_id: 0,
        messages: conversation.messages.clone(),
        has_prev_block: false,
        has_next_block: false,
    }
}

fn delegate_conversation_store_read_block_page(
    data_path: &PathBuf,
    conversation_id: &str,
    requested_block_id: Option<u32>,
) -> Result<Option<message_store::MessageStoreBlockPage>, String> {
    let conversation_id = conversation_id.trim();
    let paths = delegate_conversation_message_store_paths(data_path, conversation_id)?;
    match delegate_conversation_store_read_ready_meta(&paths, conversation_id) {
        Ok(Some(_)) => {
            if let Some(page) =
                message_store::read_ready_message_store_block_page(&paths, requested_block_id)?
            {
                return Ok(Some(page));
            }
        }
        Ok(None) => {}
        Err(err) => {
            if !delegate_conversation_store_path(data_path, conversation_id)?.exists() {
                return Err(err);
            }
            runtime_log_info(format!(
                "[委托会话] 跳过，任务=读取目录型委托会话分页，conversation_id={}，reason={}",
                conversation_id, err
            ));
        }
    }

    let Some(legacy) = delegate_conversation_store_read_legacy(data_path, conversation_id)? else {
        if let Some(status) = message_store::read_message_store_manifest_status(&paths)? {
            return Err(format!(
                "委托会话消息仓库未处于可读取状态，conversation_id={}，kind={}，state={}",
                conversation_id, status.message_store_kind, status.migration_state
            ));
        }
        return Ok(None);
    };
    if let Err(err) = delegate_conversation_store_migrate_legacy(&paths, &legacy) {
        runtime_log_info(format!(
            "[委托会话] 失败，任务=迁移委托会话分页正文，conversation_id={}，error={}",
            conversation_id, err
        ));
        return Ok(Some(delegate_conversation_store_block_page_from_legacy(
            &legacy,
        )));
    }
    match message_store::read_ready_message_store_block_page(&paths, requested_block_id)? {
        Some(page) => Ok(Some(page)),
        None => Ok(Some(delegate_conversation_store_block_page_from_legacy(
            &legacy,
        ))),
    }
}

#[cfg(test)]
mod delegate_conversation_store_tests {
    use super::*;

    fn test_message(id: &str, role: &str) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            role: role.to_string(),
            created_at: format!("2026-06-08T00:00:0{}Z", id.len()),
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

    fn test_delegate_conversation(conversation_id: &str) -> Conversation {
        Conversation {
            id: conversation_id.to_string(),
            title: "委托会话".to_string(),
            agent_id: DEFAULT_AGENT_ID.to_string(),
            department_id: ASSISTANT_DEPARTMENT_ID.to_string(),
            bound_conversation_id: None,
            parent_conversation_id: None,
            child_conversation_ids: Vec::new(),
            fork_message_cursor: None,
            unread_count: 0,
            conversation_kind: CONVERSATION_KIND_DELEGATE.to_string(),
            root_conversation_id: Some("root-conversation".to_string()),
            delegate_id: Some(conversation_id.to_string()),
            created_at: "2026-06-08T00:00:00Z".to_string(),
            updated_at: "2026-06-08T00:00:02Z".to_string(),
            last_user_at: Some("2026-06-08T00:00:01Z".to_string()),
            last_assistant_at: Some("2026-06-08T00:00:02Z".to_string()),
            status: String::new(),
            summary: String::new(),
            user_profile_snapshot: String::new(),
            shell_workspace_path: None,
            shell_workspaces: Vec::new(),
            shell_autonomous_mode: false,
            archived_at: None,
            messages: vec![test_message("m1", "user"), test_message("m2", "assistant")],
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
    fn delegate_conversation_store_should_migrate_legacy_json_on_read() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-delegate-store-read-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let conversation = test_delegate_conversation("delegate-a");
        let legacy_path =
            delegate_conversation_store_path(&data_path, &conversation.id).expect("legacy path");
        write_json_file_atomic(&legacy_path, &conversation, "legacy delegate conversation")
            .expect("write legacy");

        let loaded = delegate_conversation_store_read(&data_path, &conversation.id)
            .expect("read delegate")
            .expect("delegate exists");
        let summary = delegate_conversation_store_summary_read(&data_path, &conversation.id)
            .expect("summary")
            .expect("summary exists");
        let shard_dir = delegate_conversation_store_dir(&data_path).join(&conversation.id);

        assert_eq!(loaded.id, conversation.id);
        assert_eq!(loaded.messages.len(), 2);
        assert_eq!(summary.message_count, 2);
        assert!(legacy_path.exists());
        assert!(shard_dir.join(message_store::MESSAGE_STORE_MANIFEST_FILE_NAME).exists());
        assert!(shard_dir.join(message_store::MESSAGE_STORE_META_FILE_NAME).exists());
        assert!(shard_dir
            .join(message_store::MESSAGE_STORE_BLOCKS_DIR_NAME)
            .exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn delegate_conversation_store_should_write_directory_store_without_legacy_json() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-delegate-store-write-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let conversation = test_delegate_conversation("delegate-b");

        delegate_conversation_store_write(&data_path, &conversation).expect("write delegate");
        let legacy_path =
            delegate_conversation_store_path(&data_path, &conversation.id).expect("legacy path");
        let page = delegate_conversation_store_read_block_page(&data_path, &conversation.id, None)
            .expect("read block page")
            .expect("page exists");

        assert!(!legacy_path.exists());
        assert_eq!(page.messages.len(), 2);
        assert_eq!(page.blocks.len(), 1);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn delegate_conversation_store_delete_should_remove_directory_and_legacy_json() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-delegate-store-delete-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");
        let conversation = test_delegate_conversation("delegate-c");
        let legacy_path =
            delegate_conversation_store_path(&data_path, &conversation.id).expect("legacy path");
        write_json_file_atomic(&legacy_path, &conversation, "legacy delegate conversation")
            .expect("write legacy");
        delegate_conversation_store_write(&data_path, &conversation).expect("write delegate");
        let shard_dir = delegate_conversation_store_dir(&data_path).join(&conversation.id);

        let deleted =
            delegate_conversation_store_delete(&data_path, &conversation.id).expect("delete");

        assert!(deleted);
        assert!(!legacy_path.exists());
        assert!(!shard_dir.exists());
        let _ = fs::remove_dir_all(root);
    }
}

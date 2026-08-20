#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JsonlSnapshotMessageLine {
    kind: String,
    message: ChatMessage,
}

const JSONL_SNAPSHOT_MESSAGE_KIND: &str = "message";

fn encode_jsonl_snapshot_message(message: &ChatMessage) -> Result<String, String> {
    let line = JsonlSnapshotMessageLine {
        kind: JSONL_SNAPSHOT_MESSAGE_KIND.to_string(),
        message: message.clone(),
    };
    serde_json::to_string(&line)
        .map(|value| format!("{value}\n"))
        .map_err(|err| format!("序列化 JSONL 消息失败: {err}"))
}

fn decode_jsonl_snapshot_message(line: &str) -> Result<ChatMessage, String> {
    let parsed: JsonlSnapshotMessageLine =
        serde_json::from_str(line).map_err(|err| format!("解析 JSONL 消息失败: {err}"))?;
    if parsed.kind.trim() != JSONL_SNAPSHOT_MESSAGE_KIND {
        return Err(format!("不支持的 JSONL 消息类型: {}", parsed.kind));
    }
    Ok(parsed.message)
}

fn encode_jsonl_snapshot_messages(messages: &[ChatMessage]) -> Result<String, String> {
    let mut out = String::new();
    for message in messages {
        out.push_str(&encode_jsonl_snapshot_message(message)?);
    }
    Ok(out)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct JsonlSnapshotConversationBlock {
    block_id: u32,
    block_file: String,
    content: String,
    index_items: Vec<MessageStoreIndexItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct JsonlSnapshotConversationBlocks {
    blocks: Vec<JsonlSnapshotConversationBlock>,
    index: MessageStoreIndexFile,
    message_count: usize,
    last_message_id: String,
    total_bytes: u64,
}

#[derive(Debug, Clone)]
struct ConversationBlockMessageRefs<'a> {
    block_id: u32,
    block_file: String,
    messages: Vec<&'a ChatMessage>,
}

fn build_jsonl_snapshot_conversation_blocks(
    messages: &[ChatMessage],
) -> Result<JsonlSnapshotConversationBlocks, String> {
    build_jsonl_snapshot_conversation_blocks_from_refs(
        false,
        &split_messages_into_conversation_blocks(messages),
    )
}

fn build_jsonl_snapshot_conversation_blocks_for_conversation(
    conversation: &Conversation,
) -> Result<JsonlSnapshotConversationBlocks, String> {
    build_jsonl_snapshot_conversation_blocks_from_refs(
        conversation.status.trim() == "archived",
        &split_conversation_messages_into_blocks(conversation),
    )
}

fn build_jsonl_snapshot_conversation_blocks_from_refs(
    archived_conversation: bool,
    source_blocks: &[ConversationBlockMessageRefs<'_>],
) -> Result<JsonlSnapshotConversationBlocks, String> {
    let message_count = source_blocks
        .iter()
        .map(|block| block.messages.len())
        .sum::<usize>();
    let mut blocks = Vec::<JsonlSnapshotConversationBlock>::with_capacity(source_blocks.len());
    let mut all_items = Vec::<MessageStoreIndexItem>::with_capacity(message_count);
    let mut total_bytes = 0_u64;
    let mut last_message_id = String::new();

    for (block_idx, block_messages) in source_blocks.iter().enumerate() {
        let should_slim =
            should_slim_conversation_block(archived_conversation, block_idx, source_blocks.len());
        let block = build_jsonl_snapshot_conversation_block(block_messages, should_slim)?;
        last_message_id = block
            .index_items
            .last()
            .map(|item| item.message_id.clone())
            .unwrap_or(last_message_id);
        total_bytes = total_bytes
            .checked_add(block.content.as_bytes().len() as u64)
            .ok_or_else(|| format!("构建会话块失败：总字节数溢出，block_file={}", block.block_file))?;
        all_items.extend(block.index_items.iter().cloned());
        blocks.push(block);
    }

    Ok(JsonlSnapshotConversationBlocks {
        blocks,
        index: MessageStoreIndexFile::new(MESSAGE_STORE_MANIFEST_VERSION, all_items),
        message_count,
        last_message_id,
        total_bytes,
    })
}

fn split_conversation_messages_into_blocks(
    conversation: &Conversation,
) -> Vec<ConversationBlockMessageRefs<'_>> {
    split_messages_into_conversation_blocks(&conversation.messages)
}

fn split_messages_into_conversation_blocks(
    messages: &[ChatMessage],
) -> Vec<ConversationBlockMessageRefs<'_>> {
    let mut raw_blocks = Vec::<Vec<&ChatMessage>>::new();
    let mut current = Vec::<&ChatMessage>::new();
    for message in messages {
        if message_store_compaction_kind(message).is_some() && !current.is_empty() {
            raw_blocks.push(current);
            current = Vec::new();
        }
        current.push(message);
    }
    if !current.is_empty() {
        raw_blocks.push(current);
    }
    raw_blocks
        .into_iter()
        .enumerate()
        .map(|(idx, messages)| ConversationBlockMessageRefs {
            block_id: idx as u32,
            block_file: format!("{MESSAGE_STORE_BLOCKS_DIR_NAME}/{idx:06}.jsonl"),
            messages,
        })
        .collect()
}

fn should_slim_conversation_block(
    archived_conversation: bool,
    _block_idx: usize,
    _block_count: usize,
) -> bool {
    archived_conversation
}

fn raw_blocks_to_conversation_block_refs(
    raw_blocks: Vec<Vec<&ChatMessage>>,
) -> Vec<ConversationBlockMessageRefs<'_>> {
    raw_blocks
        .into_iter()
        .enumerate()
        .map(|(idx, messages)| ConversationBlockMessageRefs {
            block_id: idx as u32,
            block_file: format!("{MESSAGE_STORE_BLOCKS_DIR_NAME}/{idx:06}.jsonl"),
            messages,
        })
        .collect()
}

fn message_store_message_day_key(message: &ChatMessage) -> String {
    message_store_message_business_day_key(&message.created_at).unwrap_or_else(|| {
        message
            .created_at
            .split('T')
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("unknown")
            .to_string()
    })
}

fn message_store_message_business_day_key(created_at: &str) -> Option<String> {
    let parsed = chrono::DateTime::parse_from_rfc3339(created_at.trim()).ok()?;
    let local = parsed.with_timezone(&chrono::Local);
    let day = if local.time() < chrono::NaiveTime::from_hms_opt(4, 0, 0)? {
        local.date_naive().pred_opt()?
    } else {
        local.date_naive()
    };
    Some(day.format("%Y-%m-%d").to_string())
}

fn build_jsonl_snapshot_conversation_block(
    block: &ConversationBlockMessageRefs<'_>,
    should_slim: bool,
) -> Result<JsonlSnapshotConversationBlock, String> {
    let mut content = String::new();
    let mut offset = 0_u64;
    let mut block_items = Vec::<MessageStoreIndexItem>::with_capacity(block.messages.len());

    for message in &block.messages {
        let stored_message = if should_slim {
            slim_older_conversation_block_message(message)
        } else {
            (*message).clone()
        };
        let encoded = encode_jsonl_snapshot_message(&stored_message)?;
        let byte_len = encoded.as_bytes().len() as u64;
        let item = message_store_index_item_for_message_in_block(
            &stored_message,
            Some(block.block_id),
            offset,
            byte_len,
        );
        if item.compaction_kind.is_none() && message_store_compaction_kind(message).is_some() {
            return Err(format!(
                "构建会话块失败：瘦身后丢失压缩边界，message_id={}",
                message.id
            ));
        }
        content.push_str(&encoded);
        offset = offset.checked_add(byte_len).ok_or_else(|| {
            format!(
                "构建会话块失败：block offset 溢出，block_file={}，message_id={}",
                block.block_file, message.id
            )
        })?;
        block_items.push(item);
    }

    Ok(JsonlSnapshotConversationBlock {
        block_id: block.block_id,
        block_file: block.block_file.clone(),
        content,
        index_items: block_items,
    })
}

fn slim_older_conversation_block_message(message: &ChatMessage) -> ChatMessage {
    let mut next = message.clone();
    next.parts = message
        .parts
        .iter()
        .filter_map(slim_older_conversation_block_part)
        .collect();
    next.extra_text_blocks.clear();
    next.provider_meta = slim_older_conversation_block_provider_meta(message);
    next.tool_call = None;
    next.mcp_call = None;
    next
}

fn slim_older_conversation_block_part(part: &MessagePart) -> Option<MessagePart> {
    match part {
        MessagePart::Text {
            text,
            reasoning_content,
        } => Some(MessagePart::Text {
            text: text.clone(),
            reasoning_content: reasoning_content.clone(),
        }),
        MessagePart::Image {
            mime,
            bytes_base64,
            name,
            compressed,
        } => {
            let trimmed = bytes_base64.trim();
            if !(trimmed.starts_with("@media:")
                || trimmed.starts_with("@download:")
                || trimmed.starts_with("http://")
                || trimmed.starts_with("https://"))
            {
                return None;
            }
            Some(MessagePart::Image {
                mime: mime.clone(),
                bytes_base64: bytes_base64.clone(),
                name: name.clone(),
                compressed: *compressed,
            })
        }
        MessagePart::Audio {
            mime,
            bytes_base64,
            name,
            compressed,
        } => {
            let trimmed = bytes_base64.trim();
            if !(trimmed.starts_with("@media:")
                || trimmed.starts_with("@download:")
                || trimmed.starts_with("http://")
                || trimmed.starts_with("https://"))
            {
                return None;
            }
            Some(MessagePart::Audio {
                mime: mime.clone(),
                bytes_base64: bytes_base64.clone(),
                name: name.clone(),
                compressed: *compressed,
            })
        }
        MessagePart::Attachment { path, mime, name } => Some(MessagePart::Attachment {
            path: path.clone(),
            mime: mime.clone(),
            name: name.clone(),
        }),
    }
}

fn slim_older_conversation_block_provider_meta(message: &ChatMessage) -> Option<Value> {
    let mut meta = serde_json::Map::new();
    if let Some(kind) = message_store_compaction_kind(message) {
        meta.insert(
            "message_meta".to_string(),
            serde_json::json!({
                "kind": kind,
            }),
        );
    }
    if let Some(origin) = remote_im_origin_from_message(message) {
        meta.insert("origin".to_string(), origin.clone());
    }
    if meta.is_empty() {
        None
    } else {
        Some(Value::Object(meta))
    }
}

fn write_jsonl_snapshot_atomic(path: &PathBuf, content: &str) -> Result<(), String> {
    write_message_store_text_atomic(path, "jsonl.tmp", content, "JSONL 快照")
}

#[cfg(test)]
mod jsonl_snapshot_conversation_block_tests {
    use super::*;

    fn text_message(id: &str, role: &str, text: &str) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            role: role.to_string(),
            created_at: "2026-04-25T00:00:00Z".to_string(),
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
        }
    }

    fn summary_seed_message(id: &str) -> ChatMessage {
        let mut message = text_message(id, "assistant", "summary");
        message.provider_meta = Some(serde_json::json!({
            "message_meta": {
                "kind": "summary_context_seed",
            },
            "runtime": {
                "shouldBeDroppedInOldBlocks": true,
            },
        }));
        message
    }

    fn legacy_media_message_json(id: &str, part_type: &str, mime: &str, stored: &str) -> String {
        serde_json::json!({
            "kind": "message",
            "message": {
                "id": id,
                "role": "user",
                "createdAt": "2026-04-25T00:00:00Z",
                "speakerAgentId": null,
                "parts": [{
                    "type": part_type,
                    "mime": mime,
                    "bytesBase64": stored,
                    "name": "legacy.bin",
                    "compressed": false
                }],
                "extraTextBlocks": [],
                "providerMeta": null,
                "toolCall": null,
                "mcpCall": null,
                "memeAnnotations": null
            }
        })
        .to_string()
    }

    #[test]
    fn decode_jsonl_snapshot_message_should_accept_legacy_image_and_audio_wire() {
        let image = decode_jsonl_snapshot_message(&legacy_media_message_json(
            "legacy-image",
            "image",
            "image/png",
            "@download:conversation-a/image.png",
        ))
        .expect("decode legacy image");
        let audio = decode_jsonl_snapshot_message(&legacy_media_message_json(
            "legacy-audio",
            "audio",
            "audio/webm",
            "@media:audio.webm",
        ))
        .expect("decode legacy audio");

        assert!(matches!(image.parts.first(), Some(MessagePart::Image { .. })));
        assert!(matches!(audio.parts.first(), Some(MessagePart::Audio { .. })));
    }

    #[test]
    fn repair_jsonl_snapshot_file_should_not_discard_legacy_media_lines() {
        let root = std::env::temp_dir().join(format!("eca-legacy-jsonl-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let path = root.join("000000.jsonl");
        let content = format!(
            "{}\n{}\n",
            legacy_media_message_json(
                "legacy-image",
                "image",
                "image/png",
                "@download:conversation-a/image.png",
            ),
            legacy_media_message_json(
                "legacy-audio",
                "audio",
                "audio/webm",
                "@media:audio.webm",
            )
        );
        std::fs::write(&path, content).expect("write legacy jsonl");

        let (report, discarded, duplicate) = repair_jsonl_snapshot_file(
            &path,
            &mut std::collections::HashSet::new(),
        )
        .expect("repair legacy jsonl");

        assert_eq!(discarded, 0);
        assert_eq!(duplicate, 0);
        assert_eq!(report.message_count, 2);
        let repaired = std::fs::read_to_string(&path).expect("read repaired jsonl");
        assert!(repaired.contains("legacy-image"));
        assert!(repaired.contains("legacy-audio"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn canonical_attachment_json_should_not_contain_legacy_binary_fields() {
        let mut message = text_message("canonical-attachment", "user", "look");
        message.parts.push(MessagePart::Attachment {
            path: "C:/attachments/a.png".to_string(),
            mime: "image/png".to_string(),
            name: "a.png".to_string(),
        });

        let json = encode_jsonl_snapshot_message(&message).expect("encode canonical attachment");

        assert!(json.contains("\"type\":\"attachment\""));
        assert!(json.contains("C:/attachments/a.png"));
        assert!(!json.contains("bytesBase64"));
        assert!(!json.contains("@download:"));
        assert!(!json.contains("@media:"));
    }

    #[test]
    fn slim_older_conversation_block_part_should_keep_text_reasoning_content() {
        let part = MessagePart::Text {
            text: "最终回答".to_string(),
            reasoning_content: Some("最终思考".to_string()),
        };

        let slimmed = slim_older_conversation_block_part(&part).expect("slimmed text part");

        match slimmed {
            MessagePart::Text {
                text,
                reasoning_content,
            } => {
                assert_eq!(text, "最终回答");
                assert_eq!(reasoning_content.as_deref(), Some("最终思考"));
            }
            other => panic!("unexpected part: {:?}", other),
        }
    }

    #[test]
    fn slim_older_conversation_block_message_should_keep_remote_origin_only() {
        let mut message = text_message("remote-user", "user", "旧消息");
        message.parts.push(MessagePart::Image {
            mime: "image/png".to_string(),
            bytes_base64: "iVBORw0KGgoAAA".to_string(),
            name: Some("inline.png".to_string()),
            compressed: false,
        });
        message.provider_meta = Some(serde_json::json!({
            "origin": {
                "kind": "remote_im",
                "channel_id": "channel-a",
                "contact_type": "group",
                "contact_id": "group-1",
                "contact_name": "测试群",
                "sender_id": "member-001",
                "sender_name": "群友甲"
            },
            "runtime": {
                "temporary": true
            }
        }));
        message.tool_call = Some(vec![serde_json::json!({"name": "heavy_tool"})]);
        message.mcp_call = Some(vec![serde_json::json!({"name": "heavy_mcp"})]);

        let slimmed = slim_older_conversation_block_message(&message);

        assert_eq!(slimmed.parts.len(), 1);
        assert!(slimmed.tool_call.is_none());
        assert!(slimmed.mcp_call.is_none());
        let meta = slimmed.provider_meta.expect("remote origin meta");
        assert_eq!(
            meta.pointer("/origin/sender_name").and_then(Value::as_str),
            Some("群友甲")
        );
        assert_eq!(
            meta.pointer("/origin/channel_id").and_then(Value::as_str),
            Some("channel-a")
        );
        assert!(meta.get("runtime").is_none());
    }

    #[test]
    fn conversation_blocks_should_split_at_compaction_seed_and_slim_only_when_archived() {
        let mut first = text_message("m1", "assistant", "first");
        first.extra_text_blocks.push("memory widget".to_string());
        first.tool_call = Some(vec![serde_json::json!({"name": "tool"})]);
        let mut messages = vec![first, summary_seed_message("s1")];
        messages.push(text_message("m2", "user", "second"));
        messages.push(summary_seed_message("s2"));
        messages.push(text_message("m3", "user", "third"));
        messages.push(summary_seed_message("s3"));
        messages.push(text_message("m4", "user", "latest"));

        let mut conversation = test_conversation_for_blocks(messages);
        let blocks = build_jsonl_snapshot_conversation_blocks_for_conversation(&conversation)
            .expect("build active blocks");

        assert_eq!(blocks.blocks.len(), 4);
        assert_eq!(blocks.blocks[0].block_file, "blocks/000000.jsonl");
        assert_eq!(blocks.index.items[1].block_id, Some(1));
        // 活跃会话不瘦身：块 0 保留 extraTextBlocks 与 toolCall
        assert!(blocks.blocks[0].content.contains("memory widget"));
        assert!(blocks.blocks[0].content.contains("\"toolCall\""));
        assert!(blocks.blocks[2].content.contains("summary_context_seed"));
        assert!(blocks.blocks[3].content.contains("latest"));

        // 归档会话全瘦身：extraTextBlocks 清空、toolCall 置 null
        conversation.status = "archived".to_string();
        let blocks = build_jsonl_snapshot_conversation_blocks_for_conversation(&conversation)
            .expect("build archived blocks");
        assert_eq!(blocks.blocks.len(), 4);
        assert!(blocks.blocks[0].content.contains("\"extraTextBlocks\":[]"));
        assert!(blocks.blocks[0].content.contains("\"toolCall\":null"));
        assert!(blocks.blocks[2].content.contains("summary_context_seed"));
        assert!(blocks.blocks[3].content.contains("latest"));
    }

    #[test]
    fn remote_im_conversation_without_compaction_should_keep_one_block_across_days() {
        let mut conversation = test_conversation_for_blocks(vec![
            text_message_at("m1", "user", "day 1", "2026-04-20T10:00:00Z"),
            text_message_at("m2", "assistant", "day 1 reply", "2026-04-20T10:01:00Z"),
            text_message_at("m3", "user", "day 2", "2026-04-21T10:00:00Z"),
        ]);
        conversation.conversation_kind = CONVERSATION_KIND_REMOTE_IM_CONTACT.to_string();

        let blocks = build_jsonl_snapshot_conversation_blocks_for_conversation(&conversation)
            .expect("build remote im blocks");

        assert_eq!(blocks.blocks.len(), 1);
        assert_eq!(blocks.index.items[0].block_id, Some(0));
        assert_eq!(blocks.index.items[1].block_id, Some(0));
        assert_eq!(blocks.index.items[2].block_id, Some(0));
    }

    #[test]
    fn remote_im_conversation_should_not_split_at_four_am_boundary() {
        let mut conversation = test_conversation_for_blocks(vec![
            text_message_at("m1", "user", "late night", "2026-04-20T19:00:00Z"),
            text_message_at("m2", "assistant", "before 4am local", "2026-04-20T19:30:00Z"),
            text_message_at("m3", "user", "after 4am local", "2026-04-20T20:10:00Z"),
        ]);
        conversation.conversation_kind = CONVERSATION_KIND_REMOTE_IM_CONTACT.to_string();

        let blocks = build_jsonl_snapshot_conversation_blocks_for_conversation(&conversation)
            .expect("build remote im day boundary blocks");

        assert_eq!(blocks.blocks.len(), 1);
        assert_eq!(blocks.index.items[0].block_id, Some(0));
        assert_eq!(blocks.index.items[1].block_id, Some(0));
        assert_eq!(blocks.index.items[2].block_id, Some(0));
    }

    #[test]
    fn remote_im_conversation_should_only_split_at_compaction_boundary() {
        let mut messages = vec![
            text_message_at("m1", "user", "day 1", "2026-04-20T10:00:00Z"),
            text_message_at("m2", "assistant", "day 2 before compaction", "2026-04-21T10:00:00Z"),
            summary_seed_message("s1"),
            text_message_at("m3", "user", "day 2 after compaction", "2026-04-21T10:01:00Z"),
        ];
        messages[2].created_at = "2026-04-21T10:00:30Z".to_string();
        let mut conversation = test_conversation_for_blocks(messages);
        conversation.conversation_kind = CONVERSATION_KIND_REMOTE_IM_CONTACT.to_string();

        let blocks = build_jsonl_snapshot_conversation_blocks_for_conversation(&conversation)
            .expect("build remote im compaction blocks");

        assert_eq!(blocks.blocks.len(), 2);
        assert_eq!(blocks.index.items[0].block_id, Some(0));
        assert_eq!(blocks.index.items[1].block_id, Some(0));
        assert_eq!(blocks.index.items[2].block_id, Some(1));
        assert_eq!(blocks.index.items[3].block_id, Some(1));
    }

    fn text_message_at(id: &str, role: &str, text: &str, created_at: &str) -> ChatMessage {
        let mut message = text_message(id, role, text);
        message.created_at = created_at.to_string();
        message
    }

    fn test_conversation_for_blocks(messages: Vec<ChatMessage>) -> Conversation {
        Conversation {
            id: "conversation-block-test".to_string(),
            title: "test".to_string(),
            agent_id: "agent".to_string(),
            department_id: String::new(),
            bound_conversation_id: None,
            parent_conversation_id: None,
            child_conversation_ids: Vec::new(),
            fork_message_cursor: None,
            unread_count: 0,
            conversation_kind: String::new(),
            root_conversation_id: None,
            delegate_id: None,
            created_at: "2026-04-20T00:00:00Z".to_string(),
            updated_at: "2026-04-20T00:00:00Z".to_string(),
            last_user_at: None,
            last_assistant_at: None,
            status: "active".to_string(),
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
}

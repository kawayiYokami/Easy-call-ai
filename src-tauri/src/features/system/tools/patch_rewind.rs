// ========== apply_patch rewind (by backupRecordId) ==========

/// 从被移除的消息中提取所有 backupRecordId，用于恢复或清理。
fn collect_backup_record_ids_from_messages(messages: &[ChatMessage]) -> Vec<String> {
    let mut ids = Vec::<String>::new();
    for message in messages {
        let Some(events) = message.tool_call.as_ref() else {
            continue;
        };
        for event in events {
            let role = event.get("role").and_then(Value::as_str).unwrap_or_default();
            if role != "tool" {
                continue;
            }
            let content = event
                .get("content")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            if content.is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<Value>(content) else {
                continue;
            };
            if let Some(id) = value
                .get("backupRecordId")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                ids.push(id.to_string());
            }
        }
    }
    ids
}

/// 按 backupRecordId 列表恢复文件（逆序执行，最后的补丁先撤回）。
/// 返回 (恢复文件总数, 有非LLM修改被覆盖的文件列表)。
fn try_undo_apply_patch_from_removed_messages(
    state: &AppState,
    removed_messages: &[ChatMessage],
) -> Result<(usize, Vec<String>), String> {
    let ids = collect_backup_record_ids_from_messages(removed_messages);
    if ids.is_empty() {
        return Ok((0, Vec::new()));
    }
    let mut total_restored = 0usize;
    let mut all_overwritten = Vec::<String>::new();
    // 逆序恢复：最后执行的补丁先撤回
    for record_id in ids.iter().rev() {
        let record_path = apply_patch_record_path(&state.data_path, record_id);
        if !record_path.exists() {
            eprintln!(
                "[apply_patch撤回] 跳过：备份记录不存在，record_id={}",
                record_id
            );
            continue;
        }
        let record = match apply_patch_read_backup_record(&record_path) {
            Ok(r) => r,
            Err(err) => {
                eprintln!(
                    "[apply_patch撤回] 跳过：读取备份记录失败，record_id={}，error={}",
                    record_id, err
                );
                continue;
            }
        };
        let (restored, overwritten) = match apply_patch_restore_backup_record(&state.data_path, &record) {
            Ok(result) => result,
            Err(err) => {
                eprintln!(
                    "[apply_patch撤回] 跳过：恢复备份失败，record_id={}，error={}",
                    record_id, err
                );
                continue;
            }
        };
        if let Err(err) = apply_patch_cleanup_backup_record_by_value(&state.data_path, &record) {
            eprintln!(
                "[apply_patch撤回] 警告：清理备份记录失败，record_id={}，error={}",
                record_id, err
            );
        }
        total_restored = total_restored.saturating_add(restored);
        all_overwritten.extend(overwritten);
    }
    Ok((total_restored, all_overwritten))
}

/// 按 backupRecordId 列表清理备份文件（不恢复，仅删除记录和 blob）。
/// 用于归档/裁剪/删除会话时清理不再需要的备份。
fn cleanup_backup_records_by_ids(data_path: &PathBuf, ids: &[String]) -> Result<usize, String> {
    let mut cleaned = 0usize;
    for record_id in ids {
        let record_path = apply_patch_record_path(data_path, record_id);
        if !record_path.exists() {
            continue;
        }
        let record = match apply_patch_read_backup_record(&record_path) {
            Ok(r) => r,
            Err(err) => {
                eprintln!(
                    "[apply_patch清理] 跳过：读取备份记录失败，record_id={}，error={}",
                    record_id, err
                );
                continue;
            }
        };
        if let Err(err) = apply_patch_cleanup_backup_record_by_value(data_path, &record) {
            eprintln!(
                "[apply_patch清理] 跳过：清理备份失败，record_id={}，error={}",
                record_id, err
            );
            continue;
        }
        cleaned = cleaned.saturating_add(1);
    }
    Ok(cleaned)
}

/// 从消息中提取 backupRecordId 并清理对应备份（归档/裁剪/删除时调用）。
fn cleanup_backup_records_from_messages(
    data_path: &PathBuf,
    messages: &[ChatMessage],
) -> Result<usize, String> {
    let ids = collect_backup_record_ids_from_messages(messages);
    if ids.is_empty() {
        return Ok(0);
    }
    cleanup_backup_records_by_ids(data_path, &ids)
}

#[cfg(test)]
mod rewind_apply_patch_tests {
    use super::*;
    use serde_json::json;

    fn make_temp_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("{prefix}-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir.canonicalize().expect("canonical temp dir")
    }

    fn make_message_with_tool_events(events: Vec<Value>) -> ChatMessage {
        ChatMessage {
            id: Uuid::new_v4().to_string(),
            role: "assistant".to_string(),
            created_at: now_iso(),
            speaker_agent_id: None,
            parts: vec![],
            extra_text_blocks: vec![],
            provider_meta: None,
            tool_call: Some(events),
            mcp_call: None,
        }
    }

    #[test]
    fn collect_should_extract_backup_record_ids_from_tool_results() {
        let events = vec![
            json!({
                "role": "tool",
                "tool_call_id": "call_1",
                "content": json!({"ok": true, "approved": true, "backupRecordId": "rec-001"}).to_string()
            }),
            json!({
                "role": "tool",
                "tool_call_id": "call_2",
                "content": json!({"ok": true, "approved": true, "backupRecordId": "rec-002"}).to_string()
            }),
        ];
        let messages = vec![make_message_with_tool_events(events)];
        let ids = collect_backup_record_ids_from_messages(&messages);
        assert_eq!(ids, vec!["rec-001", "rec-002"]);
    }

    #[test]
    fn collect_should_skip_missing_or_empty_backup_record_id() {
        let events = vec![
            json!({
                "role": "tool",
                "tool_call_id": "call_1",
                "content": json!({"ok": true, "approved": true}).to_string()
            }),
            json!({
                "role": "tool",
                "tool_call_id": "call_2",
                "content": json!({"ok": true, "approved": true, "backupRecordId": ""}).to_string()
            }),
            json!({
                "role": "tool",
                "tool_call_id": "call_3",
                "content": json!({"ok": true, "approved": true, "backupRecordId": "rec-003"}).to_string()
            }),
        ];
        let messages = vec![make_message_with_tool_events(events)];
        let ids = collect_backup_record_ids_from_messages(&messages);
        assert_eq!(ids, vec!["rec-003"]);
    }

    #[test]
    fn collect_should_skip_non_tool_role_events() {
        let events = vec![
            json!({
                "role": "assistant",
                "tool_calls": [{
                    "id": "call_1",
                    "function": { "name": "apply_patch", "arguments": "{}" }
                }]
            }),
        ];
        let messages = vec![make_message_with_tool_events(events)];
        let ids = collect_backup_record_ids_from_messages(&messages);
        assert!(ids.is_empty());
    }

    #[test]
    fn restore_should_work_with_valid_backup_record_by_id() {
        let root = make_temp_dir("rewind-id-restore");
        let data_path = root.join("config").join("app_data.json");
        std::fs::create_dir_all(root.join("config")).expect("create config");

        let file = root.join("a.txt");
        std::fs::write(&file, "new content").expect("seed file");

        let record_id = Uuid::new_v4().to_string();
        let record = ApplyPatchBackupRecord {
            record_id: record_id.clone(),
            session_id: "s1".to_string(),
            cwd: root.to_string_lossy().to_string(),
            fingerprint: "unused".to_string(),
            created_at: now_iso(),
            entries: vec![ApplyPatchBackupEntry {
                kind: ApplyPatchBackupKind::Update,
                path: file.to_string_lossy().to_string(),
                from_path: None,
                to_path: None,
                expected_current_content: Some("new content".to_string()),
                backup_blob_file: Some("blob-update.bin".to_string()),
            }],
        };

        // 写入记录和 blob
        std::fs::create_dir_all(apply_patch_temp_records_dir(&data_path)).expect("create records dir");
        std::fs::create_dir_all(apply_patch_temp_blobs_dir(&data_path)).expect("create blobs dir");
        let record_json = serde_json::to_string_pretty(&record).expect("serialize record");
        std::fs::write(apply_patch_record_path(&data_path, &record_id), record_json)
            .expect("write record");
        std::fs::write(
            apply_patch_blob_path(&data_path, "blob-update.bin"),
            "old content",
        )
        .expect("write blob");

        // 直接按 ID 恢复（模拟 try_undo 内部逻辑）
        let record_path = apply_patch_record_path(&data_path, &record_id);
        assert!(record_path.exists());
        let loaded = apply_patch_read_backup_record(&record_path).expect("read record");
        let (restored, overwritten) = apply_patch_restore_backup_record(&data_path, &loaded).expect("restore");
        apply_patch_cleanup_backup_record_by_value(&data_path, &loaded).expect("cleanup");

        assert_eq!(restored, 1);
        assert!(overwritten.is_empty());
        assert_eq!(
            std::fs::read_to_string(&file).expect("read restored"),
            "old content"
        );
        // 记录和 blob 应已清理
        assert!(!apply_patch_record_path(&data_path, &record_id).exists());
        assert!(!apply_patch_blob_path(&data_path, "blob-update.bin").exists());
    }

    #[test]
    fn cleanup_should_skip_missing_record_gracefully() {
        let root = make_temp_dir("rewind-id-missing");
        let data_path = root.join("config").join("app_data.json");
        std::fs::create_dir_all(root.join("config")).expect("create config");

        let cleaned = cleanup_backup_records_by_ids(&data_path, &["nonexistent-id".to_string()])
            .expect("should not error on missing record");
        assert_eq!(cleaned, 0);
    }

    #[test]
    fn cleanup_should_remove_records_and_blobs() {
        let root = make_temp_dir("rewind-id-cleanup");
        let data_path = root.join("config").join("app_data.json");
        std::fs::create_dir_all(root.join("config")).expect("create config");

        let record_id = Uuid::new_v4().to_string();
        let record = ApplyPatchBackupRecord {
            record_id: record_id.clone(),
            session_id: "s1".to_string(),
            cwd: root.to_string_lossy().to_string(),
            fingerprint: "unused".to_string(),
            created_at: now_iso(),
            entries: vec![ApplyPatchBackupEntry {
                kind: ApplyPatchBackupKind::Delete,
                path: root.join("deleted.txt").to_string_lossy().to_string(),
                from_path: None,
                to_path: None,
                expected_current_content: None,
                backup_blob_file: Some("blob-del.bin".to_string()),
            }],
        };

        std::fs::create_dir_all(apply_patch_temp_records_dir(&data_path)).expect("create records dir");
        std::fs::create_dir_all(apply_patch_temp_blobs_dir(&data_path)).expect("create blobs dir");
        let record_json = serde_json::to_string_pretty(&record).expect("serialize record");
        std::fs::write(apply_patch_record_path(&data_path, &record_id), &record_json)
            .expect("write record");
        std::fs::write(
            apply_patch_blob_path(&data_path, "blob-del.bin"),
            "deleted file content",
        )
        .expect("write blob");

        let cleaned = cleanup_backup_records_by_ids(&data_path, &[record_id.clone()])
            .expect("cleanup should succeed");
        assert_eq!(cleaned, 1);
        assert!(!apply_patch_record_path(&data_path, &record_id).exists());
        assert!(!apply_patch_blob_path(&data_path, "blob-del.bin").exists());
    }
}

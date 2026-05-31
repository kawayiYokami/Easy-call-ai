#[derive(Debug, Clone)]
enum ApplyPatchSafetyCheck {
    AutoApprove,
    AskUser { existing_paths: Vec<PathBuf> },
    Reject { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApplyPatchToolArgs {
    operations: Vec<ApplyPatchToolOpArgs>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApplyPatchToolOpArgs {
    action: String,
    path: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default, alias = "oldString")]
    old_string: Option<String>,
    #[serde(default, alias = "newString")]
    new_string: Option<String>,
    #[serde(default, alias = "replaceAll")]
    replace_all: Option<bool>,
    #[serde(default)]
    to: Option<String>,
}

#[derive(Debug, Clone)]
enum ApplyPatchOp {
    Add { path: String, content: String },
    Delete { path: String },
    Update { path: String, old_string: String, new_string: String, replace_all: bool },
    Move { path: String, to: String },
}

#[derive(Debug, Clone)]
enum ApplyPatchResolvedOp {
    Add { path: PathBuf, content: String },
    Delete { path: PathBuf },
    Update { from: PathBuf, to: Option<PathBuf>, old_string: String, new_string: String, replace_all: bool },
}

#[derive(Debug, Clone)]
struct ApplyPatchExecutionFailure {
    index: usize,
    op: String,
    path: Option<String>,
    message: String,
}

#[derive(Debug, Clone)]
struct ApplyPatchExecutionOutcome {
    changed: Vec<Value>,
    failure: Option<ApplyPatchExecutionFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ApplyPatchBackupKind {
    Add,
    Delete,
    Update,
    MoveUpdate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyPatchBackupEntry {
    kind: ApplyPatchBackupKind,
    path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    from_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    to_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expected_current_content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    backup_blob_file: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyPatchBackupRecord {
    record_id: String,
    session_id: String,
    cwd: String,
    fingerprint: String,
    created_at: String,
    entries: Vec<ApplyPatchBackupEntry>,
}

fn apply_patch_temp_root(data_path: &PathBuf) -> PathBuf {
    app_root_from_data_path(data_path).join("temp").join("apply_patch")
}

fn apply_patch_temp_records_dir(data_path: &PathBuf) -> PathBuf {
    apply_patch_temp_root(data_path).join("records")
}

fn apply_patch_temp_blobs_dir(data_path: &PathBuf) -> PathBuf {
    apply_patch_temp_root(data_path).join("blobs")
}

fn apply_patch_fingerprint(session_id: &str, cwd: &Path, input: &str) -> String {
    let mut hasher = sha2::Sha256::new();
    sha2::Digest::update(&mut hasher, session_id.as_bytes());
    sha2::Digest::update(&mut hasher, b"\n");
    sha2::Digest::update(&mut hasher, cwd.to_string_lossy().as_bytes());
    sha2::Digest::update(&mut hasher, b"\n");
    sha2::Digest::update(&mut hasher, input.as_bytes());
    sha2::Digest::finalize(hasher)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn apply_patch_record_path(data_path: &PathBuf, record_id: &str) -> PathBuf {
    apply_patch_temp_records_dir(data_path).join(format!("{record_id}.json"))
}

fn apply_patch_blob_path(data_path: &PathBuf, blob_file: &str) -> PathBuf {
    apply_patch_temp_blobs_dir(data_path).join(blob_file)
}

#[cfg(test)]
fn apply_patch_prepare_backup_record(
    data_path: &PathBuf,
    session_id: &str,
    cwd: &Path,
    input: &str,
    ops: &[ApplyPatchResolvedOp],
) -> Result<ApplyPatchBackupRecord, String> {
    std::fs::create_dir_all(apply_patch_temp_records_dir(data_path))
        .map_err(|err| format!("创建 apply_patch 记录目录失败：{err}"))?;
    std::fs::create_dir_all(apply_patch_temp_blobs_dir(data_path))
        .map_err(|err| format!("创建 apply_patch 备份目录失败：{err}"))?;

    let record_id = Uuid::new_v4().to_string();
    let mut entries = Vec::<ApplyPatchBackupEntry>::new();
    for op in ops {
        entries.push(apply_patch_prepare_backup_entry(data_path, op)?);
    }

    Ok(ApplyPatchBackupRecord {
        record_id,
        session_id: session_id.to_string(),
        cwd: cwd.to_string_lossy().to_string(),
        fingerprint: apply_patch_fingerprint(session_id, cwd, input),
        created_at: now_iso(),
        entries,
    })
}

fn apply_patch_empty_backup_record(
    data_path: &PathBuf,
    session_id: &str,
    cwd: &Path,
    input: &str,
) -> Result<ApplyPatchBackupRecord, String> {
    std::fs::create_dir_all(apply_patch_temp_records_dir(data_path))
        .map_err(|err| format!("创建 apply_patch 记录目录失败：{err}"))?;
    std::fs::create_dir_all(apply_patch_temp_blobs_dir(data_path))
        .map_err(|err| format!("创建 apply_patch 备份目录失败：{err}"))?;
    Ok(ApplyPatchBackupRecord {
        record_id: Uuid::new_v4().to_string(),
        session_id: session_id.to_string(),
        cwd: cwd.to_string_lossy().to_string(),
        fingerprint: apply_patch_fingerprint(session_id, cwd, input),
        created_at: now_iso(),
        entries: Vec::new(),
    })
}

fn apply_patch_prepare_backup_entry(
    data_path: &PathBuf,
    op: &ApplyPatchResolvedOp,
) -> Result<ApplyPatchBackupEntry, String> {
    match op {
        ApplyPatchResolvedOp::Add { path, content } => Ok(ApplyPatchBackupEntry {
            kind: ApplyPatchBackupKind::Add,
            path: path.to_string_lossy().to_string(),
            from_path: None,
            to_path: None,
            expected_current_content: Some(content.clone()),
            backup_blob_file: None,
        }),
        ApplyPatchResolvedOp::Delete { path } => {
            let raw = std::fs::read(path)
                .map_err(|_| format!("Delete File 失败，文件不存在：{}", path.to_string_lossy()))?;
            let metadata = std::fs::metadata(path)
                .map_err(|_| format!("Delete File 失败，文件不存在：{}", path.to_string_lossy()))?;
            if !metadata.is_file() {
                return Err(format!("Delete File 失败，目标不是文件：{}", path.to_string_lossy()));
            }
            let blob_file = format!("{}.bin", Uuid::new_v4());
            std::fs::write(apply_patch_blob_path(data_path, &blob_file), raw)
                .map_err(|err| format!("写入删除备份失败（{}）：{err}", path.to_string_lossy()))?;
            Ok(ApplyPatchBackupEntry {
                kind: ApplyPatchBackupKind::Delete,
                path: path.to_string_lossy().to_string(),
                from_path: None,
                to_path: None,
                expected_current_content: None,
                backup_blob_file: Some(blob_file),
            })
        }
        ApplyPatchResolvedOp::Update { from, to, old_string, new_string, replace_all } => {
            if old_string.is_empty() && new_string.is_empty() {
                let raw = std::fs::read(from)
                    .map_err(|_| format!("Move 操作失败，文件不存在：{}", from.to_string_lossy()))?;
                let blob_file = format!("{}.bin", Uuid::new_v4());
                std::fs::write(apply_patch_blob_path(data_path, &blob_file), raw)
                    .map_err(|err| format!("写入移动备份失败（{}）：{err}", from.to_string_lossy()))?;
                return Ok(ApplyPatchBackupEntry {
                    kind: ApplyPatchBackupKind::MoveUpdate,
                    path: to.as_ref().unwrap_or(from).to_string_lossy().to_string(),
                    from_path: to.as_ref().map(|_| from.to_string_lossy().to_string()),
                    to_path: to.as_ref().map(|dest| dest.to_string_lossy().to_string()),
                    expected_current_content: Some(String::new()),
                    backup_blob_file: Some(blob_file),
                });
            }
            let raw = std::fs::read(from)
                .map_err(|_| format!("Update 操作失败，文件不存在：{}", from.to_string_lossy()))?;
            let old_content = decode_text_file_bytes(&raw)
                .map_err(|err| format!("Update 操作失败，{}：{}", err, from.to_string_lossy()))?
                .text;
            let new_content =
                apply_patch_apply_update_with_line_ending_fallback(&old_content, old_string, new_string, *replace_all)?;
            let blob_file = format!("{}.bin", Uuid::new_v4());
            std::fs::write(apply_patch_blob_path(data_path, &blob_file), raw)
                .map_err(|err| format!("写入修改备份失败（{}）：{err}", from.to_string_lossy()))?;
            Ok(ApplyPatchBackupEntry {
                kind: if to.is_some() {
                    ApplyPatchBackupKind::MoveUpdate
                } else {
                    ApplyPatchBackupKind::Update
                },
                path: to.as_ref().unwrap_or(from).to_string_lossy().to_string(),
                from_path: to.as_ref().map(|_| from.to_string_lossy().to_string()),
                to_path: to.as_ref().map(|dest| dest.to_string_lossy().to_string()),
                expected_current_content: Some(new_content),
                backup_blob_file: Some(blob_file),
            })
        }
    }
}

fn apply_patch_store_backup_record(
    state: &AppState,
    record: &ApplyPatchBackupRecord,
) -> Result<PathBuf, String> {
    let path = apply_patch_record_path(&state.data_path, &record.record_id);
    let body = serde_json::to_vec_pretty(record)
        .map_err(|err| format!("序列化 apply_patch 恢复记录失败：{err}"))?;
    std::fs::write(&path, body).map_err(|err| {
        format!("写入 apply_patch 恢复记录失败（{}）：{err}", path.to_string_lossy())
    })?;
    Ok(path)
}

fn apply_patch_cleanup_backup_record_by_value(
    data_path: &PathBuf,
    record: &ApplyPatchBackupRecord,
) -> Result<(), String> {
    for entry in &record.entries {
        if let Some(blob_file) = entry.backup_blob_file.as_deref() {
            let blob_path = apply_patch_blob_path(data_path, blob_file);
            if blob_path.exists() {
                std::fs::remove_file(&blob_path).map_err(|err| {
                    format!("清理 apply_patch 备份文件失败（{}）：{err}", blob_path.to_string_lossy())
                })?;
            }
        }
    }
    let record_path = apply_patch_record_path(data_path, &record.record_id);
    if record_path.exists() {
        std::fs::remove_file(&record_path).map_err(|err| {
            format!("清理 apply_patch 恢复记录失败（{}）：{err}", record_path.to_string_lossy())
        })?;
    }
    Ok(())
}

fn apply_patch_cleanup_backup_entry(
    data_path: &PathBuf,
    entry: &ApplyPatchBackupEntry,
) -> Result<(), String> {
    if let Some(blob_file) = entry.backup_blob_file.as_deref() {
        let blob_path = apply_patch_blob_path(data_path, blob_file);
        if blob_path.exists() {
            std::fs::remove_file(&blob_path).map_err(|err| {
                format!("清理 apply_patch 备份文件失败（{}）：{err}", blob_path.to_string_lossy())
            })?;
        }
    }
    Ok(())
}

fn apply_patch_read_backup_record(path: &Path) -> Result<ApplyPatchBackupRecord, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|err| format!("读取 apply_patch 恢复记录失败（{}）：{err}", path.to_string_lossy()))?;
    serde_json::from_str::<ApplyPatchBackupRecord>(&raw)
        .map_err(|err| format!("解析 apply_patch 恢复记录失败（{}）：{err}", path.to_string_lossy()))
}

fn apply_patch_read_text_file(path: &Path) -> Result<String, String> {
    decode_text_file_from_path(path).map(|decoded| decoded.text)
}

fn apply_patch_write_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("创建目录失败（{}）：{err}", terminal_path_for_user(parent)))?;
    }
    Ok(())
}

/// 恢复备份记录中的所有文件。
/// 无论文件当前状态如何都强制恢复，返回 (恢复文件数, 有非LLM修改被覆盖的文件列表)。
fn apply_patch_restore_backup_record(
    data_path: &PathBuf,
    record: &ApplyPatchBackupRecord,
) -> Result<(usize, Vec<String>), String> {
    let mut restored = 0usize;
    let mut overwritten_files = Vec::<String>::new();
    for entry in record.entries.iter().rev() {
        match entry.kind {
            ApplyPatchBackupKind::Add => {
                // Add 操作的撤回 = 删除该文件
                let path = PathBuf::from(&entry.path);
                if !path.exists() {
                    // 文件已不存在，视为已撤回，跳过
                    continue;
                }
                let expected = entry.expected_current_content.as_deref().unwrap_or_default();
                if let Ok(current) = apply_patch_read_text_file(&path) {
                    if current != expected {
                        overwritten_files.push(terminal_path_for_user(&path));
                    }
                }
                if let Err(err) = std::fs::remove_file(&path) {
                    eprintln!(
                        "[apply_patch撤回] 删除文件失败（跳过）: path={}, error={}",
                        terminal_path_for_user(&path), err
                    );
                    continue;
                }
                restored = restored.saturating_add(1);
            }
            ApplyPatchBackupKind::Delete => {
                // Delete 操作的撤回 = 从 blob 恢复文件
                let path = PathBuf::from(&entry.path);
                let blob_file = match entry.backup_blob_file.as_deref() {
                    Some(f) => f,
                    None => continue,
                };
                let raw = match std::fs::read(apply_patch_blob_path(data_path, blob_file)) {
                    Ok(data) => data,
                    Err(err) => {
                        eprintln!(
                            "[apply_patch撤回] 读取删除备份失败（跳过）: path={}, error={}",
                            terminal_path_for_user(&path), err
                        );
                        continue;
                    }
                };
                if path.exists() {
                    // 文件已存在（用户或其他操作重新创建了），强制覆盖但记录
                    overwritten_files.push(terminal_path_for_user(&path));
                }
                let _ = apply_patch_write_parent_dir(&path);
                if let Err(err) = std::fs::write(&path, raw) {
                    eprintln!(
                        "[apply_patch撤回] 恢复文件失败（跳过）: path={}, error={}",
                        terminal_path_for_user(&path), err
                    );
                    continue;
                }
                restored = restored.saturating_add(1);
            }
            ApplyPatchBackupKind::Update => {
                // Update 操作的撤回 = 从 blob 恢复原始内容
                let path = PathBuf::from(&entry.path);
                let blob_file = match entry.backup_blob_file.as_deref() {
                    Some(f) => f,
                    None => continue,
                };
                let raw = match std::fs::read(apply_patch_blob_path(data_path, blob_file)) {
                    Ok(data) => data,
                    Err(err) => {
                        eprintln!(
                            "[apply_patch撤回] 读取修改备份失败（跳过）: path={}, error={}",
                            terminal_path_for_user(&path), err
                        );
                        continue;
                    }
                };
                if path.exists() {
                    let expected = entry.expected_current_content.as_deref().unwrap_or_default();
                    if let Ok(current) = apply_patch_read_text_file(&path) {
                        if current != expected {
                            overwritten_files.push(terminal_path_for_user(&path));
                        }
                    }
                }
                let _ = apply_patch_write_parent_dir(&path);
                if let Err(err) = std::fs::write(&path, raw) {
                    eprintln!(
                        "[apply_patch撤回] 恢复文件失败（跳过）: path={}, error={}",
                        terminal_path_for_user(&path), err
                    );
                    continue;
                }
                restored = restored.saturating_add(1);
            }
            ApplyPatchBackupKind::MoveUpdate => {
                // MoveUpdate 操作的撤回 = 从 blob 恢复到原始路径，删除移动后的路径
                let from_path = PathBuf::from(entry.from_path.as_deref().unwrap_or_default());
                let to_path = PathBuf::from(entry.to_path.as_deref().unwrap_or_default());
                let blob_file = match entry.backup_blob_file.as_deref() {
                    Some(f) => f,
                    None => continue,
                };
                let raw = match std::fs::read(apply_patch_blob_path(data_path, blob_file)) {
                    Ok(data) => data,
                    Err(err) => {
                        eprintln!(
                            "[apply_patch撤回] 读取移动备份失败（跳过）: path={}, error={}",
                            terminal_path_for_user(&to_path), err
                        );
                        continue;
                    }
                };
                // 检查移动后的文件是否有非 LLM 修改
                if to_path.exists() {
                    let expected = entry.expected_current_content.as_deref().unwrap_or_default();
                    if let Ok(current) = apply_patch_read_text_file(&to_path) {
                        if current != expected {
                            overwritten_files.push(terminal_path_for_user(&to_path));
                        }
                    }
                }
                // 检查原始路径是否已被占用
                if from_path.exists()
                    && terminal_normalize_for_access_check(&from_path)
                        != terminal_normalize_for_access_check(&to_path)
                {
                    overwritten_files.push(terminal_path_for_user(&from_path));
                }
                let _ = apply_patch_write_parent_dir(&from_path);
                if let Err(err) = std::fs::write(&from_path, raw) {
                    eprintln!(
                        "[apply_patch撤回] 恢复原始文件失败（跳过）: path={}, error={}",
                        terminal_path_for_user(&from_path), err
                    );
                    continue;
                }
                // 删除移动后的文件（如果和原始路径不同）
                if terminal_normalize_for_access_check(&from_path)
                    != terminal_normalize_for_access_check(&to_path)
                    && to_path.exists()
                {
                    let _ = std::fs::remove_file(&to_path);
                }
                restored = restored.saturating_add(1);
            }
        }
    }
    Ok((restored, overwritten_files))
}

fn clear_apply_patch_temp(data_path: &PathBuf) -> Result<(usize, usize), String> {
    let records_dir = apply_patch_temp_records_dir(data_path);
    let blobs_dir = apply_patch_temp_blobs_dir(data_path);
    let mut removed_records = 0usize;
    let mut removed_blobs = 0usize;
    for dir in [&records_dir, &blobs_dir] {
        std::fs::create_dir_all(dir)
            .map_err(|err| format!("创建 apply_patch temp 目录失败（{}）：{err}", dir.to_string_lossy()))?;
    }
    if let Ok(entries) = std::fs::read_dir(&records_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                std::fs::remove_file(&path).map_err(|err| {
                    format!("清理 apply_patch 记录失败（{}）：{err}", path.to_string_lossy())
                })?;
                removed_records = removed_records.saturating_add(1);
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir(&blobs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                std::fs::remove_file(&path).map_err(|err| {
                    format!("清理 apply_patch 备份失败（{}）：{err}", path.to_string_lossy())
                })?;
                removed_blobs = removed_blobs.saturating_add(1);
            }
        }
    }
    Ok((removed_records, removed_blobs))
}

fn apply_patch_tool_args_to_raw_json(args: &ApplyPatchToolArgs) -> Result<String, String> {
    serde_json::to_string(args).map_err(|err| format!("apply_patch 参数序列化失败：{err}"))
}

fn apply_patch_json_example() -> &'static str {
    r#"{"operations":[{"action":"update","path":"src/example.ts","old_string":"before","new_string":"after"}]}"#
}

fn apply_patch_ops_from_tool_args(args: ApplyPatchToolArgs) -> Result<Vec<ApplyPatchOp>, String> {
    if args.operations.is_empty() {
        return Err(apply_patch_format_error(
            "apply_patch 操作列表为空。至少需要一项操作。",
        ));
    }
    let mut ops = Vec::<ApplyPatchOp>::new();
    for (i, op) in args.operations.into_iter().enumerate() {
        match op.action.as_str() {
            "add" => {
                let Some(content) = op.content else {
                    return Err(apply_patch_format_error(format!(
                        r#"apply_patch 操作[{}] (add) 缺少 "content" 字段。add 操作必须提供文件内容。\n最小 add 示例：{}"#,
                        i,
                        apply_patch_operation_example("add")
                    )));
                };
                ops.push(ApplyPatchOp::Add { path: op.path, content });
            }
            "delete" => ops.push(ApplyPatchOp::Delete { path: op.path }),
            "update" => {
                let Some(old_string) = op.old_string else {
                    return Err(apply_patch_format_error(format!(
                        r#"apply_patch 操作[{}] (update) 缺少 "old_string" 字段。\n最小 update 示例：{}"#,
                        i,
                        apply_patch_operation_example("update")
                    )));
                };
                let Some(new_string) = op.new_string else {
                    return Err(apply_patch_format_error(format!(
                        r#"apply_patch 操作[{}] (update) 缺少 "new_string" 字段。\n最小 update 示例：{}"#,
                        i,
                        apply_patch_operation_example("update")
                    )));
                };
                if old_string == new_string {
                    return Err(apply_patch_format_error(format!(
                        "apply_patch 操作[{}] (update) old_string 和 new_string 完全相同。update 必须真的修改内容。\n最小 update 示例：{}",
                        i,
                        apply_patch_operation_example("update")
                    )));
                }
                ops.push(ApplyPatchOp::Update {
                    path: op.path,
                    old_string,
                    new_string,
                    replace_all: op.replace_all.unwrap_or(false),
                });
            }
            "move" => {
                let Some(to) = op.to else {
                    return Err(apply_patch_format_error(format!(
                        r#"apply_patch 操作[{}] (move) 缺少 "to" 字段。\n最小 move 示例：{}"#,
                        i,
                        apply_patch_operation_example("move")
                    )));
                };
                ops.push(ApplyPatchOp::Move { path: op.path, to });
            }
            other => {
                return Err(apply_patch_format_error(format!(
                    r#"apply_patch 操作[{}] 的 action "{}" 无效。必须是 "add"、"update"、"delete" 或 "move"。\n可参考最小 update 示例：{}"#,
                    i,
                    other,
                    apply_patch_operation_example("update")
                )));
            }
        }
    }
    Ok(ops)
}

fn apply_patch_operation_example(action: &str) -> &'static str {
    match action {
        "add" => r#"{"action":"add","path":"src/new.ts","content":"export const value = 1;\n"}"#,
        "update" => r#"{"action":"update","path":"src/example.ts","old_string":"before","new_string":"after","replace_all":false}"#,
        "delete" => r#"{"action":"delete","path":"src/old.ts"}"#,
        "move" => r#"{"action":"move","path":"src/old.ts","to":"src/new.ts"}"#,
        _ => r#"{"action":"update","path":"src/example.ts","old_string":"before","new_string":"after"}"#,
    }
}

fn apply_patch_format_error(message: impl AsRef<str>) -> String {
    format!(
        "{}\n\napply_patch 只支持 JSON 格式，不支持标准 git diff / unified diff。\n顶层格式示例：\n{}",
        message.as_ref(),
        apply_patch_json_example()
    )
}

fn apply_patch_preview_text(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}


#[cfg(target_os = "windows")]
fn apply_patch_has_windows_drive_prefix(path: &str) -> bool {
    terminal_has_windows_drive_prefix(path)
}

#[cfg(not(target_os = "windows"))]
fn apply_patch_has_windows_drive_prefix(_path: &str) -> bool {
    false
}

fn apply_patch_resolve_path(base: &Path, raw: &str) -> Result<PathBuf, String> {
    let normalized = normalize_terminal_path_input_for_current_platform(raw.trim());
    if normalized.is_empty() {
        return Err("补丁路径为空。".to_string());
    }
    let candidate = PathBuf::from(&normalized);
    let joined = if candidate.is_absolute() || apply_patch_has_windows_drive_prefix(&normalized) {
        candidate
    } else {
        base.join(candidate)
    };
    Ok(terminal_normalize_for_access_check(&joined))
}

fn apply_patch_resolve_ops(base: &Path, ops: Vec<ApplyPatchOp>) -> Result<Vec<ApplyPatchResolvedOp>, String> {
    let mut out = Vec::<ApplyPatchResolvedOp>::new();
    for op in ops {
        match op {
            ApplyPatchOp::Add { path, content } => out.push(ApplyPatchResolvedOp::Add {
                path: apply_patch_resolve_path(base, &path)?,
                content,
            }),
            ApplyPatchOp::Delete { path } => out.push(ApplyPatchResolvedOp::Delete {
                path: apply_patch_resolve_path(base, &path)?,
            }),
            ApplyPatchOp::Update { path, old_string, new_string, replace_all } => {
                let from = apply_patch_resolve_path(base, &path)?;
                out.push(ApplyPatchResolvedOp::Update { from, to: None, old_string, new_string, replace_all });
            }
            ApplyPatchOp::Move { path, to } => {
                let from = apply_patch_resolve_path(base, &path)?;
                let dest = apply_patch_resolve_path(base, &to)?;
                out.push(ApplyPatchResolvedOp::Update {
                    from, to: Some(dest),
                    old_string: String::new(), new_string: String::new(), replace_all: false,
                });
            }
        }
    }
    Ok(out)
}


fn apply_patch_collect_existing_paths(ops: &[ApplyPatchResolvedOp]) -> Vec<PathBuf> {
    let mut out = Vec::<PathBuf>::new();
    for op in ops {
        match op {
            ApplyPatchResolvedOp::Add { .. } => {}
            ApplyPatchResolvedOp::Delete { path } => out.push(path.clone()),
            ApplyPatchResolvedOp::Update { from, to, .. } => {
                out.push(from.clone());
                if let Some(next) = to {
                    out.push(next.clone());
                }
            }
        }
    }
    terminal_dedup_paths(out)
}

fn apply_patch_collect_target_paths(ops: &[ApplyPatchResolvedOp]) -> Vec<PathBuf> {
    let mut out = Vec::<PathBuf>::new();
    for op in ops {
        match op {
            ApplyPatchResolvedOp::Add { path, .. } | ApplyPatchResolvedOp::Delete { path } => {
                out.push(path.clone());
            }
            ApplyPatchResolvedOp::Update { from, to, .. } => {
                out.push(from.clone());
                if let Some(next) = to {
                    out.push(next.clone());
                }
            }
        }
    }
    terminal_dedup_paths(out)
}

fn apply_patch_operation_summary(ops: &[ApplyPatchResolvedOp]) -> String {
    let mut add_count = 0usize;
    let mut delete_count = 0usize;
    let mut update_count = 0usize;
    let mut move_count = 0usize;
    for op in ops {
        match op {
            ApplyPatchResolvedOp::Add { .. } => add_count += 1,
            ApplyPatchResolvedOp::Delete { .. } => delete_count += 1,
            ApplyPatchResolvedOp::Update { to, .. } => {
                if to.is_some() {
                    move_count += 1;
                } else {
                    update_count += 1;
                }
            }
        }
    }
    let total = ops.len();
    let mut parts = Vec::<String>::new();
    if add_count > 0 {
        parts.push(format!("新增 {add_count}"));
    }
    if update_count > 0 {
        parts.push(format!("修改 {update_count}"));
    }
    if delete_count > 0 {
        parts.push(format!("删除 {delete_count}"));
    }
    if move_count > 0 {
        parts.push(format!("重命名 {move_count}"));
    }
    if parts.is_empty() {
        "计划执行补丁操作。".to_string()
    } else {
        format!("计划执行 {total} 项补丁操作（{}）。", parts.join("，"))
    }
}

fn apply_patch_assess_safety(
    state: &AppState,
    _session_id: &str,
    _cwd: &Path,
    ops: &[ApplyPatchResolvedOp],
) -> ApplyPatchSafetyCheck {
    if ops.is_empty() {
        return ApplyPatchSafetyCheck::Reject {
            reason: "empty patch".to_string(),
        };
    }
    let mut target_paths = Vec::<PathBuf>::new();
    for op in ops {
        match op {
            ApplyPatchResolvedOp::Add { path, .. } => target_paths.push(path.clone()),
            ApplyPatchResolvedOp::Delete { path } => target_paths.push(path.clone()),
            ApplyPatchResolvedOp::Update { from, to, .. } => {
                target_paths.push(from.clone());
                if let Some(dest) = to {
                    target_paths.push(dest.clone());
                }
            }
        }
    }
    let target_paths = terminal_dedup_paths(target_paths);
    if target_paths.is_empty() {
        return ApplyPatchSafetyCheck::Reject {
            reason: "empty patch".to_string(),
        };
    }
    let mut accesses = Vec::<String>::new();
    for path in terminal_dedup_paths(target_paths) {
        let Some(workspace) = terminal_match_workspace_for_session_target(state, _session_id, &path)
            .unwrap_or(None)
        else {
            return ApplyPatchSafetyCheck::Reject {
                reason: format!(
                    "补丁路径未命中已配置工作目录：{}",
                    terminal_path_for_user(&path)
                ),
            };
        };
        accesses.push(workspace.access);
    }
    let effective_access = terminal_strictest_workspace_access(&accesses);
    if effective_access == SHELL_WORKSPACE_ACCESS_READ_ONLY {
        return ApplyPatchSafetyCheck::Reject {
            reason: "当前目录权限为只读，禁止执行补丁。".to_string(),
        };
    }
    if effective_access == SHELL_WORKSPACE_ACCESS_APPROVAL {
        return ApplyPatchSafetyCheck::AskUser {
            existing_paths: apply_patch_collect_existing_paths(ops)
                .into_iter()
                .filter(|path| path.exists())
                .collect::<Vec<_>>(),
        };
    }
    ApplyPatchSafetyCheck::AutoApprove
}

fn apply_patch_apply_update(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Result<String, String> {
    if old_string.is_empty() {
        return Ok(new_string.to_string());
    }
    let old_preview = apply_patch_preview_text(old_string, 300);
    if !content.contains(old_string) {
        let similar = apply_patch_similar_line_ranges(content, old_string, 1);
        let similar_hint = if similar.is_empty() {
            "最相似候选行范围：line 1，未找到可用候选。".to_string()
        } else {
            let rows = similar
                .into_iter()
                .map(|(start, end, summary)| {
                    format!(
                        "- {}，摘要：{}",
                        apply_patch_format_line_range((start, end)),
                        summary
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!("最相似候选行范围：\n{rows}")
        };
        return Err(format!(
            "apply_patch update 失败：old_string 在文件中未找到。\n{}\n请先重新读取目标文件，直接复制文件中的原文作为 old_string。\n如果你原本想提交标准 git diff，请改成 JSON update 操作。\n最小 update 示例：{}\nold_string 预览（前 300 字符）：\n{}",
            similar_hint,
            apply_patch_operation_example("update"),
            old_preview
        ));
    }
    if !replace_all {
        let count = content.matches(old_string).count();
        if count > 1 {
            let ranges = apply_patch_format_ranges_limited(
                apply_patch_exact_match_ranges(content, old_string),
                12,
            );
            return Err(format!(
                "apply_patch update 失败：old_string 在文件中出现了 {} 次，但 replace_all 为 false。\n命中行范围：{}\n请改用以下两种方式之一：\n1. 扩大 old_string，上下多带几行稳定上下文，使其只命中 1 处。\n2. 如果你确实要全部替换，再设置 replace_all: true。\n最小 update 示例：{}\nold_string 预览（前 300 字符）：\n{}",
                count,
                ranges,
                apply_patch_operation_example("update"),
                old_preview
            ));
        }
    }
    let result = if replace_all {
        content.replace(old_string, new_string)
    } else {
        content.replacen(old_string, new_string, 1)
    };
    if result == content {
        return Err("apply_patch update 失败：替换后文件内容未变化。".to_string());
    }
    Ok(result)
}

fn apply_patch_apply_update_with_line_ending_fallback(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Result<String, String> {
    match apply_patch_apply_update(content, old_string, new_string, replace_all) {
        Ok(value) => Ok(value),
        Err(original_err) => {
            let Some((normalized_content, normalized_old, normalized_new, newline)) =
                apply_patch_prepare_line_ending_fallback(content, old_string, new_string)
            else {
                return Err(original_err);
            };
            apply_patch_apply_update(&normalized_content, &normalized_old, &normalized_new, replace_all)
                .map(|value| apply_patch_normalized_lf_to_newline(&value, newline))
                .map_err(|_| original_err)
        }
    }
}

fn apply_patch_prepare_line_ending_fallback(
    content: &str,
    old_string: &str,
    new_string: &str,
) -> Option<(String, String, String, &'static str)> {
    let newline = if content.contains("\r\n") {
        "\r\n"
    } else if content.contains('\r') {
        "\r"
    } else if old_string.contains('\r') || new_string.contains('\r') {
        "\n"
    } else {
        return None;
    };
    let normalized_content = apply_patch_normalize_to_lf(content);
    let normalized_old = apply_patch_normalize_to_lf(old_string);
    if normalized_old == old_string && normalized_content == content {
        return None;
    }
    Some((
        normalized_content,
        normalized_old,
        apply_patch_normalize_to_lf(new_string),
        newline,
    ))
}

fn apply_patch_normalize_to_lf(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

fn apply_patch_normalized_lf_to_newline(value: &str, newline: &str) -> String {
    if newline == "\n" {
        value.to_string()
    } else {
        value.replace('\n', newline)
    }
}

fn apply_patch_line_number_at_byte(content: &str, offset: usize) -> usize {
    content
        .as_bytes()
        .get(..offset.min(content.len()))
        .unwrap_or_default()
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
        .saturating_add(1)
}

fn apply_patch_match_line_range(content: &str, start: usize, len: usize) -> (usize, usize) {
    let end = start.saturating_add(len).min(content.len());
    let adjusted_end = if end > start && content.as_bytes().get(end.saturating_sub(1)) == Some(&b'\n') {
        end.saturating_sub(1)
    } else {
        end
    };
    (
        apply_patch_line_number_at_byte(content, start),
        apply_patch_line_number_at_byte(content, adjusted_end),
    )
}

fn apply_patch_format_line_range((start, end): (usize, usize)) -> String {
    if start == end {
        format!("line {start}")
    } else {
        format!("lines {start}-{end}")
    }
}

fn apply_patch_exact_match_ranges(content: &str, needle: &str) -> Vec<(usize, usize)> {
    content
        .match_indices(needle)
        .map(|(start, _)| apply_patch_match_line_range(content, start, needle.len()))
        .collect()
}

fn apply_patch_format_ranges_limited(ranges: Vec<(usize, usize)>, limit: usize) -> String {
    let total = ranges.len();
    let mut out = ranges
        .into_iter()
        .take(limit)
        .map(apply_patch_format_line_range)
        .collect::<Vec<_>>()
        .join(", ");
    if total > limit {
        out.push_str(&format!(" ... 另有 {} 处", total - limit));
    }
    out
}

fn apply_patch_compact_line_summary(value: &str, limit: usize) -> String {
    let compact = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if compact.is_empty() {
        return "<空行>".to_string();
    }
    apply_patch_preview_text(&compact, limit)
}

fn apply_patch_normalize_for_similarity(value: &str) -> String {
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .to_lowercase()
}

fn apply_patch_levenshtein_distance(left: &str, right: &str) -> usize {
    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    if left_chars.is_empty() {
        return right_chars.len();
    }
    if right_chars.is_empty() {
        return left_chars.len();
    }
    let mut prev = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut curr = vec![0usize; right_chars.len() + 1];
    for (i, left_ch) in left_chars.iter().enumerate() {
        curr[0] = i + 1;
        for (j, right_ch) in right_chars.iter().enumerate() {
            let cost = usize::from(left_ch != right_ch);
            curr[j + 1] = (curr[j] + 1).min(prev[j + 1] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[right_chars.len()]
}

fn apply_patch_similarity_score(left: &str, right: &str) -> f64 {
    let left = apply_patch_normalize_for_similarity(left);
    let right = apply_patch_normalize_for_similarity(right);
    let max_len = left.chars().count().max(right.chars().count());
    if max_len == 0 {
        return 1.0;
    }
    let distance = apply_patch_levenshtein_distance(&left, &right);
    1.0 - (distance as f64 / max_len as f64)
}

fn apply_patch_similar_line_ranges(content: &str, old_string: &str, limit: usize) -> Vec<(usize, usize, String)> {
    const MAX_FILE_LINES_FOR_SIMILARITY: usize = 5000;
    const MIN_LINE_OVERLAP_RATIO: f64 = 0.9;

    let lines = content.lines().collect::<Vec<_>>();
    if lines.is_empty() || lines.len() > MAX_FILE_LINES_FOR_SIMILARITY {
        return Vec::new();
    }
    let old_line_count = old_string.lines().count().max(1);
    let old_lines_set: std::collections::HashSet<&str> = old_string.lines().collect();
    let min_overlap = ((old_line_count as f64) * MIN_LINE_OVERLAP_RATIO).ceil() as usize;
    let mut candidates = Vec::<(f64, usize, usize, String)>::new();
    for window_len in [
        old_line_count.saturating_sub(1).max(1),
        old_line_count,
        old_line_count.saturating_add(1),
    ] {
        if window_len > lines.len() {
            continue;
        }
        for start in 0..=lines.len() - window_len {
            let window = &lines[start..start + window_len];
            let overlap = window.iter().filter(|l| old_lines_set.contains(*l)).count();
            if overlap < min_overlap {
                continue;
            }
            let preview = window.join("\n");
            let score = apply_patch_similarity_score(old_string, &preview);
            candidates.push((score, start + 1, start + window_len, preview));
        }
    }
    candidates.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.1.cmp(&right.1))
            .then_with(|| left.2.cmp(&right.2))
    });
    candidates.dedup_by(|left, right| left.1 == right.1 && left.2 == right.2);
    candidates
        .into_iter()
        .take(limit)
        .map(|(_, start, end, preview)| (start, end, apply_patch_compact_line_summary(&preview, 80)))
        .collect()
}

fn apply_patch_build_preview(ops: &[ApplyPatchResolvedOp]) -> Result<String, String> {
    let mut lines = Vec::<String>::new();
    for op in ops {
        match op {
            ApplyPatchResolvedOp::Add { path, .. } => lines.push(format!("  add    {}", terminal_path_for_user(path))),
            ApplyPatchResolvedOp::Delete { path } => lines.push(format!("  delete {}", terminal_path_for_user(path))),
            ApplyPatchResolvedOp::Update { from, to, .. } => {
                if let Some(dest) = to {
                    lines.push(format!("  move   {} -> {}", terminal_path_for_user(from), terminal_path_for_user(dest)));
                } else {
                    lines.push(format!("  update {}", terminal_path_for_user(from)));
                }
            }
        }
    }
    Ok(lines.join("\n"))
}

fn apply_patch_op_name(op: &ApplyPatchResolvedOp) -> &'static str {
    match op {
        ApplyPatchResolvedOp::Add { .. } => "add",
        ApplyPatchResolvedOp::Delete { .. } => "delete",
        ApplyPatchResolvedOp::Update { old_string, new_string, to, .. } => {
            if old_string.is_empty() && new_string.is_empty() && to.is_some() {
                "move"
            } else {
                "update"
            }
        }
    }
}

fn apply_patch_op_path(op: &ApplyPatchResolvedOp) -> Option<String> {
    match op {
        ApplyPatchResolvedOp::Add { path, .. } => Some(terminal_path_for_user(path)),
        ApplyPatchResolvedOp::Delete { path } => Some(terminal_path_for_user(path)),
        ApplyPatchResolvedOp::Update { from, .. } => Some(terminal_path_for_user(from)),
    }
}

async fn apply_patch_execute_single_op(op: &ApplyPatchResolvedOp) -> Result<Value, String> {
    match op {
        ApplyPatchResolvedOp::Add { path, content } => {
            if path.exists() {
                return Err(format!("Add File 失败，文件已存在：{}", path.to_string_lossy()));
            }
            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|err| format!("创建目录失败（{}）：{err}", parent.to_string_lossy()))?;
            }
            tokio::fs::write(path, content.as_bytes())
                .await
                .map_err(|err| format!("写入文件失败（{}）：{err}", path.to_string_lossy()))?;
            Ok(serde_json::json!({
                "op": "add",
                "path": terminal_path_for_user(path),
            }))
        }
        ApplyPatchResolvedOp::Delete { path } => {
            let metadata = tokio::fs::metadata(path)
                .await
                .map_err(|_| format!("Delete File 失败，文件不存在：{}", path.to_string_lossy()))?;
            if !metadata.is_file() {
                return Err(format!("Delete File 失败，目标不是文件：{}", path.to_string_lossy()));
            }
            tokio::fs::remove_file(path)
                .await
                .map_err(|err| format!("删除文件失败（{}）：{err}", path.to_string_lossy()))?;
            Ok(serde_json::json!({
                "op": "delete",
                "path": terminal_path_for_user(path),
            }))
        }
        ApplyPatchResolvedOp::Update { from, to, old_string, new_string, replace_all } => {
            if old_string.is_empty() && new_string.is_empty() {
                if let Some(dest) = to {
                    let raw_move = tokio::fs::read(from).await
                        .map_err(|_| format!("Move 操作失败，文件不存在：{}", from.to_string_lossy()))?;
                    let _ = decode_text_file_bytes(&raw_move)
                        .map_err(|err| format!("Move 操作失败，{}：{}", err, from.to_string_lossy()))?;
                    if let Some(parent) = dest.parent() {
                        tokio::fs::create_dir_all(parent)
                            .await
                            .map_err(|err| format!("创建目录失败（{}）：{err}", parent.to_string_lossy()))?;
                    }
                    if dest.exists() && terminal_normalize_for_access_check(dest) != terminal_normalize_for_access_check(from) {
                        return Err(format!("重命名目标已存在：{}", dest.to_string_lossy()));
                    }
                    tokio::fs::rename(from, dest).await
                        .map_err(|err| format!("重命名失败（{} -> {}）：{err}", from.to_string_lossy(), dest.to_string_lossy()))?;
                    return Ok(serde_json::json!({ "op": "move", "from": terminal_path_for_user(from), "to": terminal_path_for_user(dest) }));
                }
                return Ok(serde_json::json!({ "op": "move", "path": terminal_path_for_user(from), "skipped": true }));
            }
            let raw = tokio::fs::read(from)
                .await
                .map_err(|_| format!("Update 操作失败，文件不存在：{}", from.to_string_lossy()))?;
            let decoded = decode_text_file_bytes(&raw)
                .map_err(|err| format!("Update 操作失败，{}：{}", err, from.to_string_lossy()))?;
            let new_content =
                apply_patch_apply_update_with_line_ending_fallback(&decoded.text, old_string, new_string, *replace_all)?;
            let encoded = decoded
                .encode_like_original(&new_content)
                .map_err(|err| format!("Update 操作失败，{}：{}", err, from.to_string_lossy()))?;
            tokio::fs::write(from, encoded)
                .await
                .map_err(|err| format!("更新文件失败（{}）：{err}", from.to_string_lossy()))?;
            Ok(serde_json::json!({ "op": "update", "path": terminal_path_for_user(from) }))
        }
    }
}

async fn apply_patch_execute_ops(
    data_path: &PathBuf,
    record: &mut ApplyPatchBackupRecord,
    ops: &[ApplyPatchResolvedOp],
) -> Result<ApplyPatchExecutionOutcome, String> {
    let mut changed = Vec::<Value>::new();
    for (index, op) in ops.iter().enumerate() {
        let entry = match apply_patch_prepare_backup_entry(data_path, op) {
            Ok(value) => value,
            Err(message) => {
                // prepare_backup_entry 返回 Err 有两种情况：
                // 1. 操作级失败（文件不存在等）：此时无 blob 被创建，changed 为空
                // 2. 系统级故障（写 blob I/O 失败）：部分 blob 可能已写入
                // 两种情况都返回 Ok(failure)，调用方根据 changed/entries 是否为空决定是否保留备份
                return Ok(ApplyPatchExecutionOutcome {
                    changed,
                    failure: Some(ApplyPatchExecutionFailure {
                        index,
                        op: apply_patch_op_name(op).to_string(),
                        path: apply_patch_op_path(op),
                        message,
                    }),
                });
            }
        };
        match apply_patch_execute_single_op(op).await {
            Ok(value) => {
                record.entries.push(entry);
                changed.push(value);
            }
            Err(message) => {
                let _ = apply_patch_cleanup_backup_entry(data_path, &entry);
                return Ok(ApplyPatchExecutionOutcome {
                    changed,
                    failure: Some(ApplyPatchExecutionFailure {
                        index,
                        op: apply_patch_op_name(op).to_string(),
                        path: apply_patch_op_path(op),
                        message,
                    }),
                });
            }
        }
    }
    Ok(ApplyPatchExecutionOutcome { changed, failure: None })
}

async fn builtin_apply_patch(
    state: &AppState,
    session_id: &str,
    args: ApplyPatchToolArgs,
) -> Result<Value, String> {
    let normalized_session = normalize_terminal_tool_session_id(session_id);
    let cwd = resolve_terminal_cwd(state, &normalized_session, None)?;
    let raw_input = apply_patch_tool_args_to_raw_json(&args)?;
    let parsed = apply_patch_ops_from_tool_args(args)?;
    let resolved = apply_patch_resolve_ops(&cwd, parsed)?;
    let preview = apply_patch_build_preview(&resolved)?;
    let target_paths = apply_patch_collect_target_paths(&resolved);
    let existing_paths = apply_patch_collect_existing_paths(&resolved);
    let summary = apply_patch_operation_summary(&resolved);

    let safety = apply_patch_assess_safety(state, &normalized_session, &cwd, &resolved);
    let mut smart_review_unavailable_notice = None::<String>;
    let mut smart_review_handled = false;
    let mut smart_review_history = None::<Value>;
    if matches!(safety, ApplyPatchSafetyCheck::AskUser { .. }) {
        if let Some(review_api_config_id) = current_tool_review_api_config_id(state)? {
            let context = serde_json::json!({
                "cwd": terminal_path_for_user(&cwd),
                "operation_summary": summary.clone(),
                "target_paths": terminal_smart_review_paths(&target_paths),
                "existing_paths": terminal_smart_review_paths(&existing_paths),
                "patch_preview": preview.clone(),
            });
            match run_tool_smart_review(
                state,
                &review_api_config_id,
                "apply_patch",
                "Tool safety review",
                context,
            )
            .await
            {
                Ok(TerminalSmartReviewOutcome::Decision(review)) => {
                    smart_review_history = Some(serde_json::json!({
                        "kind": "decision",
                        "allow": review.allow,
                        "reviewOpinion": review.review_opinion,
                        "modelName": review.model_name,
                    }));
                    if !review.allow {
                        let mut lines = vec!["智能评估建议先由你确认后再执行。".to_string()];
                        if !review.review_opinion.is_empty() {
                            lines.push(format!("评估意见: {}", review.review_opinion));
                        }
                        if !state
                            .delegate_active_ids
                            .lock()
                            .map(|ids| ids.is_empty())
                            .unwrap_or(false)
                        {
                            return Ok(serde_json::json!({
                                "ok": false,
                                "approved": false,
                                "blockedReason": "delegate_denied_ai_reviewed_patch",
                                "message": "子代理工具调用被自动拒绝（智能评估不通过）。",
                                "toolReview": smart_review_history.clone(),
                                "cwd": terminal_path_for_user(&cwd),
                            }));
                        }
                        let approved = match terminal_request_user_approval(
                            state,
                            "工具智能评估",
                            &lines.join("\n"),
                            &normalized_session,
                            "ai_tool_review",
                            Some("apply_patch"),
                            None,
                            Some(&preview),
                            Some(&cwd),
                            None,
                            None,
                            None,
                            &existing_paths,
                            &target_paths,
                            (!review.review_opinion.is_empty()).then_some(review.review_opinion.as_str()),
                            (!review.model_name.is_empty()).then_some(review.model_name.as_str()),
                        )
                        .await
                        {
                            Ok(v) => v,
                            Err(err) => return Err(err),
                        };
                        if !approved {
                            return Ok(serde_json::json!({
                                "ok": false,
                                "approved": false,
                                "blockedReason": "user_denied_ai_reviewed_patch",
                                "message": "用户拒绝了智能评估后的补丁执行。",
                                "toolReview": smart_review_history.clone(),
                                "cwd": terminal_path_for_user(&cwd),
                            }));
                        }
                    }
                    smart_review_handled = true;
                }
                Ok(TerminalSmartReviewOutcome::RawJson {
                    raw_json,
                    model_name,
                }) => {
                    let review_note =
                        "当前工具评估模型返回了不符合约定的结果，请直接查看原始返回内容后决定是否执行。";
                    smart_review_history = Some(serde_json::json!({
                        "kind": "raw_json",
                        "allow": false,
                        "reviewOpinion": review_note,
                        "modelName": model_name,
                        "rawContent": raw_json,
                    }));
                    if !state
                        .delegate_active_ids
                        .lock()
                        .map(|ids| ids.is_empty())
                        .unwrap_or(false)
                    {
                        return Ok(serde_json::json!({
                            "ok": false,
                            "approved": false,
                            "blockedReason": "delegate_denied_ai_review_raw_patch",
                            "message": "子代理工具调用被自动拒绝（智能评估返回了不符合约定的结果）。",
                            "toolReview": smart_review_history.clone(),
                            "cwd": terminal_path_for_user(&cwd),
                        }));
                    }
                    let approved = match terminal_request_user_approval(
                        state,
                        "工具智能评估",
                        review_note,
                        &normalized_session,
                        "ai_tool_review_raw_json",
                        Some("apply_patch"),
                        Some(review_note),
                        Some(&raw_json),
                        Some(&cwd),
                        None,
                        None,
                        None,
                        &existing_paths,
                        &target_paths,
                        Some(review_note),
                        Some(model_name.as_str()),
                    )
                    .await
                    {
                        Ok(v) => v,
                        Err(err) => return Err(err),
                    };
                    if !approved {
                        return Ok(serde_json::json!({
                            "ok": false,
                            "approved": false,
                            "blockedReason": "user_denied_ai_review_raw_patch",
                            "message": "用户拒绝了查看原始评估结果后的补丁执行。",
                            "toolReview": smart_review_history.clone(),
                            "cwd": terminal_path_for_user(&cwd),
                        }));
                    }
                    smart_review_handled = true;
                }
                Err(err) => {
                    runtime_log_warn(format!(
                        "[补丁审查] 失败 session={} err={:?}",
                        normalized_session, err
                    ));
                    smart_review_unavailable_notice =
                        Some("当前评估模型不可用，已降级为本地规则评估。".to_string());
                }
            }
        }
    }

    if !smart_review_handled {
        match safety {
            ApplyPatchSafetyCheck::Reject { reason } => {
                return Ok(serde_json::json!({
                    "ok": false,
                    "approved": false,
                    "blockedReason": "rejected",
                    "message": reason,
                    "cwd": terminal_path_for_user(&cwd),
                }));
            }
            ApplyPatchSafetyCheck::AskUser { existing_paths } => {
                let mut lines = vec![
                    "该补丁将在用户工具区执行，是否批准本次修改？".to_string(),
                    format!("会话: {}", normalized_session),
                    format!("工作目录: {}", terminal_path_for_user(&cwd)),
                    "命中已有文件：".to_string(),
                ];
                if let Some(notice) = &smart_review_unavailable_notice {
                    lines.insert(0, notice.clone());
                }
                if existing_paths.is_empty() {
                    lines.push("- 未识别到已存在文件，但该区域仍需确认。".to_string());
                } else {
                    for path in existing_paths.iter().take(8) {
                        lines.push(format!("- {}", terminal_path_for_user(path)));
                    }
                }
                if !state
                    .delegate_active_ids
                    .lock()
                    .map(|ids| ids.is_empty())
                    .unwrap_or(false)
                {
                    return Ok(serde_json::json!({
                        "ok": false,
                        "approved": false,
                        "blockedReason": "delegate_denied_apply_patch",
                        "message": "子代理工具调用被自动拒绝（补丁执行需要审批）。",
                        "cwd": terminal_path_for_user(&cwd),
                    }));
                }
                let approved = match terminal_request_user_approval(
                    state,
                    "补丁执行审批",
                    &lines.join("\n"),
                    &normalized_session,
                    "apply_patch_workspace_write",
                    Some("apply_patch"),
                    Some(&summary),
                    Some(&preview),
                    Some(&cwd),
                    None,
                    None,
                    smart_review_unavailable_notice
                        .as_deref()
                        .or(Some("用户工具区修改需要审批")),
                    &existing_paths,
                    &target_paths,
                    None,
                    None,
                )
                .await
                {
                    Ok(v) => v,
                    Err(err) => return Err(err),
                };
                if !approved {
                    return Ok(serde_json::json!({
                        "ok": false,
                        "approved": false,
                        "blockedReason": "user_denied_apply_patch",
                        "message": "用户拒绝了本次补丁执行。",
                        "cwd": terminal_path_for_user(&cwd),
                    }));
                }
            }
            ApplyPatchSafetyCheck::AutoApprove => {
                if let Some(notice) = &smart_review_unavailable_notice {
                    if !state
                        .delegate_active_ids
                        .lock()
                        .map(|ids| ids.is_empty())
                        .unwrap_or(false)
                    {
                        return Ok(serde_json::json!({
                            "ok": false,
                            "approved": false,
                            "blockedReason": "delegate_denied_apply_patch_after_review_fallback",
                            "message": "子代理工具调用被自动拒绝（审查模型不可用，降级后仍需审批）。",
                            "cwd": terminal_path_for_user(&cwd),
                        }));
                    }
                    let approved = match terminal_request_user_approval(
                        state,
                        "补丁执行审批",
                        notice,
                        &normalized_session,
                        "apply_patch_workspace_write",
                        Some("apply_patch"),
                        Some(&summary),
                        Some(&preview),
                        Some(&cwd),
                        None,
                        None,
                        Some(notice.as_str()),
                        &existing_paths,
                        &target_paths,
                        None,
                        None,
                    )
                    .await
                    {
                        Ok(v) => v,
                        Err(err) => return Err(err),
                    };
                    if !approved {
                        return Ok(serde_json::json!({
                            "ok": false,
                            "approved": false,
                            "blockedReason": "user_denied_apply_patch_after_review_fallback",
                            "message": "用户拒绝了降级后的补丁执行。",
                            "cwd": terminal_path_for_user(&cwd),
                        }));
                    }
                }
            }
        }
    }

    let mut backup_record = apply_patch_empty_backup_record(
        &state.data_path,
        &normalized_session,
        &cwd,
        &raw_input,
    )?;
    let started = std::time::Instant::now();
    let outcome = match apply_patch_execute_ops(&state.data_path, &mut backup_record, &resolved).await {
        Ok(value) => value,
        Err(err) => {
            let _ = apply_patch_cleanup_backup_record_by_value(&state.data_path, &backup_record);
            return Err(err);
        }
    };
    let record_path = if backup_record.entries.is_empty() {
        None
    } else {
        match apply_patch_store_backup_record(state, &backup_record) {
            Ok(path) => Some(path),
            Err(err) => {
                let _ = apply_patch_cleanup_backup_record_by_value(&state.data_path, &backup_record);
                return Err(err);
            }
        }
    };
    let elapsed_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
    if let Some(failure) = outcome.failure {
        eprintln!(
            "[补丁执行] 失败 task=apply_patch session={} changed={} failed_index={} elapsed_ms={} record_id={}",
            normalized_session,
            outcome.changed.len(),
            failure.index,
            elapsed_ms,
            backup_record.record_id
        );
        return Ok(serde_json::json!({
            "ok": false,
            "approved": true,
            "partial": !outcome.changed.is_empty(),
            "toolReview": smart_review_history,
            "cwd": terminal_path_for_user(&cwd),
            "changed": outcome.changed,
            "changedCount": outcome.changed.len(),
            "failed": {
                "index": failure.index,
                "op": failure.op,
                "path": failure.path,
                "message": failure.message,
            },
            "elapsedMs": elapsed_ms,
            "backupRecordId": record_path.as_ref().map(|_| backup_record.record_id.clone()),
            "backupFingerprint": record_path.as_ref().map(|_| backup_record.fingerprint.clone()),
            "backupRecordPath": record_path.as_ref().map(|path| terminal_path_for_user(path)),
        }));
    }
    eprintln!(
        "[补丁执行] 完成 task=apply_patch session={} changed={} elapsed_ms={} record_id={}",
        normalized_session,
        outcome.changed.len(),
        elapsed_ms,
        backup_record.record_id
    );
    Ok(serde_json::json!({
        "ok": true,
        "approved": true,
        "toolReview": smart_review_history,
        "cwd": terminal_path_for_user(&cwd),
        "changed": outcome.changed,
        "changedCount": outcome.changed.len(),
        "elapsedMs": elapsed_ms,
        "backupRecordId": record_path.as_ref().map(|_| backup_record.record_id.clone()),
        "backupFingerprint": record_path.as_ref().map(|_| backup_record.fingerprint.clone()),
        "backupRecordPath": record_path.as_ref().map(|path| terminal_path_for_user(path)),
    }))
}

#[cfg(test)]
mod apply_patch_tool_tests {
    use super::*;

    fn absolute_user_path(path: &Path) -> String {
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .to_string()
    }

    fn make_temp_data_path(prefix: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("{prefix}-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("config")).expect("create config dir");
        root.join("config").join("app_data.json")
    }

    #[test]
    fn tool_args_should_accept_snake_case_update_fields() {
        let args: ApplyPatchToolArgs = serde_json::from_str(
            r#"{"operations":[{"action":"update","path":"a.txt","old_string":"before","new_string":"after","replace_all":true}]}"#,
        )
        .expect("parse snake case args");
        let ops = apply_patch_ops_from_tool_args(args).expect("convert snake case args");
        let ApplyPatchOp::Update { old_string, new_string, replace_all, .. } = &ops[0] else {
            panic!("expected update op");
        };
        assert_eq!(old_string, "before");
        assert_eq!(new_string, "after");
        assert!(*replace_all);
    }

    #[test]
    fn tool_args_should_accept_camel_case_update_fields() {
        let args: ApplyPatchToolArgs = serde_json::from_str(
            r#"{"operations":[{"action":"update","path":"a.txt","oldString":"before","newString":"after","replaceAll":true}]}"#,
        )
        .expect("parse camel case args");
        let ops = apply_patch_ops_from_tool_args(args).expect("convert camel case args");
        let ApplyPatchOp::Update { old_string, new_string, replace_all, .. } = &ops[0] else {
            panic!("expected update op");
        };
        assert_eq!(old_string, "before");
        assert_eq!(new_string, "after");
        assert!(*replace_all);
    }

    #[test]
    fn apply_update_should_replace_single_occurrence() {
        let updated = apply_patch_apply_update("a\nb\nc\n", "b", "B", false).expect("apply");
        assert_eq!(updated, "a\nB\nc\n");
    }

    #[test]
    fn apply_update_should_return_new_string_when_old_string_is_empty() {
        let updated = apply_patch_apply_update("ignored", "", "new content\n", false)
            .expect("empty old_string should succeed");
        assert_eq!(updated, "new content\n");
    }

    #[test]
    fn apply_update_should_explain_not_found() {
        let err = apply_patch_apply_update("a\nb\nc\n", "b\nmissing", "x", false).expect_err("not found should fail");
        assert!(err.contains("old_string 在文件中未找到"));
        assert!(err.contains("最相似候选行范围"));
        assert!(err.contains("line ") || err.contains("lines "));
        assert!(err.contains("b"));
        assert!(err.contains("先重新读取目标文件"));
        assert!(err.contains("最小 update 示例"));
    }

    #[test]
    fn apply_update_should_still_fail_when_replace_all_true_but_old_string_missing() {
        let err = apply_patch_apply_update("a\nb\nc\n", "missing", "x", true)
            .expect_err("missing old_string should still fail");
        assert!(err.contains("old_string 在文件中未找到"));
    }

    #[test]
    fn apply_update_should_truncate_long_old_string_preview_in_error() {
        let old_string = "x".repeat(400);
        let err = apply_patch_apply_update("short", &old_string, "new", false)
            .expect_err("long old_string preview should fail");
        let preview = err
            .split("old_string 预览（前 300 字符）：\n")
            .nth(1)
            .expect("preview should exist");
        assert_eq!(preview.chars().count(), 300);
    }

    #[test]
    fn apply_update_should_reject_non_unique_without_replace_all() {
        let err = apply_patch_apply_update("target\nkeep\ntarget\nkeep\n", "target", "changed", false)
            .expect_err("ambiguous match should fail");
        assert!(err.contains("replace_all"));
        assert!(err.contains("2 次"));
        assert!(err.contains("命中行范围：line 1, line 3"));
        assert!(err.contains("扩大 old_string"));
        assert!(err.contains("全部替换"));
    }

    #[test]
    fn apply_update_should_limit_many_match_ranges() {
        let content = (0..20).map(|_| "target\n").collect::<String>();
        let err = apply_patch_apply_update(&content, "target", "changed", false)
            .expect_err("ambiguous match should fail");
        assert!(err.contains("出现了 20 次"));
        assert!(err.contains("另有 8 处"));
        assert!(!err.contains("line 20"));
    }

    #[test]
    fn apply_update_should_replace_all() {
        let updated = apply_patch_apply_update("target\nkeep\ntarget\nkeep\n", "target", "changed", true)
            .expect("replace_all apply");
        assert_eq!(updated, "changed\nkeep\nchanged\nkeep\n");
    }

    #[test]
    fn apply_update_should_match_lf_old_string_against_crlf_file() {
        let updated = apply_patch_apply_update_with_line_ending_fallback(
            "a\r\nold\r\nvalue\r\nc\r\n",
            "old\nvalue",
            "new\nvalue",
            false,
        )
        .expect("line ending fallback should apply");
        assert_eq!(updated, "a\r\nnew\r\nvalue\r\nc\r\n");
    }

    #[test]
    fn apply_update_should_match_crlf_old_string_against_lf_file() {
        let updated = apply_patch_apply_update_with_line_ending_fallback(
            "a\nold\nvalue\nc\n",
            "old\r\nvalue",
            "new\r\nvalue",
            false,
        )
        .expect("line ending fallback should apply");
        assert_eq!(updated, "a\nnew\nvalue\nc\n");
    }

    #[tokio::test]
    async fn execute_ops_should_keep_prior_success_and_return_failed_operation() {
        let data_path = make_temp_data_path("apply-patch-partial");
        let cwd = app_root_from_data_path(&data_path).join("workspace");
        std::fs::create_dir_all(&cwd).expect("create cwd");
        let first = cwd.join("first.txt");
        let second = cwd.join("second.txt");
        std::fs::write(&first, "old\n").expect("seed first");
        std::fs::write(&second, "still old\n").expect("seed second");

        let ops = vec![
            ApplyPatchResolvedOp::Update {
                from: first.clone(),
                to: None,
                old_string: "old".to_string(),
                new_string: "new".to_string(),
                replace_all: false,
            },
            ApplyPatchResolvedOp::Update {
                from: second.clone(),
                to: None,
                old_string: "missing".to_string(),
                new_string: "new".to_string(),
                replace_all: false,
            },
        ];
        let mut record = apply_patch_empty_backup_record(&data_path, "s1", &cwd, "raw")
            .expect("empty record");

        let outcome = apply_patch_execute_ops(&data_path, &mut record, &ops)
            .await
            .expect("execute should return structured outcome");

        assert_eq!(std::fs::read_to_string(&first).expect("read first"), "new\n");
        assert_eq!(std::fs::read_to_string(&second).expect("read second"), "still old\n");
        assert_eq!(outcome.changed.len(), 1);
        assert_eq!(record.entries.len(), 1);
        let failure = outcome.failure.expect("second op should fail");
        assert_eq!(failure.index, 1);
        assert_eq!(failure.op, "update");
        assert!(failure.message.contains("old_string 在文件中未找到"));
        assert!(failure.message.contains("最相似候选行范围"));
    }

    #[tokio::test]
    async fn execute_ops_should_update_gbk_file_without_converting_to_utf8() {
        let data_path = make_temp_data_path("apply-patch-gbk");
        let cwd = app_root_from_data_path(&data_path).join("workspace");
        std::fs::create_dir_all(&cwd).expect("create cwd");
        let file = cwd.join("gbk.txt");
        std::fs::write(&file, [0xd6, 0xd0, 0xce, 0xc4, b'\n', b'o', b'l', b'd', b'\n'])
            .expect("seed gbk file");
        let ops = vec![ApplyPatchResolvedOp::Update {
            from: file.clone(),
            to: None,
            old_string: "old".to_string(),
            new_string: "new".to_string(),
            replace_all: false,
        }];
        let mut record = apply_patch_empty_backup_record(&data_path, "s1", &cwd, "raw")
            .expect("empty record");

        let outcome = apply_patch_execute_ops(&data_path, &mut record, &ops)
            .await
            .expect("execute");

        assert!(outcome.failure.is_none());
        assert_eq!(
            std::fs::read(&file).expect("read updated gbk"),
            vec![0xd6, 0xd0, 0xce, 0xc4, b'\n', b'n', b'e', b'w', b'\n']
        );
    }

    #[test]
    fn resolve_path_should_allow_relative_path_from_cwd() {
        let base = std::env::temp_dir().join(format!("eca-apply-patch-tests-{}", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&base);
        let result = apply_patch_resolve_path(&base, "relative.txt");
        assert_eq!(
            result.expect("resolve"),
            terminal_normalize_for_access_check(&base.join("relative.txt"))
        );
    }

    #[test]
    fn resolve_path_should_allow_absolute_path_outside_workspace() {
        let base = std::env::temp_dir().join(format!("eca-apply-patch-base-{}", Uuid::new_v4()));
        let outside = std::env::temp_dir().join(format!("eca-apply-patch-outside-{}.txt", Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&base);
        let result = apply_patch_resolve_path(&base, &outside.to_string_lossy());
        assert_eq!(result.expect("resolve"), terminal_normalize_for_access_check(&outside));
    }

    #[test]
    fn resolve_ops_should_treat_move_as_rename_update_with_empty_strings() {
        let cwd = std::env::temp_dir().join(format!("eca-apply-patch-move-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&cwd).expect("create cwd");
        let ops = vec![ApplyPatchOp::Move {
            path: "from.txt".to_string(),
            to: "nested/to.txt".to_string(),
        }];
        let resolved = apply_patch_resolve_ops(&cwd, ops).expect("resolve move");
        match &resolved[0] {
            ApplyPatchResolvedOp::Update {
                from,
                to,
                old_string,
                new_string,
                replace_all,
            } => {
                assert_eq!(
                    from,
                    &terminal_normalize_for_access_check(&cwd.join("from.txt"))
                );
                assert_eq!(
                    to.as_ref().expect("move should have target"),
                    &terminal_normalize_for_access_check(&cwd.join("nested/to.txt"))
                );
                assert!(old_string.is_empty());
                assert!(new_string.is_empty());
                assert!(!replace_all);
            }
            _ => panic!("expected move to resolve as update op"),
        }
    }

    #[test]
    fn backup_record_should_capture_delete_update_and_move() {
        let data_path = make_temp_data_path("apply-patch-backup");
        let cwd = app_root_from_data_path(&data_path).join("workspace");
        std::fs::create_dir_all(&cwd).expect("create cwd");
        let cwd = cwd.canonicalize().expect("canonical cwd");
        std::fs::write(cwd.join("delete.txt"), b"\x00\x01delete").expect("seed delete");
        std::fs::write(cwd.join("update.txt"), "old\n").expect("seed update");
        std::fs::write(cwd.join("move.txt"), "before\nold\n").expect("seed move");
        let delete_path = absolute_user_path(&cwd.join("delete.txt"));
        let update_path = absolute_user_path(&cwd.join("update.txt"));
        let move_from_path = absolute_user_path(&cwd.join("move.txt"));
        let move_to_path = cwd.join("moved.txt").to_string_lossy().to_string();
        let input = serde_json::json!({
            "operations": [
                {"action": "delete", "path": delete_path},
                {"action": "update", "path": update_path, "old_string": "old", "new_string": "new"},
                {"action": "move", "path": move_from_path, "to": move_to_path}
            ]
        })
        .to_string();
        let args: ApplyPatchToolArgs = serde_json::from_str(&input).expect("parse tool args");
        let ops = apply_patch_resolve_ops(&cwd, apply_patch_ops_from_tool_args(args).expect("convert")).expect("resolve");
        let record = apply_patch_prepare_backup_record(&data_path, "s1", &cwd, &input, &ops)
            .expect("prepare");
        assert_eq!(record.entries.len(), 3);
        assert!(record.entries.iter().any(|entry| entry.kind == ApplyPatchBackupKind::Delete));
        assert!(record.entries.iter().any(|entry| entry.kind == ApplyPatchBackupKind::Update));
        assert!(record.entries.iter().any(|entry| entry.kind == ApplyPatchBackupKind::MoveUpdate));
    }

    #[test]
    fn clear_apply_patch_temp_should_remove_records_and_blobs() {
        let data_path = make_temp_data_path("apply-patch-clear");
        let records_dir = apply_patch_temp_records_dir(&data_path);
        let blobs_dir = apply_patch_temp_blobs_dir(&data_path);
        std::fs::create_dir_all(&records_dir).expect("create records dir");
        std::fs::create_dir_all(&blobs_dir).expect("create blobs dir");
        std::fs::write(records_dir.join("a.json"), "{}").expect("seed record");
        std::fs::write(blobs_dir.join("b.bin"), "x").expect("seed blob");

        let (records, blobs) = clear_apply_patch_temp(&data_path).expect("clear temp");
        assert_eq!(records, 1);
        assert_eq!(blobs, 1);
        assert_eq!(std::fs::read_dir(&records_dir).expect("read records").count(), 0);
        assert_eq!(std::fs::read_dir(&blobs_dir).expect("read blobs").count(), 0);
    }

    #[test]
    fn backup_record_should_restore_added_file_by_deleting_it() {
        let data_path = make_temp_data_path("apply-patch-restore-add");
        let path = app_root_from_data_path(&data_path).join("added.txt");
        std::fs::write(&path, "hello").expect("seed add");
        let record = ApplyPatchBackupRecord {
            record_id: Uuid::new_v4().to_string(),
            session_id: "s1".to_string(),
            cwd: app_root_from_data_path(&data_path).to_string_lossy().to_string(),
            fingerprint: "fp".to_string(),
            created_at: now_iso(),
            entries: vec![ApplyPatchBackupEntry {
                kind: ApplyPatchBackupKind::Add,
                path: path.to_string_lossy().to_string(),
                from_path: None,
                to_path: None,
                expected_current_content: Some("hello".to_string()),
                backup_blob_file: None,
            }],
        };
        let (restored, overwritten) = apply_patch_restore_backup_record(&data_path, &record).expect("restore");
        assert_eq!(restored, 1);
        assert!(overwritten.is_empty());
        assert!(!path.exists());
    }

    #[test]
    fn backup_record_should_restore_updated_file_content() {
        let data_path = make_temp_data_path("apply-patch-restore-update");
        let path = app_root_from_data_path(&data_path).join("update.txt");
        std::fs::write(&path, "new\n").expect("seed update");
        std::fs::create_dir_all(apply_patch_temp_blobs_dir(&data_path)).expect("create blobs");
        std::fs::write(apply_patch_blob_path(&data_path, "update.bin"), "old\n")
            .expect("write blob");
        let record = ApplyPatchBackupRecord {
            record_id: Uuid::new_v4().to_string(),
            session_id: "s1".to_string(),
            cwd: app_root_from_data_path(&data_path).to_string_lossy().to_string(),
            fingerprint: "fp".to_string(),
            created_at: now_iso(),
            entries: vec![ApplyPatchBackupEntry {
                kind: ApplyPatchBackupKind::Update,
                path: path.to_string_lossy().to_string(),
                from_path: None,
                to_path: None,
                expected_current_content: Some("new\n".to_string()),
                backup_blob_file: Some("update.bin".to_string()),
            }],
        };
        let (restored, overwritten) = apply_patch_restore_backup_record(&data_path, &record).expect("restore");
        assert_eq!(restored, 1);
        assert!(overwritten.is_empty());
        assert_eq!(std::fs::read_to_string(&path).expect("read restored"), "old\n");
    }
}

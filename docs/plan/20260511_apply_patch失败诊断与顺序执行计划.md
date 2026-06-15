# apply_patch 失败诊断与顺序执行计划

## 背景

当前 `apply_patch` 在准备备份记录时会提前校验整批操作。结果是后续某个 `update.old_string` 匹配失败时，前面本可成功的操作也不会执行；同时 `old_string` 多命中或未命中时，错误信息缺少具体行号范围，模型很难自己修正。

## 目标

1. 多个操作仍按输入顺序执行。
2. 某个操作失败时，返回该操作的失败信息，并保留已成功执行的前序操作。
3. `old_string` 命中多处且 `replace_all=false` 时，返回所有命中的行号范围。
4. `old_string` 未命中时，返回最相似的候选行号范围，帮助模型重新构造补丁。
5. 超出本次失败点的后续操作不再继续执行，避免在未知上下文下扩大修改面。
6. 部分成功的补丁在会话撤回时也能被正确恢复。

## 已完成的实现

### 1. 备份准备拆为单操作粒度

- 新增 `apply_patch_prepare_backup_entry`，将原来 `apply_patch_prepare_backup_record` 中按操作类型准备备份的逻辑拆为单操作粒度。
- 新增 `apply_patch_empty_backup_record`，创建空 record 结构，执行过程中逐条追加 entry。
- `apply_patch_execute_ops` 改为逐条执行：先 `prepare_backup_entry`，再 `execute_single_op`，成功后才 `push` 到 record。

### 2. 批量执行部分成功返回

- 新增 `ApplyPatchExecutionOutcome` 和 `ApplyPatchExecutionFailure` 结构体。
- `apply_patch_execute_ops` 遇到失败时返回 `Ok(outcome)`，其中 `failure` 包含失败操作的 index/op/path/message，`changed` 包含已成功操作的结果。
- `builtin_apply_patch` 根据 `outcome.failure` 是否存在返回 `ok=false`、`partial=true`、`changed`、`changedCount`、`failed`，并保留已成功操作的备份 record（`backupRecordId`、`backupFingerprint`、`backupRecordPath`）。

### 3. old_string 诊断增强

- `old_string` 未命中时，调用 `apply_patch_similar_line_ranges` 返回最相似的候选行范围（含摘要）。
- `old_string` 多命中且 `replace_all=false` 时，调用 `apply_patch_exact_match_ranges` 返回所有命中行范围。
- 错误信息中附带 `old_string` 预览（前 300 字符）和最小 update 示例，帮助模型修正。

### 4. 相似度搜索性能优化

- `apply_patch_similar_line_ranges` 增加两层限制：
  - 文件超过 5000 行时跳过相似度诊断（`MAX_FILE_LINES_FOR_SIMILARITY`）。
  - 窗口行重叠率低于 90% 时直接跳过 Levenshtein 计算（`MIN_LINE_OVERLAP_RATIO`），用 HashSet 做廉价预过滤。

### 5. 行尾兼容

- `read_file` 新增 `detect_read_file_line_ending` 和 `normalize_text_line_endings_for_read_file`，统一返回 LF 行尾，并在 metadata 中报告 `sourceLineEnding`、`contentLineEnding`、`lineEndingNote`。
- 新增 `apply_patch_apply_update_with_line_ending_fallback`：当 LF 匹配失败时，尝试将 content 和 old_string/new_string 归一化为同一行尾后重试。
- `apply_patch_prepare_backup_entry` 中 Update 操作改用 `apply_patch_apply_update_with_line_ending_fallback`。

### 6. 撤回逻辑修复

- `patch_rewind.rs` 中 `apply_patch_tool_result_is_success` 重命名为 `apply_patch_tool_result_is_undo_eligible`。
- 新增 partial failure 分支：`partial=true && changedCount>0 && backupRecordId存在` 时也视为可撤回。
- `parse_apply_patch_tool_args` 增加对嵌套 `input` 字段的解析支持。

## 单测覆盖

- `execute_ops_should_keep_prior_success_and_return_failed_operation`：验证部分成功时已成功操作被保留、失败操作信息正确返回。
- `apply_update_should_match_crlf_old_string_against_lf_file`：验证 CRLF old_string 能匹配 LF 文件内容。
- `build_text_read_result_should_normalize_crlf_to_lf_and_report_source_line_ending`：验证 read_file 行尾归一化和 metadata 报告。
- `collect_records_should_only_pick_success_apply_patch`：验证撤回逻辑能正确收集可撤回记录。

## 验证

`cargo test apply_patch` 和 `cargo test rewind_apply_patch` 全部通过。`cargo check` 编译无新增警告。

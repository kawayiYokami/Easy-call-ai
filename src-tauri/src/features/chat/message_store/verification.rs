#[derive(Debug, Clone, PartialEq, Eq)]
struct MessageStoreVerificationReport {
    message_count: usize,
    last_message_id: String,
    compaction_count: usize,
    index: MessageStoreIndexFile,
}

fn verify_jsonl_snapshot_content(
    content: &str,
    expected_message_count: usize,
    expected_last_message_id: &str,
) -> Result<MessageStoreVerificationReport, String> {
    let mut offset = 0_u64;
    let mut items = Vec::<MessageStoreIndexItem>::new();
    let mut compaction_count = 0_usize;
    let mut last_message_id = String::new();

    for raw_line in content.split_inclusive('\n') {
        let byte_len = raw_line.as_bytes().len() as u64;
        let line = raw_line.trim_end_matches(['\r', '\n']);
        if line.trim().is_empty() {
            offset += byte_len;
            continue;
        }
        let message = decode_jsonl_snapshot_message(line)
            .map_err(|err| format!("JSONL 校验失败，offset={offset}: {err}"))?;
        let item = message_store_index_item_for_message(&message, offset, byte_len);
        if item.compaction_kind.is_some() {
            compaction_count += 1;
        }
        last_message_id = item.message_id.clone();
        items.push(item);
        offset += byte_len;
    }

    if !content.is_empty() && !content.ends_with('\n') {
        return Err("JSONL 校验失败: 文件末尾存在未换行的半行".to_string());
    }
    if expected_message_count != usize::MAX && items.len() != expected_message_count {
        return Err(format!(
            "JSONL 校验失败: 消息数量不一致，expected={}, actual={}",
            expected_message_count,
            items.len()
        ));
    }
    let expected_last_message_id = expected_last_message_id.trim();
    if !expected_last_message_id.is_empty() && last_message_id != expected_last_message_id {
        return Err(format!(
            "JSONL 校验失败: 最后一条消息不一致，expected={}, actual={}",
            expected_last_message_id,
            last_message_id
        ));
    }

    Ok(MessageStoreVerificationReport {
        message_count: items.len(),
        last_message_id,
        compaction_count,
        index: MessageStoreIndexFile::new(MESSAGE_STORE_MANIFEST_VERSION, items),
    })
}

fn verify_jsonl_snapshot_file(
    path: &PathBuf,
    expected_message_count: usize,
    expected_last_message_id: &str,
) -> Result<MessageStoreVerificationReport, String> {
    let content = fs::read_to_string(path)
        .map_err(|err| format!("读取 JSONL 快照失败，path={}，error={err}", path.display()))?;
    verify_jsonl_snapshot_content(&content, expected_message_count, expected_last_message_id)
}

// ==================== V4 按组解析校验（生产路径） ====================
//
// V4 块 = zstd 压缩的多行组明文。校验 = 解压 → 按组解析 → 组级 locator。
// 组边界：工具行/正文行按 id 归组；普通消息行（kind=message）是单行组。
// 未闭合组（只有工具行）也闭合为一条「只有工具、无正文」的消息（Q3 定案 A）。

/// V4 明文中的一个组（一条消息）：明文偏移 + 覆盖长度 + 组内行
#[derive(Debug, Clone)]
pub(super) struct V4GroupPlainBlock {
    pub(super) offset: u64,
    pub(super) byte_len: u64,
    pub(super) lines: Vec<String>,
}

/// 解析 V4 多行组明文内容（调用方负责已解压），产出组级 index items
fn parse_jsonl_snapshot_groups_v4(
    content: &str,
) -> Result<(Vec<MessageStoreIndexItem>, usize, String), String> {
    let groups = parse_jsonl_snapshot_group_blocks_v4(content)?;
    let mut items = Vec::<MessageStoreIndexItem>::with_capacity(groups.len());
    let mut compaction_count = 0_usize;
    let mut last_message_id = String::new();
    for group in groups {
        let message = assemble_group_message(&group.lines)?;
        let item = message_store_index_item_for_message(&message, group.offset, group.byte_len);
        if item.compaction_kind.is_some() {
            compaction_count += 1;
        }
        last_message_id = item.message_id.clone();
        items.push(item);
    }
    Ok((items, compaction_count, last_message_id))
}

/// 按组切分 V4 明文，返回每组 (明文偏移, 覆盖长度, 组内行)
/// 组边界：工具行/正文行按 id 归组；普通消息行（kind=message）是单行组。
/// 未闭合组（只有工具行）也在文件尾闭合（Q3 定案 A）。
pub(super) fn parse_jsonl_snapshot_group_blocks_v4(
    content: &str,
) -> Result<Vec<V4GroupPlainBlock>, String> {
    let mut groups = Vec::<V4GroupPlainBlock>::new();
    let mut offset = 0_u64;
    let mut group_start_offset = 0_u64;
    let mut group_lines = Vec::<String>::new();
    let mut group_byte_len = 0_u64;
    let mut group_id = String::new();
    let mut group_closed = false;

    let close_group = |group_lines: &mut Vec<String>,
                       group_byte_len: &mut u64,
                       group_start_offset: u64,
                       groups: &mut Vec<V4GroupPlainBlock>|
     -> Result<(), String> {
        if group_lines.is_empty() {
            *group_byte_len = 0;
            return Ok(());
        }
        groups.push(V4GroupPlainBlock {
            offset: group_start_offset,
            byte_len: std::mem::take(group_byte_len),
            lines: std::mem::take(group_lines),
        });
        Ok(())
    };

    for raw_line in content.split_inclusive('\n') {
        let byte_len = raw_line.as_bytes().len() as u64;
        let line = raw_line.trim_end_matches(['\r', '\n']);
        if line.trim().is_empty() {
            // 组已开始时，空行字节归入当前组（保持组覆盖区间连续）；组间空行只推进 offset
            if !group_lines.is_empty() {
                group_byte_len += byte_len;
            }
            offset += byte_len;
            continue;
        }
        // 判断行类型与归属组
        let is_group_start = group_lines.is_empty();
        let parsed = serde_json::from_str::<serde_json::Value>(line)
            .map_err(|err| format!("V4 组解析失败，offset={offset}: {err}"))?;
        let kind = parsed
            .get("kind")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let line_id = parsed
            .get("id")
            .or_else(|| parsed.get("message").and_then(|m| m.get("id")))
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        // 新组开始的三种情形：普通消息行、上一行是正文行（组已终结）、tool 行 id 变化
        let starts_new_group = !is_group_start
            && (kind == GROUP_LINE_KIND_MESSAGE
                || group_closed
                || (kind == GROUP_LINE_KIND_TOOL && line_id != group_id));
        if starts_new_group {
            close_group(&mut group_lines, &mut group_byte_len, group_start_offset, &mut groups)?;
            group_id.clear();
        }
        if group_lines.is_empty() {
            group_start_offset = offset;
        }
        if kind == GROUP_LINE_KIND_MESSAGE {
            // 普通消息单行组：立即闭合
            group_lines.push(line.to_string());
            group_byte_len += byte_len;
            close_group(&mut group_lines, &mut group_byte_len, group_start_offset, &mut groups)?;
            group_id.clear();
            group_closed = false;
        } else {
            group_lines.push(line.to_string());
            group_byte_len += byte_len;
            group_id = line_id;
            group_closed = kind == GROUP_LINE_KIND_ASSISTANT;
        }
        offset += byte_len;
    }
    // 文件尾闭合最后一组（含未闭合组：只有工具行，Q3 定案 A）
    close_group(&mut group_lines, &mut group_byte_len, group_start_offset, &mut groups)?;

    if !content.is_empty() && !content.ends_with('\n') {
        return Err("V4 组校验失败: 文件末尾存在未换行的半行".to_string());
    }
    Ok(groups)
}

/// V4 校验明文内容（已解压）：按组解析并生成组级 index
fn verify_jsonl_snapshot_content_v4(
    content: &str,
    expected_message_count: usize,
    expected_last_message_id: &str,
) -> Result<MessageStoreVerificationReport, String> {
    let (items, compaction_count, last_message_id) = parse_jsonl_snapshot_groups_v4(content)?;
    if expected_message_count != usize::MAX && items.len() != expected_message_count {
        return Err(format!(
            "V4 组校验失败: 消息数量不一致，expected={}, actual={}",
            expected_message_count,
            items.len()
        ));
    }
    let expected_last_message_id = expected_last_message_id.trim();
    if !expected_last_message_id.is_empty() && last_message_id != expected_last_message_id {
        return Err(format!(
            "V4 组校验失败: 最后一条消息不一致，expected={}, actual={}",
            expected_last_message_id,
            last_message_id
        ));
    }
    Ok(MessageStoreVerificationReport {
        message_count: items.len(),
        last_message_id,
        compaction_count,
        index: MessageStoreIndexFile::new(MESSAGE_STORE_MANIFEST_VERSION, items),
    })
}

/// V4 校验压缩块文件：解压 → 按组解析
fn verify_jsonl_snapshot_file_v4(
    path: &PathBuf,
    expected_message_count: usize,
    expected_last_message_id: &str,
) -> Result<MessageStoreVerificationReport, String> {
    let compressed = fs::read(path)
        .map_err(|err| format!("读取 V4 压缩块失败，path={}，error={err}", path.display()))?;
    let content = zstd_decompress_block(&compressed)?;
    let content = String::from_utf8(content)
        .map_err(|err| format!("V4 压缩块解压后不是 UTF-8，path={}，error={err}", path.display()))?;
    verify_jsonl_snapshot_content_v4(&content, expected_message_count, expected_last_message_id)
}

fn rebuild_jsonl_snapshot_index_from_file(path: &PathBuf) -> Result<MessageStoreIndexFile, String> {
    let report = verify_jsonl_snapshot_file(path, usize::MAX, "")?;
    Ok(report.index)
}

#[cfg(test)]
mod v4_group_parse_tests {
    use super::*;

    fn tool_line(id: &str) -> String {
        encode_group_tool_line(
            id,
            "assistant",
            "2026-08-22T00:00:00Z",
            &None,
            &serde_json::json!({"role": "assistant", "tool_calls": [{"id": format!("call-{id}")}]}),
        )
        .expect("tool line")
    }

    fn tool_result_line(id: &str) -> String {
        encode_group_tool_line(
            id,
            "assistant",
            "2026-08-22T00:00:00Z",
            &None,
            &serde_json::json!({"role": "tool", "tool_call_id": format!("call-{id}")}),
        )
        .expect("tool result line")
    }

    fn assistant_line(id: &str) -> String {
        encode_group_assistant_line(
            id,
            "assistant",
            "2026-08-22T00:00:00Z",
            &None,
            &[MessagePart::Text {
                text: format!("answer {id}"),
                reasoning_content: None,
            }],
            &[],
            &None,
            &None,
            &None,
        )
        .expect("assistant line")
    }

    fn message_line(id: &str) -> String {
        let message = ChatMessage {
            id: id.to_string(),
            role: "user".to_string(),
            created_at: "2026-08-22T00:00:00Z".to_string(),
            speaker_agent_id: None,
            parts: vec![MessagePart::Text {
                text: format!("hello {id}"),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta: None,
            tool_call: None,
            mcp_call: None,
            meme_annotations: None,
        };
        encode_group_message_line(&message).expect("message line")
    }

    #[test]
    fn v4_group_parse_should_split_tool_group_then_plain_message() {
        // [tool(A), tool_result(A), assistant(A), message(B)] → 2 组，A 的工具/正文不丢失
        let content = format!(
            "{}{}{}{}",
            tool_line("A"),
            tool_result_line("A"),
            assistant_line("A"),
            message_line("B")
        );
        let groups = parse_jsonl_snapshot_group_blocks_v4(&content).expect("parse");
        assert_eq!(groups.len(), 2, "tool+assistant 组与普通消息组应分开");
        let a = assemble_group_message(&groups[0].lines).expect("assemble A");
        assert_eq!(a.id, "A");
        assert_eq!(a.tool_call.as_ref().expect("A tool_call").len(), 2);
        assert!(!a.parts.is_empty(), "A 正文行不应丢失");
        let b = assemble_group_message(&groups[1].lines).expect("assemble B");
        assert_eq!(b.id, "B");
    }

    #[test]
    fn v4_group_parse_should_close_unclosed_tool_group_before_plain_message() {
        // [tool(A), tool_result(A), message(B)]（未闭合组后接普通消息）→ 2 组
        let content = format!("{}{}{}", tool_line("A"), tool_result_line("A"), message_line("B"));
        let groups = parse_jsonl_snapshot_group_blocks_v4(&content).expect("parse");
        assert_eq!(groups.len(), 2, "未闭合工具组应闭合后再开普通消息组");
        let a = assemble_group_message(&groups[0].lines).expect("assemble A");
        assert_eq!(a.id, "A");
        assert_eq!(a.tool_call.as_ref().expect("A tool_call").len(), 2);
        let b = assemble_group_message(&groups[1].lines).expect("assemble B");
        assert_eq!(b.id, "B");
    }

    #[test]
    fn v4_group_parse_should_split_two_tool_groups() {
        // [tool(A), tool_result(A), assistant(A), tool(B), tool_result(B), assistant(B)] → 2 组
        let content = format!(
            "{}{}{}{}{}{}",
            tool_line("A"),
            tool_result_line("A"),
            assistant_line("A"),
            tool_line("B"),
            tool_result_line("B"),
            assistant_line("B")
        );
        let groups = parse_jsonl_snapshot_group_blocks_v4(&content).expect("parse");
        assert_eq!(groups.len(), 2, "两组工具消息应分开");
        let a = assemble_group_message(&groups[0].lines).expect("assemble A");
        let b = assemble_group_message(&groups[1].lines).expect("assemble B");
        assert_eq!(a.id, "A");
        assert_eq!(b.id, "B");
        assert_eq!(a.tool_call.as_ref().expect("A tool_call").len(), 2);
        assert_eq!(b.tool_call.as_ref().expect("B tool_call").len(), 2);
    }

    #[test]
    fn v4_group_parse_should_include_newline_in_group_byte_len() {
        // byte_len 与写入端同口径：含换行符（与 offset 推进一致）
        let content = format!("{}{}", tool_line("A"), assistant_line("A"));
        let groups = parse_jsonl_snapshot_group_blocks_v4(&content).expect("parse");
        assert_eq!(groups.len(), 1);
        let group = &groups[0];
        let lines_total: usize = group.lines.iter().map(|line| line.as_bytes().len()).sum();
        assert_eq!(
            group.byte_len as usize,
            lines_total + group.lines.len(),
            "group.byte_len 应等于组内行字节数（每行含 1 字节换行符）"
        );
        // offset 连续：下一组起点 = 上一组 offset + byte_len
        let full = format!("{}{}", content, message_line("B"));
        let groups2 = parse_jsonl_snapshot_group_blocks_v4(&full).expect("parse");
        assert_eq!(groups2.len(), 2);
        assert_eq!(groups2[1].offset, groups2[0].offset + groups2[0].byte_len);
        assert_eq!(groups2[0].byte_len, group.byte_len);
    }

    #[test]
    fn v4_group_parse_should_include_inter_group_blank_line_in_prev_group_len() {
        // 组内空行：字节归入当前组，保持覆盖区间连续
        let content = format!("{}{}{}", tool_line("A"), "\n", assistant_line("A"));
        let groups = parse_jsonl_snapshot_group_blocks_v4(&content).expect("parse");
        assert_eq!(groups.len(), 1, "工具行与正文行之间的空行应留在组内");
        let group = &groups[0];
        let lines_total: usize = group.lines.iter().map(|line| line.as_bytes().len()).sum();
        assert_eq!(
            group.byte_len as usize,
            lines_total + group.lines.len() + 1,
            "组内空行（1 字节 \\n）应计入 group.byte_len"
        );
    }
}

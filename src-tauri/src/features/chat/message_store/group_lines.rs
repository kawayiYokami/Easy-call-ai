// ========== V4 多行组行 schema（工具行/正文行/普通消息行） ==========
//
// V4 存储模型（定案 D12，见 .pai/plan/storage/20260822_消息按组多行存储重构计划.md）：
// 一次 AI 调度（一条 ChatMessage）= 一组多行：
// - 工具行：每完成一轮工具执行追加一行 {"kind":"tool", 公共字段, "call":{...},"result":{...}}
// - 正文行：最终正文到达后追加一行 {"kind":"assistant", 公共字段, parts/extraTextBlocks/providerMeta/memeAnnotations}
// - 普通消息（无工具）：单行 {"kind":"message","message":{完整消息}}
// 公共字段（id/role/createdAt/speakerAgentId）冗余在组内每一行——正文行最后才写，
// 未闭合组只有工具行，必须能独立还原消息骨架。

use serde::{Deserialize, Serialize};

const GROUP_LINE_KIND_TOOL: &str = "tool";
const GROUP_LINE_KIND_ASSISTANT: &str = "assistant";
const GROUP_LINE_KIND_MESSAGE: &str = "message";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupToolLine {
    kind: String,
    id: String,
    role: String,
    created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    speaker_agent_id: Option<String>,
    call: Value,
    result: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupAssistantLine {
    kind: String,
    id: String,
    role: String,
    created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    speaker_agent_id: Option<String>,
    #[serde(default)]
    parts: Vec<MessagePart>,
    #[serde(default)]
    extra_text_blocks: Vec<String>,
    #[serde(default)]
    provider_meta: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    meme_annotations: Option<Vec<MemeAnnotation>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mcp_call: Option<Vec<Value>>,
}

/// 编码一行工具行（工具调用 + 工具回答，即 tool_call 数组交替的两个 Value）
pub(super) fn encode_group_tool_line(
    id: &str,
    role: &str,
    created_at: &str,
    speaker_agent_id: &Option<String>,
    call: &Value,
    result: &Value,
) -> Result<String, String> {
    let line = GroupToolLine {
        kind: GROUP_LINE_KIND_TOOL.to_string(),
        id: id.to_string(),
        role: role.to_string(),
        created_at: created_at.to_string(),
        speaker_agent_id: speaker_agent_id.clone(),
        call: call.clone(),
        result: result.clone(),
    };
    serde_json::to_string(&line)
        .map(|value| format!("{value}\n"))
        .map_err(|err| format!("序列化工具行失败: {err}"))
}

/// 编码一行正文行（消息的非工具部分：parts/extraTextBlocks/providerMeta/memeAnnotations）
pub(super) fn encode_group_assistant_line(
    id: &str,
    role: &str,
    created_at: &str,
    speaker_agent_id: &Option<String>,
    parts: &[MessagePart],
    extra_text_blocks: &[String],
    provider_meta: &Option<Value>,
    meme_annotations: &Option<Vec<MemeAnnotation>>,
    mcp_call: &Option<Vec<Value>>,
) -> Result<String, String> {
    let line = GroupAssistantLine {
        kind: GROUP_LINE_KIND_ASSISTANT.to_string(),
        id: id.to_string(),
        role: role.to_string(),
        created_at: created_at.to_string(),
        speaker_agent_id: speaker_agent_id.clone(),
        parts: parts.to_vec(),
        extra_text_blocks: extra_text_blocks.to_vec(),
        provider_meta: provider_meta.clone(),
        meme_annotations: meme_annotations.clone(),
        mcp_call: mcp_call.clone(),
    };
    serde_json::to_string(&line)
        .map(|value| format!("{value}\n"))
        .map_err(|err| format!("序列化正文行失败: {err}"))
}

/// 编码一行普通消息（无工具消息的单行组，保持完整消息结构）
pub(super) fn encode_group_message_line(message: &ChatMessage) -> Result<String, String> {
    encode_jsonl_snapshot_message(message)
}

/// 组装：组内多行 → 聚合 ChatMessage。
/// - 公共字段（id/role/createdAt/speakerAgentId）取第一行（各行冗余一致）
/// - 工具行 → tool_call 数组（call、result 交替 push，与旧格式一致）
/// - 正文行 → parts/extraTextBlocks/providerMeta/memeAnnotations/mcpCall
/// - 未闭合组（只有工具行、无正文行）→ parts 等默认值，仅工具调用有效
pub(super) fn assemble_group_message(lines: &[String]) -> Result<ChatMessage, String> {
    let mut tool_call = Vec::<Value>::new();
    let mut parts = Vec::<MessagePart>::new();
    let mut extra_text_blocks = Vec::<String>::new();
    let mut provider_meta: Option<Value> = None;
    let mut meme_annotations: Option<Vec<MemeAnnotation>> = None;
    let mut mcp_call: Option<Vec<Value>> = None;
    let mut common: Option<(String, String, String, Option<String>)> = None;
    let mut saw_line = false;

    for line in lines {
        let trimmed = line.trim_end_matches('\n');
        if trimmed.is_empty() {
            continue;
        }
        let parsed: Value = serde_json::from_str(trimmed)
            .map_err(|err| format!("解析 V4 组行失败: {err}"))?;
        let kind = parsed
            .get("kind")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        saw_line = true;
        match kind {
            GROUP_LINE_KIND_TOOL => {
                let tool_line: GroupToolLine = serde_json::from_value(parsed)
                    .map_err(|err| format!("解析工具行失败: {err}"))?;
                if common.is_none() {
                    common = Some((
                        tool_line.id,
                        tool_line.role,
                        tool_line.created_at,
                        tool_line.speaker_agent_id,
                    ));
                }
                tool_call.push(tool_line.call);
                tool_call.push(tool_line.result);
            }
            GROUP_LINE_KIND_ASSISTANT => {
                let assistant_line: GroupAssistantLine = serde_json::from_value(parsed)
                    .map_err(|err| format!("解析正文行失败: {err}"))?;
                if common.is_none() {
                    common = Some((
                        assistant_line.id,
                        assistant_line.role,
                        assistant_line.created_at,
                        assistant_line.speaker_agent_id,
                    ));
                }
                parts = assistant_line.parts;
                extra_text_blocks = assistant_line.extra_text_blocks;
                provider_meta = assistant_line.provider_meta;
                meme_annotations = assistant_line.meme_annotations;
                mcp_call = assistant_line.mcp_call;
            }
            GROUP_LINE_KIND_MESSAGE => {
                let message = decode_jsonl_snapshot_message(trimmed)?;
                return Ok(message);
            }
            other => {
                return Err(format!("不支持的 V4 组行类型: {other}"));
            }
        }
    }
    if !saw_line {
        return Err("组装 V4 消息失败：组内没有行".to_string());
    }
    let (id, role, created_at, speaker_agent_id) =
        common.ok_or_else(|| "组装 V4 消息失败：缺少公共字段".to_string())?;
    Ok(ChatMessage {
        id,
        role,
        created_at,
        speaker_agent_id,
        parts,
        extra_text_blocks,
        provider_meta,
        tool_call: if tool_call.is_empty() {
            None
        } else {
            Some(tool_call)
        },
        mcp_call,
        meme_annotations,
    })
}

/// 把一条完整消息拆成多行组（迁移/整块重建用）：
/// - 有工具：N 个工具行（tool_call 两两拆分）+ 1 个正文行
/// - 无工具：1 行普通消息（完整结构）
pub(super) fn split_message_into_group_lines(message: &ChatMessage) -> Result<Vec<String>, String> {
    let tool_events = message.tool_call.as_deref().unwrap_or_default();
    if tool_events.is_empty() {
        return Ok(vec![encode_group_message_line(message)?]);
    }
    if tool_events.len() % 2 != 0 {
        return Err(format!(
            "拆分 V4 工具行失败：tool_call 数量不是偶数（应为调用/回答交替），message_id={}，count={}",
            message.id,
            tool_events.len()
        ));
    }
    let mut lines = Vec::with_capacity(tool_events.len() / 2 + 1);
    for pair in tool_events.chunks(2) {
        lines.push(encode_group_tool_line(
            &message.id,
            &message.role,
            &message.created_at,
            &message.speaker_agent_id,
            &pair[0],
            &pair[1],
        )?);
    }
    lines.push(encode_group_assistant_line(
        &message.id,
        &message.role,
        &message.created_at,
        &message.speaker_agent_id,
        &message.parts,
        &message.extra_text_blocks,
        &message.provider_meta,
        &message.meme_annotations,
        &message.mcp_call,
    )?);
    Ok(lines)
}

#[cfg(test)]
mod group_lines_tests {
    use super::*;

    fn tool_event(name: &str, tag: &str) -> Value {
        serde_json::json!({
            "type": "tool_call",
            "name": name,
            "tag": tag,
        })
    }

    fn text_message(id: &str, role: &str, text: &str) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            role: role.to_string(),
            created_at: "2026-08-22T00:00:00Z".to_string(),
            speaker_agent_id: Some("agent-a".to_string()),
            parts: vec![MessagePart::Text {
                text: text.to_string(),
                reasoning_content: None,
            }],
            extra_text_blocks: vec!["block".to_string()],
            provider_meta: Some(serde_json::json!({"model": "gpt-4"})),
            tool_call: None,
            mcp_call: None,
            meme_annotations: None,
        }
    }

    #[test]
    fn split_and_assemble_tool_message_should_preserve_all_fields() {
        let mut message = text_message("m1", "assistant", "final answer");
        message.tool_call = Some(vec![
            tool_event("weather", "call-1"),
            tool_event("weather", "result-1"),
            tool_event("flight", "call-2"),
            tool_event("flight", "result-2"),
        ]);
        let lines = split_message_into_group_lines(&message).expect("split");
        assert_eq!(lines.len(), 3, "2 个工具行 + 1 个正文行");

        let assembled = assemble_group_message(&lines).expect("assemble");
        assert_eq!(assembled.id, message.id);
        assert_eq!(assembled.role, message.role);
        assert_eq!(assembled.created_at, message.created_at);
        assert_eq!(assembled.speaker_agent_id, message.speaker_agent_id);
        assert_eq!(assembled.parts.len(), message.parts.len());
        assert_eq!(assembled.extra_text_blocks, message.extra_text_blocks);
        assert_eq!(assembled.provider_meta, message.provider_meta);
        assert_eq!(assembled.meme_annotations, message.meme_annotations);
        assert_eq!(assembled.mcp_call, message.mcp_call);
        assert_eq!(
            assembled.tool_call.as_ref().expect("tool_call"),
            message.tool_call.as_ref().expect("tool_call")
        );
    }

    #[test]
    fn assemble_unclosed_group_should_return_tools_without_parts() {
        let lines = vec![
            encode_group_tool_line(
                "m1",
                "assistant",
                "2026-08-22T00:00:00Z",
                &Some("agent-a".to_string()),
                &tool_event("weather", "call-1"),
                &tool_event("weather", "result-1"),
            )
            .expect("tool line"),
        ];
        let assembled = assemble_group_message(&lines).expect("assemble unclosed");
        assert_eq!(assembled.id, "m1");
        assert_eq!(assembled.role, "assistant");
        assert_eq!(assembled.speaker_agent_id.as_deref(), Some("agent-a"));
        assert!(assembled.parts.is_empty(), "未闭合组无正文");
        assert_eq!(assembled.tool_call.as_ref().expect("tool_call").len(), 2);
    }

    #[test]
    fn split_plain_message_should_be_single_message_line() {
        let message = text_message("m2", "user", "hello");
        let lines = split_message_into_group_lines(&message).expect("split");
        assert_eq!(lines.len(), 1);
        let assembled = assemble_group_message(&lines).expect("assemble");
        assert_eq!(assembled.id, "m2");
        assert_eq!(assembled.parts.len(), 1);
    }

    #[test]
    fn assemble_should_reject_odd_tool_event_count() {
        let mut message = text_message("m3", "assistant", "");
        message.tool_call = Some(vec![tool_event("weather", "call-1")]);
        let err = split_message_into_group_lines(&message).expect_err("odd count should fail");
        assert!(err.contains("不是偶数"), "err={err}");
    }
}

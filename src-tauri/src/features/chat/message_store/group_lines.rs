// ========== V4 多行组行 schema（工具行/正文行/普通消息行） ==========
//
// V4 存储模型（定案 D12，见 .pai/plan/storage/20260822_消息按组多行存储重构计划.md）：
// 一次 AI 调度（一条 ChatMessage）= 一组多行：
// - 工具行：tool_call 数组的一个元素一行 {"kind":"tool", 公共字段, "event":{...}}，
//   按序拆行/按序拼接，不做 call/result 配对假设
// - 正文行：最终正文到达后追加一行 {"kind":"assistant", 公共字段, parts/extraTextBlocks/providerMeta/memeAnnotations}
// - 普通消息（无工具）：单行 {"kind":"message","message":{完整消息}}
// 公共字段（id/role/createdAt/speakerAgentId）冗余在组内每一行——正文行最后才写，
// 未闭合组只有工具行，必须能独立还原消息骨架。
//
// 工具行不配对的原因（真实数据扫描，见 .pai/report/20260822_V4消息组zstd重构风险评估报告.md）：
// tool_call 元素流并非严格 call/result 交替——并行调用是 1 个 call 元素（内含 N 个
// tool_calls）后跟 N 个 result 元素，还存在截图元素等第三方形态；按元素逐行原样搬运
// 对任何形态成立，且严格保留顺序。

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
    event: Value,
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

/// 编码一行工具事件行（tool_call 数组中的一个元素原样一行）
pub(super) fn encode_group_tool_line(
    id: &str,
    role: &str,
    created_at: &str,
    speaker_agent_id: &Option<String>,
    event: &Value,
) -> Result<String, String> {
    let line = GroupToolLine {
        kind: GROUP_LINE_KIND_TOOL.to_string(),
        id: id.to_string(),
        role: role.to_string(),
        created_at: created_at.to_string(),
        speaker_agent_id: speaker_agent_id.clone(),
        event: event.clone(),
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
/// - 工具行 → tool_call 数组（每行一个元素按序 push，不配对）
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
                tool_call.push(tool_line.event);
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
/// - 有工具：每个 tool_call 元素一行（原样按序拆行，不假设 call/result 配对）+ 1 个正文行
/// - 无工具：1 行普通消息（完整结构）
pub(super) fn split_message_into_group_lines(message: &ChatMessage) -> Result<Vec<String>, String> {
    let tool_events = message.tool_call.as_deref().unwrap_or_default();
    if tool_events.is_empty() {
        return Ok(vec![encode_group_message_line(message)?]);
    }
    let mut lines = Vec::with_capacity(tool_events.len() + 1);
    for event in tool_events {
        lines.push(encode_group_tool_line(
            &message.id,
            &message.role,
            &message.created_at,
            &message.speaker_agent_id,
            event,
        )?);
    }
    // 正文全空（占位空壳）时不写正文行：开放组只有工具行，
    // 正文行等 final text 到达再追加一次。否则正文行会闭合组，
    // 后续工具事件被切分到新组（或追加到正文行后被 assemble 短路无视）。
    // 判据不能只看 parts.is_empty()：占位空壳是 parts=[Text{text:""}]，
    // 要看是否有需要正文行承载的实质内容（非空文本/附加块/meta/表情/工具结果元数据）。
    let has_assistant_body_content = message.parts.iter().any(|part| match part {
        MessagePart::Text {
            text,
            reasoning_content,
        } => {
            !text.trim().is_empty()
                || reasoning_content
                    .as_deref()
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false)
        }
        _ => true,
    }) || !message.extra_text_blocks.is_empty()
        || message.provider_meta.is_some()
        || message
            .meme_annotations
            .as_ref()
            .map(|items| !items.is_empty())
            .unwrap_or(false)
        || message
            .mcp_call
            .as_ref()
            .map(|items| !items.is_empty())
            .unwrap_or(false);
    if has_assistant_body_content {
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
    }
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
        assert_eq!(lines.len(), 5, "4 个工具事件行 + 1 个正文行");

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
            )
            .expect("tool line"),
        ];
        let assembled = assemble_group_message(&lines).expect("assemble unclosed");
        assert_eq!(assembled.id, "m1");
        assert_eq!(assembled.role, "assistant");
        assert_eq!(assembled.speaker_agent_id.as_deref(), Some("agent-a"));
        assert!(assembled.parts.is_empty(), "未闭合组无正文");
        assert_eq!(assembled.tool_call.as_ref().expect("tool_call").len(), 1);
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
    fn split_and_assemble_should_preserve_unpaired_tool_event_stream() {
        // 真实数据形态（见风险评估报告扫描结论）：1 个 call（内含并行调用）跟多个
        // result，另有截图元素等第三方形态，奇偶不限，逐行原样搬运、顺序严格保留。
        let call = serde_json::json!({
            "role": "assistant",
            "tool_calls": [{"id": "call-1"}, {"id": "call-2"}],
        });
        let result_one =
            serde_json::json!({"role": "tool", "tool_call_id": "call-1", "content": "ok"});
        let result_two =
            serde_json::json!({"role": "tool", "tool_call_id": "call-2", "content": "ok"});
        let screenshot = serde_json::json!({
            "role": "assistant",
            "content": "截图",
            "screenshotArtifactId": "art-1",
            "screenshotWidth": 800,
            "screenshotHeight": 600,
            "screenshotArtifactMaxRetained": 3,
        });
        let mut message = text_message("m3", "assistant", "final answer");
        message.tool_call = Some(vec![call, result_one, result_two, screenshot]);
        let lines = split_message_into_group_lines(&message).expect("split");
        assert_eq!(lines.len(), 5, "4 个工具事件行 + 1 个正文行");
        let assembled = assemble_group_message(&lines).expect("assemble");
        assert_eq!(
            assembled.tool_call.as_ref().expect("tool_call"),
            message.tool_call.as_ref().expect("tool_call"),
            "元素流顺序与内容逐字保留"
        );
    }
}

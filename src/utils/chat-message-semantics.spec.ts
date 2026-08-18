import { describe, expect, it } from "vitest";
import {
  projectMessageForDisplay,
  TOOL_TEXT_BREAK_PLACEHOLDER,
} from "./chat-message-semantics";
import type { ChatMessage, ToolCallMessage } from "../types/app";

const MARK = TOOL_TEXT_BREAK_PLACEHOLDER;
const CALL_A = "call_a";
const CALL_B = "call_b";

function makeToolCallEvent(
  role: "assistant" | "tool",
  content: string,
  toolCallIds: string[] = [],
  toolCallId?: string,
): ToolCallMessage {
  return {
    role,
    content,
    tool_calls: toolCallIds.map((id) => ({
      id,
      type: "function",
      function: { name: "toolA", arguments: "{}" },
    })),
    tool_call_id: toolCallId,
  };
}

function makeAssistantMessage(
  partsText: string,
  toolCall: ToolCallMessage[],
): ChatMessage {
  return {
    id: "test-message",
    role: "assistant",
    parts: [{ type: "text", text: partsText }],
    toolCall,
  };
}

describe("projectMessageForDisplay 分段占位符一致性", () => {
  it("分支4：工具标记后紧邻正文时插入占位符（完成态与流式一致分段）", () => {
    const msg = makeAssistantMessage(
      `正文1 [toolcall:${CALL_A}]正文2`,
      [
        makeToolCallEvent("assistant", "正文1", [CALL_A]),
        makeToolCallEvent("tool", "结果", [], CALL_A),
        makeToolCallEvent("assistant", "正文2"),
      ],
    );
    const projection = projectMessageForDisplay(msg);
    const segments = projection.text.split(MARK);
    expect(segments.length).toBe(2);
    expect(segments[0]).toBe(`正文1 [toolcall:${CALL_A}]`);
    expect(segments[1]).toBe("正文2");
  });

  it("分支4：纯标记事件后的正文边界也插入占位符（连续标记合并为前段）", () => {
    const msg = makeAssistantMessage(
      `正文1 [toolcall:${CALL_A}][toolcall:${CALL_B}]正文2`,
      [
        makeToolCallEvent("assistant", "正文1", [CALL_A]),
        makeToolCallEvent("tool", "结果", [], CALL_A),
        makeToolCallEvent("assistant", "", [CALL_B]),
        makeToolCallEvent("tool", "结果2", [], CALL_B),
        makeToolCallEvent("assistant", "正文2"),
      ],
    );
    const projection = projectMessageForDisplay(msg);
    const segments = projection.text.split(MARK);
    expect(segments.length).toBe(2);
    expect(segments[0]).toBe(`正文1 [toolcall:${CALL_A}][toolcall:${CALL_B}]`);
    expect(segments[1]).toBe("正文2");
  });

  it("分支4：工具标记后无正文时不插入占位符", () => {
    const msg = makeAssistantMessage(
      `正文1 [toolcall:${CALL_A}]`,
      [
        makeToolCallEvent("assistant", "正文1", [CALL_A]),
        makeToolCallEvent("tool", "结果", [], CALL_A),
      ],
    );
    const projection = projectMessageForDisplay(msg);
    expect(projection.text).not.toContain(MARK);
  });

  it("分支4：已含占位符（流式缓存文本）时不重复插入", () => {
    const msg = makeAssistantMessage(
      `正文1 [toolcall:${CALL_A}]${MARK}正文2`,
      [
        makeToolCallEvent("assistant", "正文1", [CALL_A]),
        makeToolCallEvent("tool", "结果", [], CALL_A),
        makeToolCallEvent("assistant", "正文2"),
      ],
    );
    const projection = projectMessageForDisplay(msg);
    const segments = projection.text.split(MARK);
    expect(segments.length).toBe(2);
    expect(projection.text).toBe(`正文1 [toolcall:${CALL_A}]${MARK}正文2`);
  });

  it("无工具调用消息不产生占位符（现状回归）", () => {
    const msg = makeAssistantMessage("纯正文文本", []);
    const projection = projectMessageForDisplay(msg);
    expect(projection.text).toBe("纯正文文本");
    expect(projection.text).not.toContain(MARK);
  });

  it("分支4：事件文本已含同名工具标记时，占位符插在追加标记（最后位置）后", () => {
    const msg = makeAssistantMessage(
      `开场 [toolcall:${CALL_A}] 过渡正文2`,
      [
        makeToolCallEvent("assistant", `开场 [toolcall:${CALL_A}] 过渡`, [CALL_A]),
        makeToolCallEvent("tool", "结果", [], CALL_A),
        makeToolCallEvent("assistant", "正文2"),
      ],
    );
    const projection = projectMessageForDisplay(msg);
    const segments = projection.text.split(MARK);
    expect(segments.length).toBe(2);
    // 事件文本自带的 [toolcall:call_a] 保持原样，占位符只出现在事件追加的标记（最后一个）之后
    expect(segments[0]).toBe(`开场 [toolcall:${CALL_A}] 过渡 [toolcall:${CALL_A}]`);
    expect(segments[1]).toBe("正文2");
  });

  it("分支5（join 路径）行为不变：事件正文不在 parts 时仍按旧规则分段", () => {
    const msg = makeAssistantMessage(
      "最终答案正文",
      [
        makeToolCallEvent("assistant", "过渡语", [CALL_A]),
        makeToolCallEvent("tool", "结果", [], CALL_A),
        makeToolCallEvent("assistant", "最终答案正文"),
      ],
    );
    const projection = projectMessageForDisplay(msg);
    const segments = projection.text.split(MARK);
    expect(segments.length).toBe(2);
    expect(segments[0]).toContain(`[toolcall:${CALL_A}]`);
  });
});
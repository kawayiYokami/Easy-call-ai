import { describe, expect, it } from "vitest";
import { resolveConversationDisplayTitle } from "../src/features/chat/utils/conversation-title";

describe("resolveConversationDisplayTitle", () => {
  it("ignores legacy titles that equal the conversation id", () => {
    const conversationId = "96deeda7-0d43-4a59-9f10-3897e967dcfa";

    expect(resolveConversationDisplayTitle({
      conversationId,
      title: conversationId,
      summaryTitle: "抽卡数据分析",
      updatedAt: "2026-06-09T10:00:00Z",
    }, {
      untitledLabel: "未命名会话",
    })).toBe("抽卡数据分析");
  });

  it("falls back to the time label instead of showing the raw id", () => {
    const conversationId = "e88e4fe9-b08d-4125-8387-3b92e78b522d";

    const title = resolveConversationDisplayTitle({
      conversationId,
      title: conversationId,
      updatedAt: "2026-06-09T10:00:00Z",
    }, {
      locale: "zh-CN",
      untitledLabel: "未命名会话",
    });

    expect(title).not.toBe(conversationId);
    expect(title).not.toBe("未命名会话");
  });
});

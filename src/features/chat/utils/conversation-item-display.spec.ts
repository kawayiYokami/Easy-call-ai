import { describe, expect, it } from "vitest";
import type { ChatConversationOverviewItem, ConversationPreviewMessage } from "../../../types/app";
import {
  conversationIndicatorClass,
  conversationRuntimeBusy,
  conversationSimpleIndicatorClass,
  conversationStatusIndicatorTone,
  conversationSummaryClass,
  conversationUnreadBadge,
  hasUnreadOrRecentActivity,
  simpleConversationItemLevel,
} from "./conversation-item-display";

const NOW = Date.parse("2026-08-12T15:00:00+08:00");

function item(partial: Partial<ChatConversationOverviewItem>): ChatConversationOverviewItem {
  return {
    conversationId: "c1",
    title: "会话",
    messageCount: 0,
    ...partial,
  } as ChatConversationOverviewItem;
}

function preview(partial: Partial<ConversationPreviewMessage>): ConversationPreviewMessage {
  return { role: "assistant", textPreview: "hello", ...partial } as ConversationPreviewMessage;
}

describe("simpleConversationItemLevel / hasUnreadOrRecentActivity", () => {
  it("未读 → sim（摘要常显）", () => {
    expect(simpleConversationItemLevel(item({ unreadCount: 3 }), NOW)).toBe("sim");
    expect(hasUnreadOrRecentActivity(item({ unreadCount: 3 }), NOW)).toBe(true);
  });

  it("无未读但 7 天内有更新 → sim", () => {
    const recent = item({ updatedAt: "2026-08-10T10:00:00+08:00", lastMessageAt: "2026-08-10T10:00:00+08:00" });
    expect(simpleConversationItemLevel(recent, NOW)).toBe("sim");
  });

  it("7 天窗口边界：恰好 7 天 → sim；超过 → mini", () => {
    const atBoundary = item({ updatedAt: new Date(NOW - 7 * 24 * 60 * 60 * 1000).toISOString() });
    expect(simpleConversationItemLevel(atBoundary, NOW)).toBe("sim");
    const older = item({ updatedAt: new Date(NOW - 7 * 24 * 60 * 60 * 1000 - 1).toISOString() });
    expect(simpleConversationItemLevel(older, NOW)).toBe("mini");
  });

  it("无未读且 7 天外无更新 → mini（折叠一行）", () => {
    const old = item({ updatedAt: "2026-07-01T10:00:00+08:00", lastMessageAt: "2026-07-01T10:00:00+08:00" });
    expect(simpleConversationItemLevel(old, NOW)).toBe("mini");
    expect(hasUnreadOrRecentActivity(old, NOW)).toBe(false);
  });

  it("无任何时间信息 → 非近期（mini）", () => {
    const empty = item({ updatedAt: "", lastMessageAt: "" });
    expect(simpleConversationItemLevel(empty, NOW)).toBe("mini");
  });
});

describe("conversationUnreadBadge", () => {
  it("当前会话不显示未读角标", () => {
    expect(conversationUnreadBadge(item({ unreadCount: 5 }), "c1")).toBe("");
  });

  it("0 或负数不显示", () => {
    expect(conversationUnreadBadge(item({ unreadCount: 0 }), "c2")).toBe("");
    expect(conversationUnreadBadge(item({ unreadCount: -1 }), "c2")).toBe("");
  });

  it("1~99 显示数字", () => {
    expect(conversationUnreadBadge(item({ unreadCount: 1 }), "c2")).toBe("1");
    expect(conversationUnreadBadge(item({ unreadCount: 99 }), "c2")).toBe("99");
  });

  it("超过 99 显示 99+", () => {
    expect(conversationUnreadBadge(item({ unreadCount: 100 }), "c2")).toBe("99+");
    expect(conversationUnreadBadge(item({ unreadCount: 1000 }), "c2")).toBe("99+");
  });
});

describe("conversationRuntimeBusy", () => {
  it("流式/整理上下文/归档/压缩均为忙碌", () => {
    expect(conversationRuntimeBusy("assistant_streaming")).toBe(true);
    expect(conversationRuntimeBusy("organizing_context")).toBe(true);
    expect(conversationRuntimeBusy("archiving")).toBe(true);
    expect(conversationRuntimeBusy("compacting")).toBe(true);
  });

  it("idle 与空值不忙碌", () => {
    expect(conversationRuntimeBusy("idle")).toBe(false);
    expect(conversationRuntimeBusy(undefined)).toBe(false);
  });
});

describe("conversationStatusIndicatorTone / conversationIndicatorClass", () => {
  it("当前会话不显示指示点", () => {
    expect(conversationStatusIndicatorTone("error", true)).toBe("");
    expect(conversationStatusIndicatorTone("busy", true)).toBe("");
    expect(conversationStatusIndicatorTone("success", true)).toBe("");
  });

  it("pipeline 状态映射：error→error、busy→info、success→success、其他→空", () => {
    expect(conversationStatusIndicatorTone("error", false)).toBe("error");
    expect(conversationStatusIndicatorTone("busy", false)).toBe("info");
    expect(conversationStatusIndicatorTone("success", false)).toBe("success");
    expect(conversationStatusIndicatorTone("", false)).toBe("");
    expect(conversationStatusIndicatorTone("unknown", false)).toBe("");
  });

  it("tone 到颜色 class 映射", () => {
    expect(conversationIndicatorClass("error")).toBe("bg-error");
    expect(conversationIndicatorClass("info")).toBe("bg-warning");
    expect(conversationIndicatorClass("success")).toBe("bg-success");
    expect(conversationIndicatorClass("")).toBe("");
  });
});

describe("conversationSimpleIndicatorClass", () => {
  it("未读优先显示 error 色条", () => {
    const result = conversationSimpleIndicatorClass(item({}), "3", [preview({ role: "assistant" })]);
    expect(result).toBe("bg-error");
  });

  it("无预览消息 → success", () => {
    expect(conversationSimpleIndicatorClass(item({}), "", [])).toBe("bg-success");
  });

  it("tool/system 消息 → warning", () => {
    expect(conversationSimpleIndicatorClass(item({}), "", [preview({ role: "tool" })])).toBe("bg-warning");
    expect(conversationSimpleIndicatorClass(item({}), "", [preview({ role: "system" })])).toBe("bg-warning");
  });

  it("用户消息：无 speakerId 或 user-persona → info，其他 agent → warning", () => {
    expect(conversationSimpleIndicatorClass(item({}), "", [preview({ role: "user", speakerAgentId: "" })])).toBe("bg-info");
    expect(conversationSimpleIndicatorClass(item({}), "", [preview({ role: "user", speakerAgentId: "user-persona" })])).toBe("bg-info");
    expect(conversationSimpleIndicatorClass(item({}), "", [preview({ role: "user", speakerAgentId: "agent-x" })])).toBe("bg-warning");
  });

  it("助手消息 → success", () => {
    expect(conversationSimpleIndicatorClass(item({}), "", [preview({ role: "assistant" })])).toBe("bg-success");
  });

  it("只取最后一条预览判定", () => {
    const previews = [
      preview({ role: "user", speakerAgentId: "agent-x" }),
      preview({ role: "assistant" }),
    ];
    expect(conversationSimpleIndicatorClass(item({}), "", previews)).toBe("bg-success");
  });
});

describe("conversationSummaryClass", () => {
  it("sim 摘要常显两行", () => {
    expect(conversationSummaryClass("sim")).toBe("max-h-8 opacity-60");
  });

  it("mini 折叠一行、hover 展开", () => {
    expect(conversationSummaryClass("mini")).toBe("max-h-0 opacity-0 group-hover:max-h-8 group-hover:opacity-60");
  });

  it("full 无摘要行（不参与简单项摘要逻辑）", () => {
    expect(conversationSummaryClass("full")).toBe("max-h-0 opacity-0 group-hover:max-h-8 group-hover:opacity-60");
  });
});

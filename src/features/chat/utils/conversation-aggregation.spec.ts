import { describe, expect, it } from "vitest";
import type { ChatConversationOverviewItem } from "../../../types/app";
import { aggregateConversationItems, conversationLastUsedMs } from "./conversation-aggregation";

function item(id: string, agentId: string | undefined, updatedAt: string, workspaceRootPath?: string): ChatConversationOverviewItem {
  return {
    conversationId: id,
    title: `会话${id}`,
    messageCount: 0,
    agentId,
    updatedAt,
    lastMessageAt: updatedAt,
    workspaceRootPath,
  };
}

describe("aggregateConversationItems", () => {
  it("应把同 agentId 会话聚合成一块：最新为完整条目，旧会话按时间倒序作为简单条目", () => {
    const items = [
      item("c1", "agent-x", "2026-08-05T10:00:00+08:00"),
      item("c2", "agent-y", "2026-08-05T11:00:00+08:00"),
      item("c3", "agent-x", "2026-08-05T12:00:00+08:00"),
    ];

    const { reorderedItems, simpleFollowers } = aggregateConversationItems(items, { searchActive: false });

    expect(reorderedItems.map((entry) => entry.conversationId)).toEqual(["c2", "c3"]);
    expect(simpleFollowers["c3"].map((entry) => entry.conversationId)).toEqual(["c1"]);
  });

  it("多个同 agentId 会话时简单条目按更新时间倒序（新的在上）", () => {
    const items = [
      item("old", "agent-x", "2026-08-05T08:00:00+08:00"),
      item("mid", "agent-x", "2026-08-05T09:00:00+08:00"),
      item("new", "agent-x", "2026-08-05T10:00:00+08:00"),
    ];

    const { reorderedItems, simpleFollowers } = aggregateConversationItems(items, { searchActive: false });

    expect(reorderedItems.map((entry) => entry.conversationId)).toEqual(["new"]);
    expect(simpleFollowers["new"].map((entry) => entry.conversationId)).toEqual(["mid", "old"]);
  });

  it("空 agentId 的会话不聚合，保持原顺序原形态", () => {
    const items = [
      item("c1", undefined, "2026-08-05T10:00:00+08:00"),
      item("c2", "agent-x", "2026-08-05T11:00:00+08:00"),
      item("c3", undefined, "2026-08-05T12:00:00+08:00"),
    ];

    const { reorderedItems, simpleFollowers } = aggregateConversationItems(items, { searchActive: false });

    expect(reorderedItems.map((entry) => entry.conversationId)).toEqual(["c1", "c2", "c3"]);
    expect(simpleFollowers["c1"]).toBeUndefined();
    expect(simpleFollowers["c3"]).toBeUndefined();
  });

  it("搜索模式下不聚合，保持原顺序原形态", () => {
    const items = [
      item("c1", "agent-x", "2026-08-05T10:00:00+08:00"),
      item("c2", "agent-x", "2026-08-05T12:00:00+08:00"),
    ];

    const { reorderedItems, simpleFollowers } = aggregateConversationItems(items, { searchActive: true });

    expect(reorderedItems.map((entry) => entry.conversationId)).toEqual(["c1", "c2"]);
    expect(simpleFollowers["c1"]).toBeUndefined();
    expect(simpleFollowers["c2"]).toBeUndefined();
  });

  it("聚合块位置由块内最新会话的原位置决定，块间保持原相对顺序", () => {
    const items = [
      item("c1", "agent-x", "2026-08-05T10:00:00+08:00"),
      item("c2", "agent-y", "2026-08-05T11:00:00+08:00"),
      item("c3", "agent-x", "2026-08-05T12:00:00+08:00"),
      item("c4", "agent-y", "2026-08-05T13:00:00+08:00"),
    ];

    const { reorderedItems, simpleFollowers } = aggregateConversationItems(items, { searchActive: false });

    expect(reorderedItems.map((entry) => entry.conversationId)).toEqual(["c3", "c4"]);
    expect(simpleFollowers["c3"].map((entry) => entry.conversationId)).toEqual(["c1"]);
    expect(simpleFollowers["c4"].map((entry) => entry.conversationId)).toEqual(["c2"]);
  });

  it("同一 agentId 但目录不同不聚合", () => {
    const items = [
      item("c1", "agent-x", "2026-08-05T10:00:00+08:00", "E:/repo-a"),
      item("c2", "agent-x", "2026-08-05T12:00:00+08:00", "E:/repo-b"),
      item("c3", "agent-x", "2026-08-05T11:00:00+08:00", "E:/repo-a"),
    ];

    const { reorderedItems, simpleFollowers } = aggregateConversationItems(items, { searchActive: false });

    expect(reorderedItems.map((entry) => entry.conversationId)).toEqual(["c2", "c3"]);
    expect(simpleFollowers["c3"].map((entry) => entry.conversationId)).toEqual(["c1"]);
    expect(simpleFollowers["c2"]).toBeUndefined();
  });

  it("空路径视为默认目录，同 agentId 空路径会话聚合；有路径与无路径不混合", () => {
    const items = [
      item("c1", "agent-x", "2026-08-05T10:00:00+08:00"),
      item("c2", "agent-x", "2026-08-05T12:00:00+08:00"),
      item("c3", "agent-x", "2026-08-05T11:00:00+08:00", "E:/repo-a"),
    ];

    const { reorderedItems, simpleFollowers } = aggregateConversationItems(items, { searchActive: false });

    expect(reorderedItems.map((entry) => entry.conversationId)).toEqual(["c2", "c3"]);
    expect(simpleFollowers["c2"].map((entry) => entry.conversationId)).toEqual(["c1"]);
    expect(simpleFollowers["c3"]).toBeUndefined();
  });
});

describe("conversationLastUsedMs", () => {
  it("优先取 lastMessageAt，缺失时回退 updatedAt", () => {
    expect(conversationLastUsedMs(item("c1", undefined, "2026-08-05T10:00:00+08:00")))
      .toBe(Date.parse("2026-08-05T10:00:00+08:00"));
    const fallback = {
      conversationId: "c2",
      title: "x",
      messageCount: 0,
      updatedAt: "2026-08-05T09:00:00+08:00",
    } satisfies ChatConversationOverviewItem;
    expect(conversationLastUsedMs(fallback)).toBe(Date.parse("2026-08-05T09:00:00+08:00"));
  });
});

import { describe, expect, it } from "vitest";
import { isViewLayerBusy } from "./chat-view-busy";

describe("isViewLayerBusy", () => {
  const base = {
    trimming: false,
    compactingConversation: false,
    activeConversationId: "conversation-a",
    organizingContext: false,
  };

  it("默认不忙碌", () => {
    expect(isViewLayerBusy(base)).toBe(false);
  });

  it("修剪当前会话算忙碌，其他会话不算", () => {
    expect(isViewLayerBusy({ ...base, trimming: true })).toBe(true);
    expect(
      isViewLayerBusy({
        ...base,
        trimming: true,
        trimmingConversationId: "conversation-b",
      }),
    ).toBe(false);
    expect(
      isViewLayerBusy({
        ...base,
        trimming: true,
        trimmingConversationId: "conversation-a",
      }),
    ).toBe(true);
  });

  it("压缩当前会话算忙碌，其他会话不算", () => {
    expect(isViewLayerBusy({ ...base, compactingConversation: true })).toBe(true);
    expect(
      isViewLayerBusy({
        ...base,
        compactingConversation: true,
        compactingConversationId: "conversation-b",
      }),
    ).toBe(false);
    expect(
      isViewLayerBusy({
        ...base,
        compactingConversation: true,
        compactingConversationId: "conversation-a",
      }),
    ).toBe(true);
  });

  it("组织上下文算忙碌", () => {
    expect(isViewLayerBusy({ ...base, organizingContext: true })).toBe(true);
  });

  it("流式态不是忙碌（chatting/streaming 由调用方单独表达，不进入本函数）", () => {
    // 本函数不接收 chatting/streaming 输入：流式时停止按钮必须可用。
    expect(isViewLayerBusy(base)).toBe(false);
  });
});

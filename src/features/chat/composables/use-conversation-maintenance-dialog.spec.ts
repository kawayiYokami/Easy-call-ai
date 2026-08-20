import { ref } from "vue";
import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeTauriMock = vi.hoisted(() => vi.fn());

vi.mock("../../../services/tauri-api", () => ({
  invokeTauri: invokeTauriMock,
}));

import { useConversationMaintenanceDialog } from "./use-conversation-maintenance-dialog";

describe("useConversationMaintenanceDialog", () => {
  beforeEach(() => {
    invokeTauriMock.mockReset();
  });

  it("两端共用同一块页预览和压缩执行入口，占用率直接复用 chatUsagePercent 数据源", async () => {
    const messages = Array.from({ length: 10 }, (_, index) => ({
      id: `message-${index}`,
      role: index % 2 === 0 ? "user" : "assistant",
      parts: [{ type: "text", text: `text-${index}` }],
      providerMeta: undefined,
    }));
    invokeTauriMock.mockResolvedValue({ selectedBlockId: 1, messages });
    const trimCompactNow = vi.fn(async () => {});
    const trimNow = vi.fn(async () => {});
    const deleteConversation = vi.fn(async () => {});
    const flow = useConversationMaintenanceDialog({
      t: (key) => key,
      currentConversationId: ref("conversation-a"),
      conversationSummaries: ref([{
        conversationId: "conversation-a",
        messageCount: 10,
        bodyMessageCount: 10,
        hasAssistantReply: true,
        runtimeState: "idle",
      }]),
      chatUsagePercent: ref(20),
      trimCompactNow,
      trimNow,
      deleteConversation,
      setStatus: vi.fn(),
      setStatusError: vi.fn(),
    });

    await flow.openTrimActionDialog();

    expect(invokeTauriMock).toHaveBeenCalledWith("conversation.blockPage", {
      input: { conversationId: "conversation-a" },
    });
    expect(flow.trimActionDialogOpen.value).toBe(true);
    expect(flow.trimPreview.value?.canArchive).toBe(true);
    expect(flow.trimCompactionPreview.value).toEqual(expect.objectContaining({
      canCompact: true,
      messageCount: 10,
      contextUsagePercent: 20,
    }));

    await flow.confirmTrimCompactionAction();
    expect(trimCompactNow).toHaveBeenCalledTimes(1);
    expect(flow.trimActionDialogOpen.value).toBe(false);
  });

  it("从最后一条 assistant 消息的 providerMeta 读取 system/tools 词元，正文用消息估算", async () => {
    const messages = [
      {
        id: "message-0",
        role: "user",
        parts: [{ type: "text", text: "你好" }],
        providerMeta: undefined,
      },
      {
        id: "message-1",
        role: "assistant",
        parts: [{ type: "text", text: "你好，有什么可以帮你" }],
        providerMeta: {
          contextBreakdown: { systemTokens: 1200, toolsTokens: 3400 },
        },
      },
    ];
    invokeTauriMock.mockResolvedValue({ selectedBlockId: 1, messages });
    const flow = useConversationMaintenanceDialog({
      t: (key) => key,
      currentConversationId: ref("conversation-b"),
      conversationSummaries: ref([{
        conversationId: "conversation-b",
        messageCount: 2,
        bodyMessageCount: 2,
        hasAssistantReply: true,
        runtimeState: "idle",
      }]),
      chatUsagePercent: ref(50),
      trimCompactNow: vi.fn(async () => {}),
      trimNow: vi.fn(async () => {}),
      deleteConversation: vi.fn(async () => {}),
      setStatus: vi.fn(),
      setStatusError: vi.fn(),
    });

    await flow.openTrimActionDialog();

    expect(flow.trimCompactionPreview.value?.tokenBreakdown).toEqual(expect.objectContaining({
      systemTokens: 1200,
      toolsTokens: 3400,
    }));
    expect(flow.trimCompactionPreview.value?.tokenBreakdown?.messageTokens).toBeGreaterThan(0);
  });

  it("providerMeta 缺失时 tokenBreakdown 只含正文估算，system/tools 为 undefined", async () => {
    const messages = Array.from({ length: 3 }, (_, index) => ({
      id: `message-${index}`,
      role: index % 2 === 0 ? "user" : "assistant",
      parts: [{ type: "text", text: `text-${index}` }],
      providerMeta: undefined,
    }));
    invokeTauriMock.mockResolvedValue({ selectedBlockId: 1, messages });
    const flow = useConversationMaintenanceDialog({
      t: (key) => key,
      currentConversationId: ref("conversation-c"),
      conversationSummaries: ref([{
        conversationId: "conversation-c",
        messageCount: 3,
        bodyMessageCount: 3,
        hasAssistantReply: true,
        runtimeState: "idle",
      }]),
      chatUsagePercent: ref(30),
      trimCompactNow: vi.fn(async () => {}),
      trimNow: vi.fn(async () => {}),
      deleteConversation: vi.fn(async () => {}),
      setStatus: vi.fn(),
      setStatusError: vi.fn(),
    });

    await flow.openTrimActionDialog();

    expect(flow.trimCompactionPreview.value?.tokenBreakdown?.systemTokens).toBeUndefined();
    expect(flow.trimCompactionPreview.value?.tokenBreakdown?.toolsTokens).toBeUndefined();
    expect(flow.trimCompactionPreview.value?.tokenBreakdown?.messageTokens).toBeGreaterThan(0);
  });
});

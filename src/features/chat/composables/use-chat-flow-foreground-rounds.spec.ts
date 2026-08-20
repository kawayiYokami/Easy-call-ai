import { ref, shallowRef } from "vue";
import type { ChatMessage } from "../../../types/app";
import { describe, expect, it, vi } from "vitest";
import { useChatFlowForegroundRounds } from "./use-chat-flow-foreground-rounds";

describe("useChatFlowForegroundRounds", () => {
  it("压缩中且尚无后端消息 id 时不显示 assistant 气泡，直到 round_started", () => {
    const allMessages = shallowRef<ChatMessage[]>([]);
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const streamBlocks = ref([]);
    let round: { phase: "idle" | "queued" | "streaming"; gen?: number; messageId?: string } = { phase: "idle" };
    let cache: any = null;

    const flow = useChatFlowForegroundRounds({
      allMessages,
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      streamBlocks,
      t: () => "调度中",
      getConversationId: () => "conversation-1",
      getRound: () => ({ phase: round.phase, gen: round.gen || 0, messageId: round.messageId || "" }),
      setRound: (next: typeof round) => { round = next; },
      nextGeneration: () => 1,
      getSendChatActiveGen: () => 0,
      setActiveActivationId: () => {},
      getActiveActivationId: () => "",
      setActiveRoundAgentId: () => {},
      channelBinding: { setBoundDisplayGeneration: () => {} },
      clearConversationStreamCache: () => { cache = null; },
      readConversationStreamCache: () => cache,
      writeConversationStreamCacheSnapshot: (_conversationId: string, nextCache: any) => { cache = { ...cache, ...nextCache }; },
      setPendingTerminalEvent: () => {},
      setDeferredRoundCompletion: () => {},
      setQueuedStreamingState: () => {},
      setActiveHistoryMessageCount: () => {},
      startFrontendDispatchTimer: () => {},
      sendStartedAtMsByGen: new Map<number, number>(),
      setFrontendRoundPhase: () => {},
      chatting: ref(false),
      updateQueuedAssistantMessageStatus: (messageId: string, statusText: string) => {
        if (!messageId || allMessages.value.some((message) => message.id === messageId)) return;
        allMessages.value = [{
          id: messageId,
          role: "assistant",
          parts: [{ type: "text", text: "" }],
          providerMeta: { _streaming: true, _preStreamingStatusText: statusText },
        }];
      },
      hasStreamingAssistantMessageInMessages: () => allMessages.value.some((message) => message.providerMeta?._streaming === true),
      insertStreamingAssistantMessage: (messageId: string) => messageId,
      applyConversationStreamCacheToDisplay: () => false,
      loadStreamBlocksFromMessage: () => {},
      applyPendingTerminalEvent: () => false,
      getQueuedStreamingState: () => null,
      updateMessageText: () => {},
      frontendDispatch: { getStartedAtMs: () => 0, getElapsedMs: () => 0 },
    });

    flow.resumeForegroundRuntimeRound({
      conversationId: "conversation-1",
      streamCache: { hasVisibleProgress: false },
    });

    expect(round).toMatchObject({ phase: "queued", messageId: "" });
    expect(allMessages.value).toHaveLength(0);

    flow.beginAssistantActivationFromEvent({
      conversationId: "conversation-1",
      assistantMessageId: "assistant-1",
      activationId: "activation-1",
    });

    expect(allMessages.value).toHaveLength(1);
    expect(allMessages.value[0].id).toBe("assistant-1");
    expect(round).toMatchObject({ phase: "queued", messageId: "assistant-1" });
  });

  it("streaming 中消息已存在且缓存快照匹配时，恢复会用缓存 streamBlocks 更新消息文本", () => {
    const allMessages = shallowRef<ChatMessage[]>([{
      id: "assistant-1",
      role: "assistant",
      parts: [{ type: "text", text: "旧内容" }],
      providerMeta: { _streaming: true },
    }]);
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const streamBlocks = ref([]);
    let round: { phase: "idle" | "queued" | "streaming"; gen?: number; messageId?: string } = {
      phase: "streaming",
      gen: 1,
      messageId: "assistant-1",
    };
    let cache: any = {
      persistedAssistantMessageId: "assistant-1",
      streamBlocks: [{ type: "text", text: "缓存最新内容" }],
      toolStatusText: "运行中",
      toolStatusState: "running",
    };
    const updateMessageText = vi.fn();

    const flow = useChatFlowForegroundRounds({
      allMessages,
      latestAssistantText,
      toolStatusText,
      toolStatusState,
      streamBlocks,
      t: () => "调度中",
      getConversationId: () => "conversation-1",
      getRound: () => ({ phase: round.phase, gen: round.gen || 0, messageId: round.messageId || "" }),
      setRound: (next: typeof round) => { round = next; },
      nextGeneration: () => 1,
      getSendChatActiveGen: () => 0,
      setActiveActivationId: () => {},
      getActiveActivationId: () => "",
      setActiveRoundAgentId: () => {},
      channelBinding: { setBoundDisplayGeneration: () => {} },
      clearConversationStreamCache: () => { cache = null; },
      readConversationStreamCache: () => cache,
      writeConversationStreamCacheSnapshot: (_conversationId: string, nextCache: any) => { cache = { ...cache, ...nextCache }; },
      setPendingTerminalEvent: () => {},
      setDeferredRoundCompletion: () => {},
      setQueuedStreamingState: () => {},
      setActiveHistoryMessageCount: () => {},
      startFrontendDispatchTimer: () => {},
      sendStartedAtMsByGen: new Map<number, number>(),
      setFrontendRoundPhase: () => {},
      chatting: ref(false),
      updateQueuedAssistantMessageStatus: () => {},
      hasStreamingAssistantMessageInMessages: () => allMessages.value.some((message) => message.providerMeta?._streaming === true),
      insertStreamingAssistantMessage: (messageId: string) => messageId,
      applyConversationStreamCacheToDisplay: () => false,
      loadStreamBlocksFromMessage: () => {},
      applyPendingTerminalEvent: () => false,
      getQueuedStreamingState: () => null,
      updateMessageText,
      frontendDispatch: { getStartedAtMs: () => 0, getElapsedMs: () => 0 },
    });

    flow.ensureForegroundStreamingRound();

    expect(updateMessageText).toHaveBeenCalledTimes(1);
    expect(updateMessageText).toHaveBeenCalledWith(
      "assistant-1",
      expect.arrayContaining([expect.objectContaining({ text: "缓存最新内容" })]),
    );
  });
});

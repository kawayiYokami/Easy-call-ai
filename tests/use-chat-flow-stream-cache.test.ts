import { describe, expect, it } from "vitest";
import { ref } from "vue";
import {
  streamCacheHasVisibleProgress,
  useChatFlowStreamCache,
} from "../src/features/chat/composables/use-chat-flow-stream-cache";
import type { AssistantStreamBlock } from "../src/types/app";

describe("useChatFlowStreamCache stream block snapshots", () => {
  it("restores visible thinking progress from stream block runtime snapshots", () => {
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const streamBlocks = ref<AssistantStreamBlock[]>([]);
    const restoredTimers: unknown[] = [];

    const cache = useChatFlowStreamCache({
      getConversationId: () => "conversation-1",
      getCurrentDisplayState: () => ({
        assistantText: latestAssistantText.value,
        toolStatusText: toolStatusText.value,
        toolStatusState: toolStatusState.value,
        streamBlocks: streamBlocks.value,
      }),
      getActiveActivationId: () => "request-1",
      getFrontendDispatchStartedAtMs: () => 100,
      getFrontendDispatchElapsedMs: () => 8,
      currentFrontendDispatchElapsedMs: () => 8,
      restoreFrontendDispatchTimerFromCache: (snapshot) => {
        restoredTimers.push(snapshot);
      },
    });

    const reasoningBlock: AssistantStreamBlock = { reasoning: "R1" };

    cache.writeConversationStreamCacheSnapshot("conversation-1", {
      activationId: "request-1",
      requestId: "request-1",
      assistantText: "",
      toolStatusText: "",
      toolStatusState: "",
      streamBlocks: [reasoningBlock],
    });

    expect(streamCacheHasVisibleProgress(cache.readConversationStreamCache("conversation-1"))).toBe(true);
    expect(cache.applyConversationStreamCacheToDisplay("conversation-1")).toBe(true);
    expect(cache.readConversationStreamCache("conversation-1")?.streamBlocks).toEqual([{
      reasoning: "R1",
      reasoningCharCount: 2,
      text: "",
      tools: [],
      pendingTextBreak: false,
    }]);
    expect(cache.readConversationStreamCache("conversation-1")?.assistantText).toBe("");
    expect(restoredTimers).toHaveLength(1);
  });

  it("can apply active stream snapshots after channel and generation already matched", () => {
    const latestAssistantText = ref("");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const streamBlocks = ref<AssistantStreamBlock[]>([]);

    const cache = useChatFlowStreamCache({
      getConversationId: () => "conversation-1",
      getCurrentDisplayState: () => ({
        assistantText: latestAssistantText.value,
        toolStatusText: toolStatusText.value,
        toolStatusState: toolStatusState.value,
        streamBlocks: streamBlocks.value,
      }),
      getActiveActivationId: () => "foreground-activation",
      getFrontendDispatchStartedAtMs: () => 100,
      getFrontendDispatchElapsedMs: () => 8,
      currentFrontendDispatchElapsedMs: () => 8,
      restoreFrontendDispatchTimerFromCache: () => {},
    });

    expect(cache.applyConversationStreamCacheSnapshotToDisplay("conversation-1", {
      activationId: "backend-activation",
      requestId: "backend-activation",
      streamBlocks: [{ reasoning: "R2" }],
    })).toBe(false);

    expect(cache.applyConversationStreamCacheSnapshotToDisplay("conversation-1", {
      activationId: "backend-activation",
      requestId: "backend-activation",
      streamBlocks: [{ reasoning: "R2" }],
    }, { ignoreActivationId: true })).toBe(true);
    expect(cache.readConversationStreamCache("conversation-1")?.streamBlocks).toEqual([{
      reasoning: "R2",
      reasoningCharCount: 2,
      text: "",
      tools: [],
      pendingTextBreak: false,
    }]);
  });

  it("preserves persisted assistant ids across cache reads and later display sync writes", () => {
    const latestAssistantText = ref("A1");
    const toolStatusText = ref("");
    const toolStatusState = ref<"running" | "done" | "failed" | "">("");
    const streamBlocks = ref<AssistantStreamBlock[]>([]);

    const cache = useChatFlowStreamCache({
      getConversationId: () => "conversation-1",
      getCurrentDisplayState: () => ({
        assistantText: latestAssistantText.value,
        toolStatusText: toolStatusText.value,
        toolStatusState: toolStatusState.value,
        streamBlocks: streamBlocks.value,
      }),
      getActiveActivationId: () => "request-1",
      getFrontendDispatchStartedAtMs: () => 100,
      getFrontendDispatchElapsedMs: () => 8,
      currentFrontendDispatchElapsedMs: () => 8,
      restoreFrontendDispatchTimerFromCache: () => {},
    });

    cache.writeConversationStreamCacheSnapshot("conversation-1", {
      activationId: "request-1",
      requestId: "request-1",
      persistedAssistantMessageId: "assistant-1",
    });
    expect(cache.readConversationStreamCache("conversation-1")?.persistedAssistantMessageId).toBe("assistant-1");

    cache.syncCurrentDisplayStateToConversationStreamCache("conversation-1");
    expect(cache.readConversationStreamCache("conversation-1")?.persistedAssistantMessageId).toBe("assistant-1");

    cache.applyAssistantEventToConversationStreamCache("conversation-1", {
      kind: "assistant_delta",
      message: "delta-1",
    });
    expect(cache.readConversationStreamCache("conversation-1")?.persistedAssistantMessageId).toBe("assistant-1");
  });
});

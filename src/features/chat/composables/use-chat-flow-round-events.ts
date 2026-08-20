import type { Ref } from "vue";
import type { AssistantStreamBlock, ChatMessage } from "../../../types/app";
import {
  readHistoryFlushedPayload,
  type AssistantDeltaEvent,
} from "./use-chat-flow-events";
import type { PendingTerminalEvent, RoundState } from "./use-chat-flow-types";

type UseChatFlowRoundEventsOptions = {
  chatting: Ref<boolean>;
  allMessages: Ref<ChatMessage[]>;
  reasoningStartedAtMs: Ref<number>;
  getRound: () => RoundState;
  setRound: (next: RoundState, frontendPhase?: "idle" | "queued" | "waiting" | "streaming") => void;
  getGeneration: () => number;
  setPendingTerminalEvent: (event: PendingTerminalEvent | null) => void;
  getPendingTerminalEvent: () => PendingTerminalEvent | null;
  setDeferredRoundCompletion: (event: {
    gen: number;
    result: {
      assistantText: string;
      assistantMessage?: ChatMessage;
      activationId?: string;
      requestId?: string;
    };
  } | null) => void;
  clearConversationStreamCache: (conversationId?: string | null) => void;
  clearFrontendDispatchTimer: () => void;
  setActiveActivationId: (value: string) => void;
  setSendChatActiveGen: (value: number) => void;
  sendStartedAtMsByGen: Map<number, number>;
  hasStreamingAssistantMessageInMessages: () => boolean;
  applyConversationStreamCacheToDisplay: (conversationId?: string | null) => boolean;
  updateQueuedAssistantMessageStatus: (messageId: string, statusText: string) => void;
  insertStreamingAssistantMessage: (messageId: string, gen?: number, initialText?: string) => string;
  updateMessageText: (
    messageId: string,
    rawBlocks?: AssistantStreamBlock[],
    updateOptions?: { preserveActivityProjection?: boolean },
    runtimeStatus?: { toolStatusText?: string; toolStatusState?: string },
  ) => void;
  finalizeMessage: (
    messageId: string,
    finalMessage?: ChatMessage,
    identity?: { activationId?: string; requestId?: string },
  ) => void;
  failMessage: (
    messageId: string,
    error?: unknown,
    identity?: { activationId?: string; requestId?: string },
  ) => void;
  syncStreamBlocksToMessage: (messageId: string) => void;
  applyPendingTerminalEvent: (gen: number) => boolean;
  promoteQueuedRoundToStreaming: (gen: number) => number;
  finalizeDeferredRoundCompletion: () => Promise<void>;
  finalizeQueuedRoundWithoutMessage: (
    gen: number,
    result: {
      assistantText: string;
      assistantMessage?: ChatMessage;
      activationId?: string;
      requestId?: string;
    },
  ) => Promise<void>;
  failQueuedRoundWithoutMessage: (
    gen: number,
    error: unknown,
    identity?: { activationId?: string; requestId?: string },
  ) => Promise<void>;
  enqueueStreamDelta: (gen: number, delta: string) => void;
  setChatErrorText: (text: string, conversationId?: string | null) => void;
  formatRequestFailed: (error: unknown) => string;
  onReloadMessages: () => Promise<void>;
  optionsT: (key: string, params?: Record<string, unknown>) => string;
};

export function useChatFlowRoundEvents(options: UseChatFlowRoundEventsOptions) {
  async function handleHistoryFlushed(
    gen: number,
    parsed: AssistantDeltaEvent,
    source: "sendChat" | "bound",
  ) {
    const flushed = readHistoryFlushedPayload(parsed.message);
    const startedAtMs = options.sendStartedAtMsByGen.get(gen) || 0;
    const elapsedMs = startedAtMs > 0 ? Math.max(0, Date.now() - startedAtMs) : -1;
    const wasQueuedForActivation = !!flushed?.activateAssistant;
    const shouldForceReset = !!flushed?.compactionApplied;
    if (shouldForceReset) {
      options.clearConversationStreamCache();
      options.clearFrontendDispatchTimer();
      options.setActiveActivationId("");
      options.setPendingTerminalEvent(null);
      options.setDeferredRoundCompletion(null);
    }
    if (wasQueuedForActivation) {
      const existingRound = options.getRound();
      if (existingRound.phase === "queued" && existingRound.gen === gen) {
        options.setRound({ phase: "queued", gen, messageId: existingRound.messageId }, "waiting");
        options.updateQueuedAssistantMessageStatus(existingRound.messageId, options.optionsT("chat.statusWaitingReply"));
      }
      options.chatting.value = true;
      return;
    }
    if (gen !== options.getGeneration()) return;
    options.setRound({ phase: "idle" });
    options.clearFrontendDispatchTimer();
    options.chatting.value = false;
  }

  async function markRoundStarted(gen: number) {
    const round = options.getRound();
    if (round.phase !== "queued" || round.gen !== gen) return;
    if (options.getPendingTerminalEvent() && options.getPendingTerminalEvent()?.gen === gen) {
      const pending = options.getPendingTerminalEvent();
      options.setPendingTerminalEvent(null);
      options.setDeferredRoundCompletion(null);
      if (pending?.kind === "completed") {
        await options.finalizeQueuedRoundWithoutMessage(gen, pending.result);
        return;
      }
      await options.failQueuedRoundWithoutMessage(gen, pending?.error, {
        activationId: pending?.activationId,
        requestId: pending?.requestId,
      });
      return;
    }
    options.updateQueuedAssistantMessageStatus(round.messageId, options.optionsT("chat.statusWaitingReply"));
    options.chatting.value = true;
  }

  async function handleRoundCompleted(
    gen: number,
    result: {
      assistantText: string;
      assistantMessage?: ChatMessage;
      activationId?: string;
      requestId?: string;
    },
  ) {
    options.sendStartedAtMsByGen.delete(gen);
    const round = options.getRound();
    if (round.phase === "queued" && round.gen === gen) {
      await options.finalizeQueuedRoundWithoutMessage(gen, result);
      return;
    }
    if (round.phase !== "streaming" || round.gen !== gen) return;
    options.setDeferredRoundCompletion({ gen, result });
    await options.finalizeDeferredRoundCompletion();
  }

  async function handleRoundFailed(
    gen: number,
    error: unknown,
    identity?: { activationId?: string; requestId?: string },
  ) {
    options.sendStartedAtMsByGen.delete(gen);
    const round = options.getRound();
    if (round.phase === "queued" && round.gen === gen) {
      await options.failQueuedRoundWithoutMessage(gen, error, identity);
      return;
    }
    if (round.phase !== "streaming" || round.gen !== gen) return;
    options.clearConversationStreamCache();
    options.clearFrontendDispatchTimer();
    options.setActiveActivationId("");
    options.setChatErrorText(options.formatRequestFailed(error));
    const failedMessage = options.allMessages.value.find((message) => message.id === round.messageId);
    const failedMeta = (failedMessage?.providerMeta || {}) as Record<string, unknown>;
    if (!String(failedMeta._toolStatusText || "").trim()) {
      options.updateMessageText(round.messageId, undefined, undefined, {
        toolStatusText: options.optionsT("status.toolCallFailed"),
        toolStatusState: "failed",
      });
    } else {
      options.updateMessageText(round.messageId);
    }
    // streaming failed: 保留已经显示出来的内容，只结束流式态，不再重载。
    options.failMessage(round.messageId, error, identity);
    options.setRound({ phase: "idle" });
    options.chatting.value = false;
    options.reasoningStartedAtMs.value = 0;
  }

  function applyPendingTerminalEvent(gen: number) {
    const pending = options.getPendingTerminalEvent();
    if (!pending || pending.gen !== gen) return false;
    options.setPendingTerminalEvent(null);
    options.setDeferredRoundCompletion(null);
    if (pending.kind === "completed") {
      void handleRoundCompleted(gen, pending.result);
      return true;
    }
    void handleRoundFailed(gen, pending.error, {
      activationId: pending.activationId,
      requestId: pending.requestId,
    });
    return true;
  }

  return {
    applyPendingTerminalEvent,
    handleHistoryFlushed,
    handleRoundCompleted,
    handleRoundFailed,
    markRoundStarted,
  };
}

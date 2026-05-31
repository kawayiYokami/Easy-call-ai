import type { Ref } from "vue";
import type { ChatMessage } from "../../../types/app";
import {
  appendReasoningStandardDelta,
  readDeltaMessage,
  readHistoryFlushedPayload,
  readRoundCompletedPayload,
  readRoundFailedPayload,
  type AssistantDeltaEvent,
  type HistoryFlushedPayload,
  type RoundCompletedPayload,
  type RoundFailedPayload,
  type RoundStartedPayload,
} from "./use-chat-flow-events";
import type { PendingTerminalEvent, RoundState } from "./use-chat-flow-types";

type UseChatFlowRoundEventsOptions = {
  chatting: Ref<boolean>;
  latestAssistantText: Ref<string>;
  latestReasoningStandardText: Ref<string>;
  latestReasoningInlineText: Ref<string>;
  toolStatusText: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  streamToolCalls?: Ref<any[]>;
  reasoningStartedAtMs: Ref<number>;
  getRound: () => RoundState;
  setRound: (next: RoundState, frontendPhase?: "idle" | "queued" | "waiting" | "streaming") => void;
  getGeneration: () => number;
  setPendingTerminalEvent: (event: PendingTerminalEvent | null) => void;
  getPendingTerminalEvent: () => PendingTerminalEvent | null;
  setDeferredRoundCompletion: (event: { gen: number; result: { assistantText: string; reasoningStandard?: string; reasoningInline?: string; assistantMessage?: ChatMessage } } | null) => void;
  clearConversationStreamCache: (conversationId?: string | null) => void;
  clearFrontendDispatchTimer: () => void;
  setActiveActivationId: (value: string) => void;
  setSendChatActiveGen: (value: number) => void;
  sendStartedAtMsByGen: Map<number, number>;
  hasAssistantDraftInMessages: () => boolean;
  applyConversationStreamCacheToDisplay: (conversationId?: string | null) => boolean;
  loadStreamToolCallsFromDraft: (draftId: string) => void;
  updateQueuedAssistantDraftStatus: (draftId: string, statusText: string) => void;
  insertDraft: (gen: number, initialText?: string) => string;
  updateDraftText: (draftId: string) => void;
  syncStreamToolCallsToDraft: (draftId: string) => void;
  applyPendingTerminalEvent: (gen: number) => boolean;
  promoteQueuedRoundToStreaming: (gen: number) => number;
  finalizeDeferredRoundCompletion: () => void;
  finalizeQueuedRoundWithoutDraft: (
    gen: number,
    result: { assistantText: string; reasoningStandard?: string; reasoningInline?: string; assistantMessage?: ChatMessage },
  ) => Promise<void>;
  failQueuedRoundWithoutDraft: (gen: number, error: unknown) => Promise<void>;
  enqueueStreamDelta: (gen: number, delta: string) => void;
  setPendingReasoningStandardBreak: (value: boolean) => void;
  getPendingReasoningStandardBreak: () => boolean;
  getStreamToolCallCount: () => number;
  setStreamToolCallCount: (value: number) => void;
  getStreamLastToolName: () => string;
  setStreamLastToolName: (value: string) => void;
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
      options.setRound({ phase: "queued", gen }, "waiting");
      options.chatting.value = true;
      options.updateQueuedAssistantDraftStatus(`__draft_assistant__:${gen}`, options.optionsT("chat.statusWaitingReply"));
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
        await options.finalizeQueuedRoundWithoutDraft(gen, pending.result);
        return;
      }
      await options.failQueuedRoundWithoutDraft(gen, pending?.error);
      return;
    }
    options.updateQueuedAssistantDraftStatus(`__draft_assistant__:${gen}`, options.optionsT("chat.statusWaitingReply"));
    options.chatting.value = true;
  }

  async function handleRoundCompleted(
    gen: number,
    result: {
      assistantText: string;
      reasoningStandard?: string;
      reasoningInline?: string;
      assistantMessage?: ChatMessage;
    },
  ) {
    options.sendStartedAtMsByGen.delete(gen);
    const round = options.getRound();
    if (round.phase === "queued" && round.gen === gen) {
      await options.finalizeQueuedRoundWithoutDraft(gen, result);
      return;
    }
    if (round.phase !== "streaming" || round.gen !== gen) return;
    options.setDeferredRoundCompletion({ gen, result });
    options.finalizeDeferredRoundCompletion();
  }

  async function handleRoundFailed(gen: number, error: unknown) {
    options.sendStartedAtMsByGen.delete(gen);
    const round = options.getRound();
    if (round.phase === "queued" && round.gen === gen) {
      await options.failQueuedRoundWithoutDraft(gen, error);
      return;
    }
    if (round.phase !== "streaming" || round.gen !== gen) return;
    options.clearConversationStreamCache();
    options.clearFrontendDispatchTimer();
    options.setActiveActivationId("");
    options.latestAssistantText.value = "";
    options.latestReasoningStandardText.value = "";
    options.latestReasoningInlineText.value = "";
    options.setPendingReasoningStandardBreak(false);
    options.setChatErrorText(options.formatRequestFailed(error));
    if (!options.toolStatusText.value) {
      options.toolStatusState.value = "failed";
      options.toolStatusText.value = options.optionsT("status.toolCallFailed");
    }
    options.setRound({ phase: "idle" });
    options.chatting.value = false;
    options.reasoningStartedAtMs.value = 0;
    await options.onReloadMessages();
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
    void handleRoundFailed(gen, pending.error);
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

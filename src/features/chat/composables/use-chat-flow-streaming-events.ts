import type { Ref } from "vue";
import {
  appendReasoningStandardDelta,
  assistantEventHasVisibleProgress,
  readDeltaMessage,
  readRoundCompletedPayload,
  readRoundFailedPayload,
  type AssistantDeltaEvent,
} from "./use-chat-flow-events";
import { applyToolStatusToStreamToolCalls, type StreamToolCallView } from "./use-chat-flow-tool-calls";
import type { PendingTerminalEvent, RoundState } from "./use-chat-flow-types";

type UseChatFlowStreamingEventsOptions = {
  latestReasoningStandardText: Ref<string>;
  latestReasoningInlineText: Ref<string>;
  toolStatusText: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  streamToolCalls?: Ref<StreamToolCallView[]>;
  reasoningStartedAtMs: Ref<number>;
  getRound: () => RoundState;
  promoteQueuedRoundToStreaming: (gen: number) => number;
  setPendingTerminalEvent: (event: PendingTerminalEvent | null) => void;
  clearConversationStreamCache: (conversationId?: string | null) => void;
  getConversationId?: () => string;
  setActiveActivationId: (value: string) => void;
  handleRoundCompleted: (
    gen: number,
    result: {
      assistantText: string;
      reasoningStandard?: string;
      reasoningInline?: string;
      assistantMessage?: any;
    },
  ) => Promise<void>;
  handleRoundFailed: (gen: number, error: unknown) => Promise<void>;
  getPendingReasoningStandardBreak: () => boolean;
  setPendingReasoningStandardBreak: (value: boolean) => void;
  getStreamToolCallCount: () => number;
  setStreamToolCallCount: (value: number) => void;
  getStreamLastToolName: () => string;
  setStreamLastToolName: (value: string) => void;
  syncStreamToolCallsToDraft: (draftId: string) => void;
  syncCurrentDisplayStateToConversationStreamCache: () => void;
  updateDraftText: (draftId: string) => void;
  enqueueStreamDelta: (gen: number, delta: string) => void;
};

export function useChatFlowStreamingEvents(options: UseChatFlowStreamingEventsOptions) {
  function handleStreamingEvent(currentGen: number, parsed: AssistantDeltaEvent) {
    if (!currentGen) return;
    const round = options.getRound();
    if (round.phase === "queued" && round.gen === currentGen && assistantEventHasVisibleProgress(parsed)) {
      options.promoteQueuedRoundToStreaming(currentGen);
    }
    const currentRound = options.getRound();
    if (currentRound.phase !== "streaming" && currentRound.phase !== "queued") return;
    if (currentRound.gen !== currentGen) return;

    if (parsed.kind === "round_completed") {
      const p = readRoundCompletedPayload(parsed.message);
      const result = {
        assistantText: String(p?.assistantText || ""),
        reasoningStandard: p?.reasoningStandard,
        reasoningInline: p?.reasoningInline,
        assistantMessage: p?.assistantMessage,
      };
      if (currentRound.phase === "queued") {
        options.setPendingTerminalEvent({
          kind: "completed",
          gen: currentGen,
          result,
        });
        options.clearConversationStreamCache(options.getConversationId ? options.getConversationId() : "");
        options.setActiveActivationId("");
        return;
      }
      void options.handleRoundCompleted(currentGen, result);
      return;
    }

    if (parsed.kind === "round_failed") {
      const p = readRoundFailedPayload(parsed.message);
      const error = p?.error || parsed.message || JSON.stringify(parsed);
      if (currentRound.phase === "queued") {
        options.setPendingTerminalEvent({
          kind: "failed",
          gen: currentGen,
          error,
        });
        options.clearConversationStreamCache(options.getConversationId ? options.getConversationId() : "");
        options.setActiveActivationId("");
        return;
      }
      void options.handleRoundFailed(currentGen, error);
      return;
    }

    if (parsed.kind === "tool_status") {
      if (String(options.latestReasoningStandardText.value || "").trim()) {
        options.setPendingReasoningStandardBreak(true);
      }
      const toolName = String(parsed.toolName || "").trim();
      if (options.streamToolCalls) {
        const statusUpdate = applyToolStatusToStreamToolCalls(options.streamToolCalls.value, parsed);
        options.streamToolCalls.value = statusUpdate.calls;
        if (parsed.toolStatus === "running" && toolName && parsed.toolCallId) {
          options.setStreamLastToolName(toolName);
          if (statusUpdate.appended) {
            options.setStreamToolCallCount(options.getStreamToolCallCount() + 1);
          }
        } else if (statusUpdate.appended) {
          options.setStreamToolCallCount(Math.max(options.getStreamToolCallCount(), options.streamToolCalls.value.length));
        }
      } else if (parsed.toolStatus === "running" && toolName && parsed.toolCallId) {
        options.setStreamLastToolName(toolName);
      }
      if (currentRound.phase === "streaming") {
        options.syncStreamToolCallsToDraft(currentRound.draftId);
      }
      options.toolStatusText.value = parsed.message || "";
      options.toolStatusState.value =
        parsed.toolStatus === "running" || parsed.toolStatus === "done" || parsed.toolStatus === "failed"
          ? parsed.toolStatus : "";
      options.syncCurrentDisplayStateToConversationStreamCache();
      if (currentRound.phase === "streaming") {
        options.updateDraftText(currentRound.draftId);
      }
      return;
    }

    if (parsed.kind === "reasoning_standard") {
      const dt = readDeltaMessage(parsed);
      if (dt && options.reasoningStartedAtMs.value === 0) options.reasoningStartedAtMs.value = Date.now();
      options.latestReasoningStandardText.value = appendReasoningStandardDelta(
        options.latestReasoningStandardText.value,
        dt,
        options.getPendingReasoningStandardBreak(),
      );
      if (dt.trim()) options.setPendingReasoningStandardBreak(false);
      options.syncCurrentDisplayStateToConversationStreamCache();
      if (currentRound.phase === "streaming") {
        options.updateDraftText(currentRound.draftId);
      }
      return;
    }

    if (parsed.kind === "reasoning_inline") {
      const dt = readDeltaMessage(parsed);
      if (dt && options.reasoningStartedAtMs.value === 0) options.reasoningStartedAtMs.value = Date.now();
      options.latestReasoningInlineText.value += dt;
      options.syncCurrentDisplayStateToConversationStreamCache();
      if (currentRound.phase === "streaming") {
        options.updateDraftText(currentRound.draftId);
      }
      return;
    }

    options.enqueueStreamDelta(currentGen, readDeltaMessage(parsed));
    options.syncCurrentDisplayStateToConversationStreamCache();
  }

  return {
    handleStreamingEvent,
  };
}

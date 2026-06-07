import type { Ref } from "vue";
import type { AssistantStreamBlock } from "../../../types/app";
import {
  normalizeAssistantStreamBlocks,
  streamBlocksActivitySignature,
} from "../../../utils/chat-message-semantics";
import {
  assistantEventHasVisibleProgress,
  readDeltaMessage,
  readContextUsageUpdatePayload,
  readRoundCompletedPayload,
  readRoundFailedPayload,
  type AssistantDeltaEvent,
  type ContextUsageUpdatePayload,
} from "./use-chat-flow-events";
import type { PendingTerminalEvent, RoundState } from "./use-chat-flow-types";

type UseChatFlowStreamingEventsOptions = {
  toolStatusText: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  streamBlocks?: Ref<AssistantStreamBlock[]>;
  contextUsagePreview?: Ref<ContextUsageUpdatePayload | null>;
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
      assistantMessage?: any;
    },
  ) => Promise<void>;
  handleRoundFailed: (gen: number, error: unknown) => Promise<void>;
  getDraftStreamBlocks: (draftId: string) => AssistantStreamBlock[];
  syncStreamBlocksToDraft: (draftId: string, rawBlocks?: AssistantStreamBlock[]) => void;
  syncCurrentDisplayStateToConversationStreamCache: () => void;
  applyConversationStreamCacheSnapshotToDisplay: (
    conversationId: string,
    snapshot?: any,
    input?: { ignoreActivationId?: boolean },
  ) => boolean;
  updateDraftText: (
    draftId: string,
    streamSegments?: string[],
    streamTail?: string,
    streamAnimatedDelta?: string,
    rawBlocks?: AssistantStreamBlock[],
    updateOptions?: { preserveActivityProjection?: boolean },
  ) => void;
  enqueueStreamDelta: (gen: number, delta: string) => void;
};

export function useChatFlowStreamingEvents(options: UseChatFlowStreamingEventsOptions) {
  function streamBlockStats(rawBlocks: unknown = options.streamBlocks?.value || []) {
    const blocks = normalizeAssistantStreamBlocks(rawBlocks);
    return {
      blockCount: blocks.length,
      reasoningLen: blocks.reduce((total, block) => total + String(block.reasoning || "").length, 0),
      textLen: blocks.reduce((total, block) => total + String(block.text || "").length, 0),
      toolCount: blocks.reduce((total, block) => total + (block.tools || []).length, 0),
    };
  }

  function shouldCorrectDraftProjectionFromSnapshot(
    draftId: string,
    snapshotBlocks: AssistantStreamBlock[],
  ): boolean {
    if (!draftId || snapshotBlocks.length <= 0) return false;
    const currentDraftBlocks = options.getDraftStreamBlocks(draftId);
    return streamBlocksActivitySignature(currentDraftBlocks) !== streamBlocksActivitySignature(snapshotBlocks);
  }

  function handleStreamingEvent(currentGen: number, parsed: AssistantDeltaEvent) {
    const snapshotStats = streamBlockStats(parsed.streamCache?.streamBlocks);
    if (!currentGen) {
      console.warn("[聊天流式块][前端关键] 丢弃：currentGen 为空", {
        kind: parsed.kind || "delta",
        hasStreamCache: !!parsed.streamCache,
        snapshot: snapshotStats,
        round: options.getRound(),
      });
      return;
    }
    const round = options.getRound();
    if (round.phase === "queued" && round.gen === currentGen && assistantEventHasVisibleProgress(parsed)) {
      options.promoteQueuedRoundToStreaming(currentGen);
    }
    const currentRound = options.getRound();
    if (currentRound.phase !== "streaming" && currentRound.phase !== "queued") {
      console.warn("[聊天流式块][前端关键] 丢弃：当前轮次不可接收", {
        currentGen,
        kind: parsed.kind || "delta",
        snapshot: snapshotStats,
        currentRound,
      });
      return;
    }
    if (currentRound.gen !== currentGen) {
      console.warn("[聊天流式块][前端关键] 丢弃：generation 不一致", {
        currentGen,
        kind: parsed.kind || "delta",
        snapshot: snapshotStats,
        currentRound,
      });
      return;
    }
    if (parsed.kind === "context_usage_update") {
      const p = readContextUsageUpdatePayload(parsed.message);
      const activeConversationId = options.getConversationId ? options.getConversationId() : "";
      if (p && (!activeConversationId || p.conversationId === activeConversationId)) {
        if (options.contextUsagePreview) {
          options.contextUsagePreview.value = p;
        }
      }
      return;
    }
    if (parsed.kind === "round_completed") {
      if (options.contextUsagePreview) {
        options.contextUsagePreview.value = null;
      }
      const p = readRoundCompletedPayload(parsed.message);
      const result = {
        assistantText: String(p?.assistantText || ""),
        assistantMessage: p?.assistantMessage,
      };
      if (currentRound.phase === "queued" && parsed.reason === "context_compaction_boundary") {
        void options.handleRoundCompleted(currentGen, result);
        return;
      }
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
      if (options.contextUsagePreview) {
        options.contextUsagePreview.value = null;
      }
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

    const conversationId = options.getConversationId ? options.getConversationId() : "";
    const delta = readDeltaMessage(parsed);
    const isActivityProjectionEvent =
      parsed.kind === "activity_reasoning_delta"
      || parsed.kind === "assistant_tool_event"
      || parsed.kind === "assistant_tool_result";
    let shouldCorrectProjectionFromSnapshot = false;
    let authoritativeBlocks: AssistantStreamBlock[] = currentRound.phase === "streaming"
      ? options.getDraftStreamBlocks(currentRound.draftId)
      : [];
    if (conversationId && parsed.streamCache) {
      const snapshotBlocks = normalizeAssistantStreamBlocks(parsed.streamCache.streamBlocks);
      if (currentRound.phase === "streaming") {
        shouldCorrectProjectionFromSnapshot = shouldCorrectDraftProjectionFromSnapshot(
          currentRound.draftId,
          snapshotBlocks,
        );
      }
      options.applyConversationStreamCacheSnapshotToDisplay(
        conversationId,
        parsed.streamCache,
        { ignoreActivationId: true },
      );
      if (snapshotBlocks.length > 0) {
        authoritativeBlocks = snapshotBlocks;
        if (options.streamBlocks) {
          options.streamBlocks.value = snapshotBlocks;
        }
      }
    }

    if (parsed.kind === "tool_status") {
      options.toolStatusText.value = parsed.message || "";
      options.toolStatusState.value =
        parsed.toolStatus === "running" || parsed.toolStatus === "done" || parsed.toolStatus === "failed"
          ? parsed.toolStatus : "";
    }

    if (isActivityProjectionEvent) {
      if (delta && options.reasoningStartedAtMs.value === 0) options.reasoningStartedAtMs.value = Date.now();
    }

    if (currentRound.phase === "streaming") {
      if (parsed.kind === "tool_status") {
        options.updateDraftText(
          currentRound.draftId,
          undefined,
          undefined,
          "",
          authoritativeBlocks,
          { preserveActivityProjection: true },
        );
      } else if (isActivityProjectionEvent) {
        options.syncStreamBlocksToDraft(currentRound.draftId, authoritativeBlocks);
        options.updateDraftText(currentRound.draftId, undefined, undefined, "", authoritativeBlocks);
      } else if (parsed.streamCache && shouldCorrectProjectionFromSnapshot) {
        options.syncStreamBlocksToDraft(currentRound.draftId, authoritativeBlocks);
        options.updateDraftText(currentRound.draftId, undefined, undefined, "", authoritativeBlocks);
      } else if (parsed.streamCache && parsed.kind !== "tool_status") {
        options.updateDraftText(
          currentRound.draftId,
          undefined,
          undefined,
          "",
          authoritativeBlocks,
          { preserveActivityProjection: true },
        );
      }
    }

    if (parsed.kind === "tool_status" || isActivityProjectionEvent || parsed.streamCache) {
      return;
    }

    options.enqueueStreamDelta(currentGen, delta);
    options.syncCurrentDisplayStateToConversationStreamCache();
  }

  return {
    handleStreamingEvent,
  };
}

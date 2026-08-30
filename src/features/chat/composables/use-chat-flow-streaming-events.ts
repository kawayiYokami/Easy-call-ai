import type { Ref } from "vue";
import { normalizeAssistantStreamBlocks } from "../../../utils/chat-message-semantics";
import {
  assistantEventHasVisibleProgress,
  readDeltaMessage,
  readContextUsageUpdatePayload,
  readRoundCompletedPayload,
  readRoundFailedPayload,
  type AssistantDeltaEvent,
  type ContextUsageUpdatePayload,
} from "./use-chat-flow-events";
import type { ConversationRuntimeStreamCacheSnapshot } from "./use-chat-flow-stream-cache";
import type { PendingTerminalEvent, RoundState } from "./use-chat-flow-types";

type UseChatFlowStreamingEventsOptions = {
  contextUsagePreview?: Ref<ContextUsageUpdatePayload | null>;
  reasoningStartedAtMs: Ref<number>;
  getRound: () => RoundState;
  promoteQueuedRoundToStreaming: (gen: number) => number;
  setPendingTerminalEvent: (event: PendingTerminalEvent | null) => void;
  clearConversationStreamCache: (conversationId?: string | null) => void;
  getConversationId?: () => string;
  getActiveActivationId: () => string;
  setActiveActivationId: (value: string) => void;
  applyConversationStreamCacheSnapshotToDisplay: (
    conversationId: string,
    snapshot: ConversationRuntimeStreamCacheSnapshot,
  ) => boolean;
  handleRoundCompleted: (
    gen: number,
    result: {
      assistantText: string;
      assistantMessage?: any;
      activationId?: string;
      requestId?: string;
    },
  ) => Promise<void>;
  handleRoundFailed: (
    gen: number,
    error: unknown,
    identity?: { activationId?: string; requestId?: string },
  ) => Promise<void>;
  applyAssistantEventToMessage: (messageId: string, parsed: AssistantDeltaEvent) => void;
  enqueueStreamDelta: (gen: number, delta: string) => void;
};

export function streamingTerminalTargetsRound(
  round: RoundState,
  activeActivationId: string,
  input: { activationId?: string; requestId?: string; assistantMessageId?: string },
): boolean {
  if (round.phase !== "queued" && round.phase !== "streaming") return false;
  const incomingMessageId = String(input.assistantMessageId || "").trim();
  if (incomingMessageId && incomingMessageId !== round.messageId) return false;
  const currentActivationId = String(activeActivationId || "").trim();
  const incomingIds = [String(input.activationId || "").trim(), String(input.requestId || "").trim()]
    .filter(Boolean);
  if (currentActivationId && incomingIds.length > 0 && !incomingIds.includes(currentActivationId)) return false;
  return true;
}

export function useChatFlowStreamingEvents(options: UseChatFlowStreamingEventsOptions) {
  function applyStreamCacheContextUsageToPreview(parsed: AssistantDeltaEvent) {
    const cache = parsed.streamCache;
    if (!cache || typeof cache.contextUsageRatio !== "number") return;
    if (!options.contextUsagePreview) return;
    const conversationId = options.getConversationId ? options.getConversationId() : "";
    if (!conversationId) return;
    const ratio = Math.max(0, cache.contextUsageRatio);
    if (!(ratio > 0) && !(Number(cache.contextUsagePercent) > 0)) return;
    const percent = typeof cache.contextUsagePercent === "number"
      ? Math.round(cache.contextUsagePercent)
      : Math.round(ratio * 100);
    options.contextUsagePreview.value = {
      conversationId,
      contextUsagePercent: Math.min(100, Math.max(0, percent)),
      contextUsageRatio: ratio,
      effectivePromptTokens: Math.max(0, Math.round(Number(cache.effectivePromptTokens) || 0)),
      contextWindowTokens: Math.max(0, Math.round(Number(cache.contextWindowTokens) || 0)),
      source: "stream_cache",
      eventReason: "provider_tool_round",
    };
  }

  // ==================== 流式正文节流 ====================
  // 单一有序日志：LLM 本体是线性流，但曾分 text/reasoning 双队列攒批，flush 时
  // 先刷 reasoning 再刷 text，导致 100ms 窗内 textA->reasoning->textB 被重排为
  // reasoning->textA+textB，正文被“吸”到思维链后。现改为单一有序队列按到达
  // seq 保序 flush，渲染顺序恒等于到达顺序。
  const STREAM_TEXT_FLUSH_INTERVAL_MS = 100;
  type PendingOrderedItem = { kind: "text" | "reasoning"; delta: string };
  let pendingOrdered: PendingOrderedItem[] = [];
  let pendingStreamGen = 0;
  let pendingStreamMessageId = "";
  let streamTextFlushTimer: ReturnType<typeof setTimeout> | null = null;

  function flushStreamTextBuffer() {
    if (streamTextFlushTimer) {
      clearTimeout(streamTextFlushTimer);
      streamTextFlushTimer = null;
    }
    const gen = pendingStreamGen;
    const messageId = pendingStreamMessageId;
    const ordered = pendingOrdered;
    pendingOrdered = [];
    pendingStreamGen = 0;
    pendingStreamMessageId = "";
    for (const item of ordered) {
      if (!item.delta) continue;
      if (item.kind === "reasoning" && messageId) {
        options.applyAssistantEventToMessage(messageId, { kind: "activity_reasoning_delta", delta: item.delta });
      } else if (item.kind === "text" && gen) {
        options.enqueueStreamDelta(gen, item.delta);
      }
    }
  }

  function scheduleStreamTextFlush() {
    if (streamTextFlushTimer) return;
    streamTextFlushTimer = setTimeout(() => {
      streamTextFlushTimer = null;
      flushStreamTextBuffer();
    }, STREAM_TEXT_FLUSH_INTERVAL_MS);
  }

  function bufferStreamText(input: { gen: number; messageId: string; text?: string; reasoning?: string }) {
    const genMismatched = !!pendingStreamGen && pendingStreamGen !== input.gen;
    const idMismatched = !!pendingStreamMessageId && pendingStreamMessageId !== input.messageId;
    if ((genMismatched || idMismatched) && pendingOrdered.length > 0) {
      flushStreamTextBuffer();
    }
    if (input.reasoning) {
      const last = pendingOrdered[pendingOrdered.length - 1];
      if (last?.kind === "reasoning") last.delta += input.reasoning;
      else pendingOrdered.push({ kind: "reasoning", delta: input.reasoning });
    }
    if (input.text) {
      const last = pendingOrdered[pendingOrdered.length - 1];
      if (last?.kind === "text") last.delta += input.text;
      else pendingOrdered.push({ kind: "text", delta: input.text });
    }
    if (!pendingStreamGen) pendingStreamGen = input.gen;
    if (!pendingStreamMessageId) pendingStreamMessageId = input.messageId;
    if (pendingOrdered.length > 0) scheduleStreamTextFlush();
  }

  function handleStreamingEvent(currentGen: number, parsed: AssistantDeltaEvent) {
    if (parsed.kind === "round_completed" || parsed.kind === "round_failed") {
      // 终态事件到达时先冲刷文本缓冲，避免最后一段正文/思维链丢失。
      flushStreamTextBuffer();
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
    // 工具执行期间用量随流式缓存下发：直接更新预览，无需旁路广播。
    applyStreamCacheContextUsageToPreview(parsed);
    const round = options.getRound();
    if (
      !currentGen
      || (round.phase !== "queued" && round.phase !== "streaming")
      || round.gen !== currentGen
    ) {
      return;
    }
    if (parsed.kind === "tool_status") {
      // 工具/重试状态本身就是促使 waiting -> streaming 的可见进度。
      // 状态写入消息由 applyAssistantEventToMessage 完成（状态机 tool_status 分支
      // 写 _toolStatusText/_toolStatusState），投影初始化会保留消息已有值，无需 refs。
    }
    if (round.phase === "queued" && round.gen === currentGen && assistantEventHasVisibleProgress(parsed)) {
      options.promoteQueuedRoundToStreaming(currentGen);
    }
    const currentRound = options.getRound();
    if (currentRound.phase !== "streaming" && currentRound.phase !== "queued") {
      return;
    }
    if (currentRound.gen !== currentGen) {
      return;
    }
    if (parsed.kind === "round_completed") {
      const p = readRoundCompletedPayload(parsed.message);
      const identity = {
        activationId: p?.activationId || parsed.activationId,
        requestId: p?.requestId || parsed.requestId,
        assistantMessageId: p?.assistantMessage?.id,
      };
      if (!streamingTerminalTargetsRound(
        currentRound,
        options.getActiveActivationId(),
        identity,
      )) return;
      const result = {
        assistantText: String(p?.assistantText || ""),
        assistantMessage: p?.assistantMessage,
        activationId: identity.activationId,
        requestId: identity.requestId,
        ...(currentRound.phase === "queued"
          && parsed.reason === "context_compaction_boundary"
          && !String(p?.assistantText || "").trim()
          && !p?.assistantMessage
          ? { skipCanonicalReadback: true }
          : {}),
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
      const p = readRoundFailedPayload(parsed.message);
      const identity = {
        activationId: p?.activationId || parsed.activationId,
        requestId: p?.requestId || parsed.requestId,
      };
      if (!streamingTerminalTargetsRound(
        currentRound,
        options.getActiveActivationId(),
        identity,
      )) return;
      if (options.contextUsagePreview) {
        options.contextUsagePreview.value = null;
      }
      const error = p?.error || parsed.message || JSON.stringify(parsed);
      if (currentRound.phase === "queued") {
        options.setPendingTerminalEvent({
          kind: "failed",
          gen: currentGen,
          error,
          activationId: identity.activationId,
          requestId: identity.requestId,
        });
        options.clearConversationStreamCache(options.getConversationId ? options.getConversationId() : "");
        options.setActiveActivationId("");
        return;
      }
      void options.handleRoundFailed(currentGen, error, {
        activationId: identity.activationId,
        requestId: identity.requestId,
      });
      return;
    }

    const conversationId = options.getConversationId ? options.getConversationId() : "";
    const delta = readDeltaMessage(parsed);
    const isActivityProjectionEvent =
      parsed.kind === "activity_reasoning_delta"
      || parsed.kind === "assistant_tool_event"
      || parsed.kind === "assistant_tool_result";
    let receivedCanonicalSnapshot = false;
    if (conversationId && parsed.streamCache) {
      const streamCacheMessageId = String(parsed.streamCache.persistedAssistantMessageId || "").trim();
      if (streamCacheMessageId && currentRound.messageId && streamCacheMessageId !== currentRound.messageId) {
        return;
      }
      const snapshotBlocks = normalizeAssistantStreamBlocks(parsed.streamCache.streamBlocks);
      const snapshotHasVisibleProgress = !!(
        String(parsed.streamCache.assistantText || "").trim()
        || String(parsed.streamCache.toolStatusText || "").trim()
        || String(parsed.streamCache.toolStatusState || "").trim()
        || snapshotBlocks.length > 0
      );
      if (currentRound.phase === "streaming" && snapshotHasVisibleProgress) {
        // 权威快照到达前先冲刷缓冲，避免旧增量文本追加到快照状态之后造成错乱。
        flushStreamTextBuffer();
        options.applyConversationStreamCacheSnapshotToDisplay(conversationId, parsed.streamCache);
        options.applyAssistantEventToMessage(currentRound.messageId, parsed);
        receivedCanonicalSnapshot = true;
      }
    }

    if (parsed.kind === "tool_status") {
      // 工具状态即时处理（先冲刷缓冲保持事件顺序），不参与正文节流。
      flushStreamTextBuffer();
      if (currentRound.phase === "streaming" && !receivedCanonicalSnapshot) {
        options.applyAssistantEventToMessage(currentRound.messageId, parsed);
      }
    }

    if (isActivityProjectionEvent) {
      if (delta && options.reasoningStartedAtMs.value === 0) options.reasoningStartedAtMs.value = Date.now();
      if (parsed.kind === "activity_reasoning_delta" && delta) {
        // 思维链文本与正文一起 100ms 节流。
        if (currentRound.phase === "streaming" && !receivedCanonicalSnapshot) {
          bufferStreamText({ gen: currentGen, messageId: currentRound.messageId, reasoning: delta });
        }
      } else {
        // 工具事件即时处理（先冲刷缓冲保持顺序）。
        flushStreamTextBuffer();
        if (currentRound.phase === "streaming" && !receivedCanonicalSnapshot) {
          options.applyAssistantEventToMessage(currentRound.messageId, parsed);
        }
      }
    }

    if (parsed.kind === "tool_status" || isActivityProjectionEvent || receivedCanonicalSnapshot) {
      return;
    }

    if (currentRound.phase === "streaming") {
      bufferStreamText({ gen: currentGen, messageId: currentRound.messageId, text: delta });
    }
  }

  return {
    handleStreamingEvent,
    flushStreamTextBuffer,
  };
}

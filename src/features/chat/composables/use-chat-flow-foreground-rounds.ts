import type { ChatMessage } from "../../../types/app";
import { DRAFT_ASSISTANT_ID_PREFIX } from "./use-chat-flow-drafts";
import { streamCacheHasVisibleProgress } from "./use-chat-flow-stream-cache";
import { applyStreamingHistoryOverlay } from "./use-chat-flow-stream-overlay";
import { formalizeMessages, normalizeConversationId, positiveRoundedNumber, readMessagePlainText } from "./use-chat-flow-utils";
import type { ResumeForegroundRuntimeRoundInput } from "./use-chat-flow-types";
import type { RoundStartedPayload } from "./use-chat-flow-events";

type ResumeForegroundStreamCacheProjectionInput = {
  conversationId?: string | null;
  reason?: string;
};

export function useChatFlowForegroundRounds(bindings: Record<string, any>) {
  function applyStreamingOverlayForConversation(conversationId?: string | null) {
    const cid = normalizeConversationId(conversationId);
    if (!cid) return;
    const overlay = applyStreamingHistoryOverlay(
      bindings.allMessages.value,
      bindings.readConversationStreamCache(cid),
    );
    if (!overlay.removed) return;
    bindings.allMessages.value = overlay.messages;
    console.info("[聊天流式恢复] 应用流式覆盖投影，隐藏已持久化半成品消息", {
      conversationId: cid,
      replacedMessageId: overlay.replacedMessageId,
      afterLen: overlay.messages.length,
    });
  }

  function applyQueuedStreamingStateIfNeeded(draftId: string) {
    const queuedStreamingState = bindings.getQueuedStreamingState();
    if (!queuedStreamingState) return;
    bindings.latestAssistantText.value = queuedStreamingState.assistantText;
    bindings.latestReasoningStandardText.value = queuedStreamingState.reasoningStandard;
    bindings.latestReasoningInlineText.value = queuedStreamingState.reasoningInline;
    bindings.toolStatusText.value = queuedStreamingState.toolStatusText;
    bindings.toolStatusState.value = queuedStreamingState.toolStatusState;
    if (bindings.streamToolCalls) {
      bindings.streamToolCalls.value = queuedStreamingState.streamToolCalls;
    }
    bindings.setStreamToolCallCount(queuedStreamingState.streamToolCallCount);
    bindings.setStreamLastToolName(queuedStreamingState.streamLastToolName);
    if (queuedStreamingState.frontendDispatchStartedAtMs || queuedStreamingState.frontendDispatchElapsedMs) {
      const round = bindings.getRound();
      bindings.startFrontendDispatchTimer(
        round.phase === "queued" || round.phase === "streaming" ? round.gen : 0,
        queuedStreamingState.frontendDispatchStartedAtMs,
        queuedStreamingState.frontendDispatchElapsedMs,
      );
    }
    bindings.setQueuedStreamingState(null);
    bindings.updateDraftText(draftId);
  }

  function beginAssistantActivationFromEvent(payload: RoundStartedPayload): number {
    const payloadConversationId = normalizeConversationId(payload.conversationId);
    const currentConversationId = normalizeConversationId(bindings.getConversationId ? bindings.getConversationId() : "");
    if (currentConversationId && payloadConversationId && currentConversationId !== payloadConversationId) {
      return 0;
    }
    const nextActivationId = String(payload.activationId || payload.requestId || "").trim();
    const cid = payloadConversationId || currentConversationId;
    const round = bindings.getRound();
    if (bindings.getActiveActivationId() && nextActivationId && bindings.getActiveActivationId() === nextActivationId && round.phase !== "idle") {
      return round.gen;
    }
    if (cid) bindings.clearConversationStreamCache(cid);
    bindings.setActiveActivationId(nextActivationId);
    if (cid && positiveRoundedNumber(payload.startedAtMs)) {
      bindings.writeConversationStreamCacheSnapshot(cid, {
        activationId: nextActivationId,
        requestId: String(payload.requestId || nextActivationId || "").trim(),
        startedAt: String(payload.startedAt || "").trim(),
        startedAtMs: positiveRoundedNumber(payload.startedAtMs),
      });
    }
    let gen = round.phase === "queued" ? round.gen : bindings.getSendChatActiveGen();
    if (!gen) {
      gen = bindings.nextGeneration();
      bindings.channelBinding.setBoundDisplayGeneration(gen);
      bindings.setPendingTerminalEvent(null);
      bindings.setDeferredRoundCompletion(null);
      bindings.setQueuedStreamingState(null);
      bindings.removeAssistantDrafts();
      bindings.resetDisplayState();
      bindings.setActiveHistoryMessageCount(formalizeMessages(bindings.allMessages.value).length);
      bindings.setRound({ phase: "queued", gen }, "waiting");
    }
    bindings.startFrontendDispatchTimer(
      gen,
      positiveRoundedNumber(payload.startedAtMs) || bindings.sendStartedAtMsByGen.get(gen),
    );
    bindings.chatting.value = true;
    bindings.updateQueuedAssistantDraftStatus(`${DRAFT_ASSISTANT_ID_PREFIX}${gen}`, bindings.t("chat.statusWaitingReply"));
    bindings.setFrontendRoundPhase("waiting");
    return gen;
  }

  function cachedDispatchTimerForConversation(): { startedAtMs: number; elapsedMs: number } {
    const conversationId = normalizeConversationId(bindings.getConversationId ? bindings.getConversationId() : "");
    const cache = conversationId ? bindings.readConversationStreamCache(conversationId) : null;
    return {
      startedAtMs: positiveRoundedNumber(cache?.frontendDispatchStartedAtMs || cache?.startedAtMs),
      elapsedMs: positiveRoundedNumber(cache?.frontendDispatchElapsedMs),
    };
  }

  function ensureForegroundWaitingRound(statusText = bindings.t("chat.statusWaitingReply")) {
    const round = bindings.getRound();
    const cachedTimer = cachedDispatchTimerForConversation();
    if (round.phase === "queued") {
      bindings.startFrontendDispatchTimer(
        round.gen,
        bindings.frontendDispatch.getStartedAtMs() || cachedTimer.startedAtMs || undefined,
        bindings.frontendDispatch.getElapsedMs() || cachedTimer.elapsedMs,
      );
      bindings.updateQueuedAssistantDraftStatus(`${DRAFT_ASSISTANT_ID_PREFIX}${round.gen}`, statusText);
      bindings.chatting.value = true;
      bindings.setFrontendRoundPhase("waiting");
      return round.gen;
    }
    if (round.phase === "streaming") {
      bindings.startFrontendDispatchTimer(
        round.gen,
        bindings.frontendDispatch.getStartedAtMs() || cachedTimer.startedAtMs || undefined,
        bindings.frontendDispatch.getElapsedMs() || cachedTimer.elapsedMs,
      );
      if (!bindings.hasAssistantDraftInMessages()) {
        const draftId = bindings.insertDraft(round.gen, statusText);
        bindings.updateDraftText(draftId);
        bindings.setRound({ phase: "streaming", gen: round.gen, draftId });
      }
      bindings.chatting.value = true;
      return round.gen;
    }
    const gen = bindings.nextGeneration();
    bindings.channelBinding.setBoundDisplayGeneration(gen);
    bindings.setPendingTerminalEvent(null);
    bindings.setDeferredRoundCompletion(null);
    bindings.setQueuedStreamingState(null);
    bindings.setActiveHistoryMessageCount(formalizeMessages(bindings.allMessages.value).length);
    bindings.setRound({ phase: "queued", gen }, "waiting");
    bindings.startFrontendDispatchTimer(gen, cachedTimer.startedAtMs || undefined, cachedTimer.elapsedMs);
    bindings.chatting.value = true;
    bindings.updateQueuedAssistantDraftStatus(`${DRAFT_ASSISTANT_ID_PREFIX}${gen}`, statusText);
    return gen;
  }

  function ensureForegroundStreamingRound() {
    const conversationId = bindings.getConversationId ? bindings.getConversationId() : "";
    const round = bindings.getRound();
    if (round.phase === "streaming") {
      if (!bindings.hasAssistantDraftInMessages()) {
        if (bindings.streamToolCalls) bindings.streamToolCalls.value = [];
        bindings.applyConversationStreamCacheToDisplay(conversationId);
        const draftId = bindings.insertDraft(round.gen);
        bindings.updateDraftText(draftId);
        bindings.setRound({ phase: "streaming", gen: round.gen, draftId });
      } else {
        bindings.loadStreamToolCallsFromDraft(round.draftId);
      }
      return round.gen;
    }
    const gen = bindings.nextGeneration();
    if (bindings.streamToolCalls) bindings.streamToolCalls.value = [];
    const existingDraft = [...bindings.allMessages.value]
      .reverse()
      .find((message: ChatMessage) => String(message?.id || "").trim().startsWith(DRAFT_ASSISTANT_ID_PREFIX));
    const existingDraftId = String(existingDraft?.id || "").trim();
    const existingDraftMeta = ((existingDraft?.providerMeta || {}) as Record<string, unknown>);
    const restoredFromCache = !existingDraftId && bindings.applyConversationStreamCacheToDisplay(conversationId);
    applyStreamingOverlayForConversation(conversationId);
    const existingDraftStartedAtMs = existingDraftId ? positiveRoundedNumber(existingDraftMeta._frontendDispatchStartedAtMs) : 0;
    const existingDraftElapsedMs = existingDraftId ? positiveRoundedNumber(existingDraftMeta._frontendDispatchElapsedMs) : 0;
    console.info("[聊天流式阶段] 前台恢复流式投影", {
      conversationId,
      restoredFromCache,
      existingDraftId,
      assistantTextLength: String(bindings.latestAssistantText.value || "").length,
      roundPhase: round.phase,
    });
    if (!restoredFromCache) {
      bindings.latestAssistantText.value = readMessagePlainText(existingDraft);
      bindings.latestReasoningStandardText.value = String(existingDraftMeta.reasoningStandard || "");
      bindings.latestReasoningInlineText.value = String(existingDraftMeta.reasoningInline || "");
      bindings.setPendingReasoningStandardBreak(false);
    }
    bindings.setActiveHistoryMessageCount(formalizeMessages(bindings.allMessages.value).length);
    const draftId = existingDraftId || bindings.insertDraft(gen);
    if (existingDraftId) {
      bindings.loadStreamToolCallsFromDraft(draftId);
    }
    if (existingDraftId || restoredFromCache) {
      bindings.updateDraftText(draftId);
    }
    bindings.setRound({ phase: "streaming", gen, draftId });
    bindings.startFrontendDispatchTimer(
      gen,
      existingDraftStartedAtMs || bindings.frontendDispatch.getStartedAtMs() || undefined,
      existingDraftElapsedMs || bindings.frontendDispatch.getElapsedMs(),
    );
    bindings.chatting.value = true;
    applyQueuedStreamingStateIfNeeded(draftId);
    return gen;
  }

  function resumeForegroundRuntimeRound(input?: ResumeForegroundRuntimeRoundInput) {
    const conversationId = normalizeConversationId(input?.conversationId || (bindings.getConversationId ? bindings.getConversationId() : ""));
    if (!conversationId) return 0;
    if (input?.streamCache) {
      bindings.writeConversationStreamCacheSnapshot(conversationId, input.streamCache);
    }
    const cache = bindings.readConversationStreamCache(conversationId);
    applyStreamingOverlayForConversation(conversationId);
    const hasVisibleProgress =
      !!input?.streamCache?.hasVisibleProgress
      || streamCacheHasVisibleProgress(input?.streamCache)
      || streamCacheHasVisibleProgress(cache);
    console.info("[聊天流式恢复] 应用后端运行态快照", {
      conversationId,
      reason: String(input?.reason || ""),
      hasVisibleProgress,
      assistantTextLength: String(cache?.assistantText || input?.streamCache?.assistantText || "").length,
      reasoningStandardLength: String(cache?.reasoningStandard || input?.streamCache?.reasoningStandard || "").length,
      reasoningInlineLength: String(cache?.reasoningInline || input?.streamCache?.reasoningInline || "").length,
      toolCallCount: Math.max(
        Number(cache?.streamToolCallCount || 0),
        Number(input?.streamCache?.streamToolCallCount || 0),
      ),
    });
    return hasVisibleProgress
      ? ensureForegroundStreamingRound()
      : ensureForegroundWaitingRound(input?.statusText || bindings.t("chat.statusWaitingReply"));
  }

  function resumeForegroundStreamCacheProjection(input?: ResumeForegroundStreamCacheProjectionInput) {
    const currentConversationId = normalizeConversationId(bindings.getConversationId ? bindings.getConversationId() : "");
    const conversationId = normalizeConversationId(input?.conversationId || currentConversationId);
    if (!conversationId || conversationId !== currentConversationId) return 0;
    const cache = bindings.readConversationStreamCache(conversationId);
    if (!streamCacheHasVisibleProgress(cache)) return 0;
    console.info("[聊天流式恢复] 从前端缓存恢复当前会话投影", {
      conversationId,
      reason: String(input?.reason || ""),
      assistantTextLength: String(cache?.assistantText || "").length,
      reasoningStandardLength: String(cache?.reasoningStandard || "").length,
      reasoningInlineLength: String(cache?.reasoningInline || "").length,
      toolCallCount: Number(cache?.streamToolCallCount || 0),
    });
    return ensureForegroundStreamingRound();
  }

  function promoteQueuedRoundToStreaming(gen: number) {
    const round = bindings.getRound();
    if (round.phase === "streaming" && round.gen === gen) {
      return gen;
    }
    if (round.phase !== "queued" || round.gen !== gen) {
      return 0;
    }
    const conversationId = bindings.getConversationId ? bindings.getConversationId() : "";
    if (bindings.streamToolCalls) bindings.streamToolCalls.value = [];
    const existingDraft = [...bindings.allMessages.value]
      .reverse()
      .find((message: ChatMessage) => String(message?.id || "").trim().startsWith(DRAFT_ASSISTANT_ID_PREFIX));
    const existingDraftId = String(existingDraft?.id || "").trim();
    const existingDraftMeta = ((existingDraft?.providerMeta || {}) as Record<string, unknown>);
    const restoredFromCache = !existingDraftId && bindings.applyConversationStreamCacheToDisplay(conversationId);
    applyStreamingOverlayForConversation(conversationId);
    const existingDraftStartedAtMs = existingDraftId ? positiveRoundedNumber(existingDraftMeta._frontendDispatchStartedAtMs) : 0;
    const existingDraftElapsedMs = existingDraftId ? positiveRoundedNumber(existingDraftMeta._frontendDispatchElapsedMs) : 0;
    if (!restoredFromCache) {
      bindings.latestAssistantText.value = readMessagePlainText(existingDraft);
      bindings.latestReasoningStandardText.value = String(existingDraftMeta.reasoningStandard || "");
      bindings.latestReasoningInlineText.value = String(existingDraftMeta.reasoningInline || "");
      bindings.setPendingReasoningStandardBreak(false);
    }
    bindings.setActiveHistoryMessageCount(formalizeMessages(bindings.allMessages.value).length);
    const draftId = existingDraftId || bindings.insertDraft(gen);
    if (existingDraftId) {
      bindings.loadStreamToolCallsFromDraft(draftId);
    }
    if (existingDraftId || restoredFromCache) {
      bindings.updateDraftText(draftId);
    }
    bindings.setRound({ phase: "streaming", gen, draftId });
    bindings.startFrontendDispatchTimer(
      gen,
      existingDraftStartedAtMs || bindings.frontendDispatch.getStartedAtMs() || undefined,
      existingDraftElapsedMs || bindings.frontendDispatch.getElapsedMs(),
    );
    bindings.chatting.value = true;
    applyQueuedStreamingStateIfNeeded(draftId);
    bindings.applyPendingTerminalEvent(gen);
    return gen;
  }

  return {
    beginAssistantActivationFromEvent,
    ensureForegroundWaitingRound,
    ensureForegroundStreamingRound,
    resumeForegroundRuntimeRound,
    resumeForegroundStreamCacheProjection,
    promoteQueuedRoundToStreaming,
  };
}

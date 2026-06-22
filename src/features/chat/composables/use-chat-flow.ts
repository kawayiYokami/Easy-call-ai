import { Channel } from "@tauri-apps/api/core";
import { ref } from "vue";
import type { AssistantStreamBlock, ChatMessage } from "../../../types/app";
import { normalizeAssistantStreamBlocks } from "../../../utils/chat-message-semantics";
import { useChatFlowChannelBinding } from "./use-chat-flow-channel-binding";
import {
  DRAFT_ASSISTANT_ID_PREFIX,
  useChatFlowDrafts,
} from "./use-chat-flow-drafts";
import { useChatFlowExternalEvents } from "./use-chat-flow-external-events";
import { useChatFlowFrontendDispatch } from "./use-chat-flow-frontend-dispatch";
import { useChatFlowSendInput } from "./use-chat-flow-send-input";
import { useChatFlowSendController } from "./use-chat-flow-send-controller";
import {
  readDeltaMessage,
  readHistoryFlushedPayload,
  type AssistantDeltaEvent,
  type HistoryFlushedPayload,
  type RoundCompletedPayload,
  type RoundFailedPayload,
  type RoundStartedPayload,
} from "./use-chat-flow-events";
import { useChatFlowSendPayloads } from "./use-chat-flow-send-payloads";
import {
  useChatFlowStreamCache,
  type ConversationStreamCache,
} from "./use-chat-flow-stream-cache";
import { useChatFlowSendRecovery } from "./use-chat-flow-send-recovery";
import { useChatFlowStop } from "./use-chat-flow-stop";
import { useChatFlowStreamingEvents } from "./use-chat-flow-streaming-events";
import { useChatFlowRoundEvents } from "./use-chat-flow-round-events";
import { useChatFlowForegroundReset } from "./use-chat-flow-foreground-reset";
import { useChatFlowRoundFinalizers } from "./use-chat-flow-round-finalizers";
import { useChatFlowForegroundRounds } from "./use-chat-flow-foreground-rounds";
import {
  normalizeConversationId,
} from "./use-chat-flow-utils";
import type {
  DeferredRoundCompletion,
  FrontendRoundPhase,
  PendingTerminalEvent,
  ResumeForegroundRuntimeRoundInput,
  RoundState,
  SendChatOverrides,
  UseChatFlowOptions,
} from "./use-chat-flow-types";

const CHAT_STREAM_DEBUG = typeof window !== "undefined"
  && window.localStorage.getItem("easy-call.debug.chat-stream") === "1";

export type { ConversationRuntimeStreamCacheSnapshot } from "./use-chat-flow-stream-cache";
export type { AssistantDeltaEvent } from "./use-chat-flow-events";
export type { FrontendRoundPhase, ResumeForegroundRuntimeRoundInput } from "./use-chat-flow-types";

export function useChatFlow(options: UseChatFlowOptions) {
  // ── 状态 ──
  let round: RoundState = { phase: "idle" };
  const frontendRoundPhase = ref<FrontendRoundPhase>("idle");
  const submitPending = options.submitPending ?? ref(false);
  let generation = 0;
  let sendChatActiveGen = 0; // 防止 bound channel 抢占 sendChat 轮次
  let historyFlushedReceivedGen = 0; // 记录 sendChat 轮次是否已收到 history_flushed，避免 finally 误回收
  let pendingTerminalEvent: PendingTerminalEvent | null = null;
  let deferredRoundCompletion: DeferredRoundCompletion | null = null;
  let foregroundRounds: ReturnType<typeof useChatFlowForegroundRounds> | null = null;
  let activeActivationId = "";
  let activeRoundAgentId = "";
  let queuedStreamingState: {
    assistantText: string;
    toolStatusText: string;
    toolStatusState: "running" | "done" | "failed" | "";
    streamBlocks: AssistantStreamBlock[];
    frontendDispatchStartedAtMs: number;
    frontendDispatchElapsedMs: number;
  } | null = null;
  const sendStartedAtMsByGen = new Map<number, number>();
  let activeHistoryMessageCount = 0;
  const {
    buildQueuedAttachmentPayload,
    buildImageAttachmentPayload,
  } = useChatFlowSendPayloads({
    queuedAttachmentNotices: options.queuedAttachmentNotices,
  });
  const sendInput = useChatFlowSendInput({
    chatInput: options.chatInput,
    clipboardImages: options.clipboardImages,
    queuedAttachmentNotices: options.queuedAttachmentNotices,
    selectedMentions: options.selectedMentions,
    latestUserText: options.latestUserText,
    latestUserImages: options.latestUserImages,
    getSession: options.getSession,
    getConversationId: options.getConversationId,
    buildQueuedAttachmentPayload,
    buildImageAttachmentPayload,
  });
  const frontendDispatch = useChatFlowFrontendDispatch({
    allMessages: options.allMessages,
    getDraftIdForGen: (gen) => {
      if (round.phase === "streaming" && round.gen === gen) return round.draftId;
      return `${DRAFT_ASSISTANT_ID_PREFIX}${gen}`;
    },
    isRoundActiveForGen: (gen) => (
      (round.phase === "queued" || round.phase === "streaming")
      && round.gen === gen
    ),
    syncCurrentDisplayStateToConversationStreamCache: () => {
      syncCurrentDisplayStateToConversationStreamCache();
    },
  });
  const {
    applyAssistantDeltaToDraft,
    finalizeDraft,
    getDraftStreamBlocks,
    getPendingUserDraftId,
    hasAssistantDraftInMessages,
    insertDraft,
    insertUserDraft,
    loadStreamBlocksFromDraft,
    removeAssistantDrafts,
    removeDraft,
    syncStreamBlocksToDraft,
    updateDraftText,
    updateQueuedAssistantDraftStatus,
  } = useChatFlowDrafts({
    allMessages: options.allMessages,
    latestUserText: options.latestUserText,
    latestAssistantText: options.latestAssistantText,
    toolStatusText: options.toolStatusText,
    toolStatusState: options.toolStatusState,
    streamBlocks: options.streamBlocks,
    getActiveRoundAgentId: () => activeRoundAgentId,
    getConversationId: options.getConversationId,
    buildImageAttachmentPayload,
    getSendStartedAtMs: (gen) => sendStartedAtMsByGen.get(gen) || 0,
    getActiveHistoryMessageCount: () => activeHistoryMessageCount,
    getFrontendDispatchStartedAtMs: frontendDispatch.getStartedAtMs,
    currentFrontendDispatchElapsedMs: frontendDispatch.currentElapsedMs,
  });
  const {
    applyAssistantEventToConversationStreamCache,
    applyConversationStreamCacheSnapshotToDisplay,
    applyConversationStreamCacheToDisplay,
    clearConversationStreamCache,
    readConversationStreamCache,
    syncCurrentDisplayStateToConversationStreamCache,
    writeConversationStreamCacheSnapshot,
  } = useChatFlowStreamCache({
    getConversationId: options.getConversationId,
    latestAssistantText: options.latestAssistantText,
    toolStatusText: options.toolStatusText,
    toolStatusState: options.toolStatusState,
    streamBlocks: options.streamBlocks,
    getActiveActivationId: () => activeActivationId,
    getFrontendDispatchStartedAtMs: frontendDispatch.getStartedAtMs,
    getFrontendDispatchElapsedMs: frontendDispatch.getElapsedMs,
    currentFrontendDispatchElapsedMs: frontendDispatch.currentElapsedMs,
    restoreFrontendDispatchTimerFromCache,
    setActiveRoundAgentId: (value: string) => {
      activeRoundAgentId = String(value || "").trim();
    },
  });
  const reasoningStartedAtMs = ref(0);
  const roundFinalizers = useChatFlowRoundFinalizers({
    getRound: () => round,
    setRound,
    getDeferredRoundCompletion: () => deferredRoundCompletion,
    setDeferredRoundCompletion: (value: DeferredRoundCompletion | null) => {
      deferredRoundCompletion = value;
    },
    latestAssistantText: options.latestAssistantText,
    toolStatusText: options.toolStatusText,
    toolStatusState: options.toolStatusState,
    streamBlocks: options.streamBlocks,
    chatting: options.chatting,
    reasoningStartedAtMs,
    t: options.t,
    clearChatErrorText,
    updateDraftText,
    finalizeDraft,
    clearConversationStreamCache,
    clearFrontendDispatchTimer,
    setActiveActivationId: (value: string) => {
      activeActivationId = value;
    },
    setActiveRoundAgentId: (value: string) => {
      activeRoundAgentId = String(value || "").trim();
    },
    onReloadMessages: options.onReloadMessages,
    removeDraft,
    setPendingTerminalEvent: (event: PendingTerminalEvent | null) => {
      pendingTerminalEvent = event;
    },
    setQueuedStreamingState: (value: typeof queuedStreamingState) => {
      queuedStreamingState = value
        ? { ...value, streamBlocks: normalizeAssistantStreamBlocks(value.streamBlocks) }
        : null;
    },
    sendStartedAtMsByGen,
    getPendingUserDraftId,
    formatRequestFailed: options.formatRequestFailed,
    setChatErrorText,
    applyAssistantDeltaToDraft,
    submitPending,
  });
  const streamingEvents = useChatFlowStreamingEvents({
    toolStatusText: options.toolStatusText,
    toolStatusState: options.toolStatusState,
    streamBlocks: options.streamBlocks,
    contextUsagePreview: options.contextUsagePreview,
    reasoningStartedAtMs,
    getRound: () => round,
    promoteQueuedRoundToStreaming,
    setPendingTerminalEvent: (event) => {
      pendingTerminalEvent = event;
    },
    clearConversationStreamCache,
    getConversationId: options.getConversationId,
    setActiveActivationId: (value) => {
      activeActivationId = value;
    },
    handleRoundCompleted,
    handleRoundFailed,
    getDraftStreamBlocks,
    syncStreamBlocksToDraft,
    syncCurrentDisplayStateToConversationStreamCache,
    applyConversationStreamCacheSnapshotToDisplay,
    updateDraftText,
    enqueueStreamDelta: roundFinalizers.enqueueStreamDelta,
  });
  const channelBinding = useChatFlowChannelBinding({
    debug: CHAT_STREAM_DEBUG,
    getConversationId: options.getConversationId,
    invokeBindActiveChatViewStream: options.invokeBindActiveChatViewStream,
    getRoundActiveGen: () => (
      round.phase === "queued" || round.phase === "streaming" ? round.gen : 0
    ),
    getCurrentGeneration: () => generation,
    markHistoryFlushedReceived: (gen) => {
      historyFlushedReceivedGen = Math.max(historyFlushedReceivedGen, gen);
    },
    handleHistoryFlushed,
    handleStreamingEvent,
    formatRequestFailed: options.formatRequestFailed,
    setChatErrorText,
  });
  foregroundRounds = useChatFlowForegroundRounds({
    getRound: () => round,
    setRound,
    frontendDispatch,
    setFrontendRoundPhase: (value: FrontendRoundPhase) => {
      frontendRoundPhase.value = value;
    },
    nextGeneration: () => ++generation,
    getSendChatActiveGen: () => sendChatActiveGen,
    getActiveActivationId: () => activeActivationId,
    setActiveActivationId: (value: string) => {
      activeActivationId = value;
    },
    setActiveRoundAgentId: (value: string) => {
      activeRoundAgentId = String(value || "").trim();
    },
    setPendingTerminalEvent: (event: PendingTerminalEvent | null) => {
      pendingTerminalEvent = event;
    },
    setDeferredRoundCompletion: (event: DeferredRoundCompletion | null) => {
      deferredRoundCompletion = event;
    },
    getQueuedStreamingState: () => queuedStreamingState,
    setQueuedStreamingState: (value: typeof queuedStreamingState) => {
      queuedStreamingState = value
        ? { ...value, streamBlocks: normalizeAssistantStreamBlocks(value.streamBlocks) }
        : null;
    },
    setActiveHistoryMessageCount: (value: number) => {
      activeHistoryMessageCount = value;
    },
    getConversationId: options.getConversationId,
    allMessages: options.allMessages,
    latestAssistantText: options.latestAssistantText,
    toolStatusText: options.toolStatusText,
    toolStatusState: options.toolStatusState,
    streamBlocks: options.streamBlocks,
    chatting: options.chatting,
    t: options.t,
    channelBinding,
    sendStartedAtMsByGen,
    clearConversationStreamCache,
    removeAssistantDrafts,
    resetDisplayState,
    startFrontendDispatchTimer,
    updateQueuedAssistantDraftStatus,
    hasAssistantDraftInMessages,
    insertDraft,
    syncStreamBlocksToDraft,
    updateDraftText,
    applyConversationStreamCacheToDisplay,
    loadStreamBlocksFromDraft,
    readConversationStreamCache,
    writeConversationStreamCacheSnapshot,
    applyPendingTerminalEvent,
  });
  const externalEvents = useChatFlowExternalEvents({
    debug: CHAT_STREAM_DEBUG,
    getCurrentConversationId: () => String(options.getConversationId ? options.getConversationId() : "").trim(),
    getActiveActivationId: () => activeActivationId,
    setActiveActivationId: (value) => {
      activeActivationId = value;
    },
    getRound: () => round,
    getSendChatActiveGen: () => sendChatActiveGen,
    nextGeneration: () => ++generation,
    channelBinding,
    handleHistoryFlushed,
    beginAssistantActivationFromEvent,
    markRoundStarted,
    payloadMatchesActiveActivation,
    handleRoundCompleted,
    handleRoundFailed,
    clearConversationStreamCache,
    clearFrontendDispatchTimer,
    onReloadMessages: options.onReloadMessages,
    onAssistantMessageCompleted: options.onAssistantMessageCompleted,
    setChatErrorText,
    formatRequestFailed: options.formatRequestFailed,
    latestAssistantText: options.latestAssistantText,
    chatting: options.chatting,
    reasoningStartedAtMs,
    applyAssistantEventToConversationStreamCache,
    writeConversationStreamCacheSnapshot,
    applyConversationStreamCacheToDisplay,
    hasAssistantDraftInMessages,
    ensureForegroundStreamingRound,
    handleStreamingEvent,
    syncStreamBlocksToDraft,
    updateDraftText,
  });
  const stopController = useChatFlowStop({
    chatting: options.chatting,
    latestAssistantText: options.latestAssistantText,
    toolStatusText: options.toolStatusText,
    toolStatusState: options.toolStatusState,
    streamBlocks: options.streamBlocks,
    allMessages: options.allMessages,
    getSession: options.getSession,
    getConversationId: options.getConversationId,
    invokeStopChatMessage: options.invokeStopChatMessage,
    onReloadMessages: options.onReloadMessages,
    t: options.t,
    getRound: () => round,
    setRound,
    advanceGeneration: () => {
      generation += 1;
    },
    setSendChatActiveGen: (gen) => {
      sendChatActiveGen = gen;
    },
    clearDeferredRoundCompletion: () => {
      deferredRoundCompletion = null;
    },
    clearPendingTerminalEvent: () => {
      pendingTerminalEvent = null;
    },
    setActiveActivationId: (value) => {
      activeActivationId = value;
    },
    setActiveRoundAgentId: (value: string) => {
      activeRoundAgentId = String(value || "").trim();
    },
    clearFrontendDispatchTimer,
    getPendingUserDraftId,
    removeDraft,
    deleteSendStartedAtMs: (gen) => {
      sendStartedAtMsByGen.delete(gen);
    },
    clearConversationStreamCache,
    reasoningStartedAtMs,
  });
  const sendRecovery = useChatFlowSendRecovery({
    chatting: options.chatting,
    submitPending,
    latestAssistantText: options.latestAssistantText,
    toolStatusText: options.toolStatusText,
    toolStatusState: options.toolStatusState,
    streamBlocks: options.streamBlocks,
    reasoningStartedAtMs,
    getRound: () => round,
    setRound,
    getSession: options.getSession,
    getHistoryFlushedReceivedGen: () => historyFlushedReceivedGen,
    setSendChatActiveGenIfCurrent: (gen, value) => {
      if (sendChatActiveGen === gen) sendChatActiveGen = value;
    },
    clearFrontendDispatchTimer,
    clearChatErrorText,
    setChatErrorText,
    formatRequestFailed: options.formatRequestFailed,
    getPendingUserDraftId,
    removeDraft,
    deleteSendStartedAtMs: (gen) => {
      sendStartedAtMsByGen.delete(gen);
    },
    failQueuedRoundWithoutDraft: roundFinalizers.failQueuedRoundWithoutDraft,
    setActiveRoundAgentId: (value: string) => {
      activeRoundAgentId = String(value || "").trim();
    },
    onReloadMessages: options.onReloadMessages,
    t: options.t,
  });
  const roundEvents = useChatFlowRoundEvents({
    chatting: options.chatting,
    latestAssistantText: options.latestAssistantText,
    toolStatusText: options.toolStatusText,
    toolStatusState: options.toolStatusState,
    streamBlocks: options.streamBlocks,
    reasoningStartedAtMs,
    getRound: () => round,
    setRound,
    getGeneration: () => generation,
    setPendingTerminalEvent: (event) => {
      pendingTerminalEvent = event;
    },
    getPendingTerminalEvent: () => pendingTerminalEvent,
    setDeferredRoundCompletion: (event) => {
      deferredRoundCompletion = event;
    },
    clearConversationStreamCache,
    clearFrontendDispatchTimer,
    setActiveActivationId: (value) => {
      activeActivationId = value;
    },
    setSendChatActiveGen: (value) => {
      sendChatActiveGen = value;
    },
    sendStartedAtMsByGen,
    hasAssistantDraftInMessages,
    applyConversationStreamCacheToDisplay,
    loadStreamBlocksFromDraft,
    updateQueuedAssistantDraftStatus,
    insertDraft,
    updateDraftText,
    syncStreamBlocksToDraft,
    applyPendingTerminalEvent,
    promoteQueuedRoundToStreaming,
    finalizeDeferredRoundCompletion: roundFinalizers.finalizeDeferredRoundCompletion,
    finalizeQueuedRoundWithoutDraft: roundFinalizers.finalizeQueuedRoundWithoutDraft,
    failQueuedRoundWithoutDraft: roundFinalizers.failQueuedRoundWithoutDraft,
    enqueueStreamDelta: roundFinalizers.enqueueStreamDelta,
    setChatErrorText,
    formatRequestFailed: options.formatRequestFailed,
    onReloadMessages: options.onReloadMessages,
    optionsT: options.t,
  });
  const sendController = useChatFlowSendController({
    chatting: options.chatting,
    submitPending,
    isConversationBusy: options.isConversationBusy,
    toolStatusText: options.toolStatusText,
    toolStatusState: options.toolStatusState,
    streamBlocks: options.streamBlocks,
    getConversationId: options.getConversationId,
    getSession: options.getSession,
    invokeSendChatMessage: options.invokeSendChatMessage,
    onOwnUserDraftInserted: options.onOwnUserDraftInserted,
    onAssistantDraftInserted: options.onAssistantDraftInserted,
    t: options.t,
    getRound: () => round,
    setRound,
    nextGeneration: () => ++generation,
    setSendChatActiveGen: (gen) => {
      sendChatActiveGen = gen;
    },
    setActiveActivationId: (value) => {
      activeActivationId = value;
    },
    setActiveRoundAgentId: (value) => {
      activeRoundAgentId = value;
    },
    setPendingTerminalEventNull: () => {
      pendingTerminalEvent = null;
    },
    sendStartedAtMsByGen,
    startFrontendDispatchTimer,
    clearFrontendDispatchTimer,
    clearConversationStreamCache,
    clearChatErrorText,
    applyPreparedSendInput: sendInput.applyPreparedSendInput,
    prepareSendInput: sendInput.prepareSendInput,
    insertUserDraft,
    resetDisplayState,
    removeDraft,
    updateQueuedAssistantDraftStatus,
    channelBinding,
    handleRoundCompleted,
    sendRecovery,
  });
  const foregroundReset = useChatFlowForegroundReset({
    latestUserText: options.latestUserText,
    latestUserImages: options.latestUserImages,
    latestAssistantText: options.latestAssistantText,
    toolStatusText: options.toolStatusText,
    toolStatusState: options.toolStatusState,
    streamBlocks: options.streamBlocks,
    chatting: options.chatting,
    submitPending,
    getConversationId: options.getConversationId,
    getRound: () => round,
    setRound,
    tickGeneration: () => {
      generation += 1;
    },
    setSendChatActiveGen: (value) => {
      sendChatActiveGen = value;
    },
    setActiveActivationId: (value) => {
      activeActivationId = value;
    },
    setActiveRoundAgentId: (value: string) => {
      activeRoundAgentId = String(value || "").trim();
    },
    setDeferredRoundCompletionNull: () => {
      deferredRoundCompletion = null;
    },
    setPendingTerminalEventNull: () => {
      pendingTerminalEvent = null;
    },
    resetQueuedStreamingState: () => {
      queuedStreamingState = null;
    },
    clearFrontendDispatchTimer,
    getPendingUserDraftId,
    removeDraft,
    removeAssistantDrafts,
    finalizeDraft,
    clearConversationStreamCache,
    setActiveHistoryMessageCount: (value) => {
      activeHistoryMessageCount = value;
    },
    reasoningStartedAtMs,
  });

  function setRound(next: RoundState, frontendPhase?: FrontendRoundPhase) {
    round = next;
    frontendRoundPhase.value = frontendPhase ?? next.phase;
  }

  // =========================================================================
  // 工具函数（纯逻辑，无副作用）
  // =========================================================================

  function setChatErrorText(text: string, conversationId?: string | null) {
    const cid = normalizeConversationId(conversationId || (options.getConversationId ? options.getConversationId() : ""));
    if (cid && options.setConversationChatError) {
      options.setConversationChatError(cid, text);
      return;
    }
    options.chatErrorText.value = text;
  }

  function clearChatErrorText(conversationId?: string | null) {
    setChatErrorText("", conversationId);
  }

  function clearFrontendDispatchTimer() {
    frontendDispatch.clear();
  }

  function clearContextUsagePreview() {
    if (options.contextUsagePreview) {
      options.contextUsagePreview.value = null;
    }
  }

  function startFrontendDispatchTimer(gen: number, startedAtMs?: number, elapsedMs?: number) {
    frontendDispatch.start(gen, startedAtMs, elapsedMs);
  }

  function restoreFrontendDispatchTimerFromCache(cache: ConversationStreamCache) {
    const gen = round.phase === "queued" || round.phase === "streaming" ? round.gen : 0;
    frontendDispatch.restoreFromCache(cache, gen);
  }

  function payloadMatchesActiveActivation(payload: { activationId?: string; requestId?: string } | null | undefined): boolean {
    if (!activeActivationId) return true;
    const payloadActivationId = String(payload?.activationId || payload?.requestId || "").trim();
    return !payloadActivationId || payloadActivationId === activeActivationId;
  }

  // =========================================================================
  // 显示状态重置（只在 history_flushed 清屏时调用）
  // =========================================================================

  function resetDisplayState() {
    clearContextUsagePreview();
    foregroundReset.resetDisplayState();
  }

  function clearForegroundRoundState() {
    clearContextUsagePreview();
    foregroundReset.clearForegroundRoundState();
  }

  function clearForegroundRuntimeState() {
    clearContextUsagePreview();
    foregroundReset.clearForegroundRuntimeState();
  }

  function freezeForegroundRoundState() {
    clearContextUsagePreview();
    foregroundReset.freezeForegroundRoundState();
  }

  function beginAssistantActivationFromEvent(payload: RoundStartedPayload): number {
    return foregroundRounds?.beginAssistantActivationFromEvent(payload) ?? 0;
  }

  function ensureForegroundWaitingRound(statusText = options.t("chat.statusWaitingReply")) {
    return foregroundRounds?.ensureForegroundWaitingRound(statusText) ?? 0;
  }

  function ensureForegroundStreamingRound() {
    return foregroundRounds?.ensureForegroundStreamingRound() ?? 0;
  }

  function resumeForegroundRuntimeRound(input?: ResumeForegroundRuntimeRoundInput) {
    return foregroundRounds?.resumeForegroundRuntimeRound(input) ?? 0;
  }

  function resumeForegroundStreamCacheProjection(input?: { conversationId?: string | null; reason?: string }) {
    return foregroundRounds?.resumeForegroundStreamCacheProjection(input) ?? 0;
  }

  function promoteQueuedRoundToStreaming(gen: number) {
    return foregroundRounds?.promoteQueuedRoundToStreaming(gen) ?? 0;
  }

  // =========================================================================
  // 事件处理
  // =========================================================================

  /**
 * history_flushed：唯一做 allMessages 大规模合并的地方。
 * 只表达“消息已落入正式历史”，不再推进助理轮次状态。
 * 助理是否已启动由 round_started 表达。
   */
  async function handleHistoryFlushed(
    gen: number,
    parsed: AssistantDeltaEvent,
    source: "sendChat" | "bound",
  ) {
    if (source === "sendChat") {
      submitPending.value = false;
    }
    const flushed = readHistoryFlushedPayload(parsed.message);
    if (flushed && options.onHistoryFlushed) {
      await options.onHistoryFlushed({
        conversationId: flushed.conversationId,
        messageCount: flushed.messageCount,
        pendingMessages: flushed.messages,
        activateAssistant: !!flushed.activateAssistant,
      });
    }
    await roundEvents.handleHistoryFlushed(gen, parsed, source);
  }

  async function markRoundStarted(gen: number) {
    await roundEvents.markRoundStarted(gen);
  }

  /**
   * round_completed：终结当前轮次。
   * 只做文字收尾 + 状态转换，不碰 allMessages（除了 updateDraftText）。
   */
  async function handleRoundCompleted(
    gen: number,
    result: {
      assistantText: string;
      assistantMessage?: ChatMessage;
    },
  ) {
    await roundEvents.handleRoundCompleted(gen, result);
  }

  async function handleRoundFailed(gen: number, error: unknown) {
    await roundEvents.handleRoundFailed(gen, error);
  }

  function applyPendingTerminalEvent(gen: number) {
    return roundEvents.applyPendingTerminalEvent(gen);
  }

  // =========================================================================
  // Delta 分发
  // =========================================================================
  function handleStreamingEvent(currentGen: number, parsed: AssistantDeltaEvent) {
    streamingEvents.handleStreamingEvent(currentGen, parsed);
  }

  // =========================================================================
  // 公共方法
  // =========================================================================

  async function sendChat(overrides?: SendChatOverrides) {
    clearContextUsagePreview();
    await sendController.sendChat(overrides);
  }

  async function stopChat() {
    clearContextUsagePreview();
    await stopController.stopChat();
  }

  return {
    sendChat,
    stopChat,
    clearForegroundRoundState,
    clearForegroundRuntimeState,
    freezeForegroundRoundState,
    readConversationStreamCache,
    resumeForegroundStreamingRound: ensureForegroundStreamingRound,
    resumeForegroundRuntimeRound,
    resumeForegroundStreamCacheProjection,
    bindActiveConversationStream: channelBinding.bindActiveConversationStream,
    handleExternalStreamRebindRequired: externalEvents.handleExternalStreamRebindRequired,
    handleExternalHistoryFlushed: externalEvents.handleExternalHistoryFlushed,
    handleExternalRoundStarted: externalEvents.handleExternalRoundStarted,
    handleExternalRoundCompleted: externalEvents.handleExternalRoundCompleted,
    handleExternalRoundFailed: externalEvents.handleExternalRoundFailed,
    handleExternalAssistantDelta: externalEvents.handleExternalAssistantDelta,
    frontendRoundPhase,
    submitPending,
    reasoningStartedAtMs,
  };
}

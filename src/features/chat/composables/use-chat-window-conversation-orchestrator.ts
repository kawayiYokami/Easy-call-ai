import { computed } from "vue";
import { formatI18nError } from "../../../utils/error";
import { useChatConversationActionsOrchestrator } from "./use-chat-conversation-actions-orchestrator";
import { useChatConversationDialogGlue } from "./use-chat-conversation-dialog-glue";
import { useChatConversationSync } from "./use-chat-conversation-sync";
import { useChatForegroundCacheScheduler } from "./use-chat-foreground-cache-scheduler";
import { useChatForegroundOrchestrator } from "./use-chat-foreground-orchestrator";
import { useChatRemoteConversationOrchestrator } from "./use-chat-remote-conversation-orchestrator";

export function useChatWindowConversationOrchestrator(bindings: Record<string, any>) {
  const {
    matchesForegroundConversation,
    formalizeConversationMessages,
    freezeConversationMessages,
    isAssistantDraftMessage,
    insertMessagesBeforeAssistantDraft,
    currentConversationRuntimeState,
    maybeResumeForegroundStreamingDraft,
    conversationRuntimeSnapshotIsBusy,
    requestConversationRuntimeSnapshot,
    resumeForegroundRuntimeFromBackend,
    areMessagesEquivalent,
    messageContentSignature,
    reuseStableMessageReferences,
    beginForegroundPaintTrace,
    logForegroundPaintTrace,
    cacheConversationMessages,
    inferHasMoreHistoryFromSnapshot,
    clearConversationBadge,
    markConversationReadPersisted,
    applyConversationOverviewAppendedMessage,
    setConversationBadge,
    readConversationIdFromPayload,
    readMessagesFromPayload,
    mergeIncomingMessagesIntoCache,
    buildConversationMessagesAfterAnchor,
    requestConversationMessagesAfterAsync,
    requestConversationMessageById,
    replaceConversationMessage,
    reloadForegroundConversationMessages,
    refreshForegroundConversationMessageById,
    loadOlderConversationHistory,
    mergeConversationMessagesFromSyncPayload,
    applyConversationMessagesAfterSynced,
    applyConversationMessageAppended,
    applyConversationSnapshot,
    applyConversationTodosUpdated,
    applyConversationOverviewUpdated,
    applyConversationPinUpdated,
    applyConversationRuntimeStateUpdated,
    isOverviewDraftMessage,
    previewMessageFromChatMessage,
    unarchivedConversationActivityAt,
    sortUnarchivedConversationOverviewItems,
    updateForegroundConversationOverviewFromMessages,
    maybeUpdateForegroundConversationOverviewFromLoadedMessages,
  } = useChatConversationSync(bindings.sync);

  const {
    clearForegroundConversationCacheRaf,
    scheduleForegroundConversationCachePersist,
  } = useChatForegroundCacheScheduler({
    cacheConversationMessages,
  });

  const chatForeground = useChatForegroundOrchestrator({
    FOREGROUND_SNAPSHOT_RECENT_LIMIT: bindings.FOREGROUND_SNAPSHOT_RECENT_LIMIT,
    BACKGROUND_CONVERSATION_CACHE_LIMIT: bindings.BACKGROUND_CONVERSATION_CACHE_LIMIT,
    config: bindings.config,
    tauriWindowLabel: bindings.tauriWindowLabel,
    detachedChatWindow: bindings.detachedChatWindow,
    detachedChatConversationId: bindings.detachedChatConversationId,
    detachedTemporaryApiConfigId: bindings.detachedTemporaryApiConfigId,
    currentChatConversationId: bindings.currentChatConversationId,
    currentChatTodos: bindings.currentChatTodos,
    currentForegroundAgentId: bindings.currentForegroundAgentId,
    currentForegroundConversationSummary: bindings.currentForegroundConversationSummary,
    unarchivedConversations: bindings.unarchivedConversations,
    remoteImContactConversations: bindings.remoteImContactConversations,
    conversationForegroundSyncing: bindings.conversationForegroundSyncing,
    trimmingConversationId: bindings.trimmingConversationId,
    compactingConversationId: bindings.compactingConversationId,
    trimming: bindings.trimming,
    compactingConversation: bindings.compactingConversation,
    chatting: bindings.chatting,
    sideConversationListVisible: bindings.sideConversationListVisible,
    allMessages: bindings.allMessages,
    hasMoreBackendHistory: bindings.hasMoreBackendHistory,
    foregroundTailLatestReady: bindings.foregroundTailLatestReady,
    chatWorkspaceName: bindings.chatWorkspaceName,
    cacheConversationMessages,
    clearConversationBadge,
    markConversationReadPersisted,
    beginForegroundPaintTrace,
    logForegroundPaintTrace,
    applyConversationSnapshot,
    resumeForegroundRuntimeFromBackend,
    maybeResumeForegroundStreamingDraft,
    buildConversationMessagesAfterAnchor,
    clearPendingManualScrollToBottom: bindings.clearPendingManualScrollToBottom,
    triggerConversationScrollToBottom: bindings.triggerConversationScrollToBottom,
    setPendingManualScrollState: bindings.setPendingManualScrollState,
    setStatus: bindings.setStatus,
    setStatusError: bindings.setStatusError,
    perfNow: bindings.perfNow,
    isChatWindowActiveNow: bindings.isChatWindowActiveNow,
    closeWindow: bindings.closeWindow,
    freezeForegroundConversation: bindings.freezeForegroundConversation,
    getChatFlow: bindings.getChatFlow,
  });

  const chatRemoteConversation = useChatRemoteConversationOrchestrator({
    remoteImContactConversations: bindings.remoteImContactConversations,
    currentChatConversationId: bindings.currentChatConversationId,
    currentChatTodos: bindings.currentChatTodos,
    conversationForegroundSyncing: bindings.conversationForegroundSyncing,
    allMessages: bindings.allMessages,
    hasMoreBackendHistory: bindings.hasMoreBackendHistory,
    foregroundTailLatestReady: bindings.foregroundTailLatestReady,
    conversationMessageCache: bindings.conversationMessageCache,
    cacheConversationMessages,
    clearConversationBadge,
    markConversationReadPersisted,
    getChatFlow: bindings.getChatFlow,
    clearPendingManualScrollToBottom: bindings.clearPendingManualScrollToBottom,
    freezeConversationMessages,
    reuseStableMessageReferences,
    refreshUnarchivedConversationOverview: chatForeground.refreshUnarchivedConversationOverview,
    refreshRemoteImConversationOverview: chatForeground.refreshRemoteImConversationOverview,
    switchUnarchivedConversation: chatForeground.switchUnarchivedConversation,
    setStatusError: bindings.setStatusError,
    FOREGROUND_SNAPSHOT_RECENT_LIMIT: bindings.FOREGROUND_SNAPSHOT_RECENT_LIMIT,
  });

  const chatConversationActions = useChatConversationActionsOrchestrator({
    t: bindings.t,
    tr: bindings.tr,
    detachedChatWindow: bindings.detachedChatWindow,
    currentChatConversationId: bindings.currentChatConversationId,
    currentForegroundAgentId: bindings.currentForegroundAgentId,
    createConversationDepartmentOptions: bindings.createConversationDepartmentOptions,
    defaultCreateConversationDepartmentId: bindings.defaultCreateConversationDepartmentId,
    unarchivedConversations: bindings.unarchivedConversations,
    branchingConversation: bindings.branchingConversation,
    forwardingConversationSelection: bindings.forwardingConversationSelection,
    trimming: bindings.trimming,
    refreshUnarchivedConversationOverview: chatForeground.refreshUnarchivedConversationOverview,
    switchUnarchivedConversation: chatForeground.switchUnarchivedConversation,
    requestConversationLightSnapshot: chatForeground.requestConversationLightSnapshot,
    applyConversationSnapshot,
    applyConversationPinUpdated,
    setStatus: bindings.setStatus,
    setStatusError: bindings.setStatusError,
    formatRequestFailed: (error: unknown) => formatI18nError(bindings.tr, "status.requestFailed", error),
  });

  const chatConversationDialogGlue = useChatConversationDialogGlue({
    detachedChatWindow: bindings.detachedChatWindow,
    currentChatConversationId: bindings.currentChatConversationId,
    currentForegroundApiConfigId: bindings.currentForegroundApiConfigId,
    currentForegroundAgentId: bindings.currentForegroundAgentId,
    unarchivedConversations: bindings.unarchivedConversations,
    conversationForegroundSyncing: bindings.conversationForegroundSyncing,
    deleteUnarchivedConversationFromArchivesRaw: bindings.deleteUnarchivedConversationFromArchivesRaw,
    requestConversationLightSnapshot: chatForeground.requestConversationLightSnapshot,
    applyConversationSnapshot,
    pickForegroundConversationId: chatForeground.pickForegroundConversationId,
    clearForegroundConversation: chatForeground.clearForegroundConversation,
    recoverForegroundConversationFromOverview: chatForeground.recoverForegroundConversationFromOverview,
    switchUnarchivedConversation: chatForeground.switchUnarchivedConversation,
    archiveCurrentConversation: bindings.archiveCurrentConversation,
    getOpenTrimActionDialog: () => bindings.openTrimActionDialog,
    getConfirmTrimAction: () => bindings.confirmTrimAction,
    getCloseTrimActionDialog: () => bindings.closeTrimActionDialog,
    setStatus: bindings.setStatus,
    setStatusError: bindings.setStatusError,
  });

  async function refreshChatUnarchivedConversations() {
    await chatForeground.refreshChatUnarchivedConversations();
  }

  async function sendChatFromCurrentWindow(overrides?: { extraTextBlocks?: string[] }) {
    await chatForeground.sendChatFromCurrentWindow(overrides);
  }

  function freezeForegroundConversation(reason: string) {
    chatForeground.freezeForegroundConversation(reason);
  }

  async function restoreForegroundConversationProjection(conversationId: string, reason: string) {
    const cid = String(conversationId || "").trim();
    if (!cid) return;
    // 不再从前端本地缓存恢复流式投影，统一只从后端读取。
    // 后端 snapshot 天然包含 persistedAssistantMessageId，去重逻辑只需在 resumeForegroundRuntimeRound 中做一次。
    const runtimeState = await resumeForegroundRuntimeFromBackend(cid, reason);
    if (runtimeState === "idle") {
      bindings.getChatFlow().clearForegroundRoundState();
    }
  }

  async function deleteUnarchivedConversationFromArchives(conversationId: string) {
    await chatConversationDialogGlue.deleteUnarchivedConversationFromArchives(conversationId);
  }

  return {
    clearForegroundConversationCacheRaf,
    scheduleForegroundConversationCachePersist,
    matchesForegroundConversation,
    formalizeConversationMessages,
    freezeConversationMessages,
    isAssistantDraftMessage,
    insertMessagesBeforeAssistantDraft,
    currentConversationRuntimeState,
    maybeResumeForegroundStreamingDraft,
    conversationRuntimeSnapshotIsBusy,
    requestConversationRuntimeSnapshot,
    resumeForegroundRuntimeFromBackend,
    areMessagesEquivalent,
    messageContentSignature,
    reuseStableMessageReferences,
    beginForegroundPaintTrace,
    logForegroundPaintTrace,
    cacheConversationMessages,
    inferHasMoreHistoryFromSnapshot,
    clearConversationBadge,
    markConversationReadPersisted,
    applyConversationOverviewAppendedMessage,
    setConversationBadge,
    readConversationIdFromPayload,
    readMessagesFromPayload,
    mergeIncomingMessagesIntoCache,
    buildConversationMessagesAfterAnchor,
    requestConversationMessagesAfterAsync,
    requestConversationMessageById,
    replaceConversationMessage,
    reloadForegroundConversationMessages,
    refreshForegroundConversationMessageById,
    loadOlderConversationHistory,
    mergeConversationMessagesFromSyncPayload,
    applyConversationMessagesAfterSynced,
    applyConversationMessageAppended,
    applyConversationSnapshot,
    applyConversationTodosUpdated,
    applyConversationOverviewUpdated,
    applyConversationPinUpdated,
    applyConversationRuntimeStateUpdated,
    isOverviewDraftMessage,
    previewMessageFromChatMessage,
    unarchivedConversationActivityAt,
    sortUnarchivedConversationOverviewItems,
    updateForegroundConversationOverviewFromMessages,
    maybeUpdateForegroundConversationOverviewFromLoadedMessages,
    pickForegroundConversationId: chatForeground.pickForegroundConversationId,
    clearForegroundConversation: chatForeground.clearForegroundConversation,
    initializeDetachedChatWindow: chatForeground.initializeDetachedChatWindow,
    handleCloseWindow: chatForeground.handleCloseWindow,
    detachCurrentConversationToWindow: chatForeground.detachCurrentConversationToWindow,
    hasActiveForegroundConversation: chatForeground.hasActiveForegroundConversation,
    requestConversationLightSnapshot: chatForeground.requestConversationLightSnapshot,
    requestUnarchivedConversationOverview: chatForeground.requestUnarchivedConversationOverview,
    refreshRemoteImConversationOverview: chatForeground.refreshRemoteImConversationOverview,
    refreshUnarchivedConversationOverview: chatForeground.refreshUnarchivedConversationOverview,
    recoverForegroundConversationFromOverview: chatForeground.recoverForegroundConversationFromOverview,
    syncCurrentConversationWorkspaceLabel: chatForeground.syncCurrentConversationWorkspaceLabel,
    switchUnarchivedConversation: chatForeground.switchUnarchivedConversation,
    ensureLatestForegroundTailThenScrollToBottom: chatForeground.ensureLatestForegroundTailThenScrollToBottom,
    refreshChatUnarchivedConversations,
    sendChatFromCurrentWindow,
    freezeForegroundConversation,
    restoreForegroundConversationProjection,
    switchRemoteImContactConversation: chatRemoteConversation.switchRemoteImContactConversation,
    openConversationInDetachedWindowById: chatRemoteConversation.openConversationInDetachedWindowById,
    switchChatConversation: chatRemoteConversation.switchChatConversation,
    createUnarchivedConversation: chatConversationActions.createUnarchivedConversation,
    branchConversationFromSelection: chatConversationActions.branchConversationFromSelection,
    forwardConversationFromSelection: chatConversationActions.forwardConversationFromSelection,
    userAsyncDelegateFromSelection: chatConversationActions.userAsyncDelegateFromSelection,
    renameCurrentConversation: chatConversationActions.renameCurrentConversation,
    toggleConversationPin: chatConversationActions.toggleConversationPin,
    archiveConversationFromList: chatConversationDialogGlue.archiveConversationFromList,
    handleConfirmTrimAction: chatConversationDialogGlue.handleConfirmTrimAction,
    deleteUnarchivedConversationFromArchives,
  };
}

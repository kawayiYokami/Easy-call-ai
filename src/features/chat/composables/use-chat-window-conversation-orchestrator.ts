import { computed } from "vue";
import { formatI18nError } from "../../../utils/error";
import { useChatConversationActionsOrchestrator } from "./use-chat-conversation-actions-orchestrator";
import { useChatConversationDialogGlue } from "./use-chat-conversation-dialog-glue";
import { useChatConversationSync } from "./use-chat-conversation-sync";
import { useChatForegroundOrchestrator } from "./use-chat-foreground-orchestrator";
import { useChatRemoteConversationOrchestrator } from "./use-chat-remote-conversation-orchestrator";

export function useChatWindowConversationOrchestrator(bindings: Record<string, any>) {
  const {
    matchesForegroundConversation,
    formalizeConversationMessages,
    freezeConversationMessages,
    isAssistantDraftMessage,
    insertMessagesBeforeAssistantDraft,
    mergeMessagesIntoTimeline,
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
    applyConversationOverviewUpdated: applyConversationOverviewUpdatedRaw,
    applyConversationPinUpdated,
    applyConversationRuntimeStateUpdated,
    isOverviewDraftMessage,
    previewMessageFromChatMessage,
    unarchivedConversationActivityAt,
    sortUnarchivedConversationOverviewItems,
    updateForegroundConversationOverviewFromMessages,
    maybeUpdateForegroundConversationOverviewFromLoadedMessages,
  } = useChatConversationSync(bindings.sync);

  const chatForeground = useChatForegroundOrchestrator({
    FOREGROUND_SNAPSHOT_RECENT_LIMIT: bindings.FOREGROUND_SNAPSHOT_RECENT_LIMIT,
    BACKGROUND_CONVERSATION_CACHE_LIMIT: bindings.BACKGROUND_CONVERSATION_CACHE_LIMIT,
    config: bindings.config,
    tauriWindowLabel: bindings.tauriWindowLabel,
    detachedChatWindow: bindings.detachedChatWindow,
    detachedChatConversationId: bindings.detachedChatConversationId,
    detachedTemporaryApiConfigId: bindings.detachedTemporaryApiConfigId,
    currentChatConversationId: bindings.currentChatConversationId,
    currentChatPreferredApiConfigId: bindings.currentChatPreferredApiConfigId,
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
    waitPendingConversationPreferredModelPersist: bindings.waitPendingConversationPreferredModelPersist,
    setStatus: bindings.setStatus,
    setStatusError: bindings.setStatusError,
    perfNow: bindings.perfNow,
    isChatWindowActiveNow: bindings.isChatWindowActiveNow,
    closeWindow: bindings.closeWindow,
    freezeForegroundConversation: bindings.freezeForegroundConversation,
    getChatFlow: bindings.getChatFlow,
  });

  function applyConversationOverviewUpdated(payload?: Record<string, any> | null) {
    applyConversationOverviewUpdatedRaw(payload);
    const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (!currentConversationId) return;
    const stillVisible = bindings.unarchivedConversations.value.some(
      (item: any) => String(item.conversationId || "").trim() === currentConversationId,
    );
    if (stillVisible) return;
    const preferredConversationId = String(payload?.preferredConversationId || "").trim() || null;
    void chatForeground.recoverForegroundConversationFromOverview(
      "conversation_overview_updated_missing_current",
      preferredConversationId,
    ).catch((error: unknown) => {
      bindings.setStatusError("status.loadMessagesFailed", error);
    });
  }

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
    // 如果前端流式仍在进行中，跳过恢复——流式通道会自动推送最新内容
    const chatFlow = bindings.getChatFlow();
    if (chatFlow.frontendRoundPhase?.value !== "idle") {
      return;
    }
    // 和 switchUnarchivedConversation 一致的快照恢复路径，不走缓存直灌
    const snapshot = await chatForeground.requestConversationLightSnapshot(cid, { resumeProjection: true });
    if (cid !== String(bindings.currentChatConversationId.value || "").trim()) return;
    applyConversationSnapshot(snapshot);
    if (cid !== String(bindings.currentChatConversationId.value || "").trim()) return;
    const shouldBindStream = !!snapshot?.shouldBindStream;
    if (shouldBindStream) {
      await chatFlow.bindActiveConversationStream(cid, true);
      return;
    }
    const snapshotRuntimeState = String(snapshot?.runtimeState || "").trim();
    if (snapshotRuntimeState !== "assistant_streaming") {
      chatFlow.clearForegroundRoundState();
    }
  }

  async function deleteUnarchivedConversationFromArchives(conversationId: string) {
    await chatConversationDialogGlue.deleteUnarchivedConversationFromArchives(conversationId);
  }

  return {
    matchesForegroundConversation,
    formalizeConversationMessages,
    freezeConversationMessages,
    isAssistantDraftMessage,
    insertMessagesBeforeAssistantDraft,
    mergeMessagesIntoTimeline,
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

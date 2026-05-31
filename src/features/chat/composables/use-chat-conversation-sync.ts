import { nextTick } from "vue";
import { invokeTauri } from "../../../services/tauri-api";
import {
  readConversationIdFromPayload,
  readMessagesFromPayload,
  useChatConversationMessageUtils,
} from "./use-chat-conversation-message-utils";
import { useChatConversationOverviewUtils } from "./use-chat-conversation-overview-utils";

type ForegroundPaintTrace = {
  id: number;
  conversationId: string;
  startedAt: number;
};

export function useChatConversationSync(bindings: Record<string, any>) {
  let foregroundPaintTraceSeq = 0;
  let foregroundRuntimeResumeSeq = 0;
  const {
    areMessagesEquivalent,
    formalizeConversationMessages,
    freezeConversationMessages,
    insertMessagesBeforeAssistantDraft,
    isAssistantDraftMessage,
    messageContentSignature,
    replaceConversationMessage,
    reuseStableMessageReferences,
  } = useChatConversationMessageUtils({
    draftAssistantIdPrefix: bindings.DRAFT_ASSISTANT_ID_PREFIX,
    ensureConversationMessageIds: bindings.ensureConversationMessageIds,
  });
  const {
    isOverviewDraftMessage,
    previewMessageFromChatMessage,
    sortUnarchivedConversationOverviewItems,
    unarchivedConversationActivityAt,
  } = useChatConversationOverviewUtils({
    draftAssistantIdPrefix: bindings.DRAFT_ASSISTANT_ID_PREFIX,
  });

  function matchesForegroundConversation(conversationId?: string | null): boolean {
    const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (!currentConversationId) return false;
    const targetConversationId = String(conversationId || "").trim();
    return !targetConversationId || targetConversationId === currentConversationId;
  }

  function currentConversationRuntimeState(conversationId?: string | null) {
    const cid = String(conversationId || "").trim();
    if (!cid) return "";
    return String(
      bindings.unarchivedConversations.value.find((item: any) => String(item.conversationId || "").trim() === cid)?.runtimeState || "",
    ).trim();
  }

  function maybeResumeForegroundStreamingDraft(conversationId?: string | null, reason = "unknown") {
    const cid = String(conversationId || "").trim();
    if (!cid) return;
    if (cid !== String(bindings.currentChatConversationId.value || "").trim()) return;
    if (currentConversationRuntimeState(cid) !== "assistant_streaming") return;
    bindings.getChatFlow().resumeForegroundStreamingRound();
  }

  function conversationRuntimeSnapshotIsBusy(snapshot?: any): boolean {
    if (!snapshot) return false;
    return snapshot.runtimeState === "assistant_streaming"
      || !!snapshot.isProcessing
      || !!snapshot.hasPendingQueue
      || Math.max(0, Number(snapshot.pendingQueueCount || 0)) > 0;
  }

  async function requestConversationRuntimeSnapshot(conversationId: string) {
    return invokeTauri<any>("get_conversation_runtime_snapshot", {
      conversationId,
    });
  }

  async function resumeForegroundRuntimeFromBackend(conversationId?: string | null, reason = "unknown") {
    const cid = String(conversationId || "").trim();
    if (!cid || cid !== String(bindings.currentChatConversationId.value || "").trim()) return "unknown";
    const resumeSeq = ++foregroundRuntimeResumeSeq;
    try {
      const snapshot = await requestConversationRuntimeSnapshot(cid);
      if (resumeSeq !== foregroundRuntimeResumeSeq) return "unknown";
      if (cid !== String(bindings.currentChatConversationId.value || "").trim()) return "unknown";
      const busy = conversationRuntimeSnapshotIsBusy(snapshot);
      if (!busy) return "idle";
      await bindings.getChatFlow().bindActiveConversationStream(cid, true);
      if (resumeSeq !== foregroundRuntimeResumeSeq) return "unknown";
      if (cid !== String(bindings.currentChatConversationId.value || "").trim()) return "unknown";
      bindings.getChatFlow().resumeForegroundRuntimeRound({
        conversationId: cid,
        streamCache: snapshot.streamCache || null,
        statusText: bindings.tr("chat.statusWaitingReply"),
        reason,
      });
      return "busy";
    } catch (error) {
      console.warn("[聊天运行态恢复] 后端快照读取失败", {
        conversationId: cid,
        reason,
        error,
      });
      return "unknown";
    }
  }

  function beginForegroundPaintTrace(conversationId: string): ForegroundPaintTrace {
    return {
      id: ++foregroundPaintTraceSeq,
      conversationId: String(conversationId || "").trim(),
      startedAt: bindings.perfNow(),
    };
  }

  function logForegroundPaintTrace(
    trace: ForegroundPaintTrace,
    label: string,
    detail?: Record<string, unknown>,
  ) {
    void trace;
    void label;
    void detail;
  }

  function cacheConversationMessages(conversationId: string, messages: any[]) {
    const cid = String(conversationId || "").trim();
    if (!cid) return;
    const cachedMessages = freezeConversationMessages(Array.isArray(messages) ? messages : []);
    bindings.conversationMessageCache.value = {
      ...bindings.conversationMessageCache.value,
      [cid]: cachedMessages.slice(-bindings.BACKGROUND_CONVERSATION_CACHE_LIMIT),
    };
  }

  function inferHasMoreHistoryFromSnapshot(messages: any[]): boolean {
    return Array.isArray(messages) && messages.length >= bindings.BACKGROUND_CONVERSATION_CACHE_LIMIT;
  }

  function clearConversationBadge(conversationId: string) {
    const cid = String(conversationId || "").trim();
    if (!cid) return;
    const hasBackgroundBadge = !!bindings.backgroundConversationBadgeMap.value[cid];
    if (hasBackgroundBadge) {
      const next = { ...bindings.backgroundConversationBadgeMap.value };
      delete next[cid];
      bindings.backgroundConversationBadgeMap.value = next;
    }
    let changed = false;
    const nextItems = bindings.unarchivedConversations.value.map((item: any) => {
      if (String(item.conversationId || "").trim() !== cid) return item;
      if (Math.max(0, Number(item.unreadCount || 0)) <= 0) return item;
      changed = true;
      return {
        ...item,
        unreadCount: 0,
      };
    });
    if (changed) {
      bindings.unarchivedConversations.value = nextItems;
    }
  }

  function markConversationReadPersisted(conversationId: string) {
    const cid = String(conversationId || "").trim();
    if (!cid) return;
    void invokeTauri("mark_conversation_read", {
      input: { conversationId: cid },
    }).catch((error) => {
      console.warn("[会话已读] 持久化失败", {
        conversationId: cid,
        error,
      });
    });
  }

  function applyConversationOverviewAppendedMessage(conversationId: string, message: any) {
    const cid = String(conversationId || "").trim();
    const messageId = String(message?.id || "").trim();
    if (!cid || !message || !messageId || isOverviewDraftMessage(message)) return;
    const preview = previewMessageFromChatMessage(message);
    const messageAt = String(message.createdAt || "").trim();
    let changed = false;
    const nextItems = bindings.unarchivedConversations.value.map((item: any) => {
      if (String(item.conversationId || "").trim() !== cid) {
        return item;
      }
      const existingPreviewMessages = Array.isArray(item.previewMessages) ? item.previewMessages : [];
      if (existingPreviewMessages.some((previewItem: any) => String(previewItem.messageId || "").trim() === messageId)) {
        return item;
      }
      changed = true;
      const shouldMarkUnread = cid !== String(bindings.currentChatConversationId.value || "").trim();
      return {
        ...item,
        messageCount: Math.max(0, Number(item.messageCount || 0)) + 1,
        unreadCount: Math.max(0, Number(item.unreadCount || 0)) + (shouldMarkUnread ? 1 : 0),
        updatedAt: messageAt || item.updatedAt,
        lastMessageAt: messageAt || item.lastMessageAt,
        previewMessages: [...existingPreviewMessages, preview].slice(-2),
      };
    });
    if (changed) {
      bindings.unarchivedConversations.value = sortUnarchivedConversationOverviewItems(nextItems);
    }
  }

  function setConversationBadge(conversationId: string, status: string) {
    const cid = String(conversationId || "").trim();
    if (!cid) return;
    bindings.backgroundConversationBadgeMap.value = {
      ...bindings.backgroundConversationBadgeMap.value,
      [cid]: status,
    };
  }

  function mergeIncomingMessagesIntoCache(conversationId: string, messages: any[]) {
    const cid = String(conversationId || "").trim();
    if (!cid || !Array.isArray(messages) || messages.length <= 0) return;
    const incoming = messages.filter((message) => !!String(message?.id || "").trim());
    if (incoming.length <= 0) return;
    const cachedDisplay = freezeConversationMessages(bindings.conversationMessageCache.value[cid] || []);
    const cachedFormal = formalizeConversationMessages(cachedDisplay);
    const incomingIds = new Set(incoming.map((message) => String(message.id || "").trim()));
    const nextCached = [
      ...cachedFormal.filter((message) => !incomingIds.has(String(message?.id || "").trim())),
      ...incoming,
    ];
    cacheConversationMessages(cid, nextCached);
    const latestMessage = incoming[incoming.length - 1];
    if (latestMessage) applyConversationOverviewAppendedMessage(cid, latestMessage);
  }

  function buildConversationMessagesAfterAnchor(conversationId: string): string | null {
    const cid = String(conversationId || "").trim();
    if (!cid) return null;
    const cachedDisplay = freezeConversationMessages(bindings.conversationMessageCache.value[cid] || []);
    const cachedFormal = formalizeConversationMessages(cachedDisplay);
    const lastFormalMessageId = String(cachedFormal[cachedFormal.length - 1]?.id || "").trim();
    return lastFormalMessageId || null;
  }

  async function requestConversationMessagesAfterAsync(conversationId: string, trace?: ForegroundPaintTrace) {
    const cid = String(conversationId || "").trim();
    if (!cid) return;
    const afterMessageId = buildConversationMessagesAfterAnchor(cid);
    if (trace) {
      logForegroundPaintTrace(trace, "开始请求后台异步补消息", {
        afterMessageId: afterMessageId || "",
      });
    }
    await invokeTauri("request_conversation_messages_after_async", {
      input: {
        conversationId: cid,
        afterMessageId,
        fallbackLimit: bindings.BACKGROUND_CONVERSATION_CACHE_LIMIT,
      },
    });
  }

  async function requestConversationMessageById(conversationId: string, messageId: string) {
    return invokeTauri("get_unarchived_conversation_message_by_id", {
      input: {
        conversationId,
        messageId,
      },
    });
  }

  async function reloadForegroundConversationMessages(reason = "unknown") {
    const conversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (!conversationId) {
      await bindings.loadAllMessages();
      return;
    }
    try {
      await requestConversationMessagesAfterAsync(conversationId);
    } catch (error) {
      console.warn("[会话缓存] 增量补消息失败，回退全量加载", {
        reason,
        conversationId,
        error,
      });
      await bindings.loadAllMessages();
    }
  }

  async function refreshForegroundConversationMessageById(payload: { conversationId: string; messageId: string }) {
    const conversationId = String(payload?.conversationId || "").trim();
    const messageId = String(payload?.messageId || "").trim();
    if (!conversationId || !messageId) return;
    try {
      const refreshedMessage = freezeConversationMessages([
        await requestConversationMessageById(conversationId, messageId),
      ])[0];
      if (!refreshedMessage) return;

      const cachedDisplay = freezeConversationMessages(bindings.conversationMessageCache.value[conversationId] || []);
      const nextCached = replaceConversationMessage(cachedDisplay, refreshedMessage);
      if (nextCached !== cachedDisplay) {
        cacheConversationMessages(conversationId, nextCached);
      }

      if (String(bindings.currentChatConversationId.value || "").trim() !== conversationId) {
        return;
      }
      const nextMessages = replaceConversationMessage(bindings.allMessages.value, refreshedMessage);
      if (nextMessages === bindings.allMessages.value) {
        return;
      }
      bindings.allMessages.value = nextMessages;
      cacheConversationMessages(conversationId, nextMessages);
    } catch (error) {
      console.warn("[会话缓存] 单条消息刷新失败", {
        conversationId,
        messageId,
        error,
      });
    }
  }

  async function loadOlderConversationHistory() {
    const conversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (!conversationId || bindings.loadingOlderConversationHistory.value || !bindings.hasMoreBackendHistory.value) {
      return;
    }
    const apiConfigId = String(bindings.currentForegroundApiConfigId.value || "").trim();
    const agentId = String(bindings.currentForegroundAgentId.value || "").trim();
    if (!apiConfigId || !agentId) return;

    const formalMessages = formalizeConversationMessages(bindings.allMessages.value);
    const oldestMessageId = String(formalMessages[0]?.id || "").trim();
    if (!oldestMessageId) {
      bindings.hasMoreBackendHistory.value = false;
      return;
    }

    bindings.loadingOlderConversationHistory.value = true;
    try {
      const result = await invokeTauri("get_active_conversation_messages_before", {
        input: {
          session: {
            apiConfigId,
            agentId,
            conversationId,
          },
          beforeMessageId: oldestMessageId,
          limit: bindings.OLDER_HISTORY_PAGE_SIZE,
        },
      }) as { messages?: any[]; hasMore?: boolean };
      if (
        String(bindings.currentChatConversationId.value || "").trim() !== conversationId
        || String(bindings.currentForegroundApiConfigId.value || "").trim() !== apiConfigId
        || String(bindings.currentForegroundAgentId.value || "").trim() !== agentId
      ) {
        return;
      }
      const previousMessages = Array.isArray(bindings.allMessages.value) ? bindings.allMessages.value : [];
      const incomingMessages = freezeConversationMessages(Array.isArray(result?.messages) ? result.messages : []);
      const existingIds = new Set(previousMessages.map((item: any) => String(item?.id || "").trim()).filter(Boolean));
      const uniqueIncoming = incomingMessages.filter((item: any) => {
        const messageId = String(item?.id || "").trim();
        return !!messageId && !existingIds.has(messageId);
      });
      const nextMessages = reuseStableMessageReferences([...uniqueIncoming, ...previousMessages], previousMessages);
      bindings.allMessages.value = nextMessages;
      cacheConversationMessages(conversationId, nextMessages);
      bindings.hasMoreBackendHistory.value = !!result?.hasMore;
    } catch (error) {
      console.warn("[会话缓存] 向上补历史失败", {
        conversationId,
        error,
      });
      bindings.setStatusError("status.loadMessagesFailed", error);
    } finally {
      bindings.loadingOlderConversationHistory.value = false;
    }
  }

  function mergeConversationMessagesFromSyncPayload(
    conversationId: string,
    payloadMessages: any[],
    fallbackMode?: string | null,
  ) {
    const cid = String(conversationId || "").trim();
    const nextPayloadMessages = freezeConversationMessages(Array.isArray(payloadMessages) ? payloadMessages : []);
    const cachedDisplay = freezeConversationMessages(bindings.conversationMessageCache.value[cid] || []);
    const cachedFormal = formalizeConversationMessages(cachedDisplay);
    const fallback = String(fallbackMode || "").trim();
    if (fallback === "recent_limit") {
      return nextPayloadMessages;
    }
    const merged = [...cachedFormal];
    const existingIds = new Set(merged.map((item) => String(item?.id || "").trim()).filter(Boolean));
    for (const message of nextPayloadMessages) {
      const messageId = String(message?.id || "").trim();
      if (!messageId || existingIds.has(messageId)) continue;
      existingIds.add(messageId);
      merged.push(message);
    }
    const nextMerged = merged.length > 0 ? merged : cachedDisplay;
    return reuseStableMessageReferences(nextMerged, cachedDisplay);
  }

  async function applyConversationMessagesAfterSynced(payload: Record<string, any>) {
    const conversationId = String(payload?.conversationId || "").trim();
    const requestId = String(payload?.requestId || "").trim();
    if (!conversationId) return;
    if (payload?.error) {
      console.warn("[会话缓存] 异步补消息失败", {
        conversationId,
        requestId,
        error: payload.error,
      });
      if (
        requestId
        && requestId === bindings.getPendingManualScrollToBottomRequestId()
        && conversationId === bindings.getPendingManualScrollToBottomConversationId()
      ) {
        bindings.clearPendingManualScrollToBottom();
      }
      return;
    }
    const nextMessages = mergeConversationMessagesFromSyncPayload(
      conversationId,
      Array.isArray(payload?.messages) ? payload.messages : [],
      payload?.fallbackMode ?? null,
    );
    cacheConversationMessages(conversationId, nextMessages);
    if (String(bindings.currentChatConversationId.value || "").trim() === conversationId) {
      if (!areMessagesEquivalent(bindings.allMessages.value, nextMessages)) {
        bindings.allMessages.value = nextMessages;
      }
      bindings.foregroundTailLatestReady.value = true;
      await nextTick();
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      if (
        requestId
        && requestId === bindings.getPendingManualScrollToBottomRequestId()
        && conversationId === bindings.getPendingManualScrollToBottomConversationId()
      ) {
        bindings.clearPendingManualScrollToBottom();
        bindings.triggerConversationScrollToBottom(conversationId, "manual_after_synced");
      }
    }
  }

  function applyConversationMessageAppended(payload?: Record<string, any> | null) {
    const conversationId = String(payload?.conversationId || "").trim();
    const message = payload?.message || null;
    const messageId = String(message?.id || "").trim();
    if (!conversationId || !message || !messageId) return;

    const cachedDisplay = freezeConversationMessages(bindings.conversationMessageCache.value[conversationId] || []);
    const cachedFormal = formalizeConversationMessages(cachedDisplay);
    const messageAlreadyCached = cachedFormal.some((item) => String(item?.id || "").trim() === messageId);
    const nextCached = messageAlreadyCached
      ? cachedFormal
      : [...cachedFormal, message];
    cacheConversationMessages(conversationId, nextCached);

    const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (conversationId !== currentConversationId) {
      if (!messageAlreadyCached) {
        applyConversationOverviewAppendedMessage(conversationId, message);
      }
      setConversationBadge(conversationId, "completed");
      return;
    }

    const existing = bindings.allMessages.value.filter((item: any) => String(item?.id || "").trim() !== messageId);
    const stableMessage = reuseStableMessageReferences([message], bindings.allMessages.value)[0] || message;
    bindings.allMessages.value = [...existing, stableMessage];
    bindings.foregroundTailLatestReady.value = true;
    clearConversationBadge(conversationId);
    updateForegroundConversationOverviewFromMessages(conversationId, message);
  }

  function applyConversationSnapshot(snapshot: Record<string, any>) {
    const nextConversationId = String(snapshot.conversationId || "").trim();
    const detachedConversationId = String(bindings.detachedChatConversationId.value || "").trim();
    if (
      bindings.detachedChatWindow.value
      && detachedConversationId
      && nextConversationId
      && nextConversationId !== detachedConversationId
    ) {
      console.warn("[独立窗口] 跳过非绑定会话快照", {
        windowLabel: bindings.tauriWindowLabel.value,
        detachedConversationId,
        snapshotConversationId: nextConversationId,
      });
      return;
    }
    const previousMessages = Array.isArray(bindings.allMessages.value) ? bindings.allMessages.value : [];
    const rawNextMessages = freezeConversationMessages(Array.isArray(snapshot.messages) ? snapshot.messages : []);
    const nextRuntimeState = String(snapshot.runtimeState || "").trim();
    const hasAssistantDraftInSnapshot = rawNextMessages.some((message) => isAssistantDraftMessage(message));
    if (!hasAssistantDraftInSnapshot && nextRuntimeState === "assistant_streaming") {
      const preservedDraft = [...previousMessages].reverse().find((message) => isAssistantDraftMessage(message));
      if (preservedDraft) {
        rawNextMessages.push(preservedDraft);
      }
    }
    const nextMessages = reuseStableMessageReferences(rawNextMessages, bindings.allMessages.value);
    bindings.currentChatConversationId.value = nextConversationId;
    bindings.currentChatTodos.value = Array.isArray(snapshot.currentTodos)
      ? snapshot.currentTodos
        .map((item: any) => ({
          content: String(item?.content || "").trim(),
          status: String(item?.status || "").trim(),
        }))
        .filter((item: any) => item.content && (item.status === "pending" || item.status === "in_progress" || item.status === "completed"))
      : [];
    bindings.allMessages.value = nextMessages;
    bindings.hasMoreBackendHistory.value = !!snapshot.hasMoreHistory;
    bindings.foregroundTailLatestReady.value = true;
    cacheConversationMessages(nextConversationId, nextMessages);
    clearConversationBadge(nextConversationId);
    if (Array.isArray(snapshot.unarchivedConversations)) {
      bindings.unarchivedConversations.value = snapshot.unarchivedConversations;
    }
    if (nextRuntimeState === "assistant_streaming") {
      maybeResumeForegroundStreamingDraft(nextConversationId, "apply_snapshot");
      void resumeForegroundRuntimeFromBackend(nextConversationId, "apply_snapshot");
    }
  }

  function applyConversationTodosUpdated(payload?: Record<string, any> | null) {
    const conversationId = String(payload?.conversationId || "").trim();
    if (!conversationId) return;
    const nextTodos = Array.isArray(payload?.currentTodos)
      ? payload.currentTodos
        .map((item: any) => ({
          content: String(item?.content || "").trim(),
          status: String(item?.status || "").trim(),
        }))
        .filter((item: any) => item.content && (item.status === "pending" || item.status === "in_progress" || item.status === "completed"))
      : [];
    if (conversationId === String(bindings.currentChatConversationId.value || "").trim()) {
      bindings.currentChatTodos.value = nextTodos;
    }
    const nextCurrentTodo = String(payload?.currentTodo || "").trim();
    bindings.unarchivedConversations.value = bindings.unarchivedConversations.value.map((item: any) =>
      String(item.conversationId || "").trim() === conversationId
        ? {
          ...item,
          currentTodo: nextCurrentTodo,
          currentTodos: nextTodos,
        }
        : item
    );
  }

  function applyConversationOverviewUpdated(payload?: Record<string, any> | null) {
    if (!Array.isArray(payload?.unarchivedConversations)) return;
    bindings.unarchivedConversations.value = payload.unarchivedConversations;
  }

  function applyConversationPinUpdated(payload?: Record<string, any> | null) {
    const conversationId = String(payload?.conversationId || "").trim();
    if (!conversationId) return;
    const isPinned = !!payload?.isPinned;
    const pinIndex = Number.isFinite(Number(payload?.pinIndex)) ? Number(payload?.pinIndex) : undefined;
    let changed = false;
    const nextItems = bindings.unarchivedConversations.value.map((item: any) => {
      if (String(item.conversationId || "").trim() !== conversationId) {
        return item;
      }
      changed = true;
      return {
        ...item,
        isPinned,
        pinIndex,
      };
    });
    if (!changed) return;
    bindings.unarchivedConversations.value = sortUnarchivedConversationOverviewItems(nextItems);
  }

  function applyConversationRuntimeStateUpdated(payload?: Record<string, any> | null) {
    const conversationId = String(payload?.conversationId || "").trim();
    const runtimeState = String(payload?.runtimeState || "").trim();
    if (!conversationId) return;
    if (runtimeState !== "idle" && runtimeState !== "assistant_streaming" && runtimeState !== "organizing_context") {
      return;
    }
    let changed = false;
    const nextItems = bindings.unarchivedConversations.value.map((item: any) => {
      if (String(item.conversationId || "").trim() !== conversationId) {
        return item;
      }
      if (item.runtimeState === runtimeState) {
        return item;
      }
      changed = true;
      return {
        ...item,
        runtimeState,
      };
    });
    if (!changed) return;
    bindings.unarchivedConversations.value = nextItems;
  }

  function updateForegroundConversationOverviewFromMessages(conversationId: string, assistantMessage?: any) {
    const cid = String(conversationId || "").trim();
    if (!cid) return;
    const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (cid !== currentConversationId) return;
    const formalMessages = bindings.allMessages.value.filter((message: any) => !isOverviewDraftMessage(message));
    const nextMessages = assistantMessage
      ? [...formalMessages, assistantMessage].filter((message, index, items) => {
        const messageId = String(message?.id || "").trim();
        if (!messageId) return true;
        return items.findIndex((item) => String(item?.id || "").trim() === messageId) === index;
      })
      : formalMessages;
    const previewMessages = nextMessages.slice(-2).map(previewMessageFromChatMessage);
    const lastMessage = nextMessages[nextMessages.length - 1];
    const lastMessageAt = String(lastMessage?.createdAt || "").trim();
    let changed = false;
    const nextItems = bindings.unarchivedConversations.value.map((item: any) => {
      if (String(item.conversationId || "").trim() !== cid) {
        return item;
      }
      changed = true;
      return {
        ...item,
        messageCount: nextMessages.length,
        updatedAt: lastMessageAt || item.updatedAt,
        lastMessageAt: lastMessageAt || item.lastMessageAt,
        previewMessages,
      };
    });
    if (changed) {
      bindings.unarchivedConversations.value = sortUnarchivedConversationOverviewItems(nextItems);
    }
  }

  function maybeUpdateForegroundConversationOverviewFromLoadedMessages(
    conversationId: string,
    messages: any[],
    remainingCount: number,
  ) {
    const cid = String(conversationId || "").trim();
    if (!cid) return;
    const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (cid !== currentConversationId) return;
    const formalMessages = (Array.isArray(messages) ? messages : [])
      .filter((message) => !isOverviewDraftMessage(message));
    const requiredPreviewCount = Math.min(2, Math.max(0, Number(remainingCount) || 0));
    if (requiredPreviewCount <= 0) return;
    if (formalMessages.length < requiredPreviewCount) return;
    const previewMessages = formalMessages
      .slice(-requiredPreviewCount)
      .map(previewMessageFromChatMessage);
    const lastMessage = formalMessages[formalMessages.length - 1];
    const lastMessageAt = String(lastMessage?.createdAt || "").trim();
    let changed = false;
    const nextItems = bindings.unarchivedConversations.value.map((item: any) => {
      if (String(item.conversationId || "").trim() !== cid) {
        return item;
      }
      changed = true;
      return {
        ...item,
        messageCount: Math.max(0, Number(remainingCount) || formalMessages.length),
        updatedAt: lastMessageAt || item.updatedAt,
        lastMessageAt: lastMessageAt || item.lastMessageAt,
        previewMessages,
      };
    });
    if (changed) {
      bindings.unarchivedConversations.value = sortUnarchivedConversationOverviewItems(nextItems);
    }
  }

  return {
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
  };
}

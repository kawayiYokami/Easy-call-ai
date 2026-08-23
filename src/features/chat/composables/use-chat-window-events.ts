import { onBeforeUnmount, onMounted } from "vue";
import { onTransportNotification } from "../../../services/tauri-api";
import { chatFlowRuntimesForConversation } from "./chat-flow-runtime-registry";

/**
 * 窗口级事件胶水只负责把非流式会话通知交给当前运行时。
 * 流式 history/round/delta/rebind 事件由 useChatFlow 自己订阅，确保所有窗口
 * 共用同一条消息状态机，不再维护第二套事件处理器。
 */
export function useChatWindowEvents(bindings: Record<string, any>) {
  function flowsForConversation(conversationId: string) {
    const normalizedConversationId = String(conversationId || "").trim();
    const registered = chatFlowRuntimesForConversation(normalizedConversationId);
    if (registered.length > 0) return registered;
    if (!normalizedConversationId || bindings.matchesForegroundConversation(normalizedConversationId)) {
      const fallback = bindings.getChatFlow?.();
      return fallback ? [fallback] : [];
    }
    return [];
  }

  const subscriptions: Array<{ key: string; stop: () => void }> = [];

  function subscribe<T>(key: string, method: string, handler: (payload: T) => void) {
    const stop = onTransportNotification<T>(method, handler);
    subscriptions.push({ key, stop });
    bindings.unlisteners[key] = stop;
  }

  onMounted(() => {
    subscribe<any>("chatHistoryFlushed", "chat.historyFlushed", (payload) => {
      const conversationId = bindings.readConversationIdFromPayload(payload);
      if (!conversationId || flowsForConversation(conversationId).length > 0) return;
      bindings.mergeIncomingMessagesIntoCache(
        conversationId,
        bindings.readMessagesFromPayload(payload),
      );
    });

    subscribe<any>("chatRoundCompleted", "chat.roundFinished", (payload) => {
      const conversationId = bindings.readConversationIdFromPayload(payload);
      const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
      const value = payload && typeof payload === "object" ? payload as Record<string, any> : null;
      const failed = String(value?.status || "").trim() === "failed" || !!String(value?.error || "").trim();
      const targetFlows = flowsForConversation(conversationId);
      if (targetFlows.length === 0 && conversationId && conversationId !== currentConversationId) {
        if (failed) {
          bindings.setConversationBadge(conversationId, "failed");
          return;
        }
        const assistantMessage = value?.assistantMessage || null;
        if (assistantMessage && String(assistantMessage?.id || "").trim()) {
          const cachedMessages = bindings.formalizeConversationMessages(
            bindings.conversationMessageCache.value[conversationId] || [],
          );
          bindings.cacheConversationMessages(
            conversationId,
            bindings.mergeMessagesIntoTimeline(cachedMessages, [assistantMessage]),
          );
        }
        return;
      }
      if (conversationId !== currentConversationId) return;
      bindings.clearConversationBadge(conversationId);
      if (failed) return;
      bindings.toolReviewRefreshTick.value += 1;
      queueMicrotask(() => {
        void bindings.refreshActiveGoalTask({ silent: true });
      });
    });

    subscribe<any>("chatConversationTodosUpdated", "conversation.todosUpdated", (payload) => {
      const conversationId = bindings.readConversationIdFromPayload(payload);
      for (const flow of flowsForConversation(conversationId)) {
        void flow.handleExternalTodosUpdated?.(payload);
      }
      bindings.applyConversationTodosUpdated(payload);
    });

    subscribe<any>("chatConversationPinUpdated", "conversation.pinUpdated", (payload) => {
      bindings.applyConversationPinUpdated(payload);
    });

    subscribe<any>("chatConversationGoalUpdated", "conversation.goalUpdated", (payload) => {
      bindings.applyConversationGoalUpdated(payload);
    });

    subscribe<any>("chatConversationRuntimeStateUpdated", "conversation.runtimeStateUpdated", (payload) => {
      const conversationId = bindings.readConversationIdFromPayload(payload);
      for (const flow of flowsForConversation(conversationId)) {
        void flow.handleExternalRuntimeStateUpdated?.(payload);
      }
      bindings.applyConversationRuntimeStateUpdated(payload);
    });

    subscribe<any>("chatConversationOverviewUpdated", "conversation.overviewUpdated", (payload) => {
      bindings.applyConversationOverviewUpdated(payload);
    });

    subscribe<any>("chatConversationOverviewItemUpdated", "conversation.overviewItemUpdated", (payload) => {
      bindings.applyConversationOverviewItemUpdated(payload);
    });

    subscribe<any>("chatConversationMessagesAfterSynced", "conversation.messagesAfterSynced", (payload) => {
      const conversationId = bindings.readConversationIdFromPayload(payload);
      for (const flow of flowsForConversation(conversationId)) {
        void flow.handleExternalMessagesAfterSynced?.(payload);
      }
      void bindings.applyConversationMessagesAfterSynced(payload);
    });

    subscribe<any>("chatConversationMessageAppended", "conversation.messageAppended", (payload) => {
      const conversationId = bindings.readConversationIdFromPayload(payload);
      for (const flow of flowsForConversation(conversationId)) {
        void flow.handleExternalMessageAppended?.(payload);
      }
      bindings.applyConversationMessageAppended(payload);
    });

    bindings.scheduleChatWindowActiveStateSync("mounted");
    bindings.startGoalTaskPolling();
    void bindings.refreshActiveGoalTask({ silent: true });
    window.addEventListener("focus", bindings.handleWindowFocusForStateSync);
    window.addEventListener("blur", bindings.handleWindowBlurForStateSync);
    document.addEventListener("visibilitychange", bindings.handleVisibilityForStateSync);
    window.addEventListener("focus", bindings.handleWindowFocusForMicPrewarm);
    document.addEventListener("visibilitychange", bindings.handleVisibilityForMicPrewarm);
  });

  onBeforeUnmount(() => {
    for (const { key, stop } of subscriptions.splice(0)) {
      stop();
      if (bindings.unlisteners[key] === stop) bindings.unlisteners[key] = null;
    }
    window.removeEventListener("focus", bindings.handleWindowFocusForStateSync);
    window.removeEventListener("blur", bindings.handleWindowBlurForStateSync);
    document.removeEventListener("visibilitychange", bindings.handleVisibilityForStateSync);
    bindings.clearChatWindowActiveSyncTimer();
    bindings.clearChatMicPrewarmTimer();
    bindings.clearGoalTaskPollTimer();
    bindings.cleanupChatForegroundActivity();
    bindings.agentWorkPresence.cleanup();
    void bindings.getChatFlow()?.unbindActiveConversationStream?.().catch(() => {});
    window.removeEventListener("focus", bindings.handleWindowFocusForMicPrewarm);
    document.removeEventListener("visibilitychange", bindings.handleVisibilityForMicPrewarm);
    bindings.cancelPendingRewindConfirm();
  });
}

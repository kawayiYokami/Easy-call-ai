import { onBeforeUnmount, onMounted } from "vue";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invokeTauri } from "../../../services/tauri-api";

export function useChatWindowEvents(bindings: Record<string, any>) {
  onMounted(() => {
    try {
      const label = String(getCurrentWindow().label || "").trim();
      bindings.tauriWindowLabel.value = label || "unknown";
      bindings.detachedChatWindow.value = bindings.tauriWindowLabel.value.startsWith("chat-detached-");
      bindings.isChatTauriWindow.value = bindings.tauriWindowLabel.value === "chat" || bindings.detachedChatWindow.value;
      if (bindings.detachedChatWindow.value) {
        bindings.sideConversationListVisible.value = false;
        bindings.toolReviewPanelOpenVisible.value = false;
      }
    } catch {
      bindings.tauriWindowLabel.value = "unknown";
      bindings.detachedChatWindow.value = false;
      bindings.isChatTauriWindow.value = false;
    }
    if (bindings.isChatTauriWindow.value) {
      void listen<any>("easy-call:history-flushed", (event) => {
        const payloadConversationId = bindings.readConversationIdFromPayload(event.payload);
        if (bindings.matchesForegroundConversation(payloadConversationId)) {
          void bindings.getChatFlow().handleExternalHistoryFlushed(event.payload);
        } else if (payloadConversationId) {
          bindings.mergeIncomingMessagesIntoCache(payloadConversationId, bindings.readMessagesFromPayload(event.payload));
          bindings.setConversationBadge(payloadConversationId, "completed");
        }
      }).then((unlisten) => {
        bindings.unlisteners.chatHistoryFlushed = unlisten;
      }).catch((error) => {
        console.error("[聊天追踪][历史刷写] 监听器注册失败", error);
      });

      void listen<any>("easy-call:round-completed", (event) => {
        const payloadConversationId = bindings.readConversationIdFromPayload(event.payload);
        const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
        const payloadObject = event.payload && typeof event.payload === "object"
          ? event.payload
          : null;
        const assistantMessage = payloadObject?.assistantMessage || null;
        if (payloadConversationId && payloadConversationId !== currentConversationId) {
          const assistantMessageId = String(assistantMessage?.id || "").trim();
          const cachedMessages = bindings.formalizeConversationMessages(bindings.conversationMessageCache.value[payloadConversationId] || []);
          const messageAlreadyCached = !!assistantMessageId && cachedMessages.some((message: any) => String(message?.id || "").trim() === assistantMessageId);
          if (assistantMessage && assistantMessageId && !messageAlreadyCached) {
            bindings.cacheConversationMessages(payloadConversationId, [...cachedMessages, assistantMessage]);
          }
          if (!assistantMessageId || !messageAlreadyCached) {
            if (assistantMessage && assistantMessageId) {
              bindings.applyConversationOverviewAppendedMessage(payloadConversationId, assistantMessage);
            }
          }
          bindings.setConversationBadge(payloadConversationId, "completed");
          void bindings.getChatFlow().handleExternalRoundCompleted(event.payload);
          return;
        }
        if (!bindings.matchesForegroundConversation(payloadConversationId)) return;
        bindings.clearConversationBadge(payloadConversationId);
        bindings.toolReviewRefreshTick.value += 1;
        bindings.updateForegroundConversationOverviewFromMessages(payloadConversationId || currentConversationId, assistantMessage);
        void bindings.getChatFlow().handleExternalRoundCompleted(event.payload);
      }).then((unlisten) => {
        bindings.unlisteners.chatRoundCompleted = unlisten;
      }).catch((error) => {
        console.error("[聊天追踪][轮次完成] 监听器注册失败", error);
      });

      void listen<any>("easy-call:round-started", (event) => {
        const payloadConversationId = bindings.readConversationIdFromPayload(event.payload);
        if (!bindings.matchesForegroundConversation(payloadConversationId)) return;
        void bindings.getChatFlow().handleExternalRoundStarted(event.payload);
      }).then((unlisten) => {
        bindings.unlisteners.chatRoundStarted = unlisten;
      }).catch((error) => {
        console.error("[聊天追踪][轮次开始] 监听器注册失败", error);
      });

      void listen<any>("easy-call:round-failed", (event) => {
        const payloadConversationId = bindings.readConversationIdFromPayload(event.payload);
        const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
        if (payloadConversationId && payloadConversationId !== currentConversationId) {
          bindings.setConversationBadge(payloadConversationId, "failed");
          void bindings.getChatFlow().handleExternalRoundFailed(event.payload);
          return;
        }
        if (!bindings.matchesForegroundConversation(payloadConversationId)) return;
        bindings.clearConversationBadge(payloadConversationId);
        void bindings.getChatFlow().handleExternalRoundFailed(event.payload);
      }).then((unlisten) => {
        bindings.unlisteners.chatRoundFailed = unlisten;
      }).catch((error) => {
        console.error("[聊天追踪][轮次失败] 监听器注册失败", error);
      });

      void listen<any>("easy-call:conversation-todos-updated", (event) => {
        bindings.applyConversationTodosUpdated(event.payload);
      }).then((unlisten) => {
        bindings.unlisteners.chatConversationTodosUpdated = unlisten;
      }).catch((error) => {
        console.error("[Todo] 监听器注册失败", error);
      });

      void listen<any>("easy-call:conversation-pin-updated", (event) => {
        bindings.applyConversationPinUpdated(event.payload);
      }).then((unlisten) => {
        bindings.unlisteners.chatConversationPinUpdated = unlisten;
      }).catch((error) => {
        console.error("[会话置顶] 监听器注册失败", error);
      });

      void listen<any>("easy-call:conversation-runtime-state-updated", (event) => {
        bindings.applyConversationRuntimeStateUpdated(event.payload);
      }).then((unlisten) => {
        bindings.unlisteners.chatConversationRuntimeStateUpdated = unlisten;
      }).catch((error) => {
        console.error("[会话运行态] 监听器注册失败", error);
      });

      void listen<any>("easy-call:conversation-overview-updated", (event) => {
        bindings.applyConversationOverviewUpdated(event.payload);
      }).then((unlisten) => {
        bindings.unlisteners.chatConversationOverviewUpdated = unlisten;
      }).catch((error) => {
        console.error("[会话概览] 监听器注册失败", error);
      });

      void listen<any>("easy-call:assistant-delta", (event) => {
        const conversationId = bindings.readConversationIdFromPayload(event.payload);
        if (bindings.CHAT_STREAM_DEBUG) {
          console.debug("[聊天流式重绑][前端] 收到助手增量普通事件", {
            conversationId,
            currentConversationId: String(bindings.currentChatConversationId.value || "").trim(),
          });
        }
        void bindings.getChatFlow().handleExternalAssistantDelta(event.payload);
      }).then((unlisten) => {
        bindings.unlisteners.chatAssistantDelta = unlisten;
      }).catch((error) => {
        console.error("[聊天追踪][助手增量] 监听器注册失败", error);
      });

      void listen<any>("easy-call:stream-rebind-required", (event) => {
        const conversationId = bindings.readConversationIdFromPayload(event.payload);
        if (bindings.CHAT_STREAM_DEBUG) {
          console.debug("[聊天流式重绑][前端] 收到重绑普通事件", {
            conversationId,
            currentConversationId: String(bindings.currentChatConversationId.value || "").trim(),
            payload: event.payload,
          });
        }
        void bindings.getChatFlow().handleExternalStreamRebindRequired(event.payload).catch((error: unknown) => {
          console.error("[聊天追踪][流重绑] 处理失败", { conversationId, error });
        });
      }).then((unlisten) => {
        bindings.unlisteners.chatStreamRebindRequired = unlisten;
      }).catch((error) => {
        console.error("[聊天追踪][流重绑] 监听器注册失败", error);
      });

      void listen<any>("easy-call:conversation-messages-after-synced", (event) => {
        void bindings.applyConversationMessagesAfterSynced(event.payload);
      }).then((unlisten) => {
        bindings.unlisteners.chatConversationMessagesAfterSynced = unlisten;
      }).catch((error) => {
        console.error("[聊天追踪][异步补消息] 监听器注册失败", error);
      });

      void listen<any>("easy-call:conversation-message-appended", (event) => {
        bindings.applyConversationMessageAppended(event.payload);
      }).then((unlisten) => {
        bindings.unlisteners.chatConversationMessageAppended = unlisten;
      }).catch((error) => {
        console.error("[聊天追踪][追加消息] 监听器注册失败", error);
      });
    }

    bindings.scheduleChatWindowActiveStateSync("mounted");
    bindings.startSupervisionTaskPolling();
    void bindings.refreshActiveSupervisionTask({ silent: true });
    window.addEventListener("focus", bindings.handleWindowFocusForStateSync);
    window.addEventListener("blur", bindings.handleWindowBlurForStateSync);
    document.addEventListener("visibilitychange", bindings.handleVisibilityForStateSync);
    window.addEventListener("focus", bindings.handleWindowFocusForMicPrewarm);
    document.addEventListener("visibilitychange", bindings.handleVisibilityForMicPrewarm);
  });

  onBeforeUnmount(() => {
    if (bindings.unlisteners.chatHistoryFlushed) {
      bindings.unlisteners.chatHistoryFlushed();
      bindings.unlisteners.chatHistoryFlushed = null;
    }
    if (bindings.unlisteners.chatRoundCompleted) {
      bindings.unlisteners.chatRoundCompleted();
      bindings.unlisteners.chatRoundCompleted = null;
    }
    if (bindings.unlisteners.chatRoundStarted) {
      bindings.unlisteners.chatRoundStarted();
      bindings.unlisteners.chatRoundStarted = null;
    }
    if (bindings.unlisteners.chatRoundFailed) {
      bindings.unlisteners.chatRoundFailed();
      bindings.unlisteners.chatRoundFailed = null;
    }
    if (bindings.unlisteners.chatAssistantDelta) {
      bindings.unlisteners.chatAssistantDelta();
      bindings.unlisteners.chatAssistantDelta = null;
    }
    if (bindings.unlisteners.chatStreamRebindRequired) {
      bindings.unlisteners.chatStreamRebindRequired();
      bindings.unlisteners.chatStreamRebindRequired = null;
    }
    if (bindings.unlisteners.chatConversationMessagesAfterSynced) {
      bindings.unlisteners.chatConversationMessagesAfterSynced();
      bindings.unlisteners.chatConversationMessagesAfterSynced = null;
    }
    if (bindings.unlisteners.chatConversationMessageAppended) {
      bindings.unlisteners.chatConversationMessageAppended();
      bindings.unlisteners.chatConversationMessageAppended = null;
    }
    if (bindings.unlisteners.chatConversationTodosUpdated) {
      bindings.unlisteners.chatConversationTodosUpdated();
      bindings.unlisteners.chatConversationTodosUpdated = null;
    }
    if (bindings.unlisteners.chatConversationPinUpdated) {
      bindings.unlisteners.chatConversationPinUpdated();
      bindings.unlisteners.chatConversationPinUpdated = null;
    }
    if (bindings.unlisteners.chatConversationRuntimeStateUpdated) {
      bindings.unlisteners.chatConversationRuntimeStateUpdated();
      bindings.unlisteners.chatConversationRuntimeStateUpdated = null;
    }
    if (bindings.unlisteners.chatConversationOverviewUpdated) {
      bindings.unlisteners.chatConversationOverviewUpdated();
      bindings.unlisteners.chatConversationOverviewUpdated = null;
    }
    window.removeEventListener("focus", bindings.handleWindowFocusForStateSync);
    window.removeEventListener("blur", bindings.handleWindowBlurForStateSync);
    document.removeEventListener("visibilitychange", bindings.handleVisibilityForStateSync);
    bindings.clearChatWindowActiveSyncTimer();
    bindings.clearChatMicPrewarmTimer();
    bindings.clearSupervisionTaskPollTimer();
    bindings.clearForegroundConversationCacheRaf();
    bindings.clearRecordHotkeyProbeState();
    bindings.agentWorkPresence.cleanup();
    bindings.chatWindowActiveSynced.value = null;
    if (bindings.isPrimaryChatWindow()) {
      void invokeTauri("set_chat_window_active", { active: false }).catch(() => {});
    }
    window.removeEventListener("focus", bindings.handleWindowFocusForMicPrewarm);
    document.removeEventListener("visibilitychange", bindings.handleVisibilityForMicPrewarm);
    bindings.cancelPendingRewindConfirm();
  });
}

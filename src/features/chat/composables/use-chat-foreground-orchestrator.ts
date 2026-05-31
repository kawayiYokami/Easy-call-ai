import { nextTick } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invokeTauri } from "../../../services/tauri-api";

export function useChatForegroundOrchestrator(bindings: Record<string, any>) {
  async function requestConversationLightSnapshot(conversationId?: string | null) {
    return invokeTauri<any>("get_foreground_conversation_light_snapshot", {
      input: {
        conversationId: String(conversationId || "").trim() || null,
        agentId: String(bindings.currentForegroundAgentId.value || "").trim() || null,
        limit: bindings.FOREGROUND_SNAPSHOT_RECENT_LIMIT,
      },
    });
  }

  async function requestUnarchivedConversationOverview() {
    return invokeTauri<any[]>("list_unarchived_conversations");
  }

  async function refreshRemoteImConversationOverview() {
    bindings.remoteImContactConversations.value = await invokeTauri<any[]>("remote_im_list_contact_conversations");
  }

  async function refreshUnarchivedConversationOverview() {
    const items = await requestUnarchivedConversationOverview();
    bindings.unarchivedConversations.value = Array.isArray(items) ? items : [];
  }

  function pickForegroundConversationId(candidates: any[]): string {
    const target =
      candidates.find((item) => !!item.isActive)
      || candidates.find((item) => !!item.isMainConversation)
      || candidates[0];
    return String(target?.conversationId || "").trim();
  }

  function clearForegroundConversation(reason: string) {
    const previousConversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (!previousConversationId) return;
    bindings.cacheConversationMessages(previousConversationId, bindings.allMessages.value);
    bindings.currentChatConversationId.value = "";
    bindings.currentChatTodos.value = [];
    if (bindings.trimmingConversationId.value === previousConversationId) {
      bindings.trimmingConversationId.value = "";
      bindings.trimming.value = false;
    }
    if (bindings.compactingConversationId.value === previousConversationId) {
      bindings.compactingConversationId.value = "";
      bindings.compactingConversation.value = false;
    }
    bindings.allMessages.value = [];
    bindings.hasMoreBackendHistory.value = false;
    bindings.foregroundTailLatestReady.value = true;
    bindings.clearPendingManualScrollToBottom();
    bindings.getChatFlow().freezeForegroundRoundState();
    void reason;
  }

  async function recoverForegroundConversationFromOverview(reason: string, preferredConversationId?: string | null) {
    if (bindings.conversationForegroundSyncing.value) return;
    try {
      bindings.conversationForegroundSyncing.value = true;
      const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
      if (currentConversationId && bindings.unarchivedConversations.value.some((item: any) => String(item.conversationId || "").trim() === currentConversationId)) {
        return;
      }
      const nextConversationId = String(preferredConversationId || "").trim() || pickForegroundConversationId(bindings.unarchivedConversations.value);
      if (!nextConversationId) {
        clearForegroundConversation(reason);
        return;
      }
      const snapshot = await requestConversationLightSnapshot(nextConversationId);
      bindings.applyConversationSnapshot(snapshot);
    } finally {
      bindings.conversationForegroundSyncing.value = false;
    }
  }

  function syncCurrentConversationWorkspaceLabel() {
    const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (!currentConversationId) return;
    const nextLabel = String(bindings.chatWorkspaceName.value || "").trim() || "默认会话目录";
    let changed = false;
    const nextItems = bindings.unarchivedConversations.value.map((item: any) => {
      if (String(item.conversationId || "").trim() !== currentConversationId) {
        return item;
      }
      if (String(item.workspaceLabel || "").trim() === nextLabel) {
        return item;
      }
      changed = true;
      return {
        ...item,
        workspaceLabel: nextLabel,
      };
    });
    if (changed) {
      bindings.unarchivedConversations.value = nextItems;
    }
  }

  async function switchUnarchivedConversation(conversationId: string) {
    const cid = String(conversationId || "").trim();
    if (!cid) return;
    const targetOverview = bindings.unarchivedConversations.value.find((item: any) => String(item.conversationId || "").trim() === cid);
    if (targetOverview?.detachedWindowOpen) {
      try {
        await invokeTauri<boolean>("focus_detached_chat_window_by_conversation", {
          input: { conversationId: cid },
        });
      } catch (error) {
        console.warn("[独立聊天窗口] 聚焦已占用会话失败", error);
      }
      return;
    }
    const previousConversationId = String(bindings.currentChatConversationId.value || "").trim();
    const startedAt = bindings.perfNow();
    try {
      bindings.conversationForegroundSyncing.value = true;
      if (previousConversationId) {
        bindings.cacheConversationMessages(previousConversationId, bindings.allMessages.value);
        bindings.clearConversationBadge(previousConversationId);
        bindings.markConversationReadPersisted(previousConversationId);
      }
      bindings.getChatFlow().freezeForegroundRoundState();
      bindings.clearPendingManualScrollToBottom();
      bindings.foregroundTailLatestReady.value = false;
      const trace = bindings.beginForegroundPaintTrace(cid);
      const snapshot = await requestConversationLightSnapshot(cid);
      bindings.applyConversationSnapshot(snapshot);
      await bindings.resumeForegroundRuntimeFromBackend(cid, "switch_snapshot");
      bindings.clearConversationBadge(cid);
      bindings.markConversationReadPersisted(cid);
      await nextTick();
      bindings.logForegroundPaintTrace(trace, "前台轻量快照已接管最新消息", {
        conversationId: cid,
        snapshotCount: Array.isArray(snapshot?.messages) ? snapshot.messages.length : 0,
        hasMoreHistory: !!snapshot?.hasMoreHistory,
        fromConversationId: previousConversationId,
        syncCostMs: Math.round((bindings.perfNow() - startedAt) * 10) / 10,
      });
    } catch (error) {
      bindings.setStatusError("status.loadMessagesFailed", error);
    } finally {
      bindings.conversationForegroundSyncing.value = false;
    }
  }

  async function ensureLatestForegroundTailThenScrollToBottom() {
    const conversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (!conversationId) return;
    if (bindings.foregroundTailLatestReady.value) {
      bindings.triggerConversationScrollToBottom(conversationId, "manual_ready");
      return;
    }
    try {
      const result = await invokeTauri<{ accepted: boolean; requestId: string }>("request_conversation_messages_after_async", {
        input: {
          conversationId,
          afterMessageId: bindings.buildConversationMessagesAfterAnchor(conversationId),
          fallbackLimit: bindings.BACKGROUND_CONVERSATION_CACHE_LIMIT,
        },
      });
      if (!result?.accepted) {
        bindings.triggerConversationScrollToBottom(conversationId, "manual_request_rejected");
        return;
      }
      bindings.setPendingManualScrollState(conversationId, String(result.requestId || "").trim());
      if (!String(result.requestId || "").trim()) {
        bindings.triggerConversationScrollToBottom(conversationId, "manual_request_missing_id");
      }
    } catch (error) {
      console.warn("[会话切换] 手动滚到底前请求尾部增量失败", {
        conversationId,
        error,
      });
      bindings.triggerConversationScrollToBottom(conversationId, "manual_request_failed");
    }
  }

  async function refreshChatUnarchivedConversations() {
    if (bindings.conversationForegroundSyncing.value) return;
    if (bindings.detachedChatWindow.value) {
      await refreshRemoteImConversationOverview();
      return;
    }
    try {
      bindings.conversationForegroundSyncing.value = true;
      await refreshUnarchivedConversationOverview();
      await refreshRemoteImConversationOverview();
      const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
      const nextConversationId = currentConversationId && bindings.unarchivedConversations.value.some((item: any) =>
        String(item.conversationId || "").trim() === currentConversationId
      )
        ? currentConversationId
        : pickForegroundConversationId(bindings.unarchivedConversations.value);
      if (!nextConversationId) {
        clearForegroundConversation("refresh_unarchived_conversations_empty");
        return;
      }
      const snapshot = await requestConversationLightSnapshot(nextConversationId);
      bindings.applyConversationSnapshot(snapshot);
    } finally {
      bindings.conversationForegroundSyncing.value = false;
    }
  }

  async function initializeDetachedChatWindow() {
    if (!bindings.detachedChatWindow.value) return;
    try {
      const info = await invokeTauri<any>("get_detached_chat_window_info");
      const conversationId = String(info?.conversationId || "").trim();
      if (!info?.detached || !conversationId) {
        bindings.setStatus("独立聊天窗口缺少绑定会话，窗口即将关闭。");
        try {
          await getCurrentWindow().close();
        } catch (closeError) {
          console.error("[独立聊天窗口] 缺少绑定会话时关闭窗口失败", closeError);
          bindings.setStatusError("status.requestFailed", closeError);
        }
        return;
      }
      bindings.detachedChatConversationId.value = conversationId;
      bindings.currentChatConversationId.value = conversationId;
      bindings.sideConversationListVisible.value = false;
      await refreshRemoteImConversationOverview();
      const snapshot = await requestConversationLightSnapshot(conversationId);
      bindings.applyConversationSnapshot(snapshot);
      await nextTick();
      bindings.maybeResumeForegroundStreamingDraft(conversationId, "detached_window_init");
      await bindings.resumeForegroundRuntimeFromBackend(conversationId, "detached_window_init");
    } catch (error) {
      bindings.setStatusError("status.loadMessagesFailed", error);
    }
  }

  async function handleCloseWindow() {
    if (bindings.detachedChatWindow.value) {
      await getCurrentWindow().close();
      return;
    }
    bindings.freezeForegroundConversation("close_window");
    await bindings.closeWindow();
  }

  async function detachCurrentConversationToWindow() {
    console.info("[独立聊天窗口][前端链路] ChatWindowApp 进入 detachCurrentConversationToWindow", {
      windowLabel: bindings.tauriWindowLabel.value,
      detachedChatWindow: bindings.detachedChatWindow.value,
      currentConversationId: String(bindings.currentChatConversationId.value || "").trim(),
      chatting: bindings.chatting.value,
      trimming: bindings.trimming.value,
      compactingConversation: bindings.compactingConversation.value,
      isMainConversation: !!bindings.currentForegroundConversationSummary.value?.isMainConversation,
    });
    bindings.setStatus("正在打开独立聊天窗口...");
    if (bindings.detachedChatWindow.value) {
      console.warn("[独立聊天窗口][前端链路] 当前已经是独立窗口，忽略独立窗口请求");
      return;
    }
    const conversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (!conversationId || bindings.chatting.value || bindings.trimming.value || bindings.compactingConversation.value) {
      console.warn("[独立聊天窗口][前端链路] 当前状态不允许独立窗口", {
        conversationId,
        chatting: bindings.chatting.value,
        trimming: bindings.trimming.value,
        compactingConversation: bindings.compactingConversation.value,
      });
      return;
    }
    if (bindings.currentForegroundConversationSummary.value?.isMainConversation) {
      console.warn("[独立聊天窗口][前端链路] 主会话不允许独立窗口", { conversationId });
      bindings.setStatus("主会话不能独立打开，请选择一个子会话。");
      return;
    }
    try {
      console.info("[独立聊天窗口][前端链路] 准备 invoke detach_current_conversation_to_window", {
        conversationId,
      });
      void invokeTauri<{ conversationId: string; windowLabel: string; mainConversationId?: string | null }>("detach_current_conversation_to_window", {
        input: { conversationId },
      }).then((result) => {
        console.info("[独立聊天窗口][前端链路] invoke detach_current_conversation_to_window 已返回", result);
        void refreshUnarchivedConversationOverview();
      }).catch((error) => {
        console.error("[独立聊天窗口][前端链路] 打开独立窗口失败", error);
        bindings.setStatusError("status.loadMessagesFailed", error);
        void refreshUnarchivedConversationOverview();
      });
      clearForegroundConversation("detach_current_conversation");
      const mainConversationId = String(bindings.unarchivedConversations.value.find((item: any) => !!item.isMainConversation)?.conversationId || "").trim();
      if (mainConversationId) {
        await switchUnarchivedConversation(mainConversationId);
      } else {
        await refreshChatUnarchivedConversations();
      }
      bindings.setStatus("已发送独立聊天窗口请求");
    } catch (error) {
      console.error("[独立聊天窗口][前端链路] 打开独立窗口失败", error);
      bindings.setStatusError("status.loadMessagesFailed", error);
    }
  }

  async function sendChatFromCurrentWindow(overrides?: { extraTextBlocks?: string[] }) {
    if (bindings.detachedChatWindow.value) {
      const temporaryApiConfigId = String(bindings.detachedTemporaryApiConfigId.value || "").trim();
      if (temporaryApiConfigId && !bindings.config.apiConfigs.some((item: any) => item.id === temporaryApiConfigId && item.enableText)) {
        bindings.setStatus("临时模型已不可用，请重新选择模型。");
        return;
      }
    }
    await bindings.getChatFlow().sendChat(overrides);
  }

  function freezeForegroundConversation(reason: string) {
    const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (currentConversationId) {
      bindings.cacheConversationMessages(currentConversationId, bindings.allMessages.value);
    }
    bindings.getChatFlow().freezeForegroundRoundState();
    void reason;
  }

  function hasActiveForegroundConversation(conversationId?: string | null): boolean {
    if (!bindings.isChatWindowActiveNow()) return false;
    const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (!currentConversationId) return false;
    const targetConversationId = String(conversationId || "").trim();
    return !targetConversationId || targetConversationId === currentConversationId;
  }

  return {
    requestConversationLightSnapshot,
    requestUnarchivedConversationOverview,
    refreshRemoteImConversationOverview,
    refreshUnarchivedConversationOverview,
    pickForegroundConversationId,
    clearForegroundConversation,
    recoverForegroundConversationFromOverview,
    syncCurrentConversationWorkspaceLabel,
    switchUnarchivedConversation,
    ensureLatestForegroundTailThenScrollToBottom,
    refreshChatUnarchivedConversations,
    initializeDetachedChatWindow,
    handleCloseWindow,
    detachCurrentConversationToWindow,
    sendChatFromCurrentWindow,
    freezeForegroundConversation,
    hasActiveForegroundConversation,
  };
}

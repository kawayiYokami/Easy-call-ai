import { invokeTauri } from "../../../services/tauri-api";
import type { ChatMessage } from "../../../types/app";

export function useChatRemoteConversationOrchestrator(bindings: Record<string, any>) {
  async function requestRemoteImConversationMessages(contactId: string): Promise<ChatMessage[]> {
    return invokeTauri<ChatMessage[]>("remote_im_get_contact_conversation_messages", {
      input: { contactId },
    });
  }

  async function switchRemoteImContactConversation(contactId: string) {
    const normalizedContactId = String(contactId || "").trim();
    if (!normalizedContactId) return;
    const targetOverview = bindings.remoteImContactConversations.value.find((item: any) =>
      String(item.contactId || "").trim() === normalizedContactId,
    );
    const conversationId = String(targetOverview?.conversationId || "").trim();
    if (!conversationId) return;
    const previousConversationId = String(bindings.currentChatConversationId.value || "").trim();
    try {
      bindings.conversationForegroundSyncing.value = true;
      if (previousConversationId) {
        bindings.cacheConversationMessages(previousConversationId, bindings.allMessages.value);
        bindings.clearConversationBadge(previousConversationId);
        bindings.markConversationReadPersisted(previousConversationId);
      }
      bindings.getChatFlow().freezeForegroundRoundState();
      bindings.currentChatConversationId.value = conversationId;
      bindings.currentChatTodos.value = [];
      bindings.clearPendingManualScrollToBottom();
      const cachedDisplay = bindings.freezeConversationMessages(bindings.conversationMessageCache.value[conversationId] || []);
      bindings.allMessages.value = cachedDisplay;
      bindings.hasMoreBackendHistory.value = cachedDisplay.length >= bindings.FOREGROUND_SNAPSHOT_RECENT_LIMIT;
      bindings.foregroundTailLatestReady.value = true;
      const messages = await requestRemoteImConversationMessages(normalizedContactId);
      const nextMessages = bindings.reuseStableMessageReferences(
        bindings.freezeConversationMessages(Array.isArray(messages) ? messages : []),
        bindings.allMessages.value,
      );
      bindings.allMessages.value = nextMessages;
      bindings.cacheConversationMessages(conversationId, nextMessages);
      bindings.hasMoreBackendHistory.value = nextMessages.length >= bindings.FOREGROUND_SNAPSHOT_RECENT_LIMIT;
      bindings.clearConversationBadge(conversationId);
      bindings.markConversationReadPersisted(conversationId);
    } catch (error) {
      bindings.setStatusError("status.loadMessagesFailed", error);
    } finally {
      bindings.conversationForegroundSyncing.value = false;
    }
  }

  async function openConversationInDetachedWindowById(conversationId: string) {
    const cid = String(conversationId || "").trim();
    if (!cid) return;
    try {
      const focused = await invokeTauri<boolean>("focus_detached_chat_window_by_conversation", {
        input: { conversationId: cid },
      });
      if (focused) return;
    } catch (error) {
      console.warn("[独立聊天窗口] 聚焦已占用会话失败", {
        conversationId: cid,
        error,
      });
    }
    try {
      await invokeTauri<{ conversationId: string; windowLabel: string; mainConversationId?: string | null }>("detach_current_conversation_to_window", {
        input: { conversationId: cid },
      });
    } catch (error) {
      console.warn("[独立聊天窗口] 打开独立会话窗口失败", {
        conversationId: cid,
        error,
      });
    }
    await bindings.refreshUnarchivedConversationOverview();
    await bindings.refreshRemoteImConversationOverview();
  }

  async function switchChatConversation(payload: { kind?: string; conversationId: string; remoteContactId?: string }) {
    const kind = payload.kind === "remote_im_contact" ? "remote_im_contact" : "local_unarchived";
    if (kind === "remote_im_contact") {
      await openConversationInDetachedWindowById(payload.conversationId);
      return;
    }
    await bindings.switchUnarchivedConversation(payload.conversationId);
  }

  return {
    requestRemoteImConversationMessages,
    switchRemoteImContactConversation,
    openConversationInDetachedWindowById,
    switchChatConversation,
  };
}

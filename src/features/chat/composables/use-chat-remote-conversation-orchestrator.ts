import { invokeTauri } from "../../../services/tauri-api";

export function useChatRemoteConversationOrchestrator(bindings: Record<string, any>) {
  async function switchRemoteImContactConversation(contactId: string) {
    const normalizedContactId = String(contactId || "").trim();
    if (!normalizedContactId) return;
    const targetOverview = bindings.remoteImContactConversations.value.find((item: any) =>
      String(item.contactId || "").trim() === normalizedContactId,
    );
    const conversationId = String(targetOverview?.conversationId || "").trim();
    if (!conversationId) return;
    await bindings.switchUnarchivedConversation(conversationId);
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
      await invokeTauri<{ conversationId: string; windowLabel: string; systemNotificationConversationId?: string | null }>("detach_current_conversation_to_window", {
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
      const contactId = String(payload.remoteContactId || "").trim();
      if (contactId) {
        await switchRemoteImContactConversation(contactId);
      } else {
        await bindings.switchUnarchivedConversation(payload.conversationId);
      }
      return;
    }
    await bindings.switchUnarchivedConversation(payload.conversationId);
  }

  return {
    switchRemoteImContactConversation,
    openConversationInDetachedWindowById,
    switchChatConversation,
  };
}

import { getCurrentWindow } from "@tauri-apps/api/window";
import { invokeTauri } from "../../../services/tauri-api";

export function useChatConversationDialogGlue(bindings: Record<string, any>) {
  async function deleteUnarchivedConversationFromArchives(conversationId: string) {
    const normalizedConversationId = String(conversationId || "").trim();
    if (!normalizedConversationId) return;
    const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
    const deletingCurrentConversation = currentConversationId === normalizedConversationId;
    if (bindings.detachedChatWindow.value && deletingCurrentConversation) {
      void bindings.deleteUnarchivedConversationFromArchivesRaw(normalizedConversationId).catch((error: unknown) => {
        console.error("[独立聊天窗口] 后台删除会话失败", error);
      });
      await getCurrentWindow().close();
      return;
    }
    if (deletingCurrentConversation) {
      const optimisticNextConversationId = bindings.pickForegroundConversationId(
        bindings.unarchivedConversations.value.filter((item: any) => String(item.conversationId || "").trim() !== normalizedConversationId),
      );
      if (optimisticNextConversationId) {
        try {
          bindings.conversationForegroundSyncing.value = true;
          const snapshot = await bindings.requestConversationLightSnapshot(optimisticNextConversationId);
          bindings.applyConversationSnapshot({
            ...snapshot,
            unarchivedConversations: bindings.unarchivedConversations.value,
          });
        } finally {
          bindings.conversationForegroundSyncing.value = false;
        }
      } else {
        bindings.clearForegroundConversation("delete_unarchived_conversation_optimistic_empty");
      }
    }
    const result = await bindings.deleteUnarchivedConversationFromArchivesRaw(normalizedConversationId);
    if (!deletingCurrentConversation) return;
    if (String(bindings.currentChatConversationId.value || "").trim()) return;
    await bindings.recoverForegroundConversationFromOverview(
      "delete_unarchived_conversation",
      String(result?.activeConversationId || "").trim() || null,
    );
  }

  async function archiveConversationFromList(conversationId: string) {
    const normalizedConversationId = String(conversationId || "").trim();
    if (!normalizedConversationId) return;
    console.info("[会话归档] 点击归档会话", {
      conversationId: normalizedConversationId,
      source: "conversation_list",
    });
    try {
      await bindings.archiveCurrentConversation(normalizedConversationId);
    } catch (error) {
      console.warn("[会话归档] 归档会话失败", {
        conversationId: normalizedConversationId,
        error,
      });
      bindings.setStatusError("status.trimArchiveFailed", error);
    }
  }

  async function handleConfirmTrimAction() {
    if (!bindings.detachedChatWindow.value) {
      await bindings.getConfirmTrimAction()();
      return;
    }
    const conversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (!conversationId) {
      bindings.setStatus("当前没有可归档的会话。");
      bindings.getCloseTrimActionDialog()();
      return;
    }
    bindings.getCloseTrimActionDialog()();
    console.info("[会话归档] 点击归档会话", {
      conversationId,
      source: "detached_chat_window",
    });
    void invokeTauri("archive_conversation", {
      input: {
        conversationId,
      },
    }).catch((error) => {
      console.error("[独立聊天窗口] 后台归档会话失败", error);
    });
    await getCurrentWindow().close();
  }

  return {
    deleteUnarchivedConversationFromArchives,
    archiveConversationFromList,
    handleConfirmTrimAction,
  };
}

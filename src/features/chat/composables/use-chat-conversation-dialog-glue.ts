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
    if (normalizedConversationId !== String(bindings.currentChatConversationId.value || "").trim()) {
      await bindings.switchUnarchivedConversation(normalizedConversationId);
    }
    try {
      await bindings.archiveCurrentConversation();
    } catch (error) {
      bindings.setStatusError("status.trimArchiveFailed", error);
    }
  }

  async function handleConfirmTrimAction() {
    if (!bindings.detachedChatWindow.value) {
      await bindings.getConfirmTrimAction()();
      return;
    }
    const conversationId = String(bindings.currentChatConversationId.value || "").trim();
    const apiConfigId = String(bindings.currentForegroundApiConfigId.value || "").trim();
    const agentId = String(bindings.currentForegroundAgentId.value || "").trim();
    if (!conversationId || !apiConfigId || !agentId) {
      bindings.setStatus("当前没有可归档的会话。");
      bindings.getCloseTrimActionDialog()();
      return;
    }
    bindings.getCloseTrimActionDialog()();
    void invokeTauri("trim_current_conversation", {
      input: {
        session: {
          apiConfigId,
          agentId,
          conversationId,
        },
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

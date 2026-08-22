import { coordinateConversationDelete } from "./conversation-delete-coordinator";

export function useChatConversationDialogGlue(bindings: Record<string, any>) {
  async function deleteUnarchivedConversationFromArchives(conversationId: string) {
    const normalizedConversationId = String(conversationId || "").trim();
    if (!normalizedConversationId) return;
    await coordinateConversationDelete({
      conversationId: normalizedConversationId,
      currentConversationId: () => String(bindings.currentChatConversationId.value || "").trim(),
      deleteConversation: bindings.deleteUnarchivedConversationFromArchivesRaw,
      applyConversationList: (items) => {
        bindings.unarchivedConversations.value = items;
      },
      readConversationList: () => bindings.unarchivedConversations.value,
      syncConversationList: () => bindings.syncUnarchivedConversationOverviewChangedSinceWatermark?.("conversation_deleted"),
      conversationIds: () => bindings.unarchivedConversations.value
        .map((item: any) => String(item?.conversationId || "").trim()),
      clearCurrentConversation: bindings.clearForegroundConversation,
      openConversation: bindings.switchUnarchivedConversation,
      onNoReplacement: () => bindings.recoverForegroundConversationFromOverview(
        "conversation_delete_current_missing_replacement",
      ),
    });
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
    await bindings.getConfirmTrimAction()();
  }

  return {
    deleteUnarchivedConversationFromArchives,
    archiveConversationFromList,
    handleConfirmTrimAction,
  };
}

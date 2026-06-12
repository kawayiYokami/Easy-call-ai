import { watch } from "vue";

export function useChatRuntimeWatchers(bindings: Record<string, any>) {
  watch(
    () => ({
      mode: bindings.viewMode.value,
      departmentId: String(bindings.currentForegroundDepartmentId.value || "").trim(),
      agentId: String(bindings.currentForegroundAgentId.value || "").trim(),
    }),
    ({ mode }) => {
      if (mode !== "chat" || !bindings.startupDataReady.value) return;
      void bindings.refreshChatUnarchivedConversations().catch((error: unknown) => {
        bindings.setStatusError("status.loadMessagesFailed", error);
      });
    },
    { immediate: true },
  );

  watch(
    () => ({
      mode: bindings.viewMode.value,
      conversationId: String(bindings.currentChatConversationId.value || "").trim(),
    }),
    ({ mode, conversationId }) => {
      if (mode !== "chat" || !bindings.startupDataReady.value) return;
      void (async () => {
        try {
          await bindings.getChatFlow().bindActiveConversationStream(conversationId);
          bindings.maybeResumeForegroundStreamingDraft(conversationId, "bind_active_stream");
          await bindings.resumeForegroundRuntimeFromBackend(conversationId, "bind_active_stream");
        } catch (error) {
          console.warn("[聊天推送] 绑定前台流失败", {
            conversationId,
            error,
          });
        }
      })();
    },
    { immediate: true },
  );
}

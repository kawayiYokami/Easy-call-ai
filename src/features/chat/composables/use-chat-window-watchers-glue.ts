import { watch } from "vue";

export function useChatWindowWatchersGlue(bindings: Record<string, any>) {
  watch(
    () => bindings.viewMode.value,
    () => {
      bindings.scheduleChatWindowActiveStateSync("viewmode_changed");
    },
  );

  watch(
    () => bindings.currentChatConversationId.value,
    () => {
      bindings.handleSupervisionConversationChanged();
    },
    { immediate: true },
  );

  watch(
    () => ({
      apiId: bindings.currentForegroundApiConfigId.value,
      imageEnabled: !!bindings.currentForegroundApiConfig.value?.enableImage,
      visionEnabled: bindings.hasVisionFallback.value,
    }),
    () => {
      bindings.clearMatchingConversationChatErrors((text: string) =>
        text.includes("不支持图片附件") || text.includes("PDF 附件"),
      );
    },
  );

  watch(
    () => ({
      mode: bindings.viewMode.value,
      apiId: bindings.currentForegroundApiConfigId.value,
      agentId: bindings.currentForegroundAgentId.value,
      conversationId: bindings.currentChatConversationId.value,
      conversationIds: bindings.unarchivedConversations.value
        .map((item: any) => String(item.conversationId || "").trim())
        .join("|"),
    }),
    ({ mode }) => {
      if (mode !== "chat" || !bindings.startupDataReady.value) return;
      void bindings.refreshChatWorkspaceState();
    },
    { immediate: true },
  );

  watch(
    () => ({
      mode: bindings.viewMode.value,
      workspaceName: bindings.chatWorkspaceName.value,
    }),
    ({ mode }) => {
      if (mode !== "chat") return;
      bindings.syncCurrentConversationWorkspaceLabel();
    },
  );

  watch(
    () => bindings.viewMode.value,
    (mode) => {
      if (mode !== "chat") return;
      bindings.handleWindowFocusForMicPrewarm();
    },
  );

  watch(
    () => ({ uiFont: bindings.config.uiFont, uiLanguage: bindings.config.uiLanguage }),
    ({ uiFont, uiLanguage }) => {
      bindings.applyUiFont(uiFont, uiLanguage);
    },
    { immediate: true },
  );
}

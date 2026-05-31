import { computed, type Ref } from "vue";

type UseChatWindowBasicDerivedStateOptions<MessageBlock, TerminalApproval> = {
  t: (key: string, params?: Record<string, unknown>) => string;
  viewMode: Ref<"chat" | "archives" | "config">;
  maximized: Ref<boolean>;
  detachedChatWindow: Ref<boolean>;
  trimming: Ref<boolean>;
  compactingConversation: Ref<boolean>;
  currentForegroundPersona: Ref<{ name?: string } | null>;
  currentChatConversationId: Ref<string>;
  visibleMessageBlocks: Ref<MessageBlock[]>;
  listConversationTerminalApprovals: (conversationId: string) => TerminalApproval[];
};

export function useChatWindowBasicDerivedState<MessageBlock, TerminalApproval>(
  options: UseChatWindowBasicDerivedStateOptions<MessageBlock, TerminalApproval>,
) {
  const resizeHandlesEnabled = computed(() => {
    if (options.maximized.value) return false;
    return options.viewMode.value === "chat" || options.viewMode.value === "archives" || options.detachedChatWindow.value;
  });
  const conversationBusy = computed(() => options.trimming.value || options.compactingConversation.value);
  const titleText = computed(() => {
    if (options.viewMode.value === "chat") {
      return options.t("window.chatTitle", {
        name: options.currentForegroundPersona.value?.name || options.t("archives.roleAssistant"),
      });
    }
    if (options.viewMode.value === "archives") {
      return options.t("window.archivesTitle");
    }
    return options.t("window.configTitle");
  });
  const displayMessageBlocks = computed(() => options.visibleMessageBlocks.value);
  const terminalApprovalConversationItems = computed(() =>
    options.listConversationTerminalApprovals(options.currentChatConversationId.value),
  );

  return {
    resizeHandlesEnabled,
    conversationBusy,
    titleText,
    displayMessageBlocks,
    terminalApprovalConversationItems,
  };
}

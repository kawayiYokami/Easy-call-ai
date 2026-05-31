import { computed } from "vue";
import type { ChatConversationOverviewItem } from "../../../types/app";

export function useChatConversationItemsDerivedState(bindings: Record<string, any>) {
  const CONVERSATION_COLORS = [
    "primary",
    "warning",
    "secondary",
    "error",
    "accent",
    "info",
    "success",
    "neutral",
  ] as const;

  const chatUnarchivedConversationItems = computed(() => {
    const trimmingConversationId = String(bindings.trimmingConversationId?.value || "").trim();
    const compactingConversationId = String(bindings.compactingConversationId?.value || "").trim();
    const items = bindings.unarchivedConversations.value
      .map((item: any) => {
        const conversationId = String(item.conversationId || "").trim();
        const localRuntimeState = conversationId && conversationId === trimmingConversationId
          ? "archiving"
          : conversationId && conversationId === compactingConversationId
            ? "compacting"
            : "";
        return {
        conversationId: item.conversationId,
        title: item.title,
        summaryTitle: item.summaryTitle,
        kind: "local_unarchived" as const,
        messageCount: Number(item.messageCount || 0),
        unreadCount: Number(item.unreadCount || 0),
        agentId: String(item.agentId || "").trim(),
        departmentId: String(item.departmentId || "").trim(),
        departmentName: String(item.departmentName || "").trim(),
        parentConversationId: String(item.parentConversationId || "").trim() || undefined,
        forkMessageCursor: String(item.forkMessageCursor || "").trim() || undefined,
        workspaceLabel: String(item.workspaceLabel || "").trim() || "默认会话目录",
        isActive: !!item.isActive,
        isMainConversation: !!item.isMainConversation,
        isPinned: !!item.isPinned,
        pinIndex: Number.isFinite(Number(item.pinIndex)) ? Number(item.pinIndex) : undefined,
        runtimeState: localRuntimeState || item.runtimeState,
        currentTodo: String(item.currentTodo || "").trim(),
        currentTodos: Array.isArray(item.currentTodos) ? item.currentTodos : [],
        detachedWindowOpen: !!item.detachedWindowOpen,
        detachedWindowLabel: String(item.detachedWindowLabel || "").trim() || undefined,
        updatedAt: item.lastMessageAt || item.updatedAt || "",
        lastMessageAt: item.lastMessageAt || item.updatedAt || "",
        previewMessages: Array.isArray(item.previewMessages) ? item.previewMessages : [],
        backgroundStatus:
          bindings.backgroundConversationBadgeMap.value[String(item.conversationId || "").trim()] || undefined,
        };
      });

    const usedIndices = new Set<number>();
    return items.map((item: any) => {
      let colorIdx = 0;
      for (let i = 0; i < 8; i++) {
        if (!usedIndices.has(i)) {
          colorIdx = i;
          usedIndices.add(i);
          break;
        }
      }
      return {
        ...item,
        color: CONVERSATION_COLORS[colorIdx],
        canCreateNew: items.length < 8,
      };
    });
  });

  function resolveRemoteConversationDepartmentName(boundDepartmentId?: string): string {
    const normalizedDepartmentId = String(boundDepartmentId || "").trim();
    if (!normalizedDepartmentId) return "主部门";
    return (
      bindings.config.departments.find((item: any) => String(item.id || "").trim() === normalizedDepartmentId)?.name
      || normalizedDepartmentId
    );
  }

  const chatRemoteImConversationItems = computed<ChatConversationOverviewItem[]>(() =>
    bindings.remoteImContactConversations.value.map((item: any) => ({
      conversationId: String(item.conversationId || "").trim(),
      title: String(item.title || "").trim() || String(item.contactDisplayName || "").trim(),
      kind: "remote_im_contact",
      remoteContactId: String(item.contactId || "").trim(),
      remoteContactDisplayName: String(item.contactDisplayName || "").trim(),
      messageCount: Number(item.messageCount || 0),
      departmentId: String(item.boundDepartmentId || "").trim() || undefined,
      departmentName: [
        String(item.channelName || "").trim(),
        resolveRemoteConversationDepartmentName(item.boundDepartmentId),
      ].filter(Boolean).join(" · "),
      updatedAt: item.lastMessageAt || item.updatedAt || "",
      lastMessageAt: item.lastMessageAt || item.updatedAt || "",
      previewMessages: Array.isArray(item.previewMessages) ? item.previewMessages : [],
    })),
  );

  const chatConversationItems = computed<ChatConversationOverviewItem[]>(() => ([
    ...chatUnarchivedConversationItems.value,
    ...chatRemoteImConversationItems.value,
  ]));

  return {
    CONVERSATION_COLORS,
    chatUnarchivedConversationItems,
    resolveRemoteConversationDepartmentName,
    chatRemoteImConversationItems,
    chatConversationItems,
  };
}

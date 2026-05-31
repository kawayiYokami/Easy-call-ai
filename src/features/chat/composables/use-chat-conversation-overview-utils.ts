type ConversationOverviewUtilsOptions = {
  draftAssistantIdPrefix: string;
};

export function useChatConversationOverviewUtils(options: ConversationOverviewUtilsOptions) {
  function isOverviewDraftMessage(message?: any): boolean {
    const messageId = String(message?.id || "").trim();
    return messageId.startsWith(options.draftAssistantIdPrefix) || messageId.startsWith("__draft_user__:");
  }

  function previewMessageFromChatMessage(message: any) {
    const parts = Array.isArray(message.parts) ? message.parts : [];
    const textPreview = parts
      .filter((part: any) => part && typeof part === "object" && (part as { type?: unknown }).type === "text")
      .map((part: any) => String((part as { text?: unknown }).text || "").trim())
      .filter(Boolean)
      .join(" | ")
      .slice(0, 160);
    const providerMeta = (message.providerMeta || {}) as Record<string, unknown>;
    const attachmentEntries = Array.isArray(providerMeta.attachments) ? providerMeta.attachments : [];
    const hasPdfAttachment = attachmentEntries.some((entry) => {
      const item = entry as Record<string, unknown>;
      return String(item?.mime || "").toLowerCase().includes("pdf");
    });
    return {
      messageId: String(message.id || "").trim(),
      role: String(message.role || "").trim() || "assistant",
      speakerAgentId: String(message.speakerAgentId || "").trim() || undefined,
      createdAt: String(message.createdAt || "").trim() || undefined,
      textPreview: textPreview || undefined,
      hasImage: parts.some((part: any) => part && typeof part === "object" && (part as { type?: unknown }).type === "image"),
      hasPdf: hasPdfAttachment,
      hasAudio: parts.some((part: any) => part && typeof part === "object" && (part as { type?: unknown }).type === "audio"),
      hasAttachment: attachmentEntries.length > 0,
    };
  }

  function unarchivedConversationActivityAt(item: Record<string, any>): string {
    return String(item.lastMessageAt || item.updatedAt || "").trim();
  }

  function sortUnarchivedConversationOverviewItems(items: any[]): any[] {
    return [...items].sort((a, b) => {
      if (!!a.isMainConversation !== !!b.isMainConversation) {
        return Number(!!b.isMainConversation) - Number(!!a.isMainConversation);
      }
      if (!!a.isPinned !== !!b.isPinned) {
        return Number(!!b.isPinned) - Number(!!a.isPinned);
      }
      if (a.isPinned && b.isPinned) {
        const aIndex = Number.isFinite(Number(a.pinIndex)) ? Number(a.pinIndex) : Number.MAX_SAFE_INTEGER;
        const bIndex = Number.isFinite(Number(b.pinIndex)) ? Number(b.pinIndex) : Number.MAX_SAFE_INTEGER;
        return aIndex - bIndex || String(a.conversationId || "").localeCompare(String(b.conversationId || ""));
      }
      const aActivity = unarchivedConversationActivityAt(a);
      const bActivity = unarchivedConversationActivityAt(b);
      return bActivity.localeCompare(aActivity) || String(a.conversationId || "").localeCompare(String(b.conversationId || ""));
    });
  }

  return {
    isOverviewDraftMessage,
    previewMessageFromChatMessage,
    sortUnarchivedConversationOverviewItems,
    unarchivedConversationActivityAt,
  };
}

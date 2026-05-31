type ConversationMessageUtilsOptions = {
  draftAssistantIdPrefix: string;
  ensureConversationMessageIds: (messages: any[]) => any[];
};

export function useChatConversationMessageUtils(options: ConversationMessageUtilsOptions) {
  function isAssistantDraftMessage(message?: any): boolean {
    return String(message?.id || "").trim().startsWith(options.draftAssistantIdPrefix);
  }

  function formalizeConversationMessages(messages: any[]): any[] {
    return options.ensureConversationMessageIds(messages)
      .filter((item: any) => !isAssistantDraftMessage(item));
  }

  function freezeConversationMessages(messages: any[]): any[] {
    return options.ensureConversationMessageIds(messages).map((message: any) => {
      const messageId = String(message?.id || "").trim();
      if (!messageId.startsWith(options.draftAssistantIdPrefix)) {
        return message;
      }
      const providerMeta = { ...((message.providerMeta || {}) as Record<string, unknown>) };
      delete providerMeta._streaming;
      delete providerMeta._streamSegments;
      delete providerMeta._streamTail;
      return {
        ...message,
        providerMeta,
      };
    });
  }

  function insertMessagesBeforeAssistantDraft(messages: any[], incoming: any[]): any[] {
    if (!Array.isArray(incoming) || incoming.length <= 0) return messages;
    const draftIdx = messages.findIndex((message) => isAssistantDraftMessage(message));
    if (draftIdx < 0) {
      return [...messages, ...incoming];
    }
    return [
      ...messages.slice(0, draftIdx),
      ...incoming,
      ...messages.slice(draftIdx),
    ];
  }

  function areMessagesEquivalent(left: any[], right: any[]): boolean {
    if (left === right) return true;
    if (left.length !== right.length) return false;
    for (let index = 0; index < left.length; index += 1) {
      const leftMessage = left[index];
      const rightMessage = right[index];
      const leftId = String(leftMessage?.id || "").trim();
      const rightId = String(rightMessage?.id || "").trim();
      if (leftId !== rightId) return false;
      const leftCreatedAt = String(leftMessage?.createdAt || "").trim();
      const rightCreatedAt = String(rightMessage?.createdAt || "").trim();
      if (leftCreatedAt !== rightCreatedAt) return false;
      const leftMeta = JSON.stringify(leftMessage?.providerMeta || {});
      const rightMeta = JSON.stringify(rightMessage?.providerMeta || {});
      if (leftMeta !== rightMeta) return false;
      const leftParts = JSON.stringify(leftMessage?.parts || []);
      const rightParts = JSON.stringify(rightMessage?.parts || []);
      if (leftParts !== rightParts) return false;
    }
    return true;
  }

  function messageContentSignature(message?: any): string {
    return [
      String(message?.id || "").trim(),
      String(message?.createdAt || "").trim(),
      String(message?.role || "").trim(),
      String(message?.speakerAgentId || "").trim(),
      JSON.stringify(message?.providerMeta || {}),
      JSON.stringify(message?.parts || []),
      JSON.stringify(message?.extraTextBlocks || []),
      JSON.stringify(message?.toolCall || []),
    ].join("|");
  }

  function reuseStableMessageReferences(nextMessages: any[], previousMessages: any[]): any[] {
    if (!Array.isArray(nextMessages) || nextMessages.length <= 0) {
      return [];
    }
    const previousById = new Map<string, any>();
    for (const message of Array.isArray(previousMessages) ? previousMessages : []) {
      const messageId = String(message?.id || "").trim();
      if (!messageId) continue;
      previousById.set(messageId, message);
    }
    return nextMessages.map((message) => {
      const messageId = String(message?.id || "").trim();
      if (!messageId) return message;
      const previous = previousById.get(messageId);
      if (!previous) return message;
      return messageContentSignature(previous) === messageContentSignature(message)
        ? previous
        : message;
    });
  }

  function replaceConversationMessage(messages: any[], nextMessage: any): any[] {
    const targetMessageId = String(nextMessage?.id || "").trim();
    if (!targetMessageId || !Array.isArray(messages) || messages.length <= 0) {
      return messages;
    }
    let changed = false;
    const nextMessages = messages.map((message) => {
      if (String(message?.id || "").trim() !== targetMessageId) {
        return message;
      }
      changed = true;
      return nextMessage;
    });
    return changed ? reuseStableMessageReferences(nextMessages, messages) : messages;
  }

  return {
    areMessagesEquivalent,
    formalizeConversationMessages,
    freezeConversationMessages,
    insertMessagesBeforeAssistantDraft,
    isAssistantDraftMessage,
    messageContentSignature,
    replaceConversationMessage,
    reuseStableMessageReferences,
  };
}

export function readConversationIdFromPayload(payload: unknown): string {
  if (!payload || typeof payload !== "object") return "";
  return String((payload as { conversationId?: unknown }).conversationId || "").trim();
}

export function readMessagesFromPayload(payload: unknown): any[] {
  if (!payload || typeof payload !== "object") return [];
  const rawMessages = (payload as { messages?: unknown }).messages;
  return Array.isArray(rawMessages) ? rawMessages as any[] : [];
}

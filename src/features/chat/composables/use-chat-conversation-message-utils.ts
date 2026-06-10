import {
  messageWithoutStableRenderId,
  preserveStableRenderId,
  providerMetaWithoutStableRenderId,
} from "../utils/stable-render-id";

type ConversationMessageUtilsOptions = {
  draftAssistantIdPrefix: string;
  ensureConversationMessageIds: (messages: any[]) => any[];
};

const TRANSIENT_PROVIDER_META_KEYS = [
  "_streaming",
  "_streamSegments",
  "_streamTail",
  "_streamAnimatedDelta",
  "_preStreamingStatusText",
  "_frontendDispatchStartedAtMs",
  "_frontendDispatchElapsedMs",
  "_streamBlocks",
  "_stableRenderId",
];

export function useChatConversationMessageUtils(options: ConversationMessageUtilsOptions) {
  function isAssistantDraftMessage(message?: any): boolean {
    return String(message?.id || "").trim().startsWith(options.draftAssistantIdPrefix);
  }

  function messageCreatedAtMs(message?: any): number | null {
    const raw = String(message?.createdAt || "").trim();
    if (!raw) return null;
    const ms = Date.parse(raw);
    return Number.isFinite(ms) ? ms : null;
  }

  function stripTransientProviderMeta(message: any): any {
    const rawProviderMeta = message?.providerMeta;
    if (!rawProviderMeta || typeof rawProviderMeta !== "object") return message;
    const providerMeta = { ...(rawProviderMeta as Record<string, unknown>) };
    let changed = false;
    for (const key of TRANSIENT_PROVIDER_META_KEYS) {
      if (key in providerMeta) {
        delete providerMeta[key];
        changed = true;
      }
    }
    return changed ? { ...message, providerMeta } : message;
  }

  function formalizeConversationMessages(messages: any[]): any[] {
    return options.ensureConversationMessageIds(messages)
      .filter((item: any) => !isAssistantDraftMessage(item))
      .map((item: any) => stripTransientProviderMeta(messageWithoutStableRenderId(item)));
  }

  function freezeConversationMessages(messages: any[]): any[] {
    return options.ensureConversationMessageIds(messages)
      .map((message: any) => stripTransientProviderMeta(messageWithoutStableRenderId(message)));
  }

  function insertMessageIntoTimeline(messages: any[], incoming: any): any[] {
    const incomingAtMs = messageCreatedAtMs(incoming);
    if (incomingAtMs === null) {
      return [...messages, incoming];
    }
    const insertIdx = messages.findIndex((message) => {
      const existingAtMs = messageCreatedAtMs(message);
      return existingAtMs !== null && existingAtMs > incomingAtMs;
    });
    if (insertIdx < 0) {
      return [...messages, incoming];
    }
    return [
      ...messages.slice(0, insertIdx),
      incoming,
      ...messages.slice(insertIdx),
    ];
  }

  function mergeMessagesIntoTimeline(messages: any[], incoming: any[]): any[] {
    if (!Array.isArray(incoming) || incoming.length <= 0) return messages;
    let nextMessages = Array.isArray(messages) ? [...messages] : [];
    for (const rawIncoming of incoming) {
      const incomingMessage = stripTransientProviderMeta(messageWithoutStableRenderId(rawIncoming));
      const incomingId = String(incomingMessage?.id || "").trim();
      if (!incomingId) {
        nextMessages = insertMessageIntoTimeline(nextMessages, incomingMessage);
        continue;
      }
      const existingIdx = nextMessages.findIndex((message) =>
        String(message?.id || "").trim() === incomingId
      );
      if (existingIdx >= 0) {
        let replaced = false;
        nextMessages = nextMessages.flatMap((message) => {
          if (String(message?.id || "").trim() !== incomingId) {
            return [message];
          }
          if (replaced) {
            return [];
          }
          replaced = true;
          return [incomingMessage];
        });
        continue;
      }
      nextMessages = insertMessageIntoTimeline(nextMessages, incomingMessage);
    }
    return reuseStableMessageReferences(nextMessages, messages);
  }

  function insertMessagesBeforeAssistantDraft(messages: any[], incoming: any[]): any[] {
    return mergeMessagesIntoTimeline(messages, incoming);
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
      const leftMeta = JSON.stringify(providerMetaWithoutStableRenderId(leftMessage?.providerMeta));
      const rightMeta = JSON.stringify(providerMetaWithoutStableRenderId(rightMessage?.providerMeta));
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
      JSON.stringify(providerMetaWithoutStableRenderId(message?.providerMeta)),
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
      const nextMessage = preserveStableRenderId(message, previous);
      return messageContentSignature(previous) === messageContentSignature(message)
        ? previous
        : nextMessage;
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
    mergeMessagesIntoTimeline,
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

import {
  messageWithoutStableRenderId,
  preserveStableRenderId,
  providerMetaWithoutStableRenderId,
} from "../utils/stable-render-id";
import {
  mergeAuthoritativeConversationMessages,
  replaceConversationHistory as replaceSharedConversationHistory,
  type AuthoritativeMessageMergeOptions,
} from "./chat-message-state-machine";

type ConversationMessageUtilsOptions = {
  ensureConversationMessageIds: (messages: any[]) => any[];
};

const TRANSIENT_PROVIDER_META_KEYS = [
  "_streaming",
  "_preStreamingStatusText",
  "_frontendDispatchStartedAtMs",
  "_frontendDispatchElapsedMs",
  "_stableRenderId",
];

export function useChatConversationMessageUtils(options: ConversationMessageUtilsOptions) {
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
      .map((item: any) => stripTransientProviderMeta(messageWithoutStableRenderId(item)));
  }

  function freezeConversationMessages(messages: any[]): any[] {
    return options.ensureConversationMessageIds(messages)
      .map((message: any) => stripTransientProviderMeta(messageWithoutStableRenderId(message)));
  }

  function mergeMessagesIntoTimeline(
    messages: any[],
    incoming: any[],
    mergeOptions?: AuthoritativeMessageMergeOptions,
  ): any[] {
    if (!Array.isArray(incoming) || incoming.length <= 0) return messages;
    const normalizedIncoming = incoming.map((message) => (
      stripTransientProviderMeta(messageWithoutStableRenderId(message))
    ));
    const nextMessages = mergeAuthoritativeConversationMessages(
      Array.isArray(messages) ? messages : [],
      normalizedIncoming,
      mergeOptions,
    );
    return reuseStableMessageReferences(nextMessages, messages);
  }

  function insertMessagesBeforeStreamingAssistantProjection(messages: any[], incoming: any[]): any[] {
    return mergeMessagesIntoTimeline(messages, incoming);
  }

  function replaceConversationHistory(messages: any[], incoming: any[]): any[] {
    const nextMessages = replaceSharedConversationHistory(
      Array.isArray(messages) ? messages : [],
      options.ensureConversationMessageIds(Array.isArray(incoming) ? incoming : []),
    );
    return reuseStableMessageReferences(nextMessages, messages);
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
    return mergeMessagesIntoTimeline(messages, [nextMessage]);
  }

  return {
    areMessagesEquivalent,
    formalizeConversationMessages,
    freezeConversationMessages,
    insertMessagesBeforeStreamingAssistantProjection,
    mergeMessagesIntoTimeline,
    messageContentSignature,
    replaceConversationMessage,
    replaceConversationHistory,
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

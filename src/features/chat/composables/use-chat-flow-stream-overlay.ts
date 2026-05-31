import type { ChatMessage } from "../../../types/app";
import type {
  ConversationRuntimeStreamCacheSnapshot,
  ConversationStreamCache,
} from "./use-chat-flow-stream-cache";

type StreamOverlayCache = ConversationRuntimeStreamCacheSnapshot | ConversationStreamCache | null | undefined;

export type StreamingHistoryOverlayResult = {
  messages: ChatMessage[];
  replacedMessageId: string;
  removed: boolean;
};

export function streamingOverlayReplacedMessageId(cache?: StreamOverlayCache): string {
  return String(cache?.persistedAssistantMessageId || "").trim();
}

export function applyStreamingHistoryOverlay(
  messages: ChatMessage[],
  cache?: StreamOverlayCache,
): StreamingHistoryOverlayResult {
  const replacedMessageId = streamingOverlayReplacedMessageId(cache);
  if (!replacedMessageId || !Array.isArray(messages) || messages.length === 0) {
    return { messages, replacedMessageId, removed: false };
  }
  let removed = false;
  const nextMessages = messages.filter((message) => {
    const matched = String(message?.id || "").trim() === replacedMessageId;
    if (matched) removed = true;
    return !matched;
  });
  return { messages: removed ? nextMessages : messages, replacedMessageId, removed };
}

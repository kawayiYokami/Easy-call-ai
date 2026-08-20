import type { ChatMessage } from "../../../types/app";

const DRAFT_USER_ID_PREFIX = "__draft_user__:";

export function normalizeConversationId(conversationId?: string | null): string {
  return String(conversationId || "").trim();
}

export function positiveRoundedNumber(value: unknown): number {
  const numeric = Number(value || 0);
  if (!Number.isFinite(numeric) || numeric <= 0) return 0;
  return Math.round(numeric);
}

export function isChatAbortedByUser(error: unknown): boolean {
  const normalized = String(
    typeof error === "string"
      ? error
      : (error as { message?: unknown } | null)?.message ?? error ?? "",
  ).trim();
  return normalized === "CHAT_ABORTED_BY_USER";
}

export function stringifyExternalEventPayload(payload: unknown, eventName: string): string {
  if (typeof payload === "string") return payload;
  if (payload && typeof payload === "object") {
    try {
      return JSON.stringify(payload);
    } catch {
      // payload 不可序列化时按空串兜底，调用方自行处理
    }
  }
  return "";
}

export function sameActivationId(left?: string | null, right?: string | null): boolean {
  const normalizedLeft = String(left || "").trim();
  const normalizedRight = String(right || "").trim();
  return !!normalizedLeft && !!normalizedRight && normalizedLeft === normalizedRight;
}

export function readMessagePlainText(message?: ChatMessage): string {
  if (!message) return "";
  const parts = Array.isArray(message.parts) ? message.parts : [];
  return parts
    .filter((part) => part && typeof part === "object" && (part as { type?: unknown }).type === "text")
    .map((part) => String((part as { text?: unknown }).text || ""))
    .join("");
}

/** 消息是否已有可见内容。有内容时，除撤回外禁止前端删除。 */
export function messageHasVisibleContent(message?: ChatMessage | null): boolean {
  if (!message) return false;
  if (readMessagePlainText(message).trim()) return true;
  const parts = Array.isArray(message.parts) ? message.parts : [];
  if (parts.some((part) => {
    if (!part || typeof part !== "object") return false;
    return String((part as { type?: unknown }).type || "").trim() !== "text";
  })) {
    return true;
  }
  if (Array.isArray(message.extraTextBlocks) && message.extraTextBlocks.some((item) => String(item || "").trim())) {
    return true;
  }
  if (Array.isArray(message.toolCall) && message.toolCall.length > 0) return true;
  if (Array.isArray(message.activityItems) && message.activityItems.length > 0) return true;
  const meta = (message.providerMeta || {}) as Record<string, unknown>;
  if (Array.isArray(message.contentBlocks) && message.contentBlocks.length > 0) return true;
  if (String(meta._preStreamingStatusText || "").trim()) return true;
  if (String(meta._toolStatusText || "").trim()) return true;
  return false;
}

export function formalizeMessages(messages: ChatMessage[]): ChatMessage[] {
  return messages.filter((item) => {
    const messageId = String(item?.id || "").trim();
    return (
      !messageId.startsWith(DRAFT_USER_ID_PREFIX)
    );
  });
}

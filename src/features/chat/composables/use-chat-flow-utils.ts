import type { ChatMessage } from "../../../types/app";

const DRAFT_ASSISTANT_ID_PREFIX = "__draft_assistant__:";
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
    } catch (error) {
      console.warn("[聊天事件] 外部事件 payload 序列化失败", {
        eventName,
        error,
      });
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

export function formalizeMessages(messages: ChatMessage[]): ChatMessage[] {
  return messages.filter((item) => {
    const messageId = String(item?.id || "").trim();
    return (
      !messageId.startsWith(DRAFT_ASSISTANT_ID_PREFIX)
      && !messageId.startsWith(DRAFT_USER_ID_PREFIX)
    );
  });
}

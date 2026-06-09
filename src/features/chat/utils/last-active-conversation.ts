const LAST_ACTIVE_CONVERSATION_STORAGE_KEY = "easy_call.chat.last_active_conversation_id.v1";

export function readLastActiveConversationId(): string {
  if (typeof window === "undefined") return "";
  return String(window.localStorage.getItem(LAST_ACTIVE_CONVERSATION_STORAGE_KEY) || "").trim();
}

export function writeLastActiveConversationId(conversationId: string) {
  if (typeof window === "undefined") return;
  const cid = String(conversationId || "").trim();
  if (!cid) return;
  window.localStorage.setItem(LAST_ACTIVE_CONVERSATION_STORAGE_KEY, cid);
}

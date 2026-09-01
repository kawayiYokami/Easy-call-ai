const LAST_ACTIVE_CONVERSATION_STORAGE_KEY = "easy_call.chat.last_active_conversation_id.v1";
const LAST_ACTIVE_APP_ROOT_STORAGE_KEY = "easy_call.chat.last_active_app_root.v1";

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

export function clearLastActiveConversationId(): void {
  if (typeof window === "undefined") return;
  window.localStorage.removeItem(LAST_ACTIVE_CONVERSATION_STORAGE_KEY);
}

export function readLastActiveAppRoot(): string {
  if (typeof window === "undefined") return "";
  return String(window.localStorage.getItem(LAST_ACTIVE_APP_ROOT_STORAGE_KEY) || "").trim();
}

export function writeLastActiveAppRoot(appRoot: string): void {
  if (typeof window === "undefined") return;
  const normalized = String(appRoot || "").trim();
  if (!normalized) return;
  window.localStorage.setItem(LAST_ACTIVE_APP_ROOT_STORAGE_KEY, normalized);
}

export function isValidConversationIdInCandidates(
  conversationId: string,
  candidates: any[],
): boolean {
  const cid = String(conversationId || "").trim();
  if (!cid || !Array.isArray(candidates)) return false;
  return candidates.some((item) => String(item?.conversationId || "").trim() === cid);
}

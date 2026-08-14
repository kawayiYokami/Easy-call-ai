import { ref } from "vue";
import { emitTransportEvent, onTransportNotification } from "../../../services/tauri-api";

const CHAT_BUBBLE_BACKGROUND_STORAGE_KEY = "easy-call.chat.bubble-background.v1";
const CHAT_SEGMENTED_MARKDOWN_STORAGE_KEY = "easy-call.chat.segmented-markdown.v1";
const CHAT_TIME_DISPLAY_MODE_STORAGE_KEY = "easy-call.chat.time-display-mode.v1";

type ChatTimeDisplayMode = "relative" | "absolute";

type ChatMessageAppearancePayload = {
  assistantBubbleBackgroundEnabled?: boolean;
  segmentedMarkdownEnabled?: boolean;
  chatTimeDisplayMode?: ChatTimeDisplayMode;
};

const assistantBubbleBackgroundEnabled = ref(readBooleanPreferenceDefault(CHAT_BUBBLE_BACKGROUND_STORAGE_KEY, true));
const segmentedMarkdownEnabled = ref(readBooleanPreferenceDefault(CHAT_SEGMENTED_MARKDOWN_STORAGE_KEY, true));
const chatTimeDisplayMode = ref<ChatTimeDisplayMode>(readChatTimeDisplayModePreference());
let initialized = false;
let eventUnlisten: (() => void) | null = null;

function readBooleanPreference(storageKey: string): boolean {
  if (typeof window === "undefined") return false;
  return window.localStorage.getItem(storageKey) === "1";
}

/** 读布尔偏好：无存储记录时返回默认值（区别于显式存 "0"） */
function readBooleanPreferenceDefault(storageKey: string, defaultValue: boolean): boolean {
  if (typeof window === "undefined") return defaultValue;
  const stored = window.localStorage.getItem(storageKey);
  return stored === null ? defaultValue : stored === "1";
}

function persistBooleanPreference(storageKey: string, enabled: boolean) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(storageKey, enabled ? "1" : "0");
}

function readChatTimeDisplayModePreference(): ChatTimeDisplayMode {
  if (typeof window === "undefined") return "relative";
  return window.localStorage.getItem(CHAT_TIME_DISPLAY_MODE_STORAGE_KEY) === "absolute" ? "absolute" : "relative";
}

function persistChatTimeDisplayMode(mode: ChatTimeDisplayMode) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(CHAT_TIME_DISPLAY_MODE_STORAGE_KEY, mode);
}

function applyPayload(payload: ChatMessageAppearancePayload | undefined) {
  if (typeof payload?.assistantBubbleBackgroundEnabled === "boolean") {
    assistantBubbleBackgroundEnabled.value = payload.assistantBubbleBackgroundEnabled;
  }
  if (typeof payload?.segmentedMarkdownEnabled === "boolean") {
    segmentedMarkdownEnabled.value = payload.segmentedMarkdownEnabled;
  }
  if (payload?.chatTimeDisplayMode === "absolute" || payload?.chatTimeDisplayMode === "relative") {
    chatTimeDisplayMode.value = payload.chatTimeDisplayMode;
  }
}

function restoreFromStorage() {
  assistantBubbleBackgroundEnabled.value = readBooleanPreferenceDefault(CHAT_BUBBLE_BACKGROUND_STORAGE_KEY, true);
  segmentedMarkdownEnabled.value = readBooleanPreferenceDefault(CHAT_SEGMENTED_MARKDOWN_STORAGE_KEY, true);
  chatTimeDisplayMode.value = readChatTimeDisplayModePreference();
}

function handleStorageEvent(event: StorageEvent) {
  if (
    event.key !== CHAT_BUBBLE_BACKGROUND_STORAGE_KEY
    && event.key !== CHAT_SEGMENTED_MARKDOWN_STORAGE_KEY
    && event.key !== CHAT_TIME_DISPLAY_MODE_STORAGE_KEY
  ) return;
  restoreFromStorage();
}

export function initChatMessageAppearance() {
  if (initialized) return;
  initialized = true;
  restoreFromStorage();
  if (typeof window !== "undefined") {
    window.addEventListener("storage", handleStorageEvent);
  }
  eventUnlisten = onTransportNotification<ChatMessageAppearancePayload>("chatMessageAppearance.changed", (payload) => {
    applyPayload(payload);
  });
}

function emitAppearanceChanged() {
  void emitTransportEvent("chatMessageAppearance.changed", {
    assistantBubbleBackgroundEnabled: assistantBubbleBackgroundEnabled.value,
    segmentedMarkdownEnabled: segmentedMarkdownEnabled.value,
    chatTimeDisplayMode: chatTimeDisplayMode.value,
  } satisfies ChatMessageAppearancePayload).catch((error) => {
    console.warn("[聊天外观] 同步消息外观变化失败", error);
  });
}

export function useChatMessageAppearance() {
  initChatMessageAppearance();

  function setAssistantBubbleBackgroundEnabled(enabled: boolean) {
    assistantBubbleBackgroundEnabled.value = enabled;
    persistBooleanPreference(CHAT_BUBBLE_BACKGROUND_STORAGE_KEY, enabled);
    emitAppearanceChanged();
  }

  function setSegmentedMarkdownEnabled(enabled: boolean) {
    segmentedMarkdownEnabled.value = enabled;
    persistBooleanPreference(CHAT_SEGMENTED_MARKDOWN_STORAGE_KEY, enabled);
    emitAppearanceChanged();
  }

  function setChatTimeDisplayMode(mode: ChatTimeDisplayMode) {
    chatTimeDisplayMode.value = mode;
    persistChatTimeDisplayMode(mode);
    emitAppearanceChanged();
  }

  return {
    assistantBubbleBackgroundEnabled,
    segmentedMarkdownEnabled,
    chatTimeDisplayMode,
    setAssistantBubbleBackgroundEnabled,
    setSegmentedMarkdownEnabled,
    setChatTimeDisplayMode,
  };
}

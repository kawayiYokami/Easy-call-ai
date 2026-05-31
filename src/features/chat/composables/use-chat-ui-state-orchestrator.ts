import { computed, ref, watch, type Ref } from "vue";
import type { ConfigSearchTab, ConfigSearchResult } from "../../config/search/config-search";
import type { ChatMentionTarget, PromptCommandPreset } from "../../../types/app";
import type { ConversationPipelineStatus } from "../../shell/composables/use-pipeline-status";
import type { searchConfigTabs } from "../../config/search/config-search";

const CHAT_CONVERSATION_LIST_TAB_STORAGE_KEY = "easy_call.chat_conversation_list_tab.v1";
const CHAT_LEFT_PANEL_MODE_STORAGE_KEY = "easy_call.chat_left_panel_mode.v1";
const CHAT_RIGHT_PANEL_MODE_STORAGE_KEY = "easy_call.chat_right_panel_mode.v1";
const LEGACY_CHAT_LEFT_PANEL_MODE_STORAGE_KEY = "easy-call.chat.left-panel-mode";
const LEGACY_CHAT_RIGHT_PANEL_MODE_STORAGE_KEY = "easy-call.chat.right-panel-mode";
const CHAT_SIDE_PANEL_VISIBILITY_STORAGE_KEYS = {
  left: "easy_call.chat_left_sidebar_visible.v1",
  right: "easy_call.chat_right_sidebar_visible.v1",
} as const;
const LEGACY_CHAT_SIDE_PANEL_VISIBILITY_STORAGE_KEYS = {
  left: "easy-call.chat.left-sidebar-visible",
  right: "easy-call.chat.right-sidebar-visible",
} as const;
const CHAT_SIDE_PANEL_WIDTH_STORAGE_KEYS = {
  left: "easy_call.chat_left_sidebar_width.v1",
  right: "easy_call.chat_right_sidebar_width.v1",
} as const;
const LEGACY_CHAT_SIDE_PANEL_WIDTH_STORAGE_KEYS = {
  left: "easy-call.chat.left-sidebar-width",
  right: "easy-call.chat.right-sidebar-width",
} as const;

function loadStoredConversationListTab(): "local" | "contact" {
  if (typeof window === "undefined") return "local";
  const stored = String(window.localStorage.getItem(CHAT_CONVERSATION_LIST_TAB_STORAGE_KEY) || "").trim();
  return stored === "contact" ? "contact" : "local";
}

function loadStoredChatLeftPanelMode(): "local" | "contact" {
  if (typeof window === "undefined") return loadStoredConversationListTab();
  const stored = String(window.localStorage.getItem(CHAT_LEFT_PANEL_MODE_STORAGE_KEY) || window.localStorage.getItem(LEGACY_CHAT_LEFT_PANEL_MODE_STORAGE_KEY) || "").trim();
  return stored === "contact" ? "contact" : loadStoredConversationListTab();
}

function loadStoredChatRightPanelMode(): "reader" | "review" | "delegate" {
  if (typeof window === "undefined") return "delegate";
  const stored = String(window.localStorage.getItem(CHAT_RIGHT_PANEL_MODE_STORAGE_KEY) || window.localStorage.getItem(LEGACY_CHAT_RIGHT_PANEL_MODE_STORAGE_KEY) || "").trim();
  return stored === "reader" || stored === "delegate" ? stored : "delegate";
}

function loadStoredChatSidePanelVisibility(side: "left" | "right"): boolean {
  if (typeof window === "undefined") return false;
  const stored = window.localStorage.getItem(CHAT_SIDE_PANEL_VISIBILITY_STORAGE_KEYS[side])
    ?? window.localStorage.getItem(LEGACY_CHAT_SIDE_PANEL_VISIBILITY_STORAGE_KEYS[side]);
  return stored === "true";
}

function storeChatSidePanelVisibility(side: "left" | "right", visible: boolean) {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(CHAT_SIDE_PANEL_VISIBILITY_STORAGE_KEYS[side], visible ? "true" : "false");
}

function loadStoredChatSidePanelWidths(): { leftWidth: number; rightWidth: number } {
  if (typeof window === "undefined") {
    return { leftWidth: 320, rightWidth: 320 };
  }
  const leftWidth = Number(window.localStorage.getItem(CHAT_SIDE_PANEL_WIDTH_STORAGE_KEYS.left) ?? window.localStorage.getItem(LEGACY_CHAT_SIDE_PANEL_WIDTH_STORAGE_KEYS.left));
  const rightWidth = Number(window.localStorage.getItem(CHAT_SIDE_PANEL_WIDTH_STORAGE_KEYS.right) ?? window.localStorage.getItem(LEGACY_CHAT_SIDE_PANEL_WIDTH_STORAGE_KEYS.right));
  return {
    leftWidth: Number.isFinite(leftWidth) ? leftWidth : 320,
    rightWidth: Number.isFinite(rightWidth) ? rightWidth : 320,
  };
}

export type ChatUiStateBindings = {
  viewMode: Ref<"chat" | "archives" | "config">;
  detachedChatWindow: Ref<boolean>;
  currentChatConversationId: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  clearConversationStatus: (conversationId: string, status?: ConversationPipelineStatus) => void;
  searchConfigTabs: typeof searchConfigTabs;
  resolveConfigLocale: () => Parameters<typeof searchConfigTabs>[1];
};

export function useChatUiStateOrchestrator(bindings: ChatUiStateBindings) {
  const configTab = ref<ConfigSearchTab>("hotkey");
  const configSearchQuery = ref("");
  const selectedInstructionPrompts = ref<PromptCommandPreset[]>([]);
  const selectedChatMentions = ref<ChatMentionTarget[]>([]);
  const chatInput = ref("");

  const conversationListTab = ref<"local" | "contact">(loadStoredConversationListTab());
  const chatLeftPanelMode = ref<"local" | "contact">(loadStoredChatLeftPanelMode());
  const chatRightPanelMode = ref<"reader" | "review" | "delegate">(loadStoredChatRightPanelMode());
  const sideConversationListVisible = ref(loadStoredChatSidePanelVisibility("left"));
  const toolReviewPanelOpenVisible = ref(loadStoredChatSidePanelVisibility("right"));
  const chatSidePanelWidths = ref(loadStoredChatSidePanelWidths());

  const conversationChatErrorTextMap = ref<Record<string, string>>({});
  const fallbackChatErrorText = ref("");

  function getConversationChatErrorText(conversationId: string) {
    const cid = String(conversationId || "").trim();
    if (!cid) return fallbackChatErrorText.value;
    return conversationChatErrorTextMap.value[cid] || "";
  }

  function setConversationChatErrorText(conversationId: string, text: string) {
    const cid = String(conversationId || "").trim();
    const normalizedText = String(text || "");
    if (!cid) {
      fallbackChatErrorText.value = normalizedText;
      return;
    }
    const next = { ...conversationChatErrorTextMap.value };
    if (normalizedText.trim()) {
      next[cid] = normalizedText;
    } else {
      delete next[cid];
    }
    conversationChatErrorTextMap.value = next;
  }

  function clearMatchingConversationChatErrors(predicate: (text: string) => boolean) {
    let changed = false;
    const next: Record<string, string> = {};
    for (const [conversationId, text] of Object.entries(conversationChatErrorTextMap.value)) {
      if (predicate(text)) {
        changed = true;
        continue;
      }
      next[conversationId] = text;
    }
    if (changed) {
      conversationChatErrorTextMap.value = next;
    }
    if (predicate(fallbackChatErrorText.value)) {
      fallbackChatErrorText.value = "";
    }
  }

  const chatErrorText = computed({
    get: () => getConversationChatErrorText(bindings.currentChatConversationId.value),
    set: (text: string) => {
      setConversationChatErrorText(bindings.currentChatConversationId.value, text);
    },
  });

  function clearChatError() {
    const conversationId = String(bindings.currentChatConversationId.value || "").trim();
    setConversationChatErrorText(conversationId, "");
    bindings.clearConversationStatus(conversationId, "error");
    if (bindings.toolStatusState.value === "failed") {
      bindings.toolStatusState.value = "";
    }
  }

  function handleChatInputUpdate(value: string) {
    chatInput.value = value;
  }

  function updateConfigSearchQuery(value: string) {
    configSearchQuery.value = String(value || "");
  }

  function handleSelectConfigSearchResult(tab: ConfigSearchTab) {
    configTab.value = tab;
    configSearchQuery.value = "";
  }

  function updateSelectedInstructionPrompts(value: PromptCommandPreset[]) {
    selectedInstructionPrompts.value = Array.isArray(value)
      ? value
          .map((item) => ({
            id: String(item?.id || "").trim(),
            name: String(item?.prompt || item?.name || "").trim(),
            prompt: String(item?.prompt || item?.name || "").trim(),
          }))
          .filter((item) => !!item.id && !!item.prompt)
      : [];
  }

  function addChatMention(value: ChatMentionTarget) {
    const agentId = String(value?.agentId || "").trim();
    const departmentId = String(value?.departmentId || "").trim();
    const agentName = String(value?.agentName || "").trim();
    if (!agentId || !departmentId || !agentName) return;
    if (selectedChatMentions.value.some((item) => item.agentId === agentId && item.departmentId === departmentId)) return;
    selectedChatMentions.value = [
      ...selectedChatMentions.value,
      {
        agentId,
        agentName,
        departmentId,
        departmentName: String(value?.departmentName || "").trim(),
        avatarUrl: String(value?.avatarUrl || "").trim() || undefined,
      },
    ];
  }

  function removeChatMention(value: string | { agentId?: string; departmentId?: string }) {
    const normalizedAgentId =
      typeof value === "string"
        ? String(value || "").trim()
        : String(value?.agentId || "").trim();
    const normalizedDepartmentId =
      typeof value === "string"
        ? ""
        : String(value?.departmentId || "").trim();
    selectedChatMentions.value = selectedChatMentions.value.filter((item) => {
      if (item.agentId !== normalizedAgentId) return true;
      if (!normalizedDepartmentId) return false;
      return item.departmentId !== normalizedDepartmentId;
    });
  }

  function handleSideConversationListVisibleChange(value: boolean) {
    sideConversationListVisible.value = value;
    storeChatSidePanelVisibility("left", value);
  }

  function handleToolReviewPanelOpenChange(value: boolean) {
    toolReviewPanelOpenVisible.value = value;
    if (value || String(bindings.currentChatConversationId.value || "").trim()) {
      storeChatSidePanelVisibility("right", value);
    }
  }

  function updateConversationListTab(value: "local" | "contact") {
    conversationListTab.value = value === "contact" ? "contact" : "local";
    chatLeftPanelMode.value = conversationListTab.value;
    if (typeof window === "undefined") return;
    window.localStorage.setItem(CHAT_CONVERSATION_LIST_TAB_STORAGE_KEY, conversationListTab.value);
    window.localStorage.setItem(CHAT_LEFT_PANEL_MODE_STORAGE_KEY, chatLeftPanelMode.value);
  }

  function updateChatLeftPanelMode(value: "local" | "contact") {
    const nextMode = value === "contact" ? "contact" : "local";
    chatLeftPanelMode.value = nextMode;
    conversationListTab.value = nextMode;
    if (typeof window !== "undefined") {
      window.localStorage.setItem(CHAT_LEFT_PANEL_MODE_STORAGE_KEY, nextMode);
      window.localStorage.setItem(CHAT_CONVERSATION_LIST_TAB_STORAGE_KEY, nextMode);
    }
    if (!sideConversationListVisible.value && bindings.viewMode.value === "chat" && !bindings.detachedChatWindow.value) {
      sideConversationListVisible.value = true;
      storeChatSidePanelVisibility("left", true);
    }
  }

  let _panelOpenedByFile = false;

  function updateChatRightPanelMode(value: "reader" | "review" | "delegate") {
    const nextMode = value === "reader" || value === "delegate" ? value : "review";
    _panelOpenedByFile = value === "reader";
    chatRightPanelMode.value = nextMode;
    if (typeof window !== "undefined") {
      window.localStorage.setItem(CHAT_RIGHT_PANEL_MODE_STORAGE_KEY, nextMode);
    }
    if (!toolReviewPanelOpenVisible.value && bindings.viewMode.value === "chat") {
      toolReviewPanelOpenVisible.value = true;
      storeChatSidePanelVisibility("right", true);
    }
  }

  function handleChatSidePanelWidthsChange(value: { leftWidth: number; rightWidth: number }) {
    const leftWidth = Number(value.leftWidth);
    const rightWidth = Number(value.rightWidth);
    chatSidePanelWidths.value = {
      leftWidth: Number.isFinite(leftWidth) ? leftWidth : 320,
      rightWidth: Number.isFinite(rightWidth) ? rightWidth : 320,
    };
  }

  async function toggleSideConversationList() {
    const nextVisible = !sideConversationListVisible.value;
    sideConversationListVisible.value = nextVisible;
    storeChatSidePanelVisibility("left", nextVisible);
  }

  async function toggleToolReviewPanel() {
    const nextVisible = !toolReviewPanelOpenVisible.value;
    toolReviewPanelOpenVisible.value = nextVisible;
    storeChatSidePanelVisibility("right", nextVisible);
    if (!nextVisible) {
      _panelOpenedByFile = false;
    }
  }

  const configSearchResults = computed<ConfigSearchResult[]>(() => {
    if (bindings.viewMode.value !== "config") return [];
    return bindings.searchConfigTabs(configSearchQuery.value, bindings.resolveConfigLocale());
  });

  watch(
    () => String(bindings.currentChatConversationId.value || "").trim(),
    () => {
      selectedChatMentions.value = [];
    },
  );

  return {
    configTab,
    configSearchQuery,
    configSearchResults,
    selectedInstructionPrompts,
    selectedChatMentions,
    chatInput,
    conversationListTab,
    chatLeftPanelMode,
    chatRightPanelMode,
    sideConversationListVisible,
    toolReviewPanelOpenVisible,
    chatSidePanelWidths,
    chatErrorText,
    handleChatInputUpdate,
    updateConfigSearchQuery,
    handleSelectConfigSearchResult,
    updateSelectedInstructionPrompts,
    addChatMention,
    removeChatMention,
    handleSideConversationListVisibleChange,
    handleToolReviewPanelOpenChange,
    updateConversationListTab,
    updateChatLeftPanelMode,
    updateChatRightPanelMode,
    handleChatSidePanelWidthsChange,
    toggleSideConversationList,
    toggleToolReviewPanel,
    setConversationChatErrorText,
    clearMatchingConversationChatErrors,
    clearChatError,
  };
}

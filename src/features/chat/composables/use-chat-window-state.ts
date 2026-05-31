import { reactive, ref, shallowRef } from "vue";
import type { UnlistenFn } from "@tauri-apps/api/event";
import type { TerminalApprovalRequestPayload } from "../../shell/composables/use-terminal-approval";
import type {
  AppConfig,
  ChatMessage,
  ChatTodoItem,
  ImageTextCacheStats,
  PersonaProfile,
  PromptCommandPreset,
  ToolLoadStatus,
} from "../../../types/app";

type UseChatWindowStateOptions = {
  isMacPlatform: boolean;
  t: (key: string) => string;
};

export function useChatWindowState(options: UseChatWindowStateOptions) {
  const DRAFT_ASSISTANT_ID_PREFIX = "__draft_assistant__:";
  const BACKGROUND_CONVERSATION_CACHE_LIMIT = 10;
  const FOREGROUND_SNAPSHOT_RECENT_LIMIT = 4;
  const OLDER_HISTORY_PAGE_SIZE = 2;
  type BackgroundConversationBadgeState = "completed" | "failed";

  const viewMode = ref<"chat" | "archives" | "config">("config");
  const config = reactive<AppConfig>({
    hotkey: "Alt+·",
    uiLanguage: "zh-CN",
    uiFont: "auto",
    webviewZoomPercent: 100,
    githubUpdateMethod: "auto",
    recordHotkey: options.isMacPlatform ? "Option+Space" : "Alt",
    recordBackgroundWakeEnabled: true,
    minRecordSeconds: 1,
    maxRecordSeconds: 60,
    llmRoundLogCapacity: 3,
    messageNotificationEnabled: true,
    messageNotificationSoundEnabled: false,
    selectedApiConfigId: "",
    assistantDepartmentApiConfigId: "",
    visionApiConfigId: undefined,
    toolReviewApiConfigId: undefined,
    sttApiConfigId: undefined,
    sttAutoSend: false,
    terminalShellKind: "auto",
    shellWorkspaces: [],
    mcpServers: [],
    remoteImChannels: [],
    departments: [],
    apiProviders: [],
    apiConfigs: [],
  });
  const recordHotkeyProbeLastSeq = ref(0);
  const recordHotkeyProbeDown = ref(false);
  const chatWindowActiveSynced = ref<boolean | null>(null);
  const tauriWindowLabel = ref("unknown");
  const isChatTauriWindow = ref(false);
  const detachedChatWindow = ref(false);
  const detachedChatConversationId = ref("");
  const detachedTemporaryApiConfigId = ref("");
  const chatWindowEventUnlisteners: Record<string, UnlistenFn | null> = {
    chatHistoryFlushed: null,
    chatRoundStarted: null,
    chatRoundCompleted: null,
    chatRoundFailed: null,
    chatAssistantDelta: null,
    chatStreamRebindRequired: null,
    chatConversationMessagesAfterSynced: null,
    chatConversationMessageAppended: null,
    chatConversationTodosUpdated: null,
    chatConversationPinUpdated: null,
    chatConversationRuntimeStateUpdated: null,
    chatConversationOverviewUpdated: null,
  };
  const currentChatConversationId = ref("");
  const personas = ref<PersonaProfile[]>([]);
  const assistantDepartmentAgentId = ref("default-agent");
  const personaEditorId = ref("default-agent");
  const userAlias = ref(options.t("archives.roleUser"));
  const selectedResponseStyleId = ref("concise");
  const selectedPdfReadMode = ref<"text" | "image">("image");
  const backgroundVoiceScreenshotKeywords = ref("");
  const backgroundVoiceScreenshotMode = ref<"desktop" | "focused_window">("focused_window");
  const instructionPresets = ref<PromptCommandPreset[]>([]);
  const conversationForegroundSyncing = ref(false);
  const backgroundConversationBadgeMap = ref<Record<string, BackgroundConversationBadgeState>>({});
  const conversationMessageCache = ref<Record<string, ChatMessage[]>>({});
  const latestUserText = ref("");
  const latestUserImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);
  const latestAssistantText = ref("");
  const latestReasoningStandardText = ref("");
  const latestReasoningInlineText = ref("");
  const latestOwnMessageAlignRequest = ref(0);
  const toolStatusText = ref("");
  const toolStatusState = ref<"running" | "done" | "failed" | "">("");
  const streamToolCalls = ref<Array<{ name: string; argsText: string }>>([]);
  const clipboardImages = ref<Array<{ mime: string; bytesBase64: string; savedPath?: string }>>([]);
  const queuedAttachmentNotices = ref<Array<{ id: string; fileName: string; relativePath: string; mime: string }>>([]);
  const allMessages = shallowRef<ChatMessage[]>([]);
  const status = ref("Ready.");
  const terminalApprovalQueue = ref<TerminalApprovalRequestPayload[]>([]);
  const terminalApprovalResolving = ref(false);
  const loading = ref(false);
  const saving = ref(false);
  const startupDataReady = ref(false);
  const startupOverlayVisible = ref(false);
  const startupOverlayMessage = ref("等待后端加载中...");
  const chatting = ref(false);
  const trimming = ref(false);
  const compactingConversation = ref(false);
  const trimmingConversationId = ref("");
  const compactingConversationId = ref("");
  const suppressNextCompactionReload = ref(false);
  const branchingConversation = ref(false);
  const forwardingConversationSelection = ref(false);
  const hasMoreBackendHistory = ref(false);
  const loadingOlderConversationHistory = ref(false);
  const refreshingModels = ref(false);
  const modelRefreshError = ref("");
  const modelRefreshOkFlags = ref<Record<string, boolean>>({});
  const checkingToolsStatus = ref(false);
  const toolStatuses = ref<ToolLoadStatus[]>([]);
  const imageCacheStats = ref<ImageTextCacheStats>({ entries: 0, totalChars: 0 });
  const imageCacheStatsLoading = ref(false);
  const avatarSaving = ref(false);
  const avatarError = ref("");
  const personaSaving = ref(false);
  const apiModelOptions = ref<Record<string, string[]>>({});
  const suppressAutosave = ref(false);
  const lastSavedConfigJson = ref("");
  const lastSavedPersonasJson = ref("");
  const PERF_DEBUG = import.meta.env.DEV;
  const CHAT_STREAM_DEBUG = false;
  const toolReviewRefreshTick = ref(0);
  const currentChatTodos = ref<ChatTodoItem[]>([]);
  const foregroundTailLatestReady = ref(true);

  return {
    DRAFT_ASSISTANT_ID_PREFIX,
    BACKGROUND_CONVERSATION_CACHE_LIMIT,
    FOREGROUND_SNAPSHOT_RECENT_LIMIT,
    OLDER_HISTORY_PAGE_SIZE,
    viewMode,
    config,
    recordHotkeyProbeLastSeq,
    recordHotkeyProbeDown,
    chatWindowActiveSynced,
    tauriWindowLabel,
    isChatTauriWindow,
    detachedChatWindow,
    detachedChatConversationId,
    detachedTemporaryApiConfigId,
    chatWindowEventUnlisteners,
    currentChatConversationId,
    personas,
    assistantDepartmentAgentId,
    personaEditorId,
    userAlias,
    selectedResponseStyleId,
    selectedPdfReadMode,
    backgroundVoiceScreenshotKeywords,
    backgroundVoiceScreenshotMode,
    instructionPresets,
    conversationForegroundSyncing,
    backgroundConversationBadgeMap,
    conversationMessageCache,
    latestUserText,
    latestUserImages,
    latestAssistantText,
    latestReasoningStandardText,
    latestReasoningInlineText,
    latestOwnMessageAlignRequest,
    toolStatusText,
    toolStatusState,
    streamToolCalls,
    clipboardImages,
    queuedAttachmentNotices,
    allMessages,
    status,
    terminalApprovalQueue,
    terminalApprovalResolving,
    loading,
    saving,
    startupDataReady,
    startupOverlayVisible,
    startupOverlayMessage,
    chatting,
    trimming,
    compactingConversation,
    trimmingConversationId,
    compactingConversationId,
    suppressNextCompactionReload,
    branchingConversation,
    forwardingConversationSelection,
    hasMoreBackendHistory,
    loadingOlderConversationHistory,
    refreshingModels,
    modelRefreshError,
    modelRefreshOkFlags,
    checkingToolsStatus,
    toolStatuses,
    imageCacheStats,
    imageCacheStatsLoading,
    avatarSaving,
    avatarError,
    personaSaving,
    apiModelOptions,
    suppressAutosave,
    lastSavedConfigJson,
    lastSavedPersonasJson,
    PERF_DEBUG,
    CHAT_STREAM_DEBUG,
    toolReviewRefreshTick,
    currentChatTodos,
    foregroundTailLatestReady,
  };
}

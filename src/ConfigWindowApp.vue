<template>
  <div class="window-shell text-sm bg-base-200">
    <AppWindowHeader
      view-mode="config"
      :detached-chat-window="false"
      :current-theme="currentTheme"
      :title-text="t('window.configTitle')"
      :chat-usage-percent="0"
      :trimming="false"
      :chatting="false"
      :current-persona-name="String(selectedPersonaEditor?.name || '').trim() || t('archives.roleAssistant')"
      :side-conversation-list-visible="false"
      :tool-review-panel-open-visible="false"
      :chat-side-panel-widths="{ leftWidth: 0, rightWidth: 0 }"
      conversation-list-tab="local"
      chat-left-panel-mode="local"
      chat-right-panel-mode="reader"
      active-conversation-id=""
      :conversation-items="[]"
      :user-alias="userAlias"
      :user-avatar-url="userAvatarUrl"
      :persona-name-map="chatPersonaNameMap"
      :persona-avatar-url-map="chatPersonaAvatarUrlMap"
      :create-conversation-department-options="[]"
      default-create-conversation-department-id=""
      :trim-tip="t('chat.trimTip')"
      :maximized="maximized"
      :window-ready="windowReady"
      :open-settings-title="t('window.configTitle')"
      :close-title="t('common.close')"
      :config-search-query="configSearchQuery"
      :config-search-results="configSearchResults"
      :config-search-placeholder="t('config.search.placeholder')"
      :show-update-to-latest-button="showUpdateToLatestButton"
      :has-available-update="hasAvailableUpdate"
      :checking-update="checkingUpdate"
      :update-to-latest-label="updateToLatestLabel"
      :update-to-latest-title="updateToLatestTitle"
      @update:config-search-query="updateConfigSearchQuery"
      @select-config-search-result="handleSelectConfigSearchResult"
      @update-to-latest="triggerUpdateToLatest"
      @minimize-window="minimizeWindow"
      @toggle-maximize-window="toggleMaximizeWindow"
      @close-window="closeWindow"
    />

    <div class="window-content p-0 min-h-0 overflow-hidden">
      <ConfigView
        :config="config"
        :config-tab="configTab"
        :ui-language="config.uiLanguage"
        :locale-options="localeOptions"
        :current-theme="currentTheme"
        :generated-theme-controls="generatedThemeControls"
        :generated-theme-tokens="generatedThemeTokens"
        :webview-zoom-percent="config.webviewZoomPercent ?? 100"
        :selected-api-config="selectedApiConfig"
        :tool-api-config="toolApiConfig"
        :base-url-reference="baseUrlReference"
        :refreshing-models="refreshingModels"
        :model-options="selectedModelOptions"
        :model-refresh-ok="selectedModelRefreshOk"
        :model-refresh-error="modelRefreshError"
        :tool-statuses="toolStatuses"
        :personas="personas"
        :assistant-personas="assistantPersonas"
        :user-persona="userPersona"
        :persona-editor-id="personaEditorId"
        :assistant-department-agent-id="assistantDepartmentAgentId"
        :selected-persona="selectedPersonaEditor"
        :tool-persona="selectedPersonaEditor"
        :selected-persona-avatar-url="selectedPersonaEditorAvatarUrl"
        :user-persona-avatar-url="userPersonaAvatarUrl"
        :response-style-options="responseStyleOptions"
        :response-style-id="selectedResponseStyleId"
        :pdf-read-mode="selectedPdfReadMode"
        :background-voice-screenshot-keywords="backgroundVoiceScreenshotKeywords"
        :background-voice-screenshot-mode="backgroundVoiceScreenshotMode"
        :instruction-presets="instructionPresets"
        :text-capable-api-configs="textCapableApiConfigs"
        :image-capable-api-configs="imageCapableApiConfigs"
        :stt-capable-api-configs="sttCapableApiConfigs"
        :cache-stats="imageCacheStats"
        :cache-stats-loading="imageCacheStatsLoading"
        :avatar-saving="avatarSaving"
        :avatar-error="avatarError"
        :persona-saving="personaSaving"
        :persona-dirty="personaDirty"
        :config-dirty="configDirty"
        :saving-config="saving"
        :normalize-api-bindings-action="normalizeApiBindingsLocal"
        :hotkey-test-recording="hotkeyTestRecording"
        :hotkey-test-recording-ms="hotkeyTestRecordingMs"
        :hotkey-test-audio-ready="!!hotkeyTestAudio"
        :checking-update="checkingUpdate"
        :has-available-update="hasAvailableUpdate"
        :save-config-action="saveConfig"
        :restore-config-action="restoreLastSavedConfigSnapshot"
        :last-saved-config-json="lastSavedConfigJson"
        :set-status-action="setStatus"
        @update:config-tab="(value) => { configTab = value; }"
        @update:ui-language="setUiLanguage"
        @update:persona-editor-id="updatePersonaEditorIdWithNotice"
        @update:assistant-department-agent-id="updateAssistantDepartmentAgentId"
        @update:response-style-id="(value) => { selectedResponseStyleId = value; }"
        @update:pdf-read-mode="(value) => { selectedPdfReadMode = value; }"
        @update:background-voice-screenshot-keywords="(value) => { backgroundVoiceScreenshotKeywords = String(value || '').replace(/，/g, ','); }"
        @update:background-voice-screenshot-mode="(value) => { backgroundVoiceScreenshotMode = value; }"
        @update:instruction-presets="updateInstructionPresets"
        @patch-conversation-api-settings="patchConversationApiSettings"
        @patch-chat-settings="patchChatSettings"
        @update:webview-zoom-percent="updateWebviewZoomPercent"
        @update:github-update-method="updateGithubUpdateMethod"
        @set-theme="setTheme"
        @activate-generated-theme="activateGeneratedTheme"
        @update-generated-theme-controls="updateGeneratedThemeControls"
        @reset-generated-theme="resetGeneratedTheme"
        @refresh-models="refreshModels"
        @tool-switch-changed="handleToolsChanged"
        @save-api-config="saveConfig"
        @add-api-config="addApiConfig"
        @remove-selected-api-config="removeSelectedApiConfig"
        @add-persona="addPersona"
        @remove-selected-persona="removeSelectedPersona"
        @reset-personas="loadPersonas"
        @save-personas="savePersonas"
        @import-persona-memories="importPersonaMemories"
        @open-conversation-list="openConversationList"
        @open-prompt-preview="openPromptPreviewFromConfig"
        @open-system-prompt-preview="openSystemPromptPreviewFromConfig"
        @open-memory-viewer="openMemoryViewer"
        @refresh-image-cache-stats="refreshImageCacheStats"
        @clear-image-cache="clearImageCache"
        @open-runtime-logs="openRuntimeLogs"
        @start-hotkey-record-test="startHotkeyRecordTest"
        @stop-hotkey-record-test="stopHotkeyRecordTest"
        @play-hotkey-record-test="playHotkeyRecordTest"
        @capture-hotkey="captureHotkey"
        @summon-chat-now="summonChatNow"
        @save-agent-avatar="saveAgentAvatar"
        @clear-agent-avatar="clearAgentAvatar"
        @check-update="manualCheckGithubUpdate"
        @open-github="openGithubRepository"
      />
    </div>

    <dialog ref="memoryDialog" class="modal">
      <MemoryDialog
        :title="t('memory.title')"
        :empty-text="t('memory.empty')"
        :page-text="t('memory.page', { page: memoryPage, total: memoryPageCount })"
        :prev-page-text="t('memory.prevPage')"
        :next-page-text="t('memory.nextPage')"
        :export-text="t('memory.export')"
        :import-text="t('memory.import')"
        :close-text="t('common.close')"
        :memory-list="memoryList"
        :paged-memories="pagedMemories"
        :memory-page="memoryPage"
        :memory-page-count="memoryPageCount"
        @close="closeMemoryViewer"
        @prev-page="() => { memoryPage -= 1; }"
        @next-page="() => { memoryPage += 1; }"
        @export-memories="exportMemories"
        @trigger-import="triggerMemoryImport"
        @import-file="handleMemoryImportFile"
      />
    </dialog>

    <dialog ref="promptPreviewDialog" class="modal">
      <PromptPreviewDialog
        :mode="promptPreviewMode"
        :loading="promptPreviewLoading"
        :title="promptPreviewMode === 'system' ? t('prompt.systemPreview') : t('prompt.requestPreview')"
        :loading-text="t('common.loading')"
        :empty-hint="t('prompt.emptyHint')"
        :chat-text="t('prompt.chat')"
        :compaction-text="t('prompt.compaction')"
        :archive-text="t('prompt.archive')"
        :conversation-text="t('prompt.conversation')"
        :selected-conversation-id="promptPreviewConversationId"
        :conversation-options="promptPreviewConversationOptions"
        :latest-input-length-text="t('prompt.latestInputLength')"
        :images-text="t('prompt.images')"
        :audios-text="t('prompt.audios')"
        :close-text="t('common.close')"
        :latest-user-text="promptPreviewLatestUserText"
        :latest-images="promptPreviewLatestImages"
        :latest-audios="promptPreviewLatestAudios"
        :text="promptPreviewText"
        @select-mode="loadPromptPreview"
        @select-conversation="selectPromptPreviewConversation"
        @close="closePromptPreview"
      />
    </dialog>

    <ShellDialogsHost
      :update-dialog-open="updateDialogOpen"
      :update-dialog-title="updateDialogTitle"
      :update-dialog-body="updateDialogBody"
      :update-dialog-kind="updateDialogKind"
      :update-dialog-release-url="updateDialogReleaseUrl"
      :update-dialog-primary-action="updateDialogPrimaryAction"
      :update-progress-percent="updateProgressPercent"
      :runtime-logs-dialog-open="false"
      :runtime-logs="[]"
      :runtime-logs-loading="false"
      runtime-logs-error=""
      :rewind-confirm-dialog-open="false"
      :rewind-confirm-can-undo-patch="false"
      :config-save-error-dialog-open="configSaveErrorDialogOpen"
      :config-save-error-dialog-title="configSaveErrorDialogTitle"
      :config-save-error-dialog-body="configSaveErrorDialogBody"
      :config-save-error-dialog-kind="configSaveErrorDialogKind"
      :archive-import-preview-dialog-open="false"
      :archive-import-preview="null"
      :archive-import-running="false"
      :skill-placeholder-dialog-open="false"
      :trim-action-dialog-open="false"
      :trim-preview-loading="false"
      :trim-preview="null"
      :trim-compaction-preview="null"
      :trimming="false"
      @close-update-dialog="closeUpdateDialog"
      @confirm-update-dialog-primary="confirmUpdateDialogPrimary"
      @open-update-release="openUpdateRelease"
      @close-settings-save-error-dialog="closeSettingsSaveErrorDialog"
    />

    <div
      v-if="startupOverlayVisible"
      class="fixed inset-0 z-9998 flex items-center justify-center bg-base-300/90 p-6 backdrop-blur"
    >
      <div class="flex min-w-72 max-w-sm items-center gap-3 rounded-box border border-base-content/10 bg-base-100 px-5 py-4 shadow-2xl">
        <span class="loading loading-spinner loading-md text-primary"></span>
        <div class="min-w-0">
          <div class="font-medium">{{ startupOverlayMessage }}</div>
          <div class="mt-1 text-xs opacity-60">请稍候...</div>
        </div>
      </div>
    </div>

    <Win10ResizeHandles :enabled="!maximized" />
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ConfigView from "./features/config/views/ConfigView.vue";
import AppWindowHeader from "./features/shell/components/AppWindowHeader.vue";
import ShellDialogsHost from "./features/shell/components/ShellDialogsHost.vue";
import Win10ResizeHandles from "./features/shell/components/Win10ResizeHandles.vue";
import MemoryDialog from "./features/memory/components/dialogs/MemoryDialog.vue";
import PromptPreviewDialog from "./features/chat/components/dialogs/PromptPreviewDialog.vue";
import { invokeTauri } from "./services/tauri-api";
import type { AppConfig, PromptCommandPreset } from "./types/app";
import { normalizeLocale } from "./i18n";
import { useWindowShell } from "./features/shell/composables/use-window-shell";
import { useAppTheme } from "./features/shell/composables/use-app-theme";
import { useAppLifecycle } from "./features/shell/composables/use-app-lifecycle";
import { useAppCore } from "./features/shell/composables/use-app-core";
import { useConfigCore } from "./features/config/composables/use-config-core";
import { useConfigRuntime } from "./features/config/composables/use-config-runtime";
import { useConfigPersistence } from "./features/config/composables/use-config-persistence";
import { useConfigEditors } from "./features/config/composables/use-config-editors";
import { useAppWatchers } from "./features/shell/composables/use-app-watchers";
import { searchConfigTabs, type ConfigSearchTab } from "./features/config/search/config-search";
import { applyUiFont, normalizeUiFont } from "./features/shell/composables/use-ui-font";
import { useWebviewZoomOrchestrator } from "./features/shell/composables/use-webview-zoom-orchestrator";
import { useGithubUpdateView } from "./features/shell/composables/use-github-update-view";
import { useConfigSaveErrorDialog } from "./features/shell/composables/use-config-save-error-dialog";
import { useWindowActions } from "./features/shell/composables/use-window-actions";
import { useAvatarCache } from "./features/chat/composables/use-avatar-cache";
import { useMemoryViewer } from "./features/memory/composables/use-memory-viewer";
import { usePromptPreview } from "./features/chat/composables/use-prompt-preview";
import { useHotkeyRecordTest } from "./features/shell/composables/use-hotkey-record-test";
import { useChatConfigActionsGlue } from "./features/chat/composables/use-chat-config-actions-glue";
import { useChatConfigDerivedState } from "./features/chat/composables/use-chat-config-derived-state";
import { useChatConfigUiDerivedState } from "./features/chat/composables/use-chat-config-ui-derived-state";
import { useConfigWindowBootstrap } from "./features/config/composables/use-config-window-bootstrap";

const { t, locale } = useI18n();
const tr = (key: string, params?: Record<string, unknown>) => t(key, params as never);
const isMacPlatform = /Mac|iPhone|iPad|iPod/i.test(window.navigator.platform || "");

type ConfigTab =
  | "welcome"
  | "hotkey"
  | "api"
  | "tools"
  | "mcp"
  | "skill"
  | "persona"
  | "department"
  | "departmentTree"
  | "demo"
  | "chatSettings"
  | "notification"
  | "remoteIm"
  | "memory"
  | "task"
  | "logs"
  | "appearance"
  | "migration"
  | "about";

const config = reactive<AppConfig>({
  hotkey: "Alt+·",
  uiLanguage: "zh-CN",
  uiFont: "auto",
  webviewZoomPercent: 100,
  githubUpdateMethod: "auto",
  recordHotkey: isMacPlatform ? "Option+Space" : "Alt",
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

const personas = ref([] as import("./types/app").PersonaProfile[]);
const assistantDepartmentAgentId = ref("default-agent");
const personaEditorId = ref("default-agent");
const userAlias = ref(t("archives.roleUser"));
const selectedResponseStyleId = ref("concise");
const selectedPdfReadMode = ref<"text" | "image">("image");
const backgroundVoiceScreenshotKeywords = ref("");
const backgroundVoiceScreenshotMode = ref<"desktop" | "focused_window">("focused_window");
const instructionPresets = ref<PromptCommandPreset[]>([]);
const configTab = ref<ConfigTab>("hotkey");
const configSearchQuery = ref("");

const status = ref("");
const suppressAutosave = ref(false);
const loading = ref(false);
const saving = ref(false);
const personaSaving = ref(false);
const lastSavedConfigJson = ref("");
const lastSavedPersonasJson = ref("");

const refreshingModels = ref(false);
const modelRefreshError = ref("");
const modelRefreshOkFlags = ref<Record<string, boolean>>({});
const apiModelOptions = ref<Record<string, string[]>>({});
const checkingToolsStatus = ref(false);
const toolStatuses = ref([] as import("./types/app").ToolLoadStatus[]);
const imageCacheStats = ref<import("./types/app").ImageTextCacheStats>({ entries: 0, totalChars: 0 });
const imageCacheStatsLoading = ref(false);
const avatarSaving = ref(false);
const avatarError = ref("");

const viewMode = ref<"chat" | "archives" | "config">("config");
const {
  windowReady,
  maximized,
  initWindow,
  syncWindowControlsState,
  closeWindow,
  minimizeWindow,
  toggleMaximizeWindow,
} = useWindowShell();

const {
  currentTheme,
  generatedThemeControls,
  generatedThemeTokens,
  applyTheme,
  setTheme,
  activateGeneratedTheme,
  updateGeneratedThemeControls,
  resetGeneratedTheme,
  restoreThemeFromStorage,
} = useAppTheme();

const {
  setStatus,
  setStatusError,
  localeOptions,
  applyUiLanguage,
} = useAppCore({
  t: tr,
  config,
  locale,
  status,
  perfDebug: false,
});
const {
  hotkeyTestRecording,
  hotkeyTestRecordingMs,
  hotkeyTestAudio,
  startHotkeyRecordTest,
  stopHotkeyRecordTest,
  playHotkeyRecordTest,
  cleanupHotkeyRecordTest,
} = useHotkeyRecordTest({
  t: tr,
  setStatus,
  setStatusError,
});
const startupOverlayVisible = ref(false);
const startupOverlayMessage = ref("等待后端加载中...");
const {
  normalizeWebviewZoomPercent,
  updateWebviewZoomPercent,
  updateGithubUpdateMethod,
} = useWebviewZoomOrchestrator({
  config,
  setStatusError,
});

const {
  selectedApiConfig,
  selectedApiProvider,
  textCapableApiConfigs,
  imageCapableApiConfigs,
  sttCapableApiConfigs,
  normalizeRuntimeConfigNumbers,
} = useChatConfigDerivedState(config);

const userPersona = computed(() => personas.value.find((p) => p.isBuiltInUser || p.id === "user-persona") ?? null);
const assistantPersonas = computed(() =>
  personas.value.filter((p) => !p.isBuiltInUser && !p.isBuiltInSystem && p.id !== "user-persona" && p.id !== "system-persona"),
);
const selectedPersonaEditor = computed(() => personas.value.find((p) => p.id === personaEditorId.value) ?? null);
const toolDepartment = computed(() =>
  config.departments.find((item) => item.id === "assistant-department" || item.isBuiltInAssistant)
  ?? config.departments.find((item) => (item.agentIds || []).includes(assistantDepartmentAgentId.value))
  ?? null,
);
const toolApiConfig = computed(() => config.apiConfigs.find((a) => a.id === (toolDepartment.value?.apiConfigId || "")) ?? null);

const { resolveAvatarUrl, ensureAvatarCached, preloadPersonaAvatars } = useAvatarCache({ personas });
const userAvatarUrl = computed(() => resolveAvatarUrl(userPersona.value?.avatarPath, userPersona.value?.avatarUpdatedAt));
const userPersonaAvatarUrl = computed(() => userAvatarUrl.value);
const selectedPersonaEditorAvatarUrl = computed(() => resolveAvatarUrl(selectedPersonaEditor.value?.avatarPath, selectedPersonaEditor.value?.avatarUpdatedAt));
const chatPersonaNameMap = computed<Record<string, string>>(() => {
  const next: Record<string, string> = {};
  for (const persona of personas.value) {
    const id = String(persona.id || "").trim();
    if (!id) continue;
    next[id] = String(persona.name || "").trim() || id;
  }
  return next;
});
const chatPersonaAvatarUrlMap = computed<Record<string, string>>(() => {
  const next: Record<string, string> = {};
  for (const persona of personas.value) {
    const id = String(persona.id || "").trim();
    if (!id) continue;
    const url = resolveAvatarUrl(persona.avatarPath, persona.avatarUpdatedAt);
    if (url) next[id] = url;
  }
  return next;
});

const configSearchResults = computed(() => searchConfigTabs(configSearchQuery.value, normalizeLocale(config.uiLanguage)));

const {
  defaultApiTools,
  createApiProvider,
  createApiConfig,
  normalizeApiBindingsLocal,
  buildConfigPayload,
  buildConfigSnapshotJson,
} = useConfigCore({
  config,
  textCapableApiConfigs,
});

const {
  selectedModelOptions,
  selectedModelRefreshOk,
  responseStyleOptions,
  baseUrlReference,
  configDirty,
  personaDirty,
  responseStyleIds,
} = useChatConfigUiDerivedState({
  config,
  apiModelOptions,
  modelRefreshOkFlags,
  selectedApiConfig,
  personas,
  lastSavedConfigJson,
  lastSavedPersonasJson,
  buildConfigSnapshotJson,
  t: tr,
});

const {
  syncTrayIcon,
  saveAgentAvatar,
  clearAgentAvatar,
  refreshModels,
  refreshToolsStatus,
  refreshImageCacheStats,
  clearImageCache,
} = useConfigRuntime({
  t: tr,
  setStatus,
  setStatusError,
  personas,
  assistantDepartmentAgentId,
  toolAgentId: assistantDepartmentAgentId,
  avatarSaving,
  avatarError,
  selectedApiConfig,
  selectedApiProvider,
  refreshingModels,
  modelRefreshError,
  apiModelOptions,
  modelRefreshOkFlags,
  toolApiConfig,
  checkingToolsStatus,
  toolStatuses,
  imageCacheStats,
  imageCacheStatsLoading,
  ensureAvatarCached,
});

function syncUserAliasFromPersona() {
  const next = (userPersona.value?.name || "").trim() || t("archives.roleUser");
  if (userAlias.value !== next) {
    userAlias.value = next;
  }
}

const {
  configSaveErrorDialogOpen,
  configSaveErrorDialogTitle,
  configSaveErrorDialogBody,
  configSaveErrorDialogKind,
  closeSettingsSaveErrorDialog,
  openSettingsSaveErrorDialog,
} = useConfigSaveErrorDialog({
  t: tr,
  configTab,
});

const {
  buildPersonasSnapshotJson,
  setUiLanguage,
  importPersonaMemories,
  handleToolsChanged,
} = useChatConfigActionsGlue({
  t: tr,
  config,
  locale,
  personas,
  configTab,
  lastSavedConfigJson,
  normalizeLocale,
  applyUiLanguage,
  buildConfigSnapshotJson,
  refreshToolsStatus,
  setStatus,
  setStatusError,
});

const configPersistence = useConfigPersistence({
  t: tr,
  setStatus,
  setStatusError,
  onSaveConfigError: openSettingsSaveErrorDialog,
  config,
  locale,
  normalizeLocale,
  suppressAutosave,
  loading,
  saving,
  savingPersonas: personaSaving,
  personas,
  assistantPersonas,
  assistantDepartmentAgentId,
  personaEditorId,
  userAlias,
  selectedResponseStyleId,
  selectedPdfReadMode,
  backgroundVoiceScreenshotKeywords,
  backgroundVoiceScreenshotMode,
  instructionPresets,
  responseStyleIds,
  createApiConfig,
  normalizeApiBindingsLocal,
  buildConfigPayload,
  buildConfigSnapshotJson,
  buildPersonasSnapshotJson,
  lastSavedConfigJson,
  lastSavedPersonasJson,
  syncUserAliasFromPersona,
  preloadPersonaAvatars,
  syncTrayIcon,
});

const {
  loadBootstrapSnapshot,
  saveConfig,
  captureHotkey,
  loadPersonas,
  savePersonas,
  patchChatSettings,
  patchConversationApiSettings,
  restoreLastSavedConfigSnapshot,
} = configPersistence;

const {
  addApiConfig,
  removeSelectedApiConfig,
  addPersona,
  removeSelectedPersona,
} = useConfigEditors({
  t: tr,
  config,
  personas,
  assistantPersonas,
  assistantDepartmentAgentId,
  personaEditorId,
  selectedPersonaEditor,
  createApiConfig,
  createApiProvider,
  normalizeApiBindingsLocal,
});

const {
  checkingUpdate,
  hasAvailableUpdate,
  updateDialogOpen,
  updateDialogTitle,
  updateDialogBody,
  updateDialogKind,
  updateDialogReleaseUrl,
  updateDialogPrimaryAction,
  updateProgressPercent,
  closeUpdateDialog,
  openUpdateRelease,
  confirmUpdateDialogPrimary,
  autoCheckGithubUpdate,
  manualCheckGithubUpdate,
  triggerUpdateToLatest,
  showUpdateToLatestButton,
  updateToLatestLabel,
  updateToLatestTitle,
} = useGithubUpdateView({
  t: tr,
  viewMode,
  status,
  updateMethod: computed(() => config.githubUpdateMethod || "auto"),
});

const { summonChatWindowFromConfig, openGithubRepository } = useWindowActions({
  isChatTauriWindow: computed(() => false),
  closeWindow,
  minimizeWindow,
  freezeForegroundConversation: () => undefined,
});

const {
  memoryDialog,
  memoryList,
  memoryPage,
  memoryPageCount,
  pagedMemories,
  openMemoryViewer,
  closeMemoryViewer,
  exportMemories,
  triggerMemoryImport,
  handleMemoryImportFile,
} = useMemoryViewer({
  t: tr,
  setStatus,
  setStatusError,
});

const promptPreviewCurrentConversationId = ref("");
const promptPreviewLocalConversations = computed(() => [] as import("./types/app").UnarchivedConversationSummary[]);
const {
  promptPreviewDialog,
  promptPreviewLoading,
  promptPreviewText,
  promptPreviewLatestUserText,
  promptPreviewLatestImages,
  promptPreviewLatestAudios,
  promptPreviewMode,
  promptPreviewConversationId,
  promptPreviewConversationOptions,
  loadPromptPreview,
  openPromptPreview,
  openSystemPromptPreview,
  selectPromptPreviewConversation,
  closePromptPreview,
} = usePromptPreview({
  t: tr,
  currentConversationId: promptPreviewCurrentConversationId,
  localConversations: promptPreviewLocalConversations,
});

function updateConfigSearchQuery(value: string) {
  configSearchQuery.value = String(value || "");
}

function handleSelectConfigSearchResult(tab: ConfigSearchTab) {
  configTab.value = tab;
  configSearchQuery.value = "";
}

function updatePersonaEditorIdWithNotice(value: string) {
  const nextId = String(value || "").trim();
  if (!nextId || nextId === personaEditorId.value) return;
  if (personaDirty.value) {
    const currentName = String(selectedPersonaEditor.value?.name || personaEditorId.value || "").trim() || t("config.persona.title");
    setStatus(t("status.personaUnsavedSwitchHint", { name: currentName }));
  }
  personaEditorId.value = nextId;
}

function updateAssistantDepartmentAgentId(value: string) {
  assistantDepartmentAgentId.value = String(value || "").trim();
}

function updateInstructionPresets(value: PromptCommandPreset[]) {
  instructionPresets.value = Array.isArray(value)
    ? value
      .map((item) => ({
        id: String(item?.id || "").trim(),
        name: String(item?.prompt || item?.name || "").trim(),
        prompt: String(item?.prompt || item?.name || "").trim(),
      }))
      .filter((item) => !!item.id && !!item.prompt)
    : [];
}

async function openConversationList() {
  try {
    await invokeTauri("show_archives_window");
  } catch (error) {
    setStatusError("status.requestFailed", error);
  }
}

async function openPromptPreviewFromConfig() {
  const apiConfigId = String(config.assistantDepartmentApiConfigId || config.selectedApiConfigId || "").trim();
  const agentId = String(assistantDepartmentAgentId.value || "").trim();
  if (!apiConfigId || !agentId) return;
  await openPromptPreview(apiConfigId, agentId);
}

async function openSystemPromptPreviewFromConfig() {
  const apiConfigId = String(config.assistantDepartmentApiConfigId || config.selectedApiConfigId || "").trim();
  const agentId = String(assistantDepartmentAgentId.value || "").trim();
  if (!apiConfigId || !agentId) return;
  await openSystemPromptPreview(apiConfigId, agentId);
}

function openRuntimeLogs() {
  void invokeTauri("open_runtime_logs_window").catch((error) => {
    console.warn("[运行日志] 打开日志窗口失败", error);
  });
}

function summonChatNow() {
  summonChatWindowFromConfig();
}

async function refreshAllViewData() {
  await loadBootstrapSnapshot();
  await refreshImageCacheStats();
}

const appBootstrap = useConfigWindowBootstrap({
  viewMode,
  initWindow,
  applyTheme,
  normalizeLocale,
  config,
  locale,
  assistantDepartmentAgentId,
  personaEditorId,
  userAlias,
  selectedResponseStyleId,
  selectedPdfReadMode,
  backgroundVoiceScreenshotKeywords,
  backgroundVoiceScreenshotMode,
  instructionPresets,
  createApiConfig,
  buildConfigSnapshotJson,
  lastSavedConfigJson,
  normalizeWebviewZoomPercent,
  updateGithubUpdateMethod,
  normalizeRuntimeConfigNumbers,
});

useAppLifecycle({
  appBootstrapMount: appBootstrap.mount,
  appBootstrapUnmount: appBootstrap.unmount,
  restoreThemeFromStorage,
  onPaste: () => undefined,
  onDragOver: (event) => { event.preventDefault(); },
  onDrop: (event) => { event.preventDefault(); },
  recordHotkeyMount: () => undefined,
  recordHotkeyUnmount: () => undefined,
  refreshAllViewData,
  viewMode,
  syncWindowControlsState,
  stopRecording: async () => undefined,
  cleanupSpeechRecording: () => undefined,
  cleanupChatMedia: cleanupHotkeyRecordTest,
  onStartupOverlayChange: (visible, message) => {
    startupOverlayVisible.value = visible;
    startupOverlayMessage.value = message || "等待后端加载中...";
  },
  afterMountedReady: async () => {
    await autoCheckGithubUpdate();
  },
});

useAppWatchers({
  config,
  configTab,
  viewMode,
  personas,
  userPersona,
  assistantPersonas,
  assistantDepartmentAgentId,
  personaEditorId,
  selectedApiConfig,
  toolApiConfig,
  modelRefreshError,
  toolStatuses,
  defaultApiTools,
  t: tr,
  normalizeApiBindingsLocal,
  syncUserAliasFromPersona,
  syncTrayIcon,
  refreshToolsStatus,
  refreshImageCacheStats,
});

watch(
  () => ({ uiFont: config.uiFont, uiLanguage: config.uiLanguage }),
  ({ uiFont, uiLanguage }) => {
    applyUiFont(uiFont, uiLanguage);
    config.uiFont = normalizeUiFont(uiFont);
  },
  { immediate: true },
);

</script>

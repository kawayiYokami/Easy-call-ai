<template>
  <div class="window-shell text-sm bg-base-200">
    <AppWindowHeader
      view-mode="archives"
      :detached-chat-window="false"
      :current-theme="currentTheme"
      :title-text="t('window.archivesTitle')"
      :chat-usage-percent="0"
      :trimming="false"
      :chatting="false"
      :current-persona-name="t('archives.roleAssistant')"
      :side-conversation-list-visible="false"
      :tool-review-panel-open-visible="false"
      :chat-side-panel-widths="{ leftWidth: 0, rightWidth: 0 }"
      conversation-list-tab="local"
      chat-left-panel-mode="local"
      chat-right-panel-mode="reader"
      active-conversation-id=""
      :conversation-items="[]"
      :user-alias="userAlias"
      user-avatar-url=""
      :persona-name-map="personaNameMap"
      :persona-avatar-url-map="{}"
      :create-conversation-department-options="[]"
      default-create-conversation-department-id=""
      :trim-tip="t('chat.trimTip')"
      :maximized="maximized"
      :window-ready="windowReady"
      :open-settings-title="t('window.configTitle')"
      :close-title="t('common.close')"
      @minimize-window="minimizeWindow"
      @toggle-maximize-window="toggleMaximizeWindow"
      @close-window="closeWindow"
    />

    <div class="window-content p-0 min-h-0 overflow-hidden">
      <ArchivesView
        :archives="archives"
        :selected-archive-id="selectedArchiveId"
        :archive-blocks="archiveBlocks"
        :selected-archive-block-id="selectedArchiveBlockId"
        :archive-has-prev-block="archiveHasPrevBlock"
        :archive-has-next-block="archiveHasNextBlock"
        :archive-messages="archiveMessages"
        :archive-summary-text="archiveSummaryText"
        :unarchived-conversations="unarchivedConversations"
        :unarchived-blocks="unarchivedBlocks"
        :selected-unarchived-conversation-id="selectedUnarchivedConversationId"
        :selected-unarchived-block-id="selectedUnarchivedBlockId"
        :unarchived-has-prev-block="unarchivedHasPrevBlock"
        :unarchived-has-next-block="unarchivedHasNextBlock"
        :unarchived-messages="unarchivedMessages"
        :delegate-conversations="delegateConversations"
        :selected-delegate-conversation-id="selectedDelegateConversationId"
        :delegate-messages="delegateMessages"
        :remote-im-contact-conversations="remoteImContactConversations"
        :remote-im-contact-blocks="remoteImContactBlocks"
        :selected-remote-im-contact-id="selectedRemoteImContactId"
        :selected-remote-im-contact-block-id="selectedRemoteImContactBlockId"
        :remote-im-has-prev-block="remoteImHasPrevBlock"
        :remote-im-has-next-block="remoteImHasNextBlock"
        :remote-im-contact-messages="remoteImContactMessages"
        :user-alias="userAlias"
        :persona-name-map="personaNameMap"
        :current-theme="currentTheme"
        @load-archives="loadArchives"
        @select-archive="selectArchive"
        @select-archive-block="selectArchiveBlock"
        @select-unarchived-conversation="selectUnarchivedConversation"
        @select-unarchived-block="selectUnarchivedConversationBlock"
        @select-delegate-conversation="selectDelegateConversation"
        @select-remote-im-contact-conversation="selectRemoteImContactConversation"
        @select-remote-im-contact-block="selectRemoteImContactConversationBlock"
        @export-archive="exportArchive"
        @import-archive-file="prepareArchiveImport"
        @delete-archive="deleteArchive"
        @delete-unarchived-conversation="deleteUnarchivedConversation"
        @delete-delegate-conversation="deleteDelegateConversation"
        @delete-remote-im-contact-conversation="deleteRemoteImContactConversation"
      />
    </div>

    <ShellDialogsHost
      :update-dialog-open="false"
      update-dialog-title=""
      update-dialog-body=""
      update-dialog-kind="info"
      :runtime-logs-dialog-open="false"
      :runtime-logs="[]"
      :runtime-logs-loading="false"
      runtime-logs-error=""
      :rewind-confirm-dialog-open="false"
      :rewind-confirm-can-undo-patch="false"
      :config-save-error-dialog-open="false"
      config-save-error-dialog-title=""
      config-save-error-dialog-body=""
      config-save-error-dialog-kind="error"
      :archive-import-preview-dialog-open="archiveImportPreviewDialogOpen"
      :archive-import-preview="archiveImportPreview"
      :archive-import-running="archiveImportRunning"
      :skill-placeholder-dialog-open="false"
      :trim-action-dialog-open="false"
      :trim-preview-loading="false"
      :trim-preview="null"
      :trim-compaction-preview="null"
      :trimming="false"
      @close-archive-import-preview-dialog="closeArchiveImportPreviewDialog"
      @confirm-archive-import="confirmArchiveImport"
    />

    <div
      v-if="messageStoreMigration.visible"
      class="fixed inset-0 z-9999 flex items-center justify-center bg-base-300/90 p-6 backdrop-blur"
    >
      <div class="w-full max-w-2xl rounded-box border border-base-content/10 bg-base-100 p-6 shadow-2xl">
        <div class="text-xl font-semibold">会话消息仓库迁移</div>
        <div class="mt-2 text-sm opacity-70">{{ messageStoreMigration.message }}</div>
        <progress
          v-if="messageStoreMigration.mode === 'migrating'"
          class="progress progress-primary mt-5 w-full"
          :value="messageStoreMigration.current"
          :max="Math.max(messageStoreMigration.total, 1)"
        />
        <div v-if="messageStoreMigration.mode === 'migrating'" class="mt-2 text-xs opacity-60">
          {{ messageStoreMigration.current }} / {{ messageStoreMigration.total }}
        </div>
        <div
          v-if="messageStoreMigration.blockedItems.length > 0"
          class="mt-5 max-h-64 space-y-2 overflow-auto rounded-box bg-base-200 p-3"
        >
          <div
            v-for="item in messageStoreMigration.blockedItems"
            :key="item.conversationId"
            class="rounded-box bg-base-100 p-3 text-sm"
          >
            <div class="font-medium">{{ item.title || item.conversationId }}</div>
            <div class="mt-1 text-xs opacity-60">{{ item.conversationId }}</div>
            <div class="mt-2 text-error">{{ item.reason || "未知错误" }}</div>
          </div>
        </div>
        <div v-if="messageStoreMigration.mode === 'blocked'" class="mt-5 flex justify-end gap-3">
          <button class="btn btn-ghost" @click="cancelMessageStoreMigration">取消启动</button>
          <button class="btn btn-error" @click="continueMessageStoreMigrationWithDiscard">
            抛弃异常会话并继续迁移
          </button>
        </div>
      </div>
    </div>

    <Win10ResizeHandles :enabled="!maximized" />
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invokeTauri } from "./services/tauri-api";
import type { AppBootstrapSnapshot, AppConfig } from "./types/app";
import { normalizeLocale } from "./i18n";
import AppWindowHeader from "./features/shell/components/AppWindowHeader.vue";
import ArchivesView from "./features/archive/views/ArchivesView.vue";
import ShellDialogsHost from "./features/shell/components/ShellDialogsHost.vue";
import Win10ResizeHandles from "./features/shell/components/Win10ResizeHandles.vue";
import { useWindowShell } from "./features/shell/composables/use-window-shell";
import { useAppTheme } from "./features/shell/composables/use-app-theme";
import { useAppBootstrap } from "./features/shell/composables/use-app-bootstrap";
import { useAppLifecycle } from "./features/shell/composables/use-app-lifecycle";
import { useAppCore } from "./features/shell/composables/use-app-core";
import { applyUiFont, normalizeUiFont } from "./features/shell/composables/use-ui-font";
import { useArchivesView } from "./features/chat/composables/use-archives-view";
import { useArchiveImport } from "./features/chat/composables/use-archive-import";
import { useMessageStoreMigrationGate } from "./features/shell/composables/use-message-store-migration-gate";
import { formatI18nError } from "./utils/error";

const { t, locale } = useI18n();
const tr = (key: string, params?: Record<string, unknown>) => t(key, params as never);

const viewMode = ref<"chat" | "archives" | "config">("archives");
const status = ref("");
const config = reactive<AppConfig>({
  hotkey: "Alt+·",
  uiLanguage: "zh-CN",
  uiFont: "auto",
  webviewZoomPercent: 100,
  githubUpdateMethod: "auto",
  recordHotkey: "Alt",
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
const userAlias = ref(t("archives.roleUser"));

const {
  setStatus,
  setStatusError,
} = useAppCore({
  t: tr,
  config,
  locale,
  status,
  perfDebug: false,
});

const {
  messageStoreMigration,
  ensureMessageStoreMigrationGate,
  cancelMessageStoreMigration,
  continueMessageStoreMigrationWithDiscard,
} = useMessageStoreMigrationGate({
  formatRequestFailed: (error) => formatI18nError(tr, "status.requestFailed", error),
});

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
  applyTheme,
  restoreThemeFromStorage,
} = useAppTheme();

const {
  archives,
  archiveBlocks,
  archiveMessages,
  archiveSummaryText,
  selectedArchiveId,
  selectedArchiveBlockId,
  archiveHasPrevBlock,
  archiveHasNextBlock,
  unarchivedConversations,
  unarchivedBlocks,
  unarchivedMessages,
  selectedUnarchivedConversationId,
  selectedUnarchivedBlockId,
  unarchivedHasPrevBlock,
  unarchivedHasNextBlock,
  delegateConversations,
  delegateMessages,
  selectedDelegateConversationId,
  remoteImContactConversations,
  remoteImContactBlocks,
  remoteImContactMessages,
  selectedRemoteImContactId,
  selectedRemoteImContactBlockId,
  remoteImHasPrevBlock,
  remoteImHasNextBlock,
  selectUnarchivedConversation,
  selectUnarchivedConversationBlock,
  selectDelegateConversation,
  selectRemoteImContactConversation,
  selectRemoteImContactConversationBlock,
  loadArchives,
  selectArchive,
  selectArchiveBlock,
  deleteUnarchivedConversation,
  deleteDelegateConversation,
  deleteRemoteImContactConversation,
  deleteArchive,
  exportArchive,
  buildArchiveImportPreview,
  importArchivePayload,
} = useArchivesView({
  t: tr,
  setStatus,
  setStatusError,
});

const {
  archiveImportPreviewDialogOpen,
  archiveImportPreview,
  archiveImportRunning,
  closeArchiveImportPreviewDialog,
  prepareArchiveImport,
  confirmArchiveImport,
} = useArchiveImport({
  buildArchiveImportPreview,
  importArchivePayload,
  setStatusError,
});

const personaNameMap = computed<Record<string, string>>(() => {
  const next: Record<string, string> = {};
  for (const persona of personas.value) {
    const id = String(persona.id || "").trim();
    if (!id) continue;
    next[id] = String(persona.name || "").trim() || id;
  }
  return next;
});

async function refreshArchivesWindowData() {
  try {
    const snapshot = await invokeTauri<AppBootstrapSnapshot>("load_app_bootstrap_snapshot");
    config.uiLanguage = normalizeLocale(snapshot.config.uiLanguage);
    config.uiFont = String(snapshot.config.uiFont || "");
    personas.value = Array.isArray(snapshot.agents) ? snapshot.agents : [];
    userAlias.value = String(snapshot.chatSettings?.userAlias || "").trim() || t("archives.roleUser");
  } catch (error) {
    setStatusError("status.loadConfigFailed", error);
  }
  await loadArchives();
}

const appBootstrap = useAppBootstrap({
  setViewMode: (mode) => {
    viewMode.value = mode;
  },
  initWindowMode: () => initWindow(),
  onThemeChanged: (theme) => {
    applyTheme(theme);
  },
  onLocaleChanged: (payload) => {
    const lang = normalizeLocale(payload);
    config.uiLanguage = lang;
    locale.value = lang;
  },
  onConfigUpdated: (payload) => {
    if (!payload || typeof payload !== "object") return;
    if ("uiFont" in payload) {
      config.uiFont = String(payload.uiFont ?? "");
    }
  },
  onChatSettingsUpdated: (payload) => {
    if ("userAlias" in payload) {
      userAlias.value = String(payload.userAlias ?? "").trim() || t("archives.roleUser");
    }
  },
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
  beforeRefreshData: ensureMessageStoreMigrationGate,
  refreshAllViewData: refreshArchivesWindowData,
  viewMode,
  syncWindowControlsState,
  stopRecording: async () => undefined,
  cleanupSpeechRecording: () => undefined,
  cleanupChatMedia: async () => undefined,
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

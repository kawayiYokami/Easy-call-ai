<template>
  <div class="h-full min-h-0 overflow-hidden">
  <SimpleSetupPanel v-if="props.simpleSetupMode" class="h-full" />
  <div v-else class="config-drawer-shell drawer md:drawer-open h-full min-h-0 overflow-hidden">
    <input
      id="config-drawer-toggle"
      v-model="configDrawerOpen"
      type="checkbox"
      class="drawer-toggle"
      aria-label="设置导航"
    />

    <div class="drawer-content flex min-h-0 min-w-0 flex-col overflow-hidden bg-base-200">
      <div class="flex shrink-0 items-center gap-2 border-b border-base-300 bg-base-100/80 px-3 py-2 md:hidden">
        <label
          for="config-drawer-toggle"
          class="btn btn-square btn-ghost btn-sm"
          aria-label="打开设置导航"
          title="打开设置导航"
        >
          <Menu class="h-4 w-4" />
        </label>
        <div class="min-w-0 truncate text-sm font-medium">{{ activeConfigTabTitle }}</div>
      </div>

      <div class="flex min-h-0 flex-1 min-w-0 flex-col overflow-hidden">
      <div v-if="props.configTab === 'api'" class="flex-1 min-h-0">
        <ApiTab
          :config="config"
          :base-url-reference="baseUrlReference"
          :refreshing-models="refreshingModels"
          :model-options="modelOptions"
          :model-refresh-ok="modelRefreshOk"
          :model-refresh-error="modelRefreshError"
          :config-dirty="configDirty"
          :saving-config="savingConfig"
          :save-api-config-action="props.saveConfigAction"
          :restore-api-config-action="props.restoreConfigAction"
          :normalize-api-bindings-action="normalizeApiBindingsAction"
          :last-saved-config-json="props.lastSavedConfigJson"
          :set-status-action="setStatusAction"
          @refresh-models="$emit('refreshModels')"
        />
      </div>

      <div v-else-if="props.configTab === 'department'" class="flex-1 min-h-0">
        <DepartmentTab
          :config="config"
          :api-configs="config.apiConfigs"
          :personas="personas"
          :assistant-department-agent-id="assistantDepartmentAgentId"
          :saving-config="savingConfig"
          :save-config-action="saveConfigAction"
          :set-status-action="setStatusAction"
          @update:assistant-department-assignee-id="$emit('update:assistantDepartmentAgentId', $event)"
        />
      </div>

      <div v-else-if="props.configTab === 'departmentTree'" class="flex-1 min-h-0">
        <DepartmentTreeTab
          :config="config"
          :personas="personas"
          :saving-config="savingConfig"
          :save-config-action="saveConfigAction"
          :set-status-action="setStatusAction"
          @open-persona-page="handleOpenPersonaPage"
        />
      </div>

      <div v-else-if="props.configTab === 'mcp'" class="flex-1 min-h-0">
        <McpTab />
      </div>

      <div v-else-if="props.configTab === 'skill'" class="flex-1 min-h-0">
        <SkillTab />
      </div>

      <div v-else-if="props.configTab === 'persona'" class="flex-1 min-h-0">
        <PersonaTab
          :personas="personas"
          :assistant-personas="assistantPersonas"
          :persona-editor-id="personaEditorId"
          :selected-persona="selectedPersona"
          :selected-persona-avatar-url="selectedPersonaAvatarUrl"
          :avatar-saving="avatarSaving"
          :avatar-error="avatarError"
          :persona-saving="personaSaving"
          :persona-dirty="personaDirty"
          @update:persona-editor-id="$emit('update:personaEditorId', $event)"
          @add-persona="$emit('addPersona')"
          @remove-selected-persona="$emit('removeSelectedPersona')"
          @reset-personas="$emit('resetPersonas')"
          @open-avatar-editor="openAvatarEditorForSelected"
          @import-persona-memories="$emit('importPersonaMemories', $event)"
          @save-personas="$emit('savePersonas')"
          @convert-private-persona-to-public="$emit('convertPrivatePersonaToPublic', $event)"
        />
      </div>

      <div v-else-if="props.configTab === 'demo' && SHOW_DEV_DEMO_TAB" class="flex-1 min-h-0">
        <DemoTab
          :config="config"
          :personas="personas"
          :persona-avatar-url-map="props.personaAvatarUrlMap"
          :assistant-department-agent-id="assistantDepartmentAgentId"
          @update:config-tab="$emit('update:configTab', $event)"
          @update:persona-editor-id="$emit('update:personaEditorId', $event)"
        />
      </div>

      <SettingsStickyLayout v-else>
          <WelcomeTab
            v-if="props.configTab === 'welcome'"
            :config="config"
            @jump="$emit('update:configTab', $event)"
            @start-chat="$emit('start-chat')"
          />

          <HotkeyTab
            v-else-if="props.configTab === 'hotkey'"
            :config="config"
            :hotkey-test-recording="hotkeyTestRecording"
            :hotkey-test-recording-ms="hotkeyTestRecordingMs"
            :hotkey-test-audio-ready="hotkeyTestAudioReady"
            :microphone-permission-state="microphonePermissionState"
            :microphone-permission-requesting="microphonePermissionRequesting"
            :background-voice-screenshot-keywords="backgroundVoiceScreenshotKeywords"
            :background-voice-screenshot-mode="backgroundVoiceScreenshotMode"
            @start-hotkey-record-test="$emit('startHotkeyRecordTest')"
            @stop-hotkey-record-test="$emit('stopHotkeyRecordTest')"
            @play-hotkey-record-test="$emit('playHotkeyRecordTest')"
            @request-microphone-permission="$emit('requestMicrophonePermission')"
            @capture-hotkey="$emit('captureHotkey', $event)"
            @update:record-hotkey="onRecordHotkeyChanged"
            @update:record-background-wake-enabled="onRecordBackgroundWakeChanged"
            @update:min-record-seconds="onMinRecordSecondsChanged"
            @update:max-record-seconds="onMaxRecordSecondsChanged"
            @update:background-voice-screenshot-keywords="$emit('update:backgroundVoiceScreenshotKeywords', $event)"
            @update:background-voice-screenshot-mode="$emit('update:backgroundVoiceScreenshotMode', $event)"
            @patch-chat-settings="$emit('patchChatSettings', $event)"
          />

          <ToolsTab
            v-else-if="props.configTab === 'tools'"
            :config="config"
            :personas="assistantPersonas"
            :tool-api-config="toolApiConfig"
            :tool-statuses="toolStatuses"
            :saving-config="savingConfig"
            @save-api-config="onSaveToolsConfig"
            @refresh-tool-statuses="$emit('refreshToolsStatus')"
          />

          <ChatSettingsTab
            v-else-if="props.configTab === 'chatSettings'"
            :config="config"
            :text-capable-api-configs="textCapableApiConfigs"
            :image-capable-api-configs="imageCapableApiConfigs"
            :stt-capable-api-configs="sttCapableApiConfigs"
            :response-style-options="responseStyleOptions"
            :response-style-id="responseStyleId"
            :pdf-read-mode="pdfReadMode"
            :instruction-presets="instructionPresets"
            :tool-statuses="toolStatuses"
            :saving-config="savingConfig"
            @update:response-style-id="$emit('update:responseStyleId', $event)"
            @update:pdf-read-mode="$emit('update:pdfReadMode', $event)"
            @update:instruction-presets="$emit('update:instructionPresets', $event)"
            @patch-conversation-api-settings="$emit('patchConversationApiSettings', $event)"
            @patch-chat-settings="$emit('patchChatSettings', $event)"
          />
          <NotificationTab
            v-else-if="props.configTab === 'notification'"
            :config="config"
            :saving-config="savingConfig"
            :save-config-action="saveConfigAction"
            :last-saved-config-json="lastSavedConfigJson"
          />
          <NetworkAccessTab
            v-else-if="props.configTab === 'networkAccess'"
            :config="config"
            :saving-config="savingConfig"
            :save-config-action="saveConfigAction"
            :last-saved-config-json="lastSavedConfigJson"
          />
          <RemoteImTab
            v-else-if="props.configTab === 'remoteIm'"
            :config="config"
            :personas="personas"
            :persona-avatar-url-map="props.personaAvatarUrlMap"
            :save-config-action="saveConfigAction"
            :set-status-action="setStatusAction"
          />

          <UsageTab
            v-else-if="props.configTab === 'usage'"
          />

          <MemoryTab
            v-else-if="props.configTab === 'memory'"
            :sync-locked="memorySyncLocked"
            @sync-lock-change="onMemorySyncLockChange"
          />

          <TaskTab
            v-else-if="props.configTab === 'task'"
            :config="config"
            :personas="personas"
            :persona-avatar-url-map="props.personaAvatarUrlMap"
          />

          <LogTab
            v-else-if="props.configTab === 'logs'"
            :config="config"
            :open-runtime-logs="() => $emit('openRuntimeLogs')"
            :open-conversation-list="() => $emit('openConversationList')"
            :open-prompt-preview="() => $emit('openPromptPreview')"
            :open-system-prompt-preview="() => $emit('openSystemPromptPreview')"
          />

          <AppearanceTab
            v-else-if="props.configTab === 'appearance'"
            :ui-language="uiLanguage"
            :locale-options="localeOptions"
            :current-theme="currentTheme"
            :theme-mode="themeMode"
            :auto-light-theme="autoLightTheme"
            :auto-dark-theme="autoDarkTheme"
            :generated-theme-controls="generatedThemeControls"
            :generated-theme-tokens="generatedThemeTokens"
            :generated-light-tokens="generatedLightTokens"
            :generated-dark-tokens="generatedDarkTokens"
            :ui-size-scale="uiSizeScale"
            @update:ui-language="$emit('update:uiLanguage', $event)"
            @update:ui-size-scale="$emit('update:uiSizeScale', $event)"
            @set-theme="$emit('setTheme', $event)"
            @set-theme-mode="$emit('setThemeMode', $event)"
            @set-auto-theme="(side, value) => $emit('setAutoTheme', side, value)"
            @activate-generated-theme="$emit('activateGeneratedTheme')"
            @update-generated-theme-controls="$emit('updateGeneratedThemeControls', $event)"
            @reset-generated-theme="$emit('resetGeneratedTheme')"
          />

          <StorageTab
            v-else-if="props.configTab === 'migration'"
          />

          <AboutTab
            v-else-if="props.configTab === 'about'"
            :github-update-method="props.config.githubUpdateMethod || 'auto'"
            :checking-update="checkingUpdate"
            :current-theme="currentTheme"
            @update:github-update-method="$emit('update:githubUpdateMethod', $event)"
            @check-update="$emit('checkUpdate')"
            @open-github="$emit('openGithub')"
          />
      </SettingsStickyLayout>
    </div>
    </div>

    <div class="drawer-side z-40 min-h-0 overflow-hidden">
      <label for="config-drawer-toggle" aria-label="关闭设置导航" class="drawer-overlay"></label>
      <aside
        class="relative flex h-full min-h-0 w-44 flex-col border-r border-base-300 bg-base-200 px-2"
        @mouseenter="navScrollbarRef?.reveal()"
        @mouseleave="navScrollbarRef?.hide()"
      >
        <div ref="navScrollerRef" class="ecall-floating-scroll-target min-h-0 flex-1 overflow-y-auto pr-1">
          <ul class="menu w-full gap-1 p-0 pt-2 [&>li>a]:w-full">
            <li v-for="item in visibleConfigNavItems" :key="item.tab">
              <a :class="configNavLinkClass(item.tab)" @click="selectConfigNavTab(item.tab)">
                <component :is="item.icon" class="h-4 w-4 shrink-0" />
                <span class="min-w-0 truncate">{{ item.labelKey ? t(item.labelKey) : item.label }}</span>
                <span
                  v-if="item.tab === 'about' && props.hasAvailableUpdate"
                  class="ml-auto inline-flex h-2.5 w-2.5 shrink-0 rounded-full bg-error"
                  :title="t('about.updateAvailableBadge')"
                ></span>
              </a>
            </li>
          </ul>
        </div>
        <FloatingScrollbar ref="navScrollbarRef" :target="navScrollerRef" />
      </aside>
    </div>
  </div>

    <!-- Dialogs -->

    <input ref="avatarFileInput" type="file" accept="image/*" class="hidden" @change="onAvatarFilePicked" />
    <dialog ref="avatarEditorDialog" class="modal">
    <div class="modal-box p-3 max-w-sm">
      <h3 class="text-sm font-semibold mb-2">{{ t("config.persona.editAvatar") }}</h3>
      <div class="rounded border border-base-300 bg-base-100 p-3">
        <div class="flex items-center gap-3">
          <div v-if="avatarEditorAvatarUrl" class="avatar">
            <div class="w-14 rounded-full">
              <img :src="avatarEditorAvatarUrl" :alt="avatarEditorName" :title="avatarEditorName" />
            </div>
          </div>
          <div v-else class="avatar placeholder">
            <div class="bg-neutral text-neutral-content w-14 rounded-full">
              <span>{{ avatarInitial(avatarEditorName) }}</span>
            </div>
          </div>
          <div class="text-sm opacity-70 break-all">{{ avatarEditorName }}</div>
        </div>
        <div class="mt-3 flex gap-2">
          <button class="btn btn-sm" :disabled="!avatarEditorTargetId || avatarSaving" @click="openAvatarPickerForEditor">{{ t("config.persona.uploadAvatar") }}</button>
          <button class="btn btn-sm btn-ghost" :disabled="!avatarEditorTargetHasAvatar || avatarSaving" @click="clearAvatarFromEditor">{{ t("config.persona.clearAvatar") }}</button>
        </div>
        <div class="mt-2 text-xs opacity-60">{{ t("config.persona.pasteImageHint") }}</div>
        <div v-if="avatarError" class="mt-2 text-sm text-error break-all">{{ avatarError }}</div>
      </div>
      <div class="modal-action mt-2">
        <button class="btn btn-sm btn-ghost" @click="closeAvatarEditor">{{ t("common.close") }}</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button aria-label="close">close</button>
    </form>
    </dialog>
    <dialog ref="cropDialog" class="modal" @close="destroyCropper">
    <div class="modal-box p-3 max-w-md">
      <h3 class="text-sm font-semibold mb-2">{{ t("config.persona.cropAvatar") }}</h3>
      <div class="rounded border border-base-300 bg-base-100 p-2 min-h-64">
        <img ref="cropImageEl" :src="cropSource" alt="crop source" class="max-w-full block" />
      </div>
      <div v-if="localCropError || avatarError" class="mt-2 text-sm text-error break-all">{{ localCropError || avatarError }}</div>
      <div class="modal-action mt-2">
        <button class="btn btn-sm btn-ghost" @click="closeCropDialog">{{ t("common.cancel") }}</button>
        <button class="btn btn-sm btn-primary" :disabled="!cropperReady || avatarSaving" @click="confirmCrop">
          {{ avatarSaving ? t("config.api.saving") : t("config.persona.saveAvatar") }}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button aria-label="close">close</button>
    </form>
    </dialog>
    <dialog ref="migrateWorkspaceDialog" class="modal">
      <div class="modal-box max-w-lg p-4">
        <h3 class="text-sm font-semibold">{{ t("config.tools.migrateWorkspaceTitle") }}</h3>
        <p v-if="workspaceMigrationUi.mode === 'confirm'" class="mt-3 text-sm whitespace-pre-wrap">
          {{ t("config.tools.migrateWorkspaceConfirm", { oldPath: pendingWorkspaceMigration.oldPath, newPath: pendingWorkspaceMigration.newPath }) }}
        </p>
        <div v-else class="mt-3 grid gap-3">
          <div class="text-sm">{{ workspaceMigrationUi.message || t("config.tools.migrateWorkspacePreparing") }}</div>
          <progress class="progress progress-primary w-full" :value="workspaceMigrationProgressValue" max="100"></progress>
          <div class="flex items-center justify-between text-xs opacity-70">
            <span>{{ workspaceMigrationStageLabel }}</span>
            <span>{{ workspaceMigrationUi.processed }}/{{ workspaceMigrationUi.total }}</span>
          </div>
          <div v-if="workspaceMigrationUi.currentPath" class="text-xs font-mono break-all opacity-70">
            {{ workspaceMigrationUi.currentPath }}
          </div>
          <div v-if="workspaceMigrationUi.error" class="rounded bg-error/10 px-3 py-2 text-sm text-error whitespace-pre-wrap break-all">
            {{ workspaceMigrationUi.error }}
          </div>
        </div>
        <div class="modal-action mt-4">
          <button
            class="btn btn-sm btn-ghost"
            type="button"
            :disabled="workspaceMigrationUi.mode === 'running'"
            @click="cancelWorkspaceMigration"
          >
            {{ workspaceMigrationUi.mode === 'error' ? t("common.close") : t("common.cancel") }}
          </button>
          <button
            v-if="workspaceMigrationUi.mode === 'confirm'"
            class="btn btn-sm btn-outline"
            type="button"
            @click="skipWorkspaceMigration"
          >
            {{ t("config.tools.migrateWorkspaceSkip") }}
          </button>
          <button
            v-if="workspaceMigrationUi.mode === 'confirm'"
            class="btn btn-sm btn-primary"
            type="button"
            @click="confirmWorkspaceMigration"
          >
            {{ t("common.confirm") }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button aria-label="close" @click="cancelWorkspaceMigration">close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, type Component } from "vue";
import { useI18n } from "vue-i18n";
import type { ApiConfigItem, AppConfig, ChatSettingsPatch, ConversationApiSettingsPatch, PersonaProfile, PromptCommandPreset, ResponseStyleOption, ToolLoadStatus } from "../../../types/app";
import type { GeneratedThemeControls, GeneratedThemeTokens, ThemeMode, ThemeModeKind } from "../../shell/theme/theme-types";
import Cropper from "cropperjs";
import SettingsStickyLayout from "../components/SettingsStickyLayout.vue";
import WelcomeTab from "./config-tabs/WelcomeTab.vue";
import HotkeyTab from "./config-tabs/HotkeyTab.vue";
import ApiTab from "./config-tabs/ApiTab.vue";
import ToolsTab from "./config-tabs/ToolsTab.vue";
import McpTab from "./config-tabs/McpTab.vue";
import SkillTab from "./config-tabs/SkillTab.vue";
import PersonaTab from "./config-tabs/PersonaTab.vue";
import DepartmentTab from "./config-tabs/DepartmentTab.vue";
import DepartmentTreeTab from "./config-tabs/DepartmentTreeTab.vue";
import DemoTab from "./config-tabs/DemoTab.vue";
import ChatSettingsTab from "./config-tabs/ChatSettingsTab.vue";
import NotificationTab from "./config-tabs/NotificationTab.vue";
import NetworkAccessTab from "./config-tabs/NetworkAccessTab.vue";
import RemoteImTab from "./config-tabs/RemoteImTab.vue";
import UsageTab from "./config-tabs/UsageTab.vue";
import MemoryTab from "./config-tabs/MemoryTab.vue";
import TaskTab from "./config-tabs/TaskTab.vue";
import LogTab from "./config-tabs/LogTab.vue";
import AppearanceTab from "./config-tabs/AppearanceTab.vue";
import StorageTab from "./config-tabs/StorageTab.vue";
import AboutTab from "./config-tabs/AboutTab.vue";
import SimpleSetupPanel from "./config-tabs/SimpleSetupPanel.vue";
import {
  invokeTauri,
  migrateTransportShellWorkspaceDirectory,
  onTransportNotification,
} from "../../../services/tauri-api";
import { toErrorMessage } from "../../../utils/error";
import { ArrowLeftRight, Beaker, Bell, Building2, ClipboardList, Code, Cpu, Database, Home, Info, Keyboard, Menu, Network, Palette, Puzzle, Radio, ScrollText, Star, User, Wifi, Wrench } from "@lucide/vue";
import FloatingScrollbar from "../../shell/components/FloatingScrollbar.vue";

type ConfigTab = "welcome" | "hotkey" | "api" | "tools" | "mcp" | "skill" | "persona" | "department" | "departmentTree" | "demo" | "chatSettings" | "notification" | "networkAccess" | "remoteIm" | "usage" | "memory" | "task" | "logs" | "appearance" | "migration" | "about";
type AvatarTarget = { agentId: string };
type ConfigNavItem = {
  tab: ConfigTab;
  icon: Component;
  labelKey?: string;
  label?: string;
  devOnly?: boolean;
};
const SHOW_DEV_DEMO_TAB = import.meta.env.DEV;

const CONFIG_NAV_ITEMS: ConfigNavItem[] = [
  { tab: "welcome", icon: Home, labelKey: "config.tabs.welcome" },
  { tab: "chatSettings", icon: Star, labelKey: "config.tabs.chatSettings" },
  { tab: "notification", icon: Bell, labelKey: "config.tabs.notification" },
  { tab: "networkAccess", icon: Wifi, labelKey: "config.tabs.networkAccess" },
  { tab: "hotkey", icon: Keyboard, labelKey: "config.tabs.hotkey" },
  { tab: "api", icon: Cpu, labelKey: "config.tabs.api" },
  { tab: "tools", icon: Wrench, labelKey: "config.tabs.tools" },
  { tab: "mcp", icon: Puzzle, label: "MCP" },
  { tab: "skill", icon: Code, labelKey: "config.tabs.skill" },
  { tab: "persona", icon: User, labelKey: "config.tabs.persona" },
  { tab: "department", icon: Building2, labelKey: "config.tabs.department" },
  { tab: "departmentTree", icon: Network, labelKey: "config.tabs.departmentTree" },
  { tab: "remoteIm", icon: Radio, labelKey: "config.tabs.remoteIm" },
  { tab: "memory", icon: Database, labelKey: "config.tabs.memory" },
  { tab: "task", icon: ClipboardList, labelKey: "config.tabs.task" },
  { tab: "logs", icon: ScrollText, labelKey: "config.tabs.logs" },
  { tab: "appearance", icon: Palette, labelKey: "config.tabs.appearance" },
  { tab: "migration", icon: ArrowLeftRight, labelKey: "config.tabs.migration" },
  { tab: "usage", icon: ScrollText, labelKey: "config.tabs.usage" },
  { tab: "about", icon: Info, labelKey: "config.tabs.about" },
  { tab: "demo", icon: Beaker, labelKey: "config.tabs.demo", devOnly: true },
];

const props = defineProps<{
  config: AppConfig;
  configTab: ConfigTab;
  simpleSetupMode?: boolean;
  uiLanguage: "zh-CN" | "en-US" | "zh-TW";
  localeOptions: Array<{ value: "zh-CN" | "en-US" | "zh-TW"; label: string }>;
  currentTheme: string;
  themeMode: ThemeModeKind;
  autoLightTheme: string;
  autoDarkTheme: string;
  generatedThemeControls: GeneratedThemeControls;
  generatedThemeTokens: GeneratedThemeTokens;
  generatedLightTokens: GeneratedThemeTokens;
  generatedDarkTokens: GeneratedThemeTokens;
  uiSizeScale: number;
  selectedApiConfig: ApiConfigItem | null;
  toolApiConfig: ApiConfigItem | null;
  baseUrlReference: string;
  refreshingModels: boolean;
  modelOptions: string[];
  modelRefreshOk: boolean;
  modelRefreshError: string;
  toolStatuses: ToolLoadStatus[];
  personas: PersonaProfile[];
  personaAvatarUrlMap: Record<string, string>;
  assistantPersonas: PersonaProfile[];
  userPersona: PersonaProfile | null;
  personaEditorId: string;
  assistantDepartmentAgentId: string;
  responseStyleOptions: ResponseStyleOption[];
  responseStyleId: string;
  pdfReadMode: "text" | "image";
  backgroundVoiceScreenshotKeywords: string;
  backgroundVoiceScreenshotMode: "desktop" | "focused_window";
  instructionPresets: PromptCommandPreset[];
  selectedPersona: PersonaProfile | null;
  toolPersona: PersonaProfile | null;
  selectedPersonaAvatarUrl: string;
  userPersonaAvatarUrl: string;
  textCapableApiConfigs: ApiConfigItem[];
  imageCapableApiConfigs: ApiConfigItem[];
  sttCapableApiConfigs: ApiConfigItem[];
  avatarSaving: boolean;
  avatarError: string;
  personaSaving: boolean;
  personaDirty: boolean;
  configDirty: boolean;
  savingConfig: boolean;
  normalizeApiBindingsAction: () => void;
  hotkeyTestRecording: boolean;
  hotkeyTestRecordingMs: number;
  hotkeyTestAudioReady: boolean;
  microphonePermissionState: "granted" | "denied" | "prompt" | "unsupported" | "unknown";
  microphonePermissionRequesting: boolean;
  checkingUpdate: boolean;
  hasAvailableUpdate: boolean;
  saveConfigAction: () => Promise<boolean> | boolean;
  updateRecordHotkeyAction: (value: string) => Promise<boolean> | boolean;
  updateRecordBackgroundWakeEnabledAction: (value: boolean) => Promise<boolean> | boolean;
  restoreConfigAction: () => boolean;
  lastSavedConfigJson: string;
  setStatusAction: (text: string) => void;
}>();

const emit = defineEmits<{
  (e: "update:configTab", value: ConfigTab): void;
  (e: "update:uiLanguage", value: string): void;
  (e: "update:uiSizeScale", value: number): void;
  (e: "update:githubUpdateMethod", value: AppConfig["githubUpdateMethod"]): void;
  (e: "update:personaEditorId", value: string): void;
  (e: "update:assistantDepartmentAgentId", value: string): void;
  (e: "update:responseStyleId", value: string): void;
  (e: "update:pdfReadMode", value: "text" | "image"): void;
  (e: "update:backgroundVoiceScreenshotKeywords", value: string): void;
  (e: "update:backgroundVoiceScreenshotMode", value: "desktop" | "focused_window"): void;
  (e: "update:instructionPresets", value: PromptCommandPreset[]): void;
  (e: "patchConversationApiSettings", value: ConversationApiSettingsPatch): void;
  (e: "patchChatSettings", value: ChatSettingsPatch): void;
  (e: "setTheme", value: string): void;
  (e: "setThemeMode", value: ThemeModeKind): void;
  (e: "setAutoTheme", side: ThemeMode, value: string): void;
  (e: "activateGeneratedTheme"): void;
  (e: "updateGeneratedThemeControls", value: Partial<GeneratedThemeControls>): void;
  (e: "resetGeneratedTheme"): void;
  (e: "refreshModels"): void;
  (e: "refreshToolsStatus"): void;
  (e: "toolSwitchChanged"): void;
  (e: "openMemoryViewer"): void;
  (e: "addApiConfig"): void;
  (e: "removeSelectedApiConfig"): void;
  (e: "saveApiConfig"): void;
  (e: "addPersona"): void;
  (e: "removeSelectedPersona"): void;
  (e: "resetPersonas"): void;
  (e: "savePersonas"): void;
  (e: "convertPrivatePersonaToPublic", agentId: string): void;
  (e: "importPersonaMemories", value: { agentId: string; file: File }): void;
  (e: "openConversationList"): void;
  (e: "openPromptPreview"): void;
  (e: "openSystemPromptPreview"): void;
  (e: "openRuntimeLogs"): void;
  (e: "startHotkeyRecordTest"): void;
  (e: "stopHotkeyRecordTest"): void;
  (e: "playHotkeyRecordTest"): void;
  (e: "requestMicrophonePermission"): void;
  (e: "captureHotkey", value: string): void;
  (e: "saveAgentAvatar", value: { agentId: string; mime: string; bytesBase64: string }): void;
  (e: "clearAgentAvatar", value: { agentId: string }): void;
  (e: "checkUpdate"): void;
  (e: "openGithub"): void;
  (e: "start-chat"): void;
}>();

const { t } = useI18n();

const avatarFileInput = ref<HTMLInputElement | null>(null);
const avatarEditorDialog = ref<HTMLDialogElement | null>(null);
const cropDialog = ref<HTMLDialogElement | null>(null);
const migrateWorkspaceDialog = ref<HTMLDialogElement | null>(null);
const cropImageEl = ref<HTMLImageElement | null>(null);
const cropSource = ref("");
const pendingWorkspaceMigration = ref({ oldPath: "", newPath: "" });
const workspaceMigrationUi = ref({
  taskId: "",
  mode: "confirm" as "confirm" | "running" | "error",
  stage: "idle",
  message: "",
  processed: 0,
  total: 0,
  currentPath: "",
  error: "",
});
type WorkspaceMigrationDecision = "migrate" | "skip" | "cancel";
let resolveWorkspaceMigrationConfirm: ((value: WorkspaceMigrationDecision) => void) | null = null;
let workspaceMigrationProgressUnlisten: (() => void) | null = null;
const cropperReady = ref(false);
const localCropError = ref("");
const avatarEditorTargetId = ref("");
const configDrawerOpen = ref(false);
const navScrollerRef = ref<HTMLElement | null>(null);
const navScrollbarRef = ref<InstanceType<typeof FloatingScrollbar> | null>(null);
const memorySyncLocked = ref(false);
const savingToolsConfig = ref(false);
let cropper: Cropper | null = null;
let cropTarget: AvatarTarget | null = null;
const MIN_RECORD_SECONDS = 1;
const MAX_MIN_RECORD_SECONDS = 30;
const MAX_RECORD_SECONDS = 600;
const workspaceMigrationProgressValue = computed(() => {
  if (workspaceMigrationUi.value.total <= 0) return 0;
  return Math.max(0, Math.min(100, Math.round((workspaceMigrationUi.value.processed / workspaceMigrationUi.value.total) * 100)));
});
const workspaceMigrationStageLabel = computed(() => {
  const stage = workspaceMigrationUi.value.stage;
  if (stage === "scanning") return t("config.tools.migrateWorkspaceStageScanning");
  if (stage === "copying") return t("config.tools.migrateWorkspaceStageCopying");
  if (stage === "deleting") return t("config.tools.migrateWorkspaceStageDeleting");
  if (stage === "completed") return t("config.tools.migrateWorkspaceStageCompleted");
  if (stage === "failed") return t("config.tools.migrateWorkspaceStageFailed");
  return t("config.tools.migrateWorkspacePreparing");
});
const visibleConfigNavItems = computed(() => CONFIG_NAV_ITEMS.filter((item) => !item.devOnly || SHOW_DEV_DEMO_TAB));
const activeConfigNavItem = computed(() =>
  visibleConfigNavItems.value.find((item) => item.tab === props.configTab)
  ?? visibleConfigNavItems.value.find((item) => item.tab === "welcome")
  ?? null,
);
const activeConfigTabTitle = computed(() => {
  const item = activeConfigNavItem.value;
  if (!item) return "";
  return item.labelKey ? t(item.labelKey) : (item.label || "");
});

function isConfigNavItemLocked(tab: ConfigTab): boolean {
  return memorySyncLocked.value && tab !== "memory";
}

function configNavLinkClass(tab: ConfigTab) {
  return {
    active: props.configTab === tab,
    "menu-active": props.configTab === tab,
    "opacity-50 pointer-events-none": isConfigNavItemLocked(tab),
  };
}

function selectConfigNavTab(tab: ConfigTab) {
  if (isConfigNavItemLocked(tab)) return;
  requestTabChange(tab);
  configDrawerOpen.value = false;
}

function avatarInitial(name: string): string {
  const text = (name || "").trim();
  if (!text) return "?";
  return text[0].toUpperCase();
}

function openAvatarPicker(target: AvatarTarget) {
  cropTarget = target;
  if (avatarFileInput.value) {
    avatarFileInput.value.value = "";
    avatarFileInput.value.click();
  }
}

function openAvatarEditorForSelected() {
  if (!props.selectedPersona) return;
  avatarEditorTargetId.value = props.selectedPersona.id;
  cropTarget = { agentId: props.selectedPersona.id };
  avatarEditorDialog.value?.showModal();
}

function closeAvatarEditor() {
  avatarEditorDialog.value?.close();
}

function openAvatarPickerForEditor() {
  if (!avatarEditorTargetId.value) return;
  openAvatarPicker({ agentId: avatarEditorTargetId.value });
}

function ensureEditorCropTarget() {
  if (cropTarget || !avatarEditorTargetId.value) return;
  cropTarget = { agentId: avatarEditorTargetId.value };
}

function clearAvatarFromEditor() {
  if (!avatarEditorTargetId.value) return;
  emit("clearAgentAvatar", { agentId: avatarEditorTargetId.value });
}

function avatarById(id: string): PersonaProfile | null {
  return props.personas.find((p) => p.id === id) ?? null;
}

const avatarEditorTarget = () => avatarById(avatarEditorTargetId.value);

const avatarEditorName = computed(() => avatarEditorTarget()?.name || t("config.persona.avatarFallbackName"));
const avatarEditorAvatarUrl = computed(() => {
  const target = avatarEditorTarget();
  if (!target) return "";
  if (target.id === props.userPersona?.id) return props.userPersonaAvatarUrl;
  if (target.id === props.selectedPersona?.id) return props.selectedPersonaAvatarUrl;
  return "";
});
const avatarEditorTargetHasAvatar = computed(() => !!avatarEditorTarget()?.avatarPath);

async function readFileAsDataUrl(file: File): Promise<string> {
  return await new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result || ""));
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(file);
  });
}

async function loadImage(dataUrl: string): Promise<HTMLImageElement> {
  return await new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error("load image failed"));
    img.src = dataUrl;
  });
}

async function downscaleDataUrl(dataUrl: string, maxSide = 1024): Promise<string> {
  const img = await loadImage(dataUrl);
  const w = img.naturalWidth || img.width;
  const h = img.naturalHeight || img.height;
  if (w <= maxSide && h <= maxSide) return dataUrl;
  const scale = Math.min(1, maxSide / Math.max(w, h));
  const targetW = Math.max(1, Math.round(w * scale));
  const targetH = Math.max(1, Math.round(h * scale));
  const canvas = document.createElement("canvas");
  canvas.width = targetW;
  canvas.height = targetH;
  const ctx = canvas.getContext("2d");
  if (!ctx) return dataUrl;
  ctx.imageSmoothingEnabled = true;
  ctx.imageSmoothingQuality = "high";
  ctx.drawImage(img, 0, 0, targetW, targetH);
  return canvas.toDataURL("image/webp", 0.9);
}

function destroyCropper() {
  if (cropper) {
    cropper.destroy();
    cropper = null;
  }
  cropperReady.value = false;
}

function closeCropDialog() {
  cropDialog.value?.close();
  cropSource.value = "";
  cropTarget = null;
  localCropError.value = "";
}

// `config` is a shared reactive object from the root app state.
// Direct mutation here is intentional and immediately reflected upstream.
async function onRecordHotkeyChanged(value: string) {
  const next = String(value || "").trim();
  if (props.config.recordHotkey === next) return;
  await Promise.resolve(props.updateRecordHotkeyAction(next));
}

async function onRecordBackgroundWakeChanged(value: boolean) {
  const next = !!value;
  if (!!props.config.recordBackgroundWakeEnabled === next) return;
  await Promise.resolve(props.updateRecordBackgroundWakeEnabledAction(next));
}

function onMinRecordSecondsChanged(value: number) {
  const next = Math.max(MIN_RECORD_SECONDS, Math.min(MAX_MIN_RECORD_SECONDS, Math.round(Number(value) || MIN_RECORD_SECONDS)));
  props.config.minRecordSeconds = next;
  if (props.config.maxRecordSeconds < next) {
    props.config.maxRecordSeconds = next;
  }
}

function onMaxRecordSecondsChanged(value: number) {
  const next = Math.max(
    props.config.minRecordSeconds,
    Math.min(MAX_RECORD_SECONDS, Math.round(Number(value) || props.config.minRecordSeconds)),
  );
  props.config.maxRecordSeconds = next;
}

function requestTabChange(nextTab: ConfigTab) {
  if (memorySyncLocked.value && nextTab !== "memory") {
    return;
  }
  if (!SHOW_DEV_DEMO_TAB && nextTab === "demo") {
    emit("update:configTab", "hotkey");
    return;
  }
  emit("update:configTab", nextTab);
}

function handleOpenPersonaPage() {
  requestTabChange("persona");
}

function onMemorySyncLockChange(locked: boolean) {
  memorySyncLocked.value = !!locked;
}

async function onSaveToolsConfig() {
  if (savingToolsConfig.value) return;
  savingToolsConfig.value = true;
  const previousShellWorkspaces = Array.isArray(props.config.shellWorkspaces)
    ? props.config.shellWorkspaces.map((item) => ({
      id: String(item.id || "") || "system-workspace",
      name: String(item.name || ""),
      path: String(item.path || ""),
      level: "system" as const,
      access: "full_access" as const,
      builtIn: !!item.builtIn,
    }))
    : [];
  const previousTerminalShellKind = String(props.config.terminalShellKind || "auto").trim() || "auto";
  try {
    let savedSnapshot: { shellWorkspaces?: Array<{ path?: string }> } = {};
    try {
      savedSnapshot = JSON.parse(props.lastSavedConfigJson || "{}") as { shellWorkspaces?: Array<{ path?: string }> };
    } catch {
      savedSnapshot = {};
    }
    const previousSavedPath = String(savedSnapshot.shellWorkspaces?.[0]?.path || "").trim();
    const nextPath = String(props.config.shellWorkspaces?.[0]?.path || "").trim();
    const normalizedPreviousSavedPath = previousSavedPath.replace(/[\\/]+$/g, "");
    const normalizedNextPath = nextPath.replace(/[\\/]+$/g, "");
    if (
      normalizedPreviousSavedPath
      && normalizedNextPath
      && normalizedPreviousSavedPath !== normalizedNextPath
    ) {
      const decision = await requestWorkspaceMigrationConfirm(previousSavedPath, nextPath);
      if (decision === "cancel") {
        props.config.shellWorkspaces = previousShellWorkspaces;
        props.config.terminalShellKind = previousTerminalShellKind;
        return;
      }
      if (decision === "migrate") {
        const migratedPath = await migrateWorkspaceWithProgress(previousSavedPath, nextPath);
        props.setStatusAction(t("config.tools.migrateWorkspaceDone", { path: migratedPath }));
      }
    }
    const saved = await Promise.resolve(props.saveConfigAction());
    if (!saved) {
      props.config.shellWorkspaces = previousShellWorkspaces;
      props.config.terminalShellKind = previousTerminalShellKind;
      props.setStatusAction(t("status.saveConfigFailed", { err: "tools settings save rejected" }));
    }
  } catch (error) {
    props.config.shellWorkspaces = previousShellWorkspaces;
    props.config.terminalShellKind = previousTerminalShellKind;
    props.setStatusAction(t("status.saveConfigFailed", { err: toErrorMessage(error) }));
    throw error;
  } finally {
    savingToolsConfig.value = false;
  }
}

function requestWorkspaceMigrationConfirm(oldPath: string, newPath: string): Promise<WorkspaceMigrationDecision> {
  const dialog = migrateWorkspaceDialog.value;
  if (!dialog) return Promise.resolve("cancel");
  pendingWorkspaceMigration.value = { oldPath, newPath };
  workspaceMigrationUi.value = {
    taskId: "",
    mode: "confirm",
    stage: "idle",
    message: "",
    processed: 0,
    total: 0,
    currentPath: "",
    error: "",
  };
  return new Promise<WorkspaceMigrationDecision>((resolve) => {
    resolveWorkspaceMigrationConfirm = resolve;
    dialog.showModal();
  });
}

function finishWorkspaceMigrationConfirm(value: WorkspaceMigrationDecision) {
  const dialog = migrateWorkspaceDialog.value;
  if (dialog?.open && value !== "migrate") {
    dialog.close();
  }
  resolveWorkspaceMigrationConfirm?.(value);
  resolveWorkspaceMigrationConfirm = null;
}

function confirmWorkspaceMigration() {
  workspaceMigrationUi.value.mode = "running";
  workspaceMigrationUi.value.stage = "scanning";
  workspaceMigrationUi.value.message = t("config.tools.migrateWorkspacePreparing");
  finishWorkspaceMigrationConfirm("migrate");
}

function skipWorkspaceMigration() {
  finishWorkspaceMigrationConfirm("skip");
}

function cancelWorkspaceMigration() {
  if (workspaceMigrationUi.value.mode === "running") return;
  finishWorkspaceMigrationConfirm("cancel");
}

async function ensureWorkspaceMigrationListener() {
  if (workspaceMigrationProgressUnlisten) return;
  workspaceMigrationProgressUnlisten = onTransportNotification<{
    taskId: string;
    stage: string;
    processed: number;
    total: number;
    currentPath?: string | null;
    message: string;
    done: boolean;
    error?: string | null;
  }>("workspace.migrationProgress", (payload) => {
    if (!payload || payload.taskId !== workspaceMigrationUi.value.taskId) return;
    workspaceMigrationUi.value.stage = String(payload.stage || "");
    workspaceMigrationUi.value.message = String(payload.message || "");
    workspaceMigrationUi.value.processed = Number(payload.processed || 0);
    workspaceMigrationUi.value.total = Number(payload.total || 0);
    workspaceMigrationUi.value.currentPath = String(payload.currentPath || "");
    workspaceMigrationUi.value.error = String(payload.error || "");
    if (payload.error) {
      workspaceMigrationUi.value.mode = "error";
    }
  });
}

async function migrateWorkspaceWithProgress(oldPath: string, newPath: string): Promise<string> {
  await ensureWorkspaceMigrationListener();
  const taskId = `workspace-migration-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
  workspaceMigrationUi.value.taskId = taskId;
  workspaceMigrationUi.value.mode = "running";
  workspaceMigrationUi.value.stage = "scanning";
  workspaceMigrationUi.value.message = t("config.tools.migrateWorkspacePreparing");
  workspaceMigrationUi.value.processed = 0;
  workspaceMigrationUi.value.total = 0;
  workspaceMigrationUi.value.currentPath = oldPath;
  workspaceMigrationUi.value.error = "";
  try {
    const migratedPath = await migrateTransportShellWorkspaceDirectory({ oldPath, newPath, taskId });
    const dialog = migrateWorkspaceDialog.value;
    if (dialog?.open) {
      dialog.close();
    }
    return migratedPath;
  } catch (error) {
    workspaceMigrationUi.value.mode = "error";
    workspaceMigrationUi.value.stage = "failed";
    workspaceMigrationUi.value.message = t("config.tools.migrateWorkspaceFailed");
    workspaceMigrationUi.value.error = toErrorMessage(error);
    throw error;
  }
}

async function onAvatarFilePicked(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  void processAvatarFile(file);
}

async function processAvatarFile(file: File) {
  ensureEditorCropTarget();
  if (!cropTarget) return;
  localCropError.value = "";
  try {
    const dataUrl = await readFileAsDataUrl(file);
    cropSource.value = await downscaleDataUrl(dataUrl, 1024);
    await nextTick();
    destroyCropper();
    if (!cropImageEl.value) {
      localCropError.value = t("config.persona.cropInitFailed");
      return;
    }
    cropper = new Cropper(cropImageEl.value);
    const selection = cropper.getCropperSelection();
    if (selection) {
      selection.aspectRatio = 1;
      selection.initialAspectRatio = 1;
      selection.initialCoverage = 1;
      selection.$center();
    }
    cropperReady.value = true;
    cropDialog.value?.showModal();
  } catch (e) {
    localCropError.value = t("config.persona.avatarReadFailed", { err: String(e) });
  }
}

function handleAvatarPaste(event: ClipboardEvent) {
  if (!avatarEditorDialog.value?.open) return;
  const items = event.clipboardData?.items;
  if (!items || items.length === 0) return;
  const imageItem = Array.from(items).find((item) => item.type.startsWith("image/"));
  if (!imageItem) {
    localCropError.value = t("config.persona.pasteNoImage");
    return;
  }
  const file = imageItem.getAsFile();
  if (!file) {
    localCropError.value = t("config.persona.pasteReadFailed");
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  void processAvatarFile(file);
}

onMounted(() => {
  window.addEventListener("paste", handleAvatarPaste);
});

async function confirmCrop() {
  if (!cropTarget) {
    localCropError.value = t("config.persona.cropMissingTarget");
    return;
  }
  if (!cropper) {
    localCropError.value = t("config.persona.cropperNotReady");
    return;
  }
  localCropError.value = "";
  const selection = cropper.getCropperSelection();
  if (!selection) {
    localCropError.value = t("config.persona.cropperNotReady");
    return;
  }
  try {
    const canvas = await selection.$toCanvas({
      width: 128,
      height: 128,
      beforeDraw(context) {
        context.imageSmoothingEnabled = true;
        context.imageSmoothingQuality = "high";
      },
    });
    const dataUrl = canvas.toDataURL("image/webp", 0.8);
    const marker = "base64,";
    const idx = dataUrl.indexOf(marker);
    if (idx < 0) {
      localCropError.value = t("config.persona.avatarSaveEncodeFailed");
      return;
    }
    const bytesBase64 = dataUrl.slice(idx + marker.length);
    emit("saveAgentAvatar", {
      agentId: cropTarget.agentId,
      mime: "image/webp",
      bytesBase64,
    });
    closeCropDialog();
  } catch (error) {
    localCropError.value = t("config.persona.avatarSaveEncodeFailed");
    console.warn("[配置][头像裁剪] 保存失败", error);
  }
}

onBeforeUnmount(() => {
  window.removeEventListener("paste", handleAvatarPaste);
  workspaceMigrationProgressUnlisten?.();
  destroyCropper();
});
</script>

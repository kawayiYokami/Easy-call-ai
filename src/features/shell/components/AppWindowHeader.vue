<template>
  <div
    class="min-h-10 h-10 shrink-0 relative z-40 overflow-visible select-none"
    :class="viewMode === 'chat' ? 'grid items-center bg-base-200 border-b border-base-300' : 'grid grid-cols-[1fr_auto_1fr] items-center bg-base-200 border-b border-base-300 px-2'"
    :style="viewMode === 'chat' ? chatHeaderGridStyle : undefined"
    @mousedown="handleHeaderMouseDown"
  >
    <div
      v-if="viewMode !== 'chat'"
      data-tauri-drag-region
      class="absolute inset-0"
      aria-hidden="true"
    ></div>
    <div
      v-else
      data-tauri-drag-region
      class="absolute inset-0 z-10"
      aria-hidden="true"
    ></div>
    <div
      v-if="viewMode === 'chat'"
      data-tauri-drag-region
      class="relative z-30 flex h-full min-w-0 items-center gap-1 px-2"
    >
      <div v-if="!detachedChatWindow && leftHeaderInLayout" class="indicator" @mousedown.stop>
        <span
          v-if="conversationUnreadTotal > 0"
          class="indicator-item indicator-top indicator-start z-10 h-2.5 w-2.5 -translate-x-0.5 -translate-y-0.5 rounded-full bg-error"
          aria-hidden="true"
        ></span>
        <button
          class="btn btn-ghost btn-sm h-8 min-h-8 px-2"
          :class="sideConversationListVisible ? 'btn-active' : ''"
          :title="t('chat.conversationList')"
          @click.stop="emit('toggle-side-conversation-list')"
        >
          <LayoutList class="h-3.5 w-3.5" />
        </button>
      </div>
    </div>

    <div
      v-if="viewMode === 'chat'"
      data-tauri-drag-region
      class="relative z-30 grid h-full min-w-0 grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-1 px-2"
    >
      <div class="relative z-40 flex min-w-0 items-center gap-1" @mousedown.stop>
        <div v-if="!detachedChatWindow && !leftHeaderInLayout" class="indicator">
          <span
            v-if="conversationUnreadTotal > 0"
            class="indicator-item indicator-top indicator-start z-10 h-2.5 w-2.5 -translate-x-0.5 -translate-y-0.5 rounded-full bg-error"
            aria-hidden="true"
          ></span>
          <button
            class="btn btn-ghost btn-sm h-8 min-h-8 px-2"
            :class="sideConversationListVisible ? 'btn-active' : ''"
            :title="t('chat.conversationList')"
            @click.stop="emit('toggle-side-conversation-list')"
          >
            <LayoutList class="h-3.5 w-3.5" />
          </button>
        </div>
        <button
          v-if="!detachedChatWindow"
          class="btn btn-ghost btn-sm h-8 min-h-8 px-2"
          :title="t('chat.newConversation')"
          @click.stop="handleCreateConversation"
        >
          <SquarePen class="h-4 w-4" />
        </button>
        <button
          class="btn btn-ghost btn-sm btn-square h-8 min-h-8 w-8 shrink-0"
          :disabled="trimming || chatting"
          :title="`${t('chat.contextUsageTitle', { percent: normalizedChatUsagePercent })} · ${trimTip}`"
          @click.stop="$emit('trimConversation')"
        >
          <svg
            class="h-5 w-5 -rotate-90"
            viewBox="0 0 36 36"
          >
            <circle
              cx="18"
              cy="18"
              r="14"
              fill="none"
              stroke="currentColor"
              stroke-width="4"
              class="opacity-20"
            />
            <circle
              cx="18"
              cy="18"
              r="14"
              fill="none"
              stroke="currentColor"
              stroke-width="4"
              stroke-linecap="round"
              :stroke-dasharray="circumference"
              :stroke-dashoffset="strokeDashoffset"
            />
          </svg>
        </button>
      </div>

      <div
        data-tauri-drag-region
        class="relative z-30 flex min-w-0 flex-1 self-stretch items-center justify-start gap-1 px-2"
        :title="combinedTitleTooltip"
      >
        <span
          class="truncate text-sm font-semibold text-base-content"
        >{{ combinedTitle }}</span>
      </div>

      <div class="relative z-40 flex min-w-0 items-center justify-end gap-1" @mousedown.stop>
      </div>
    </div>

    <div
      v-if="viewMode === 'chat'"
      data-tauri-drag-region
      class="relative z-30 flex h-full min-w-0 flex-nowrap items-center justify-end gap-1 px-2"
    >
      <button
        type="button"
        class="btn btn-ghost btn-sm btn-square h-8 min-h-8 w-8 shrink-0"
        :class="toolReviewPanelOpenVisible && chatRightPanelMode === 'reader' ? 'btn-active' : ''"
        :title="t('chat.readerPanelTab')"
        @mousedown.stop
        @click.stop="toggleChatRightPanelMode('reader')"
      >
        <Files class="h-3.5 w-3.5" />
      </button>
      <button
        type="button"
        class="btn btn-ghost btn-sm btn-square h-8 min-h-8 w-8 shrink-0"
        :class="toolReviewPanelOpenVisible && chatRightPanelMode !== 'reader' ? 'btn-active' : ''"
        :title="t('chat.delegatePanelTab')"
        @mousedown.stop
        @click.stop="toggleChatRightPanelMode('delegate')"
      >
        <View class="h-3.5 w-3.5" />
      </button>

      <button
        class="btn btn-ghost btn-sm"
        :title="openSettingsTitle || t('common.settings')"
        @mousedown.stop
        @click.stop="$emit('open-settings')"
      >
        <Settings class="h-3.5 w-3.5" />
      </button>

      <button
        v-if="showWindowControls"
        class="btn btn-ghost btn-sm"
        :title="t('window.minimize')"
        @mousedown.stop
        @click.stop="$emit('minimize-window')"
        :disabled="!windowReady"
      >
        <Minus class="h-3.5 w-3.5" />
      </button>
      <button
        v-if="showWindowControls"
        class="btn btn-ghost btn-sm"
        :title="maximized ? t('window.restore') : t('window.maximize')"
        @mousedown.stop
        @click.stop="$emit('toggle-maximize-window')"
        :disabled="!windowReady"
      >
        <Square class="h-3.5 w-3.5" />
      </button>
      <button
        v-if="showWindowControls"
        class="btn btn-sm btn-ghost hover:bg-error"
        :title="closeTitle || t('common.close')"
        @mousedown.stop
        @click.stop="$emit('close-window')"
        :disabled="!windowReady"
      >
        <X class="h-3.5 w-3.5" />
      </button>
    </div>

    <div v-if="viewMode !== 'chat'" class="relative z-10 min-w-0 justify-self-start flex items-center gap-2" @mousedown.stop>
      <button
        v-if="viewMode === 'config'"
        class="btn btn-ghost btn-sm h-8 min-h-8 w-8 px-0"
        :title="t('about.changelog')"
        @click.stop="openChangelogDialog"
      >
        <ScrollText class="h-3.5 w-3.5" />
      </button>

      <button
        v-if="viewMode === 'config' && showUpdateToLatestButton"
        class="btn btn-success btn-sm h-8 min-h-8 gap-2 px-3 relative shadow-sm"
        :title="updateToLatestTitle || ''"
        @click.stop="$emit('update-to-latest')"
      >
        <span
          v-if="hasAvailableUpdate && !checkingUpdate"
          class="absolute right-1.5 top-1.5 h-2 w-2 rounded-full bg-error"
          aria-hidden="true"
        ></span>
        <span v-if="checkingUpdate" class="loading loading-spinner loading-xs"></span>
        <Download v-else class="h-3.5 w-3.5" />
        <span>{{ updateToLatestLabel }}</span>
      </button>
    </div>

    <div
      v-if="viewMode !== 'chat'"
      class="relative z-10 w-fit min-w-0 flex items-center justify-center justify-self-center"
      @mousedown.stop
    >
      <label
        v-if="viewMode === 'config'"
        ref="configSearchPopoverRef"
        class="input input-bordered input-sm relative flex h-8 w-[clamp(9rem,24vw,13rem)] items-center gap-2 bg-base-100"
      >
        <Search class="h-3.5 w-3.5 opacity-70" />
        <input
          ref="configSearchInputRef"
          type="text"
          class="w-full bg-transparent outline-none"
          :value="configSearchQuery"
          :placeholder="configSearchPlaceholder"
          @focus="openSettingsSearchPopover"
          @input="handleConfigSearchInput"
          @keydown="handleConfigSearchKeydown"
        />
        <div
          v-if="configSearchOpen && String(configSearchQuery || '').trim()"
          class="absolute left-0 top-full mt-2 w-full overflow-hidden rounded-box border border-base-300 bg-base-100 shadow-lg"
        >
          <button
            v-for="result in configSearchResults"
            :key="result.tab"
            type="button"
            class="flex w-full flex-col items-start gap-0.5 px-3 py-2 text-left hover:bg-base-200"
            @click="selectConfigSearchResult(result.tab)"
          >
            <span class="text-sm font-medium">{{ result.title }}</span>
            <span v-if="result.matchedTexts[0]" class="text-xs opacity-60 truncate w-full">{{ result.matchedTexts[0] }}</span>
          </button>
          <div v-if="(configSearchResults || []).length === 0" class="px-3 py-3 text-sm opacity-60">
            {{ t("config.search.noResults") }}
          </div>
        </div>
      </label>

      <div v-else class="pointer-events-none flex items-center px-2">
        <span class="font-semibold text-sm">{{ titleText }}</span>
      </div>
    </div>

    <div v-if="viewMode !== 'chat'" class="relative z-10 flex shrink-0 flex-nowrap justify-self-end gap-1 px-2" @mousedown.stop>
      <button
        v-if="showWindowControls"
        class="btn btn-ghost btn-sm"
        :title="t('window.minimize')"
        @click.stop="$emit('minimize-window')"
        :disabled="!windowReady"
      >
        <Minus class="h-3.5 w-3.5" />
      </button>
      <button
        v-if="showWindowControls"
        class="btn btn-ghost btn-sm"
        :title="maximized ? t('window.restore') : t('window.maximize')"
        @click.stop="$emit('toggle-maximize-window')"
        :disabled="!windowReady"
      >
        <Square class="h-3.5 w-3.5" />
      </button>
      <button
        v-if="showWindowControls"
        class="btn btn-sm btn-ghost hover:bg-error"
        :title="closeTitle || t('common.close')"
        @click.stop="$emit('close-window')"
        :disabled="!windowReady"
      >
        <X class="h-3.5 w-3.5" />
      </button>
    </div>
  </div>

  <dialog v-if="viewMode === 'chat'" class="modal" :class="{ 'modal-open': createConversationDialogOpen }">
    <div class="modal-box max-w-lg">
      <div class="flex items-center justify-between gap-3">
        <h3 class="text-base font-semibold">{{ t("chat.newConversation") }}</h3>
        <button
          type="button"
          class="btn btn-ghost btn-sm btn-square"
          :disabled="importConversationLoading"
          :title="t('chat.importConversationExternal')"
          @click="importConversationFromExternal"
        >
          <span v-if="importConversationLoading" class="loading loading-spinner loading-xs"></span>
          <Download v-else class="h-4 w-4" />
        </button>
      </div>
      <div class="mt-3 flex flex-col gap-2.5">
        <div ref="createConversationTopicRootRef" class="relative flex flex-col gap-2">
          <input
            ref="createConversationInputRef"
            v-model="createConversationTitle"
            type="text"
            class="input input-bordered w-full"
            :placeholder="t('chat.newConversationTopicPlaceholder')"
            @focus="handleCreateConversationTopicFocus"
            @pointerdown="createConversationTopicSuggestionsOpen = true"
            @input="createConversationTopicSuggestionsOpen = true"
            @keydown="handleCreateConversationDialogKeydown"
          />
          <div
            v-if="showCreateConversationTopicSuggestions"
            class="absolute left-0 right-0 top-full z-50 mt-1 rounded-box border border-base-300 bg-base-100 p-2 shadow-xl"
          >
            <div class="flex max-h-24 flex-wrap gap-1.5 overflow-y-auto">
            <button
              v-for="topic in filteredRecentConversationTopics"
              :key="topic"
              type="button"
              class="btn btn-ghost btn-xs h-7 min-h-7 rounded-full px-3 text-xs font-medium text-base-content/75 hover:bg-base-200 hover:text-base-content"
              @click="applyRecentConversationTopic(topic)"
            >
              {{ topic }}
            </button>
            </div>
          </div>
        </div>
        <DepartmentPersonaSelect
          v-model:department-id="createConversationDepartmentId"
          v-model:agent-id="createConversationAgentId"
          :options="createConversationDepartmentOptions"
          auto-select-first
        />
        <div class="grid grid-cols-[minmax(0,1fr)_5rem] gap-2">
          <div class="join min-w-0">
            <select
              v-model="createConversationWorkspacePath"
              class="select select-bordered join-item min-w-0 flex-1"
              @change="handleCreateConversationWorkspaceChange"
            >
              <option value="">{{ t("chat.createConversationNoWorkspace") }}</option>
              <option
                v-for="workspace in selectableCreateConversationWorkspaces"
                :key="workspace.id"
                :value="workspace.path"
              >
                {{ workspace.name }}
              </option>
              <option
                v-if="createConversationCustomWorkspace && !selectableCreateConversationWorkspaces.some((item) => item.path === createConversationCustomWorkspace?.path)"
                :value="createConversationCustomWorkspace.path"
              >
                {{ createConversationCustomWorkspace.name }}
              </option>
            </select>
            <button
              type="button"
              class="btn btn-square join-item"
              :title="t('common.browse')"
              @click="pickCreateConversationWorkspace"
            >
              <FolderOpen class="h-4 w-4" />
            </button>
          </div>
          <select
            v-model="createConversationWorkspaceAccess"
            class="select select-bordered min-w-0"
            :disabled="!createConversationWorkspacePath"
          >
            <option value="approval">{{ workspaceAccessLabel("approval") }}</option>
            <option value="full_access">{{ workspaceAccessLabel("full_access") }}</option>
            <option value="read_only">{{ workspaceAccessLabel("read_only") }}</option>
          </select>
        </div>
      </div>
      <div class="modal-action mt-4 items-center justify-between gap-3">
        <label class="label min-w-0 cursor-pointer justify-start gap-3 p-0">
          <input v-model="createConversationMaxPermission" type="checkbox" class="checkbox checkbox-sm" />
          <span class="label-text truncate text-sm">{{ t("chat.createConversationMaxPermission") }}</span>
        </label>
        <div class="flex shrink-0 items-center justify-end gap-2">
          <button class="btn btn-sm" @click="closeCreateConversationDialog">{{ t("common.cancel") }}</button>
          <button class="btn btn-sm btn-primary" @click="confirmCreateConversation">{{ t("common.confirm") }}</button>
        </div>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="closeCreateConversationDialog">close</button>
    </form>
  </dialog>

  <WorkspaceDirectoryPickerDialog
    v-if="viewMode === 'chat'"
    :open="createConversationWorkspacePickerOpen"
    :saving="false"
    :loading="createConversationWorkspaceDirectoryLoading"
    :error-text="createConversationWorkspaceDirectoryError"
    :browser-path="createConversationWorkspaceBrowserPath"
    :manual-path="createConversationWorkspaceManualPath"
    :access="createConversationWorkspaceAccess"
    :autonomous-mode="createConversationMaxPermission"
    :directories="createConversationWorkspaceDirectoryItems"
    @close="closeCreateConversationWorkspacePicker"
    @browse="loadCreateConversationWorkspaceDirectory"
    @save="confirmCreateConversationWorkspacePicker"
    @update:manual-path="createConversationWorkspaceManualPath = $event"
    @update:access="handleCreateConversationWorkspacePickerAccessChange"
    @update:autonomous-mode="createConversationMaxPermission = $event"
  />

  <dialog v-if="viewMode === 'config'" class="modal" :class="{ 'modal-open': changelogDialogOpen }">
    <div class="modal-box h-[90vh] w-[90vw] max-w-none p-0">
      <div class="flex items-center justify-between border-b border-base-300 px-4 py-3">
        <div class="text-sm font-medium">{{ t("about.changelog") }}</div>
        <div class="flex items-center gap-2">
          <button
            type="button"
            class="btn btn-sm btn-ghost"
            :disabled="changelogLoading"
            @click="loadProjectChangelog(true)"
          >
            <span v-if="changelogLoading" class="loading loading-spinner loading-xs"></span>
            {{ t("common.refresh") }}
          </button>
          <button
            type="button"
            class="btn btn-sm btn-ghost"
            @click="closeChangelogDialog"
          >
            {{ t("common.close") }}
          </button>
        </div>
      </div>
      <div class="config-changelog-markdown h-[calc(90vh-118px)] overflow-auto px-5 py-4">
        <div v-if="changelogLoading && !changelogMarkdown" class="flex h-full min-h-0 items-center justify-center text-sm text-base-content/70">
          <span class="loading loading-spinner loading-sm mr-2"></span>
          {{ t("about.changelogLoading") }}
        </div>
        <div v-else-if="changelogError" class="rounded-box border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
          {{ changelogError }}
        </div>
        <AppMarkdownRenderer
          v-else-if="changelogMarkdown"
          class="ecall-markdown-content max-w-none"
          :text="changelogMarkdown"
          :is-dark="markdownIsDark"
          variant="document"
        />
        <div v-else class="flex h-full min-h-0 items-center justify-center text-sm text-base-content/70">
          {{ t("about.changelogEmpty") }}
        </div>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="closeChangelogDialog">close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { invokeTauri, isTauriRuntimeAvailable } from "../../../services/tauri-api";
import { Download, Files, FoldVertical, FolderOpen, History, LayoutList, Minus, ScrollText, Search, Settings, Square, SquarePen, View, X } from "@lucide/vue";
import type { ChatConversationOverviewItem, ShellWorkspace, ShellWorkspaceAccess } from "../../../types/app";
import { defaultWorkspaceNameFromPath } from "../../../utils/shell-workspaces";
import { buildWorkspaceConversationSections } from "../../chat/utils/conversation-sections";
import { resolveConversationDisplayTitle } from "../../chat/utils/conversation-title";
import { AppMarkdownRenderer, initKatex } from "../../chat/markdown";
import type { ConfigSearchResult, ConfigSearchTab } from "../../config/search/config-search";
import DepartmentPersonaSelect from "../../shared/components/DepartmentPersonaSelect.vue";
import WorkspaceDirectoryPickerDialog from "../../shared/components/WorkspaceDirectoryPickerDialog.vue";
import type { DepartmentPersonaOption } from "../../shared/department-persona-options";
import { isDarkAppTheme } from "../composables/use-app-theme";
import { usePipelineStatus } from "../composables/use-pipeline-status";

initKatex();

const RING_RADIUS = 14;
const RING_CIRCUMFERENCE = 2 * Math.PI * RING_RADIUS;

type ConversationDepartmentOption = DepartmentPersonaOption;

type CreateConversationInput = {
  title?: string;
  departmentId?: string;
  agentId?: string;
  copyCurrent?: boolean;
  importPath?: string;
  shellWorkspaces?: ShellWorkspace[];
  shellAutonomousMode?: boolean;
};

type WorkspaceDirectoryListResult = {
  path?: string;
  name?: string;
  entries?: Array<{ path?: string; name?: string; isDirectory?: boolean }>;
  directories?: Array<{ path?: string; name?: string }>;
};

const RECENT_CONVERSATION_TOPICS_STORAGE_KEY = "easy_call.recent_conversation_topics.v1";
const RECENT_CONVERSATION_TOPICS_LIMIT = 7;

const markdownIsDark = computed(() => isDarkAppTheme(props.currentTheme));

const props = withDefaults(defineProps<{
  viewMode: "chat" | "archives" | "config";
  detachedChatWindow?: boolean;
  currentTheme: string;
  titleText: string;
  chatUsagePercent: number;
  trimming: boolean;
  chatting: boolean;
  currentPersonaName: string;
  sideConversationListVisible: boolean;
  toolReviewPanelOpenVisible: boolean;
  chatSidePanelWidths: { leftWidth: number; rightWidth: number };
  conversationListTab: "local" | "contact" | "task";
  chatLeftPanelMode: "local" | "contact" | "task";
  chatRightPanelMode: "reader" | "review" | "delegate";
  activeConversationId: string;
  currentDepartmentId?: string;
  conversationItems: ChatConversationOverviewItem[];
  currentChatWorkspaces?: ShellWorkspace[];
  userAlias: string;
  userAvatarUrl: string;
  personaNameMap: Record<string, string>;
  personaAvatarUrlMap: Record<string, string>;
  createConversationDepartmentOptions: ConversationDepartmentOption[];
  defaultCreateConversationDepartmentId: string;
  trimTip: string;
  maximized: boolean;
  windowReady: boolean;
  openSettingsTitle: string;
  closeTitle?: string;
  configSearchQuery?: string;
  configSearchResults?: ConfigSearchResult[];
  configSearchPlaceholder?: string;
  showUpdateToLatestButton?: boolean;
  hasAvailableUpdate?: boolean;
  checkingUpdate?: boolean;
  updateToLatestLabel?: string;
  updateToLatestTitle?: string;
  windowControlsVisible?: boolean;
  directoryPickRestricted?: boolean;
  pipelineStatusEnabled?: boolean;
}>(), {
  windowControlsVisible: true,
  directoryPickRestricted: false,
  pipelineStatusEnabled: true,
});

const pipelineStatus = props.pipelineStatusEnabled
  ? usePipelineStatus({
    activeConversationId: computed(() => String(props.activeConversationId || "").trim()),
  })
  : null;

const emit = defineEmits<{
  (e: "open-settings"): void;
  (e: "open-archives"): void;
  (e: "toggle-side-conversation-list"): void;
  (e: "toggle-tool-review-panel"): void;
  (e: "update:conversation-list-tab", value: "local" | "contact" | "task"): void;
  (e: "update:chat-left-panel-mode", value: "local" | "contact" | "task"): void;
  (e: "update:chat-right-panel-mode", value: "reader" | "review" | "delegate"): void;
  (e: "minimize-window"): void;
  (e: "toggle-maximize-window"): void;
  (e: "switch-conversation", payload: { conversationId: string; kind?: "local_unarchived" | "remote_im_contact"; remoteContactId?: string }): void;
  (e: "rename-conversation", payload: { conversationId: string; title: string }): void;
  (e: "toggle-pin-conversation", conversationId: string): void;
  (e: "archive-conversation", conversationId: string): void;
  (e: "delete-conversation", conversationId: string): void;
  (e: "create-conversation", input?: CreateConversationInput): void;
  (e: "trimConversation"): void;
  (e: "startDrag"): void;
  (e: "close-window"): void;
  (e: "update:config-search-query", value: string): void;
  (e: "select-config-search-result", tab: ConfigSearchTab): void;
  (e: "update-to-latest"): void;
  (e: "directory-pick-restricted"): void;
}>();

const { t, locale } = useI18n();

const circumference = RING_CIRCUMFERENCE;
const showWindowControls = computed(() => props.windowControlsVisible !== false);

const normalizedChatUsagePercent = computed(() =>
  Math.min(100, Math.max(0, Math.round(Number(props.chatUsagePercent || 0)))),
);

const strokeDashoffset = computed(() => {
  const percent = normalizedChatUsagePercent.value;
  return RING_CIRCUMFERENCE * (1 - percent / 100);
});

// ========== responsive header pane layout ==========

const windowWidth = ref(typeof window === "undefined" ? 0 : window.innerWidth);

function updateWindowWidth() {
  windowWidth.value = typeof window === "undefined" ? 0 : window.innerWidth;
}

function headerPaneWidth(side: "left" | "right"): number {
  const raw = side === "left"
    ? Number(props.chatSidePanelWidths?.leftWidth || 0)
    : Number(props.chatSidePanelWidths?.rightWidth || 0);
  const min = 260;
  return Math.max(min, Number.isFinite(raw) && raw > 0 ? Math.round(raw) : min);
}

function headerCanFit(leftW: number, rightW: number): boolean {
  return windowWidth.value <= 0 || leftW + 300 + rightW <= windowWidth.value;
}

const leftHeaderInLayout = computed(() => {
  if (props.viewMode !== "chat" || !props.sideConversationListVisible || props.detachedChatWindow) return false;
  return headerCanFit(headerPaneWidth("left"), 0);
});

const rightHeaderInLayout = computed(() => {
  if (props.viewMode !== "chat" || !props.toolReviewPanelOpenVisible) return false;
  const rightW = headerPaneWidth("right");
  return leftHeaderInLayout.value
    ? headerCanFit(headerPaneWidth("left"), rightW)
    : headerCanFit(0, rightW);
});

const chatHeaderGridStyle = computed(() => {
  const leftColumn = leftHeaderInLayout.value
    ? `${headerPaneWidth("left")}px`
    : "0px";
  const rightColumn = rightHeaderInLayout.value
    ? `${headerPaneWidth("right")}px`
    : "max-content";
  return {
    gridTemplateColumns: `${leftColumn} minmax(0, 1fr) ${rightColumn}`,
  };
});


const currentConversationTitle = computed(() => {
  const activeId = String(props.activeConversationId || "").trim();
  if (!activeId) return "";
  const item = props.conversationItems.find((i) => i.conversationId === activeId);
  if (!item) return "";
  return resolveConversationDisplayTitle(item, {
    locale: locale.value,
    untitledLabel: t("chat.untitledConversation"),
  });
});

const currentConversationDepartmentName = computed(() => {
  const departmentId = String(props.currentDepartmentId || "").trim();
  if (departmentId) {
    const department = props.createConversationDepartmentOptions.find(
      (item) => String(item.departmentId || item.id || "").trim() === departmentId,
    );
    const departmentName = String(department?.name || "").trim();
    if (departmentName) return departmentName;
  }
  const activeId = String(props.activeConversationId || "").trim();
  if (!activeId) return "";
  const item = props.conversationItems.find((i) => i.conversationId === activeId);
  return item?.departmentName || "";
});

const conversationUnreadTotal = computed(() =>
  props.conversationItems.reduce((total, item) => total + Math.max(0, Number(item.unreadCount || 0)), 0),
);

const combinedTitle = computed(() => {
  const title = currentConversationTitle.value || props.currentPersonaName;
  const departmentName = currentConversationDepartmentName.value;
  return title && departmentName ? `${title} · ${departmentName}` : title;
});

const combinedTitleTooltip = computed(() => {
  return combinedTitle.value;
});

watch(
  () => props.activeConversationId,
  (conversationId) => pipelineStatus?.markConversationRead(conversationId),
  { immediate: true },
);
const configSearchPopoverRef = ref<HTMLElement | null>(null);
const configSearchInputRef = ref<HTMLInputElement | null>(null);
const createConversationInputRef = ref<HTMLInputElement | null>(null);
const createConversationTopicRootRef = ref<HTMLElement | null>(null);
const recentConversationTopics = ref<string[]>([]);
const createConversationDialogOpen = ref(false);
const createConversationTitle = ref("");
const createConversationDepartmentId = ref("");
const createConversationAgentId = ref("");
const createConversationTopicSuggestionsOpen = ref(false);
const suppressNextCreateConversationTopicFocus = ref(false);
const createConversationWorkspacePath = ref("");
const createConversationWorkspaceAccess = ref<ShellWorkspaceAccess>("approval");
const createConversationCustomWorkspace = ref<ShellWorkspace | null>(null);
const createConversationMaxPermission = ref(false);
const createConversationWorkspacePickerOpen = ref(false);
const createConversationWorkspaceManualPath = ref("");
const createConversationWorkspaceBrowserPath = ref("");
const createConversationWorkspaceDirectoryLoading = ref(false);
const createConversationWorkspaceDirectoryError = ref("");
const createConversationWorkspaceDirectoryItems = ref<Array<{ path: string; name: string }>>([]);
const importConversationLoading = ref(false);
const configSearchOpen = ref(false);
const changelogDialogOpen = ref(false);
const changelogLoading = ref(false);
const changelogError = ref("");
const changelogMarkdown = ref("");
const changelogLoaded = ref(false);

const currentWorkspaceAccessByPath = computed(() => {
  const map = new Map<string, ShellWorkspaceAccess>();
  for (const workspace of Array.isArray(props.currentChatWorkspaces) ? props.currentChatWorkspaces : []) {
    const path = String(workspace?.path || "").trim();
    const key = normalizeWorkspacePathKey(path);
    if (!path || map.has(key)) continue;
    map.set(key, normalizeWorkspaceAccess(workspace?.access));
  }
  return map;
});

const selectableCreateConversationWorkspaces = computed<ShellWorkspace[]>(() => {
  const deduped = new Map<string, ShellWorkspace>();
  for (const workspace of Array.isArray(props.currentChatWorkspaces) ? props.currentChatWorkspaces : []) {
    const path = String(workspace?.path || "").trim();
    const key = normalizeWorkspacePathKey(path);
    if (!path || deduped.has(key)) continue;
    deduped.set(key, {
      id: String(workspace?.id || "").trim() || `conversation-workspace-${path}`,
      name: String(workspace?.name || "").trim() || defaultWorkspaceNameFromPath(path) || path,
      path,
      level: "main",
      access: normalizeWorkspaceAccess(workspace?.access),
      builtIn: false,
    });
  }
  const localConversationItems = props.conversationItems.filter((item) =>
    item.kind !== "remote_im_contact" && !item.isPinned && !item.isSystemNotificationConversation,
  );
  for (const section of buildWorkspaceConversationSections(localConversationItems, {
    defaultWorkspaceTitle: t("chat.defaultWorkspace"),
    locale: locale.value,
  })) {
    const path = String(section.workspaceRootPath || "").trim();
    if (!path) continue;
    const key = normalizeWorkspacePathKey(path);
    if (deduped.has(key)) continue;
    deduped.set(key, {
      id: `conversation-workspace-${path}`,
      name: String(section.title || "").trim() || defaultWorkspaceNameFromPath(path) || path,
      path,
      level: "main",
      access: currentWorkspaceAccessByPath.value.get(normalizeWorkspacePathKey(path)) || "approval",
      builtIn: false,
    });
  }
  return Array.from(deduped.values());
});

const filteredRecentConversationTopics = computed(() => {
  const query = String(createConversationTitle.value || "").trim().toLocaleLowerCase();
  if (!query) return recentConversationTopics.value;
  const matched = recentConversationTopics.value.filter((topic) => topic.toLocaleLowerCase().includes(query));
  return matched.length > 0 ? matched : recentConversationTopics.value;
});

const showCreateConversationTopicSuggestions = computed(() =>
  createConversationTopicSuggestionsOpen.value
  && filteredRecentConversationTopics.value.length > 0,
);
async function openNativeDialog(options: { directory?: boolean; multiple?: boolean; filters?: Array<{ name: string; extensions: string[] }> }) {
  const dialog = await import("@tauri-apps/plugin-dialog");
  return dialog.open(options);
}

function normalizeWorkspacePathKey(path: string): string {
  return String(path || "").trim().toLowerCase();
}

function normalizeWorkspaceAccess(value: unknown): ShellWorkspaceAccess {
  const access = String(value || "").trim();
  if (access === "full_access" || access === "read_only" || access === "approval") {
    return access;
  }
  return "approval";
}

function resetCreateConversationWorkspace() {
  createConversationWorkspacePath.value = "";
  createConversationWorkspaceAccess.value = "approval";
  createConversationCustomWorkspace.value = null;
  createConversationMaxPermission.value = false;
  closeCreateConversationWorkspacePicker();
}

function workspaceAccessLabel(access: ShellWorkspaceAccess): string {
  if (access === "full_access") return t("config.tools.workspaceAccessFullAccess");
  if (access === "read_only") return t("config.tools.workspaceAccessReadOnly");
  return t("config.tools.workspaceAccessApproval");
}

function selectedCreateConversationWorkspace(): ShellWorkspace | undefined {
  const path = String(createConversationWorkspacePath.value || "").trim();
  if (!path) return undefined;
  const source = selectableCreateConversationWorkspaces.value.find((item) => item.path === path)
    || (createConversationCustomWorkspace.value?.path === path ? createConversationCustomWorkspace.value : null);
  const name = String(source?.name || "").trim() || defaultWorkspaceNameFromPath(path) || path;
  return {
    id: String(source?.id || "").trim() || `conversation-workspace-${Date.now().toString(36)}`,
    name,
    path,
    level: "main",
    access: normalizeWorkspaceAccess(createConversationWorkspaceAccess.value),
    builtIn: false,
  };
}

function createConversationWorkspacePayload(): ShellWorkspace[] | undefined {
  const workspace = selectedCreateConversationWorkspace();
  return workspace ? [workspace] : undefined;
}

function handleCreateConversationWorkspaceChange() {
  const path = String(createConversationWorkspacePath.value || "").trim();
  if (!path) {
    createConversationWorkspaceAccess.value = "approval";
    return;
  }
  const source = selectableCreateConversationWorkspaces.value.find((item) => item.path === path)
    || (createConversationCustomWorkspace.value?.path === path ? createConversationCustomWorkspace.value : null);
  createConversationWorkspaceAccess.value = normalizeWorkspaceAccess(source?.access);
}

function handleCreateConversationWorkspacePickerAccessChange(value: string) {
  createConversationWorkspaceAccess.value = normalizeWorkspaceAccess(value);
}

async function pickCreateConversationWorkspace() {
  if (props.directoryPickRestricted || !isTauriRuntimeAvailable()) {
    openCreateConversationWorkspacePicker();
    return;
  }
  const selected = await openNativeDialog({
    multiple: false,
    directory: true,
  });
  const path = String(Array.isArray(selected) ? selected[0] : selected || "").trim();
  if (!path) return;
  const existing = selectableCreateConversationWorkspaces.value.find((item) => item.path.toLowerCase() === path.toLowerCase());
  if (existing) {
    createConversationWorkspacePath.value = existing.path;
    createConversationWorkspaceAccess.value = normalizeWorkspaceAccess(existing.access);
    return;
  }
  const workspace: ShellWorkspace = {
    id: `conversation-workspace-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`,
    name: defaultWorkspaceNameFromPath(path) || path,
    path,
    level: "main",
    access: "approval",
    builtIn: false,
  };
  createConversationCustomWorkspace.value = workspace;
  createConversationWorkspacePath.value = workspace.path;
  createConversationWorkspaceAccess.value = workspace.access;
}

function openCreateConversationWorkspacePicker() {
  createConversationWorkspacePickerOpen.value = true;
  createConversationWorkspaceDirectoryError.value = "";
  createConversationWorkspaceDirectoryItems.value = [];
  const initialPath = String(createConversationWorkspacePath.value || "").trim();
  createConversationWorkspaceManualPath.value = initialPath;
  createConversationWorkspaceBrowserPath.value = initialPath;
  if (initialPath) {
    void loadCreateConversationWorkspaceDirectory(initialPath);
  }
}

function closeCreateConversationWorkspacePicker() {
  createConversationWorkspacePickerOpen.value = false;
  createConversationWorkspaceDirectoryLoading.value = false;
  createConversationWorkspaceDirectoryError.value = "";
  createConversationWorkspaceDirectoryItems.value = [];
  createConversationWorkspaceManualPath.value = "";
  createConversationWorkspaceBrowserPath.value = "";
}

async function loadCreateConversationWorkspaceDirectory(pathInput: string) {
  const path = String(pathInput || "").trim();
  if (!path || createConversationWorkspaceDirectoryLoading.value) return;
  createConversationWorkspaceDirectoryLoading.value = true;
  createConversationWorkspaceDirectoryError.value = "";
  try {
    const result = isTauriRuntimeAvailable()
      ? await invokeTauri<WorkspaceDirectoryListResult>("list_file_reader_directory", { path })
      : await invokeTauri<WorkspaceDirectoryListResult>("workspace.directory.list", { path });
    createConversationWorkspaceBrowserPath.value = String(result.path || path).trim();
    createConversationWorkspaceManualPath.value = createConversationWorkspaceBrowserPath.value;
    const rawDirectories = isTauriRuntimeAvailable()
      ? (Array.isArray(result.entries) ? result.entries.filter((item) => item && item.isDirectory) : [])
      : (Array.isArray(result.directories) ? result.directories : []);
    createConversationWorkspaceDirectoryItems.value = rawDirectories
      .map((item) => ({
        path: String(item.path || "").trim(),
        name: String(item.name || "").trim(),
      }))
      .filter((item) => !!item.path && !!item.name);
  } catch (error) {
    createConversationWorkspaceDirectoryError.value = String(error || "读取目录失败");
    createConversationWorkspaceDirectoryItems.value = [];
  } finally {
    createConversationWorkspaceDirectoryLoading.value = false;
  }
}

function confirmCreateConversationWorkspacePicker() {
  const path = String(createConversationWorkspaceManualPath.value || "").trim();
  if (!path) return;
  const existing = selectableCreateConversationWorkspaces.value.find((item) => item.path.toLowerCase() === path.toLowerCase());
  if (existing) {
    createConversationWorkspacePath.value = existing.path;
    createConversationWorkspaceAccess.value = normalizeWorkspaceAccess(existing.access);
    createConversationCustomWorkspace.value = null;
    closeCreateConversationWorkspacePicker();
    return;
  }
  const workspace: ShellWorkspace = {
    id: `conversation-workspace-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`,
    name: defaultWorkspaceNameFromPath(path) || path,
    path,
    level: "main",
    access: "approval",
    builtIn: false,
  };
  createConversationCustomWorkspace.value = workspace;
  createConversationWorkspacePath.value = workspace.path;
  createConversationWorkspaceAccess.value = workspace.access;
  closeCreateConversationWorkspacePicker();
}

function loadRecentConversationTopics() {
  try {
    const raw = window.localStorage.getItem(RECENT_CONVERSATION_TOPICS_STORAGE_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return;
    const normalized: string[] = [];
    const seen = new Set<string>();
    for (const item of parsed) {
      const text = String(item || "").trim();
      if (!text || seen.has(text)) continue;
      seen.add(text);
      normalized.push(text);
      if (normalized.length >= RECENT_CONVERSATION_TOPICS_LIMIT) break;
    }
    recentConversationTopics.value = normalized;
  } catch {
    recentConversationTopics.value = [];
  }
}

function saveRecentConversationTopics() {
  try {
    window.localStorage.setItem(RECENT_CONVERSATION_TOPICS_STORAGE_KEY, JSON.stringify(recentConversationTopics.value));
  } catch {
    // ignore persistence failures
  }
}

function pushRecentConversationTopic(rawText: string) {
  const text = String(rawText || "").trim();
  if (!text) return;
  recentConversationTopics.value = [text, ...recentConversationTopics.value.filter((item) => item !== text)].slice(0, RECENT_CONVERSATION_TOPICS_LIMIT);
  saveRecentConversationTopics();
}

function handleDocumentPointerDown(event: PointerEvent) {
  const target = event.target as Node | null;
  const searchRoot = configSearchPopoverRef.value;
  if (configSearchOpen.value && searchRoot && target && !searchRoot.contains(target)) {
    configSearchOpen.value = false;
  }
  const topicRoot = createConversationTopicRootRef.value;
  if (createConversationTopicSuggestionsOpen.value && topicRoot && target && !topicRoot.contains(target)) {
    createConversationTopicSuggestionsOpen.value = false;
  }
}

function handleWindowKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && configSearchOpen.value) {
    configSearchOpen.value = false;
  }
  if (event.key === "Escape" && changelogDialogOpen.value) {
    closeChangelogDialog();
  }
}

function openSettingsSearchPopover() {
  if (props.viewMode !== "config") return;
  configSearchOpen.value = true;
}

function handleConfigSearchInput(event: Event) {
  const value = (event.target as HTMLInputElement).value;
  emit("update:config-search-query", value);
  configSearchOpen.value = true;
}

function selectConfigSearchResult(tab: ConfigSearchTab) {
  emit("select-config-search-result", tab);
  configSearchOpen.value = false;
}

function handleConfigSearchKeydown(event: KeyboardEvent) {
  if (event.key === "Enter" && props.configSearchResults && props.configSearchResults.length > 0) {
    event.preventDefault();
    selectConfigSearchResult(props.configSearchResults[0].tab);
    return;
  }
  if (event.key === "Escape") {
    event.preventDefault();
    configSearchOpen.value = false;
  }
}

function handleHeaderMouseDown(event: MouseEvent) {
  if (event.button !== 0) return;
  if (event.defaultPrevented) return;
  if (event.ctrlKey || event.shiftKey || event.altKey || event.metaKey) return;
  const target = event.target as HTMLElement | null;
  if (!target) return;
  if (target.closest("button, input, textarea, select, option, a, label, summary, [role='button'], [contenteditable='true'], [data-no-drag]")) {
    return;
  }
  emit("startDrag");
}

function toggleChatRightPanelMode(mode: "reader" | "delegate") {
  const currentMode = props.chatRightPanelMode === "reader" ? "reader" : "delegate";
  if (props.toolReviewPanelOpenVisible && currentMode === mode) {
    emit("toggle-tool-review-panel");
    return;
  }
  emit("update:chat-right-panel-mode", mode);
}

function handleCreateConversation() {
  createConversationTitle.value = "";
  const activeConversation = props.conversationItems.find(
    (item) => String(item.conversationId || "").trim() === String(props.activeConversationId || "").trim(),
  );
  const option = resolveCreateConversationOption(
    String(activeConversation?.departmentId || "").trim() || String(props.defaultCreateConversationDepartmentId || "").trim(),
    String(activeConversation?.agentId || "").trim(),
  );
  createConversationDepartmentId.value = option?.departmentId || "";
  createConversationAgentId.value = option?.agentId || "";
  createConversationTopicSuggestionsOpen.value = false;
  suppressNextCreateConversationTopicFocus.value = true;
  resetCreateConversationWorkspace();
  const preferredWorkspace = selectableCreateConversationWorkspaces.value.find((item) => item.level === "main")
    || selectableCreateConversationWorkspaces.value[0];
  if (preferredWorkspace?.path) {
    createConversationWorkspacePath.value = preferredWorkspace.path;
    createConversationWorkspaceAccess.value = normalizeWorkspaceAccess(preferredWorkspace.access);
  }
  createConversationDialogOpen.value = true;
  nextTick(() => createConversationInputRef.value?.focus());
}

function openCreateConversationDialogWithWorkspace(workspace: ShellWorkspace) {
  handleCreateConversation();
  const path = String(workspace.path || "").trim();
  if (!path) return;
  const existing = selectableCreateConversationWorkspaces.value.find((item) => item.path.toLowerCase() === path.toLowerCase());
  const target: ShellWorkspace = existing || {
    id: String(workspace.id || "").trim() || `conversation-workspace-${Date.now().toString(36)}`,
    name: String(workspace.name || "").trim() || defaultWorkspaceNameFromPath(path) || path,
    path,
    level: "main",
    access: normalizeWorkspaceAccess(workspace.access),
    builtIn: false,
  };
  if (!existing) {
    createConversationCustomWorkspace.value = target;
  }
  createConversationWorkspacePath.value = target.path;
  createConversationWorkspaceAccess.value = normalizeWorkspaceAccess(target.access);
}

function handleOpenCreateConversationDialogEvent(event: Event) {
  const detail = (event as CustomEvent<{ workspace?: ShellWorkspace }>).detail;
  const workspace = detail?.workspace;
  if (!workspace?.path) {
    handleCreateConversation();
    return;
  }
  openCreateConversationDialogWithWorkspace(workspace);
}

async function loadProjectChangelog(force = false) {
  if (changelogLoading.value) return;
  if (changelogLoaded.value && !force) return;
  changelogLoading.value = true;
  changelogError.value = "";
  try {
    changelogMarkdown.value = await invokeTauri<string>("fetch_project_changelog_markdown");
    changelogLoaded.value = true;
  } catch (error) {
    changelogError.value = String(error);
  } finally {
    changelogLoading.value = false;
  }
}

function openChangelogDialog() {
  changelogDialogOpen.value = true;
  void loadProjectChangelog();
}

function closeChangelogDialog() {
  changelogDialogOpen.value = false;
}

function closeCreateConversationDialog() {
  createConversationDialogOpen.value = false;
  createConversationTitle.value = "";
  createConversationDepartmentId.value = "";
  createConversationAgentId.value = "";
  createConversationTopicSuggestionsOpen.value = false;
  resetCreateConversationWorkspace();
  importConversationLoading.value = false;
}

function applyRecentConversationTopic(topic: string) {
  createConversationTitle.value = String(topic || "").trim();
  createConversationTopicSuggestionsOpen.value = false;
  nextTick(() => createConversationInputRef.value?.focus());
}

function handleCreateConversationTopicFocus() {
  if (suppressNextCreateConversationTopicFocus.value) {
    suppressNextCreateConversationTopicFocus.value = false;
    return;
  }
  createConversationTopicSuggestionsOpen.value = true;
}

function resolveCreateConversationOption(departmentId?: string, agentId?: string): ConversationDepartmentOption | null {
  const did = String(departmentId || "").trim();
  const aid = String(agentId || "").trim();
  if (did && aid) {
    const exact = props.createConversationDepartmentOptions.find(
      (item) => item.departmentId === did && item.agentId === aid,
    );
    if (exact) return exact;
  }
  if (did) {
    const byDepartment = props.createConversationDepartmentOptions.find((item) => item.departmentId === did);
    if (byDepartment) return byDepartment;
  }
  return props.createConversationDepartmentOptions[0] || null;
}

function confirmCreateConversation() {
  const title = String(createConversationTitle.value || "").trim();
  const departmentId = String(createConversationDepartmentId.value || "").trim();
  const agentId = String(createConversationAgentId.value || "").trim();
  if (title) {
    pushRecentConversationTopic(title);
  }
  createConversationDialogOpen.value = false;
  createConversationTitle.value = "";
  createConversationDepartmentId.value = "";
  createConversationAgentId.value = "";
  createConversationTopicSuggestionsOpen.value = false;
  const shellWorkspaces = createConversationWorkspacePayload();
  const shellAutonomousMode = createConversationMaxPermission.value;
  resetCreateConversationWorkspace();
  emit("create-conversation", {
    title,
    departmentId: departmentId || undefined,
    agentId: agentId || undefined,
    shellWorkspaces,
    shellAutonomousMode,
  });
}

async function importConversationFromExternal() {
  if (importConversationLoading.value) return;
  importConversationLoading.value = true;
  try {
    const selected = await openNativeDialog({
      multiple: false,
      directory: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    const importPath = Array.isArray(selected) ? selected[0] : selected;
    const path = String(importPath || "").trim();
    if (!path) return;
    const title = String(createConversationTitle.value || "").trim();
    const departmentId = String(createConversationDepartmentId.value || "").trim();
    const agentId = String(createConversationAgentId.value || "").trim();
    if (title) {
      pushRecentConversationTopic(title);
    }
    createConversationDialogOpen.value = false;
    createConversationTitle.value = "";
    createConversationDepartmentId.value = "";
    createConversationAgentId.value = "";
    createConversationTopicSuggestionsOpen.value = false;
    const shellWorkspaces = createConversationWorkspacePayload();
    const shellAutonomousMode = createConversationMaxPermission.value;
    resetCreateConversationWorkspace();
    emit("create-conversation", {
      title,
      departmentId: departmentId || undefined,
      agentId: agentId || undefined,
      importPath: path,
      shellWorkspaces,
      shellAutonomousMode,
    });
  } finally {
    importConversationLoading.value = false;
  }
}

function handleCreateConversationDialogKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && createConversationTopicSuggestionsOpen.value) {
    event.preventDefault();
    createConversationTopicSuggestionsOpen.value = false;
    return;
  }
  if (event.key === "Enter" && !event.shiftKey && !event.ctrlKey && !event.altKey && !event.metaKey) {
    event.preventDefault();
    confirmCreateConversation();
  }
}

onMounted(() => {
  loadRecentConversationTopics();
  document.addEventListener("pointerdown", handleDocumentPointerDown);
  window.addEventListener("keydown", handleWindowKeydown);
  window.addEventListener("easy-call:open-create-conversation-dialog", handleOpenCreateConversationDialogEvent);
  updateWindowWidth();
  window.addEventListener("resize", updateWindowWidth, { passive: true });
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", handleDocumentPointerDown);
  window.removeEventListener("keydown", handleWindowKeydown);
  window.removeEventListener("easy-call:open-create-conversation-dialog", handleOpenCreateConversationDialogEvent);
  window.removeEventListener("resize", updateWindowWidth);
});
</script>

<style scoped>
.config-changelog-markdown:deep(.ecall-markdown-content.prose) {
  max-width: none;
}

.config-changelog-markdown:deep(.ecall-markdown-content) {
  color: inherit;
  line-height: 1.75;
  font-size: 0.95rem;
}

.config-changelog-markdown:deep(.ecall-markdown-content :where(p,ul,ol,blockquote,pre,table,figure,.paragraph-node,.list-node,.blockquote,.table-node-wrapper,.code-block-container,._mermaid,.vmr-container)) {
  margin-top: 0.85rem;
  margin-bottom: 0.85rem;
}

.config-changelog-markdown:deep(.ecall-markdown-content :where(h1,h2,h3,h4,.heading-node)) {
  margin-top: 1.25rem;
  margin-bottom: 0.75rem;
  font-weight: 700;
}

.config-changelog-markdown:deep(.ecall-markdown-content :where(a,.link-node)) {
  color: hsl(var(--p));
  text-decoration: underline;
}

.config-changelog-markdown:deep(.ecall-markdown-content :where(blockquote,.blockquote)) {
  border-left: 3px solid color-mix(in srgb, currentColor 18%, transparent);
  padding-left: 0.9rem;
  opacity: 0.9;
}

.config-changelog-markdown:deep(.ecall-markdown-content :where(:not(pre) > code,.inline-code)) {
  border: 1px solid color-mix(in srgb, currentColor 12%, transparent);
  border-radius: 0.45rem;
  padding: 0.08rem 0.35rem;
  background: color-mix(in srgb, currentColor 6%, transparent);
}
</style>

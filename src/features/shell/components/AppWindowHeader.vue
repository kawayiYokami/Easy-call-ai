<template>
  <div
    class="min-h-10 h-10 shrink-0 relative z-40 overflow-visible select-none"
    :class="viewMode === 'chat' ? 'grid items-center bg-base-200 border-b border-base-300' : 'grid grid-cols-[1fr_auto_1fr] items-center bg-base-200 border-b border-base-300 px-2'"
    :style="viewMode === 'chat' ? chatHeaderGridStyle : undefined"
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
      <div v-if="toolReviewPanelOpenVisible && rightHeaderInLayout" role="tablist" class="tabs tabs-border min-w-0 shrink-0" @mousedown.stop>
        <button
          type="button"
          role="tab"
          class="tab h-8 px-2"
          :class="chatRightPanelMode === 'reader' ? 'tab-active font-semibold' : ''"
          @click.stop="emit('update:chat-right-panel-mode', 'reader')"
        >
          {{ t("chat.readerPanelTab") }}
        </button>
        <button
          type="button"
          role="tab"
          class="tab h-8 px-2"
          :class="chatRightPanelMode !== 'reader' ? 'tab-active font-semibold' : ''"
          @click.stop="emit('update:chat-right-panel-mode', 'delegate')"
        >
          {{ t("chat.delegatePanelTab") }}
        </button>
      </div>
      <button
        v-else-if="toolReviewPanelOpenVisible"
        type="button"
        class="btn btn-ghost btn-sm btn-square h-8 min-h-8 w-8 shrink-0"
        :title="chatRightPanelMode === 'reader' ? t('chat.readerPanelTab') : t('chat.delegatePanelTab')"
        @mousedown.stop
        @click.stop="emit('update:chat-right-panel-mode', chatRightPanelMode === 'reader' ? 'delegate' : 'reader')"
      >
        <ArrowUpDown class="h-3.5 w-3.5" />
      </button>

      <button
        type="button"
        class="btn btn-ghost btn-sm h-8 min-h-8 px-2"
        :class="toolReviewPanelOpenVisible ? 'btn-active' : ''"
        :title="t('chat.rightSidebarToggle')"
        @mousedown.stop
        @click.stop="emit('toggle-tool-review-panel')"
      >
        <LayoutPanelLeft class="h-3.5 w-3.5 -scale-x-100" />
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
        class="btn btn-ghost btn-sm"
        :title="t('window.minimize')"
        @mousedown.stop
        @click.stop="$emit('minimize-window')"
        :disabled="!windowReady"
      >
        <Minus class="h-3.5 w-3.5" />
      </button>
      <button
        class="btn btn-ghost btn-sm"
        :title="maximized ? t('window.restore') : t('window.maximize')"
        @mousedown.stop
        @click.stop="$emit('toggle-maximize-window')"
        :disabled="!windowReady"
      >
        <Square class="h-3.5 w-3.5" />
      </button>
      <button
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
        class="input input-bordered input-sm relative flex h-8 w-[min(18rem,calc(100vw-8rem))] items-center gap-2 bg-base-100"
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
        class="btn btn-ghost btn-sm"
        :title="t('window.minimize')"
        @click.stop="$emit('minimize-window')"
        :disabled="!windowReady"
      >
        <Minus class="h-3.5 w-3.5" />
      </button>
      <button
        class="btn btn-ghost btn-sm"
        :title="maximized ? t('window.restore') : t('window.maximize')"
        @click.stop="$emit('toggle-maximize-window')"
        :disabled="!windowReady"
      >
        <Square class="h-3.5 w-3.5" />
      </button>
      <button
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
      <h3 class="text-base font-semibold">{{ t("chat.newConversation") }}</h3>
      <div class="mt-4 flex flex-col gap-3">
        <div class="flex flex-col gap-2">
          <input
            ref="createConversationInputRef"
            v-model="createConversationTitle"
            type="text"
            class="input input-bordered w-full"
            :placeholder="t('chat.newConversationTopicPlaceholder')"
            @keydown="handleCreateConversationDialogKeydown"
          />
          <div v-if="recentConversationTopics.length > 0" class="flex flex-wrap gap-1.5">
            <button
              v-for="topic in recentConversationTopics"
              :key="topic"
              type="button"
              class="btn btn-ghost btn-xs h-7 min-h-7 rounded-full px-3 text-xs font-medium text-base-content/75"
              @click="applyRecentConversationTopic(topic)"
            >
              {{ topic }}
            </button>
          </div>
        </div>
        <select
          v-model="createConversationDepartmentId"
          class="select select-bordered w-full"
        >
          <option
            v-for="department in createConversationDepartmentOptions"
            :key="department.id"
            :value="department.id"
          >
            {{ departmentOptionLabel(department) }}
          </option>
        </select>
        <label class="flex h-10 cursor-pointer items-center justify-start gap-3 px-1">
          <input v-model="createConversationCopyCurrent" type="checkbox" class="checkbox checkbox-sm" />
          <span class="label-text text-sm">{{ t("chat.copyCurrentConversation") }}</span>
        </label>
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
      <div class="modal-action mt-5 items-center justify-between gap-3">
        <label class="label min-w-0 cursor-pointer justify-start gap-2 p-0">
          <input v-model="createConversationMaxPermission" type="checkbox" class="checkbox checkbox-sm" />
          <span class="label-text truncate text-sm">{{ t("chat.createConversationMaxPermission") }}</span>
        </label>
        <div class="flex shrink-0 items-center justify-end gap-2">
          <button class="btn btn-sm btn-ghost gap-2 px-2" :disabled="importConversationLoading" @click="importConversationFromExternal">
            <span v-if="importConversationLoading" class="loading loading-spinner loading-xs"></span>
            <Upload v-else class="h-4 w-4" />
            <span>{{ t("chat.importConversationExternal") }}</span>
          </button>
          <button class="btn btn-sm" @click="closeCreateConversationDialog">{{ t("common.cancel") }}</button>
          <button class="btn btn-sm btn-primary" @click="confirmCreateConversation">{{ t("common.confirm") }}</button>
        </div>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="closeCreateConversationDialog">close</button>
    </form>
  </dialog>

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
import { invokeTauri } from "../../../services/tauri-api";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { ArrowUpDown, Download, FoldVertical, FolderOpen, History, LayoutList, LayoutPanelLeft, Minus, ScrollText, Search, Settings, Square, SquarePen, Upload, X } from "@lucide/vue";
import type { ChatConversationOverviewItem, ShellWorkspace, ShellWorkspaceAccess } from "../../../types/app";
import { defaultWorkspaceNameFromPath } from "../../../utils/shell-workspaces";
import { resolveConversationDisplayTitle } from "../../chat/utils/conversation-title";
import { AppMarkdownRenderer, initKatex } from "../../chat/markdown";
import type { ConfigSearchResult, ConfigSearchTab } from "../../config/search/config-search";
import { isDarkAppTheme } from "../composables/use-app-theme";
import { usePipelineStatus } from "../composables/use-pipeline-status";

initKatex();

const RING_RADIUS = 14;
const RING_CIRCUMFERENCE = 2 * Math.PI * RING_RADIUS;

type ConversationDepartmentOption = {
  id: string;
  name: string;
  ownerAgentId?: string;
  ownerName: string;
  providerName?: string;
  modelName?: string;
};

type CreateConversationInput = {
  title?: string;
  departmentId?: string;
  copyCurrent?: boolean;
  importPath?: string;
  shellWorkspaces?: ShellWorkspace[];
  shellAutonomousMode?: boolean;
};

const RECENT_CONVERSATION_TOPICS_STORAGE_KEY = "easy_call.recent_conversation_topics.v1";
const RECENT_CONVERSATION_TOPICS_LIMIT = 7;
const RECENT_CREATE_CONVERSATION_WORKSPACES_STORAGE_KEY = "easy_call.recent_create_conversation_workspaces.v1";
const RECENT_CREATE_CONVERSATION_WORKSPACES_LIMIT = 12;

const { markConversationRead } = usePipelineStatus({
  activeConversationId: computed(() => String(props.activeConversationId || "").trim()),
});

const markdownIsDark = computed(() => isDarkAppTheme(props.currentTheme));

const props = defineProps<{
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
  conversationListTab: "local" | "contact";
  chatLeftPanelMode: "local" | "contact";
  chatRightPanelMode: "reader" | "review" | "delegate";
  activeConversationId: string;
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
}>();

const emit = defineEmits<{
  (e: "open-settings"): void;
  (e: "open-archives"): void;
  (e: "toggle-side-conversation-list"): void;
  (e: "toggle-tool-review-panel"): void;
  (e: "update:conversation-list-tab", value: "local" | "contact"): void;
  (e: "update:chat-left-panel-mode", value: "local" | "contact"): void;
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
}>();

const { t, locale } = useI18n();

const circumference = RING_CIRCUMFERENCE;

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
  const activeId = String(props.activeConversationId || "").trim();
  if (!activeId) return "";
  const item = props.conversationItems.find((i) => i.conversationId === activeId);
  return item?.departmentName || "";
});

const conversationUnreadTotal = computed(() =>
  props.conversationItems.reduce((total, item) => total + Math.max(0, Number(item.unreadCount || 0)), 0),
);

const combinedTitle = computed(() => {
  return currentConversationTitle.value || props.currentPersonaName;
});

const combinedTitleTooltip = computed(() => {
  return currentConversationTitle.value || props.currentPersonaName;
});

watch(
  () => props.activeConversationId,
  (conversationId) => markConversationRead(conversationId),
  { immediate: true },
);
const configSearchPopoverRef = ref<HTMLElement | null>(null);
const configSearchInputRef = ref<HTMLInputElement | null>(null);
const createConversationInputRef = ref<HTMLInputElement | null>(null);
const recentConversationTopics = ref<string[]>([]);
const createConversationDialogOpen = ref(false);
const createConversationTitle = ref("");
const createConversationDepartmentId = ref("");
const createConversationCopyCurrent = ref(false);
const createConversationWorkspacePath = ref("");
const createConversationWorkspaceAccess = ref<ShellWorkspaceAccess>("approval");
const createConversationCustomWorkspace = ref<ShellWorkspace | null>(null);
const createConversationMaxPermission = ref(false);
const recentCreateConversationWorkspaces = ref<ShellWorkspace[]>([]);
const importConversationLoading = ref(false);
const configSearchOpen = ref(false);
const changelogDialogOpen = ref(false);
const changelogLoading = ref(false);
const changelogError = ref("");
const changelogMarkdown = ref("");
const changelogLoaded = ref(false);

const selectableCreateConversationWorkspaces = computed<ShellWorkspace[]>(() =>
  mergeCreateConversationWorkspaces([
    ...(props.currentChatWorkspaces || []),
    ...recentCreateConversationWorkspaces.value,
  ])
    .filter((item) => item.level !== "system" && String(item.path || "").trim())
    .map((item) => ({
      id: String(item.id || "").trim() || `conversation-workspace-${String(item.path || "").trim()}`,
      name: String(item.name || "").trim() || defaultWorkspaceNameFromPath(item.path) || item.path,
      path: String(item.path || "").trim(),
      level: item.level === "main" ? "main" : "secondary",
      access: normalizeWorkspaceAccess(item.access),
      builtIn: false,
    })),
);

function normalizeWorkspacePathKey(path: string): string {
  return String(path || "").trim().toLowerCase();
}

function mergeCreateConversationWorkspaces(items: ShellWorkspace[]): ShellWorkspace[] {
  const merged: ShellWorkspace[] = [];
  const seen = new Set<string>();
  for (const item of items) {
    const path = String(item.path || "").trim();
    const key = normalizeWorkspacePathKey(path);
    if (!path || seen.has(key)) continue;
    seen.add(key);
    merged.push({ ...item, path });
  }
  return merged;
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

function loadRecentCreateConversationWorkspaces() {
  try {
    const raw = window.localStorage.getItem(RECENT_CREATE_CONVERSATION_WORKSPACES_STORAGE_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return;
    recentCreateConversationWorkspaces.value = mergeCreateConversationWorkspaces(parsed.map((item): ShellWorkspace => ({
      id: String(item?.id || "").trim(),
      name: String(item?.name || "").trim(),
      path: String(item?.path || "").trim(),
      level: "secondary",
      access: normalizeWorkspaceAccess(item?.access),
      builtIn: false,
    }))).slice(0, RECENT_CREATE_CONVERSATION_WORKSPACES_LIMIT);
  } catch {
    recentCreateConversationWorkspaces.value = [];
  }
}

function saveRecentCreateConversationWorkspaces() {
  try {
    window.localStorage.setItem(
      RECENT_CREATE_CONVERSATION_WORKSPACES_STORAGE_KEY,
      JSON.stringify(recentCreateConversationWorkspaces.value),
    );
  } catch {
    // ignore persistence failures
  }
}

function pushRecentCreateConversationWorkspace(workspace: ShellWorkspace | undefined) {
  if (!workspace?.path) return;
  const normalized: ShellWorkspace = {
    id: workspace.id || `conversation-workspace-${Date.now().toString(36)}`,
    name: workspace.name || defaultWorkspaceNameFromPath(workspace.path) || workspace.path,
    path: workspace.path,
    level: "secondary",
    access: normalizeWorkspaceAccess(workspace.access),
    builtIn: false,
  };
  const targetKey = normalizeWorkspacePathKey(normalized.path);
  recentCreateConversationWorkspaces.value = [
    normalized,
    ...recentCreateConversationWorkspaces.value.filter((item) => normalizeWorkspacePathKey(item.path) !== targetKey),
  ].slice(0, RECENT_CREATE_CONVERSATION_WORKSPACES_LIMIT);
  saveRecentCreateConversationWorkspaces();
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

async function pickCreateConversationWorkspace() {
  const selected = await openDialog({
    multiple: false,
    directory: true,
  });
  const path = String(Array.isArray(selected) ? selected[0] : selected || "").trim();
  if (!path) return;
  const existing = selectableCreateConversationWorkspaces.value.find((item) => item.path.toLowerCase() === path.toLowerCase());
  if (existing) {
    createConversationWorkspacePath.value = existing.path;
    createConversationWorkspaceAccess.value = normalizeWorkspaceAccess(existing.access);
    pushRecentCreateConversationWorkspace(existing);
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
  pushRecentCreateConversationWorkspace(workspace);
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

function handleCreateConversation() {
  createConversationTitle.value = "";
  const activeConversation = props.conversationItems.find(
    (item) => String(item.conversationId || "").trim() === String(props.activeConversationId || "").trim(),
  );
  createConversationDepartmentId.value =
    String(activeConversation?.departmentId || "").trim()
    || String(props.defaultCreateConversationDepartmentId || "").trim()
    || String(props.createConversationDepartmentOptions[0]?.id || "").trim();
  createConversationCopyCurrent.value = false;
  resetCreateConversationWorkspace();
  createConversationDialogOpen.value = true;
  nextTick(() => createConversationInputRef.value?.focus());
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
  createConversationCopyCurrent.value = false;
  resetCreateConversationWorkspace();
  importConversationLoading.value = false;
}

function applyRecentConversationTopic(topic: string) {
  createConversationTitle.value = String(topic || "").trim();
  nextTick(() => createConversationInputRef.value?.focus());
}

function departmentOptionLabel(department: ConversationDepartmentOption): string {
  const departmentName = String(department.name || "").trim();
  const ownerName = String(department.ownerName || "").trim();
  return ownerName ? `${departmentName} / ${ownerName}` : departmentName;
}

function confirmCreateConversation() {
  const title = String(createConversationTitle.value || "").trim();
  const departmentId = String(createConversationDepartmentId.value || "").trim();
  const copyCurrent = !!createConversationCopyCurrent.value;
  if (title) {
    pushRecentConversationTopic(title);
  }
  createConversationDialogOpen.value = false;
  createConversationTitle.value = "";
  createConversationDepartmentId.value = "";
  createConversationCopyCurrent.value = false;
  const shellWorkspaces = createConversationWorkspacePayload();
  const shellAutonomousMode = createConversationMaxPermission.value;
  pushRecentCreateConversationWorkspace(shellWorkspaces?.[0]);
  resetCreateConversationWorkspace();
  emit("create-conversation", {
    title,
    departmentId: departmentId || undefined,
    copyCurrent,
    shellWorkspaces,
    shellAutonomousMode,
  });
}

async function importConversationFromExternal() {
  if (importConversationLoading.value) return;
  importConversationLoading.value = true;
  try {
    const selected = await openDialog({
      multiple: false,
      directory: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    const importPath = Array.isArray(selected) ? selected[0] : selected;
    const path = String(importPath || "").trim();
    if (!path) return;
    const title = String(createConversationTitle.value || "").trim();
    const departmentId = String(createConversationDepartmentId.value || "").trim();
    if (title) {
      pushRecentConversationTopic(title);
    }
    createConversationDialogOpen.value = false;
    createConversationTitle.value = "";
    createConversationDepartmentId.value = "";
    createConversationCopyCurrent.value = false;
    const shellWorkspaces = createConversationWorkspacePayload();
    const shellAutonomousMode = createConversationMaxPermission.value;
    pushRecentCreateConversationWorkspace(shellWorkspaces?.[0]);
    resetCreateConversationWorkspace();
    emit("create-conversation", {
      title,
      departmentId: departmentId || undefined,
      copyCurrent: false,
      importPath: path,
      shellWorkspaces,
      shellAutonomousMode,
    });
  } finally {
    importConversationLoading.value = false;
  }
}

function handleCreateConversationDialogKeydown(event: KeyboardEvent) {
  if (event.key === "Enter" && !event.shiftKey && !event.ctrlKey && !event.altKey && !event.metaKey) {
    event.preventDefault();
    confirmCreateConversation();
  }
}

onMounted(() => {
  loadRecentConversationTopics();
  loadRecentCreateConversationWorkspaces();
  document.addEventListener("pointerdown", handleDocumentPointerDown);
  window.addEventListener("keydown", handleWindowKeydown);
  updateWindowWidth();
  window.addEventListener("resize", updateWindowWidth, { passive: true });
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", handleDocumentPointerDown);
  window.removeEventListener("keydown", handleWindowKeydown);
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

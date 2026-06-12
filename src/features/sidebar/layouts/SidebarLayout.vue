<template>
  <div class="relative flex h-full min-h-0 flex-col bg-base-200 text-base-content">
    <AppWindowHeader
      v-if="connected"
      view-mode="chat"
      current-theme=""
      :title-text="activeTitle"
      :chat-usage-percent="chatUsagePercent || 0"
      :trimming="compacting"
      :chatting="false"
      current-persona-name="PAI"
      :side-conversation-list-visible="sideConversationListVisible"
      :tool-review-panel-open-visible="toolReviewPanelOpenVisible"
      :chat-side-panel-widths="chatSidePanelWidths || { leftWidth: 320, rightWidth: 320 }"
      :conversation-list-tab="conversationListTab || 'local'"
      :chat-left-panel-mode="chatLeftPanelMode || 'local'"
      :chat-right-panel-mode="chatRightPanelMode || 'review'"
      :active-conversation-id="activeConversationId"
      :current-department-id="currentDepartmentId"
      :conversation-items="conversationItems || []"
      :current-chat-workspaces="currentWorkspaces || []"
      :user-alias="userAlias || '我'"
      :user-avatar-url="userAvatarUrl || ''"
      :persona-name-map="personaNameMap || {}"
      :persona-avatar-url-map="personaAvatarUrlMap || {}"
      :create-conversation-department-options="createConversationDepartmentOptions || []"
      :default-create-conversation-department-id="defaultCreateConversationDepartmentId || ''"
      trim-tip=""
      :maximized="false"
      :window-ready="false"
      :open-settings-title="'设置'"
      :window-controls-visible="false"
      directory-pick-restricted
      @toggle-side-conversation-list="$emit('toggleSideConversationList')"
      @toggle-tool-review-panel="$emit('toggleToolReviewPanel')"
      @update:conversation-list-tab="$emit('updateConversationListTab', $event)"
      @update:chat-left-panel-mode="$emit('updateChatLeftPanelMode', $event)"
      @update:chat-right-panel-mode="$emit('updateChatRightPanelMode', $event)"
      @open-settings="$emit('openSettings')"
      @create-conversation="$emit('createConversation', $event)"
      @trim-conversation="$emit('compactConversation')"
      @directory-pick-restricted="$emit('directoryPickRestricted')"
    />
    <header v-else class="flex h-10 shrink-0 items-center gap-1 border-b border-base-300 px-2">
      <button
        class="btn btn-ghost btn-sm h-8 min-h-8 w-8 shrink-0 px-0"
        title="会话列表"
        @click="view === 'list' ? $emit('showChat') : $emit('showList')"
      >
        <ChevronLeft class="h-4 w-4" />
      </button>

      <div class="flex min-w-0 flex-1 items-center justify-center gap-1 px-1">
        <button
          class="btn btn-ghost btn-sm h-8 min-h-8 w-8 shrink-0 px-0"
          title="新建会话"
          @click="$emit('newConversation')"
        >
          <SquarePen class="h-4 w-4" />
        </button>
        <span class="pointer-events-none min-w-0 truncate text-sm font-semibold text-base-content">{{ displayTitle }}</span>
        <button
          class="btn btn-ghost btn-sm btn-square h-8 min-h-8 w-8 shrink-0"
          :disabled="compacting || !activeConversationId"
          :title="`上下文用量 ${chatUsagePercent}%`"
          @click="$emit('compactConversation')"
        >
          <svg class="h-5 w-5 -rotate-90" viewBox="0 0 36 36">
            <circle cx="18" cy="18" r="14" fill="none" stroke="currentColor" stroke-width="4" class="opacity-20" />
            <circle
              cx="18" cy="18" r="14" fill="none" stroke="currentColor" stroke-width="4" stroke-linecap="round"
              :stroke-dasharray="usageRingCircumference" :stroke-dashoffset="usageRingOffset"
            />
          </svg>
        </button>
      </div>

      <button
        class="btn btn-ghost btn-sm h-8 min-h-8 w-8 shrink-0 px-0"
        title="审查"
        @click="$emit('toggleReviewPanel')"
      >
        <FileSearch class="h-4 w-4" />
      </button>
      <button
        class="btn btn-ghost btn-sm h-8 min-h-8 w-8 shrink-0 px-0"
        title="设置"
        @click="$emit('openSettings')"
      >
        <Settings class="h-4 w-4" />
      </button>
    </header>
    <div v-if="!connected" class="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 px-4 text-center">
      <div class="text-sm font-medium">{{ connecting ? "正在连接 PAI" : (errorText || "PAI 未运行") }}</div>
      <button class="btn btn-sm btn-primary" :disabled="connecting" @click="$emit('reconnect')">
        <RefreshCcw class="h-4 w-4" />
        重连
      </button>
    </div>
    <main v-else class="min-h-0 flex-1">
      <slot />
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { ChevronLeft, FileSearch, RefreshCcw, Settings, SquarePen } from "@lucide/vue";
import AppWindowHeader from "../../shell/components/AppWindowHeader.vue";
import type { ChatConversationOverviewItem, ShellWorkspace } from "../../../types/app";
import type { DepartmentPersonaOption } from "../../shared/department-persona-options";

const props = defineProps<{
  view: "list" | "chat";
  connected: boolean;
  connecting: boolean;
  errorText: string;
  activeTitle: string;
  activeConversationId: string;
  compacting: boolean;
  chatUsagePercent?: number;
  sideConversationListVisible?: boolean;
  toolReviewPanelOpenVisible?: boolean;
  chatSidePanelWidths?: { leftWidth: number; rightWidth: number };
  conversationListTab?: "local" | "contact" | "task";
  chatLeftPanelMode?: "local" | "contact" | "task";
  chatRightPanelMode?: "reader" | "review" | "delegate";
  currentDepartmentId?: string;
  conversationItems?: ChatConversationOverviewItem[];
  currentWorkspaces?: ShellWorkspace[];
  userAlias?: string;
  userAvatarUrl?: string;
  personaNameMap?: Record<string, string>;
  personaAvatarUrlMap?: Record<string, string>;
  createConversationDepartmentOptions?: DepartmentPersonaOption[];
  defaultCreateConversationDepartmentId?: string;
}>();

const usageRingCircumference = 2 * Math.PI * 14;
const usageRingOffset = computed(() => {
  const percent = Math.min(100, Math.max(0, Number(props.chatUsagePercent || 0)));
  return usageRingCircumference * (1 - percent / 100);
});

const displayTitle = computed(() => {
  const raw = props.activeTitle || "PAI";
  return raw.length > 10 ? raw.slice(0, 10) + "…" : raw;
});

defineEmits<{
  showList: [];
  showChat: [];
  newConversation: [];
  openSettings: [];
  compactConversation: [];
  reconnect: [];
  toggleReviewPanel: [];
  toggleSideConversationList: [];
  toggleToolReviewPanel: [];
  updateConversationListTab: [value: "local" | "contact" | "task"];
  updateChatLeftPanelMode: [value: "local" | "contact" | "task"];
  updateChatRightPanelMode: [value: "reader" | "review" | "delegate"];
  createConversation: [input?: { title?: string; departmentId?: string; agentId?: string; copyCurrent?: boolean; importPath?: string; shellWorkspaces?: ShellWorkspace[]; shellAutonomousMode?: boolean }];
  directoryPickRestricted: [];
}>();
</script>

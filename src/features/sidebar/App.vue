<template>
  <SidebarLayout
    :view="view"
    :connected="transport.connected.value"
    :connecting="transport.connecting.value"
    :error-text="transport.errorText.value"
    :active-title="activeTitle"
    :active-conversation-id="activeConversationId"
    :compacting="compacting"
    :chat-usage-percent="chatUsagePercent"
    :side-conversation-list-visible="sideConversationListVisible"
    :tool-review-panel-open-visible="toolReviewPanelOpenVisible"
    :chat-side-panel-widths="chatSidePanelWidths"
    :conversation-list-tab="conversationListTab"
    :chat-left-panel-mode="chatLeftPanelMode"
    :chat-right-panel-mode="chatRightPanelMode"
    :current-department-id="activeDepartmentId"
    :conversation-items="chatConversationItems"
    :current-workspaces="sidebarShellWorkspaces"
    :user-alias="sidebarUserAlias"
    :user-avatar-url="sidebarUserAvatarUrl"
    :persona-name-map="sidebarPersonaNameMap"
    :persona-avatar-url-map="sidebarPersonaAvatarUrlMap"
    :create-conversation-department-options="createConversationDepartmentOptions"
    :default-create-conversation-department-id="defaultCreateConversationDepartmentId"
    @new-conversation="openCreateConversationDialog"
    @open-settings="openSettings"
    @compact-conversation="openCompactionDialog"
    @reconnect="refreshDiscovery"
    @toggle-review-panel="toggleReviewPanel"
    @toggle-side-conversation-list="toggleSideConversationList"
    @toggle-tool-review-panel="toggleToolReviewPanel"
    @update-conversation-list-tab="updateConversationListTab"
    @update-chat-left-panel-mode="updateChatLeftPanelMode"
    @update-chat-right-panel-mode="updateChatRightPanelMode"
    @create-conversation="handleCreateConversationRequest"
    @directory-pick-restricted="handleDirectoryPickRestricted"
  >
    <ChatViewWrapper
      ref="chatViewWrapperRef"
      v-model:input="inputText"
      :active-conversation-id="activeConversationId"
      :active-agent-id="activeAgentId"
      :persona="persona"
      :conversation-call-primary-api-config-id="conversationCallPrimaryApiConfigId"
      :preferred-chat-model-id="preferredChatModelId"
      :chat-model-options="chatModelOptions"
      :workspace-access="workspaceAccess"
      :plan-mode-enabled="activeConversationPlanModeEnabled"
      :system-notification-mode="activeConversationSystemNotificationMode"
      :messages="messages"
      :conversation-items="chatConversationItems"
      :remote-im-contact-conversations="remoteImContactConversations"
      :clipboard-images="clipboardImages"
      :streaming-text="streamingText"
      :tool-status-text="toolStatusText"
      :tool-status-state="toolStatusState"
      :stream-blocks="streamBlocks"
      :busy="busy"
      :runtime-state="activeConversationRuntimeState"
      :has-prev-block="hasPrevBlock"
      :create-conversation-department-options="createConversationDepartmentOptions"
      :default-create-conversation-department-id="defaultCreateConversationDepartmentId"
      :current-department-id="activeDepartmentId"
      :current-workspace-name="currentWorkspaceName"
      :current-workspace-root-path="workspaceRootPath"
      :current-workspaces="sidebarShellWorkspaces"
      :current-todos="sidebarTodos"
      :hide-workspace-button="hideWorkspaceButton"
      :terminal-approvals="activeConversationTerminalApprovals"
      :terminal-approval-resolving="terminalApprovalResolving"
      :ide-context-groups="vscodeIdeContextGroups"
      :read-plan-file-content="readPlanFileContent"
      :bridge-request="transport.request"
      :side-conversation-list-visible="sideConversationListVisible"
      :tool-review-panel-open-visible="toolReviewPanelOpenVisible"
      :conversation-list-tab="conversationListTab"
      :chat-left-panel-mode="chatLeftPanelMode"
      :chat-right-panel-mode="chatRightPanelMode"
      :supervision-active="sidebarSupervisionActive"
      :supervision-title="sidebarSupervisionTitle"
      @send="send"
      @stop="stop"
      @remove-clipboard-image="removeClipboardImage"
      @pick-attachments="pickAttachments"
      @load-prev-block="loadPrevBlock"
      @update:conversation-preferred-api-config-id="selectConversationPreferredModel"
      @update-workspace-access="selectWorkspaceAccess"
      @side-conversation-list-visible-change="sideConversationListVisible = $event"
      @tool-review-panel-open-change="toolReviewPanelOpenVisible = $event"
      @side-panel-widths-change="chatSidePanelWidths = $event"
      @side-panel-widths-commit="chatSidePanelWidths = $event"
      @update-conversation-list-tab="updateConversationListTab"
      @update-chat-left-panel-mode="updateChatLeftPanelMode"
      @update-chat-right-panel-mode="updateChatRightPanelMode"
      @create-conversation-branch-from-turn="createConversationBranchFromTurn"
      @recall-turn="recallTurn"
      @confirm-plan="confirmPlan"
      @lock-workspace="openWorkspacePicker"
      @open-code-review="openCodeReview"
      @open-supervision-task="openSupervisionTask"
      @approve-terminal-approval="approveTerminalApproval"
      @deny-terminal-approval="denyTerminalApproval"
      @switch-conversation="openConversation($event.conversationId)"
      @create-conversation="handleCreateConversationRequest"
      @selection-action-branch="branchConversationFromSelection"
      @selection-action-delegate="delegateFromSelection"
    />
    <SidebarReviewPanel
      :open="reviewPanelOpen"
      :loading="reviewReportsLoading"
      :submitting="codeReviewSubmitting"
      :deleting="reviewReportDeleting"
      :error-text="reviewErrorText"
      :reports="reviewReports"
      @close="closeReviewPanel"
      @open-code-review="openCodeReview"
      @delete-report="deleteReviewReport"
      @retry-report="retryReviewReport"
    />
    <SidebarCompactionDialog
      :open="compactionDialogOpen"
      :loading="compactionPreviewLoading"
      :running="compacting"
      :preview="compactionPreview"
      :error-text="compactionErrorText"
      @close="closeCompactionDialog"
      @confirm="confirmCompaction"
    />
    <CreateConversationDialog
      :open="createConversationDialogOpen"
      :creating="creatingConversation"
      :departments="createConversationDepartmentOptions"
      :default-department-id="defaultCreateConversationDepartmentId"
      :error-text="createConversationErrorText"
      @close="closeCreateConversationDialog"
      @confirm="createConversation"
    />
    <dialog class="modal" :class="{ 'modal-open': remoteAuthDialogOpen }">
      <div class="modal-box max-w-sm">
        <h3 class="font-semibold text-base">{{ t("sidebar.remoteAuthTitle") }}</h3>
        <div class="mt-2 text-sm opacity-75">{{ t("sidebar.remoteAuthHint") }}</div>
        <form class="mt-4 flex flex-col gap-3" @submit.prevent="submitRemoteAuth">
          <input
            v-model.trim="remoteAuthPassword"
            class="input input-bordered input-sm w-full"
            type="password"
            autocomplete="current-password"
            :placeholder="t('sidebar.remoteAuthPlaceholder')"
            :disabled="remoteAuthSubmitting"
          />
          <div v-if="remoteAuthError" class="text-xs text-error">{{ remoteAuthError }}</div>
          <button class="btn btn-sm btn-primary w-full" type="submit" :disabled="remoteAuthSubmitting || !remoteAuthPassword">
            {{ remoteAuthSubmitting ? t("sidebar.remoteAuthSubmitting") : t("sidebar.remoteAuthSubmit") }}
          </button>
        </form>
      </div>
    </dialog>
    <ToolReviewTargetDialog
      :open="codeReviewDialogOpen"
      :submitting="codeReviewSubmitting"
      :error-text="codeReviewErrorText"
      :current-department-id="activeDepartmentId"
      :department-options="createConversationDepartmentOptions"
      :commit-options="commitOptions"
      :commit-options-loading="commitOptionsLoading"
      :commit-total="commitTotal"
      :commit-page="commitPage"
      :commit-page-size="commitPageSize"
      @close="closeCodeReviewDialog"
      @pick-commit-review="loadCodeReviewCommitOptions"
      @review-code="submitCodeReview"
    />
    <ChatSupervisionTaskDialog
      :open="supervisionDialogOpen"
      :saving="supervisionSaving"
      :error-text="supervisionErrorText"
      :active-task="sidebarActiveSupervisionTask"
      :recent-history="[]"
      @close="closeSupervisionTask"
      @save="saveSupervisionTask"
      @stop="stopSupervisionTask"
    />
    <dialog class="modal" :class="{ 'modal-open': rewindConfirmDialogOpen }">
      <div class="modal-box max-w-md">
        <h3 class="font-semibold text-base">{{ t("dialogs.rewind.title") }}</h3>
        <div class="mt-2 text-sm opacity-80">{{ t("dialogs.rewind.hint") }}</div>
        <div class="mt-4 flex flex-col items-center gap-2">
          <button
            v-if="rewindConfirmCanUndoPatch"
            class="btn btn-sm btn-error w-full"
            @click="confirmRewindWithPatch"
          >
            {{ t("dialogs.rewind.withPatch") }}
          </button>
          <button class="btn btn-sm w-full" @click="confirmRewindMessageOnly">
            {{ t("dialogs.rewind.messageOnly") }}
          </button>
          <button class="btn btn-sm btn-primary w-full" @click="cancelRewindConfirm">
            {{ t("common.cancel") }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click.prevent="cancelRewindConfirm">close</button>
      </form>
    </dialog>
    <dialog class="modal" :class="{ 'modal-open': branchFromMessageConfirmDialogOpen }">
      <div class="modal-box max-w-md">
        <h3 class="font-semibold text-base">{{ t("dialogs.branchFromMessage.title") }}</h3>
        <div class="mt-2 text-sm opacity-80">{{ t("dialogs.branchFromMessage.hint") }}</div>
        <div class="modal-action">
          <button class="btn btn-sm" @click="cancelBranchFromMessageConfirm">{{ t("common.cancel") }}</button>
          <button class="btn btn-sm btn-primary" @click="confirmBranchFromMessage">{{ t("dialogs.branchFromMessage.confirm") }}</button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click.prevent="cancelBranchFromMessageConfirm">close</button>
      </form>
    </dialog>
    <div
      v-if="workspacePickerOpen"
      class="fixed inset-0 z-80 flex items-center justify-center bg-black/30 px-4 py-8"
      @click.self="closeWorkspacePicker"
    >
      <div class="w-full max-w-lg rounded-2xl border border-base-300 bg-base-100 shadow-2xl">
        <div class="border-b border-base-300 px-4 py-3">
          <div class="text-sm font-semibold">{{ t("chat.workspacePickerTitle") }}</div>
          <div class="mt-1 text-xs opacity-70">Web 端请手动输入可由服务端访问的工作目录路径。</div>
        </div>
        <div class="space-y-4 px-4 py-4">
          <label class="form-control w-full">
            <div class="label">
              <span class="label-text text-xs">工作目录路径</span>
            </div>
            <div class="join w-full">
              <input
                v-model.trim="workspaceManualPath"
                class="input input-bordered input-sm join-item min-w-0 flex-1 font-mono"
                type="text"
                :disabled="workspacePickerSaving"
                placeholder="例如 E:\\github\\easy_call_ai 或 /home/me/project"
                @keydown.enter.prevent="loadWorkspaceDirectory(workspaceManualPath)"
              />
              <button
                type="button"
                class="btn btn-sm join-item"
                :disabled="workspacePickerSaving || workspaceDirectoryLoading || !workspaceManualPath.trim()"
                @click="loadWorkspaceDirectory(workspaceManualPath)"
              >
                浏览
              </button>
            </div>
          </label>
          <div class="rounded-box border border-base-300 bg-base-200/30">
            <div class="flex items-center gap-2 border-b border-base-300 px-2 py-2">
              <button
                type="button"
                class="btn btn-xs"
                :disabled="workspacePickerSaving || workspaceDirectoryLoading || !workspaceParentPath"
                @click="workspaceParentPath && loadWorkspaceDirectory(workspaceParentPath)"
              >
                上一级
              </button>
              <div class="min-w-0 flex-1 truncate font-mono text-xs" :title="workspaceBrowserPath || workspaceManualPath">
                {{ workspaceBrowserPath || workspaceManualPath || "输入路径后开始浏览" }}
              </div>
              <button
                type="button"
                class="btn btn-xs btn-ghost"
                :disabled="workspacePickerSaving || workspaceDirectoryLoading || !workspaceBrowserPath"
                @click="loadWorkspaceDirectory(workspaceBrowserPath)"
              >
                刷新
              </button>
            </div>
            <div class="max-h-64 overflow-y-auto py-1">
              <div v-if="workspaceDirectoryLoading" class="flex items-center gap-2 px-3 py-3 text-sm text-base-content/65">
                <span class="loading loading-spinner loading-xs"></span>
                正在读取目录
              </div>
              <div v-else-if="workspaceDirectoryError" class="px-3 py-3 text-sm text-error">
                {{ workspaceDirectoryError }}
              </div>
              <div v-else-if="workspaceDirectoryItems.length === 0" class="px-3 py-3 text-sm text-base-content/55">
                当前目录没有可继续进入的子目录
              </div>
              <template v-else>
                <button
                  v-for="item in workspaceDirectoryItems"
                  :key="item.path"
                  type="button"
                  class="flex min-h-8 w-full items-center gap-2 px-3 py-1.5 text-left text-sm hover:bg-base-300/60"
                  :disabled="workspacePickerSaving"
                  :title="item.path"
                  @click="loadWorkspaceDirectory(item.path)"
                >
                  <span class="shrink-0 text-base-content/55">▸</span>
                  <span class="min-w-0 flex-1 truncate">{{ item.name }}</span>
                </button>
              </template>
            </div>
          </div>
          <div class="grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
            <label class="form-control w-full">
              <div class="label">
                <span class="label-text text-xs">访问权限</span>
              </div>
              <select
                v-model="workspaceManualAccess"
                class="select select-bordered select-sm w-full"
                :disabled="workspacePickerSaving"
              >
                <option value="approval">{{ t("config.tools.workspaceAccessApproval") }}</option>
                <option value="full_access">{{ t("config.tools.workspaceAccessFullAccess") }}</option>
                <option value="read_only">{{ t("config.tools.workspaceAccessReadOnly") }}</option>
              </select>
            </label>
            <label
              class="flex cursor-pointer items-center gap-2 rounded-box bg-base-200 px-3 py-2 text-xs"
              :title="t('chat.workspacePickerAutonomousHint')"
            >
              <span>{{ t("chat.workspacePickerAutonomous") }}</span>
              <input
                v-model="workspaceDraftAutonomousMode"
                type="checkbox"
                class="checkbox checkbox-primary checkbox-sm"
                :disabled="workspacePickerSaving"
              />
            </label>
          </div>
        </div>
        <div class="flex items-center justify-end gap-2 border-t border-base-300 px-4 py-3">
          <button class="btn btn-sm btn-ghost" type="button" :disabled="workspacePickerSaving" @click="closeWorkspacePicker">
            {{ t("common.cancel") }}
          </button>
          <button
            class="btn btn-sm btn-primary"
            type="button"
            :disabled="workspacePickerSaving || !workspaceManualPath.trim()"
            @click="saveWorkspacePicker"
          >
            {{ workspacePickerSaving ? t("common.saving") : "使用此目录" }}
          </button>
        </div>
      </div>
    </div>
    <input
      ref="attachmentInputRef"
      class="hidden"
      type="file"
      accept="image/*,application/pdf"
      multiple
      @change="handleAttachmentInputChange"
    />
  </SidebarLayout>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import type { ApiConfigItem, ChatConversationOverviewItem, ChatMessage, ChatTodoItem, ConversationGoalState, IdeContextWorkspaceGroup, ShellWorkspace } from "../../types/app";
import { removeBinaryPlaceholders, messageText } from "../../utils/chat-message";
import {
  applyAssistantToolEventToStreamBlocks,
  assistantTextFromStreamBlocks,
  normalizeAssistantStreamBlocks,
} from "../../utils/chat-message-semantics";
import { formatConversationFallbackTitle } from "../chat/utils/conversation-title";
import { useI18n } from "vue-i18n";
import SidebarLayout from "./layouts/SidebarLayout.vue";
import ChatViewWrapper from "./views/ChatViewWrapper.vue";
import SidebarCompactionDialog from "./views/SidebarCompactionDialog.vue";
import SidebarReviewPanel from "./views/SidebarReviewPanel.vue";
import CreateConversationDialog, { type SidebarCreateDepartmentOption } from "./views/CreateConversationDialog.vue";
import { useWsTransport, type SidebarBridgeConfig } from "./composables/use-ws-transport";
import ToolReviewTargetDialog from "../chat/components/ToolReviewTargetDialog.vue";
import ChatSupervisionTaskDialog from "../chat/components/dialogs/ChatSupervisionTaskDialog.vue";
import type { ChatWorkspaceChoice } from "../chat/composables/use-chat-workspace";
import type { ToolReviewCodeReviewScope, ToolReviewCommitOption, ToolReviewReportRecord } from "../chat/composables/use-chat-tool-review";
import type { TerminalApprovalConversationItem, TerminalApprovalRequestPayload } from "../shell/composables/use-terminal-approval";
import { readLastActiveConversationId, writeLastActiveConversationId } from "../chat/utils/last-active-conversation";

type ConversationSummary = {
  conversationId: string;
  title: string;
  summaryTitle?: string;
  updatedAt: string;
  lastMessageAt?: string;
  messageCount?: number;
  bodyMessageCount?: number;
  bodyTextLength?: number;
  unreadCount?: number;
  agentId?: string;
  departmentId?: string;
  departmentName?: string;
  runtimeState?: string;
  planModeEnabled?: boolean;
  detachedWindowOpen?: boolean;
  detachedWindowLabel?: string;
  isSystemNotificationConversation?: boolean;
  isMainConversation?: boolean;
  isActive?: boolean;
  isPinned?: boolean;
  pinIndex?: number;
  workspaceLabel?: string;
  workspaceRootPath?: string;
  currentTodo?: string;
  activeGoal?: ConversationGoalState | null;
  currentTodos?: ChatTodoItem[];
  state?: ChatConversationOverviewItem["state"];
  previewMessages?: Array<{
    messageId: string;
    role: ChatMessage["role"];
    speakerAgentId?: string;
    createdAt?: string;
    textPreview?: string;
    hasImage?: boolean;
    hasPdf?: boolean;
    hasAudio?: boolean;
    hasAttachment?: boolean;
  }>;
};

type RemoteImContactConversationSummary = {
  contactId: string;
  conversationId: string;
  title: string;
  updatedAt: string;
  lastMessageAt?: string;
  messageCount: number;
  channelId: string;
  channelName?: string;
  contactDisplayName: string;
  boundDepartmentId?: string;
  boundAgentId?: string;
  processingMode?: string;
  previewMessages?: ConversationSummary["previewMessages"];
};

type OpenConversationResult = {
  conversationId: string;
  title: string;
  agentId?: string;
  departmentId?: string;
  messages: ChatMessage[];
  runtime?: SidebarConversationRuntimePayload | null;
  persona?: SidebarPersonaPayload;
  model?: SidebarModelPayload;
  currentTodos?: ChatTodoItem[];
  activeGoal?: ConversationGoalState | null;
};

type SidebarWorkspacePermission = {
  access?: "read_only" | "approval" | "full_access" | "";
  workspaceName?: string;
  rootPath?: string;
};

type SidebarClipboardImage = {
  mime: string;
  bytesBase64: string;
};

type RewindConversationResult = {
  conversationId: string;
  removedCount: number;
  remainingCount: number;
  recalledUserMessage?: ChatMessage;
  conversation?: OpenConversationResult;
};

type RewindConversationPreviewResult = {
  conversationId: string;
  canUndoPatch: boolean;
  hint?: string | null;
};

type BlockPageResult = {
  selectedBlockId: number;
  messages: ChatMessage[];
  hasPrevBlock: boolean;
  hasNextBlock: boolean;
};

type CompactionPreviewResult = {
  conversationId: string;
  canCompact: boolean;
  messageCount: number;
  hasAssistantReply: boolean;
  isEmpty: boolean;
  contextUsagePercent: number;
  compactionDisabledReason?: string | null;
};

type SidebarPersonaPayload = {
  userAlias?: string;
  userAvatarUrl?: string;
  assistantName?: string;
  assistantAvatarUrl?: string;
  personaNameMap?: Record<string, string>;
  personaAvatarUrlMap?: Record<string, string>;
};

type SidebarModelPayload = {
  conversationCallPrimaryApiConfigId?: string;
  preferredChatModelId?: string;
  chatModelOptions?: ApiConfigItem[];
};

type SidebarStreamCachePayload = {
  assistantText?: string;
  toolStatusText?: string;
  toolStatusState?: string;
  streamBlocks?: unknown[];
};

type SidebarConversationRuntimePayload = {
  runtimeState?: string;
  streamCache?: SidebarStreamCachePayload;
};

type GoalMutationOutput = {
  conversationId: string;
  goal: ConversationGoalState;
};

type SidebarAssistantDeltaPayload = {
  conversationId?: string;
  event?: {
    delta?: string;
    kind?: string;
    toolName?: string;
    toolCallId?: string;
    toolStatus?: string;
    toolArgs?: string;
    message?: string;
    streamCache?: SidebarStreamCachePayload;
  };
};

type CreateConversationOptionsResult = {
  departments: SidebarCreateDepartmentOption[];
  defaultDepartmentId: string;
  defaultAgentId?: string;
};

type DiscoveryPayload = {
  chatUrl?: string;
  bridgeUrl?: string;
  url?: string;
  token?: string;
  workspaceRoots?: Array<{ path?: string; name?: string }>;
};

type IdeContextQueryResult = {
  groups?: IdeContextWorkspaceGroup[];
  updatedAt?: string;
};

const SYSTEM_NOTIFICATION_CONVERSATION_ID = "system-notification-conversation";
const SYSTEM_NOTIFICATION_DISPLAY_TITLE = "P-ai系统";
const CHAT_CONVERSATION_LIST_TAB_STORAGE_KEY = "easy_call.chat_conversation_list_tab.v1";
const CHAT_LEFT_PANEL_MODE_STORAGE_KEY = "easy_call.chat_left_panel_mode.v1";
const LEGACY_CHAT_LEFT_PANEL_MODE_STORAGE_KEY = "easy-call.chat.left-panel-mode";

type SidebarConversationTab = "local" | "contact" | "task";

const transport = useWsTransport();
const { t } = useI18n();
const conversations = ref<ConversationSummary[]>([]);
const remoteImContactConversations = ref<RemoteImContactConversationSummary[]>([]);
const sidebarViewerId = ref("");
const activeConversationId = ref("");
const activeTitle = computed(() => {
  const item = activeSummary.value;
  if (!item) {
    const remoteItem = remoteImContactConversations.value.find((entry) =>
      String(entry.conversationId || "").trim() === String(activeConversationId.value || "").trim()
    );
    return String(remoteItem?.title || "").trim()
      || String(remoteItem?.contactDisplayName || "").trim()
      || "PAI";
  }
  const title = String(item.title || "").trim();
  if (title) return title;
  const summary = String(item.summaryTitle || "").trim();
  if (summary) return summary;
  return formatConversationFallbackTitle(item.lastMessageAt || item.updatedAt) || "PAI";
});
const activeAgentId = ref("");
const persona = ref<SidebarPersonaPayload>({});
const conversationCallPrimaryApiConfigId = ref("");
const preferredChatModelId = ref("");
const chatModelOptions = ref<ApiConfigItem[]>([]);
const workspaceAccess = ref<"read_only" | "approval" | "full_access" | "">("approval");
const workspaceRootPath = ref("");
const workspaceRootName = ref("");
const vscodeWorkspaceRoots = ref<Array<{ path: string; name: string }>>([]);
const vscodeIdeContextGroups = ref<IdeContextWorkspaceGroup[]>([]);
const messages = ref<ChatMessage[]>([]);
const sidebarTodos = ref<ChatTodoItem[]>([]);
const inputText = ref("");
const clipboardImages = ref<SidebarClipboardImage[]>([]);
const streamingText = ref("");
const toolStatusText = ref("");
const toolStatusState = ref<"running" | "done" | "failed" | "">("");
const streamBlocks = ref<ReturnType<typeof normalizeAssistantStreamBlocks>>([]);
const busy = ref(false);
const compacting = ref(false);
const chatViewWrapperRef = ref<{ exitMessageSelectionMode: () => void; chatUsagePercent?: number } | null>(null);
const chatUsagePercent = computed(() => chatViewWrapperRef.value?.chatUsagePercent ?? 0);
const compactionDialogOpen = ref(false);
const compactionPreviewLoading = ref(false);
const compactionPreview = ref<CompactionPreviewResult | null>(null);
const compactionErrorText = ref("");
const createConversationDialogOpen = ref(false);
const creatingConversation = ref(false);
const branchingConversation = ref(false);
const createConversationDepartmentOptions = ref<SidebarCreateDepartmentOption[]>([]);
const defaultCreateConversationDepartmentId = ref("");
const createConversationErrorText = ref("");
const createConversationOptionsStale = ref(true);
const remoteAuthDialogOpen = ref(false);
const remoteAuthPassword = ref("");
const remoteAuthSubmitting = ref(false);
const remoteAuthError = ref("");
const codeReviewDialogOpen = ref(false);
const codeReviewSubmitting = ref(false);
const codeReviewErrorText = ref("");
const reviewPanelOpen = ref(false);
const reviewReports = ref<ToolReviewReportRecord[]>([]);
const reviewReportsLoading = ref(false);
const reviewReportDeleting = ref(false);
const reviewErrorText = ref("");
const commitOptions = ref<ToolReviewCommitOption[]>([]);
const commitOptionsLoading = ref(false);
const commitTotal = ref(0);
const commitPage = ref(1);
const commitPageSize = ref(30);
let unlistenCodeReviewFn: (() => void) | null = null;
const supervisionDialogOpen = ref(false);
const supervisionSaving = ref(false);
const supervisionErrorText = ref("");
const activeConversationGoal = ref<ConversationGoalState | null>(null);
const selectedBlockId = ref<number | null>(null);
const hasPrevBlock = ref(false);
const view = ref<"list" | "chat">("chat");
const rewindConfirmDialogOpen = ref(false);
const rewindConfirmCanUndoPatch = ref(false);
let rewindConfirmResolver: ((mode: "message_only" | "with_patch" | "cancel") => void) | null = null;
const branchFromMessageConfirmDialogOpen = ref(false);
let branchFromMessageConfirmResolver: ((confirmed: boolean) => void) | null = null;
let rewindInFlight = false;
const currentWorkspaceName = ref("");
const attachmentInputRef = ref<HTMLInputElement | null>(null);
const workspacePickerOpen = ref(false);
const workspacePickerSaving = ref(false);
const workspaceDraftChoices = ref<ChatWorkspaceChoice[]>([]);
const workspaceDraftAutonomousMode = ref(false);
const workspaceManualPath = ref("");
const workspaceManualAccess = ref<ChatWorkspaceChoice["access"]>("approval");
const workspaceBrowserPath = ref("");
const workspaceDirectoryItems = ref<Array<{ path: string; name: string }>>([]);
const workspaceDirectoryLoading = ref(false);
const workspaceDirectoryError = ref("");
const workspaceParentPath = computed(() => parentWorkspaceDirectoryPath(workspaceBrowserPath.value || workspaceManualPath.value));
const terminalApprovalQueue = ref<TerminalApprovalRequestPayload[]>([]);
const terminalApprovalResolving = ref(false);
const hideWorkspaceButton = computed(() => false);
const sideConversationListVisible = ref(false);
const toolReviewPanelOpenVisible = ref(false);
const conversationListTab = ref<SidebarConversationTab>(loadStoredConversationListTab());
const chatLeftPanelMode = ref<SidebarConversationTab>(loadStoredChatLeftPanelMode());
const chatRightPanelMode = ref<"reader" | "review" | "delegate">("review");
const chatSidePanelWidths = ref({ leftWidth: 320, rightWidth: 320 });
let discoveryRefreshTimer: number | null = null;

const activeSummary = computed(() => conversations.value.find((item) => item.conversationId === activeConversationId.value));
const sidebarSupervisionActive = computed(() => String(activeConversationGoal.value?.status || "").trim() === "active");
const sidebarSupervisionTitle = computed(() =>
  sidebarSupervisionActive.value
    ? t("chat.supervision.activeHintShort", { goal: String(activeConversationGoal.value?.objective || "").trim() })
    : t("chat.supervision.buttonHint"),
);
const sidebarActiveSupervisionTask = computed(() => {
  const goal = activeConversationGoal.value;
  if (String(goal?.status || "").trim() !== "active") return null;
  return {
    taskId: String(goal?.goalId || "").trim(),
    goal: String(goal?.objective || "").trim(),
    why: "",
    todo: "",
    endAtLocal: String(goal?.startedAt || "").trim(),
    remainingHours: 0,
  };
});
const sidebarUserAlias = computed(() => String(persona.value?.userAlias || "我").trim() || "我");
const sidebarUserAvatarUrl = computed(() => String(persona.value?.userAvatarUrl || "").trim());
const sidebarAssistantName = computed(() => String(persona.value?.assistantName || "PAI").trim() || "PAI");
const sidebarAssistantAvatarUrl = computed(() => String(persona.value?.assistantAvatarUrl || "").trim());
const sidebarPersonaNameMap = computed<Record<string, string>>(() => ({
  "user-persona": sidebarUserAlias.value,
  ...(persona.value?.personaNameMap || {}),
  ...(activeAgentId.value ? { [activeAgentId.value]: sidebarAssistantName.value } : {}),
}));
const sidebarPersonaAvatarUrlMap = computed<Record<string, string>>(() => {
  const next = { ...(persona.value?.personaAvatarUrlMap || {}) };
  if (activeAgentId.value && sidebarAssistantAvatarUrl.value) next[activeAgentId.value] = sidebarAssistantAvatarUrl.value;
  return next;
});
const visibleConversations = computed(() => conversations.value);
const chatUnarchivedConversationItems = computed<ChatConversationOverviewItem[]>(() =>
  conversations.value
    .map((item) => {
      const conversationId = String(item.conversationId || "").trim();
      const isSystemNotificationConversation = isSidebarSystemConversation(item);
      return {
        conversationId,
        title: String(item.title || "").trim(),
        summaryTitle: item.summaryTitle,
        kind: "local_unarchived" as const,
        messageCount: Number(item.messageCount || 0),
        bodyMessageCount: item.bodyMessageCount,
        bodyTextLength: item.bodyTextLength,
        unreadCount: item.unreadCount,
        agentId: item.agentId,
        departmentId: item.departmentId,
        departmentName: item.departmentName,
        updatedAt: item.updatedAt,
        lastMessageAt: item.lastMessageAt,
        workspaceLabel: item.workspaceLabel,
        workspaceRootPath: item.workspaceRootPath,
        isSystemNotificationConversation,
        isMainConversation: !!item.isMainConversation || isSystemNotificationConversation,
        isPinned: !!item.isPinned || isSystemNotificationConversation,
        pinIndex: item.pinIndex,
        runtimeState: normalizeConversationRuntimeState(item.runtimeState),
        currentTodo: item.currentTodo,
        activeGoal: item.activeGoal || null,
        currentTodos: item.currentTodos,
        planModeEnabled: !!item.planModeEnabled,
        detachedWindowOpen: sidebarConversationUnavailableForCurrentViewer(item),
        detachedWindowLabel: item.detachedWindowLabel,
        previewMessages: Array.isArray(item.previewMessages) ? item.previewMessages : [],
        state: item.state
          ? {
            ...item.state,
            currentViewerId: sidebarViewerId.value || item.state.currentViewerId,
          }
          : item.state,
      };
    })
    .filter((item) => !!item.conversationId),
);
const chatRemoteImConversationItems = computed<ChatConversationOverviewItem[]>(() =>
  remoteImContactConversations.value
    .map((item) => ({
      conversationId: String(item.conversationId || "").trim(),
      title: String(item.title || "").trim() || String(item.contactDisplayName || "").trim(),
      kind: "remote_im_contact" as const,
      remoteContactId: String(item.contactId || "").trim(),
      remoteContactDisplayName: String(item.contactDisplayName || "").trim(),
      channelId: String(item.channelId || "").trim() || undefined,
      channelName: String(item.channelName || "").trim() || undefined,
      messageCount: Number(item.messageCount || 0),
      departmentId: String(item.boundDepartmentId || "").trim() || undefined,
      departmentName: [
        String(item.channelName || "").trim(),
        resolveRemoteConversationDepartmentName(item.boundDepartmentId),
      ].filter(Boolean).join(" · "),
      updatedAt: item.lastMessageAt || item.updatedAt || "",
      lastMessageAt: item.lastMessageAt || item.updatedAt || "",
      previewMessages: Array.isArray(item.previewMessages) ? item.previewMessages : [],
    }))
    .filter((item) => !!item.conversationId),
);
const chatConversationItems = computed<ChatConversationOverviewItem[]>(() => ([
  ...chatUnarchivedConversationItems.value,
  ...chatRemoteImConversationItems.value,
]));
const activeConversationRuntimeState = computed(() => String(activeSummary.value?.runtimeState || "").trim());
const activeConversationPlanModeEnabled = computed(() => !!activeSummary.value?.planModeEnabled);
const activeConversationSystemNotificationMode = computed(() => {
  const item = activeSummary.value;
  return item ? isSidebarSystemConversation(item) : activeConversationId.value === SYSTEM_NOTIFICATION_CONVERSATION_ID;
});
const activeDepartmentId = computed(() => String(activeSummary.value?.departmentId || "").trim());
const activeConversationTerminalApprovals = computed<TerminalApprovalConversationItem[]>(() =>
  listConversationTerminalApprovals(activeConversationId.value),
);
const sidebarShellWorkspaces = computed<ShellWorkspace[]>(() => {
  const draftItems = workspaceDraftChoices.value
    .map((item): ShellWorkspace | null => {
      const path = String(item.path || "").trim();
      if (!path) return null;
      return {
        id: String(item.id || "").trim() || path,
        name: String(item.name || "").trim() || path,
        path,
        level: item.level,
        access: item.access || "approval",
      };
    })
    .filter((item): item is ShellWorkspace => !!item);
  if (draftItems.length > 0) return draftItems;
  return vscodeWorkspaceRoots.value
    .map((item, index): ShellWorkspace | null => {
      const path = String(item.path || "").trim();
      if (!path) return null;
      return {
        id: `vscode-workspace-${index}`,
        name: String(item.name || "").trim() || path,
        path,
        level: index === 0 ? "main" : "secondary",
        access: workspaceAccess.value || "approval",
      };
    })
    .filter((item): item is ShellWorkspace => !!item);
});

watch(
  () => ({
    activeConversationId: activeConversationId.value,
    title: activeSummary.value?.title,
    isSystemNotificationConversation: activeSummary.value?.isSystemNotificationConversation,
    isMainConversation: activeSummary.value?.isMainConversation,
    resolvedSystemMode: activeConversationSystemNotificationMode.value,
    overviewItem: chatConversationItems.value.find((item) => item.conversationId === activeConversationId.value),
  }),
  (snapshot) => {
    // 系统会话识别日志已移除
  },
  { immediate: true, deep: true },
);

watch(
  () => ({
    activeConversationId: activeConversationId.value,
    view: view.value,
    conversationIds: conversations.value.map((item) => String(item.conversationId || "").trim()).join("|"),
  }),
  ({ activeConversationId: conversationId, view }) => {
    const cid = String(conversationId || "").trim();
    if (!cid || view !== "chat") return;
    if (!conversations.value.some((item) => String(item.conversationId || "").trim() === cid)) return;
    writeLastActiveConversationId(cid);
  },
  { immediate: true },
);

function normalizeSidebarConversationTab(value: string): SidebarConversationTab {
  if (value === "contact" || value === "task") return value;
  return "local";
}

function loadStoredConversationListTab(): SidebarConversationTab {
  if (typeof window === "undefined") return "local";
  const stored = String(window.localStorage.getItem(CHAT_CONVERSATION_LIST_TAB_STORAGE_KEY) || "").trim();
  return normalizeSidebarConversationTab(stored);
}

function loadStoredChatLeftPanelMode(): SidebarConversationTab {
  if (typeof window === "undefined") return loadStoredConversationListTab();
  const stored = String(
    window.localStorage.getItem(CHAT_LEFT_PANEL_MODE_STORAGE_KEY)
    || window.localStorage.getItem(LEGACY_CHAT_LEFT_PANEL_MODE_STORAGE_KEY)
    || "",
  ).trim();
  return stored ? normalizeSidebarConversationTab(stored) : loadStoredConversationListTab();
}

function updateConversationListTab(value: SidebarConversationTab) {
  const next = normalizeSidebarConversationTab(value);
  conversationListTab.value = next;
  chatLeftPanelMode.value = next;
  if (typeof window !== "undefined") {
    window.localStorage.setItem(CHAT_CONVERSATION_LIST_TAB_STORAGE_KEY, next);
    window.localStorage.setItem(CHAT_LEFT_PANEL_MODE_STORAGE_KEY, next);
  }
}

function updateChatLeftPanelMode(value: SidebarConversationTab) {
  updateConversationListTab(value);
}

function updateChatRightPanelMode(value: "reader" | "review" | "delegate") {
  chatRightPanelMode.value = value;
  view.value = "chat";
  toolReviewPanelOpenVisible.value = true;
}

function normalizeDiscovery(payload: DiscoveryPayload): SidebarBridgeConfig | null {
  const chatUrl = String(payload.chatUrl || "").trim() || String(payload.url || "").trim().replace(/\/ide-context$/, "/chat");
  const token = String(payload.token || "").trim();
  if (!chatUrl) return null;
  return token ? { chatUrl, token } : { chatUrl };
}

async function loadDiscovery(): Promise<SidebarBridgeConfig | null> {
  const injected = (window as unknown as { __PAI_SIDEBAR_BRIDGE__?: DiscoveryPayload }).__PAI_SIDEBAR_BRIDGE__;
  if (injected) {
    applyWorkspaceRoots(injected.workspaceRoots);
    return normalizeDiscovery(injected);
  }
  const params = new URLSearchParams(window.location.search);
  const fromQuery = normalizeDiscovery({
    chatUrl: params.get("chatUrl") || undefined,
    token: params.get("token") || undefined,
  });
  if (fromQuery) return fromQuery;
  return null;
}

function applyWorkspaceRoots(rawRoots: DiscoveryPayload["workspaceRoots"]) {
  vscodeWorkspaceRoots.value = (Array.isArray(rawRoots) ? rawRoots : [])
    .map((item) => ({
      path: String(item?.path || "").trim(),
      name: String(item?.name || "").trim(),
    }))
    .filter((item) => item.path);
}

function currentIdeContextWorkspaces() {
  return vscodeWorkspaceRoots.value
    .map((item) => ({
      path: String(item.path || "").trim(),
      name: String(item.name || "").trim() || undefined,
    }))
    .filter((item) => item.path);
}

async function refreshIdeContextGroups() {
  if (!transport.connected.value) return;
  const workspaces = currentIdeContextWorkspaces();
  if (workspaces.length === 0) {
    vscodeIdeContextGroups.value = [];
    return;
  }
  try {
    const result = await transport.request<IdeContextQueryResult>("ideContext.query", { workspaces }, 8000);
    applyIdeContextGroups(result.groups || []);
  } catch {
    // IDE 上下文是辅助信息，查询失败时不打断聊天主流程。
  }
}

function applyIdeContextGroups(rawGroups: IdeContextWorkspaceGroup[] | undefined) {
  vscodeIdeContextGroups.value = (Array.isArray(rawGroups) ? rawGroups : [])
    .map((group) => ({
      workspacePath: String(group?.workspacePath || "").trim(),
      workspaceName: String(group?.workspaceName || "").trim(),
      references: (Array.isArray(group?.references) ? group.references : [])
        .map((item) => ({
          ...item,
          id: String(item?.id || "").trim(),
          workspacePath: String(item?.workspacePath || group?.workspacePath || "").trim(),
          workspaceName: String(item?.workspaceName || group?.workspaceName || "").trim(),
          filePath: String(item?.filePath || "").trim(),
          fileName: String(item?.fileName || "").trim(),
          relativePath: String(item?.relativePath || "").trim(),
          displayLabel: String(item?.displayLabel || "").trim(),
          content: String(item?.content || ""),
          source: String(item?.source || "").trim(),
          capturedAt: String(item?.capturedAt || "").trim(),
          textBlock: String(item?.textBlock || "").trim(),
        }))
        .filter((item) => item.id && item.filePath && item.textBlock)
        .reduce((items, item) => {
          const fileKey = String(item.filePath || "").replace(/\\/g, "/").toLowerCase();
          const existingIndex = items.findIndex((existing) =>
            String(existing.filePath || "").replace(/\\/g, "/").toLowerCase() === fileKey,
          );
          if (existingIndex < 0) return [...items, item];
          const existing = items[existingIndex];
          const itemIsSelection = String(item.source || "").trim() === "selection";
          const existingIsSelection = String(existing.source || "").trim() === "selection";
          if (!itemIsSelection && existingIsSelection) return items;
          if (itemIsSelection && !existingIsSelection) {
            const next = [...items];
            next[existingIndex] = item;
            return next;
          }
          const itemLineCount = Math.max(1, Number(item.endLine || item.startLine || 0) - Number(item.startLine || 0) + 1);
          const existingLineCount = Math.max(1, Number(existing.endLine || existing.startLine || 0) - Number(existing.startLine || 0) + 1);
          if (itemLineCount >= existingLineCount) return items;
          const next = [...items];
          next[existingIndex] = item;
          return next;
        }, [] as IdeContextWorkspaceGroup["references"]),
    }))
    .filter((group) => group.references.length > 0);
}

async function refreshList() {
  const result = await transport.request<{
    conversations?: ConversationSummary[];
    unarchivedConversations?: ConversationSummary[];
    remoteImContactConversations?: RemoteImContactConversationSummary[];
    persona?: SidebarPersonaPayload;
    viewerId?: string;
  }>("conversation.list");
  const localConversations = Array.isArray(result.unarchivedConversations)
    ? result.unarchivedConversations
    : Array.isArray(result.conversations)
      ? result.conversations
      : [];
  conversations.value = localConversations;
  remoteImContactConversations.value = Array.isArray(result.remoteImContactConversations)
    ? result.remoteImContactConversations
    : [];
  const activeConversation = localConversations.find((item) =>
    String(item.conversationId || "").trim() === String(activeConversationId.value || "").trim()
  );
  if (activeConversation) {
    const goal = activeConversation.activeGoal || null;
    activeConversationGoal.value = String(activeConversation.activeGoal?.status || "").trim() === "active"
      ? goal
      : null;
  }
  syncConversationTabForRemoteContacts();
  sidebarViewerId.value = String(result.viewerId || sidebarViewerId.value || "").trim();
  if (result.persona && !activeConversationId.value) persona.value = result.persona;
  // Sidebar 会话列表日志已移除
}

async function loadCreateConversationOptions() {
  const result = await transport.request<CreateConversationOptionsResult>("conversation.createOptions", {});
  createConversationDepartmentOptions.value = Array.isArray(result.departments) ? result.departments : [];
  defaultCreateConversationDepartmentId.value = String(result.defaultDepartmentId || "").trim()
    || createConversationDepartmentOptions.value[0]?.departmentId
    || "";
  createConversationOptionsStale.value = false;
}

async function refreshCreateConversationOptionsIfNeeded(force = false) {
  if (!force && !createConversationOptionsStale.value && createConversationDepartmentOptions.value.length > 0) return;
  await loadCreateConversationOptions();
}

function markCreateConversationOptionsStale() {
  createConversationOptionsStale.value = true;
  if (!createConversationDialogOpen.value) return;
  void refreshCreateConversationOptionsIfNeeded(true).catch((error) => {
    createConversationErrorText.value = String(error || t('sidebar.loadDepartmentFailed'));
  });
}

function clearCompletedRuntimeStateForConversation(conversationId: string) {
  const targetId = String(conversationId || "").trim();
  if (!targetId) return;
  conversations.value = conversations.value.map((item) => {
    if (String(item.conversationId || "").trim() !== targetId) return item;
    const state = String(item.runtimeState || "").trim();
    if (state === "done" || state === "failed" || state === "completed") {
      return { ...item, runtimeState: "" };
    }
    return item;
  });
}

function patchConversationRuntimeState(conversationId: string, runtimeState: string) {
  const targetId = String(conversationId || "").trim();
  if (!targetId) return;
  conversations.value = conversations.value.map((item) =>
    String(item.conversationId || "").trim() === targetId
      ? { ...item, runtimeState }
      : item,
  );
}

function patchConversationPlanMode(conversationId: string, planModeEnabled: boolean) {
  const targetId = String(conversationId || "").trim();
  if (!targetId) return;
  conversations.value = conversations.value.map((item) =>
    String(item.conversationId || "").trim() === targetId
      ? { ...item, planModeEnabled }
      : item,
  );
}

function sortConversationSummaries(items: ConversationSummary[]) {
  return [...items].sort((left, right) => {
    if (!!left.isSystemNotificationConversation !== !!right.isSystemNotificationConversation) {
      return Number(!!right.isSystemNotificationConversation) - Number(!!left.isSystemNotificationConversation);
    }
    if (!!left.isPinned !== !!right.isPinned) {
      return Number(!!right.isPinned) - Number(!!left.isPinned);
    }
    if (left.isPinned && right.isPinned) {
      const leftIndex = Number.isFinite(Number(left.pinIndex)) ? Number(left.pinIndex) : Number.MAX_SAFE_INTEGER;
      const rightIndex = Number.isFinite(Number(right.pinIndex)) ? Number(right.pinIndex) : Number.MAX_SAFE_INTEGER;
      return leftIndex - rightIndex || String(left.conversationId || "").localeCompare(String(right.conversationId || ""));
    }
    return conversationActivityTime(right) - conversationActivityTime(left)
      || String(right.conversationId || "").localeCompare(String(left.conversationId || ""));
  });
}

function patchConversationOverviewItem(conversation?: ConversationSummary | null) {
  const conversationId = String(conversation?.conversationId || "").trim();
  if (!conversationId || !conversation) return;
  let replaced = false;
  const nextItems = conversations.value.map((item) => {
    if (String(item.conversationId || "").trim() !== conversationId) return item;
    replaced = true;
    return { ...item, ...conversation };
  });
  if (!replaced) {
    nextItems.push(conversation);
  }
  conversations.value = sortConversationSummaries(nextItems);
}

function normalizeToolStatusState(value: unknown): "running" | "done" | "failed" | "" {
  const state = String(value || "").trim();
  return state === "running" || state === "done" || state === "failed" ? state : "";
}

function isSidebarSystemConversation(item: ConversationSummary): boolean {
  if (!!item.isSystemNotificationConversation || !!item.isMainConversation) return true;
  const conversationId = String(item.conversationId || "").trim();
  if (conversationId === SYSTEM_NOTIFICATION_CONVERSATION_ID) return true;
  return String(item.title || "").trim() === SYSTEM_NOTIFICATION_DISPLAY_TITLE;
}

function resolveRemoteConversationDepartmentName(boundDepartmentId?: string): string {
  const normalizedDepartmentId = String(boundDepartmentId || "").trim();
  if (!normalizedDepartmentId) return "主部门";
  return createConversationDepartmentOptions.value.find((item) =>
    String(item.departmentId || item.id || "").trim() === normalizedDepartmentId
  )?.name || normalizedDepartmentId;
}

function sidebarConversationOpenedOutside(item: ConversationSummary): boolean {
  if (isSidebarSystemConversation(item)) return false;
  const openState = String(item.state?.openState || "").trim();
  const openViewerId = String(item.state?.openViewerId || "").trim();
  const currentViewerId = String(sidebarViewerId.value || item.state?.currentViewerId || "").trim();
  return openState === "open" && !!openViewerId && !!currentViewerId && openViewerId !== currentViewerId;
}

function sidebarConversationUnavailableForCurrentViewer(item: ConversationSummary): boolean {
  if (sidebarConversationOpenedOutside(item)) return true;
  const openViewerId = String(item.state?.openViewerId || "").trim();
  const currentViewerId = String(sidebarViewerId.value || item.state?.currentViewerId || "").trim();
  if (openViewerId && currentViewerId) return false;
  return !!item.detachedWindowOpen;
}

function normalizeSidebarWorkspacePath(path: string): string {
  return String(path || "").trim().replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
}

function sidebarWorkspacePathMatches(conversationPath: string, workspacePath: string): boolean {
  const normalizedConversationPath = normalizeSidebarWorkspacePath(conversationPath);
  const normalizedWorkspacePath = normalizeSidebarWorkspacePath(workspacePath);
  if (!normalizedConversationPath || !normalizedWorkspacePath) return false;
  return normalizedConversationPath === normalizedWorkspacePath
    || normalizedConversationPath.startsWith(`${normalizedWorkspacePath}/`);
}

function conversationMatchesCurrentSidebarWorkspace(item: ConversationSummary): boolean {
  const conversationPath = String(item.workspaceRootPath || "").trim();
  if (!conversationPath) return false;
  return vscodeWorkspaceRoots.value.some((workspace) =>
    !!String(workspace.path || "").trim() && sidebarWorkspacePathMatches(conversationPath, workspace.path)
  );
}

function currentSidebarWorkspaceCandidates(items: ConversationSummary[]): ConversationSummary[] {
  return items.filter((item) =>
    !isSidebarSystemConversation(item) && conversationMatchesCurrentSidebarWorkspace(item)
  );
}

function sidebarHasWorkspaceContext(): boolean {
  return vscodeWorkspaceRoots.value.some((workspace) => !!String(workspace.path || "").trim());
}

function isSidebarConversationOpenable(item: ConversationSummary): boolean {
  const state = String(item.runtimeState || "").trim();
  return state !== "organizing_context"
    && state !== "archiving"
    && state !== "compacting"
    && !sidebarConversationUnavailableForCurrentViewer(item);
}

function conversationActivityTime(item: ConversationSummary): number {
  const timestamp = Date.parse(String(item.lastMessageAt || item.updatedAt || "").trim());
  return Number.isFinite(timestamp) ? timestamp : 0;
}

function latestConversation(items: ConversationSummary[]): ConversationSummary | undefined {
  return [...items].sort((left, right) =>
    conversationActivityTime(right) - conversationActivityTime(left)
    || String(right.conversationId || "").localeCompare(String(left.conversationId || ""))
  )[0];
}

function pickInitialSidebarConversationId(items: ConversationSummary[]): string {
  const candidates = items.filter((item) => !!String(item.conversationId || "").trim());
  const openableCandidates = candidates.filter(isSidebarConversationOpenable);
  const workspaceCandidates = sidebarHasWorkspaceContext()
    ? currentSidebarWorkspaceCandidates(openableCandidates)
    : [];
  const latestWorkspaceConversation = latestConversation(workspaceCandidates);
  if (latestWorkspaceConversation) {
    return String(latestWorkspaceConversation.conversationId || "").trim();
  }
  const storedConversationId = readLastActiveConversationId();
  if (storedConversationId) {
    const stored = openableCandidates.find((item) => String(item.conversationId || "").trim() === storedConversationId);
    if (stored) return storedConversationId;
  }
  const target =
    openableCandidates.find(isSidebarSystemConversation)
    || openableCandidates.find((item) => !!item.isActive)
    || openableCandidates[0]
    || candidates.find(isSidebarSystemConversation)
    || candidates[0];
  return String(target?.conversationId || "").trim();
}

function normalizeConversationRuntimeState(value: unknown): ChatConversationOverviewItem["runtimeState"] {
  const state = String(value || "").trim();
  if (
    state === "idle"
    || state === "assistant_streaming"
    || state === "organizing_context"
    || state === "archiving"
    || state === "compacting"
  ) {
    return state;
  }
  return undefined;
}

function clearStreamingState() {
  streamingText.value = "";
  toolStatusText.value = "";
  toolStatusState.value = "";
  streamBlocks.value = [];
}

function resetActiveConversationTransientState(reason: string) {
  console.info("[Sidebar会话恢复] 重置前端瞬时流式状态", {
    reason,
    activeConversationId: activeConversationId.value,
    busy: busy.value,
    streamingTextLength: String(streamingText.value || "").length,
    streamBlockCount: Array.isArray(streamBlocks.value) ? streamBlocks.value.length : 0,
  });
  busy.value = false;
  clearStreamingState();
}

function applyRuntimeStreamCache(runtime: SidebarConversationRuntimePayload | null | undefined) {
  const cache = runtime?.streamCache;
  if (!cache) return;
  const blocks = normalizeAssistantStreamBlocks(cache.streamBlocks);
  if (blocks.length > 0 || streamBlocks.value.length === 0) {
    streamBlocks.value = blocks;
  }
  streamingText.value = assistantTextFromStreamBlocks(streamBlocks.value) || String(cache.assistantText || "");
  toolStatusText.value = String(cache.toolStatusText || "");
  toolStatusState.value = normalizeToolStatusState(cache.toolStatusState);
}

function applyAssistantToolStatusEvent(event: NonNullable<SidebarAssistantDeltaPayload["event"]>) {
  const toolStatus = String(event.toolStatus || "").trim();
  toolStatusText.value = String(event.message || "");
  toolStatusState.value = normalizeToolStatusState(toolStatus);
}

async function openConversation(conversationId: string) {
  sideConversationListVisible.value = false;
  const beforeSummary = conversations.value.find((item) => String(item.conversationId || "").trim() === String(conversationId || "").trim());
  if (beforeSummary && String(beforeSummary.conversationId || "").trim() !== String(activeConversationId.value || "").trim() && !isSidebarConversationOpenable(beforeSummary)) {
    console.info("[Sidebar会话打开] 跳过被占用会话", {
      conversationId,
      state: beforeSummary.state,
      detachedWindowOpen: beforeSummary.detachedWindowOpen,
      runtimeState: beforeSummary.runtimeState,
    });
    return;
  }
  console.info("[Sidebar会话打开][start]", {
    conversationId,
    activeConversationId: activeConversationId.value,
    summary: beforeSummary,
    isSystemBySummary: beforeSummary ? isSidebarSystemConversation(beforeSummary) : false,
  });
  if (String(activeConversationId.value || "").trim() === String(conversationId || "").trim()) {
    resetActiveConversationTransientState("reload_current_conversation");
  }
  clearCompletedRuntimeStateForConversation(activeConversationId.value);
  const vscodeRoot = vscodeWorkspaceRoots.value[0];
  let result: OpenConversationResult;
  try {
    result = await transport.request<OpenConversationResult>("conversation.open", {
      conversationId,
      workspacePath: vscodeRoot?.path || undefined,
      workspaceName: vscodeRoot?.name || undefined,
    });
  } catch (error) {
    console.warn("[Sidebar会话打开][failed]", {
      conversationId,
      error,
    });
    throw error;
  }
  console.info("[Sidebar会话打开][success]", {
    requestedConversationId: conversationId,
    resultConversationId: result.conversationId,
    title: result.title,
    agentId: result.agentId,
    departmentId: result.departmentId,
  });
  activeConversationId.value = result.conversationId;
  clearCompletedRuntimeStateForConversation(result.conversationId);
  activeAgentId.value = String(result.agentId || "").trim();
  persona.value = result.persona || {};
  applyModelPayload(result.model || {});
  await refreshWorkspacePermission();
  await refreshWorkspaceList();
  messages.value = Array.isArray(result.messages) ? result.messages : [];
  sidebarTodos.value = Array.isArray(result.currentTodos) ? result.currentTodos : [];
  const resultActiveGoal = result.activeGoal || null;
  activeConversationGoal.value = String(resultActiveGoal?.status || "").trim() === "active"
    ? resultActiveGoal
    : null;
  clearStreamingState();
  applyRuntimeStreamCache(result.runtime);
  selectedBlockId.value = null;
  hasPrevBlock.value = true;
  view.value = "chat";
  syncConversationTabForActiveConversation();
  void refreshCreateConversationOptionsIfNeeded();
}

function syncConversationTabForActiveConversation() {
  const activeId = String(activeConversationId.value || "").trim();
  if (!activeId) return;
  const activeItem = chatConversationItems.value.find((item) =>
    String(item.conversationId || "").trim() === activeId
  );
  if (activeItem?.kind === "remote_im_contact") {
    updateConversationListTab("contact");
  }
}

function syncConversationTabForRemoteContacts() {
  if (remoteImContactConversations.value.length === 0) return;
  const activeId = String(activeConversationId.value || "").trim();
  if (activeId && remoteImContactConversations.value.some((item) =>
    String(item.conversationId || "").trim() === activeId
  )) {
    updateConversationListTab("contact");
    return;
  }
  const hasNonSystemLocalConversation = visibleConversations.value.some((item) =>
    !isSidebarSystemConversation(item)
  );
  if (!hasNonSystemLocalConversation && conversationListTab.value === "local") {
    updateConversationListTab("contact");
  }
}

async function refreshWorkspacePermission() {
  if (!activeConversationId.value) return;
  try {
    const result = await transport.request<SidebarWorkspacePermission>("workspace.permission", {
      conversationId: activeConversationId.value,
    });
    applyWorkspacePermission(result);
  } catch {
    workspaceAccess.value = "approval";
  }
}

function applyWorkspacePermission(payload: SidebarWorkspacePermission) {
  const access = String(payload.access || "").trim();
  workspaceAccess.value = access === "read_only" || access === "full_access" ? access : "approval";
  const rootPath = String(payload.rootPath || "").trim();
  if (rootPath) workspaceRootPath.value = rootPath;
  const rootName = String(payload.workspaceName || "").trim();
  if (rootName) workspaceRootName.value = rootName;
}

function normalizeTerminalApprovalConversationId(payload: Pick<TerminalApprovalRequestPayload, "sessionId"> | null | undefined): string {
  const sessionId = String(payload?.sessionId || "").trim();
  if (!sessionId) return "";
  const parts = sessionId.split("::");
  return String(parts[parts.length - 1] || "").trim();
}

function listConversationTerminalApprovals(conversationId: string): TerminalApprovalConversationItem[] {
  const normalizedConversationId = String(conversationId || "").trim();
  if (!normalizedConversationId) return [];
  return terminalApprovalQueue.value
    .filter((item) => normalizeTerminalApprovalConversationId(item) === normalizedConversationId)
    .map((item) => ({ ...item, conversationId: normalizedConversationId }));
}

function enqueueTerminalApprovalRequest(payload: TerminalApprovalRequestPayload) {
  const requestId = String(payload.requestId || "").trim();
  if (!requestId) return;
  if (terminalApprovalQueue.value.some((item) => item.requestId === requestId)) return;
  terminalApprovalQueue.value.push({
    ...payload,
    requestId,
    title: String(payload.title || t('sidebar.terminalApproval')),
    message: String(payload.message || ""),
    approvalKind: String(payload.approvalKind || "unknown"),
    sessionId: String(payload.sessionId || ""),
    toolName: String(payload.toolName || ""),
    summary: String(payload.summary || ""),
    callPreview: String(payload.callPreview || ""),
    cwd: String(payload.cwd || ""),
    command: String(payload.command || ""),
    requestedPath: String(payload.requestedPath || ""),
    reason: String(payload.reason || ""),
    reviewOpinion: String(payload.reviewOpinion || ""),
    reviewModelName: String(payload.reviewModelName || ""),
    existingPaths: Array.isArray(payload.existingPaths)
      ? payload.existingPaths.map((item) => String(item || "").trim()).filter(Boolean)
      : [],
    targetPaths: Array.isArray(payload.targetPaths)
      ? payload.targetPaths.map((item) => String(item || "").trim()).filter(Boolean)
      : [],
  });
}

async function resolveTerminalApproval(approved: boolean, requestId?: string) {
  if (terminalApprovalResolving.value) return;
  const normalizedRequestId = String(requestId || "").trim();
  const targetIndex = terminalApprovalQueue.value.findIndex((item) => item.requestId === normalizedRequestId);
  if (targetIndex < 0) return;
  terminalApprovalResolving.value = true;
  try {
    await transport.request("terminalApproval.resolve", {
      requestId: terminalApprovalQueue.value[targetIndex].requestId,
      approved,
    });
    terminalApprovalQueue.value.splice(targetIndex, 1);
  } catch (error) {
    transport.errorText.value = String(error || t('sidebar.approvalFailed'));
  } finally {
    terminalApprovalResolving.value = false;
  }
}

function approveTerminalApproval(requestId: string) {
  void resolveTerminalApproval(true, requestId);
}

function denyTerminalApproval(requestId: string) {
  void resolveTerminalApproval(false, requestId);
}

async function loadPrevBlock() {
  if (!activeConversationId.value || !hasPrevBlock.value) return;
  const result = await transport.request<BlockPageResult>("conversation.blockPage", {
    conversationId: activeConversationId.value,
    blockId: selectedBlockId.value || undefined,
  });
  selectedBlockId.value = result.selectedBlockId;
  hasPrevBlock.value = result.hasPrevBlock;
  const existingIds = new Set(messages.value.map((item) => item.id));
  const previous = (result.messages || []).filter((item) => !existingIds.has(item.id));
  messages.value = [...previous, ...messages.value];
}

async function openCreateConversationDialog() {
  createConversationErrorText.value = "";
  try {
    await refreshCreateConversationOptionsIfNeeded();
    createConversationDialogOpen.value = true;
  } catch (error) {
    createConversationErrorText.value = String(error || t('sidebar.loadDepartmentFailed'));
    createConversationDialogOpen.value = true;
  }
}

function closeCreateConversationDialog() {
  if (creatingConversation.value) return;
  createConversationDialogOpen.value = false;
  createConversationErrorText.value = "";
}

async function createConversation(input: { title?: string; departmentId: string; agentId: string }) {
  const departmentId = String(input.departmentId || "").trim();
  const agentId = String(input.agentId || "").trim();
  if (!departmentId || !agentId || creatingConversation.value) return;
  const vscodeRoot = vscodeWorkspaceRoots.value[0];
  const workspacePath = String(vscodeRoot?.path || "").trim();
  const workspaceName = String(vscodeRoot?.name || "").trim() || workspaceRootName.value || workspacePath;
  const shellWorkspaces = workspacePath
    ? [{
      id: `vscode-workspace-${workspacePath}`,
      name: workspaceName,
      path: workspacePath,
      level: "main" as const,
      access: workspaceAccess.value || "approval",
    }]
    : null;
  creatingConversation.value = true;
  createConversationErrorText.value = "";
  try {
    const result = await transport.request<{ conversationId: string; conversation?: OpenConversationResult }>("conversation.create", {
      title: input.title,
      departmentId,
      agentId,
      shellWorkspaces,
    });
    await refreshList();
    await openConversation(result.conversationId);
    createConversationDialogOpen.value = false;
  } catch (error) {
    createConversationErrorText.value = String(error || t('sidebar.createConversationFailed'));
  } finally {
    creatingConversation.value = false;
  }
}

function handleCreateConversationRequest(input?: { title?: string; departmentId?: string; agentId?: string; copyCurrent?: boolean; importPath?: string; shellWorkspaces?: ShellWorkspace[]; shellAutonomousMode?: boolean }) {
  const departmentId = String(input?.departmentId || "").trim();
  const agentId = String(input?.agentId || "").trim();
  if (departmentId && agentId) {
    void createConversation({ title: input?.title, departmentId, agentId });
    return;
  }
  void openCreateConversationDialog();
}

async function openSettings() {
  if (!isVsCodeWebviewRuntime()) {
    const opened = window.open(buildWebSettingsUrl(), "_blank", "noopener");
    if (!opened) {
      transport.errorText.value = t('sidebar.openSettingsFailed');
    }
    return;
  }
  try {
    await transport.request("settings.open", {});
  } catch (error) {
    transport.errorText.value = String(error || t('sidebar.openSettingsFailed'));
  }
}

function isVsCodeWebviewRuntime(): boolean {
  const maybeVsCodeApi = (window as Window & { acquireVsCodeApi?: unknown }).acquireVsCodeApi;
  return typeof maybeVsCodeApi === "function" || window.location.protocol === "vscode-webview:";
}

function buildWebSettingsUrl(): string {
  const path = window.location.pathname.endsWith(".html") ? "settings.html" : "/settings";
  const url = new URL(path, window.location.href);
  const config = transport.bridgeConfig.value;
  if (config?.chatUrl) {
    url.searchParams.set("chatUrl", config.chatUrl);
  }
  return url.toString();
}

async function openCodeReview() {
  codeReviewErrorText.value = "";
  try {
    if (createConversationDepartmentOptions.value.length === 0) {
      await refreshCreateConversationOptionsIfNeeded();
    }
    codeReviewDialogOpen.value = true;
  } catch (error) {
    codeReviewErrorText.value = String(error || t('sidebar.loadReviewDepartmentFailed'));
    codeReviewDialogOpen.value = true;
  }
}

function closeCodeReviewDialog() {
  if (codeReviewSubmitting.value) return;
  codeReviewDialogOpen.value = false;
  codeReviewErrorText.value = "";
}

async function loadCodeReviewCommitOptions(page = 1) {
  if (!activeConversationId.value) return;
  commitOptionsLoading.value = true;
  try {
    const result = await transport.request<{ total: number; page: number; pageSize: number; commits: ToolReviewCommitOption[] }>("toolReview.commitOptions.list", {
      conversationId: activeConversationId.value,
      page,
      pageSize: commitPageSize.value,
    });
    commitOptions.value = Array.isArray(result.commits) ? result.commits : [];
    commitTotal.value = Number(result.total || 0);
    commitPage.value = Number(result.page || page);
    commitPageSize.value = Number(result.pageSize || commitPageSize.value);
    codeReviewErrorText.value = "";
  } catch (error) {
    commitOptions.value = [];
    codeReviewErrorText.value = String(error || t('sidebar.readCommitFailed'));
  } finally {
    commitOptionsLoading.value = false;
  }
}

async function submitCodeReview(input: { scope: ToolReviewCodeReviewScope; target?: string; departmentId: string }) {
  if (!activeConversationId.value || codeReviewSubmitting.value) return;
  codeReviewSubmitting.value = true;
  codeReviewErrorText.value = "";
  try {
    await transport.request("toolReview.code.submit", {
      conversationId: activeConversationId.value,
      scope: input.scope,
      target: String(input.target || "").trim() || undefined,
      departmentId: input.departmentId,
    });
    codeReviewDialogOpen.value = false;
    if (reviewPanelOpen.value) loadReviewReports();
  } catch (error) {
    codeReviewErrorText.value = String(error || t('sidebar.startCodeReviewFailed'));
  } finally {
    codeReviewSubmitting.value = false;
  }
}

function toggleReviewPanel() {
  if (reviewPanelOpen.value) {
    reviewPanelOpen.value = false;
  } else {
    reviewPanelOpen.value = true;
    loadReviewReports();
  }
}

function toggleSideConversationList() {
  view.value = "chat";
  sideConversationListVisible.value = !sideConversationListVisible.value;
}

function toggleToolReviewPanel() {
  view.value = "chat";
  toolReviewPanelOpenVisible.value = !toolReviewPanelOpenVisible.value;
}

function handleDirectoryPickRestricted() {
  console.info("[Sidebar工作区] 跳过选择目录：web/sidebar 不允许唤起本机目录");
  transport.errorText.value = t("sidebar.openDirectoryRestricted");
}

function closeReviewPanel() {
  reviewPanelOpen.value = false;
}

async function loadReviewReports() {
  if (!activeConversationId.value) return;
  reviewReportsLoading.value = true;
  reviewErrorText.value = "";
  try {
    const result = await transport.request<{ reports: ToolReviewReportRecord[] }>("toolReview.reports.list", {
      conversationId: activeConversationId.value,
    });
    reviewReports.value = Array.isArray(result.reports) ? result.reports : [];
  } catch (error) {
    reviewErrorText.value = String(error || t('sidebar.loadReviewReportFailed'));
  } finally {
    reviewReportsLoading.value = false;
  }
}

async function deleteReviewReport(report: ToolReviewReportRecord) {
  if (!activeConversationId.value || reviewReportDeleting.value) return;
  reviewReportDeleting.value = true;
  try {
    await transport.request("toolReview.report.delete", {
      conversationId: activeConversationId.value,
      reportId: report.id,
    });
    await loadReviewReports();
  } catch (error) {
    reviewErrorText.value = String(error || t('sidebar.deleteReviewReportFailed'));
  } finally {
    reviewReportDeleting.value = false;
  }
}

async function retryReviewReport(report: ToolReviewReportRecord) {
  if (!activeConversationId.value || codeReviewSubmitting.value) return;
  codeReviewSubmitting.value = true;
  codeReviewErrorText.value = "";
  try {
    await transport.request("toolReview.code.submit", {
      conversationId: activeConversationId.value,
      scope: report.scope || "uncommitted",
      target: String(report.target || "").trim() || undefined,
      departmentId: String(report.departmentId || "").trim() || undefined,
    });
    await loadReviewReports();
  } catch (error) {
    reviewErrorText.value = String(error || t('sidebar.regenerateReviewReportFailed'));
  } finally {
    codeReviewSubmitting.value = false;
  }
}

async function branchConversationFromSelection(payload: { count: number; messageIds: string[] }) {
  const selectedMessageIds = Array.isArray(payload?.messageIds)
    ? payload.messageIds.map((item) => String(item || "").trim()).filter((item, index, array) => !!item && array.indexOf(item) === index)
    : [];
  if (!activeConversationId.value || selectedMessageIds.length === 0) return;
  try {
    const result = await transport.request<{ conversationId: string; title?: string; warning?: string | null }>("conversation.branchFromSelection", {
      sourceConversationId: activeConversationId.value,
      selectedMessageIds,
    });
    await refreshList();
    await openConversation(result.conversationId);
  } catch (error) {
    transport.errorText.value = String(error || t('sidebar.createBranchFailed'));
  }
}

async function delegateFromSelection(payload: { count: number; messageIds: string[]; departmentId: string; agentId: string; presetId: string; why: string; goal: string; todo: string }) {
  const selectedMessageIds = Array.isArray(payload?.messageIds)
    ? payload.messageIds.map((item) => String(item || "").trim()).filter((item, index, array) => !!item && array.indexOf(item) === index)
    : [];
  const targetDepartmentId = String(payload.departmentId || "").trim();
  const targetAgentId = String(payload.agentId || "").trim();
  const goal = String(payload.goal || "").trim();
  if (!activeConversationId.value || !targetDepartmentId || !targetAgentId || !goal) return;
  try {
    await transport.request("delegate.submit", {
      conversationId: activeConversationId.value,
      targetDepartmentId,
      targetAgentId,
      presetId: String(payload.presetId || "review").trim() || "review",
      why: String(payload.why || "").trim(),
      goal,
      todo: String(payload.todo || "").trim(),
      selectedMessageIds,
    });
    chatViewWrapperRef.value?.exitMessageSelectionMode();
  } catch (error) {
    transport.errorText.value = String(error || t('sidebar.startDelegateFailed'));
  }
}

function openSupervisionTask() {
  if (!activeConversationId.value) {
    transport.errorText.value = t('sidebar.noConversationForTask');
    return;
  }
  try {
    const selection = window.getSelection?.();
    if (selection && selection.rangeCount > 0 && String(selection.toString() || "").trim()) {
      selection.removeAllRanges();
    }
  } catch {
    // ignore selection cleanup failures
  }
  supervisionErrorText.value = "";
  supervisionDialogOpen.value = true;
}

function closeSupervisionTask() {
  if (supervisionSaving.value) return;
  supervisionDialogOpen.value = false;
  supervisionErrorText.value = "";
}

async function saveSupervisionTask(payload: { durationHours: number; goal: string; why: string; todo: string }) {
  if (!activeConversationId.value || supervisionSaving.value) return;
  const objective = String(payload.goal || "").trim();
  if (!objective) {
    supervisionErrorText.value = t('chat.supervision.goalPlaceholder');
    return;
  }
  supervisionSaving.value = true;
  supervisionErrorText.value = "";
  try {
    if (sidebarSupervisionActive.value) {
      await transport.request<GoalMutationOutput>("goal.cancel", {
        conversationId: activeConversationId.value,
      });
    }
    const created = await transport.request<GoalMutationOutput>("goal.create", {
      conversationId: activeConversationId.value,
      objective,
    });
    activeConversationGoal.value = String(created.goal?.status || "").trim() === "active"
      ? created.goal
      : null;
    supervisionDialogOpen.value = false;
  } catch (error) {
    supervisionErrorText.value = String(error || t('chat.supervision.saveFailed'));
  } finally {
    supervisionSaving.value = false;
  }
}

async function stopSupervisionTask() {
  if (!activeConversationId.value || supervisionSaving.value) return;
  if (!sidebarSupervisionActive.value) {
    supervisionErrorText.value = t('chat.supervision.noActiveTask');
    return;
  }
  supervisionSaving.value = true;
  supervisionErrorText.value = "";
  try {
    await transport.request<GoalMutationOutput>("goal.cancel", {
      conversationId: activeConversationId.value,
    });
    activeConversationGoal.value = null;
    supervisionDialogOpen.value = false;
  } catch (error) {
    supervisionErrorText.value = String(error || t('chat.supervision.stopFailed'));
  } finally {
    supervisionSaving.value = false;
  }
}

function applyModelPayload(payload: SidebarModelPayload) {
  conversationCallPrimaryApiConfigId.value = String(payload.conversationCallPrimaryApiConfigId || "").trim();
  preferredChatModelId.value = String(payload.preferredChatModelId || "").trim();
  chatModelOptions.value = Array.isArray(payload.chatModelOptions) ? payload.chatModelOptions : [];
}

async function selectConversationPreferredModel(apiConfigId: string) {
  const nextId = String(apiConfigId || "").trim();
  if (!activeConversationId.value || nextId === preferredChatModelId.value) return;
  const previousId = conversationCallPrimaryApiConfigId.value;
  const previousPreferredId = preferredChatModelId.value;
  console.info("[会话首选模型] VS Code sidebar 切换会话首选模型", {
    conversationId: activeConversationId.value,
    preferredApiConfigId: nextId || null,
  });
  conversationCallPrimaryApiConfigId.value = nextId;
  preferredChatModelId.value = nextId;
  try {
    const result = await transport.request<SidebarModelPayload>("model.select", {
      conversationId: activeConversationId.value,
      apiConfigId: nextId,
    });
    applyModelPayload(result);
    if (busy.value) {
      transport.errorText.value = t('sidebar.modelSwitched');
    }
  } catch (error) {
    conversationCallPrimaryApiConfigId.value = previousId;
    preferredChatModelId.value = previousPreferredId;
    transport.errorText.value = String(error || t('sidebar.modelSwitchFailed'));
  }
}

async function selectWorkspaceAccess(access: "read_only" | "approval" | "full_access") {
  if (!activeConversationId.value || workspaceAccess.value === access) return;
  const previous = workspaceAccess.value;
  workspaceAccess.value = access;
  const vscodeRoot = vscodeWorkspaceRoots.value[0];
  const workspacePath = vscodeRoot?.path || workspaceRootPath.value || undefined;
  const workspaceName = vscodeRoot?.name || workspaceRootName.value || undefined;
  try {
    const result = await transport.request<SidebarWorkspacePermission>("workspace.permission.select", {
      conversationId: activeConversationId.value,
      access,
      workspacePath,
      workspaceName,
    });
    applyWorkspacePermission(result);
  } catch (error) {
    workspaceAccess.value = previous;
    transport.errorText.value = String(error || t('sidebar.permissionSwitchFailed'));
  }
}

async function openCompactionDialog() {
  if (!activeConversationId.value || compacting.value) return;
  compactionDialogOpen.value = true;
  compactionPreviewLoading.value = true;
  compactionPreview.value = null;
  compactionErrorText.value = "";
  try {
    compactionPreview.value = await transport.request<CompactionPreviewResult>("conversation.compactPreview", {
      conversationId: activeConversationId.value,
    });
  } catch (error) {
    compactionErrorText.value = String(error || t('sidebar.loadCompactionPreviewFailed'));
  } finally {
    compactionPreviewLoading.value = false;
  }
}

function closeCompactionDialog() {
  if (compacting.value) return;
  compactionDialogOpen.value = false;
  compactionPreview.value = null;
  compactionErrorText.value = "";
}

async function confirmCompaction() {
  if (!activeConversationId.value || compacting.value || !compactionPreview.value?.canCompact) return;
  compacting.value = true;
  compactionErrorText.value = "";
  try {
    const result = await transport.request<{ compactionMessage?: ChatMessage }>("conversation.compact", {
      conversationId: activeConversationId.value,
    });
    if (result.compactionMessage) appendMessages({ conversationId: activeConversationId.value, message: result.compactionMessage });
    await refreshList();
    await openConversation(activeConversationId.value);
    compactionDialogOpen.value = false;
  } catch (error) {
    compactionErrorText.value = String(error || t('sidebar.compactionFailed'));
  } finally {
    compacting.value = false;
  }
}

function readBlobAsDataUrl(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result || ""));
    reader.onerror = () => reject(reader.error || new Error("读取剪贴板图片失败"));
    reader.readAsDataURL(blob);
  });
}

function pastedImageFiles(event: ClipboardEvent): File[] {
  const data = event.clipboardData;
  if (!data) return [];
  const filesFromItems = data.items && data.items.length > 0
    ? Array.from(data.items)
      .filter((item) => item.kind === "file" && item.type.toLowerCase().startsWith("image/"))
      .map((item) => item.getAsFile())
      .filter((file): file is File => !!file)
    : [];
  if (filesFromItems.length > 0) return filesFromItems;
  return data.files
    ? Array.from(data.files).filter((file) => String(file.type || "").toLowerCase().startsWith("image/"))
    : [];
}

async function appendClipboardImagesFromPaste(event: ClipboardEvent) {
  if (view.value !== "chat" || busy.value || compacting.value) return;
  const files = pastedImageFiles(event);
  if (files.length === 0) return;
  event.preventDefault();
  try {
    for (const file of files) {
      const dataUrl = await readBlobAsDataUrl(file);
      const bytesBase64 = dataUrl.includes(",") ? dataUrl.split(",")[1] : "";
      if (!bytesBase64) continue;
      clipboardImages.value.push({
        mime: String(file.type || "image/png").trim() || "image/png",
        bytesBase64,
      });
    }
  } catch (error) {
    transport.errorText.value = String(error || t('sidebar.readClipboardImageFailed'));
  }
}

function removeClipboardImage(index: number) {
  if (index < 0 || index >= clipboardImages.value.length) return;
  clipboardImages.value.splice(index, 1);
}

function pickAttachments() {
  if (busy.value || compacting.value) return;
  if (!attachmentInputRef.value) return;
  attachmentInputRef.value.value = "";
  attachmentInputRef.value.click();
}

async function appendAttachmentFiles(files: File[]) {
  const supported = files.filter((file) => {
    const mime = String(file.type || "").toLowerCase();
    return mime.startsWith("image/") || mime === "application/pdf";
  });
  if (supported.length === 0) return;
  try {
    for (const file of supported) {
      const dataUrl = await readBlobAsDataUrl(file);
      const bytesBase64 = dataUrl.includes(",") ? dataUrl.split(",")[1] : "";
      if (!bytesBase64) continue;
      clipboardImages.value.push({
        mime: String(file.type || "").trim() || "application/octet-stream",
        bytesBase64,
      });
    }
  } catch (error) {
    transport.errorText.value = String(error || t('sidebar.readClipboardImageFailed'));
  }
}

function handleAttachmentInputChange(event: Event) {
  const target = event.target as HTMLInputElement | null;
  const files = target?.files ? Array.from(target.files) : [];
  void appendAttachmentFiles(files);
}

async function send(payload?: { extraTextBlocks?: string[] }) {
  const text = inputText.value.trim();
  const images = clipboardImages.value.map((item) => ({ ...item }));
  const extraTextBlocks = (Array.isArray(payload?.extraTextBlocks) ? payload.extraTextBlocks : [])
    .map((item) => String(item || "").trim())
    .filter(Boolean);
  if ((!text && images.length === 0 && extraTextBlocks.length === 0) || !activeConversationId.value || busy.value) return;
  inputText.value = "";
  clipboardImages.value = [];
  busy.value = true;
  try {
    await transport.request("chat.send", {
      conversationId: activeConversationId.value,
      text,
      images,
      extraTextBlocks,
    });
  } catch (error) {
    busy.value = false;
    clearStreamingState();
    if (!inputText.value.trim()) inputText.value = text;
    clipboardImages.value = [...images, ...clipboardImages.value];
    transport.errorText.value = String(error || t('sidebar.sendFailed'));
  }
}

async function stop() {
  if (!activeConversationId.value) return;
  await transport.request("chat.stop", {
    conversationId: activeConversationId.value,
    partialAssistantText: streamingText.value,
    partialStreamBlocks: normalizeAssistantStreamBlocks(streamBlocks.value),
  });
  busy.value = false;
}

function resolveRewindTargetUserMessage(turnId: string): { targetUserMessageId: string; keepCount: number } | null {
  const targetId = String(turnId || "").trim();
  if (!targetId) return null;
  const index = messages.value.findIndex((item) => item.id === targetId);
  if (index < 0) return null;
  if (String(messages.value[index]?.role || "").trim() === "user") {
    return { targetUserMessageId: targetId, keepCount: index };
  }
  for (let i = index - 1; i >= 0; i -= 1) {
    if (String(messages.value[i]?.role || "").trim() === "user") {
      return { targetUserMessageId: String(messages.value[i]?.id || "").trim(), keepCount: i };
    }
  }
  return null;
}

async function recallTurn(payload: { turnId: string }) {
  if (!activeConversationId.value) return;
  if (rewindInFlight) {
    console.info("[会话撤回] 跳过：已有撤回流程正在进行", { turnId: payload?.turnId });
    return;
  }
  if (busy.value || compacting.value) {
    transport.errorText.value = t('sidebar.rewindRunning');
    return;
  }
  const target = resolveRewindTargetUserMessage(payload.turnId);
  if (!target?.targetUserMessageId) {
    transport.errorText.value = t('sidebar.rewindNotFound');
    return;
  }
  rewindInFlight = true;
  try {
    const mode = await requestRecallMode(target.targetUserMessageId);
    if (mode === "cancel") return;
    const result = await transport.request<RewindConversationResult>("conversation.rewind", {
      conversationId: activeConversationId.value,
      messageId: target.targetUserMessageId,
      undoApplyPatch: mode === "with_patch",
    });
    clearStreamingState();
    const recalled = result.recalledUserMessage || messages.value[target.keepCount];
    inputText.value = recalled ? removeBinaryPlaceholders(messageText(recalled)) : inputText.value;
    if (result.conversation) {
      activeConversationId.value = result.conversation.conversationId;
      messages.value = Array.isArray(result.conversation.messages) ? result.conversation.messages : messages.value.slice(0, target.keepCount);
      persona.value = result.conversation.persona || persona.value;
      applyModelPayload(result.conversation.model || {});
    } else {
      messages.value = messages.value.slice(0, target.keepCount);
    }
    selectedBlockId.value = null;
    hasPrevBlock.value = true;
    await refreshList();
  } catch (error) {
    transport.errorText.value = String(error || t('sidebar.rewindFailed'));
  } finally {
    rewindInFlight = false;
  }
}

async function createConversationBranchFromTurn(payload: { turnId: string }) {
  if (!activeConversationId.value) return;
  if (branchingConversation.value || busy.value || compacting.value) {
    transport.errorText.value = t('sidebar.createBranchFailed');
    return;
  }
  const target = resolveRewindTargetUserMessage(payload.turnId);
  if (!target?.targetUserMessageId) {
    transport.errorText.value = t('sidebar.rewindNotFound');
    return;
  }
  const confirmed = await requestCreateConversationBranchFromMessageConfirm();
  if (!confirmed) return;
  branchingConversation.value = true;
  try {
    const result = await transport.request<{ conversationId: string }>("conversation.branchFromMessage", {
      sourceConversationId: activeConversationId.value,
      turnMessageId: target.targetUserMessageId,
    });
    const conversationId = String(result?.conversationId || "").trim();
    if (!conversationId) return;
    await refreshList();
    await openConversation(conversationId);
  } catch (error) {
    transport.errorText.value = String(error || t('sidebar.createBranchFailed'));
  } finally {
    branchingConversation.value = false;
  }
}

function requestCreateConversationBranchFromMessageConfirm(): Promise<boolean> {
  cancelPendingBranchFromMessageConfirm();
  branchFromMessageConfirmDialogOpen.value = true;
  return new Promise((resolve) => {
    branchFromMessageConfirmResolver = resolve;
  });
}

function confirmBranchFromMessage() {
  const resolver = branchFromMessageConfirmResolver;
  branchFromMessageConfirmResolver = null;
  branchFromMessageConfirmDialogOpen.value = false;
  if (resolver) resolver(true);
}

function cancelBranchFromMessageConfirm() {
  cancelPendingBranchFromMessageConfirm();
}

function cancelPendingBranchFromMessageConfirm() {
  const resolver = branchFromMessageConfirmResolver;
  branchFromMessageConfirmResolver = null;
  branchFromMessageConfirmDialogOpen.value = false;
  if (resolver) resolver(false);
}

async function confirmPlan(payload: { messageId: string }) {
  const conversationId = activeConversationId.value;
  const planMessageId = String(payload?.messageId || "").trim();
  if (!conversationId || !planMessageId || busy.value || compacting.value) return;
  clearStreamingState();
  try {
    await transport.request("conversation.planMode.set", {
      conversationId,
      planModeEnabled: false,
    });
    patchConversationPlanMode(conversationId, false);
    busy.value = true;
    await transport.request("conversation.plan.confirm", {
      conversationId,
      planMessageId,
      departmentId: activeDepartmentId.value || undefined,
      agentId: activeAgentId.value || undefined,
    });
  } catch (error) {
    busy.value = false;
    transport.errorText.value = String(error || t('sidebar.confirmPlanFailed'));
  }
}

async function readPlanFileContent(input: { conversationId: string; path: string }): Promise<string> {
  const result = await transport.request<{ content?: string }>("conversation.plan.readFile", {
    conversationId: input.conversationId,
    path: input.path,
  });
  return String(result.content || "");
}

async function getRewindPreview(targetUserMessageId: string): Promise<RewindConversationPreviewResult> {
  const conversationId = String(activeConversationId.value || "").trim();
  const messageId = String(targetUserMessageId || "").trim();
  if (!conversationId || !messageId) {
    return { conversationId, canUndoPatch: false, hint: "缺少撤回预览所需的会话上下文。" };
  }
  return await transport.request<RewindConversationPreviewResult>("conversation.rewindPreview", {
    conversationId,
    messageId,
  });
}

async function requestRecallMode(targetUserMessageId: string): Promise<"message_only" | "with_patch" | "cancel"> {
  cancelPendingRewindConfirm();
  try {
    const preview = await getRewindPreview(targetUserMessageId);
    rewindConfirmCanUndoPatch.value = !!preview.canUndoPatch;
    console.info("[会话撤回] 完成：侧边栏撤回预览", {
      conversationId: preview.conversationId,
      messageId: targetUserMessageId,
      canUndoPatch: preview.canUndoPatch,
      hint: String(preview.hint || "").trim(),
    });
  } catch (error) {
    rewindConfirmCanUndoPatch.value = false;
    console.warn("[会话撤回] 失败：侧边栏撤回预览失败，仅撤回消息", {
      messageId: targetUserMessageId,
      error,
    });
  }
  rewindConfirmDialogOpen.value = true;
  return new Promise((resolve) => {
    rewindConfirmResolver = resolve;
  });
}

function confirmRewindWithPatch() {
  const resolver = rewindConfirmResolver;
  rewindConfirmResolver = null;
  rewindConfirmDialogOpen.value = false;
  rewindConfirmCanUndoPatch.value = false;
  if (resolver) resolver("with_patch");
}

function confirmRewindMessageOnly() {
  const resolver = rewindConfirmResolver;
  rewindConfirmResolver = null;
  rewindConfirmDialogOpen.value = false;
  rewindConfirmCanUndoPatch.value = false;
  if (resolver) resolver("message_only");
}

function cancelRewindConfirm() {
  cancelPendingRewindConfirm();
}

function cancelPendingRewindConfirm() {
  const resolver = rewindConfirmResolver;
  rewindConfirmResolver = null;
  rewindConfirmDialogOpen.value = false;
  rewindConfirmCanUndoPatch.value = false;
  if (resolver) resolver("cancel");
}

type WorkspaceListResult = {
  workspaces: Array<{ id: string; name: string; path: string; level: string; access: string; builtIn: boolean }>;
  rootPath: string;
  workspaceName: string;
  autonomousMode: boolean;
};

type WorkspaceDirectoryListResult = {
  path: string;
  name: string;
  directories: Array<{ path: string; name: string }>;
};

async function refreshWorkspaceList() {
  if (!activeConversationId.value) return;
  try {
    const result = await transport.request<WorkspaceListResult>("workspace.list", {
      conversationId: activeConversationId.value,
    });
    const workspaces = Array.isArray(result.workspaces) ? result.workspaces : [];
    workspaceDraftChoices.value = workspaces.map((item) => ({
      id: String(item.id || "").trim(),
      name: String(item.name || "").trim(),
      path: String(item.path || "").trim(),
      level: (String(item.level || "").trim().toLowerCase() === "main" ? "main" : String(item.level || "").trim().toLowerCase() === "system" ? "system" : "secondary") as ChatWorkspaceChoice["level"],
      access: String(item.access || "approval").trim() as ChatWorkspaceChoice["access"],
    }));
    workspaceDraftAutonomousMode.value = Boolean(result.autonomousMode);
    currentWorkspaceName.value = String(result.workspaceName || "").trim();
    workspaceRootPath.value = String(result.rootPath || "").trim();
    const mainWorkspace = workspaceDraftChoices.value.find((item) => item.level === "main")
      || workspaceDraftChoices.value.find((item) => item.level !== "system")
      || null;
    workspaceManualPath.value = String(mainWorkspace?.path || result.rootPath || "").trim();
    workspaceManualAccess.value = normalizeWorkspaceAccess(mainWorkspace?.access || workspaceAccess.value || "approval");
    if (workspacePickerOpen.value && workspaceManualPath.value) {
      void loadWorkspaceDirectory(workspaceManualPath.value);
    }
  } catch {
    workspaceDraftChoices.value = [];
    workspaceManualPath.value = String(workspaceRootPath.value || "").trim();
    workspaceManualAccess.value = normalizeWorkspaceAccess(workspaceAccess.value || "approval");
  }
}

function openWorkspacePicker() {
  workspaceManualPath.value = String(workspaceRootPath.value || "").trim();
  workspaceManualAccess.value = normalizeWorkspaceAccess(workspaceAccess.value || "approval");
  workspaceBrowserPath.value = "";
  workspaceDirectoryItems.value = [];
  workspaceDirectoryError.value = "";
  if (workspaceManualPath.value) {
    void loadWorkspaceDirectory(workspaceManualPath.value);
  }
  void refreshWorkspaceList();
  workspacePickerOpen.value = true;
}

function closeWorkspacePicker() {
  if (workspacePickerSaving.value) return;
  workspacePickerOpen.value = false;
}

async function loadWorkspaceDirectory(pathInput: string) {
  const path = String(pathInput || "").trim();
  if (!path || workspaceDirectoryLoading.value) return;
  workspaceDirectoryLoading.value = true;
  workspaceDirectoryError.value = "";
  try {
    const result = await transport.request<WorkspaceDirectoryListResult>("workspace.directory.list", { path }, 10000);
    workspaceBrowserPath.value = String(result.path || path).trim();
    workspaceManualPath.value = workspaceBrowserPath.value;
    workspaceDirectoryItems.value = (Array.isArray(result.directories) ? result.directories : [])
      .map((item) => ({
        path: String(item.path || "").trim(),
        name: String(item.name || "").trim(),
      }))
      .filter((item) => !!item.path && !!item.name);
  } catch (error) {
    workspaceDirectoryError.value = String(error || "读取目录失败");
    workspaceDirectoryItems.value = [];
  } finally {
    workspaceDirectoryLoading.value = false;
  }
}

async function saveWorkspacePicker() {
  if (workspacePickerSaving.value || !activeConversationId.value) return;
  const path = String(workspaceManualPath.value || "").trim();
  if (!path) return;
  workspacePickerSaving.value = true;
  try {
    const id = stableManualWorkspaceId(path);
    const name = workspaceNameFromPath(path);
    await transport.request("workspace.layout.save", {
      conversationId: activeConversationId.value,
      workspaces: [{
        id,
        name,
        path,
        level: "main",
        access: normalizeWorkspaceAccess(workspaceManualAccess.value),
        builtIn: false,
      }],
      autonomousMode: workspaceDraftAutonomousMode.value,
    });
    workspacePickerOpen.value = false;
    await refreshWorkspacePermission();
    await refreshWorkspaceList();
  } catch (error) {
    transport.errorText.value = String(error || t('sidebar.saveWorkspaceFailed'));
  } finally {
    workspacePickerSaving.value = false;
  }
}

function normalizeWorkspaceAccess(access: string): ChatWorkspaceChoice["access"] {
  if (access === "read_only" || access === "full_access") return access;
  return "approval";
}

function stableManualWorkspaceId(path: string): string {
  let hash = 0;
  for (let index = 0; index < path.length; index += 1) {
    hash = ((hash << 5) - hash + path.charCodeAt(index)) | 0;
  }
  return `manual-workspace-${Math.abs(hash).toString(36)}`;
}

function workspaceNameFromPath(path: string): string {
  const normalized = String(path || "").trim().replace(/[\\/]+$/, "");
  const parts = normalized.split(/[\\/]+/).filter(Boolean);
  return parts[parts.length - 1] || normalized || "workspace";
}

function parentWorkspaceDirectoryPath(path: string): string {
  const normalized = String(path || "").trim().replace(/[\\/]+$/, "");
  if (!normalized) return "";
  const separatorIndex = Math.max(normalized.lastIndexOf("/"), normalized.lastIndexOf("\\"));
  if (separatorIndex < 0) return "";
  if (separatorIndex === 0) return normalized.slice(0, 1);
  const windowsDriveRoot = /^[A-Za-z]:[\\/]?$/.test(normalized.slice(0, separatorIndex + 1));
  if (windowsDriveRoot) return normalized.slice(0, separatorIndex + 1);
  return normalized.slice(0, separatorIndex);
}

function appendMessages(next: unknown) {
  const payload = next as { conversationId?: string; messages?: ChatMessage[]; message?: ChatMessage };
  if (payload.conversationId && payload.conversationId !== activeConversationId.value) return;
  const incoming = payload.messages || (payload.message ? [payload.message] : []);
  if (!incoming.length) return;
  const existingIds = new Set(messages.value.map((item) => item.id));
  messages.value = [...messages.value, ...incoming.filter((item) => !existingIds.has(item.id))];
}

async function initializeAfterBridgeAuthenticated() {
  if (!transport.connected.value || !transport.authenticated.value) return;
  await refreshList();
  const currentConversationId = String(activeConversationId.value || "").trim();
  const initialConversationId = pickInitialSidebarConversationId(visibleConversations.value);
  const initialSummary = visibleConversations.value.find((item) =>
    String(item.conversationId || "").trim() === initialConversationId
  );
  const currentSummary = conversations.value.find((item) =>
    String(item.conversationId || "").trim() === currentConversationId
  );
  const currentRemoteSummary = remoteImContactConversations.value.find((item) =>
    String(item.conversationId || "").trim() === currentConversationId
  );
  const shouldSwitchForWorkspace =
    sidebarHasWorkspaceContext()
    && !!initialSummary
    && !isSidebarSystemConversation(initialSummary)
    && initialConversationId !== currentConversationId
    && !currentRemoteSummary
    && (!currentSummary || !conversationMatchesCurrentSidebarWorkspace(currentSummary));
  if (!currentConversationId || shouldSwitchForWorkspace) {
    if (initialConversationId) {
      try {
        await openConversation(initialConversationId);
      } catch (error) {
        const fallbackConversationId = String(visibleConversations.value.find((item) =>
          isSidebarSystemConversation(item) && isSidebarConversationOpenable(item)
        )?.conversationId || "").trim();
        if (fallbackConversationId && fallbackConversationId !== initialConversationId) {
          await openConversation(fallbackConversationId);
        } else {
          console.warn("[Sidebar首屏会话选择] 打开初始会话失败", {
            initialConversationId,
            error,
          });
        }
      }
    }
  }
  await refreshIdeContextGroups();
}

async function reloadActiveSidebarConversation(reason: string) {
  const currentConversationId = String(activeConversationId.value || "").trim();
  if (!currentConversationId) return;
  try {
    await openConversation(currentConversationId);
  } catch (error) {
    console.warn("[Sidebar会话恢复] 重新加载当前会话失败", {
      conversationId: currentConversationId,
      reason,
      error,
    });
  }
}

async function reconnectSidebarBridge(options?: { forceReloadActiveConversation?: boolean; reason?: string }) {
  const reason = String(options?.reason || "unknown").trim() || "unknown";
  const forceReloadActiveConversation = !!options?.forceReloadActiveConversation;
  transport.errorText.value = "";
  if (forceReloadActiveConversation) {
    resetActiveConversationTransientState(`${reason}_before_reconnect`);
  }
  const existingConfig = transport.bridgeConfig.value;
  if (existingConfig) {
    await transport.reconnect();
    if (transport.connected.value && transport.bridgeReady.value && transport.authenticated.value) {
      await initializeAfterBridgeAuthenticated();
      if (forceReloadActiveConversation) {
        await reloadActiveSidebarConversation(reason);
      }
      return;
    }
  }
  const config = await loadDiscovery();
  if (!config) {
    transport.errorText.value = t('sidebar.paiNotRunning');
    return;
  }
  await transport.connect(config);
  if (transport.connected.value && transport.bridgeReady.value && transport.authenticated.value) {
    await initializeAfterBridgeAuthenticated();
    if (forceReloadActiveConversation) {
      await reloadActiveSidebarConversation(reason);
    }
  }
}

function openRemoteAuthDialog() {
  remoteAuthDialogOpen.value = true;
  remoteAuthPassword.value = "";
  remoteAuthError.value = "";
}

async function submitRemoteAuth() {
  const password = remoteAuthPassword.value.trim();
  if (!password || remoteAuthSubmitting.value) return;
  remoteAuthSubmitting.value = true;
  remoteAuthError.value = "";
  try {
    await transport.login(password);
    remoteAuthDialogOpen.value = false;
    remoteAuthPassword.value = "";
    await initializeAfterBridgeAuthenticated();
  } catch (error) {
    remoteAuthError.value = String(error || t("sidebar.remoteAuthFailed"));
  } finally {
    remoteAuthSubmitting.value = false;
  }
}

function registerNotifications() {
  transport.onNotification("bridge.ready", (payload) => {
    const value = payload as { authRequired?: boolean };
    if (value.authRequired && !transport.authenticated.value) {
      openRemoteAuthDialog();
      return;
    }
    void initializeAfterBridgeAuthenticated();
  });
  transport.onNotification("conversation.overviewUpdated", (payload) => {
    const value = payload as { unarchivedConversations?: ConversationSummary[] };
    if (Array.isArray(value.unarchivedConversations)) {
      const incomingLocalConversations = value.unarchivedConversations;
      void refreshList().catch((error) => {
        console.warn("[Sidebar会话列表] 刷新完整列表失败，使用通知中的本地会话", {
          error,
          incomingLocal: incomingLocalConversations.length,
        });
        conversations.value = incomingLocalConversations;
      });
      clearCompletedRuntimeStateForConversation(activeConversationId.value);
    }
  });
  transport.onNotification("conversation.overviewItemUpdated", (payload) => {
    const value = payload as { conversation?: ConversationSummary };
    patchConversationOverviewItem(value.conversation);
  });
  transport.onNotification("ideContext.updated", () => {
    void refreshIdeContextGroups();
  });
  transport.onNotification("persona.changed", () => {
    markCreateConversationOptionsStale();
  });
  transport.onNotification("department.changed", () => {
    markCreateConversationOptionsStale();
  });
  transport.onNotification("departmentTree.changed", () => {
    markCreateConversationOptionsStale();
  });
  transport.onNotification("provider.changed", () => {
    markCreateConversationOptionsStale();
  });
  transport.onNotification("conversation.runtimeStateUpdated", (payload) => {
    const value = payload as { conversationId?: string; runtimeState?: string };
    const conversationId = String(value.conversationId || "").trim();
    if (!conversationId) return;
    const runtimeState = String(value.runtimeState || "").trim();
    patchConversationRuntimeState(conversationId, runtimeState);
    if (conversationId === activeConversationId.value && (runtimeState === "done" || runtimeState === "failed" || runtimeState === "completed" || !runtimeState)) {
      clearCompletedRuntimeStateForConversation(conversationId);
    }
  });
  transport.onNotification("conversation.todosUpdated", (payload) => {
    const value = payload as { conversationId?: string; currentTodos?: ChatTodoItem[] };
    if (String(value.conversationId || "").trim() === activeConversationId.value) {
      sidebarTodos.value = Array.isArray(value.currentTodos) ? value.currentTodos : [];
    }
  });
  transport.onNotification("conversation.goalUpdated", (payload) => {
    const value = payload as { conversationId?: string; goal?: ConversationGoalState | null };
    const conversationId = String(value.conversationId || "").trim();
    if (!conversationId) return;
    conversations.value = conversations.value.map((item) =>
      String(item.conversationId || "").trim() === conversationId
        ? { ...item, activeGoal: value.goal || null }
        : item,
    );
    if (conversationId === activeConversationId.value) {
      activeConversationGoal.value = String(value.goal?.status || "").trim() === "active"
        ? value.goal || null
        : null;
    }
  });
  transport.onNotification("conversation.delegateStatusUpdated", (payload) => {
    window.dispatchEvent(new CustomEvent("easy-call:conversation-delegate-status-updated", {
      detail: payload,
    }));
  });
  transport.onNotification("conversation.messageAppended", appendMessages);
  transport.onNotification("terminalApproval.requested", (payload) => {
    enqueueTerminalApprovalRequest(payload as TerminalApprovalRequestPayload);
  });
  transport.onNotification("chat.historyFlushed", appendMessages);
  transport.onNotification("chat.roundStarted", (payload) => {
    const value = payload as { conversationId?: string };
    if (value.conversationId === activeConversationId.value) {
      busy.value = true;
      clearStreamingState();
    }
  });
  transport.onNotification("chat.assistantDelta", (payload) => {
    const value = payload as SidebarAssistantDeltaPayload;
    if (value.conversationId !== activeConversationId.value) return;
    const delta = String(value.event?.delta || "");
    const kind = String(value.event?.kind || "").trim();
    const hasStreamCache = !!value.event?.streamCache;
    if (value.event?.streamCache) {
      applyRuntimeStreamCache({ streamCache: value.event.streamCache });
    }
    if (kind === "tool_status" && value.event) {
      applyAssistantToolStatusEvent(value.event);
      return;
    }
    if (kind === "assistant_tool_event" && value.event) {
      if (hasStreamCache) return;
      streamBlocks.value = applyAssistantToolEventToStreamBlocks(streamBlocks.value, value.event.message || "");
      return;
    }
    if (kind === "assistant_tool_result") return;
    if (!delta) return;
    if (kind === "activity_reasoning_delta") {
      return;
    } else if (!hasStreamCache) {
      streamingText.value += delta;
    } else {
      return;
    }
  });
  transport.onNotification("chat.roundFinished", (payload) => {
    const value = payload as { conversationId?: string; assistantMessage?: ChatMessage };
    clearCompletedRuntimeStateForConversation(value.conversationId || "");
    if (value.conversationId !== activeConversationId.value) return;
    busy.value = false;
    // 先追加正式消息再清流式状态，避免 Vue 先删草稿再插正式消息导致一帧闪烁。
    if (value.assistantMessage) appendMessages({ conversationId: value.conversationId, message: value.assistantMessage });
    clearStreamingState();
  });
}

async function bootstrap() {
  const config = await loadDiscovery();
  if (!config) {
    transport.errorText.value = t('sidebar.paiNotRunning');
    return;
  }
  await transport.connect(config);
  if (transport.connected.value && transport.bridgeReady.value && transport.authenticated.value) {
    await initializeAfterBridgeAuthenticated();
  }
}

function clearDiscoveryRefreshTimer() {
  if (discoveryRefreshTimer === null) return;
  window.clearTimeout(discoveryRefreshTimer);
  discoveryRefreshTimer = null;
}

function refreshDiscovery() {
  clearDiscoveryRefreshTimer();
  if (transport.bridgeConfig.value) {
    void reconnectSidebarBridge({
      forceReloadActiveConversation: true,
      reason: "manual_reconnect",
    }).catch((error) => {
      console.warn("[Sidebar桥接] 直接重连失败，回退 discovery 刷新", error);
      transport.connecting.value = true;
      window.parent.postMessage({ type: "pai-refresh-discovery" }, "*");
    });
    discoveryRefreshTimer = window.setTimeout(() => {
      discoveryRefreshTimer = null;
      if (transport.connected.value) return;
      transport.connecting.value = false;
      transport.errorText.value = t('sidebar.paiNotRunning');
    }, 3000);
    return;
  }
  transport.errorText.value = "";
  transport.connecting.value = true;
  window.parent.postMessage({ type: "pai-refresh-discovery" }, "*");
  discoveryRefreshTimer = window.setTimeout(() => {
    discoveryRefreshTimer = null;
    if (transport.connected.value) return;
    transport.connecting.value = false;
    transport.errorText.value = t('sidebar.paiNotRunning');
  }, 3000);
}

function handleWindowPaste(event: ClipboardEvent) {
  void appendClipboardImagesFromPaste(event);
}

function handleWindowMessage(event: MessageEvent) {
  const data = event.data as { type?: string; discovery?: DiscoveryPayload };
  if (data?.type === "pai-discovery" && data.discovery) {
    clearDiscoveryRefreshTimer();
    applyWorkspaceRoots(data.discovery.workspaceRoots);
    const config = normalizeDiscovery(data.discovery);
    if (config) void transport.connect(config).then(async () => {
      if (transport.bridgeReady.value && transport.authenticated.value) {
        await initializeAfterBridgeAuthenticated();
      }
    });
    else {
      transport.connecting.value = false;
      transport.errorText.value = t('sidebar.paiNotRunning');
    }
  }
}

function handleDocumentVisibilityChange() {
  if (document.visibilityState !== "visible") {
    resetActiveConversationTransientState("visibility_hidden");
    // 手机浏览器 / WebView 切后台后，旧 websocket 很容易变成僵尸连接。
    // 先主动关闭，回前台时再走“重连 + 重开当前会话”，避免旧流式态和新流式态并存。
    transport.close();
    return;
  }
  void reconnectSidebarBridge({
    forceReloadActiveConversation: true,
    reason: "visibility_visible",
  }).catch((error) => {
    console.warn("[Sidebar桥接] 前台恢复重连失败", error);
  });
}

onMounted(() => {
  registerNotifications();
  transport.onAuthRefreshNeeded(() => {
    refreshDiscovery();
  });
  window.addEventListener("message", handleWindowMessage);
  window.addEventListener("paste", handleWindowPaste);
  document.addEventListener("visibilitychange", handleDocumentVisibilityChange);
  const unlistenCodeReviewPromise = listen("code-review-requested", () => {
    openCodeReview();
  }).then((fn) => { unlistenCodeReviewFn = fn; });
  void bootstrap();
});

onBeforeUnmount(() => {
  clearDiscoveryRefreshTimer();
  cancelPendingRewindConfirm();
  window.removeEventListener("message", handleWindowMessage);
  window.removeEventListener("paste", handleWindowPaste);
  document.removeEventListener("visibilitychange", handleDocumentVisibilityChange);
  if (unlistenCodeReviewFn) unlistenCodeReviewFn();
});
</script>

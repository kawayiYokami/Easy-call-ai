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
    @show-list="view = 'list'"
    @show-chat="view = 'chat'"
    @new-conversation="openCreateConversationDialog"
    @open-settings="openSettings"
    @compact-conversation="openCompactionDialog"
    @reconnect="refreshDiscovery"
    @toggle-review-panel="toggleReviewPanel"
  >
    <ConversationListView
      v-if="view === 'list'"
      :items="conversations"
      :active-conversation-id="activeConversationId"
      :persona="listPersona"
      @select="openConversation"
    />
    <ChatViewWrapper
      v-else
      ref="chatViewWrapperRef"
      v-model:input="inputText"
      :active-conversation-id="activeConversationId"
      :active-agent-id="activeAgentId"
      :persona="persona"
      :selected-chat-model-id="selectedChatModelId"
      :chat-model-options="chatModelOptions"
      :workspace-access="workspaceAccess"
      :plan-mode-enabled="activeConversationPlanModeEnabled"
      :messages="messages"
      :clipboard-images="clipboardImages"
      :streaming-text="streamingText"
      :streaming-reasoning-standard="streamingReasoningStandard"
      :streaming-reasoning-inline="streamingReasoningInline"
      :tool-status-text="toolStatusText"
      :tool-status-state="toolStatusState"
      :stream-tool-calls="streamToolCalls"
      :busy="busy"
      :runtime-state="activeConversationRuntimeState"
      :has-prev-block="hasPrevBlock"
      :create-conversation-department-options="createConversationDepartmentOptions"
      :delegate-department-ids="sidebarDelegateDepartmentIds"
      :default-create-conversation-department-id="defaultCreateConversationDepartmentId"
      :current-department-id="activeDepartmentId"
      :current-workspace-name="currentWorkspaceName"
      :current-todos="sidebarTodos"
      :hide-workspace-button="hideWorkspaceButton"
      :terminal-approvals="activeConversationTerminalApprovals"
      :terminal-approval-resolving="terminalApprovalResolving"
      :ide-context-groups="vscodeIdeContextGroups"
      :read-plan-file-content="readPlanFileContent"
      @send="send"
      @stop="stop"
      @remove-clipboard-image="removeClipboardImage"
      @load-prev-block="loadPrevBlock"
      @update:selected-chat-model-id="selectChatModel"
      @update-workspace-access="selectWorkspaceAccess"
      @recall-turn="recallTurn"
      @confirm-plan="confirmPlan"
      @lock-workspace="openWorkspacePicker"
      @open-code-review="openCodeReview"
      @open-supervision-task="openSupervisionTask"
      @approve-terminal-approval="approveTerminalApproval"
      @deny-terminal-approval="denyTerminalApproval"
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
      :active-task="null"
      :recent-history="[]"
      @close="closeSupervisionTask"
      @save="saveSupervisionTask"
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
    <ChatWorkspacePickerDialog
      :open="workspacePickerOpen"
      :saving="workspacePickerSaving"
      :workspaces="workspaceDraftChoices"
      :autonomous-mode="workspaceDraftAutonomousMode"
      hide-add-workspace
      @close="closeWorkspacePicker"
      @set-main="setWorkspaceAsMain"
      @set-access="setWorkspaceAccess"
      @set-autonomous-mode="setWorkspaceAutonomousMode"
      @remove-workspace="removeWorkspace"
      @open-dir="openWorkspaceDir"
      @save="saveWorkspacePicker"
    />
  </SidebarLayout>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import type { ApiConfigItem, ChatMessage, ChatTodoItem, IdeContextWorkspaceGroup } from "../../types/app";
import { removeBinaryPlaceholders, messageText } from "../../utils/chat-message";
import { normalizeDepartmentChildIds } from "../config/utils/department-graph";
import { formatConversationFallbackTitle } from "../chat/utils/conversation-title";
import { useI18n } from "vue-i18n";
import SidebarLayout from "./layouts/SidebarLayout.vue";
import ConversationListView from "./views/ConversationListView.vue";
import ChatViewWrapper from "./views/ChatViewWrapper.vue";
import SidebarCompactionDialog from "./views/SidebarCompactionDialog.vue";
import SidebarReviewPanel from "./views/SidebarReviewPanel.vue";
import CreateConversationDialog, { type SidebarCreateDepartmentOption } from "./views/CreateConversationDialog.vue";
import { useWsTransport, type SidebarBridgeConfig } from "./composables/use-ws-transport";
import ToolReviewTargetDialog from "../chat/components/ToolReviewTargetDialog.vue";
import ChatSupervisionTaskDialog from "../chat/components/dialogs/ChatSupervisionTaskDialog.vue";
import ChatWorkspacePickerDialog from "../chat/components/dialogs/ChatWorkspacePickerDialog.vue";
import type { ChatWorkspaceChoice } from "../chat/composables/use-chat-workspace";
import type { ToolReviewCodeReviewScope, ToolReviewCommitOption, ToolReviewReportRecord } from "../chat/composables/use-chat-tool-review";
import type { TerminalApprovalConversationItem, TerminalApprovalRequestPayload } from "../shell/composables/use-terminal-approval";

type ConversationSummary = {
  conversationId: string;
  title: string;
  summaryTitle?: string;
  updatedAt: string;
  lastMessageAt?: string;
  messageCount?: number;
  unreadCount?: number;
  agentId?: string;
  departmentId?: string;
  departmentName?: string;
  runtimeState?: string;
  planModeEnabled?: boolean;
  detachedWindowOpen?: boolean;
  detachedWindowLabel?: string;
  previewMessages?: Array<{
    messageId: string;
    role: string;
    speakerAgentId?: string;
    createdAt?: string;
    textPreview?: string;
    hasImage?: boolean;
    hasPdf?: boolean;
    hasAudio?: boolean;
    hasAttachment?: boolean;
  }>;
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
  selectedChatModelId?: string;
  chatModelOptions?: ApiConfigItem[];
};

type SidebarStreamToolCallView = {
  toolCallId?: string;
  name: string;
  argsText: string;
  status?: "doing" | "done";
};

type SidebarStreamCachePayload = {
  assistantText?: string;
  reasoningStandard?: string;
  reasoningInline?: string;
  toolStatusText?: string;
  toolStatusState?: string;
  streamToolCalls?: unknown[];
};

type SidebarConversationRuntimePayload = {
  runtimeState?: string;
  streamCache?: SidebarStreamCachePayload;
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
  };
};

type CreateConversationOptionsResult = {
  departments: SidebarCreateDepartmentOption[];
  defaultDepartmentId: string;
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

const transport = useWsTransport();
const { t } = useI18n();
const conversations = ref<ConversationSummary[]>([]);
const activeConversationId = ref("");
const activeTitle = computed(() => {
  const item = activeSummary.value;
  if (!item) return "PAI";
  const title = String(item.title || "").trim();
  if (title) return title;
  const summary = String(item.summaryTitle || "").trim();
  if (summary) return summary;
  return formatConversationFallbackTitle(item.lastMessageAt || item.updatedAt) || "PAI";
});
const activeAgentId = ref("");
const persona = ref<SidebarPersonaPayload>({});
const listPersona = ref<SidebarPersonaPayload>({});
const selectedChatModelId = ref("");
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
const streamingReasoningStandard = ref("");
const streamingReasoningInline = ref("");
const toolStatusText = ref("");
const toolStatusState = ref<"running" | "done" | "failed" | "">("");
const streamToolCalls = ref<SidebarStreamToolCallView[]>([]);
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
const createConversationDepartmentOptions = ref<SidebarCreateDepartmentOption[]>([]);
const defaultCreateConversationDepartmentId = ref("");
const createConversationErrorText = ref("");
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
const supervisionDialogOpen = ref(false);
const supervisionSaving = ref(false);
const supervisionErrorText = ref("");
const selectedBlockId = ref<number | null>(null);
const hasPrevBlock = ref(false);
const view = ref<"list" | "chat">("list");
const rewindConfirmDialogOpen = ref(false);
const rewindConfirmCanUndoPatch = ref(false);
let rewindConfirmResolver: ((mode: "message_only" | "with_patch" | "cancel") => void) | null = null;
const currentWorkspaceName = ref("");
const workspacePickerOpen = ref(false);
const workspacePickerSaving = ref(false);
const workspaceDraftChoices = ref<ChatWorkspaceChoice[]>([]);
const workspaceDraftAutonomousMode = ref(false);
const terminalApprovalQueue = ref<TerminalApprovalRequestPayload[]>([]);
const terminalApprovalResolving = ref(false);
const hideWorkspaceButton = computed(() => false);
let discoveryRefreshTimer: number | null = null;

const activeSummary = computed(() => conversations.value.find((item) => item.conversationId === activeConversationId.value));
const activeConversationRuntimeState = computed(() => String(activeSummary.value?.runtimeState || "").trim());
const activeConversationPlanModeEnabled = computed(() => !!activeSummary.value?.planModeEnabled);
const activeDepartmentId = computed(() => String(activeSummary.value?.departmentId || "").trim());
const activeConversationTerminalApprovals = computed<TerminalApprovalConversationItem[]>(() =>
  listConversationTerminalApprovals(activeConversationId.value),
);
const sidebarDelegateDepartmentIds = computed(() => {
  const currentDept = createConversationDepartmentOptions.value.find(
    (item) => String(item.id || "").trim() === activeDepartmentId.value,
  );
  if (!currentDept) return [];
  const existingIds = new Set(
    createConversationDepartmentOptions.value
      .map((item) => String(item.id || "").trim())
      .filter(Boolean),
  );
  return normalizeDepartmentChildIds(currentDept.childDepartmentIds, currentDept.id)
    .filter((id: string) => existingIds.has(id));
});

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
  const result = await transport.request<{ conversations: ConversationSummary[]; persona?: SidebarPersonaPayload }>("conversation.list");
  conversations.value = Array.isArray(result.conversations) ? result.conversations : [];
  if (result.persona) listPersona.value = result.persona;
}

async function loadCreateConversationOptions() {
  const result = await transport.request<CreateConversationOptionsResult>("conversation.createOptions", {});
  createConversationDepartmentOptions.value = Array.isArray(result.departments) ? result.departments : [];
  defaultCreateConversationDepartmentId.value = String(result.defaultDepartmentId || "").trim()
    || createConversationDepartmentOptions.value[0]?.id
    || "";
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

function normalizeToolStatusState(value: unknown): "running" | "done" | "failed" | "" {
  const state = String(value || "").trim();
  return state === "running" || state === "done" || state === "failed" ? state : "";
}

function normalizeStreamToolCallView(value: unknown): SidebarStreamToolCallView | null {
  const raw = value && typeof value === "object" ? value as Record<string, unknown> : null;
  const toolCallId = String(raw?.toolCallId || "").trim();
  const name = String(raw?.name || "").trim();
  if (!toolCallId || !name) return null;
  return {
    toolCallId,
    name,
    argsText: String(raw?.argsText || ""),
    status: String(raw?.status || "") === "doing" ? "doing" : "done",
  };
}

function normalizeStreamToolCalls(values: unknown): SidebarStreamToolCallView[] {
  return Array.isArray(values)
    ? values
      .map((item) => normalizeStreamToolCallView(item))
      .filter((item): item is SidebarStreamToolCallView => !!item)
    : [];
}

function upsertStreamToolCall(nextCall: SidebarStreamToolCallView) {
  const toolCallId = String(nextCall.toolCallId || "").trim();
  if (!toolCallId) return;
  const index = streamToolCalls.value.findIndex((item) => String(item.toolCallId || "").trim() === toolCallId);
  if (index >= 0) {
    const current = streamToolCalls.value[index];
    streamToolCalls.value.splice(index, 1, {
      ...current,
      ...nextCall,
      argsText: nextCall.argsText || current.argsText,
      status: nextCall.status,
    });
    return;
  }
  streamToolCalls.value.push(nextCall);
}

function clearStreamingState() {
  streamingText.value = "";
  streamingReasoningStandard.value = "";
  streamingReasoningInline.value = "";
  toolStatusText.value = "";
  toolStatusState.value = "";
  streamToolCalls.value = [];
}

function applyRuntimeStreamCache(runtime: SidebarConversationRuntimePayload | null | undefined) {
  const cache = runtime?.streamCache;
  if (!cache) return;
  streamingText.value = String(cache.assistantText || "");
  streamingReasoningStandard.value = String(cache.reasoningStandard || "");
  streamingReasoningInline.value = String(cache.reasoningInline || "");
  toolStatusText.value = String(cache.toolStatusText || "");
  toolStatusState.value = normalizeToolStatusState(cache.toolStatusState);
  streamToolCalls.value = normalizeStreamToolCalls(cache.streamToolCalls);
}

function applyAssistantToolStatusEvent(event: NonNullable<SidebarAssistantDeltaPayload["event"]>) {
  const toolStatus = String(event.toolStatus || "").trim();
  const toolName = String(event.toolName || "").trim();
  const toolCallId = String(event.toolCallId || "").trim();
  if (toolCallId && toolName && (toolStatus === "running" || toolStatus === "done" || toolStatus === "failed")) {
    upsertStreamToolCall({
      toolCallId,
      name: toolName,
      argsText: String(event.toolArgs || ""),
      status: toolStatus === "running" ? "doing" : "done",
    });
  }
  toolStatusText.value = String(event.message || "");
  toolStatusState.value = normalizeToolStatusState(toolStatus);
}

async function openConversation(conversationId: string) {
  clearCompletedRuntimeStateForConversation(activeConversationId.value);
  const vscodeRoot = vscodeWorkspaceRoots.value[0];
  const result = await transport.request<OpenConversationResult>("conversation.open", {
    conversationId,
    workspacePath: vscodeRoot?.path || undefined,
    workspaceName: vscodeRoot?.name || undefined,
  });
  activeConversationId.value = result.conversationId;
  clearCompletedRuntimeStateForConversation(result.conversationId);
  activeAgentId.value = String(result.agentId || "").trim();
  persona.value = result.persona || {};
  if (result.persona) listPersona.value = result.persona;
  applyModelPayload(result.model || {});
  await refreshWorkspacePermission();
  messages.value = Array.isArray(result.messages) ? result.messages : [];
  sidebarTodos.value = Array.isArray(result.currentTodos) ? result.currentTodos : [];
  clearStreamingState();
  applyRuntimeStreamCache(result.runtime);
  selectedBlockId.value = null;
  hasPrevBlock.value = true;
  view.value = "chat";
  loadCreateConversationOptions();
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
    title: String(payload.title || "终端审批"),
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
    transport.errorText.value = String(error || "处理审批失败");
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
    await loadCreateConversationOptions();
    createConversationDialogOpen.value = true;
  } catch (error) {
    createConversationErrorText.value = String(error || "加载部门列表失败");
    createConversationDialogOpen.value = true;
  }
}

function closeCreateConversationDialog() {
  if (creatingConversation.value) return;
  createConversationDialogOpen.value = false;
  createConversationErrorText.value = "";
}

async function createConversation(input: { title?: string; departmentId: string }) {
  const departmentId = String(input.departmentId || "").trim();
  if (!departmentId || creatingConversation.value) return;
  creatingConversation.value = true;
  createConversationErrorText.value = "";
  try {
    const result = await transport.request<{ conversationId: string; conversation?: OpenConversationResult }>("conversation.create", {
      title: input.title,
      departmentId,
    });
    await refreshList();
    await openConversation(result.conversationId);
    createConversationDialogOpen.value = false;
  } catch (error) {
    createConversationErrorText.value = String(error || "创建会话失败");
  } finally {
    creatingConversation.value = false;
  }
}

async function openSettings() {
  try {
    await transport.request("settings.open", {});
  } catch (error) {
    transport.errorText.value = String(error || "打开设置失败");
  }
}

async function openCodeReview() {
  codeReviewErrorText.value = "";
  try {
    if (createConversationDepartmentOptions.value.length === 0) {
      await loadCreateConversationOptions();
    }
    codeReviewDialogOpen.value = true;
  } catch (error) {
    codeReviewErrorText.value = String(error || "加载审查部门失败");
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
    codeReviewErrorText.value = String(error || "读取 commit 失败");
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
    codeReviewErrorText.value = String(error || "发起代码审查失败");
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
    reviewErrorText.value = String(error || "加载审查报告失败");
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
    reviewErrorText.value = String(error || "删除审查报告失败");
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
    reviewErrorText.value = String(error || "重新生成审查报告失败");
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
    transport.errorText.value = String(error || "创建会话分支失败");
  }
}

async function delegateFromSelection(payload: { count: number; messageIds: string[]; departmentId: string; presetId: string; background: string; question: string; focus: string }) {
  const selectedMessageIds = Array.isArray(payload?.messageIds)
    ? payload.messageIds.map((item) => String(item || "").trim()).filter((item, index, array) => !!item && array.indexOf(item) === index)
    : [];
  const targetDepartmentId = String(payload.departmentId || "").trim();
  const question = String(payload.question || "").trim();
  if (!activeConversationId.value || !targetDepartmentId || !question) return;
  try {
    await transport.request("delegate.submit", {
      conversationId: activeConversationId.value,
      targetDepartmentId,
      presetId: String(payload.presetId || "review").trim() || "review",
      background: String(payload.background || "").trim(),
      question,
      focus: String(payload.focus || "").trim(),
      selectedMessageIds,
    });
    chatViewWrapperRef.value?.exitMessageSelectionMode();
  } catch (error) {
    transport.errorText.value = String(error || "发起委托失败");
  }
}

function openSupervisionTask() {
  if (!activeConversationId.value) {
    transport.errorText.value = "当前没有会话，无法创建目标任务";
    return;
  }
  supervisionErrorText.value = "";
  supervisionDialogOpen.value = true;
}

function closeSupervisionTask() {
  if (supervisionSaving.value) return;
  supervisionDialogOpen.value = false;
  supervisionErrorText.value = "";
}

function formatDateToLocalRfc3339(date: Date): string {
  const pad = (value: number) => String(value).padStart(2, "0");
  const offsetMinutes = -date.getTimezoneOffset();
  const offsetSign = offsetMinutes >= 0 ? "+" : "-";
  const absOffset = Math.abs(offsetMinutes);
  return [
    `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`,
    `T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`,
    `${offsetSign}${pad(Math.floor(absOffset / 60))}:${pad(absOffset % 60)}`,
  ].join("");
}

async function saveSupervisionTask(payload: { durationHours: number; goal: string; why: string; todo: string }) {
  if (!activeConversationId.value || supervisionSaving.value) return;
  supervisionSaving.value = true;
  supervisionErrorText.value = "";
  try {
    const now = new Date();
    now.setSeconds(0, 0);
    const endAt = new Date(now.getTime() + payload.durationHours * 3_600_000);
    const created = await transport.request<{ taskId?: string }>("task.create", {
      conversationId: activeConversationId.value,
      targetScope: "desktop",
      goal: `Goal Task：${String(payload.goal || "").trim()}`,
      why: String(payload.why || "").trim(),
      todo: String(payload.todo || "").trim(),
      trigger: {
        runAt: formatDateToLocalRfc3339(now),
        cronExpression: "* * * * *",
        endAt: formatDateToLocalRfc3339(endAt),
      },
    });
    const taskId = String(created.taskId || "").trim();
    if (taskId) {
      try { await transport.request("task.dispatchNow", { taskId }); } catch { /* best-effort */ }
    }
    supervisionDialogOpen.value = false;
  } catch (error) {
    supervisionErrorText.value = String(error || "目标任务保存失败");
  } finally {
    supervisionSaving.value = false;
  }
}

function applyModelPayload(payload: SidebarModelPayload) {
  selectedChatModelId.value = String(payload.selectedChatModelId || "").trim();
  chatModelOptions.value = Array.isArray(payload.chatModelOptions) ? payload.chatModelOptions : [];
}

async function selectChatModel(apiConfigId: string) {
  const nextId = String(apiConfigId || "").trim();
  if (!nextId || !activeConversationId.value || nextId === selectedChatModelId.value) return;
  const previousId = selectedChatModelId.value;
  selectedChatModelId.value = nextId;
  try {
    const result = await transport.request<SidebarModelPayload>("model.select", {
      conversationId: activeConversationId.value,
      apiConfigId: nextId,
    });
    applyModelPayload(result);
  } catch (error) {
    selectedChatModelId.value = previousId;
    transport.errorText.value = String(error || "切换模型失败");
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
    transport.errorText.value = String(error || "切换权限失败");
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
    compactionErrorText.value = String(error || "加载压缩预览失败");
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
    compactionErrorText.value = String(error || "压缩失败");
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
    transport.errorText.value = String(error || "读取剪贴板图片失败");
  }
}

function removeClipboardImage(index: number) {
  if (index < 0 || index >= clipboardImages.value.length) return;
  clipboardImages.value.splice(index, 1);
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
    transport.errorText.value = String(error || "发送失败");
  }
}

async function stop() {
  if (!activeConversationId.value) return;
  await transport.request("chat.stop", {
    conversationId: activeConversationId.value,
    partialAssistantText: streamingText.value,
    partialReasoningStandard: streamingReasoningStandard.value,
    partialReasoningInline: streamingReasoningInline.value,
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
  if (!activeConversationId.value || compacting.value) return;
  const target = resolveRewindTargetUserMessage(payload.turnId);
  if (!target?.targetUserMessageId) {
    transport.errorText.value = "撤回失败：未找到可撤回的用户消息";
    return;
  }
  const mode = await requestRecallMode(target.keepCount);
  if (mode === "cancel") return;
  if (busy.value) await stop();
  clearStreamingState();
  try {
    const result = await transport.request<RewindConversationResult>("conversation.rewind", {
      conversationId: activeConversationId.value,
      agentId: activeAgentId.value,
      messageId: target.targetUserMessageId,
      undoApplyPatch: mode === "with_patch",
    });
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
    transport.errorText.value = String(error || "撤回失败");
  }
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
    transport.errorText.value = String(error || "确认计划失败");
  }
}

async function readPlanFileContent(input: { conversationId: string; path: string }): Promise<string> {
  const result = await transport.request<{ content?: string }>("conversation.plan.readFile", {
    conversationId: input.conversationId,
    path: input.path,
  });
  return String(result.content || "");
}

function hasBackupRecordIdInMessages(fromIndex: number): boolean {
  for (const message of messages.value.slice(fromIndex)) {
    const events = (message as any).tool_call as any[] | undefined;
    if (!Array.isArray(events)) continue;
    for (const event of events) {
      if (event?.role !== "tool") continue;
      const content = String(event?.content || "").trim();
      if (!content) continue;
      try {
        const parsed = JSON.parse(content);
        if (typeof parsed?.backupRecordId === "string" && parsed.backupRecordId.trim()) {
          return true;
        }
      } catch { /* ignore */ }
    }
  }
  return false;
}

function requestRecallMode(keepCount: number): Promise<"message_only" | "with_patch" | "cancel"> {
  rewindConfirmCanUndoPatch.value = hasBackupRecordIdInMessages(keepCount);
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
  } catch {
    workspaceDraftChoices.value = [];
  }
}

function openWorkspacePicker() {
  refreshWorkspaceList();
  workspacePickerOpen.value = true;
}

function closeWorkspacePicker() {
  if (workspacePickerSaving.value) return;
  workspacePickerOpen.value = false;
}

function cloneWorkspaceDraft(items: ChatWorkspaceChoice[]): ChatWorkspaceChoice[] {
  return (items || []).map((item) => ({
    id: String(item.id || "").trim(),
    name: String(item.name || "").trim(),
    path: String(item.path || "").trim(),
    level: item.level,
    access: item.access,
  }));
}

function setWorkspaceAsMain(workspaceId: string) {
  const draft = cloneWorkspaceDraft(workspaceDraftChoices.value).map((item): ChatWorkspaceChoice => {
    if (item.level === "system") return item;
    if (item.id === workspaceId) return { ...item, level: "main", access: item.access || "approval" };
    if (item.level === "main") return { ...item, level: "secondary" };
    return item;
  });
  workspaceDraftChoices.value = draft;
}

function setWorkspaceAccess(workspaceId: string, access: ChatWorkspaceChoice["access"]) {
  const draft = cloneWorkspaceDraft(workspaceDraftChoices.value);
  const target = draft.find((item) => item.id === workspaceId);
  if (!target || target.level === "system") return;
  target.access = access;
  workspaceDraftChoices.value = draft;
}

function setWorkspaceAutonomousMode(enabled: boolean) {
  workspaceDraftAutonomousMode.value = Boolean(enabled);
}

function removeWorkspace(workspaceId: string) {
  const current = cloneWorkspaceDraft(workspaceDraftChoices.value);
  const removing = current.find((item) => item.id === workspaceId);
  const draft = current.filter((item) => item.id !== workspaceId || item.level === "system");
  if (removing?.level === "main") {
    const promoteTarget = draft.find((item) => item.level === "secondary");
    if (promoteTarget) {
      draft.forEach((item) => {
        if (item.level === "system") return;
        if (item.id === promoteTarget.id) item.level = "main";
        else if (item.level === "main") item.level = "secondary";
      });
    }
  }
  workspaceDraftChoices.value = draft;
}

async function openWorkspaceDir(workspaceId: string) {
  const target = workspaceDraftChoices.value.find((item) => item.id === workspaceId);
  if (!target?.path) return;
  try {
    await transport.request("workspace.openDir", { workspacePath: target.path });
  } catch { /* ignore */ }
}

async function saveWorkspacePicker() {
  if (workspacePickerSaving.value || !activeConversationId.value) return;
  workspacePickerSaving.value = true;
  try {
    const draft = cloneWorkspaceDraft(workspaceDraftChoices.value);
    await transport.request("workspace.layout.save", {
      conversationId: activeConversationId.value,
      workspaces: draft
        .filter((item) => item.level !== "system")
        .map((item) => ({
          id: item.id,
          name: item.name,
          path: item.path,
          level: item.level,
          access: item.access,
          builtIn: false,
        })),
      autonomousMode: workspaceDraftAutonomousMode.value,
    });
    workspacePickerOpen.value = false;
    await refreshWorkspacePermission();
    await refreshWorkspaceList();
  } catch (error) {
    transport.errorText.value = String(error || "保存工作区设置失败");
  } finally {
    workspacePickerSaving.value = false;
  }
}

function appendMessages(next: unknown) {
  const payload = next as { conversationId?: string; messages?: ChatMessage[]; message?: ChatMessage };
  if (payload.conversationId && payload.conversationId !== activeConversationId.value) return;
  const incoming = payload.messages || (payload.message ? [payload.message] : []);
  if (!incoming.length) return;
  const existingIds = new Set(messages.value.map((item) => item.id));
  messages.value = [...messages.value, ...incoming.filter((item) => !existingIds.has(item.id))];
}

function registerNotifications() {
  transport.onNotification("conversation.overviewUpdated", (payload) => {
    const value = payload as { unarchivedConversations?: ConversationSummary[] };
    if (Array.isArray(value.unarchivedConversations)) {
      conversations.value = value.unarchivedConversations;
      clearCompletedRuntimeStateForConversation(activeConversationId.value);
    }
  });
  transport.onNotification("ideContext.updated", () => {
    void refreshIdeContextGroups();
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
    if (kind === "tool_status" && value.event) {
      applyAssistantToolStatusEvent(value.event);
      return;
    }
    if (!delta) return;
    if (kind === "reasoning_standard") streamingReasoningStandard.value += delta;
    else if (kind === "reasoning_inline") streamingReasoningInline.value += delta;
    else streamingText.value += delta;
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
    transport.errorText.value = "PAI 未运行";
    return;
  }
  await transport.connect(config);
  if (transport.connected.value) {
    await refreshList();
    await refreshIdeContextGroups();
  }
}

function clearDiscoveryRefreshTimer() {
  if (discoveryRefreshTimer === null) return;
  window.clearTimeout(discoveryRefreshTimer);
  discoveryRefreshTimer = null;
}

function refreshDiscovery() {
  clearDiscoveryRefreshTimer();
  transport.errorText.value = "";
  transport.connecting.value = true;
  window.parent.postMessage({ type: "pai-refresh-discovery" }, "*");
  discoveryRefreshTimer = window.setTimeout(() => {
    discoveryRefreshTimer = null;
    if (transport.connected.value) return;
    transport.connecting.value = false;
    transport.errorText.value = "PAI 未运行";
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
      await refreshList();
      await refreshIdeContextGroups();
    });
    else {
      transport.connecting.value = false;
      transport.errorText.value = "PAI 未运行";
    }
  }
}

onMounted(() => {
  registerNotifications();
  transport.onAuthRefreshNeeded(() => {
    refreshDiscovery();
  });
  window.addEventListener("message", handleWindowMessage);
  window.addEventListener("paste", handleWindowPaste);
  void bootstrap();
});

onBeforeUnmount(() => {
  clearDiscoveryRefreshTimer();
  window.removeEventListener("message", handleWindowMessage);
  window.removeEventListener("paste", handleWindowPaste);
});
</script>

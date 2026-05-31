<template>
  <ChatView
    ref="chatViewRef"
    :user-alias="userAlias"
    :persona-name="assistantName"
    :user-avatar-url="userAvatarUrl"
    :assistant-avatar-url="assistantAvatarUrl"
    :persona-name-map="personaNameMap"
    :persona-avatar-url-map="personaAvatarUrlMap"
    :mention-entries="sidebarMentionEntries"
    :selected-mentions="[]"
    latest-user-text=""
    :latest-user-images="[]"
    latest-assistant-text=""
    latest-reasoning-standard-text=""
    latest-reasoning-inline-text=""
    :frontend-round-phase="chatFrontendRoundPhase"
    :tool-status-text="toolStatusText"
    :tool-status-state="toolStatusState"
    :stream-tool-calls="streamToolCalls"
    chat-error-text=""
    :clipboard-images="clipboardImages"
    :queued-attachment-notices="[]"
    :chat-input="input"
    :instruction-presets="[]"
    chat-input-placeholder="输入消息"
    :can-record="false"
    :recording="false"
    :recording-ms="0"
    :transcribing="false"
    record-hotkey=""
    :selected-chat-model-id="selectedChatModelId"
    :tool-review-refresh-tick="0"
    :terminal-approvals="terminalApprovals"
    :terminal-approval-resolving="terminalApprovalResolving"
    :chat-model-options="chatModelOptions"
    :workspace-access="workspaceAccess"
    :plan-mode-enabled="planModeEnabled"
    :chat-usage-percent="chatUsagePercent"
    trim-tip=""
    :media-drag-active="false"
    :chatting="busy"
    :trimming="false"
    :compacting-conversation="false"
    :conversation-busy="false"
    :frozen="false"
    :message-blocks="visibleMessageBlocks"
    :has-more-history="hasPrevBlock"
    :loading-older-history="false"
    :latest-own-message-align-request="0"
    :conversation-scroll-to-bottom-request="scrollToBottomRequest"
    :current-workspace-name="currentWorkspaceName"
    current-workspace-root-path=""
    :workspaces="[]"
    :current-department-id="currentDepartmentId"
    :active-conversation-id="activeConversationId"
    :active-agent-id="activeAgentId"
    :current-todos="props.currentTodos"
    :supervision-active="false"
    supervision-title=""
    :supervision-dialog-open="false"
    :supervision-task-saving="false"
    supervision-task-error=""
    :active-supervision-task="null"
    :recent-supervision-task-history="[]"
    :unarchived-conversation-items="[]"
    :conversation-items="[]"
    :create-conversation-department-options="createConversationDepartmentOptions"
    :delegate-department-ids="delegateDepartmentIds"
    :default-create-conversation-department-id="defaultCreateConversationDepartmentId"
    :ide-context-groups="ideContextGroups"
    :attached-ide-context-references="[]"
    :current-theme="vscodeTheme"
    :detached-chat-window="true"
    :sidebar-mode="true"
    :hide-workspace-button="hideWorkspaceButton"
    :read-plan-file-content="readPlanFileContent"
    :side-conversation-list-visible="false"
    :initial-tool-review-panel-open="false"
    conversation-list-tab="local"
    chat-left-panel-mode="local"
    chat-right-panel-mode="reader"
    @update:chat-input="$emit('update:input', $event)"
    @send-chat="$emit('send', $event)"
    @stop-chat="$emit('stop')"
    @load-older-history="$emit('loadPrevBlock')"
    @clear-chat-error="noop"
    @reached-bottom="noop"
    @jump-to-conversation-bottom="noop"
    @add-mention="noop"
    @remove-mention="noop"
    @side-conversation-list-visible-change="noop"
    @tool-review-panel-open-change="noop"
    @side-panel-widths-change="noop"
    @side-panel-widths-commit="noop"
    @update:conversation-list-tab="noop"
    @update:chat-left-panel-mode="noop"
    @update:chat-right-panel-mode="noop"
    @remove-clipboard-image="$emit('removeClipboardImage', $event)"
    @remove-queued-attachment-notice="noop"
    @start-recording="noop"
    @stop-recording="noop"
    @pick-attachments="noop"
    @update:selected-chat-model-id="$emit('update:selectedChatModelId', $event)"
    @update-workspace-access="$emit('updateWorkspaceAccess', $event)"
    @update:plan-mode-enabled="noop"
    @trim-conversation="noop"
    @recall-turn="$emit('recallTurn', $event)"
    @regenerate-turn="noop"
    @confirm-plan="$emit('confirmPlan', $event)"
    @lock-workspace="$emit('lockWorkspace')"
    @open-code-review="$emit('openCodeReview')"
    @open-supervision-task="$emit('openSupervisionTask')"
    @detach-conversation="noop"
    @close-supervision-task="noop"
    @save-supervision-task="$emit('saveSupervisionTask', $event)"
    @switch-conversation="noop"
    @rename-conversation="noop"
    @toggle-pin-conversation="noop"
    @archive-conversation="noop"
    @delete-conversation="noop"
    @create-conversation="noop"
    @refresh-tool-review-message="noop"
    @attach-tool-review-report="noop"
    @selection-action-copy="noop"
    @selection-action-copy-error="noop"
    @selection-action-branch="$emit('selectionActionBranch', $event)"
    @selection-action-forward="noop"
    @selection-action-delegate="$emit('selectionActionDelegate', $event)"
    @selection-action-share="noop"
    @approve-terminal-approval="$emit('approveTerminalApproval', $event)"
    @deny-terminal-approval="$emit('denyTerminalApproval', $event)"
    @open-sidebar-file-reference="openSidebarFileReference"
  />
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from "vue";
import type { ApiConfigItem, ChatMentionEntry, ChatMessage, ChatTodoItem, IdeContextWorkspaceGroup } from "../../../types/app";
import ChatView from "../../chat/views/ChatView.vue";
import { useChatMessageBlocks } from "../../chat/composables/use-chat-turns";
import type { TerminalApprovalConversationItem } from "../../shell/composables/use-terminal-approval";

type VsCodeApi = { postMessage: (message: unknown) => void };

let cachedVsCodeApi: VsCodeApi | null | undefined;

function getVsCodeApi(): VsCodeApi | null {
  if (cachedVsCodeApi !== undefined) return cachedVsCodeApi;
  const bridgeWindow = window as Window & { acquireVsCodeApi?: () => VsCodeApi };
  try {
    cachedVsCodeApi = typeof bridgeWindow.acquireVsCodeApi === "function" ? bridgeWindow.acquireVsCodeApi() : null;
  } catch {
    cachedVsCodeApi = null;
  }
  return cachedVsCodeApi;
}

const props = defineProps<{
  activeConversationId: string;
  activeAgentId: string;
  persona: {
    userAlias?: string;
    userAvatarUrl?: string;
    assistantName?: string;
    assistantAvatarUrl?: string;
    personaNameMap?: Record<string, string>;
    personaAvatarUrlMap?: Record<string, string>;
  };
  selectedChatModelId: string;
  chatModelOptions: ApiConfigItem[];
  workspaceAccess: "read_only" | "approval" | "full_access" | "";
  planModeEnabled: boolean;
  input: string;
  messages: ChatMessage[];
  clipboardImages: Array<{ mime: string; bytesBase64: string }>;
  streamingText: string;
  streamingReasoningStandard: string;
  streamingReasoningInline: string;
  toolStatusText: string;
  toolStatusState: "running" | "done" | "failed" | "";
  streamToolCalls: Array<{ toolCallId?: string; name: string; argsText: string; status?: "doing" | "done" }>;
  busy: boolean;
  runtimeState?: string;
  hasPrevBlock: boolean;
  createConversationDepartmentOptions: Array<{ id: string; name: string; ownerAgentId?: string; ownerName: string; providerName?: string; modelName?: string; childDepartmentIds?: string[] }>;
  delegateDepartmentIds: string[];
  defaultCreateConversationDepartmentId: string;
  currentDepartmentId: string;
  currentWorkspaceName: string;
  currentTodos: ChatTodoItem[];
  hideWorkspaceButton?: boolean;
  terminalApprovals: TerminalApprovalConversationItem[];
  terminalApprovalResolving: boolean;
  ideContextGroups: IdeContextWorkspaceGroup[];
  readPlanFileContent: (input: { conversationId: string; path: string }) => Promise<string>;
}>();

defineEmits<{
  "update:input": [value: string];
  send: [payload?: { extraTextBlocks?: string[] }];
  stop: [];
  removeClipboardImage: [index: number];
  loadPrevBlock: [];
  "update:selectedChatModelId": [value: string];
  updateWorkspaceAccess: [value: "read_only" | "approval" | "full_access"];
  recallTurn: [payload: { turnId: string }];
  confirmPlan: [payload: { messageId: string }];
  lockWorkspace: [];
  openCodeReview: [];
  openSupervisionTask: [];
  saveSupervisionTask: [payload: { durationHours: number; goal: string; why: string; todo: string }];
  approveTerminalApproval: [requestId: string];
  denyTerminalApproval: [requestId: string];
  selectionActionBranch: [payload: { count: number; messageIds: string[] }];
  selectionActionDelegate: [payload: { count: number; messageIds: string[]; departmentId: string; presetId: string; background: string; question: string; focus: string }];
}>();

const allMessages = shallowRef<ChatMessage[]>([]);
const activeChatApiConfig = computed<ApiConfigItem | null>(
  () => props.chatModelOptions.find((item) => item.id === props.selectedChatModelId) || null,
);
const userAlias = computed(() => String(props.persona?.userAlias || "我").trim() || "我");
const userAvatarUrl = computed(() => String(props.persona?.userAvatarUrl || "").trim());
const assistantName = computed(() => String(props.persona?.assistantName || "PAI").trim() || "PAI");
const assistantAvatarUrl = computed(() => String(props.persona?.assistantAvatarUrl || "").trim());
const personaNameMap = computed<Record<string, string>>(() => ({
  "user-persona": userAlias.value,
  ...(props.persona?.personaNameMap || {}),
  ...(props.activeAgentId ? { [props.activeAgentId]: assistantName.value } : {}),
}));
const personaAvatarUrlMap = computed<Record<string, string>>(() => {
  const next = { ...(props.persona?.personaAvatarUrlMap || {}) };
  if (props.activeAgentId && assistantAvatarUrl.value) next[props.activeAgentId] = assistantAvatarUrl.value;
  return next;
});
const sidebarMentionEntries = computed<ChatMentionEntry[]>(() => {
  const nameMap = personaNameMap.value;
  const avatarMap = personaAvatarUrlMap.value;
  return Object.entries(nameMap)
    .filter(([agentId]) => agentId !== "user-persona")
    .map(([agentId, name]) => {
      const agentName = String(name || agentId).trim() || agentId;
      return {
        agentId,
        agentName,
        avatarUrl: String(avatarMap[agentId] || "").trim() || undefined,
        departmentId: agentId,
        departmentName: agentName,
        departmentNames: [agentName],
        isFrontSpeaking: agentId === props.activeAgentId,
        hasBackgroundTask: false,
        mentionable: true,
      };
    });
});
const vscodeTheme = ref(resolveVsCodeTheme());
const scrollToBottomRequest = ref(0);
const streamingDraftCreatedAt = ref("");
const streamingDraftStartedAtMs = ref(0);
let lastSeenOwnMessageId = "";
const chatFrontendRoundPhase = computed<"idle" | "waiting" | "queued" | "streaming">(() => {
  if (props.busy) return "streaming";
  const state = String(props.runtimeState || "").trim();
  if (state === "assistant_streaming" || state === "organizing_context") return "streaming";
  return "idle";
});

function resolveVsCodeTheme(): "dark" | "corporate" {
  if (document.body.classList.contains("vscode-dark") || document.body.classList.contains("vscode-high-contrast")) {
    return "dark";
  }
  return "corporate";
}

function latestOwnMessageId(messages: ChatMessage[]): string {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (String(message?.role || "").trim() !== "user") continue;
    const id = String(message?.id || "").trim();
    if (id) return id;
  }
  return "";
}

let themeObserver: MutationObserver | null = null;

onMounted(() => {
  themeObserver = new MutationObserver(() => {
    vscodeTheme.value = resolveVsCodeTheme();
  });
  themeObserver.observe(document.body, { attributes: true, attributeFilter: ["class"] });
});

onBeforeUnmount(() => {
  themeObserver?.disconnect();
  themeObserver = null;
});

watch(
  () => [
    props.busy,
    props.messages,
    props.streamingText,
    props.streamingReasoningStandard,
    props.streamingReasoningInline,
    props.toolStatusText,
    props.toolStatusState,
    props.streamToolCalls,
  ] as const,
  () => {
    const next = [...props.messages];
    const text = String(props.streamingText || "");
    const reasoningStandard = String(props.streamingReasoningStandard || "");
    const reasoningInline = String(props.streamingReasoningInline || "");
    const toolStatusText = String(props.toolStatusText || "");
    const streamToolCalls = Array.isArray(props.streamToolCalls)
      ? props.streamToolCalls.map((item) => ({ ...item }))
      : [];
    if (
      props.busy
      || text.trim()
      || reasoningStandard.trim()
      || reasoningInline.trim()
      || toolStatusText.trim()
      || props.toolStatusState
      || streamToolCalls.length > 0
    ) {
      if (!streamingDraftCreatedAt.value) {
        streamingDraftCreatedAt.value = new Date().toISOString();
        streamingDraftStartedAtMs.value = Date.now();
      }
      const hasStreamingContent = !!(
        text.trim()
        || reasoningStandard.trim()
        || reasoningInline.trim()
        || toolStatusText.trim()
        || props.toolStatusState
        || streamToolCalls.length > 0
      );
      next.push({
        id: `sidebar-stream-${props.activeConversationId || "conversation"}`,
        role: "assistant",
        createdAt: streamingDraftCreatedAt.value,
        speakerAgentId: props.activeAgentId || undefined,
        parts: [{ type: "text", text }],
        extraTextBlocks: [],
        providerMeta: {
          _streaming: true,
          _streamSegments: [text],
          reasoningStandard,
          reasoningInline,
          _toolStatusText: toolStatusText,
          _toolStatusState: props.toolStatusState,
          _streamToolCalls: streamToolCalls,
          _preStreamingStatusText: hasStreamingContent ? "" : "等待回复",
          _frontendDispatchElapsedMs: Date.now() - streamingDraftStartedAtMs.value,
        },
        toolCall: [],
      });
    } else {
      streamingDraftCreatedAt.value = "";
      streamingDraftStartedAtMs.value = 0;
    }
    allMessages.value = next;
  },
  { immediate: true, deep: true },
);

watch(
  () => props.activeConversationId,
  () => {
    streamingDraftCreatedAt.value = "";
    streamingDraftStartedAtMs.value = 0;
    lastSeenOwnMessageId = latestOwnMessageId(Array.isArray(props.messages) ? props.messages : []);
  },
  { immediate: true },
);

watch(
  () => props.messages,
  (messages) => {
    const nextOwnMessageId = latestOwnMessageId(Array.isArray(messages) ? messages : []);
    if (!nextOwnMessageId) {
      lastSeenOwnMessageId = "";
      return;
    }
    if (!lastSeenOwnMessageId) {
      lastSeenOwnMessageId = nextOwnMessageId;
      return;
    }
    if (nextOwnMessageId === lastSeenOwnMessageId) return;
    lastSeenOwnMessageId = nextOwnMessageId;
    scrollToBottomRequest.value += 1;
  },
  { flush: "post" },
);

const { visibleMessageBlocks, chatUsagePercent } = useChatMessageBlocks({
  allMessages,
  activeChatApiConfig,
  perfDebug: false,
  perfNow: () => performance.now(),
});

const chatViewRef = ref<{ exitMessageSelectionMode: () => void } | null>(null);

function exitMessageSelectionMode() {
  chatViewRef.value?.exitMessageSelectionMode();
}

defineExpose({ exitMessageSelectionMode, chatUsagePercent });

function noop() {}

function openSidebarFileReference(href: string) {
  const normalizedHref = String(href || "").trim();
  if (!normalizedHref) return;
  const vscodeApi = getVsCodeApi();
  if (vscodeApi) {
    vscodeApi.postMessage({ type: "pai-open-file", href: normalizedHref });
    return;
  }
  window.parent.postMessage({ type: "pai-open-file", href: normalizedHref }, "*");
}
</script>

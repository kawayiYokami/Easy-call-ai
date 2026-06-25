<template>
  <aside v-bind="rootAttrs" class="w-full flex h-full min-h-0 flex-col bg-base-200">
    <div role="tablist" class="tabs tabs-border px-2 pb-2">
      <button type="button" role="tab" class="tab" :class="{ 'tab-active': activeTab === 'delegates' }" @click="activeTab = 'delegates'">{{ t("chat.toolReview.delegatesTab") }}</button>
      <button type="button" role="tab" class="tab" :class="{ 'tab-active': activeTab === 'tasks' }" @click="activeTab = 'tasks'">{{ t("chat.toolReview.tasksTab") }}</button>
      <button type="button" role="tab" class="tab" :class="{ 'tab-active': activeTab === 'tools' }" @click="activeTab = 'tools'">{{ t("chat.toolReview.toolsTab") }}</button>
    </div>

    <div ref="contentScroller" class="ecall-chat-scroll-container flex min-h-0 flex-1 flex-col overflow-y-auto p-1">
      <div v-if="errorText" class="mx-4 my-4 rounded-box border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
        {{ errorText }}
      </div>

      <template v-if="activeTab === 'tools' && currentBatch">
        <div class="flex min-h-full flex-col">
          <div class="sticky top-0 z-30 bg-base-200 px-4">
            <button
              type="button"
              class="btn btn-sm w-full gap-1.5 bg-base-100 hover:bg-base-100"
              :disabled="batchReviewing || currentBatchUnreviewedCount <= 0"
              @click="emit('reviewBatch', currentBatch.batchKey)"
            >
              <span v-if="batchReviewing" class="loading loading-spinner loading-xs"></span>
              <CircleCheckBig v-else class="size-4" aria-hidden="true" />
              <span>{{ t("chat.toolReview.evaluateBatchWithCount", { count: currentBatchUnreviewedCount }) }}</span>
            </button>
          </div>
          <div class="flex min-h-0 flex-1 flex-col py-2">
            <CollapsibleGroup
              v-for="group in reviewGroups"
              :key="group.key"
              :title="group.title"
              :count="group.items.length"
              :model-value="isToolAssessmentSectionCollapsed(group.key)"
              @update:model-value="toggleToolAssessmentSection(group.key)"
              @collapse-all="collapseAllToolAssessmentSections"
            >
              <div v-if="!isToolAssessmentSectionCollapsed(group.key)">
                <ToolAssessmentCard
                  v-for="item in group.items"
                  :key="`${group.key}:${item.callId}`"
                  :item="item"
                  :detail="detailMap[item.callId]"
                  :loading="detailLoadingCallId === item.callId"
                  :is-dark="markdownIsDark"
                  @load-detail="emit('loadItemDetail', $event)"
                />
              </div>
            </CollapsibleGroup>
          </div>
          <div v-if="props.batches.length > 1" class="px-4 py-3">
            <div class="join flex justify-center">
              <button
                type="button"
                class="join-item btn btn-sm bg-base-100 hover:bg-base-100"
                :disabled="!previousBatch"
                @click="previousBatch && emit('selectBatch', previousBatch.batchKey)"
              >
                «
              </button>
              <button
                type="button"
                class="join-item btn btn-sm bg-base-100 hover:bg-base-100"
                @click.prevent
              >
                {{ t("chat.toolReview.pageLabel", { current: currentBatchIndex + 1, total: props.batches.length }) }}
              </button>
              <button
                type="button"
                class="join-item btn btn-sm bg-base-100 hover:bg-base-100"
                :disabled="!nextBatch"
                @click="nextBatch && emit('selectBatch', nextBatch.batchKey)"
              >
                »
              </button>
            </div>
          </div>
        </div>
      </template>

      <div v-else-if="activeTab === 'tools'" class="flex flex-1 items-center justify-center px-4 py-8 text-sm text-base-content/65">
        {{ t("chat.toolReview.empty") }}
      </div>

      <template v-else-if="activeTab === 'delegates'">
        <div v-if="delegateStatusesErrorText" class="mx-4 my-4 rounded-box border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
          {{ delegateStatusesErrorText }}
        </div>
        <div v-else-if="delegateStatuses.length === 0" class="flex min-h-0 flex-1 items-center justify-center px-4 py-8 text-sm text-base-content/65">
          {{ t("chat.toolReview.delegateEmpty") }}
        </div>
        <CollapsibleGroup
          v-for="section in delegateStatusSections"
          :key="section.key"
          :title="section.title"
          :count="section.items.length"
          :model-value="isDelegateSectionCollapsed(section.key)"
          @update:model-value="toggleDelegateSection(section.key)"
          @collapse-all="collapseAllDelegateSections"
        >
          <div v-if="!isDelegateSectionCollapsed(section.key)">
            <section v-for="delegate in section.items" :key="delegate.delegateId">
              <DelegateCard
                :title="delegate.title || delegate.delegateId"
                :running="isDelegateRunning(delegate)"
                :elapsed-ms="delegate.elapsedMs"
                :request-count="delegate.requestCount"
                :token-count="delegate.tokenCount"
                :last-tool-name="delegate.lastToolName"
                :show-result="canShowDelegateResult(delegate)"
                :avatar-url="personaAvatarUrlMap[delegate.targetAgentId || ''] || ''"
                @abort="emit('abortDelegate', delegate)"
                @open-detail="openDelegateResult(delegate)"
              />
            </section>
          </div>
        </CollapsibleGroup>
      </template>

      <template v-else-if="activeTab === 'tasks'">
        <div v-if="taskErrorText" class="mx-4 my-4 rounded-box border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
          {{ taskErrorText }}
        </div>
        <div v-else-if="taskLoading && taskSections.length === 0" class="flex min-h-0 flex-1 items-center justify-center px-4 py-8">
          <span class="loading loading-spinner loading-sm text-base-content/45"></span>
        </div>
        <div v-else-if="taskSections.length === 0" class="flex min-h-0 flex-1 items-center justify-center px-4 py-8 text-sm text-base-content/65">
          {{ t("chat.toolReview.taskEmpty") }}
        </div>
        <CollapsibleGroup
          v-for="section in taskSections"
          :key="section.key"
          :title="section.title"
          :count="section.items.length"
          :model-value="isTaskSectionCollapsed(section.key)"
          @update:model-value="toggleTaskSection(section.key)"
          @collapse-all="collapseAllTaskSections"
        >
          <div v-if="!isTaskSectionCollapsed(section.key)">
            <TaskListItem
              v-for="task in section.items"
              :key="task.taskId"
              :label="taskListTitle(task)"
              :description="taskListTodo(task)"
              :title="taskListTitle(task)"
              :time-label="taskListTime(task)"
              :disabled="!canEditTaskInSidebar"
              @click="openTaskEditor(task)"
            />
          </div>
        </CollapsibleGroup>
      </template>
    </div>
    <FloatingScrollbar :target="contentScroller" />
  </aside>

  <dialog class="modal" :class="{ 'modal-open': delegateResultDialogOpen }">
    <div class="modal-box max-h-[80vh] max-w-2xl overflow-y-auto">
      <div class="mb-3 flex items-center justify-between gap-3">
        <div class="min-w-0 truncate text-sm font-semibold text-base-content">{{ delegateResultTitle }}</div>
        <button type="button" class="btn btn-ghost btn-sm" @click="delegateResultDialogOpen = false">×</button>
      </div>
      <div v-if="delegateResultLoading" class="flex items-center gap-2 text-sm text-base-content/65">
        <span class="loading loading-spinner loading-sm"></span>
        加载中
      </div>
      <div v-else-if="delegateResultText" class="tool-review-report-markdown assistant-markdown text-sm leading-7 text-base-content/80">
        <AppMarkdownRenderer :text="delegateResultText" :is-dark="markdownIsDark" />
      </div>
      <div v-else class="whitespace-pre-wrap wrap-break-word text-sm leading-7 text-base-content/80">
        没有可显示的结果
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button type="button" @click="delegateResultDialogOpen = false">close</button>
    </form>
  </dialog>

  <TaskCreateCard
    :open="taskEditorOpen"
    mode="edit"
    :conversation-id="activeConversationId"
    :task="taskEditorTask"
    :bridge-request="bridgeRequest"
    @close="closeTaskEditor"
    @created="handleTaskMutated"
    @updated="handleTaskMutated"
  />

</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, useAttrs } from "vue";
import { useI18n } from "vue-i18n";
import { CircleCheckBig } from "@lucide/vue";
import { invokeTauri } from "../../../services/tauri-api";
import type { ArchiveBlockPage, ChatMessage, ConversationDelegateStatusSummary, ShellWorkspace } from "../../../types/app";
import { toErrorMessage } from "../../../utils/error";
import { defaultWorkspaceNameFromPath, inferWorkspaceName, isLegacyGenericWorkspaceName, normalizeWorkspaceLevel } from "../../../utils/shell-workspaces";
import type { ToolReviewBatchSummary, ToolReviewItemDetail, ToolReviewItemSummary } from "../composables/use-chat-tool-review";
import { formatConversationListTime } from "../utils/conversation-time";
import { AppMarkdownRenderer, initKatex } from "../markdown";
import ToolAssessmentCard from "./ToolAssessmentCard.vue";
import DelegateCard from "./DelegateCard.vue";
import FloatingScrollbar from "../../shell/components/FloatingScrollbar.vue";
import CollapsibleGroup from "./CollapsibleGroup.vue";
import TaskListItem from "./TaskListItem.vue";
import TaskCreateCard from "./dialogs/TaskCreateCard.vue";
import type { TaskEntry } from "../../config/views/config-tabs/task-editor";

initKatex();

const props = defineProps<{
  batches: ToolReviewBatchSummary[];
  currentBatchKey: string;
  detailMap: Record<string, ToolReviewItemDetail>;
  detailLoadingCallId: string;
  reviewingCallId: string;
  batchReviewingKey: string;
  errorText: string;
  markdownIsDark: boolean;
  activeConversationId: string;
  currentWorkspaceName: string;
  currentWorkspaceRootPath: string;
  workspaces: ShellWorkspace[];
  currentDepartmentId: string;
  departmentOptions: Array<{ id: string; name: string; ownerName: string; providerName?: string; modelName?: string }>;
  delegateStatuses: ConversationDelegateStatusSummary[];
  delegateStatusesErrorText: string;
  personaAvatarUrlMap: Record<string, string>;
  bridgeRequest?: <T = unknown>(method: string, params?: Record<string, unknown>, timeoutMs?: number) => Promise<T>;
}>();

const emit = defineEmits<{
  (e: "selectBatch", batchKey: string): void;
  (e: "loadItemDetail", callId: string): void;
  (e: "reviewItem", callId: string): void;
  (e: "reviewBatch", batchKey: string): void;
  (e: "openDelegateDetail", status: ConversationDelegateStatusSummary): void;
  (e: "abortDelegate", status: ConversationDelegateStatusSummary): void;
}>();

const { t, locale } = useI18n();
const activeTab = ref<"tools" | "delegates" | "tasks">("delegates");
const contentScroller = ref<HTMLElement | null>(null);
const delegateResultDialogOpen = ref(false);
const delegateResultLoading = ref(false);
const delegateResultTitle = ref("");
const delegateResultText = ref("");
const rootAttrs = useAttrs();
const collapsedToolAssessmentSectionKeys = ref<Record<string, boolean>>({});
const collapsedDelegateSectionKeys = ref<Record<string, boolean>>({
  running: false,
  completed: true,
  interrupted: true,
  failed: true,
});
const collapsedTaskSectionKeys = ref<Record<string, boolean>>({
  active: false,
  completed: true,
  failed_completed: true,
});
const taskEntries = ref<TaskEntry[]>([]);
const taskLoading = ref(false);
const taskErrorText = ref("");
const taskEditorOpen = ref(false);
const taskEditorTask = ref<TaskEntry | null>(null);

type DelegateStatusSection = {
  key: string;
  title: string;
  items: ConversationDelegateStatusSummary[];
};

type TaskSectionKey = "active" | "completed" | "failed_completed";

type TaskSection = {
  key: TaskSectionKey;
  title: string;
  items: TaskEntry[];
  order: number;
};

const delegateStatusSections = computed<DelegateStatusSection[]>(() => {
  const sections: DelegateStatusSection[] = [
    { key: "running", title: "正在运行中", items: [] },
    { key: "completed", title: "已完成", items: [] },
    { key: "interrupted", title: "被中断", items: [] },
    { key: "failed", title: "已失败", items: [] },
  ];
  for (const delegate of props.delegateStatuses) {
    sections[delegateStatusSectionIndex(delegate)].items.push(delegate);
  }
  return sections.filter((section) => section.items.length > 0);
});

const canEditTaskInSidebar = computed(() => true);
const currentConversationTasks = computed(() => {
  const conversationId = String(props.activeConversationId || "").trim();
  if (!conversationId) return [];
  return taskEntries.value.filter((task) => String(task.conversationId || "").trim() === conversationId);
});
const taskSections = computed<TaskSection[]>(() => {
  const sections: TaskSection[] = [
    { key: "active", title: t("config.task.filters.active"), items: [], order: 0 },
    { key: "completed", title: t("config.task.completionStates.completed"), items: [], order: 1 },
    { key: "failed_completed", title: t("config.task.completionStates.failedCompleted"), items: [], order: 2 },
  ];
  for (const task of currentConversationTasks.value) {
    const section = sections.find((item) => item.key === normalizeTaskSectionKey(task.completionState));
    section?.items.push(task);
  }
  return sections
    .map((section) => ({
      ...section,
      items: section.items.slice().sort(compareTaskForSidebar),
    }))
    .filter((section) => section.items.length > 0)
    .sort((left, right) => left.order - right.order);
});
function delegateStatusSectionIndex(delegate: ConversationDelegateStatusSummary) {
  const status = String(delegate.status || "").trim();
  if (status === "failed") return 3;
  if ((status === "running" || status === "delivered") && delegate.active) return 0;
  if (status === "running" || status === "delivered") return 2;
  return 1;
}

function isToolAssessmentSectionCollapsed(key: string) {
  return !!collapsedToolAssessmentSectionKeys.value[key];
}

function toggleToolAssessmentSection(key: string) {
  collapsedToolAssessmentSectionKeys.value = {
    ...collapsedToolAssessmentSectionKeys.value,
    [key]: !collapsedToolAssessmentSectionKeys.value[key],
  };
}

function collapseAllToolAssessmentSections() {
  collapsedToolAssessmentSectionKeys.value = reviewGroups.value.reduce((next, section) => {
    next[section.key] = true;
    return next;
  }, { ...collapsedToolAssessmentSectionKeys.value } as Record<string, boolean>);
}

function isDelegateSectionCollapsed(key: string) {
  return !!collapsedDelegateSectionKeys.value[key];
}

function toggleDelegateSection(key: string) {
  collapsedDelegateSectionKeys.value = {
    ...collapsedDelegateSectionKeys.value,
    [key]: !collapsedDelegateSectionKeys.value[key],
  };
}

function collapseAllDelegateSections() {
  collapsedDelegateSectionKeys.value = delegateStatusSections.value.reduce((next, section) => {
    next[section.key] = true;
    return next;
  }, { ...collapsedDelegateSectionKeys.value } as Record<string, boolean>);
}

function normalizeTaskSectionKey(value: string): TaskSectionKey {
  return value === "completed" || value === "failed_completed" ? value : "active";
}

function isTaskSectionCollapsed(key: TaskSectionKey) {
  return !!collapsedTaskSectionKeys.value[key];
}

function toggleTaskSection(key: TaskSectionKey) {
  collapsedTaskSectionKeys.value = {
    ...collapsedTaskSectionKeys.value,
    [key]: !collapsedTaskSectionKeys.value[key],
  };
}

function collapseAllTaskSections() {
  collapsedTaskSectionKeys.value = taskSections.value.reduce((next, section) => {
    next[section.key] = true;
    return next;
  }, { ...collapsedTaskSectionKeys.value } as Record<string, boolean>);
}

function taskListTitle(task: TaskEntry) {
  return String(task.goal || "").trim() || t("config.task.noTodo");
}

function taskListTodo(task: TaskEntry) {
  return String(task.todo || "").trim() || t("config.task.noTodo");
}

function taskListTime(task: TaskEntry) {
  const raw = String(task.trigger?.next_run_at || task.trigger?.run_at || task.updatedAtLocal || "").trim();
  return raw ? formatConversationListTime(raw, locale.value) : "";
}

function compareTaskForSidebar(left: TaskEntry, right: TaskEntry) {
  const leftRaw = String(left.trigger?.next_run_at || left.trigger?.run_at || left.updatedAtLocal || "").trim();
  const rightRaw = String(right.trigger?.next_run_at || right.trigger?.run_at || right.updatedAtLocal || "").trim();
  const leftTime = leftRaw ? new Date(leftRaw).getTime() : Number.POSITIVE_INFINITY;
  const rightTime = rightRaw ? new Date(rightRaw).getTime() : Number.POSITIVE_INFINITY;
  if (leftTime !== rightTime) return leftTime - rightTime;
  return Number(left.orderIndex || 0) - Number(right.orderIndex || 0);
}

const currentBatchIndex = computed(() => {
  const currentKey = String(props.currentBatchKey || "").trim();
  if (!currentKey) return -1;
  return props.batches.findIndex((batch) => batch.batchKey === currentKey);
});

const currentBatch = computed(() => {
  const currentKey = String(props.currentBatchKey || "").trim();
  if (!currentKey) return null;
  return props.batches.find((batch) => batch.batchKey === currentKey) || null;
});

const previousBatch = computed(() => {
  const index = currentBatchIndex.value;
  if (index < 0) {
    return props.batches[props.batches.length - 1] || null;
  }
  if (index <= 0) return null;
  return props.batches[index - 1] || null;
});

const nextBatch = computed(() => {
  const index = currentBatchIndex.value;
  if (index < 0 || index >= props.batches.length - 1) return null;
  return props.batches[index + 1] || null;
});

const batchReviewing = computed(() =>
  !!currentBatch.value && props.batchReviewingKey === currentBatch.value.batchKey
);

async function openDelegateResult(status: import("../../../types/app").ConversationDelegateStatusSummary) {
  const conversationId = String(status?.conversationId || "").trim();
  if (!conversationId) return;
  delegateResultTitle.value = String(status?.title || status?.delegateId || "委托结果");
  delegateResultText.value = "";
  delegateResultDialogOpen.value = true;
  delegateResultLoading.value = true;
  try {
    const page = props.bridgeRequest
      ? await props.bridgeRequest<ArchiveBlockPage>("delegate.blockPage", { conversationId }, 10000)
      : await invokeTauri<ArchiveBlockPage>("get_delegate_conversation_block_page", {
          input: { conversationId },
        });
    delegateResultText.value = formatDelegateResultText(findLastAssistantText(Array.isArray(page?.messages) ? page.messages : []));
  } catch (error) {
    delegateResultText.value = `读取委托结果失败：${String(error)}`;
  } finally {
    delegateResultLoading.value = false;
  }
}

function findLastAssistantText(messages: ChatMessage[]) {
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index];
    if (message?.role !== "assistant") continue;
    const text = message.parts
      ?.filter((part) => part.type === "text")
      .map((part) => part.text)
      .join("\n")
      .trim();
    if (text) return text;
  }
  return "";
}

function formatDelegateResultText(text: string) {
  const trimmed = text.trim();
  if (!trimmed) return "";
  try {
    const parsed = JSON.parse(trimmed);
    return `\`\`\`json\n${JSON.stringify(parsed, null, 2)}\n\`\`\``;
  } catch {
    return text;
  }
}

type ToolReviewGroup = {
  key: string;
  title: string;
  firstOrderIndex: number;
  items: ToolReviewItemSummary[];
};

function isTerminalTool(toolName: string) {
  const normalized = String(toolName || "").trim();
  return normalized === "shell_exec" || normalized === "exec";
}

function isFileChangeTool(toolName: string) {
  const normalized = String(toolName || "").trim();
  return normalized === "apply_patch"
    || normalized === "write"
    || normalized === "delete"
    || normalized === "update"
    || normalized === "move";
}

const reviewGroups = computed<ToolReviewGroup[]>(() => {
  const terminalItems = [] as ToolReviewItemSummary[];
  const patchGroups = new Map<string, ToolReviewGroup>();
  const otherGroups = new Map<string, ToolReviewGroup>();
  for (const item of currentBatch.value?.items ?? []) {
    if (isTerminalTool(item.toolName)) {
      terminalItems.push(item);
      continue;
    }
    if (!isFileChangeTool(item.toolName)) {
      const toolName = String(item.toolName || "").trim() || t("chat.toolReview.otherGroup");
      const groupKey = `other:${toolName}`;
      const group = otherGroups.get(groupKey) || {
        key: groupKey,
        title: toolName,
        firstOrderIndex: Number(item.orderIndex || 0),
        items: [],
      };
      group.firstOrderIndex = Math.min(group.firstOrderIndex, Number(item.orderIndex || 0));
      group.items.push(item);
      otherGroups.set(groupKey, group);
      continue;
    }
    const paths = Array.isArray(item.affectedPaths) ? item.affectedPaths.filter(Boolean) : [];
    const key = paths.length === 1 ? paths[0] : "__multi_patch__";
    const title = paths.length === 1
      ? formatPatchGroupTitle(paths[0])
      : t("chat.toolReview.patchMultiFileGroup");
    const group = patchGroups.get(key) || {
      key: `patch:${key}`,
      title,
      firstOrderIndex: Number(item.orderIndex || 0),
      items: [],
    };
    group.firstOrderIndex = Math.min(group.firstOrderIndex, Number(item.orderIndex || 0));
    group.items.push(item);
    patchGroups.set(key, group);
  }
  const groups = [] as ToolReviewGroup[];
  if (terminalItems.length > 0) {
    groups.push({
      key: "terminal",
      title: t("chat.toolReview.terminalGroup"),
      firstOrderIndex: Math.min(...terminalItems.map((item) => Number(item.orderIndex || 0))),
      items: terminalItems.sort(sortByOrderIndex),
    });
  }
  groups.push(
    ...Array.from(patchGroups.values())
      .map((group) => ({ ...group, items: group.items.sort(sortByOrderIndex) }))
      .sort((a, b) => a.firstOrderIndex - b.firstOrderIndex)
  );
  groups.push(
    ...Array.from(otherGroups.values())
      .map((group) => ({ ...group, items: group.items.sort(sortByOrderIndex) }))
      .sort((a, b) => a.firstOrderIndex - b.firstOrderIndex)
  );
  return groups;
});

const currentBatchUnreviewedCount = computed(() =>
  currentBatch.value?.items.filter((item) => !item.hasReview).length ?? 0
);

function sortByOrderIndex(left: ToolReviewItemSummary, right: ToolReviewItemSummary) {
  return Number(left.orderIndex || 0) - Number(right.orderIndex || 0);
}

function formatPatchGroupTitle(path: string) {
  const normalized = String(path || "").replace(/\\/g, "/").trim();
  if (!normalized) return t("chat.toolReview.patchUnknownFileGroup");
  return compactPathByWorkspace(normalized);
}

function compactPathByWorkspace(path: string) {
  const normalizedPath = normalizePathForDisplay(path);
  const matches = workspacePathDisplayCandidates.value
    .map((candidate) => {
      const root = candidate.root;
      if (!root) return null;
      if (isSameNormalizedPath(normalizedPath, root)) {
        return { root, name: candidate.name, rest: "" };
      }
      if (!isPathUnderWorkspace(normalizedPath, root)) return null;
      return {
        root,
        name: candidate.name,
        rest: normalizedPath.slice(root.length + 1),
      };
    })
    .filter((item): item is { root: string; name: string; rest: string } => !!item)
    .sort((left, right) => right.root.length - left.root.length);
  const matched = matches[0];
  if (!matched) return normalizedPath;
  return matched.rest ? `${matched.name}/${matched.rest}` : matched.name;
}

const workspacePathDisplayCandidates = computed(() =>
  [currentWorkspaceCandidate.value, ...workspaceListCandidates.value]
    .filter((item): item is { root: string; name: string } => !!item)
    .sort((left, right) => right.root.length - left.root.length)
);

const currentWorkspaceCandidate = computed(() => {
  const root = normalizePathForDisplay(props.currentWorkspaceRootPath);
  if (!root) return null;
  const matchedWorkspace = (Array.isArray(props.workspaces) ? props.workspaces : []).find((workspace) =>
    isSameNormalizedPath(root, normalizePathForDisplay(workspace.path))
  );
  const currentName = String(props.currentWorkspaceName || "").trim();
  const matchedName = currentName || (matchedWorkspace ? workspaceDisplayName(matchedWorkspace, root, 0) : "");
  return {
    root,
    name: matchedName || defaultWorkspaceNameFromPath(root) || root,
  };
});

const workspaceListCandidates = computed(() =>
  (Array.isArray(props.workspaces) ? props.workspaces : [])
    .map((workspace, index) => {
      const root = normalizePathForDisplay(workspace.path);
      if (!root) return null;
      return {
        root,
        name: workspaceDisplayName(workspace, root, index),
      };
    })
    .filter((item): item is { root: string; name: string } => !!item)
);

function normalizePathForDisplay(path: string) {
  return String(path || "")
    .replace(/^\\\\\?\\/, "")
    .replace(/^\/\/\?\//, "")
    .replace(/\\/g, "/")
    .replace(/\/+$/, "")
    .trim();
}

function normalizePathForCompare(path: string) {
  return normalizePathForDisplay(path).toLowerCase();
}

function isSameNormalizedPath(path: string, root: string) {
  return normalizePathForCompare(path) === normalizePathForCompare(root);
}

function isPathUnderWorkspace(path: string, root: string) {
  const normalizedPath = normalizePathForCompare(path);
  const normalizedRoot = normalizePathForCompare(root);
  return normalizedPath.startsWith(`${normalizedRoot}/`);
}

function workspaceDisplayName(workspace: ShellWorkspace, root: string, index: number) {
  const level = normalizeWorkspaceLevel(String(workspace.level || ""));
  const rawName = String(workspace.name || "").trim();
  if (!isLegacyGenericWorkspaceName(level, rawName)) {
    return rawName;
  }
  return inferWorkspaceName(level, root, index) || defaultWorkspaceNameFromPath(root) || root;
}

async function requestTaskList() {
  if (props.bridgeRequest) return props.bridgeRequest<TaskEntry[]>("task.list", {});
  return invokeTauri<TaskEntry[]>("task_list_tasks");
}

async function loadConversationTasks() {
  taskLoading.value = true;
  taskErrorText.value = "";
  try {
    taskEntries.value = await requestTaskList();
  } catch (error) {
    taskErrorText.value = `${t("config.task.listLoadFailed")}: ${toErrorMessage(error)}`;
  } finally {
    taskLoading.value = false;
  }
}

function openTaskEditor(task: TaskEntry) {
  if (!canEditTaskInSidebar.value) return;
  if (!String(task.taskId || "").trim()) return;
  taskEditorTask.value = task;
  taskEditorOpen.value = true;
}

function closeTaskEditor() {
  taskEditorOpen.value = false;
  taskEditorTask.value = null;
}

function handleTaskRefreshEvent() {
  void loadConversationTasks();
}

function handleTaskMutated(task: TaskEntry) {
  taskEditorOpen.value = false;
  taskEditorTask.value = task;
  void loadConversationTasks();
}

function openDelegatesTab() {
  activeTab.value = "delegates";
}

defineExpose({
  openDelegatesTab,
});

function isDelegateRunning(delegate: ConversationDelegateStatusSummary) {
  const status = String(delegate.status || "").trim();
  return delegate.active && (status === "running" || status === "delivered");
}

function canShowDelegateResult(delegate: ConversationDelegateStatusSummary) {
  const status = String(delegate.status || "").trim();
  if (status === "running" || status === "delivered") return false;
  return true;
}

onMounted(() => {
  void loadConversationTasks();
  window.addEventListener("easy-call:task-created", handleTaskRefreshEvent);
  window.addEventListener("easy-call:task-updated", handleTaskRefreshEvent);
  window.addEventListener("easy-call:task-completed", handleTaskRefreshEvent);
  window.addEventListener("easy-call:task-deleted", handleTaskRefreshEvent);
});

onBeforeUnmount(() => {
  window.removeEventListener("easy-call:task-created", handleTaskRefreshEvent);
  window.removeEventListener("easy-call:task-updated", handleTaskRefreshEvent);
  window.removeEventListener("easy-call:task-completed", handleTaskRefreshEvent);
  window.removeEventListener("easy-call:task-deleted", handleTaskRefreshEvent);
});
</script>

<style scoped>

.assistant-markdown :deep(.ecall-markdown-content.prose) {
  --tw-prose-body: currentColor;
  --tw-prose-headings: currentColor;
  --tw-prose-lead: currentColor;
  --tw-prose-links: var(--color-base-content);
  --tw-prose-bold: currentColor;
  --tw-prose-counters: currentColor;
  --tw-prose-bullets: color-mix(in srgb, var(--color-base-content) 50%, transparent);
  --tw-prose-hr: color-mix(in srgb, var(--color-base-content) 15%, transparent);
  --tw-prose-quotes: currentColor;
  --tw-prose-quote-borders: color-mix(in srgb, var(--color-base-content) 20%, transparent);
  --tw-prose-captions: color-mix(in srgb, var(--color-base-content) 75%, transparent);
  --tw-prose-code: currentColor;
  --tw-prose-pre-code: currentColor;
  --tw-prose-pre-bg: var(--color-base-200);
  --tw-prose-th-borders: color-mix(in srgb, var(--color-base-content) 20%, transparent);
  --tw-prose-td-borders: color-mix(in srgb, var(--color-base-content) 15%, transparent);
}

.assistant-markdown :deep(.ecall-markdown-content) {
  --ms-font-sans: var(
    --app-font-family,
    system-ui,
    -apple-system,
    BlinkMacSystemFont,
    "Segoe UI",
    Roboto,
    "Helvetica Neue",
    Arial,
    sans-serif
  );
  --ms-text-body: 0.875rem;
  --ms-leading-body: 1.5;
  --ms-text-h1: 1.02rem;
  --ms-leading-h1: 1.5;
  --ms-text-h2: 0.98rem;
  --ms-leading-h2: 1.5;
  --ms-text-h3: 0.94rem;
  --ms-leading-h3: 1.5;
  --ms-text-h4: 0.9rem;
  --ms-text-h5: 0.875rem;
  --ms-text-h6: 0.875rem;
  --ms-flow-paragraph-y: 0.25rem;
  --ms-flow-list-y: 0.25rem;
  --ms-flow-list-item-y: 0.12rem;
  --ms-flow-list-indent: 1.05rem;
  --ms-flow-list-indent-mobile: 1.05rem;
  --ms-flow-blockquote-y: 0.25rem;
  --ms-flow-blockquote-indent: 0.68rem;
  min-width: 0;
  max-width: 100%;
  overflow-x: hidden;
  font-family: inherit;
  font-size: 0.875rem;
  line-height: 1.5;
}

.assistant-markdown :deep(.ecall-markdown-content .paragraph-node),
.assistant-markdown :deep(.ecall-markdown-content .heading-node),
.assistant-markdown :deep(.ecall-markdown-content .list-node),
.assistant-markdown :deep(.ecall-markdown-content .list-item),
.assistant-markdown :deep(.ecall-markdown-content .blockquote),
.assistant-markdown :deep(.ecall-markdown-content .link-node),
.assistant-markdown :deep(.ecall-markdown-content .strong-node),
.assistant-markdown :deep(.ecall-markdown-content .inline-code),
.assistant-markdown :deep(.ecall-markdown-content .table-node-wrapper),
.assistant-markdown :deep(.ecall-markdown-content .hr-node) {
  font-size: inherit;
  line-height: inherit;
}

.assistant-markdown :deep(.ecall-markdown-content.markdown-renderer) {
  content-visibility: visible !important;
  contain: none !important;
  contain-intrinsic-size: auto !important;
}

.assistant-markdown :deep(.ecall-markdown-content .markdown-renderer),
.assistant-markdown :deep(.ecall-markdown-content .node-slot),
.assistant-markdown :deep(.ecall-markdown-content .node-content),
.assistant-markdown :deep(.ecall-markdown-content .text-node) {
  font-size: inherit;
  line-height: inherit;
}

.assistant-markdown :deep(.ecall-markdown-content .code-block-container),
.assistant-markdown :deep(.ecall-markdown-content ._mermaid) {
  content-visibility: visible !important;
  contain: none !important;
  contain-intrinsic-size: auto !important;
}

.assistant-markdown :deep(.ecall-markdown-content > :first-child) {
  margin-top: 0;
}

.assistant-markdown :deep(.ecall-markdown-content > :last-child) {
  margin-bottom: 0;
}

.assistant-markdown :deep(.ecall-markdown-content :where(p,ul,ol,blockquote,pre,table,figure,.paragraph-node,.list-node,.blockquote,.table-node-wrapper,.code-block-container,._mermaid,.vmr-container)) {
  margin-top: 0.25rem;
  margin-bottom: 0.25rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(h1,h2,h3,h4,.heading-node)) {
  margin-top: 0.7rem;
  margin-bottom: 0.32rem;
  line-height: 1.5;
}

.assistant-markdown :deep(.ecall-markdown-content :where(h1,.heading-node.heading-1)) {
  font-size: 1.02rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(h2,.heading-node.heading-2)) {
  font-size: 0.98rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(h3,.heading-node.heading-3)) {
  font-size: 0.94rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(h4,.heading-node.heading-4)) {
  font-size: 0.9rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(ul,ol,.list-node)) {
  padding-left: 1.05rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(li,.list-item)) {
  margin: 0.12rem 0;
  padding-left: 0;
}

.assistant-markdown :deep(.ecall-markdown-content :where(li,.list-item) > :where(p,ul,ol,.paragraph-node,.list-node)) {
  margin-top: 0.16rem;
  margin-bottom: 0.16rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(blockquote,.blockquote)) {
  padding: 0.5rem 0.68rem 0.5rem 0.82rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(blockquote,.blockquote) .markdown-renderer),
.assistant-markdown :deep(.ecall-markdown-content :where(ul,ol,.list-node,li,.list-item) .markdown-renderer) {
  font-size: inherit;
  line-height: inherit;
}

.assistant-markdown :deep(.ecall-markdown-content :where(hr,.hr-node)) {
  margin: 0.65rem 0;
}

.assistant-markdown :deep(.ecall-markdown-content :where(:not(pre) > code,.inline-code):not(.code-block-container *)) {
  font-size: 0.86em;
}

.assistant-markdown :deep(.ecall-markdown-content :where(table,.table-node)) {
  font-size: 0.9rem;
}

.assistant-markdown :deep(.ecall-markdown-content ._mermaid) {
  width: 100%;
}

.tool-review-report-markdown:deep(.code-block-container),
.tool-review-report-markdown:deep(._mermaid) {
  margin: 1rem 0;
}

.tool-review-report-markdown:deep(> :first-child) {
  margin-top: 0;
}

.tool-review-report-markdown:deep(> :last-child) {
  margin-bottom: 0;
}
</style>

<template>
  <div class="conversation-tab-panel">
    <div v-if="loading && groupedTaskSections.length === 0" class="flex justify-center px-3 py-6">
      <span class="loading loading-spinner loading-sm text-base-content/45"></span>
    </div>
    <div v-else-if="errorText" class="px-3 py-4 text-center text-sm text-error">
      {{ errorText }}
    </div>
    <template v-else>
      <CollapsibleGroup
        v-for="section in groupedTaskSections"
        :key="section.key"
        :title="section.title"
        :count="section.items.length"
        :model-value="isTaskSectionCollapsed(section.key)"
        @update:model-value="toggleTaskSection(section.key)"
        @after-enter="emit('layoutChange')"
        @after-leave="emit('layoutChange')"
      >
        <div v-for="task in section.items" :key="task.taskId" class="group relative mx-1">
          <button
            type="button"
            class="block w-full rounded-lg px-2 py-2 text-left transition-colors hover:bg-base-100/70"
            :title="taskItemTitle(task, section.title)"
            @click="emit('editTask', task)"
          >
            <div class="flex min-w-0 items-start gap-2">
              <div class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-base-100 text-base-content/65">
                <ListTodo class="h-4 w-4" />
              </div>
              <div class="min-w-0 flex-1">
                <div class="flex min-w-0 items-start justify-between gap-2">
                  <div class="min-w-0 truncate text-sm font-medium">
                    {{ taskTitle(task) }}
                  </div>
                  <span v-if="taskTimeLabel(task)" class="shrink-0 text-[11px] text-base-content/55">
                    {{ taskTimeLabel(task) }}
                  </span>
                </div>
                <div class="mt-1 truncate text-xs text-base-content/55">
                  {{ taskTodo(task) }}
                </div>
              </div>
            </div>
          </button>
        </div>
      </CollapsibleGroup>
      <div v-if="groupedTaskSections.length === 0" class="px-3 py-4 text-center text-sm text-base-content/60">
        {{ normalizedSearchQuery ? t("chat.taskSidebar.searchEmpty") : t("chat.taskSidebar.empty") }}
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { ListTodo } from "@lucide/vue";
import { invokeTauri } from "../../../services/tauri-api";
import type { ChatConversationOverviewItem } from "../../../types/app";
import type { TaskEntry } from "../../config/views/config-tabs/task-editor";
import { toErrorMessage } from "../../../utils/error";
import CollapsibleGroup from "./CollapsibleGroup.vue";
import { formatConversationListTime } from "../utils/conversation-time";
import { resolveConversationDisplayTitle } from "../utils/conversation-title";

const SYSTEM_NOTIFICATION_CONVERSATION_ID = "system-notification-conversation";

type TaskSection = {
  key: string;
  title: string;
  items: TaskEntry[];
  order: number;
};

const props = defineProps<{
  conversationItems: ChatConversationOverviewItem[];
  searchQuery: string;
}>();

const emit = defineEmits<{
  (e: "editTask", task: TaskEntry): void;
  (e: "layoutChange"): void;
}>();

const { t, locale } = useI18n();
const tasks = ref<TaskEntry[]>([]);
const loading = ref(false);
const errorText = ref("");
const collapsedTaskSectionKeys = ref<Record<string, boolean>>({});

const normalizedSearchQuery = computed(() =>
  String(props.searchQuery || "").trim().toLocaleLowerCase(),
);

const conversationTitleById = computed(() => {
  const map = new Map<string, string>();
  for (const item of props.conversationItems || []) {
    const conversationId = String(item.conversationId || "").trim();
    if (!conversationId) continue;
    map.set(conversationId, resolveConversationDisplayTitle(item, {
      locale: locale.value,
      untitledLabel: t("chat.untitledConversation"),
    }));
  }
  return map;
});

const conversationOrderById = computed(() => {
  const map = new Map<string, number>();
  (props.conversationItems || []).forEach((item, index) => {
    const conversationId = String(item.conversationId || "").trim();
    if (conversationId && !map.has(conversationId)) {
      map.set(conversationId, index);
    }
  });
  return map;
});

const groupedTaskSections = computed<TaskSection[]>(() => {
  const query = normalizedSearchQuery.value;
  const sections = new Map<string, TaskSection>();
  for (const task of tasks.value) {
    if (String(task.completionState || "").trim() !== "active") continue;
    const rawConversationId = String(task.conversationId || "").trim();
    const conversationId = rawConversationId || SYSTEM_NOTIFICATION_CONVERSATION_ID;
    const isSystemTask = conversationId === SYSTEM_NOTIFICATION_CONVERSATION_ID;
    const sectionKey = `conversation:${conversationId}`;
    const title = isSystemTask
      ? t("chat.taskSidebar.systemConversation")
      : conversationTitleById.value.get(conversationId) || t("chat.taskSidebar.unknownConversation");
    if (query && !taskMatchesSearch(task, title, query)) continue;
    if (!sections.has(sectionKey)) {
      sections.set(sectionKey, {
        key: sectionKey,
        title,
        items: [],
        order: isSystemTask ? -1 : conversationOrderById.value.get(conversationId) ?? 1_000_000,
      });
    }
    sections.get(sectionKey)?.items.push(task);
  }
  return Array.from(sections.values())
    .map((section) => ({
      ...section,
      items: section.items.slice().sort(compareTaskForList),
    }))
    .sort((left, right) => left.order - right.order || left.title.localeCompare(right.title, locale.value));
});

function taskMatchesSearch(task: TaskEntry, sectionTitle: string, query: string): boolean {
  return [
    sectionTitle,
    task.goal,
    task.todo,
    task.taskId,
  ].some((value) => String(value || "").toLocaleLowerCase().includes(query));
}

function taskSortTime(task: TaskEntry): number {
  const raw = String(task.trigger?.next_run_at || task.trigger?.run_at || task.updatedAtLocal || "").trim();
  if (!raw) return Number.POSITIVE_INFINITY;
  const time = new Date(raw).getTime();
  return Number.isFinite(time) ? time : Number.POSITIVE_INFINITY;
}

function compareTaskForList(left: TaskEntry, right: TaskEntry): number {
  const timeDiff = taskSortTime(left) - taskSortTime(right);
  if (timeDiff !== 0) return timeDiff;
  return Number(left.orderIndex || 0) - Number(right.orderIndex || 0);
}

function taskTitle(task: TaskEntry): string {
  return String(task.goal || "").trim() || t("config.task.noTodo");
}

function taskTodo(task: TaskEntry): string {
  return String(task.todo || "").trim() || t("config.task.noTodo");
}

function taskTimeLabel(task: TaskEntry): string {
  const raw = String(task.trigger?.next_run_at || task.trigger?.run_at || "").trim();
  return raw ? formatConversationListTime(raw, locale.value) : "";
}

function taskItemTitle(task: TaskEntry, sectionTitle: string): string {
  return `${sectionTitle}\n${taskTitle(task)}`;
}

function isTaskSectionCollapsed(key: string): boolean {
  return !!collapsedTaskSectionKeys.value[key];
}

function toggleTaskSection(key: string) {
  collapsedTaskSectionKeys.value = {
    ...collapsedTaskSectionKeys.value,
    [key]: !collapsedTaskSectionKeys.value[key],
  };
  emit("layoutChange");
}

async function loadTasks() {
  loading.value = true;
  errorText.value = "";
  try {
    tasks.value = await invokeTauri<TaskEntry[]>("task_list_tasks");
  } catch (error) {
    errorText.value = `${t("config.task.listLoadFailed")}: ${toErrorMessage(error)}`;
  } finally {
    loading.value = false;
    emit("layoutChange");
  }
}

function handleTaskRefreshEvent() {
  void loadTasks();
}

onMounted(() => {
  void loadTasks();
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

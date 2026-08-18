<template>
  <div v-if="hasActiveOrPendingTodo" class="pointer-events-none flex justify-center pb-1">
    <div
      class="dropdown dropdown-bottom pointer-events-auto"
      :aria-label="t('config.task.fields.todo')"
      @click.stop
      @mousedown.stop
    >
      <label
        tabindex="0"
        class="todo-trigger flex w-full max-w-[min(88vw,30rem)] cursor-pointer items-center gap-2 overflow-hidden border border-base-300 bg-base-300 py-1.5 pl-3 pr-2 text-base-content transition-colors hover:border-base-300 hover:bg-base-200"
      >
        <span class="shrink-0" :class="activeTodoStatus === 'in_progress' ? 'todo-dot-active' : 'todo-dot-idle'"></span>
        <span class="min-w-0 flex-1 truncate text-left text-sm">{{ activeConversationTodoDisplay }}</span>
        <span
          v-if="todos.length > 1"
          class="badge badge-ghost badge-sm shrink-0 font-normal tabular-nums"
        >{{ completedCount }}/{{ todos.length }}</span>
      </label>
      <div
        tabindex="0"
        class="dropdown-content card card-compact mt-2 w-max max-w-[min(88vw,30rem)] border border-base-300 bg-base-100 shadow-xl"
      >
        <div class="card-body gap-2 p-3">
          <div class="flex items-center justify-between gap-3 text-xs text-base-content/60">
            <span class="font-medium">{{ t("config.task.fields.todo") }}</span>
            <span class="tabular-nums">{{ t("chat.todoProgress", { done: completedCount, total: todos.length }) }}</span>
          </div>
          <progress
            class="progress progress-primary h-1.5"
            :value="completedCount"
            :max="todos.length"
          ></progress>
          <ul class="mt-1 flex flex-col gap-1.5">
            <li
              v-for="(item, index) in todos"
              :key="`${item.status}-${index}-${item.content}`"
              class="flex items-start gap-2.5"
              :title="item.content"
            >
              <span
                class="todo-dot mt-2 shrink-0"
                :class="todoStatusDotClass(item.status)"
              ></span>
              <span
                class="min-w-0 flex-1 wrap-break-word text-sm leading-6"
                :class="todoTextClass(item.status)"
              >{{ item.content }}</span>
            </li>
          </ul>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

interface NormalizedTodo {
  content: string;
  status: "pending" | "in_progress" | "completed";
}

const props = defineProps<{
  todos: NormalizedTodo[];
  personaName: string;
}>();

const { t } = useI18n();

const hasActiveOrPendingTodo = computed(() =>
  props.todos.some((item) => item.status === "pending" || item.status === "in_progress"),
);

const activeTodoIndex = computed(() => {
  const inProgressIndex = props.todos.findIndex((item) => item.status === "in_progress");
  if (inProgressIndex >= 0) return inProgressIndex;
  const pendingIndex = props.todos.findIndex((item) => item.status === "pending");
  if (pendingIndex >= 0) return pendingIndex;
  return props.todos.length ? 0 : -1;
});

const activeTodoStatus = computed(() => {
  const index = activeTodoIndex.value;
  if (index < 0) return "pending";
  return props.todos[index]?.status ?? "pending";
});

const completedCount = computed(() => props.todos.filter((item) => item.status === "completed").length);

const activeConversationTodo = computed(() => {
  const index = activeTodoIndex.value;
  if (index < 0) return "";
  return String(props.todos[index]?.content || "").trim();
});

const activeConversationTodoDisplay = computed(() => {
  const todo = activeConversationTodo.value;
  if (!todo) return "";
  const name = String(props.personaName || "").trim();
  return name
    ? t("chat.todoIntentionWithPersona", { name, todo })
    : t("chat.todoIntention", { todo });
});

function todoStatusDotClass(status: NormalizedTodo["status"]): string {
  if (status === "completed") return "bg-success";
  if (status === "in_progress") return "bg-primary shadow-[0_0_0_3px] shadow-primary/20";
  return "bg-base-300";
}

function todoTextClass(status: NormalizedTodo["status"]): string {
  if (status === "completed") return "text-base-content/50 line-through";
  if (status === "in_progress") return "text-base-content font-semibold";
  return "text-base-content";
}
</script>

<style scoped>
.todo-trigger {
  border-radius: 0 0 var(--radius-field) var(--radius-field);
}

.todo-dot {
  width: 0.5rem;
  height: 0.5rem;
  border-radius: 9999px;
}

.todo-dot-active {
  width: 0.5rem;
  height: 0.5rem;
  border-radius: 9999px;
  background: var(--color-primary);
  animation: todo-dot-pulse 1.8s ease-in-out infinite;
}

.todo-dot-idle {
  width: 0.5rem;
  height: 0.5rem;
  border-radius: 9999px;
  background: var(--color-base-content);
  opacity: 0.4;
}

@keyframes todo-dot-pulse {
  0%,
  100% {
    box-shadow: 0 0 0 0 color-mix(in oklab, var(--color-primary) 45%, transparent);
  }
  60% {
    box-shadow: 0 0 0 6px transparent;
  }
}
</style>
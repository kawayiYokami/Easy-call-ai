<template>
  <div v-if="hasActiveOrPendingTodo" class="pointer-events-none sticky -top-3 z-20 flex justify-center pb-1">
    <div
      class="dropdown dropdown-bottom pointer-events-auto"
      :aria-label="t('config.task.fields.todo')"
      @click.stop
      @mousedown.stop
    >
      <label
        tabindex="0"
        class="todobar-btn btn btn-sm max-w-[min(88vw,30rem)] flex-nowrap justify-start gap-2 overflow-hidden border-base-300 bg-base-300 text-base-content hover:border-base-300 hover:bg-base-200 normal-case"
      >
        <ListTodo class="h-4 w-4 shrink-0 opacity-70" />
        <span class="min-w-0 flex-1 truncate text-left">{{ activeConversationTodoDisplay }}</span>
        <span
          v-if="todos.length > 1"
          class="badge badge-ghost badge-sm shrink-0"
        >+{{ todos.length - 1 }}</span>
      </label>
      <div
        v-if="todos.length > 1"
        tabindex="0"
        class="dropdown-content card card-compact mt-2 w-max max-w-[min(88vw,30rem)] border border-base-300 bg-base-100 shadow-xl"
      >
        <div class="card-body p-3">
          <ul class="flex flex-col gap-3">
            <li
              v-for="(item, index) in todos"
              :key="`${item.status}-${index}-${item.content}`"
              class="flex items-start gap-3"
              :title="item.content"
            >
              <span
                class="inline-flex h-7 min-w-7 shrink-0 items-center justify-center rounded-full text-sm font-semibold"
                :class="todoStatusClass(item.status)"
              >{{ index + 1 }}</span>
              <span
                class="min-w-0 wrap-break-word pt-0.5 text-sm leading-6"
                :class="item.status === 'completed'
                  ? 'text-base-content/55 line-through'
                  : item.status === 'in_progress'
                    ? 'text-base-content font-semibold'
                    : 'text-base-content'"
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
import { ListTodo } from "@lucide/vue";

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

function todoStatusClass(status: NormalizedTodo["status"]): string {
  if (status === "completed") return "bg-success text-success-content";
  if (status === "in_progress") return "bg-primary text-primary-content";
  return "bg-base-200 text-base-content/70";
}
</script>

<style scoped>
.todobar-btn {
  border-radius: 0 0 var(--radius-field) var(--radius-field);
}
.todobar-btn:hover {
  transform: translateY(1px);
}
</style>

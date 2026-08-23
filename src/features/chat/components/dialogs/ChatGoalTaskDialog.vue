<template>
  <dialog class="modal" :class="{ 'modal-open': open }">
    <div class="modal-box w-11/12 max-w-lg p-4">
      <div v-if="activeTask" class="flex items-center justify-between gap-3">
        <div class="min-w-0">
          <div class="truncate text-sm font-medium">
            {{ t("chat.goal.activeHintShort", { goal: activeTask.goal }) }}
          </div>
        </div>
      </div>

      <div
        v-if="errorText"
        class="mt-3 rounded-box bg-error/10 px-3 py-2 text-sm text-error whitespace-pre-wrap break-all"
      >
        {{ errorText }}
      </div>

      <div class="mt-4 flex items-center gap-3">
        <input
          v-model="goal"
          class="h-9 min-w-0 flex-1 border-b border-base-300 bg-transparent text-base outline-none transition-colors placeholder:text-base-content/40 focus:border-primary"
          type="text"
          :placeholder="t('chat.goal.goalPlaceholder')"
          :disabled="saving"
          @keydown.enter.prevent="handleSave"
        />
      </div>

      <div
        v-if="recentHistory.length"
        class="mt-3 flex flex-wrap items-center gap-x-3 gap-y-1"
      >
        <span class="text-xs text-base-content/40">{{ t("chat.goal.recentTitle") }}</span>
        <button
          v-for="(entry, index) in recentHistory"
          :key="`${entry.goal}-${entry.todo}-${index}`"
          type="button"
          class="max-w-56 truncate text-left text-xs text-base-content/60 underline decoration-base-content/20 underline-offset-4 transition hover:text-primary"
          :disabled="saving"
          :title="entry.goal"
          @click="applyRecentHistory(entry)"
        >
          {{ entry.goal }}
        </button>
      </div>

      <div class="mt-5 flex items-center justify-end gap-2">
        <button
          v-if="activeTask"
          class="btn btn-sm btn-ghost text-error"
          :disabled="saving"
          @click="emit('stop')"
        >
          {{ t("chat.goal.stopAction") }}
        </button>
        <button class="btn btn-sm btn-neutral" :disabled="saving" @click="emit('close')">
          {{ t("common.cancel") }}
        </button>
        <button class="btn btn-sm btn-primary" :disabled="saving || !canSubmit" @click="handleSave">
          {{ saving ? t("common.loading") : (activeTask ? t("chat.goal.updateAction") : t("chat.goal.createAction")) }}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('close')">close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

const GOAL_TASK_DURATION_HOURS = 24;

type ActiveGoalTask = {
  taskId: string;
  goal: string;
  why: string;
  todo: string;
  endAtLocal: string;
  remainingHours: number;
};

type GoalHistoryEntry = {
  goal: string;
  why: string;
  todo: string;
  durationHours: number;
};

const props = defineProps<{
  open: boolean;
  saving: boolean;
  errorText: string;
  activeTask: ActiveGoalTask | null;
  recentHistory: GoalHistoryEntry[];
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "save", payload: { durationHours: number; goal: string; why: string; todo: string }): void;
  (e: "stop"): void;
}>();

const { t } = useI18n();

const GOAL_TASK_WHY = "";
const GOAL_TASK_TODO = "请自行判断";
const goal = ref("");

const canSubmit = computed(() => {
  return !!goal.value.trim();
});

function resetForm() {
  goal.value = String(props.activeTask?.goal || t("chat.goal.defaultGoal")).trim();
}

function handleSave() {
  if (!canSubmit.value) return;
  const normalizedGoal = goal.value.trim();
  emit("save", {
    durationHours: GOAL_TASK_DURATION_HOURS,
    goal: normalizedGoal,
    why: GOAL_TASK_WHY,
    todo: GOAL_TASK_TODO,
  });
}

function applyRecentHistory(entry: GoalHistoryEntry) {
  goal.value = String(entry.goal || "").trim();
}

watch(
  () => [props.open, props.activeTask?.taskId, props.activeTask?.endAtLocal] as const,
  ([open]) => {
    if (!open) return;
    resetForm();
  },
  { immediate: true },
);
</script>

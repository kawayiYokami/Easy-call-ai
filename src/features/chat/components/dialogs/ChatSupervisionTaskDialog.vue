<template>
  <dialog class="modal" :class="{ 'modal-open': open }">
    <div class="modal-box w-11/12 max-w-lg p-0">
      <div class="border-b border-base-300/70 px-5 py-4">
        <div class="text-base font-semibold">
          {{ activeTask ? t("chat.supervision.updateTitle") : t("chat.supervision.createTitle") }}
        </div>
      </div>

      <div class="space-y-4 px-5 py-4">
        <div
          v-if="activeTask"
          class="rounded-box border border-primary/20 bg-primary/5 px-3 py-2 text-sm text-base-content/80"
        >
          {{ t("chat.supervision.activeHint", { endAt: activeTask.endAtLocal }) }}
        </div>

        <div
          v-if="errorText"
          class="rounded-box border border-error/30 bg-error/10 px-3 py-2 text-sm text-error whitespace-pre-wrap break-all"
        >
          {{ errorText }}
        </div>

        <label class="block space-y-2">
          <span class="block text-sm font-medium">{{ t("chat.supervision.goalLabel") }}</span>
          <input
            v-model="goal"
            class="input input-bordered w-full"
            type="text"
            :placeholder="t('chat.supervision.goalPlaceholder')"
            :disabled="saving"
            @keydown.enter.prevent="handleSave"
          />
        </label>
      </div>

      <div class="border-t border-base-300/70 bg-base-100 px-5 py-4">
        <div class="flex items-end gap-4">
          <div v-if="recentHistory.length" class="min-w-0 flex-1">
            <div class="mb-2 text-xs font-medium uppercase tracking-[0.08em] text-base-content/50">
              {{ t("chat.supervision.recentTitle") }}
            </div>
            <div class="flex flex-wrap gap-2">
              <button
                v-for="(entry, index) in recentHistory"
                :key="`${entry.goal}-${entry.todo}-${index}`"
                type="button"
                class="min-w-0 max-w-full rounded-box border border-base-300 bg-base-200/70 px-3 py-2 text-left transition hover:border-primary/40 hover:bg-base-200"
                :disabled="saving"
                @click="applyRecentHistory(entry)"
              >
                <div class="truncate text-sm font-medium text-base-content">
                  {{ entry.goal }}
                </div>
              </button>
            </div>
          </div>
          <div class="ml-auto flex shrink-0 items-center justify-end gap-2">
            <button class="btn btn-ghost" :disabled="saving" @click="emit('close')">
              {{ t("common.cancel") }}
            </button>
            <button class="btn btn-primary" :disabled="saving || !canSubmit" @click="handleSave">
              {{ saving ? t("common.loading") : (activeTask ? t("chat.supervision.updateAction") : t("chat.supervision.createAction")) }}
            </button>
          </div>
        </div>
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

type ActiveSupervisionTask = {
  taskId: string;
  goal: string;
  why: string;
  todo: string;
  endAtLocal: string;
  remainingHours: number;
};

type SupervisionHistoryEntry = {
  goal: string;
  why: string;
  todo: string;
  durationHours: number;
};

const props = defineProps<{
  open: boolean;
  saving: boolean;
  errorText: string;
  activeTask: ActiveSupervisionTask | null;
  recentHistory: SupervisionHistoryEntry[];
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "save", payload: { durationHours: number; goal: string; why: string; todo: string }): void;
}>();

const { t } = useI18n();

const GOAL_TASK_WHY = "用户希望你完成目标之前持续推进";
const GOAL_TASK_TODO = "请自行判断";
const goal = ref("");

const canSubmit = computed(() => {
  return !!goal.value.trim();
});

function resetForm() {
  goal.value = String(props.activeTask?.goal || t("chat.supervision.defaultGoal")).trim();
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

function applyRecentHistory(entry: SupervisionHistoryEntry) {
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

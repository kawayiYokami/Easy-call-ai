<template>
  <dialog class="modal" :class="{ 'modal-open': open }" @cancel.prevent="handleClose">
    <div class="modal-box w-11/12 max-w-2xl p-0">
      <div class="flex items-center justify-between gap-3 border-b border-base-300/70 px-5 py-4">
        <h3 class="text-base font-semibold">{{ dialogTitle }}</h3>
        <button
          type="button"
          class="btn btn-ghost btn-sm btn-circle"
          :disabled="dialogBusy"
          :title="t('common.close')"
          @click="handleClose"
        >
          <X class="h-4 w-4" />
        </button>
      </div>

      <form @submit.prevent="handleSubmit">
        <div class="space-y-4 px-5 py-4">
          <div
            v-if="errorText"
            class="rounded-box border border-error/30 bg-error/10 px-3 py-2 text-sm text-error whitespace-pre-wrap break-all"
          >
            {{ errorText }}
          </div>

          <label class="block space-y-2">
            <span class="flex items-center justify-between gap-2">
              <span class="block text-sm font-medium">{{ t("chat.taskCreate.contentLabel") }}</span>
              <button
                v-if="!isEditMode"
                type="button"
                class="btn btn-ghost btn-xs"
                :disabled="dialogBusy || !content.trim()"
                :title="t('chat.taskCreate.optimizeTitle')"
                @click="handleOptimizeDraft"
              >
                <span v-if="optimizing" class="loading loading-spinner loading-xs"></span>
                <WandSparkles v-else class="h-3.5 w-3.5" />
                {{ t("chat.taskCreate.optimizeAction") }}
              </button>
            </span>
            <textarea
              ref="contentInputRef"
              v-model="content"
              class="textarea textarea-bordered min-h-36 w-full resize-none"
              rows="6"
              :placeholder="t('chat.taskCreate.contentPlaceholder')"
              :disabled="dialogBusy"
            ></textarea>
          </label>

          <label class="block space-y-2">
            <span class="block text-sm font-medium">{{ t("chat.taskCreate.titleLabel") }}</span>
            <input
              v-model="title"
              class="input input-bordered w-full"
              type="text"
              :placeholder="t('chat.taskCreate.titlePlaceholder')"
              :disabled="dialogBusy"
            />
          </label>

          <label class="block space-y-2">
            <span class="block text-sm font-medium">{{ t("config.task.fields.scheduleMode") }}</span>
            <SegmentedControl
              v-model="scheduleMode"
              :options="scheduleModeOptions"
              :disabled="dialogBusy"
              size="sm"
            />
          </label>

          <label class="block space-y-2">
            <span class="block text-sm font-medium">{{ t("config.task.fields.runAt") }}</span>
            <TaskDateTimeInput v-model="runAt" :disabled="dialogBusy" />
          </label>

          <div v-if="scheduleMode === 'interval'" class="space-y-3">
            <label class="block space-y-2">
              <span class="block text-sm font-medium">{{ t("chat.taskCreate.intervalLabel") }}</span>
              <div class="join w-full">
                <input
                  v-model="repeatEvery"
                  class="input input-bordered join-item min-w-0 flex-1"
                  type="number"
                  min="1"
                  :max="repeatUnit === 'months' ? 12 : undefined"
                  step="1"
                  :disabled="dialogBusy"
                />
                <select
                  v-model="repeatUnit"
                  class="select select-bordered join-item w-32 shrink-0"
                  :disabled="dialogBusy"
                >
                  <option value="minutes">{{ t("chat.taskCreate.intervalUnits.minutes") }}</option>
                  <option value="hours">{{ t("chat.taskCreate.intervalUnits.hours") }}</option>
                  <option value="days">{{ t("chat.taskCreate.intervalUnits.days") }}</option>
                  <option value="weeks">{{ t("chat.taskCreate.intervalUnits.weeks") }}</option>
                  <option value="months">{{ t("chat.taskCreate.intervalUnits.months") }}</option>
                </select>
              </div>
            </label>
            <label class="block space-y-2">
              <span class="block text-sm font-medium">{{ t("config.task.fields.endAt") }}</span>
              <TaskDateTimeInput v-model="endAt" :disabled="dialogBusy" />
            </label>
          </div>
        </div>

        <div class="border-t border-base-300/70 bg-base-100 px-5 py-4">
          <div class="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
            <button
              v-if="isEditMode"
              type="button"
              class="btn btn-error btn-outline"
              :disabled="dialogBusy"
              @click="requestDeleteConfirm"
            >
              <Trash2 class="h-4 w-4" />
              {{ t("common.delete") }}
            </button>
            <span v-else class="hidden sm:block"></span>
            <div class="flex items-center justify-end gap-2">
              <button type="button" class="btn btn-ghost" :disabled="dialogBusy" @click="handleClose">
                {{ t("common.cancel") }}
              </button>
              <button type="submit" class="btn btn-primary" :disabled="dialogBusy">
                <span v-if="saving" class="loading loading-spinner loading-sm"></span>
                {{ submitButtonLabel }}
              </button>
            </div>
          </div>
        </div>
      </form>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="handleClose">close</button>
    </form>
  </dialog>

  <dialog class="modal" :class="{ 'modal-open': deleteConfirmOpen }" @cancel.prevent="closeDeleteConfirm">
    <div class="modal-box max-w-md p-4">
      <h3 class="text-sm font-semibold">{{ t("common.delete") }}</h3>
      <p class="mt-3 whitespace-pre-wrap text-sm">{{ t("config.task.deleteConfirm") }}</p>
      <div class="modal-action mt-4">
        <button class="btn btn-sm btn-ghost" type="button" :disabled="dialogBusy" @click="closeDeleteConfirm">
          {{ t("common.cancel") }}
        </button>
        <button class="btn btn-sm btn-error" type="button" :disabled="dialogBusy" @click="handleDeleteConfirmed">
          <span v-if="saving" class="loading loading-spinner loading-xs"></span>
          {{ t("common.confirm") }}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="closeDeleteConfirm">close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { Trash2, WandSparkles, X } from "@lucide/vue";
import { invokeTauri } from "../../../../services/tauri-api";
import { formatDateToLocalRfc3339 } from "../../../../utils/time";
import { toErrorMessage } from "../../../../utils/error";
import SegmentedControl from "../../../config/components/SegmentedControl.vue";
import TaskDateTimeInput from "../../../config/views/config-tabs/TaskDateTimeInput.vue";
import type { TaskEntry, TaskScheduleMode } from "../../../config/views/config-tabs/task-editor";

type RepeatIntervalUnit = "minutes" | "hours" | "days" | "weeks" | "months";

type TaskTriggerInputWire = {
  run_at: string;
  everyMinutes?: number;
  cron_expression?: string;
  end_at?: string;
};

type TaskCreateInputWire = {
  conversationId: string;
  targetScope: "desktop";
  goal: string;
  why: string;
  todo: string;
  trigger: TaskTriggerInputWire;
};

type TaskUpdateInputWire = {
  taskId: string;
  conversationId?: string;
  goal: string;
  why: string;
  todo: string;
  trigger: TaskTriggerInputWire;
};

type TaskDeleteInputWire = {
  taskId: string;
};

type TaskOptimizeDraftInputWire = {
  title: string;
  content: string;
  scheduleMode: TaskScheduleMode;
  runAt: string;
  repeatEvery: string;
  repeatUnit: RepeatIntervalUnit;
  endAt: string;
};

type TaskOptimizeDraftOutputWire = {
  title: string;
  content: string;
  scheduleMode: TaskScheduleMode;
  runAt: string;
  repeatEvery: string;
  repeatUnit: RepeatIntervalUnit;
  endAt: string;
};

const DEFAULT_RUN_AT_DELAY_MINUTES = 10;
const DERIVED_TITLE_LIMIT = 80;

const props = defineProps<{
  open: boolean;
  mode?: "create" | "edit";
  conversationId?: string;
  task?: TaskEntry | null;
  bridgeRequest?: <T = unknown>(method: string, params?: Record<string, unknown>, timeoutMs?: number) => Promise<T>;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "created", task: TaskEntry): void;
  (e: "updated", task: TaskEntry): void;
}>();

const { t } = useI18n();
const contentInputRef = ref<HTMLTextAreaElement | null>(null);
const saving = ref(false);
const optimizing = ref(false);
const deleteConfirmOpen = ref(false);
const errorText = ref("");
const title = ref("");
const content = ref("");
const runAt = ref("");
const scheduleMode = ref<TaskScheduleMode>("once");
const repeatEvery = ref("1");
const repeatUnit = ref<RepeatIntervalUnit>("hours");
const endAt = ref("");
const preservedCronExpression = ref("");
const initialScheduleSnapshot = ref("");
const dialogMode = computed(() => props.mode === "edit" ? "edit" : "create");
const isEditMode = computed(() => dialogMode.value === "edit");
const dialogBusy = computed(() => saving.value || optimizing.value);
const existingRecurringTask = computed(() =>
  isEditMode.value
  && (
    !!String(props.task?.trigger?.cron_expression || "").trim()
    || (Number.isFinite(Number(props.task?.trigger?.every_minutes)) && Number(props.task?.trigger?.every_minutes) > 0)
  ),
);
const scheduleModeOptions = computed(() => [
  { value: "once" as const, label: t("config.task.scheduleModes.once"), disabled: existingRecurringTask.value },
  { value: "interval" as const, label: t("config.task.scheduleModes.interval") },
]);
const dialogTitle = computed(() => isEditMode.value ? t("config.task.editorEditTitle") : t("chat.taskCreate.title"));
const submitButtonLabel = computed(() => {
  if (saving.value) return t("config.task.saving");
  return isEditMode.value ? t("config.task.saveUpdate") : t("config.task.createAction");
});

function defaultRunAt(): string {
  const next = new Date(Date.now() + DEFAULT_RUN_AT_DELAY_MINUTES * 60_000);
  next.setSeconds(0, 0);
  return formatDateToLocalRfc3339(next);
}

function resetForm() {
  title.value = "";
  content.value = "";
  runAt.value = defaultRunAt();
  scheduleMode.value = "once";
  repeatEvery.value = "1";
  repeatUnit.value = "hours";
  endAt.value = "";
  preservedCronExpression.value = "";
  initialScheduleSnapshot.value = scheduleInputSnapshot();
  errorText.value = "";
}

function resetFormFromTask(task: TaskEntry) {
  title.value = String(task.goal || "").trim();
  content.value = String(task.todo || "").trim();
  runAt.value = String(task.trigger?.run_at || task.trigger?.next_run_at || "").trim() || defaultRunAt();
  endAt.value = String(task.trigger?.end_at || "").trim();
  preservedCronExpression.value = String(task.trigger?.cron_expression || "").trim();
  const everyMinutes = Number(task.trigger?.every_minutes);
  if (Number.isFinite(everyMinutes) && everyMinutes > 0) {
    scheduleMode.value = "interval";
    setRepeatFromEveryMinutes(everyMinutes);
  } else if (preservedCronExpression.value) {
    scheduleMode.value = "interval";
    const monthlyInterval = inferMonthlyIntervalFromCronExpression(preservedCronExpression.value);
    if (monthlyInterval) {
      repeatEvery.value = String(monthlyInterval);
      repeatUnit.value = "months";
    } else {
      repeatEvery.value = "1";
      repeatUnit.value = "hours";
    }
  } else {
    scheduleMode.value = "once";
    repeatEvery.value = "1";
    repeatUnit.value = "hours";
  }
  initialScheduleSnapshot.value = scheduleInputSnapshot();
  errorText.value = "";
}

function deriveTitleFromContent(value: string): string {
  const firstLine = value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .find((line) => !!line);
  const compact = String(firstLine || value).replace(/\s+/g, " ").trim();
  if (compact.length <= DERIVED_TITLE_LIMIT) return compact;
  return `${compact.slice(0, DERIVED_TITLE_LIMIT)}...`;
}

function buildMonthlyCronExpression(runAtDate: Date, repeatEveryValue: number): string {
  const minute = runAtDate.getMinutes();
  const hour = runAtDate.getHours();
  const day = runAtDate.getDate();
  if (repeatEveryValue === 1) {
    return `${minute} ${hour} ${day} * *`;
  }
  const startMonth = runAtDate.getMonth() + 1;
  const months: number[] = [];
  let current = startMonth;
  for (let index = 0; index < 12; index += 1) {
    if (months.includes(current)) break;
    months.push(current);
    current += repeatEveryValue;
    while (current > 12) {
      current -= 12;
    }
  }
  return `${minute} ${hour} ${day} ${months.sort((left, right) => left - right).join(",")} *`;
}

function inferMonthlyIntervalFromCronExpression(value: string): number | null {
  const parts = String(value || "").trim().split(/\s+/);
  if (parts.length !== 5 || parts[4] !== "*") return null;
  const monthPart = parts[3];
  if (monthPart === "*") return 1;
  const months = monthPart
    .split(",")
    .map((item) => Number.parseInt(item, 10))
    .filter((item) => Number.isInteger(item) && item >= 1 && item <= 12)
    .sort((left, right) => left - right);
  if (months.length === 1) return 12;
  if (months.length < 2) return null;
  const diffs = months.map((month, index) => {
    const nextMonth = index === months.length - 1 ? months[0] + 12 : months[index + 1];
    return nextMonth - month;
  });
  const firstDiff = diffs[0];
  return firstDiff > 0 && diffs.every((item) => item === firstDiff) ? firstDiff : null;
}

function setRepeatFromEveryMinutes(value: number) {
  const minuteValue = Math.max(1, Math.floor(value));
  const unitToMinutes: Record<RepeatIntervalUnit, number> = {
    minutes: 1,
    hours: 60,
    days: 24 * 60,
    weeks: 7 * 24 * 60,
    months: 30 * 24 * 60,
  };
  const preferredUnits: RepeatIntervalUnit[] = ["weeks", "days", "hours", "minutes"];
  const matchedUnit = preferredUnits.find((unit) => minuteValue % unitToMinutes[unit] === 0) || "minutes";
  repeatUnit.value = matchedUnit;
  repeatEvery.value = String(Math.max(1, Math.floor(minuteValue / unitToMinutes[matchedUnit])));
}

function normalizeOptimizedScheduleMode(value: string): TaskScheduleMode | null {
  return value === "once" || value === "interval" ? value : null;
}

function normalizeOptimizedRepeatUnit(value: string): RepeatIntervalUnit | null {
  const normalized = String(value || "").trim();
  if (
    normalized === "minutes"
    || normalized === "hours"
    || normalized === "days"
    || normalized === "weeks"
    || normalized === "months"
  ) {
    return normalized;
  }
  return null;
}

function normalizeOptimizedRepeatEvery(value: string, unit: RepeatIntervalUnit): string | null {
  const parsed = Number.parseInt(String(value || "").trim(), 10);
  if (!Number.isInteger(parsed) || parsed <= 0) return null;
  if (unit === "months" && ![1, 2, 3, 4, 6, 12].includes(parsed)) return null;
  return String(parsed);
}

function normalizeOptimizedDateTime(value: string): string | null {
  const normalized = String(value || "").trim();
  if (!normalized) return null;
  const parsed = new Date(normalized);
  return Number.isFinite(parsed.getTime()) ? normalized : null;
}

function scheduleInputSnapshot(): string {
  return JSON.stringify({
    runAt: String(runAt.value || "").trim(),
    scheduleMode: scheduleMode.value,
    repeatEvery: String(repeatEvery.value || "").trim(),
    repeatUnit: repeatUnit.value,
    endAt: String(endAt.value || "").trim(),
  });
}

function buildPayload(): TaskCreateInputWire | TaskUpdateInputWire | null {
  const normalizedContent = content.value.trim();
  if (!normalizedContent) {
    errorText.value = t("chat.taskCreate.validation.contentRequired");
    void nextTick(() => contentInputRef.value?.focus());
    return null;
  }

  const normalizedRunAt = runAt.value.trim();
  if (!normalizedRunAt) {
    errorText.value = t("chat.taskCreate.validation.runAtRequired");
    return null;
  }
  const runAtDate = new Date(normalizedRunAt);
  if (!Number.isFinite(runAtDate.getTime())) {
    errorText.value = t("chat.taskCreate.validation.runAtInvalid");
    return null;
  }
  if (!isEditMode.value && runAtDate.getTime() < Date.now() - 60_000) {
    errorText.value = t("chat.taskCreate.validation.runAtPast");
    return null;
  }

  const trigger: TaskTriggerInputWire = {
    run_at: normalizedRunAt,
  };
  if (existingRecurringTask.value && scheduleMode.value !== "interval") {
    errorText.value = t("config.task.validation.recurringToOnceNotAllowed");
    return null;
  }
  if (scheduleMode.value === "interval") {
    const existingEveryMinutes = Number(props.task?.trigger?.every_minutes);
    if (
      isEditMode.value
      && preservedCronExpression.value
      && !Number.isFinite(existingEveryMinutes)
      && initialScheduleSnapshot.value === scheduleInputSnapshot()
    ) {
      trigger.cron_expression = preservedCronExpression.value;
    } else {
      const repeatEveryValue = Number(String(repeatEvery.value || "1").trim() || "1");
      if (!Number.isInteger(repeatEveryValue) || repeatEveryValue <= 0) {
        errorText.value = t("chat.taskCreate.validation.intervalInvalid");
        return null;
      }
      if (repeatUnit.value === "months") {
        if (repeatEveryValue > 12 || (repeatEveryValue > 1 && 12 % repeatEveryValue !== 0)) {
          errorText.value = t("chat.taskCreate.validation.monthIntervalInvalid");
          return null;
        }
        trigger.cron_expression = buildMonthlyCronExpression(runAtDate, repeatEveryValue);
      } else {
        const unitToMinutes: Record<Exclude<RepeatIntervalUnit, "months">, number> = {
          minutes: 1,
          hours: 60,
          days: 24 * 60,
          weeks: 7 * 24 * 60,
        };
        trigger.everyMinutes = repeatEveryValue * unitToMinutes[repeatUnit.value];
      }
    }

    const normalizedEndAt = endAt.value.trim();
    if (normalizedEndAt) {
      const endAtDate = new Date(normalizedEndAt);
      if (!Number.isFinite(endAtDate.getTime())) {
        errorText.value = t("chat.taskCreate.validation.endAtInvalid");
        return null;
      }
      if (endAtDate.getTime() <= runAtDate.getTime()) {
        errorText.value = t("chat.taskCreate.validation.endAtNotAfterRunAt");
        return null;
      }
      trigger.end_at = normalizedEndAt;
    }
  }

  const basePayload: TaskCreateInputWire = {
    conversationId: String(props.conversationId || props.task?.conversationId || "").trim(),
    targetScope: "desktop",
    goal: title.value.trim() || deriveTitleFromContent(normalizedContent),
    why: "",
    todo: normalizedContent,
    trigger,
  };
  if (!basePayload.conversationId) {
    errorText.value = t("chat.taskCreate.validation.noConversation");
    return null;
  }
  if (!isEditMode.value) return basePayload;
  const taskId = String(props.task?.taskId || "").trim();
  if (!taskId) {
    errorText.value = t("config.task.detailLoadFailed");
    return null;
  }
  const conversationId = String(props.task?.conversationId || "").trim();
  return {
    taskId,
    ...(conversationId ? { conversationId } : {}),
    goal: basePayload.goal,
    why: basePayload.why,
    todo: basePayload.todo,
    trigger,
  };
}

function dispatchTaskCreatedEvent(task: TaskEntry) {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent("easy-call:task-created", {
    detail: {
      taskId: String(task.taskId || "").trim(),
    },
  }));
}

function dispatchTaskUpdatedEvent(task: TaskEntry) {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent("easy-call:task-updated", {
    detail: {
      taskId: String(task.taskId || "").trim(),
    },
  }));
}

function dispatchTaskDeletedEvent(taskId: string) {
  if (typeof window === "undefined") return;
  window.dispatchEvent(new CustomEvent("easy-call:task-deleted", {
    detail: {
      taskId: String(taskId || "").trim(),
    },
  }));
}

async function requestTaskCreate(payload: TaskCreateInputWire): Promise<TaskEntry> {
  if (props.bridgeRequest) return props.bridgeRequest<TaskEntry>("task.create", payload);
  return invokeTauri<TaskEntry>("task_create_task", { input: payload });
}

async function requestTaskUpdate(payload: TaskUpdateInputWire): Promise<TaskEntry> {
  if (props.bridgeRequest) return props.bridgeRequest<TaskEntry>("task.update", payload);
  return invokeTauri<TaskEntry>("task_update_task", { input: payload });
}

async function requestTaskDelete(payload: TaskDeleteInputWire): Promise<void> {
  if (props.bridgeRequest) {
    await props.bridgeRequest("task.delete", payload);
    return;
  }
  await invokeTauri("task_delete_task", { input: payload });
}

async function requestTaskOptimize(payload: TaskOptimizeDraftInputWire): Promise<TaskOptimizeDraftOutputWire> {
  if (props.bridgeRequest) return props.bridgeRequest<TaskOptimizeDraftOutputWire>("task.optimizeDraft", payload, 60_000);
  return invokeTauri<TaskOptimizeDraftOutputWire>("task_optimize_draft", { input: payload });
}

async function handleSubmit() {
  if (dialogBusy.value) return;
  const payload = buildPayload();
  if (!payload) return;

  saving.value = true;
  errorText.value = "";
  try {
    if (isEditMode.value) {
      const updated = await requestTaskUpdate(payload as TaskUpdateInputWire);
      dispatchTaskUpdatedEvent(updated);
      emit("updated", updated);
      emit("close");
      return;
    }
    const created = await requestTaskCreate(payload as TaskCreateInputWire);
    dispatchTaskCreatedEvent(created);
    emit("created", created);
    emit("close");
  } catch (error) {
    errorText.value = `${isEditMode.value ? t("config.task.updateFailed") : t("config.task.createFailed")}: ${toErrorMessage(error)}`;
  } finally {
    saving.value = false;
  }
}

async function handleOptimizeDraft() {
  if (dialogBusy.value || isEditMode.value) return;
  const normalizedContent = content.value.trim();
  if (!normalizedContent) {
    errorText.value = t("chat.taskCreate.validation.contentRequired");
    void nextTick(() => contentInputRef.value?.focus());
    return;
  }
  const payload: TaskOptimizeDraftInputWire = {
    title: title.value.trim(),
    content: normalizedContent,
    scheduleMode: scheduleMode.value,
    runAt: runAt.value.trim(),
    repeatEvery: repeatEvery.value.trim(),
    repeatUnit: repeatUnit.value,
    endAt: endAt.value.trim(),
  };
  optimizing.value = true;
  errorText.value = "";
  try {
    const optimized = await requestTaskOptimize(payload);
    const nextContent = String(optimized.content || "").trim();
    const nextTitle = String(optimized.title || "").trim();
    if (nextContent) content.value = nextContent;
    if (nextTitle) title.value = nextTitle;
    const nextScheduleMode = normalizeOptimizedScheduleMode(String(optimized.scheduleMode || ""));
    if (nextScheduleMode) scheduleMode.value = nextScheduleMode;
    const nextRunAt = normalizeOptimizedDateTime(String(optimized.runAt || ""));
    if (nextRunAt) runAt.value = nextRunAt;
    const nextRepeatUnit = normalizeOptimizedRepeatUnit(String(optimized.repeatUnit || ""));
    if (nextRepeatUnit) repeatUnit.value = nextRepeatUnit;
    const nextRepeatEvery = normalizeOptimizedRepeatEvery(String(optimized.repeatEvery || ""), repeatUnit.value);
    if (nextRepeatEvery) repeatEvery.value = nextRepeatEvery;
    const nextEndAt = normalizeOptimizedDateTime(String(optimized.endAt || ""));
    endAt.value = scheduleMode.value === "interval" && nextEndAt ? nextEndAt : "";
  } catch (error) {
    errorText.value = `${t("chat.taskCreate.optimizeFailed")}: ${toErrorMessage(error)}`;
  } finally {
    optimizing.value = false;
  }
}

function requestDeleteConfirm() {
  if (dialogBusy.value || !isEditMode.value) return;
  deleteConfirmOpen.value = true;
}

function closeDeleteConfirm() {
  if (dialogBusy.value) return;
  deleteConfirmOpen.value = false;
}

async function handleDeleteConfirmed() {
  if (dialogBusy.value || !isEditMode.value) return;
  const taskId = String(props.task?.taskId || "").trim();
  if (!taskId) {
    deleteConfirmOpen.value = false;
    errorText.value = t("config.task.detailLoadFailed");
    return;
  }
  const payload: TaskDeleteInputWire = { taskId };
  saving.value = true;
  errorText.value = "";
  try {
    await requestTaskDelete(payload);
    dispatchTaskDeletedEvent(taskId);
    deleteConfirmOpen.value = false;
    emit("close");
  } catch (error) {
    deleteConfirmOpen.value = false;
    errorText.value = `${t("config.task.deleteFailed")}: ${toErrorMessage(error)}`;
  } finally {
    saving.value = false;
  }
}

function handleClose() {
  if (dialogBusy.value) return;
  deleteConfirmOpen.value = false;
  emit("close");
}

watch(
  () => props.open,
  async (open) => {
    if (!open) return;
    if (isEditMode.value && props.task) {
      resetFormFromTask(props.task);
    } else {
      resetForm();
    }
    await nextTick();
    contentInputRef.value?.focus();
  },
  { immediate: true },
);

watch(
  () => props.task,
  (task) => {
    if (!props.open || !isEditMode.value || !task) return;
    resetFormFromTask(task);
  },
);
</script>

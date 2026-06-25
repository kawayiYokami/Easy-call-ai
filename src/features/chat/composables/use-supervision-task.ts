import { computed, ref, type Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";
import { toErrorMessage } from "../../../utils/error";
import type { ConversationGoalState } from "../../../types/app";
import { clearNativeTextSelection } from "../../../utils/native-selection";

const SUPERVISION_TASK_HISTORY_STORAGE_KEY = "chat-supervision-task-history";
const SUPERVISION_TASK_HISTORY_LIMIT = 3;

export type ActiveSupervisionTaskSummary = {
  taskId: string;
  goal: string;
  why: string;
  todo: string;
  endAtLocal: string;
  remainingHours: number;
};

export type SupervisionTaskHistoryEntry = {
  goal: string;
  why: string;
  todo: string;
  durationHours: number;
};

type GoalMutationOutput = {
  conversationId: string;
  goal: ConversationGoalState;
};

type GoalUpdatedPayload = {
  conversationId?: string;
  goal?: ConversationGoalState | null;
};

type UseSupervisionTaskOptions = {
  t: (key: string, params?: Record<string, unknown>) => string;
  currentConversationId: Ref<string>;
  setStatus: (message: string) => void;
};

function goalIsActive(goal?: ConversationGoalState | null): goal is ConversationGoalState {
  return String(goal?.status || "").trim() === "active";
}

function activeSupervisionTaskFromGoal(goal: ConversationGoalState): ActiveSupervisionTaskSummary {
  return {
    taskId: String(goal.goalId || "").trim(),
    goal: String(goal.objective || "").trim(),
    why: "",
    todo: "",
    endAtLocal: String(goal.startedAt || "").trim(),
    remainingHours: 0,
  };
}

export function useSupervisionTask(options: UseSupervisionTaskOptions) {
  const supervisionTaskDialogOpen = ref(false);
  const supervisionTaskSaving = ref(false);
  const supervisionTaskError = ref("");
  const activeSupervisionTask = ref<ActiveSupervisionTaskSummary | null>(null);
  const recentSupervisionTaskHistory = ref<SupervisionTaskHistoryEntry[]>([]);

  function clearSupervisionTaskPollTimer() {
    // Goal 状态由会话快照和 goalUpdated 事件驱动，不再轮询 task。
  }

  function normalizeSupervisionTaskHistoryEntry(entry: Partial<SupervisionTaskHistoryEntry>): SupervisionTaskHistoryEntry | null {
    const goal = String(entry.goal || "").trim();
    const why = String(entry.why || "").trim();
    const todo = String(entry.todo || "").trim();
    const durationHours = Math.min(24, Math.max(1, Number(entry.durationHours || 1)));
    if (!goal) return null;
    return {
      goal,
      why,
      todo,
      durationHours,
    };
  }

  function loadRecentSupervisionTaskHistory() {
    try {
      const raw = window.localStorage.getItem(SUPERVISION_TASK_HISTORY_STORAGE_KEY);
      if (!raw) {
        recentSupervisionTaskHistory.value = [];
        return;
      }
      const parsed = JSON.parse(raw);
      if (!Array.isArray(parsed)) {
        recentSupervisionTaskHistory.value = [];
        return;
      }
      const normalized: SupervisionTaskHistoryEntry[] = [];
      const seen = new Set<string>();
      for (const item of parsed) {
        const entry = normalizeSupervisionTaskHistoryEntry(
          (item || {}) as Partial<SupervisionTaskHistoryEntry>,
        );
        if (!entry) continue;
        const dedupeKey = JSON.stringify(entry);
        if (seen.has(dedupeKey)) continue;
        seen.add(dedupeKey);
        normalized.push(entry);
        if (normalized.length >= SUPERVISION_TASK_HISTORY_LIMIT) break;
      }
      recentSupervisionTaskHistory.value = normalized;
    } catch {
      recentSupervisionTaskHistory.value = [];
    }
  }

  function saveRecentSupervisionTaskHistory() {
    try {
      window.localStorage.setItem(
        SUPERVISION_TASK_HISTORY_STORAGE_KEY,
        JSON.stringify(recentSupervisionTaskHistory.value),
      );
    } catch {
      // ignore persistence failures
    }
  }

  function pushRecentSupervisionTaskHistory(entry: Partial<SupervisionTaskHistoryEntry>) {
    const normalized = normalizeSupervisionTaskHistoryEntry(entry);
    if (!normalized) return;
    const dedupeKey = JSON.stringify(normalized);
    recentSupervisionTaskHistory.value = [
      normalized,
      ...recentSupervisionTaskHistory.value.filter((item) => JSON.stringify(item) !== dedupeKey),
    ].slice(0, SUPERVISION_TASK_HISTORY_LIMIT);
    saveRecentSupervisionTaskHistory();
  }

  const chatSupervisionActive = computed(() => !!activeSupervisionTask.value);
  const chatSupervisionTitle = computed(() => {
    const task = activeSupervisionTask.value;
    if (!task) {
      return options.t("chat.supervision.buttonHint");
    }
    return options.t("chat.supervision.activeHintShort", { goal: task.goal });
  });

  async function refreshActiveSupervisionTask(params: { silent?: boolean } = {}) {
    const conversationId = String(options.currentConversationId.value || "").trim();
    if (!conversationId) {
      activeSupervisionTask.value = null;
      return;
    }
    try {
      const goal = await invokeTauri<ConversationGoalState | null>("goal_get_current", {
        conversationId,
      });
      activeSupervisionTask.value = goalIsActive(goal) ? activeSupervisionTaskFromGoal(goal) : null;
    } catch (error) {
      activeSupervisionTask.value = null;
      if (!params.silent) {
        console.warn("[目标] 读取当前会话 goal 失败", error);
      }
    }
  }

  function applyConversationGoalUpdated(payload: GoalUpdatedPayload) {
    const conversationId = String(payload?.conversationId || "").trim();
    if (conversationId !== String(options.currentConversationId.value || "").trim()) return;
    const goal = payload?.goal || null;
    activeSupervisionTask.value = goalIsActive(goal) ? activeSupervisionTaskFromGoal(goal) : null;
  }

  function openSupervisionTaskDialog() {
    if (!String(options.currentConversationId.value || "").trim()) {
      options.setStatus(options.t("chat.supervision.noConversation"));
      return;
    }
    clearNativeTextSelection();
    supervisionTaskError.value = "";
    supervisionTaskDialogOpen.value = true;
  }

  function closeSupervisionTaskDialog() {
    if (supervisionTaskSaving.value) return;
    supervisionTaskDialogOpen.value = false;
    supervisionTaskError.value = "";
  }

  async function saveSupervisionTask(payload: {
    durationHours: number;
    goal: string;
    why: string;
    todo: string;
  }) {
    if (supervisionTaskSaving.value) return;
    const conversationId = String(options.currentConversationId.value || "").trim();
    const objective = String(payload.goal || "").trim();
    if (!conversationId) {
      supervisionTaskError.value = options.t("chat.supervision.noConversation");
      return;
    }
    if (!objective) {
      supervisionTaskError.value = options.t("chat.supervision.goalPlaceholder");
      return;
    }
    supervisionTaskSaving.value = true;
    supervisionTaskError.value = "";
    try {
      const hadActiveGoal = !!activeSupervisionTask.value;
      if (hadActiveGoal) {
        await invokeTauri<GoalMutationOutput>("goal_cancel_goal", {
          input: { conversationId },
        });
      }
      const created = await invokeTauri<GoalMutationOutput>("goal_create_goal", {
        input: {
          conversationId,
          objective,
        },
      });
      activeSupervisionTask.value = goalIsActive(created.goal)
        ? activeSupervisionTaskFromGoal(created.goal)
        : null;
      options.setStatus(
        options.t(hadActiveGoal ? "chat.supervision.updatedStatus" : "chat.supervision.createdStatus"),
      );
      pushRecentSupervisionTaskHistory(payload);
      supervisionTaskDialogOpen.value = false;
    } catch (error) {
      supervisionTaskError.value = `${options.t("chat.supervision.saveFailed")}: ${toErrorMessage(error)}`;
    } finally {
      supervisionTaskSaving.value = false;
    }
  }

  async function stopSupervisionTask() {
    if (supervisionTaskSaving.value) return;
    const conversationId = String(options.currentConversationId.value || "").trim();
    if (!conversationId || !activeSupervisionTask.value) {
      supervisionTaskError.value = options.t("chat.supervision.noActiveTask");
      return;
    }
    supervisionTaskSaving.value = true;
    supervisionTaskError.value = "";
    try {
      await invokeTauri<GoalMutationOutput>("goal_cancel_goal", {
        input: { conversationId },
      });
      options.setStatus(options.t("chat.supervision.stoppedStatus"));
      activeSupervisionTask.value = null;
      supervisionTaskDialogOpen.value = false;
      await refreshActiveSupervisionTask({ silent: true });
    } catch (error) {
      supervisionTaskError.value = `${options.t("chat.supervision.stopFailed")}: ${toErrorMessage(error)}`;
    } finally {
      supervisionTaskSaving.value = false;
    }
  }

  function startSupervisionTaskPolling() {
    void refreshActiveSupervisionTask({ silent: true });
  }

  function handleConversationChanged() {
    supervisionTaskDialogOpen.value = false;
    supervisionTaskError.value = "";
    void refreshActiveSupervisionTask({ silent: true });
  }

  loadRecentSupervisionTaskHistory();

  return {
    supervisionTaskDialogOpen,
    supervisionTaskSaving,
    supervisionTaskError,
    activeSupervisionTask,
    recentSupervisionTaskHistory,
    chatSupervisionActive,
    chatSupervisionTitle,
    openSupervisionTaskDialog,
    closeSupervisionTaskDialog,
    saveSupervisionTask,
    stopSupervisionTask,
    refreshActiveSupervisionTask,
    startSupervisionTaskPolling,
    clearSupervisionTaskPollTimer,
    handleConversationChanged,
    applyConversationGoalUpdated,
  };
}

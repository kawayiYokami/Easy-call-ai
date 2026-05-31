import { computed, type Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";
import type { UnarchivedConversationSummary } from "../../../types/app";

type UseConversationPlanModeOptions = {
  currentConversationId: Ref<string>;
  unarchivedConversations: Ref<UnarchivedConversationSummary[]>;
};

export function useConversationPlanMode(options: UseConversationPlanModeOptions) {
  const inFlightPlanModeRequests = new Map<string, Promise<boolean>>();
  const confirmedConversationPlanModeStates = new Map<string, boolean>();

  function getConversationPlanModeEnabledById(conversationId: string): boolean {
    const normalizedConversationId = String(conversationId || "").trim();
    if (!normalizedConversationId) return false;
    return !!options.unarchivedConversations.value.find((item) =>
      String(item.conversationId || "").trim() === normalizedConversationId
    )?.planModeEnabled;
  }

  function patchConversationPlanModeInOverview(conversationId: string, planModeEnabled: boolean) {
    const normalizedConversationId = String(conversationId || "").trim();
    if (!normalizedConversationId) return;
    let changed = false;
    const next = options.unarchivedConversations.value.map((item) => {
      if (String(item.conversationId || "").trim() !== normalizedConversationId) {
        return item;
      }
      if (!!item.planModeEnabled === !!planModeEnabled) {
        return item;
      }
      changed = true;
      return {
        ...item,
        planModeEnabled: !!planModeEnabled,
      };
    });
    if (changed) {
      options.unarchivedConversations.value = next;
    }
  }

  function queueConversationPlanModeUpdate(
    conversationId: string,
    task: () => Promise<boolean>,
  ): Promise<boolean> {
    const previous = inFlightPlanModeRequests.get(conversationId) ?? Promise.resolve(true);
    let queued!: Promise<boolean>;
    queued = previous
      .catch(() => false)
      .then(task, task)
      .finally(() => {
        if (inFlightPlanModeRequests.get(conversationId) === queued) {
          inFlightPlanModeRequests.delete(conversationId);
        }
      });
    inFlightPlanModeRequests.set(conversationId, queued);
    return queued;
  }

  async function setConversationPlanMode(conversationId: string, value: boolean): Promise<boolean> {
    const normalizedConversationId = String(conversationId || "").trim();
    if (!normalizedConversationId) return false;
    const nextValue = !!value;
    const previousValue = getConversationPlanModeEnabledById(normalizedConversationId);
    if (!confirmedConversationPlanModeStates.has(normalizedConversationId)) {
      confirmedConversationPlanModeStates.set(normalizedConversationId, previousValue);
    }
    if (previousValue === nextValue) return true;
    patchConversationPlanModeInOverview(normalizedConversationId, nextValue);
    return queueConversationPlanModeUpdate(normalizedConversationId, async () => {
      try {
        await invokeTauri<{ conversationId: string; planModeEnabled: boolean }>("set_conversation_plan_mode", {
          input: {
            conversationId: normalizedConversationId,
            planModeEnabled: nextValue,
          },
        });
        confirmedConversationPlanModeStates.set(normalizedConversationId, nextValue);
        return true;
      } catch (error) {
        const fallbackValue = confirmedConversationPlanModeStates.get(normalizedConversationId) ?? previousValue;
        if (getConversationPlanModeEnabledById(normalizedConversationId) === nextValue) {
          patchConversationPlanModeInOverview(normalizedConversationId, fallbackValue);
        }
        console.warn("[计划模式] 保存会话计划状态失败", {
          conversationId: normalizedConversationId,
          nextValue,
          error,
        });
        return false;
      }
    });
  }

  async function setCurrentConversationPlanMode(value: boolean): Promise<boolean> {
    const conversationId = String(options.currentConversationId.value || "").trim();
    if (!conversationId) return false;
    return setConversationPlanMode(conversationId, value);
  }

  const currentConversationPlanModeEnabled = computed(() => {
    const conversationId = String(options.currentConversationId.value || "").trim();
    if (!conversationId) return false;
    return getConversationPlanModeEnabledById(conversationId);
  });

  return {
    currentConversationPlanModeEnabled,
    setConversationPlanMode,
    setCurrentConversationPlanMode,
    updatePlanModeEnabled: setCurrentConversationPlanMode,
  };
}

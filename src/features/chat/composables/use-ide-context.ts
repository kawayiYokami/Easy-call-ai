import { ref, computed, watch, onMounted, onBeforeUnmount, type Ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invokeTauri, isTauriRuntimeAvailable } from "../../../services/tauri-api";
import type { IdeContextQueryResult, IdeContextReferenceItem, IdeContextWorkspaceGroup, IdeContextWorkspaceInput, ShellWorkspace } from "../../../types/app";

interface UseIdeContextOptions {
  activeConversationId: Ref<string>;
  workspaces: Ref<ShellWorkspace[]>;
  currentWorkspaceRootPath: Ref<string>;
  currentWorkspaceName: Ref<string>;
  enabled?: Ref<boolean> | boolean;
}

export function useIdeContext(options: UseIdeContextOptions) {
  const { activeConversationId, workspaces, currentWorkspaceRootPath, currentWorkspaceName } = options;

  const ideContextGroups = ref<IdeContextWorkspaceGroup[]>([]);
  const attachedIdeContextReferences = ref<IdeContextReferenceItem[]>([]);
  const enabled = computed(() => {
    const configured = options.enabled;
    const configuredValue = typeof configured === "object" && configured && "value" in configured
      ? configured.value
      : configured;
    return configuredValue !== false && isTauriRuntimeAvailable();
  });

  let refreshTimer: ReturnType<typeof setInterval> | null = null;
  let eventUnlisten: UnlistenFn | null = null;
  let refreshSeq = 0;

  function normalizedWorkspaceInputs(): IdeContextWorkspaceInput[] {
    const sourceWorkspaces = Array.isArray(workspaces.value) ? workspaces.value : [];
    const normalized = sourceWorkspaces
      .map((ws) => ({
        path: String(ws?.path || "").trim(),
        name: String(ws?.name || "").trim() || undefined,
      }))
      .filter((ws) => !!ws.path);
    if (normalized.length > 0) return normalized;
    const fallbackPath = String(currentWorkspaceRootPath.value || "").trim();
    if (!fallbackPath) return [];
    return [{
      path: fallbackPath,
      name: String(currentWorkspaceName.value || "").trim() || undefined,
    }];
  }

  async function refresh() {
    if (!enabled.value) {
      ideContextGroups.value = [];
      return;
    }
    const wsInputs = normalizedWorkspaceInputs();
    if (wsInputs.length === 0) {
      ideContextGroups.value = [];
      return;
    }
    const seq = ++refreshSeq;
    try {
      const result = await invokeTauri<IdeContextQueryResult>("query_ide_context_references", {
        input: { workspaces: wsInputs },
      });
      if (seq !== refreshSeq) return;
      ideContextGroups.value = Array.isArray(result?.groups) ? result.groups : [];
    } catch (error) {
      if (seq === refreshSeq) ideContextGroups.value = [];
      console.warn("[IDE 上下文] 查询引用失败", error);
    }
  }

  function startTimer() {
    if (!enabled.value) return;
    stopTimer();
    refreshTimer = window.setInterval(() => void refresh(), 5000);
  }

  function stopTimer() {
    if (!refreshTimer) return;
    clearInterval(refreshTimer);
    refreshTimer = null;
  }

  async function startEventListener() {
    if (!enabled.value) return;
    stopEventListener();
    try {
      eventUnlisten = await listen("ide-context-updated", () => void refresh());
    } catch (error) {
      console.warn("[IDE 上下文] 监听更新事件失败", error);
    }
  }

  function stopEventListener() {
    if (!eventUnlisten) return;
    eventUnlisten();
    eventUnlisten = null;
  }

  function attachReference(reference: IdeContextReferenceItem) {
    if (attachedIdeContextReferences.value.some((item) => item.id === reference.id)) return;
    attachedIdeContextReferences.value = [...attachedIdeContextReferences.value, { ...reference }];
  }

  function removeReference(referenceId: string) {
    attachedIdeContextReferences.value = attachedIdeContextReferences.value.filter((item) => item.id !== referenceId);
  }

  function clearAttachedReferences() {
    attachedIdeContextReferences.value = [];
  }

  // 会话切换时清空附加引用并刷新
  watch(activeConversationId, () => {
    clearAttachedReferences();
    void refresh();
  });

  // 工作区路径变化时刷新
  watch(
    () => normalizedWorkspaceInputs().map((item) => `${item.path}\n${item.name || ""}`).join("|"),
    () => void refresh(),
    { immediate: true },
  );

  onMounted(() => {
    if (!enabled.value) return;
    void refresh();
    void startEventListener();
    startTimer();
  });

  onBeforeUnmount(() => {
    stopTimer();
    stopEventListener();
  });

  return {
    visibleIdeContextGroups: computed<IdeContextWorkspaceGroup[]>(() => ideContextGroups.value),
    attachedIdeContextReferences,
    attachReference,
    removeReference,
    clearAttachedReferences,
  };
}

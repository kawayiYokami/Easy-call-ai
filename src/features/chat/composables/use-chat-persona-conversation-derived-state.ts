import { computed } from "vue";
import type { ChatMentionEntry } from "../../../types/app";
import { departmentDirectChildIds } from "../../config/utils/department-graph";

export function useChatPersonaConversationDerivedState(bindings: Record<string, any>) {
  const userPersona = computed(
    () => bindings.personas.value.find((p: any) => p.isBuiltInUser || p.id === "user-persona") ?? null,
  );
  const assistantPersonas = computed(() =>
    bindings.personas.value.filter((p: any) =>
      !p.isBuiltInUser && !p.isBuiltInSystem && p.id !== "user-persona" && p.id !== "system-persona",
    ),
  );
  const assistantDepartmentPersona = computed(
    () =>
      assistantPersonas.value.find((p: any) => p.id === bindings.assistantDepartmentAgentId.value)
      ?? assistantPersonas.value[0]
      ?? null,
  );
  const currentForegroundConversationSummary = computed(() => {
    const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
    if (currentConversationId) {
      const matched = bindings.chatConversationItems.value.find(
        (item: any) => String(item.conversationId || "").trim() === currentConversationId,
      );
      if (matched) return matched;
    }
    return (
      bindings.unarchivedConversations.value.find((item: any) => !!item.isMainConversation)
      || bindings.unarchivedConversations.value[0]
      || null
    );
  });
  const currentForegroundDepartmentId = computed(
    () => String(currentForegroundConversationSummary.value?.departmentId || "").trim() || "assistant-department",
  );
  const currentForegroundDepartment = computed(
    () =>
      bindings.config.departments.find((item: any) => String(item.id || "").trim() === currentForegroundDepartmentId.value)
      || bindings.config.departments.find((item: any) => item.id === "assistant-department" || item.isBuiltInAssistant)
      || null,
  );
  const currentForegroundAgentId = computed(
    () =>
      String(currentForegroundConversationSummary.value?.agentId || "").trim()
      || String(currentForegroundDepartment.value?.agentIds?.[0] || "").trim()
      || String(bindings.assistantDepartmentAgentId.value || "").trim(),
  );
  const currentForegroundApiConfigId = computed(
    () => {
      if (bindings.detachedChatWindow.value) {
        const temporaryApiConfigId = String(bindings.detachedTemporaryApiConfigId.value || "").trim();
        if (temporaryApiConfigId && bindings.config.apiConfigs.some((item: any) => item.id === temporaryApiConfigId && item.enableText)) {
          return temporaryApiConfigId;
        }
      }
      return bindings.departmentConversationApiConfigId(currentForegroundDepartment.value);
    },
  );
  const currentForegroundApiConfig = computed(
    () => bindings.config.apiConfigs.find((a: any) => a.id === currentForegroundApiConfigId.value) ?? null,
  );
  const currentForegroundPersona = computed(
    () =>
      assistantPersonas.value.find((p: any) => p.id === currentForegroundAgentId.value)
      ?? assistantDepartmentPersona.value
      ?? assistantPersonas.value[0]
      ?? null,
  );
  const selectedPersonaEditor = computed(
    () => bindings.personas.value.find((p: any) => p.id === bindings.personaEditorId.value) ?? null,
  );
  const toolDepartment = computed(() =>
    bindings.config.departments.find((item: any) => item.id === "assistant-department" || item.isBuiltInAssistant)
    ?? bindings.config.departments.find((item: any) => (item.agentIds || []).includes(bindings.assistantDepartmentAgentId.value))
    ?? null,
  );
  const toolApiConfig = computed(() =>
    bindings.config.apiConfigs.find((a: any) => a.id === (toolDepartment.value?.apiConfigId || "")) ?? null,
  );
  const userAvatarUrl = computed(
    () => bindings.resolveAvatarUrl(userPersona.value?.avatarPath, userPersona.value?.avatarUpdatedAt),
  );
  const userPersonaAvatarUrl = computed(() => userAvatarUrl.value);
  const selectedPersonaAvatarUrl = computed(
    () => bindings.resolveAvatarUrl(assistantDepartmentPersona.value?.avatarPath, assistantDepartmentPersona.value?.avatarUpdatedAt),
  );
  const currentForegroundPersonaAvatarUrl = computed(
    () => bindings.resolveAvatarUrl(currentForegroundPersona.value?.avatarPath, currentForegroundPersona.value?.avatarUpdatedAt),
  );
  const selectedPersonaEditorAvatarUrl = computed(
    () => bindings.resolveAvatarUrl(selectedPersonaEditor.value?.avatarPath, selectedPersonaEditor.value?.avatarUpdatedAt),
  );
  const chatPersonaNameMap = computed<Record<string, string>>(() => {
    const next: Record<string, string> = {};
    for (const persona of bindings.personas.value) {
      const id = String(persona.id || "").trim();
      if (!id) continue;
      const name = String(persona.name || "").trim();
      next[id] = name || id;
    }
    return next;
  });
  const chatPersonaAvatarUrlMap = computed<Record<string, string>>(() => {
    const next: Record<string, string> = {};
    for (const persona of bindings.personas.value) {
      const id = String(persona.id || "").trim();
      if (!id) continue;
      const url = bindings.resolveAvatarUrl(persona.avatarPath, persona.avatarUpdatedAt);
      if (url) next[id] = url;
    }
    return next;
  });
  const chatMentionEntries = computed<ChatMentionEntry[]>(() => {
    const localeName = bindings.config.uiLanguage === "en-US" ? "en" : "zh-CN";
    const currentAgentId = String(currentForegroundAgentId.value || "").trim();
    const currentDepartmentId = String(currentForegroundDepartmentId.value || "").trim();
    const textCapableApiIds = new Set(
      (bindings.config.apiConfigs || [])
        .filter((api: any) => !!api.enableText && bindings.isTextRequestFormat(api.requestFormat))
        .map((api: any) => String(api.id || "").trim())
        .filter(Boolean),
    );
    const departmentsByPersonaId = new Map<string, typeof bindings.config.departments>();
    for (const department of bindings.config.departments || []) {
      for (const rawAgentId of department.agentIds || []) {
        const agentId = String(rawAgentId || "").trim();
        if (!agentId) continue;
        const current = departmentsByPersonaId.get(agentId) || [];
        current.push(department);
        departmentsByPersonaId.set(agentId, current);
      }
    }
    const items: ChatMentionEntry[] = [];
    for (const persona of bindings.personas.value) {
      if (persona.isBuiltInSystem || persona.id === "system-persona") continue;
      const agentId = String(persona.id || "").trim();
      if (!agentId) continue;
      const agentName = String(persona.name || "").trim() || agentId;
      const avatarUrl = String(chatPersonaAvatarUrlMap.value[agentId] || "").trim() || undefined;
      const backgroundTaskCount = bindings.agentWorkPresence.activeWorkCountForAgent(
        String(bindings.currentChatConversationId.value || "").trim(),
        agentId,
      );
      const boundDepartments = (departmentsByPersonaId.get(agentId) || [])
        .map((department: any) => ({
          departmentId: String(department.id || "").trim(),
          departmentName: String(department.name || "").trim() || String(department.id || "").trim(),
          apiConfigIds: bindings.departmentOrderedApiConfigIds(department),
        }))
        .filter((item: any, index: number, list: any[]) =>
          !!item.departmentId && list.findIndex((candidate) => candidate.departmentId === item.departmentId) === index,
        );

      if (boundDepartments.length === 0) {
        items.push({
          agentId,
          agentName,
          avatarUrl,
          departmentName: agentId === "user-persona" ? "用户" : "未归属部门",
          departmentNames: [],
          isFrontSpeaking: false,
          hasBackgroundTask: backgroundTaskCount > 0,
          mentionable: false,
          unavailableReason: agentId === "user-persona"
            ? bindings.t("chat.mentionUnavailableUserPersona")
            : bindings.t("chat.mentionUnavailableUnassigned"),
        });
        continue;
      }

      for (const department of boundDepartments) {
        const isCurrentRuntimeAgent = department.departmentId === currentDepartmentId && agentId === currentAgentId;
        const hasTextModel = department.apiConfigIds.some((apiConfigId: string) => textCapableApiIds.has(apiConfigId));
        let mentionable = true;
        let unavailableReason = "";
        if (agentId === "user-persona" || persona.isBuiltInUser) {
          mentionable = false;
          unavailableReason = bindings.t("chat.mentionUnavailableUserPersona");
        } else if (!department.departmentId) {
          mentionable = false;
          unavailableReason = bindings.t("chat.mentionUnavailableUnassigned");
        } else if (isCurrentRuntimeAgent) {
          mentionable = false;
          unavailableReason = bindings.t("chat.mentionUnavailableSelf");
        } else if (!currentDepartmentId) {
          mentionable = false;
          unavailableReason = bindings.t("chat.mentionUnavailableNoForegroundDepartment");
        } else if (!hasTextModel) {
          mentionable = false;
          unavailableReason = bindings.t("chat.mentionUnavailableNoModel");
        }
        items.push({
          agentId,
          agentName,
          avatarUrl,
          departmentId: department.departmentId,
          departmentName: department.departmentName,
          departmentNames: boundDepartments.map((item: any) => item.departmentName),
          isFrontSpeaking: isCurrentRuntimeAgent,
          hasBackgroundTask: backgroundTaskCount > 0,
          mentionable,
          unavailableReason: unavailableReason || undefined,
        });
      }
    }
    return items.sort((left, right) => {
      if (left.isFrontSpeaking !== right.isFrontSpeaking) return left.isFrontSpeaking ? -1 : 1;
      if (left.mentionable !== right.mentionable) return left.mentionable ? -1 : 1;
      if (left.hasBackgroundTask !== right.hasBackgroundTask) return left.hasBackgroundTask ? -1 : 1;
      if (left.agentId === "user-persona" && right.agentId !== "user-persona") return -1;
      if (right.agentId === "user-persona" && left.agentId !== "user-persona") return 1;
      const nameCompare = left.agentName.localeCompare(right.agentName, localeName);
      if (nameCompare !== 0) return nameCompare;
      return left.departmentName.localeCompare(right.departmentName, localeName);
    });
  });
  const createConversationDepartmentOptions = computed(() =>
    (bindings.config.departments || [])
      .filter((department: any) => {
        const departmentId = String(department.id || "").trim();
        if (!departmentId) return false;
        const apiConfigId = bindings.departmentConversationApiConfigId(department);
        if (!apiConfigId) return false;
        return bindings.config.apiConfigs.some((api: any) => api.id === apiConfigId && api.enableText);
      })
      .map((department: any) => {
        const ownerId = String((department.agentIds || [])[0] || "").trim();
        const owner = bindings.personas.value.find((persona: any) => String(persona.id || "").trim() === ownerId) ?? null;
        const apiConfigId = bindings.departmentConversationApiConfigId(department);
        const apiConfig = bindings.config.apiConfigs.find((api: any) => api.id === apiConfigId) ?? null;
        return {
          id: String(department.id || "").trim(),
          name: String(department.name || "").trim() || String(department.id || "").trim(),
          ownerAgentId: ownerId,
          ownerName: String(owner?.name || "").trim() || ownerId || "未设置负责人",
          providerName: String(apiConfig?.name || apiConfig?.id || "").trim(),
          modelName: String(apiConfig?.model || "").trim(),
        };
      }),
  );
  const delegateDepartmentIds = computed(() =>
    departmentDirectChildIds(currentForegroundDepartment.value, bindings.config.departments || []),
  );

  return {
    userPersona,
    assistantPersonas,
    assistantDepartmentPersona,
    currentForegroundConversationSummary,
    currentForegroundDepartmentId,
    currentForegroundDepartment,
    currentForegroundAgentId,
    currentForegroundApiConfigId,
    currentForegroundApiConfig,
    currentForegroundPersona,
    selectedPersonaEditor,
    toolDepartment,
    toolApiConfig,
    userAvatarUrl,
    userPersonaAvatarUrl,
    selectedPersonaAvatarUrl,
    currentForegroundPersonaAvatarUrl,
    selectedPersonaEditorAvatarUrl,
    chatPersonaNameMap,
    chatPersonaAvatarUrlMap,
    chatMentionEntries,
    createConversationDepartmentOptions,
    delegateDepartmentIds,
  };
}

import { computed } from "vue";
import { useConfigEditors } from "../../config/composables/use-config-editors";
import { useConfigPersistence } from "../../config/composables/use-config-persistence";
import { useConfigRuntime } from "../../config/composables/use-config-runtime";
import { useMemoryViewer } from "../../memory/composables/use-memory-viewer";
import { useChatDialogActions } from "./use-chat-dialog-actions";
import { usePromptPreview } from "./use-prompt-preview";
import { useShellDialogFlows } from "../../shell/composables/use-shell-dialog-flows";
import { useChatRuntime } from "./use-chat-runtime";

export function useChatServiceAssemblies(bindings: Record<string, any>) {
  const configRuntime = useConfigRuntime({
    t: bindings.tr,
    setStatus: bindings.setStatus,
    setStatusError: bindings.setStatusError,
    personas: bindings.personas,
    assistantDepartmentAgentId: bindings.assistantDepartmentAgentId,
    avatarSaving: bindings.avatarSaving,
    avatarError: bindings.avatarError,
    toolAgentId: bindings.assistantDepartmentAgentId,
    selectedApiConfig: bindings.selectedApiConfig,
    selectedApiProvider: bindings.selectedApiProvider,
    refreshingModels: bindings.refreshingModels,
    modelRefreshError: bindings.modelRefreshError,
    apiModelOptions: bindings.apiModelOptions,
    modelRefreshOkFlags: bindings.modelRefreshOkFlags,
    toolApiConfig: bindings.toolApiConfig,
    checkingToolsStatus: bindings.checkingToolsStatus,
    toolStatuses: bindings.toolStatuses,
    imageCacheStats: bindings.imageCacheStats,
    imageCacheStatsLoading: bindings.imageCacheStatsLoading,
    ensureAvatarCached: bindings.ensureAvatarCached,
  });
  const configPersistence = useConfigPersistence({
    t: bindings.tr,
    setStatus: bindings.setStatus,
    setStatusError: bindings.setStatusError,
    onSaveConfigError: bindings.openSettingsSaveErrorDialog,
    config: bindings.config,
    locale: bindings.locale,
    normalizeLocale: bindings.normalizeLocale,
    suppressAutosave: bindings.suppressAutosave,
    loading: bindings.loading,
    saving: bindings.saving,
    savingPersonas: bindings.personaSaving,
    personas: bindings.personas,
    assistantPersonas: bindings.assistantPersonas,
    assistantDepartmentAgentId: bindings.assistantDepartmentAgentId,
    personaEditorId: bindings.personaEditorId,
    userAlias: bindings.userAlias,
    selectedResponseStyleId: bindings.selectedResponseStyleId,
    selectedPdfReadMode: bindings.selectedPdfReadMode,
    backgroundVoiceScreenshotKeywords: bindings.backgroundVoiceScreenshotKeywords,
    backgroundVoiceScreenshotMode: bindings.backgroundVoiceScreenshotMode,
    instructionPresets: bindings.instructionPresets,
    responseStyleIds: bindings.responseStyleIds,
    createApiConfig: bindings.createApiConfig,
    normalizeApiBindingsLocal: bindings.normalizeApiBindingsLocal,
    buildConfigPayload: bindings.buildConfigPayload,
    buildConfigSnapshotJson: bindings.buildConfigSnapshotJson,
    buildPersonasSnapshotJson: bindings.buildPersonasSnapshotJson,
    lastSavedConfigJson: bindings.lastSavedConfigJson,
    lastSavedPersonasJson: bindings.lastSavedPersonasJson,
    syncUserAliasFromPersona: bindings.syncUserAliasFromPersona,
    preloadPersonaAvatars: bindings.preloadPersonaAvatars,
    syncTrayIcon: configRuntime.syncTrayIcon,
  });
  const configEditors = useConfigEditors({
    t: bindings.tr,
    config: bindings.config,
    personas: bindings.personas,
    assistantPersonas: bindings.assistantPersonas,
    assistantDepartmentAgentId: bindings.assistantDepartmentAgentId,
    personaEditorId: bindings.personaEditorId,
    selectedPersonaEditor: bindings.selectedPersonaEditor,
    createApiConfig: bindings.createApiConfig,
    createApiProvider: bindings.createApiProvider,
    normalizeApiBindingsLocal: bindings.normalizeApiBindingsLocal,
  });

  const chatRuntime = useChatRuntime({
    t: bindings.tr,
    setStatus: bindings.setStatus,
    setStatusError: bindings.setStatusError,
    setChatError: (text) => {
      bindings.chatErrorText.value = text;
    },
    setConversationRuntimeState: bindings.setConversationRuntimeState,
    activeChatApiConfigId: bindings.currentForegroundApiConfigId,
    assistantDepartmentAgentId: bindings.currentForegroundAgentId,
    currentConversationId: bindings.currentChatConversationId,
    trimmingConversationId: bindings.trimmingConversationId,
    compactingConversationId: bindings.compactingConversationId,
    chatting: bindings.chatting,
    trimming: bindings.trimming,
    compactingConversation: bindings.compactingConversation,
    suppressNextCompactionReload: bindings.suppressNextCompactionReload,
    allMessages: bindings.allMessages,
    refreshUnarchivedConversations: bindings.refreshChatUnarchivedConversations,
    perfNow: bindings.perfNow,
    perfLog: bindings.perfLog,
    perfDebug: bindings.PERF_DEBUG,
  });

  const promptPreviewActions = usePromptPreview({
    t: bindings.tr,
    currentConversationId: bindings.currentChatConversationId,
    localConversations: computed(() => bindings.unarchivedConversations.value),
  });
  const memoryViewerActions = useMemoryViewer({
    t: bindings.tr,
    setStatus: bindings.setStatus,
    setStatusError: bindings.setStatusError,
  });
  const shellDialogFlows = useShellDialogFlows({
    t: bindings.tr,
    configTab: bindings.configTab,
    allMessages: bindings.allMessages,
    tauriWindowLabel: bindings.tauriWindowLabel,
    currentForegroundApiConfigId: bindings.currentForegroundApiConfigId,
    currentForegroundAgentId: bindings.currentForegroundAgentId,
    currentForegroundDepartmentId: bindings.currentForegroundDepartmentId,
    currentChatConversationId: bindings.currentChatConversationId,
    unarchivedConversations: bindings.unarchivedConversations,
    setStatus: bindings.setStatus,
    setStatusError: bindings.setStatusError,
    trimCompactNow: chatRuntime.trimCompactNow,
    trimNow: chatRuntime.trimNow,
    deleteUnarchivedConversationFromArchives: bindings.deleteUnarchivedConversationFromArchives,
  });
  const chatDialogActions = useChatDialogActions({
    activeChatApiConfigId: bindings.currentForegroundApiConfigId,
    assistantDepartmentAgentId: bindings.currentForegroundAgentId,
    openPromptPreviewDialog: promptPreviewActions.openPromptPreview,
    openSystemPromptPreviewDialog: promptPreviewActions.openSystemPromptPreview,
  });

  return {
    configRuntime,
    configPersistence,
    configEditors,
    chatRuntime,
    promptPreviewActions,
    memoryViewerActions,
    shellDialogFlows,
    chatDialogActions,
  };
}

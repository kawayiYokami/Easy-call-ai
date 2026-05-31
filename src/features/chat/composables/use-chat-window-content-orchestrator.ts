import { useAvatarCache } from "./use-avatar-cache";
import { useChatConversationItemsDerivedState } from "./use-chat-conversation-items-derived-state";
import { useChatMessageBlocks } from "./use-chat-turns";
import { useChatPersonaConversationDerivedState } from "./use-chat-persona-conversation-derived-state";
import { useChatWindowBasicDerivedState } from "./use-chat-window-basic-derived-state";
import { useChatWindowLocalTools } from "./use-chat-window-local-tools";
import { useChatWindowMessageHelpers } from "./use-chat-window-message-helpers";
import { useTerminalApproval } from "../../shell/composables/use-terminal-approval";

export function useChatWindowContentOrchestrator(bindings: Record<string, any>) {
  const configDerived = bindings.configDerived;
  const avatarCache = useAvatarCache({ personas: bindings.personas });
  const conversationItems = useChatConversationItemsDerivedState({
    config: bindings.config,
    unarchivedConversations: bindings.unarchivedConversations,
    remoteImContactConversations: bindings.remoteImContactConversations,
    backgroundConversationBadgeMap: bindings.backgroundConversationBadgeMap,
  });
  const personaConversation = useChatPersonaConversationDerivedState({
    t: bindings.t,
    config: bindings.config,
    personas: bindings.personas,
    assistantDepartmentAgentId: bindings.assistantDepartmentAgentId,
    personaEditorId: bindings.personaEditorId,
    currentChatConversationId: bindings.currentChatConversationId,
    chatConversationItems: conversationItems.chatConversationItems,
    unarchivedConversations: bindings.unarchivedConversations,
    detachedChatWindow: bindings.detachedChatWindow,
    detachedTemporaryApiConfigId: bindings.detachedTemporaryApiConfigId,
    departmentConversationApiConfigId: configDerived.departmentConversationApiConfigId,
    departmentOrderedApiConfigIds: configDerived.departmentOrderedApiConfigIds,
    isTextRequestFormat: configDerived.isTextRequestFormat,
    resolveAvatarUrl: avatarCache.resolveAvatarUrl,
    agentWorkPresence: bindings.agentWorkPresence,
  });
  const messageHelpers = useChatWindowMessageHelpers({
    t: bindings.t,
    userPersona: personaConversation.userPersona,
    userAlias: bindings.userAlias,
    allMessages: bindings.allMessages,
    foregroundTailLatestReady: bindings.foregroundTailLatestReady,
  });
  const localTools = useChatWindowLocalTools({
    t: bindings.t,
    status: bindings.status,
    setStatus: bindings.setStatus,
    setStatusError: bindings.setStatusError,
    personaEditorId: bindings.personaEditorId,
    personaDirty: bindings.personaDirty,
    selectedPersonaEditor: personaConversation.selectedPersonaEditor,
    assistantDepartmentAgentId: bindings.assistantDepartmentAgentId,
    currentForegroundDepartmentId: personaConversation.currentForegroundDepartmentId,
    detachedChatWindow: bindings.detachedChatWindow,
    detachedTemporaryApiConfigId: bindings.detachedTemporaryApiConfigId,
    currentForegroundApiConfig: personaConversation.currentForegroundApiConfig,
    selectedResponseStyleId: bindings.selectedResponseStyleId,
    selectedPdfReadMode: bindings.selectedPdfReadMode,
    backgroundVoiceScreenshotKeywords: bindings.backgroundVoiceScreenshotKeywords,
    backgroundVoiceScreenshotMode: bindings.backgroundVoiceScreenshotMode,
    instructionPresets: bindings.instructionPresets,
    clipboardImages: bindings.clipboardImages,
    queuedAttachmentNotices: bindings.queuedAttachmentNotices,
    hasVisionFallback: configDerived.hasVisionFallback,
    config: bindings.config,
    applyDepartmentPrimaryApiConfigLocally: configDerived.applyDepartmentPrimaryApiConfigLocally,
  });
  const messageBlocks = useChatMessageBlocks({
    allMessages: bindings.allMessages,
    activeChatApiConfig: personaConversation.currentForegroundApiConfig,
    perfDebug: bindings.PERF_DEBUG,
    perfNow: bindings.perfNow,
    taskTriggerLabels: {
      goal: bindings.tr("config.task.fields.goal"),
      todo: bindings.tr("config.task.fields.todo"),
    },
  });
  const terminalApproval = useTerminalApproval({
    queue: bindings.terminalApprovalQueue,
    resolving: bindings.terminalApprovalResolving,
  });
  const basicState = useChatWindowBasicDerivedState({
    t: bindings.tr,
    viewMode: bindings.viewMode,
    maximized: bindings.maximized,
    detachedChatWindow: bindings.detachedChatWindow,
    trimming: bindings.trimming,
    compactingConversation: bindings.compactingConversation,
    currentForegroundPersona: personaConversation.currentForegroundPersona,
    currentChatConversationId: bindings.currentChatConversationId,
    visibleMessageBlocks: messageBlocks.visibleMessageBlocks,
    listConversationTerminalApprovals: terminalApproval.listConversationTerminalApprovals,
  });

  return {
    configDerived,
    avatarCache,
    conversationItems,
    personaConversation,
    messageHelpers,
    localTools,
    messageBlocks,
    terminalApproval,
    basicState,
  };
}

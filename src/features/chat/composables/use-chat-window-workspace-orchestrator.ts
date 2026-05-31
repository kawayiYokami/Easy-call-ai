import { computed } from "vue";
import { useChatWorkspace } from "./use-chat-workspace";
import { useChatWorkspacePickerFlow } from "./use-chat-workspace-picker-flow";

export function useChatWindowWorkspaceOrchestrator(bindings: Record<string, any>) {
  const workspace = useChatWorkspace({
    activeApiConfigId: bindings.currentForegroundApiConfigId,
    activeAgentId: bindings.currentForegroundAgentId,
    activeConversationId: computed(() => bindings.currentChatConversationId.value),
    setStatus: bindings.setStatus,
    setStatusError: bindings.setStatusError,
  });
  const picker = useChatWorkspacePickerFlow({
    chatWorkspaceChoices: workspace.chatWorkspaceChoices,
    chatWorkspaceAutonomousMode: workspace.chatWorkspaceAutonomousMode,
    openChatWorkspacePickerBase: workspace.openChatWorkspacePicker,
    closeChatWorkspacePickerBase: workspace.closeChatWorkspacePicker,
    saveChatWorkspaces: workspace.saveChatWorkspaces,
    setStatus: bindings.setStatus,
    setStatusError: bindings.setStatusError,
    workspaceAlreadyExistsText: bindings.tr("config.tools.workspaceAlreadyExists"),
  });

  return {
    ...workspace,
    ...picker,
  };
}

import { computed } from "vue";
import { useChatWorkspace } from "./use-chat-workspace";
import { useChatWorkspacePickerFlow } from "./use-chat-workspace-picker-flow";

export function useChatWindowWorkspaceOrchestrator(bindings: Record<string, any>) {
  const workspace = useChatWorkspace({
    activeConversationId: computed(() => bindings.currentChatConversationId.value),
    setStatus: bindings.setStatus,
    setStatusError: bindings.setStatusError,
  });
  const picker = useChatWorkspacePickerFlow({
    chatWorkspaceChoices: workspace.chatWorkspaceChoices,
    chatWorkspaceAutonomousMode: workspace.chatWorkspaceAutonomousMode,
    chatWorkspaceWorkMode: workspace.chatWorkspaceWorkMode,
    chatWorkspaceBranch: workspace.chatWorkspaceBranch,
    openChatWorkspacePickerBase: workspace.openChatWorkspacePicker,
    closeChatWorkspacePickerBase: workspace.closeChatWorkspacePicker,
    saveChatWorkspaces: workspace.saveChatWorkspaces,
    setStatus: bindings.setStatus,
    setStatusError: bindings.setStatusError,
    workspaceAlreadyExistsText: bindings.tr("config.tools.workspaceAlreadyExists"),
    worktreeRequiresApprovalText: bindings.tr("chat.workspaceWorktreeRequiresApproval"),
    worktreeUnavailableText: bindings.tr("chat.workspaceWorktreeUnavailable"),
    checkChatWorkspaceGitRoot: workspace.checkChatWorkspaceGitRoot,
  });

  return {
    ...workspace,
    ...picker,
  };
}

import { invokeTauri } from "../../../services/tauri-api";
import type { ShellWorkspace } from "../../../types/app";

export function useChatConversationActionsOrchestrator(bindings: Record<string, any>) {
  function normalizeSelectedMessageIds(messageIds: unknown): string[] {
    return Array.isArray(messageIds)
      ? messageIds
          .map((item) => String(item || "").trim())
          .filter((item, index, array) => !!item && array.indexOf(item) === index)
      : [];
  }

  async function createUnarchivedConversation(input?: { title?: string; departmentId?: string; copyCurrent?: boolean; importPath?: string; shellWorkspaces?: ShellWorkspace[]; shellAutonomousMode?: boolean }) {
    const departmentId =
      String(input?.departmentId || "").trim()
      || bindings.defaultCreateConversationDepartmentId.value;
    if (!departmentId) return;
    try {
      const copySourceConversationId = input?.copyCurrent
        ? String(bindings.currentChatConversationId.value || "").trim()
        : "";
      const importPath = String(input?.importPath || "").trim();
      const result = await invokeTauri<{
        conversationId: string;
        unarchivedConversations?: any[];
      }>(importPath ? "import_conversation_share_from_file" : "create_unarchived_conversation", importPath
        ? {
          input: {
            path: importPath,
            departmentId,
            title: String(input?.title || "").trim() || null,
            shellWorkspaces: input?.shellWorkspaces || null,
            shellAutonomousMode: Boolean(input?.shellAutonomousMode),
          },
        }
        : {
          input: {
            departmentId,
            title: String(input?.title || "").trim() || null,
            copySourceConversationId: copySourceConversationId || null,
            shellWorkspaces: input?.shellWorkspaces || null,
            shellAutonomousMode: Boolean(input?.shellAutonomousMode),
          },
        });
      const conversationId = String(result?.conversationId || "").trim();
      if (!conversationId) return;
      if (Array.isArray(result.unarchivedConversations)) {
        bindings.unarchivedConversations.value = result.unarchivedConversations;
      } else {
        await bindings.refreshUnarchivedConversationOverview();
      }
      await bindings.switchUnarchivedConversation(conversationId);
      if (importPath) {
        bindings.setStatus(bindings.tr("status.conversationShareImported"));
      }
    } catch (error) {
      bindings.setStatus(bindings.tr("status.conversationCreateFailed", { err: bindings.formatRequestFailed(error) }));
    }
  }

  async function branchConversationFromSelection(payload: { count: number; messageIds: string[] }) {
    const sourceConversationId = String(bindings.currentChatConversationId.value || "").trim();
    const selectedMessageIds = normalizeSelectedMessageIds(payload?.messageIds);
    if (
      !sourceConversationId
      || selectedMessageIds.length === 0
      || bindings.branchingConversation.value
      || bindings.forwardingConversationSelection.value
    ) return;
    bindings.branchingConversation.value = true;
    try {
      const result = await invokeTauri<{
        conversationId: string;
        title: string;
        warning?: string | null;
      }>("branch_unarchived_conversation_from_selection", {
        input: {
          sourceConversationId,
          selectedMessageIds,
        },
      });
      const conversationId = String(result?.conversationId || "").trim();
      if (!conversationId) return;
      await bindings.refreshUnarchivedConversationOverview();
      const warning = String(result?.warning || "").trim();
      if (bindings.detachedChatWindow.value) {
        try {
          await invokeTauri<{ conversationId: string; windowLabel: string }>("detach_current_conversation_to_window", {
            input: { conversationId },
          });
          if (warning) {
            bindings.setStatus(bindings.tr("status.conversationBranchOpenedWithWarning", { warning }));
          } else {
            bindings.setStatus(bindings.tr("status.conversationBranchOpened", { title: String(result?.title || "").trim() || conversationId }));
          }
        } catch (detachError) {
          console.error("[独立聊天窗口] 会话分支创建成功，但打开新独立窗口失败", detachError);
          bindings.setStatus(bindings.tr("status.conversationBranchDetachFailed", { err: bindings.formatRequestFailed(detachError) }));
        }
        return;
      }
      const snapshot = await bindings.requestConversationLightSnapshot(conversationId);
      bindings.applyConversationSnapshot(snapshot);
      if (warning) {
        bindings.setStatus(bindings.tr("status.conversationBranchCreatedWithWarning", { warning }));
      } else {
        bindings.setStatus(bindings.tr("status.conversationBranchCreated", { title: String(result?.title || "").trim() || conversationId }));
      }
    } catch (error) {
      bindings.setStatusError("status.loadMessagesFailed", error);
    } finally {
      bindings.branchingConversation.value = false;
    }
  }

  async function forwardConversationFromSelection(payload: {
    count: number;
    messageIds: string[];
    targetConversationId: string;
  }) {
    const sourceConversationId = String(bindings.currentChatConversationId.value || "").trim();
    const targetConversationId = String(payload?.targetConversationId || "").trim();
    const selectedMessageIds = normalizeSelectedMessageIds(payload?.messageIds);
    if (
      !sourceConversationId
      || !targetConversationId
      || selectedMessageIds.length === 0
      || bindings.trimming.value
      || bindings.branchingConversation.value
      || bindings.forwardingConversationSelection.value
    ) return;
    bindings.forwardingConversationSelection.value = true;
    try {
      const result = await invokeTauri<{
        targetConversationId: string;
        forwardedCount: number;
      }>("forward_unarchived_conversation_selection", {
        input: {
          sourceConversationId,
          targetConversationId,
          selectedMessageIds,
        },
      });
      const effectiveTargetConversationId = String(result?.targetConversationId || targetConversationId).trim();
      if (!effectiveTargetConversationId) return;
      await bindings.refreshUnarchivedConversationOverview();
      const snapshot = await bindings.requestConversationLightSnapshot(effectiveTargetConversationId);
      bindings.applyConversationSnapshot(snapshot);
      bindings.setStatus(bindings.tr("status.conversationSelectionForwarded", {
        count: Number(result?.forwardedCount || selectedMessageIds.length),
      }));
    } catch (error) {
      bindings.setStatusError("status.loadMessagesFailed", error);
    } finally {
      bindings.forwardingConversationSelection.value = false;
    }
  }

  async function userAsyncDelegateFromSelection(payload: {
    count: number;
    messageIds: string[];
    departmentId: string;
    presetId: string;
    background: string;
    question: string;
    focus: string;
  }) {
    const conversationId = String(bindings.currentChatConversationId.value || "").trim();
    const targetDepartmentId = String(payload?.departmentId || "").trim();
    const selectedMessageIds = normalizeSelectedMessageIds(payload?.messageIds);
    const question = String(payload?.question || "").trim();
    const focus = String(payload?.focus || "").trim();
    if (!conversationId || !targetDepartmentId || !question) return false;
    const sourceAgentId = String(bindings.currentForegroundAgentId.value || "").trim();
    const targetOwnerAgentId = String(
      bindings.createConversationDepartmentOptions.value.find((item: any) => item.id === targetDepartmentId)?.ownerAgentId || "",
    ).trim();
    if (sourceAgentId && targetOwnerAgentId && sourceAgentId === targetOwnerAgentId) {
      bindings.setStatus(bindings.tr("status.asyncDelegateSelfSyncOnly"));
      return false;
    }
    try {
      const result = await invokeTauri<{
        delegateId: string;
        conversationId: string;
        targetAgentId: string;
        targetAgentName: string;
        selectedMessageCount: number;
      }>("submit_user_async_delegate", {
        input: {
          conversationId,
          targetDepartmentId,
          presetId: String(payload?.presetId || "review").trim() || "review",
          background: String(payload?.background || "").trim(),
          question,
          focus,
          selectedMessageIds,
        },
      });
      const targetName = String(result?.targetAgentName || result?.targetAgentId || "").trim() || "子代理";
      const selectedCount = Number(result?.selectedMessageCount || selectedMessageIds.length);
      bindings.setStatus(selectedCount > 0
        ? bindings.tr("status.asyncDelegateStartedWithMessages", { name: targetName, count: selectedCount })
        : bindings.tr("status.asyncDelegateStarted", { name: targetName }));
      return true;
    } catch (error) {
      bindings.setStatus(bindings.tr("status.asyncDelegateFailed", { err: bindings.formatRequestFailed(error) }));
      return false;
    }
  }

  async function renameCurrentConversation(payload: { conversationId: string; title: string }) {
    const conversationId = String(payload?.conversationId || "").trim();
    const title = String(payload?.title || "").trim();
    if (!conversationId) return;
    try {
      const result = await invokeTauri<{ conversationId: string; title: string }>("rename_unarchived_conversation", {
        input: {
          conversationId,
          title,
        },
      });
      const nextTitle = String(result?.title || "").trim();
      bindings.unarchivedConversations.value = bindings.unarchivedConversations.value.map((item: any) =>
        String(item.conversationId || "").trim() === conversationId
          ? {
            ...item,
            title: nextTitle,
          }
          : item
      );
      bindings.setStatus(bindings.t("status.conversationRenamed"));
    } catch (error) {
      bindings.setStatusError("status.renameConversationFailed", error);
    }
  }

  async function toggleConversationPin(conversationId: string) {
    const cid = String(conversationId || "").trim();
    if (!cid) return;
    try {
      const result = await invokeTauri<{ conversationId: string; isPinned: boolean; pinIndex?: number | null }>("toggle_unarchived_conversation_pin", {
        input: {
          conversationId: cid,
        },
      });
      bindings.applyConversationPinUpdated({
        conversationId: String(result?.conversationId || cid).trim(),
        isPinned: !!result?.isPinned,
        pinIndex: Number.isFinite(Number(result?.pinIndex)) ? Number(result?.pinIndex) : undefined,
      });
    } catch (error) {
      bindings.setStatusError("status.requestFailed", error);
    }
  }

  return {
    createUnarchivedConversation,
    branchConversationFromSelection,
    forwardConversationFromSelection,
    userAsyncDelegateFromSelection,
    renameCurrentConversation,
    toggleConversationPin,
  };
}

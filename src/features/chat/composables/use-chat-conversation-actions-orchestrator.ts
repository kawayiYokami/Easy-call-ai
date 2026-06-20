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

  async function createUnarchivedConversation(input?: { title?: string; departmentId?: string; agentId?: string; copyCurrent?: boolean; importPath?: string; shellWorkspaces?: ShellWorkspace[]; shellAutonomousMode?: boolean }) {
    const departmentId =
      String(input?.departmentId || "").trim()
      || bindings.defaultCreateConversationDepartmentId.value;
    const selectedOption = Array.isArray(bindings.createConversationDepartmentOptions?.value)
      ? bindings.createConversationDepartmentOptions.value.find((item: any) =>
        String(item.departmentId || item.id || "").trim() === departmentId
        && (!input?.agentId || String(item.agentId || "").trim() === String(input.agentId || "").trim())
      )
      : null;
    const agentId = String(input?.agentId || selectedOption?.agentId || "").trim();
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
            agentId: agentId || null,
            title: String(input?.title || "").trim() || null,
            shellWorkspaces: input?.shellWorkspaces || null,
            shellAutonomousMode: Boolean(input?.shellAutonomousMode),
          },
        }
        : {
          input: {
            departmentId,
            agentId: agentId || null,
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

  async function createConversationBranchFromMessage(payload: { turnId: string; targetUserMessageId: string }) {
    const sourceConversationId = String(bindings.currentChatConversationId.value || "").trim();
    const turnMessageId = String(payload?.targetUserMessageId || payload?.turnId || "").trim();
    if (
      !sourceConversationId
      || !turnMessageId
      || bindings.branchingConversation.value
      || bindings.forwardingConversationSelection.value
    ) return;
    bindings.branchingConversation.value = true;
    try {
      const result = await invokeTauri<{
        conversationId: string;
        title: string;
        warning?: string | null;
      }>("create_conversation_branch_from_message", {
        input: {
          sourceConversationId,
          turnMessageId,
        },
      });
      const conversationId = String(result?.conversationId || "").trim();
      if (!conversationId) return;
      await bindings.refreshUnarchivedConversationOverview();
      if (bindings.detachedChatWindow.value) {
        try {
          await invokeTauri<{ conversationId: string; windowLabel: string }>("detach_current_conversation_to_window", {
            input: { conversationId },
          });
          bindings.setStatus(bindings.tr("status.conversationBranchOpened", { title: String(result?.title || "").trim() || conversationId }));
        } catch (detachError) {
          console.error("[独立聊天窗口] 从消息创建会话分支成功，但打开新独立窗口失败", detachError);
          bindings.setStatus(bindings.tr("status.conversationBranchDetachFailed", { err: bindings.formatRequestFailed(detachError) }));
        }
        return;
      }
      const snapshot = await bindings.requestConversationLightSnapshot(conversationId);
      bindings.applyConversationSnapshot(snapshot);
      bindings.setStatus(bindings.tr("status.conversationBranchCreated", { title: String(result?.title || "").trim() || conversationId }));
    } catch (error) {
      bindings.setStatusError("status.loadMessagesFailed", error);
    } finally {
      bindings.branchingConversation.value = false;
    }
  }

  async function forwardConversationFromSelection(payload: {
    count: number;
    messageIds: string[];
    target: {
      kind: "local_unarchived" | "remote_im_contact";
      conversationId: string;
      remoteContactId?: string;
    };
  }) {
    const sourceConversationId = String(bindings.currentChatConversationId.value || "").trim();
    const targetKind = payload?.target?.kind === "remote_im_contact" ? "remote_im_contact" : "local_unarchived";
    const targetConversationId = String(payload?.target?.conversationId || "").trim();
    const remoteContactId = String(payload?.target?.remoteContactId || "").trim();
    const selectedMessageIds = normalizeSelectedMessageIds(payload?.messageIds);
    if (
      !sourceConversationId
      || !targetConversationId
      || (targetKind === "remote_im_contact" && !remoteContactId)
      || selectedMessageIds.length === 0
      || bindings.trimming.value
      || bindings.branchingConversation.value
      || bindings.forwardingConversationSelection.value
    ) return;
    bindings.forwardingConversationSelection.value = true;
    try {
      if (targetKind === "remote_im_contact") {
        const result = await invokeTauri<{
          targetConversationId: string;
          remoteContactId: string;
          forwardedCount: number;
        }>("forward_selection_to_remote_im_contact", {
          input: {
            sourceConversationId,
            targetConversationId,
            remoteContactId,
            selectedMessageIds,
          },
        });
        const effectiveTargetConversationId = String(result?.targetConversationId || targetConversationId).trim();
        if (!effectiveTargetConversationId) return;
        await bindings.refreshUnarchivedConversationOverview();
        await bindings.refreshRemoteImConversationOverview();
        await bindings.switchRemoteImContactConversation(
          String(result?.remoteContactId || remoteContactId).trim(),
        );
        bindings.setStatus(bindings.tr("status.conversationSelectionForwardedToRemoteContact", {
          count: Number(result?.forwardedCount || selectedMessageIds.length),
        }));
      } else {
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
      }
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
    agentId: string;
    presetId: string;
    why: string;
    goal: string;
    todo: string;
  }) {
    const conversationId = String(bindings.currentChatConversationId.value || "").trim();
    const targetDepartmentId = String(payload?.departmentId || "").trim();
    const targetAgentId = String(payload?.agentId || "").trim();
    const selectedMessageIds = normalizeSelectedMessageIds(payload?.messageIds);
    const goal = String(payload?.goal || "").trim();
    const todo = String(payload?.todo || "").trim();
    if (!conversationId || !targetDepartmentId || !targetAgentId || !goal) return false;
    const sourceAgentId = String(bindings.currentForegroundAgentId.value || "").trim();
    if (sourceAgentId && sourceAgentId === targetAgentId) {
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
          targetAgentId,
          presetId: String(payload?.presetId || "review").trim() || "review",
          why: String(payload?.why || "").trim(),
          goal,
          todo,
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
    createConversationBranchFromMessage,
    forwardConversationFromSelection,
    userAsyncDelegateFromSelection,
    renameCurrentConversation,
    toggleConversationPin,
  };
}

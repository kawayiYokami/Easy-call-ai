import { ref, type Ref } from "vue";
import { i18n } from "../../../i18n";
import { invokeTauri } from "../../../services/tauri-api";
import type { ChatMessage, RuntimeLogEntry, UnarchivedConversationSummary } from "../../../types/app";
import { useConfigSaveErrorDialog } from "./use-config-save-error-dialog";

const t = i18n.global.t;

export type TrimPreviewResult = {
  conversationId: string;
  canArchive: boolean;
  canDropConversation?: boolean;
  deleteOnly?: boolean;
  messageCount: number;
  bodyTextLength?: number;
  hasAssistantReply: boolean;
  isEmpty: boolean;
  archiveDisabledReason?: string | null;
};

export type TrimCompactionPreviewResult = {
  conversationId: string;
  canCompact: boolean;
  messageCount: number;
  hasAssistantReply: boolean;
  isEmpty: boolean;
  contextUsagePercent: number;
  compactionDisabledReason?: string | null;
};

type RecallMode = "with_patch" | "message_only" | "cancel";

type RewindConversationPreviewResult = {
  conversationId: string;
  canUndoPatch: boolean;
  hint: string;
};

const SHORT_CONVERSATION_COMPACTION_THRESHOLD = 10;

type UseShellDialogFlowsOptions = {
  t: (key: string, params?: Record<string, unknown>) => string;
  configTab: Ref<string>;
  allMessages: Ref<ChatMessage[]>;
  tauriWindowLabel: Ref<string>;
  currentForegroundApiConfigId: Ref<string>;
  currentForegroundAgentId: Ref<string>;
  currentForegroundDepartmentId: Ref<string>;
  currentChatConversationId: Ref<string>;
  unarchivedConversations: Ref<UnarchivedConversationSummary[]>;
  setStatus: (message: string) => void;
  setStatusError: (key: string, error: unknown) => void;
  trimCompactNow: () => Promise<void>;
  trimNow: (conversationId?: string | null) => Promise<void>;
  deleteConversation: (conversationId: string) => Promise<void> | void;
};

export function useShellDialogFlows(options: UseShellDialogFlowsOptions) {
  const runtimeLogsDialogOpen = ref(false);
  const runtimeLogs = ref<RuntimeLogEntry[]>([]);
  const runtimeLogsLoading = ref(false);
  const runtimeLogsError = ref("");
  const configSaveErrorDialog = useConfigSaveErrorDialog({
    t: options.t,
    configTab: options.configTab,
  });
  const skillPlaceholderDialogOpen = ref(false);
  const trimActionDialogOpen = ref(false);
  const trimPreviewLoading = ref(false);
  const trimPreview = ref<TrimPreviewResult | null>(null);
  const trimCompactionPreview = ref<TrimCompactionPreviewResult | null>(null);
  const rewindConfirmDialogOpen = ref(false);
  const rewindConfirmCanUndoPatch = ref(false);
  const rewindConfirmUndoHint = ref("");
  let rewindConfirmResolver: ((mode: RecallMode) => void) | null = null;

  function closeTrimActionDialog() {
    trimActionDialogOpen.value = false;
    trimPreviewLoading.value = false;
    trimPreview.value = null;
    trimCompactionPreview.value = null;
  }

  function currentUnarchivedConversationSummary(): UnarchivedConversationSummary | null {
    const conversationId = String(options.currentChatConversationId.value || "").trim();
    if (!conversationId) return null;
    return options.unarchivedConversations.value.find(
      (item) => String(item.conversationId || "").trim() === conversationId,
    ) ?? null;
  }

  function countArchiveCandidateMessages(messages: ChatMessage[]): number {
    return messages.filter((message) => {
      const role = String(message.role || "").trim().toLowerCase();
      return role === "user" || role === "assistant";
    }).length;
  }

  function hasAssistantReply(messages: ChatMessage[]): boolean {
    return messages.some((message) => String(message.role || "").trim().toLowerCase() === "assistant");
  }

  function latestBackendContextUsagePercent(messages: ChatMessage[]): number {
    for (let idx = messages.length - 1; idx >= 0; idx -= 1) {
      const message = messages[idx];
      if (String(message.role || "").trim().toLowerCase() !== "assistant") continue;
      const raw = Number((message.providerMeta || {}).contextUsagePercent);
      if (!Number.isFinite(raw)) continue;
      return Math.min(100, Math.max(0, Math.round(raw)));
    }
    return 0;
  }

  function buildTrimCompactionPreview(conversationId: string, archivePreview?: TrimPreviewResult | null): TrimCompactionPreviewResult {
    const messages = options.allMessages.value || [];
    const summary = currentUnarchivedConversationSummary();
    const messageCount = countArchiveCandidateMessages(messages);
    const assistantReplyPresent = hasAssistantReply(messages);
    const isEmpty = messages.length === 0;
    const contextUsagePercent = latestBackendContextUsagePercent(messages);
    const conversationLongEnough = messageCount >= SHORT_CONVERSATION_COMPACTION_THRESHOLD;
    const contextUsageHighEnough = contextUsagePercent >= 10;
    let compactionDisabledReason: string | null = null;
    if (summary?.runtimeState === "organizing_context") {
      compactionDisabledReason = t('sidebar.compactRunning');
    } else if (isEmpty) {
      compactionDisabledReason = t('sidebar.compactEmpty');
    } else if (!assistantReplyPresent) {
      compactionDisabledReason = t('sidebar.compactNoAssistant');
    } else if (!conversationLongEnough && !contextUsageHighEnough) {
      compactionDisabledReason = contextUsagePercent > 0
        ? t('sidebar.compactShortWithUsage', { count: messageCount, percent: contextUsagePercent })
        : t('sidebar.compactShort', { count: messageCount });
    }
    return {
      conversationId,
      canCompact: !compactionDisabledReason,
      messageCount,
      hasAssistantReply: assistantReplyPresent,
      isEmpty,
      contextUsagePercent,
      compactionDisabledReason,
    };
  }

  async function openTrimActionDialog() {
    const conversationId = String(options.currentChatConversationId.value || "").trim();
    if (!conversationId) {
      options.setStatus(t('sidebar.noConversation'));
      return;
    }
    console.info("[会话归档] 打开归档/压缩面板", { conversationId });
    trimActionDialogOpen.value = false;
    trimPreviewLoading.value = true;
    trimPreview.value = null;
    trimCompactionPreview.value = null;
    try {
      const archivePreview = await invokeTauri<TrimPreviewResult>("preview_trim_current_conversation", {
        input: { conversationId },
      });
      const compactionPreview = buildTrimCompactionPreview(conversationId, archivePreview);
      trimPreview.value = archivePreview;
      trimCompactionPreview.value = compactionPreview;
      trimActionDialogOpen.value = true;
    } catch (error) {
      console.warn("[会话归档] 加载归档/压缩面板失败", { conversationId, error });
      closeTrimActionDialog();
      options.setStatusError("status.loadConversationActionPreviewFailed", error);
    } finally {
      trimPreviewLoading.value = false;
    }
  }

  async function confirmTrimCompactionAction() {
    if (!trimCompactionPreview.value?.canCompact) return;
    closeTrimActionDialog();
    await options.trimCompactNow();
  }

  async function confirmTrimAction() {
    if (!trimPreview.value?.canArchive) return;
    const conversationId = String(trimPreview.value.conversationId || "").trim();
    console.info("[会话归档] 确认归档当前会话", {
      conversationId,
    });
    closeTrimActionDialog();
    await options.trimNow(conversationId || null);
  }

  async function confirmTrimDeleteAction() {
    const preview = trimPreview.value;
    if (!preview?.canDropConversation) return;
    const conversationId = String(preview.conversationId || "").trim();
    if (!conversationId) return;
    console.info("[会话归档] 确认删除会话", {
      conversationId,
      messageCount: Number(preview.messageCount || 0),
      bodyTextLength: Number(preview.bodyTextLength || 0),
    });
    closeTrimActionDialog();
    await options.deleteConversation(conversationId);
  }

  function openSkillPlaceholderDialog() {
    skillPlaceholderDialogOpen.value = true;
  }

  function closeSkillPlaceholderDialog() {
    skillPlaceholderDialogOpen.value = false;
  }

  async function getUndoAvailabilityForTurn(targetUserMessageId: string): Promise<{ canUndo: boolean; hint: string }> {
    const conversationId = String(options.currentChatConversationId.value || "").trim();
    const messageId = String(targetUserMessageId || "").trim();
    if (!messageId || !conversationId) {
      return { canUndo: false, hint: "缺少撤回预览所需的会话上下文。" };
    }
    try {
      const preview = await invokeTauri<RewindConversationPreviewResult>("preview_rewind_conversation_from_message", {
        input: {
          session: {
            agentId: "",
            conversationId,
          },
          messageId,
          undoApplyPatch: false,
        },
      });
      return {
        canUndo: !!preview.canUndoPatch,
        hint: String(preview.hint || "").trim(),
      };
    } catch (error) {
      console.warn("[会话撤回] 撤回预览失败，隐藏文件修改撤回入口", {
        messageId,
        conversationId,
        error,
      });
      return { canUndo: false, hint: "撤回预览失败，仅撤回消息。" };
    }
  }

  async function requestRecallMode(payload: { turnId: string; targetUserMessageId: string }): Promise<RecallMode> {
    cancelPendingRewindConfirm();
    const availability = await getUndoAvailabilityForTurn(payload.targetUserMessageId);
    console.info("[会话撤回] 打开撤回弹窗", {
      turnId: payload.turnId,
      targetUserMessageId: payload.targetUserMessageId,
      canUndoPatch: availability.canUndo,
      hint: availability.hint || "",
    });
    rewindConfirmCanUndoPatch.value = availability.canUndo;
    rewindConfirmUndoHint.value = availability.hint;
    rewindConfirmDialogOpen.value = true;
    return new Promise((resolve) => {
      rewindConfirmResolver = resolve;
    });
  }

  function resolveRewindConfirm(mode: RecallMode) {
    console.info("[会话撤回] 弹窗确认", {
      mode,
      canUndoPatch: rewindConfirmCanUndoPatch.value,
      dialogOpen: rewindConfirmDialogOpen.value,
    });
    const resolver = rewindConfirmResolver;
    rewindConfirmResolver = null;
    rewindConfirmDialogOpen.value = false;
    rewindConfirmCanUndoPatch.value = false;
    rewindConfirmUndoHint.value = "";
    if (resolver) {
      resolver(mode);
    }
  }

  function confirmRewindWithPatch() {
    console.info("[会话撤回] 点击：撤回消息并撤回修改");
    resolveRewindConfirm("with_patch");
  }

  function confirmRewindMessageOnly() {
    console.info("[会话撤回] 点击：仅撤回消息");
    resolveRewindConfirm("message_only");
  }

  function cancelRewindConfirm() {
    console.info("[会话撤回] 点击：取消撤回");
    resolveRewindConfirm("cancel");
  }

  function cancelPendingRewindConfirm() {
    if (!rewindConfirmResolver) {
      rewindConfirmDialogOpen.value = false;
      rewindConfirmCanUndoPatch.value = false;
      rewindConfirmUndoHint.value = "";
      return;
    }
    const resolver = rewindConfirmResolver;
    rewindConfirmResolver = null;
    rewindConfirmDialogOpen.value = false;
    rewindConfirmCanUndoPatch.value = false;
    rewindConfirmUndoHint.value = "";
    resolver("cancel");
  }

  async function refreshRuntimeLogs() {
    runtimeLogsLoading.value = true;
    runtimeLogsError.value = "";
    try {
      const items = await invokeTauri<RuntimeLogEntry[]>("list_recent_runtime_logs");
      runtimeLogs.value = items;
    } catch (error) {
      runtimeLogsError.value = t('sidebar.loadRuntimeLogsFailed', { error: String(error) });
    } finally {
      runtimeLogsLoading.value = false;
    }
  }

  function openRuntimeLogsDialog() {
    void invokeTauri("open_runtime_logs_window").catch((err) => {
      console.warn("[运行日志] 打开日志窗口失败", err);
    });
  }

  function closeRuntimeLogsDialog() {
    runtimeLogsDialogOpen.value = false;
  }

  async function clearRuntimeLogs() {
    runtimeLogsLoading.value = true;
    runtimeLogsError.value = "";
    try {
      await invokeTauri("clear_recent_runtime_logs");
      runtimeLogs.value = [];
    } catch (error) {
      runtimeLogsError.value = t('sidebar.clearRuntimeLogsFailed', { error: String(error) });
    } finally {
      runtimeLogsLoading.value = false;
    }
  }

  return {
    runtimeLogsDialogOpen,
    runtimeLogs,
    runtimeLogsLoading,
    runtimeLogsError,
    ...configSaveErrorDialog,
    skillPlaceholderDialogOpen,
    trimActionDialogOpen,
    trimPreviewLoading,
    trimPreview,
    trimCompactionPreview,
    rewindConfirmDialogOpen,
    rewindConfirmCanUndoPatch,
    rewindConfirmUndoHint,
    openTrimActionDialog,
    closeTrimActionDialog,
    confirmTrimCompactionAction,
    confirmTrimAction,
    confirmTrimDeleteAction,
    openSkillPlaceholderDialog,
    closeSkillPlaceholderDialog,
    requestRecallMode,
    confirmRewindWithPatch,
    confirmRewindMessageOnly,
    cancelRewindConfirm,
    cancelPendingRewindConfirm,
    refreshRuntimeLogs,
    openRuntimeLogsDialog,
    closeRuntimeLogsDialog,
    clearRuntimeLogs,
  };
}

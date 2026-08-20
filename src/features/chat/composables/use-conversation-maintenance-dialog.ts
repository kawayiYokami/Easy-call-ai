import { ref, type Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";
import type { ChatMessage } from "../../../types/app";
import { estimateConversationTokens } from "../../../utils/chat-message";

export type ConversationMaintenanceSummary = {
  conversationId: string;
  messageCount?: number;
  bodyMessageCount?: number;
  bodyTextLength?: number;
  hasAssistantReply?: boolean;
  runtimeState?: string;
  isSystemNotificationConversation?: boolean;
};

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
  /** 词元账单：系统提示词 / 工具 schema / 正文（按真实 usage 比例分配，均可能缺失）。 */
  tokenBreakdown?: {
    systemTokens?: number;
    toolsTokens?: number;
    messageTokens?: number;
    contextWindowTokens?: number;
  };
};

type ConversationBlockPageOutput = {
  selectedBlockId: number;
  messages: ChatMessage[];
};

type UseConversationMaintenanceDialogOptions = {
  t: (key: string, params?: Record<string, unknown>) => string;
  currentConversationId: Readonly<Ref<string>>;
  conversationSummaries: Readonly<Ref<readonly ConversationMaintenanceSummary[]>>;
  chatUsagePercent: Readonly<Ref<number>>;
  trimCompactNow: () => Promise<void>;
  trimNow: (conversationId?: string | null) => Promise<void>;
  deleteConversation: (conversationId: string) => Promise<void> | void;
  setStatus: (message: string) => void;
  setStatusError: (key: string, error: unknown) => void;
};

const SHORT_CONVERSATION_COMPACTION_THRESHOLD = 10;

export function useConversationMaintenanceDialog(options: UseConversationMaintenanceDialogOptions) {
  const trimActionDialogOpen = ref(false);
  const trimPreviewLoading = ref(false);
  const trimPreview = ref<TrimPreviewResult | null>(null);
  const trimCompactionPreview = ref<TrimCompactionPreviewResult | null>(null);

  function closeTrimActionDialog() {
    trimActionDialogOpen.value = false;
    trimPreviewLoading.value = false;
    trimPreview.value = null;
    trimCompactionPreview.value = null;
  }

  function currentConversationSummary(): ConversationMaintenanceSummary | null {
    const conversationId = String(options.currentConversationId.value || "").trim();
    if (!conversationId) return null;
    return options.conversationSummaries.value.find(
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

  function readTokenBreakdown(messages: ChatMessage[]): TrimCompactionPreviewResult["tokenBreakdown"] {
    const meta = (() => {
      for (let index = messages.length - 1; index >= 0; index -= 1) {
        const message = messages[index];
        if (String(message.role || "").trim().toLowerCase() !== "assistant") continue;
        const providerMeta = (message.providerMeta || {}) as Record<string, unknown>;
        const breakdown = providerMeta.contextBreakdown;
        if (breakdown && typeof breakdown === "object") {
          return breakdown as Record<string, unknown>;
        }
      }
      return undefined;
    })();
    const readTokens = (value: unknown): number | undefined => {
      const next = Math.round(Number(value) || 0);
      return next > 0 ? next : undefined;
    };
    const backendMessageTokens = readTokens(meta?.messageTokens);
    const breakdown: NonNullable<TrimCompactionPreviewResult["tokenBreakdown"]> = {
      systemTokens: readTokens(meta?.systemTokens),
      toolsTokens: readTokens(meta?.toolsTokens),
      messageTokens:
        backendMessageTokens ?? Math.max(0, Math.ceil(estimateConversationTokens(messages))),
    };
    // 上下文窗口大小取自最后一条 assistant 消息的 providerMeta（后端落库）。
    const contextWindowTokens = readTokens(
      (() => {
        for (let index = messages.length - 1; index >= 0; index -= 1) {
          const message = messages[index];
          if (String(message.role || "").trim().toLowerCase() !== "assistant") continue;
          const providerMeta = (message.providerMeta || {}) as Record<string, unknown>;
          if (providerMeta.contextWindowTokens != null) return providerMeta.contextWindowTokens;
        }
        return undefined;
      })(),
    );
    if (contextWindowTokens != null) {
      breakdown.contextWindowTokens = contextWindowTokens;
    }
    return breakdown;
  }

  function buildTrimCompactionPreview(
    conversationId: string,
    lastBlockMessages: ChatMessage[],
  ): TrimCompactionPreviewResult {
    const summary = currentConversationSummary();
    const messageCount = countArchiveCandidateMessages(lastBlockMessages);
    const assistantReplyPresent = hasAssistantReply(lastBlockMessages);
    const isEmpty = lastBlockMessages.length === 0;
    const contextUsagePercent = Math.min(100, Math.max(0, Math.round(Number(options.chatUsagePercent.value || 0))));
    const conversationLongEnough = messageCount >= SHORT_CONVERSATION_COMPACTION_THRESHOLD;
    const contextUsageHighEnough = contextUsagePercent >= 10;
    let compactionDisabledReason: string | null = null;
    if (summary?.runtimeState === "organizing_context" || summary?.runtimeState === "compacting") {
      compactionDisabledReason = options.t("sidebar.compactRunning");
    } else if (isEmpty) {
      compactionDisabledReason = options.t("sidebar.compactEmpty");
    } else if (!assistantReplyPresent) {
      compactionDisabledReason = options.t("sidebar.compactNoAssistant");
    } else if (!conversationLongEnough && !contextUsageHighEnough) {
      compactionDisabledReason = contextUsagePercent > 0
        ? options.t("sidebar.compactShortWithUsage", { count: messageCount, percent: contextUsagePercent })
        : options.t("sidebar.compactShort", { count: messageCount });
    }
    return {
      conversationId,
      canCompact: !compactionDisabledReason,
      messageCount,
      hasAssistantReply: assistantReplyPresent,
      isEmpty,
      contextUsagePercent,
      compactionDisabledReason,
      tokenBreakdown: readTokenBreakdown(lastBlockMessages),
    };
  }

  function buildTrimArchivePreview(conversationId: string): TrimPreviewResult {
    const summary = currentConversationSummary();
    const messageCount = Math.max(0, Number(summary?.bodyMessageCount ?? summary?.messageCount ?? 0));
    const assistantReplyPresent = Boolean(summary?.hasAssistantReply);
    const isEmpty = messageCount === 0;
    const bodyTextLength = Math.max(0, Number(summary?.bodyTextLength ?? 0));
    let archiveDisabledReason: string | null = null;
    if (summary?.isSystemNotificationConversation) {
      archiveDisabledReason = "系统通知会话暂不支持归档。";
    } else if (summary?.runtimeState === "organizing_context" || summary?.runtimeState === "compacting") {
      archiveDisabledReason = options.t("sidebar.compactRunning");
    }
    return {
      conversationId,
      canArchive: !archiveDisabledReason,
      canDropConversation: !summary?.isSystemNotificationConversation,
      deleteOnly: false,
      messageCount,
      bodyTextLength,
      hasAssistantReply: assistantReplyPresent,
      isEmpty,
      archiveDisabledReason,
    };
  }

  async function openTrimActionDialog() {
    const conversationId = String(options.currentConversationId.value || "").trim();
    if (!conversationId) {
      options.setStatus(options.t("sidebar.noConversation"));
      return;
    }
    console.info("[会话归档] 打开归档/压缩面板", { conversationId });
    trimActionDialogOpen.value = false;
    trimPreviewLoading.value = true;
    trimPreview.value = null;
    trimCompactionPreview.value = null;
    try {
      const archivePreview = buildTrimArchivePreview(conversationId);
      const blockPage = await invokeTauri<ConversationBlockPageOutput>("conversation.blockPage", {
        input: { conversationId },
      });
      const compactionPreview = buildTrimCompactionPreview(
        conversationId,
        Array.isArray(blockPage?.messages) ? blockPage.messages : [],
      );
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
    console.info("[会话归档] 确认归档当前会话", { conversationId });
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

  return {
    trimActionDialogOpen,
    trimPreviewLoading,
    trimPreview,
    trimCompactionPreview,
    openTrimActionDialog,
    closeTrimActionDialog,
    confirmTrimCompactionAction,
    confirmTrimAction,
    confirmTrimDeleteAction,
  };
}

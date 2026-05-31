import { ref, watch, type Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";

export type ToolReviewStoredReview = {
  kind: string;
  allow: boolean;
  reviewOpinion: string;
  modelName: string;
  rawContent?: string;
};

export type ToolReviewItemSummary = {
  callId: string;
  toolName: string;
  orderIndex: number;
  hasReview: boolean;
  affectedPaths?: string[];
  patchOperation?: "add" | "update" | "delete" | "mixed" | string;
  command?: string;
};

export type ToolReviewReportRecord = {
  id: string;
  conversationId: string;
  title: string;
  status: "pending" | "failed" | "success" | string;
  scope: string;
  target: string;
  departmentId?: string;
  delegateId?: string;
  workspacePath: string;
  createdAt: string;
  updatedAt: string;
  reportText: string;
  errorText?: string;
};

export type ToolReviewCodeReviewScope = "uncommitted" | "main" | "commit" | "custom";

export type ToolReviewCommitOption = {
  hash: string;
  shortHash: string;
  subject: string;
  authorTime: string;
};

export type ToolReviewCommitPage = {
  total: number;
  page: number;
  pageSize: number;
  commits: ToolReviewCommitOption[];
};

export type ToolReviewBatchSummary = {
  batchKey: string;
  userMessageId: string;
  userMessageText: string;
  itemCount: number;
  unreviewedCount: number;
  items: ToolReviewItemSummary[];
};

export type ToolReviewItemDetail = {
  batchKey: string;
  callId: string;
  messageId: string;
  toolName: string;
  orderIndex: number;
  hasReview: boolean;
  previewKind: string;
  previewText: string;
  resultText: string;
  review?: ToolReviewStoredReview;
};

type ToolReviewBatchListOutput = {
  batches: ToolReviewBatchSummary[];
  currentBatchKey?: string;
};

type SubmitToolReviewTaskOutput = {
  report: ToolReviewReportRecord;
};

type ListToolReviewReportsOutput = {
  reports: ToolReviewReportRecord[];
};

type SubmitToolReviewCodeInput = {
  conversationId: string;
  scope: ToolReviewCodeReviewScope;
  target?: string;
  departmentId?: string;
};

type DeleteToolReviewReportInput = {
  conversationId: string;
  reportId: string;
};

type ListToolReviewCommitOptionsOutput = {
  total: number;
  page: number;
  pageSize: number;
  commits: ToolReviewCommitOption[];
};

type UseChatToolReviewOptions = {
  activeConversationId: Ref<string>;
  refreshTick: Ref<number>;
  initialPanelOpen?: Ref<boolean>;
  t: (key: string, params?: Record<string, unknown>) => string;
  onRefreshMessage?: (input: { conversationId: string; messageId: string }) => void | Promise<void>;
};

export function useChatToolReview(options: UseChatToolReviewOptions) {
  const toolReviewPanelOpen = ref(!!options.initialPanelOpen?.value);
  const toolReviewBatches = ref<ToolReviewBatchSummary[]>([]);
  const toolReviewCurrentBatchKey = ref("");
  const toolReviewDetailMap = ref<Record<string, ToolReviewItemDetail>>({});
  const toolReviewDetailLoadingCallId = ref("");
  const toolReviewReviewingCallId = ref("");
  const toolReviewBatchReviewingKey = ref("");
  const toolReviewSubmittingBatchKey = ref("");
  const toolReviewErrorText = ref("");
  const toolReviewReportErrorText = ref("");
  const toolReviewReports = ref<ToolReviewReportRecord[]>([]);
  const toolReviewCurrentReportId = ref("");

  function formatToolReviewError(error: unknown): string {
    const message = error instanceof Error ? String(error.message || "").trim() : String(error);
    const stack = error instanceof Error ? String(error.stack || "").trim() : "";
    if (stack && stack !== message) {
      return message ? `${message}\n${stack}` : stack;
    }
    return message || "Unknown error";
  }

  async function refreshToolReviewReports() {
    const conversationId = String(options.activeConversationId.value || "").trim();
    if (!conversationId) {
      toolReviewReports.value = [];
      toolReviewCurrentReportId.value = "";
      return;
    }
    try {
      const result = await invokeTauri<ListToolReviewReportsOutput>("list_tool_review_reports", {
        input: { conversationId },
      });
      toolReviewReports.value = Array.isArray(result?.reports) ? result.reports : [];
      const currentId = String(toolReviewCurrentReportId.value || "").trim();
      if (currentId && !toolReviewReports.value.some((item) => item.id === currentId)) {
        toolReviewCurrentReportId.value = "";
      }
      toolReviewReportErrorText.value = "";
    } catch (error) {
      console.error("[工具审查][前端] 刷新审查报告失败", {
        conversationId,
        error,
      });
      toolReviewReports.value = [];
      toolReviewCurrentReportId.value = "";
      toolReviewReportErrorText.value = options.t("chat.toolReview.loadFailed", { err: formatToolReviewError(error) });
    }
  }

  async function deleteToolReviewReport(input: DeleteToolReviewReportInput) {
    const conversationId = String(input.conversationId || options.activeConversationId.value || "").trim();
    const reportId = String(input.reportId || "").trim();
    if (!conversationId || !reportId) return;
    toolReviewReportErrorText.value = "";
    try {
      await invokeTauri("delete_tool_review_report", {
        input: {
          conversationId,
          reportId,
        },
      });
      if (toolReviewCurrentReportId.value === reportId) {
        toolReviewCurrentReportId.value = "";
      }
      toolReviewReportErrorText.value = "";
    } catch (error) {
      toolReviewReportErrorText.value = options.t("chat.toolReview.loadFailed", { err: formatToolReviewError(error) });
      throw error;
    }
  }

  async function listToolReviewCommitOptions(conversationId?: string, page = 1, pageSize = 30) {
    const normalizedConversationId = String(conversationId || options.activeConversationId.value || "").trim();
    if (!normalizedConversationId) {
      return { total: 0, page, pageSize, commits: [] } as ToolReviewCommitPage;
    }
    const result = await invokeTauri<ListToolReviewCommitOptionsOutput>("list_tool_review_commit_options", {
      input: {
        conversationId: normalizedConversationId,
        page,
        pageSize,
      },
    });
    return {
      total: Number(result?.total || 0),
      page: Number(result?.page || page),
      pageSize: Number(result?.pageSize || pageSize),
      commits: Array.isArray(result?.commits) ? result.commits : [],
    } as ToolReviewCommitPage;
  }

  async function submitToolReviewCode(input: SubmitToolReviewCodeInput): Promise<ToolReviewReportRecord | null> {
    const conversationId = String(input.conversationId || options.activeConversationId.value || "").trim();
    const scope = String(input.scope || "").trim() as ToolReviewCodeReviewScope;
    if (!conversationId || !scope) return null;
    toolReviewSubmittingBatchKey.value = `scope:${scope}`;
    toolReviewReportErrorText.value = "";
    try {
      console.info("[工具审查][前端] 调用 submit_tool_review_code", {
        conversationId,
        scope,
        target: String(input.target || "").trim(),
        departmentId: String(input.departmentId || "").trim(),
      });
      const result = await invokeTauri<SubmitToolReviewTaskOutput>("submit_tool_review_code", {
        input: {
          conversationId,
          scope,
          target: String(input.target || "").trim() || undefined,
          departmentId: String(input.departmentId || "").trim() || undefined,
        },
      });
      toolReviewCurrentReportId.value = String(result?.report?.id || "").trim();
      toolReviewReportErrorText.value = "";
      toolReviewErrorText.value = "";
      return result?.report || null;
    } catch (error) {
      toolReviewReportErrorText.value = options.t("chat.toolReview.loadFailed", { err: formatToolReviewError(error) });
      return null;
    } finally {
      if (toolReviewSubmittingBatchKey.value === `scope:${scope}`) {
        toolReviewSubmittingBatchKey.value = "";
      }
    }
  }

  async function refreshMessagesAfterReviewMutation(
    conversationId: string,
    messageIds: string[],
  ) {
    if (!options.onRefreshMessage) return;
    const normalizedMessageIds = messageIds
      .map((item) => String(item || "").trim())
      .filter((item, index, list) => !!item && list.indexOf(item) === index);
    for (const messageId of normalizedMessageIds) {
      await options.onRefreshMessage({
        conversationId,
        messageId,
      });
    }
  }

  function resolveValidBatchKey(
    batches: ToolReviewBatchSummary[],
    preferredKey?: string | null,
  ): string {
    const normalizedPreferredKey = String(preferredKey || "").trim();
    if (normalizedPreferredKey && batches.some((batch) => batch.batchKey === normalizedPreferredKey)) {
      return normalizedPreferredKey;
    }
    return String(batches[batches.length - 1]?.batchKey || "").trim();
  }

  async function refreshToolReviewBatches() {
    const conversationId = String(options.activeConversationId.value || "").trim();
    if (!conversationId) {
      toolReviewBatches.value = [];
      toolReviewCurrentBatchKey.value = "";
      toolReviewDetailMap.value = {};
      return;
    }
    try {
      const result = await invokeTauri<ToolReviewBatchListOutput>("list_tool_review_batches", {
        input: {
          conversationId,
        },
      });
      toolReviewBatches.value = Array.isArray(result?.batches) ? result.batches : [];
      const currentKey = String(toolReviewCurrentBatchKey.value || "").trim();
      toolReviewCurrentBatchKey.value = currentKey
        ? resolveValidBatchKey(toolReviewBatches.value, currentKey)
        : resolveValidBatchKey(toolReviewBatches.value, result?.currentBatchKey);
      const validCallIds = new Set(toolReviewBatches.value.flatMap((batch) => batch.items.map((item) => item.callId)));
      toolReviewDetailMap.value = Object.fromEntries(
        Object.entries(toolReviewDetailMap.value).filter(([callId]) => validCallIds.has(callId))
      );
      toolReviewErrorText.value = "";
      await refreshToolReviewReports();
    } catch (error) {
      toolReviewErrorText.value = options.t("chat.toolReview.loadFailed", { err: formatToolReviewError(error) });
    }
  }

  async function loadToolReviewItemDetail(callId: string, force = false) {
    const normalizedCallId = String(callId || "").trim();
    const conversationId = String(options.activeConversationId.value || "").trim();
    if (!normalizedCallId || !conversationId) return null;
    const cachedDetail = toolReviewDetailMap.value[normalizedCallId];
    if (!force && cachedDetail) return cachedDetail;
    toolReviewDetailLoadingCallId.value = normalizedCallId;
    try {
      const detail = await invokeTauri<ToolReviewItemDetail>("get_tool_review_item_detail", {
        input: {
          conversationId,
          callId: normalizedCallId,
        },
      });
      toolReviewDetailMap.value = {
        ...toolReviewDetailMap.value,
        [normalizedCallId]: detail,
      };
      toolReviewErrorText.value = "";
      return detail;
    } catch (error) {
      toolReviewErrorText.value = options.t("chat.toolReview.loadFailed", { err: formatToolReviewError(error) });
      return null;
    } finally {
      if (toolReviewDetailLoadingCallId.value === normalizedCallId) {
        toolReviewDetailLoadingCallId.value = "";
      }
    }
  }

  async function runToolReviewForCall(callId: string) {
    const normalizedCallId = String(callId || "").trim();
    const conversationId = String(options.activeConversationId.value || "").trim();
    if (!normalizedCallId || !conversationId) return;
    toolReviewReviewingCallId.value = normalizedCallId;
    try {
      const detail = await invokeTauri<ToolReviewItemDetail>("run_tool_review_for_call", {
        input: {
          conversationId,
          callId: normalizedCallId,
        },
      });
      toolReviewDetailMap.value = {
        ...toolReviewDetailMap.value,
        [normalizedCallId]: detail,
      };
      await refreshToolReviewBatches();
      await refreshMessagesAfterReviewMutation(conversationId, [detail.messageId]);
      toolReviewErrorText.value = "";
    } catch (error) {
      toolReviewErrorText.value = options.t("chat.toolReview.loadFailed", { err: formatToolReviewError(error) });
    } finally {
      if (toolReviewReviewingCallId.value === normalizedCallId) {
        toolReviewReviewingCallId.value = "";
      }
    }
  }

  async function runToolReviewForBatch(batchKey?: string) {
    const normalizedBatchKey = String(batchKey || toolReviewCurrentBatchKey.value || "").trim();
    const conversationId = String(options.activeConversationId.value || "").trim();
    if (!normalizedBatchKey || !conversationId) return;
    const batchIndex = toolReviewBatches.value.findIndex((batch) => String(batch.batchKey || "").trim() === normalizedBatchKey);
    if (batchIndex < 0) return;
    toolReviewBatchReviewingKey.value = normalizedBatchKey;
    try {
      const result = await invokeTauri<{ reviewedCallIds: string[] }>("run_tool_review_for_batch", {
        input: {
          conversationId,
          batchIndex,
        },
      });
      const reviewedCallIds = Array.isArray(result?.reviewedCallIds) ? result.reviewedCallIds : [];
      const refreshedDetails = await Promise.all(reviewedCallIds.map((callId) => loadToolReviewItemDetail(callId, true)));
      await refreshToolReviewBatches();
      await refreshMessagesAfterReviewMutation(
        conversationId,
        refreshedDetails
          .map((detail) => String(detail?.messageId || "").trim())
          .filter(Boolean),
      );
      toolReviewErrorText.value = "";
    } catch (error) {
      toolReviewErrorText.value = options.t("chat.toolReview.loadFailed", { err: formatToolReviewError(error) });
    } finally {
      if (toolReviewBatchReviewingKey.value === normalizedBatchKey) {
        toolReviewBatchReviewingKey.value = "";
      }
    }
  }

  function toggleToolReviewPanel() {
    toolReviewPanelOpen.value = !toolReviewPanelOpen.value;
  }

  function setToolReviewCurrentBatchKey(batchKey: string) {
    toolReviewCurrentBatchKey.value = String(batchKey || "").trim();
  }

  watch(
    () => options.refreshTick.value,
    () => {
      if (!String(options.activeConversationId.value || "").trim()) return;
      void refreshToolReviewBatches();
    },
  );

  if (options.initialPanelOpen) {
    watch(
      () => options.initialPanelOpen?.value,
      (open) => {
        toolReviewPanelOpen.value = !!open;
      },
    );
  }

  watch(
    () => String(options.activeConversationId.value || "").trim(),
    (conversationId) => {
      toolReviewBatches.value = [];
      toolReviewDetailMap.value = {};
      toolReviewDetailLoadingCallId.value = "";
      toolReviewReviewingCallId.value = "";
      toolReviewBatchReviewingKey.value = "";
      toolReviewSubmittingBatchKey.value = "";
      toolReviewCurrentBatchKey.value = "";
      toolReviewReports.value = [];
      toolReviewCurrentReportId.value = "";
      toolReviewErrorText.value = "";
      toolReviewReportErrorText.value = "";
      if (conversationId) {
        void refreshToolReviewBatches();
      }
    },
    { immediate: true },
  );

  return {
    toolReviewPanelOpen,
    toolReviewBatches,
    toolReviewCurrentBatchKey,
    toolReviewDetailMap,
    toolReviewDetailLoadingCallId,
    toolReviewReviewingCallId,
    toolReviewBatchReviewingKey,
    toolReviewSubmittingBatchKey,
    toolReviewErrorText,
    toolReviewReportErrorText,
    toolReviewReports,
    toolReviewCurrentReportId,
    toggleToolReviewPanel,
    refreshToolReviewBatches,
    refreshToolReviewReports,
    setToolReviewCurrentBatchKey,
    loadToolReviewItemDetail,
    runToolReviewForCall,
    runToolReviewForBatch,
    submitToolReviewCode,
    deleteToolReviewReport,
    listToolReviewCommitOptions,
  };
}

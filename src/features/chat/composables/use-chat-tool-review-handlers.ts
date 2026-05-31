import { watch, type Ref, type ComponentPublicInstance } from "vue";
import { useChatToolReview, type ToolReviewCodeReviewScope, type ToolReviewCommitOption, type ToolReviewReportRecord } from "./use-chat-tool-review";
import { resolveRetryToolReviewDepartmentId } from "../utils/tool-review-department";

export interface UseChatToolReviewHandlersOptions {
  activeConversationId: Ref<string>;
  toolReviewRefreshTick: Ref<number>;
  currentDepartmentId: Ref<string>;
  departmentOptions: Ref<Array<{ id: string }>>;
  initialPanelOpen?: Ref<boolean>;
  t: (key: string, params?: Record<string, unknown>) => string;
  syncViewportMetrics: () => void;
  onRefreshMessage: (payload: { conversationId: string; messageId: string }) => void;
  onToolReviewPanelOpenChange: (open: boolean) => void;
  toolReviewSidebarRef: Ref<ComponentPublicInstance<{ setCommitOptions: (items: ToolReviewCommitOption[], loading?: boolean, total?: number, page?: number, pageSize?: number) => void }> | null>;
}

export function useChatToolReviewHandlers(options: UseChatToolReviewHandlersOptions) {
  const { t, syncViewportMetrics, onToolReviewPanelOpenChange, toolReviewSidebarRef } = options;

  const {
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
    setToolReviewCurrentBatchKey,
    loadToolReviewItemDetail,
    runToolReviewForCall,
    runToolReviewForBatch,
    submitToolReviewCode,
    deleteToolReviewReport,
    listToolReviewCommitOptions,
  } = useChatToolReview({
    activeConversationId: options.activeConversationId,
    refreshTick: options.toolReviewRefreshTick,
    initialPanelOpen: options.initialPanelOpen,
    t,
    onRefreshMessage: options.onRefreshMessage,
  });

  watch(
    toolReviewPanelOpen,
    (value) => {
      onToolReviewPanelOpenChange(value);
    },
    { immediate: true },
  );

  async function handlePickCommitReview(page = 1) {
    const conversationId = String(options.activeConversationId.value || "").trim();
    if (!conversationId) return;
    toolReviewSidebarRef.value?.setCommitOptions([], true, 0, page, 30);
    try {
      const result = await listToolReviewCommitOptions(conversationId, page, 30);
      toolReviewSidebarRef.value?.setCommitOptions(result.commits, false, result.total, result.page, result.pageSize);
    } catch (error) {
      const detail = error instanceof Error ? String(error.message || "").trim() : String(error);
      toolReviewErrorText.value = t("chat.toolReview.loadFailed", { err: detail || "Unknown error" });
      toolReviewSidebarRef.value?.setCommitOptions([], false, 0, page, 30);
    }
  }

  async function handleDeleteToolReviewReport(report: ToolReviewReportRecord) {
    const conversationId = String(options.activeConversationId.value || "").trim();
    const reportId = String(report.id || "").trim();
    if (!conversationId || !reportId) return;
    try {
      await deleteToolReviewReport({ conversationId, reportId });
    } catch (error) {
      const detail = error instanceof Error ? String(error.message || "").trim() : String(error);
      toolReviewErrorText.value = t("chat.toolReview.loadFailed", { err: detail || t("chat.codeReviewDeleteFailed") });
    }
  }

  async function handleToolReviewCode(input: { scope: ToolReviewCodeReviewScope; target?: string; departmentId: string }) {
    const conversationId = String(options.activeConversationId.value || "").trim();
    const scope = input.scope;
    const departmentId = String(input.departmentId || options.currentDepartmentId.value || "").trim();
    if (!conversationId) { toolReviewErrorText.value = t("chat.codeReviewNoConversation"); return; }
    if (!departmentId) { toolReviewErrorText.value = t("chat.codeReviewNoDepartment"); return; }
    try {
      await submitToolReviewCode({ conversationId, scope, target: String(input.target || "").trim() || undefined, departmentId });
    } catch (error) {
      const detail = error instanceof Error ? String(error.message || "").trim() : String(error);
      toolReviewErrorText.value = t("chat.toolReview.loadFailed", { err: detail || t("chat.codeReviewSubmitFailed") });
    }
  }

  async function handleRetryToolReviewReport(report: ToolReviewReportRecord) {
    const scope = String(report.scope || "").trim() as ToolReviewCodeReviewScope;
    const target = String(report.target || "").trim();
    const conversationId = String(options.activeConversationId.value || "").trim();
    const reportId = String(report.id || "").trim();
    if (!conversationId || !reportId) { toolReviewErrorText.value = t("chat.codeReviewRetryNoConversation"); return; }
    const retryDepartmentId = resolveRetryToolReviewDepartmentId({
      reportDepartmentId: String(report.departmentId || "").trim(),
      currentDepartmentId: String(options.currentDepartmentId.value || "").trim(),
      departmentOptions: options.departmentOptions.value,
    });
    if (scope === "commit" || scope === "main" || scope === "uncommitted" || scope === "custom") {
      if (!retryDepartmentId) { toolReviewErrorText.value = t("chat.codeReviewRetryNoDepartment"); return; }
      const nextReport = await submitToolReviewCode({ conversationId, scope, target: target || undefined, departmentId: retryDepartmentId });
      if (!nextReport) return;
      try { await deleteToolReviewReport({ conversationId, reportId }); } catch { /* best-effort */ }
      return;
    }
    toolReviewErrorText.value = t("chat.codeReviewRetryUnsupportedScope", { scope: scope || "" });
  }

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
    setToolReviewCurrentBatchKey,
    loadToolReviewItemDetail,
    runToolReviewForCall,
    runToolReviewForBatch,
    handlePickCommitReview,
    handleDeleteToolReviewReport,
    handleToolReviewCode,
    handleRetryToolReviewReport,
  };
}

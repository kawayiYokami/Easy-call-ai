type UseChatAttachmentActionsOptions = {
  queueTextAttachment: (fileName: string, text: string, mime: string) => Promise<void>;
  status: { value: string };
  setStatusError: (key: string, error: unknown) => void;
};

export function useChatAttachmentActions(options: UseChatAttachmentActionsOptions) {
  async function attachToolReviewReport(reportText: string) {
    try {
      await options.queueTextAttachment("code-review-report.md", reportText, "text/markdown");
      options.status.value = "已附加代码审查报告";
    } catch (error) {
      options.setStatusError("status.pasteImageReadFailed", error);
    }
  }

  return {
    attachToolReviewReport,
  };
}

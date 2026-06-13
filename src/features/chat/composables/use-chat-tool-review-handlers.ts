import { watch, type Ref, type ComponentPublicInstance } from "vue";
import { useChatToolReview, type ToolReviewBridgeRequest } from "./use-chat-tool-review";

export interface UseChatToolReviewHandlersOptions {
  activeConversationId: Ref<string>;
  toolReviewRefreshTick: Ref<number>;
  currentDepartmentId: Ref<string>;
  departmentOptions: Ref<Array<{ id: string }>>;
  initialPanelOpen?: Ref<boolean>;
  bridgeRequest?: Ref<ToolReviewBridgeRequest | undefined>;
  t: (key: string, params?: Record<string, unknown>) => string;
  syncViewportMetrics: () => void;
  onRefreshMessage: (payload: { conversationId: string; messageId: string }) => void;
  onToolReviewPanelOpenChange: (open: boolean) => void;
}

export function useChatToolReviewHandlers(options: UseChatToolReviewHandlersOptions) {
  const { t, syncViewportMetrics, onToolReviewPanelOpenChange } = options;

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
    toggleToolReviewPanel,
    setToolReviewCurrentBatchKey,
    loadToolReviewItemDetail,
    runToolReviewForCall,
    runToolReviewForBatch,
    submitToolReviewCode,
    listToolReviewCommitOptions,
  } = useChatToolReview({
    activeConversationId: options.activeConversationId,
    refreshTick: options.toolReviewRefreshTick,
    initialPanelOpen: options.initialPanelOpen,
    bridgeRequest: options.bridgeRequest,
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
    toggleToolReviewPanel,
    setToolReviewCurrentBatchKey,
    loadToolReviewItemDetail,
    runToolReviewForCall,
    runToolReviewForBatch,
    submitToolReviewCode,
    listToolReviewCommitOptions,
  };
}

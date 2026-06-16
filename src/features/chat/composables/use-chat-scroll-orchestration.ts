import { nextTick, onBeforeUnmount, ref, watch, type Ref } from "vue";
import type { ChatMessageBlock } from "../../../types/app";

export interface UseChatScrollOrchestrationOptions {
  scrollContainer: Ref<HTMLElement | null>;
  chatScrollbarRef: Ref<{ updateThumb: () => void; hide?: () => void } | null>;
  prepareBottomAlignmentLayout?: () => Promise<void> | void;
  onScroll: () => void;
  scheduleVirtualMeasure: () => void;
  syncViewportMetrics: () => void;
  resetConversationToBottom: () => void;
  alignItemToTop: (itemId: string, behavior?: ScrollBehavior) => void;
  refreshObservedVirtualItemElements: () => void;
  latestOwnElasticItemId: Ref<string>;
  props: {
    hasMoreHistory: Ref<boolean>;
    loadingOlderHistory: Ref<boolean>;
    chatting: Ref<boolean>;
    conversationBusy: Ref<boolean>;
    frozen: Ref<boolean>;
    activeConversationId: Ref<string>;
    conversationScrollToBottomRequest: Ref<number>;
    latestOwnMessageAlignRequest: Ref<number>;
    messageBlocks: Ref<ChatMessageBlock[]>;
  };
  emit: {
    loadOlderHistory: () => void;
    jumpToConversationBottom: () => void;
  };
}

export function useChatScrollOrchestration(options: UseChatScrollOrchestrationOptions) {
  const {
    scrollContainer,
    chatScrollbarRef,
    prepareBottomAlignmentLayout,
    onScroll,
    scheduleVirtualMeasure,
    syncViewportMetrics,
    resetConversationToBottom,
    alignItemToTop,
    refreshObservedVirtualItemElements,
    latestOwnElasticItemId,
    props,
    emit,
  } = options;

  const LOAD_OLDER_HISTORY_THRESHOLD_PX = 8;
  const olderHistoryRequestPending = ref(false);
  const suppressOlderHistoryPaginationOnce = ref(false);
  let pendingProgrammaticScrollPaginationResetFrame = 0;
  let pendingAutoOlderHistoryFrame = 0;
  let autoOlderHistoryScheduled = false;

  function armProgrammaticScrollPaginationSuppression() {
    suppressOlderHistoryPaginationOnce.value = true;
    if (pendingProgrammaticScrollPaginationResetFrame) {
      cancelAnimationFrame(pendingProgrammaticScrollPaginationResetFrame);
      pendingProgrammaticScrollPaginationResetFrame = 0;
    }
    pendingProgrammaticScrollPaginationResetFrame = requestAnimationFrame(() => {
      pendingProgrammaticScrollPaginationResetFrame = requestAnimationFrame(() => {
        suppressOlderHistoryPaginationOnce.value = false;
        pendingProgrammaticScrollPaginationResetFrame = 0;
      });
    });
  }

  function maybeRequestOlderHistory() {
    const scrollEl = scrollContainer.value;
    if (!scrollEl) return;
    if (!props.hasMoreHistory.value || props.loadingOlderHistory.value || olderHistoryRequestPending.value) return;
    if (scrollEl.scrollTop > LOAD_OLDER_HISTORY_THRESHOLD_PX) return;
    olderHistoryRequestPending.value = true;
    emit.loadOlderHistory();
  }

  function scheduleAutoRequestOlderHistory() {
    if (autoOlderHistoryScheduled) return;
    autoOlderHistoryScheduled = true;
    void nextTick(() => {
      pendingAutoOlderHistoryFrame = requestAnimationFrame(() => {
        pendingAutoOlderHistoryFrame = 0;
        autoOlderHistoryScheduled = false;
        maybeRequestOlderHistory();
      });
    });
  }

  function onConversationScroll() {
    onScroll();
    chatScrollbarRef.value?.updateThumb();
    if (suppressOlderHistoryPaginationOnce.value) {
      suppressOlderHistoryPaginationOnce.value = false;
      if (pendingProgrammaticScrollPaginationResetFrame) {
        cancelAnimationFrame(pendingProgrammaticScrollPaginationResetFrame);
        pendingProgrammaticScrollPaginationResetFrame = 0;
      }
    } else {
      maybeRequestOlderHistory();
    }
  }

  function doScrollToBottom() {
    armProgrammaticScrollPaginationSuppression();
    scheduleVirtualMeasure();
    void nextTick(async () => {
      await prepareBottomAlignmentLayout?.();
      resetConversationToBottom();
    });
  }

  function handleJumpToBottom() {
    doScrollToBottom();
    if (props.chatting.value || props.conversationBusy.value || props.frozen.value) return;
    emit.jumpToConversationBottom();
  }

  function alignLatestOwnMessageToTop(behavior: ScrollBehavior = "smooth") {
    alignItemToTop(latestOwnElasticItemId.value, behavior);
  }

  // ==================== watchers ====================

  watch(
    () => String(props.activeConversationId.value || "").trim(),
    () => {
      chatScrollbarRef.value?.hide?.();
      olderHistoryRequestPending.value = false;
      armProgrammaticScrollPaginationSuppression();
      void prepareBottomAlignmentLayout?.();
      scheduleAutoRequestOlderHistory();
    },
    { immediate: true },
  );

  watch(
    () => props.conversationScrollToBottomRequest.value,
    (nextValue, prevValue) => {
      if (!nextValue || nextValue === prevValue) return;
      doScrollToBottom();
    },
  );

  watch(
    () => props.latestOwnMessageAlignRequest.value,
    (nextValue, prevValue) => {
      if (!nextValue || nextValue === prevValue) return;
      void nextTick(async () => {
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
        refreshObservedVirtualItemElements();
        alignLatestOwnMessageToTop("smooth");
      });
    },
  );

  watch(
    () => props.messageBlocks.value,
    () => {
      refreshObservedVirtualItemElements();
      void nextTick(() => {
        chatScrollbarRef.value?.updateThumb();
        scheduleAutoRequestOlderHistory();
      });
    },
  );

  watch(
    () => props.loadingOlderHistory.value,
    async (loading, wasLoading) => {
      if (loading) return;
      if (!wasLoading) return;
      const scrollEl = scrollContainer.value;
      if (!scrollEl) return;
      await nextTick();
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      refreshObservedVirtualItemElements();

      // 加载完成后，如果还在最顶部，主动往下滚一点，避免立即再次触发加载
      const SAFE_SCROLL_POSITION = 12;
      if (scrollEl.scrollTop < SAFE_SCROLL_POSITION) {
        scrollEl.scrollTop = SAFE_SCROLL_POSITION;
      }

      olderHistoryRequestPending.value = false;
      scheduleAutoRequestOlderHistory();
    },
  );

  onBeforeUnmount(() => {
    if (pendingProgrammaticScrollPaginationResetFrame) {
      cancelAnimationFrame(pendingProgrammaticScrollPaginationResetFrame);
      pendingProgrammaticScrollPaginationResetFrame = 0;
    }
    if (pendingAutoOlderHistoryFrame) {
      cancelAnimationFrame(pendingAutoOlderHistoryFrame);
      pendingAutoOlderHistoryFrame = 0;
    }
    autoOlderHistoryScheduled = false;
  });

  return {
    onConversationScroll,
    handleJumpToBottom,
    alignLatestOwnMessageToTop,
    activeConversationChangedCleanup: () => {
      olderHistoryRequestPending.value = false;
    },
    armProgrammaticScrollPaginationSuppression,
  };
}

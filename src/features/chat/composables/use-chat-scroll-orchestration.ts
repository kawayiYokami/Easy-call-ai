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
  captureViewportAnchor: () => { blockId: string; offsetTop: number } | null;
  restoreViewportAnchor: (anchor: { blockId: string; offsetTop: number } | null) => boolean;
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
    captureViewportAnchor,
    restoreViewportAnchor,
    latestOwnElasticItemId,
    props,
    emit,
  } = options;

  const LOAD_OLDER_HISTORY_THRESHOLD_PX = 8;
  const OLDER_HISTORY_THROTTLE_MS = 1000;
  const SCROLL_SETTLE_DELAY_MS = 120;
  const olderHistoryRequestPending = ref(false);
  const suppressOlderHistoryPaginationOnce = ref(false);
  let pendingProgrammaticScrollPaginationResetFrame = 0;
  let pendingScrollSettleTimer = 0;
  let olderHistoryCooldownUntil = 0;
  let pendingOlderHistoryAnchor: { blockId: string; offsetTop: number } | null = null;

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
    if (Date.now() < olderHistoryCooldownUntil) return;
    olderHistoryRequestPending.value = true;
    olderHistoryCooldownUntil = Date.now() + OLDER_HISTORY_THROTTLE_MS;
    emit.loadOlderHistory();
  }

  function scheduleSettledOlderHistoryCheck() {
    if (pendingScrollSettleTimer) {
      window.clearTimeout(pendingScrollSettleTimer);
      pendingScrollSettleTimer = 0;
    }
    pendingScrollSettleTimer = window.setTimeout(() => {
      pendingScrollSettleTimer = 0;
      maybeRequestOlderHistory();
    }, SCROLL_SETTLE_DELAY_MS);
  }

  function shouldLockUpwardWheelInput(deltaY: number): boolean {
    if (deltaY >= 0) return false;
    const scrollEl = scrollContainer.value;
    if (!scrollEl) return false;
    if (!props.hasMoreHistory.value) return false;
    if (props.loadingOlderHistory.value || olderHistoryRequestPending.value) return true;
    const nearTop = scrollEl.scrollTop <= LOAD_OLDER_HISTORY_THRESHOLD_PX;
    const inCooldown = Date.now() < olderHistoryCooldownUntil;
    return nearTop && inCooldown;
  }

  function onConversationWheel(event: WheelEvent) {
    if (event.shiftKey) return;
    if (!shouldLockUpwardWheelInput(event.deltaY)) return;
    event.preventDefault();
    event.stopPropagation();
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
      scheduleSettledOlderHistoryCheck();
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
      });
    },
  );

  watch(
    () => props.loadingOlderHistory.value,
    async (loading, wasLoading) => {
      if (loading) {
        pendingOlderHistoryAnchor = captureViewportAnchor();
        return;
      }
      if (loading) return;
      if (!wasLoading) return;
      await nextTick();
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      const hasAnchor = !!pendingOlderHistoryAnchor;
      if (hasAnchor) {
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
        refreshObservedVirtualItemElements();
        restoreViewportAnchor(pendingOlderHistoryAnchor);
      }
      pendingOlderHistoryAnchor = null;

      olderHistoryRequestPending.value = false;
    },
  );

  onBeforeUnmount(() => {
    if (pendingProgrammaticScrollPaginationResetFrame) {
      cancelAnimationFrame(pendingProgrammaticScrollPaginationResetFrame);
      pendingProgrammaticScrollPaginationResetFrame = 0;
    }
    if (pendingScrollSettleTimer) {
      window.clearTimeout(pendingScrollSettleTimer);
      pendingScrollSettleTimer = 0;
    }
    pendingOlderHistoryAnchor = null;
  });

  return {
    onConversationScroll,
    onConversationWheel,
    handleJumpToBottom,
    alignLatestOwnMessageToTop,
    activeConversationChangedCleanup: () => {
      olderHistoryRequestPending.value = false;
    },
    armProgrammaticScrollPaginationSuppression,
  };
}

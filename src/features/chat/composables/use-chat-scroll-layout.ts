import { computed, nextTick, onBeforeUnmount, onMounted, ref, type Ref, watch } from "vue";

type UseChatScrollLayoutOptions = {
  activeConversationId: Ref<string>;
  chatting: Ref<boolean>;
  busy: Ref<boolean>;
  frozen: Ref<boolean>;
  messageBlockCount: Ref<number>;
  onReachedBottom: () => void;
  focusComposerInput: (options?: FocusOptions) => void;
};

export function useChatScrollLayout(options: UseChatScrollLayoutOptions) {
  const scrollContainer = ref<HTMLElement | null>(null);
  const composerContainer = ref<HTMLElement | null>(null);
  const toolbarContainer = ref<HTMLElement | null>(null);
  const chatLayoutRoot = ref<HTMLElement | null>(null);
  const latestOwnElasticMinHeight = ref(0);
  const jumpToBottomOffset = ref(96);
  const lastBottomState = ref(false);
  const lastScrollTop = ref(0);
  const userScrollingDown = ref(false);
  let composerResizeObserver: ResizeObserver | null = null;
  let chatLayoutResizeObserver: ResizeObserver | null = null;
  let pendingComposerResizeFrame = 0;
  let pendingChatLayoutResizeFrame = 0;

  const showJumpToBottom = computed(() => !lastBottomState.value && userScrollingDown.value);
  const jumpToBottomStyle = computed(() => ({
    bottom: `${jumpToBottomOffset.value}px`,
  }));

  function updateJumpToBottomOffset() {
    const composerHeight = composerContainer.value?.offsetHeight ?? 0;
    const nextOffset = Math.max(16, composerHeight + 12);
    if (jumpToBottomOffset.value !== nextOffset) {
      jumpToBottomOffset.value = nextOffset;
    }
  }

  function updateLatestOwnElasticMinHeight() {
    const scrollEl = scrollContainer.value;
    if (!scrollEl) {
      if (latestOwnElasticMinHeight.value !== 0) {
        latestOwnElasticMinHeight.value = 0;
      }
      return;
    }
    const scrollStyles = window.getComputedStyle(scrollEl);
    const scrollViewportHeight =
      scrollEl.clientHeight
      - parseFloat(scrollStyles.paddingTop || "0")
      - parseFloat(scrollStyles.paddingBottom || "0");
    const toolbarHeight = toolbarContainer.value?.offsetHeight ?? 0;
    const nextMinHeight = Math.max(0, scrollViewportHeight - toolbarHeight);
    if (latestOwnElasticMinHeight.value !== nextMinHeight) {
      latestOwnElasticMinHeight.value = nextMinHeight;
    }
  }

  async function prepareBottomAlignmentLayout() {
    updateJumpToBottomOffset();
    updateLatestOwnElasticMinHeight();
    await nextTick();
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    updateJumpToBottomOffset();
    updateLatestOwnElasticMinHeight();
    await nextTick();
  }

  function isNearBottom(el: HTMLElement): boolean {
    const threshold = 24;
    const distance = el.scrollHeight - (el.scrollTop + el.clientHeight);
    return distance <= threshold;
  }

  function onScroll() {
    const el = scrollContainer.value;
    if (!el) return;
    userScrollingDown.value = el.scrollTop > lastScrollTop.value;
    lastScrollTop.value = el.scrollTop;
    const nearBottom = isNearBottom(el);
    if (nearBottom && !lastBottomState.value) {
      options.onReachedBottom();
    }
    if (nearBottom) {
      userScrollingDown.value = false;
    }
    lastBottomState.value = nearBottom;
  }

  onMounted(() => {
    nextTick(() => {
      updateJumpToBottomOffset();
      updateLatestOwnElasticMinHeight();
      if (composerContainer.value && typeof ResizeObserver !== "undefined") {
        composerResizeObserver = new ResizeObserver(() => {
          if (typeof window === "undefined") {
            updateJumpToBottomOffset();
            updateLatestOwnElasticMinHeight();
            return;
          }
          if (pendingComposerResizeFrame) return;
          pendingComposerResizeFrame = window.requestAnimationFrame(() => {
            pendingComposerResizeFrame = 0;
            updateJumpToBottomOffset();
            updateLatestOwnElasticMinHeight();
          });
        });
        composerResizeObserver.observe(composerContainer.value);
      }
      if (chatLayoutRoot.value && typeof ResizeObserver !== "undefined") {
        chatLayoutResizeObserver = new ResizeObserver(() => {
          if (typeof window === "undefined") {
            updateJumpToBottomOffset();
            updateLatestOwnElasticMinHeight();
            return;
          }
          if (pendingChatLayoutResizeFrame) return;
          pendingChatLayoutResizeFrame = window.requestAnimationFrame(() => {
            pendingChatLayoutResizeFrame = 0;
            updateJumpToBottomOffset();
            updateLatestOwnElasticMinHeight();
          });
        });
        chatLayoutResizeObserver.observe(chatLayoutRoot.value);
      }
      const el = scrollContainer.value;
      if (el) {
        lastBottomState.value = isNearBottom(el);
        lastScrollTop.value = el.scrollTop;
        userScrollingDown.value = false;
      }
    });
  });

  onBeforeUnmount(() => {
    if (composerResizeObserver) {
      composerResizeObserver.disconnect();
      composerResizeObserver = null;
    }
    if (pendingComposerResizeFrame && typeof window !== "undefined") {
      window.cancelAnimationFrame(pendingComposerResizeFrame);
      pendingComposerResizeFrame = 0;
    }
    if (chatLayoutResizeObserver) {
      chatLayoutResizeObserver.disconnect();
      chatLayoutResizeObserver = null;
    }
    if (pendingChatLayoutResizeFrame && typeof window !== "undefined") {
      window.cancelAnimationFrame(pendingChatLayoutResizeFrame);
      pendingChatLayoutResizeFrame = 0;
    }
  });

  watch(
    options.chatting,
    (isChatting, wasChatting) => {
      if (wasChatting && !isChatting && !options.frozen.value && !options.busy.value) {
        nextTick(() => options.focusComposerInput({ preventScroll: true }));
      }
    },
  );

  watch(
    options.activeConversationId,
    () => {
      nextTick(() => {
        updateJumpToBottomOffset();
        updateLatestOwnElasticMinHeight();
        const el = scrollContainer.value;
        if (el) {
          lastBottomState.value = isNearBottom(el);
          lastScrollTop.value = el.scrollTop;
          userScrollingDown.value = false;
        }
      });
    },
    { immediate: true },
  );

  watch(
    options.messageBlockCount,
    () => {
      nextTick(() => {
        updateJumpToBottomOffset();
        updateLatestOwnElasticMinHeight();
        const el = scrollContainer.value;
        if (el) {
          lastBottomState.value = isNearBottom(el);
          lastScrollTop.value = el.scrollTop;
          userScrollingDown.value = false;
        }
      });
    },
  );

  return {
    scrollContainer,
    composerContainer,
    toolbarContainer,
    chatLayoutRoot,
    latestOwnElasticMinHeight,
    showJumpToBottom,
    jumpToBottomStyle,
    onScroll,
    prepareBottomAlignmentLayout,
  };
}

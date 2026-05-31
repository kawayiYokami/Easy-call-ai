import { computed, nextTick, ref, watch, type Ref } from "vue";

type PaneResizeSide = "left" | "right";

export const PANE_WIDTH_LIMITS = {
  left: { min: 260, max: 10000, default: 320 },
  right: { min: 260, max: 10000, default: 320 },
} as const;

export const PANE_CENTER_MIN_WIDTH = 300;
export const PANE_OVERLAY_VISIBLE_MARGIN = 56;
export const PANE_WIDTH_STORAGE_KEYS = {
  left: "easy_call.chat_left_sidebar_width.v1",
  right: "easy_call.chat_right_sidebar_width.v1",
} as const;
const LEGACY_PANE_WIDTH_STORAGE_KEYS = {
  left: "easy-call.chat.left-sidebar-width",
  right: "easy-call.chat.right-sidebar-width",
} as const;

export function loadStoredPaneWidth(side: PaneResizeSide): number {
  if (typeof window === "undefined") return PANE_WIDTH_LIMITS[side].default;
  const raw = window.localStorage.getItem(PANE_WIDTH_STORAGE_KEYS[side])
    ?? window.localStorage.getItem(LEGACY_PANE_WIDTH_STORAGE_KEYS[side]);
  const parsed = Number(raw);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : PANE_WIDTH_LIMITS[side].default;
}

export interface UseChatPanesOptions {
  chatLayoutRoot: Ref<HTMLElement | null>;
  toolReviewPanelOpen: Ref<boolean>;
  showSideConversationList: Ref<boolean>;
  detachedChatWindow: boolean;
  syncViewportMetrics: () => void;
  onPaneWidthsChange: (leftWidth: number, rightWidth: number) => void;
  onPaneWidthsCommit: (leftWidth: number, rightWidth: number) => void;
  onBeforeUnmountCleanup: (fn: () => void) => void;
}

export function useChatPanes(options: UseChatPanesOptions) {
  const { chatLayoutRoot, toolReviewPanelOpen, showSideConversationList, detachedChatWindow, syncViewportMetrics, onPaneWidthsChange, onPaneWidthsCommit, onBeforeUnmountCleanup } = options;

  const leftSidebarWidth = ref(loadStoredPaneWidth("left"));
  const rightSidebarWidth = ref(loadStoredPaneWidth("right"));
  const activePaneResizeSide = ref<PaneResizeSide | null>(null);
  const resizeLockedLeftPaneInLayout = ref<boolean | null>(null);
  const resizeLockedRightPaneInLayout = ref<boolean | null>(null);
  const lastOpenedPane = ref<PaneResizeSide | null>(null);
  let paneResizeStartX = 0;
  let paneResizeStartWidth = 0;
  let paneResizePreviousBodyCursor = "";
  let paneResizePreviousBodyUserSelect = "";

  // ========== responsive layout measurement ==========

  const containerWidth = ref(0);
  let layoutResizeObserver: ResizeObserver | null = null;

  function measureContainerWidth() {
    const el = chatLayoutRoot.value;
    containerWidth.value = el ? Math.round(el.getBoundingClientRect().width) : 0;
  }

  function effectivePaneWidth(side: PaneResizeSide): number {
    const limits = PANE_WIDTH_LIMITS[side];
    const width = side === "left" ? leftSidebarWidth.value : rightSidebarWidth.value;
    return Math.max(limits.min, Math.round(width || limits.default));
  }

  function overlayVisibleWidth(side: PaneResizeSide): number {
    const width = effectivePaneWidth(side);
    if (containerWidth.value <= 0) return width;
    return Math.min(width, Math.max(PANE_WIDTH_LIMITS[side].min, containerWidth.value - PANE_OVERLAY_VISIBLE_MARGIN));
  }

  function overlayMaxWidth(side: PaneResizeSide): number {
    if (containerWidth.value <= 0) return PANE_WIDTH_LIMITS[side].max;
    return Math.max(PANE_WIDTH_LIMITS[side].min, containerWidth.value - PANE_OVERLAY_VISIBLE_MARGIN);
  }

  function canFitInLayout(leftW: number, rightW: number): boolean {
    return containerWidth.value <= 0 || leftW + PANE_CENTER_MIN_WIDTH + rightW <= containerWidth.value;
  }

  // ========== layout mode computeds ==========

  const basePaneLayoutState = computed(() => {
    const leftOpen = showSideConversationList.value && !detachedChatWindow;
    const rightOpen = toolReviewPanelOpen.value;
    const leftW = effectivePaneWidth("left");
    const rightW = effectivePaneWidth("right");

    if (!leftOpen && !rightOpen) return { left: false, right: false };
    if (!leftOpen) return { left: false, right: canFitInLayout(0, rightW) };
    if (!rightOpen) return { left: canFitInLayout(leftW, 0), right: false };
    if (canFitInLayout(leftW, rightW)) return { left: true, right: true };

    if (lastOpenedPane.value === "left") {
      return { left: false, right: canFitInLayout(0, rightW) };
    }
    if (lastOpenedPane.value === "right") {
      return { left: canFitInLayout(leftW, 0), right: false };
    }

    // Fallback when no reliable open-order is available (e.g. restored state):
    // keep at most one pane embedded so the center area never collapses.
    if (canFitInLayout(leftW, 0)) return { left: true, right: false };
    if (canFitInLayout(0, rightW)) return { left: false, right: true };
    return { left: false, right: false };
  });

  /** Left pane is embedded in the flex layout (not overlay) */
  const leftPaneInLayout = computed(() => {
    if (activePaneResizeSide.value === "left" && resizeLockedLeftPaneInLayout.value === false) {
      return false;
    }
    return basePaneLayoutState.value.left;
  });

  /** Right pane is embedded in the flex layout (not overlay) */
  const rightPaneInLayout = computed(() => {
    if (activePaneResizeSide.value === "right" && resizeLockedRightPaneInLayout.value === false) {
      return false;
    }
    if (activePaneResizeSide.value === "left" && resizeLockedRightPaneInLayout.value === false) {
      return false;
    }
    return basePaneLayoutState.value.right;
  });

  /** Left pane is open but shown as overlay (not in layout) */
  const leftPaneOverlay = computed(() =>
    showSideConversationList.value && !detachedChatWindow && !leftPaneInLayout.value,
  );

  /** Right pane is open but shown as overlay (not in layout) */
  const rightPaneOverlay = computed(() =>
    toolReviewPanelOpen.value && !rightPaneInLayout.value,
  );

  const leftPaneVisibleWidth = computed(() =>
    leftPaneOverlay.value ? overlayVisibleWidth("left") : effectivePaneWidth("left"),
  );
  const rightPaneVisibleWidth = computed(() =>
    rightPaneOverlay.value ? overlayVisibleWidth("right") : effectivePaneWidth("right"),
  );

  // ========== width clamping & persistence ==========

  function storePaneWidth(side: PaneResizeSide, width: number) {
    if (typeof window === "undefined") return;
    window.localStorage.setItem(PANE_WIDTH_STORAGE_KEYS[side], String(Math.round(width)));
  }

  function clampPaneWidth(side: PaneResizeSide, width: number): number {
    const limits = PANE_WIDTH_LIMITS[side];
    if (activePaneResizeSide.value === side) {
      return Math.round(Math.min(overlayMaxWidth(side), Math.max(limits.min, width)));
    }
    const cw = containerWidth.value || 0;
    const otherPaneWidth =
      side === "left"
        ? (rightPaneInLayout.value ? rightSidebarWidth.value : 0)
        : (leftPaneInLayout.value ? leftSidebarWidth.value : 0);
    const layoutMax = cw > 0 ? cw - otherPaneWidth - PANE_CENTER_MIN_WIDTH : limits.max;
    const effectiveMax = Math.max(limits.min, layoutMax > 0 ? layoutMax : limits.max);
    return Math.round(Math.min(effectiveMax, Math.max(limits.min, width)));
  }

  async function setPaneWidth(side: PaneResizeSide, width: number, persist = false) {
    const nextWidth = clampPaneWidth(side, width);
    if (side === "left") {
      leftSidebarWidth.value = nextWidth;
    } else {
      rightSidebarWidth.value = nextWidth;
    }
    if (persist) storePaneWidth(side, nextWidth);
    await nextTick();
    syncViewportMetrics();
  }

  // ========== pointer resize ==========

  function startPaneResize(side: PaneResizeSide, event: PointerEvent) {
    if (event.button !== 0) return;
    event.preventDefault();
    activePaneResizeSide.value = side;
    resizeLockedLeftPaneInLayout.value = leftPaneInLayout.value;
    resizeLockedRightPaneInLayout.value = rightPaneInLayout.value;
    paneResizeStartX = event.clientX;
    paneResizeStartWidth = side === "left" ? leftPaneVisibleWidth.value : rightPaneVisibleWidth.value;
    paneResizePreviousBodyCursor = document.body.style.cursor;
    paneResizePreviousBodyUserSelect = document.body.style.userSelect;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    window.addEventListener("pointermove", handlePaneResizeMove);
    window.addEventListener("pointerup", stopPaneResize, { once: true });
    window.addEventListener("pointercancel", stopPaneResize, { once: true });
  }

  function handlePaneResizeMove(event: PointerEvent) {
    const side = activePaneResizeSide.value;
    if (!side) return;
    const pointerDelta = event.clientX - paneResizeStartX;
    const nextWidth = side === "left" ? paneResizeStartWidth + pointerDelta : paneResizeStartWidth - pointerDelta;
    const nextWidthClamped = clampPaneWidth(side, nextWidth);
    if (side === "left") {
      leftSidebarWidth.value = nextWidthClamped;
    } else {
      rightSidebarWidth.value = nextWidthClamped;
    }
  }

  async function stopPaneResize() {
    const side = activePaneResizeSide.value;
    window.removeEventListener("pointermove", handlePaneResizeMove);
    window.removeEventListener("pointerup", stopPaneResize);
    window.removeEventListener("pointercancel", stopPaneResize);
    document.body.style.cursor = paneResizePreviousBodyCursor;
    document.body.style.userSelect = paneResizePreviousBodyUserSelect;
    activePaneResizeSide.value = null;
    resizeLockedLeftPaneInLayout.value = null;
    resizeLockedRightPaneInLayout.value = null;
    if (side) {
      storePaneWidth(side, side === "left" ? leftSidebarWidth.value : rightSidebarWidth.value);
      onPaneWidthsCommit(leftSidebarWidth.value, rightSidebarWidth.value);
    }
  }

  function adjustPaneWidthByKeyboard(side: PaneResizeSide, delta: number) {
    const currentWidth = side === "left" ? leftPaneVisibleWidth.value : rightPaneVisibleWidth.value;
    setPaneWidth(side, currentWidth + delta, true);
    onPaneWidthsCommit(leftSidebarWidth.value, rightSidebarWidth.value);
  }

  // ========== watchers & lifecycle ==========

  watch(
    [leftSidebarWidth, rightSidebarWidth],
    ([leftWidth, rightWidth]) => {
      onPaneWidthsChange(leftWidth, rightWidth);
    },
    { immediate: true },
  );

  watch(
    () => showSideConversationList.value,
    (open, prevOpen) => {
      if (open && !prevOpen) lastOpenedPane.value = "left";
    },
  );

  watch(
    () => toolReviewPanelOpen.value,
    (open, prevOpen) => {
      if (open && !prevOpen) lastOpenedPane.value = "right";
    },
  );


  void nextTick(() => {
    measureContainerWidth();
    if (typeof ResizeObserver !== "undefined" && chatLayoutRoot.value) {
      layoutResizeObserver = new ResizeObserver(() => measureContainerWidth());
      layoutResizeObserver.observe(chatLayoutRoot.value);
    }
  });

  onBeforeUnmountCleanup(() => {
    stopPaneResize();
    layoutResizeObserver?.disconnect();
    layoutResizeObserver = null;
  });

  return {
    leftSidebarWidth,
    rightSidebarWidth,
    leftPaneInLayout,
    rightPaneInLayout,
    leftPaneOverlay,
    rightPaneOverlay,
    leftPaneVisibleWidth,
    rightPaneVisibleWidth,
    activePaneResizeSide,
    clampPaneWidth,
    setPaneWidth,
    startPaneResize,
    adjustPaneWidthByKeyboard,
  };
}

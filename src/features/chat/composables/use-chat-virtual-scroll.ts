import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch, type Ref } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import type { ChatRenderItem } from "../utils/chat-render";
import { estimateChatRenderItemHeight } from "./use-chat-virtual-list";

interface UseChatVirtualScrollOptions {
  renderItems: Ref<ChatRenderItem[]>;
  scrollContainer: Ref<HTMLElement | null>;
  scrollbarRef: Ref<{ updateThumb: () => void } | null>;
  activeConversationId: Ref<string>;
  latestOwnElasticItemId: Ref<string>;
  latestOwnElasticMinHeight: Ref<number>;
  debugEnabled?: Ref<boolean> | boolean;
  smoothScrollEnabled?: Ref<boolean> | boolean;
  onUserScroll: () => void;
}

export function useChatVirtualScroll(options: UseChatVirtualScrollOptions) {
  const {
    renderItems,
    scrollContainer,
    scrollbarRef,
    activeConversationId,
    latestOwnElasticItemId,
    latestOwnElasticMinHeight,
    debugEnabled,
    smoothScrollEnabled,
    onUserScroll,
  } = options;

  const observedVirtualItemElements = new Map<string, HTMLElement>();
  const observedVirtualItemResizeElements = new Map<string, HTMLElement>();
  const measuredVirtualItemHeights = new Map<string, number>();
  const measuredVirtualItemRevision = ref(0);

  let pendingMeasureFrame = 0;
  let pendingVirtualResizeFrame = 0;
  const pendingVirtualResizeElements = new Set<HTMLElement>();
  let virtualItemResizeObserver: ResizeObserver | null = null;

  const initialBottomOffset = ref(0);
  let conversationVirtualizerResetRequest = 0;
  let debugTraceRequest = 0;
  let pendingConversationBottomInitializationId = "";

  // ==================== virtualizer ====================

  function estimateRenderItemSize(index: number): number {
    const item = renderItems.value[index];
    return estimateChatRenderItemHeight(item);
  }

  function estimateTotalRenderSize(): number {
    return renderItems.value.reduce((total, _item, index) => total + estimateRenderItemSize(index), 0);
  }

  function measuredOrEstimatedRenderItemSize(index: number): number {
    const item = renderItems.value[index];
    if (!item) return 0;
    return measuredVirtualItemHeights.get(item.id) ?? estimateRenderItemSize(index);
  }

  const latestOwnTailContentHeight = computed(() => {
    if (measuredVirtualItemRevision.value < 0) return 0;
    const itemId = String(latestOwnElasticItemId.value || "").trim();
    if (!itemId) return 0;
    const startIndex = renderItems.value.findIndex((item) => item.id === itemId);
    if (startIndex < 0) return 0;
    let total = 0;
    for (let index = startIndex; index < renderItems.value.length; index += 1) {
      total += measuredOrEstimatedRenderItemSize(index);
    }
    return total;
  });

  function chatVirtualScrollDebugEnabled(): boolean {
    if (typeof window === "undefined") return false;
    const configuredDebugEnabled = typeof debugEnabled === "object" && debugEnabled && "value" in debugEnabled
      ? debugEnabled.value
      : debugEnabled;
    if (configuredDebugEnabled === false) return false;
    return window.localStorage.getItem("easy-call.debug.chat-virtual-scroll") === "1"
      || (window as any).__easyCallDebugChatVirtualScroll === true;
  }

  function nativeSmoothScrollEnabled(): boolean {
    const configured = typeof smoothScrollEnabled === "object" && smoothScrollEnabled && "value" in smoothScrollEnabled
      ? smoothScrollEnabled.value
      : smoothScrollEnabled;
    return configured !== false;
  }

  function debugVirtualScrollState(label: string) {
    const scrollEl = scrollContainer.value;
    const rows = virtualizer.value.getVirtualItems();
    const firstRow = rows[0];
    const lastRow = rows[rows.length - 1];
    const range = rows.length > 0 ? `${firstRow?.index}-${lastRow?.index}` : "empty";
    const scrollTop = Math.round(scrollEl?.scrollTop ?? 0);
    const scrollHeight = Math.round(scrollEl?.scrollHeight ?? 0);
    const clientHeight = Math.round(scrollEl?.clientHeight ?? 0);
    const distanceToBottom = Math.round(scrollHeight - scrollTop - clientHeight);
    console.warn(
      `[聊天虚拟滚动] ${label}`
      + ` count=${renderItems.value.length}`
      + ` range=${range}`
      + ` scroll=${scrollTop}/${clientHeight}/${scrollHeight}`
      + ` bottom=${distanceToBottom}`
      + ` init=${Math.round(initialBottomOffset.value)}`
      + ` est=${Math.round(estimateTotalRenderSize())}`
      + ` total=${Math.round(virtualizer.value.getTotalSize())}`
      + ` elastic=${latestOwnElasticItemId.value ? "yes" : "no"}:${Math.round(latestOwnElasticMinHeight.value)}`
      + ` tail=${Math.round(latestOwnTailContentHeight.value)}`,
    );
  }

  const virtualizer = useVirtualizer(
    computed(() => ({
      count: renderItems.value.length,
      getScrollElement: () => scrollContainer.value,
      getItemKey: (index: number) => renderItems.value[index]?.id ?? `row-${index}`,
      estimateSize: estimateRenderItemSize,
      initialOffset: () => initialBottomOffset.value,
      anchorTo: "end",
      followOnAppend: true,
      scrollEndThreshold: 100,
      measureElement: (element: Element) => (element as HTMLElement).getBoundingClientRect().height,
      overscan: 600,
    })),
  );

  const virtualRows = computed(() => virtualizer.value.getVirtualItems());
  const virtualEntries = computed(() =>
    virtualRows.value
      .map((row) => {
        const item = renderItems.value[row.index];
        return item ? { row, item } : null;
      })
      .filter((entry): entry is { row: typeof virtualRows.value[number]; item: ChatRenderItem } => Boolean(entry)),
  );
  const totalVirtualSize = computed(() => virtualizer.value.getTotalSize());
  const virtualDebugVisible = computed(() => chatVirtualScrollDebugEnabled());
  const virtualDebugState = computed(() => {
    const scrollEl = scrollContainer.value;
    const rows = virtualizer.value.getVirtualItems();
    const firstRow = rows[0];
    const lastRow = rows[rows.length - 1];
    const firstItem = firstRow ? renderItems.value[firstRow.index] : undefined;
    const lastItem = lastRow ? renderItems.value[lastRow.index] : undefined;
    return {
      conversationId: String(activeConversationId.value || "").trim(),
      itemCount: renderItems.value.length,
      initialBottomOffset: Math.round(initialBottomOffset.value),
      estimatedTotal: Math.round(estimateTotalRenderSize()),
      totalSize: Math.round(virtualizer.value.getTotalSize()),
      scrollTop: Math.round(scrollEl?.scrollTop ?? 0),
      scrollHeight: Math.round(scrollEl?.scrollHeight ?? 0),
      clientHeight: Math.round(scrollEl?.clientHeight ?? 0),
      range: rows.length > 0 ? `${firstRow?.index}-${lastRow?.index}` : "empty",
      firstItemId: firstItem?.id || "",
      lastItemId: lastItem?.id || "",
      latestOwnElasticItemId: latestOwnElasticItemId.value,
      latestOwnElasticMinHeight: Math.round(latestOwnElasticMinHeight.value),
      latestOwnTailContentHeight: Math.round(latestOwnTailContentHeight.value),
    };
  });

  // ==================== helpers ====================

  function stableBlockIdFromRenderItem(item: ChatRenderItem): string {
    if (item.kind === "message" || item.kind === "compaction" || item.kind === "plan_started") {
      return String(item.block.id || item.block.sourceMessageId || item.renderId || "").trim();
    }
    if (item.kind === "group") {
      const first = item.items[0];
      return String(first?.block.id || first?.block.sourceMessageId || first?.renderId || "").trim();
    }
    return "";
  }

  function findRenderItemIdByStableBlockId(stableBlockId: string): string {
    const normalizedStableBlockId = String(stableBlockId || "").trim();
    if (!normalizedStableBlockId) return "";
    const matchedEntry = virtualEntries.value.find((entry) => stableBlockIdFromRenderItem(entry.item) === normalizedStableBlockId);
    return String(matchedEntry?.item.id || "").trim();
  }

  function captureViewportAnchor(): { blockId: string; offsetTop: number } | null {
    const scrollEl = scrollContainer.value;
    if (!scrollEl) return null;
    const containerRect = scrollEl.getBoundingClientRect();
    const containerTop = containerRect.top;
    const containerBottom = containerRect.bottom;
    let fallbackAnchor: { blockId: string; offsetTop: number } | null = null;
    for (const entry of virtualEntries.value) {
      const blockId = stableBlockIdFromRenderItem(entry.item);
      if (!blockId) continue;
      const wrapper = observedVirtualItemElements.get(entry.item.id);
      if (!wrapper || !wrapper.isConnected) continue;
      const wrapperRect = wrapper.getBoundingClientRect();
      const nextAnchor = {
        blockId,
        offsetTop: wrapperRect.top - containerRect.top,
      };
      if (!fallbackAnchor) {
        fallbackAnchor = nextAnchor;
      }
      const isVisible = wrapperRect.bottom > containerTop && wrapperRect.top < containerBottom;
      if (isVisible) {
        return nextAnchor;
      }
    }
    return fallbackAnchor;
  }

  function restoreViewportAnchor(anchor: { blockId: string; offsetTop: number } | null): boolean {
    if (!anchor) return false;
    const scrollEl = scrollContainer.value;
    if (!scrollEl) return false;
    const renderItemId = findRenderItemIdByStableBlockId(anchor.blockId);
    if (!renderItemId) return false;
    const wrapper = observedVirtualItemElements.get(renderItemId);
    if (!wrapper || !wrapper.isConnected) return false;
    const containerRect = scrollEl.getBoundingClientRect();
    const wrapperRect = wrapper.getBoundingClientRect();
    const delta = (wrapperRect.top - containerRect.top) - anchor.offsetTop;
    if (Math.abs(delta) < 1) return true;
    scrollEl.scrollTop += delta;
    return true;
  }

  // ==================== resize handling ====================

  function handleVirtualItemResize(element: HTMLElement) {
    const itemId = String(element.getAttribute("data-render-item-id") || "").trim();
    if (!itemId) return;
    const nextHeight = Math.round(element.getBoundingClientRect().height);
    const previousHeight = measuredVirtualItemHeights.get(itemId);
    if (previousHeight === nextHeight) {
      observedVirtualItemElements.set(itemId, element);
      return;
    }
    virtualizer.value.measureElement(element);
    measuredVirtualItemHeights.set(itemId, nextHeight);
    measuredVirtualItemRevision.value += 1;
    observedVirtualItemElements.set(itemId, element);
  }

  function scheduleVirtualMeasure() {
    if (pendingMeasureFrame) return;
    void nextTick(() => {
      if (pendingMeasureFrame) return;
      pendingMeasureFrame = requestAnimationFrame(() => {
        pendingMeasureFrame = 0;
        refreshObservedVirtualItemElements();
        virtualizer.value.measure();
      });
    });
  }

  function scheduleVirtualResizeMeasure(entries: ResizeObserverEntry[]) {
    for (const entry of entries) {
      if (entry.target instanceof HTMLElement) {
        pendingVirtualResizeElements.add(entry.target);
      }
    }
    if (pendingVirtualResizeFrame) return;
    pendingVirtualResizeFrame = requestAnimationFrame(() => {
      pendingVirtualResizeFrame = 0;
      const elements = Array.from(pendingVirtualResizeElements);
      pendingVirtualResizeElements.clear();
      for (const element of elements) {
        if (!element.isConnected) continue;
        handleVirtualItemResize(element);
      }
    });
  }

  // ==================== measurement ====================

  function measureVirtualRow(itemId: string, element: Element | { $el?: Element } | null) {
    const normalizedItemId = String(itemId || "").trim();
    if (!element) {
      if (normalizedItemId) {
        const previousResizeElement = observedVirtualItemResizeElements.get(normalizedItemId);
        if (previousResizeElement && virtualItemResizeObserver) {
          virtualItemResizeObserver.unobserve(previousResizeElement);
        }
        observedVirtualItemResizeElements.delete(normalizedItemId);
        observedVirtualItemElements.delete(normalizedItemId);
        if (measuredVirtualItemHeights.delete(normalizedItemId)) {
          measuredVirtualItemRevision.value += 1;
        }
      }
      return;
    }
    const target = element instanceof Element ? element : ((element as any).$el as Element | undefined) ?? null;
    if (!target) {
      if (normalizedItemId) {
        const previousResizeElement = observedVirtualItemResizeElements.get(normalizedItemId);
        if (previousResizeElement && virtualItemResizeObserver) {
          virtualItemResizeObserver.unobserve(previousResizeElement);
        }
        observedVirtualItemResizeElements.delete(normalizedItemId);
        observedVirtualItemElements.delete(normalizedItemId);
        if (measuredVirtualItemHeights.delete(normalizedItemId)) {
          measuredVirtualItemRevision.value += 1;
        }
      }
      return;
    }
    virtualizer.value.measureElement(target);
    const resolvedItemId = normalizedItemId || String(target.getAttribute("data-render-item-id") || "").trim();
    if (resolvedItemId && target instanceof HTMLElement) {
      const previousResizeElement = observedVirtualItemResizeElements.get(resolvedItemId);
      if (previousResizeElement && previousResizeElement !== target && virtualItemResizeObserver) {
        virtualItemResizeObserver.unobserve(previousResizeElement);
      }
      if (virtualItemResizeObserver && previousResizeElement !== target) {
        virtualItemResizeObserver.observe(target);
      }
      observedVirtualItemResizeElements.set(resolvedItemId, target);
      const nextHeight = Math.round(target.getBoundingClientRect().height);
      if (measuredVirtualItemHeights.get(resolvedItemId) !== nextHeight) {
        measuredVirtualItemHeights.set(resolvedItemId, nextHeight);
        measuredVirtualItemRevision.value += 1;
      }
      observedVirtualItemElements.set(resolvedItemId, target);
    }
  }

  function refreshObservedVirtualItemElements() {
    const validIds = new Set<string>();
    for (const entry of virtualEntries.value) {
      const itemId = String(entry.item.id || "").trim();
      if (!itemId) continue;
      validIds.add(itemId);
      if (entry.item.kind === "message" || entry.item.kind === "group") {
        const blocks = entry.item.kind === "message" ? [entry.item.block] : entry.item.items.map((g) => g.block);
        blocks.forEach((block) => {
          const blockId = String(block.id || "").trim();
          if (blockId) validIds.add(blockId);
        });
      }
    }
    for (const [itemId] of observedVirtualItemElements.entries()) {
      if (!validIds.has(itemId)) {
        const resizeElement = observedVirtualItemResizeElements.get(itemId);
        if (resizeElement && virtualItemResizeObserver) {
          virtualItemResizeObserver.unobserve(resizeElement);
        }
        observedVirtualItemResizeElements.delete(itemId);
        observedVirtualItemElements.delete(itemId);
        if (measuredVirtualItemHeights.delete(itemId)) {
          measuredVirtualItemRevision.value += 1;
        }
      }
    }
  }

  function clearMeasuredVirtualState() {
    for (const element of observedVirtualItemResizeElements.values()) {
      virtualItemResizeObserver?.unobserve(element);
    }
    observedVirtualItemElements.clear();
    observedVirtualItemResizeElements.clear();
    measuredVirtualItemHeights.clear();
    measuredVirtualItemRevision.value += 1;
    pendingVirtualResizeElements.clear();
  }

  function resetVirtualizerAtConversationBottom() {
    const requestId = ++conversationVirtualizerResetRequest;
    clearMeasuredVirtualState();
    initialBottomOffset.value = estimateTotalRenderSize();
    virtualizer.value.measure();
    void nextTick(async () => {
      if (requestId !== conversationVirtualizerResetRequest) return;
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      if (requestId !== conversationVirtualizerResetRequest) return;
      if (renderItems.value.length > 0) {
        virtualizer.value.scrollToEnd({ behavior: "auto" });
      }
      const scrollEl = scrollContainer.value;
      if (scrollEl) {
        scrollEl.scrollTop = Math.max(0, scrollEl.scrollHeight - scrollEl.clientHeight);
      }
      scrollbarRef.value?.updateThumb();
    });
  }

  function beginConversationBottomInitialization() {
    const conversationId = String(activeConversationId.value || "").trim();
    pendingConversationBottomInitializationId = conversationId;
    clearMeasuredVirtualState();
    initialBottomOffset.value = estimateTotalRenderSize();
  }

  function renderListReadyKey() {
    const items = renderItems.value;
    const firstId = String(items[0]?.id || "").trim();
    const lastId = String(items[items.length - 1]?.id || "").trim();
    return `${items.length}:${firstId}:${lastId}`;
  }

  function resolvePendingConversationBottomInitialization() {
    const conversationId = String(activeConversationId.value || "").trim();
    if (!pendingConversationBottomInitializationId || pendingConversationBottomInitializationId !== conversationId) return;
    if (renderItems.value.length <= 0) return;
    pendingConversationBottomInitializationId = "";
    resetVirtualizerAtConversationBottom();
  }

  // ==================== scroll helpers ====================

  function syncViewportMetrics() {
    scheduleVirtualMeasure();
    void nextTick(() => scrollbarRef.value?.updateThumb());
  }

  function alignItemToTop(itemId: string, behavior: ScrollBehavior = "smooth") {
    const scrollEl = scrollContainer.value;
    if (!scrollEl || !itemId) return;
    const wrapper = observedVirtualItemElements.get(itemId);
    if (!wrapper || !wrapper.isConnected) return;
    const containerRect = scrollEl.getBoundingClientRect();
    const wrapperRect = wrapper.getBoundingClientRect();
    const scrollStyles = window.getComputedStyle(scrollEl);
    const targetTop = parseFloat(scrollStyles.paddingTop || "0");
    const nextTop = scrollEl.scrollTop + (wrapperRect.top - containerRect.top) - targetTop;
    const resolvedBehavior: ScrollBehavior = behavior === "smooth" && !nativeSmoothScrollEnabled() ? "auto" : behavior;
    scrollEl.scrollTo({ top: Math.max(0, nextTop), behavior: resolvedBehavior });
    onUserScroll();
  }

  // ==================== lifecycle ====================

  onMounted(() => {
    virtualizer.value.shouldAdjustScrollPositionOnItemSizeChange = () => false;
    if (typeof ResizeObserver !== "undefined") {
      virtualItemResizeObserver = new ResizeObserver((entries) => {
        scheduleVirtualResizeMeasure(entries);
      });
      for (const element of observedVirtualItemResizeElements.values()) {
        if (!element.isConnected) continue;
        virtualItemResizeObserver.observe(element);
      }
    }
  });

  watch(
    () => String(activeConversationId.value || "").trim(),
    () => {
      beginConversationBottomInitialization();
    },
    { immediate: true, flush: "post" },
  );

  watch(
    renderListReadyKey,
    () => {
      void nextTick(() => resolvePendingConversationBottomInitialization());
    },
    { immediate: true, flush: "post" },
  );

  onBeforeUnmount(() => {
    conversationVirtualizerResetRequest += 1;
    debugTraceRequest += 1;
    pendingConversationBottomInitializationId = "";
    virtualItemResizeObserver?.disconnect();
    virtualItemResizeObserver = null;
    if (pendingMeasureFrame) {
      cancelAnimationFrame(pendingMeasureFrame);
      pendingMeasureFrame = 0;
    }
    if (pendingVirtualResizeFrame) {
      cancelAnimationFrame(pendingVirtualResizeFrame);
      pendingVirtualResizeFrame = 0;
    }
    pendingVirtualResizeElements.clear();
    observedVirtualItemElements.clear();
    observedVirtualItemResizeElements.clear();
    measuredVirtualItemHeights.clear();
  });

  return {
    virtualizer,
    virtualRows,
    virtualEntries,
    totalVirtualSize,
    latestOwnTailContentHeight,
    virtualDebugVisible,
    virtualDebugState,
    measureVirtualRow,
    refreshObservedVirtualItemElements,
    scheduleVirtualMeasure,
    syncViewportMetrics,
    resetVirtualizerAtConversationBottom,
    alignItemToTop,
    captureViewportAnchor,
    restoreViewportAnchor,
  };
}

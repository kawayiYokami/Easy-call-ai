import { computed, nextTick, onBeforeUnmount, ref, watch, type ComponentPublicInstance, type Ref } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import type { ChatRenderItem } from "../utils/chat-render";

interface UseChatVirtualScrollOptions {
  renderItems: Ref<ChatRenderItem[]>;
  scrollContainer: Ref<HTMLElement | null>;
  scrollbarRef: Ref<{ updateThumb: () => void } | null>;
  activeConversationId: Ref<string>;
  latestOwnElasticItemId: Ref<string>;
  latestOwnElasticMinHeight: Ref<number>;
  chatting?: Ref<boolean> | boolean;
  olderHistoryCorrectionAllowed?: Ref<boolean> | boolean;
  debugEnabled?: Ref<boolean> | boolean;
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
    chatting,
    olderHistoryCorrectionAllowed,
    debugEnabled,
    onUserScroll,
  } = options;

  let pendingMeasureFrame = 0;

  let completionLayoutGuardActive = false;
  let completionLayoutGuardFrame = 0;
  let explicitVirtualScrollActive = false;
  let explicitVirtualScrollFrame = 0;
  // 切换会话后「最后一条消息可见」意图：非空且匹配当前会话时，放行 tanstack 滚动，
  // 并在行测量期间持续重试 scrollToIndex，直到最后一条消息进入视口或用户介入。
  let pendingLastItemVisibleIntent = "";
  let lastItemIntentRetryFrame = 0;
  let lastItemIntentStableTotal = -1;
  let lastItemIntentStableFrames = 0;
  let lastItemIntentStartedAt = 0;
  let stopScrollContainerIntentWatch: (() => void) | null = null;

  const initialBottomOffset = ref(0);
  let conversationVirtualizerResetRequest = 0;
  let pendingConversationBottomInitializationId = "";
  // 顶部 10% 区域切换到 anchorTo="end"，让加载更多 prepend 走 tanstack 锚定补偿；
  // 其余区域流式期间保持 "start"，不随行增长推 offset。
  const ANCHOR_TOP_ZONE_RATIO = 0.1;
  const nearTopZone = ref(false);
  let stopScrollContainerAnchorWatch: (() => void) | null = null;

  function evaluateNearTopZone() {
    const scrollEl = scrollContainer.value;
    if (!scrollEl) return;
    nearTopZone.value = scrollEl.scrollTop <= scrollEl.clientHeight * ANCHOR_TOP_ZONE_RATIO;
  }

  // ==================== virtualizer ====================

  // 弹性尾部高度直接读 TanStack itemSizeCache（与布局同源，官方 measureElement
  // 维护）；virtualizer 是 shallowRef，TanStack notify 时 vue-virtual 会 triggerRef，
  // 因此 computed 读 virtualizer.value 会在测量更新时自动重算。
  const latestOwnTailContentRange = computed(() => {
    const itemId = String(latestOwnElasticItemId.value || "").trim();
    if (!itemId) return [];
    const startIndex = renderItems.value.findIndex((item) => item.id === itemId);
    return startIndex < 0 ? [] : renderItems.value.slice(startIndex);
  });

  const latestOwnTailContentHeight = computed(() => {
    const itemSizeCache = virtualizer.value.itemSizeCache;
    return latestOwnTailContentRange.value.reduce(
      (total, item) => total + (itemSizeCache.get(item.id) ?? 0),
      0,
    );
  });

  const latestOwnTailContentMeasured = computed(() => {
    const tailItems = latestOwnTailContentRange.value;
    const itemSizeCache = virtualizer.value.itemSizeCache;
    return tailItems.length > 0 && tailItems.every((item) => itemSizeCache.has(item.id));
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

  function chatStreamingActive(): boolean {
    const configured = typeof chatting === "object" && chatting && "value" in chatting
      ? chatting.value
      : chatting;
    return !!configured;
  }

  function olderHistoryCorrectionActive(): boolean {
    const configured = typeof olderHistoryCorrectionAllowed === "object" && olderHistoryCorrectionAllowed && "value" in olderHistoryCorrectionAllowed
      ? olderHistoryCorrectionAllowed.value
      : olderHistoryCorrectionAllowed;
    return !!configured;
  }

  function virtualizerScrollToFn(
    offset: number,
    options: { adjustments?: number; behavior?: ScrollBehavior },
    instance: { scrollElement: Element | Window | null },
  ) {
    const scrollEl = instance.scrollElement instanceof HTMLElement ? instance.scrollElement : null;
    if (!scrollEl) return;
    const nextTop = Math.max(0, Math.round(Number(offset || 0)));
    // 切换会话后的「最后一条可见」意图期间放行滚动：scrollToIndex 的偏移基于当前
    // 最新测量，行未挂载时可能是估计值，需要在行测高后重试修正，因此期间不能拦。
    const currentConversationId = String(activeConversationId.value || "").trim();
    const intentActive = pendingLastItemVisibleIntent && pendingLastItemVisibleIntent === currentConversationId;
    if (intentActive) {
      scrollEl.scrollTo({
        top: nextTop,
        behavior: options.behavior || "auto",
      });
      return;
    }
    // 覆盖 virtualizer 的尺寸修正滚动：我们定位到流式消息高度持续增长时，
    // @tanstack/virtual 会走 ResizeObserver -> resizeItem -> applyScrollAdjustment
    // -> _scrollToOffset -> scrollToFn，把 scrollTop 连续往下补，维持距底部固定偏移。
    // 这会让聊天窗口在流式期间“自己往下走”。尺寸变化既可能携带 adjustments，
    // 也可能以普通 offset 重定位，因此流式期间只放行我们明确发起的滚动到底。
    if (
      (chatStreamingActive() || completionLayoutGuardActive)
      && !explicitVirtualScrollActive
      && !olderHistoryCorrectionActive()
    ) {
      return;
    }
    scrollEl.scrollTo({
      top: nextTop,
      behavior: options.behavior || "auto",
    });
  }

  function markExplicitVirtualScrollForNextFrame() {
    explicitVirtualScrollActive = true;
    if (explicitVirtualScrollFrame) {
      cancelAnimationFrame(explicitVirtualScrollFrame);
    }
    explicitVirtualScrollFrame = requestAnimationFrame(() => {
      explicitVirtualScrollFrame = 0;
      explicitVirtualScrollActive = false;
    });
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
      // 不用预估高度参与 absolute 行定位。富文本、工具、图片和代码块没有可靠上界，
      // 低估会让后一行在首帧压住前一行；挂载后统一以真实行元素实测高度定位。
      estimateSize: () => 1,
      initialOffset: () => initialBottomOffset.value,
      scrollToFn: virtualizerScrollToFn,
      // 流式期间锚定顶部：流式拦截会挡住 scrollToFn 的 DOM 滚动，但 tanstack 内部
      // 逻辑 offset 仍会因 anchorTo="end" 持续下推，造成逻辑位置与 DOM 脱节、
      // 行定位错位（切会话滚到底也依赖 tanstack 数据而失效）。流式期间改为
      // start（锚定视口顶部），行增长不再推 offset；静止后恢复 end 锚定。
      // 靠近顶部 10% 区域时切回 end：tanstack 的 prepend 锚定补偿只在 anchorTo="end"
      // 时计算（setOptions 里 merged.anchorTo === "end" 分支），顶部区域对应加载更多
      // 场景，prepend 后视口需要补偿保持。
      anchorTo: chatStreamingActive() && !nearTopZone.value ? "start" : "end",
      // 仅在精确贴底时才允许尾部锚定跟随，避免“接近底部”时被持续往下带。
      scrollEndThreshold: 0,
      shouldAdjustScrollPositionOnItemSizeChange: (item: { end: number }, _delta: number, instance: {
        getScrollOffset: () => number;
        getSize: () => number;
      }) => {
        const viewportTop = instance.getScrollOffset();
        const viewportBottom = viewportTop + instance.getSize();
        // 只补偿当前视口上方的尺寸变化；底部新增内容和视口内变化不要自动推着用户往下走。
        return item.end <= viewportTop || viewportBottom <= 0;
      },
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
      measuredTotal: Math.round(virtualizer.value.getTotalSize()),
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

  watch(
    () => chatStreamingActive(),
    (isStreaming, wasStreaming) => {
      if (!wasStreaming || isStreaming) return;
      const scrollEl = scrollContainer.value;
      if (!scrollEl || typeof window === "undefined") return;
      const preservedScrollTop = scrollEl.scrollTop;
      completionLayoutGuardActive = true;
      if (completionLayoutGuardFrame) {
        window.cancelAnimationFrame(completionLayoutGuardFrame);
      }
      completionLayoutGuardFrame = window.requestAnimationFrame(() => {
        completionLayoutGuardFrame = window.requestAnimationFrame(() => {
          completionLayoutGuardFrame = 0;
          scrollEl.scrollTop = preservedScrollTop;
          completionLayoutGuardActive = false;
          scrollbarRef.value?.updateThumb();
        });
      });
    },
  );

  // ==================== helpers ====================

  function cancelCompletionLayoutGuard() {
    if (completionLayoutGuardFrame && typeof window !== "undefined") {
      window.cancelAnimationFrame(completionLayoutGuardFrame);
      completionLayoutGuardFrame = 0;
    }
    completionLayoutGuardActive = false;
  }

  function cancelPendingLastItemIntent(source: string) {
    pendingLastItemVisibleIntent = "";
    if (lastItemIntentRetryFrame && typeof window !== "undefined") {
      window.cancelAnimationFrame(lastItemIntentRetryFrame);
      lastItemIntentRetryFrame = 0;
    }
  }

  // 切换会话后的「弹性尾部可见」等待：行挂载且测量稳定后，用 tanstack 的
  // scrollToIndex(最后一条, align=end) 一次性定位到弹性尾部底部（对齐视口底部）。
  // 不用 DOM 逐帧修正：行测量/translateY/tailSpacer 更新与滚动互相交错时，
  // DOM 偏差修正会形成反馈振荡（scrollHeight 反复横跳、scrollTop 被钳到顶）。
  // 等 totalSize 连续 2 帧稳定（或超时兜底）再定位，此时行定位已正确。
  function maybeRetryLastItemVisibleIntent() {
    const conversationId = String(activeConversationId.value || "").trim();
    if (!pendingLastItemVisibleIntent || pendingLastItemVisibleIntent !== conversationId) return;
    const lastIndex = renderItems.value.length - 1;
    if (lastIndex < 0) {
      pendingLastItemVisibleIntent = "";
      return;
    }
    const scrollEl = scrollContainer.value;
    if (!scrollEl) return;
    const lastItem = renderItems.value[lastIndex];
    const lastItemId = String(lastItem?.id || "").trim();
    // 元素挂载判断走官方 elementsCache（ref 回调注册），与项目缓存解耦。
    const lastElement = lastItemId ? virtualizer.value.elementsCache.get(lastItemId) : undefined;
    const total = Math.round(virtualizer.value.getTotalSize());
    if (total === lastItemIntentStableTotal) {
      lastItemIntentStableFrames += 1;
    } else {
      lastItemIntentStableTotal = total;
      lastItemIntentStableFrames = 0;
    }
    const elapsedMs = lastItemIntentStartedAt > 0 ? Date.now() - lastItemIntentStartedAt : 0;
    if (lastElement && (lastItemIntentStableFrames >= 2 || elapsedMs > 1000)) {
      virtualizer.value.scrollToIndex(lastIndex, { align: "end", behavior: "auto" });
      pendingLastItemVisibleIntent = "";
      scrollbarRef.value?.updateThumb();
      return;
    }
    if (!lastItemIntentRetryFrame && typeof window !== "undefined") {
      lastItemIntentRetryFrame = requestAnimationFrame(() => {
        lastItemIntentRetryFrame = 0;
        maybeRetryLastItemVisibleIntent();
      });
    }
  }

  // ==================== measurement settled ====================

  // 初始测高完成信号：视口内所有行都已进入 itemSizeCache（实测高度完成）
  // 且 getTotalSize 连续两帧稳定。会话切换后行首帧按 estimateSize=1 定位会
  // 全部重叠在顶部，覆盖层据此信号放行显示；信号满足后保持 true，直到再次
  // 切换会话重置。
  const measurementSettled = ref(false);
  let settledStableTotal = -1;
  let settledStableFrames = 0;
  let settledRetryFrame = 0;

  function viewportRowsAllMeasured(): boolean {
    const rows = virtualizer.value.getVirtualItems();
    if (rows.length <= 0) return false;
    const cache = virtualizer.value.itemSizeCache;
    return rows.every((row) => cache.has(row.key));
  }

  function evaluateMeasurementSettled() {
    if (measurementSettled.value) return;
    const total = Math.round(virtualizer.value.getTotalSize());
    const allMeasured = viewportRowsAllMeasured();
    if (allMeasured) {
      if (total === settledStableTotal) {
        settledStableFrames += 1;
      } else {
        settledStableTotal = total;
        settledStableFrames = 0;
      }
      if (settledStableFrames >= 2) {
        measurementSettled.value = true;
        settledRetryFrame = 0;
        return;
      }
    } else {
      settledStableTotal = -1;
      settledStableFrames = 0;
    }
    if (!settledRetryFrame && typeof window !== "undefined") {
      settledRetryFrame = requestAnimationFrame(() => {
        settledRetryFrame = 0;
        evaluateMeasurementSettled();
      });
    }
  }

  // ==================== measurement ====================

  // 测量完全走官方 virtualizer.measureElement（模板 ref 直接绑定）：
  // 它负责 observe 元素（内部 ResizeObserver 驱动布局）并自带 itemSizeCache
  // 缓存短路，重复 patch 调用零 DOM 访问。弹性尾部高度从 itemSizeCache 读取，
  // 与布局同源，不再维护项目侧测量缓存。
  // measureElementRef 只是 Vue 模板 ref 回调签名的纯类型适配（官方签名只收
  // Element，Vue 的回调可能传入组件实例），不做任何测量逻辑，直接透传官方。
  const measureElementRef = (node: Element | ComponentPublicInstance | null) => {
    if (node instanceof Element) {
      virtualizer.value.measureElement(node);
    }
  };

  function scheduleVirtualMeasure() {
    if (pendingMeasureFrame) return;
    void nextTick(() => {
      if (pendingMeasureFrame) return;
      pendingMeasureFrame = requestAnimationFrame(() => {
        pendingMeasureFrame = 0;
        virtualizer.value.measure();
      });
    });
  }

  function scrollVirtualizerToConversationBottomLightweight(behavior: "auto" | "smooth" = "auto") {
    const scrollEl = scrollContainer.value;
    if (!scrollEl) return;
    void nextTick(async () => {
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      const targetTop = Math.max(0, scrollEl.scrollHeight - scrollEl.clientHeight);
      scrollEl.scrollTo({ top: targetTop, behavior });
      scrollbarRef.value?.updateThumb();
    });
  }

  function resetVirtualizerAtConversationBottom(behavior: "auto" | "smooth" = "auto") {
    const requestId = ++conversationVirtualizerResetRequest;
    initialBottomOffset.value = 0;
    virtualizer.value.measure();
    void nextTick(async () => {
      if (requestId !== conversationVirtualizerResetRequest) return;
      await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
      if (requestId !== conversationVirtualizerResetRequest) return;
      if (renderItems.value.length > 0) {
        markExplicitVirtualScrollForNextFrame();
        virtualizer.value.scrollToEnd({ behavior });
      }
      // smooth 滚动依赖浏览器原生动画，强制赋值 scrollTop 会打断它；仅在 auto 时兜底钳制到真实底端
      if (behavior !== "smooth") {
        const scrollEl = scrollContainer.value;
        if (scrollEl) {
          scrollEl.scrollTop = Math.max(0, scrollEl.scrollHeight - scrollEl.clientHeight);
        }
      }
      scrollbarRef.value?.updateThumb();
    });
  }

  function beginConversationBottomInitialization() {
    const conversationId = String(activeConversationId.value || "").trim();
    pendingConversationBottomInitializationId = conversationId;
    cancelPendingLastItemIntent("begin");
    cancelCompletionLayoutGuard();
    initialBottomOffset.value = 0;
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
    // 以「最后一条消息可见」为意图替代单次 scrollToEnd：切会话时行尚未挂载，
    // scrollToEnd 的偏移基于 estimateSize=1 的低估 totalSize，会卡在中间；
    // 意图模式下滚动目标锚定最后一条消息，行测高期间由 maybeRetryLastItemVisibleIntent
    // 持续重试，直到最后一条消息进入视口，或用户滚动介入清除。
    pendingLastItemVisibleIntent = conversationId;
    lastItemIntentStartedAt = Date.now();
    lastItemIntentStableTotal = -1;
    lastItemIntentStableFrames = 0;
    cancelCompletionLayoutGuard();
    initialBottomOffset.value = 0;
    virtualizer.value.measure();
    void nextTick(() => {
      if (pendingLastItemVisibleIntent !== conversationId) return;
      // 首次定位统一走弹性尾部定位（maybeRetry 内部等测量稳定后 scrollToIndex 一次到位）
      maybeRetryLastItemVisibleIntent();
    });
  }

  function syncViewportMetrics() {
    scheduleVirtualMeasure();
    void nextTick(() => scrollbarRef.value?.updateThumb());
  }

  function scrollVirtualizerToIndex(
    index: number,
    options?: { align?: "auto" | "start" | "center" | "end"; behavior?: ScrollBehavior },
  ) {
    markExplicitVirtualScrollForNextFrame();
    virtualizer.value.scrollToIndex(index, options);
  }

  // ==================== lifecycle ====================

  // 用户主动滚动（滚轮/触屏）立即取消「最后一条可见」意图，避免重试循环与用户对抗。
  const cancelByWheel = () => cancelPendingLastItemIntent("wheel");
  const cancelByTouch = () => cancelPendingLastItemIntent("touch");
  stopScrollContainerIntentWatch = watch(
    scrollContainer,
    (el, oldEl) => {
      if (oldEl) {
        oldEl.removeEventListener("wheel", cancelByWheel);
        oldEl.removeEventListener("touchstart", cancelByTouch);
      }
      if (el) {
        el.addEventListener("wheel", cancelByWheel, { passive: true });
        el.addEventListener("touchstart", cancelByTouch, { passive: true });
      }
    },
    { immediate: true },
  );

  // 监听滚动更新顶部 10% 区域，驱动 anchorTo 切换（带 1 秒冷却）。
  stopScrollContainerAnchorWatch = watch(
    scrollContainer,
    (el, oldEl) => {
      if (oldEl) {
        oldEl.removeEventListener("scroll", evaluateNearTopZone);
      }
      if (el) {
        el.addEventListener("scroll", evaluateNearTopZone, { passive: true });
      }
    },
    { immediate: true },
  );

  watch(
    () => String(activeConversationId.value || "").trim(),
    () => {
      beginConversationBottomInitialization();
      // 会话切换后行需要重新测高，重置覆盖层放行信号并启动检测。
      measurementSettled.value = false;
      settledStableTotal = -1;
      settledStableFrames = 0;
      if (settledRetryFrame && typeof window !== "undefined") {
        cancelAnimationFrame(settledRetryFrame);
        settledRetryFrame = 0;
      }
      if (typeof window !== "undefined") {
        settledRetryFrame = requestAnimationFrame(() => {
          settledRetryFrame = 0;
          evaluateMeasurementSettled();
        });
      }
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
    if (completionLayoutGuardFrame && typeof window !== "undefined") {
      window.cancelAnimationFrame(completionLayoutGuardFrame);
      completionLayoutGuardFrame = 0;
    }
    completionLayoutGuardActive = false;
    if (explicitVirtualScrollFrame && typeof window !== "undefined") {
      window.cancelAnimationFrame(explicitVirtualScrollFrame);
      explicitVirtualScrollFrame = 0;
    }
    explicitVirtualScrollActive = false;
    pendingLastItemVisibleIntent = "";
    if (lastItemIntentRetryFrame && typeof window !== "undefined") {
      window.cancelAnimationFrame(lastItemIntentRetryFrame);
      lastItemIntentRetryFrame = 0;
    }
    stopScrollContainerIntentWatch?.();
    stopScrollContainerIntentWatch = null;
    stopScrollContainerAnchorWatch?.();
    stopScrollContainerAnchorWatch = null;
    conversationVirtualizerResetRequest += 1;
    pendingConversationBottomInitializationId = "";
    if (pendingMeasureFrame) {
      cancelAnimationFrame(pendingMeasureFrame);
      pendingMeasureFrame = 0;
    }
    if (settledRetryFrame && typeof window !== "undefined") {
      cancelAnimationFrame(settledRetryFrame);
      settledRetryFrame = 0;
    }
  });

  return {
    virtualizer,
    virtualRows,
    virtualEntries,
    totalVirtualSize,
    latestOwnTailContentHeight,
    latestOwnTailContentMeasured,
    measurementSettled,
    measureElementRef,
    virtualDebugVisible,
    virtualDebugState,
    scheduleVirtualMeasure,
    syncViewportMetrics,
    scrollVirtualizerToIndex,
    scrollVirtualizerToConversationBottomLightweight,
    resetVirtualizerAtConversationBottom,
  };
}

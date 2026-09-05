<template>
  <div
    v-if="canScroll"
    ref="trackRef"
    class="floating-scrollbar-track absolute z-20 transition-opacity"
    :class="[trackClassName, scrollbarVisible || dragging ? 'opacity-100' : 'opacity-0']"
    @mouseenter="reveal"
    @mouseleave="hide"
    @pointerdown="onTrackPointerDown"
  >
    <div
      ref="thumbRef"
      class="floating-scrollbar-thumb absolute rounded-full transition-[width,height,background-color] hover:w-2 hover:h-2"
      :class="[thumbClassName, thumbGeometryClassName, dragging ? draggingClassName : '']"
      :style="thumbStyle"
      @pointerdown.stop="onThumbPointerDown"
    ></div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, toRef, watch, type Ref } from "vue";

const props = defineProps<{
  target: HTMLElement | null;
  variant?: "theme" | "code-dark";
  orientation?: "vertical" | "horizontal";
}>();

const targetRef = toRef(props, "target") as Ref<HTMLElement | null>;
const trackRef = ref<HTMLElement | null>(null);
const thumbRef = ref<HTMLElement | null>(null);
const canScroll = ref(false);
const scrollbarVisible = ref(false);
const dragging = ref(false);
const thumbSize = ref(24);
const thumbOffset = ref(0);

let resizeObserver: ResizeObserver | null = null;
let dragStartPointer = 0;
let dragStartScrollOffset = 0;
let activePointerId: number | null = null;
let observedScroller: HTMLElement | null = null;
let pendingThumbFrame = 0;
let cachedClientExtent = 0;
let cachedScrollExtent = 0;
let lastExtentRefreshAt = 0;
// 滚动热路径只读 scrollTop（廉价）；client/scroll extent 是布局依赖读，
// 强制同步重排代价高，改为缓存 + 最小间隔刷新
const EXTENT_REFRESH_MIN_INTERVAL_MS = 300;


const isHorizontal = computed(() => props.orientation === "horizontal");

const thumbClassName = computed(() => (
  props.variant === "code-dark"
    ? "bg-slate-400/45 hover:bg-slate-300/60"
    : "bg-base-content/30 hover:bg-base-content/45"
));

const trackClassName = computed(() => (
  isHorizontal.value
    ? "bottom-1 left-1 right-1 h-2"
    : "bottom-1 right-1 top-1 w-2"
));

const thumbGeometryClassName = computed(() => (
  isHorizontal.value
    ? "bottom-0 h-1.5"
    : "right-0 w-1.5"
));

const draggingClassName = computed(() => (isHorizontal.value ? "h-2" : "w-2"));

const thumbStyle = computed(() => (
  isHorizontal.value
    ? {
      width: `${thumbSize.value}px`,
      transform: `translateX(${thumbOffset.value}px)`,
    }
    : {
      height: `${thumbSize.value}px`,
      transform: `translateY(${thumbOffset.value}px)`,
    }
));

function setDocumentDragging(active: boolean) {
  document.body.classList.toggle("floating-scrollbar-dragging", active);
}

function updateThumbNow(forceExtentRefresh = false) {
  const scroller = targetRef.value;
  if (!scroller) return;

  const scrollOffset = isHorizontal.value ? scroller.scrollLeft : scroller.scrollTop;
  const now = performance.now();
  if (forceExtentRefresh || now - lastExtentRefreshAt >= EXTENT_REFRESH_MIN_INTERVAL_MS) {
    cachedClientExtent = isHorizontal.value ? scroller.clientWidth : scroller.clientHeight;
    cachedScrollExtent = isHorizontal.value ? scroller.scrollWidth : scroller.scrollHeight;
    lastExtentRefreshAt = now;
  }
  const clientExtent = cachedClientExtent;
  const scrollExtent = cachedScrollExtent;
  const scrollable = scrollExtent > clientExtent + 1;
  if (canScroll.value !== scrollable) {
    canScroll.value = scrollable;
  }
  if (!scrollable) {
    if (thumbOffset.value !== 0) {
      thumbOffset.value = 0;
    }
    return;
  }

  const trackExtent = Math.max(clientExtent - 8, 0);
  const size = Math.max(24, Math.round((clientExtent / scrollExtent) * trackExtent));
  const maxOffset = Math.max(trackExtent - size, 0);
  const nextOffset = maxOffset === 0
    ? 0
    : Math.round((scrollOffset / (scrollExtent - clientExtent)) * maxOffset);
  if (thumbSize.value !== size) {
    thumbSize.value = size;
  }
  if (thumbOffset.value !== nextOffset) {
    thumbOffset.value = nextOffset;
  }
}

function updateThumb(forceExtentRefresh = false) {
  if (pendingThumbFrame) return;
  pendingThumbFrame = requestAnimationFrame(() => {
    pendingThumbFrame = 0;
    updateThumbNow(forceExtentRefresh);
  });
}

function reveal() {
  updateThumbNow(true);
  if (!canScroll.value) return;
  scrollbarVisible.value = true;
}

function hide() {
  if (dragging.value) return;
  scrollbarVisible.value = false;
}

function handleScroll() {
  updateThumb();
  if (!scrollbarVisible.value) scrollbarVisible.value = true;
}

function disconnectObservers() {
  resizeObserver?.disconnect();
  resizeObserver = null;
  if (pendingThumbFrame) {
    cancelAnimationFrame(pendingThumbFrame);
    pendingThumbFrame = 0;
  }
}

function removeObservedScrollerListener(scroller = observedScroller) {
  scroller?.removeEventListener("scroll", handleScroll);
  if (!scroller || scroller === observedScroller) {
    observedScroller = null;
  }
}

function observeScroller(scroller: HTMLElement | null) {
  removeObservedScrollerListener();
  disconnectObservers();
  if (!scroller) {
    canScroll.value = false;
    return;
  }

  observedScroller = scroller;
  scroller.addEventListener("scroll", handleScroll, { passive: true });
  if (typeof ResizeObserver !== "undefined") {
    resizeObserver = new ResizeObserver(() => updateThumb(true));
    resizeObserver.observe(scroller);
  }
  void nextTick(() => updateThumb(true));
}

function scrollByThumbDelta(deltaPointer: number) {
  const scroller = targetRef.value;
  if (!scroller) return;
  const maxScrollOffset = Math.max(
    (isHorizontal.value ? scroller.scrollWidth : scroller.scrollHeight)
      - (isHorizontal.value ? scroller.clientWidth : scroller.clientHeight),
    0,
  );
  const maxThumbOffset = Math.max(
    (isHorizontal.value ? scroller.clientWidth : scroller.clientHeight) - 8 - thumbSize.value,
    0,
  );
  if (maxScrollOffset <= 0 || maxThumbOffset <= 0) return;
  const nextOffset = dragStartScrollOffset + (deltaPointer / maxThumbOffset) * maxScrollOffset;
  if (isHorizontal.value) {
    scroller.scrollLeft = nextOffset;
  } else {
    scroller.scrollTop = nextOffset;
  }
}

function onDocumentPointerMove(event: PointerEvent) {
  if (!dragging.value || activePointerId !== event.pointerId) return;
  event.preventDefault();
  scrollByThumbDelta((isHorizontal.value ? event.clientX : event.clientY) - dragStartPointer);
}

function stopDragging() {
  if (!dragging.value) return;
  dragging.value = false;
  setDocumentDragging(false);
  document.removeEventListener("pointermove", onDocumentPointerMove);
  document.removeEventListener("pointerup", onDocumentPointerUp);
  document.removeEventListener("pointercancel", onDocumentPointerUp);
  if (activePointerId !== null) {
    thumbRef.value?.releasePointerCapture?.(activePointerId);
  }
  activePointerId = null;
  updateThumb();
  if (!trackRef.value?.matches(":hover")) hide();
}

function onDocumentPointerUp(event: PointerEvent) {
  if (activePointerId !== null && activePointerId !== event.pointerId) return;
  stopDragging();
}

function onThumbPointerDown(event: PointerEvent) {
  if (event.button !== 0) return;
  const scroller = targetRef.value;
  if (!scroller) return;
  event.preventDefault();
  reveal();
  dragging.value = true;
  activePointerId = event.pointerId;
  dragStartPointer = isHorizontal.value ? event.clientX : event.clientY;
  dragStartScrollOffset = isHorizontal.value ? scroller.scrollLeft : scroller.scrollTop;
  setDocumentDragging(true);
  thumbRef.value?.setPointerCapture?.(event.pointerId);
  document.addEventListener("pointermove", onDocumentPointerMove, { passive: false });
  document.addEventListener("pointerup", onDocumentPointerUp);
  document.addEventListener("pointercancel", onDocumentPointerUp);
}

function onTrackPointerDown(event: PointerEvent) {
  if (event.button !== 0 || event.target === thumbRef.value) return;
  const scroller = targetRef.value;
  const track = trackRef.value;
  if (!scroller || !track) return;
  event.preventDefault();
  reveal();
  const trackRect = track.getBoundingClientRect();
  const trackStart = isHorizontal.value ? trackRect.left : trackRect.top;
  const trackExtent = isHorizontal.value ? trackRect.width : trackRect.height;
  const pointer = isHorizontal.value ? event.clientX : event.clientY;
  const nextThumbOffset = Math.min(
    Math.max(pointer - trackStart - thumbSize.value / 2, 0),
    Math.max(trackExtent - thumbSize.value, 0),
  );
  const maxThumbOffset = Math.max(trackExtent - thumbSize.value, 0);
  const maxScrollOffset = Math.max(
    (isHorizontal.value ? scroller.scrollWidth : scroller.scrollHeight)
      - (isHorizontal.value ? scroller.clientWidth : scroller.clientHeight),
    0,
  );
  const nextScrollOffset = maxThumbOffset === 0 ? 0 : (nextThumbOffset / maxThumbOffset) * maxScrollOffset;
  if (isHorizontal.value) {
    scroller.scrollLeft = nextScrollOffset;
  } else {
    scroller.scrollTop = nextScrollOffset;
  }
}

defineExpose({
  reveal,
  hide,
  updateThumb,
});

onMounted(() => observeScroller(targetRef.value));

watch(targetRef, (nextScroller, previousScroller) => {
  removeObservedScrollerListener(previousScroller);
  observeScroller(nextScroller);
});

onBeforeUnmount(() => {
  stopDragging();
  removeObservedScrollerListener();
  disconnectObservers();
});
</script>

<style scoped>
.floating-scrollbar-track {
  cursor: pointer;
  touch-action: none;
}

.floating-scrollbar-thumb {
  cursor: grab;
  touch-action: none;
}

.floating-scrollbar-thumb:active {
  cursor: grabbing;
}
</style>

<style>
body.floating-scrollbar-dragging {
  cursor: grabbing !important;
  user-select: none !important;
}

body.floating-scrollbar-dragging * {
  cursor: grabbing !important;
  user-select: none !important;
}
</style>

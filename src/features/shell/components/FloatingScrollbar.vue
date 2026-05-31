<template>
  <div
    v-if="canScroll"
    ref="trackRef"
    class="floating-scrollbar-track absolute bottom-1 right-1 top-1 z-20 w-2 transition-opacity"
    :class="scrollbarVisible || dragging ? 'opacity-100' : 'opacity-0'"
    @mouseenter="reveal"
    @mouseleave="hide"
    @pointerdown="onTrackPointerDown"
  >
    <div
      ref="thumbRef"
      class="floating-scrollbar-thumb absolute right-0 w-1.5 rounded-full bg-base-content/30 transition-[width,background-color] hover:w-2 hover:bg-base-content/45"
      :class="dragging ? 'w-2 bg-base-content/50' : ''"
      :style="thumbStyle"
      @pointerdown.stop="onThumbPointerDown"
    ></div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, toRef, watch, type Ref } from "vue";

const props = defineProps<{
  target: HTMLElement | null;
}>();

const targetRef = toRef(props, "target") as Ref<HTMLElement | null>;
const trackRef = ref<HTMLElement | null>(null);
const thumbRef = ref<HTMLElement | null>(null);
const canScroll = ref(false);
const scrollbarVisible = ref(false);
const dragging = ref(false);
const thumbHeight = ref(24);
const thumbTop = ref(0);

let resizeObserver: ResizeObserver | null = null;
let dragStartY = 0;
let dragStartScrollTop = 0;
let activePointerId: number | null = null;
let observedScroller: HTMLElement | null = null;
let pendingThumbFrame = 0;

const thumbStyle = computed(() => ({
  height: `${thumbHeight.value}px`,
  transform: `translateY(${thumbTop.value}px)`,
}));

function setDocumentDragging(active: boolean) {
  document.body.classList.toggle("floating-scrollbar-dragging", active);
}

function updateThumbNow() {
  const scroller = targetRef.value;
  if (!scroller) return;

  const { clientHeight, scrollHeight, scrollTop } = scroller;
  const scrollable = scrollHeight > clientHeight + 1;
  if (canScroll.value !== scrollable) {
    canScroll.value = scrollable;
  }
  if (!scrollable) {
    if (thumbTop.value !== 0) {
      thumbTop.value = 0;
    }
    return;
  }

  const trackHeight = Math.max(clientHeight - 8, 0);
  const height = Math.max(24, Math.round((clientHeight / scrollHeight) * trackHeight));
  const maxTop = Math.max(trackHeight - height, 0);
  const top = maxTop === 0
    ? 0
    : Math.round((scrollTop / (scrollHeight - clientHeight)) * maxTop);
  if (thumbHeight.value !== height) {
    thumbHeight.value = height;
  }
  if (thumbTop.value !== top) {
    thumbTop.value = top;
  }
}

function updateThumb() {
  if (pendingThumbFrame) return;
  pendingThumbFrame = requestAnimationFrame(() => {
    pendingThumbFrame = 0;
    updateThumbNow();
  });
}

function reveal() {
  updateThumbNow();
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
    resizeObserver = new ResizeObserver(updateThumb);
    resizeObserver.observe(scroller);
  }
  void nextTick(updateThumb);
}

function scrollByThumbDelta(deltaY: number) {
  const scroller = targetRef.value;
  if (!scroller) return;
  const maxScrollTop = Math.max(scroller.scrollHeight - scroller.clientHeight, 0);
  const maxThumbTop = Math.max(scroller.clientHeight - 8 - thumbHeight.value, 0);
  if (maxScrollTop <= 0 || maxThumbTop <= 0) return;
  scroller.scrollTop = dragStartScrollTop + (deltaY / maxThumbTop) * maxScrollTop;
}

function onDocumentPointerMove(event: PointerEvent) {
  if (!dragging.value || activePointerId !== event.pointerId) return;
  event.preventDefault();
  scrollByThumbDelta(event.clientY - dragStartY);
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
  dragStartY = event.clientY;
  dragStartScrollTop = scroller.scrollTop;
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
  const nextThumbTop = Math.min(
    Math.max(event.clientY - trackRect.top - thumbHeight.value / 2, 0),
    Math.max(trackRect.height - thumbHeight.value, 0),
  );
  const maxThumbTop = Math.max(trackRect.height - thumbHeight.value, 0);
  const maxScrollTop = Math.max(scroller.scrollHeight - scroller.clientHeight, 0);
  scroller.scrollTop = maxThumbTop === 0 ? 0 : (nextThumbTop / maxThumbTop) * maxScrollTop;
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

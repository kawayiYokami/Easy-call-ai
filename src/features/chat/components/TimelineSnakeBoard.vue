<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import TimelinePreviewMarkdown from "./TimelinePreviewMarkdown.vue";

type TimelineAnchor = {
  id: string;
  userText: string;
  assistantTail: string;
  index: number;
};

const props = withDefaults(defineProps<{
  anchors: TimelineAnchor[];
  activeIndex: number | null;
  hoveredIndex: number | null;
  anchorEl?: HTMLElement | null;
  visible?: boolean;
}>(), {
  visible: true,
});

const emit = defineEmits<{
  (e: "hover", index: number | null): void;
  (e: "jump", index: number): void;
  (e: "enter-zone"): void;
  (e: "leave-zone"): void;
}>();

const SAFE = 12;
const MAX_GAP = 31;
const MIN_GAP = 21;
const PADDING = 21;
const DOT_HIT = 23;
const DOT_SMALL = 10;
const DOT_FOCUSED = 21;
const PREVIEW_MAX_W = 380;
const PREVIEW_PAD = 10;
const PREVIEW_GAP = 4;
const PREVIEW_H = 152;

const hostRef = ref<HTMLElement | null>(null);
const viewport = ref({
  w: typeof window !== "undefined" ? window.innerWidth : 1920,
  h: typeof window !== "undefined" ? window.innerHeight : 1080,
});
const anchorRect = ref<DOMRect | null>(null);
let anchorRo: ResizeObserver | null = null;
let viewportTimer: number | null = null;

function updateViewport() {
  if (typeof window === "undefined") return;
  viewport.value = { w: window.innerWidth, h: window.innerHeight };
}

function updateAnchorRect() {
  const el = props.anchorEl;
  if (!el) {
    anchorRect.value = null;
    return;
  }
  anchorRect.value = el.getBoundingClientRect();
}

function measureAll() {
  updateViewport();
  updateAnchorRect();
}

function scheduleMeasure() {
  if (viewportTimer != null) return;
  viewportTimer = window.setTimeout(() => {
    viewportTimer = null;
    measureAll();
  }, 16);
}

function onWindowResize() {
  scheduleMeasure();
}

function onWindowScroll() {
  scheduleMeasure();
}

watch(
  () => props.anchorEl,
  (el) => {
    if (anchorRo) {
      anchorRo.disconnect();
      anchorRo = null;
    }
    if (el && typeof ResizeObserver !== "undefined") {
      anchorRo = new ResizeObserver(() => scheduleMeasure());
      anchorRo.observe(el);
      const parent = el.parentElement;
      if (parent) anchorRo.observe(parent);
    }
    scheduleMeasure();
  },
  { immediate: true },
);

onMounted(() => {
  measureAll();
  window.addEventListener("resize", onWindowResize);
  window.addEventListener("scroll", onWindowScroll, true);
  if (typeof window !== "undefined" && (window as any).visualViewport) {
    (window as any).visualViewport.addEventListener("resize", onWindowResize);
    (window as any).visualViewport.addEventListener("scroll", onWindowScroll);
  }
});

onBeforeUnmount(() => {
  window.removeEventListener("resize", onWindowResize);
  window.removeEventListener("scroll", onWindowScroll, true);
  if (typeof window !== "undefined" && (window as any).visualViewport) {
    (window as any).visualViewport.removeEventListener("resize", onWindowResize);
    (window as any).visualViewport.removeEventListener("scroll", onWindowScroll);
  }
  if (anchorRo) {
    anchorRo.disconnect();
    anchorRo = null;
  }
  if (viewportTimer != null) {
    clearTimeout(viewportTimer);
    viewportTimer = null;
  }
});

const layout = computed(() => {
  const N = props.anchors.length;
  const vw = viewport.value.w;
  const vh = viewport.value.h;
  const ar = anchorRect.value;
  const availW = vw - SAFE * 2;
  const availH = ar ? Math.max(32, ar.bottom - SAFE) : vh - SAFE * 2;
  if (N === 0 || availW < MIN_GAP || availH < MIN_GAP) return null;
  let gap = MAX_GAP;
  let cols = 1;
  let rows = 1;
  let fits = false;
  for (; gap >= MIN_GAP; gap -= 2) {
    const maxCols = Math.max(1, Math.floor(availW / gap));
    const maxRows = Math.max(1, Math.floor(availH / gap));
    if (maxCols * maxRows >= N) {
      cols = Math.min(maxCols, N);
      rows = Math.ceil(N / cols);
      fits = true;
      break;
    }
  }
  if (!fits) {
    gap = MIN_GAP;
    cols = Math.max(1, Math.floor(availW / MIN_GAP));
    rows = Math.max(1, Math.floor(availH / MIN_GAP));
  }
  const cardW = Math.max(Math.min((cols - 1) * gap + PADDING * 2, availW), 32);
  const cardH = Math.max(Math.min((rows - 1) * gap + PADDING * 2, availH), 32);
  const capacity = cols * rows;
  const posByIndex = new Map<number, { x: number; y: number }>();
  const ordered: Array<{ x: number; y: number }> = [];
  for (let k = 0; k < N && k < capacity; k++) {
    const anchor = props.anchors[N - 1 - k];
    if (!anchor) continue;
    const rowFromBottom = Math.floor(k / cols);
    const colMod = k % cols;
    const col = rowFromBottom % 2 === 0 ? cols - 1 - colMod : colMod;
    const row = rows - 1 - rowFromBottom;
    const x = PADDING + col * gap;
    const y = PADDING + row * gap;
    posByIndex.set(anchor.index, { x, y });
    ordered.push({ x, y });
  }
  return { gap, cols, rows, cardW, cardH, posByIndex, ordered };
});

const boardFixedStyle = computed(() => {
  const vw = viewport.value.w;
  const vh = viewport.value.h;
  if (!layout.value) {
    const ar = anchorRect.value;
    if (ar) {
      let right = vw - ar.right;
      let bottom = vh - ar.bottom;
      right = Math.max(SAFE, Math.min(right, vw - 32 - SAFE));
      bottom = Math.max(SAFE, Math.min(bottom, vh - 32 - SAFE));
      return {
        position: "fixed",
        right: `${Math.round(right)}px`,
        bottom: `${Math.round(bottom)}px`,
        width: "32px",
        height: "32px",
      } as Record<string, string>;
    }
    return {
      position: "fixed",
      right: `${SAFE}px`,
      bottom: `${SAFE + 40}px`,
      width: "32px",
      height: "32px",
    } as Record<string, string>;
  }
  const cardW = layout.value.cardW;
  const cardH = layout.value.cardH;
  const ar = anchorRect.value;
  if (!ar) {
    return {
      position: "fixed",
      right: `${SAFE}px`,
      bottom: `${SAFE + 40}px`,
      width: `${cardW}px`,
      height: `${cardH}px`,
    } as Record<string, string>;
  }
  // 蛇板底边对齐 anchor 底边：从按钮原位展开，不悬浮
  let right = vw - ar.right;
  let bottom = vh - ar.bottom;
  const maxRight = vw - cardW - SAFE;
  const maxBottom = vh - cardH - SAFE;
  right = Math.min(Math.max(right, SAFE), Math.max(SAFE, maxRight));
  bottom = Math.min(Math.max(bottom, SAFE), Math.max(SAFE, maxBottom));
  return {
    position: "fixed",
    right: `${Math.round(right)}px`,
    bottom: `${Math.round(bottom)}px`,
    width: `${cardW}px`,
    height: `${cardH}px`,
  } as Record<string, string>;
});

const boardViewportPos = computed(() => {
  if (!layout.value) {
    const ar = anchorRect.value;
    const vw = viewport.value.w;
    const vh = viewport.value.h;
    if (ar) {
      let right = vw - ar.right;
      let bottom = vh - ar.bottom;
      right = Math.max(SAFE, Math.min(right, vw - 32 - SAFE));
      bottom = Math.max(SAFE, Math.min(bottom, vh - 32 - SAFE));
      return { left: vw - right - 32, top: vh - bottom - 32 };
    }
    return { left: vw - SAFE - 32, top: vh - SAFE - 40 - 32 };
  }
  const s = boardFixedStyle.value;
  const right = parseFloat(String(s.right).replace("px", "")) || 0;
  const bottom = parseFloat(String(s.bottom).replace("px", "")) || 0;
  const vw = viewport.value.w;
  const vh = viewport.value.h;
  return {
    left: vw - right - layout.value.cardW,
    top: vh - bottom - layout.value.cardH,
  };
});

const polylinePoints = computed(() => {
  if (!layout.value) return "";
  return layout.value.ordered.map((p) => `${p.x},${p.y}`).join(" ");
});

const focusedIndex = computed(() => props.hoveredIndex ?? props.activeIndex);

function isFocused(index: number) {
  return focusedIndex.value === index;
}

function isActive(index: number) {
  return props.activeIndex === index;
}

const TRI_OUTER = 18;

function dotStyle(anchor: TimelineAnchor): Record<string, string> {
  const p = layout.value?.posByIndex.get(anchor.index);
  if (!p) return { display: "none" };
  return {
    left: `${p.x}px`,
    top: `${p.y}px`,
    width: `${DOT_HIT}px`,
    height: `${DOT_HIT}px`,
    transform: "translate(-50%, -50%)",
  };
}

const mouseLocal = ref({ x: 0, y: 0 });

function handleBoardMouseMove(event: MouseEvent) {
  const r = hostRef.value?.getBoundingClientRect();
  if (!r) return;
  mouseLocal.value = { x: event.clientX - r.left, y: event.clientY - r.top };
}

function toLocal(clientX: number, clientY: number) {
  const r = hostRef.value?.getBoundingClientRect();
  if (!r) return null;
  return { x: clientX - r.left, y: clientY - r.top };
}

function findNearestAnchor(clientX: number, clientY: number): TimelineAnchor | null {
  const l = layout.value;
  if (!l || props.anchors.length === 0) return null;
  const p = toLocal(clientX, clientY);
  if (!p) return null;
  let best: TimelineAnchor | null = null;
  let bestD = Infinity;
  for (const a of props.anchors) {
    const pos = l.posByIndex.get(a.index);
    if (!pos) continue;
    const dx = pos.x - p.x;
    const dy = pos.y - p.y;
    const d = dx * dx + dy * dy;
    if (d < bestD) { bestD = d; best = a; }
  }
  return best;
}

let scrubPointerId: number | null = null;
let scrubStartX = 0;
let scrubStartY = 0;
let lastScrubJumpAt = 0;
let lastScrubJumpIndex: number | null = null;

function handleBoardPointerDown(event: PointerEvent) {
  if (event.button !== 0) return;
  const nearest = findNearestAnchor(event.clientX, event.clientY);
  if (!nearest) return;
  scrubPointerId = event.pointerId;
  scrubStartX = event.clientX;
  scrubStartY = event.clientY;
  const local = toLocal(event.clientX, event.clientY);
  if (local) mouseLocal.value = local;
  emit("hover", nearest.index);
  try { (hostRef.value as HTMLElement | null)?.setPointerCapture(event.pointerId); } catch {}
  if (event.pointerType === "touch") event.preventDefault();
}

function handleBoardPointerMove(event: PointerEvent) {
  const local = toLocal(event.clientX, event.clientY);
  if (local) mouseLocal.value = local;
  if (scrubPointerId == null || scrubPointerId !== event.pointerId) return;
  const dx = event.clientX - scrubStartX;
  const dy = event.clientY - scrubStartY;
  if (dx * dx + dy * dy > 16) {}
  const nearest = findNearestAnchor(event.clientX, event.clientY);
  if (nearest) emit("hover", nearest.index);
}

function handleBoardPointerUp(event: PointerEvent) {
  if (scrubPointerId == null || scrubPointerId !== event.pointerId) return;
  const local = toLocal(event.clientX, event.clientY);
  if (local) mouseLocal.value = local;
  const nearest = findNearestAnchor(event.clientX, event.clientY);
  const jumpIndex = nearest?.index ?? props.hoveredIndex ?? props.activeIndex;
  if (jumpIndex != null) {
    lastScrubJumpAt = Date.now();
    lastScrubJumpIndex = jumpIndex;
    emit("jump", jumpIndex);
  }
  try { (hostRef.value as HTMLElement | null)?.releasePointerCapture(event.pointerId); } catch {}
  scrubPointerId = null;
}

function handleDotClick(anchor: TimelineAnchor) {
  if (lastScrubJumpIndex === anchor.index && Date.now() - lastScrubJumpAt < 500) return;
  emit("jump", anchor.index);
}

const previewAnchor = computed(() => {
  const idx = props.hoveredIndex;
  if (idx == null) return null;
  return props.anchors.find((a) => a.index === idx) ?? null;
});

const tooltipW = computed(() => {
  const cardW = layout.value?.cardW ?? PREVIEW_MAX_W;
  return Math.min(PREVIEW_MAX_W, Math.max(220, cardW - PREVIEW_PAD * 2 - 8));
});

const tooltipBelow = computed(() => {
  const y = mouseLocal.value.y;
  const vh = viewport.value.h;
  const boardTop = boardViewportPos.value.top;
  const cursorY = boardTop + y;
  const aboveTop = cursorY - 14 - PREVIEW_H;
  const belowBottom = cursorY + 18 + PREVIEW_H;
  const aboveOkViewport = aboveTop >= SAFE;
  const belowOkViewport = belowBottom <= vh - SAFE;
  if (aboveOkViewport) return false;
  if (belowOkViewport) return true;
  const spaceAbove = cursorY - SAFE;
  const spaceBelow = vh - SAFE - cursorY;
  return spaceBelow > spaceAbove;
});

const tooltipStyle = computed(() => {
  const w = tooltipW.value;
  const vw = viewport.value.w;
  const vh = viewport.value.h;
  const boardPos = boardViewportPos.value;
  const cardW = layout.value?.cardW ?? w;
  const half = w / 2 + 6;
  const clampedInsideX = Math.min(Math.max(mouseLocal.value.x, half), Math.max(half, cardW - half));
  const desiredViewportX = boardPos.left + clampedInsideX;
  const clampedViewportX = Math.min(Math.max(desiredViewportX, SAFE + w / 2), vw - SAFE - w / 2);
  const cursorViewportY = boardPos.top + mouseLocal.value.y;
  let top: number;
  if (tooltipBelow.value) {
    top = cursorViewportY + 18;
    if (top + PREVIEW_H > vh - SAFE) {
      top = Math.max(SAFE, vh - SAFE - PREVIEW_H);
    }
  } else {
    top = cursorViewportY - 14 - PREVIEW_H;
    if (top < SAFE) top = SAFE;
    if (top + PREVIEW_H > vh - SAFE) {
      top = Math.max(SAFE, vh - SAFE - PREVIEW_H);
    }
  }
  return {
    position: "fixed",
    left: `${Math.round(clampedViewportX)}px`,
    top: `${Math.round(top)}px`,
    width: `${w}px`,
    transform: "translateX(-50%)",
  } as Record<string, string>;
});
</script>

<template>
  <Teleport to="body">
    <Transition name="ecall-snake-board">
      <div
        v-if="visible && layout"
        ref="hostRef"
        class="ecall-snake-board-card pointer-events-auto fixed z-[100] rounded-2xl bg-base-100/70 shadow backdrop-blur-md select-none touch-none"
        :style="boardFixedStyle"
        @mousemove="handleBoardMouseMove"
        @pointerdown="handleBoardPointerDown"
        @pointermove="handleBoardPointerMove"
        @pointerup="handleBoardPointerUp"
        @pointercancel="handleBoardPointerUp"
        @mouseenter="emit('enter-zone')"
        @mouseleave="emit('leave-zone'); emit('hover', null)"
      >
        <div class="ecall-snake-board-content relative h-full w-full overflow-hidden rounded-2xl">
          <svg
            v-if="polylinePoints"
            class="pointer-events-none absolute inset-0 h-full w-full"
            :viewBox="`0 0 ${layout.cardW} ${layout.cardH}`"
          >
            <polyline
              :points="polylinePoints"
              fill="none"
              stroke="var(--color-base-300)"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </svg>
          <button
            v-for="anchor in anchors"
            :key="anchor.id"
            type="button"
            class="absolute flex items-center justify-center rounded-full"
            :style="dotStyle(anchor)"
            :aria-label="anchor.userText.slice(0, 30)"
            @mouseenter="emit('hover', anchor.index)"
            @focus="emit('hover', anchor.index)"
            @click.stop="handleDotClick(anchor)"
          >
            <span
              v-if="isActive(anchor.index)"
              class="pointer-events-none flex items-center justify-center transition-transform duration-150"
              :style="isFocused(anchor.index) ? 'transform: scale(1.15)' : ''"
              :aria-hidden="true"
            >
              <svg :width="TRI_OUTER" :height="TRI_OUTER" viewBox="0 0 20 18" class="overflow-visible drop-shadow-sm" shape-rendering="geometricPrecision">
                <path d="M10 2 L18.5 16 L1.5 16 Z" fill="none" stroke="var(--color-primary)" stroke-width="1.9" stroke-linejoin="round" stroke-linecap="round" />
                <path d="M10 7.2 L14.6 14.2 L5.4 14.2 Z" fill="var(--color-base-100)" stroke="none" />
              </svg>
            </span>
            <span
              v-else
              class="rounded-full transition-[width,height,background-color] duration-150"
              :class="isFocused(anchor.index) ? 'bg-primary' : 'bg-base-content/55'"
              :style="isFocused(anchor.index)
                ? `width:${DOT_FOCUSED}px;height:${DOT_FOCUSED}px`
                : `width:${DOT_SMALL}px;height:${DOT_SMALL}px`"
            />
          </button>
        </div>
      </div>
    </Transition>
    <Transition name="ecall-timeline-preview">
      <div
        v-if="visible && previewAnchor"
        class="pointer-events-none fixed z-[101]"
        :style="tooltipStyle"
      >
        <div v-if="tooltipBelow" class="mx-auto h-2 w-2 -translate-y-[1px] rotate-45 border-t border-l border-base-200 bg-base-100" />
        <div class="overflow-hidden rounded-xl border border-base-200 bg-base-100 shadow-lg" :style="{ padding: `${PREVIEW_PAD}px` }">
          <div class="flex min-w-0 flex-col overflow-hidden" :style="{ maxHeight: `calc(5 * 1.25rem + ${PREVIEW_GAP}px)` }">
            <span class="block shrink-0 truncate font-bold leading-5 text-base-content" style="height: 1.25rem; line-height: 1.25rem">
              <TimelinePreviewMarkdown :text="previewAnchor.userText" :clamp="80" />
            </span>
            <span
              v-if="(previewAnchor.assistantTail || '').trim()"
              class="block overflow-hidden leading-5 text-base-content/60"
              :style="{ marginTop: `${PREVIEW_GAP}px`, display: '-webkit-box', WebkitLineClamp: 4, WebkitBoxOrient: 'vertical', maxHeight: 'calc(4 * 1.25rem)' }"
            >
              <TimelinePreviewMarkdown :text="previewAnchor.assistantTail" :clamp="320" />
            </span>
          </div>
        </div>
        <div v-if="!tooltipBelow" class="mx-auto h-2 w-2 -translate-y-[1px] rotate-45 border-b border-r border-base-200 bg-base-100" />
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.ecall-snake-board-enter-active {
  transition:
    width 280ms cubic-bezier(0.22, 1, 0.36, 1),
    height 280ms cubic-bezier(0.22, 1, 0.36, 1),
    right 280ms cubic-bezier(0.22, 1, 0.36, 1),
    bottom 280ms cubic-bezier(0.22, 1, 0.36, 1),
    border-radius 280ms cubic-bezier(0.22, 1, 0.36, 1),
    background-color 280ms ease;
}
.ecall-snake-board-leave-active {
  transition:
    width 200ms ease,
    height 200ms ease,
    right 200ms ease,
    bottom 200ms ease,
    border-radius 200ms ease,
    background-color 200ms ease,
    opacity 160ms ease;
}
.ecall-snake-board-enter-from,
.ecall-snake-board-leave-to {
  width: 2rem !important;
  height: 2rem !important;
  border-radius: 9999px !important;
  background-color: var(--color-neutral);
  opacity: 1;
}
.ecall-snake-board-content {
  transition: opacity 220ms ease 60ms;
}
.ecall-snake-board-enter-from .ecall-snake-board-content,
.ecall-snake-board-leave-to .ecall-snake-board-content {
  opacity: 0;
}
.ecall-timeline-preview-enter-active,
.ecall-timeline-preview-leave-active {
  transition: opacity 160ms ease;
}
.ecall-timeline-preview-enter-from,
.ecall-timeline-preview-leave-to {
  opacity: 0;
}
</style>

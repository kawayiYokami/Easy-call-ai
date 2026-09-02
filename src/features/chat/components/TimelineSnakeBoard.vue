<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import TimelinePreviewMarkdown from "./TimelinePreviewMarkdown.vue";

type TimelineAnchor = {
  id: string;
  userText: string;
  assistantTail: string;
  index: number;
};

const props = defineProps<{
  anchors: TimelineAnchor[];
  activeIndex: number | null;
  hoveredIndex: number | null;
}>();

const emit = defineEmits<{
  (e: "hover", index: number | null): void;
  (e: "jump", index: number): void;
  (e: "enter-zone"): void;
  (e: "leave-zone"): void;
}>();

// 卡片挂在右下角按钮容器内：容器即定位锚（随 jumpToBottomStyle 走），
// 卡片底边压在按钮底边上原位向上长
const SAFE = 12;
// 间距（原 56/32 的 52%）：24/16 放大 30%
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
const avail = ref({ w: 0, h: 0 });
let regionRo: ResizeObserver | null = null;

function measureAvail() {
  const host = hostRef.value;
  const wrap = host?.parentElement as HTMLElement | null;
  const region = wrap?.offsetParent as HTMLElement | null;
  if (!wrap || !region) return;
  const rr = region.getBoundingClientRect();
  const wr = wrap.getBoundingClientRect();
  avail.value = {
    w: Math.round(wr.right - rr.left - SAFE),
    h: Math.round(wr.bottom - rr.top - SAFE),
  };
}

onMounted(() => {
  measureAvail();
  const wrap = hostRef.value?.parentElement as HTMLElement | null;
  const region = wrap?.offsetParent as HTMLElement | null;
  if (region && typeof ResizeObserver !== "undefined") {
    regionRo = new ResizeObserver(() => measureAvail());
    regionRo.observe(region);
  }
});

onBeforeUnmount(() => {
  if (regionRo) { regionRo.disconnect(); regionRo = null; }
});

// 按消息数与窗口算行列，行列定卡片尺寸：gap 从 12 压到 8 取能放下 N 点的最大间距
const layout = computed(() => {
  const N = props.anchors.length;
  const availW = avail.value.w;
  const availH = avail.value.h;
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
    // k=0 最新消息，右下角起按时间倒序蛇形来回拐弯；末行不满一行时：单数行（1/3…从下往上）靠右、双数行靠左，空位留对侧
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

const boardStyle = computed(() => {
  if (!layout.value) {
    return { right: "0px", bottom: "0px", width: "32px", height: "32px" } as Record<string, string>;
  }
  return {
    right: "0px",
    bottom: "0px",
    width: `${layout.value.cardW}px`,
    height: `${layout.value.cardH}px`,
  } as Record<string, string>;
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

// hint 跟随鼠标/手指：坐标相对卡片，上下自动翻转，水平钳制在卡片内
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
let scrubHasMoved = false;
let scrubStartX = 0;
let scrubStartY = 0;
let lastScrubJumpAt = 0;
let lastScrubJumpIndex: number | null = null;

function handleBoardPointerDown(event: PointerEvent) {
  if (event.button !== 0) return;
  const nearest = findNearestAnchor(event.clientX, event.clientY);
  if (!nearest) return;
  scrubPointerId = event.pointerId;
  scrubHasMoved = false;
  scrubStartX = event.clientX;
  scrubStartY = event.clientY;
  const local = toLocal(event.clientX, event.clientY);
  if (local) mouseLocal.value = local;
  emit("hover", nearest.index);
  try { (hostRef.value as HTMLElement | null)?.setPointerCapture(event.pointerId); } catch {}
  // 阻止触摸滚动
  if (event.pointerType === "touch") event.preventDefault();
}

function handleBoardPointerMove(event: PointerEvent) {
  const local = toLocal(event.clientX, event.clientY);
  if (local) mouseLocal.value = local;
  if (scrubPointerId == null || scrubPointerId !== event.pointerId) return;
  const dx = event.clientX - scrubStartX;
  const dy = event.clientY - scrubStartY;
  if (dx * dx + dy * dy > 16) scrubHasMoved = true;
  // 按住滑动预览：实时吸附到最近点
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
  // 刚通过拖动松手跳转过同一索引，避免双触发
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
  // 卡片内边距 10*2，留 8px 安全边
  return Math.min(PREVIEW_MAX_W, Math.max(220, cardW - PREVIEW_PAD * 2 - 8));
});

const tooltipBelow = computed(() => {
  const y = mouseLocal.value.y;
  const cardH = layout.value?.cardH ?? 0;
  const aboveOk = y - 14 - PREVIEW_H >= 8;
  const belowOk = y + 18 + PREVIEW_H <= cardH - 8;
  if (aboveOk) return false;
  return belowOk;
});

const tooltipStyle = computed(() => {
  const w = tooltipW.value;
  const half = w / 2 + 6;
  const cardW = layout.value?.cardW ?? 0;
  const x = Math.min(Math.max(mouseLocal.value.x, half), Math.max(half, cardW - half));
  if (tooltipBelow.value) {
    return { left: `${x}px`, top: `${mouseLocal.value.y + 18}px`, width: `${w}px`, transform: "translate(-50%, 0)" };
  }
  return { left: `${x}px`, top: `${mouseLocal.value.y - 14}px`, width: `${w}px`, transform: "translate(-50%, -100%)" };
});
</script>

<template>
  <div
    ref="hostRef"
    class="ecall-snake-board-card pointer-events-auto absolute z-30 rounded-2xl bg-base-100/70 shadow backdrop-blur-md select-none touch-none"
    :style="boardStyle"
    @mousemove="handleBoardMouseMove"
    @pointerdown="handleBoardPointerDown"
    @pointermove="handleBoardPointerMove"
    @pointerup="handleBoardPointerUp"
    @pointercancel="handleBoardPointerUp"
    @mouseenter="emit('enter-zone')"
    @mouseleave="emit('leave-zone'); emit('hover', null)"
  >
    <!-- 内容层：裁剪圆角随卡片长大，tooltip 不在此层内避免被边缘吞掉 -->
    <div class="ecall-snake-board-content relative h-full w-full overflow-hidden rounded-2xl">
      <!-- 蛇形连线 -->
      <svg
        v-if="layout && polylinePoints"
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

      <!-- git 时间线风格：当前位置三角挖孔（空心三角），其余实色圆点；悬停放大 -->
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
        <!-- 当前位置：三角挖孔（描边三角，中间透明），悬停在当前位则略放大 -->
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

    <!-- 悬停 hint：跟鼠标上方弹出，5 行预览（用户 1 行 + 助理 4 行） -->
    <Transition name="ecall-timeline-preview">
      <div
        v-if="previewAnchor"
        class="pointer-events-none absolute z-20"
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
  </div>
</template>

<style scoped>
/* 灵动岛：卡片从按钮原位（32px 圆）长开 */
.ecall-snake-board-enter-active {
  transition:
    width 280ms cubic-bezier(0.22, 1, 0.36, 1),
    height 280ms cubic-bezier(0.22, 1, 0.36, 1),
    border-radius 280ms cubic-bezier(0.22, 1, 0.36, 1),
    background-color 280ms ease;
}
.ecall-snake-board-leave-active {
  transition:
    width 200ms ease,
    height 200ms ease,
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

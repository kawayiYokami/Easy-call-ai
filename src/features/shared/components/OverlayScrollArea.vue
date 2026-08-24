<template>
  <div
    class="overlay-scroll-area relative"
    @mouseenter="revealScrollbars"
    @mouseleave="hideScrollbars"
  >
    <div
      ref="scrollerRef"
      class="overlay-scroll-area-scroller"
      :class="[props.scrollerClass, scrollerOverflowClass]"
      @wheel="handleWheel"
    >
      <slot />
    </div>
    <FloatingScrollbar
      v-if="showVerticalScrollbar"
      ref="verticalScrollbarRef"
      :target="scrollerRef"
      :variant="variant"
    />
    <FloatingScrollbar
      v-if="showHorizontalScrollbar"
      ref="horizontalScrollbarRef"
      :target="scrollerRef"
      :variant="variant"
      orientation="horizontal"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import FloatingScrollbar from "../../shell/components/FloatingScrollbar.vue";

const props = withDefaults(defineProps<{
  orientation?: "vertical" | "horizontal" | "both";
  variant?: "theme" | "code-dark";
  scrollerClass?: string;
}>(), {
  orientation: "vertical",
  variant: "theme",
  scrollerClass: "",
});

const scrollerRef = ref<HTMLElement | null>(null);
const verticalScrollbarRef = ref<InstanceType<typeof FloatingScrollbar> | null>(null);
const horizontalScrollbarRef = ref<InstanceType<typeof FloatingScrollbar> | null>(null);

const showVerticalScrollbar = computed(() => props.orientation === "vertical" || props.orientation === "both");
const showHorizontalScrollbar = computed(() => props.orientation === "horizontal" || props.orientation === "both");

const scrollerOverflowClass = computed(() => {
  if (props.orientation === "horizontal") return "overflow-x-auto overflow-y-hidden";
  if (props.orientation === "both") return "overflow-auto";
  return "overflow-x-hidden overflow-y-auto";
});

function revealScrollbars() {
  verticalScrollbarRef.value?.reveal();
  horizontalScrollbarRef.value?.reveal();
}

function hideScrollbars() {
  verticalScrollbarRef.value?.hide();
  horizontalScrollbarRef.value?.hide();
}

function updateScrollbars() {
  verticalScrollbarRef.value?.updateThumb();
  horizontalScrollbarRef.value?.updateThumb();
}

function wheelDeltaUnit(event: WheelEvent, scroller: HTMLElement) {
  if (event.deltaMode === WheelEvent.DOM_DELTA_LINE) return 16;
  if (event.deltaMode === WheelEvent.DOM_DELTA_PAGE) return scroller.clientWidth;
  return 1;
}

function handleWheel(event: WheelEvent) {
  if (props.orientation !== "horizontal") return;
  const scroller = scrollerRef.value;
  if (!scroller) return;

  const maxScrollLeft = Math.max(scroller.scrollWidth - scroller.clientWidth, 0);
  if (maxScrollLeft <= 0) return;

  const unit = wheelDeltaUnit(event, scroller);
  const deltaX = event.deltaX * unit;
  const deltaY = event.deltaY * unit;
  const delta = Math.abs(deltaX) > Math.abs(deltaY) ? deltaX : deltaY;
  if (!delta) return;

  const current = scroller.scrollLeft;
  const next = Math.min(maxScrollLeft, Math.max(0, current + delta));
  if (next === current) return;

  event.preventDefault();
  scroller.scrollLeft = next;
  revealScrollbars();
  updateScrollbars();
}

watch(
  () => props.orientation,
  () => {
    void nextTick(updateScrollbars);
  },
);

defineExpose({
  scrollerRef,
  updateThumb: updateScrollbars,
  reveal: revealScrollbars,
  hide: hideScrollbars,
});
</script>

<style scoped>
.overlay-scroll-area-scroller {
  scrollbar-width: none;
}

.overlay-scroll-area-scroller::-webkit-scrollbar {
  display: none;
}
</style>

<template>
  <div class="relative overflow-hidden" @mouseenter="scrollbarRef?.reveal()" @mouseleave="scrollbarRef?.hide()">
    <div ref="scrollerRef" class="conversation-list-scroll h-full overflow-y-auto">
      <slot />
    </div>
    <FloatingScrollbar ref="scrollbarRef" :target="scrollerRef" />
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import FloatingScrollbar from "../../shell/components/FloatingScrollbar.vue";

const scrollerRef = ref<HTMLElement | null>(null);
const scrollbarRef = ref<InstanceType<typeof FloatingScrollbar> | null>(null);

function scrollToElement(element: HTMLElement | null | undefined) {
  const scroller = scrollerRef.value;
  if (!scroller || !element) return;
  const scrollerRect = scroller.getBoundingClientRect();
  const elementRect = element.getBoundingClientRect();
  const targetTop = scroller.scrollTop + (elementRect.top - scrollerRect.top);
  scroller.scrollTo({ top: Math.max(0, targetTop - 4), behavior: "smooth" });
}

defineExpose({
  updateThumb: () => scrollbarRef.value?.updateThumb(),
  scrollToElement,
});
</script>

<style scoped>
.conversation-list-scroll {
  scrollbar-gutter: auto;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.conversation-list-scroll::-webkit-scrollbar {
  width: 0;
  height: 0;
}
</style>

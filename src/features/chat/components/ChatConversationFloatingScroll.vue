<template>
  <OverlayScrollArea ref="areaRef" class="h-full overflow-hidden" scroller-class="h-full">
    <slot />
  </OverlayScrollArea>
</template>

<script setup lang="ts">
import { ref } from "vue";
import OverlayScrollArea from "../../shared/components/OverlayScrollArea.vue";

const areaRef = ref<InstanceType<typeof OverlayScrollArea> | null>(null);

function scrollToElement(element: HTMLElement | null | undefined) {
  const scroller = areaRef.value?.scrollerRef;
  if (!scroller || !element) return;
  const scrollerRect = scroller.getBoundingClientRect();
  const elementRect = element.getBoundingClientRect();
  const targetTop = scroller.scrollTop + (elementRect.top - scrollerRect.top);
  scroller.scrollTo({ top: Math.max(0, targetTop - 4), behavior: "smooth" });
}

defineExpose({
  updateThumb: () => areaRef.value?.updateThumb(),
  scrollToElement,
});
</script>

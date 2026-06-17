<template>
  <section>
    <div
      role="button"
      tabindex="0"
      class="group/section sticky top-0 z-20 mx-1 flex h-9 items-center gap-2 rounded-lg bg-base-200/95 px-2 text-left text-xs font-semibold text-base-content backdrop-blur transition-colors hover:bg-base-300/70"
      :title="title"
      @click="toggle"
      @contextmenu.stop.prevent="collapseAll"
      @dblclick.stop.prevent="collapseAll"
      @keydown.enter.prevent="toggle"
      @keydown.space.prevent="toggle"
    >
      <ChevronRight
        class="h-4 w-4 shrink-0 transition-transform duration-200 ease-out"
        :class="modelValue ? '' : 'rotate-90'"
      />
      <span class="min-w-0 truncate">{{ title }}</span>
      <span class="shrink-0 tabular-nums text-base-content/45">{{ count }}</span>
      <slot name="actions" />
    </div>
    <Transition
      :css="false"
      @enter="animateEnter"
      @leave="animateLeave"
      @enter-cancelled="cleanupAnimation"
      @leave-cancelled="cleanupAnimation"
    >
      <div v-if="!modelValue" class="collapsible-group-shell">
        <slot />
      </div>
    </Transition>
  </section>
</template>

<script setup lang="ts">
import { ChevronRight } from "@lucide/vue";

const props = defineProps<{
  title: string;
  count: number;
  modelValue: boolean;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: boolean];
  "collapse-all": [];
  "after-enter": [];
  "after-leave": [];
}>();

function toggle() {
  emit("update:modelValue", !props.modelValue);
}

function collapseAll(event: MouseEvent) {
  const target = event.target;
  if (target instanceof HTMLElement && target.closest("button,a,input,textarea,select")) return;
  emit("collapse-all");
}

function cleanupAnimation(element: Element) {
  const el = element as HTMLElement;
  el.style.height = "";
  el.style.opacity = "";
  el.style.transform = "";
  el.style.overflow = "";
  el.style.willChange = "";
  el.style.transition = "";
}

function animateEnter(element: Element, done: () => void) {
  const sectionElement = element as HTMLElement;
  cleanupAnimation(sectionElement);
  sectionElement.style.height = "0px";
  sectionElement.style.opacity = "0";
  sectionElement.style.transform = "translateY(-6px)";
  sectionElement.style.overflow = "hidden";
  sectionElement.style.willChange = "height, opacity, transform";
  void sectionElement.offsetHeight;
  const onTransitionEnd = (event: TransitionEvent) => {
    if (event.target !== sectionElement || event.propertyName !== "height") return;
    sectionElement.removeEventListener("transitionend", onTransitionEnd);
    cleanupAnimation(sectionElement);
    emit("after-enter");
    done();
  };
  sectionElement.addEventListener("transitionend", onTransitionEnd);
  sectionElement.style.transition = [
    "height 180ms cubic-bezier(0.22, 1, 0.36, 1)",
    "opacity 140ms ease-out",
    "transform 180ms cubic-bezier(0.22, 1, 0.36, 1)",
  ].join(", ");
  requestAnimationFrame(() => {
    sectionElement.style.height = `${sectionElement.scrollHeight}px`;
    sectionElement.style.opacity = "1";
    sectionElement.style.transform = "translateY(0)";
  });
}

function animateLeave(element: Element, done: () => void) {
  const sectionElement = element as HTMLElement;
  cleanupAnimation(sectionElement);
  sectionElement.style.height = `${sectionElement.scrollHeight}px`;
  sectionElement.style.opacity = "1";
  sectionElement.style.transform = "translateY(0)";
  sectionElement.style.overflow = "hidden";
  sectionElement.style.willChange = "height, opacity, transform";
  void sectionElement.offsetHeight;
  const onTransitionEnd = (event: TransitionEvent) => {
    if (event.target !== sectionElement || event.propertyName !== "height") return;
    sectionElement.removeEventListener("transitionend", onTransitionEnd);
    cleanupAnimation(sectionElement);
    emit("after-leave");
    done();
  };
  sectionElement.addEventListener("transitionend", onTransitionEnd);
  sectionElement.style.transition = [
    "height 180ms cubic-bezier(0.22, 1, 0.36, 1)",
    "opacity 140ms ease-out",
    "transform 180ms cubic-bezier(0.22, 1, 0.36, 1)",
  ].join(", ");
  requestAnimationFrame(() => {
    sectionElement.style.height = "0px";
    sectionElement.style.opacity = "0";
    sectionElement.style.transform = "translateY(-6px)";
  });
}
</script>

<style scoped>
.collapsible-group-shell {
  transform-origin: top;
}
</style>

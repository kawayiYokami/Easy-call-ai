<template>
  <section>
    <div
      role="button"
      tabindex="0"
      :draggable="draggable"
      class="group/section relative sticky top-0 z-20 mx-1 flex h-9 select-none items-center gap-2 rounded-lg bg-base-200/95 px-2 text-left text-xs font-semibold text-base-content backdrop-blur transition-colors hover:bg-base-300/70"
      :title="title"
      @click="toggle"
      @contextmenu.stop.prevent="collapseAll"
      @dblclick.stop.prevent="collapseAll"
      @keydown.enter.prevent="toggle"
      @keydown.space.prevent="toggle"
      @dragstart="onDragStart"
      @dragover="onDragOver"
      @drop="onDrop"
      @dragend="onDragEnd"
    >
      <div
        v-if="dropIndicator === 'before'"
        class="pointer-events-none absolute left-2 right-2 top-0 h-[3px] -translate-y-1/2 rounded-full bg-neutral shadow-[0_0_0_1px_color-mix(in_oklab,var(--color-neutral)_28%,transparent)]"
        aria-hidden="true"
      ></div>
      <div
        v-if="dropIndicator === 'after'"
        class="pointer-events-none absolute left-2 right-2 bottom-0 translate-y-1/2 rounded-full bg-neutral h-[3px] shadow-[0_0_0_1px_color-mix(in_oklab,var(--color-neutral)_28%,transparent)]"
        aria-hidden="true"
      ></div>
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
  draggable?: boolean;
  dropIndicator?: "before" | "after" | null;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: boolean];
  "collapse-all": [];
  "after-enter": [];
  "after-leave": [];
  "dragstart": [event: DragEvent];
  "dragover": [event: DragEvent];
  "drop": [event: DragEvent];
  "dragend": [event: DragEvent];
}>();

function toggle() {
  emit("update:modelValue", !props.modelValue);
}

function collapseAll(event: MouseEvent) {
  const target = event.target;
  if (target instanceof HTMLElement && target.closest("button,a,input,textarea,select")) return;
  emit("collapse-all");
}

function onDragStart(event: DragEvent) {
  if (!props.draggable) {
    event.preventDefault();
    event.stopPropagation();
    return;
  }
  emit("dragstart", event);
}

function onDragOver(event: DragEvent) {
  if (!props.draggable) return;
  emit("dragover", event);
}

function onDrop(event: DragEvent) {
  if (!props.draggable) return;
  emit("drop", event);
}

function onDragEnd(event: DragEvent) {
  if (!props.draggable) return;
  emit("dragend", event);
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
  delete sectionElement.dataset.ecallCollapseFinished;
  sectionElement.style.height = "0px";
  sectionElement.style.opacity = "0";
  sectionElement.style.transform = "translateY(-6px)";
  sectionElement.style.overflow = "hidden";
  sectionElement.style.willChange = "height, opacity, transform";
  void sectionElement.offsetHeight;
  const onTransitionEnd = (event: TransitionEvent) => {
    if (event.target !== sectionElement || event.propertyName !== "height") return;
    finishAnimation(sectionElement, onTransitionEnd, "after-enter", done);
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
  // 兜底：transitionend 可能因高度无变化等场景不触发，超时后强制清理，避免 overflow:hidden 残留裁剪内容
  window.setTimeout(() => finishAnimation(sectionElement, onTransitionEnd, "after-enter", done), 400);
}

function animateLeave(element: Element, done: () => void) {
  const sectionElement = element as HTMLElement;
  cleanupAnimation(sectionElement);
  delete sectionElement.dataset.ecallCollapseFinished;
  sectionElement.style.height = `${sectionElement.scrollHeight}px`;
  sectionElement.style.opacity = "1";
  sectionElement.style.transform = "translateY(0)";
  sectionElement.style.overflow = "hidden";
  sectionElement.style.willChange = "height, opacity, transform";
  void sectionElement.offsetHeight;
  const onTransitionEnd = (event: TransitionEvent) => {
    if (event.target !== sectionElement || event.propertyName !== "height") return;
    finishAnimation(sectionElement, onTransitionEnd, "after-leave", done);
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
  // 兜底：同 enter，防止 transitionend 不触发时样式残留
  window.setTimeout(() => finishAnimation(sectionElement, onTransitionEnd, "after-leave", done), 400);
}

function finishAnimation(
  sectionElement: HTMLElement,
  onTransitionEnd: (event: TransitionEvent) => void,
  eventName: "after-enter" | "after-leave",
  done: () => void,
) {
  if (sectionElement.dataset.ecallCollapseFinished === "1") return;
  sectionElement.dataset.ecallCollapseFinished = "1";
  sectionElement.removeEventListener("transitionend", onTransitionEnd);
  cleanupAnimation(sectionElement);
  if (eventName === "after-enter") {
    emit("after-enter");
  } else {
    emit("after-leave");
  }
  done();
}
</script>

<style scoped>
.collapsible-group-shell {
  transform-origin: top;
}
</style>

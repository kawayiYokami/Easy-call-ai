<template>
  <div
    class="git-resize-handle shrink-0"
    :class="{ 'is-active': active }"
    role="separator"
    aria-orientation="horizontal"
    @pointerdown="onPointerDown"
  ></div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, ref } from "vue";

const emit = defineEmits<{
  (e: "resizeStart"): void;
  (e: "resize", dy: number): void;
  (e: "resizeEnd"): void;
}>();

// ==================== 拖拽调整高度（参照 FileReaderPanel 分界线实现） ====================
const active = ref(false);
let dragStartY = 0;
let previousBodyCursor = "";
let previousBodyUserSelect = "";

function onPointerDown(event: PointerEvent) {
  if (event.button !== 0) return;
  event.preventDefault();
  active.value = true;
  dragStartY = event.clientY;
  (event.currentTarget as HTMLElement | null)?.setPointerCapture?.(event.pointerId);
  previousBodyCursor = document.body.style.cursor;
  previousBodyUserSelect = document.body.style.userSelect;
  document.body.style.cursor = "row-resize";
  document.body.style.userSelect = "none";
  emit("resizeStart");
  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", stopDrag, { once: true });
  window.addEventListener("pointercancel", stopDrag, { once: true });
}

function onPointerMove(event: PointerEvent) {
  if (!active.value) return;
  emit("resize", event.clientY - dragStartY);
}

function stopDrag() {
  if (!active.value) return;
  active.value = false;
  document.body.style.cursor = previousBodyCursor;
  document.body.style.userSelect = previousBodyUserSelect;
  emit("resizeEnd");
  window.removeEventListener("pointermove", onPointerMove);
  window.removeEventListener("pointerup", stopDrag);
  window.removeEventListener("pointercancel", stopDrag);
}

onBeforeUnmount(() => {
  window.removeEventListener("pointermove", onPointerMove);
  window.removeEventListener("pointerup", stopDrag);
  window.removeEventListener("pointercancel", stopDrag);
});
</script>

<style scoped>
.git-resize-handle {
  position: relative;
  height: 5px;
  cursor: row-resize;
  border-top: 1px solid var(--color-base-300);
  background: var(--color-base-200);
}
.git-resize-handle::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  top: -1px;
  height: 2px;
  background: color-mix(in srgb, var(--color-primary) 70%, transparent);
  opacity: 0;
  transition: opacity 160ms ease;
  pointer-events: none;
}
.git-resize-handle:hover::after,
.git-resize-handle.is-active::after {
  opacity: 1;
}
</style>

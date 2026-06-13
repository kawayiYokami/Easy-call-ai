<template>
  <Teleport to="body">
    <ul
      v-if="open"
      ref="menuRef"
      tabindex="0"
      class="menu fixed z-[1200] w-40 rounded-box border border-base-300 bg-base-100 p-1 shadow-xl"
      :style="menuStyle"
      @click.stop
      @mousedown.stop
      @keydown.esc.prevent.stop="closeMenu"
    >
      <slot :close="closeMenu" />
    </ul>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref } from "vue";

defineProps<{
  title: string;
}>();

const menuRef = ref<HTMLElement | null>(null);
const open = ref(false);
const menuPosition = ref({ left: 0, top: 0 });

const menuStyle = computed(() => ({
  left: `${menuPosition.value.left}px`,
  top: `${menuPosition.value.top}px`,
}));

function updateMenuPosition(x?: number, y?: number) {
  const menuWidth = menuRef.value?.offsetWidth || 160;
  const menuHeight = menuRef.value?.offsetHeight || 168;
  const margin = 8;
  const viewportWidth = window.innerWidth || document.documentElement.clientWidth || 0;
  const viewportHeight = window.innerHeight || document.documentElement.clientHeight || 0;
  const maxLeft = Math.max(margin, viewportWidth - menuWidth - margin);
  const left = x !== undefined
    ? Math.min(Math.max(margin, x), maxLeft)
    : Math.max(margin, Math.floor((viewportWidth - menuWidth) / 2));
  const spaceBelow = y !== undefined ? viewportHeight - y : 0;
  const spaceAbove = y ?? 0;
  const openUpward = y !== undefined && spaceBelow < menuHeight + margin && spaceAbove > spaceBelow;
  const rawTop = openUpward ? (y ?? 0) - menuHeight - margin : (y ?? 0) + margin;
  const maxTop = Math.max(margin, viewportHeight - menuHeight - margin);
  const top = y !== undefined ? Math.min(Math.max(margin, rawTop), maxTop) : Math.max(margin, Math.floor((viewportHeight - menuHeight) / 2));
  menuPosition.value = { left, top };
}

function handleGlobalPointerDown(event: PointerEvent) {
  const target = event.target;
  if (!(target instanceof Node)) {
    closeMenu();
    return;
  }
  if (menuRef.value?.contains(target)) return;
  closeMenu();
}

const positionSnapshot = { x: 0, y: 0 };

function addGlobalListeners() {
  window.addEventListener("pointerdown", handleGlobalPointerDown, true);
  window.addEventListener("scroll", handleScrollOrResize, true);
  window.addEventListener("resize", handleScrollOrResize, true);
}

function handleScrollOrResize() {
  // Keep position from last snapshot; no button to recalculate from
  updateMenuPosition(positionSnapshot.x, positionSnapshot.y);
}

function removeGlobalListeners() {
  window.removeEventListener("pointerdown", handleGlobalPointerDown, true);
  window.removeEventListener("scroll", handleScrollOrResize, true);
  window.removeEventListener("resize", handleScrollOrResize, true);
}

async function openMenu(x?: number, y?: number) {
  if (open.value) return;
  positionSnapshot.x = x ?? positionSnapshot.x;
  positionSnapshot.y = y ?? positionSnapshot.y;
  updateMenuPosition(x, y);
  open.value = true;
  addGlobalListeners();
  await nextTick();
  updateMenuPosition(x, y);
  menuRef.value?.focus();
}

function closeMenu() {
  if (!open.value) return;
  open.value = false;
  removeGlobalListeners();
}

function toggleMenu(x?: number, y?: number) {
  if (open.value) {
    closeMenu();
    return;
  }
  void openMenu(x, y);
}

onBeforeUnmount(() => {
  removeGlobalListeners();
});

defineExpose({ openMenu, closeMenu, toggleMenu });</script>

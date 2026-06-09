<template>
  <div class="relative">
    <button
      ref="buttonRef"
      type="button"
      tabindex="0"
      class="btn btn-ghost btn-xs h-6 min-h-6 w-6 min-w-6 p-0 text-base-content/55 opacity-0 pointer-events-none transition-opacity group-hover:opacity-100 group-hover:pointer-events-auto group-focus-within:opacity-100 group-focus-within:pointer-events-auto hover:text-base-content"
      :title="title"
      @click.stop="toggleMenu"
      @keydown.enter.prevent.stop="toggleMenu"
      @keydown.space.prevent.stop="toggleMenu"
      @mousedown.stop
    >
      <Ellipsis class="h-3.5 w-3.5" />
    </button>
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
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref } from "vue";
import { Ellipsis } from "@lucide/vue";

defineProps<{
  title: string;
}>();

const buttonRef = ref<HTMLButtonElement | null>(null);
const menuRef = ref<HTMLElement | null>(null);
const open = ref(false);
const menuPosition = ref({ left: 0, top: 0 });

const menuStyle = computed(() => ({
  left: `${menuPosition.value.left}px`,
  top: `${menuPosition.value.top}px`,
}));

function updateMenuPosition() {
  const button = buttonRef.value;
  if (!button) return;
  const rect = button.getBoundingClientRect();
  const menuWidth = menuRef.value?.offsetWidth || 160;
  const menuHeight = menuRef.value?.offsetHeight || 168;
  const margin = 8;
  const viewportWidth = window.innerWidth || document.documentElement.clientWidth || 0;
  const viewportHeight = window.innerHeight || document.documentElement.clientHeight || 0;
  const maxLeft = Math.max(margin, viewportWidth - menuWidth - margin);
  const left = Math.min(Math.max(margin, rect.right - menuWidth), maxLeft);
  const spaceBelow = viewportHeight - rect.bottom;
  const spaceAbove = rect.top;
  const openUpward = spaceBelow < menuHeight + margin && spaceAbove > spaceBelow;
  const rawTop = openUpward ? rect.top - menuHeight - margin : rect.bottom + margin;
  const maxTop = Math.max(margin, viewportHeight - menuHeight - margin);
  const top = Math.min(Math.max(margin, rawTop), maxTop);
  menuPosition.value = { left, top };
}

function handleGlobalPointerDown(event: PointerEvent) {
  const target = event.target;
  if (!(target instanceof Node)) {
    closeMenu();
    return;
  }
  if (buttonRef.value?.contains(target) || menuRef.value?.contains(target)) return;
  closeMenu();
}

function addGlobalListeners() {
  window.addEventListener("pointerdown", handleGlobalPointerDown, true);
  window.addEventListener("scroll", updateMenuPosition, true);
  window.addEventListener("resize", updateMenuPosition, true);
}

function removeGlobalListeners() {
  window.removeEventListener("pointerdown", handleGlobalPointerDown, true);
  window.removeEventListener("scroll", updateMenuPosition, true);
  window.removeEventListener("resize", updateMenuPosition, true);
}

async function openMenu() {
  if (open.value) return;
  updateMenuPosition();
  open.value = true;
  addGlobalListeners();
  await nextTick();
  updateMenuPosition();
  menuRef.value?.focus();
}

function closeMenu() {
  if (!open.value) return;
  open.value = false;
  removeGlobalListeners();
}

function toggleMenu() {
  if (open.value) {
    closeMenu();
    return;
  }
  void openMenu();
}

onBeforeUnmount(() => {
  removeGlobalListeners();
});
</script>

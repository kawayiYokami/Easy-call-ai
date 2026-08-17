<template>
  <Transition name="config-status-toast">
    <div
      v-if="visible && text"
      class="pointer-events-none fixed inset-x-0 bottom-4 z-50 flex justify-center px-4"
    >
      <div
        class="alert pointer-events-auto w-fit max-w-full px-4 py-2 text-sm shadow-lg"
        :class="toneClass"
      >
        <span class="block min-w-0 whitespace-pre-wrap break-words leading-5">{{ text }}</span>
      </div>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import type { StatusTone } from "../../shell/composables/use-app-core";

const props = withDefaults(defineProps<{
  text: string;
  tone: StatusTone;
}>(), {
  text: "",
  tone: "default",
});

const visible = ref(false);
let hideTimer = 0;

const toneClass = computed(() => {
  if (props.tone === "error") return "alert-error alert-soft";
  if (props.tone === "success") return "alert-success alert-soft";
  return "bg-base-200 text-base-content";
});

watch(
  () => props.text,
  (next) => {
    if (!next) return;
    visible.value = true;
    if (hideTimer) window.clearTimeout(hideTimer);
    hideTimer = window.setTimeout(() => {
      visible.value = false;
      hideTimer = 0;
    }, 4000);
  },
);

defineExpose({ hide: () => { visible.value = false; } });
</script>

<style scoped>
.config-status-toast-enter-active,
.config-status-toast-leave-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}
.config-status-toast-enter-from,
.config-status-toast-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>
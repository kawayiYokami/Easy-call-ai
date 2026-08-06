<template>
  <div class="dropdown dropdown-bottom">
    <button
      type="button"
      tabindex="0"
      class="btn btn-sm min-w-0 shrink-0 gap-1.5 border-0 bg-base-100/60 shadow-none hover:bg-base-300"
      :title="t('chat.rightPanelSwitcherTitle')"
      @click.stop
    >
      <component :is="activeOption.icon" class="size-4 shrink-0 opacity-70" aria-hidden="true" />
      <span class="truncate">{{ activeOption.label }}</span>
      <ChevronDown class="size-3.5 shrink-0 opacity-60" aria-hidden="true" />
    </button>
    <ul
      tabindex="0"
      class="dropdown-content menu z-70 mt-1 w-40 rounded-box border border-base-300 bg-base-100 p-1 shadow-xl"
    >
      <li v-for="option in panelOptions" :key="option.value">
        <button type="button" class="gap-2" @click.stop="selectPanel(option.value)">
          <component :is="option.icon" class="size-4 shrink-0" aria-hidden="true" />
          <span class="min-w-0 flex-1 truncate text-left">{{ option.label }}</span>
          <Check v-if="option.value === modelValue" class="size-4 shrink-0" aria-hidden="true" />
        </button>
      </li>
    </ul>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { Activity, Check, ChevronDown, Files, MessageSquareMore } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import type { ChatRightPanelMode } from "../composables/chat-ui-layout-storage";

const props = withDefaults(defineProps<{
  modelValue: ChatRightPanelMode;
  sideChatEnabled?: boolean;
}>(), {
  sideChatEnabled: false,
});

const emit = defineEmits<{
  (e: "update:modelValue", value: ChatRightPanelMode): void;
}>();

const { t } = useI18n();

const panelOptions = computed(() => [
  { value: "reader" as const, label: t("chat.filePanelTab"), icon: Files },
  { value: "monitor" as const, label: t("chat.monitorPanelTab"), icon: Activity },
  ...(props.sideChatEnabled
    ? [{ value: "sideChat" as const, label: t("chat.sideChat.title"), icon: MessageSquareMore }]
    : []),
]);

const activeOption = computed(() =>
  panelOptions.value.find((option) => option.value === props.modelValue) || panelOptions.value[0],
);

function selectPanel(value: ChatRightPanelMode) {
  emit("update:modelValue", value);
  const activeElement = document.activeElement;
  if (activeElement instanceof HTMLElement) activeElement.blur();
}
</script>

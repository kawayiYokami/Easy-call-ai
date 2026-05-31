<template>
  <div class="relative flex h-full min-h-0 flex-col bg-base-100 text-base-content">
    <header class="flex h-10 shrink-0 items-center gap-1 border-b border-base-300 px-2">
      <button
        class="btn btn-ghost btn-sm h-8 min-h-8 w-8 shrink-0 px-0"
        title="会话列表"
        @click="view === 'list' ? $emit('showChat') : $emit('showList')"
      >
        <ChevronLeft class="h-4 w-4" />
      </button>

      <div class="flex min-w-0 flex-1 items-center justify-center gap-1 px-1">
        <button
          class="btn btn-ghost btn-sm h-8 min-h-8 w-8 shrink-0 px-0"
          title="新建会话"
          @click="$emit('newConversation')"
        >
          <SquarePen class="h-4 w-4" />
        </button>
        <span class="pointer-events-none min-w-0 truncate text-sm font-semibold text-base-content">{{ displayTitle }}</span>
        <button
          class="btn btn-ghost btn-sm btn-square h-8 min-h-8 w-8 shrink-0"
          :disabled="compacting || !activeConversationId"
          :title="`上下文用量 ${chatUsagePercent}%`"
          @click="$emit('compactConversation')"
        >
          <svg class="h-5 w-5 -rotate-90" viewBox="0 0 36 36">
            <circle cx="18" cy="18" r="14" fill="none" stroke="currentColor" stroke-width="4" class="opacity-20" />
            <circle
              cx="18" cy="18" r="14" fill="none" stroke="currentColor" stroke-width="4" stroke-linecap="round"
              :stroke-dasharray="usageRingCircumference" :stroke-dashoffset="usageRingOffset"
            />
          </svg>
        </button>
      </div>

      <button
        class="btn btn-ghost btn-sm h-8 min-h-8 w-8 shrink-0 px-0"
        title="审查"
        @click="$emit('toggleReviewPanel')"
      >
        <FileSearch class="h-4 w-4" />
      </button>
      <button
        class="btn btn-ghost btn-sm h-8 min-h-8 w-8 shrink-0 px-0"
        title="设置"
        @click="$emit('openSettings')"
      >
        <Settings class="h-4 w-4" />
      </button>
    </header>
    <div v-if="!connected" class="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 px-4 text-center">
      <div class="text-sm font-medium">{{ connecting ? "正在连接 PAI" : (errorText || "PAI 未运行") }}</div>
      <button class="btn btn-sm btn-primary" :disabled="connecting" @click="$emit('reconnect')">
        <RefreshCcw class="h-4 w-4" />
        重连
      </button>
    </div>
    <main v-else class="min-h-0 flex-1">
      <slot />
    </main>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { ChevronLeft, FileSearch, RefreshCcw, Settings, SquarePen } from "@lucide/vue";

const props = defineProps<{
  view: "list" | "chat";
  connected: boolean;
  connecting: boolean;
  errorText: string;
  activeTitle: string;
  activeConversationId: string;
  compacting: boolean;
  chatUsagePercent?: number;
}>();

const usageRingCircumference = 2 * Math.PI * 14;
const usageRingOffset = computed(() => {
  const percent = Math.min(100, Math.max(0, Number(props.chatUsagePercent || 0)));
  return usageRingCircumference * (1 - percent / 100);
});

const displayTitle = computed(() => {
  const raw = props.activeTitle || "PAI";
  return raw.length > 10 ? raw.slice(0, 10) + "…" : raw;
});

defineEmits<{
  showList: [];
  showChat: [];
  newConversation: [];
  openSettings: [];
  compactConversation: [];
  reconnect: [];
  toggleReviewPanel: [];
}>();
</script>

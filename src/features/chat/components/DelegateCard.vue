<template>
  <section class="last:mb-0">
    <div class="group/card flex w-full items-center gap-2 rounded-2xl px-2 py-1 transition-colors hover:bg-base-200">
      <img v-if="avatarUrl" :src="avatarUrl" class="h-10 w-10 shrink-0 rounded-full object-cover" />
      <div class="min-w-0 flex-1 overflow-hidden">
        <span class="block truncate text-xs font-normal text-base-content">{{ title }}</span>
        <DelegateProgressLine
          :running="running"
          :elapsed-ms="elapsedMs"
          :request-count="requestCount"
          :token-count="tokenCount"
          :last-tool-name="lastToolName"
          :text="text"
        />
      </div>
      <div class="flex shrink-0 items-center gap-1">
        <button
          v-if="running"
          type="button"
          class="btn btn-ghost btn-sm h-8 min-h-8 shrink-0 gap-1.5 px-2 font-normal hover:bg-warning hover:text-warning-content"
          title="打断委托"
          @click.stop="emit('abort')"
        >
          <X class="size-3.5" aria-hidden="true" />
          <span>打断</span>
        </button>
        <button
          v-if="showResult"
          type="button"
          class="btn btn-ghost btn-sm h-8 min-h-8 shrink-0 gap-1.5 px-2 font-normal"
          title="查看结果"
          @click.stop="emit('openDetail')"
        >
          <FileText class="size-3.5" aria-hidden="true" />
          <span>结果</span>
        </button>
      </div>
    </div>
  </section>
</template>

<script setup lang="ts">
import { FileText, X } from "@lucide/vue";
import DelegateProgressLine from "./DelegateProgressLine.vue";

defineProps<{
  title: string;
  avatarUrl?: string;
  running?: boolean;
  elapsedMs?: number;
  requestCount?: number;
  tokenCount?: number;
  lastToolName?: string;
  text?: string;
  showResult?: boolean;
}>();

const emit = defineEmits<{
  abort: [];
  openDetail: [];
}>();
</script>

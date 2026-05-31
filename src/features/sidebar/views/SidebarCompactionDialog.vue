<template>
  <dialog class="modal" :class="{ 'modal-open': open }">
    <div class="modal-box max-w-md">
      <h3 class="text-base font-semibold">整理上下文</h3>
      <div v-if="loading" class="mt-3 flex items-center gap-2 text-sm opacity-70">
        <span class="loading loading-spinner loading-sm"></span>
        <span>正在检查当前会话</span>
      </div>
      <template v-else>
        <div class="mt-3 rounded-box border border-base-300 bg-base-200/40 px-3 py-3 text-sm">
          <div class="font-medium">压缩当前会话</div>
          <div class="mt-1 opacity-80">把较早的上下文整理成摘要，保留当前会话继续对话。</div>
          <div class="mt-2 text-xs opacity-70">适合上下文变长后继续推进同一主题。</div>
          <div
            v-if="preview?.compactionDisabledReason"
            class="mt-3 rounded border border-warning/30 bg-warning/10 px-3 py-2 text-sm text-warning-content"
          >
            {{ preview.compactionDisabledReason }}
          </div>
        </div>
        <div class="mt-3 text-xs opacity-60">
          <div>消息数：{{ preview?.messageCount ?? 0 }}</div>
          <div>上下文占用：{{ preview?.contextUsagePercent ?? 0 }}%</div>
        </div>
      </template>
      <div v-if="errorText" class="mt-3 rounded border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
        {{ errorText }}
      </div>
      <div class="modal-action">
        <button class="btn btn-sm" :disabled="loading || running" @click="emit('close')">取消</button>
        <button
          class="btn btn-sm btn-primary"
          :disabled="loading || running || !preview?.canCompact"
          @click="emit('confirm')"
        >
          <span v-if="running" class="loading loading-spinner loading-xs"></span>
          <span>{{ running ? "正在压缩" : "压缩" }}</span>
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('close')">close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
type CompactionPreviewResult = {
  conversationId: string;
  canCompact: boolean;
  messageCount: number;
  hasAssistantReply: boolean;
  isEmpty: boolean;
  contextUsagePercent: number;
  compactionDisabledReason?: string | null;
};

defineProps<{
  open: boolean;
  loading: boolean;
  running: boolean;
  preview: CompactionPreviewResult | null;
  errorText: string;
}>();

const emit = defineEmits<{
  close: [];
  confirm: [];
}>();
</script>

<template>
  <aside class="flex h-full min-h-0 flex-col overflow-x-hidden">
    <div v-if="errorText" class="mx-4 my-4 rounded-box border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
      {{ errorText }}
    </div>
    <div v-if="loading && statuses.length === 0" class="flex min-h-0 flex-1 items-center justify-center text-sm text-base-content/65">
      <span class="loading loading-spinner loading-sm mr-2"></span>
      {{ t("chat.toolReview.delegateLoading") }}
    </div>
    <div v-else-if="statuses.length === 0" class="px-4 py-4 text-sm text-base-content/65">
      {{ t("chat.toolReview.delegateEmpty") }}
    </div>
    <div v-else class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto px-4 py-3">
      <section
        v-for="delegate in statuses"
        :key="delegate.delegateId"
        class="w-full min-w-0 rounded-box border border-base-300 bg-base-200 px-3 py-3"
      >
        <div class="flex min-w-0 items-center justify-between gap-3">
          <div class="min-w-0 truncate text-sm font-medium text-base-content/85">
            {{ delegate.title || delegate.delegateId }}
          </div>
          <div class="badge badge-sm shrink-0 whitespace-nowrap" :class="delegateStatusBadgeClass(delegate.status)">
            {{ formatDelegateStatus(delegate.status) }}
          </div>
        </div>
        <div class="mt-3 flex flex-col gap-2 text-xs text-base-content/70">
          <div class="flex min-w-0 items-center justify-between gap-3">
            <span class="shrink-0">用时 {{ formatElapsedMs(delegate.elapsedMs) }}</span>
            <span class="shrink-0">{{ delegate.requestCount }}步</span>
          </div>
          <div class="flex min-w-0 items-center justify-between gap-3">
            <span class="min-w-0 truncate">最近工具 {{ delegate.lastToolName || "-" }}</span>
            <span class="shrink-0">用量 {{ formatTokenK(delegate.tokenCount) }}</span>
          </div>
        </div>
        <div class="mt-3 flex justify-end gap-2">
          <button
            v-if="isDelegateRunning(delegate.status)"
            type="button"
            class="btn btn-sm btn-error btn-outline gap-1.5 font-normal"
            @click="emit('abortDelegate', delegate)"
          >打断</button>
          <button
            type="button"
            class="btn btn-sm gap-1.5 border-base-300 bg-base-100 font-normal hover:bg-base-100"
            @click="emit('openDelegateDetail', delegate)"
          >查看详情</button>
        </div>
      </section>
    </div>
  </aside>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import type { ConversationDelegateStatusSummary } from "../../../types/app";

defineProps<{
  statuses: ConversationDelegateStatusSummary[];
  loading: boolean;
  errorText: string;
}>();

const emit = defineEmits<{
  (e: "openDelegateDetail", status: ConversationDelegateStatusSummary): void;
  (e: "abortDelegate", status: ConversationDelegateStatusSummary): void;
}>();

const { t } = useI18n();

function formatDelegateStatus(status: string) {
  if (status === "running" || status === "delivered") return "执行中";
  if (status === "completed") return "已完成";
  if (status === "failed") return "失败";
  return "未知";
}

function isDelegateRunning(status: string) {
  return status === "running" || status === "delivered";
}

function delegateStatusBadgeClass(status: string) {
  if (status === "completed") return "badge-primary";
  if (status === "failed") return "badge-error";
  if (status === "running" || status === "delivered") return "badge-warning";
  return "badge-ghost";
}

function formatTokenK(value: number) {
  if (!Number.isFinite(value) || value <= 0) return "0K";
  const k = value / 1000;
  if (k < 10) return `${k.toFixed(1)}K`;
  return `${Math.round(k)}K`;
}

function formatElapsedMs(value: number) {
  if (!Number.isFinite(value) || value <= 0) return "0秒";
  const totalSeconds = Math.floor(value / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  if (hours > 0) return `${hours}时${minutes}分`;
  if (minutes > 0) return `${minutes}分${seconds}秒`;
  return `${seconds}秒`;
}
</script>

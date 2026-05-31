<template>
  <div class="mt-1 text-xs text-base-content/65">
    <template v-if="running">
      <span>用时 {{ elapsedText }}</span>
      <span class="mx-1">·</span>
      <span>{{ requestCount }}步</span>
      <span class="mx-1">·</span>
      <span>用量 {{ tokenText }}</span>
      <template v-if="lastToolName">
        <span class="mx-1">·</span>
        <span>{{ lastToolName }}</span>
      </template>
    </template>
    <template v-else-if="text">
      {{ text }}
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  elapsedMs?: number;
  requestCount?: number;
  tokenCount?: number;
  lastToolName?: string;
  text?: string;
  running?: boolean;
}>();

const elapsedText = computed(() => formatElapsedMs(props.elapsedMs ?? 0));
const requestCount = computed(() => props.requestCount ?? 0);
const tokenText = computed(() => formatTokenK(props.tokenCount ?? 0));

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

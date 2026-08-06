<script setup lang="ts">
// 动态数字更新器（双重缓冲）：props.target 是真实目标值（节流后每 100ms 跳变），
// 显示值用 rAF 在 300ms 内线性爬升到目标，数字平滑增长不闪烁。
// 只 re-render 本组件，不拖累消息主体。
import { onBeforeUnmount, ref, watch } from "vue";

const props = defineProps<{ target: number }>();

const ANIMATE_DURATION_MS = 300;

const display = ref(props.target);
let rafId = 0;
let fromValue = props.target;
let startedAtMs = 0;

function stopRaf() {
  if (rafId) {
    cancelAnimationFrame(rafId);
    rafId = 0;
  }
}

function tick(ts: number) {
  const elapsed = Math.max(0, ts - startedAtMs);
  const progress = Math.min(1, elapsed / ANIMATE_DURATION_MS);
  display.value = fromValue + (props.target - fromValue) * progress;
  if (progress < 1) {
    rafId = requestAnimationFrame(tick);
  } else {
    display.value = props.target;
    stopRaf();
  }
}

watch(
  () => props.target,
  (next) => {
    if (next === display.value) return;
    fromValue = display.value;
    startedAtMs = performance.now();
    if (!rafId) {
      rafId = requestAnimationFrame(tick);
    }
  },
);

onBeforeUnmount(stopRaf);
</script>

<template>
  <span v-if="display > 0" class="tabular-nums">({{ Math.round(display).toLocaleString("zh-CN") }})</span>
</template>

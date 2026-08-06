<template>
  <div class="relative h-full w-full">
    <canvas ref="chartContainer"></canvas>
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  ArcElement,
  BarController,
  BarElement,
  CategoryScale,
  Chart,
  Filler,
  Legend,
  LineController,
  LinearScale,
  LineElement,
  PointElement,
  PolarAreaController,
  RadialLinearScale,
  Tooltip,
} from "chart.js";

Chart.register(
  ArcElement,
  BarController,
  BarElement,
  CategoryScale,
  Filler,
  Legend,
  LineController,
  LinearScale,
  LineElement,
  PointElement,
  PolarAreaController,
  RadialLinearScale,
  Tooltip,
);

const props = defineProps<{
  config: Record<string, unknown>;
}>();

const chartContainer = ref<HTMLCanvasElement | null>(null);
let chart: Chart | null = null;
let themeObserver: MutationObserver | null = null;

function cssVar(name: string, fallback: string): string {
  if (typeof document === "undefined") return fallback;
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return value || fallback;
}

function render(): void {
  if (!chartContainer.value) return;
  const axisColor = cssVar("--color-base-content", "#000000");
  const gridColor = cssVar("--color-base-content", "#000000");
  const tooltipBg = cssVar("--color-base-content", "#000000");
  const tooltipText = cssVar("--color-base-100", "#ffffff");
  // 注意：不能 JSON 序列化配置，会丢失 ticks callback 等函数；直接引用原对象，
  // 只就地覆盖颜色相关字段。
  const config = props.config as {
    type?: string;
    data?: Record<string, unknown>;
    options?: Record<string, unknown>;
  };
  const options = (config.options ?? {}) as Record<string, unknown>;
  const scales = (options.scales ?? {}) as Record<string, Record<string, unknown>>;
  const ticksStyle = { color: axisColor, font: { size: 10 } };
  for (const key of Object.keys(scales)) {
    const scale = scales[key];
    scale.ticks = { ...((scale.ticks as Record<string, unknown>) ?? {}), ...ticksStyle };
  }
  options.scales = scales;
  options.plugins = {
    ...((options.plugins as Record<string, unknown>) ?? {}),
    tooltip: {
      ...(((options.plugins as Record<string, unknown>)?.tooltip as Record<string, unknown>) ?? {}),
      backgroundColor: tooltipBg,
      titleColor: tooltipText,
      bodyColor: tooltipText,
    },
  };
  config.options = options;
  if (!chart) {
    chart = new Chart(chartContainer.value, config as never);
  } else if ((chart.config as { type?: string }).type !== config.type) {
    // 图表类型切换（柱↔折线）：Chart.js 控制器不会随 update 重建，销毁重建
    chart.destroy();
    chart = new Chart(chartContainer.value, config as never);
  } else {
    chart.data = (config.data ?? { datasets: [] }) as never;
    chart.options = options as never;
    chart.update();
  }
}

onMounted(() => {
  render();
  if (typeof document !== "undefined" && typeof MutationObserver !== "undefined") {
    themeObserver = new MutationObserver(() => {
      render();
    });
    themeObserver.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });
  }
});
watch(() => props.config, render, { deep: true });

onBeforeUnmount(() => {
  themeObserver?.disconnect();
  themeObserver = null;
  if (chart) {
    chart.destroy();
    chart = null;
  }
});
</script>

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
  DoughnutController,
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
import type { ChartType, TooltipModel } from "chart.js";

Chart.register(
  ArcElement,
  BarController,
  BarElement,
  CategoryScale,
  DoughnutController,
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
let tooltipEl: HTMLDivElement | null = null;

function ensureTooltipEl(): HTMLDivElement {
  if (!tooltipEl) {
    tooltipEl = document.createElement("div");
    tooltipEl.style.position = "fixed";
    tooltipEl.style.pointerEvents = "none";
    tooltipEl.style.zIndex = "9999";
    tooltipEl.style.transition = "opacity 0.15s ease";
    tooltipEl.style.borderRadius = "6px";
    tooltipEl.style.padding = "6px 10px";
    tooltipEl.style.fontSize = "12px";
    tooltipEl.style.boxShadow = "0 4px 12px rgba(0,0,0,0.18)";
    tooltipEl.style.opacity = "0";
    document.body.appendChild(tooltipEl);
  }
  return tooltipEl;
}

// 外部 tooltip：挂到 body，脱离容器 overflow 裁剪链（Chart.js 官方推荐方案）。
function externalTooltip(context: { chart: Chart; tooltip: TooltipModel<ChartType> }): void {
  const { chart, tooltip } = context;
  const el = ensureTooltipEl();
  if (tooltip.opacity === 0) {
    el.style.opacity = "0";
    return;
  }
  const options = tooltip.options as {
    backgroundColor?: string;
    titleColor?: string;
    bodyColor?: string;
  };
  const rect = chart.canvas.getBoundingClientRect();
  el.style.opacity = "1";
  el.style.background = options.backgroundColor ?? "#000000";
  el.style.color = options.bodyColor ?? "#ffffff";
  el.style.left = `${rect.left + tooltip.caretX}px`;
  el.style.top = `${rect.top + tooltip.caretY}px`;
  el.style.transform = "translate(-50%, -110%)";
  const title = (tooltip.title ?? []).map((t) => `<div style="font-weight:600;margin-bottom:2px;color:${options.titleColor ?? options.bodyColor ?? "#ffffff"}">${t}</div>`).join("");
  const body = (tooltip.body ?? [])
    .map((item, i) => {
      const lines = item.lines ?? [];
      if (lines.length === 0) return "";
      const color = tooltip.labelColors?.[i]?.backgroundColor ?? "transparent";
      const swatch = `<span style="display:inline-block;width:8px;height:8px;border-radius:2px;background:${color};margin-right:6px;vertical-align:middle"></span>`;
      return `<div>${swatch}${lines.join("<br>")}</div>`;
    })
    .filter(Boolean)
    .join("");
  el.innerHTML = title + body;
}

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
  // dataset 背景/边框颜色若为 CSS 变量，解析为实际色值（canvas 不认 var()）。
  const data = (config.data ?? { datasets: [] }) as {
    datasets?: Array<Record<string, unknown>>;
  };
  for (const dataset of data.datasets ?? []) {
    for (const key of ["backgroundColor", "borderColor", "pointBackgroundColor", "pointBorderColor"] as const) {
      const raw = dataset[key];
      if (typeof raw === "string" && raw.trim().startsWith("var(")) {
        dataset[key] = cssVar(raw.trim().slice(4, -1), "#000000");
      } else if (Array.isArray(raw)) {
        dataset[key] = raw.map((item) =>
          typeof item === "string" && item.trim().startsWith("var(")
            ? cssVar(item.trim().slice(4, -1), "#000000")
            : item,
        );
      }
    }
  }
  config.data = data;
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
      // enabled: false 关闭自带 canvas 绘制，external 挂到 body 独立渲染（避免双 tooltip）
      enabled: false,
      external: externalTooltip,
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
  if (tooltipEl) {
    tooltipEl.remove();
    tooltipEl = null;
  }
  if (chart) {
    chart.destroy();
    chart = null;
  }
});
</script>

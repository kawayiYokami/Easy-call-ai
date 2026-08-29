<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

const props = withDefaults(
  defineProps<{
    /** 完整文本，直接平铺渲染 */
    text: string;
    /** 预览高度（px），超出截断并显示「展开」 */
    previewHeight?: number;
    /** 流式跟随：不截断，高度随内容增长 */
    follow?: boolean;
    /** 文本容器附加类（颜色等由调用方决定） */
    textClass?: string;
  }>(),
  { previewHeight: 160, follow: false, textClass: "" },
);

const { t } = useI18n();

const bodyRef = ref<HTMLElement | null>(null);
const overflowing = ref(false);
const expanded = ref(false);

const clamped = computed(() => overflowing.value && !props.follow && !expanded.value);

let resizeObserver: ResizeObserver | null = null;

function measure(): void {
  const el = bodyRef.value;
  if (!el) return;
  overflowing.value = el.scrollHeight > props.previewHeight + 1;
}

watch(
  () => [props.text, props.previewHeight, props.follow] as const,
  () => {
    nextTick(measure);
  },
);

onMounted(() => {
  measure();
  if (bodyRef.value && typeof ResizeObserver !== "undefined") {
    resizeObserver = new ResizeObserver(() => measure());
    resizeObserver.observe(bodyRef.value);
  }
});

onBeforeUnmount(() => {
  resizeObserver?.disconnect();
  resizeObserver = null;
});
</script>

<template>
  <div class="flex min-w-0 flex-col">
    <div
      ref="bodyRef"
      class="whitespace-pre-wrap wrap-break-word text-xs leading-relaxed"
      :class="[props.textClass, { 'ecall-expandable-text-clamped': clamped }]"
      :style="clamped ? { '--ecall-expandable-preview-height': `${previewHeight}px` } : undefined"
    >{{ text }}</div>
    <div v-if="clamped || expanded" class="mt-0.5">
      <button
        v-if="clamped"
        type="button"
        class="inline-flex items-center text-xs text-base-content/45 hover:text-base-content/80"
        data-selection-ignore="true"
        @click.stop="expanded = true"
      >
        {{ t("common.expand") }}
      </button>
      <button
        v-else
        type="button"
        class="inline-flex items-center text-xs text-base-content/45 hover:text-base-content/80"
        data-selection-ignore="true"
        @click.stop="expanded = false"
      >
        {{ t("common.collapse") }}
      </button>
    </div>
  </div>
</template>

<style scoped>
/* 截断态：限制预览高度，底部渐隐提示还有更多内容 */
.ecall-expandable-text-clamped {
  max-height: var(--ecall-expandable-preview-height, 160px);
  overflow: hidden;
  mask-image: linear-gradient(to bottom, #000 calc(100% - 2.25rem), transparent 100%);
}
</style>

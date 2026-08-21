<template>
  <div v-if="adjusterOptions.length > 1" class="w-full min-w-0">
    <input
      type="range"
      min="0"
      :max="adjusterOptions.length - 1"
      step="1"
      :value="activeIndex"
      class="range"
      @input="onSlide"
    />
    <div class="-mt-0.5 flex justify-between px-2.5 leading-none text-sm">
      <span v-for="option in adjusterOptions" :key="option.effort" class="font-bold">.</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { ApiConfigItem } from "../../../types/app";
import { normalizeReasoningEffortValue, reasoningEffortDisplayLabel, sortReasoningEffortValues } from "../utils/api-config-display";
import { buildModelGroupIndex, rememberModelEffort } from "../utils/model-effort-memory";

const props = withDefaults(defineProps<{
  options: ApiConfigItem[];
  /** 当前生效的模型配置 id */
  selectedId?: string;
}>(), {
  selectedId: "",
});

const emit = defineEmits<{
  (event: "select", value: string): void;
}>();

const { t } = useI18n();

function effortOf(item: ApiConfigItem | null | undefined): string {
  return normalizeReasoningEffortValue(item?.reasoningEffort) || "default";
}

const current = computed(() => {
  const target = String(props.selectedId || "").trim();
  if (!target) return undefined;
  return (Array.isArray(props.options) ? props.options : [])
    .find((item) => String(item?.id || "").trim() === target);
});

// 分组索引只在选项列表变化时重建一次
const groupIndex = computed(() => buildModelGroupIndex(Array.isArray(props.options) ? props.options : []));

const groupItems = computed(() => {
  const entry = groupIndex.value.get(String(props.selectedId || "").trim());
  if (!entry) return [];
  return [...entry.items].sort((left, right) => {
    const ordered = sortReasoningEffortValues([effortOf(left), effortOf(right)]);
    return ordered.indexOf(effortOf(left)) - ordered.indexOf(effortOf(right));
  });
});

const activeEffort = computed(() => (current.value ? effortOf(current.value) : ""));

const activeIndex = computed(() => {
  const index = adjusterOptions.value.findIndex((option) => option.effort === activeEffort.value);
  return index >= 0 ? index : 0;
});

const adjusterOptions = computed(() => {
  if (groupItems.value.length <= 1) return [];
  const seen = new Set<string>();
  return groupItems.value
    .map((item) => effortOf(item))
    .filter((effort) => {
      if (seen.has(effort)) return false;
      seen.add(effort);
      return true;
    })
    .map((effort) => ({ effort, label: reasoningEffortDisplayLabel(effort, t) || effort }));
});

/** 滑动条按档位序号换档：每吸附一格即生效一档。 */
function onSlide(event: Event) {
  const index = Number((event.target as HTMLInputElement).valueAsNumber);
  const option = adjusterOptions.value[index];
  if (!option) return;
  selectEffort(option.effort);
}

/** 调节器点击即显式换档：目标叶子一定存在，直接生效并更新记忆。 */
function selectEffort(effort: string) {
  const target = groupItems.value.find((item) => effortOf(item) === effort);
  if (!target) return;
  const entry = groupIndex.value.get(String(target.id).trim());
  if (entry) rememberModelEffort(entry.groupKey, effort);
  emit("select", String(target.id));
}
</script>

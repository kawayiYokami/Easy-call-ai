<template>
  <div ref="triggerWrapRef" :class="variant === 'field' ? 'relative min-w-0' : 'relative min-w-0'" v-bind="$attrs">
    <button
      ref="triggerButtonRef"
      type="button"
      :class="variant === 'field'
        ? 'select select-bordered flex w-full items-center justify-between gap-2 pr-3 text-left'
        : 'btn btn-sm h-8 min-h-8 w-auto min-w-28 max-w-56 justify-between border-0 shadow-none bg-base-100 text-base-content hover:bg-base-200 max-md:min-w-0'"
      :disabled="disabled || (normalizedOptions.length === 0 && extraOptions.length === 0 && !placeholder)"
      :title="selectedModelTitle"
      @click="toggleDropdown"
    >
      <span
        :class="variant === 'field'
          ? ['min-w-0 flex-1 truncate', selectedModelName ? '' : 'text-base-content/50']
          : 'min-w-0 flex-1 truncate'"
      >
        {{ triggerLabel }}
      </span>
      <!-- 等级独立成段且不收缩：模型名再长也不会把它截掉 -->
      <span v-if="triggerEffort" class="ml-1 shrink-0 whitespace-nowrap text-base-content/70">· {{ triggerEffort }}</span>
      <ChevronDown
        :class="['h-3 w-3 shrink-0 opacity-50 transition-transform', dropdownOpen ? 'rotate-0' : 'rotate-180']"
        :size="variant === 'field' ? 16 : 12"
      />
    </button>
  </div>
  <Teleport to="body">
    <div
      v-if="dropdownOpen && !disabled"
      ref="panelRef"
      class="fixed z-1200 flex flex-col overflow-hidden rounded-box border border-base-300 bg-base-100 text-base-content shadow-xl"
      :data-theme="teleportTheme"
      :style="panelStyle"
    >
      <div class="relative flex min-h-0 flex-1 flex-col">
        <div
          ref="scrollRef"
          class="ecall-api-picker-scroll min-h-0 flex-1 overflow-y-auto overflow-x-hidden"
        >
          <ApiConfigSelectionMenu
            :tree="modelOnlyTree"
            :selected-id="modelValue"
            :extra-options="extraOptions"
            :placeholder="placeholder"
            @select="handleSelect"
          />
        </div>
        <FloatingScrollbar ref="scrollbarRef" :target="scrollRef" />
      </div>
      <div class="shrink-0 border-t border-base-300 p-1">
        <ChatModelEffortAdjuster
          class="px-2 py-1.5"
          :options="apiConfigs"
          :selected-id="modelValue"
          @select="handleEffortSelect"
        />
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { ChevronDown } from "@lucide/vue";
import type { ApiConfigItem } from "../../../types/app";
import ApiConfigSelectionMenu from "./ApiConfigSelectionMenu.vue";
import ChatModelEffortAdjuster from "./ChatModelEffortAdjuster.vue";
import FloatingScrollbar from "../../shell/components/FloatingScrollbar.vue";
import { formatApiConfigOptionLabel, normalizeReasoningEffortValue, reasoningEffortDisplayLabel, sortReasoningEffortValues } from "../utils/api-config-display";
import { buildApiConfigSelectionTree } from "../utils/api-config-selection-tree";
import { buildModelGroupIndex, getModelEffortMemory, rememberModelEffort, resolveModelEffortSelection } from "../utils/model-effort-memory";

defineOptions({ inheritAttrs: false });

const props = withDefaults(defineProps<{
  modelValue?: string;
  apiConfigs: ApiConfigItem[];
  /** 触发器形态：chip = 聊天输入面板小按钮；field = 配置表单整行选择框 */
  variant?: "chip" | "field";
  placeholder?: string;
  extraOptions?: Array<{ id: string; label: string }>;
  disabled?: boolean;
  /** Teleport 面板主题（默认取 document 根主题） */
  theme?: string;
}>(), {
  modelValue: "",
  variant: "field",
  placeholder: "",
  extraOptions: () => [],
  disabled: false,
  theme: "",
});

const emit = defineEmits<{
  (event: "update:modelValue", value: string): void;
}>();

const { t } = useI18n();

const dropdownOpen = ref(false);
const triggerWrapRef = ref<HTMLElement | null>(null);
const triggerButtonRef = ref<HTMLButtonElement | null>(null);
const panelRef = ref<HTMLElement | null>(null);
const scrollRef = ref<HTMLElement | null>(null);
const scrollbarRef = ref<InstanceType<typeof FloatingScrollbar> | null>(null);
const panelStyle = ref<Record<string, string>>({
  left: "0px",
  top: "0px",
  width: "20rem",
  maxHeight: "80vh",
});

const normalizedOptions = computed(() =>
  (Array.isArray(props.apiConfigs) ? props.apiConfigs : [])
    .filter((item) => !!String(item?.id || "").trim()),
);

// 分组索引只在选项列表变化时重建一次，避免每次选中变化重复深度序列化
const groupIndex = computed(() => buildModelGroupIndex(normalizedOptions.value));

function effortOf(item: ApiConfigItem | null | undefined): string {
  return normalizeReasoningEffortValue(item?.reasoningEffort) || "default";
}

function itemById(id: string): ApiConfigItem | undefined {
  const entry = groupIndex.value.get(String(id || "").trim());
  return entry?.items.find((item) => String(item.id || "").trim() === String(id || "").trim());
}

const teleportTheme = computed(() => {
  const documentTheme = typeof document === "undefined" ? "" : document.documentElement.getAttribute("data-theme");
  return String(props.theme || documentTheme || "light").trim() || "light";
});

// 模型名段不含等级：等级由独立不收缩段渲染，模型名再长也截不掉等级
const selectedModelName = computed(() => {
  const found = itemById(props.modelValue);
  if (!found) return "";
  return formatApiConfigOptionLabel(
    { ...found, reasoningEffort: "" },
    t,
    { providerMaxCharacters: props.variant === "chip" ? 2 : undefined },
  );
});
// 悬浮标题与 triggerLabel 对齐：extraOptions 用自身文案，模型用完整「供应商/模型 · 等级」
const selectedModelTitle = computed(() => {
  const extraOption = props.extraOptions.find((option) => option.id === props.modelValue);
  if (extraOption) return extraOption.label;
  const found = itemById(props.modelValue);
  if (found) return formatApiConfigOptionLabel({ ...found, reasoningEffort: effortOf(found) }, t);
  return props.placeholder || props.modelValue;
});
// 等级段：空等级归一为 default，保证「· 默认」始终可见；extraOptions 无等级概念
const triggerEffort = computed(() => {
  if (props.extraOptions.some((option) => option.id === props.modelValue)) return "";
  const found = itemById(props.modelValue);
  if (!found) return "";
  return reasoningEffortDisplayLabel(effortOf(found), t);
});

const triggerLabel = computed(() => {
  const extraOption = props.extraOptions.find((option) => option.id === props.modelValue);
  if (extraOption) return extraOption.label;
  return selectedModelName.value || props.placeholder || props.modelValue;
});

/**
 * 下拉只列模型，不展示思维等级子菜单：每个模型组折叠成单叶。
 * 叶子取当前选中项（保持高亮与真实档位）；未选中的组按与点击生效
 * 完全相同的规则预演（记忆档 → default → 最近档），保证徽章显示的
 * 等级就是点击后实际生效的等级，且不随当前选择漂移。
 */
const modelOnlyTree = computed(() => {
  const tree = buildApiConfigSelectionTree(props.apiConfigs, t);
  return tree.map((provider) => ({
    ...provider,
    models: provider.models.map((group) => {
      const orderedLeaves = [...group.leaves].sort((left, right) => {
        const ordered = sortReasoningEffortValues([effortOf(left.item), effortOf(right.item)]);
        return ordered.indexOf(effortOf(left.item)) - ordered.indexOf(effortOf(right.item));
      });
      if (orderedLeaves.length === 0) return { ...group, leaves: [] };
      const activeLeaf = orderedLeaves.find((leaf) => leaf.id === props.modelValue);
      if (activeLeaf) return { ...group, leaves: [activeLeaf] };
      const indexEntry = groupIndex.value.get(orderedLeaves[0].id);
      const preview = indexEntry
        ? resolveModelEffortSelection(indexEntry.items, indexEntry.groupKey, orderedLeaves[0].id, getModelEffortMemory())
        : null;
      const previewLeaf = (preview && orderedLeaves.find((leaf) => leaf.id === preview.leafId)) || orderedLeaves[0];
      return { ...group, leaves: [previewLeaf] };
    }),
  }));
});

function toggleDropdown() {
  if (props.disabled) return;
  dropdownOpen.value = !dropdownOpen.value;
}

function handleClickOutside(event: MouseEvent) {
  const target = event.target as Node | null;
  if (!target) return;
  if (triggerWrapRef.value?.contains(target)) return;
  if (panelRef.value?.contains(target)) return;
  dropdownOpen.value = false;
}

async function refreshPosition() {
  if (!dropdownOpen.value) return;
  const trigger = triggerButtonRef.value || triggerWrapRef.value;
  if (!trigger) return;
  await nextTick();
  const margin = 8;
  const gap = 8;
  const triggerRect = trigger.getBoundingClientRect();
  const viewportWidth = window.innerWidth || document.documentElement.clientWidth || 0;
  const viewportHeight = window.innerHeight || document.documentElement.clientHeight || 0;
  const preferredWidth = Math.max(Math.round(triggerRect.width), 320);
  const maxAllowedWidth = Math.max(220, viewportWidth - margin * 2);
  const width = Math.min(preferredWidth, maxAllowedWidth);
  const spaceAbove = Math.max(0, triggerRect.top - margin - gap);
  const spaceBelow = Math.max(0, viewportHeight - triggerRect.bottom - margin - gap);
  // 优先下方；下方更挤时才向上开
  const openUpward = spaceAbove > spaceBelow;
  const availableHeight = openUpward ? spaceAbove : spaceBelow;
  const maxHeight = Math.max(
    0,
    Math.min(Math.floor(viewportHeight * 0.8), Math.floor(availableHeight)),
  );
  const left = Math.min(
    Math.max(margin, triggerRect.left),
    Math.max(margin, viewportWidth - width - margin),
  );
  const maxHeightPx = `${Math.round(maxHeight)}px`;

  if (openUpward) {
    // 用 bottom 锚定触发器上方，避免 top 计算误差把面板顶出屏幕
    const bottom = Math.max(margin, viewportHeight - triggerRect.top + gap);
    panelStyle.value = {
      left: `${Math.round(left)}px`,
      right: "auto",
      top: "auto",
      bottom: `${Math.round(bottom)}px`,
      width: `${Math.round(width)}px`,
      maxWidth: `calc(100vw - ${margin * 2}px)`,
      maxHeight: maxHeightPx,
      height: "auto",
    };
  } else {
    const top = triggerRect.bottom + gap;
    panelStyle.value = {
      left: `${Math.round(left)}px`,
      right: "auto",
      top: `${Math.round(top)}px`,
      bottom: "auto",
      width: `${Math.round(width)}px`,
      maxWidth: `calc(100vw - ${margin * 2}px)`,
      maxHeight: maxHeightPx,
      height: "auto",
    };
  }
  await nextTick();
  scrollbarRef.value?.updateThumb();
}

watch(dropdownOpen, (open) => {
  if (open) {
    nextTick(() => {
      void refreshPosition();
      document.addEventListener("click", handleClickOutside);
    });
  } else {
    document.removeEventListener("click", handleClickOutside);
  }
});

onMounted(() => {
  window.addEventListener("resize", refreshPosition);
  window.addEventListener("scroll", refreshPosition, true);
});

onBeforeUnmount(() => {
  window.removeEventListener("resize", refreshPosition);
  window.removeEventListener("scroll", refreshPosition, true);
  document.removeEventListener("click", handleClickOutside);
});

/** 面板底部的等级调节：显式换档，不关闭下拉，便于连续浏览调整。 */
function handleEffortSelect(id: string) {
  emit("update:modelValue", String(id || "").trim());
}

/**
 * 下拉选模型：点击的叶子按模型组解析实际生效等级
 * （记忆等级 → default → 最近档），并回写记忆。
 * 目标模型有多个等级时保持面板打开，方便继续在底部调档；单档位直接关闭。
 * placeholder / extraOptions 的值（不在模型列表内）原样透传。
 */
function handleSelect(id: string) {
  const nextId = String(id || "").trim();
  if (nextId === props.modelValue) {
    dropdownOpen.value = false;
    return;
  }
  const requestedEntry = nextId ? groupIndex.value.get(nextId) : undefined;
  let effectiveId = nextId;
  let keepOpen = false;
  if (requestedEntry) {
    const current = itemById(props.modelValue);
    const currentEntry = current ? groupIndex.value.get(String(current.id)) : undefined;
    if (!currentEntry || currentEntry.groupKey !== requestedEntry.groupKey) {
      const resolution = resolveModelEffortSelection(
        requestedEntry.items,
        requestedEntry.groupKey,
        nextId,
        getModelEffortMemory(),
      );
      if (resolution) {
        effectiveId = resolution.leafId;
        rememberModelEffort(resolution.groupKey, resolution.effort);
        // 目标模型有多个等级时保持打开，方便接着在底部调档
        keepOpen = requestedEntry.items.length > 1;
      }
    } else {
      // 同模型组重选：视为确认当前选择，刷新记忆
      rememberModelEffort(requestedEntry.groupKey, effortOf(itemById(nextId)));
    }
  }
  dropdownOpen.value = keepOpen;
  // 先回写选中值，父组件同步后面板底部等级条完成渲染，refreshPosition
  // 的 nextTick 才能测到最终尺寸
  emit("update:modelValue", effectiveId);
  if (keepOpen) {
    void refreshPosition();
  }
}
</script>

<style scoped>
/* 滚动条由 FloatingScrollbar 接管，原生滚动条隐藏 */
.ecall-api-picker-scroll {
  scrollbar-gutter: auto;
  scrollbar-width: none;
  -ms-overflow-style: none;
}
.ecall-api-picker-scroll::-webkit-scrollbar {
  width: 0;
  height: 0;
}
</style>

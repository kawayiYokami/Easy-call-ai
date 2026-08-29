<template>
  <div
    role="radiogroup"
    class="inline-flex items-stretch rounded-field bg-base-200 p-0.5"
    :class="[
      fullWidth ? 'flex w-full min-w-0' : '',
      disabled ? 'pointer-events-none opacity-60' : '',
    ]"
  >
    <button
      v-for="option in options"
      :key="String(option.value)"
      type="button"
      role="radio"
      class="flex items-center justify-center whitespace-nowrap rounded-[calc(var(--radius-field)-2px)] transition-colors"
      :class="[
        fullWidth ? 'min-w-0 flex-1' : '',
        sizeClass,
        isSelected(option.value)
          ? 'bg-base-100 font-medium text-base-content shadow-sm'
          : 'text-base-content/60 hover:text-base-content',
      ]"
      :disabled="disabled || !!option.disabled"
      :aria-checked="isSelected(option.value)"
      @click="selectValue(option.value)"
    >
      {{ option.label }}
    </button>
  </div>
</template>

<script setup lang="ts" generic="T extends string | number | boolean">
import { computed } from "vue";

export type SegmentedControlOption<T extends string | number | boolean> = {
  value: T;
  label: string;
  disabled?: boolean;
};

const props = withDefaults(defineProps<{
  modelValue: T;
  options: Array<SegmentedControlOption<T>>;
  disabled?: boolean;
  fullWidth?: boolean;
  size?: "xs" | "sm" | "md";
}>(), {
  disabled: false,
  fullWidth: true,
  size: "md",
});

const emit = defineEmits<{
  (e: "update:modelValue", value: T): void;
  (e: "change", value: T): void;
}>();

// DaisyUI 5 的 tab 尺寸类（tab-sm 等）已不存在，尺寸全部用 utility 自控
const sizeClass = computed(() => {
  if (props.size === "xs") return "h-6 px-2 text-xs leading-none";
  if (props.size === "sm") return "h-7 px-3 text-xs leading-none";
  return "h-8 px-3.5 text-sm leading-none";
});

function isSelected(value: T): boolean {
  return props.modelValue === value;
}

function selectValue(value: T) {
  if (props.disabled || props.modelValue === value) return;
  emit("update:modelValue", value);
  emit("change", value);
}
</script>

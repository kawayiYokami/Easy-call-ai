<template>
  <div class="group relative mx-1">
    <button
      type="button"
      class="block w-full rounded-lg px-2 py-2 text-left transition-colors hover:bg-base-100/70 disabled:cursor-default disabled:hover:bg-transparent"
      :disabled="disabled"
      :title="title"
      @click="$emit('click')"
    >
      <div class="flex min-w-0 items-start gap-2">
        <div class="mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-full bg-base-100 text-base-content/65">
          <ListTodo class="h-4 w-4" />
        </div>
        <div class="min-w-0 flex-1">
          <div class="flex min-w-0 items-start justify-between gap-2">
            <div class="min-w-0 truncate text-sm font-medium">
              {{ label }}
            </div>
            <span v-if="timeLabel" class="shrink-0 text-[11px] text-base-content/55">
              {{ timeLabel }}
            </span>
          </div>
          <div class="mt-1 truncate text-xs text-base-content/55">
            {{ description }}
          </div>
        </div>
      </div>
    </button>
  </div>
</template>

<script setup lang="ts">
import { ListTodo } from "@lucide/vue";

defineProps<{
  label: string;
  description: string;
  title?: string;
  timeLabel?: string;
  disabled?: boolean;
}>();

defineEmits<{
  click: [];
}>();
</script>

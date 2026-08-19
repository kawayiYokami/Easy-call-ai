<template>
  <div class="overflow-hidden rounded-box bg-base-100">
    <header
      v-if="title"
      class="flex items-center justify-between gap-3 border-b-2 border-base-200 px-4 py-3"
    >
      <h3 class="text-sm font-semibold">{{ title }}</h3>
      <slot name="actions" />
    </header>
    <div class="px-3">
      <slot />
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{ title?: string }>();
</script>

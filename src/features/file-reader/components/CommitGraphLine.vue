<script setup lang="ts">
import { SWIMLANE_HEIGHT } from "../git-commit-graph";

defineProps<{ x: number; color: string; width: number }>();
</script>

<template>
  <svg :width="width" :height="SWIMLANE_HEIGHT" class="shrink-0">
    <line
      :x1="x"
      :y1="0"
      :x2="x"
      :y2="SWIMLANE_HEIGHT"
      :stroke="color"
      stroke-width="2"
      stroke-linecap="round"
    />
  </svg>
</template>

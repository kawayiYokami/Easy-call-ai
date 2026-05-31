<template>
  <dialog class="modal" :class="{ 'modal-open': open }">
    <div class="modal-box w-11/12 max-w-6xl p-2 bg-base-100">
      <div class="mb-2 flex items-center justify-end gap-1">
        <button class="btn btn-xs" :disabled="zoom <= minZoom" @click="emit('zoomOut')">
          <Minus class="h-3 w-3" />
        </button>
        <button class="btn btn-xs" :disabled="zoom >= maxZoom" @click="emit('zoomIn')">
          <Plus class="h-3 w-3" />
        </button>
        <button class="btn btn-xs" :disabled="Math.abs(zoom - 1) < 0.001" @click="emit('reset')">
          {{ Math.round(zoom * 100) }}%
        </button>
      </div>
      <div
        class="max-h-[80vh] overflow-hidden flex items-center justify-center"
        :class="zoom > 1 ? (dragging ? 'cursor-grabbing' : 'cursor-grab') : ''"
        @wheel.prevent="emit('wheel', $event)"
        @pointermove="emit('pointerMove', $event)"
        @pointerup="emit('pointerUp', $event)"
        @pointercancel="emit('pointerUp', $event)"
        @pointerleave="emit('pointerUp', $event)"
      >
        <img
          v-if="dataUrl"
          :src="dataUrl"
          class="max-h-[80vh] max-w-full object-contain rounded select-none"
          :style="{ transform: `translate(${offsetX}px, ${offsetY}px) scale(${zoom})`, transformOrigin: 'center center' }"
          @pointerdown="emit('pointerDown', $event)"
        />
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('close')">close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import { Minus, Plus } from "@lucide/vue";

defineProps<{
  open: boolean;
  dataUrl: string;
  zoom: number;
  minZoom: number;
  maxZoom: number;
  offsetX: number;
  offsetY: number;
  dragging: boolean;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "zoomIn"): void;
  (e: "zoomOut"): void;
  (e: "reset"): void;
  (e: "wheel", event: WheelEvent): void;
  (e: "pointerDown", event: PointerEvent): void;
  (e: "pointerMove", event: PointerEvent): void;
  (e: "pointerUp", event: PointerEvent): void;
}>();
</script>

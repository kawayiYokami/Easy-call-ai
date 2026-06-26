<template>
  <dialog class="modal" :class="{ 'modal-open': open }" @cancel.prevent="emit('close')">
    <div class="modal-box flex h-[90vh] max-h-[90vh] w-[90vw] max-w-[90vw] flex-col p-2 bg-base-100">
      <div class="mb-2 flex shrink-0 items-center justify-end gap-1">
        <button class="btn btn-xs" :disabled="zoom <= minZoom" @click="emit('zoomOut')">
          <Minus class="h-3 w-3" />
        </button>
        <button class="btn btn-xs" :disabled="zoom >= maxZoom" @click="emit('zoomIn')">
          <Plus class="h-3 w-3" />
        </button>
        <button class="btn btn-xs" :disabled="Math.abs(zoom - 1) < 0.001" @click="emit('reset')">
          {{ Math.round(zoom * 100) }}%
        </button>
        <template v-if="localPath">
          <button class="btn btn-xs" :disabled="copyStatus === 'doing'" @click="emit('copyImage', localPath)">
            <Copy class="h-3 w-3" />
          </button>
          <button class="btn btn-xs" :disabled="saveStatus === 'doing'" @click="emit('saveImage', localPath)">
            <Download class="h-3 w-3" />
          </button>
        </template>
        <button class="btn btn-xs btn-ghost btn-square" type="button" title="关闭" aria-label="关闭" @click="emit('close')">
          <X class="h-3.5 w-3.5" />
        </button>
      </div>
      <div
        class="flex min-h-0 flex-1 items-center justify-center overflow-hidden"
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
          class="max-h-full max-w-full object-contain rounded select-none"
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
import { Copy, Download, Minus, Plus, X } from "@lucide/vue";

defineProps<{
  open: boolean;
  dataUrl: string;
  zoom: number;
  minZoom: number;
  maxZoom: number;
  offsetX: number;
  offsetY: number;
  dragging: boolean;
  localPath?: string;
  copyStatus?: "idle" | "doing";
  saveStatus?: "idle" | "doing";
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
  (e: "copyImage", path: string): void;
  (e: "saveImage", path: string): void;
}>();
</script>

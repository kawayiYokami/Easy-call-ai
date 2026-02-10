<template>
  <div class="grid gap-2">
    <button class="btn btn-sm" @click="$emit('loadArchives')">刷新归档</button>
    <div class="grid grid-cols-1 gap-1 max-h-56 overflow-auto">
      <button v-for="a in archives" :key="a.archiveId" class="btn btn-sm justify-start" @click="$emit('selectArchive', a.archiveId)">
        {{ a.archivedAt }} · {{ a.title }}
      </button>
    </div>
    <div class="divider my-1">归档内容</div>
    <div class="max-h-80 overflow-auto space-y-2">
      <div v-for="m in archiveMessages" :key="m.id" class="text-xs border border-base-300 rounded p-2">
        <div class="font-semibold uppercase text-[11px]">{{ m.role }}</div>
        <div v-if="messageText(m)" class="whitespace-pre-wrap">{{ messageText(m) }}</div>
        <div v-if="messageImages(m).length > 0" class="mt-2 grid gap-1">
          <img
            v-for="(img, idx) in messageImages(m)"
            :key="`${img.mime}-${idx}`"
            :src="`data:${img.mime};base64,${img.bytesBase64}`"
            class="rounded max-h-32 object-contain bg-base-100/40"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { ArchiveSummary, ChatMessage, MessagePart } from "../types/app";

defineProps<{
  archives: ArchiveSummary[];
  archiveMessages: ChatMessage[];
  renderMessage: (msg: ChatMessage) => string;
}>();

defineEmits<{
  (e: "loadArchives"): void;
  (e: "selectArchive", archiveId: string): void;
}>();

function messageText(msg: ChatMessage): string {
  return msg.parts
    .filter((p): p is Extract<MessagePart, { type: "text" }> => p.type === "text")
    .map((p) => p.text)
    .join("\n")
    .trim();
}

function messageImages(msg: ChatMessage): Array<{ mime: string; bytesBase64: string }> {
  return msg.parts
    .filter((p): p is Extract<MessagePart, { type: "image" }> => p.type === "image")
    .map((p) => ({ mime: p.mime, bytesBase64: p.bytesBase64 }));
}
</script>

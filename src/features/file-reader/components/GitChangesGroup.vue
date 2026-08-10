<template>
  <div class="px-1">
    <div class="mb-0.5 flex h-6 items-center gap-1 px-1.5">
      <span class="min-w-0 flex-1 truncate text-xs font-medium opacity-70">{{ title }}</span>
      <span class="shrink-0 text-[11px] tabular-nums opacity-50">{{ entries.length }}</span>
      <button
        v-if="entries.length > 0"
        type="button"
        class="btn btn-ghost btn-xs h-5 min-h-5 px-1.5 text-[11px]"
        :disabled="busy"
        @click="emit('action', entries.map((entry) => entry.path))"
      >
        {{ actionTitle }}
      </button>
    </div>
    <div v-for="entry in entries" :key="entry.path" class="group flex h-7 items-center gap-1 rounded px-1.5 hover:bg-base-300/40">
      <button
        type="button"
        class="flex min-w-0 flex-1 items-center gap-1.5 text-left text-xs"
        :title="entry.path"
        @click="emit('openDiff', { path: entry.path, staged: actionKind === 'unstage' })"
      >
        <span
          class="shrink-0 font-mono text-[10px] font-bold"
          :class="statusClass(entry)"
        >{{ statusLabel(entry) }}</span>
        <span class="min-w-0 truncate">{{ entry.path }}</span>
      </button>
      <span class="hidden shrink-0 items-center gap-0.5 group-hover:flex">
        <button
          type="button"
          class="btn btn-ghost btn-xs h-5 min-h-5 w-5 px-0"
          :title="actionTitle"
          :disabled="busy"
          @click="emit('action', [entry.path])"
        >
          <Plus v-if="actionKind === 'stage'" class="h-3 w-3" />
          <Minus v-else class="h-3 w-3" />
        </button>
        <button
          type="button"
          class="btn btn-ghost btn-xs h-5 min-h-5 w-5 px-0 text-error/70"
          :title="discardTitle"
          :disabled="busy"
          @click="emit('discard', [entry.path])"
        >
          <Trash2 class="h-3 w-3" />
        </button>
      </span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Minus, Plus, Trash2 } from "@lucide/vue";
import type { GitPanelStatusEntry } from "../../../services/tauri-api";

const props = withDefaults(defineProps<{
  title: string;
  entries: GitPanelStatusEntry[];
  busy?: boolean;
  actionKind: "stage" | "unstage";
  actionTitle: string;
  discardTitle: string;
}>(), {
  busy: false,
});

const emit = defineEmits<{
  (e: "openDiff", payload: { path: string; staged: boolean }): void;
  (e: "action", paths: string[]): void;
  (e: "discard", paths: string[]): void;
}>();

function statusLabel(entry: GitPanelStatusEntry) {
  const staged = entry.stagedStatus.trim();
  const unstaged = entry.unstagedStatus.trim();
  if (staged === "?" && unstaged === "?") return "U";
  const code = props.actionKind === "unstage" ? staged : unstaged;
  return code || "M";
}

function statusClass(entry: GitPanelStatusEntry) {
  const label = statusLabel(entry);
  if (label === "A" || label === "U") return "text-success";
  if (label === "D") return "text-error";
  return "text-warning";
}
</script>

<template>
  <dialog
    ref="dialogRef"
    class="modal"
    :open="open"
    @close="onDialogClose"
    @cancel.prevent="onDialogClose"
    @keydown.esc.prevent="onDialogClose"
  >
    <div
      class="modal-box flex w-[clamp(360px,68vw,640px)] max-w-none flex-col overflow-hidden p-0"
      :style="{
        height: 'clamp(420px, 68vh, 560px)',
        maxHeight: 'min(82vh, 560px)',
      }"
    >
      <div class="border-b border-base-300 px-4 py-3">
        <div class="text-sm font-semibold">{{ title }}</div>
        <div v-if="hint" class="mt-1 text-xs opacity-70">{{ hint }}</div>
      </div>

      <div class="flex min-h-0 flex-1 flex-col gap-3 px-4 py-3">
        <div class="flex flex-wrap items-center gap-3">
          <label class="flex cursor-pointer items-center gap-1.5 text-xs">
            <input v-model="filterHidden" type="checkbox" class="checkbox checkbox-primary checkbox-xs" />
            <span>过滤隐藏目录</span>
          </label>
          <label class="flex cursor-pointer items-center gap-1.5 text-xs">
            <input v-model="filterGit" type="checkbox" class="checkbox checkbox-primary checkbox-xs" />
            <span>过滤 .git 目录</span>
          </label>
          <span class="ml-auto text-xs opacity-60">{{ filteredDirectories.length }} 项</span>
        </div>

        <div class="flex items-center gap-2">
          <button
            type="button"
            class="btn btn-xs"
            :disabled="loading || !parentPath"
            @click="onUp"
          >
            上一级
          </button>
          <BreadcrumbAddressBar
            :path="currentPath"
            :switchable="props.switchable"
            @navigate="onBreadcrumb"
            @submit="onAddressSubmit"
          />
          <button
            type="button"
            class="btn btn-xs btn-ghost"
            :disabled="loading"
            @click="onRefresh"
          >
            刷新
          </button>
        </div>

        <div class="flex min-h-0 flex-1 rounded-box border border-base-300 bg-base-200/30">
          <OverlayScrollArea class="min-h-0 flex-1" scroller-class="h-full" orientation="vertical">
            <div class="py-1">
              <div v-if="loading" class="flex items-center gap-2 px-3 py-3 text-sm opacity-65">
                <span class="loading loading-spinner loading-xs"></span>
                正在读取目录
              </div>
              <div v-else-if="errorText" class="px-3 py-3 text-sm text-error">{{ errorText }}</div>
              <div v-else-if="filteredDirectories.length === 0" class="px-3 py-3 text-sm opacity-55">
                {{ currentPath ? '当前目录没有子目录' : '没有可用驱动器' }}
              </div>
              <template v-else>
                <button
                  v-for="item in filteredDirectories"
                  :key="item.path"
                  type="button"
                  class="flex min-h-8 w-full items-center gap-2 px-3 py-1.5 text-left text-sm hover:bg-base-300/60"
                  :title="item.path"
                  @click="onEntryClick(item.path)"
                >
                  <span class="shrink-0 opacity-55">📁</span>
                  <span class="min-w-0 flex-1 truncate">{{ item.name }}</span>
                  <span class="shrink-0 font-mono text-xs opacity-40 truncate max-w-[12rem]">{{ item.path }}</span>
                </button>
              </template>
            </div>
          </OverlayScrollArea>
        </div>

        <div class="text-xs opacity-60">
          已选：<span class="font-mono break-all">{{ currentPath || '（空，将使用助理空间）' }}</span>
        </div>
      </div>

      <div class="flex shrink-0 items-center justify-end gap-2 border-t border-base-300 px-4 py-3">
        <button class="btn btn-sm btn-ghost" type="button" @click="onDialogClose">取消</button>
        <button
          class="btn btn-sm btn-primary"
          type="button"
          :disabled="!canConfirm"
          @click="onConfirm"
        >
          {{ confirmLabel }}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="onDialogClose">close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import OverlayScrollArea from "./OverlayScrollArea.vue";
import BreadcrumbAddressBar from "./BreadcrumbAddressBar.vue";
import { invokeTauri } from "../../../services/tauri-api";

type DirectoryItem = {
  path: string;
  name: string;
};

const props = withDefaults(
  defineProps<{
    open: boolean;
    initialPath?: string;
    title?: string;
    hint?: string;
    confirmLabel?: string;
    switchable?: boolean;
  }>(),
  {
    initialPath: "",
    title: "选择目录",
    hint: "仅可选择目录，空选择将回退到助理空间",
    confirmLabel: "选择此目录",
    switchable: true,
  },
);

const emit = defineEmits<{
  (e: "close"): void;
  (e: "select", path: string): void;
}>();

const dialogRef = ref<HTMLDialogElement | null>(null);

const currentPath = ref("");
const directories = ref<DirectoryItem[]>([]);
const loading = ref(false);
const errorText = ref("");
const filterHidden = ref(true);
const filterGit = ref(true);

let seq = 0;

const canConfirm = computed(() => {
  const p = String(currentPath.value || "").trim();
  if (!p) return false;
  return true;
});

const filteredDirectories = computed(() => {
  const list = directories.value || [];
  return list.filter((item) => {
    const name = String(item.name || "").trim();
    if (!name) return false;
    if (filterGit.value && name === ".git") return false;
    if (filterHidden.value && name.startsWith(".")) return false;
    return true;
  });
});

const parentPath = computed(() => {
  const normalized = String(currentPath.value || "").trim().replace(/[\\/]+$/, "");
  if (!normalized) return "";
  if (/^[A-Za-z]:\/?$/.test(normalized) || normalized === "/") return "";
  const lastSlash = Math.max(normalized.lastIndexOf("/"), normalized.lastIndexOf("\\"));
  if (lastSlash < 0) return "";
  if (lastSlash === 0) return normalized.slice(0, 1);
  const candidate = normalized.slice(0, lastSlash);
  if (/^[A-Za-z]:$/.test(candidate)) return `${candidate}/`;
  if (candidate === "") return "/";
  if (/^[A-Za-z]:\/$/.test(normalized.slice(0, lastSlash + 1))) {
    return normalized.slice(0, lastSlash + 1);
  }
  return candidate;
});

function onDialogClose() {
  if (loading.value) return;
  emit("close");
}

async function loadDirectory(target: string) {
  const nextSeq = ++seq;
  const raw = String(target || "").trim();
  loading.value = true;
  errorText.value = "";
  try {
    const payload = await invokeTauri<{ path: string; name: string; directories: DirectoryItem[]; entries?: DirectoryItem[] }>(
      "workspace.directory.list",
      { path: raw },
    );
    if (nextSeq !== seq) return;
    const resolvedPath = String((payload as unknown as { path: string })?.path || raw || "").trim();
    const rawEntries = Array.isArray((payload as unknown as { directories: unknown })?.directories)
      ? (payload as unknown as { directories: DirectoryItem[] }).directories
      : Array.isArray((payload as unknown as { entries: unknown })?.entries)
        ? (payload as unknown as { entries: DirectoryItem[] }).entries.filter((e) => (e as DirectoryItem).path)
        : [];
    const dirs = rawEntries
      .map((e) => ({
        path: String(e.path || "").trim().replace(/\\/g, "/"),
        name: String(e.name || "").trim(),
      }))
      .filter((e) => e.path && e.name);
    directories.value = dirs;
    if (resolvedPath) {
      currentPath.value = resolvedPath.replace(/\\/g, "/");
    } else {
      currentPath.value = "";
    }
  } catch (error) {
    if (nextSeq !== seq) return;
    const message = error instanceof Error ? error.message : String(error);
    errorText.value = message || "读取目录失败";
    directories.value = [];
  } finally {
    if (nextSeq === seq) loading.value = false;
  }
}

function onEntryClick(path: string) {
  const normalized = String(path || "").trim();
  if (!normalized) return;
  void loadDirectory(normalized);
}

function onBreadcrumb(path: string) {
  const normalized = String(path || "").trim();
  if (!normalized) return;
  void loadDirectory(normalized);
}

function onUp() {
  const p = parentPath.value;
  if (!p) return;
  void loadDirectory(p);
}

function onRefresh() {
  void loadDirectory(currentPath.value || "");
}

function onAddressSubmit(path: string) {
  const normalized = String(path || "").trim();
  void loadDirectory(normalized);
}

function onConfirm() {
  const p = String(currentPath.value || "").trim();
  if (!p) return;
  emit("select", p);
}

watch(
  () => props.open,
  (open) => {
    if (open) {
      const init = String(props.initialPath || "").trim();
      currentPath.value = init;
      directories.value = [];
      errorText.value = "";
      void loadDirectory(init);
    } else {
      seq += 1;
    }
  },
  { immediate: true },
);

watch(
  () => props.initialPath,
  (next) => {
    if (props.open) {
      const normalized = String(next || "").trim();
      if (normalized !== String(currentPath.value || "").trim()) {
        void loadDirectory(normalized);
      }
    }
  },
);
</script>

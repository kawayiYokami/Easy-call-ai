<template>
  <div class="flex h-screen min-h-0 flex-col bg-base-100 text-base-content">
    <header class="flex h-10 shrink-0 items-center gap-2 bg-base-200 px-3" data-tauri-drag-region>
      <span class="pointer-events-none text-sm font-semibold opacity-80">运行日志</span>
      <span class="pointer-events-none text-xs opacity-50">内存 · 进程退出即清空</span>
      <div class="flex-1" data-tauri-drag-region />
      <div class="flex items-center gap-1 text-xs">
        <label class="flex items-center gap-1">
          <span class="opacity-60">级别</span>
          <select v-model="selectedLevel" class="select select-bordered select-xs w-24">
            <option value="all">全部</option>
            <option v-for="level in levelOptions" :key="level" :value="level">{{ level.toUpperCase() }}</option>
          </select>
        </label>
        <label class="flex items-center gap-1">
          <span class="opacity-60">模块</span>
          <select v-model="selectedModule" class="select select-bordered select-xs w-36">
            <option value="all">全部</option>
            <option v-for="m in moduleOptions" :key="m" :value="m">{{ m }}</option>
          </select>
        </label>
        <button class="btn btn-ghost btn-xs" title="复制" :disabled="filteredLogs.length === 0" @click="copyLogs">复制</button>
        <button class="btn btn-ghost btn-xs" title="清空" :disabled="logs.length === 0" @click="clearLogs">清空</button>
        <label class="flex items-center gap-1 cursor-pointer">
          <input v-model="autoScroll" type="checkbox" class="checkbox checkbox-xs" />
          <span class="opacity-60">自动滚动</span>
        </label>
      </div>
      <button class="btn btn-ghost btn-xs" title="最小化" @click="minimizeWindow">
        <span class="text-sm">─</span>
      </button>
      <button class="btn btn-ghost btn-xs hover:bg-error" title="关闭" @click="closeWindow">
        <span class="text-sm">✕</span>
      </button>
    </header>

    <pre
      ref="logContainer"
      class="flex-1 overflow-auto border-t border-base-300 bg-base-100 p-3 font-mono text-xs leading-5 whitespace-pre-wrap break-words"
    ><code v-if="filteredLogs.length === 0" class="opacity-50">{{ loading ? '正在加载...' : '暂无日志' }}</code><code v-else>{{ renderedText }}</code></pre>

    <footer class="flex h-6 shrink-0 items-center gap-2 border-t border-base-300 bg-base-200 px-3 text-xs opacity-60">
      <span>显示 {{ filteredLogs.length }} / {{ logs.length }}</span>
      <span v-if="errorText" class="text-error">{{ errorText }}</span>
    </footer>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { useAppTheme } from "../../features/shell/composables/use-app-theme";
import type { AppThemeState, GeneratedThemeControls } from "../../features/shell/theme/theme-types";
import { buildGeneratedThemeStyleText, generateGeneratedThemeTokens, GENERATED_THEME_NAME } from "../../features/shell/theme/theme-generator";

type RuntimeLogEntry = {
  id: string;
  createdAt: string;
  level: string;
  message: string;
  repeat: number;
};

const POLL_INTERVAL_MS = 100;
const levelOptions = ["info", "warn", "error", "debug", "trace"] as const;

const logs = ref<RuntimeLogEntry[]>([]);
const loading = ref(false);
const errorText = ref("");
const selectedLevel = ref<"all" | string>("info");
const selectedModule = ref("all");
const autoScroll = ref(true);
const logContainer = ref<HTMLElement | null>(null);

let pollTimer: ReturnType<typeof setInterval> | null = null;
let lastCreatedAt = "";
let unlistenTheme: UnlistenFn | null = null;

const { restoreThemeFromStorage } = useAppTheme();

const moduleOptions = computed(() => {
  const set = new Set<string>();
  for (const item of logs.value) {
    const m = extractModule(item.message);
    if (m) set.add(m);
  }
  return Array.from(set).sort((a, b) => a.localeCompare(b, "zh-CN"));
});

const filteredLogs = computed(() =>
  logs.value.filter((item) => {
    if (selectedLevel.value !== "all" && item.level !== selectedLevel.value) return false;
    if (selectedModule.value !== "all" && extractModule(item.message) !== selectedModule.value) return false;
    return true;
  }),
);

const renderedText = computed(() => filteredLogs.value.map(formatLine).join("\n"));

watch(filteredLogs, () => {
  if (autoScroll.value) {
    nextTick(() => {
      const el = logContainer.value;
      if (el) el.scrollTop = el.scrollHeight;
    });
  }
});

onMounted(async () => {
  restoreThemeFromStorage();
  try {
    unlistenTheme = await listen<AppThemeState>("easy-call:theme-changed", (event) => {
      const state = event.payload;
      if (state.kind === "preset" && state.name) {
        document.documentElement.setAttribute("data-theme", state.name);
      } else if (state.kind === "generated" && state.controls) {
        const tokens = generateGeneratedThemeTokens(state.controls as GeneratedThemeControls);
        const styleText = buildGeneratedThemeStyleText(tokens);
        let existing = document.getElementById("easy-call-generated-theme-style");
        if (existing instanceof HTMLStyleElement) {
          existing.textContent = styleText;
        } else {
          existing = document.createElement("style");
          existing.id = "easy-call-generated-theme-style";
          existing.textContent = styleText;
          document.head.appendChild(existing);
        }
        document.documentElement.setAttribute("data-theme", GENERATED_THEME_NAME);
      }
    });
  } catch (err) {
    console.error("[运行日志窗口] 监听主题变化失败", err);
    errorText.value = `监听主题变化失败：${String(err)}`;
  }
  await loadInitial();
  startPolling();
});

onBeforeUnmount(() => {
  stopPolling();
  unlistenTheme?.();
});

async function loadInitial() {
  loading.value = true;
  try {
    await invoke("append_runtime_log_probe", { message: "运行日志窗口已打开" });
  } catch {
    // ignore
  }
  try {
    const items = await invoke<RuntimeLogEntry[]>("list_recent_runtime_logs");
    logs.value = items;
    if (items.length > 0) {
      lastCreatedAt = items[items.length - 1].createdAt;
    }
  } catch (err) {
    errorText.value = String(err);
  } finally {
    loading.value = false;
  }
}

function startPolling() {
  stopPolling();
  pollTimer = setInterval(pollIncremental, POLL_INTERVAL_MS);
}

function stopPolling() {
  if (pollTimer) {
    clearInterval(pollTimer);
    pollTimer = null;
  }
}

async function pollIncremental() {
  try {
    const items = await invoke<RuntimeLogEntry[]>("list_runtime_logs_since", {
      sinceCreatedAt: lastCreatedAt,
    });
    if (items.length > 0) {
      logs.value = [...logs.value, ...items];
      lastCreatedAt = items[items.length - 1].createdAt;
      errorText.value = "";
    }
  } catch (err) {
    errorText.value = String(err);
  }
}

async function clearLogs() {
  try {
    await invoke("clear_recent_runtime_logs");
    logs.value = [];
    lastCreatedAt = "";
    errorText.value = "";
  } catch (err) {
    errorText.value = String(err);
  }
}

async function copyLogs() {
  const text = filteredLogs.value.map(formatLine).join("\n");
  if (!text) return;
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    errorText.value = "复制失败";
  }
}

function minimizeWindow() {
  getCurrentWindow().minimize();
}

function closeWindow() {
  getCurrentWindow().hide();
}

function extractModule(message: string): string | null {
  if (!message.startsWith("[")) return null;
  const m = message.match(/^\[([^\]]+)\]/);
  return m?.[1]?.trim() || null;
}

function formatLine(item: RuntimeLogEntry): string {
  const time = formatTime(item.createdAt);
  const parts = [`[${time}]`, item.level.toUpperCase(), item.message];
  if (item.repeat > 1) parts.push(`x${item.repeat}`);
  return parts.join(" ");
}

function formatTime(value: string): string {
  const d = new Date(value);
  if (Number.isNaN(d.getTime())) return value.replace("T", " ").replace(/(\.\d+)?Z?$/, "");
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}
</script>

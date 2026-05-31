<template>
  <div class="relative flex h-screen min-h-0 flex-col bg-base-100 text-base-content">
    <header class="flex h-10 shrink-0 items-end gap-2 bg-base-200 px-2" data-tauri-drag-region>
      <button class="btn btn-ghost btn-sm shrink-0" type="button" title="打开文件" @click.stop="pickFile">
        <FilePlus class="h-4 w-4" />
      </button>
      <div class="flex min-w-0 flex-1 items-end gap-1 overflow-hidden" data-tauri-drag-region>
        <div
          v-for="tab in fileReaderPanelRef?.tabs || []"
          :key="tab.path"
          class="group flex h-9 min-w-0 max-w-64 flex-1 basis-0 items-center gap-2 overflow-hidden rounded-t-box border border-b-0 px-2 text-sm"
          :class="tab.path === fileReaderPanelRef?.activePath ? 'border-base-300 bg-base-100 text-base-content' : 'border-transparent bg-base-200 text-base-content/65 hover:bg-base-100/70 hover:text-base-content'"
          :title="tab.path"
          role="button"
          tabindex="0"
          :aria-selected="tab.path === fileReaderPanelRef?.activePath"
          @click="fileReaderPanelRef?.setActiveTab(tab.path)"
          @keydown.enter.prevent="fileReaderPanelRef?.setActiveTab(tab.path)"
          @keydown.space.prevent="fileReaderPanelRef?.setActiveTab(tab.path)"
        >
          <FileText class="h-4 w-4 shrink-0 opacity-70" />
          <span class="min-w-0 flex-1 truncate font-medium">{{ tab.title }}</span>
          <button
            type="button"
            class="btn btn-ghost btn-xs h-5 min-h-5 w-5 p-0 opacity-60 hover:opacity-100"
            title="关闭"
            @click.stop="fileReaderPanelRef?.closeTab(tab.path)"
          >
            <X class="h-3.5 w-3.5" />
          </button>
        </div>
      </div>
      <button class="btn btn-ghost btn-sm shrink-0" type="button" title="最小化" @click.stop="minimizeWindow">
        <Minus class="h-3.5 w-3.5" />
      </button>
      <button class="btn btn-ghost btn-sm shrink-0" type="button" :title="maximized ? '还原窗口' : '最大化'" @click.stop="toggleMaximizeWindow">
        <Square class="h-3.5 w-3.5" />
      </button>
      <button class="btn btn-sm btn-ghost shrink-0 hover:bg-error" type="button" title="关闭" @click.stop="closeWindow">
        <X class="h-3.5 w-3.5" />
      </button>
    </header>

    <FileReaderPanel
      ref="fileReaderPanelRef"
      class="min-h-0 flex-1"
      :show-tabs="false"
      :show-pick-file-button="false"
      :markdown-is-dark="markdownIsDark"
      custom-markstream-id="file-reader-markstream"
    />

    <Win10ResizeHandles :enabled="!maximized" />
  </div>
</template>

<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { FilePlus, FileText, Minus, Square, X } from "@lucide/vue";
import { open } from "@tauri-apps/plugin-dialog";
import FileReaderPanel from "../../features/file-reader/components/FileReaderPanel.vue";
import { type AppThemeState, type GeneratedThemeControls } from "../../features/shell/theme/theme-types";
import { useAppTheme } from "../../features/shell/composables/use-app-theme";
import { buildGeneratedThemeStyleText, generateGeneratedThemeTokens, GENERATED_THEME_NAME } from "../../features/shell/theme/theme-generator";
import Win10ResizeHandles from "../../features/shell/components/Win10ResizeHandles.vue";

const FILE_READER_SESSION_STORAGE_KEY = "easy_call.file_reader_session.v1";
const LEGACY_FILE_READER_SESSION_STORAGE_KEY = "easy-call:file-reader-session:v1";

type FileReaderSessionState = {
  tabs?: string[];
  activePath?: string;
  directoryRootPath?: string;
};

const { currentTheme, restoreThemeFromStorage } = useAppTheme();
const appWindow = getCurrentWindow();
const maximized = ref(false);
const fileReaderPanelRef = ref<InstanceType<typeof FileReaderPanel> | null>(null);
const markdownIsDark = ref(currentTheme.value !== "light");

let unlistenOpenPath: UnlistenFn | null = null;
let unlistenThemeChanged: UnlistenFn | null = null;

function normalizePath(path: string) {
  return String(path || "")
    .trim()
    .replace(/^\\\\\?\\/, "")
    .replace(/^\/\/\?\//, "")
    .replace(/^\/\?\//, "")
    .replace(/^\?\//, "")
    .replace(/^\?\\/, "")
    .replace(/\\/g, "/");
}

function readFileReaderSessionState(): FileReaderSessionState {
  try {
    return JSON.parse(localStorage.getItem(FILE_READER_SESSION_STORAGE_KEY) || localStorage.getItem(LEGACY_FILE_READER_SESSION_STORAGE_KEY) || "{}") as FileReaderSessionState;
  } catch {
    return {};
  }
}

function persistFileReaderSession() {
  const panel = fileReaderPanelRef.value;
  if (!panel) return;
  const uniqueTabs = Array.from(new Set(panel.tabs.map((tab) => normalizePath(tab.path)).filter(Boolean)));
  const state: FileReaderSessionState = {
    tabs: uniqueTabs,
    activePath: normalizePath(panel.activePath),
    directoryRootPath: normalizePath(panel.directoryRootPath),
  };
  localStorage.setItem(FILE_READER_SESSION_STORAGE_KEY, JSON.stringify(state));
}

async function restoreFileReaderSession(loadActiveTab: boolean) {
  const panel = fileReaderPanelRef.value;
  if (!panel) return;
  const state = readFileReaderSessionState();
  const restoredTabs = Array.from(new Set((state.tabs || []).map((path) => normalizePath(path)).filter(Boolean)));

  for (const path of restoredTabs) {
    await panel.openPath(path);
  }

  if (restoredTabs.length > 0) {
    const restoredActivePath = normalizePath(state.activePath || "");
    panel.setActiveTab(restoredTabs.includes(restoredActivePath) ? restoredActivePath : restoredTabs[0]);
  }

  const restoredDirectoryRoot = normalizePath(state.directoryRootPath || "");
  if (restoredDirectoryRoot) {
    await panel.openDirectoryTree(restoredDirectoryRoot);
  }
}

async function pickFile() {
  const picked = await open({ multiple: false, directory: false, title: "打开文件" });
  if (!picked || Array.isArray(picked)) return;
  await fileReaderPanelRef.value?.openPath(String(picked));
}

async function syncWindowState() {
  try {
    maximized.value = await appWindow.isMaximized();
  } catch {
    maximized.value = false;
  }
}

async function minimizeWindow() {
  await appWindow.minimize();
}

async function toggleMaximizeWindow() {
  await appWindow.toggleMaximize();
  await syncWindowState();
}

async function closeWindow() {
  persistFileReaderSession();
  await appWindow.hide();
}

onMounted(async () => {
  restoreThemeFromStorage();
  try {
    unlistenThemeChanged = await listen<AppThemeState>("easy-call:theme-changed", (event) => {
      const state = event.payload;
      if (state.kind === "preset" && state.name) {
        document.documentElement.setAttribute("data-theme", state.name);
      } else if (state.kind === "generated" && state.controls) {
        const tokens = generateGeneratedThemeTokens(state.controls as GeneratedThemeControls);
        const styleText = buildGeneratedThemeStyleText(tokens);
        const existing = document.getElementById("easy-call-generated-theme-style");
        if (existing instanceof HTMLStyleElement) {
          existing.textContent = styleText;
        } else {
          const element = document.createElement("style");
          element.id = "easy-call-generated-theme-style";
          element.textContent = styleText;
          document.head.appendChild(element);
        }
        document.documentElement.setAttribute("data-theme", GENERATED_THEME_NAME);
      }
      markdownIsDark.value = currentTheme.value !== "light";
    });
  } catch (error) {
    console.error("[文件阅读窗口] 监听主题变化失败", error);
  }
  void syncWindowState();
  const path = new URLSearchParams(window.location.search).get("path") || "";
  await restoreFileReaderSession(!path);
  if (path) {
    void fileReaderPanelRef.value?.openPath(path);
  }
  try {
    unlistenOpenPath = await listen<{ path?: string }>("file-reader-open-path", (event) => {
      void fileReaderPanelRef.value?.openPath(event.payload?.path || "");
    });
  } catch (error) {
    console.error("[文件阅读窗口] 监听打开文件事件失败", error);
  }
});

onBeforeUnmount(() => {
  unlistenOpenPath?.();
  unlistenThemeChanged?.();
});
</script>

<template>
  <div class="relative flex h-full min-h-0 flex-col bg-base-100 text-base-content">
    <div
      v-if="showTabs"
      class="flex h-8 shrink-0 items-stretch overflow-hidden bg-base-200 px-2"
    >
      <button
        v-if="showPickFileButton"
        class="btn btn-ghost btn-sm shrink-0"
        type="button"
        :title="t('fileReader.openFile')"
        @click.stop="pickFile"
      >
        <FilePlus class="h-4 w-4" />
      </button>
      <div class="flex min-w-0 flex-1 items-stretch overflow-hidden">
        <div
          v-for="tab in tabs"
          :key="tab.path"
          class="group flex min-w-24 max-w-56 flex-none items-center gap-2 overflow-hidden border border-b-0 px-2 text-sm"
          :class="tab.path === activePath ? 'border-base-300 bg-base-100 text-base-content' : 'border-transparent bg-base-200 text-base-content/65 hover:bg-base-100/70 hover:text-base-content'"
          :title="tab.path"
          role="button"
          tabindex="0"
          :aria-selected="tab.path === activePath"
          @click="setActiveTab(tab.path)"
          @keydown.enter.prevent="setActiveTab(tab.path)"
          @keydown.space.prevent="setActiveTab(tab.path)"
        >
          <FileText class="h-4 w-4 shrink-0 opacity-70" />
          <span class="min-w-0 flex-1 truncate font-medium">{{ tab.title }}</span>
          <button
            type="button"
            class="btn btn-ghost btn-xs h-5 min-h-5 w-5 p-0 opacity-60 hover:opacity-100"
            :title="t('fileReader.close')"
            @click.stop="closeTab(tab.path)"
          >
            <X class="h-3.5 w-3.5" />
          </button>
        </div>
      </div>
      <button
        type="button"
        class="btn btn-ghost btn-sm h-8 min-h-8 w-8 shrink-0 px-0"
        :disabled="!directoryToggleTargetPath"
        :title="directoryTreeRoot ? t('fileReader.collapseTree') : t('fileReader.expandTree', { path: directoryToggleTargetPath })"
        @click="toggleDirectoryTree"
      >
        <ListIndentIncrease v-if="directoryTreeRoot" class="h-4 w-4" />
        <ListIndentDecrease v-else class="h-4 w-4" />
      </button>
    </div>

    <div class="flex min-h-0 flex-1" :class="directoryOnly ? '' : 'flex-row-reverse'">
      <aside
        v-if="directoryTreeRoot"
        class="flex shrink-0 flex-col border-base-300 bg-base-200/35"
        :class="directoryOnly ? 'w-full border-r-0' : 'w-64 border-l'"
      >
        <div class="flex h-8 shrink-0 items-center gap-1.5 border-b border-base-300 px-3 text-sm">
          <button
            type="button"
            class="btn btn-ghost btn-xs min-w-0 flex-1 truncate justify-start font-medium"
            :title="directoryTreeRoot.path"
            @click="openDirectoryInFileManager(directoryTreeRoot.path)"
          >{{ directoryTreeRoot.name }}</button>
          <button
            class="btn btn-ghost btn-xs h-7 min-h-7 w-7 shrink-0 px-0"
            type="button"
            :disabled="directoryTreeRoot.loading"
            :title="t('fileReader.openShellHere')"
            @click="openShellAtDirectoryTreeRoot"
          >
            <SquareTerminal class="h-4 w-4" />
          </button>
          <button
            class="btn btn-ghost btn-xs h-7 min-h-7 w-7 shrink-0 px-0"
            type="button"
            :class="directoryTreeSearchVisible ? 'text-primary' : ''"
            :title="t('fileReader.toggleSearch')"
            @click="toggleDirectoryTreeSearch"
          >
            <Search class="h-4 w-4" />
          </button>
        </div>
        <div v-if="directoryTreeSearchVisible" class="border-b border-base-300 p-2">
          <input
            v-model="directoryTreeFilter"
            class="input input-bordered input-xs w-full"
            type="search"
            :placeholder="t('fileReader.filterFiles')"
          />
        </div>
        <div class="relative min-h-0 flex-1" @mouseenter="directoryScrollbarRef?.reveal()" @mouseleave="directoryScrollbarRef?.hide()">
        <div ref="directoryScroller" class="file-reader-scroll-container min-h-0 h-full overflow-auto py-1 text-sm">
          <div v-if="directoryTreeRoot.loading" class="flex items-center gap-2 px-3 py-2 text-xs opacity-65">
            <span class="loading loading-spinner loading-xs"></span>
            {{ t('fileReader.loadingDirectory') }}
          </div>
          <div v-else-if="directoryTreeRoot.error" class="px-3 py-2 text-xs text-error">
            {{ directoryTreeRoot.error }}
          </div>
          <div v-else-if="visibleTreeRows.length === 0" class="px-3 py-2 text-xs opacity-60">
            {{ directoryTreeFilter.trim() ? t('fileReader.noMatches') : t('fileReader.emptyDirectory') }}
          </div>
          <template v-else>
            <div
              v-for="row in visibleTreeRows"
              :key="row.key"
              class="flex h-7 items-center gap-1 px-2"
              :class="row.kind === 'entry' && !row.entry.isDirectory && normalizePath(row.entry.path) === activePath ? 'bg-primary/10 text-primary' : 'hover:bg-base-300/55'"
              :style="{ paddingLeft: `${8 + row.depth * 14}px` }"
            >
              <template v-if="row.kind === 'entry'">
                <button
                  v-if="row.entry.isDirectory"
                  class="btn btn-ghost btn-xs h-5 min-h-5 w-5 shrink-0 px-0"
                  type="button"
                  :title="isTreeDirectoryExpanded(row.entry.path) ? t('fileReader.collapseDirectory') : t('fileReader.expandDirectory')"
                  @click.stop="toggleTreeDirectory(row.entry)"
                >
                  <ChevronDown v-if="isTreeDirectoryExpanded(row.entry.path)" class="h-3.5 w-3.5" />
                  <ChevronRight v-else class="h-3.5 w-3.5" />
                </button>
                <span v-else class="h-5 w-5 shrink-0"></span>
                <button
                  type="button"
                  class="flex min-w-0 flex-1 items-center gap-1.5 rounded px-1 py-0.5 text-left"
                  :title="row.entry.path"
                  @click="handleTreeEntryClick(row.entry)"
                >
                  <Folder v-if="row.entry.isDirectory" class="h-4 w-4 shrink-0 text-warning" />
                  <FileText v-else class="h-4 w-4 shrink-0 opacity-70" />
                  <span class="min-w-0 truncate">{{ row.entry.name }}</span>
                </button>
              </template>
              <template v-else>
                <span class="h-5 w-5 shrink-0"></span>
                <span class="truncate px-1 text-xs opacity-60">{{ row.text }}</span>
              </template>
            </div>
          </template>
        </div>
        <FloatingScrollbar ref="directoryScrollbarRef" :target="directoryScroller" />
        </div>
      </aside>

      <main v-if="!directoryOnly" class="flex min-h-0 flex-1 flex-col overflow-hidden bg-base-100">
        <div
          v-if="activeTab"
          class="relative flex h-8 shrink-0 items-center gap-2 border-b border-base-300 bg-base-100 px-3 text-sm text-base-content/60"
        >
          <div class="relative min-w-0 flex-1">
            <div
              ref="addressScroller"
              class="file-reader-address-scroll flex min-w-0 items-center gap-1 overflow-x-auto overflow-y-hidden"
              @scroll="updateAddressScrollState"
              @wheel="handleAddressWheel"
            >
              <template v-for="segment in activePathSegments" :key="segment.key">
                <span v-if="segment.index > 0" class="shrink-0 text-base-content/35">›</span>
                <button
                  type="button"
                  class="inline-flex shrink-0 items-center rounded px-1.5 py-1 hover:bg-base-200 hover:text-base-content"
                  :title="t('fileReader.previewDirectory', { path: segment.path })"
                  @click="showHoverDirectoryTree(segment.path, $event)"
                  @mouseenter="showHoverDirectoryTree(segment.path, $event)"
                  @mouseleave="hideHoverDirectoryTree"
                >
                  {{ segment.label }}
                </button>
              </template>
              <span v-if="activePathSegments.length > 0" class="shrink-0 text-base-content/35">›</span>
              <span
                class="inline-flex shrink-0 items-center rounded px-1.5 py-1 font-medium text-base-content/80"
                :title="activeTab.path"
              >
                {{ activeTab.title }}
              </span>
            </div>
            <div
              v-if="addressScrollState.scrollable"
              class="pointer-events-none absolute inset-x-0 bottom-0 h-px"
            >
              <div
                class="file-reader-address-scrollbar-thumb h-px rounded-full bg-primary/55"
                :style="addressScrollbarThumbStyle"
              ></div>
            </div>
          </div>
          <button
            class="btn btn-ghost btn-xs h-6 min-h-6 w-6 shrink-0 px-0"
            type="button"
            :disabled="!activeTab"
            :title="t('fileReader.openWithDefault')"
            @click.stop="openWithDefaultProgram"
          >
            <ExternalLink class="h-4 w-4" />
          </button>
          <button class="btn btn-ghost btn-xs h-6 min-h-6 w-6 shrink-0 px-0" type="button" :disabled="!activeTab" :title="t('fileReader.refresh')" @click.stop="refreshActiveTab">
            <RefreshCw class="h-4 w-4" />
          </button>
          <button
            v-if="activeTab && canToggleRawMode(activeTab)"
            class="btn btn-ghost btn-xs h-6 min-h-6 w-6 shrink-0 px-0"
            type="button"
            :title="isTabRawMode(activeTab) ? t('fileReader.switchToRendered') : t('fileReader.switchToRaw')"
            :aria-label="isTabRawMode(activeTab) ? t('fileReader.currentRawView') : t('fileReader.currentRenderedView')"
            @click.stop="toggleActiveRawMode"
          >
            <Code2 v-if="isTabRawMode(activeTab)" class="h-4 w-4" />
            <Eye v-else class="h-4 w-4" />
          </button>
        </div>
        <div class="relative min-h-0 flex-1">
          <div
            ref="contentScroller"
            class="file-reader-content-stage h-full min-h-0 overflow-hidden"
          >
            <div v-if="!activeTab" class="flex h-full items-center justify-center text-sm text-base-content/55">
              <slot name="empty">
                <span>{{ t('fileReader.noFileOpen') }}</span>
              </slot>
            </div>
            <div v-else-if="activeTab.loading" class="flex h-full items-center justify-center gap-3 text-sm text-base-content/65">
              <span class="loading loading-spinner loading-sm"></span>
              {{ t('fileReader.loadingFile') }}
            </div>
            <div v-else-if="activeTab.error" class="m-4 rounded-box border border-error/30 bg-error/10 p-4 text-sm text-error">
              {{ activeTab.error }}
            </div>
            <div
              v-else-if="activeTab.kind === 'markdown' && !isTabRawMode(activeTab)"
              ref="markdownScroller"
              class="file-reader-content file-reader-markdown-scroller mx-auto h-full w-full max-w-300 overflow-auto px-4 py-4"
              @scroll="captureVisibleRangeContext"
              @mouseup="captureCurrentTextSelection"
              @keyup="captureCurrentTextSelection"
            >
              <AppMarkdownRenderer
                class="ecall-markdown-content max-w-none"
                :text="activeMarkdownSource"
                :is-dark="markdownIsDark"
                variant="document"
              />
            </div>
            <div
              v-else-if="activeTab.virtualized"
              class="relative h-full min-h-0"
              @mouseenter="!isTabRawMode(activeTab) && virtualCodeScrollbarRef?.reveal()"
              @mouseleave="!isTabRawMode(activeTab) && virtualCodeScrollbarRef?.hide()"
            >
              <div
                ref="virtualCodeScroller"
                class="file-reader-code-virtual-scroller h-full min-h-0 overflow-auto"
                :class="isTabRawMode(activeTab) ? 'file-reader-code-virtual-scroller-raw' : 'file-reader-code-virtual-scroller-shiki'"
                @scroll="captureVisibleRangeContext"
                @mouseup="captureCurrentTextSelection"
                @keyup="captureCurrentTextSelection"
              >
                <div class="file-reader-code-virtual-canvas" :style="{ height: `${activeVirtualCodeTotalSize}px` }">
                  <div
                    v-for="entry in activeVirtualCodeEntries"
                    :key="entry.block.key"
                    :data-index="entry.row.index"
                    :data-file-block-key="entry.block.key"
                    :data-start-line="entry.block.startLine"
                    :data-end-line="entry.block.endLine"
                    :ref="measureVirtualCodeRow"
                    class="file-reader-code-virtual-row"
                    :style="{
                      top: `${entry.row.start}px`,
                      '--file-reader-code-gutter-ch': String(virtualCodeLineNumberDigits),
                    }"
                  >
                    <div
                      class="file-reader-code-virtual-block"
                      :class="isTabRawMode(activeTab) ? 'file-reader-code-virtual-block-raw' : 'file-reader-code-virtual-block-shiki'"
                    >
                      <div
                        aria-hidden="true"
                        class="file-reader-code-virtual-gutter"
                        :class="isTabRawMode(activeTab) ? 'file-reader-code-virtual-gutter-raw' : 'file-reader-code-virtual-gutter-shiki'"
                      >
                        <div
                          v-for="lineNumber in blockLineNumbers(entry.block)"
                          :key="`${entry.block.key}-${lineNumber}`"
                          class="file-reader-code-virtual-gutter-line"
                        >
                          {{ lineNumber }}
                        </div>
                      </div>
                      <pre v-if="isTabRawMode(activeTab)" class="file-reader-raw-pre file-reader-code-virtual-raw">{{ blockContentText(entry.block.key) }}</pre>
                      <div
                        v-else
                        class="file-reader-code-virtual-shiki"
                        v-html="blockContentHtml(entry.block.key)"
                      ></div>
                    </div>
                  </div>
                </div>
              </div>
              <FloatingScrollbar
                v-if="!isTabRawMode(activeTab)"
                ref="virtualCodeScrollbarRef"
                :target="virtualCodeScroller"
                variant="code-dark"
              />
              <FloatingScrollbar
                v-if="!isTabRawMode(activeTab)"
                :target="virtualCodeScroller"
                variant="code-dark"
                orientation="horizontal"
              />
            </div>
          </div>
        </div>
      </main>
      <main v-else-if="!directoryTreeRoot" class="flex min-h-0 flex-1 items-center justify-center bg-base-100 px-4 text-center text-sm text-base-content/55">
        <slot name="empty">
          <span>{{ t('fileReader.noWorkspace') }}</span>
        </slot>
      </main>
    </div>

    <div
      v-if="fileDragActive"
      class="pointer-events-none fixed inset-0 z-40 flex items-center justify-center bg-base-100/70 backdrop-blur-[1px]"
    >
      <div class="rounded-box border border-primary/30 bg-base-100 px-5 py-3 text-sm font-medium text-primary shadow-lg">
        {{ t('fileReader.dropToOpen') }}
      </div>
    </div>

    <div v-if="actionErrorMessage" class="toast toast-end toast-bottom z-50">
      <div class="alert alert-error max-w-xl text-sm shadow-lg">
        <span>{{ actionErrorMessage }}</span>
      </div>
    </div>

    <Teleport to="body">
      <template v-if="hoverDirectoryTreeVisible">
        <div
          class="fixed z-1199"
          :style="hoverDirectoryBridgeStyle"
          @mouseenter="cancelHideHoverDirectoryTree"
          @mouseleave="hideHoverDirectoryTree"
        ></div>
        <div
          ref="hoverDirectoryTreeRef"
          class="fixed z-1200 flex flex-col overflow-hidden rounded-box border border-base-300 bg-base-100 shadow-xl"
          :style="hoverDirectoryTreeStyle"
          @mouseenter="cancelHideHoverDirectoryTree"
          @mouseleave="hideHoverDirectoryTree"
        >
          <div class="flex h-8 shrink-0 items-center gap-1.5 border-b border-base-300 bg-base-200 px-3 text-sm">
            <span class="flex-1 truncate font-medium">{{ hoverDirectoryTreeRoot?.name || "" }}</span>
          </div>
          <div ref="hoverDirectoryScroller" class="flex-1 overflow-auto py-1 text-sm">
            <div v-if="hoverDirectoryTreeRoot?.loading" class="flex items-center gap-2 px-3 py-2 text-xs opacity-65">
              <span class="loading loading-spinner loading-xs"></span>
              {{ t('fileReader.loadingDirectory') }}
            </div>
            <div v-else-if="hoverDirectoryTreeRoot?.error" class="px-3 py-2 text-xs text-error">
              {{ hoverDirectoryTreeRoot.error }}
            </div>
            <div v-else-if="hoverDirectoryTreeRows.length === 0" class="px-3 py-2 text-xs opacity-60">
              {{ t('fileReader.emptyDirectory') }}
            </div>
            <template v-else>
              <div
                v-for="row in hoverDirectoryTreeRows"
                :key="row.key"
                class="flex h-7 items-center gap-1 px-2"
                :class="row.kind === 'entry' && !row.entry.isDirectory ? 'hover:bg-base-300/55' : ''"
                :style="{ paddingLeft: `${8 + row.depth * 14}px` }"
              >
                <template v-if="row.kind === 'entry'">
                  <button
                    v-if="row.entry.isDirectory"
                    class="btn btn-ghost btn-xs h-5 min-h-5 w-5 shrink-0 px-0"
                    type="button"
                    :title="isHoverDirectoryExpanded(row.entry.path) ? t('fileReader.collapseDirectory') : t('fileReader.expandDirectory')"
                    @click.stop="toggleHoverDirectory(row.entry)"
                  >
                    <ChevronDown v-if="isHoverDirectoryExpanded(row.entry.path)" class="h-3.5 w-3.5" />
                    <ChevronRight v-else class="h-3.5 w-3.5" />
                  </button>
                  <span v-else class="h-5 w-5 shrink-0"></span>
                  <button
                    type="button"
                    class="flex min-w-0 flex-1 items-center gap-1.5 rounded px-1 py-0.5 text-left"
                    :title="row.entry.path"
                    @click.stop="openFileFromHoverTree(row.entry)"
                  >
                    <Folder v-if="row.entry.isDirectory" class="h-4 w-4 shrink-0 text-warning" />
                    <FileText v-else class="h-4 w-4 shrink-0 opacity-70" />
                    <span class="min-w-0 truncate">{{ row.entry.name }}</span>
                  </button>
                </template>
                <template v-else>
                  <span class="h-5 w-5 shrink-0"></span>
                  <span class="truncate px-1 text-xs opacity-60">{{ row.text }}</span>
                </template>
              </div>
            </template>
          </div>
        </div>
      </template>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useVirtualizer } from "@tanstack/vue-virtual";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { ChevronDown, ChevronRight, Code2, Eye, ExternalLink, FilePlus, FileText, Folder, ListIndentDecrease, ListIndentIncrease, RefreshCw, Search, SquareTerminal, X } from "@lucide/vue";
import { bundledLanguagesInfo, codeToHtml } from "shiki";
import { invokeTauri, isTauriRuntimeAvailable } from "../../../services/tauri-api";
import { AppMarkdownRenderer, initKatex } from "../../chat/markdown";
import FloatingScrollbar from "../../shell/components/FloatingScrollbar.vue";
import { useI18n } from "vue-i18n";
import type { IdeContextReferenceItem } from "../../../types/app";

const { t } = useI18n();

initKatex();

// ==================== Types ====================

type FileReaderFilePayload = {
  path: string;
  name: string;
  extension: string;
  kind: "markdown" | "code" | string;
  content: string;
  forcePlain?: boolean;
  virtualized?: boolean;
  totalLines?: number;
  blockLineCount?: number;
};

type FileReaderFileBlockPayload = {
  path: string;
  startLine: number;
  endLine: number;
  content: string;
};

type FileReaderDirectoryEntry = {
  path: string;
  name: string;
  isDirectory: boolean;
};

type FileReaderDirectoryPayload = {
  path: string;
  name: string;
  entries: FileReaderDirectoryEntry[];
};

type FileTab = {
  path: string;
  title: string;
  extension: string;
  kind: string;
  content: string;
  rawMode: boolean;
  forcePlain: boolean;
  virtualized: boolean;
  totalLines: number;
  blockLineCount: number;
  loaded: boolean;
  loading: boolean;
  error: string;
};

type VirtualCodeBlock = {
  key: string;
  path: string;
  startLine: number;
  endLine: number;
  lineCount: number;
};

type DirectoryNode = {
  path: string;
  name: string;
  entries: FileReaderDirectoryEntry[];
  loaded: boolean;
  loading: boolean;
  error: string;
  expanded: boolean;
};

type TreeRow =
  | { kind: "entry"; key: string; depth: number; entry: FileReaderDirectoryEntry }
  | { kind: "status"; key: string; depth: number; text: string };

type FileReaderSessionState = {
  tabs?: string[];
  activePath?: string;
  directoryRootPath?: string;
};

type FileReaderWatchTarget = {
  path: string;
  kind: "file" | "directory";
};

type FileReaderWatchEventPayload = {
  sessionId: string;
  path: string;
  kind: "file" | "directory";
};

// ==================== Props ====================

const props = withDefaults(defineProps<{
  initialRootPath?: string;
  initialOpenPath?: string;
  bridgeRequest?: <T = unknown>(method: string, params?: Record<string, unknown>, timeoutMs?: number) => Promise<T>;
  showTabs?: boolean;
  showPickFileButton?: boolean;
  directoryOnly?: boolean;
  enableGlobalDrop?: boolean;
  markdownIsDark?: boolean;
  customMarkstreamId?: string;
  sessionKey?: string;
  legacySessionKey?: string;
}>(), {
  showTabs: true,
  showPickFileButton: true,
  directoryOnly: false,
  enableGlobalDrop: true,
  markdownIsDark: false,
  customMarkstreamId: "file-reader-markstream",
  sessionKey: "",
  legacySessionKey: "",
});

const emit = defineEmits<{
  (e: "openPath", path: string): void;
  (e: "selectPath", path: string): void;
  (e: "captureContextReference", reference: IdeContextReferenceItem): void;
  (e: "clearSelectionContextReference"): void;
}>();

// ==================== Constants ====================

const SHIKI_LANGUAGE_KEYS = new Set(
  bundledLanguagesInfo.flatMap((item) => [item.id, ...(item.aliases || [])]).map((item) => item.toLowerCase()),
);

const CODE_LANGUAGE_BY_EXTENSION: Record<string, string> = {
  ts: "typescript", tsx: "tsx", c: "c", cc: "cpp", cpp: "cpp", cxx: "cpp",
  h: "c", hpp: "cpp", cs: "csharp", java: "java", kt: "kotlin", kts: "kotlin",
  go: "go", js: "javascript", jsx: "jsx", vue: "vue", rs: "rust", py: "python",
  rb: "ruby", php: "php", swift: "swift", scala: "scala", dart: "dart", lua: "lua",
  r: "r", m: "objective-c", mm: "objective-cpp", pl: "perl", pm: "perl",
  json: "json", jsonc: "jsonc", json5: "json5", toml: "toml", yaml: "yaml", yml: "yaml",
  css: "css", scss: "scss", sass: "sass", less: "less", html: "html", htm: "html",
  xml: "xml", svg: "xml", sql: "sql", sh: "bash", bash: "bash", zsh: "bash",
  fish: "fish", ps1: "powershell", bat: "bat", cmd: "bat", dockerfile: "dockerfile",
  ini: "ini", env: "dotenv", gitignore: "gitignore", gitattributes: "gitignore",
  editorconfig: "ini", lock: "text", csv: "csv", tsv: "tsv", txt: "text", log: "log",
  md: "markdown", markdown: "markdown", mdx: "mdx",
};

const CONTEXT_TEXT_BLOCK_CONTENT_LIMIT = 2000;
const FILE_READER_VIRTUAL_BLOCK_OVERSCAN = 6;
const FILE_READER_VIRTUAL_BLOCK_LINE_HEIGHT_PX = 24;
const FILE_READER_VIRTUAL_BLOCK_PADDING_Y_PX = 8;

// ==================== State ====================

const tabs = ref<FileTab[]>([]);
const activePath = ref("");
const actionErrorMessage = ref("");
const highlightedCodeHtmlByBlockKey = ref<Record<string, string>>({});
const fileBlockContentByKey = ref<Record<string, string>>({});
const fileBlockLoadingByKey = ref<Record<string, boolean>>({});
const fileBlockErrorByKey = ref<Record<string, string>>({});
const directoryRootPath = ref("");
const directoryTreeFilter = ref("");
const directoryTreeSearchVisible = ref(false);
const directoryNodes = ref<Record<string, DirectoryNode>>({});
const fileDragActive = ref(false);
const addressScroller = ref<HTMLElement | null>(null);
const contentScroller = ref<HTMLElement | null>(null);
const markdownScroller = ref<HTMLElement | null>(null);
const virtualCodeScroller = ref<HTMLElement | null>(null);
const directoryScroller = ref<HTMLElement | null>(null);
const directoryScrollbarRef = ref<InstanceType<typeof FloatingScrollbar> | null>(null);
const virtualCodeScrollbarRef = ref<InstanceType<typeof FloatingScrollbar> | null>(null);
const addressScrollState = ref({ scrollable: false, left: 0, clientWidth: 0, scrollWidth: 0 });
const showTabs = computed(() => props.showTabs !== false && !props.directoryOnly);
const showPickFileButton = computed(() => props.showPickFileButton !== false && !props.directoryOnly);
const directoryOnly = computed(() => !!props.directoryOnly);
const fileReaderWatchSessionId = computed(() => String(props.sessionKey || props.customMarkstreamId || "file-reader").trim());

const hoverDirectoryTreeVisible = ref(false);
const hoverDirectoryTreeRoot = ref<DirectoryNode | null>(null);
const hoverDirectoryTreeNodes = ref<Record<string, DirectoryNode>>({});
const hoverDirectoryTreeStyle = ref<Record<string, string>>({ left: "0px", top: "0px", width: "280px" });
const hoverDirectoryTreeRef = ref<HTMLElement | null>(null);
const hoverDirectoryScroller = ref<HTMLElement | null>(null);
const hoverDirectoryTreeAnchor = ref("");
let hoverHideTimer: number | null = null;

let unlistenFileDrop: (() => void) | null = null;
let unlistenFileReaderWatch: UnlistenFn | null = null;
let restoringSessionId = 0;
let suppressSessionPersist = false;
let lastCapturedSelectionKey = "";
let lastCapturedVisibleRangeKey = "";
let visibleRangeCaptureTimer = 0;
let watchTargetUpdateTimer = 0;
let autoRefreshFileTimer = 0;
let autoRefreshDirectoryTimer = 0;
const pendingAutoRefreshDirectoryPaths = new Set<string>();
let activeHighlightRefreshId = 0;

// ==================== Computed ====================

const activeTab = computed(() => tabs.value.find((tab) => tab.path === activePath.value) || tabs.value[0] || null);

const directoryTreeRoot = computed(() => {
  const rootPath = normalizePath(directoryRootPath.value);
  return rootPath ? directoryNodes.value[rootPath] || null : null;
});

const hoverDirectoryTreeRows = computed<TreeRow[]>(() => {
  const root = hoverDirectoryTreeRoot.value;
  if (!root || root.loading || root.error) return [];
  return flattenDirectoryEntriesFromNodes(root.entries, root.path, hoverDirectoryTreeNodes.value, 0, directoryTreeFilter.value);
});

const hoverDirectoryBridgeStyle = computed(() => {
  const style = hoverDirectoryTreeStyle.value;
  const width = parseInt(style.width || "280", 10);
  return {
    left: style.left,
    top: `${parseInt(style.top || "0", 10) - 4}px`,
    width: `${width}px`,
    height: "8px",
  };
});

function isHoverDirectoryExpanded(path: string) {
  return !!hoverDirectoryTreeNodes.value[normalizePath(path)]?.expanded;
}

function isHoverDirectoryCollapsed(path: string) {
  const node = hoverDirectoryTreeNodes.value[normalizePath(path)];
  return node !== undefined && node.loaded && !node.loading && !node.expanded;
}

const activeMarkdownSource = computed(() => {
  const tab = activeTab.value;
  if (!tab) return "";
  if (isTabRawMode(tab)) return "";
  return tab.kind === "markdown" ? stripMarkdownHtmlComments(tab.content) : "";
});

const activeVirtualCodeBlocks = computed<VirtualCodeBlock[]>(() => {
  const tab = activeTab.value;
  if (!tab || !tab.virtualized) return [];
  const totalLines = Math.max(0, tab.totalLines);
  const blockLineCount = Math.max(1, tab.blockLineCount || 120);
  const blocks: VirtualCodeBlock[] = [];
  for (let startLine = 1; startLine <= totalLines; startLine += blockLineCount) {
    const endLine = Math.min(totalLines, startLine + blockLineCount - 1);
    blocks.push({
      key: buildFileBlockKey(tab.path, startLine, endLine),
      path: tab.path,
      startLine,
      endLine,
      lineCount: endLine - startLine + 1,
    });
  }
  return blocks;
});

const addressScrollbarThumbStyle = computed(() => {
  const state = addressScrollState.value;
  if (!state.scrollable || state.clientWidth <= 0 || state.scrollWidth <= 0) {
    return { width: "0px", transform: "translateX(0)" };
  }
  const width = Math.max(20, Math.round((state.clientWidth / state.scrollWidth) * state.clientWidth));
  const maxLeft = Math.max(1, state.scrollWidth - state.clientWidth);
  const maxThumbLeft = Math.max(0, state.clientWidth - width);
  const left = Math.round((state.left / maxLeft) * maxThumbLeft);
  return { width: `${width}px`, transform: `translateX(${left}px)` };
});

const virtualCodeBlockVirtualizer = useVirtualizer(
  computed(() => ({
    count: activeVirtualCodeBlocks.value.length,
    getScrollElement: () => virtualCodeScroller.value,
    getItemKey: (index: number) => activeVirtualCodeBlocks.value[index]?.key ?? `file-block-${index}`,
    estimateSize: (index: number) => {
      const block = activeVirtualCodeBlocks.value[index];
      if (!block) return FILE_READER_VIRTUAL_BLOCK_LINE_HEIGHT_PX;
      return block.lineCount * FILE_READER_VIRTUAL_BLOCK_LINE_HEIGHT_PX + FILE_READER_VIRTUAL_BLOCK_PADDING_Y_PX * 2;
    },
    overscan: FILE_READER_VIRTUAL_BLOCK_OVERSCAN,
    measureElement: (element: Element) => (element as HTMLElement).getBoundingClientRect().height,
  })),
);

const activeVirtualCodeEntries = computed(() =>
  virtualCodeBlockVirtualizer.value.getVirtualItems().map((row) => ({
    row,
    block: activeVirtualCodeBlocks.value[row.index],
  })).filter((entry): entry is { row: ReturnType<typeof virtualCodeBlockVirtualizer.value.getVirtualItems>[number]; block: VirtualCodeBlock } => Boolean(entry.block)),
);

const activeVirtualCodeTotalSize = computed(() => virtualCodeBlockVirtualizer.value.getTotalSize());

const virtualCodeLineNumberDigits = computed(() => {
  const totalLines = Math.max(1, activeTab.value?.totalLines || 1);
  return Math.max(2, String(totalLines).length);
});

const activeShikiTheme = computed(() => (props.markdownIsDark ? "github-dark" : "github-light"));

const visibleTreeRows = computed<TreeRow[]>(() => {
  const root = directoryTreeRoot.value;
  if (!root || root.loading || root.error) return [];
  return flattenDirectoryEntries(root.entries, 0, directoryTreeFilter.value);
});

const activePathSegments = computed(() => {
  const tab = activeTab.value;
  if (!tab) return [];
  const normalized = normalizePath(tab.path);
  const parts = normalized.split("/").filter(Boolean);
  if (parts.length <= 1) return [];
  const dirs = parts.slice(0, -1);
  return dirs.map((label, index) => {
    const head = dirs[0]?.endsWith(":") ? `${dirs[0]}/` : dirs[0] || "";
    const path = index === 0 ? head : [head.replace(/\/$/, ""), ...dirs.slice(1, index + 1)].join("/");
    return { key: `${index}:${label}`, index, label, path };
  });
});

const activeDirectoryPath = computed(() => activePathSegments.value[activePathSegments.value.length - 1]?.path || "");
const initialDirectoryPath = computed(() => normalizePath(props.initialRootPath || ""));
const directoryToggleTargetPath = computed(() => {
  const currentRoot = normalizePath(directoryRootPath.value);
  if (currentRoot) return currentRoot;
  if (initialDirectoryPath.value) return initialDirectoryPath.value;
  return props.directoryOnly ? activeDirectoryPath.value : "";
});

// ==================== Watchers ====================

watch(() => props.initialOpenPath, (path) => {
  if (path) {
    void openPath(path);
  }
}, { immediate: true });

watch([() => props.sessionKey, () => props.initialRootPath], ([nextKey, nextRootPath], [previousKey]) => {
  const previousSessionKey = String(previousKey || "").trim();
  if (previousSessionKey) {
    persistFileReaderSession(previousSessionKey);
  }
  const sessionKey = String(nextKey || "").trim();
  if (sessionKey) {
    void restoreFileReaderSession(sessionKey, nextRootPath);
    return;
  }
  void restoreFileReaderSession("", nextRootPath);
}, { immediate: true });

watch(
  [tabs, activePath, directoryRootPath],
  () => {
    persistFileReaderSession();
    scheduleFileReaderWatchTargetUpdate();
  },
  { deep: true },
);

watch(visibleTreeRows, () => scheduleFileReaderWatchTargetUpdate());
watch(
  activeVirtualCodeEntries,
  (entries) => {
    for (const entry of entries) {
      void ensureVirtualCodeBlockLoaded(entry.block);
    }
  },
  { immediate: true },
);

watch(
  () => props.markdownIsDark,
  () => {
    void refreshActiveCodeHighlights();
  },
);

// ==================== Address Scroll ====================

function updateAddressScrollState() {
  const el = addressScroller.value;
  if (!el) {
    addressScrollState.value = { scrollable: false, left: 0, clientWidth: 0, scrollWidth: 0 };
    return;
  }
  addressScrollState.value = {
    scrollable: el.scrollWidth > el.clientWidth + 1,
    left: el.scrollLeft,
    clientWidth: el.clientWidth,
    scrollWidth: el.scrollWidth,
  };
}

function scheduleAddressScrollStateUpdate() {
  void nextTick(() => updateAddressScrollState());
}

function handleAddressWheel(event: WheelEvent) {
  const el = addressScroller.value;
  if (!el) return;
  event.preventDefault();
  const delta = Math.abs(event.deltaX) > Math.abs(event.deltaY) ? event.deltaX : event.deltaY;
  el.scrollLeft += delta;
  updateAddressScrollState();
}

// ==================== Auto Refresh Watch ====================

function scheduleFileReaderWatchTargetUpdate() {
  if (watchTargetUpdateTimer) window.clearTimeout(watchTargetUpdateTimer);
  watchTargetUpdateTimer = window.setTimeout(() => {
    watchTargetUpdateTimer = 0;
    void updateFileReaderWatchTargets();
  }, 250);
}

async function updateFileReaderWatchTargets() {
  const sessionId = fileReaderWatchSessionId.value;
  if (!sessionId) return;
  if (!isTauriRuntimeAvailable()) return;
  const targets = collectFileReaderWatchTargets();
  try {
    await invokeTauri("update_file_reader_watch_targets", {
      input: { sessionId, targets },
    });
  } catch (error) {
    console.warn("[文件阅读器] 更新自动刷新监听目标失败", error);
  }
}

function collectFileReaderWatchTargets(): FileReaderWatchTarget[] {
  const targets: FileReaderWatchTarget[] = [];
  const active = activeTab.value;
  if (active?.loaded && !active.loading && !active.error && active.path) {
    targets.push({ path: normalizePath(active.path), kind: "file" });
  }
  const root = directoryTreeRoot.value;
  if (root?.path) {
    targets.push({ path: normalizePath(root.path), kind: "directory" });
  }
  const perDirectoryCount = new Map<string, number>();
  for (const row of visibleTreeRows.value) {
    if (row.kind !== "entry") continue;
    const path = normalizePath(row.entry.path);
    const parentPath = directoryFromPath(path);
    const currentCount = perDirectoryCount.get(parentPath) || 0;
    if (currentCount >= 100) continue;
    perDirectoryCount.set(parentPath, currentCount + 1);
    targets.push({ path, kind: row.entry.isDirectory ? "directory" : "file" });
  }
  const seen = new Set<string>();
  return targets.filter((target) => {
    const key = `${target.kind}:${normalizePath(target.path).toLowerCase()}`;
    if (!target.path || seen.has(key)) return false;
    seen.add(key);
    return true;
  });
}

async function startFileReaderWatchListener() {
  stopFileReaderWatchListener();
  if (!isTauriRuntimeAvailable()) return;
  try {
    unlistenFileReaderWatch = await listen<FileReaderWatchEventPayload>("easy-call:file-reader-watch-changed", (event) => {
      handleFileReaderWatchEvent(event.payload);
    });
  } catch (error) {
    console.warn("[文件阅读器] 监听自动刷新事件失败", error);
  }
}

function stopFileReaderWatchListener() {
  unlistenFileReaderWatch?.();
  unlistenFileReaderWatch = null;
}

function handleFileReaderWatchEvent(payload: FileReaderWatchEventPayload) {
  if (String(payload?.sessionId || "").trim() !== fileReaderWatchSessionId.value) return;
  const changedPath = normalizePath(payload?.path || "");
  if (!changedPath) return;
  const active = activeTab.value;
  if (active && sameNormalizedPath(changedPath, active.path)) {
    scheduleAutoRefreshActiveTab();
    return;
  }
  if (String(payload?.kind || "").trim() === "directory" && directoryTreeRoot.value && isPathRelevantToVisibleDirectory(changedPath)) {
    scheduleAutoRefreshDirectoryNode(changedPath);
  }
}

function scheduleAutoRefreshActiveTab() {
  if (autoRefreshFileTimer) window.clearTimeout(autoRefreshFileTimer);
  autoRefreshFileTimer = window.setTimeout(() => {
    autoRefreshFileTimer = 0;
    const active = activeTab.value;
    if (active?.path) void openPath(active.path);
  }, 350);
}

function scheduleAutoRefreshDirectoryNode(path: string) {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath) return;
  pendingAutoRefreshDirectoryPaths.add(normalizedPath);
  if (autoRefreshDirectoryTimer) window.clearTimeout(autoRefreshDirectoryTimer);
  autoRefreshDirectoryTimer = window.setTimeout(() => {
    autoRefreshDirectoryTimer = 0;
    const paths = Array.from(pendingAutoRefreshDirectoryPaths);
    pendingAutoRefreshDirectoryPaths.clear();
    for (const directoryPath of paths) {
      if (!isPathRelevantToVisibleDirectory(directoryPath)) continue;
      const node = treeDirectoryNode(directoryPath);
      const root = directoryTreeRoot.value;
      if (!node && !sameNormalizedPath(directoryPath, root?.path || "")) continue;
      void loadDirectory(directoryPath, node?.expanded ?? true);
    }
  }, 500);
}

function isPathRelevantToVisibleDirectory(path: string) {
  const normalizedPath = normalizePath(path);
  const root = directoryTreeRoot.value;
  if (root && sameNormalizedPath(normalizedPath, root.path)) return true;
  return visibleTreeRows.value.some((row) => row.kind === "entry" && sameNormalizedPath(normalizedPath, row.entry.path));
}

function sameNormalizedPath(left: string, right: string) {
  return normalizePath(left).toLowerCase() === normalizePath(right).toLowerCase();
}

// ==================== Context Capture ====================

function captureCurrentTextSelection() {
  const tab = activeTab.value;
  const scroller = activeContentScroller();
  if (!tab || !scroller || tab.loading || tab.error) return;
  const selection = window.getSelection();
  if (!selection || selection.isCollapsed || selection.rangeCount === 0) {
    lastCapturedSelectionKey = "";
    emit("clearSelectionContextReference");
    return;
  }
  const range = selection.getRangeAt(0);
  if (!scroller.contains(range.commonAncestorContainer)) {
    lastCapturedSelectionKey = "";
    emit("clearSelectionContextReference");
    return;
  }

  const selectedText = normalizeSelectedText(selection.toString());
  if (!selectedText) return;

  const lineRange = resolveSelectedLineRange(tab, scroller, selectedText);
  const meta = buildContextMeta(tab);
  const capturedAt = new Date().toISOString();
  const selectionKey = [
    meta.filePath,
    lineRange.startLine || "",
    lineRange.endLine || "",
    selectedText,
  ].join("\n");
  if (selectionKey === lastCapturedSelectionKey) return;
  lastCapturedSelectionKey = selectionKey;

  const displayLineSuffix = formatLineSuffix(lineRange.startLine, lineRange.endLine);
  emitContextReference({
    tab,
    source: "selection",
    lineRange,
    content: selectedText,
    displayLabel: `${meta.relativePath || tab.title}${displayLineSuffix}`,
    capturedAt,
  });
}

function captureVisibleRangeContext() {
  if (visibleRangeCaptureTimer) window.clearTimeout(visibleRangeCaptureTimer);
  visibleRangeCaptureTimer = window.setTimeout(() => {
    visibleRangeCaptureTimer = 0;
    captureVisibleRangeContextNow();
  }, 80);
}

function captureVisibleRangeContextNow(options: { force?: boolean } = {}) {
  const tab = activeTab.value;
  const scroller = activeContentScroller();
  if (!tab || !scroller || tab.loading || tab.error || !tab.loaded) return;
  const totalLines = tab.virtualized ? Math.max(1, tab.totalLines) : splitContentLines(tab.content).length;
  if (totalLines === 0) return;
  const lineRange = resolveVisibleLineRange(scroller, totalLines);
  const content = tab.virtualized
    ? collectVirtualizedVisibleContent(tab, lineRange)
    : splitContentLines(tab.content).slice(lineRange.startLine - 1, lineRange.endLine).join("\n").trim();
  if (!content) return;
  const meta = buildContextMeta(tab);
  const visibleRangeKey = [meta.filePath, lineRange.startLine, lineRange.endLine, content].join("\n");
  if (!options.force && visibleRangeKey === lastCapturedVisibleRangeKey) return;
  lastCapturedVisibleRangeKey = visibleRangeKey;
  const capturedAt = new Date().toISOString();
  emitContextReference({
    tab,
    source: "visible_range",
    lineRange,
    content: content.slice(0, 20_000),
    displayLabel: `${meta.relativePath || tab.title}${formatLineSuffix(lineRange.startLine, lineRange.endLine)}`,
    capturedAt,
  });
}

function emitContextReference(input: {
  tab: FileTab;
  source: "selection" | "visible_range";
  lineRange: { startLine?: number; endLine?: number };
  content: string;
  displayLabel: string;
  capturedAt: string;
}) {
  const meta = buildContextMeta(input.tab);
  const languageId = languageIdFromTab(input.tab);
  emit("captureContextReference", {
    id: `file-reader-context:${hashText([
      meta.filePath,
      input.source,
      input.lineRange.startLine || "",
      input.lineRange.endLine || "",
    ].join("\n"))}`,
    workspacePath: meta.workspacePath,
    workspaceName: meta.workspaceName,
    filePath: meta.filePath,
    fileName: input.tab.title,
    relativePath: meta.relativePath,
    startLine: input.lineRange.startLine,
    endLine: input.lineRange.endLine,
    displayLabel: input.displayLabel,
    content: input.content,
    languageId,
    source: input.source,
    capturedAt: input.capturedAt,
    textBlock: buildContextTextBlock({
      filePath: meta.filePath,
      lineRange: input.lineRange,
      languageId,
      source: input.source,
      capturedAt: input.capturedAt,
      content: input.content,
    }),
  });
}

function activeContentScroller() {
  const tab = activeTab.value;
  if (!tab) return contentScroller.value;
  if (tab.virtualized) return virtualCodeScroller.value;
  if (tab.kind === "markdown") return markdownScroller.value;
  return contentScroller.value;
}

function canToggleRawMode(tab: FileTab | null | undefined) {
  return !!tab && tab.kind === "markdown";
}

function isTabRawMode(tab: FileTab | null | undefined) {
  if (!tab) return false;
  if (tab.kind === "markdown") return tab.rawMode;
  return tab.kind !== "code";
}

function buildContextMeta(tab: FileTab) {
  const filePath = normalizePath(tab.path);
  const workspacePath = normalizePath(props.initialRootPath || directoryFromPath(filePath));
  return {
    filePath,
    workspacePath,
    workspaceName: titleFromPath(workspacePath),
    relativePath: relativePathFromWorkspace(filePath, workspacePath),
  };
}

function normalizeSelectedText(value: string) {
  return String(value || "")
    .replace(/\r\n/g, "\n")
    .replace(/\u00a0/g, " ")
    .trim()
    .slice(0, 20_000);
}

function splitContentLines(value: string) {
  const normalized = String(value || "").replace(/\r\n/g, "\n");
  return normalized.length > 0 ? normalized.split("\n") : [];
}

function buildFileBlockKey(path: string, startLine: number, endLine: number) {
  return `${normalizePath(path)}::${startLine}-${endLine}`;
}

function blockLineNumbers(block: VirtualCodeBlock) {
  return Array.from({ length: block.lineCount }, (_, index) => block.startLine + index);
}

function blockContentText(blockKey: string) {
  return fileBlockContentByKey.value[blockKey] || "";
}

function blockContentHtml(blockKey: string) {
  return highlightedCodeHtmlByBlockKey.value[blockKey] || escapeHtml(blockContentText(blockKey));
}

function normalizeShikiLineHtml(html: string) {
  return html
    .replace(/<\/span>\s+<span class="line"/g, '</span><span class="line"')
    .replace(/<span class="line"><\/span>/g, '<span class="line"><span class="file-reader-code-empty-line">&#8203;</span></span>');
}

async function renderHighlightedCodeHtml(tab: FileTab, content: string) {
  const language = resolveShikiLanguage(tab.extension);
  const html = await codeToHtml(content, { lang: language, theme: activeShikiTheme.value });
  return normalizeShikiLineHtml(html);
}

async function updateHighlightedCodeBlock(tab: FileTab, blockKey: string, content: string) {
  if (isTabRawMode(tab)) return;
  if (tab.kind === "markdown") return;
  try {
    const html = await renderHighlightedCodeHtml(tab, content);
    highlightedCodeHtmlByBlockKey.value = {
      ...highlightedCodeHtmlByBlockKey.value,
      [blockKey]: html,
    };
  } catch {
    highlightedCodeHtmlByBlockKey.value = { ...highlightedCodeHtmlByBlockKey.value, [blockKey]: escapeHtml(content) };
  }
}

async function refreshActiveCodeHighlights() {
  const active = activeTab.value;
  if (!active || isTabRawMode(active) || active.kind === "markdown") return;
  const activePathKey = `${normalizePath(active.path)}::`;
  const blockEntries = Object.entries(fileBlockContentByKey.value).filter(([key]) => key.startsWith(activePathKey));
  if (blockEntries.length <= 0) return;

  const refreshId = ++activeHighlightRefreshId;
  const nextHtml = { ...highlightedCodeHtmlByBlockKey.value };
  for (const [key] of blockEntries) {
    delete nextHtml[key];
  }

  await Promise.all(blockEntries.map(async ([key, content]) => {
    try {
      nextHtml[key] = await renderHighlightedCodeHtml(active, content);
    } catch {
      nextHtml[key] = escapeHtml(content);
    }
  }));

  if (refreshId !== activeHighlightRefreshId) return;
  highlightedCodeHtmlByBlockKey.value = nextHtml;
}

function resolveVisibleLineRange(scroller: HTMLElement, totalLines: number): { startLine: number; endLine: number } {
  const scrollableHeight = Math.max(1, scroller.scrollHeight - scroller.clientHeight);
  const startRatio = Math.max(0, Math.min(1, scroller.scrollTop / scrollableHeight));
  const visibleRatio = Math.max(0.05, Math.min(1, scroller.clientHeight / Math.max(1, scroller.scrollHeight)));
  const startLine = Math.max(1, Math.min(totalLines, Math.floor(startRatio * totalLines) + 1));
  const visibleLineCount = Math.max(12, Math.ceil(totalLines * visibleRatio));
  const endLine = Math.max(startLine, Math.min(totalLines, startLine + visibleLineCount - 1));
  return { startLine, endLine };
}

function resolveSelectedLineRange(tab: FileTab, scroller: HTMLElement, selectedText: string): { startLine: number; endLine: number } {
  if (tab.virtualized) {
    return resolveVisibleLineRange(scroller, Math.max(1, tab.totalLines));
  }
  if (tab.kind === "markdown" && !isTabRawMode(tab)) {
    return resolveVisibleLineRange(scroller, Math.max(1, splitContentLines(tab.content).length));
  }
  return resolveRawSelectedLineRange(tab.content, selectedText)
    || resolveVisibleLineRange(scroller, Math.max(1, splitContentLines(tab.content).length));
}

function resolveRawSelectedLineRange(source: string, selectedText: string): { startLine: number; endLine: number } | null {
  const normalizedSource = String(source || "").replace(/\r\n/g, "\n");
  const normalizedSelection = selectedText.replace(/\r\n/g, "\n");
  const index = normalizedSource.indexOf(normalizedSelection);
  if (index < 0) return null;
  if (normalizedSource.indexOf(normalizedSelection, index + Math.max(1, normalizedSelection.length)) >= 0) return null;
  const before = normalizedSource.slice(0, index);
  const startLine = before.split("\n").length;
  const selectedLineCount = Math.max(1, normalizedSelection.split("\n").length);
  return { startLine, endLine: startLine + selectedLineCount - 1 };
}

function relativePathFromWorkspace(filePath: string, workspacePath: string) {
  const normalizedFilePath = normalizePath(filePath);
  const normalizedWorkspacePath = normalizePath(workspacePath).replace(/\/+$/, "");
  if (!normalizedWorkspacePath) return normalizedFilePath;
  const lowerFilePath = normalizedFilePath.toLowerCase();
  const lowerWorkspacePath = normalizedWorkspacePath.toLowerCase();
  if (lowerFilePath === lowerWorkspacePath) return titleFromPath(normalizedFilePath);
  const prefix = `${lowerWorkspacePath}/`;
  if (lowerFilePath.startsWith(prefix)) {
    return normalizedFilePath.slice(normalizedWorkspacePath.length + 1);
  }
  return normalizedFilePath;
}

function languageIdFromTab(tab: FileTab) {
  return CODE_LANGUAGE_BY_EXTENSION[tab.extension] || tab.extension || tab.kind || "text";
}

function formatLineSuffix(startLine?: number, endLine?: number) {
  if (!startLine) return "";
  if (endLine && endLine > startLine) return `:${startLine}-${endLine}`;
  return `:${startLine}`;
}

function hashText(value: string) {
  let hash = 2166136261;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0).toString(16);
}

function measureVirtualCodeRow(element: Element | { $el?: Element } | null) {
  if (!element) return;
  const target = element instanceof Element ? element : element.$el;
  if (!(target instanceof Element)) return;
  virtualCodeBlockVirtualizer.value.measureElement(target);
}

function buildContextTextBlock(input: {
  filePath: string;
  lineRange: { startLine?: number; endLine?: number };
  languageId: string;
  source: string;
  capturedAt: string;
  content: string;
}) {
  const location = `${input.filePath}${formatLineSuffix(input.lineRange.startLine, input.lineRange.endLine)}`;
  const contentLength = input.content.length;
  if (contentLength > CONTEXT_TEXT_BLOCK_CONTENT_LIMIT) {
    return `${t('fileReader.referenceFile', { location })}${t('fileReader.referenceTruncated', { count: contentLength })}`;
  }
  return [
    t('fileReader.referenceFile', { location }),
    "```text",
    input.content,
    "```",
  ].join("\n");
}

function collectVirtualizedVisibleContent(tab: FileTab, lineRange: { startLine: number; endLine: number }) {
  const chunks: string[] = [];
  for (const block of activeVirtualCodeBlocks.value) {
    if (block.path !== tab.path) continue;
    if (block.endLine < lineRange.startLine || block.startLine > lineRange.endLine) continue;
    const blockContent = blockContentText(block.key);
    if (!blockContent) continue;
    const blockLines = splitContentLines(blockContent);
    const sliceStart = Math.max(0, lineRange.startLine - block.startLine);
    const sliceEndExclusive = Math.min(blockLines.length, lineRange.endLine - block.startLine + 1);
    if (sliceEndExclusive <= sliceStart) continue;
    chunks.push(blockLines.slice(sliceStart, sliceEndExclusive).join("\n"));
  }
  return chunks.join("\n").trim();
}

// ==================== Helpers ====================

function normalizePath(path: string) {
  return String(path || "")
    .trim()
    .replace(/^\\\\\?\\/, "")
    .replace(/^\/\/\?\//, "")
    .replace(/^\/\?\//, "")
    .replace(/^\?\//, "")
    .replace(/^\?\\/, "")
    .replace(/\\/g, "/")
    .replace(/^\/([A-Za-z]:)/, "$1");
}

function readFileReaderSessionState(key = props.sessionKey): FileReaderSessionState {
  const storageKey = String(key || "").trim();
  if (!storageKey || typeof window === "undefined") return {};
  try {
    const legacyStorageKey = String(props.legacySessionKey || "").trim();
    return JSON.parse(window.localStorage.getItem(storageKey) || (legacyStorageKey ? window.localStorage.getItem(legacyStorageKey) : "") || "{}") as FileReaderSessionState;
  } catch {
    return {};
  }
}

function persistFileReaderSession(key = props.sessionKey) {
  if (suppressSessionPersist) return;
  const storageKey = String(key || "").trim();
  if (!storageKey || typeof window === "undefined") return;
  const uniqueTabs = Array.from(new Set(tabs.value.map((tab) => normalizePath(tab.path)).filter(Boolean)));
  const state: FileReaderSessionState = {
    tabs: uniqueTabs,
    activePath: normalizePath(activePath.value),
    directoryRootPath: normalizePath(directoryRootPath.value),
  };
  window.localStorage.setItem(storageKey, JSON.stringify(state));
}

async function restoreFileReaderSession(key = props.sessionKey, fallbackRootPath = props.initialRootPath) {
  const storageKey = String(key || "").trim();
  const restoreId = ++restoringSessionId;
  suppressSessionPersist = true;
  try {
    tabs.value = [];
    activePath.value = "";
    highlightedCodeHtmlByBlockKey.value = {};
    fileBlockContentByKey.value = {};
    fileBlockLoadingByKey.value = {};
    fileBlockErrorByKey.value = {};
    directoryRootPath.value = "";
    directoryTreeFilter.value = "";
    directoryTreeSearchVisible.value = false;
    directoryNodes.value = {};

    const fallbackRoot = props.directoryOnly ? normalizePath(fallbackRootPath || "") : "";
    if (!storageKey) {
      if (fallbackRoot) {
        directoryRootPath.value = fallbackRoot;
        await loadDirectory(fallbackRoot, true);
      }
      return;
    }
    const state = readFileReaderSessionState(storageKey);
    if (restoreId !== restoringSessionId) return;

    const restoredTabs = Array.from(new Set((state.tabs || []).map((path) => normalizePath(path)).filter(Boolean)));
    tabs.value = restoredTabs.map((path) => createRestoredTab(path));
    const restoredActivePath = normalizePath(state.activePath || "");
    activePath.value = restoredTabs.includes(restoredActivePath) ? restoredActivePath : restoredTabs[0] || "";

    const restoredDirectoryRoot = props.directoryOnly
      ? normalizePath(state.directoryRootPath || fallbackRoot)
      : fallbackRoot;
    if (restoredDirectoryRoot) {
      directoryRootPath.value = restoredDirectoryRoot;
    }

    await nextTick();
    if (restoreId !== restoringSessionId) return;
    suppressSessionPersist = false;

    if (restoredDirectoryRoot) {
      await loadDirectory(restoredDirectoryRoot, true);
    }
    if (restoreId !== restoringSessionId) return;
    if (activePath.value) {
      await openPath(activePath.value);
    }
  } finally {
    if (restoreId === restoringSessionId) {
      suppressSessionPersist = false;
      scheduleAddressScrollStateUpdate();
    }
  }
}

function createRestoredTab(path: string): FileTab {
  const normalizedPath = normalizePath(path);
  return {
    path: normalizedPath,
    title: titleFromPath(normalizedPath),
    extension: extensionFromPath(normalizedPath),
    kind: fileKindFromPath(normalizedPath),
    content: "",
    rawMode: false,
    forcePlain: false,
    virtualized: false,
    totalLines: 0,
    blockLineCount: 0,
    loaded: false,
    loading: false,
    error: "",
  };
}

function fileKindFromPath(path: string) {
  const extension = extensionFromPath(path);
  return ["md", "markdown", "mdx"].includes(extension) ? "markdown" : "code";
}

function extensionFromPath(path: string) {
  const fileName = titleFromPath(path);
  const lowerFileName = fileName.toLowerCase();
  if (CODE_LANGUAGE_BY_EXTENSION[lowerFileName]) return lowerFileName;
  if (SHIKI_LANGUAGE_KEYS.has(lowerFileName)) return lowerFileName;
  const dotIndex = fileName.lastIndexOf(".");
  if (dotIndex <= 0 || dotIndex === fileName.length - 1) return "";
  const extension = fileName.slice(dotIndex + 1).toLowerCase();
  return CODE_LANGUAGE_BY_EXTENSION[extension] || SHIKI_LANGUAGE_KEYS.has(extension) ? extension : "";
}

function escapeHtml(value: string) {
  return String(value || "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

function stripMarkdownHtmlComments(value: string) {
  return String(value || "").replace(/<!--[\s\S]*?-->/g, "");
}

function resolveShikiLanguage(extension: string) {
  const key = String(extension || "").trim().toLowerCase();
  const mapped = CODE_LANGUAGE_BY_EXTENSION[key] || key;
  return SHIKI_LANGUAGE_KEYS.has(mapped) ? mapped : "text";
}

function titleFromPath(path: string) {
  const normalized = normalizePath(path);
  return normalized.split("/").filter(Boolean).pop() || normalized || "未命名文件";
}

function directoryFromPath(path: string) {
  const normalized = normalizePath(path).replace(/\/+$/, "");
  const slashIndex = normalized.lastIndexOf("/");
  if (slashIndex < 0) return "";
  if (slashIndex === 0) return "/";
  if (slashIndex === 2 && normalized[1] === ":") return normalized.slice(0, 3);
  return normalized.slice(0, slashIndex);
}

function replaceTabState(tab: FileTab, matchPath = tab.path) {
  const normalizedMatchPath = normalizePath(matchPath);
  tabs.value = tabs.value.map((item) => item.path === normalizedMatchPath ? { ...tab } : item);
}

function upsertLoadingTab(path: string, reuseActiveTab = false) {
  const normalizedPath = normalizePath(path);
  const existing = tabs.value.find((tab) => tab.path === normalizedPath);
  if (existing) {
    existing.loading = true;
    existing.error = "";
    existing.loaded = false;
    activePath.value = existing.path;
    replaceTabState(existing);
    scheduleAddressScrollStateUpdate();
    return existing;
  }
  const current = activeTab.value;
  if (reuseActiveTab && current && !current.loading) {
    const previousPath = current.path;
    clearFileBlockCaches(previousPath);
    current.path = normalizedPath;
    current.title = titleFromPath(normalizedPath);
    current.extension = "";
    current.kind = "code";
    current.content = "";
    current.rawMode = false;
    current.forcePlain = false;
    current.virtualized = false;
    current.totalLines = 0;
    current.blockLineCount = 0;
    current.loaded = false;
    current.loading = true;
    current.error = "";
    activePath.value = normalizedPath;
    replaceTabState(current, previousPath);
    scheduleAddressScrollStateUpdate();
    return current;
  }
  const tab: FileTab = {
    path: normalizedPath, title: titleFromPath(normalizedPath), extension: "",
    kind: "code", content: "", rawMode: false, forcePlain: false, virtualized: false, totalLines: 0, blockLineCount: 0, loaded: false, loading: true, error: "",
  };
  tabs.value = [...tabs.value, tab];
  activePath.value = normalizedPath;
  scheduleAddressScrollStateUpdate();
  return tab;
}

function setActiveTab(path: string) {
  const normalizedPath = normalizePath(path);
  activePath.value = normalizedPath;
  scheduleAddressScrollStateUpdate();
  const tab = tabs.value.find((item) => item.path === normalizedPath);
  if (tab && !tab.loaded && !tab.loading) {
    void openPath(normalizedPath);
  } else {
    void nextTick(() => captureVisibleRangeContextNow());
  }
}

function toggleActiveRawMode() {
  const tab = activeTab.value;
  if (!tab || !canToggleRawMode(tab)) return;
  tab.rawMode = !tab.rawMode;
  replaceTabState(tab);
}

function openOrActivatePath(path: string) {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath) return;
  const existing = tabs.value.some((tab) => tab.path === normalizedPath);
  if (existing) {
    setActiveTab(normalizedPath);
    return;
  }
  const current = activeTab.value;
  const sameDirectory = !!current && directoryFromPath(current.path) === directoryFromPath(normalizedPath);
  void openPath(normalizedPath, { reuseActiveTab: sameDirectory });
}

function handleTreeEntryClick(entry: FileReaderDirectoryEntry) {
  if (entry.isDirectory) {
    void toggleTreeDirectory(entry);
    return;
  }
  const normalizedPath = normalizePath(entry.path);
  if (!normalizedPath) return;
  if (props.directoryOnly) {
    emit("selectPath", normalizedPath);
    return;
  }
  openOrActivatePath(normalizedPath);
}

function migrateTabPath(tab: FileTab, fromPath: string, toPath: string) {
  if (fromPath === toPath) return tab;
  const normalizedFromPath = normalizePath(fromPath);
  const duplicated = tabs.value.find((item) => item !== tab && item.path === toPath);
  if (duplicated) {
    tabs.value = tabs.value.filter((item) => item.path !== normalizedFromPath);
    clearFileBlockCaches(fromPath);
    activePath.value = toPath;
    scheduleAddressScrollStateUpdate();
    return duplicated;
  }
  const contentNext = { ...fileBlockContentByKey.value };
  const loadingNext = { ...fileBlockLoadingByKey.value };
  const errorNext = { ...fileBlockErrorByKey.value };
  const htmlNext = { ...highlightedCodeHtmlByBlockKey.value };
  for (const key of Object.keys(contentNext)) {
    if (!key.startsWith(`${normalizedFromPath}::`)) continue;
    const suffix = key.slice(normalizedFromPath.length);
    const nextKey = `${toPath}${suffix}`;
    contentNext[nextKey] = contentNext[key];
    delete contentNext[key];
    if (loadingNext[key] !== undefined) {
      loadingNext[nextKey] = loadingNext[key];
      delete loadingNext[key];
    }
    if (errorNext[key] !== undefined) {
      errorNext[nextKey] = errorNext[key];
      delete errorNext[key];
    }
    if (htmlNext[key] !== undefined) {
      htmlNext[nextKey] = htmlNext[key];
      delete htmlNext[key];
    }
  }
  fileBlockContentByKey.value = contentNext;
  fileBlockLoadingByKey.value = loadingNext;
  fileBlockErrorByKey.value = errorNext;
  highlightedCodeHtmlByBlockKey.value = htmlNext;
  tab.path = toPath;
  activePath.value = toPath;
  replaceTabState(tab, fromPath);
  scheduleAddressScrollStateUpdate();
  return tab;
}

function reportFileReaderActionFailure(action: string, path: string, error: unknown) {
  const detail = error instanceof Error ? error.message : String(error);
  console.error(`[文件阅读器] ${action}失败`, { path, error });
  actionErrorMessage.value = t('fileReader.actionFailed', { action, path, detail });
  window.setTimeout(() => {
    if (actionErrorMessage.value === t('fileReader.actionFailed', { action, path, detail })) {
      actionErrorMessage.value = "";
    }
  }, 4500);
}

// ==================== Public API ====================

async function openPath(path: string, options: { reuseActiveTab?: boolean } = {}) {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath) return;
  const current = tabs.value.find((tab) => tab.path === normalizedPath);
  if (current?.loading) {
    activePath.value = normalizedPath;
    scheduleAddressScrollStateUpdate();
    return;
  }
  let tab = upsertLoadingTab(normalizedPath, !!options.reuseActiveTab);
  try {
    const payload = await requestFileReaderFile(normalizedPath);
    const resolvedPath = normalizePath(payload.path || normalizedPath);
    tab = migrateTabPath(tab, normalizedPath, resolvedPath);
    tab.title = payload.name || titleFromPath(resolvedPath);
    tab.extension = String(payload.extension || extensionFromPath(resolvedPath)).toLowerCase();
    tab.kind = String(payload.kind || "code");
    tab.content = String(payload.content || "");
    tab.rawMode = false;
    tab.forcePlain = !!payload.forcePlain;
    tab.virtualized = !!payload.virtualized;
    tab.totalLines = Number(payload.totalLines || 0);
    tab.blockLineCount = Number(payload.blockLineCount || 0);
    tab.loaded = true;
    tab.error = "";
    tab.loading = false;
    activePath.value = resolvedPath;
    replaceTabState(tab);
    clearFileBlockCaches(resolvedPath);
    scheduleAddressScrollStateUpdate();
    void nextTick(() => captureVisibleRangeContextNow());
    emit("openPath", resolvedPath);
  } catch (error) {
    tab.loaded = true;
    tab.loading = false;
    tab.error = error instanceof Error ? error.message : String(error);
    replaceTabState(tab);
  }
}

async function openDroppedPaths(paths: string[]) {
  const normalizedPaths = paths.map((path) => normalizePath(path)).filter(Boolean);
  for (const path of normalizedPaths) {
    await openPath(path);
  }
}

function closeTab(path: string) {
  const index = tabs.value.findIndex((tab) => tab.path === path);
  if (index < 0) return;
  const wasActive = activePath.value === path;
  tabs.value = tabs.value.filter((tab) => tab.path !== path);
  clearFileBlockCaches(path);
  if (wasActive) {
    activePath.value = tabs.value[Math.max(0, index - 1)]?.path || tabs.value[0]?.path || "";
    scheduleAddressScrollStateUpdate();
    const nextTab = tabs.value.find((tab) => tab.path === activePath.value);
    if (nextTab && !nextTab.loaded && !nextTab.loading) {
      void openPath(nextTab.path);
    }
  }
}

function refreshActiveTab() {
  const tab = activeTab.value;
  if (!tab) return;
  void openPath(tab.path);
}

async function openWithDefaultProgram() {
  const tab = activeTab.value;
  if (!tab) return;
  try {
    await invokeTauri("open_file_with_default_program", { path: tab.path });
  } catch (error) {
    reportFileReaderActionFailure(t('fileReader.actionOpenDefault'), tab.path, error);
  }
}

async function pickFile() {
  const picked = await open({ multiple: false, directory: false, title: t('fileReader.openFile') });
  if (!picked || Array.isArray(picked)) return;
  await openPath(String(picked));
}

// ==================== Directory Tree ====================

async function requestFileReaderDirectory(path: string): Promise<FileReaderDirectoryPayload> {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath) {
    return {
      path: "",
      name: "",
      entries: [],
    };
  }
  if (props.bridgeRequest) {
    return await props.bridgeRequest<FileReaderDirectoryPayload>("fileReader.directory.list", { path: normalizedPath });
  }
  return await invokeTauri<FileReaderDirectoryPayload>("list_file_reader_directory", { path: normalizedPath });
}

async function requestFileReaderFile(path: string): Promise<FileReaderFilePayload> {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath) {
    return {
      path: "",
      name: "",
      extension: "",
      kind: "code",
      content: "",
      forcePlain: false,
      virtualized: false,
      totalLines: 0,
      blockLineCount: 0,
    };
  }
  if (props.bridgeRequest) {
    return await props.bridgeRequest<FileReaderFilePayload>("fileReader.readFile", { path: normalizedPath });
  }
  return await invokeTauri<FileReaderFilePayload>("read_file_reader_file", { path: normalizedPath });
}

async function requestFileReaderFileBlock(path: string, startLine: number, lineCount: number): Promise<FileReaderFileBlockPayload> {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath) {
    return { path: "", startLine: 1, endLine: 1, content: "" };
  }
  if (props.bridgeRequest) {
    return await props.bridgeRequest<FileReaderFileBlockPayload>("fileReader.readFileBlock", {
      path: normalizedPath,
      startLine,
      lineCount,
    });
  }
  return await invokeTauri<FileReaderFileBlockPayload>("read_file_reader_file_block", {
    path: normalizedPath,
    startLine,
    lineCount,
  });
}

function clearFileBlockCaches(path: string) {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath) return;
  const contentNext = { ...fileBlockContentByKey.value };
  const loadingNext = { ...fileBlockLoadingByKey.value };
  const errorNext = { ...fileBlockErrorByKey.value };
  const htmlNext = { ...highlightedCodeHtmlByBlockKey.value };
  for (const key of Object.keys(contentNext)) {
    if (!key.startsWith(`${normalizedPath}::`)) continue;
    delete contentNext[key];
    delete loadingNext[key];
    delete errorNext[key];
    delete htmlNext[key];
  }
  fileBlockContentByKey.value = contentNext;
  fileBlockLoadingByKey.value = loadingNext;
  fileBlockErrorByKey.value = errorNext;
  highlightedCodeHtmlByBlockKey.value = htmlNext;
}

async function ensureVirtualCodeBlockLoaded(block: VirtualCodeBlock) {
  if (!block.path) return;
  if (fileBlockContentByKey.value[block.key] !== undefined) return;
  if (fileBlockLoadingByKey.value[block.key]) return;
  fileBlockLoadingByKey.value = { ...fileBlockLoadingByKey.value, [block.key]: true };
  try {
    const payload = await requestFileReaderFileBlock(block.path, block.startLine, block.lineCount);
    const normalizedKey = buildFileBlockKey(payload.path || block.path, payload.startLine, payload.endLine);
    fileBlockContentByKey.value = { ...fileBlockContentByKey.value, [normalizedKey]: String(payload.content || "") };
    const active = activeTab.value;
    if (active && sameNormalizedPath(active.path, block.path) && !isTabRawMode(active)) {
      await updateHighlightedCodeBlock(active, normalizedKey, String(payload.content || ""));
    }
    const errorNext = { ...fileBlockErrorByKey.value };
    delete errorNext[block.key];
    fileBlockErrorByKey.value = errorNext;
  } catch (error) {
    fileBlockErrorByKey.value = { ...fileBlockErrorByKey.value, [block.key]: error instanceof Error ? error.message : String(error) };
  } finally {
    const loadingNext = { ...fileBlockLoadingByKey.value };
    delete loadingNext[block.key];
    fileBlockLoadingByKey.value = loadingNext;
  }
}

async function loadDirectory(path: string, expanded: boolean) {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath) return;
  updateDirectoryNode(normalizedPath, { loading: true, error: "", expanded });
  try {
    const payload = await requestFileReaderDirectory(normalizedPath);
    const resolvedPath = normalizePath(payload.path || normalizedPath);
    if (directoryRootPath.value === normalizedPath) {
      directoryRootPath.value = resolvedPath;
    }
    updateDirectoryNode(resolvedPath, {
      name: String(payload.name || titleFromPath(resolvedPath)),
      entries: normalizeDirectoryEntries(payload.entries || []),
      loaded: true, loading: false, error: "", expanded,
    });
  } catch (error) {
    updateDirectoryNode(normalizedPath, {
      loaded: false, loading: false,
      error: error instanceof Error ? error.message : String(error), expanded,
    });
  }
}

async function openDirectoryTree(path: string) {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath) return;
  directoryRootPath.value = normalizedPath;
  await loadDirectory(normalizedPath, true);
}

function closeDirectoryTree() {
  directoryRootPath.value = "";
  directoryTreeFilter.value = "";
  directoryTreeSearchVisible.value = false;
}

// ==================== Hover Directory Tree ====================

async function showHoverDirectoryTree(path: string, event: MouseEvent) {
  if (hoverHideTimer) { window.clearTimeout(hoverHideTimer); hoverHideTimer = null; }

  const normalizedPath = normalizePath(path);
  if (!normalizedPath) return;

  hoverDirectoryTreeAnchor.value = normalizedPath;
  hoverDirectoryTreeVisible.value = true;

  await nextTick();

  const target = event.target as HTMLElement;
  const rect = target.getBoundingClientRect();
  const panelWidth = 280;
  const panelHeight = Math.round(window.innerHeight * 0.8);
  const gap = 4;

  let left = rect.left;
  if (left + panelWidth > window.innerWidth) {
    left = window.innerWidth - panelWidth - 8;
  }
  if (left < 8) left = 8;

  let top = rect.bottom + gap;
  if (top + panelHeight > window.innerHeight) {
    top = Math.max(8, rect.top - panelHeight - gap);
  }

  hoverDirectoryTreeStyle.value = {
    left: `${left}px`,
    top: `${top}px`,
    width: `${panelWidth}px`,
    height: `${panelHeight}px`,
  };

  if (hoverDirectoryTreeNodes.value[normalizedPath]?.loaded) {
    hoverDirectoryTreeRoot.value = hoverDirectoryTreeNodes.value[normalizedPath];
    return;
  }

  await loadDirectoryForHover(normalizedPath);
}

function hideHoverDirectoryTree() {
  hoverHideTimer = window.setTimeout(() => {
    hoverDirectoryTreeVisible.value = false;
    hoverDirectoryTreeRoot.value = null;
    hoverDirectoryTreeAnchor.value = "";
    hoverHideTimer = null;
  }, 150);
}

function cancelHideHoverDirectoryTree() {
  if (hoverHideTimer) { window.clearTimeout(hoverHideTimer); hoverHideTimer = null; }
}

async function loadDirectoryForHover(path: string) {
  const dirName = path.split(/[\\/]/).filter(Boolean).pop() || path;
  hoverDirectoryTreeRoot.value = updateHoverDirectoryNode(path, {
    name: dirName,
    entries: [],
    loaded: false,
    loading: true,
    error: "",
    expanded: true,
  });

  try {
    const payload = await requestFileReaderDirectory(path);
    const resolvedPath = normalizePath(payload.path || path);
    const resolvedName = String(payload.name || dirName);
    const normalizedEntries = normalizeDirectoryEntries(payload.entries || []);
    hoverDirectoryTreeRoot.value = updateHoverDirectoryNode(resolvedPath, {
      name: resolvedName,
      entries: normalizedEntries,
      loaded: true,
      loading: false,
      error: "",
      expanded: true,
    });
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error);
    hoverDirectoryTreeRoot.value = updateHoverDirectoryNode(path, {
      name: dirName,
      entries: [],
      loaded: true,
      loading: false,
      error: errorMsg,
      expanded: true,
    });
  }
}

function updateHoverDirectoryNode(path: string, patch: Partial<DirectoryNode>) {
  const normalizedPath = normalizePath(path);
  const current = hoverDirectoryTreeNodes.value[normalizedPath] || {
    path: normalizedPath,
    name: titleFromPath(normalizedPath),
    entries: [],
    loaded: false,
    loading: false,
    error: "",
    expanded: false,
  };
  const next = { ...current, ...patch, path: normalizedPath };
  hoverDirectoryTreeNodes.value = {
    ...hoverDirectoryTreeNodes.value,
    [normalizedPath]: next,
  };
  if (hoverDirectoryTreeRoot.value?.path === normalizedPath) {
    hoverDirectoryTreeRoot.value = next;
  }
  return next;
}

function toggleHoverDirectory(entry: FileReaderDirectoryEntry) {
  if (!entry.isDirectory) return;
  const normalizedPath = normalizePath(entry.path);
  const node = hoverDirectoryTreeNodes.value[normalizedPath];

  if (node?.expanded) {
    updateHoverDirectoryNode(normalizedPath, { expanded: false });
  } else if (node?.loaded) {
    updateHoverDirectoryNode(normalizedPath, { expanded: true });
  } else {
    updateHoverDirectoryNode(normalizedPath, {
      name: String(entry.name || titleFromPath(normalizedPath)),
      entries: [],
      loaded: false,
      error: "",
      loading: true,
      expanded: true,
    });
    loadHoverSubDirectory(entry);
  }
}

async function loadHoverSubDirectory(entry: FileReaderDirectoryEntry) {
  const normalizedPath = normalizePath(entry.path);
  try {
    const payload = await requestFileReaderDirectory(normalizedPath);
    const normalizedEntries = normalizeDirectoryEntries(payload.entries || []);
    updateHoverDirectoryNode(normalizedPath, { entries: normalizedEntries, loaded: true, loading: false, error: "", expanded: true });
  } catch (error) {
    const errorMsg = error instanceof Error ? error.message : String(error);
    updateHoverDirectoryNode(normalizedPath, { loading: false, loaded: true, error: errorMsg });
  }
}

async function openFileFromHoverTree(entry: FileReaderDirectoryEntry) {
  if (entry.isDirectory) {
    toggleHoverDirectory(entry);
    return;
  }
  hideHoverDirectoryTree();
  await openPath(entry.path);
}

async function toggleDirectoryTree() {
  if (directoryTreeRoot.value) {
    closeDirectoryTree();
    return;
  }
  const path = directoryToggleTargetPath.value;
  if (path) {
    await openDirectoryTree(path);
  }
}

function toggleDirectoryTreeSearch() {
  directoryTreeSearchVisible.value = !directoryTreeSearchVisible.value;
  if (!directoryTreeSearchVisible.value) {
    directoryTreeFilter.value = "";
  }
}

async function openShellAtDirectoryTreeRoot() {
  const root = directoryTreeRoot.value;
  if (!root) return;
  try {
    await invokeTauri("open_file_reader_directory_shell", { path: root.path });
  } catch (error) {
    reportFileReaderActionFailure(t('fileReader.actionOpenShell'), root.path, error);
  }
}

async function openDirectoryInFileManager(path: string) {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath) return;
  try {
    await invokeTauri("open_local_file_directory", { path: normalizedPath });
  } catch (error) {
    reportFileReaderActionFailure(t('fileReader.actionOpenDirectory'), normalizedPath, error);
  }
}

async function toggleTreeDirectory(entry: FileReaderDirectoryEntry) {
  if (!entry.isDirectory) return;
  const normalizedPath = normalizePath(entry.path);
  const node = treeDirectoryNode(normalizedPath);
  if (node?.expanded) {
    updateDirectoryNode(normalizedPath, { expanded: false });
    return;
  }
  if (node?.loaded) {
    updateDirectoryNode(normalizedPath, { expanded: true, error: "" });
    return;
  }
  await loadDirectory(normalizedPath, true);
}

function normalizeDirectoryEntries(entries: FileReaderDirectoryEntry[]) {
  return entries.map((entry) => ({
    ...entry,
    path: normalizePath(entry.path),
    name: String(entry.name || titleFromPath(entry.path)),
    isDirectory: !!entry.isDirectory,
  }));
}

function updateDirectoryNode(path: string, patch: Partial<DirectoryNode>) {
  const normalizedPath = normalizePath(path);
  if (!normalizedPath) return;
  const current = directoryNodes.value[normalizedPath] || {
    path: normalizedPath, name: titleFromPath(normalizedPath), entries: [],
    loaded: false, loading: false, error: "", expanded: false,
  };
  directoryNodes.value = {
    ...directoryNodes.value,
    [normalizedPath]: { ...current, ...patch, path: normalizedPath },
  };
}

function treeDirectoryNode(path: string) {
  return directoryNodes.value[normalizePath(path)] || null;
}

function isTreeDirectoryExpanded(path: string) {
  return !!treeDirectoryNode(path)?.expanded;
}

function flattenDirectoryEntries(entries: FileReaderDirectoryEntry[], depth: number, filter = ""): TreeRow[] {
  const normalizedFilter = filter.trim().toLowerCase();
  const rows: TreeRow[] = [];
  for (const entry of entries) {
    const normalizedPath = normalizePath(entry.path);
    const childRows = entry.isDirectory ? flattenDirectoryEntries(treeDirectoryNode(normalizedPath)?.entries || [], depth + 1, filter) : [];
    const matchesFilter = !normalizedFilter || entry.name.toLowerCase().includes(normalizedFilter);
    if (normalizedFilter && !matchesFilter && childRows.length === 0) continue;
    rows.push({ kind: "entry", key: `entry:${normalizedPath}`, depth, entry: { ...entry, path: normalizedPath } });
    if (normalizedFilter) {
      rows.push(...childRows);
      continue;
    }
    if (!entry.isDirectory || !isTreeDirectoryExpanded(normalizedPath)) continue;
    const node = treeDirectoryNode(normalizedPath);
    if (!node || node.loading) {
      rows.push({ kind: "status", key: `loading:${normalizedPath}`, depth: depth + 1, text: t('fileReader.loadingDirectory') });
    } else if (node.error) {
      rows.push({ kind: "status", key: `error:${normalizedPath}`, depth: depth + 1, text: node.error });
    } else if (node.loaded && node.entries.length === 0) {
      rows.push({ kind: "status", key: `empty:${normalizedPath}`, depth: depth + 1, text: t('fileReader.emptyDirectory') });
    } else if (node.loaded) {
      rows.push(...flattenDirectoryEntries(node.entries, depth + 1));
    }
  }
  return rows;
}

function flattenDirectoryEntriesFromNodes(entries: FileReaderDirectoryEntry[], rootPath: string, nodes: Record<string, DirectoryNode>, depth: number, filter = ""): TreeRow[] {
  const normalizedFilter = filter.trim().toLowerCase();
  const rows: TreeRow[] = [];
  for (const entry of entries) {
    const normalizedPath = normalizePath(entry.path);
    const node = nodes[normalizedPath];
    const isExpanded = node?.expanded;
    const childRows = entry.isDirectory && isExpanded ? flattenDirectoryEntriesFromNodes(nodes[normalizedPath]?.entries || [], normalizedPath, nodes, depth + 1, filter) : [];
    const matchesFilter = !normalizedFilter || entry.name.toLowerCase().includes(normalizedFilter);
    if (normalizedFilter && !matchesFilter && childRows.length === 0) continue;
    rows.push({ kind: "entry", key: `entry:${normalizedPath}`, depth, entry: { ...entry, path: normalizedPath } });
    if (normalizedFilter) {
      rows.push(...childRows);
      continue;
    }
    if (!entry.isDirectory || !isExpanded) continue;
    if (!node) continue;
    if (node.loading) {
      rows.push({ kind: "status", key: `loading:${normalizedPath}`, depth: depth + 1, text: t('fileReader.loadingDirectory') });
    } else if (node.error) {
      rows.push({ kind: "status", key: `error:${normalizedPath}`, depth: depth + 1, text: node.error });
    } else if (node.loaded && node.entries.length === 0) {
      rows.push({ kind: "status", key: `empty:${normalizedPath}`, depth: depth + 1, text: t('fileReader.emptyDirectory') });
    } else if (node.loaded) {
      rows.push(...flattenDirectoryEntriesFromNodes(node.entries, normalizedPath, nodes, depth + 1));
    }
  }
  return rows;
}

// ==================== Lifecycle ====================

onMounted(async () => {
  window.addEventListener("resize", updateAddressScrollState);
  void startFileReaderWatchListener();
  scheduleFileReaderWatchTargetUpdate();
  if (props.enableGlobalDrop === false || !isTauriRuntimeAvailable()) return;
  try {
    unlistenFileDrop = await getCurrentWebview().onDragDropEvent((event) => {
      const payload = event.payload;
      if (payload.type === "enter" || payload.type === "over") {
        fileDragActive.value = true;
        return;
      }
      fileDragActive.value = false;
      if (payload.type === "drop") {
        void openDroppedPaths(payload.paths);
      }
    });
  } catch (error) {
    console.error("[文件阅读器] 注册拖入打开失败", error);
  }
});

onBeforeUnmount(() => {
  window.removeEventListener("resize", updateAddressScrollState);
  if (watchTargetUpdateTimer) window.clearTimeout(watchTargetUpdateTimer);
  if (autoRefreshFileTimer) window.clearTimeout(autoRefreshFileTimer);
  if (autoRefreshDirectoryTimer) window.clearTimeout(autoRefreshDirectoryTimer);
  if (visibleRangeCaptureTimer) window.clearTimeout(visibleRangeCaptureTimer);
  pendingAutoRefreshDirectoryPaths.clear();
  stopFileReaderWatchListener();
  if (isTauriRuntimeAvailable()) {
    void invokeTauri("update_file_reader_watch_targets", {
      input: { sessionId: fileReaderWatchSessionId.value, targets: [] },
    }).catch(() => {});
  }
  unlistenFileDrop?.();
});

// ==================== Expose ====================

defineExpose({
  openPath,
  setActiveTab,
  closeTab,
  openDirectoryTree,
  closeDirectoryTree,
  tabs,
  activePath,
  directoryRootPath,
});
</script>

<style scoped>
.file-reader-address-scroll {
  scrollbar-width: none;
}
.file-reader-address-scroll::-webkit-scrollbar {
  display: none;
}
.file-reader-scroll-container {
  scrollbar-width: none;
}
.file-reader-scroll-container::-webkit-scrollbar {
  display: none;
}
.file-reader-content-scroller {
  scrollbar-width: thin;
  scrollbar-color: color-mix(in srgb, currentColor 28%, transparent) transparent;
}
.file-reader-content-scroller::-webkit-scrollbar {
  display: block;
  width: 10px;
  height: 10px;
}
.file-reader-content-scroller::-webkit-scrollbar-thumb {
  background-color: color-mix(in srgb, currentColor 28%, transparent);
  border-radius: 999px;
}
.file-reader-content-scroller::-webkit-scrollbar-track {
  background: transparent;
}
.file-reader-address-scrollbar-thumb {
  opacity: 0;
  transition: opacity 160ms ease;
}
.file-reader-address-scroll:hover + div .file-reader-address-scrollbar-thumb,
.file-reader-address-scroll:focus-within + div .file-reader-address-scrollbar-thumb {
  opacity: 1;
}
.file-reader-content :deep(.markdown-body),
.file-reader-content :deep(.markstream-body) {
  max-width: none;
}
.file-reader-content :deep(.ecall-markdown-content) {
  min-width: 0;
  max-width: 100%;
  overflow-x: hidden;
  color: inherit;
  font-family: inherit;
  font-size: 0.95rem;
  line-height: 1.65;
}
.file-reader-content :deep(.ecall-markdown-content :where(hr,.hr-node)) {
  margin: 0.75rem 0;
}
.file-reader-content :deep(.ecall-markdown-content :where(table,.table-node)) {
  width: 100%;
  font-size: 0.92rem;
}
.file-reader-raw-main {
  min-height: 0;
}
.file-reader-raw-scroller {
  min-height: 100%;
  overflow: auto;
  background: transparent;
}
.file-reader-raw-pre {
  min-height: 100%;
  margin: 0;
  padding: 0.75rem 1rem;
  white-space: pre;
  color: inherit;
  background: transparent;
}
.file-reader-code-virtual-scroller {
  min-height: 100%;
  overflow: auto;
}
.file-reader-code-virtual-scroller-shiki {
  background: var(--color-base-100);
  scrollbar-width: none;
  -ms-overflow-style: none;
}
.file-reader-code-virtual-scroller-shiki::-webkit-scrollbar {
  width: 0;
  height: 0;
}
.file-reader-code-virtual-canvas {
  position: relative;
  min-width: 100%;
}
.file-reader-code-virtual-row {
  position: absolute;
  left: 0;
  top: 0;
  width: 100%;
}
.file-reader-code-virtual-block {
  display: grid;
  grid-template-columns: calc(var(--file-reader-code-gutter-ch, 2) * 1ch + 1rem) minmax(0, 1fr);
  align-items: flex-start;
  min-width: 100%;
}
.file-reader-code-virtual-block-raw {
  width: 100%;
  font-family: inherit;
  font-size: 14px;
  line-height: 21px;
}
.file-reader-code-virtual-block-shiki {
  width: max-content;
  font-family: inherit;
  font-size: 14px;
  line-height: 21px;
}
.file-reader-code-virtual-gutter {
  position: sticky;
  left: 0;
  z-index: 2;
  width: calc(var(--file-reader-code-gutter-ch, 2) * 1ch + 0.75rem);
  padding-top: 5px;
  padding-bottom: 4px;
  padding-right: 0.25rem;
  text-align: right;
  user-select: none;
}
.file-reader-code-virtual-gutter::after {
  content: "";
  position: absolute;
  top: 0;
  right: -4px;
  width: 4px;
  height: 100%;
}
.file-reader-code-virtual-gutter-raw {
  background: var(--color-base-100);
}
.file-reader-code-virtual-gutter-raw::after {
  background: var(--color-base-100);
}
.file-reader-code-virtual-gutter-shiki {
  background: var(--color-base-100);
}
.file-reader-code-virtual-gutter-shiki::after {
  background: var(--color-base-100);
}
.file-reader-code-virtual-gutter-line {
  min-height: 21px;
  line-height: inherit;
  color: rgb(100 116 139 / 0.92);
}
.file-reader-code-virtual-raw {
  margin: 0;
  padding: 4px 8px;
  min-width: 0;
  flex: 1 1 auto;
  font-family: inherit;
  font-size: inherit;
  line-height: inherit;
}
.file-reader-code-virtual-shiki :deep(pre.shiki) {
  display: block;
  margin: 0;
  padding: 4px 8px;
  min-width: 100%;
  width: max-content;
  box-sizing: border-box;
  font-family: inherit !important;
  font-size: inherit !important;
  line-height: inherit !important;
  background: transparent !important;
  overflow: visible;
}
.file-reader-code-virtual-shiki :deep(pre.shiki code) {
  display: block;
  min-width: max-content;
  font-family: inherit !important;
  font-size: inherit !important;
  line-height: inherit !important;
}
.file-reader-code-virtual-shiki :deep(pre.shiki code .line) {
  display: block;
  min-height: 21px;
  line-height: inherit;
}
.file-reader-code-virtual-shiki :deep(pre.shiki span) {
  font-family: inherit !important;
}
.file-reader-code-virtual-shiki :deep(.file-reader-code-empty-line) {
  display: inline-block;
  width: 0;
  opacity: 0;
  pointer-events: none;
}
</style>

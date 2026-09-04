import { computed, ref, watch, type ComputedRef, type Ref } from "vue";
import { getSingletonHighlighter, hastToHtml, type GrammarState } from "shiki";
import {
  FILE_READER_VIRTUAL_BLOCK_LINE_HEIGHT_PX,
} from "../constants";
import type { FileReaderFileBlockPayload, FileTab, VirtualCodeBlock } from "../types";
import {
  buildFileBlockKey,
  escapeHtml,
  normalizePath,
  resolveShikiLanguage,
  sameNormalizedPath,
  splitContentLines,
} from "../utils";

type UseFileReaderVirtualCodeOptions = {
  activeTab: ComputedRef<FileTab | null>;
  markdownIsDark: ComputedRef<boolean>;
  virtualCodeScroller: Ref<HTMLElement | null>;
  isRawMode: (tab: FileTab | null | undefined) => boolean;
  requestFileBlock: (path: string, startLine: number, lineCount: number) => Promise<FileReaderFileBlockPayload>;
};

type HighlightStateEntry = {
  version: string;
  state?: GrammarState;
};

type ShikiHighlighter = Awaited<ReturnType<typeof getSingletonHighlighter>>;

type ShikiHastNode = {
  type: string;
  tagName?: string;
  properties?: Record<string, unknown>;
  children?: ShikiHastNode[];
};

type ShikiHastElement = ShikiHastNode & {
  type: "element";
  tagName: string;
  properties: Record<string, unknown>;
  children: ShikiHastNode[];
};

type ShikiHastRoot = {
  type: "root";
  children: ShikiHastNode[];
};

const EMPTY_CODE_LINE_HTML = '<span class="file-reader-code-empty-line">&#8203;</span>';

export function useFileReaderVirtualCode(options: UseFileReaderVirtualCodeOptions) {
  const highlightedCodeLinesByBlockKey = ref<Record<string, string[]>>({});
  const fileBlockContentByKey = ref<Record<string, string>>({});
  const fileBlockLoadingByKey = ref<Record<string, boolean>>({});
  const fileBlockErrorByKey = ref<Record<string, string>>({});
  const blockLoadPromises = new Map<string, Promise<string>>();
  const highlighterPromises = new Map<string, Promise<ShikiHighlighter>>();
  const grammarStateByBlockKey = new Map<string, HighlightStateEntry>();
  const highlightedVersionByBlockKey = new Map<string, string>();
  let highlightQueue = Promise.resolve();
  let activeHighlightRefreshId = 0;

  const activeShikiTheme = computed(() => (options.markdownIsDark.value ? "github-dark" : "github-light"));

  const activeVirtualCodeBlocks = computed<VirtualCodeBlock[]>(() => {
    const tab = options.activeTab.value;
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

  // virtua 接管虚拟化后，不再需要 tanstack 的 virtualizer。
  // 保留兼容的 activeVirtualCodeEntries / activeVirtualCodeTotalSize 供旧调用点渐进迁移，
  // 但不再基于虚拟窗口过滤，改为全量块（virtua 在视图层自行裁剪）。
  const activeVirtualCodeEntries = computed(() => {
    return activeVirtualCodeBlocks.value.map((block, index) => ({
      block,
      // 兼容旧结构：row 仅保留 index，start/size 由 virtua 内部管理
      row: { index, start: 0, size: block.lineCount * FILE_READER_VIRTUAL_BLOCK_LINE_HEIGHT_PX } as unknown as { index: number; start: number; size: number },
      lines: blockContentLines(block.key, block.lineCount),
    }));
  });

  const activeVirtualCodeTotalSize = computed(() => {
    // 估算总高度，仅用于非 virtua 兜底；virtua 会自动测量
    return activeVirtualCodeBlocks.value.reduce((acc, b) => acc + b.lineCount * FILE_READER_VIRTUAL_BLOCK_LINE_HEIGHT_PX, 0);
  });

  const virtualCodeLineNumberDigits = computed(() => {
    const totalLines = Math.max(1, options.activeTab.value?.totalLines || 1);
    return Math.max(2, String(totalLines).length);
  });

  function blockContentText(blockKey: string) {
    return fileBlockContentByKey.value[blockKey] || "";
  }

  function escapeCodeLines(content: string) {
    return splitContentLines(content).map((line) => escapeHtml(line) || EMPTY_CODE_LINE_HTML);
  }

  function blockContentLines(blockKey: string, lineCount: number) {
    const lines = highlightedCodeLinesByBlockKey.value[blockKey] || escapeCodeLines(blockContentText(blockKey));
    return Array.from({ length: Math.max(0, lineCount) }, (_, index) => lines[index] || EMPTY_CODE_LINE_HTML);
  }

  function hasClassName(node: ShikiHastElement, className: string) {
    const value = node.properties.class;
    if (typeof value === "string") return value.split(/\s+/).includes(className);
    if (Array.isArray(value)) return value.map(String).includes(className);
    return false;
  }

  function extractShikiLineHtml(root: ShikiHastRoot) {
    const pre = root.children.find((node): node is ShikiHastElement => node.type === "element" && node.tagName === "pre");
    const code = pre?.children?.find((node): node is ShikiHastElement => node.type === "element" && node.tagName === "code");
    const lines = code?.children?.filter((node): node is ShikiHastElement => (
      node.type === "element" && node.tagName === "span" && hasClassName(node as ShikiHastElement, "line")
    )) || [];
    return lines.map((line) => hastToHtml({ type: "root", children: line.children } as Parameters<typeof hastToHtml>[0]) || EMPTY_CODE_LINE_HTML);
  }

  async function renderHighlightedCodeHtml(tab: FileTab, content: string, grammarState?: GrammarState) {
    const language = resolveShikiLanguage(tab.extension);
    const theme = activeShikiTheme.value;
    const highlighter = await getReadyHighlighter(language, theme);
    const root = highlighter.codeToHast(content, {
      lang: language,
      theme,
      ...(grammarState ? { grammarState } : {}),
    });
    return {
      lines: extractShikiLineHtml(root as unknown as ShikiHastRoot),
      grammarState: highlighter.getLastGrammarState(root),
    };
  }

  function getReadyHighlighter(language: string, theme: string) {
    const key = `${language}::${theme}`;
    const existing = highlighterPromises.get(key);
    if (existing) return existing;
    const promise = getSingletonHighlighter({ langs: [language], themes: [theme] });
    highlighterPromises.set(key, promise);
    return promise;
  }

  function highlightVersion(tab: FileTab) {
    return [
      normalizePath(tab.path),
      tab.extension,
      tab.totalLines,
      tab.blockLineCount,
      activeShikiTheme.value,
    ].join("::");
  }

  async function loadVirtualCodeBlockContent(block: VirtualCodeBlock): Promise<string> {
    const existing = fileBlockContentByKey.value[block.key];
    if (existing !== undefined) return existing;
    const pending = blockLoadPromises.get(block.key);
    if (pending) return pending;

    const promise = (async () => {
      fileBlockLoadingByKey.value = { ...fileBlockLoadingByKey.value, [block.key]: true };
      try {
        const payload = await options.requestFileBlock(block.path, block.startLine, block.lineCount);
        const normalizedKey = buildFileBlockKey(payload.path || block.path, payload.startLine, payload.endLine);
        const content = String(payload.content || "");
        fileBlockContentByKey.value = { ...fileBlockContentByKey.value, [normalizedKey]: content };
        const errorNext = { ...fileBlockErrorByKey.value };
        delete errorNext[block.key];
        fileBlockErrorByKey.value = errorNext;
        return content;
      } catch (error) {
        fileBlockErrorByKey.value = { ...fileBlockErrorByKey.value, [block.key]: error instanceof Error ? error.message : String(error) };
        throw error;
      } finally {
        const loadingNext = { ...fileBlockLoadingByKey.value };
        delete loadingNext[block.key];
        fileBlockLoadingByKey.value = loadingNext;
        blockLoadPromises.delete(block.key);
      }
    })();

    blockLoadPromises.set(block.key, promise);
    return promise;
  }

  async function queueHighlightedThroughBlock(tab: FileTab, targetBlock: VirtualCodeBlock) {
    const refreshId = activeHighlightRefreshId;
    const queued = highlightQueue.catch(() => {}).then(async () => {
      if (refreshId !== activeHighlightRefreshId) return;
      await ensureHighlightedThroughBlock(tab, targetBlock, refreshId);
    });
    highlightQueue = queued.catch(() => {});
    await queued;
  }

  async function ensureHighlightedThroughBlock(tab: FileTab, targetBlock: VirtualCodeBlock, refreshId: number) {
    if (options.isRawMode(tab) || tab.kind === "markdown") return;
    const version = highlightVersion(tab);
    if (highlightedVersionByBlockKey.get(targetBlock.key) === version) return;

    const blocks = activeVirtualCodeBlocks.value;
    const targetIndex = blocks.findIndex((item) => item.key === targetBlock.key);
    if (targetIndex < 0) return;

    let startIndex = 0;
    let grammarState: GrammarState | undefined;
    for (let index = targetIndex - 1; index >= 0; index -= 1) {
      const entry = grammarStateByBlockKey.get(blocks[index].key);
      if (entry?.version === version) {
        startIndex = index + 1;
        grammarState = entry.state;
        break;
      }
    }

    for (let index = startIndex; index <= targetIndex; index += 1) {
      const block = blocks[index];
      if (!block) continue;
      const currentActiveTab = options.activeTab.value;
      if (refreshId !== activeHighlightRefreshId || !currentActiveTab || !sameNormalizedPath(currentActiveTab.path, tab.path)) return;
      if (highlightedVersionByBlockKey.get(block.key) === version) {
        grammarState = grammarStateByBlockKey.get(block.key)?.state;
        continue;
      }

      const content = await loadVirtualCodeBlockContent(block);
      if (refreshId !== activeHighlightRefreshId) return;

      try {
        const result = await renderHighlightedCodeHtml(tab, content, grammarState);
        highlightedCodeLinesByBlockKey.value = {
          ...highlightedCodeLinesByBlockKey.value,
          [block.key]: result.lines,
        };
        highlightedVersionByBlockKey.set(block.key, version);
        grammarStateByBlockKey.set(block.key, { version, state: result.grammarState });
        grammarState = result.grammarState;
      } catch {
        highlightedCodeLinesByBlockKey.value = {
          ...highlightedCodeLinesByBlockKey.value,
          [block.key]: escapeCodeLines(content),
        };
        highlightedVersionByBlockKey.set(block.key, version);
        grammarStateByBlockKey.set(block.key, { version, state: undefined });
        grammarState = undefined;
      }
    }
  }

  async function ensureVirtualCodeBlockLoaded(block: VirtualCodeBlock) {
    if (!block.path) return;
    const tab = options.activeTab.value;
    if (!tab || !sameNormalizedPath(tab.path, block.path)) return;
    try {
      await loadVirtualCodeBlockContent(block);
      await queueHighlightedThroughBlock(tab, block);
    } catch {
      // loadVirtualCodeBlockContent records the visible block error state.
    }
  }

  function clearFileBlockCaches(path: string) {
    activeHighlightRefreshId += 1;
    const normalizedPath = normalizePath(path);
    if (!normalizedPath) return;
    const contentNext = { ...fileBlockContentByKey.value };
    const loadingNext = { ...fileBlockLoadingByKey.value };
    const errorNext = { ...fileBlockErrorByKey.value };
    const linesNext = { ...highlightedCodeLinesByBlockKey.value };
    for (const key of new Set([
      ...Object.keys(contentNext),
      ...Object.keys(loadingNext),
      ...Object.keys(errorNext),
      ...Object.keys(linesNext),
      ...Array.from(highlightedVersionByBlockKey.keys()),
      ...Array.from(grammarStateByBlockKey.keys()),
    ])) {
      if (!key.startsWith(`${normalizedPath}::`)) continue;
      delete contentNext[key];
      delete loadingNext[key];
      delete errorNext[key];
      delete linesNext[key];
      blockLoadPromises.delete(key);
      highlightedVersionByBlockKey.delete(key);
      grammarStateByBlockKey.delete(key);
    }
    fileBlockContentByKey.value = contentNext;
    fileBlockLoadingByKey.value = loadingNext;
    fileBlockErrorByKey.value = errorNext;
    highlightedCodeLinesByBlockKey.value = linesNext;
  }

  function resetVirtualCodeCaches() {
    activeHighlightRefreshId += 1;
    highlightedCodeLinesByBlockKey.value = {};
    fileBlockContentByKey.value = {};
    fileBlockLoadingByKey.value = {};
    fileBlockErrorByKey.value = {};
    blockLoadPromises.clear();
    grammarStateByBlockKey.clear();
    highlightedVersionByBlockKey.clear();
  }

  function migrateVirtualCodeCaches(fromPath: string, toPath: string) {
    const normalizedFromPath = normalizePath(fromPath);
    const normalizedToPath = normalizePath(toPath);
    if (!normalizedFromPath || !normalizedToPath || normalizedFromPath === normalizedToPath) return;
    const fromPrefix = `${normalizedFromPath}::`;

    const contentNext = migrateRecordKeys(fileBlockContentByKey.value, fromPrefix, normalizedToPath);
    const loadingNext = migrateRecordKeys(fileBlockLoadingByKey.value, fromPrefix, normalizedToPath);
    const errorNext = migrateRecordKeys(fileBlockErrorByKey.value, fromPrefix, normalizedToPath);
    const linesNext = migrateRecordKeys(highlightedCodeLinesByBlockKey.value, fromPrefix, normalizedToPath);

    migrateMapKeys(grammarStateByBlockKey, fromPrefix, normalizedToPath);
    migrateMapKeys(highlightedVersionByBlockKey, fromPrefix, normalizedToPath);
    for (const key of Array.from(blockLoadPromises.keys())) {
      if (key.startsWith(fromPrefix)) blockLoadPromises.delete(key);
    }

    fileBlockContentByKey.value = contentNext;
    fileBlockLoadingByKey.value = loadingNext;
    fileBlockErrorByKey.value = errorNext;
    highlightedCodeLinesByBlockKey.value = linesNext;
  }

  function migrateRecordKeys<T>(record: Record<string, T>, fromPrefix: string, toPath: string) {
    const next = { ...record };
    for (const key of Object.keys(next)) {
      if (!key.startsWith(fromPrefix)) continue;
      const suffix = key.slice(fromPrefix.length);
      next[`${toPath}::${suffix}`] = next[key];
      delete next[key];
    }
    return next;
  }

  function migrateMapKeys<T>(map: Map<string, T>, fromPrefix: string, toPath: string) {
    for (const key of Array.from(map.keys())) {
      if (!key.startsWith(fromPrefix)) continue;
      const value = map.get(key);
      if (value === undefined) continue;
      const suffix = key.slice(fromPrefix.length);
      map.set(`${toPath}::${suffix}`, value);
      map.delete(key);
    }
  }

  async function refreshActiveCodeHighlights() {
    const active = options.activeTab.value;
    if (!active || options.isRawMode(active) || active.kind === "markdown") return;
    activeHighlightRefreshId += 1;
    clearHighlightCachesForPath(active.path);
    // virtua 渲染后会按需触发 ensureVirtualCodeBlockLoaded，此处不再遍历全量 visible
  }

  function clearHighlightCachesForPath(path: string) {
    const normalizedPath = normalizePath(path);
    if (!normalizedPath) return;
    const linesNext = { ...highlightedCodeLinesByBlockKey.value };
    for (const key of new Set([
      ...Object.keys(linesNext),
      ...Array.from(highlightedVersionByBlockKey.keys()),
      ...Array.from(grammarStateByBlockKey.keys()),
    ])) {
      if (!key.startsWith(`${normalizedPath}::`)) continue;
      delete linesNext[key];
      highlightedVersionByBlockKey.delete(key);
      grammarStateByBlockKey.delete(key);
    }
    highlightedCodeLinesByBlockKey.value = linesNext;
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

  function measureVirtualCodeRow(_element: Element | { $el?: Element } | null) {
    // virtua 通过 ResizeObserver 自动测量，无需手工 measureElement
  }

  function remeasureVirtualCodeRows() {
    // virtua 自动处理；换行模式切换时由 Virtualizer 重新测量
  }

  watch(activeShikiTheme, () => {
    void refreshActiveCodeHighlights();
  });

  return {
    activeVirtualCodeBlocks,
    activeVirtualCodeEntries,
    activeVirtualCodeTotalSize,
    virtualCodeLineNumberDigits,
    blockContentText,
    blockContentLines,
    clearFileBlockCaches,
    resetVirtualCodeCaches,
    migrateVirtualCodeCaches,
    collectVirtualizedVisibleContent,
    measureVirtualCodeRow,
    remeasureVirtualCodeRows,
    ensureVirtualCodeBlockLoaded,
  };
}

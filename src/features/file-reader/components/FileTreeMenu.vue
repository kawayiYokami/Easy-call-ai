<script setup lang="ts">
import type { FileReaderDirectoryEntry } from "../types";
import { normalizePath } from "../utils";

type DirectoryNodeLike = {
  loaded?: boolean;
  loading?: boolean;
  error?: string;
  expanded?: boolean;
  entries?: FileReaderDirectoryEntry[];
};

const props = withDefaults(defineProps<{
  entries: FileReaderDirectoryEntry[];
  /** 目录节点访问器：主树传 directoryNodes，hover 树传 hoverDirectoryTreeNodes */
  nodeFor: (path: string) => DirectoryNodeLike | null;
  /** 目录当前是否展开（供目录图标取开合态） */
  expanded: (path: string) => boolean;
  /** 条目图标解析：entry + 目录展开态 → icon data url */
  iconFor: (entry: FileReaderDirectoryEntry, expanded: boolean) => string;
  /** details 原生切换回调：open 为切换后的目标状态 */
  onToggleDirectory: (entry: FileReaderDirectoryEntry, open: boolean) => void;
  /** 文件条目点击 */
  onOpenEntry: (entry: FileReaderDirectoryEntry) => void;
  /** 条目右键菜单 */
  onContextMenu?: (path: string, event: MouseEvent) => void;
  /** 选中高亮路径（当前打开文件） */
  activePath?: string;
  /** 过滤模式下强制展开所有目录、隐藏状态行（与扁平过滤行为一致） */
  forceOpen?: boolean;
  /** 过滤关键字：非空时逐层保留匹配项与祖先链（含子目录递归） */
  filter?: string;
  /** 本 ul 所属目录的节点状态：非空时在顶部渲染 loading/error 行（根调用不传） */
  statusNode?: DirectoryNodeLike | null;
  loadingText?: string;
  depth?: number;
}>(), {
  onContextMenu: undefined,
  activePath: "",
  forceOpen: false,
  filter: "",
  statusNode: null,
  loadingText: "读取目录中…",
  depth: 0,
});

function nodeOf(entry: FileReaderDirectoryEntry) {
  return props.nodeFor(normalizePath(entry.path));
}

function isExpanded(entry: FileReaderDirectoryEntry) {
  return props.expanded(normalizePath(entry.path));
}

/** 递归过滤：保留名称匹配项与含匹配后代的目录链；空过滤条件原样返回 */
function filteredEntries(entries: FileReaderDirectoryEntry[]): FileReaderDirectoryEntry[] {
  const filter = props.filter.trim().toLowerCase();
  if (!filter) return entries;
  const result: FileReaderDirectoryEntry[] = [];
  for (const entry of entries) {
    if (!entry.isDirectory) {
      if (entry.name.toLowerCase().includes(filter)) result.push(entry);
      continue;
    }
    const children = filteredEntries(nodeOf(entry)?.entries || []);
    if (entry.name.toLowerCase().includes(filter) || children.length > 0) result.push(entry);
  }
  return result;
}

function childEntries(entry: FileReaderDirectoryEntry) {
  const node = nodeOf(entry);
  return filteredEntries(node?.loaded ? (node.entries || []) : []);
}

function isActive(entry: FileReaderDirectoryEntry) {
  if (entry.isDirectory || !props.activePath) return false;
  return normalizePath(entry.path) === normalizePath(props.activePath);
}

function handleToggle(entry: FileReaderDirectoryEntry, event: Event) {
  if (!(event.target instanceof HTMLDetailsElement)) return;
  props.onToggleDirectory(entry, event.target.open);
}
</script>

<template>
  <ul :class="depth === 0 ? 'menu menu-xs w-full flex-nowrap p-0' : ''">
    <li v-if="!forceOpen && statusNode?.loading" class="gap-2 text-xs">
      <span class="loading loading-spinner loading-xs"></span>
      <span>{{ loadingText }}</span>
    </li>
    <li v-else-if="!forceOpen && statusNode?.error" class="gap-2 text-xs text-error">
      <span class="truncate">{{ statusNode.error }}</span>
    </li>
    <template v-else>
      <li v-for="entry in filteredEntries(entries)" :key="normalizePath(entry.path)">
      <details
        v-if="entry.isDirectory"
        :open="forceOpen || isExpanded(entry)"
        @toggle="handleToggle(entry, $event)"
      >
        <summary>
          <img
            :src="iconFor(entry, isExpanded(entry))"
            alt=""
            class="file-reader-tree-icon h-4 w-4 shrink-0 object-contain"
          />
          <span class="min-w-0 flex-1 truncate">{{ entry.name }}</span>
        </summary>
        <FileTreeMenu
          :entries="childEntries(entry)"
          :node-for="nodeFor"
          :expanded="expanded"
          :icon-for="iconFor"
          :on-toggle-directory="onToggleDirectory"
          :on-open-entry="onOpenEntry"
          :on-context-menu="onContextMenu"
          :active-path="activePath"
          :force-open="forceOpen"
          :filter="filter"
          :status-node="nodeOf(entry)"
          :loading-text="loadingText"
          :depth="depth + 1"
        />
      </details>
      <a
        v-else
        :data-tree-path="normalizePath(entry.path)"
        :class="isActive(entry) ? 'bg-primary/10 text-primary' : ''"
        @click="onOpenEntry(entry)"
        @contextmenu.prevent.stop="onContextMenu?.(normalizePath(entry.path), $event)"
      >
        <img
          :src="iconFor(entry, false)"
          alt=""
          class="file-reader-tree-icon h-4 w-4 shrink-0 object-contain"
        />
        <span class="min-w-0 flex-1 truncate">{{ entry.name }}</span>
      </a>
      </li>
    </template>
  </ul>
</template>
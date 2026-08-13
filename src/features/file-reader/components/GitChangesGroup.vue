<template>
  <div class="px-1">
    <div class="mb-0.5 flex h-6 items-center gap-1 px-1.5">
      <button
        type="button"
        class="btn btn-ghost btn-xs h-5 min-h-5 w-5 shrink-0 px-0"
        :title="collapsed ? expandTitle : collapseTitle"
        @click="collapsed = !collapsed"
      >
        <ChevronDown v-if="!collapsed" class="h-3.5 w-3.5" />
        <ChevronRight v-else class="h-3.5 w-3.5" />
      </button>
      <button
        type="button"
        class="flex min-w-0 flex-1 items-center gap-1 truncate text-left text-xs font-medium opacity-70"
        @click="collapsed = !collapsed"
      >
        {{ title }}
      </button>
      <button
        v-if="entries.length > 0"
        type="button"
        class="btn btn-ghost btn-xs h-5 min-h-5 px-1.5 text-[11px]"
        :disabled="busy"
        @click="emit('action', entries.map((entry) => entry.path))"
      >
        {{ actionTitle }}
      </button>
      <span class="shrink-0 text-[11px] tabular-nums opacity-50">{{
        totalCount !== undefined && totalCount > 1000 ? "1000+" : (totalCount ?? entries.length)
      }}</span>
    </div>
    <div v-show="!collapsed">
      <div
        v-for="row in visibleRows"
        :key="row.key"
        class="group flex h-7 items-center gap-1 rounded px-1.5"
        :class="row.item.kind === 'file' && row.item.path === props.highlightPath ? 'bg-primary/10 text-primary' : 'hover:bg-base-300/40'"
        :style="{ paddingLeft: `${6 + row.depth * 14}px` }"
        @contextmenu="onRowContextMenu($event)"
      >
      <template v-if="row.item.kind === 'dir'">
        <button
          type="button"
          class="btn btn-ghost btn-xs h-5 min-h-5 w-5 shrink-0 px-0"
          :title="collapsedPaths.has(row.item.path) ? expandTitle : collapseTitle"
          @click.stop="toggleDirectory(row.item.path)"
        >
          <ChevronDown v-if="!collapsedPaths.has(row.item.path)" class="h-3.5 w-3.5" />
          <ChevronRight v-else class="h-3.5 w-3.5" />
        </button>
        <button
          type="button"
          class="flex min-w-0 flex-1 items-center gap-1.5 rounded px-1 py-0.5 text-left text-xs font-medium"
          :title="row.item.path"
          @click="toggleDirectory(row.item.path)"
        >
          <Folder class="h-4 w-4 shrink-0 opacity-70" />
          <span class="min-w-0 truncate">{{ row.item.name }}</span>
        </button>
        <span class="hidden shrink-0 items-center gap-0.5 group-hover:flex focus-within:flex">
          <button
            type="button"
            class="btn btn-ghost btn-xs h-5 min-h-5 w-5 px-0 text-error/70"
            :title="discardTitle"
            :disabled="busy"
            @click="emit('discard', collectDirPaths(row.item))"
          >
            <Undo2 class="h-3 w-3" />
          </button>
          <button
            type="button"
            class="btn btn-ghost btn-xs h-5 min-h-5 w-5 px-0"
            :title="actionTitle"
            :disabled="busy"
            @click="emit('action', collectDirPaths(row.item))"
          >
            <Plus v-if="actionKind === 'stage'" class="h-3 w-3" />
            <Minus v-else class="h-3 w-3" />
          </button>
        </span>
      </template>
      <template v-else>
        <span v-if="mode === 'tree'" class="h-5 w-5 shrink-0"></span>
        <button
          type="button"
          class="flex min-w-0 flex-1 items-center gap-1.5 text-left text-xs"
          :title="row.item.path"
          @click="emit('openDiff', { path: row.item.path, staged: actionKind === 'unstage' })"
        >
          <span class="min-w-0 truncate">{{ row.item.name }}</span>
        </button>
        <span
          class="hidden shrink-0 items-center gap-0.5 group-hover:flex focus-within:flex"
        >
          <button
            type="button"
            class="btn btn-ghost btn-xs h-5 min-h-5 w-5 px-0 text-error/70"
            :title="discardTitle"
            :disabled="busy"
            @click="emit('discard', [row.item.path])"
          >
            <Undo2 class="h-3 w-3" />
          </button>
          <button
            type="button"
            class="btn btn-ghost btn-xs h-5 min-h-5 w-5 px-0"
            :title="actionTitle"
            :disabled="busy"
            @click="emit('action', [row.item.path])"
          >
            <Plus v-if="actionKind === 'stage'" class="h-3 w-3" />
            <Minus v-else class="h-3 w-3" />
          </button>
        </span>
        <span
          class="ml-auto shrink-0 font-mono text-[10px] font-bold"
          :class="statusClass(row.item.entry)"
          :title="statusTitle(row.item.entry)"
        >{{ statusLabel(row.item.entry) }}</span>
      </template>
      </div>
    </div>

    <!-- 右键菜单：折叠全部（仅树状模式） -->
    <div
      v-if="contextMenu.visible && mode === 'tree'"
      class="fixed inset-0 z-50"
      @click="closeContextMenu"
      @contextmenu.prevent="closeContextMenu"
    ></div>
    <div
      v-if="contextMenu.visible && mode === 'tree'"
      class="fixed z-50 w-40 overflow-hidden rounded-lg border border-base-300 bg-base-100 py-1 shadow-xl"
      :style="{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }"
      @click.stop
      @contextmenu.prevent
    >
      <button
        type="button"
        class="flex w-full items-center gap-2 px-3 py-1.5 text-left text-xs hover:bg-base-300/40"
        @click="collapseAllDirectories"
      >
        <ChevronDown class="h-3.5 w-3.5 opacity-60" />
        {{ collapseAllTitle }}
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { ChevronDown, ChevronRight, Folder, Minus, Plus, Undo2 } from "@lucide/vue";
import type { GitPanelStatusEntry } from "../../../services/tauri-api";

const props = withDefaults(defineProps<{
  title: string;
  entries: GitPanelStatusEntry[];
  busy?: boolean;
  actionKind: "stage" | "unstage";
  actionTitle: string;
  discardTitle: string;
  expandTitle?: string;
  collapseTitle?: string;
  collapseAllTitle?: string;
  highlightPath?: string;
  /** 展示模式：tree 按目录折叠分组；list 平铺全部文件（VSCode 风格） */
  mode?: "tree" | "list";
  /** 截断前的组内实际数量；不传时回退 entries.length（未截断场景） */
  totalCount?: number;
}>(), {
  busy: false,
  expandTitle: "",
  collapseTitle: "",
  collapseAllTitle: "",
  highlightPath: "",
  mode: "tree",
  totalCount: undefined,
});

const emit = defineEmits<{
  (e: "openDiff", payload: { path: string; staged: boolean }): void;
  (e: "action", paths: string[]): void;
  (e: "discard", paths: string[]): void;
}>();

type GitTreeItem =
  | { kind: "file"; name: string; path: string; entry: GitPanelStatusEntry }
  | { kind: "dir"; name: string; path: string; children: GitTreeItem[] };

type GitTreeRow = { key: string; depth: number; item: GitTreeItem };

// 折叠的目录路径集合；默认全部展开
const collapsedPaths = ref<Set<string>>(new Set());

// 分组整体折叠状态（暂存/更改各自独立）
const collapsed = ref(false);

// 折叠分组时关闭可能残留的右键菜单
watch(collapsed, (value) => {
  if (value) closeContextMenu();
});

function buildTree(entries: GitPanelStatusEntry[]): GitTreeItem[] {
  const root: GitTreeItem[] = [];
  for (const entry of entries) {
    const segments = entry.path.split("/");
    let siblings = root;
    let acc = "";
    for (let i = 0; i < segments.length; i++) {
      const segment = segments[i];
      acc = acc ? `${acc}/${segment}` : segment;
      if (i === segments.length - 1) {
        siblings.push({ kind: "file", name: segment, path: acc, entry });
        continue;
      }
      let dir = siblings.find(
        (item): item is Extract<GitTreeItem, { kind: "dir" }> =>
          item.kind === "dir" && item.name === segment,
      );
      if (!dir) {
        dir = { kind: "dir", name: segment, path: acc, children: [] };
        siblings.push(dir);
      }
      siblings = dir.children;
    }
  }
  return root;
}

const visibleRows = computed<GitTreeRow[]>(() => {
  if (props.mode === "list") {
    // 列表模式：平铺全部文件，depth 0，不折叠
    return props.entries.map((entry) => ({
      key: entry.path,
      depth: 0,
      item: { kind: "file", name: entry.path, path: entry.path, entry },
    }));
  }
  const rows: GitTreeRow[] = [];
  const walk = (items: GitTreeItem[], depth: number) => {
    for (const item of items) {
      rows.push({ key: `${depth}:${item.path}`, depth, item });
      if (item.kind === "dir" && !collapsedPaths.value.has(item.path)) {
        walk(item.children, depth + 1);
      }
    }
  };
  walk(buildTree(props.entries), 0);
  return rows;
});

function toggleDirectory(path: string) {
  const next = new Set(collapsedPaths.value);
  if (next.has(path)) {
    next.delete(path);
  } else {
    next.add(path);
  }
  collapsedPaths.value = next;
}

// ==================== 右键菜单 ====================
const contextMenu = ref<{ visible: boolean; x: number; y: number }>({ visible: false, x: 0, y: 0 });

function openContextMenu(event: MouseEvent) {
  contextMenu.value = { visible: true, x: event.clientX, y: event.clientY };
}

/** 行右键：树状模式打开折叠菜单；列表模式不拦截，走浏览器默认菜单 */
function onRowContextMenu(event: MouseEvent) {
  if (props.mode !== "tree") return;
  event.preventDefault();
  openContextMenu(event);
}

function closeContextMenu() {
  contextMenu.value.visible = false;
}

// 一键折叠全部目录：收集所有目录路径放入 collapsedPaths
function collapseAllDirectories() {
  const next = new Set(collapsedPaths.value);
  const collect = (items: GitTreeItem[]) => {
    for (const item of items) {
      if (item.kind === "dir") {
        next.add(item.path);
        collect(item.children);
      }
    }
  };
  collect(buildTree(props.entries));
  collapsedPaths.value = next;
  closeContextMenu();
}

// 收集目录下所有文件路径（含子目录），供文件夹级暂存/取消暂存使用
function collectDirPaths(dir: Extract<GitTreeItem, { kind: "dir" }>): string[] {
  const paths: string[] = [];
  const walk = (items: GitTreeItem[]) => {
    for (const item of items) {
      if (item.kind === "file") {
        paths.push(item.path);
      } else {
        walk(item.children);
      }
    }
  };
  walk(dir.children);
  return paths;
}

function statusLabel(entry: GitPanelStatusEntry) {
  const staged = entry.stagedStatus.trim();
  const unstaged = entry.unstagedStatus.trim();
  if (staged === "?" && unstaged === "?") return "U";
  const code = props.actionKind === "unstage" ? staged : unstaged;
  return code || "M";
}

/** 状态码悬停提示：M 修改 / A 新增 / D 删除 / U 未跟踪 / R 重命名 */
function statusTitle(entry: GitPanelStatusEntry) {
  const label = statusLabel(entry);
  const names: Record<string, string> = {
    M: "Modified",
    A: "Added",
    D: "Deleted",
    U: "Untracked",
    R: "Renamed",
  };
  return names[label] || label;
}

function statusClass(entry: GitPanelStatusEntry) {
  const label = statusLabel(entry);
  if (label === "A" || label === "U") return "text-success";
  if (label === "D") return "text-error";
  return "text-warning";
}
</script>

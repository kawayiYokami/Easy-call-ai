<template>
  <div class="flex flex-col">
    <div
      v-for="row in visibleRows"
      :key="row.key"
      class="group"
    >
      <!-- 统一行容器：高度 / 悬停高亮 / 圆角 / 缩进 / 点击语义全在此，使用方只填内容 -->
      <div
        class="flex h-7 items-center gap-1 rounded px-1.5"
        :class="[rowClass(row), row.node.rowClass]"
        :style="row.node.interactive === false ? undefined : { paddingLeft: `${basePad + row.depth * indent}px` }"
        :title="row.node.title"
        @click="onRowClick(row, $event)"
        @contextmenu="onRowContextMenu(row, $event)"
      >
        <slot
          name="row"
          :row="row"
          :depth="row.depth"
          :expanded="expandedSet.has(row.key)"
          :toggle="() => toggleRow(row)"
          :selected="selectedSet.has(row.key)"
          :select="(event?: MouseEvent) => selectRow(row, event)"
          :has-children="hasChildren(row.node)"
        />
      </div>
    </div>
    <div v-if="visibleRows.length === 0">
      <slot name="empty" />
    </div>
  </div>
</template>

<script setup lang="ts" generic="T">
import { computed, ref, watch } from "vue";

/** 通用树节点：key 唯一标识，data 携带业务数据，children 可选 */
export interface GitTreeNode<T = unknown> {
  key: string;
  data: T;
  children?: GitTreeNode<T>[];
  /** 无 children 但可展开（懒加载占位，展开时触发 expand 事件由调用方填充） */
  expandable?: boolean;
  /** 行悬停提示 */
  title?: string;
  /** 不可交互行：无 hover 高亮、不响应点击/右键（如分组标题） */
  interactive?: boolean;
  /** 数据声明的行级高亮类（如当前分支），追加到行容器 */
  rowClass?: string;
}

/** 拍平后的行：depth 为层级深度（list 模式恒为 0） */
export interface GitTreeFlatRow<T = unknown> {
  key: string;
  depth: number;
  node: GitTreeNode<T>;
}

/** GitTree 暴露给调用方的方法（供 ref 调用） */
export interface GitTreeExpose {
  clearSelection(): void;
  /** 折叠所有可展开节点；keepRoots 为 true 时保留根节点展开（根常为分组头） */
  collapseAll(keepRoots?: boolean): void;
  expandAll(): void;
}

const props = withDefaults(defineProps<{
  nodes: GitTreeNode<T>[];
  /** tree 按层级缩进展示；list 平铺所有叶子节点（忽略层级） */
  mode?: "tree" | "list";
  /** 初始是否展开所有可展开节点 */
  defaultExpanded?: boolean;
  /** 是否允许选中行（选中集由组件维护，通过 update:selectedKeys 同步） */
  selectable?: boolean;
  /** selectable 时是否支持 Shift 范围多选 */
  multiple?: boolean;
  /** 每级缩进像素 */
  indent?: number;
  /** 行基础左缩进像素 */
  basePad?: number;
}>(), {
  mode: "tree",
  defaultExpanded: false,
  selectable: false,
  multiple: false,
  indent: 14,
  basePad: 6,
});

const emit = defineEmits<{
  /** 普通点击（非 Shift）触发；multiple 的 Shift 范围选择不触发 */
  (e: "select", key: string): void;
  /** 懒加载节点被展开且尚无 children 时触发，调用方异步填充 children */
  (e: "expand", key: string): void;
  (e: "update:selectedKeys", keys: string[]): void;
  /** 叶子节点（不可展开且非 selectable）被点击时触发 */
  (e: "rowClick", row: GitTreeFlatRow<T>, event: MouseEvent): void;
  /** 行右键（interactive 节点） */
  (e: "contextmenu", row: GitTreeFlatRow<T>, event: MouseEvent): void;
}>();

// ==================== 展开状态 ====================
const expandedSet = ref<Set<string>>(new Set());
/** 用户主动 toggle 过的节点：之后不再受 defaultExpanded 自动展开影响 */
const userToggledKeys = ref<Set<string>>(new Set());

function hasChildren(node: GitTreeNode<T>): boolean {
  return (node.children?.length ?? 0) > 0;
}

/**
 * defaultExpanded 时，为「尚未出现过的可展开节点」补充展开状态。
 * 挂在 watch 上：异步加载的节点首次出现时自动展开；用户主动折叠过的节点
 * （userToggledKeys 命中）保持不变。已有展开状态不被重置。
 */
function syncDefaultExpanded(items: GitTreeNode<T>[]) {
  if (!props.defaultExpanded) return;
  const next = new Set(expandedSet.value);
  const collect = (nodes: GitTreeNode<T>[]) => {
    for (const node of nodes) {
      if ((hasChildren(node) || node.expandable) && !userToggledKeys.value.has(node.key)) {
        next.add(node.key);
      }
      if (hasChildren(node)) collect(node.children!);
    }
  };
  collect(items);
  expandedSet.value = next;
}

watch(() => props.nodes, (nodes) => syncDefaultExpanded(nodes), { immediate: true });

// ==================== 行拍平 ====================
const visibleRows = computed<GitTreeFlatRow<T>[]>(() => {
  if (props.mode === "list") {
    // 平铺模式：根节点（分组头）始终显示；非根只渲染叶子，忽略层级
    const rows: GitTreeFlatRow<T>[] = [];
    const walk = (items: GitTreeNode<T>[], depth: number) => {
      for (const node of items) {
        if (depth === 0) {
          rows.push({ key: node.key, depth: 0, node });
          if (hasChildren(node) && expandedSet.value.has(node.key)) {
            walk(node.children!, 1);
          }
        } else if (hasChildren(node)) {
          walk(node.children!, depth + 1);
        } else {
          rows.push({ key: node.key, depth: 0, node });
        }
      }
    };
    walk(props.nodes, 0);
    return rows;
  }
  const rows: GitTreeFlatRow<T>[] = [];
  const walk = (items: GitTreeNode<T>[], depth: number) => {
    for (const node of items) {
      rows.push({ key: node.key, depth, node });
      if (hasChildren(node) && expandedSet.value.has(node.key)) {
        walk(node.children!, depth + 1);
      }
    }
  };
  walk(props.nodes, 0);
  return rows;
});

function toggleRow(row: GitTreeFlatRow<T>) {
  // 用户主动操作过的节点，之后不受 defaultExpanded 自动展开影响
  userToggledKeys.value.add(row.key);
  const isExpanded = expandedSet.value.has(row.key);
  if (!isExpanded && !hasChildren(row.node) && row.node.expandable) {
    // 懒加载占位节点：展开时通知调用方填充子节点
    emit("expand", row.key);
  }
  const next = new Set(expandedSet.value);
  if (isExpanded) {
    next.delete(row.key);
  } else {
    next.add(row.key);
  }
  expandedSet.value = next;
}

// ==================== 行容器交互 ====================
/** 行样式：选中高亮优先，其次悬停高亮；不可交互行无样式 */
function rowClass(row: GitTreeFlatRow<T>) {
  if (row.node.interactive === false) return "";
  if (props.selectable && selectedSet.value.has(row.key)) {
    return "bg-primary/20 text-primary";
  }
  return "hover:bg-base-300/40";
}

function onRowClick(row: GitTreeFlatRow<T>, event: MouseEvent) {
  if (row.node.interactive === false) return;
  if (hasChildren(row.node) || row.node.expandable) {
    // 可展开节点：整行点击 = 展开/折叠
    toggleRow(row);
  } else if (props.selectable) {
    // 叶子 + 可选中：整行点击 = 选中（select 内部处理 Shift 范围）
    selectRow(row, event);
  } else {
    // 叶子 + 不可选中：交给使用方
    emit("rowClick", row, event);
  }
}

function onRowContextMenu(row: GitTreeFlatRow<T>, event: MouseEvent) {
  if (row.node.interactive === false) return;
  emit("contextmenu", row, event);
}

// ==================== 选择 ====================
const selectedSet = ref<Set<string>>(new Set());
const selectionAnchor = ref("");

/** 当前可见的叶子节点 key 序列（Shift 范围选择按此顺序） */
const leafKeys = computed(() =>
  visibleRows.value.filter((row) => !hasChildren(row.node)).map((row) => row.key),
);

function selectRow(row: GitTreeFlatRow<T>, event?: MouseEvent) {
  if (!props.selectable) {
    emit("select", row.key);
    return;
  }
  if (!props.multiple || !event?.shiftKey) {
    // 普通点击：单选并作为 Shift 范围锚点
    selectedSet.value = new Set([row.key]);
    selectionAnchor.value = row.key;
    emit("update:selectedKeys", [...selectedSet.value]);
    emit("select", row.key);
    return;
  }
  // Shift：以最后一次普通点击为锚点做范围选择
  const keys = leafKeys.value;
  const currentIndex = keys.indexOf(row.key);
  if (currentIndex < 0) return;
  const anchorIndex = selectionAnchor.value ? keys.indexOf(selectionAnchor.value) : -1;
  const start = anchorIndex >= 0 ? Math.min(anchorIndex, currentIndex) : currentIndex;
  const end = Math.max(anchorIndex, currentIndex);
  const range = keys.slice(start, end + 1);
  selectedSet.value = new Set(range);
  if (selectionAnchor.value === "") selectionAnchor.value = row.key;
  emit("update:selectedKeys", [...selectedSet.value]);
}

function clearSelection() {
  selectedSet.value = new Set();
  selectionAnchor.value = "";
  emit("update:selectedKeys", []);
}

// ==================== 展开/折叠全部 ====================
function collapseAll(keepRoots = false) {
  const next = new Set(expandedSet.value);
  const collect = (items: GitTreeNode<T>[], depth: number) => {
    for (const node of items) {
      if (hasChildren(node)) {
        if (!(keepRoots && depth === 0)) {
          next.delete(node.key);
          // 折叠全部视为用户主动折叠，后续 nodes 刷新不再自动展开
          userToggledKeys.value.add(node.key);
        }
        collect(node.children!, depth + 1);
      }
    }
  };
  collect(props.nodes, 0);
  expandedSet.value = next;
}

function expandAll() {
  const next = new Set(expandedSet.value);
  const collect = (items: GitTreeNode<T>[]) => {
    for (const node of items) {
      if (hasChildren(node) || node.expandable) next.add(node.key);
      if (hasChildren(node)) collect(node.children!);
    }
  };
  collect(props.nodes);
  expandedSet.value = next;
}

defineExpose({ clearSelection, collapseAll, expandAll });
</script>

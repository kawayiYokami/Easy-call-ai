import type { DepartmentPermissionCatalogItem } from "../../../types/app";

// ========== 类型 ==========

export type DepartmentToolLeafCategory = "builtinToolNames" | "skillNames" | "mcpToolNames";

export type DepartmentToolTreeLeaf = {
  category: DepartmentToolLeafCategory;
  name: string;
  displayName: string;
  description: string;
  enabled: boolean;
};

export type DepartmentToolTreeGroup = {
  key: string;
  label: string;
  state: DepartmentToolGroupState;
  leaves: DepartmentToolTreeLeaf[];
};

export type DepartmentToolGroupState = "all" | "none" | "partial";

export type DepartmentToolTreeSection = {
  key: DepartmentToolLeafCategory;
  label: string;
  disabled: boolean;
  groups: DepartmentToolTreeGroup[];
  leaves: DepartmentToolTreeLeaf[];
};

// ========== 内置工具前端分组表 ==========

type BuiltinToolGroupDef = { key: string; tools: readonly string[] };

const BUILTIN_TOOL_GROUP_DEFS: readonly BuiltinToolGroupDef[] = [
  { key: "files", tools: ["read", "read_file", "read_media", "write", "update", "delete", "move"] },
  { key: "execConfig", tools: ["exec", "config"] },
  { key: "desktop", tools: ["operate", "windows"] },
  { key: "web", tools: ["fetch", "websearch"] },
  { key: "delegate", tools: ["delegate"] },
  { key: "media", tools: ["image_generate", "image_edit", "meme"] },
];

const OTHER_GROUP_KEY = "other";
const MCP_GROUP_KEY_PREFIX = "server:";
const MCP_NAME_SEPARATOR = "::";

function buildLeaf(category: DepartmentToolLeafCategory, item: DepartmentPermissionCatalogItem, displayName: string, isEnabled: (name: string) => boolean): DepartmentToolTreeLeaf {
  return {
    category,
    name: item.name,
    displayName,
    description: item.description,
    enabled: isEnabled(item.name),
  };
}

function aggregateGroupState(leaves: DepartmentToolTreeLeaf[]): DepartmentToolGroupState {
  const enabledCount = leaves.filter((leaf) => leaf.enabled).length;
  if (enabledCount === 0) return "none";
  if (enabledCount === leaves.length) return "all";
  return "partial";
}

function buildGroup(key: string, label: string, leaves: DepartmentToolTreeLeaf[]): DepartmentToolTreeGroup {
  return { key, label, state: aggregateGroupState(leaves), leaves };
}

// ========== 内置工具：按功能域分组 ==========

export function buildBuiltinToolGroups(
  items: DepartmentPermissionCatalogItem[],
  isEnabled: (name: string) => boolean,
  labelFor: (groupKey: string) => string,
): DepartmentToolTreeGroup[] {
  const byName = new Map(items.map((item) => [item.name, item]));
  const groups: DepartmentToolTreeGroup[] = [];
  const assigned = new Set<string>();
  for (const def of BUILTIN_TOOL_GROUP_DEFS) {
    const leaves = def.tools
      .filter((tool) => byName.has(tool))
      .map((tool) => {
        assigned.add(tool);
        return buildLeaf("builtinToolNames", byName.get(tool)!, tool, isEnabled);
      });
    if (leaves.length > 0) groups.push(buildGroup(def.key, labelFor(def.key), leaves));
  }
  const others = items.filter((item) => !assigned.has(item.name));
  if (others.length > 0) {
    groups.push(buildGroup(OTHER_GROUP_KEY, labelFor(OTHER_GROUP_KEY), others.map((item) => buildLeaf("builtinToolNames", item, item.name, isEnabled))));
  }
  return groups;
}

// ========== MCP 工具：按 server 分组 ==========

export function splitMcpToolName(fullName: string): { server: string; tool: string } | null {
  const index = fullName.indexOf(MCP_NAME_SEPARATOR);
  if (index <= 0 || index >= fullName.length - MCP_NAME_SEPARATOR.length) return null;
  return {
    server: fullName.slice(0, index),
    tool: fullName.slice(index + MCP_NAME_SEPARATOR.length),
  };
}

export function buildMcpToolGroups(
  items: DepartmentPermissionCatalogItem[],
  isEnabled: (name: string) => boolean,
  otherLabel: string,
): DepartmentToolTreeGroup[] {
  const grouped = new Map<string, DepartmentToolTreeLeaf[]>();
  const otherLeaves: DepartmentToolTreeLeaf[] = [];
  for (const item of items) {
    const parts = splitMcpToolName(item.name);
    if (!parts) {
      otherLeaves.push(buildLeaf("mcpToolNames", item, item.name, isEnabled));
      continue;
    }
    const bucket = grouped.get(parts.server) ?? [];
    bucket.push(buildLeaf("mcpToolNames", item, parts.tool, isEnabled));
    grouped.set(parts.server, bucket);
  }
  const groups = Array.from(grouped.entries()).map(([server, leaves]) =>
    buildGroup(`${MCP_GROUP_KEY_PREFIX}${server}`, server, leaves),
  );
  if (otherLeaves.length > 0) groups.push(buildGroup(OTHER_GROUP_KEY, otherLabel, otherLeaves));
  return groups;
}

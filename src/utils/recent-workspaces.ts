import { ref } from "vue";
import { normalizeWorkspacePathKey, stripExtendedPathPrefix } from "./shell-workspaces";

const STORAGE_KEY = "pai.recent_workspaces.v1";
const LIMIT = 20;

function normalizeStoredPath(path: string): string {
  const stripped = stripExtendedPathPrefix(String(path || "").trim());
  if (!stripped) return "";
  return stripped.replace(/\/+$/, "");
}

function loadRecentWorkspacePaths(): string[] {
  try {
    const raw = typeof window !== "undefined" ? window.localStorage.getItem(STORAGE_KEY) : null;
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    const out: string[] = [];
    const seen = new Set<string>();
    for (const item of parsed) {
      const normalized = normalizeStoredPath(String(item || ""));
      if (!normalized) continue;
      const key = normalizeWorkspacePathKey(normalized);
      if (!key || seen.has(key)) continue;
      seen.add(key);
      out.push(normalized);
      if (out.length >= LIMIT) break;
    }
    return out;
  } catch {
    return [];
  }
}

function saveRecentWorkspacePaths(paths: string[]) {
  try {
    if (typeof window === "undefined") return;
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(paths.slice(0, LIMIT)));
  } catch {
    // ignore quota / private mode
  }
}

export const recentWorkspacePaths = ref<string[]>(loadRecentWorkspacePaths());

export function pushRecentWorkspacePath(path: string) {
  const normalized = normalizeStoredPath(String(path || ""));
  if (!normalized) return;
  const key = normalizeWorkspacePathKey(normalized);
  if (!key) return;
  const list = [...recentWorkspacePaths.value];
  const existingIndex = list.findIndex((item) => normalizeWorkspacePathKey(String(item || "")) === key);
  if (existingIndex !== -1) list.splice(existingIndex, 1);
  list.unshift(normalized);
  if (list.length > LIMIT) list.length = LIMIT;
  recentWorkspacePaths.value = list;
  saveRecentWorkspacePaths(list);
}

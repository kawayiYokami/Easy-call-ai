import { ref } from "vue";

const STORAGE_KEY = "pai.recent_workspaces.v1";
const LIMIT = 20;

function loadRecentWorkspacePaths(): string[] {
  try {
    const raw = typeof window !== "undefined" ? window.localStorage.getItem(STORAGE_KEY) : null;
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .map((item) => String(item || "").trim())
      .filter(Boolean)
      .slice(0, LIMIT);
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
  const normalized = String(path || "").trim();
  if (!normalized) return;
  const key = normalized.toLowerCase();
  const list = [...recentWorkspacePaths.value];
  const existingIndex = list.findIndex((item) => String(item).toLowerCase() === key);
  if (existingIndex !== -1) list.splice(existingIndex, 1);
  list.unshift(normalized);
  if (list.length > LIMIT) list.length = LIMIT;
  recentWorkspacePaths.value = list;
  saveRecentWorkspacePaths(list);
}

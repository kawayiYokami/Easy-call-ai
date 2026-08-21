import type { ApiConfigItem } from "../../../types/app";
import { LEGAL_REASONING_EFFORTS, normalizeReasoningEffortValue, sortReasoningEffortValues } from "./api-config-display";
import { apiConfigModelGroupKey } from "./api-config-selection-tree";

// 全局档位刻度：最近档距离按固定档位表计算，与组内实际有哪些档无关
const globalEffortRank = new Map<string, number>(
  sortReasoningEffortValues(LEGAL_REASONING_EFFORTS).map((value, index) => [value, index]),
);

/**
 * 每个模型组最后使用的思维等级记忆（全局持久化，不分会话）。
 * key 用 apiConfigModelGroupKey（供应商无关：同模型同公开配置视为一组）。
 */
const STORAGE_KEY = "ecall.model.effort.memory";

export type ModelEffortMemory = Record<string, string>;

export function loadModelEffortMemory(): ModelEffortMemory {
  try {
    const raw = globalThis.localStorage?.getItem(STORAGE_KEY);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
    const memory: ModelEffortMemory = {};
    for (const [key, value] of Object.entries(parsed as Record<string, unknown>)) {
      const effort = normalizeReasoningEffortValue(value);
      if (effort) memory[key] = effort;
    }
    return memory;
  } catch {
    return {};
  }
}

// 模块级单例：选择器与等级调节器两个组件实例共享同一份记忆
let memorySingleton: ModelEffortMemory | null = null;

export function getModelEffortMemory(): ModelEffortMemory {
  if (!memorySingleton) memorySingleton = loadModelEffortMemory();
  return memorySingleton;
}

export function saveModelEffortMemory(memory: ModelEffortMemory): void {
  try {
    globalThis.localStorage?.setItem(STORAGE_KEY, JSON.stringify(memory));
  } catch {
    // 存储不可用时静默降级为会话内记忆
  }
}

export type ModelEffortResolution = {
  leafId: string;
  effort: string;
  groupKey: string;
};

export type ModelGroupIndexEntry = {
  groupKey: string;
  items: ApiConfigItem[];
};

/**
 * 按配置项 id 建模型分组索引（一次 O(N) 分组 key 计算）。
 * 分组 key 的深度序列化较贵，所有按组查询（预演/选中/调节器）
 * 都应走索引，避免每次选中变化重复计算。
 */
export function buildModelGroupIndex(options: ApiConfigItem[]): Map<string, ModelGroupIndexEntry> {
  const byKey = new Map<string, ApiConfigItem[]>();
  const index = new Map<string, ModelGroupIndexEntry>();
  for (const item of options) {
    const id = String(item?.id || "").trim();
    if (!id) continue;
    const key = apiConfigModelGroupKey(item);
    let items = byKey.get(key);
    if (!items) {
      items = [];
      byKey.set(key, items);
    }
    items.push(item);
    index.set(id, { groupKey: key, items });
  }
  return index;
}

function itemEffort(item: ApiConfigItem): string {
  return normalizeReasoningEffortValue(item.reasoningEffort) || "default";
}

/**
 * 在目标模型组内解析应生效的思维等级叶子：
 * 1. 记忆命中的等级可用则直接用；
 * 2. 无记忆（或不可用）优先 default；
 * 3. 仍不可用则取档位序列中距离 default 最近的档位，平手偏向更高档。
 * 单档位组直接返回请求的叶子。group 须为与 requestedId 同组的全部配置项。
 * 刻意不做「继承当前模型档位」：那会让所有无记忆模型的预演徽章随当前选择漂移。
 */
export function resolveModelEffortSelection(
  group: ApiConfigItem[],
  groupKey: string,
  requestedId: string,
  memory: ModelEffortMemory,
): ModelEffortResolution | null {
  const requested = group.find((item) => String(item?.id || "").trim() === String(requestedId || "").trim());
  if (!requested) return null;
  if (group.length <= 1) {
    return { leafId: String(requested.id), effort: itemEffort(requested), groupKey };
  }
  const orderedEfforts = sortReasoningEffortValues(group.map(itemEffort));
  const leafByEffort = new Map(group.map((item) => [itemEffort(item), item]));

  const remembered = normalizeReasoningEffortValue(memory[groupKey]);
  if (remembered && leafByEffort.has(remembered)) {
    return { leafId: String(leafByEffort.get(remembered)?.id || ""), effort: remembered, groupKey };
  }
  const fallbackDefault = leafByEffort.get("default");
  if (fallbackDefault) {
    return { leafId: String(fallbackDefault.id), effort: "default", groupKey };
  }
  const targetRank = globalEffortRank.get("default");
  const nearest = orderedEfforts.reduce((best, effort) => {
    if (best === "") return effort;
    const distance = (candidate: string) => {
      const rank = globalEffortRank.get(candidate);
      if (rank == null || targetRank == null) return Number.MAX_SAFE_INTEGER;
      return Math.abs(rank - targetRank);
    };
    const candidateDistance = distance(effort);
    const bestDistance = distance(best);
    // 平手偏向更高档（靠近 high 一侧）
    if (candidateDistance < bestDistance) return effort;
    if (candidateDistance === bestDistance
      && (globalEffortRank.get(effort) ?? -1) > (globalEffortRank.get(best) ?? -1)) return effort;
    return best;
  }, "");
  const leaf = nearest ? leafByEffort.get(nearest) : undefined;
  if (!leaf) return { leafId: String(requested.id), effort: itemEffort(requested), groupKey };
  return { leafId: String(leaf.id), effort: nearest, groupKey };
}

/** 记录某模型组最终生效的等级并持久化（更新模块级单例）。 */
export function rememberModelEffort(groupKey: string, effort: string): ModelEffortMemory {
  const normalized = normalizeReasoningEffortValue(effort);
  if (!groupKey || !normalized) return getModelEffortMemory();
  memorySingleton = { ...getModelEffortMemory(), [groupKey]: normalized };
  saveModelEffortMemory(memorySingleton);
  return memorySingleton;
}

import type { ApiConfigItem } from "../../../types/app";
import {
  normalizeReasoningEffortValue,
  reasoningEffortDisplayLabel,
  sortReasoningEffortValues,
  stripReasoningEffortDisplaySuffix,
} from "./api-config-display";

export type ApiConfigSelectionLeaf = {
  id: string;
  label: string;
  item: ApiConfigItem;
};

export type ApiConfigSelectionModel = {
  key: string;
  name: string;
  summaryFields: ApiConfigSelectionSummaryField[];
  leaves: ApiConfigSelectionLeaf[];
  representative: ApiConfigItem;
};

export type ApiConfigSelectionProvider = {
  id: string;
  name: string;
  models: ApiConfigSelectionModel[];
};

export type ApiConfigSelectionSummaryField =
  | "contextWindowTokens"
  | "maxOutputTokens"
  | "temperature"
  | "enableTools"
  | "enableImage"
  | "enableAudio"
  | "enableVideo";

const SUMMARY_FIELDS: ApiConfigSelectionSummaryField[] = [
  "contextWindowTokens",
  "maxOutputTokens",
  "temperature",
  "enableTools",
  "enableImage",
  "enableAudio",
  "enableVideo",
];

function providerIdFromApiConfigId(id: string): string {
  return String(id || "").split("::")[0]?.trim() || "";
}

function providerNameFromApiConfig(item: ApiConfigItem): string {
  const name = stripReasoningEffortDisplaySuffix(String(item.name || "").trim());
  const model = String(item.model || "").trim();
  const suffixes = model ? [` · ${model}`, `/${model}`] : [];
  const suffix = suffixes.find((item) => name.endsWith(item));
  if (suffix) return name.slice(0, -suffix.length).trim();
  return providerIdFromApiConfigId(item.id) || name;
}

function stableValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(stableValue);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, item]) => [key, stableValue(item)]),
    );
  }
  return value;
}

function modelPublicConfiguration(item: ApiConfigItem): Record<string, unknown> {
  const { id: _id, name: _name, reasoningEffort: _reasoningEffort, model: _model, ...configuration } = item;
  return stableValue(configuration) as Record<string, unknown>;
}

/** 与 buildApiConfigSelectionTree 的模型分组粒度一致，供等级记忆按同一粒度取 key。 */
export function apiConfigModelGroupKey(item: ApiConfigItem): string {
  return JSON.stringify({
    model: String(item.model || "").trim(),
    configuration: modelPublicConfiguration(item),
  });
}

function summaryFieldValue(item: ApiConfigItem, field: ApiConfigSelectionSummaryField): unknown {
  if (field === "temperature") {
    return item.customTemperatureEnabled ? Number(item.temperature) : null;
  }
  if (field === "maxOutputTokens") {
    return item.customMaxOutputTokensEnabled ? Number(item.maxOutputTokens) : null;
  }
  return item[field];
}

function distinctSummaryFields(models: ApiConfigSelectionModel[]): ApiConfigSelectionSummaryField[] {
  return SUMMARY_FIELDS.filter((field) => new Set(
    models.map((model) => JSON.stringify(summaryFieldValue(model.representative, field))),
  ).size > 1);
}

export function buildApiConfigSelectionTree(
  apiConfigs: ApiConfigItem[] | null | undefined,
  translate?: (key: string, params?: Record<string, unknown>) => string,
): ApiConfigSelectionProvider[] {
  const providerMap = new Map<string, ApiConfigSelectionProvider>();
  for (const item of apiConfigs || []) {
    const id = String(item?.id || "").trim();
    const model = String(item?.model || "").trim();
    if (!id || !model) continue;
    const providerId = providerIdFromApiConfigId(id) || providerNameFromApiConfig(item);
    const providerName = providerNameFromApiConfig(item) || providerId;
    let provider = providerMap.get(providerId);
    if (!provider) {
      provider = { id: providerId, name: providerName, models: [] };
      providerMap.set(providerId, provider);
    }
    const key = apiConfigModelGroupKey(item);
    let group = provider.models.find((candidate) => candidate.key === key);
    if (!group) {
      group = {
        key,
        name: String(item.displayName || "").trim() || model,
        summaryFields: [],
        leaves: [],
        representative: item,
      };
      provider.models.push(group);
    }
    const effort = normalizeReasoningEffortValue(item.reasoningEffort) || "default";
    group.leaves.push({
      id,
      label: reasoningEffortDisplayLabel(effort, translate) || effort,
      item,
    });
  }

  for (const provider of providerMap.values()) {
    const byModelName = new Map<string, ApiConfigSelectionModel[]>();
    for (const group of provider.models) {
      const groups = byModelName.get(group.name) || [];
      groups.push(group);
      byModelName.set(group.name, groups);
      const orderedEfforts = sortReasoningEffortValues(
        group.leaves.map((leaf) => normalizeReasoningEffortValue(leaf.item.reasoningEffort) || "default"),
      );
      const effortRank = new Map(orderedEfforts.map((value, index) => [value, index]));
      group.leaves.sort((left, right) => {
        const leftEffort = normalizeReasoningEffortValue(left.item.reasoningEffort) || "default";
        const rightEffort = normalizeReasoningEffortValue(right.item.reasoningEffort) || "default";
        return (effortRank.get(leftEffort) ?? 0) - (effortRank.get(rightEffort) ?? 0);
      });
    }
    for (const groups of byModelName.values()) {
      const fields = groups.length > 1 ? distinctSummaryFields(groups) : [];
      for (const group of groups) group.summaryFields = fields;
    }
  }
  return Array.from(providerMap.values());
}

export function apiConfigSelectionSummary(
  item: ApiConfigItem,
  fields: ApiConfigSelectionSummaryField[],
  labels: Record<ApiConfigSelectionSummaryField, string>,
): string {
  return fields.map((field) => {
    if (field === "contextWindowTokens") return `${labels[field]} ${Math.round(Number(item.contextWindowTokens || 0) / 1000)}K`;
    if (field === "maxOutputTokens") return `${labels[field]} ${Math.round(Number(item.maxOutputTokens || 0) / 1000)}K`;
    if (field === "temperature") return `${labels[field]} ${Number(item.temperature || 0).toFixed(1)}`;
    return `${labels[field]} ${item[field] ? "✓" : "—"}`;
  }).join(" · ");
}

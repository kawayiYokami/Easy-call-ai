import type { ApiModelConfigItem, ApiProviderConfigItem } from "../../../types/app";

// 草稿模型组：读取时聚合一次，编辑期固定结构；保存时拆分写回 config
export type DraftModelGroup = {
  key: string;
  primary: ApiModelConfigItem;            // 编辑载体（config 卡的克隆，编辑不碰 config）
  reasoningEfforts: string[];             // 勾选等级集合（含 default）
  variantIdByEffort: Map<string, string>; // 等级 → 持久化卡 id，保存拆分时复用/新增
};

export function isModelDeprecated(model: ApiModelConfigItem | null | undefined): boolean {
  return !!model?.deprecated;
}

export function normalizedModelReasoningEffortFor(model: ApiModelConfigItem): string {
  return String(model.reasoningEffort || "").trim().toLowerCase() || "default";
}

export const AUTO_CONTEXT_WINDOW_TOKENS = 256_000;

export function modelGroupKey(model: ApiModelConfigItem): string {
  return JSON.stringify({
    model: String(model.model || "").trim(),
    enableImage: !!model.enableImage,
    enableAudio: !!model.enableAudio,
    enableVideo: !!model.enableVideo,
    enableTools: model.enableTools !== false,
    temperature: Number(model.temperature ?? 1),
    customTemperatureEnabled: !!model.customTemperatureEnabled,
    contextWindowTokens: Math.round(Number(model.contextWindowTokens ?? AUTO_CONTEXT_WINDOW_TOKENS)),
    customMaxOutputTokensEnabled: !!model.customMaxOutputTokensEnabled,
    maxOutputTokens: Number(model.maxOutputTokens ?? 4096),
  });
}

// 读取时聚合一次：config.models → 草稿组；同 key 的卡并入一组，不同 reasoningEffort 各自记录
export function buildDraftGroups(provider: ApiProviderConfigItem | null | undefined): DraftModelGroup[] {
  if (!provider) return [];
  const groups = new Map<string, DraftModelGroup>();
  for (const model of provider.models || []) {
    if (isModelDeprecated(model)) continue;
    const key = modelGroupKey(model);
    const effort = normalizedModelReasoningEffortFor(model);
    const existing = groups.get(key);
    if (existing) {
      if (!existing.reasoningEfforts.includes(effort)) existing.reasoningEfforts.push(effort);
      if (!existing.variantIdByEffort.has(effort)) existing.variantIdByEffort.set(effort, model.id);
      continue;
    }
    groups.set(key, {
      key,
      primary: { ...model },
      reasoningEfforts: [effort],
      variantIdByEffort: new Map([[effort, model.id]]),
    });
  }
  return Array.from(groups.values());
}

// 保存时拆分一次：草稿组 → config.models；id 复用 variantIdByEffort，新等级生成新 id
export function splitDraftGroups(
  provider: ApiProviderConfigItem | null | undefined,
  groups: DraftModelGroup[],
  createModelId: () => string = () => `api-model-${Date.now().toString()}`,
): ApiModelConfigItem[] {
  if (!provider) return [];
  const nextModels: ApiModelConfigItem[] = [];
  const keptIds = new Set<string>();
  for (const group of groups) {
    const efforts = group.reasoningEfforts.length > 0 ? group.reasoningEfforts : ["default"];
    for (const effort of efforts) {
      const normalized = String(effort || "").trim().toLowerCase() || "default";
      const existingId = group.variantIdByEffort.get(normalized);
      const id = existingId ?? createModelId();
      if (!existingId) group.variantIdByEffort.set(normalized, id);
      keptIds.add(id);
      nextModels.push({
        ...group.primary,
        id,
        deprecated: false,
        reasoningEffort: normalized,
      });
    }
  }
  // 保留原 provider 中 deprecated 的卡（deprecated 历史不丢）
  for (const model of provider.models || []) {
    if (isModelDeprecated(model) && !keptIds.has(model.id)) {
      nextModels.push(model);
    }
  }
  return nextModels;
}

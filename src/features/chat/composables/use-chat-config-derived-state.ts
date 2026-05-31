import { computed } from "vue";
import type { ApiRequestFormat, AppConfig } from "../../../types/app";

export function useChatConfigDerivedState(config: AppConfig) {
  const TEXT_REQUEST_FORMATS = new Set<ApiRequestFormat>([
    "auto",
    "openai",
    "deepseek",
    "openai_responses",
    "codex",
    "gemini",
    "anthropic",
    "fireworks",
    "together",
    "groq",
    "mimo",
    "moonshot",
    "nebius",
    "xai",
    "zai",
    "bigmodel",
    "aliyun",
    "baidu",
    "cohere",
    "ollama",
    "ollama_cloud",
    "vertex",
    "github_copilot",
    "opencode_go",
    "bedrock_api",
  ]);

  function isTextRequestFormat(format: string): boolean {
    const normalized = String(format || "").trim().toLowerCase();
    return normalized === "deepseek/kimi" || TEXT_REQUEST_FORMATS.has(normalized as ApiRequestFormat);
  }

  const selectedApiConfig = computed(() => config.apiConfigs.find((a) => a.id === config.selectedApiConfigId) ?? null);
  const selectedApiProvider = computed(() => {
    const [providerId] = String(config.selectedApiConfigId || "").split("::");
    return config.apiProviders.find((provider) => provider.id === providerId) ?? null;
  });
  const textCapableApiConfigs = computed(() =>
    config.apiConfigs.filter((a) => a.enableText && isTextRequestFormat(a.requestFormat)),
  );
  const imageCapableApiConfigs = computed(() => config.apiConfigs.filter((a) => a.enableImage));
  const sttCapableApiConfigs = computed(() =>
    config.apiConfigs.filter((a) => a.requestFormat === "openai_stt"),
  );

  function departmentPrimaryApiConfigId(
    department?: { apiConfigId?: string; apiConfigIds?: string[] } | null,
  ): string {
    const ids = Array.isArray(department?.apiConfigIds)
      ? department.apiConfigIds.map((id) => String(id || "").trim()).filter(Boolean)
      : [];
    if (ids.length > 0) return ids[0];
    return String(department?.apiConfigId || "").trim();
  }

  function departmentOrderedApiConfigIds(
    department?: { apiConfigId?: string; apiConfigIds?: string[] } | null,
  ): string[] {
    return Array.from(new Set([
      ...((Array.isArray(department?.apiConfigIds) ? department.apiConfigIds : []).map((id) => String(id || "").trim()).filter(Boolean)),
      String(department?.apiConfigId || "").trim(),
    ].filter(Boolean)));
  }

  function departmentConversationApiConfigId(
    department?: { id?: string; isBuiltInAssistant?: boolean; apiConfigId?: string; apiConfigIds?: string[] } | null,
  ): string {
    const directId = departmentPrimaryApiConfigId(department);
    if (directId) return directId;
    if (department?.id === "assistant-department" || department?.isBuiltInAssistant) {
      return String(config.assistantDepartmentApiConfigId || "").trim();
    }
    return "";
  }

  function applyDepartmentPrimaryApiConfigLocally(
    department: { id?: string; isBuiltInAssistant?: boolean; apiConfigId?: string; apiConfigIds?: string[]; updatedAt?: string } | null | undefined,
    apiConfigId: string,
  ): boolean {
    if (!department) return false;
    const nextId = String(apiConfigId || "").trim();
    if (!nextId) return false;
    const next = departmentOrderedApiConfigIds(department);
    if ((next[0] || "") === nextId) {
      config.selectedApiConfigId = nextId;
      if (department.id === "assistant-department" || department.isBuiltInAssistant) {
        config.assistantDepartmentApiConfigId = nextId;
      }
      return false;
    }
    const filtered = next.filter((item) => item.toLowerCase() !== nextId.toLowerCase());
    if (filtered.length === 0) {
      filtered.push(nextId);
    } else {
      filtered[0] = nextId;
    }
    const deduped = Array.from(new Set(filtered.filter(Boolean)));
    department.apiConfigIds = deduped;
    department.apiConfigId = deduped[0] || "";
    department.updatedAt = new Date().toISOString();
    if (department.id === "assistant-department" || department.isBuiltInAssistant) {
      config.assistantDepartmentApiConfigId = department.apiConfigId;
    }
    config.selectedApiConfigId = nextId;
    return true;
  }

  function normalizeRuntimeConfigNumbers(
    minValue: unknown,
    maxValue: unknown,
    fallback?: {
      minRecordSeconds?: number;
      maxRecordSeconds?: number;
    },
  ): { minRecordSeconds: number; maxRecordSeconds: number } {
    const MIN_RECORD_SECONDS = 1;
    const MAX_MIN_RECORD_SECONDS = 30;
    const DEFAULT_MAX_RECORD_SECONDS = 60;
    const MAX_RECORD_SECONDS = 600;
    const fallbackMin = Number(fallback?.minRecordSeconds);
    const fallbackMax = Number(fallback?.maxRecordSeconds);
    const nextMin = Number(minValue);
    const nextMax = Number(maxValue);
    const resolvedMin = Number.isFinite(nextMin)
      ? nextMin
      : (Number.isFinite(fallbackMin) ? fallbackMin : MIN_RECORD_SECONDS);
    const minRecordSeconds = Math.max(
      MIN_RECORD_SECONDS,
      Math.min(MAX_MIN_RECORD_SECONDS, Math.round(resolvedMin)),
    );
    const resolvedMax = Number.isFinite(nextMax)
      ? nextMax
      : (Number.isFinite(fallbackMax) ? fallbackMax : DEFAULT_MAX_RECORD_SECONDS);
    const maxRecordSeconds = Math.max(
      minRecordSeconds,
      Math.min(MAX_RECORD_SECONDS, Math.round(resolvedMax)),
    );
    return { minRecordSeconds, maxRecordSeconds };
  }

  const assistantDepartmentApiConfigId = computed(
    () => String(config.assistantDepartmentApiConfigId || "").trim(),
  );
  const assistantDepartmentApiConfig = computed(
    () => config.apiConfigs.find((a) => a.id === assistantDepartmentApiConfigId.value) ?? null,
  );
  const hasVisionFallback = computed(() =>
    !!config.visionApiConfigId
    && config.apiConfigs.some((a) => a.id === config.visionApiConfigId && a.enableImage),
  );
  const activeSttApiConfig = computed(
    () => sttCapableApiConfigs.value.find((a) => a.id === config.sttApiConfigId) ?? null,
  );
  const shouldUseRemoteStt = computed(() => {
    const cfg = activeSttApiConfig.value;
    if (!cfg) return false;
    return !!cfg.model.trim() && !!cfg.baseUrl.trim() && !!cfg.apiKey.trim();
  });

  return {
    TEXT_REQUEST_FORMATS,
    isTextRequestFormat,
    selectedApiConfig,
    selectedApiProvider,
    textCapableApiConfigs,
    imageCapableApiConfigs,
    sttCapableApiConfigs,
    departmentPrimaryApiConfigId,
    departmentOrderedApiConfigIds,
    departmentConversationApiConfigId,
    applyDepartmentPrimaryApiConfigLocally,
    normalizeRuntimeConfigNumbers,
    assistantDepartmentApiConfigId,
    assistantDepartmentApiConfig,
    hasVisionFallback,
    activeSttApiConfig,
    shouldUseRemoteStt,
  };
}

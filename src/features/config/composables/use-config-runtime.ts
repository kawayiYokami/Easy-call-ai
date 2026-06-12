import type { ComputedRef, Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";
import type {
  ApiConfigItem,
  ApiProviderConfigItem,
  ImageTextCacheStats,
  PersonaProfile,
  ToolLoadStatus,
} from "../../../types/app";

type TrFn = (key: string, params?: Record<string, unknown>) => string;

type UseConfigRuntimeOptions = {
  t: TrFn;
  setStatus: (text: string) => void;
  setStatusError: (key: string, error: unknown) => void;
  personas: Ref<PersonaProfile[]>;
  assistantDepartmentAgentId: Ref<string>;
  toolAgentId: Readonly<Ref<string>>;
  avatarSaving: Ref<boolean>;
  avatarError: Ref<string>;
  selectedApiConfig: ComputedRef<ApiConfigItem | null>;
  selectedApiProvider: ComputedRef<ApiProviderConfigItem | null>;
  refreshingModels: Ref<boolean>;
  modelRefreshError: Ref<string>;
  apiModelOptions: Ref<Record<string, string[]>>;
  modelRefreshOkFlags: Ref<Record<string, boolean>>;
  toolApiConfig: ComputedRef<ApiConfigItem | null>;
  checkingToolsStatus: Ref<boolean>;
  toolStatuses: Ref<ToolLoadStatus[]>;
  imageCacheStats: Ref<ImageTextCacheStats>;
  imageCacheStatsLoading: Ref<boolean>;
  ensureAvatarCached: (path?: string, updatedAt?: string) => Promise<void>;
};

export function useConfigRuntime(options: UseConfigRuntimeOptions) {
  function maskKeyPreview(key: string): string {
    const trimmed = String(key || "").trim();
    if (!trimmed) return "(empty)";
    if (trimmed.length <= 8) return `${trimmed.slice(0, 2)}***${trimmed.slice(-2)}`;
    return `${trimmed.slice(0, 4)}***${trimmed.slice(-4)}`;
  }

  function normalizeAvatarError(error: unknown): string {
    const raw = String(error ?? "").trim();
    if (!raw) return "unknown";
    if (raw.includes("当前人格来自私有工作区")) {
      return options.t("config.persona.privateWorkspaceAvatarReadonly");
    }
    if (raw.includes("Agent not found")) {
      return options.t("config.persona.avatarTargetMissing");
    }
    return raw;
  }

  async function syncTrayIcon(agentId?: string) {
    try {
      await invokeTauri("sync_tray_icon", {
        input: {
          agentId: agentId ?? null,
        },
      });
    } catch (e) {
      console.warn("[TRAY] sync icon failed:", e);
    }
  }

  async function saveAgentAvatar(input: { agentId: string; mime: string; bytesBase64: string }) {
    options.avatarSaving.value = true;
    options.avatarError.value = "";
    try {
      const result = await invokeTauri<{ path: string; updatedAt: string }>("save_agent_avatar", {
        input: {
          agentId: input.agentId,
          mime: input.mime,
          bytesBase64: input.bytesBase64,
        },
      });
      const idx = options.personas.value.findIndex((p) => p.id === input.agentId);
      if (idx >= 0) {
        options.personas.value[idx].avatarPath = result.path;
        options.personas.value[idx].avatarUpdatedAt = result.updatedAt;
        options.personas.value[idx].updatedAt = new Date().toISOString();
      }
      await options.ensureAvatarCached(result.path, result.updatedAt);
      if (input.agentId === options.assistantDepartmentAgentId.value) {
        await syncTrayIcon(input.agentId);
      }
      options.setStatus(options.t("status.avatarSaved"));
    } catch (e) {
      const err = normalizeAvatarError(e);
      options.avatarError.value = err;
      options.setStatus(options.t("status.avatarSaveFailed", { err }));
    } finally {
      options.avatarSaving.value = false;
    }
  }

  async function clearAgentAvatar(input: { agentId: string }) {
    options.avatarError.value = "";
    try {
      await invokeTauri("clear_agent_avatar", { input: { agentId: input.agentId } });
      const idx = options.personas.value.findIndex((p) => p.id === input.agentId);
      if (idx >= 0) {
        options.personas.value[idx].avatarPath = undefined;
        options.personas.value[idx].avatarUpdatedAt = undefined;
        options.personas.value[idx].updatedAt = new Date().toISOString();
      }
      if (input.agentId === options.assistantDepartmentAgentId.value) {
        await syncTrayIcon(input.agentId);
      }
      options.setStatus(options.t("status.avatarCleared"));
    } catch (e) {
      const err = normalizeAvatarError(e);
      options.avatarError.value = err;
      options.setStatus(options.t("status.avatarClearFailed", { err }));
    }
  }

  async function refreshModels() {
    if (!options.selectedApiConfig.value) return;
    const apiId = options.selectedApiConfig.value.id;
    const provider = options.selectedApiProvider.value;
    const effectiveRequestFormat = provider?.requestFormat ?? options.selectedApiConfig.value.requestFormat;
    const effectiveBaseUrl = String(provider?.baseUrl || options.selectedApiConfig.value.baseUrl || "").trim();
    const effectiveCodexAuthMode = String(
      provider?.codexAuthMode || options.selectedApiConfig.value.codexAuthMode || "read_local",
    ).trim() || "read_local";
    const effectiveCodexLocalAuthPath = String(
      provider?.codexLocalAuthPath || options.selectedApiConfig.value.codexLocalAuthPath || "~/.codex/auth.json",
    ).trim() || "~/.codex/auth.json";
    const isCodex = effectiveRequestFormat === "codex";
    if (isCodex) {
      console.info(`[API] Codex mode detected: skipping API key validation; using empty candidate API key for selectedApiConfig=${options.selectedApiConfig.value.id}`);
    }
    const apiKeys = Array.isArray(provider?.apiKeys)
      ? provider.apiKeys.map((value) => String(value || "").trim()).filter(Boolean)
      : [];
    const fallbackApiKey = String(options.selectedApiConfig.value.apiKey || "").trim();
    const candidateApiKeys = isCodex
      ? [""]
      : Array.from(new Set([fallbackApiKey, ...apiKeys].filter(Boolean)));
    options.refreshingModels.value = true;
    options.modelRefreshError.value = "";
    try {
      if (!isCodex && candidateApiKeys.length === 0) {
        throw new Error("API Key 为空，无法刷新模型列表。");
      }
      const errors: string[] = [];
      let models: string[] | null = null;
      for (const apiKey of candidateApiKeys) {
        try {
          models = await invokeTauri<string[]>("refresh_models", {
            input: {
              baseUrl: effectiveBaseUrl,
              apiKey,
              requestFormat: effectiveRequestFormat,
              providerId: String(provider?.id || "").trim() || null,
              codexAuthMode: effectiveCodexAuthMode,
              codexLocalAuthPath: effectiveCodexLocalAuthPath,
            },
          });
          break;
        } catch (error) {
          const err = error instanceof Error ? error : new Error(String(error));
          const stack = String(err.stack || "").trim();
          errors.push(
            [
              `Key ${maskKeyPreview(apiKey)} 刷新模型失败：${err.message}`,
              stack ? `堆栈：${stack}` : "",
            ].filter(Boolean).join("\n"),
          );
        }
      }
      if (!models) {
        throw new Error(
          [
            "刷新模型列表失败。",
            ...errors.filter(Boolean),
          ].join("\n\n"),
        );
      }
      const normalizedModels = models.map((m) => m.trim()).filter(Boolean);
      options.apiModelOptions.value[apiId] = normalizedModels;
      if (provider) {
        provider.cachedModelOptions = normalizedModels;
      }
      options.modelRefreshOkFlags.value[apiId] = true;
      options.setStatus(options.t("status.modelListRefreshed", { count: normalizedModels.length }));
    } catch (e) {
      const err = String(e);
      options.modelRefreshError.value = err;
      options.modelRefreshOkFlags.value[apiId] = false;
      options.setStatus(options.t("status.refreshModelsFailed", { err }));
    } finally {
      options.refreshingModels.value = false;
    }
  }

  async function refreshToolsStatus() {
    const agentId = String(options.toolAgentId.value || "").trim();
    if (!agentId) return;
    options.checkingToolsStatus.value = true;
    try {
      options.toolStatuses.value = await invokeTauri<ToolLoadStatus[]>("check_tools_status", {
        input: {
          agentId,
          apiConfigId: options.toolApiConfig.value?.id ?? null,
        },
      });
    } catch (e) {
      options.toolStatuses.value = [
        {
          id: "tools",
          status: "failed",
          detail: String(e),
        },
      ];
    } finally {
      options.checkingToolsStatus.value = false;
    }
  }

  async function refreshImageCacheStats() {
    options.imageCacheStatsLoading.value = true;
    try {
      options.imageCacheStats.value = await invokeTauri<ImageTextCacheStats>("get_image_text_cache_stats");
    } catch (e) {
      options.setStatusError("status.loadImageCacheStatsFailed", e);
    } finally {
      options.imageCacheStatsLoading.value = false;
    }
  }

  async function clearImageCache() {
    options.imageCacheStatsLoading.value = true;
    try {
      options.imageCacheStats.value = await invokeTauri<ImageTextCacheStats>("clear_image_text_cache");
      options.setStatus(options.t("status.imageCacheCleared"));
    } catch (e) {
      options.setStatusError("status.clearImageCacheFailed", e);
    } finally {
      options.imageCacheStatsLoading.value = false;
    }
  }

  return {
    syncTrayIcon,
    saveAgentAvatar,
    clearAgentAvatar,
    refreshModels,
    refreshToolsStatus,
    refreshImageCacheStats,
    clearImageCache,
  };
}

import type { Ref } from "vue";
import type { AppConfig, PromptCommandPreset } from "../../../types/app";

type RuntimeNumberNormalizer = (
  minValue: unknown,
  maxValue: unknown,
  fallback?: { minRecordSeconds?: number; maxRecordSeconds?: number },
) => { minRecordSeconds: number; maxRecordSeconds: number };

function normalizeInstructionPresets(value: unknown): PromptCommandPreset[] {
  return Array.isArray(value)
    ? value
      .map((item) => ({
        id: String(item?.id || "").trim(),
        name: String(item?.prompt || item?.name || "").trim(),
        prompt: String(item?.prompt || item?.name || "").trim(),
      }))
      .filter((item) => !!item.id && !!item.prompt)
    : [];
}

export function applyConversationApiBootstrapUpdate(bindings: {
  config: AppConfig;
}, payload: Record<string, unknown>) {
  bindings.config.assistantDepartmentApiConfigId = String(payload.assistantDepartmentApiConfigId ?? "").trim();
  bindings.config.visionApiConfigId = payload.visionApiConfigId as string | undefined;
  bindings.config.toolReviewApiConfigId = payload.toolReviewApiConfigId as string | undefined;
  bindings.config.sttApiConfigId = payload.sttApiConfigId as string | undefined;
  if ("sttAutoSend" in payload) bindings.config.sttAutoSend = !!payload.sttAutoSend;
}

export function applyChatSettingsBootstrapUpdate(bindings: {
  assistantDepartmentAgentId: Ref<string>;
  personaEditorId: Ref<string>;
  userAlias: Ref<string>;
  selectedResponseStyleId: Ref<string>;
  selectedPdfReadMode: Ref<"text" | "image">;
  backgroundVoiceScreenshotKeywords: Ref<string>;
  backgroundVoiceScreenshotMode: Ref<"desktop" | "focused_window">;
  instructionPresets: Ref<PromptCommandPreset[]>;
}, payload: Record<string, unknown>) {
  if ("assistantDepartmentAgentId" in payload) {
    const nextAgentId = String(payload.assistantDepartmentAgentId ?? "").trim();
    bindings.assistantDepartmentAgentId.value = nextAgentId;
    if (bindings.personaEditorId.value !== nextAgentId) bindings.personaEditorId.value = nextAgentId;
  }
  if ("userAlias" in payload) bindings.userAlias.value = String(payload.userAlias ?? "");
  if ("responseStyleId" in payload) bindings.selectedResponseStyleId.value = String(payload.responseStyleId ?? "").trim();
  if (payload.pdfReadMode === "text" || payload.pdfReadMode === "image") {
    bindings.selectedPdfReadMode.value = payload.pdfReadMode;
  }
  if ("backgroundVoiceScreenshotKeywords" in payload) {
    bindings.backgroundVoiceScreenshotKeywords.value = String(payload.backgroundVoiceScreenshotKeywords ?? "");
  }
  if (payload.backgroundVoiceScreenshotMode === "desktop" || payload.backgroundVoiceScreenshotMode === "focused_window") {
    bindings.backgroundVoiceScreenshotMode.value = payload.backgroundVoiceScreenshotMode;
  }
  if (Array.isArray(payload.instructionPresets)) {
    bindings.instructionPresets.value = normalizeInstructionPresets(payload.instructionPresets);
  }
}

export function applyConfigBootstrapUpdate(bindings: {
  config: AppConfig;
  createApiConfig: (id: string) => AppConfig["apiConfigs"][number];
  buildConfigSnapshotJson: () => string;
  lastSavedConfigJson: Ref<string>;
  normalizeWebviewZoomPercent: (value: unknown) => number;
  updateGithubUpdateMethod: (value: unknown) => void;
  normalizeRuntimeConfigNumbers?: RuntimeNumberNormalizer;
}, payload: Record<string, unknown>) {
  if (!payload || typeof payload !== "object") return;
  if ("hotkey" in payload) bindings.config.hotkey = String(payload.hotkey ?? "").trim();
  if ("uiFont" in payload) bindings.config.uiFont = String(payload.uiFont ?? "");
  if ("webviewZoomPercent" in payload) bindings.config.webviewZoomPercent = bindings.normalizeWebviewZoomPercent(payload.webviewZoomPercent);
  if ("githubUpdateMethod" in payload) bindings.updateGithubUpdateMethod(payload.githubUpdateMethod);
  if ("recordHotkey" in payload) bindings.config.recordHotkey = String(payload.recordHotkey ?? "");
  if ("recordBackgroundWakeEnabled" in payload) bindings.config.recordBackgroundWakeEnabled = !!payload.recordBackgroundWakeEnabled;
  if (bindings.normalizeRuntimeConfigNumbers && ("minRecordSeconds" in payload || "maxRecordSeconds" in payload)) {
    const normalized = bindings.normalizeRuntimeConfigNumbers(
      "minRecordSeconds" in payload ? payload.minRecordSeconds : bindings.config.minRecordSeconds,
      "maxRecordSeconds" in payload ? payload.maxRecordSeconds : bindings.config.maxRecordSeconds,
      {
        minRecordSeconds: bindings.config.minRecordSeconds,
        maxRecordSeconds: bindings.config.maxRecordSeconds,
      },
    );
    bindings.config.minRecordSeconds = normalized.minRecordSeconds;
    bindings.config.maxRecordSeconds = normalized.maxRecordSeconds;
  }
  if ("selectedApiConfigId" in payload) bindings.config.selectedApiConfigId = String(payload.selectedApiConfigId ?? "").trim();
  if ("assistantDepartmentApiConfigId" in payload) bindings.config.assistantDepartmentApiConfigId = String(payload.assistantDepartmentApiConfigId ?? "").trim();
  if ("visionApiConfigId" in payload) bindings.config.visionApiConfigId = payload.visionApiConfigId as string | undefined;
  if ("toolReviewApiConfigId" in payload) bindings.config.toolReviewApiConfigId = payload.toolReviewApiConfigId as string | undefined;
  if ("sttApiConfigId" in payload) bindings.config.sttApiConfigId = payload.sttApiConfigId as string | undefined;
  if ("sttAutoSend" in payload) bindings.config.sttAutoSend = !!payload.sttAutoSend;
  if ("terminalShellKind" in payload) bindings.config.terminalShellKind = String(payload.terminalShellKind ?? "");
  if ("apiProviders" in payload) {
    bindings.config.apiProviders = Array.isArray(payload.apiProviders)
      ? payload.apiProviders.map((provider: any) => ({
        ...provider,
        apiKeys: Array.isArray(provider.apiKeys) ? [...provider.apiKeys] : [],
        cachedModelOptions: Array.isArray(provider.cachedModelOptions) ? [...provider.cachedModelOptions] : [],
        models: Array.isArray(provider.models) ? provider.models.map((model: any) => ({ ...model })) : [],
        tools: Array.isArray(provider.tools)
          ? provider.tools.map((tool: any) => ({
            ...tool,
            args: Array.isArray(tool.args) ? [...tool.args] : [],
            values: { ...((tool.values || {}) as Record<string, unknown>) },
          }))
          : [],
      }))
      : [];
  }
  bindings.config.apiConfigs.splice(
    0,
    bindings.config.apiConfigs.length,
    ...((Array.isArray(payload.apiConfigs) && payload.apiConfigs.length > 0)
      ? payload.apiConfigs.map((item: any) => ({
        ...item,
        tools: Array.isArray(item.tools)
          ? item.tools.map((tool: any) => ({
            ...tool,
            args: Array.isArray(tool.args) ? [...tool.args] : [],
            values: { ...((tool.values || {}) as Record<string, unknown>) },
          }))
          : [],
      }))
      : [bindings.createApiConfig("default")]),
  );
  bindings.config.departments = Array.isArray(payload.departments)
    ? payload.departments.map((item: any) => ({ ...item, agentIds: Array.isArray(item.agentIds) ? [...item.agentIds] : [] }))
    : [];
  bindings.lastSavedConfigJson.value = bindings.buildConfigSnapshotJson();
}

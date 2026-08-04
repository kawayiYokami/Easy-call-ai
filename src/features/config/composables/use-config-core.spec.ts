import { computed } from "vue";
import { describe, expect, it } from "vitest";
import type { AppConfig } from "../../../types/app";
import { createImageGenerationProvider, imageGenerationEndpointId } from "../utils/image-generation-config";
import { useConfigCore } from "./use-config-core";

function createConfig(): AppConfig {
  const imageProvider = createImageGenerationProvider("openai", "test");
  return {
    hotkey: "Alt+·",
    uiLanguage: "zh-CN",
    uiFont: "auto",
    uiSizeScale: 100,
    webAccessPort: 8429,
    webAccessEnabled: true,
    webAccessPassword: "",
    githubUpdateMethod: "auto",
    skippedGithubUpdateVersion: "",
    recordHotkey: "CapsLock",
    recordBackgroundWakeEnabled: false,
    minRecordSeconds: 1,
    maxRecordSeconds: 60,
    llmRoundLogCapacity: 3,
    messageNotificationEnabled: true,
    messageNotificationSoundEnabled: false,
    desktopOperationNoticeEnabled: true,
    selectedApiConfigId: "",
    assistantDepartmentApiConfigId: "",
    visionApiConfigId: undefined,
    imageGenerationModelId: imageGenerationEndpointId(imageProvider.id, imageProvider.models[0]?.id || ""),
    toolReviewApiConfigId: undefined,
    sttApiConfigId: undefined,
    sttAutoSend: false,
    terminalShellKind: "auto",
    shellWorkspaces: [],
    mcpServers: [],
    remoteImChannels: [],
    departments: [],
    apiProviders: [],
    imageProviders: [imageProvider],
    apiConfigs: [],
  };
}

describe("useConfigCore image generation", () => {
  it("应在保存载荷和脏检查快照中保留独立生图配置", () => {
    const config = createConfig();
    const core = useConfigCore({
      config,
      textCapableApiConfigs: computed(() => []),
    });

    const payload = core.buildConfigPayload();
    const snapshot = JSON.parse(core.buildConfigSnapshotJson()) as AppConfig;

    expect(payload.imageProviders).toHaveLength(1);
    expect(payload.imageGenerationModelId).toBe(config.imageGenerationModelId);
    expect(snapshot.imageProviders[0]?.models[0]?.model).toBe("gpt-image-2");
    expect(snapshot.imageGenerationModelId).toBe(config.imageGenerationModelId);
  });
});

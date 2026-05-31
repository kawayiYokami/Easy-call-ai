import type { WritableComputedRef, Ref } from "vue";
import { invokeTauri } from "../../../services/tauri-api";
import type { AppConfig, PersonaProfile } from "../../../types/app";
import { buildPersonasSnapshotJson as buildPersonasSnapshotJsonValue } from "./use-chat-window-message-helpers";

type UseChatConfigActionsGlueOptions = {
  t: (key: string) => string;
  config: AppConfig;
  locale: WritableComputedRef<string> | Ref<string>;
  personas: Ref<PersonaProfile[]>;
  configTab: Ref<string>;
  lastSavedConfigJson: Ref<string>;
  normalizeLocale: (value: string) => AppConfig["uiLanguage"];
  applyUiLanguage: (value: string) => boolean;
  buildConfigSnapshotJson: () => string;
  refreshToolsStatus: () => void | Promise<void>;
  setStatus: (text: string) => void;
  setStatusError: (key: string, error: unknown) => void;
};

export function useChatConfigActionsGlue(options: UseChatConfigActionsGlueOptions) {
  function buildPersonasSnapshotJson() {
    return buildPersonasSnapshotJsonValue(options.personas.value);
  }

  function setUiLanguage(value: string) {
    const changed = options.applyUiLanguage(value);
    const lang = options.normalizeLocale(value);
    void invokeTauri<AppConfig>("set_ui_language", { uiLanguage: lang })
      .then((saved) => {
        options.config.uiLanguage = options.normalizeLocale(saved.uiLanguage);
        options.locale.value = options.config.uiLanguage;
        options.lastSavedConfigJson.value = options.buildConfigSnapshotJson();
        if (changed) {
          options.setStatus(options.t("status.configSaved"));
        }
      })
      .catch((error) => {
        options.setStatusError("status.saveConfigFailed", error);
      });
  }

  async function importPersonaMemories(payload: { agentId: string; file: File }) {
    const agentId = String(payload.agentId || "").trim();
    if (!agentId) return;
    try {
      const text = await payload.file.text();
      const parsed = JSON.parse(text) as unknown;
      const memories = Array.isArray(parsed)
        ? parsed
        : parsed && typeof parsed === "object" && Array.isArray((parsed as { memories?: unknown }).memories)
          ? (parsed as { memories: unknown[] }).memories
          : null;
      if (!Array.isArray(memories)) {
        throw new Error("无效的记忆文件格式");
      }
      const result = await invokeTauri<{ importedCount: number; createdCount: number; mergedCount: number; totalCount: number }>(
        "import_agent_memories",
        {
          input: { agentId, memories },
        },
      );
      options.setStatus(`人格记忆导入完成: 新增 ${result.createdCount} 条, 合并 ${result.mergedCount} 条, 总计 ${result.totalCount} 条`);
    } catch (error) {
      options.setStatusError("status.importMemoriesFailed", error);
    }
  }

  function handleToolsChanged() {
    if (options.configTab.value === "tools") {
      void options.refreshToolsStatus();
    }
  }

  return {
    buildPersonasSnapshotJson,
    setUiLanguage,
    importPersonaMemories,
    handleToolsChanged,
  };
}

import { useAppBootstrap } from "../../shell/composables/use-app-bootstrap";
import {
  applyChatSettingsBootstrapUpdate,
  applyConfigBootstrapUpdate,
  applyConversationApiBootstrapUpdate,
} from "./use-config-bootstrap-sync";

export function useConfigWindowBootstrap(bindings: Record<string, any>) {
  return useAppBootstrap({
    setViewMode: (mode) => {
      bindings.viewMode.value = mode;
    },
    initWindowMode: () => bindings.initWindow(),
    onThemeChanged: (theme) => {
      bindings.applyTheme(theme);
    },
    onLocaleChanged: (payload) => {
      const lang = bindings.normalizeLocale(payload);
      bindings.config.uiLanguage = lang;
      bindings.locale.value = lang;
    },
    onConversationApiUpdated: (payload) => {
      applyConversationApiBootstrapUpdate({ config: bindings.config }, payload);
    },
    onChatSettingsUpdated: (payload) => {
      applyChatSettingsBootstrapUpdate({
        assistantDepartmentAgentId: bindings.assistantDepartmentAgentId,
        personaEditorId: bindings.personaEditorId,
        userAlias: bindings.userAlias,
        selectedResponseStyleId: bindings.selectedResponseStyleId,
        selectedPdfReadMode: bindings.selectedPdfReadMode,
        backgroundVoiceScreenshotKeywords: bindings.backgroundVoiceScreenshotKeywords,
        backgroundVoiceScreenshotMode: bindings.backgroundVoiceScreenshotMode,
        instructionPresets: bindings.instructionPresets,
      }, payload);
    },
    onConfigUpdated: (payload) => {
      applyConfigBootstrapUpdate({
        config: bindings.config,
        createApiConfig: bindings.createApiConfig,
        buildConfigSnapshotJson: bindings.buildConfigSnapshotJson,
        lastSavedConfigJson: bindings.lastSavedConfigJson,
        normalizeWebviewZoomPercent: bindings.normalizeWebviewZoomPercent,
        updateGithubUpdateMethod: bindings.updateGithubUpdateMethod,
        normalizeRuntimeConfigNumbers: bindings.normalizeRuntimeConfigNumbers,
      }, payload);
    },
  });
}

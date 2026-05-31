import { useAppBootstrap } from "../../shell/composables/use-app-bootstrap";
import {
  applyChatSettingsBootstrapUpdate,
  applyConfigBootstrapUpdate,
  applyConversationApiBootstrapUpdate,
} from "../../config/composables/use-config-bootstrap-sync";

export function useChatWindowBootstrap(bindings: Record<string, any>) {
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
    onAgentWorkStarted: (payload) => {
      bindings.agentWorkPresence.markAgentWorkStarted(payload);
    },
    onAgentWorkStopped: (payload) => {
      bindings.agentWorkPresence.markAgentWorkStopped(payload);
    },
    onTerminalApprovalRequested: (payload) => {
      bindings.enqueueTerminalApprovalRequest(payload);
    },
    onConversationApiUpdated: async (payload) => {
      applyConversationApiBootstrapUpdate({ config: bindings.config }, payload);
      bindings.clearMatchingConversationChatErrors((text: string) => text.includes("不支持图片附件") || text.includes("PDF 附件"));
      if (bindings.viewMode.value === "chat") {
        await bindings.refreshConversationHistory();
      }
    },
    onChatSettingsUpdated: async (payload) => {
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
      if (bindings.viewMode.value === "chat") {
        await bindings.refreshConversationHistory();
      }
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
    onToolReviewReportsUpdated: (payload) => {
      const payloadConversationId = String(payload?.conversationId || "").trim();
      const currentConversationId = String(bindings.currentChatConversationId.value || "").trim();
      if (!payloadConversationId || payloadConversationId !== currentConversationId) return;
      bindings.toolReviewRefreshTick.value += 1;
    },
    onRecordHotkeyProbe: ({ state, seq }) => {
      if (seq > 0) {
        if (seq <= bindings.recordHotkeyProbeLastSeq.value) return;
        bindings.recordHotkeyProbeLastSeq.value = seq;
      }
      if (state === "released") {
        bindings.recordHotkeyProbeDown.value = false;
      }
      if (bindings.viewMode.value !== "chat" || !bindings.isPrimaryChatWindow()) return;
      if (!bindings.config.recordBackgroundWakeEnabled) return;
      if (state === "pressed") {
        bindings.recordHotkeyProbeDown.value = true;
        void bindings.startRecording("background").then(() => {
          if (!bindings.recordHotkeyProbeDown.value) {
            void bindings.stopRecording(false);
          }
        });
        return;
      }
      if (state === "released") {
        void bindings.stopRecording(false);
      }
    },
  });
}

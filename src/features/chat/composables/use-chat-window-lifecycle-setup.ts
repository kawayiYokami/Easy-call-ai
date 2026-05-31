import { invokeTauri } from "../../../services/tauri-api";
import { useAppLifecycle } from "../../shell/composables/use-app-lifecycle";
import { useAppWatchers } from "../../shell/composables/use-app-watchers";

export function useChatWindowLifecycleSetup(bindings: Record<string, any>) {
  useAppLifecycle({
      appBootstrapMount: bindings.appBootstrap.mount,
      appBootstrapUnmount: bindings.appBootstrap.unmount,
      restoreThemeFromStorage: bindings.restoreThemeFromStorage,
      onPaste: bindings.onPaste,
      onDragOver: bindings.onDragOver,
      onDrop: bindings.onDrop,
      onNativeFileDrop: bindings.onNativeFileDrop,
      onNativeDragState: (active) => {
        bindings.mediaDragActive.value = active;
      },
      recordHotkeyMount: bindings.recordHotkey.mount,
      recordHotkeyUnmount: bindings.recordHotkey.unmount,
      beforeRefreshData: bindings.ensureMessageStoreMigrationGate,
      refreshAllViewData: bindings.refreshAllViewData,
      afterRefreshData: () => {
        bindings.startupDataReady.value = true;
        if (bindings.viewMode.value === "chat" && !String(bindings.currentChatConversationId.value || "").trim()) {
          return bindings.refreshChatUnarchivedConversations().catch((error: unknown) => {
            console.warn("[聊天追踪][前台会话] 启动数据就绪后恢复失败", error);
          });
        }
      },
      viewMode: bindings.viewMode,
      syncWindowControlsState: bindings.syncWindowControlsState,
      stopRecording: bindings.stopRecording,
      cleanupSpeechRecording: bindings.cleanupSpeechRecording,
      cleanupChatMedia: bindings.cleanupChatMedia,
      onStartupOverlayChange: (visible, message) => {
        bindings.startupOverlayVisible.value = visible;
        bindings.startupOverlayMessage.value = message || "等待后端加载中...";
      },
      onStartupStepFailed: (label, error) => {
        bindings.setStatus(`启动步骤失败：${label}：${bindings.formatRequestFailed(error)}`);
      },
      afterSafetyGateReady: async () => {
        void invokeTauri<boolean>("frontend_ready_start_remote_im_services")
          .then((started) => {
            console.info("[启动] 前端 mounted ready 已通知后端启动后台服务", { started });
          })
          .catch((error) => {
            console.warn("[启动] 通知后端启动后台服务失败", error);
          });
      },
      afterMountedReady: async () => {
        await bindings.initializeDetachedChatWindow();
        await bindings.autoCheckGithubUpdate();
      },
  });
  useAppWatchers({
    config: bindings.config,
    configTab: bindings.configTab,
    viewMode: bindings.viewMode,
    personas: bindings.personas,
    userPersona: bindings.userPersona,
    assistantPersonas: bindings.assistantPersonas,
    assistantDepartmentAgentId: bindings.assistantDepartmentAgentId,
    personaEditorId: bindings.personaEditorId,
    selectedApiConfig: bindings.selectedApiConfig,
    toolApiConfig: bindings.toolApiConfig,
    modelRefreshError: bindings.modelRefreshError,
    toolStatuses: bindings.toolStatuses,
    defaultApiTools: bindings.defaultApiTools,
    t: bindings.tr,
    normalizeApiBindingsLocal: bindings.normalizeApiBindingsLocal,
    syncUserAliasFromPersona: bindings.syncUserAliasFromPersona,
    syncTrayIcon: bindings.syncTrayIcon,
    refreshToolsStatus: bindings.refreshToolsStatus,
    refreshImageCacheStats: bindings.refreshImageCacheStats,
  });
}

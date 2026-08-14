import { onBeforeUnmount, onMounted, type Ref } from "vue";
import { invokeTauri, listenCurrentTransportFileDrop } from "../../../services/tauri-api";

type UseAppLifecycleOptions = {
  appBootstrapMount: () => Promise<void>;
  appBootstrapUnmount: () => void;
  restoreThemeFromStorage: () => void;
  onPaste: (event: ClipboardEvent) => void;
  onDragOver: (event: DragEvent) => void;
  onDrop: (event: DragEvent) => void;
  onTransportFileDrop?: (paths: string[]) => Promise<void> | void;
  onNativeDragState?: (active: boolean) => void;
  recordHotkeyMount: () => void;
  recordHotkeyUnmount: () => void;
  prepareInitialData?: () => Promise<void> | void;
  afterInitialDataReady?: () => Promise<void> | void;
  refreshAllViewData: () => Promise<void>;
  afterRefreshData?: () => Promise<void> | void;
  viewMode: Ref<"chat" | "archives" | "config">;
  syncWindowControlsState: () => Promise<void>;
  stopRecording: (discard: boolean) => Promise<void>;
  cleanupSpeechRecording: () => void;
  cleanupChatMedia: () => Promise<void>;
  afterMountedReady?: () => Promise<void> | void;
  onBackendReadyChange?: (ready: boolean) => void;
  onStartupStepFailed?: (label: string, error: unknown) => void;
};

const BACKEND_READY_POLL_INTERVAL_MS = 100;

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitForBackendReady(isActive: () => boolean): Promise<boolean> {
  while (isActive()) {
    try {
      if (await invokeTauri<boolean>("is_backend_ready")) return true;
    } catch {
      // IPC 尚不可用时继续等待。
    }
    await delay(BACKEND_READY_POLL_INTERVAL_MS);
  }
  return false;
}

export function useAppLifecycle(options: UseAppLifecycleOptions) {
  let active = true;
  let unlistenNativeFileDrop: (() => void) | null = null;

  onMounted(async () => {
    options.onBackendReadyChange?.(false);
    if (!await waitForBackendReady(() => active)) return;
    options.onBackendReadyChange?.(true);

    try {
      await options.appBootstrapMount();
      options.restoreThemeFromStorage();
      window.addEventListener("paste", options.onPaste);
      window.addEventListener("dragover", options.onDragOver, { capture: true });
      window.addEventListener("drop", options.onDrop, { capture: true });
      if (options.onTransportFileDrop) {
        try {
          unlistenNativeFileDrop = await listenCurrentTransportFileDrop((payload) => {
            if (payload.type === "enter" || payload.type === "over") {
              options.onNativeDragState?.(true);
              return;
            }
            options.onNativeDragState?.(false);
            if (payload.type === "drop") {
              void Promise.resolve(options.onTransportFileDrop?.(payload.paths));
            }
          });
        } catch {
          // 原生拖拽监听失败不影响 DOM 拖拽。
        }
      }
      options.recordHotkeyMount();
      await options.prepareInitialData?.();
      await options.afterInitialDataReady?.();
    } catch (error) {
      console.error("[生命周期] 启动初始化失败:", error);
      options.onStartupStepFailed?.("startupLifecycle", error);
      return;
    }

    void (async () => {
      try {
        await options.refreshAllViewData();
      } catch (error) {
        console.error("[生命周期] 启动刷新失败: refreshAllViewData", error);
        options.onStartupStepFailed?.("refreshAllViewData", error);
        return;
      }
      try {
        await options.afterRefreshData?.();
      } catch (error) {
        console.error("[生命周期] 刷新收尾失败: afterRefreshData", error);
        options.onStartupStepFailed?.("afterRefreshData", error);
      }
      if (options.viewMode.value === "chat") {
        try {
          await options.syncWindowControlsState();
        } catch (error) {
          console.error("[生命周期] 窗口状态同步失败: syncWindowControlsState", error);
          options.onStartupStepFailed?.("syncWindowControlsState", error);
        }
      }
      try {
        await options.afterMountedReady?.();
      } catch (error) {
        console.error("[生命周期] 启动收尾失败: afterMountedReady", error);
        options.onStartupStepFailed?.("afterMountedReady", error);
      }
    })();
  });

  onBeforeUnmount(() => {
    active = false;
    options.appBootstrapUnmount();
    void options.stopRecording(true);
    options.cleanupSpeechRecording();
    options.recordHotkeyUnmount();
    void options.cleanupChatMedia();
    window.removeEventListener("paste", options.onPaste);
    window.removeEventListener("dragover", options.onDragOver, { capture: true });
    window.removeEventListener("drop", options.onDrop, { capture: true });
    unlistenNativeFileDrop?.();
    unlistenNativeFileDrop = null;
  });
}

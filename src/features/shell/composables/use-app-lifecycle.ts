import { onBeforeUnmount, onMounted, type Ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invokeTauri } from "../../../services/tauri-api";

type UseAppLifecycleOptions = {
  appBootstrapMount: () => Promise<void>;
  appBootstrapUnmount: () => void;
  restoreThemeFromStorage: () => void;
  onPaste: (event: ClipboardEvent) => void;
  onDragOver: (event: DragEvent) => void;
  onDrop: (event: DragEvent) => void;
  onNativeFileDrop?: (paths: string[]) => Promise<void> | void;
  onNativeDragState?: (active: boolean) => void;
  recordHotkeyMount: () => void;
  recordHotkeyUnmount: () => void;
  beforeRefreshData?: () => Promise<void> | void;
  afterSafetyGateReady?: () => Promise<void> | void;
  refreshAllViewData: () => Promise<void>;
  afterRefreshData?: () => Promise<void> | void;
  viewMode: Ref<"chat" | "archives" | "config">;
  syncWindowControlsState: () => Promise<void>;
  stopRecording: (discard: boolean) => Promise<void>;
  cleanupSpeechRecording: () => void;
  cleanupChatMedia: () => Promise<void>;
  afterMountedReady?: () => Promise<void> | void;
  onStartupOverlayChange?: (visible: boolean, message: string) => void;
  onStartupStepFailed?: (label: string, error: unknown) => void;
};

const STARTUP_STEP_TIMEOUT_MS = 10_000;
const BACKEND_READY_TIMEOUT_MS = 30_000;
const BACKEND_READY_POLL_INTERVAL_MS = 100;

function startupTimeoutError(label: string): Error {
  return new Error(`启动步骤超时：${label} 超过 ${STARTUP_STEP_TIMEOUT_MS / 1000} 秒未完成，已跳过。`);
}

async function runStartupStep(
  label: string,
  task: () => Promise<void> | void,
  onFailed?: (label: string, error: unknown) => void,
): Promise<boolean> {
  let timer: ReturnType<typeof setTimeout> | null = null;
  try {
    await Promise.race([
      Promise.resolve().then(task),
      new Promise<never>((_, reject) => {
        timer = setTimeout(() => reject(startupTimeoutError(label)), STARTUP_STEP_TIMEOUT_MS);
      }),
    ]);
    return true;
  } catch (error) {
    console.error(`[LIFECYCLE] startup step failed: ${label}`, error);
    onFailed?.(label, error);
    return false;
  } finally {
    if (timer) clearTimeout(timer);
  }
}

/**
 * 等待后端就绪信号。先查询当前状态（处理窗口晚于 setup 完成的情况），
 * 如果未就绪则监听事件等待。
 */
async function waitForBackendReady(): Promise<void> {
  return new Promise<void>((resolve, reject) => {
    let timer: ReturnType<typeof setTimeout> | null = null;
    let pollTimer: ReturnType<typeof setInterval> | null = null;
    let unlisten: (() => void) | null = null;
    let settled = false;
    const cleanup = () => {
      if (timer) {
        clearTimeout(timer);
        timer = null;
      }
      if (pollTimer) {
        clearInterval(pollTimer);
        pollTimer = null;
      }
      if (unlisten) {
        unlisten();
        unlisten = null;
      }
    };
    const finishReady = (source: string) => {
      if (settled) return;
      settled = true;
      cleanup();
      console.info(`[LIFECYCLE] 后端已就绪（${source}）`);
      resolve();
    };
    const checkReady = () => {
      invokeTauri<boolean>("is_backend_ready")
        .then((ready) => {
          if (ready) finishReady("轮询确认");
        })
        .catch(() => {
          // 后端 IPC 短暂不可用时继续等待事件和下一轮查询。
        });
    };
    timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      cleanup();
      reject(new Error(`等待后端就绪超时（${BACKEND_READY_TIMEOUT_MS / 1000}秒）`));
    }, BACKEND_READY_TIMEOUT_MS);
    pollTimer = setInterval(checkReady, BACKEND_READY_POLL_INTERVAL_MS);
    listen("easy-call:backend-ready", () => {
      finishReady("事件通知");
    })
      .then((fn) => {
        unlisten = fn;
        checkReady();
      })
      .catch((error) => {
        if (settled) return;
        settled = true;
        cleanup();
        reject(error);
      });
    checkReady();
  });
}

export function useAppLifecycle(options: UseAppLifecycleOptions) {
  onMounted(async () => {
    options.onStartupOverlayChange?.(true, "等待后端加载中...");
    let unlistenProgress: (() => void) | null = null;
    try {
      const bootstrapMounted = await runStartupStep(
        "appBootstrapMount",
        () => options.appBootstrapMount(),
        options.onStartupStepFailed,
      );
      if (!bootstrapMounted) {
        options.onStartupOverlayChange?.(false, "");
        return;
      }

      try {
        await waitForBackendReady();
      } catch (error) {
        console.warn("[LIFECYCLE] wait backend ready failed, continue startup refresh", error);
      }

      // 监听后端阶段 2 延迟初始化进度，实时显示卡在哪一步
      try {
        unlistenProgress = await listen<string>("easy-call:startup-progress", (event) => {
          const step = event.payload;
          if (step === "done") {
            options.onStartupOverlayChange?.(true, "加载数据中...");
          } else {
            options.onStartupOverlayChange?.(true, `初始化: ${step}`);
          }
        });
      } catch {
        // 监听失败不影响启动
      }

      options.onStartupOverlayChange?.(true, "加载数据中...");
      options.restoreThemeFromStorage();
      window.addEventListener("paste", options.onPaste);
      window.addEventListener("dragover", options.onDragOver);
      window.addEventListener("drop", options.onDrop);
      options.recordHotkeyMount();
      try {
        await options.beforeRefreshData?.();
      } catch (error) {
        console.error("[LIFECYCLE] startup safety gate failed: beforeRefreshData", error);
        options.onStartupStepFailed?.("beforeRefreshData", error);
        options.onStartupOverlayChange?.(false, "");
        return;
      }
      const backendReadyNotified = await runStartupStep(
        "afterSafetyGateReady",
        () => options.afterSafetyGateReady?.(),
        options.onStartupStepFailed,
      );
      if (!backendReadyNotified) {
        options.onStartupOverlayChange?.(false, "");
        return;
      }
      try {
        await options.refreshAllViewData();
      } catch (error) {
        console.error("[LIFECYCLE] startup refresh failed: refreshAllViewData", error);
        options.onStartupStepFailed?.("refreshAllViewData", error);
        options.onStartupOverlayChange?.(false, "");
        return;
      }
      const afterRefreshCompleted = await runStartupStep(
        "afterRefreshData",
        () => options.afterRefreshData?.(),
        options.onStartupStepFailed,
      );
      if (!afterRefreshCompleted) {
        options.onStartupOverlayChange?.(false, "");
        return;
      }
      if (options.viewMode.value === "chat") {
        const windowControlsSynced = await runStartupStep(
          "syncWindowControlsState",
          () => options.syncWindowControlsState(),
          options.onStartupStepFailed,
        );
        if (!windowControlsSynced) {
          options.onStartupOverlayChange?.(false, "");
          return;
        }
      }
      await runStartupStep(
        "afterMountedReady",
        () => options.afterMountedReady?.(),
        options.onStartupStepFailed,
      );
    } catch (error) {
      console.error("[LIFECYCLE] startup lifecycle failed:", error);
      options.onStartupStepFailed?.("startupLifecycle", error);
    } finally {
      if (unlistenProgress) unlistenProgress();
      options.onStartupOverlayChange?.(false, "");
    }
  });

  onBeforeUnmount(() => {
    options.appBootstrapUnmount();
    void options.stopRecording(true);
    options.cleanupSpeechRecording();
    options.recordHotkeyUnmount();
    void options.cleanupChatMedia();
    window.removeEventListener("paste", options.onPaste);
    window.removeEventListener("dragover", options.onDragOver);
    window.removeEventListener("drop", options.onDrop);
  });
}

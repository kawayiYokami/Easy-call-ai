import { onBeforeUnmount, onMounted, type Ref } from "vue";
import { invokeTauri, listenCurrentTransportFileDrop, onTransportNotification } from "../../../services/tauri-api";

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
  onStartupProgressChange?: (payload: {
    title: string;
    detail: string;
    current: number;
    total: number;
  }) => void;
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
    console.error(`[生命周期] 启动步骤失败: ${label}`, error);
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
      console.info(`[生命周期] 后端已就绪（${source}）`);
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
    unlisten = onTransportNotification("backend.ready", () => {
      finishReady("事件通知");
    });
    checkReady();
  });
}

export function useAppLifecycle(options: UseAppLifecycleOptions) {
  let unlistenNativeFileDrop: (() => void) | null = null;

  onMounted(async () => {
    options.onStartupOverlayChange?.(true, "正在启动应用...");
    options.onStartupProgressChange?.({
      title: "正在启动应用...",
      detail: "正在建立启动连接...",
      current: 0,
      total: 4,
    });
    let unlistenProgress: (() => void) | null = null;
    try {
      options.onStartupProgressChange?.({
        title: "初始化窗口中...",
        detail: "正在挂载前端启动器...",
        current: 1,
        total: 4,
      });
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
        options.onStartupProgressChange?.({
          title: "正在连接后端...",
          detail: "正在等待后端就绪信号...",
          current: 2,
          total: 4,
        });
        await waitForBackendReady();
      } catch (error) {
        console.warn("[生命周期] 等待后端就绪失败，继续执行启动刷新", error);
      }

      // 监听后端阶段 2 延迟初始化进度，实时显示卡在哪一步
      try {
        unlistenProgress = onTransportNotification<string>("startup.progress", (step) => {
          if (step === "done") {
            options.onStartupOverlayChange?.(true, "正在初始化界面...");
            options.onStartupProgressChange?.({
              title: "正在初始化界面...",
              detail: "后端初始化已完成，正在准备界面...",
              current: 3,
              total: 4,
            });
          } else {
            options.onStartupOverlayChange?.(true, `初始化: ${step}`);
            options.onStartupProgressChange?.({
              title: "正在初始化后端...",
              detail: `当前步骤：${step}`,
              current: 3,
              total: 4,
            });
          }
        });
      } catch {
        // 监听失败不影响启动
      }

      options.onStartupOverlayChange?.(true, "正在初始化界面...");
      options.onStartupProgressChange?.({
        title: "正在初始化界面...",
        detail: "正在恢复主题与窗口基础状态...",
        current: 3,
        total: 4,
      });
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
      try {
        options.onStartupProgressChange?.({
          title: "等待数据迁移完成...",
          detail: "正在执行启动前安全检查...",
          current: 4,
          total: 4,
        });
        await options.beforeRefreshData?.();
      } catch (error) {
        console.error("[生命周期] 启动安全门失败: beforeRefreshData", error);
        options.onStartupStepFailed?.("beforeRefreshData", error);
        options.onStartupOverlayChange?.(false, "");
        return;
      }
      await runStartupStep(
        "afterSafetyGateReady",
        () => options.afterSafetyGateReady?.(),
        options.onStartupStepFailed,
      );
    } catch (error) {
      console.error("[生命周期] 启动生命周期失败:", error);
      options.onStartupStepFailed?.("startupLifecycle", error);
    } finally {
      if (unlistenProgress) unlistenProgress();
      options.onStartupOverlayChange?.(false, "");
    }

    // ========== 遮罩关闭后：数据加载链，组件各自 loading，不阻塞画面 ==========
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

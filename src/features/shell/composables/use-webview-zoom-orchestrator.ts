import { onBeforeUnmount, onMounted, ref, watch, type Reactive } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AppConfig } from "../../../types/app";
import { invokeTauri } from "../../../services/tauri-api";

const WEBVIEW_ZOOM_PERCENT_OPTIONS = [80, 90, 100, 110, 120, 150] as const;
const WEBVIEW_ZOOM_APPLY_DELAY_MS = 500;

export type WebviewZoomOrchestratorBindings = {
  config: Reactive<AppConfig>;
  setStatusError: (i18nKey: string, error: unknown) => void;
};

export function useWebviewZoomOrchestrator(bindings: WebviewZoomOrchestratorBindings) {
  const webviewZoomFactor = ref(1);
  let webviewZoomApplyTimer: ReturnType<typeof window.setTimeout> | null = null;
  let webviewZoomWatcherReady = false;
  let suppressNextWebviewZoomApply = false;
  let webviewZoomUpdatedUnlisten: UnlistenFn | null = null;

  function normalizeWebviewZoomPercent(value: unknown) {
    const numeric = Math.round(Number(value));
    if (!Number.isFinite(numeric)) return 100;
    return WEBVIEW_ZOOM_PERCENT_OPTIONS.reduce((best, item) => (
      Math.abs(item - numeric) < Math.abs(best - numeric) ? item : best
    ), 100);
  }

  async function applyWebviewZoomPercent(percent: unknown) {
    const normalizedPercent = normalizeWebviewZoomPercent(percent);
    const nextFactor = normalizedPercent / 100;
    if (Math.abs(nextFactor - webviewZoomFactor.value) < 0.001) return;
    const appliedPercent = await invokeTauri<number>("set_webview_zoom_percent", {
      percent: normalizedPercent,
    });
    const appliedFactor = normalizeWebviewZoomPercent(appliedPercent) / 100;
    webviewZoomFactor.value = appliedFactor;
  }

  function scheduleWebviewZoomPercentApply(percent: unknown, delayMs = WEBVIEW_ZOOM_APPLY_DELAY_MS) {
    if (webviewZoomApplyTimer) {
      window.clearTimeout(webviewZoomApplyTimer);
      webviewZoomApplyTimer = null;
    }
    webviewZoomApplyTimer = window.setTimeout(() => {
      webviewZoomApplyTimer = null;
      void applyWebviewZoomPercent(percent).catch((error) => {
        console.error("[外观] WebView 缩放失败", error);
      });
    }, delayMs);
  }

  function syncWebviewZoomPercentFromBackend(percent: unknown) {
    const normalizedPercent = normalizeWebviewZoomPercent(percent);
    if (webviewZoomApplyTimer) {
      window.clearTimeout(webviewZoomApplyTimer);
      webviewZoomApplyTimer = null;
    }
    webviewZoomFactor.value = normalizedPercent / 100;
    if (bindings.config.webviewZoomPercent !== normalizedPercent) {
      suppressNextWebviewZoomApply = true;
      bindings.config.webviewZoomPercent = normalizedPercent;
    }
  }

  function updateWebviewZoomPercent(value: unknown) {
    bindings.config.webviewZoomPercent = normalizeWebviewZoomPercent(value);
  }

  function updateGithubUpdateMethod(value: unknown) {
    const normalized = String(value || "").trim();
    const nextMethod = normalized === "direct" || normalized === "proxy" ? normalized : "auto";
    if (bindings.config.githubUpdateMethod === nextMethod) return;
    const previousMethod = bindings.config.githubUpdateMethod || "auto";
    bindings.config.githubUpdateMethod = nextMethod;
    void invokeTauri<AppConfig>("set_github_update_method", { updateMethod: nextMethod })
      .then((saved) => {
        bindings.config.githubUpdateMethod = saved.githubUpdateMethod || "auto";
      })
      .catch((error) => {
        bindings.config.githubUpdateMethod = previousMethod;
        bindings.setStatusError("status.saveConfigFailed", error);
      });
  }

  function webviewZoomOptionIndex(percent: unknown) {
    const normalizedPercent = normalizeWebviewZoomPercent(percent);
    const index = WEBVIEW_ZOOM_PERCENT_OPTIONS.findIndex((item) => item === normalizedPercent);
    return index >= 0 ? index : 2;
  }

  function stepWebviewZoomPercent(direction: number) {
    const currentPercent = Number(bindings.config.webviewZoomPercent ?? Math.round(webviewZoomFactor.value * 100));
    const currentIndex = webviewZoomOptionIndex(currentPercent);
    const nextIndex = Math.min(
      WEBVIEW_ZOOM_PERCENT_OPTIONS.length - 1,
      Math.max(0, currentIndex + (direction > 0 ? 1 : -1)),
    );
    updateWebviewZoomPercent(WEBVIEW_ZOOM_PERCENT_OPTIONS[nextIndex]);
  }

  function hasZoomModifier(event: WheelEvent | KeyboardEvent) {
    return !!event.ctrlKey || !!event.metaKey;
  }

  function handleGlobalZoomWheel(event: WheelEvent) {
    if (!hasZoomModifier(event)) return;
    event.preventDefault();
    const direction = event.deltaY < 0 ? 1 : -1;
    stepWebviewZoomPercent(direction);
  }

  function handleGlobalZoomKeydown(event: KeyboardEvent) {
    if (!hasZoomModifier(event)) return;
    const key = String(event.key || "").trim();
    if (key === "+" || key === "=") {
      event.preventDefault();
      stepWebviewZoomPercent(1);
      return;
    }
    if (key === "-" || key === "_") {
      event.preventDefault();
      stepWebviewZoomPercent(-1);
      return;
    }
    if (key === "0") {
      event.preventDefault();
      updateWebviewZoomPercent(100);
    }
  }

  onMounted(() => {
    void listen<{ percent?: unknown }>("easy-call:webview-zoom-updated", (event) => {
      syncWebviewZoomPercentFromBackend(event.payload?.percent);
    })
      .then((unlisten) => {
        webviewZoomUpdatedUnlisten = unlisten;
      })
      .catch((error) => {
        console.error("[外观] WebView 缩放同步监听器注册失败", error);
      });

    window.addEventListener("wheel", handleGlobalZoomWheel, { passive: false });
    window.addEventListener("keydown", handleGlobalZoomKeydown);
  });

  onBeforeUnmount(() => {
    if (webviewZoomUpdatedUnlisten) {
      webviewZoomUpdatedUnlisten();
      webviewZoomUpdatedUnlisten = null;
    }
    window.removeEventListener("wheel", handleGlobalZoomWheel);
    window.removeEventListener("keydown", handleGlobalZoomKeydown);
    if (webviewZoomApplyTimer) {
      window.clearTimeout(webviewZoomApplyTimer);
      webviewZoomApplyTimer = null;
    }
  });

  watch(
    () => bindings.config.webviewZoomPercent,
    (percent) => {
      const normalizedPercent = normalizeWebviewZoomPercent(percent);
      if (bindings.config.webviewZoomPercent !== normalizedPercent) {
        bindings.config.webviewZoomPercent = normalizedPercent;
        return;
      }
      if (!webviewZoomWatcherReady) {
        webviewZoomWatcherReady = true;
        void applyWebviewZoomPercent(normalizedPercent).catch((error) => {
          console.error("[外观] WebView 缩放失败", error);
        });
        return;
      }
      if (suppressNextWebviewZoomApply) {
        suppressNextWebviewZoomApply = false;
        return;
      }
      scheduleWebviewZoomPercentApply(normalizedPercent);
    },
    { immediate: true },
  );

  return {
    webviewZoomFactor,
    normalizeWebviewZoomPercent,
    updateWebviewZoomPercent,
    updateGithubUpdateMethod,
  };
}

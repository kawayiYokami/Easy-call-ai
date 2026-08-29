import { isDesktopTauriHost } from "../../../services/tauri-api";

const MOBILE_CHAT_BREAKPOINT_PX = 768;

/**
 * 移动浏览器会在输入框持续聚焦时保留软键盘。
 * 桌面端继续保留“回复结束后回焦”，窄屏触摸端则按手机交互处理。
 * 桌面 Tauri 宿主（含触摸屏电脑）不进入手机模式，仅 Web 宿主按窄屏 + 触摸判定。
 */
export function isMobileTouchViewport(): boolean {
  if (typeof window === "undefined") return false;
  if (isDesktopTauriHost()) return false;

  const viewportWidth = Number(window.visualViewport?.width || window.innerWidth || 0);
  if (!Number.isFinite(viewportWidth) || viewportWidth <= 0 || viewportWidth >= MOBILE_CHAT_BREAKPOINT_PX) {
    return false;
  }

  const hasCoarsePointer = typeof window.matchMedia === "function"
    && window.matchMedia("(pointer: coarse)").matches;
  const maxTouchPoints = typeof navigator !== "undefined" ? Number(navigator.maxTouchPoints || 0) : 0;
  return hasCoarsePointer || maxTouchPoints > 0;
}

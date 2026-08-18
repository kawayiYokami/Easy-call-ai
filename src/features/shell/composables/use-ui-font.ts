/** 内置等宽字体的 font-family 名（style.css @font-face 注册）。 */
export const BUILTIN_CODE_FONT_FAMILY = "Easy Call JetBrains Mono";

export function normalizeUiFont(value: string): string {
  const text = String(value || "").trim();
  if (!text) return "auto";
  if (text.length > 128) return text.slice(0, 128).trim() || "auto";
  return text;
}

export function resolveUiFontFamily(uiFont: string, uiLanguage: string): string {
  const normalized = normalizeUiFont(uiFont);
  if (normalized === "auto") {
    if (uiLanguage === "zh-CN") {
      return "\"Microsoft YaHei\", \"PingFang SC\", \"Noto Sans CJK SC\", \"Segoe UI\", system-ui, sans-serif";
    }
    if (uiLanguage === "zh-TW") {
      return "\"PingFang TC\", \"Microsoft JhengHei\", \"Noto Sans CJK TC\", \"Segoe UI\", system-ui, sans-serif";
    }
    return "\"Segoe UI\", \"SF Pro Text\", system-ui, -apple-system, Roboto, \"Helvetica Neue\", Arial, sans-serif";
  }
  const escaped = normalized.replace(/"/g, '\\"');
  return `"${escaped}", system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif`;
}

/** 代码字体：auto 使用内置等宽字体，指定值则优先用户字体、回退内置等宽与系统等宽字体。 */
export function resolveCodeFontFamily(codeFont: string): string {
  const normalized = normalizeUiFont(codeFont);
  if (normalized === "auto" || normalized === BUILTIN_CODE_FONT_FAMILY) {
    return `"${BUILTIN_CODE_FONT_FAMILY}", ui-monospace, SFMono-Regular, Menlo, Consolas, monospace`;
  }
  const escaped = normalized.replace(/"/g, '\\"');
  return `"${escaped}", "${BUILTIN_CODE_FONT_FAMILY}", ui-monospace, SFMono-Regular, Menlo, Consolas, monospace`;
}

export function applyUiFont(uiFont: string, uiLanguage: string) {
  const family = resolveUiFontFamily(uiFont, uiLanguage);
  document.documentElement.style.setProperty("--app-font-family", family);
}

export function applyCodeFont(codeFont: string) {
  const family = resolveCodeFontFamily(codeFont);
  document.documentElement.style.setProperty("--app-code-font-family", family);
}
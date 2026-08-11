import { computed, ref } from "vue";
import { emitTransportEvent } from "../../../services/tauri-api";
import {
  buildGeneratedThemeStyleText,
  DEFAULT_GENERATED_THEME_CONTROLS,
  GENERATED_THEME_DARK_ID,
  GENERATED_THEME_LIGHT_ID,
  GENERATED_THEME_NAME,
  generateGeneratedThemeTokens,
  getGeneratedThemeDefaultControls,
  normalizeGeneratedThemeControls,
  themeStateToThemeId,
} from "../theme/theme-generator";
import type {
  AppThemeState,
  GeneratedThemeControls,
  GeneratedThemeControlsByMode,
  GeneratedThemeTokens,
  PersistedThemePreferences,
  ThemeMode,
  ThemeModeKind,
} from "../theme/theme-types";

const THEME_STORAGE_KEY = "easy-call.theme-state.v1";
const LEGACY_THEME_STORAGE_KEY = "theme";
const GENERATED_THEME_STYLE_ID = "easy-call-generated-theme-style";
const LEGACY_DARK_THEME_NAMES = new Set<string>([
  "dark",
  "halloween",
  "forest",
  "luxury",
  "dracula",
  "business",
  "night",
  "coffee",
  "dim",
]);
export const APP_THEMES = [
  "light",
  "dark",
  "cupcake",
  "emerald",
  "corporate",
  "halloween",
  "garden",
  "forest",
  "lofi",
  "fantasy",
  "luxury",
  "dracula",
  "autumn",
  "business",
  "night",
  "coffee",
  "winter",
  "dim",
] as const;
export const DARK_APP_THEMES = new Set<string>(LEGACY_DARK_THEME_NAMES);
type AppTheme = (typeof APP_THEMES)[number];
const THEME_SET = new Set<string>(APP_THEMES);

function cloneGeneratedThemeControls(
  controls: Partial<GeneratedThemeControls> | GeneratedThemeControls,
): GeneratedThemeControls {
  return normalizeGeneratedThemeControls({ ...controls });
}

function createGeneratedThemeControlsByMode(
  input?: Partial<GeneratedThemeControlsByMode> | null,
  legacyControls?: Partial<GeneratedThemeControls> | GeneratedThemeControls | null,
): GeneratedThemeControlsByMode {
  const defaults: GeneratedThemeControlsByMode = {
    light: cloneGeneratedThemeControls({ ...getGeneratedThemeDefaultControls("light"), mode: "light" }),
    dark: cloneGeneratedThemeControls({ ...getGeneratedThemeDefaultControls("dark"), mode: "dark" }),
  };
  const normalizedLegacy = legacyControls ? cloneGeneratedThemeControls(legacyControls) : null;
  const next: GeneratedThemeControlsByMode = {
    light: input?.light
      ? cloneGeneratedThemeControls({ ...input.light, mode: "light" })
      : defaults.light,
    dark: input?.dark
      ? cloneGeneratedThemeControls({ ...input.dark, mode: "dark" })
      : defaults.dark,
  };
  if (normalizedLegacy) {
    next[normalizedLegacy.mode] = cloneGeneratedThemeControls(normalizedLegacy);
  }
  return next;
}

const currentThemeState = ref<AppThemeState>({
  kind: "generated",
  controls: cloneGeneratedThemeControls(DEFAULT_GENERATED_THEME_CONTROLS),
});
const currentTheme = ref<string>(themeStateToThemeId(currentThemeState.value));
const activeGeneratedMode = ref<ThemeMode>(DEFAULT_GENERATED_THEME_CONTROLS.mode);
const generatedThemeControlsByMode = ref<GeneratedThemeControlsByMode>(createGeneratedThemeControlsByMode());
const generatedThemeControls = computed(() => generatedThemeControlsByMode.value[activeGeneratedMode.value]);
// 自动主题模式：跟随系统深浅切换；autoLight/autoDark 为主题 id（预设名或 generated-light/dark）
const themeMode = ref<ThemeModeKind>("manual");
const autoLightTheme = ref<string>(GENERATED_THEME_LIGHT_ID);
const autoDarkTheme = ref<string>(GENERATED_THEME_DARK_ID);
const systemPrefersDark = ref(false);
let systemThemeListenerRegistered = false;

function isVscodeHost(): boolean {
  if (typeof document === "undefined") return false;
  return document.documentElement.getAttribute("data-host") === "vscode";
}

function resolveSystemPrefersDark(): boolean {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return false;
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

function applyAutoModeTheme(): boolean {
  if (isVscodeHost()) return false; // VS Code 侧边栏只跟随 VS Code 主题，不参与自动模式
  const targetId = systemPrefersDark.value ? autoDarkTheme.value : autoLightTheme.value;
  return applyThemeState(targetId);
}

function registerSystemThemeListener() {
  if (typeof window === "undefined" || typeof window.matchMedia !== "function") return;
  if (isVscodeHost()) return; // VS Code 侧边栏只跟随 VS Code 主题，不参与自动模式
  if (systemThemeListenerRegistered) return;
  systemThemeListenerRegistered = true;
  systemPrefersDark.value = resolveSystemPrefersDark();
  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", (event) => {
    systemPrefersDark.value = event.matches;
    if (themeMode.value === "auto") {
      applyAutoModeTheme();
    }
  });
}

function isGeneratedThemeState(value: unknown): value is Extract<AppThemeState, { kind: "generated" }> {
  if (!value || typeof value !== "object") return false;
  const state = value as { kind?: unknown; controls?: unknown };
  return state.kind === "generated" && !!state.controls && typeof state.controls === "object";
}

function cloneThemeState(state: AppThemeState): AppThemeState {
  if (state.kind === "preset") {
    return {
      kind: "preset",
      name: state.name,
    };
  }
  return {
    kind: "generated",
    controls: cloneGeneratedThemeControls(state.controls),
  };
}

function isValidTheme(value: unknown): value is AppTheme {
  return typeof value === "string" && THEME_SET.has(value);
}

function normalizePresetTheme(value: unknown): AppTheme | null {
  const themeName = String(value || "").trim().toLowerCase();
  if (themeName === "pastel") return "autumn";
  return isValidTheme(themeName) ? themeName : null;
}

function ensureGeneratedThemeStyleElement(): HTMLStyleElement | null {
  if (typeof document === "undefined") return null;
  const existing = document.getElementById(GENERATED_THEME_STYLE_ID);
  if (existing instanceof HTMLStyleElement) {
    return existing;
  }
  const element = document.createElement("style");
  element.id = GENERATED_THEME_STYLE_ID;
  document.head.appendChild(element);
  return element;
}

function clearGeneratedThemeStyle() {
  if (typeof document === "undefined") return;
  const element = document.getElementById(GENERATED_THEME_STYLE_ID);
  if (element) {
    element.remove();
  }
}

function persistThemePreferences() {
  if (typeof window === "undefined") return;
  const activeState = cloneThemeState(currentThemeState.value);
  const payload: PersistedThemePreferences = {
    version: 3,
    mode: themeMode.value,
    activeState,
    autoLightTheme: autoLightTheme.value,
    autoDarkTheme: autoDarkTheme.value,
    generatedControls: cloneGeneratedThemeControls(generatedThemeControls.value),
    generatedControlsByMode: createGeneratedThemeControlsByMode(generatedThemeControlsByMode.value),
  };
  window.localStorage.setItem(THEME_STORAGE_KEY, JSON.stringify(payload));
  window.localStorage.setItem(LEGACY_THEME_STORAGE_KEY, currentTheme.value);
}

function applyGeneratedTheme(controlsInput: Partial<GeneratedThemeControls> | GeneratedThemeControls): boolean {
  if (typeof document === "undefined") return false;
  const controls = cloneGeneratedThemeControls(controlsInput);
  const tokens = generateGeneratedThemeTokens(controls);
  const styleElement = ensureGeneratedThemeStyleElement();
  if (!styleElement) return false;
  generatedThemeControlsByMode.value = {
    ...generatedThemeControlsByMode.value,
    [controls.mode]: controls,
  };
  activeGeneratedMode.value = controls.mode;
  currentThemeState.value = { kind: "generated", controls };
  currentTheme.value = themeStateToThemeId(currentThemeState.value);
  styleElement.textContent = buildGeneratedThemeStyleText(tokens);
  document.documentElement.setAttribute("data-theme", GENERATED_THEME_NAME);
  persistThemePreferences();
  return true;
}

function applyPresetTheme(theme: AppTheme): boolean {
  if (typeof document === "undefined") return false;
  currentTheme.value = theme;
  currentThemeState.value = { kind: "preset", name: theme };
  clearGeneratedThemeStyle();
  document.documentElement.setAttribute("data-theme", theme);
  persistThemePreferences();
  return true;
}

function resolveLegacyThemeMode(value: unknown): ThemeMode {
  const themeName = String(value || "").trim().toLowerCase();
  return LEGACY_DARK_THEME_NAMES.has(themeName) ? "dark" : "light";
}

function applyThemeState(nextState: AppThemeState | string | null | undefined): boolean {
  if (typeof nextState === "string") {
    const presetTheme = normalizePresetTheme(nextState);
    if (presetTheme) {
      return applyPresetTheme(presetTheme);
    }
    if (nextState === GENERATED_THEME_LIGHT_ID || nextState === GENERATED_THEME_DARK_ID) {
      const targetMode = nextState === GENERATED_THEME_LIGHT_ID ? "light" : "dark";
      return applyGeneratedTheme(generatedThemeControlsByMode.value[targetMode]);
    }
    return applyGeneratedTheme(generatedThemeControlsByMode.value[resolveLegacyThemeMode(nextState)]);
  }
  if (!nextState) return false;
  if (nextState.kind === "preset") {
    const presetTheme = normalizePresetTheme(nextState.name);
    return presetTheme ? applyPresetTheme(presetTheme) : false;
  }
  return applyGeneratedTheme(nextState.controls);
}

function readStoredThemePreferences(): PersistedThemePreferences | null {
  if (typeof window === "undefined") return null;
  const raw = window.localStorage.getItem(THEME_STORAGE_KEY);
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as Partial<PersistedThemePreferences> | null;
    if (!parsed || typeof parsed !== "object") return null;
    const storedControls = cloneGeneratedThemeControls(parsed.generatedControls || DEFAULT_GENERATED_THEME_CONTROLS);
    const storedControlsByMode = createGeneratedThemeControlsByMode(
      parsed.generatedControlsByMode && typeof parsed.generatedControlsByMode === "object"
        ? parsed.generatedControlsByMode
        : null,
      storedControls,
    );
    // 自动模式主题：老数据（v2 无此字段）读取时给默认值，不写回、不改动老数据
    const autoLight = typeof parsed.autoLightTheme === "string" && parsed.autoLightTheme.trim()
      ? parsed.autoLightTheme.trim()
      : GENERATED_THEME_LIGHT_ID;
    const autoDark = typeof parsed.autoDarkTheme === "string" && parsed.autoDarkTheme.trim()
      ? parsed.autoDarkTheme.trim()
      : GENERATED_THEME_DARK_ID;
    const activeState = parsed.activeState && typeof parsed.activeState === "object" ? parsed.activeState : null;
    if (isGeneratedThemeState(activeState)) {
      const activeControls = cloneGeneratedThemeControls(activeState.controls);
      storedControlsByMode[activeControls.mode] = activeControls;
      return {
        version: 3,
        mode: parsed.mode === "auto" ? "auto" : "manual",
        activeState: {
          kind: "generated",
          controls: activeControls,
        },
        autoLightTheme: autoLight,
        autoDarkTheme: autoDark,
        generatedControls: storedControls,
        generatedControlsByMode: storedControlsByMode,
      };
    }
    const legacyThemeName = (activeState as { name?: unknown } | null)?.name;
    const presetTheme = normalizePresetTheme(legacyThemeName);
    if (presetTheme) {
      return {
        version: 3,
        mode: parsed.mode === "auto" ? "auto" : "manual",
        activeState: {
          kind: "preset",
          name: presetTheme,
        },
        autoLightTheme: autoLight,
        autoDarkTheme: autoDark,
        generatedControls: storedControls,
        generatedControlsByMode: storedControlsByMode,
      };
    }
    const migratedMode =
      legacyThemeName === undefined || legacyThemeName === null || String(legacyThemeName).trim() === ""
        ? storedControls.mode
        : resolveLegacyThemeMode(legacyThemeName);
    const migratedControls = cloneGeneratedThemeControls({
      ...storedControlsByMode[migratedMode],
      ...storedControls,
      mode: migratedMode,
    });
    storedControlsByMode[migratedMode] = migratedControls;
    return {
      version: 3,
      mode: parsed.mode === "auto" ? "auto" : "manual",
      activeState: {
        kind: "generated",
        controls: migratedControls,
      },
      autoLightTheme: autoLight,
      autoDarkTheme: autoDark,
      generatedControls: storedControls,
      generatedControlsByMode: storedControlsByMode,
    };
  } catch {
    return null;
  }
}

export function isDarkAppTheme(theme: string): boolean {
  const normalizedTheme = String(theme || "").trim();
  return normalizedTheme === GENERATED_THEME_DARK_ID || DARK_APP_THEMES.has(normalizedTheme);
}

export function useAppTheme() {
  const generatedThemeTokens = computed(() => generateGeneratedThemeTokens(generatedThemeControls.value));
  const generatedThemeTokensByMode = computed<Record<ThemeMode, GeneratedThemeTokens>>(() => ({
    light: generateGeneratedThemeTokens(generatedThemeControlsByMode.value.light),
    dark: generateGeneratedThemeTokens(generatedThemeControlsByMode.value.dark),
  }));

  function isPersistedPreferences(value: unknown): value is PersistedThemePreferences {
    return !!value && typeof value === "object" && "mode" in value && "activeState" in value;
  }

  function applyTheme(theme: PersistedThemePreferences | AppThemeState | string): boolean {
    // 跨窗口同步：配置窗口广播完整 v3 状态（含 mode/autoLight/autoDark），各窗口按模式应用
    if (isPersistedPreferences(theme)) {
      generatedThemeControlsByMode.value = createGeneratedThemeControlsByMode(
        theme.generatedControlsByMode,
        theme.generatedControls,
      );
      autoLightTheme.value = theme.autoLightTheme || GENERATED_THEME_LIGHT_ID;
      autoDarkTheme.value = theme.autoDarkTheme || GENERATED_THEME_DARK_ID;
      themeMode.value = theme.mode === "auto" ? "auto" : "manual";
      if (theme.mode === "auto") {
        return applyAutoModeTheme();
      }
      if (theme.activeState.kind === "preset") {
        if (isValidTheme(theme.activeState.name)) {
          return applyPresetTheme(theme.activeState.name);
        }
        return false;
      }
      return applyGeneratedTheme(theme.activeState.controls);
    }
    return applyThemeState(theme);
  }

  function restoreThemeFromStorage() {
    registerSystemThemeListener();
    const storedPreferences = readStoredThemePreferences();
    if (storedPreferences) {
      generatedThemeControlsByMode.value = createGeneratedThemeControlsByMode(
        storedPreferences.generatedControlsByMode,
        storedPreferences.generatedControls,
      );
      autoLightTheme.value = storedPreferences.autoLightTheme || GENERATED_THEME_LIGHT_ID;
      autoDarkTheme.value = storedPreferences.autoDarkTheme || GENERATED_THEME_DARK_ID;
      themeMode.value = storedPreferences.mode === "auto" ? "auto" : "manual";
      if (storedPreferences.mode === "auto") {
        applyAutoModeTheme();
        return;
      }
      if (storedPreferences.activeState.kind === "preset") {
        if (isValidTheme(storedPreferences.activeState.name)) {
          applyPresetTheme(storedPreferences.activeState.name);
        }
      } else {
        applyGeneratedTheme(storedPreferences.activeState.controls);
      }
      return;
    }

    if (typeof window === "undefined") return;
    const savedTheme = window.localStorage.getItem(LEGACY_THEME_STORAGE_KEY);
    if (isValidTheme(savedTheme)) {
      applyPresetTheme(savedTheme);
      return;
    }
    // 全新安装无任何主题记录：默认自动模式，浅色/深色都用自定义主题。
    themeMode.value = "auto";
    autoLightTheme.value = GENERATED_THEME_LIGHT_ID;
    autoDarkTheme.value = GENERATED_THEME_DARK_ID;
    applyAutoModeTheme();
  }

  function emitThemeChanged() {
    const payload: PersistedThemePreferences = {
      version: 3,
      mode: themeMode.value,
      activeState: cloneThemeState(currentThemeState.value),
      autoLightTheme: autoLightTheme.value,
      autoDarkTheme: autoDarkTheme.value,
      generatedControls: cloneGeneratedThemeControls(generatedThemeControls.value),
      generatedControlsByMode: createGeneratedThemeControlsByMode(generatedThemeControlsByMode.value),
    };
    return emitTransportEvent("theme.changed", payload).catch((error) => {
      console.warn("[主题] 同步主题变化失败", error);
    });
  }

  function setTheme(theme: string) {
    if (!isValidTheme(theme)) return;
    themeMode.value = "manual"; // 手动选预设主题即退出自动模式
    if (!applyPresetTheme(theme)) return;
    void emitThemeChanged();
  }

  function setThemeMode(mode: ThemeModeKind) {
    if (mode === "auto") {
      themeMode.value = "auto";
      applyAutoModeTheme();
    } else {
      themeMode.value = "manual";
    }
    void emitThemeChanged();
  }

  function setAutoTheme(side: ThemeMode, themeId: string) {
    if (side === "light") {
      autoLightTheme.value = themeId;
    } else {
      autoDarkTheme.value = themeId;
    }
    themeMode.value = "auto";
    applyAutoModeTheme();
    void emitThemeChanged();
  }

  function activateGeneratedTheme() {
    const targetMode =
      currentThemeState.value.kind === "preset" ? resolveLegacyThemeMode(currentTheme.value) : activeGeneratedMode.value;
    const nextState: AppThemeState = {
      kind: "generated",
      controls: cloneGeneratedThemeControls(generatedThemeControlsByMode.value[targetMode]),
    };
    themeMode.value = "manual"; // 手动激活自定义主题即退出自动模式
    if (!applyGeneratedTheme(nextState.controls)) return;
    void emitThemeChanged();
  }

  function updateGeneratedThemeControls(patch: Partial<GeneratedThemeControls>) {
    const targetMode = patch.mode ?? activeGeneratedMode.value;
    const baseControls = generatedThemeControlsByMode.value[targetMode];
    const nextControls = cloneGeneratedThemeControls({
      ...baseControls,
      ...patch,
      mode: targetMode,
    });
    if (!applyGeneratedTheme(nextControls)) return;
    void emitThemeChanged();
  }

  function resetGeneratedTheme() {
    updateGeneratedThemeControls(getGeneratedThemeDefaultControls(activeGeneratedMode.value));
  }

  function toggleTheme() {
    if (themeMode.value === "auto") {
      // 自动模式下忽略手动明暗切换（跟随系统）
      return;
    }
    if (currentThemeState.value.kind === "preset") {
      setTheme(currentTheme.value === "light" ? "dark" : "light");
      return;
    }
    const nextMode = currentThemeState.value.controls.mode === "light" ? "dark" : "light";
    const nextControls = cloneGeneratedThemeControls(generatedThemeControlsByMode.value[nextMode]);
    if (!applyGeneratedTheme(nextControls)) return;
    void emitThemeChanged();
  }

  return {
    currentTheme,
    generatedThemeControls,
    generatedThemeControlsByMode,
    generatedThemeTokens,
    generatedThemeTokensByMode,
    themeMode,
    autoLightTheme,
    autoDarkTheme,
    systemPrefersDark,
    applyTheme,
    setTheme,
    setThemeMode,
    setAutoTheme,
    activateGeneratedTheme,
    updateGeneratedThemeControls,
    resetGeneratedTheme,
    restoreThemeFromStorage,
    toggleTheme,
  };
}

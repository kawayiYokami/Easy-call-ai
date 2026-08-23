<template>
  <ConfigTemplate :model-value="templateValues" :groups="templateGroups">
    <template #row-language>
      <div class="flex min-w-0 flex-wrap items-center justify-between gap-3">
        <span class="text-sm">{{ t("appearance.textLanguage") }}</span>
        <select
          class="select select-bordered select-sm w-52 max-w-full shrink-0"
          :value="props.uiLanguage"
          @change="$emit('update:uiLanguage', ($event.target as HTMLSelectElement).value)"
        >
          <option v-for="opt in props.localeOptions" :key="opt.value" :value="opt.value">{{ opt.label }}</option>
        </select>
      </div>
    </template>

    <template #row-markdown-font-scale>
      <div class="flex min-w-0 flex-wrap items-center justify-between gap-3">
        <span class="text-sm">{{ t("appearance.textWeight") }}</span>
        <SegmentedControl
          :model-value="markdownFontScale < 1 ? 0 : 1"
          :options="markdownFontScaleOptions"
          :full-width="false"
          class="max-w-full shrink-0"
          @change="setMarkdownFontScale"
        />
      </div>
    </template>

    <template v-if="fontsAvailable" #row-ui-font>
      <div class="flex min-w-0 flex-wrap items-center justify-between gap-3">
        <span class="text-sm">{{ t("appearance.uiFont") }}</span>
        <FontFamilySelect
          :model-value="props.uiFont || 'auto'"
          :options="uiFontOptions"
          :auto-label="t('appearance.fontAuto')"
          :disabled="fontsLoading"
          @update:model-value="$emit('update:uiFont', $event)"
        />
      </div>
    </template>

    <template v-if="fontsAvailable" #row-code-font>
      <div class="flex min-w-0 flex-wrap items-center justify-between gap-3">
        <span class="text-sm">{{ t("appearance.codeFont") }}</span>
        <FontFamilySelect
          :model-value="props.codeFont || 'auto'"
          :options="codeFontOptions"
          :auto-label="t('appearance.fontAutoCode')"
          :disabled="fontsLoading"
          @update:model-value="$emit('update:codeFont', $event)"
        />
      </div>
    </template>

    <template #row-chat-bubble-background>
      <label class="flex min-w-0 cursor-pointer items-center justify-between gap-4">
        <span class="text-sm">{{ t("appearance.chatBubbleBackground") }}</span>
        <input
          :checked="assistantBubbleBackgroundEnabled"
          type="checkbox"
          class="toggle toggle-sm toggle-primary shrink-0"
          @change="setAssistantBubbleBackgroundEnabled(($event.target as HTMLInputElement).checked)"
        />
      </label>
    </template>
    <template #row-chat-bubble-markdown>
      <label class="flex min-w-0 cursor-pointer items-center justify-between gap-4">
        <span class="text-sm">{{ t("appearance.chatBubbleSegmentedMarkdown") }}</span>
        <input
          :checked="segmentedMarkdownEnabled"
          type="checkbox"
          class="toggle toggle-sm toggle-primary shrink-0"
          @change="setSegmentedMarkdownEnabled(($event.target as HTMLInputElement).checked)"
        />
      </label>
    </template>
    <template #row-chat-bubble-time>
      <label class="flex min-w-0 cursor-pointer items-center justify-between gap-4">
        <span class="text-sm">{{ t("appearance.chatBubbleFullTime") }}</span>
        <input
          :checked="chatTimeDisplayMode === 'absolute'"
          type="checkbox"
          class="toggle toggle-sm toggle-primary shrink-0"
          @change="setChatTimeDisplayMode(($event.target as HTMLInputElement).checked ? 'absolute' : 'relative')"
        />
      </label>
    </template>
    <template #row-chat-bubble-markdown-layout>
      <div class="flex min-w-0 flex-wrap items-center justify-between gap-3">
        <span class="text-sm">{{ t("appearance.chatBubbleMarkdownLayout") }}</span>
        <SegmentedControl
          :model-value="markdownLayout"
          :options="markdownLayoutOptions"
          size="sm"
          :full-width="false"
          class="max-w-full shrink-0"
          @change="setChatMarkdownLayout"
        />
      </div>
    </template>

    <template #row-input-side-file-tags>
      <label class="flex min-w-0 cursor-pointer items-center justify-between gap-4">
        <span class="text-sm">{{ t("appearance.inputPanelSideFileTags") }}</span>
        <input
          :checked="sideFileTagsEnabled"
          type="checkbox"
          class="toggle toggle-sm toggle-primary shrink-0"
          @change="setSideFileTagsEnabled(($event.target as HTMLInputElement).checked)"
        />
      </label>
    </template>
    <template #row-input-ide-bridge-file-tags>
      <label class="flex min-w-0 cursor-pointer items-center justify-between gap-4">
        <span class="text-sm">{{ t("appearance.inputPanelIdeBridgeFileTags") }}</span>
        <input
          :checked="ideBridgeFileTagsEnabled"
          type="checkbox"
          class="toggle toggle-sm toggle-primary shrink-0"
          @change="setIdeBridgeFileTagsEnabled(($event.target as HTMLInputElement).checked)"
        />
      </label>
    </template>

    <template #row-file-reader-line-wrap>
      <label class="flex min-w-0 cursor-pointer items-center justify-between gap-4">
        <div class="min-w-0">
          <div class="text-sm">{{ t("appearance.fileReaderLineWrap") }}</div>
          <div class="mt-1 text-xs text-base-content/60">{{ t("appearance.fileReaderLineWrapHint") }}</div>
        </div>
        <input
          :checked="fileReaderLineWrapEnabled"
          type="checkbox"
          class="toggle toggle-sm toggle-primary shrink-0"
          @change="setFileReaderLineWrapEnabled(($event.target as HTMLInputElement).checked)"
        />
      </label>
    </template>

    <template #row-ui-size-scale>
      <div class="grid min-w-0 gap-2">
        <div class="flex items-center justify-between gap-3">
          <span class="text-sm">{{ t("appearance.uiSizeScale") }}</span>
          <output class="text-sm font-medium tabular-nums">{{ uiSizeScale }}%</output>
        </div>
        <input
          class="range range-primary w-full"
          type="range"
          min="75"
          max="150"
          step="1"
          :value="uiSizeScale"
          :aria-label="t('appearance.uiSizeScale')"
          @input="$emit('update:uiSizeScale', Number(($event.target as HTMLInputElement).value))"
        />
        <div class="grid grid-cols-4 gap-1">
          <button
            v-for="scale in uiSizeScaleMarks"
            :key="scale"
            class="btn btn-xs text-caption tabular-nums"
            :class="uiSizeScale === scale ? 'btn-primary' : 'btn-ghost text-base-content/60'"
            :aria-pressed="uiSizeScale === scale"
            type="button"
            @click="$emit('update:uiSizeScale', scale)"
          >
            {{ scale }}%
          </button>
        </div>
      </div>
    </template>

    <template #row-theme>
      <div class="grid min-w-0 gap-4">
        <div class="tabs tabs-box bg-base-200 p-1">
          <button
            type="button"
            class="tab flex-1 rounded-btn"
            :class="activeTab === 'auto' ? 'tab-active' : ''"
            @click="switchToAutoTab"
          >
            {{ t("appearance.themeTabs.auto") }}
          </button>
          <button
            type="button"
            class="tab flex-1 rounded-btn"
            :class="activeTab === 'preset' ? 'tab-active' : ''"
            @click="switchToPresetTab"
          >
            {{ t("appearance.themeTabs.preset") }}
          </button>
          <button
            type="button"
            class="tab flex-1 rounded-btn"
            :class="activeTab === 'generated' ? 'tab-active' : ''"
            @click="switchToGeneratedTab"
          >
            {{ t("appearance.themeTabs.generated") }}
          </button>
        </div>

        <AutoThemeGrid
          v-if="activeTab === 'auto'"
          :light-themes="lightThemes"
          :dark-themes="darkThemes"
          :auto-light-theme="props.autoLightTheme"
          :auto-dark-theme="props.autoDarkTheme"
          :light-custom-tokens="props.generatedLightTokens"
          :dark-custom-tokens="props.generatedDarkTokens"
          @select-light="$emit('setAutoTheme', 'light', $event)"
          @select-dark="$emit('setAutoTheme', 'dark', $event)"
        />

        <ThemePreviewGrid
          v-else-if="activeTab === 'preset'"
          :light-themes="lightThemes"
          :dark-themes="darkThemes"
          :current-theme="props.currentTheme"
          @select="$emit('setTheme', $event)"
        />

        <GeneratedThemeEditor
          v-else
          :controls="props.generatedThemeControls"
          :tokens="props.generatedThemeTokens"
          @update-controls="$emit('updateGeneratedThemeControls', $event)"
          @reset="$emit('resetGeneratedTheme')"
        />
      </div>
    </template>
  </ConfigTemplate>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { canUseTransportSystemFonts, listTransportSystemFonts } from "../../../../services/tauri-api";
import SegmentedControl from "../../components/SegmentedControl.vue";
import ConfigTemplate from "../../components/ConfigTemplate.vue";
import FontFamilySelect from "../../components/FontFamilySelect.vue";
import type { ConfigTemplateGroup } from "../../components/config-template";
import ThemePreviewGrid from "../../components/ThemePreviewGrid.vue";
import AutoThemeGrid from "../../components/AutoThemeGrid.vue";
import GeneratedThemeEditor from "../../components/GeneratedThemeEditor.vue";
import {
  APP_THEMES,
  DARK_APP_THEMES,
} from "../../../shell/composables/use-app-theme";
import type { GeneratedThemeControls, GeneratedThemeTokens, ThemeMode, ThemeModeKind } from "../../../shell/theme/theme-types";
import {
  GENERATED_THEME_DARK_ID,
  GENERATED_THEME_LIGHT_ID,
} from "../../../shell/theme/theme-generator";
import {
  useMarkdownAppearance,
} from "../../../shell/composables/use-markdown-appearance";
import { useChatMessageAppearance, type ChatMarkdownLayout } from "../../../shell/composables/use-chat-message-appearance";
import { SIDE_FILE_TAGS_AVAILABLE, useChatComposerAppearance } from "../../../shell/composables/use-chat-composer-appearance";
import { useFileReaderAppearance } from "../../../shell/composables/use-file-reader-appearance";

const props = defineProps<{
  uiLanguage: "zh-CN" | "en-US" | "zh-TW";
  uiFont?: string;
  codeFont?: string;
  localeOptions: Array<{ value: "zh-CN" | "en-US" | "zh-TW"; label: string }>;
  currentTheme: string;
  themeMode: ThemeModeKind;
  autoLightTheme: string;
  autoDarkTheme: string;
  generatedThemeControls: GeneratedThemeControls;
  generatedThemeTokens: GeneratedThemeTokens;
  generatedLightTokens: GeneratedThemeTokens;
  generatedDarkTokens: GeneratedThemeTokens;
  uiSizeScale: number;
}>();

const emit = defineEmits<{
  (e: "update:uiLanguage", value: string): void;
  (e: "update:uiFont", value: string): void;
  (e: "update:codeFont", value: string): void;
  (e: "update:uiSizeScale", value: number): void;
  (e: "setTheme", value: string): void;
  (e: "setThemeMode", value: ThemeModeKind): void;
  (e: "setAutoTheme", side: ThemeMode, value: string): void;
  (e: "activateGeneratedTheme"): void;
  (e: "updateGeneratedThemeControls", value: Partial<GeneratedThemeControls>): void;
  (e: "resetGeneratedTheme"): void;
}>();

const { t } = useI18n();
const templateValues = {};
const templateGroups = computed<ConfigTemplateGroup[]>(() => [
  {
    key: "text",
    title: t("appearance.text"),
    rows: [
      { key: "language", items: [] },
      { key: "markdown-font-scale", items: [] },
      ...(fontsAvailable.value
        ? [
            { key: "ui-font", items: [] },
            { key: "code-font", items: [] },
          ]
        : []),
    ],
  },
  {
    key: "chat-bubble",
    title: t("appearance.chatBubble"),
    rows: [
      { key: "chat-bubble-background", items: [] },
      { key: "chat-bubble-markdown", items: [] },
      { key: "chat-bubble-time", items: [] },
      { key: "chat-bubble-markdown-layout", items: [] },
    ],
  },
  {
    key: "input-panel",
    title: t("appearance.inputPanel"),
    rows: [
      ...(SIDE_FILE_TAGS_AVAILABLE ? [{ key: "input-side-file-tags", items: [] }] : []),
      { key: "input-ide-bridge-file-tags", items: [] },
    ],
  },
  {
    key: "file-reader",
    title: t("appearance.fileReader"),
    rows: [{ key: "file-reader-line-wrap", items: [] }],
  },
  {
    key: "ui-size-scale",
    title: t("appearance.uiSizeScale"),
    rows: [{ key: "ui-size-scale", items: [] }],
  },
  {
    key: "theme",
    title: t("appearance.theme"),
    rows: [{ key: "theme", items: [] }],
  },
]);
const uiSizeScaleMarks = [75, 100, 125, 150] as const;
type ThemeTab = "auto" | "preset" | "generated";
const activeTab = ref<ThemeTab>(
  props.themeMode === "auto" ? "auto" : isGeneratedTheme(props.currentTheme) ? "generated" : "preset",
);
const markdownFontScaleOptions = computed(() => [
  { value: 0, label: t("appearance.markdownFontScaleLight") },
  { value: 1, label: t("appearance.markdownFontScaleHeavy") },
]);
const markdownLayoutOptions = computed<Array<{ value: ChatMarkdownLayout; label: string }>>(() => [
  { value: "compact", label: t("appearance.markdownLayoutCompact") },
  { value: "comfortable", label: t("appearance.markdownLayoutComfortable") },
  { value: "relaxed", label: t("appearance.markdownLayoutRelaxed") },
]);
const lightThemes = computed(() => APP_THEMES.filter((theme) => !DARK_APP_THEMES.has(theme)));
const darkThemes = computed(() => APP_THEMES.filter((theme) => DARK_APP_THEMES.has(theme)));
const {
  markdownFontScale,
  setMarkdownFontScale,
} = useMarkdownAppearance();
const {
  assistantBubbleBackgroundEnabled,
  segmentedMarkdownEnabled,
  chatTimeDisplayMode,
  markdownLayout,
  setAssistantBubbleBackgroundEnabled,
  setSegmentedMarkdownEnabled,
  setChatTimeDisplayMode,
  setChatMarkdownLayout,
} = useChatMessageAppearance();
const {
  sideFileTagsEnabled,
  ideBridgeFileTagsEnabled,
  setSideFileTagsEnabled,
  setIdeBridgeFileTagsEnabled,
} = useChatComposerAppearance();
const {
  fileReaderLineWrapEnabled,
  setFileReaderLineWrapEnabled,
} = useFileReaderAppearance();

const fontsAvailable = computed(() => canUseTransportSystemFonts());
const systemFonts = ref<string[]>([]);
const monospaceFonts = ref<Set<string>>(new Set());
const fontsLoading = ref(false);

const uiFontOptions = computed(() => {
  const current = String(props.uiFont || "").trim();
  const set = new Set<string>();
  if (current && current !== "auto" && !monospaceFonts.value.has(current)) set.add(current);
  for (const name of systemFonts.value) {
    if (!monospaceFonts.value.has(name)) set.add(name);
  }
  return [...set];
});

const codeFontOptions = computed(() => {
  const current = String(props.codeFont || "").trim();
  const set = new Set<string>();
  // 无条件保留用户已保存的代码字体为候选项：即使系统字体枚举失败、或后端未将其标记为 monospace，
  // 下拉列表也必须能对应上当前选中的代码字体
  if (current && current !== "auto") set.add(current);
  for (const name of systemFonts.value) {
    if (monospaceFonts.value.has(name)) set.add(name);
  }
  return [...set];
});

onMounted(async () => {
  if (!fontsAvailable.value) return;
  fontsLoading.value = true;
  try {
    const fonts = await listTransportSystemFonts<{ family: string; monospace: boolean }[]>();
    if (Array.isArray(fonts)) {
      const families: string[] = [];
      const mono = new Set<string>();
      for (const item of fonts) {
        if (!item || typeof item.family !== "string" || !item.family.trim()) continue;
        families.push(item.family.trim());
        if (item.monospace) mono.add(item.family.trim());
      }
      systemFonts.value = families;
      monospaceFonts.value = mono;
    }
  } catch (error) {
    console.warn("[APPEARANCE] list_system_fonts failed:", error);
    systemFonts.value = [];
  } finally {
    fontsLoading.value = false;
  }
});

function isGeneratedTheme(theme: string) {
  return theme === GENERATED_THEME_LIGHT_ID || theme === GENERATED_THEME_DARK_ID;
}

// 本地点击 tab 会主动设置 activeTab，且可能触发 themeMode 变化；用标志位跳过同步逻辑，避免覆盖本地意图
let localTabIntent = false;
function markLocalTabIntent() {
  localTabIntent = true;
  void nextTick(() => {
    localTabIntent = false;
  });
}

function switchToAutoTab() {
  activeTab.value = "auto";
  markLocalTabIntent();
  emit("setThemeMode", "auto");
}

function switchToPresetTab() {
  activeTab.value = "preset";
  markLocalTabIntent();
  emit("setThemeMode", "manual");
}

function switchToGeneratedTab() {
  activeTab.value = "generated";
  markLocalTabIntent();
  emit("activateGeneratedTheme");
}

watch(
  () => props.themeMode,
  (mode) => {
    if (localTabIntent) {
      localTabIntent = false;
      return;
    }
    // tab 即模式：自动模式激活时高亮自动 tab；外部同步为手动模式时，按当前生效主题对应到预设/自定义
    if (mode === "auto") {
      activeTab.value = "auto";
    } else {
      activeTab.value = isGeneratedTheme(props.currentTheme) ? "generated" : "preset";
    }
  },
  { immediate: true },
);
</script>

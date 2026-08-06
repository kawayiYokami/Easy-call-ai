<template>
  <ConfigTemplate :model-value="templateValues" :groups="templateGroups">
    <template #row-hotkey>
      <div class="flex min-w-0 items-center justify-between gap-4">
        <div class="min-w-0">
          <div class="text-sm">{{ t("config.hotkey.label") }}</div>
          <p class="mt-1 text-xs text-base-content/60">{{ hotkeyCaptureHint }}</p>
        </div>
        <div class="flex min-w-0 shrink-0 items-center gap-2">
          <input :value="config.hotkey" class="input input-bordered input-sm w-40 max-w-full" placeholder="Alt+·" readonly />
          <button
            class="btn btn-sm bg-base-200"
            :class="{ 'btn-primary': hotkeyCapturing }"
            @click="toggleHotkeyCapture"
          >
            {{ hotkeyCapturing ? t("config.hotkey.recording") : t("config.hotkey.recordButton") }}
          </button>
        </div>
      </div>
    </template>

    <template #row-send-mode>
      <div class="flex min-w-0 items-center justify-between gap-4">
        <div class="min-w-0">
          <div class="text-sm">{{ t("config.hotkey.sendMode") }}</div>
          <p class="mt-1 text-xs text-base-content/60">{{ t("config.hotkey.sendModeHint") }}</p>
        </div>
        <select
          class="select select-bordered select-sm w-56 max-w-full"
          :value="sendMode"
          @change="onSendModeChange"
        >
          <option value="enter">{{ t("chat.sendModeEnter") }}</option>
          <option value="ctrl_enter">{{ t("chat.sendModeCtrlEnter") }}</option>
        </select>
      </div>
    </template>

    <template #row-record-hotkey>
      <div class="flex min-w-0 items-center justify-between gap-4">
        <div class="text-sm">{{ t("config.hotkey.recordKey") }}</div>
        <div class="flex min-w-0 shrink-0 items-center gap-2">
          <input :value="config.recordHotkey" class="input input-bordered input-sm w-40 max-w-full" readonly />
          <button
            class="btn btn-sm bg-base-200"
            :class="{ 'btn-primary': recordHotkeyCapturing }"
            @click="toggleRecordHotkeyCapture"
          >
            {{ recordHotkeyCapturing ? t("config.hotkey.recording") : t("config.hotkey.recordButton") }}
          </button>
        </div>
      </div>
    </template>

    <template v-if="isWindowsHost" #row-background-wake>
      <div class="flex min-w-0 items-center justify-between gap-4">
        <div class="text-sm">{{ t("config.hotkey.backgroundWakeTitle") }}</div>
        <button
          type="button"
          class="btn btn-sm shrink-0"
          :class="config.recordBackgroundWakeEnabled ? 'btn-success text-success-content' : 'bg-base-200'"
          :aria-pressed="config.recordBackgroundWakeEnabled ? 'true' : 'false'"
          @click="$emit('update:recordBackgroundWakeEnabled', !config.recordBackgroundWakeEnabled)"
        >
          {{ config.recordBackgroundWakeEnabled ? t("config.hotkey.backgroundWakeOn") : t("config.hotkey.backgroundWakeOff") }}
        </button>
      </div>
    </template>

    <template #row-min-record-seconds>
      <label class="flex min-w-0 items-center justify-between gap-4">
        <span class="text-sm">{{ t("config.hotkey.minRecordSeconds") }}</span>
        <input
          :value="config.minRecordSeconds"
          type="number"
          min="1"
          max="30"
          class="input input-bordered input-sm w-40 max-w-full"
          @input="onMinRecordSecondsInput"
        />
      </label>
    </template>

    <template #row-max-record-seconds>
      <label class="flex min-w-0 items-center justify-between gap-4">
        <span class="text-sm">{{ t("config.hotkey.maxRecordSeconds") }}</span>
        <input
          :value="config.maxRecordSeconds"
          type="number"
          min="1"
          max="600"
          class="input input-bordered input-sm w-40 max-w-full"
          @input="onMaxRecordSecondsInput"
        />
      </label>
    </template>

    <template #row-record-test>
      <div class="flex min-w-0 flex-wrap items-center justify-between gap-3">
        <div class="min-w-0">
          <div class="text-sm">{{ t("config.hotkey.recordTest") }}</div>
          <div class="mt-1 text-xs text-base-content/60">{{ t("config.hotkey.microphonePermissionLabel") }}：{{ microphonePermissionLabel }}</div>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <button
            class="btn btn-sm bg-base-200"
            :disabled="microphonePermissionRequesting"
            @click="$emit('requestMicrophonePermission')"
          >
            <span v-if="microphonePermissionRequesting" class="loading loading-spinner loading-xs"></span>
            <span>{{ t("config.hotkey.requestMicrophonePermission") }}</span>
          </button>
          <button
            class="btn btn-sm bg-base-200"
            :class="{ 'btn-error text-error-content': hotkeyTestRecording }"
            :title="hotkeyTestRecording ? t('config.hotkey.releaseToStop') : t('config.hotkey.holdToRecord')"
            @mousedown.prevent="$emit('startHotkeyRecordTest')"
            @mouseup.prevent="$emit('stopHotkeyRecordTest')"
            @mouseleave.prevent="hotkeyTestRecording && $emit('stopHotkeyRecordTest')"
            @touchstart.prevent="$emit('startHotkeyRecordTest')"
            @touchend.prevent="$emit('stopHotkeyRecordTest')"
          >
            {{ hotkeyTestRecording ? t("chat.recording", { seconds: Math.max(1, Math.round(hotkeyTestRecordingMs / 1000)) }) : t("config.hotkey.holdRecordButton") }}
          </button>
          <button
            class="btn btn-sm bg-base-200"
            :disabled="!hotkeyTestAudioReady"
            @click="$emit('playHotkeyRecordTest')"
          >
            {{ t("config.hotkey.playRecord") }}
          </button>
        </div>
      </div>
    </template>

    <template #row-background-voice-keywords>
      <label class="grid min-w-0 gap-2">
        <div>
          <div class="text-sm">{{ t("config.chatSettings.backgroundVoiceScreenshotKeywords") }}</div>
          <div class="mt-1 text-xs text-base-content/60">{{ t("config.chatSettings.backgroundVoiceScreenshotKeywordsHint") }}</div>
        </div>
        <div class="flex items-center gap-2">
          <input
            v-model="backgroundVoiceScreenshotKeywordsDraft"
            type="text"
            class="input input-bordered input-sm min-w-0 flex-1"
            :placeholder="t('config.chatSettings.backgroundVoiceScreenshotKeywordsPlaceholder')"
          />
          <button class="btn btn-sm btn-primary shrink-0" :disabled="!backgroundVoiceScreenshotDirty" @click="saveBackgroundVoiceScreenshotSettings">
            {{ t("common.save") }}
          </button>
        </div>
      </label>
    </template>

    <template #row-background-voice-mode>
      <div class="grid min-w-0 gap-2">
        <div>
          <div class="text-sm">{{ t("config.chatSettings.backgroundVoiceScreenshotMode") }}</div>
          <div class="mt-1 text-xs text-base-content/60">{{ t("config.chatSettings.backgroundVoiceScreenshotModeHint") }}</div>
        </div>
        <SegmentedControl
          :model-value="backgroundVoiceScreenshotMode"
          :options="backgroundVoiceScreenshotModeOptions"
          size="sm"
          @change="onBackgroundVoiceScreenshotModeChange"
        />
      </div>
    </template>

    <template #row-builtin-tab>
      <div class="flex min-w-0 items-center justify-between gap-4">
        <div class="min-w-0">
          <div class="text-sm">{{ t("config.hotkey.builtinTabInstruction") }}</div>
          <p class="mt-1 text-xs text-base-content/60">{{ t("config.hotkey.builtinTabInstructionHint") }}</p>
        </div>
        <input value="Tab" class="input input-bordered input-sm w-40 max-w-full shrink-0 text-center font-mono" disabled />
      </div>
    </template>
    <template #row-builtin-esc>
      <div class="flex min-w-0 items-center justify-between gap-4">
        <div class="min-w-0">
          <div class="text-sm">{{ t("config.hotkey.builtinEscStopReplying") }}</div>
          <p class="mt-1 text-xs text-base-content/60">{{ t("config.hotkey.builtinEscStopReplyingHint") }}</p>
        </div>
        <input value="Esc" class="input input-bordered input-sm w-40 max-w-full shrink-0 text-center font-mono" disabled />
      </div>
    </template>
    <template #row-builtin-shift-tab>
      <div class="flex min-w-0 items-center justify-between gap-4">
        <div class="min-w-0">
          <div class="text-sm">{{ t("config.hotkey.builtinShiftTabPlanMode") }}</div>
          <p class="mt-1 text-xs text-base-content/60">{{ t("config.hotkey.builtinShiftTabPlanModeHint") }}</p>
        </div>
        <input value="Shift + Tab" class="input input-bordered input-sm w-40 max-w-full shrink-0 text-center font-mono" disabled />
      </div>
    </template>
    <template #row-builtin-alt-z>
      <div class="flex min-w-0 items-center justify-between gap-4">
        <div class="min-w-0">
          <div class="text-sm">{{ t("config.hotkey.builtinAltZLineWrap") }}</div>
          <p class="mt-1 text-xs text-base-content/60">{{ t("config.hotkey.builtinAltZLineWrapHint") }}</p>
        </div>
        <input value="Alt + Z" class="input input-bordered input-sm w-40 max-w-full shrink-0 text-center font-mono" disabled />
      </div>
    </template>
    <template #row-builtin-wheel>
      <div class="flex min-w-0 items-center justify-between gap-4">
        <div class="min-w-0">
          <div class="text-sm">{{ t("config.hotkey.builtinShiftWheelConversationSwitch") }}</div>
          <p class="mt-1 text-xs text-base-content/60">{{ t("config.hotkey.builtinShiftWheelConversationSwitchHint") }}</p>
        </div>
        <input :value="`Shift + ${t('config.hotkey.builtinWheelKey')}`" class="input input-bordered input-sm w-40 max-w-full shrink-0 text-center font-mono" disabled />
      </div>
    </template>
  </ConfigTemplate>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import SegmentedControl from "../../components/SegmentedControl.vue";
import ConfigTemplate from "../../components/ConfigTemplate.vue";
import type { ConfigTemplateGroup } from "../../components/config-template";
import type { AppConfig, ChatSettingsPatch } from "../../../../types/app";

const props = defineProps<{
  config: AppConfig;
  hotkeyTestRecording: boolean;
  hotkeyTestRecordingMs: number;
  hotkeyTestAudioReady: boolean;
  microphonePermissionState: "granted" | "denied" | "prompt" | "unsupported" | "unknown";
  microphonePermissionRequesting: boolean;
  backgroundVoiceScreenshotKeywords: string;
  backgroundVoiceScreenshotMode: "desktop" | "focused_window";
}>();

const emit = defineEmits<{
  (e: "startHotkeyRecordTest"): void;
  (e: "stopHotkeyRecordTest"): void;
  (e: "playHotkeyRecordTest"): void;
  (e: "requestMicrophonePermission"): void;
  (e: "captureHotkey", value: string): void;
  (e: "update:recordHotkey", value: string): void;
  (e: "update:recordBackgroundWakeEnabled", value: boolean): void;
  (e: "update:minRecordSeconds", value: number): void;
  (e: "update:maxRecordSeconds", value: number): void;
  (e: "update:backgroundVoiceScreenshotKeywords", value: string): void;
  (e: "update:backgroundVoiceScreenshotMode", value: "desktop" | "focused_window"): void;
  (e: "patchChatSettings", value: ChatSettingsPatch): void;
}>();

const { t } = useI18n();
// 录音后台唤醒仅 Windows 支持，其他平台直接隐藏开关
const isWindowsHost = typeof navigator !== "undefined" && /windows/i.test(String(navigator.userAgent || ""));
const templateValues = {};
const templateGroups = computed<ConfigTemplateGroup[]>(() => [
  {
    key: "hotkey",
    title: t("config.hotkey.label"),
    rows: [
      { key: "hotkey", items: [] },
      { key: "send-mode", items: [] },
      { key: "builtin-tab", items: [] },
      { key: "builtin-esc", items: [] },
      { key: "builtin-shift-tab", items: [] },
      { key: "builtin-alt-z", items: [] },
      { key: "builtin-wheel", items: [] },
    ],
  },
  {
    key: "voice",
    title: t("config.hotkey.voiceTitle"),
    rows: [
      { key: "record-hotkey", items: [] },
      { key: "background-wake", items: [] },
      { key: "min-record-seconds", items: [] },
      { key: "max-record-seconds", items: [] },
      { key: "record-test", items: [] },
      { key: "background-voice-keywords", items: [] },
      { key: "background-voice-mode", items: [] },
    ],
  },
]);
const backgroundVoiceScreenshotModeOptions = computed(() => [
  { value: "desktop" as const, label: t("config.chatSettings.backgroundVoiceScreenshotModeDesktop") },
  { value: "focused_window" as const, label: t("config.chatSettings.backgroundVoiceScreenshotModeFocusedWindow") },
]);

const microphonePermissionLabel = computed(() => {
  if (props.microphonePermissionState === "granted") return t("config.hotkey.microphonePermissionGranted");
  if (props.microphonePermissionState === "denied") return t("config.hotkey.microphonePermissionDenied");
  if (props.microphonePermissionState === "prompt") return t("config.hotkey.microphonePermissionPrompt");
  if (props.microphonePermissionState === "unsupported") return t("config.hotkey.microphonePermissionUnsupported");
  return t("config.hotkey.microphonePermissionUnknown");
});

const microphonePermissionBadgeClass = computed(() => {
  if (props.microphonePermissionState === "granted") return "badge-success";
  if (props.microphonePermissionState === "denied") return "badge-error";
  if (props.microphonePermissionState === "prompt") return "badge-warning";
  return "badge-ghost";
});

const SEND_MODE_STORAGE_KEY = "easy_call.send_mode.v1";
const sendMode = ref<"enter" | "ctrl_enter">("enter");

onMounted(() => {
  try {
    const raw = window.localStorage.getItem(SEND_MODE_STORAGE_KEY);
    if (raw === "ctrl_enter") sendMode.value = "ctrl_enter";
  } catch {
    // ignore storage failures
  }
});

function onSendModeChange(event: Event) {
  const next = String((event.target as HTMLSelectElement | null)?.value || "");
  if (next !== "enter" && next !== "ctrl_enter") return;
  sendMode.value = next;
  try {
    window.localStorage.setItem(SEND_MODE_STORAGE_KEY, next);
  } catch {
    // ignore persistence failures
  }
}

const hotkeyCapturing = ref(false);
const hotkeyCaptureHint = ref(t("config.hotkey.captureDefaultHint"));
let hotkeyCaptureHandler: ((event: KeyboardEvent) => void) | null = null;
const recordHotkeyCapturing = ref(false);
let recordHotkeyCaptureHandler: ((event: KeyboardEvent) => void) | null = null;
const backgroundVoiceScreenshotKeywordsDraft = ref(String(props.backgroundVoiceScreenshotKeywords || ""));

watch(
  () => props.backgroundVoiceScreenshotKeywords,
  (value) => {
    backgroundVoiceScreenshotKeywordsDraft.value = String(value || "");
  },
);

const backgroundVoiceScreenshotDirty = computed(
  () => backgroundVoiceScreenshotKeywordsDraft.value !== String(props.backgroundVoiceScreenshotKeywords || ""),
);

function onMinRecordSecondsInput(event: Event) {
  const raw = Number((event.target as HTMLInputElement).value);
  emit("update:minRecordSeconds", raw);
}

function onMaxRecordSecondsInput(event: Event) {
  const raw = Number((event.target as HTMLInputElement).value);
  emit("update:maxRecordSeconds", raw);
}

function saveBackgroundVoiceScreenshotSettings() {
  const keywords = backgroundVoiceScreenshotKeywordsDraft.value.replace(/，/g, ",");
  backgroundVoiceScreenshotKeywordsDraft.value = keywords;
  emit("update:backgroundVoiceScreenshotKeywords", keywords);
  emit("patchChatSettings", {
    backgroundVoiceScreenshotKeywords: keywords,
  });
}

function onBackgroundVoiceScreenshotModeChange(value: "desktop" | "focused_window") {
  emit("update:backgroundVoiceScreenshotMode", value);
  emit("patchChatSettings", {
    backgroundVoiceScreenshotMode: value,
  });
}

function isModifierKey(code: string): boolean {
  return code === "AltLeft"
    || code === "AltRight"
    || code === "ControlLeft"
    || code === "ControlRight"
    || code === "ShiftLeft"
    || code === "ShiftRight"
    || code === "MetaLeft"
    || code === "MetaRight";
}

function mainKeyFromEvent(event: KeyboardEvent): string {
  const code = event.code || "";
  if (code === "Backquote") return "·";
  if (code.startsWith("Key") && code.length === 4) return code.slice(3).toUpperCase();
  if (code.startsWith("Digit") && code.length === 6) return code.slice(5);
  if (/^F\d{1,2}$/.test(code)) return code;
  if (code === "Minus") return "-";
  if (code === "Equal") return "=";
  if (code === "BracketLeft") return "[";
  if (code === "BracketRight") return "]";
  if (code === "Backslash") return "\\";
  if (code === "Semicolon") return ";";
  if (code === "Quote") return "'";
  if (code === "Comma") return ",";
  if (code === "Period") return ".";
  if (code === "Slash") return "/";
  if (code === "Space") return "Space";
  const key = event.key || "";
  if (key.length === 1) return key.toUpperCase();
  return key;
}

function stopHotkeyCapture() {
  hotkeyCapturing.value = false;
  if (hotkeyCaptureHandler) {
    window.removeEventListener("keydown", hotkeyCaptureHandler, true);
    hotkeyCaptureHandler = null;
  }
}

function stopRecordHotkeyCapture() {
  recordHotkeyCapturing.value = false;
  if (recordHotkeyCaptureHandler) {
    window.removeEventListener("keydown", recordHotkeyCaptureHandler, true);
    recordHotkeyCaptureHandler = null;
  }
}

function startHotkeyCapture() {
  if (hotkeyCapturing.value) return;
  hotkeyCapturing.value = true;
  hotkeyCaptureHint.value = t("config.hotkey.captureListeningHint");
  hotkeyCaptureHandler = (event: KeyboardEvent) => {
    event.preventDefault();
    event.stopPropagation();

    if (event.key === "Escape") {
      hotkeyCaptureHint.value = t("config.hotkey.captureCancelledHint");
      stopHotkeyCapture();
      return;
    }
    if (event.metaKey) {
      hotkeyCaptureHint.value = t("config.hotkey.captureMetaNotSupportedHint");
      return;
    }

    const modifiers: string[] = [];
    if (event.ctrlKey) modifiers.push("Ctrl");
    if (event.altKey) modifiers.push("Alt");
    if (event.shiftKey) modifiers.push("Shift");

    if (isModifierKey(event.code)) {
      hotkeyCaptureHint.value = t("config.hotkey.captureNeedMainKeyHint");
      return;
    }
    if (modifiers.length === 0) {
      hotkeyCaptureHint.value = t("config.hotkey.captureNeedModifierHint");
      return;
    }

    const main = mainKeyFromEvent(event).trim();
    if (!main) {
      hotkeyCaptureHint.value = t("config.hotkey.captureUnrecognizedHint");
      return;
    }
    const combo = `${modifiers.join("+")}+${main}`;
    emit("captureHotkey", combo);
    hotkeyCaptureHint.value = t("config.hotkey.captureCapturedHint", { combo });
    stopHotkeyCapture();
  };
  window.addEventListener("keydown", hotkeyCaptureHandler, true);
}

function startRecordHotkeyCapture() {
  if (recordHotkeyCapturing.value) return;
  recordHotkeyCapturing.value = true;
  recordHotkeyCaptureHandler = (event: KeyboardEvent) => {
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "Escape") {
      emit("update:recordHotkey", "");
      stopRecordHotkeyCapture();
      return;
    }
    if (event.metaKey) return;

    const modifiers: string[] = [];
    if (event.ctrlKey) modifiers.push("Ctrl");
    if (event.altKey) modifiers.push("Alt");
    if (event.shiftKey) modifiers.push("Shift");

    if (isModifierKey(event.code)) {
      const modifierOnly = modifiers[0];
      if (modifiers.length === 1 && modifierOnly) {
        emit("update:recordHotkey", modifierOnly);
        stopRecordHotkeyCapture();
      }
      return;
    }

    const main = mainKeyFromEvent(event).trim();
    if (!main) return;
    emit("update:recordHotkey", modifiers.length > 0 ? `${modifiers.join("+")}+${main}` : main);
    stopRecordHotkeyCapture();
  };
  window.addEventListener("keydown", recordHotkeyCaptureHandler, true);
}

function toggleHotkeyCapture() {
  if (hotkeyCapturing.value) {
    hotkeyCaptureHint.value = t("config.hotkey.captureCancelledHint");
    stopHotkeyCapture();
    return;
  }
  startHotkeyCapture();
}

function toggleRecordHotkeyCapture() {
  if (recordHotkeyCapturing.value) {
    stopRecordHotkeyCapture();
    return;
  }
  startRecordHotkeyCapture();
}

onBeforeUnmount(() => {
  stopHotkeyCapture();
  stopRecordHotkeyCapture();
});
</script>

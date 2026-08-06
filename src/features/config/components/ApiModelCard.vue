<template>
  <div class="card border border-base-300 bg-base-200/50 transition">
    <div class="card-body gap-3 p-4">
      <div class="flex items-start justify-between gap-2">
        <div class="min-w-0 flex-1">
          <template v-if="editingTitle">
            <input
              ref="titleInputRef"
              v-model="titleDraft"
              class="input input-bordered input-sm mb-1 w-full"
              :placeholder="t('config.api.displayNamePlaceholder')"
              @blur="commitTitle"
              @keydown.enter.prevent="commitTitle"
              @keydown.esc.stop.prevent="cancelTitle"
            />
          </template>
          <template v-else>
            <button
              class="flex min-w-0 w-full items-center gap-1.5 text-left"
              type="button"
              :title="t('config.api.editDisplayName')"
              @click="startEditTitle"
            >
              <div class="card-title text-base mb-0 truncate">{{ displayTitle }}</div>
              <Pencil class="h-3.5 w-3.5 shrink-0 opacity-50" />
            </button>
            <span v-if="hint" class="text-xs font-normal opacity-60">{{ hint }}</span>
          </template>
        </div>
        <button
          v-if="showDelete"
          class="btn btn-sm btn-square btn-ghost"
          type="button"
          :class="deleteDisabled ? 'text-base-content/30' : 'text-error'"
          :disabled="deleteDisabled"
          @click="emit('remove')"
        >
          <Trash2 class="h-3.5 w-3.5" />
        </button>
      </div>
      <div class="grid gap-3">
        <label class="flex flex-col gap-1">
          <span class="text-sm font-medium">{{ t("config.api.model") }}</span>
          <div class="join">
            <input
              v-model="card.model"
              class="input input-bordered input-sm join-item flex-1"
              placeholder="model"
              @focus="emit('select')"
              @blur="emit('sync-metadata')"
              @keydown.enter.prevent="emit('sync-metadata')"
            />
            <button
              class="btn btn-sm join-item bg-base-300"
              type="button"
              :disabled="filteredModelOptions.length === 0"
              @click="togglePicker"
            >
              <ChevronDown class="h-3.5 w-3.5" />
            </button>
          </div>
          <div v-if="protocolHint" class="mt-1 text-xs opacity-70">
            {{ t("config.api.matchedProtocol", { protocol: protocolHint }) }}
          </div>
          <div v-if="warningText" class="alert alert-warning mt-2 py-2 text-xs">
            <AlertTriangle class="h-4 w-4 shrink-0" />
            <span>{{ warningText }}</span>
          </div>
        </label>
        <div v-if="pickerOpen" class="rounded-box border border-base-300 bg-base-200/50 p-3">
          <input
            v-model="modelSearch"
            class="input input-bordered input-sm mb-2 w-full"
            :placeholder="t('config.api.searchModel')"
            @keydown.esc.stop.prevent="closePicker"
          />
          <div class="max-h-48 overflow-auto">
            <button
              v-for="option in filteredModels"
              :key="option"
              class="btn btn-ghost btn-sm mb-1 mr-1"
              type="button"
              @click="selectOption(option)"
            >
              {{ option }}
            </button>
            <div v-if="filteredModels.length === 0" class="px-2 py-3 text-sm opacity-50">{{
              t("config.api.noModelFound") }}</div>
          </div>
        </div>
      </div>

      <div v-if="showCapabilityToggles" class="flex flex-wrap gap-3">
        <label
          class="flex min-w-40 flex-1 items-center justify-between rounded-box border border-base-300 bg-base-300 px-3 py-2"
        >
          <span class="text-sm">{{ t("config.api.capImage") }}</span>
          <input v-model="card.enableImage" type="checkbox" class="checkbox checkbox-sm" />
        </label>
        <label
          class="flex min-w-40 flex-1 items-center justify-between rounded-box border border-base-300 bg-base-300 px-3 py-2"
        >
          <span class="text-sm">{{ t("config.api.capAudio") }}</span>
          <input v-model="card.enableAudio" type="checkbox" class="checkbox checkbox-sm" />
        </label>
        <label
          class="flex min-w-40 flex-1 items-center justify-between rounded-box border border-base-300 bg-base-300 px-3 py-2"
        >
          <span class="text-sm">{{ t("config.api.capVideo") }}</span>
          <input v-model="card.enableVideo" type="checkbox" class="checkbox checkbox-sm" />
        </label>
        <label
          class="flex min-w-40 flex-1 items-center justify-between rounded-box border border-base-300 bg-base-300 px-3 py-2"
        >
          <span class="text-sm">{{ t("config.api.capTools") }}</span>
          <input v-model="card.enableTools" type="checkbox" class="checkbox checkbox-sm" />
        </label>
        <label
          class="flex min-w-40 flex-1 items-center justify-between rounded-box border border-base-300 bg-base-300 px-3 py-2"
        >
          <span class="text-sm">{{ t("config.api.temperature") }}</span>
          <input v-model="card.customTemperatureEnabled" type="checkbox" class="checkbox checkbox-sm" />
        </label>
        <label
          class="flex min-w-40 flex-1 items-center justify-between rounded-box border border-base-300 bg-base-300 px-3 py-2"
        >
          <span class="text-sm">{{ t("config.api.maxOutputTokens") }}</span>
          <input
            v-model="card.customMaxOutputTokensEnabled"
            type="checkbox"
            class="checkbox checkbox-sm"
            @change="emit('toggle-max-output')"
          />
        </label>
      </div>

      <div v-if="showContextWindow" class="grid gap-3">
        <label class="flex flex-col gap-1">
          <span class="text-sm font-medium">{{ t("config.api.contextWindow") }}</span>
          <div class="flex items-center gap-2">
            <input
              :value="card.contextWindowTokens"
              @input="card.contextWindowTokens = Number(($event.target as HTMLInputElement).value)"
              type="range"
              :min="SLIDER_CONTEXT_MIN"
              :max="contextWindowMax"
              step="1000"
              class="range range-sm flex-1"
            />
            <div class="relative w-28">
              <input
                :value="Math.round(Number(card.contextWindowTokens || 0) / 1000)"
                @input="card.contextWindowTokens = Number(($event.target as HTMLInputElement).value || 0) * 1000"
                @blur="clampManualContextWindowValue"
                type="number"
                :min="Math.round(SLIDER_CONTEXT_MIN / 1000)"
                :max="2000"
                step="1"
                class="input input-bordered input-sm w-full pr-7 text-right font-mono"
              />
              <span class="pointer-events-none absolute inset-y-0 right-2 flex items-center text-xs opacity-70">K</span>
            </div>
          </div>
        </label>

        <button
          v-if="documentationUrl"
          type="button"
          class="btn btn-outline btn-sm justify-start"
          @click="emit('open-documentation')"
        >
          {{ t("config.api.viewModelDocumentation") }}
        </button>

        <div v-if="showReasoning" class="flex flex-col gap-2">
          <div class="flex items-center gap-2">
            <span class="text-sm font-medium">{{ t("config.api.reasoningEffort") }}</span>
            <span
              v-if="reasoningStatus === 'unknown'"
              class="text-xs opacity-60"
              :title="t('config.api.reasoningCapabilityUnknown')"
            >
              {{ t("config.api.reasoningCapabilityUnknown") }}
            </span>
            <span
              v-else-if="reasoningStatus === 'unsupported'"
              class="text-xs text-warning"
              :title="t('config.api.reasoningCapabilityUnsupported')"
            >
              {{ t("config.api.reasoningCapabilityUnsupported") }}
            </span>
          </div>
          <div class="flex flex-wrap gap-x-4 gap-y-2">
            <label
              v-for="item in reasoningItems"
              :key="item.value"
              class="flex items-center gap-2 text-sm"
              :class="item.disabled ? 'cursor-not-allowed opacity-50' : ''"
              :title="item.disabled ? t('config.api.reasoningEffortUnsupported') : undefined"
            >
              <input
                type="checkbox"
                class="checkbox checkbox-sm"
                :checked="reasoningCheckedValues.includes(item.value)"
                :disabled="item.disabled"
                @change="emit('reasoning-change', { value: item.value, checked: ($event.target as HTMLInputElement).checked })"
              />
              <span>{{ item.label }}</span>
            </label>
          </div>
        </div>

        <label v-if="showTemperature && card.customTemperatureEnabled" class="flex flex-col gap-1">
          <span class="text-sm font-medium">{{ t("config.api.temperature") }}</span>
          <div class="flex items-center gap-2">
            <input
              :value="card.temperature"
              @input="card.temperature = Number(($event.target as HTMLInputElement).value)"
              type="range"
              min="0"
              max="2"
              step="0.1"
              class="range range-sm flex-1"
            />
            <span class="text-xs font-mono w-8 text-right">{{ card.temperature.toFixed(1) }}</span>
          </div>
        </label>

        <label v-if="showMaxOutputTokens && card.customMaxOutputTokensEnabled" class="flex flex-col gap-1">
          <span class="text-sm font-medium">{{ t("config.api.maxOutputTokens") }}</span>
          <div class="flex items-center gap-2">
            <input
              :value="card.maxOutputTokens"
              @input="card.maxOutputTokens = Number(($event.target as HTMLInputElement).value)"
              type="range"
              min="8192"
              max="128000"
              step="256"
              class="range range-sm flex-1"
            />
            <input
              :value="Math.round(Number(card.maxOutputTokens ?? 0))"
              @input="card.maxOutputTokens = Number(($event.target as HTMLInputElement).value || 0)"
              type="number"
              step="1"
              class="input input-bordered input-sm w-28 text-right font-mono"
            />
          </div>
        </label>
      </div>

      <div
        v-if="connectionResult"
        class="rounded-box border px-3 py-2 text-xs"
        :class="connectionResult.success ? 'border-success/30 text-success' : 'border-error/30 text-error'"
      >
        {{ connectionResult.success
          ? t('config.api.testConnectionSuccess', { latency: connectionResult.latencyMs })
          : t('config.api.testConnectionFailed', { error: connectionResult.error }) }}
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref } from "vue";
import { useI18n } from "vue-i18n";
import { AlertTriangle, ChevronDown, Pencil, Trash2 } from "@lucide/vue";
import type { ApiModelConfigItem } from "../../../types/app";

const SLIDER_CONTEXT_MIN = 16_000;

export interface ReasoningEffortItem {
  value: string;
  label: string;
  disabled?: boolean;
}

export interface ModelConnectionResult {
  success: boolean;
  latencyMs?: number;
  error?: string;
}

const props = withDefaults(defineProps<{
  card: ApiModelConfigItem;
  title?: string;
  hint?: string;
  modelOptions?: string[];
  showDelete?: boolean;
  deleteDisabled?: boolean;
  showCapabilityToggles?: boolean;
  showContextWindow?: boolean;
  showReasoning?: boolean;
  showTemperature?: boolean;
  showMaxOutputTokens?: boolean;
  reasoningItems?: ReasoningEffortItem[];
  reasoningCheckedValues?: string[];
  reasoningStatus?: "known" | "unknown" | "unsupported";
  protocolHint?: string;
  warningText?: string;
  documentationUrl?: string;
  connectionResult?: ModelConnectionResult | null;
  contextWindowMax?: number;
}>(), {
  title: "",
  hint: "",
  modelOptions: () => [],
  showDelete: true,
  deleteDisabled: false,
  showCapabilityToggles: true,
  showContextWindow: true,
  showReasoning: true,
  showTemperature: true,
  showMaxOutputTokens: true,
  reasoningItems: () => [],
  reasoningCheckedValues: () => [],
  reasoningStatus: "unknown",
  protocolHint: "",
  warningText: "",
  documentationUrl: "",
  connectionResult: null,
  contextWindowMax: 2_000_000,
});

const emit = defineEmits<{
  (event: "select"): void;
  (event: "remove"): void;
  (event: "sync-metadata"): void;
  (event: "select-option", option: string): void;
  (event: "toggle-max-output"): void;
  (event: "reasoning-change", payload: { value: string; checked: boolean }): void;
  (event: "open-documentation"): void;
}>();

const { t } = useI18n();

const pickerOpen = ref(false);
const modelSearch = ref("");

const displayTitle = computed(() => {
  const explicit = String(props.title || "").trim();
  if (explicit) return explicit;
  const displayName = String(props.card.displayName || "").trim();
  if (displayName) return displayName;
  return String(props.card.model || "").trim() || t("config.api.unnamedModel");
});

const editingTitle = ref(false);
const titleDraft = ref("");
const titleInputRef = ref<HTMLInputElement | null>(null);

function startEditTitle() {
  titleDraft.value = props.card.displayName ?? "";
  editingTitle.value = true;
  void nextTick(() => titleInputRef.value?.focus());
}

function commitTitle() {
  if (!editingTitle.value) return;
  props.card.displayName = titleDraft.value.trim();
  editingTitle.value = false;
  emit("sync-metadata");
}

function cancelTitle() {
  editingTitle.value = false;
}

const filteredModelOptions = computed(() => {
  const options = Array.from(new Set([
    ...(props.modelOptions || []),
    String(props.card.model || "").trim(),
  ].map((item) => String(item || "").trim()).filter(Boolean)));
  return options;
});

const filteredModels = computed(() => {
  const search = modelSearch.value.trim().toLowerCase();
  if (!search) return filteredModelOptions.value;
  return filteredModelOptions.value.filter((item) => item.toLowerCase().includes(search));
});

function togglePicker() {
  pickerOpen.value = !pickerOpen.value;
  modelSearch.value = "";
}

function closePicker() {
  pickerOpen.value = false;
  modelSearch.value = "";
}

function selectOption(option: string) {
  props.card.model = option;
  closePicker();
  emit("select");
  emit("select-option", option);
}

function clampManualContextWindowValue() {
  const nextContext = Math.round(Number(props.card.contextWindowTokens ?? 256_000));
  const clampedContext = Math.max(SLIDER_CONTEXT_MIN, Math.min(2_000_000, nextContext));
  if (!Number.isFinite(nextContext)) {
    props.card.contextWindowTokens = 256_000;
    return;
  }
  if (nextContext !== clampedContext) {
    props.card.contextWindowTokens = clampedContext;
  }
}
</script>

<template>
  <ConfigTemplate :model-value="templateValues" :groups="templateGroups">
    <template #row-vision-api>
      <label class="grid min-w-0 gap-2">
        <div>
          <div class="text-sm">{{ t("config.chatSettings.visionApi") }}</div>
          <div class="mt-1 text-xs text-base-content/60">{{ t("config.chatSettings.visionApiHint") }}</div>
        </div>
        <ApiConfigTreeSelect
          :model-value="config.visionApiConfigId ?? ''"
          :api-configs="imageCapableApiConfigs"
          :placeholder="t('config.chatSettings.noVision')"
          @update:model-value="onVisionSelect"
        />
      </label>
    </template>

    <template #row-tool-review-api>
      <label class="grid min-w-0 gap-2">
        <div>
          <div class="text-sm">{{ t("config.chatSettings.toolReviewApi") }}</div>
          <div class="mt-1 text-xs text-base-content/60">{{ t("config.chatSettings.toolReviewApiHint") }}</div>
        </div>
        <ApiConfigTreeSelect
          :model-value="config.toolReviewApiConfigId ?? ''"
          :api-configs="textCapableApiConfigs"
          @update:model-value="onToolReviewSelect"
        />
      </label>
    </template>

    <template #row-expert-chat-model>
      <label class="grid min-w-0 gap-2">
        <div>
          <div class="text-sm">{{ t("config.chatSettings.expertChatModelTitle") }}</div>
          <div class="mt-1 text-xs text-base-content/60">{{ t("config.chatSettings.expertChatModelHint") }}</div>
        </div>
        <ApiConfigTreeSelect
          :model-value="config.assistantDepartmentApiConfigId || ''"
          :api-configs="textCapableApiConfigs"
          @update:model-value="onExpertSelect"
        />
      </label>
    </template>

    <template #row-image-generation-model>
      <label class="grid min-w-0 gap-2">
        <div>
          <div class="text-sm">{{ t("config.imageGeneration.defaultModel") }}</div>
          <div class="mt-1 text-xs text-base-content/60">{{ t("config.imageGeneration.defaultModelHint") }}</div>
        </div>
        <select :value="config.imageGenerationModelId || ''" class="select select-bordered select-sm w-full" @change="onImageGenerationSelectChange">
          <option value="">{{ t("config.imageGeneration.noDefaultModel") }}</option>
          <option v-for="option in imageGenerationModelOptions" :key="option.id" :value="option.id">
            {{ option.label }}
          </option>
        </select>
      </label>
    </template>

    <template #row-stt-api>
      <label class="grid min-w-0 gap-2">
        <div>
          <div class="text-sm">{{ t("config.chatSettings.sttTitle") }}</div>
          <div class="mt-1 text-xs text-base-content/60">{{ t("config.chatSettings.sttHint") }}</div>
        </div>
        <select :value="config.sttApiConfigId ?? ''" class="select select-bordered select-sm w-full" @change="onSttSelectChange">
          <option value="">{{ t("config.chatSettings.sttLocalWebSpeech") }}</option>
          <option v-for="a in sttCapableApiConfigs" :key="a.id" :value="a.id">{{ a.name }}</option>
        </select>
      </label>
    </template>

    <template #row-stt-auto-send>
      <div class="flex min-w-0 items-center justify-between gap-4" :class="{ 'opacity-50': !config.sttApiConfigId }">
        <div class="min-w-0">
          <div class="text-sm">{{ t("config.chatSettings.sttAutoSend") }}</div>
          <p class="mt-1 text-xs text-base-content/60">{{ t("config.chatSettings.sttHint") }}</p>
        </div>
        <input
          :checked="!!config.sttAutoSend"
          type="checkbox"
          class="toggle toggle-sm toggle-primary shrink-0"
          :disabled="!config.sttApiConfigId"
          @change="onSttAutoSendChange"
        />
      </div>
    </template>

    <template #row-response-style>
      <div class="grid min-w-0 gap-2">
        <SegmentedControl
          :model-value="responseStyleId"
          :options="responseStyleSegmentOptions"
          size="sm"
          @change="onResponseStyleChange"
        />
      </div>
    </template>

    <template #row-exec-terminal>
      <div v-if="isWindowsHost" class="grid gap-2">
        <div v-if="t('config.chatSettings.execTerminalHint')" class="text-xs opacity-70">
          {{ t("config.chatSettings.execTerminalHint") }}
        </div>
        <select
          class="select select-bordered select-sm w-full"
          :value="terminalShellKindValue"
          :disabled="terminalShellOptionsLoading || savingConfig"
          @change="onTerminalShellKindChange"
        >
          <option v-for="item in terminalShellOptions" :key="item.kind" :value="item.kind">
            {{ item.label }}
          </option>
        </select>
        <div v-if="showGitInstallHintInWorkspace" class="text-xs bg-warning/10 text-base-content rounded px-2 py-1 flex items-center gap-2">
          <span>{{ t("config.chatSettings.gitRequiredHint") }}</span>
          <button class="btn btn-sm bg-base-100" @click="openGitDownloadLink">
            {{ t("config.chatSettings.installGit") }}
          </button>
        </div>
      </div>
    </template>

    <template #row-instruction-presets>
      <div class="grid min-w-0 gap-3">
        <div v-if="instructionPresetsDraft.length === 0" class="text-sm opacity-60">
          {{ t("config.chatSettings.noInstructionPresets") }}
        </div>
        <div v-else class="grid gap-2">
          <div v-for="item in instructionPresetsDraft" :key="item.id" class="flex items-center gap-2">
            <input
              v-model="item.prompt"
              type="text"
              class="input input-bordered input-sm min-w-0 flex-1"
              :placeholder="t('config.chatSettings.instructionPresetPlaceholder')"
            />
            <button class="btn btn-sm btn-ghost btn-square shrink-0" @click="removeInstructionPreset(item.id)">
              <Trash2 class="h-4 w-4" />
            </button>
          </div>
        </div>
        <div class="flex items-center justify-between">
          <button class="btn btn-sm btn-ghost shrink-0" @click="addInstructionPreset">
            <Plus class="h-4 w-4" />
            <span>{{ t("config.chatSettings.addInstructionPreset") }}</span>
          </button>
          <button class="btn btn-sm btn-primary" :disabled="!instructionPresetsDirty" @click="saveInstructionPresets">
            {{ t("config.chatSettings.saveInstructionPresets") }}
          </button>
        </div>
      </div>
    </template>

  </ConfigTemplate>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { Plus, Trash2 } from "@lucide/vue";
import SegmentedControl from "../../components/SegmentedControl.vue";
import ConfigTemplate from "../../components/ConfigTemplate.vue";
import type { ConfigTemplateGroup } from "../../components/config-template";
import ApiConfigTreeSelect from "../../components/ApiConfigTreeSelect.vue";
import type { AppConfig, ApiConfigItem, ChatSettingsPatch, ConversationApiSettingsPatch, PromptCommandPreset, ResponseStyleOption, ToolLoadStatus } from "../../../../types/app";
import { invokeTauri, openTransportExternalUrl } from "../../../../services/tauri-api";
import { deriveImageGenerationModelOptions } from "../../utils/image-generation-config";

type TerminalShellCandidate = {
  kind: string;
  label: string;
  available: boolean;
  path?: string;
};

type TerminalShellCandidatesResult = {
  preferredKind?: string;
  currentKind?: string;
  currentPath?: string;
  options?: TerminalShellCandidate[];
};

const props = defineProps<{
  config: AppConfig;
  textCapableApiConfigs: ApiConfigItem[];
  imageCapableApiConfigs: ApiConfigItem[];
  sttCapableApiConfigs: ApiConfigItem[];
  responseStyleOptions: ResponseStyleOption[];
  responseStyleId: string;
  pdfReadMode: "text" | "image";
  instructionPresets: PromptCommandPreset[];
  toolStatuses: ToolLoadStatus[];
  savingConfig: boolean;
}>();

const { t } = useI18n();
const templateValues = {};
const terminalShellOptionsLoading = ref(false);
const terminalShellOptions = ref<TerminalShellCandidate[]>([]);
const GIT_DOWNLOAD_URL = "https://git-scm.com/downloads";
const isWindowsHost = typeof navigator !== "undefined" && /windows/i.test(String(navigator.userAgent || ""));
const terminalShellKindValue = computed(() => String(props.config.terminalShellKind || "auto"));

async function loadTerminalShellCandidates() {
  if (!isWindowsHost) return;
  terminalShellOptionsLoading.value = true;
  try {
    const payload = await invokeTauri<TerminalShellCandidatesResult>("list_terminal_shell_candidates");
    const options = Array.isArray(payload.options) ? payload.options : [];
    terminalShellOptions.value =
      options.length > 0
        ? options
        : [{ kind: "auto", label: "Auto", available: true }];
    const preferred = String(payload.preferredKind || "").trim();
    if (preferred) {
      props.config.terminalShellKind = preferred;
    } else if (!String(props.config.terminalShellKind || "").trim()) {
      props.config.terminalShellKind = "auto";
    }
  } catch {
    terminalShellOptions.value = [{ kind: "auto", label: "Auto", available: true }];
    if (!String(props.config.terminalShellKind || "").trim()) {
      props.config.terminalShellKind = "auto";
    }
  } finally {
    terminalShellOptionsLoading.value = false;
  }
}

function onTerminalShellKindChange(event: Event) {
  const target = event.target as HTMLSelectElement | null;
  const next = String(target?.value || "auto").trim() || "auto";
  props.config.terminalShellKind = next;
}

function toolStatusById(id: string): ToolLoadStatus | undefined {
  return props.toolStatuses.find((s) => s.id === id);
}

const showGitInstallHintInWorkspace = computed(
  () => isWindowsHost && toolStatusById("exec")?.status === "unavailable",
);

function openGitDownloadLink() {
  void openTransportExternalUrl(GIT_DOWNLOAD_URL);
}

onMounted(() => {
  void loadTerminalShellCandidates();
});
const templateGroups = computed<ConfigTemplateGroup[]>(() => [
  {
    key: "default-models",
    title: t("config.chatSettings.defaultModelsTitle"),
    rows: [
      { key: "vision-api", items: [] },
      { key: "tool-review-api", items: [] },
      { key: "expert-chat-model", items: [] },
      { key: "image-generation-model", items: [] },
      { key: "stt-api", items: [] },
      { key: "stt-auto-send", items: [] },
    ],
  },
  {
    key: "response-style",
    title: t("config.chatSettings.responseStyle"),
    rows: [{ key: "response-style", items: [] }],
  },
  {
    key: "exec-terminal",
    title: t("config.chatSettings.execTerminalTitle"),
    rows: [{ key: "exec-terminal", items: [] }],
  },
  {
    key: "instruction-presets",
    title: t("config.chatSettings.instructionPresetsTitle"),
    rows: [{ key: "instruction-presets", items: [] }],
  },
]);
const responseStyleSegmentOptions = computed(() =>
  props.responseStyleOptions.map((style) => ({
    value: style.id,
    label: t(`responseStyle.${style.id}`),
  })),
);
const imageGenerationModelOptions = computed(() =>
  deriveImageGenerationModelOptions(props.config.imageProviders || []),
);
const emit = defineEmits<{
  (e: "update:responseStyleId", value: string): void;
  (e: "update:pdfReadMode", value: "text" | "image"): void;
  (e: "update:instructionPresets", value: PromptCommandPreset[]): void;
  (e: "patchConversationApiSettings", value: ConversationApiSettingsPatch): void;
  (e: "patchChatSettings", value: ChatSettingsPatch): void;
}>();

function onVisionSelect(value: string) {
  props.config.visionApiConfigId = value || undefined;
  emit("patchConversationApiSettings", {
    visionApiConfigId: props.config.visionApiConfigId ?? null,
  });
}

function onToolReviewSelect(value: string) {
  props.config.toolReviewApiConfigId = value || undefined;
  emit("patchConversationApiSettings", {
    toolReviewApiConfigId: props.config.toolReviewApiConfigId ?? null,
  });
}

function onExpertSelect(value: string) {
  props.config.assistantDepartmentApiConfigId = value || "";
  emit("patchConversationApiSettings", {
    assistantDepartmentApiConfigId: props.config.assistantDepartmentApiConfigId,
  });
}

function onImageGenerationSelectChange(event: Event) {
  props.config.imageGenerationModelId = ((event.target as HTMLSelectElement).value || undefined);
}

function onResponseStyleChange(value: string) {
  emit("update:responseStyleId", value);
  emit("patchChatSettings", {
    responseStyleId: value,
  });
}

function onSttSelectChange(event: Event) {
  const value = (event.target as HTMLSelectElement).value || undefined;
  props.config.sttApiConfigId = value;
  if (!value) {
    props.config.sttAutoSend = false;
  }
  emit("patchConversationApiSettings", {
    sttApiConfigId: props.config.sttApiConfigId ?? null,
    sttAutoSend: !!props.config.sttAutoSend,
  });
}

function onSttAutoSendChange(event: Event) {
  if (!props.config.sttApiConfigId) {
    props.config.sttAutoSend = false;
    emit("patchConversationApiSettings", {
      sttApiConfigId: null,
      sttAutoSend: false,
    });
    return;
  }
  props.config.sttAutoSend = (event.target as HTMLInputElement).checked;
  emit("patchConversationApiSettings", {
    sttAutoSend: !!props.config.sttAutoSend,
  });
}

function randomInstructionPresetId(): string {
  return `instruction-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

function normalizeInstructionPresets(value: PromptCommandPreset[]): PromptCommandPreset[] {
  return (Array.isArray(value) ? value : [])
    .map((item) => ({
      id: String(item?.id || "").trim() || randomInstructionPresetId(),
      name: String(item?.prompt || item?.name || "").trim(),
      prompt: String(item?.prompt || item?.name || "").trim(),
    }))
    .filter((item) => !!item.prompt);
}

const instructionPresetsDraft = ref<PromptCommandPreset[]>(normalizeInstructionPresets(props.instructionPresets));

watch(
  () => props.instructionPresets,
  (value) => {
    instructionPresetsDraft.value = normalizeInstructionPresets(value);
  },
  { deep: true },
);

const instructionPresetsDirty = computed(() =>
  JSON.stringify(instructionPresetsDraft.value) !== JSON.stringify(normalizeInstructionPresets(props.instructionPresets)),
);

function addInstructionPreset() {
  instructionPresetsDraft.value = [
    ...instructionPresetsDraft.value,
    {
      id: randomInstructionPresetId(),
      name: "",
      prompt: "",
    },
  ];
}

function removeInstructionPreset(id: string) {
  instructionPresetsDraft.value = instructionPresetsDraft.value.filter((item) => item.id !== id);
}

function saveInstructionPresets() {
  const normalized = instructionPresetsDraft.value
    .map((item) => ({
      id: String(item.id || "").trim() || randomInstructionPresetId(),
      name: String(item.prompt || item.name || "").trim(),
      prompt: String(item.prompt || item.name || "").trim(),
    }))
    .filter((item) => !!item.prompt);
  instructionPresetsDraft.value = normalized;
  emit("update:instructionPresets", normalized);
  emit("patchChatSettings", {
    instructionPresets: normalized,
  });
}
</script>

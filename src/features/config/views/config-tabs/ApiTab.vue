<template>
  <SettingsStickyLayout>
    <template #header>
      <div class="flex flex-col gap-3">
        <div class="join w-full">
          <button v-for="tab in capabilityTabs" :key="tab.id" class="btn btn-sm join-item flex-1" type="button"
            :class="activeTopTab === tab.id ? 'btn-primary' : 'bg-base-100'" @click="switchCapabilityTab(tab.id)">
            {{ tab.label }}
          </button>
        </div>

        <ProviderToolbar
          v-if="activeTopTab !== 'imageGeneration'"
          :providers="providerToolbarOptions"
          :model-value="selectedProviderId"
          :empty-label="t('config.api.currentProvider')"
          :add-title="t('config.api.addProvider')"
          :remove-title="t('config.api.removeProvider')"
          :restore-title="t('config.api.restoreProviderDraft')"
          :save-title="props.savingConfig ? t('config.api.saving') : currentProviderDirty ? t('config.api.saveConfig') : t('config.api.saved')"
          :dirty="currentProviderDirty"
          :saving="props.savingConfig"
          :remove-disabled="scopedProviderList.length <= 1"
          :restore-disabled="!currentProviderDirty || props.savingConfig"
          :save-disabled="!currentProviderDirty || props.savingConfig"
          @update:model-value="selectProvider"
          @add="addProvider"
          @remove="removeProvider(selectedProviderId)"
          @restore="handleRestoreProviderDraft"
          @save="handleSaveApiConfig"
        />
        <ProviderToolbar
          v-else
          :providers="imageToolbarProviderOptions"
          :model-value="imageToolbarSelectedProviderId"
          :empty-label="t('config.imageGeneration.emptyProviders')"
          :add-title="t('config.imageGeneration.addProvider')"
          :remove-title="t('config.imageGeneration.removeProvider')"
          :restore-title="t('common.reset')"
          :save-title="imageToolbarSaveTitle"
          :dirty="imageToolbarDirty"
          :saving="imageToolbarSaving"
          :remove-disabled="imageToolbarRemoveDisabled"
          :restore-disabled="imageToolbarRestoreDisabled"
          :save-disabled="imageToolbarSaveDisabled"
          :select-disabled="imageToolbarProviderOptions.length === 0"
          @update:model-value="selectImageProvider"
          @add="addImageProvider"
          @remove="removeImageProvider"
          @restore="restoreImageProviderConfig"
          @save="saveImageProviderConfig"
        />
      </div>
    </template>

    <div v-if="activeTopTab !== 'imageGeneration' && selectedProvider" class="grid gap-3">
      <ConfigTemplate v-model="providerTemplateValues" :groups="providerTemplateGroups">
        <template #field-baseUrl="{ field, value, update }">
          <label class="grid min-w-0 gap-2">
            <div class="flex items-center gap-2">
              <div class="text-sm">{{ field.label }}</div>
              <button class="btn btn-xs bg-base-200" type="button" @click="baseUrlHelperOpen = !baseUrlHelperOpen">
                <WandSparkles class="h-3 w-3" />
                <span>{{ t("config.api.linkHelper") }}</span>
              </button>
            </div>
            <input
              :value="String(value ?? '')"
              :placeholder="field.placeholder"
              class="input input-bordered input-sm w-full"
              @input="update(($event.target as HTMLInputElement).value)"
            />
            <div v-if="baseUrlHelperOpen" class="rounded-box border border-base-300 bg-base-200/50 p-3">
              <div class="mb-2 text-xs opacity-70">{{ t("config.api.linkHelperHint") }}</div>
              <div class="tabs tabs-boxed mb-2 bg-base-100 p-1">
                <button v-for="tab in linkHelperTabs" :key="tab.value" class="tab tab-sm flex-1"
                  :class="linkHelperActiveProtocol === tab.value ? 'tab-active' : ''" type="button"
                  @click="linkHelperActiveProtocol = tab.value">
                  {{ tab.label }}
                </button>
              </div>
              <div class="flex flex-wrap gap-1">
                <div v-for="preset in filteredProviderPresets" :key="preset.id" class="join rounded-btn shadow-sm">
                  <button class="btn btn-sm join-item"
                    :class="selectedPresetId === preset.id ? 'btn-primary' : 'bg-base-100'" type="button"
                    @click="applyGeneratedBaseUrl(preset.id)">
                    {{ preset.name }}
                  </button>
                  <button class="btn btn-sm btn-neutral join-item" type="button" @click="openProviderSite(preset)">
                    <ExternalLink class="h-3 w-3" />
                  </button>
                </div>
              </div>
            </div>
          </label>
        </template>
        <template #row-allow-concurrent>
          <div class="grid min-w-0 gap-2">
            <div class="flex items-center justify-between gap-3">
              <span class="text-sm">{{ t("config.api.allowConcurrentRequests") }}</span>
              <span class="text-sm tabular-nums">{{ providerConcurrentLimitLabel(selectedProvider) }}</span>
            </div>
            <input
              :value="providerConcurrentLimit(selectedProvider)"
              type="range"
              min="0"
              max="16"
              step="1"
              class="range range-sm w-full"
              @input="updateProviderConcurrentLimit(selectedProvider, ($event.target as HTMLInputElement).value)"
            />
          </div>
        </template>
      </ConfigTemplate>


      <ApiKeyListCard
        v-if="!selectedProviderIsCodex"
        :title="t('config.imageGeneration.apiKeys')"
        :key="selectedProvider.id"
        :model-value="selectedProvider.apiKeys"
        :connection-test-key-status="connectionTestKeyStatus"
        @update:model-value="updateSelectedApiKeys"
      />

      <CodexProviderPanel
        v-else
        :provider="selectedProvider"
        :selected-api-config-id="props.config.selectedApiConfigId"
        :refreshing-models="props.refreshingModels"
        :model-options="props.modelOptions"
        :model-refresh-error="props.modelRefreshError"
        @refresh-models="$emit('refreshModels')"
        @select-model="selectModelCard"
      />

      <ConfigCard v-if="!selectedProviderIsCodex" :title="t('config.api.connectionTest')">
        <div class="flex items-center gap-2 py-3">
          <select v-model="connectionTestModelId" class="select select-bordered select-sm flex-1">
            <option v-for="m in draftViewModels" :key="m.id" :value="m.id">
              {{ modelDisplayLabel(selectedProvider, m) }}
            </option>
          </select>
          <button class="btn btn-sm" type="button"
            :class="connectionTestFirstKeyRunning ? 'loading' : 'bg-base-200'"
            :disabled="connectionTestFirstKeyRunning || connectionTestAllKeysRunning"
            @click="runConnectionTestFirstKey">
            <span v-if="connectionTestFirstKeyRunning" class="loading loading-spinner loading-xs"></span>
            {{ t("config.api.testFirstKey") }}
          </button>
          <button class="btn btn-sm" type="button"
            :class="connectionTestAllKeysRunning ? 'loading' : 'bg-base-200'"
            :disabled="connectionTestFirstKeyRunning || connectionTestAllKeysRunning"
            @click="runConnectionTestAllKeys">
            <span v-if="connectionTestAllKeysRunning" class="loading loading-spinner loading-xs"></span>
            {{ t("config.api.testAllKeys") }}
          </button>
        </div>
      </ConfigCard>

      <ConfigCard v-if="!selectedProviderIsCodex" :title="t('config.api.modelCards')">
        <template #actions>
          <button class="btn btn-sm" type="button" :class="{ loading: props.refreshingModels }"
            :disabled="props.refreshingModels" @click="$emit('refreshModels')">
            <RefreshCw class="h-3.5 w-3.5" />
            <span>{{ t("config.api.refreshModels") }}</span>
          </button>
          <button class="btn btn-sm" type="button" @click="addModelCard">
            <Plus class="h-3.5 w-3.5" />
            <span>{{ t("config.api.addModel") }}</span>
          </button>
        </template>
        <div class="flex items-center justify-between gap-2 py-3">
          <span
            class="text-xs min-w-0 truncate"
            :class="props.modelRefreshError
              ? 'text-error'
              : props.modelRefreshOk
                ? 'text-success'
                : 'text-base-content/55'"
          >{{ modelRefreshStatusText }}</span>
        </div>

        <div class="divide-y divide-base-200/60">
              <ApiModelCard
                v-for="group in draftModelGroups"
                :key="group.primary.id"
                :card="group.primary"
                :model-options="providerModelOptions"
                :default-open="draftModelGroups.length <= 2"
                :capability="reasoningCapability(group) ?? null"
                :show-delete="true"
                :delete-disabled="draftModelGroups.length <= 1"
                :show-capability-toggles="selectedCapability === 'text'"
                :show-context-window="selectedCapability === 'text'"
                :show-reasoning="selectedCapability === 'text'"
                :show-temperature="selectedCapability === 'text'"
                :show-max-output-tokens="selectedCapability === 'text'"
                :reasoning-items="reasoningEffortItems(group)"
                :reasoning-checked-values="reasoningEffortCheckedValues(group)"
                :reasoning-status="reasoningCapabilityStatus(group)"
                :protocol-hint="selectedProtocol === 'auto' ? resolvedAdapterByModelId[group.primary.id] : ''"
                :documentation-url="modelDocumentationUrl(group)"
                :connection-result="modelConnectionResult[group.primary.id] ?? null"
                :context-window-max="contextWindowMax(group)"
                embedded
                @select="selectModelCard(group.primary.id)"
                @remove="removeModelGroup(group)"
                @sync-metadata="handleModelCardSyncMetadata(group)"
                @select-option="(option: string) => selectModelOption(group, option)"
                @toggle-max-output="handleCustomMaxOutputTokensToggle(group)"
                @reasoning-change="(payload: { value: string; checked: boolean }) => setGroupReasoningEffort(group, payload.value, payload.checked)"
                @open-documentation="openModelDocumentation(group)"
              />
        </div>
      </ConfigCard>
    </div>

    <div v-else-if="activeTopTab === 'imageGeneration'" class="grid gap-3">
      <ImageGenerationTab
        ref="imageGenerationTabRef"
        :config="config"
        :saving-config="savingConfig"
        :save-config-action="saveApiConfigAction"
        :last-saved-config-json="lastSavedConfigJson"
        :set-status-action="setStatusAction"
      />
    </div>
    <dialog class="modal" :class="{ 'modal-open': providerDeleteDialogOpen }">
      <div class="modal-box max-w-sm">
        <h3 class="text-lg font-semibold">{{ t("config.api.deleteProviderTitle") }}</h3>
        <p class="py-3 text-sm opacity-80">{{ t("config.api.deleteProviderConfirm", { name: pendingDeleteProviderName }) }}</p>
        <div class="modal-action">
          <button class="btn btn-ghost" type="button" @click="closeDeleteProviderDialog">
            {{ t("common.cancel") }}
          </button>
          <button class="btn btn-error" type="button" @click="confirmDeleteProvider">
            {{ t("common.confirm") }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop" @submit.prevent="closeDeleteProviderDialog">
        <button type="submit">close</button>
      </form>
    </dialog>
  </SettingsStickyLayout>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { ExternalLink, Plus, RefreshCw, WandSparkles } from "@lucide/vue";
import type { ApiModelConfigItem, ApiProviderConfigItem, ApiRequestFormat, AppConfig, CodexAuthMode, CodexAuthStatus } from "../../../../types/app";
import ApiKeyListCard, { type ApiKeyConnectionStatus } from "../../components/ApiKeyListCard.vue";
import ApiModelCard from "../../components/ApiModelCard.vue";
import ConfigCard from "../../components/ConfigCard.vue";
import ConfigTemplate from "../../components/ConfigTemplate.vue";
import ProviderToolbar, { type ProviderToolbarOption } from "../../components/ProviderToolbar.vue";
import SettingsStickyLayout from "../../components/SettingsStickyLayout.vue";
import { canUseTransportGenaiChatAdapters, invokeTauri, openTransportExternalUrl } from "../../../../services/tauri-api";
import CodexProviderPanel from "./CodexProviderPanel.vue";
import ImageGenerationTab from "./ImageGenerationTab.vue";
import { normalizeApiRequestFormat } from "../../utils/api-request-format";
import {
  LEGAL_REASONING_EFFORTS,
  reasoningEffortDisplayLabel as sharedReasoningEffortDisplayLabel,
  sortReasoningEffortValues,
} from "../../utils/api-config-display";
import { buildModelCapability, type ModelCapabilitySnapshot } from "../../utils/model-capability";
import {
  AUTO_CONTEXT_WINDOW_TOKENS,
  buildDraftGroups,
  modelGroupKey,
  normalizedModelReasoningEffortFor,
  splitDraftGroups,
  type DraftModelGroup,
} from "../../utils/draft-model-groups";
import type { ConfigTemplateGroup } from "../../components/config-template";

type ApiCapability = "text" | "voice" | "embedding" | "rerank";
type ApiTopTab = ApiCapability | "imageGeneration";
type ProviderPresetCategory = "official" | "domestic" | "openaiCompatible" | "local";
type ProviderPreset = {
  id: string;
  name: string;
  category: ProviderPresetCategory;
  urls: Partial<Record<ApiRequestFormat, string>>;
  docsUrl: string;
  hasFreeQuota?: boolean;
};

type ProtocolOption = { value: ApiRequestFormat; label: string };
type FetchModelMetadataResult = {
  found: boolean;
  fuzzyMatch?: boolean | null;
  providerName?: string | null;
  providerApi?: string | null;
  matchedModelId?: string | null;
  contextWindowTokens?: number | null;
  maxOutputTokens?: number | null;
  enableImage?: boolean | null;
  enableVideo?: boolean | null;
  enableTools?: boolean | null;
  enableAudio?: boolean | null;
  reasoning?: boolean | null;
  reasoningEffortOptions?: string[] | null;
  documentationUrl?: string | null;
};
type ModelCapabilityLimits = Partial<ModelCapabilitySnapshot> & {
  metadataFound?: boolean;
};
type ImageGenerationToolbarState = {
  providers: ProviderToolbarOption[];
  selectedProviderId: string;
  dirty: boolean;
  saving: boolean;
  removeDisabled: boolean;
  restoreDisabled: boolean;
  saveDisabled: boolean;
};
type ImageGenerationTabPublicInstance = {
  toolbarState: ImageGenerationToolbarState;
  selectProvider: (providerId: string) => void;
  addProvider: () => void;
  removeSelectedProvider: () => void;
  restoreImageConfig: () => void;
  saveImageConfig: () => Promise<void>;
};

const SLIDER_CONTEXT_MIN = 16_000;
const AUTO_CONTEXT_WINDOW_SMALL_MODEL_THRESHOLD = 200_000;
const FALLBACK_CONTEXT_WINDOW_MAX = 2_000_000;
const DEFAULT_CODEX_BASE_URL = "https://chatgpt.com/backend-api/codex";
const DEFAULT_CODEX_AUTH_MODE: CodexAuthMode = "read_local";
const DEFAULT_CODEX_LOCAL_AUTH_PATH = "~/.codex/auth.json";
const DEFAULT_REASONING_EFFORT = "medium";
const DEFAULT_GEMINI_REASONING_EFFORT = "high";
const DEFAULT_OPENAI_REASONING_EFFORT = "high";
const DEFAULT_DEEPSEEK_REASONING_EFFORT = "none";
const props = defineProps<{
  config: AppConfig;
  baseUrlReference: string;
  refreshingModels: boolean;
  modelOptions: string[];
  modelRefreshOk: boolean;
  modelRefreshError: string;
  configDirty: boolean;
  savingConfig: boolean;
  saveApiConfigAction: () => Promise<boolean> | boolean;
  normalizeApiBindingsAction: () => void;
  lastSavedConfigJson: string;
  setStatusAction: (text: string) => void;
}>();

const emit = defineEmits<{
  (e: "refreshModels"): void;
}>();

const { t } = useI18n();
const openaiReasoningEffortOptions = computed(() => [
  { value: "none", label: t("config.api.reasoningOff") },
  { value: "low", label: t("config.api.reasoningLow") },
  { value: "medium", label: t("config.api.reasoningMedium") },
  { value: "high", label: t("config.api.reasoningHigh") },
  { value: "xhigh", label: t("config.api.reasoningXHigh") },
]);
const deepseekReasoningEffortOptions = computed(() => [
  { value: "none", label: t("config.api.reasoningOff") },
  { value: "high", label: t("config.api.reasoningHigh") },
  { value: "xhigh", label: t("config.api.reasoningXHigh") },
]);
const geminiReasoningEffortOptions = computed(() => [
  { value: "low", label: t("config.api.reasoningLow") },
  { value: "high", label: t("config.api.reasoningHigh") },
]);
const baseUrlHelperOpen = ref(false);
const linkHelperActiveProtocol = ref<ApiRequestFormat>("openai");
const selectedPresetId = ref("openai-official");
const providerDeleteDialogOpen = ref(false);
const pendingDeleteProviderId = ref("");
const pendingDeleteProviderName = ref("");
const imageGenerationTabRef = ref<ImageGenerationTabPublicInstance | null>(null);
const modelCapabilityById = ref<Record<string, ModelCapabilityLimits>>({});
const resolvedAdapterByModelId = ref<Record<string, string>>({});
const adapterResolveRequestSeq = ref(0);
const codexAuthBusy = ref(false);
const codexAuthStatusByProvider = ref<Record<string, CodexAuthStatus>>({});
const codexAuthPollTimer = ref<number | null>(null);
type ModelConnectionResult = { success: boolean; latencyMs?: number; error?: string };
const modelConnectionTesting = ref<Record<string, boolean>>({});
const modelConnectionResult = ref<Record<string, ModelConnectionResult>>({});
type ConnectionTestResultItem = { keyPreview: string; success: boolean; latencyMs?: number; error?: string };
const connectionTestModelId = ref("");
const connectionTestFirstKeyRunning = ref(false);
const connectionTestAllKeysRunning = ref(false);
const connectionTestResults = ref<ConnectionTestResultItem[]>([]);
const connectionTestKeyStatus = ref<Record<string, { status: "success" | "failed"; latencyMs?: number; error?: string }>>({});
const capabilityTabs = computed<Array<{ id: ApiTopTab; label: string }>>(() => [
  { id: "text", label: t("config.api.capabilityText") },
  { id: "voice", label: t("config.api.capabilityVoice") },
  { id: "embedding", label: t("config.api.capabilityEmbedding") },
  { id: "rerank", label: t("config.api.capabilityRerank") },
  { id: "imageGeneration", label: t("config.tabs.imageGeneration") },
]);
// 本地补充项：genai 无对应 adapter，但项目自身支持（auto=自动探测、codex=本地协议）。
const LOCAL_TEXT_PROTOCOL_OPTIONS: ProtocolOption[] = [
  { value: "auto", label: "Auto" },
  { value: "openai", label: "OpenAI Compatible" },
  { value: "codex", label: "OpenAI Codex" },
];

// genai 清单 id → 前端协议值 映射；未命中（后端 supported=false）的适配器不进入候选。
const GENAI_ADAPTER_TO_PROTOCOL: Record<string, ApiRequestFormat> = {
  openai: "openai",
  openai_resp: "openai_responses",
  deepseek: "deepseek",
  gemini: "gemini",
  anthropic: "anthropic",
  fireworks: "fireworks",
  together: "together",
  groq: "groq",
  kimi: "moonshot",
  moonshot: "moonshot",
  mimo: "mimo",
  minimax: "minimax",
  nebius: "nebius",
  xai: "xai",
  zai: "zai",
  bigmodel: "bigmodel",
  aliyun: "aliyun",
  baidu: "baidu",
  cohere: "cohere",
  ollama: "ollama",
  ollama_cloud: "ollama_cloud",
  vertex: "vertex",
  github_copilot: "github_copilot",
  opencode_go: "opencode_go",
  bedrock_api: "bedrock_api",
};

const GENAI_ADAPTER_LABELS: Record<string, string> = {
  openai: "OpenAI Compatible",
  openai_resp: "OpenAI Responses",
  deepseek: "DeepSeek",
  gemini: "Google Gemini",
  anthropic: "Anthropic",
  fireworks: "Fireworks",
  together: "Together AI",
  groq: "Groq",
  kimi: "Moonshot/Kimi",
  moonshot: "Moonshot/Kimi",
  mimo: "Mimo",
  minimax: "MiniMax",
  nebius: "Nebius",
  xai: "xAI",
  zai: "Zai",
  bigmodel: "BigModel",
  aliyun: "Aliyun",
  baidu: "Baidu",
  cohere: "Cohere",
  ollama: "Ollama",
  ollama_cloud: "Ollama Cloud",
  vertex: "Google Vertex AI",
  github_copilot: "GitHub Copilot",
  opencode_go: "OpenCode Go",
  bedrock_api: "AWS Bedrock API",
};

const localProtocolOptionsByCapability: Record<ApiCapability, ProtocolOption[]> = {
  text: LOCAL_TEXT_PROTOCOL_OPTIONS,
  voice: [
    { value: "openai_stt", label: "OpenAI STT" },
    { value: "mimo_asr", label: "MiMo ASR" },
    { value: "openai_tts", label: "OpenAI TTS" },
  ],
  embedding: [
    { value: "openai_embedding", label: "OpenAI Embedding" },
    { value: "gemini_embedding", label: "Gemini Embedding" },
  ],
  rerank: [
    { value: "openai_rerank", label: "OpenAI Rerank" },
  ],
};

// genai 内置 chat 适配器清单（后端权威源）；未加载时为 null，text 候选仅显示本地项。
const genaiChatAdapters = ref<Array<{ id: string; label: string; supported: boolean }> | null>(null);

function protocolOptionsByCapability(): Record<ApiCapability, ProtocolOption[]> {
  const adapters = genaiChatAdapters.value;
  if (!adapters) return localProtocolOptionsByCapability;
  const seen = new Set(localProtocolOptionsByCapability.text.map((option) => option.value));
  const textFromGenai: ProtocolOption[] = [];
  for (const adapter of adapters) {
    if (!adapter.supported) continue;
    const protocol = GENAI_ADAPTER_TO_PROTOCOL[adapter.id];
    if (!protocol || seen.has(protocol)) continue;
    seen.add(protocol);
    textFromGenai.push({ value: protocol, label: GENAI_ADAPTER_LABELS[adapter.id] || adapter.label });
  }
  return {
    ...localProtocolOptionsByCapability,
    text: [...localProtocolOptionsByCapability.text, ...textFromGenai],
  };
}
const capabilityDefaultProtocol: Record<ApiCapability, ApiRequestFormat> = {
  text: "auto",
  voice: "mimo_asr",
  embedding: "openai_embedding",
  rerank: "openai_rerank",
};

const providerPresets: ProviderPreset[] = [
  { id: "openai-official", name: "OpenAI", category: "official", urls: { auto: "https://api.openai.com/v1", openai: "https://api.openai.com/v1", openai_responses: "https://api.openai.com/v1", openai_stt: "https://api.openai.com/v1", openai_tts: "https://api.openai.com/v1/audio/speech", openai_embedding: "https://api.openai.com/v1", openai_rerank: "https://api.openai.com/v1" }, docsUrl: "https://platform.openai.com/docs/overview" },
  { id: "xiaomi-mimo", name: "Xiaomi MiMo", category: "domestic", urls: { mimo: "https://api.xiaomimimo.com/v1", anthropic: "https://api.xiaomimimo.com/anthropic", mimo_asr: "https://api.xiaomimimo.com/v1" }, docsUrl: "https://mimo.mi.com/docs/zh-CN/quick-start/usage-guide/audio/Speech-Recognition" },
  { id: "xiaomi-mimo-token-plan", name: "Xiaomi MiMo Token Plan", category: "domestic", urls: { mimo: "https://token-plan-cn.xiaomimimo.com/v1", anthropic: "https://token-plan-cn.xiaomimimo.com/anthropic", mimo_asr: "https://token-plan-cn.xiaomimimo.com/v1" }, docsUrl: "https://mimo.mi.com/docs/zh-CN/tokenplan/quick-access" },
  { id: "openai-codex", name: "OpenAI Codex", category: "official", urls: { codex: DEFAULT_CODEX_BASE_URL }, docsUrl: "https://chatgpt.com" },
  { id: "anthropic-official", name: "Anthropic", category: "official", urls: { anthropic: "https://api.anthropic.com" }, docsUrl: "https://docs.anthropic.com/en/api/overview" },
  { id: "google-gemini", name: "Google Gemini", category: "official", urls: { gemini: "https://generativelanguage.googleapis.com", gemini_embedding: "https://generativelanguage.googleapis.com" }, docsUrl: "https://ai.google.dev/gemini-api/docs", hasFreeQuota: true },
  { id: "deepseek", name: "DeepSeek", category: "domestic", urls: { auto: "https://api.deepseek.com/v1", deepseek: "https://api.deepseek.com/v1", anthropic: "https://api.deepseek.com/anthropic", openai: "https://api.deepseek.com/v1", openai_responses: "https://api.deepseek.com/v1" }, docsUrl: "https://api-docs.deepseek.com/" },
  { id: "moonshot-kimi", name: "Moonshot/Kimi", category: "domestic", urls: { auto: "https://api.moonshot.cn/v1", moonshot: "https://api.moonshot.cn/v1", anthropic: "https://api.kimi.com/coding/", openai: "https://api.moonshot.cn/v1", openai_responses: "https://api.moonshot.cn/v1" }, docsUrl: "https://platform.moonshot.cn/docs/api-reference" },
  { id: "aliyun-bailian-coding", name: "百炼编程", category: "domestic", urls: { anthropic: "https://coding.dashscope.aliyuncs.com/apps/anthropic/v1", openai: "https://coding.dashscope.aliyuncs.com/v1", openai_responses: "https://coding.dashscope.aliyuncs.com/v1" }, docsUrl: "https://help.aliyun.com/zh/model-studio/" },
  { id: "aliyun-bailian", name: "百炼通用", category: "domestic", urls: { auto: "https://dashscope.aliyuncs.com/compatible-mode/v1", anthropic: "https://dashscope.aliyuncs.com/apps/anthropic", openai: "https://dashscope.aliyuncs.com/compatible-mode/v1", openai_responses: "https://dashscope.aliyuncs.com/compatible-mode/v1" }, docsUrl: "https://help.aliyun.com/zh/model-studio/" },
  { id: "baidu-qianfan", name: "百度千帆", category: "domestic", urls: { baidu: "https://qianfan.baidubce.com/v2", openai: "https://qianfan.baidubce.com/v2", openai_responses: "https://qianfan.baidubce.com/v2" }, docsUrl: "https://cloud.baidu.com/doc/WENXINWORKSHOP/index.html" },
  { id: "zhipu-glm", name: "Zhipu GLM", category: "domestic", urls: { anthropic: "https://open.bigmodel.cn/api/anthropic", openai: "https://open.bigmodel.cn/api/paas/v4", openai_responses: "https://open.bigmodel.cn/api/paas/v4" }, docsUrl: "https://open.bigmodel.cn/dev/api", hasFreeQuota: true },
  { id: "minimax", name: "MiniMax", category: "domestic", urls: { minimax: "https://api.minimax.io/anthropic/v1", anthropic: "https://api.minimax.io/anthropic/v1", openai: "https://api.minimax.io/v1", openai_responses: "https://api.minimax.io/v1" }, docsUrl: "https://platform.minimax.io/docs" },
  { id: "volcengine-ark", name: "火山方舟", category: "domestic", urls: { openai: "https://ark.cn-beijing.volces.com/api/v3", openai_responses: "https://ark.cn-beijing.volces.com/api/v3" }, docsUrl: "https://www.volcengine.com/docs/82379" },
  { id: "volcengine-ark-coding", name: "火山方舟编程", category: "domestic", urls: { anthropic: "https://ark.cn-beijing.volces.com/api/coding", openai: "https://ark.cn-beijing.volces.com/api/coding/v3", openai_responses: "https://ark.cn-beijing.volces.com/api/coding/v3" }, docsUrl: "https://www.volcengine.com/docs/82379" },
  { id: "siliconflow", name: "SiliconFlow", category: "domestic", urls: { auto: "https://api.siliconflow.cn/v1", openai: "https://api.siliconflow.cn/v1", openai_responses: "https://api.siliconflow.cn/v1", openai_stt: "https://api.siliconflow.cn/v1", openai_embedding: "https://api.siliconflow.cn/v1", openai_rerank: "https://api.siliconflow.cn/v1" }, docsUrl: "https://docs.siliconflow.cn/", hasFreeQuota: true },
  { id: "modelscope", name: "ModelScope", category: "domestic", urls: { auto: "https://api-inference.modelscope.cn/v1", openai: "https://api-inference.modelscope.cn/v1", openai_responses: "https://api-inference.modelscope.cn/v1" }, docsUrl: "https://modelscope.cn/models", hasFreeQuota: true },
  { id: "nvidia-nim", name: "NVIDIA NIM", category: "openaiCompatible", urls: { auto: "https://integrate.api.nvidia.com/v1", openai: "https://integrate.api.nvidia.com/v1", openai_responses: "https://integrate.api.nvidia.com/v1" }, docsUrl: "https://docs.api.nvidia.com/nim/", hasFreeQuota: true },
  { id: "openrouter", name: "OpenRouter", category: "openaiCompatible", urls: { auto: "https://openrouter.ai/api/v1", openai: "https://openrouter.ai/api/v1", openai_responses: "https://openrouter.ai/api/v1" }, docsUrl: "https://openrouter.ai/docs/api-reference/overview", hasFreeQuota: true },
  { id: "cloudflare-gateway", name: "Cloudflare Gateway", category: "openaiCompatible", urls: { openai: "https://gateway.ai.cloudflare.com/v1/{account_id}/{gateway_id}/{provider}", openai_responses: "https://gateway.ai.cloudflare.com/v1/{account_id}/{gateway_id}/{provider}" }, docsUrl: "https://developers.cloudflare.com/ai-gateway/" },
  { id: "opencode-go", name: "OpenCode Go", category: "openaiCompatible", urls: { opencode_go: "https://opencode.ai/zen/go/v1", openai: "https://opencode.ai/zen/go/v1" }, docsUrl: "https://opencode.ai" },
  { id: "aws-bedrock-api", name: "AWS Bedrock API", category: "official", urls: { bedrock_api: "https://bedrock-runtime.us-east-1.amazonaws.com" }, docsUrl: "https://docs.aws.amazon.com/bedrock/latest/userguide/conversation-inference.html" },
  { id: "ollama-local", name: "Ollama (Local)", category: "local", urls: { ollama: "http://localhost:11434", openai: "http://localhost:11434/v1", openai_responses: "http://localhost:11434/v1" }, docsUrl: "https://github.com/ollama/ollama/blob/main/docs/openai.md" },
];
const reasoningEffortOptions = [
  { value: "low", label: t('sidebar.apiPriorityLow') },
  { value: "medium", label: t('sidebar.apiPriorityMedium') },
  { value: "high", label: t('sidebar.apiPriorityHigh') },
  { value: "xhigh", label: t('sidebar.apiPriorityXhigh') },
];
const codexAuthModeOptions: Array<{ value: CodexAuthMode; label: string }> = [
  { value: "read_local", label: t('sidebar.apiCredentialLocal') },
  { value: "managed_oauth", label: t('sidebar.apiCredentialOauth') },
];

const TEXT_REQUEST_FORMATS = new Set<ApiRequestFormat>([
  "auto",
  "openai",
  "deepseek",
  "openai_responses",
  "codex",
  "gemini",
  "anthropic",
  "fireworks",
  "together",
  "groq",
  "mimo",
  "minimax",
  "moonshot",
  "nebius",
  "xai",
  "zai",
  "bigmodel",
  "aliyun",
  "baidu",
  "cohere",
  "ollama",
  "ollama_cloud",
  "vertex",
  "github_copilot",
  "opencode_go",
  "bedrock_api",
]);

function canonicalRequestFormat(format: string): ApiRequestFormat {
  return normalizeApiRequestFormat(format);
}

function isTextRequestFormat(format: string): format is ApiRequestFormat {
  return TEXT_REQUEST_FORMATS.has(canonicalRequestFormat(format));
}

function isProviderDeprecated(provider: ApiProviderConfigItem | null | undefined): boolean {
  return !!provider?.deprecated;
}

function isModelDeprecated(model: ApiModelConfigItem | null | undefined): boolean {
  return !!model?.deprecated;
}

function firstActiveModel(provider: ApiProviderConfigItem | null | undefined): ApiModelConfigItem | null {
  if (!provider) return null;
  return (provider.models || []).find((model) => !isModelDeprecated(model)) ?? null;
}

function reasoningEffortDisplayLabel(value: string): string {
  return sharedReasoningEffortDisplayLabel(value, t);
}

function modelDisplayLabel(
  provider: ApiProviderConfigItem | null | undefined,
  model: ApiModelConfigItem | null | undefined,
): string {
  const providerLabel = String(provider?.name || provider?.id || "").trim();
  const modelLabel = String(model?.displayName || model?.model || "").trim() || t("config.api.unnamedModel");
  const reasoningValue = String(model?.reasoningEffort || "").trim();
  const reasoningLabel = reasoningEffortDisplayLabel(reasoningValue);
  const base = providerLabel ? `${providerLabel}/${modelLabel}` : modelLabel;
  return reasoningLabel ? `${base} · ${reasoningLabel}` : base;
}

const providerList = computed(() => props.config.apiProviders || []);
const activeProviderList = computed(() => providerList.value.filter((provider) => !isProviderDeprecated(provider)));
const selectedProviderId = computed(() => {
  const [providerId] = String(props.config.selectedApiConfigId || "").split("::");
  return providerId || activeProviderList.value[0]?.id || "";
});

const selectedProvider = computed(() => {
  const [providerId] = String(props.config.selectedApiConfigId || "").split("::");
  return activeProviderList.value.find((provider) => provider.id === providerId) ?? activeProviderList.value[0] ?? null;
});

const draftModelGroups = ref<DraftModelGroup[]>([]);

// 草稿视图模型列表（纯读）：每组每个勾选等级一张卡，供连接测试等基于当前草稿内容的场景使用
const draftViewModels = computed<ApiModelConfigItem[]>(() =>
  draftModelGroups.value.flatMap((group) =>
    (group.reasoningEfforts.length > 0 ? group.reasoningEfforts : ["default"]).map((effort) => ({
      ...group.primary,
      id: group.variantIdByEffort.get(String(effort || "").trim().toLowerCase() || "default") ?? group.primary.id,
      reasoningEffort: String(effort || "").trim().toLowerCase() || "default",
      deprecated: false,
    })),
  ),
);

// ========== 草稿聚合（读取时执行一次，编辑期间不重建） ==========

function rebuildDraftGroups() {
  const provider = selectedProvider.value;
  draftModelGroups.value = buildDraftGroups(provider);
}

// ========== 草稿拆分（保存时执行一次，写回 config.models） ==========

function commitDraftGroups() {
  const provider = selectedProvider.value;
  if (!provider) return;
  // 先算草稿拆分结果，确定哪些卡被移除；被移除的活跃卡先标记 deprecated（保留历史，边界 4）
  const draftModels = splitDraftGroups(provider, draftModelGroups.value, () => `api-model-${buildProviderSeed()}`);
  const keptIds = new Set(draftModels.map((model) => model.id));
  const actuallyRemoved: string[] = [];
  for (const model of provider.models || []) {
    if (isModelDeprecated(model)) continue;
    if (!keptIds.has(model.id)) {
      model.deprecated = true;
      actuallyRemoved.push(`${provider.id}::${model.id}`);
    }
  }
  // 标记后再拆一次，确保 deprecated 卡被 splitDraftGroups 保留
  provider.models = splitDraftGroups(provider, draftModelGroups.value, () => `api-model-${buildProviderSeed()}`);
  if (actuallyRemoved.length > 0) {
    clearRemovedApiConfigReferences(actuallyRemoved);
    props.normalizeApiBindingsAction();
  }
  // 仅当原选中卡被移除时才 fallback，保留用户当前选中
  const [, selectedModelId] = String(props.config.selectedApiConfigId || "").split("::");
  const selectedStillActive = selectedModelId && provider.models.some(
    (model) => model.id === selectedModelId && !isModelDeprecated(model),
  );
  if (!selectedStillActive) {
    const fallback = firstActiveModel(provider);
    props.config.selectedApiConfigId = fallback
      ? `${provider.id}::${fallback.id}`
      : firstActiveApiConfigIdExcluding(new Set(actuallyRemoved));
  }
}

const selectedCapability = computed<ApiCapability>(() => capabilityFromRequestFormat(selectedProvider.value?.requestFormat || "openai"));
const activeTopTab = ref<ApiTopTab>(selectedCapability.value);
const emptyImageToolbarState: ImageGenerationToolbarState = {
  providers: [],
  selectedProviderId: "",
  dirty: false,
  saving: false,
  removeDisabled: true,
  restoreDisabled: true,
  saveDisabled: true,
};
const imageToolbarState = computed<ImageGenerationToolbarState>(() => imageGenerationTabRef.value?.toolbarState ?? emptyImageToolbarState);
const imageToolbarProviderOptions = computed(() => imageToolbarState.value.providers);
const imageToolbarSelectedProviderId = computed(() => imageToolbarState.value.selectedProviderId);
const imageToolbarDirty = computed(() => imageToolbarState.value.dirty);
const imageToolbarSaving = computed(() => imageToolbarState.value.saving);
const imageToolbarRemoveDisabled = computed(() => imageToolbarState.value.removeDisabled);
const imageToolbarRestoreDisabled = computed(() => imageToolbarState.value.restoreDisabled);
const imageToolbarSaveDisabled = computed(() => imageToolbarState.value.saveDisabled);
const imageToolbarSaveTitle = computed(() => (
  props.savingConfig
    ? t("config.api.saving")
    : imageToolbarDirty.value
      ? t("common.save")
      : t("common.saved")
));
const scopedProviderList = computed(() =>
  activeProviderList.value.filter((provider) => capabilityFromRequestFormat(provider.requestFormat) === selectedCapability.value),
);
const providerToolbarOptions = computed<ProviderToolbarOption[]>(() => scopedProviderList.value.map((provider) => ({
  id: provider.id,
  label: `${provider.name || provider.id}（${provider.requestFormat}）`,
})));
const protocolOptions = computed(() =>
  protocolOptionsByCapability()[selectedCapability.value].map((option) =>
    option.value === "auto"
      ? { ...option, label: t("config.api.protocolAuto") }
      : option,
  ),
);

const selectedProtocol = computed<ApiRequestFormat>(() => canonicalRequestFormat(selectedProvider.value?.requestFormat || "openai"));
const selectedProviderIsCodex = computed(() => selectedProtocol.value === "codex");
const providerTemplateValues = computed<Record<string, unknown>>({
  get: () => {
    const provider = selectedProvider.value;
    if (!provider) return {};
    return {
      name: provider.name,
      requestFormat: provider.requestFormat,
      baseUrl: provider.baseUrl,
    };
  },
  set: (values) => {
    const provider = selectedProvider.value;
    if (!provider) return;
    if (typeof values.name === "string") provider.name = values.name;
    if (typeof values.baseUrl === "string") provider.baseUrl = values.baseUrl;
    if (typeof values.requestFormat === "string" && values.requestFormat !== provider.requestFormat) {
      provider.requestFormat = values.requestFormat as ApiRequestFormat;
      if (provider.requestFormat !== "codex") {
        stopCodexAuthPolling();
      } else {
        void refreshCodexAuthStatus(provider);
      }
    }
  },
});
const providerTemplateGroups = computed<ConfigTemplateGroup[]>(() => {
  const provider = selectedProvider.value;
  if (!provider) return [];
  const rows: ConfigTemplateGroup["rows"] = [
    {
      items: [{
        key: "name",
        label: t("config.api.configName"),
        type: "text",
        placeholder: t("config.api.providerNamePlaceholder"),
      }],
    },
    {
      items: [{
        key: "requestFormat",
        label: t("config.api.requestFormat"),
        type: "select",
        options: protocolOptions.value.map((item) => ({ value: item.value, label: item.label })),
      }],
    },
  ];
  if (!selectedProviderIsCodex.value) {
    rows.push({
      items: [{
        key: "baseUrl",
        label: t("config.api.baseUrl"),
        type: "text",
        placeholder: props.baseUrlReference,
      }],
    });
    rows.push({ key: "allow-concurrent", items: [] });
  }
  return [{ title: t("config.api.providerSettings"), rows }];
});
const currentCodexAuthStatus = computed(() => {
  const providerId = String(selectedProvider.value?.id || "").trim();
  return providerId ? codexAuthStatusByProvider.value[providerId] ?? null : null;
});

watch(
  selectedCapability,
  (value) => {
    if (activeTopTab.value !== "imageGeneration") {
      activeTopTab.value = value;
    }
  },
  { immediate: true },
);

const linkHelperTabs = computed(() =>
  protocolOptions.value.filter((option) =>
    option.value !== "auto" && providerPresets.some((preset) => Boolean(preset.urls[option.value])),
  ),
);

const filteredProviderPresets = computed(() => {
  const matched = providerPresets.filter((preset) =>
    Boolean(preset.urls[linkHelperActiveProtocol.value]),
  );
  return [...matched].sort((a, b) => Number(Boolean(b.hasFreeQuota)) - Number(Boolean(a.hasFreeQuota)));
});

const selectedPreset = computed(() =>
  providerPresets.find((preset) =>
    preset.id === selectedPresetId.value && Boolean(preset.urls[linkHelperActiveProtocol.value]),
  ) ?? filteredProviderPresets.value[0] ?? providerPresets[0],
);

const generatedBaseUrl = computed(() => {
  const preset = selectedPreset.value;
  return preset?.urls[linkHelperActiveProtocol.value] || "";
});

function defaultLinkHelperProtocol(): ApiRequestFormat {
  const protocol = selectedProtocol.value;
  if (
    protocol !== "auto"
    && providerPresets.some((preset) => Boolean(preset.urls[protocol]))
  ) {
    return protocol;
  }
  return linkHelperTabs.value[0]?.value ?? "openai";
}

const providerModelOptions = computed(() => {
  const provider = selectedProvider.value;
  if (!provider) return [];
  const cached = Array.isArray(provider.cachedModelOptions) ? provider.cachedModelOptions : [];
  return Array.from(new Set([...props.modelOptions, ...cached].map((item) => String(item || "").trim()).filter(Boolean)));
});

const modelRefreshStatusText = computed(() => {
  if (props.modelRefreshError) return props.modelRefreshError;
  if (props.modelRefreshOk) return t("status.modelListRefreshed", { count: providerModelOptions.value.length });
  const initialCount = providerModelOptions.value.length;
  if (initialCount > 0) return t("config.api.modelCount", { count: initialCount });
  return t("config.api.noModels");
});

const savedProviderMap = computed(() => {
  const raw = String(props.lastSavedConfigJson || "").trim();
  if (!raw) return new Map<string, ApiProviderConfigItem>();
  try {
    const parsed = JSON.parse(raw) as { apiProviders?: ApiProviderConfigItem[] };
    return new Map(
      (Array.isArray(parsed.apiProviders) ? parsed.apiProviders : [])
        .map((provider) => [String(provider.id || "").trim(), cloneProvider(provider)] as const)
        .filter(([id]) => !!id),
    );
  } catch {
    return new Map<string, ApiProviderConfigItem>();
  }
});
const currentProviderDirty = computed(() => {
  const provider = selectedProvider.value;
  if (!provider) return false;
  const savedProvider = savedProviderMap.value.get(String(provider.id || "").trim());
  if (!savedProvider) return true;
  // 草稿态比较：用草稿拆分结果构造 provider 视图，与保存后落盘内容一致
  const draftView = {
    ...provider,
    models: splitDraftGroups(provider, draftModelGroups.value, () => `api-model-${buildProviderSeed()}`),
  };
  return JSON.stringify(normalizeProviderForCompare(draftView)) !== JSON.stringify(normalizeProviderForCompare(savedProvider));
});

function isGoogleModelAdapter(adapter: string | undefined): boolean {
  return String(adapter || "").trim().toLowerCase() === "gemini";
}

function showGeminiReasoningEffort(modelCard: ApiModelConfigItem): boolean {
  if (selectedProtocol.value === "gemini") return true;
  return selectedProtocol.value === "auto" && isGoogleModelAdapter(resolvedAdapterByModelId.value[modelCard.id]);
}

function geminiReasoningEffortValue(modelCard: ApiModelConfigItem): string {
  return String(modelCard.reasoningEffort || "").trim().toLowerCase() === "low" ? "low" : DEFAULT_GEMINI_REASONING_EFFORT;
}

function setGeminiReasoningEffort(modelCard: ApiModelConfigItem, value: string) {
  modelCard.reasoningEffort = value === "low" ? "low" : DEFAULT_GEMINI_REASONING_EFFORT;
}

function showOpenaiReasoningEffort(modelCard: ApiModelConfigItem): boolean {
  if (selectedProtocol.value === "openai" || selectedProtocol.value === "openai_responses") return true;
  return selectedProtocol.value === "auto" && isOpenaiModelAdapter(resolvedAdapterByModelId.value[modelCard.id]);
}

function showDeepSeekReasoningEffort(modelCard: ApiModelConfigItem): boolean {
  if (selectedProtocol.value === "deepseek") return true;
  return selectedProtocol.value === "auto" && isDeepSeekModelAdapter(resolvedAdapterByModelId.value[modelCard.id]);
}

function isOpenaiModelAdapter(adapter: string | undefined): boolean {
  const normalized = String(adapter || "").trim().toLowerCase().replace(/[\s_-]/g, "");
  return normalized === "openai" || normalized === "openairesp" || normalized === "openairesponses";
}

function isDeepSeekModelAdapter(adapter: string | undefined): boolean {
  return String(adapter || "").trim().toLowerCase() === "deepseek";
}

function resolveAdapterLabelForModelName(modelName: string): Promise<string> {
  return invokeTauri<string>("resolve_model_adapter_kind", {
    modelName,
    baseUrl: selectedProvider.value?.baseUrl || "",
    requestFormat: selectedProtocol.value,
  });
}

async function refreshResolvedAdaptersForSelectedProvider() {
  const provider = selectedProvider.value;
  if (!provider || selectedProtocol.value !== "auto") {
    adapterResolveRequestSeq.value += 1;
    return;
  }
  const requestSeq = ++adapterResolveRequestSeq.value;
  const models = draftModelGroups.value.map((group) => group.primary);
  const pairs = await Promise.all(models.map(async (model) => {
    const modelName = String(model.model || "").trim();
    if (!modelName) return [model.id, ""] as const;
    try {
      const adapter = await resolveAdapterLabelForModelName(modelName);
      return [model.id, adapter] as const;
    } catch (error) {
      console.warn("[API] 匹配模型协议失败:", { modelId: model.id, modelName, error });
      return [model.id, ""] as const;
    }
  }));
  if (requestSeq !== adapterResolveRequestSeq.value || selectedProtocol.value !== "auto") return;
  resolvedAdapterByModelId.value = {
    ...resolvedAdapterByModelId.value,
    ...Object.fromEntries(pairs),
  };
}

function openaiReasoningEffortValue(modelCard: ApiModelConfigItem): string {
  return openaiReasoningEffortOptions.value.some((item) => item.value === String(modelCard.reasoningEffort || "").trim().toLowerCase())
    ? String(modelCard.reasoningEffort || "").trim().toLowerCase()
    : DEFAULT_OPENAI_REASONING_EFFORT;
}

function setOpenaiReasoningEffort(modelCard: ApiModelConfigItem, value: string) {
  modelCard.reasoningEffort = openaiReasoningEffortOptions.value.some((item) => item.value === value) ? value : DEFAULT_OPENAI_REASONING_EFFORT;
}

function deepseekReasoningEffortValue(modelCard: ApiModelConfigItem): string {
  return deepseekReasoningEffortOptions.value.some((item) => item.value === String(modelCard.reasoningEffort || "").trim().toLowerCase())
    ? String(modelCard.reasoningEffort || "").trim().toLowerCase()
    : DEFAULT_DEEPSEEK_REASONING_EFFORT;
}

function setDeepSeekReasoningEffort(modelCard: ApiModelConfigItem, value: string) {
  modelCard.reasoningEffort = deepseekReasoningEffortOptions.value.some((item) => item.value === value) ? value : DEFAULT_DEEPSEEK_REASONING_EFFORT;
}

function reasoningCapability(group: DraftModelGroup): ModelCapabilityLimits | undefined {
  const primaryId = group.primary.id;
  const capability = modelCapabilityById.value[primaryId];
  if (capability) return capability;
  for (const cardId of group.variantIdByEffort.values()) {
    const item = modelCapabilityById.value[cardId];
    if (item) return item;
  }
  return undefined;
}

function configuredReasoningEffortValues(group: DraftModelGroup): string[] {
  return group.reasoningEfforts;
}

function reasoningCapabilityStatus(group: DraftModelGroup): "known" | "unknown" | "unsupported" {
  const capability = reasoningCapability(group);
  if (!capability || capability.metadataFound !== true) return "unknown";
  if (capability.reasoning?.supportsReasoning === false) return "unsupported";
  const explicitOptions = capability.reasoning?.reasoningEffortOptions?.some((value) => {
    const normalized = String(value || "").trim().toLowerCase();
    return normalized && normalized !== "default";
  });
  return explicitOptions ? "known" : "unknown";
}

function reasoningEffortSupportSet(group: DraftModelGroup): Set<string> | null {
  const capability = reasoningCapability(group);
  if (!capability || capability.metadataFound !== true) return null;
  if (capability.reasoning?.supportsReasoning === false) return new Set();
  const options = (capability.reasoning?.reasoningEffortOptions || [])
    .map((value) => String(value || "").trim().toLowerCase())
    .filter(Boolean);
  if (!options.some((value) => value !== "default")) return null;
  return new Set(options);
}

function reasoningEffortItems(group: DraftModelGroup): Array<{ value: string; label: string; disabled: boolean }> {
  const values = sortReasoningEffortValues([
    ...LEGAL_REASONING_EFFORTS,
    ...configuredReasoningEffortValues(group),
  ]);
  const supported = reasoningEffortSupportSet(group);
  return values.map((value) => ({
    value,
    label: reasoningEffortDisplayLabel(value) || value,
    disabled: supported ? !supported.has(value) : false,
  }));
}

function showReasoningEffort(group: DraftModelGroup): boolean {
  if (selectedCapability.value !== "text") return false;
  const capability = reasoningCapability(group);
  const hasConfiguredReasoningEffort = configuredReasoningEffortValues(group).some((value) => value !== "default");
  if (capability?.metadataFound === true && capability.reasoning?.supportsReasoning === false && !hasConfiguredReasoningEffort) {
    return false;
  }
  return true;
}

function groupHasReasoningEffort(group: DraftModelGroup, effort: string): boolean {
  const normalized = String(effort || "").trim().toLowerCase() || "default";
  return group.reasoningEfforts.includes(normalized);
}

function reasoningEffortCheckedValues(group: DraftModelGroup): string[] {
  return reasoningEffortItems(group)
    .filter((item) => groupHasReasoningEffort(group, item.value))
    .map((item) => item.value);
}

function setGroupReasoningEffort(group: DraftModelGroup, effort: string, enabled: boolean) {
  const normalized = String(effort || "").trim().toLowerCase() || "default";
  const selectedOption = reasoningEffortItems(group).find((item) => item.value === normalized);
  if (selectedOption?.disabled) return;
  const hasEffort = group.reasoningEfforts.includes(normalized);
  if (enabled) {
    if (hasEffort) return;
    group.reasoningEfforts.push(normalized);
    if (!group.variantIdByEffort.has(normalized)) {
      group.variantIdByEffort.set(normalized, `api-model-${buildProviderSeed()}`);
    }
    return;
  }
  if (!hasEffort) return;
  if (group.reasoningEfforts.length === 1) {
    if (normalized === "default") return;
    group.reasoningEfforts.length = 0;
    group.reasoningEfforts.push("default");
    return;
  }
  group.reasoningEfforts = group.reasoningEfforts.filter((item) => item !== normalized);
}


function modelDocumentationUrl(group: DraftModelGroup): string {
  const capability = reasoningCapability(group);
  if (capability?.metadataFound === false) return "";
  return String(capability?.documentationUrl || "").trim();
}

async function openModelDocumentation(group: DraftModelGroup) {
  const url = modelDocumentationUrl(group);
  if (!url) return;
  try {
    await openTransportExternalUrl(url);
  } catch (error) {
    console.warn("[API] 打开模型文档失败:", error);
  }
}

function capabilityFromRequestFormat(format: ApiRequestFormat | string): ApiCapability {
  const normalized = String(format || "").trim().toLowerCase();
  if (normalized === "openai_stt" || normalized === "mimo_asr" || normalized === "openai_tts" || normalized === "stt" || normalized === "tts") {
    return "voice";
  }
  if (normalized === "openai_rerank" || normalized === "rerank") {
    return "rerank";
  }
  if (normalized === "openai_embedding" || normalized === "gemini_embedding" || normalized === "embedding") {
    return "embedding";
  }
  if (isTextRequestFormat(normalized)) {
    return "text";
  }
  return "text";
}

function decodeProviderConcurrentLimit(provider: ApiProviderConfigItem): number {
  if (!provider.allowConcurrentRequests) {
    return 1;
  }
  const raw = Number(provider.maxConcurrentRequests ?? 0);
  if (!Number.isFinite(raw) || raw <= 0) {
    return 0;
  }
  return Math.min(16, Math.max(1, Math.round(raw)));
}

function encodeProviderConcurrentLimit(provider: ApiProviderConfigItem, value: string | number) {
  const parsed = Math.round(Number(value ?? 0));
  const limit = Number.isFinite(parsed) ? Math.min(16, Math.max(0, parsed)) : 0;
  provider.allowConcurrentRequests = true;
  provider.maxConcurrentRequests = limit === 0 ? null : limit;
}

function providerConcurrentLimit(provider: ApiProviderConfigItem): number {
  return decodeProviderConcurrentLimit(provider);
}

function providerConcurrentLimitLabel(provider: ApiProviderConfigItem): string {
  const value = decodeProviderConcurrentLimit(provider);
  if (value === 0) return t("config.api.concurrentUnlimited");
  if (value === 1) return t("config.api.concurrentSerial");
  return String(value);
}

function updateProviderConcurrentLimit(provider: ApiProviderConfigItem, value: string | number) {
  encodeProviderConcurrentLimit(provider, value);
}

function cloneProvider(provider: ApiProviderConfigItem): ApiProviderConfigItem {
  return {
    id: String(provider.id || "").trim(),
    name: String(provider.name || "").trim(),
    requestFormat: normalizeApiRequestFormat(provider.requestFormat),
    allowConcurrentRequests: !!provider.allowConcurrentRequests,
    maxConcurrentRequests: provider.maxConcurrentRequests ?? null,
    enableText: !!provider.enableText,
    enableImage: !!provider.enableImage,
    enableAudio: !!provider.enableAudio,
    enableVideo: !!provider.enableVideo,
    enableTools: provider.enableTools !== false,
    tools: Array.isArray(provider.tools)
      ? provider.tools.map((tool) => ({
        id: String(tool.id || "").trim(),
        command: String(tool.command || "").trim(),
        args: Array.isArray(tool.args) ? [...tool.args] : [],
        enabled: tool.enabled !== false,
        values: { ...(tool.values || {}) },
      }))
      : [],
    baseUrl: String(provider.baseUrl || "").trim(),
    codexAuthMode: (String(provider.codexAuthMode || DEFAULT_CODEX_AUTH_MODE).trim() || DEFAULT_CODEX_AUTH_MODE) as CodexAuthMode,
    codexLocalAuthPath: String(provider.codexLocalAuthPath || DEFAULT_CODEX_LOCAL_AUTH_PATH).trim() || DEFAULT_CODEX_LOCAL_AUTH_PATH,
    codexCustomUrl: String(provider.codexCustomUrl || "").trim() || undefined,
    codexCustomApiKey: String(provider.codexCustomApiKey || "").trim() || undefined,
    codexOriginator: String(provider.codexOriginator || "").trim() || undefined,
    codexResidencyRequirement: String(provider.codexResidencyRequirement || "").trim() || undefined,
    apiKeys: Array.isArray(provider.apiKeys) ? provider.apiKeys.map((value) => String(value || "")) : [],
    keyCursor: Math.max(0, Math.round(Number(provider.keyCursor ?? 0))),
    cachedModelOptions: Array.isArray(provider.cachedModelOptions)
      ? provider.cachedModelOptions.map((value) => String(value || "").trim()).filter(Boolean)
      : [],
    models: Array.isArray(provider.models)
      ? provider.models.map((model) => ({
        id: String(model.id || "").trim(),
        model: String(model.model || "").trim(),
        displayName: String(model.displayName || "").trim(),
        deprecated: !!model.deprecated,
        enableImage: !!model.enableImage,
        enableAudio: !!model.enableAudio,
        enableVideo: !!model.enableVideo,
        enableTools: model.enableTools !== false,
        reasoningEffort: normalizedModelReasoningEffort(provider, model),
        temperature: Number(model.temperature ?? 1),
        customTemperatureEnabled: !!model.customTemperatureEnabled,
        contextWindowTokens: Math.round(Number(model.contextWindowTokens ?? AUTO_CONTEXT_WINDOW_TOKENS)),
        customMaxOutputTokensEnabled: !!model.customMaxOutputTokensEnabled,
        maxOutputTokens: Number(model.maxOutputTokens ?? 4096),
      }))
      : [],
    failureRetryCount: Math.max(0, Math.round(Number(provider.failureRetryCount ?? 0))),
  };
}

function normalizedModelReasoningEffort(_provider: ApiProviderConfigItem, model: ApiModelConfigItem): string {
  return String(model.reasoningEffort || "").trim().toLowerCase() || DEFAULT_REASONING_EFFORT;
}

function normalizeProviderForCompare(provider: ApiProviderConfigItem) {
  return {
    id: String(provider.id || "").trim(),
    name: String(provider.name || "").trim(),
    deprecated: !!provider.deprecated,
    requestFormat: normalizeApiRequestFormat(provider.requestFormat),
    allowConcurrentRequests: !!provider.allowConcurrentRequests,
    maxConcurrentRequests: provider.maxConcurrentRequests ?? null,
    enableText: !!provider.enableText,
    enableImage: !!provider.enableImage,
    enableAudio: !!provider.enableAudio,
    enableVideo: !!provider.enableVideo,
    enableTools: provider.enableTools !== false,
    tools: Array.isArray(provider.tools)
      ? provider.tools.map((tool) => ({
        id: String(tool.id || "").trim(),
        command: String(tool.command || "").trim(),
        args: Array.isArray(tool.args) ? [...tool.args] : [],
        enabled: tool.enabled !== false,
        values: { ...(tool.values || {}) },
      }))
      : [],
    baseUrl: String(provider.baseUrl || "").trim(),
    codexAuthMode: (String(provider.codexAuthMode || DEFAULT_CODEX_AUTH_MODE).trim() || DEFAULT_CODEX_AUTH_MODE) as CodexAuthMode,
    codexLocalAuthPath: String(provider.codexLocalAuthPath || DEFAULT_CODEX_LOCAL_AUTH_PATH).trim() || DEFAULT_CODEX_LOCAL_AUTH_PATH,
    codexCustomUrl: String(provider.codexCustomUrl || "").trim() || undefined,
    codexCustomApiKey: String(provider.codexCustomApiKey || "").trim() || undefined,
    codexOriginator: String(provider.codexOriginator || "").trim() || undefined,
    codexResidencyRequirement: String(provider.codexResidencyRequirement || "").trim() || undefined,
    apiKeys: Array.isArray(provider.apiKeys) ? provider.apiKeys.map((value) => String(value || "")) : [],
    cachedModelOptions: Array.isArray(provider.cachedModelOptions)
      ? provider.cachedModelOptions.map((value) => String(value || "").trim()).filter(Boolean)
      : [],
    models: Array.isArray(provider.models)
      ? provider.models.map((model) => ({
        id: String(model.id || "").trim(),
        model: String(model.model || "").trim(),
        displayName: String(model.displayName || "").trim(),
        deprecated: !!model.deprecated,
        enableImage: !!model.enableImage,
        enableAudio: !!model.enableAudio,
        enableVideo: !!model.enableVideo,
        enableTools: model.enableTools !== false,
        reasoningEffort: normalizedModelReasoningEffort(provider, model),
        temperature: Number(model.temperature ?? 1),
        customTemperatureEnabled: !!model.customTemperatureEnabled,
        contextWindowTokens: Math.round(Number(model.contextWindowTokens ?? AUTO_CONTEXT_WINDOW_TOKENS)),
        customMaxOutputTokensEnabled: !!model.customMaxOutputTokensEnabled,
        maxOutputTokens: Number(model.maxOutputTokens ?? 4096),
      }))
      : [],
    failureRetryCount: Math.max(0, Math.round(Number(provider.failureRetryCount ?? 0))),
  };
}

function buildProviderSeed() {
  return Date.now().toString();
}

function stopCodexAuthPolling() {
  if (codexAuthPollTimer.value !== null) {
    window.clearInterval(codexAuthPollTimer.value);
    codexAuthPollTimer.value = null;
  }
}

function applyProtocolDefaults(provider: ApiProviderConfigItem) {
  provider.requestFormat = normalizeApiRequestFormat(provider.requestFormat);
  const isCodex = provider.requestFormat === "codex";
  const isAnthropic = provider.requestFormat === "anthropic";
  if (isCodex) {
    provider.baseUrl = DEFAULT_CODEX_BASE_URL;
    provider.codexAuthMode = (String(provider.codexAuthMode || DEFAULT_CODEX_AUTH_MODE).trim() || DEFAULT_CODEX_AUTH_MODE) as CodexAuthMode;
    provider.codexLocalAuthPath = String(provider.codexLocalAuthPath || DEFAULT_CODEX_LOCAL_AUTH_PATH).trim() || DEFAULT_CODEX_LOCAL_AUTH_PATH;
    provider.apiKeys = [];
    for (const group of draftModelGroups.value) {
      group.primary.reasoningEffort = String(group.primary.reasoningEffort || DEFAULT_REASONING_EFFORT).trim() || DEFAULT_REASONING_EFFORT;
      group.primary.temperature = 1;
      group.primary.customTemperatureEnabled = false;
      group.primary.contextWindowTokens = AUTO_CONTEXT_WINDOW_TOKENS;
      group.primary.customMaxOutputTokensEnabled = false;
      group.primary.maxOutputTokens = 4096;
    }
    return;
  }
  if (!Array.isArray(provider.apiKeys) || provider.apiKeys.length === 0) {
    provider.apiKeys = [""];
  }
  for (const group of draftModelGroups.value) {
    group.primary.reasoningEffort = String(group.primary.reasoningEffort || DEFAULT_REASONING_EFFORT).trim() || DEFAULT_REASONING_EFFORT;
    group.primary.customMaxOutputTokensEnabled = isAnthropic ? true : !!group.primary.customMaxOutputTokensEnabled;
    group.primary.maxOutputTokens = isAnthropic && Number(group.primary.maxOutputTokens ?? 4096) === 4096
      ? 128000
      : Number(group.primary.maxOutputTokens ?? 4096);
  }
}

function handleCustomMaxOutputTokensToggle(group: DraftModelGroup) {
  const modelCard = group.primary;
  if (!modelCard.customMaxOutputTokensEnabled) return;
  const provider = selectedProvider.value;
  const currentValue = Number(modelCard.maxOutputTokens ?? 4096);
  if (provider?.requestFormat === "anthropic") {
    if (!Number.isFinite(currentValue) || currentValue === 4096) {
      modelCard.maxOutputTokens = 128000;
    }
    return;
  }
  if (!Number.isFinite(currentValue) || currentValue < 8192) {
    modelCard.maxOutputTokens = 8192;
  }
}

function normalizeProviderRequestFormats() {
  for (const provider of providerList.value) {
    const normalized = normalizeApiRequestFormat(provider.requestFormat);
    if (provider.requestFormat !== normalized) {
      provider.requestFormat = normalized;
    }
  }
}

function createModel(seed: string, name = ""): ApiModelConfigItem {
  return {
    id: `api-model-${seed}`,
    model: name,
    enableImage: false,
    enableAudio: false,
    enableVideo: false,
    enableTools: true,
    reasoningEffort: "default",
    temperature: 1,
    customTemperatureEnabled: false,
    contextWindowTokens: AUTO_CONTEXT_WINDOW_TOKENS,
    customMaxOutputTokensEnabled: false,
    maxOutputTokens: 4096,
  };
}

function createProvider(seed: string, capability: ApiCapability = selectedCapability.value): ApiProviderConfigItem {
  const requestFormat = capabilityDefaultProtocol[capability];
  const isCodex = requestFormat === "codex";
  return {
    id: `api-provider-${seed}`,
    name: `API Provider ${providerList.value.length + 1}`,
    requestFormat,
    allowConcurrentRequests: true,
    maxConcurrentRequests: null,
    enableText: capability === "text",
    enableImage: false,
    enableAudio: capability === "voice",
    enableVideo: false,
    enableTools: capability === "text",
    tools: [],
    baseUrl: providerPresets.find((preset) => preset.urls[requestFormat])?.urls[requestFormat] || (isCodex ? DEFAULT_CODEX_BASE_URL : "https://api.openai.com/v1"),
    codexAuthMode: DEFAULT_CODEX_AUTH_MODE,
    codexLocalAuthPath: DEFAULT_CODEX_LOCAL_AUTH_PATH,
    apiKeys: isCodex ? [] : [""],
    keyCursor: 0,
    cachedModelOptions: [],
    models: [createModel(seed)],
    failureRetryCount: 0,
  };
}

function selectProvider(providerId: string) {
  revertUnsavedConfigIfNeeded();
  const provider = providerList.value.find((item) => item.id === providerId);
  const model = firstActiveModel(provider);
  if (!provider || !model) return;
  props.config.selectedApiConfigId = `${provider.id}::${model.id}`;
}

function selectImageProvider(providerId: string) {
  imageGenerationTabRef.value?.selectProvider(providerId);
}

function addImageProvider() {
  imageGenerationTabRef.value?.addProvider();
}

function removeImageProvider() {
  imageGenerationTabRef.value?.removeSelectedProvider();
}

function restoreImageProviderConfig() {
  imageGenerationTabRef.value?.restoreImageConfig();
}

function saveImageProviderConfig() {
  void imageGenerationTabRef.value?.saveImageConfig();
}

function selectModelCard(modelId: string) {
  const provider = selectedProvider.value;
  if (!provider) return;
  props.config.selectedApiConfigId = `${provider.id}::${modelId}`;
}

async function addProvider() {
  const seed = buildProviderSeed();
  const provider = createProvider(seed, selectedCapability.value);
  props.config.apiProviders.push(provider);
  props.config.selectedApiConfigId = `${provider.id}::${provider.models[0].id}`;
}

function firstActiveApiConfigIdExcluding(excludedIds: Set<string>): string {
  for (const provider of props.config.apiProviders || []) {
    if (provider.deprecated) continue;
    for (const model of provider.models || []) {
      if (model.deprecated) continue;
      const providerId = String(provider.id || "").trim();
      const modelId = String(model.id || "").trim();
      const endpointId = providerId && modelId ? `${providerId}::${modelId}` : "";
      if (endpointId && !excludedIds.has(endpointId)) return endpointId;
    }
  }
  return "";
}

function clearRemovedApiConfigReferences(removedIds: string[]) {
  const removedSet = new Set(removedIds.map((id) => String(id || "").trim()).filter(Boolean));
  if (removedSet.size === 0) return;
  for (const department of props.config.departments || []) {
    const nextIds = (Array.isArray(department.apiConfigIds) ? department.apiConfigIds : [])
      .map((id) => String(id || "").trim())
      .filter((id) => !!id && !removedSet.has(id));
    department.apiConfigIds = nextIds;
    if (removedSet.has(String(department.apiConfigId || "").trim())) {
      department.apiConfigId = nextIds[0] || "";
    }
  }
  if (removedSet.has(String(props.config.assistantDepartmentApiConfigId || "").trim())) {
    props.config.assistantDepartmentApiConfigId = "";
  }
  if (removedSet.has(String(props.config.sttApiConfigId || "").trim())) {
    props.config.sttApiConfigId = undefined;
    props.config.sttAutoSend = false;
  }
  if (removedSet.has(String(props.config.visionApiConfigId || "").trim())) {
    props.config.visionApiConfigId = undefined;
  }
  if (removedSet.has(String(props.config.toolReviewApiConfigId || "").trim())) {
    props.config.toolReviewApiConfigId = undefined;
  }
  if (removedSet.has(String(props.config.selectedApiConfigId || "").trim())) {
    props.config.selectedApiConfigId = firstActiveApiConfigIdExcluding(removedSet);
  }
}

function removeProvider(providerId: string) {
  if (scopedProviderList.value.length <= 1) return;
  const provider = props.config.apiProviders.find((item) => item.id === providerId);
  pendingDeleteProviderId.value = providerId;
  pendingDeleteProviderName.value = String(provider?.name || provider?.id || "").trim() || t("config.api.currentProvider");
  providerDeleteDialogOpen.value = true;
}

function closeDeleteProviderDialog() {
  providerDeleteDialogOpen.value = false;
  pendingDeleteProviderId.value = "";
  pendingDeleteProviderName.value = "";
}

async function confirmDeleteProvider() {
  const providerId = String(pendingDeleteProviderId.value || "").trim();
  if (!providerId) {
    closeDeleteProviderDialog();
    return;
  }
  const idx = props.config.apiProviders.findIndex((provider) => provider.id === providerId);
  if (idx < 0) {
    closeDeleteProviderDialog();
    return;
  }
  const target = props.config.apiProviders[idx];
  const removedIds = (target.models || [])
    .map((model) => {
      const modelKey = String(model.id || "").trim();
      return providerId && modelKey ? `${providerId}::${modelKey}` : "";
    })
    .filter(Boolean);
  target.deprecated = true;
  target.models = (target.models || []).map((model) => ({ ...model, deprecated: true }));
  clearRemovedApiConfigReferences(removedIds);
  props.normalizeApiBindingsAction();
  const fallbackProvider = scopedProviderList.value.find((provider) => provider.id !== providerId) ?? activeProviderList.value.find((provider) => provider.id !== providerId) ?? null;
  const fallbackModel = firstActiveModel(fallbackProvider);
  if (fallbackProvider && fallbackModel) {
    props.config.selectedApiConfigId = `${fallbackProvider.id}::${fallbackModel.id}`;
  } else {
    props.config.selectedApiConfigId = "";
  }
  closeDeleteProviderDialog();
  await Promise.resolve(props.saveApiConfigAction());
}

async function switchCapabilityTab(capability: ApiTopTab) {
  revertUnsavedConfigIfNeeded();
  activeTopTab.value = capability;
  if (capability === "imageGeneration") {
    return;
  }
  const nextProvider = activeProviderList.value.find((provider) => capabilityFromRequestFormat(provider.requestFormat) === capability);
  if (nextProvider) {
    selectProvider(nextProvider.id);
    return;
  }
  const seed = buildProviderSeed();
  const provider = createProvider(seed, capability);
  props.config.apiProviders.push(provider);
  props.config.selectedApiConfigId = `${provider.id}::${provider.models[0].id}`;
}

function revertUnsavedConfigIfNeeded() {
  if (!currentProviderDirty.value) return;
  const currentProviderId = String(selectedProvider.value?.id || "").trim();
  if (!currentProviderId) return;
  const providerIndex = props.config.apiProviders.findIndex((provider) => String(provider.id || "").trim() === currentProviderId);
  if (providerIndex < 0) return;
  const savedProvider = savedProviderMap.value.get(currentProviderId);
  if (!savedProvider) {
    props.config.apiProviders.splice(providerIndex, 1);
    return;
  }
  props.config.apiProviders.splice(providerIndex, 1, cloneProvider(savedProvider));
  // config 已还原，草稿需要跟随重建，避免残留旧草稿
  rebuildDraftGroups();
}

function updateSelectedApiKeys(apiKeys: string[]) {
  const provider = selectedProvider.value;
  if (!provider) return;
  provider.apiKeys = apiKeys;
}

function addModelCard() {
  const provider = selectedProvider.value;
  if (!provider) return;
  const seed = buildProviderSeed();
  const model = createModel(seed, "");
  if (provider.requestFormat === "codex") {
    model.model = "gpt-5.5";
  }
  const group: DraftModelGroup = {
    key: modelGroupKey(model),
    primary: model,
    reasoningEfforts: [normalizedModelReasoningEffortFor(model)],
    variantIdByEffort: new Map([[normalizedModelReasoningEffortFor(model), model.id]]),
  };
  draftModelGroups.value.unshift(group);
  props.config.selectedApiConfigId = `${provider.id}::${model.id}`;
}

function removeModelGroup(group: DraftModelGroup) {
  const provider = selectedProvider.value;
  if (!provider || draftModelGroups.value.length <= 1) return;
  draftModelGroups.value = draftModelGroups.value.filter((item) => item !== group);
}

function contextWindowMax(group: DraftModelGroup): number {
  const raw = Number(modelCapabilityById.value[group.primary.id]?.contextWindowMax ?? FALLBACK_CONTEXT_WINDOW_MAX);
  if (!Number.isFinite(raw)) return FALLBACK_CONTEXT_WINDOW_MAX;
  return Math.max(SLIDER_CONTEXT_MIN, Math.min(FALLBACK_CONTEXT_WINDOW_MAX, Math.round(raw)));
}

function autoContextWindowTokens(group: DraftModelGroup): number {
  const contextMax = contextWindowMax(group);
  if (contextMax < AUTO_CONTEXT_WINDOW_SMALL_MODEL_THRESHOLD) {
    return contextMax;
  }
  return Math.min(AUTO_CONTEXT_WINDOW_TOKENS, contextMax);
}

function applyAutoContextWindowTokens(group: DraftModelGroup) {
  group.primary.contextWindowTokens = autoContextWindowTokens(group);
}

function clampModelCardValues(group: DraftModelGroup) {
  const modelCard = group.primary;
  const nextContext = Math.round(Number(modelCard.contextWindowTokens ?? AUTO_CONTEXT_WINDOW_TOKENS));
  const contextMax = contextWindowMax(group);
  const contextMin = Math.min(SLIDER_CONTEXT_MIN, contextMax);
  const clampedContext = Math.max(contextMin, Math.min(contextMax, nextContext));
  if (Number.isFinite(nextContext) && nextContext !== clampedContext) {
    modelCard.contextWindowTokens = clampedContext;
  }

  const nextOutput = Math.round(Number(modelCard.maxOutputTokens ?? 4_096));
  if (!Number.isFinite(nextOutput)) {
    modelCard.maxOutputTokens = 4_096;
  }
}

function clampManualContextWindowValue(group: DraftModelGroup) {
  const modelCard = group.primary;
  const nextContext = Math.round(Number(modelCard.contextWindowTokens ?? AUTO_CONTEXT_WINDOW_TOKENS));
  const clampedContext = Math.max(SLIDER_CONTEXT_MIN, Math.min(FALLBACK_CONTEXT_WINDOW_MAX, nextContext));
  if (!Number.isFinite(nextContext)) {
    modelCard.contextWindowTokens = AUTO_CONTEXT_WINDOW_TOKENS;
    return;
  }
  if (nextContext !== clampedContext) {
    modelCard.contextWindowTokens = clampedContext;
  }
}

function selectModelOption(group: DraftModelGroup, option: string) {
  group.primary.model = option;
  const provider = selectedProvider.value;
  if (provider && !provider.cachedModelOptions.includes(option)) {
    provider.cachedModelOptions.push(option);
  }
  if (provider) {
    applyProtocolDefaults(provider);
  }
  applyAutoContextWindowTokens(group);
  void syncModelMetadata(group);
}

function handleModelCardSyncMetadata(group: DraftModelGroup) {
  void syncModelMetadata(group);
}

async function syncModelMetadata(group: DraftModelGroup) {
  const provider = selectedProvider.value;
  const modelCard = group.primary;
  const model = String(modelCard.model || "").trim();
  if (!provider || !model) return;
  try {
    if (provider.requestFormat === "auto") {
      const adapter = await invokeTauri<string>("resolve_model_adapter_kind", {
        modelName: model,
        baseUrl: provider.baseUrl,
        requestFormat: provider.requestFormat,
      });
      resolvedAdapterByModelId.value = {
        ...resolvedAdapterByModelId.value,
        [modelCard.id]: adapter,
      };
    }
    const metadata = await invokeTauri<FetchModelMetadataResult>("fetch_model_metadata", {
      input: {
        requestFormat: provider.requestFormat,
        model,
        baseUrl: provider.baseUrl,
      },
    });
    const nextCapability: ModelCapabilityLimits = metadata?.found
      ? {
          metadataFound: true,
          fuzzyMatch: metadata.fuzzyMatch === true,
          providerName: String(metadata.providerName || "").trim() || undefined,
          providerApi: String(metadata.providerApi || "").trim() || undefined,
          ...buildModelCapability({
            metadataFound: true,
            contextWindowTokens: metadata.contextWindowTokens,
            maxOutputTokens: metadata.maxOutputTokens,
            enableImage: metadata.enableImage,
            enableVideo: metadata.enableVideo,
            enableAudio: metadata.enableAudio,
            enableTools: metadata.enableTools,
            documentationUrl: metadata.documentationUrl,
            reasoning: metadata.reasoning,
            reasoningEffortOptions: metadata.reasoningEffortOptions,
          }),
        }
      : {
          metadataFound: false,
          ...buildModelCapability({
            metadataFound: false,
          }),
        };
    modelCapabilityById.value = {
      ...modelCapabilityById.value,
      [modelCard.id]: nextCapability,
    };
    applyAutoContextWindowTokens(group);
    clampModelCardValues(group);
    // 元数据已知时，自动移除不支持的思考等级（草稿态只改集合，不触发重组）
    const supported = reasoningEffortSupportSet(group);
    if (supported) {
      const nextEfforts = group.reasoningEfforts.filter((effort) => supported.has(effort));
      if (nextEfforts.length !== group.reasoningEfforts.length) {
        group.reasoningEfforts = nextEfforts.length > 0 ? nextEfforts : ["default"];
      }
    }
  } catch (error) {
    console.warn("[API] 获取模型元数据失败:", error);
  }
}

function resolveCodexProvider(provider?: ApiProviderConfigItem | null): ApiProviderConfigItem | null {
  if (!provider || provider.requestFormat !== "codex") return null;
  return provider;
}

function storeCodexAuthStatus(status: CodexAuthStatus) {
  const providerId = String(status.providerId || "").trim();
  if (!providerId) return;
  codexAuthStatusByProvider.value = {
    ...codexAuthStatusByProvider.value,
    [providerId]: status,
  };
  if (status.authenticated || status.status === "error" || status.status === "expired") {
    stopCodexAuthPolling();
  }
}

function codexAuthFailureStatus(provider: ApiProviderConfigItem, error: unknown): CodexAuthStatus {
  const message = String(error || t('sidebar.apiCodexCheckFailed'));
  const normalized = message.toLowerCase();
  const status = normalized.includes("auth.json")
    || normalized.includes("读取托管 codex 凭证失败")
    || normalized.includes("读取 codex 本地凭证失败")
    ? "unauthenticated"
    : "error";
  return {
    providerId: provider.id,
    authMode: (String(provider.codexAuthMode || DEFAULT_CODEX_AUTH_MODE).trim() || DEFAULT_CODEX_AUTH_MODE) as CodexAuthMode,
    authenticated: false,
    status,
    message,
    email: "",
    accountId: "",
    accessTokenPreview: "",
    localAuthPath: String(provider.codexLocalAuthPath || DEFAULT_CODEX_LOCAL_AUTH_PATH).trim() || DEFAULT_CODEX_LOCAL_AUTH_PATH,
    managedAuthPath: "",
    expiresAt: "",
  };
}

async function refreshCodexAuthStatus(providerArg?: ApiProviderConfigItem | null) {
  const provider = resolveCodexProvider(providerArg ?? selectedProvider.value);
  if (!provider) return null;
  try {
    const status = await invokeTauri<CodexAuthStatus>("codex_get_auth_status", {
      input: {
        providerId: provider.id,
        authMode: provider.codexAuthMode || DEFAULT_CODEX_AUTH_MODE,
        localAuthPath: provider.codexLocalAuthPath || DEFAULT_CODEX_LOCAL_AUTH_PATH,
      },
    });
    storeCodexAuthStatus(status);
    return status;
  } catch (error) {
    const status = codexAuthFailureStatus(provider, error);
    storeCodexAuthStatus(status);
    return status;
  }
}

function startCodexAuthPolling(providerId: string) {
  stopCodexAuthPolling();
  codexAuthPollTimer.value = window.setInterval(() => {
    const provider = providerList.value.find((item) => item.id === providerId) ?? null;
    void refreshCodexAuthStatus(provider);
  }, 2500);
}

async function checkLocalCodexAuth() {
  await refreshCodexAuthStatus();
}

async function startCodexOAuthLogin() {
  const provider = resolveCodexProvider(selectedProvider.value);
  if (!provider) return;
  codexAuthBusy.value = true;
  try {
    const status = await invokeTauri<CodexAuthStatus>("codex_start_oauth_login", {
      input: {
        providerId: provider.id,
      },
    });
    storeCodexAuthStatus(status);
    startCodexAuthPolling(provider.id);
  } catch (error) {
    storeCodexAuthStatus(codexAuthFailureStatus(provider, error));
  } finally {
    codexAuthBusy.value = false;
  }
}

async function logoutCodex() {
  const provider = resolveCodexProvider(selectedProvider.value);
  if (!provider) return;
  codexAuthBusy.value = true;
  try {
    await invokeTauri("codex_logout", {
      input: {
        providerId: provider.id,
      },
    });
    stopCodexAuthPolling();
    storeCodexAuthStatus({
      providerId: provider.id,
      authMode: (String(provider.codexAuthMode || DEFAULT_CODEX_AUTH_MODE).trim() || DEFAULT_CODEX_AUTH_MODE) as CodexAuthMode,
      authenticated: false,
      status: "unauthenticated",
      message: t('sidebar.apiCodexLoggedOut'),
      email: "",
      accountId: "",
      accessTokenPreview: "",
      localAuthPath: String(provider.codexLocalAuthPath || DEFAULT_CODEX_LOCAL_AUTH_PATH).trim() || DEFAULT_CODEX_LOCAL_AUTH_PATH,
      managedAuthPath: "",
      expiresAt: "",
    });
  } catch (error) {
    storeCodexAuthStatus(codexAuthFailureStatus(provider, error));
  } finally {
    codexAuthBusy.value = false;
  }
}

function applyGeneratedBaseUrl(presetId?: string) {
  if (!selectedProvider.value) return;
  if (presetId) {
    selectedPresetId.value = presetId;
  }
  if (!generatedBaseUrl.value) return;
  selectedProvider.value.requestFormat = linkHelperActiveProtocol.value;
  selectedProvider.value.baseUrl = generatedBaseUrl.value;
}

async function openProviderSite(preset: ProviderPreset) {
  if (!preset.docsUrl) return;
  try {
    await openTransportExternalUrl(preset.docsUrl);
  } catch (error) {
    console.warn("[API] 打开供应商文档失败:", error);
  }
}

async function handleSaveApiConfig() {
  const provider = selectedProvider.value;
  if (provider) {
    // 基于草稿组检查空模型，失败时不碰 config 本体
    const hasEmptyModel = draftModelGroups.value.some(
      (group) => !String(group.primary.model || "").trim(),
    );
    if (hasEmptyModel) {
      props.setStatusAction(t("config.api.emptyModelNotAllowed"));
      return;
    }
    // 保存前先把草稿拆分结果写回 config.models
    commitDraftGroups();
    provider.cachedModelOptions = Array.from(new Set(providerModelOptions.value));
  }
  await Promise.resolve(props.saveApiConfigAction());
}

function handleRestoreProviderDraft() {
  revertUnsavedConfigIfNeeded();
}

async function testModelConnection(modelCardId: string) {
  const provider = selectedProvider.value;
  if (!provider) return;
  const modelCard = draftViewModels.value.find((m) => m.id === modelCardId);
  if (!modelCard) return;
  const apiKey = (provider.apiKeys || []).find((k) => k.trim()) ?? "";
  if (!apiKey.trim()) {
    modelConnectionResult.value = {
      ...modelConnectionResult.value,
      [modelCardId]: { success: false, error: "API key is empty" },
    };
    return;
  }
  const modelName = modelCard.model.trim();
  const cap = capabilityFromRequestFormat(provider.requestFormat);
  if ((cap === "embedding" || cap === "rerank") && !modelName) {
    modelConnectionResult.value = {
      ...modelConnectionResult.value,
      [modelCardId]: { success: false, error: "Model name is empty" },
    };
    return;
  }
  modelConnectionTesting.value = { ...modelConnectionTesting.value, [modelCardId]: true };
  modelConnectionResult.value = { ...modelConnectionResult.value, [modelCardId]: undefined as unknown as ModelConnectionResult };
  try {
    const latencyMs = await runProviderConnectionProbe(provider, apiKey, modelName);
    modelConnectionResult.value = {
      ...modelConnectionResult.value,
      [modelCardId]: { success: true, latencyMs },
    };
  } catch (err) {
    modelConnectionResult.value = {
      ...modelConnectionResult.value,
      [modelCardId]: { success: false, error: String(err || "Unknown error") },
    };
  } finally {
    modelConnectionTesting.value = { ...modelConnectionTesting.value, [modelCardId]: false };
  }
}

async function runProviderConnectionProbe(
  provider: ApiProviderConfigItem,
  apiKey: string,
  modelName: string,
): Promise<number> {
  const cap = capabilityFromRequestFormat(provider.requestFormat);
  const started = Date.now();
  if (cap === "voice") {
    const result = await invokeTauri<{ elapsedMs: number }>("test_voice_connection", {
      input: {
        baseUrl: provider.baseUrl.trim(),
        apiKey: apiKey.trim(),
        requestFormat: provider.requestFormat,
      },
    });
    return result.elapsedMs;
  }
  if (cap === "embedding") {
    const result = await invokeTauri<{ vectorDim: number; elapsedMs: number }>("test_embedding_connection", {
      input: {
        baseUrl: provider.baseUrl.trim(),
        apiKey: apiKey.trim(),
        requestFormat: provider.requestFormat,
        model: modelName,
      },
    });
    return result.elapsedMs;
  }
  if (cap === "rerank") {
    const result = await invokeTauri<{ resultCount: number; elapsedMs: number }>("test_rerank_connection", {
      input: {
        baseUrl: provider.baseUrl.trim(),
        apiKey: apiKey.trim(),
        requestFormat: provider.requestFormat,
        model: modelName,
      },
    });
    return result.elapsedMs;
  }
  await invokeTauri<string>("quick_genai_chat", {
    input: {
      baseUrl: provider.baseUrl.trim(),
      apiKey: apiKey.trim(),
      requestFormat: provider.requestFormat,
      model: modelName,
      prompt: "连通性测试，恢复1代表连通",
      providerId: provider.id,
    },
  });
  return Date.now() - started;
}

function maskKeyPreview(key: string): string {
  const trimmed = key.trim();
  if (trimmed.length <= 8) return "*".repeat(trimmed.length);
  return trimmed.slice(0, 4) + "*".repeat(trimmed.length - 8) + trimmed.slice(-4);
}

async function runSingleConnectionTest(apiKey: string): Promise<ConnectionTestResultItem> {
  const provider = selectedProvider.value!;
  const modelCard = draftViewModels.value.find((m) => m.id === connectionTestModelId.value) ?? draftViewModels.value[0];
  const modelName = modelCard?.model.trim() ?? "";
  try {
    const latencyMs = await runProviderConnectionProbe(provider, apiKey, modelName);
    return { keyPreview: maskKeyPreview(apiKey), success: true, latencyMs };
  } catch (err) {
    return { keyPreview: maskKeyPreview(apiKey), success: false, error: String(err || "Unknown error") };
  }
}

async function runConnectionTestFirstKey() {
  const provider = selectedProvider.value;
  if (!provider) return;
  const apiKey = (provider.apiKeys || []).find((k) => k.trim()) ?? "";
  if (!apiKey.trim()) {
    connectionTestResults.value = [{ keyPreview: "-", success: false, error: "API key is empty" }];
    return;
  }
  connectionTestFirstKeyRunning.value = true;
  connectionTestResults.value = [];
  connectionTestKeyStatus.value = {};
  try {
    const result = await runSingleConnectionTest(apiKey);
    connectionTestResults.value = [result];
    connectionTestKeyStatus.value = { [apiKey.trim()]: result.success ? { status: "success", latencyMs: result.latencyMs } : { status: "failed", error: result.error } };
  } finally {
    connectionTestFirstKeyRunning.value = false;
  }
}

async function runConnectionTestAllKeys() {
  const provider = selectedProvider.value;
  if (!provider) return;
  const keys = (provider.apiKeys || []).filter((k) => k.trim());
  if (keys.length === 0) {
    connectionTestResults.value = [{ keyPreview: "-", success: false, error: "API key is empty" }];
    return;
  }
  connectionTestAllKeysRunning.value = true;
  connectionTestResults.value = [];
  connectionTestKeyStatus.value = {};
  try {
    const results: ConnectionTestResultItem[] = [];
    for (const key of keys) {
      const result = await runSingleConnectionTest(key);
      results.push(result);
      connectionTestResults.value = [...results];
      connectionTestKeyStatus.value = { ...connectionTestKeyStatus.value, [key.trim()]: result.success ? { status: "success", latencyMs: result.latencyMs } : { status: "failed", error: result.error } };
    }
  } finally {
    connectionTestAllKeysRunning.value = false;
  }
}

watch(
  selectedProtocol,
  () => {
    linkHelperActiveProtocol.value = defaultLinkHelperProtocol();
  },
  { immediate: true },
);

watch(
  () => providerList.value.map((provider) => provider.requestFormat).join("\0"),
  normalizeProviderRequestFormats,
  { immediate: true },
);

watch(
  () => selectedProvider.value?.id,
  (providerId, previousProviderId) => {
    const provider = selectedProvider.value;
    rebuildDraftGroups();
    if (!providerId || !provider) {
      stopCodexAuthPolling();
      return;
    }
    if (provider.requestFormat === "codex") {
      void refreshCodexAuthStatus(provider);
      return;
    }
    if (previousProviderId && previousProviderId !== providerId) {
      void refreshResolvedAdaptersForSelectedProvider();
    }
    stopCodexAuthPolling();
  },
  { immediate: true },
);

watch(
  () => selectedProvider.value?.id,
  () => {
    const provider = selectedProvider.value;
    connectionTestKeyStatus.value = {};
    modelConnectionResult.value = {};
    const activeModels = draftViewModels.value;
    if (!provider || activeModels.length === 0) {
      connectionTestModelId.value = "";
      connectionTestResults.value = [];
      return;
    }
    if (!activeModels.some((m) => m.id === connectionTestModelId.value)) {
      connectionTestModelId.value = activeModels[0].id;
    }
  },
  { immediate: true },
);

watch(
  () => {
    const provider = selectedProvider.value;
    const activeModels = draftModelGroups.value.map((group) => group.primary);
    return [
      provider?.id || "",
      selectedProtocol.value,
      ...activeModels.map((model) => `${model.id}:${String(model.model || "").trim()}`),
    ].join("\0");
  },
  () => {
    void refreshResolvedAdaptersForSelectedProvider();
  },
  { immediate: true },
);

watch(
  () => {
    const provider = selectedProvider.value;
    const activeModels = draftModelGroups.value.map((group) => group.primary);
    return [
      provider?.id || "",
      provider?.baseUrl || "",
      provider?.requestFormat || "",
      ...activeModels.map((model) => `${model.id}:${String(model.model || "").trim()}`),
    ].join("\0");
  },
  (current, previous) => {
    if (current === previous) return;
    for (const group of draftModelGroups.value) {
      void syncModelMetadata(group);
    }
  },
  { immediate: true },
);

onMounted(() => {
  if (!canUseTransportGenaiChatAdapters()) return;
  void invokeTauri<Array<{ id: string; label: string; supported: boolean }>>("list_genai_chat_adapters")
    .then((adapters) => {
      if (Array.isArray(adapters) && adapters.length > 0) {
        genaiChatAdapters.value = adapters;
      }
    })
    .catch((error) => {
      console.warn("[API] list_genai_chat_adapters failed:", error);
    });
});

onUnmounted(() => {
  stopCodexAuthPolling();
});
</script>

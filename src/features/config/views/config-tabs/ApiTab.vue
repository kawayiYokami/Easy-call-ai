<template>
  <SettingsStickyLayout>
    <template #header>
      <div class="flex flex-col gap-3">
        <div class="join w-full">
          <button v-for="tab in capabilityTabs" :key="tab.id" class="btn btn-sm join-item flex-1" type="button"
            :class="activeCapability === tab.id ? 'btn-primary' : 'bg-base-100'" @click="switchCapabilityTab(tab.id)">
            {{ tab.label }}
          </button>
        </div>

        <div class="flex items-center gap-2">
          <button class="btn btn-sm btn-square btn-primary shrink-0" type="button" :title="t('config.api.addProvider')"
            @click="addProvider()">
            <Plus class="h-4 w-4" />
          </button>
          <button class="btn btn-sm btn-square shrink-0"
            :class="scopedProviderList.length <= 1 ? 'btn-disabled bg-base-200 text-base-content/30' : 'btn-error'"
            type="button" :title="t('config.api.removeProvider')" :disabled="scopedProviderList.length <= 1"
            @click="removeProvider(selectedProviderId)">
            <Trash2 class="h-4 w-4" />
          </button>
          <select :value="selectedProviderId" class="select select-bordered select-md flex-1"
          @change="handleProviderChange($event)">
          <option v-for="provider in scopedProviderList" :key="provider.id" :value="provider.id">
            {{ provider.name || provider.id }}（{{ provider.requestFormat }}）
          </option>
          </select>
          <button
            class="btn btn-sm btn-square shrink-0"
            :class="currentProviderDirty ? 'btn-info' : 'bg-base-200 text-base-content/30 shadow-none'"
            type="button"
            :title="t('config.api.restoreProviderDraft')"
            :disabled="!currentProviderDirty || props.savingConfig"
            @click="handleRestoreProviderDraft"
          >
            <RotateCcw class="h-4 w-4" />
          </button>
          <button class="api-save-btn btn btn-sm btn-square shrink-0 transition-all duration-300"
            :class="currentProviderDirty
              ? 'btn-success api-save-btn--dirty'
              : 'bg-base-200 text-base-content/50 shadow-none'" type="button"
            :title="props.savingConfig ? t('config.api.saving') : currentProviderDirty ? t('config.api.saveConfig') : t('config.api.saved')"
            :disabled="!currentProviderDirty || props.savingConfig" @click="handleSaveApiConfig">
            <Save v-if="!props.savingConfig" class="h-4 w-4" />
            <span v-else class="loading loading-spinner loading-sm"></span>
          </button>
        </div>
      </div>
    </template>

    <div v-if="selectedProvider" class="grid gap-3">
        <div class="card bg-base-100 border border-base-300">
          <div class="card-body gap-3 p-4">
            <div class="flex items-center justify-between gap-2">
              <div class="card-title text-base mb-0">{{ t("config.api.providerSettings") }}</div>
            </div>

            <div class="grid gap-3 md:grid-cols-2">
              <label class="flex flex-col gap-1">
                <span class="text-sm font-medium">{{ t("config.api.configName") }}</span>
                <input v-model="selectedProvider.name" class="input input-bordered input-sm" :placeholder="t('config.api.providerNamePlaceholder')" />
              </label>

              <label class="flex flex-col gap-1">
                <span class="text-sm font-medium">{{ t("config.api.requestFormat") }}</span>
                <select v-model="selectedProvider.requestFormat" class="select select-bordered select-sm"
                  @change="handleRequestFormatChange($event)">
                  <option v-for="item in protocolOptions" :key="item.value" :value="item.value">{{ item.label }}
                  </option>
                </select>
              </label>
            </div>

            <div v-if="!selectedProviderIsCodex" class="flex flex-col gap-1">
              <div class="flex items-center gap-2">
                <span class="text-sm font-medium">{{ t("config.api.baseUrl") }}</span>
                <button class="btn btn-xs bg-base-200" type="button" @click="baseUrlHelperOpen = !baseUrlHelperOpen">
                  <WandSparkles class="h-3 w-3" />
                  <span>{{ t("config.api.linkHelper") }}</span>
                </button>
              </div>
              <input v-model="selectedProvider.baseUrl" class="input input-bordered input-sm"
                :placeholder="props.baseUrlReference" />
              <label class="mt-2 flex flex-col gap-1">
                <div class="flex flex-col gap-1">
                  <span class="text-sm font-medium">{{ t("config.api.allowConcurrentRequests") }}</span>
                </div>
                <div class="flex items-center gap-3">
                  <input
                    :value="providerConcurrentLimit(selectedProvider)"
                    type="range"
                    min="0"
                    max="16"
                    step="1"
                    class="range range-sm flex-1"
                    @input="updateProviderConcurrentLimit(selectedProvider, ($event.target as HTMLInputElement).value)"
                  />
                  <div class="w-16 text-right text-sm">
                    {{ providerConcurrentLimitLabel(selectedProvider) }}
                  </div>
                </div>
              </label>
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
            </div>
          </div>
        </div>

          <div v-if="!selectedProviderIsCodex" class="card bg-base-100 border border-base-300">
            <div class="card-body gap-3 p-4">
              <div class="flex items-center justify-between gap-2">
                <div>
                  <div class="card-title text-base mb-1">{{ t("config.api.apiKeyPool") }}</div>
                  <div class="text-xs opacity-60">{{ t("config.api.apiKeyPoolHint") }}</div>
                </div>
                <button class="btn btn-sm bg-base-200" type="button" @click="addApiKey">
                  <Plus class="h-3.5 w-3.5" />
                  <span>{{ t("config.api.addApiKey") }}</span>
                </button>
              </div>

              <div class="grid gap-2">
                <div v-for="(apiKey, index) in selectedProvider.apiKeys" :key="`key-${selectedProvider.id}-${index}`"
                  class="flex items-center gap-2">
                  <div v-if="connectionTestKeyStatus[selectedProvider.apiKeys[index].trim()]" class="dropdown dropdown-start">
                    <div tabindex="0" role="button" class="cursor-pointer">
                      <span v-if="connectionTestKeyStatus[selectedProvider.apiKeys[index].trim()]?.status === 'success'" class="status status-success"></span>
                      <span v-else class="status status-error"></span>
                    </div>
                    <div tabindex="0" class="dropdown-content card card-sm bg-base-100 border border-base-300 shadow-lg z-10 w-64">
                      <div class="card-body p-3">
                        <p v-if="connectionTestKeyStatus[selectedProvider.apiKeys[index].trim()]?.status === 'success'" class="text-success text-xs">
                          {{ t('config.api.testConnectionSuccess', { latency: connectionTestKeyStatus[selectedProvider.apiKeys[index].trim()]?.latencyMs }) }}
                        </p>
                        <p v-else class="text-error text-xs break-all">
                          {{ connectionTestKeyStatus[selectedProvider.apiKeys[index].trim()]?.error }}
                        </p>
                      </div>
                    </div>
                  </div>
                  <span v-else class="w-4 shrink-0"></span>
                  <input v-model="selectedProvider.apiKeys[index]"
                    :type="showApiKeys[selectedProvider.id]?.[index] ? 'text' : 'password'"
                    class="input input-bordered input-sm flex-1" :placeholder="`API Key #${index + 1}`" />
                  <button class="btn btn-sm btn-square bg-base-200" type="button"
                    :disabled="index === 0"
                    :title="t('config.api.pinApiKeyToTop')"
                    @click="pinApiKeyToTop(index)">
                    <ArrowUpToLine class="h-3.5 w-3.5" />
                  </button>
                  <button class="btn btn-sm btn-square bg-base-200" type="button"
                    @click="toggleApiKeyVisible(selectedProvider.id, index)">
                    <EyeOff v-if="showApiKeys[selectedProvider.id]?.[index]" class="h-3.5 w-3.5" />
                    <Eye v-else class="h-3.5 w-3.5" />
                  </button>
                  <button class="btn btn-sm btn-square bg-base-200 text-error" type="button"
                    :disabled="selectedProvider.apiKeys.length <= 1" @click="removeApiKey(index)">
                    <Trash2 class="h-3.5 w-3.5" />
                  </button>
                </div>
                <div v-if="selectedProvider.apiKeys.length === 0"
                  class="rounded-box border border-dashed border-base-300 px-3 py-3 text-sm opacity-60">
                  {{ t("config.api.noApiKey") }}
                </div>
              </div>
            </div>
          </div>

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

          <div v-if="!selectedProviderIsCodex" class="card bg-base-100 border border-base-300">
            <div class="card-body gap-3 p-4">
              <div class="flex items-center justify-between gap-2">
                <div class="card-title text-base mb-0">{{ t("config.api.connectionTest") }}</div>
              </div>
              <div class="flex items-center gap-2">
                <select v-model="connectionTestModelId" class="select select-bordered select-sm flex-1">
                  <option v-for="m in selectedProvider.models" :key="m.id" :value="m.id">
                    {{ m.model || t('config.api.unnamedModel') }}
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
            </div>
          </div>

          <div v-if="!selectedProviderIsCodex" class="card bg-base-100 border border-base-300">
            <div class="card-body gap-3 p-4">
              <div class="flex items-center justify-between gap-2">
                <div>
                  <div class="card-title text-base mb-1">{{ t("config.api.modelCards") }}</div>
                  <div class="text-xs opacity-60">{{ t("config.api.modelCardsHint") }}</div>
                </div>
                <div class="flex gap-2">
                  <button class="btn btn-sm bg-base-200" type="button" :class="{ loading: props.refreshingModels }"
                    :disabled="props.refreshingModels" @click="$emit('refreshModels')">
                    <RefreshCw class="h-3.5 w-3.5" />
                    <span>{{ t("config.api.refreshModels") }}</span>
                  </button>
                  <button class="btn btn-sm bg-base-200" type="button" @click="addModelCard">
                    <Plus class="h-3.5 w-3.5" />
                    <span>{{ t("config.api.addModel") }}</span>
                  </button>
                </div>
              </div>

              <div
                class="text-[11px]"
                :class="props.modelRefreshError
                  ? 'text-error'
                  : props.modelRefreshOk
                    ? 'text-success'
                    : 'text-transparent'"
              >
                {{ props.modelRefreshError || (props.modelRefreshOk ? t("status.modelListRefreshed", { count: providerModelOptions.length }) : " ") }}
              </div>

              <div class="grid gap-3">
                <div v-for="modelCard in selectedProvider.models" :key="modelCard.id"
                  class="card border border-base-300 bg-base-200/50 transition"
                  :class="selectedModel?.id === modelCard.id ? '' : ''">
                  <div class="card-body gap-3 p-4">
                    <div class="flex items-start justify-between gap-2">
                      <button class="min-w-0 flex-1 text-left" type="button" @click="selectModelCard(modelCard.id)">
                        <div class="card-title text-base mb-1">{{ `${selectedProvider.name ||
                          selectedProvider.id}/${modelCard.model || t("config.api.unnamedModel")}` }}</div>
                      </button>
                      <button class="btn btn-sm btn-square btn-ghost" type="button"
                        :class="selectedProvider.models.length <= 1 ? 'text-base-content/30' : 'text-error'"
                        :disabled="selectedProvider.models.length <= 1" @click="removeModelCard(modelCard.id)">
                        <Trash2 class="h-3.5 w-3.5" />
                      </button>
                    </div>
                    <div class="grid gap-3">
                      <label class="flex flex-col gap-1">
                        <span class="text-sm font-medium">{{ t("config.api.model") }}</span>
                        <div class="join">
                          <input v-model="modelCard.model" class="input input-bordered input-sm join-item flex-1"
                            placeholder="model" @focus="selectModelCard(modelCard.id)"
                            @blur="void syncModelMetadata(modelCard)"
                            @keydown.enter.prevent="void syncModelMetadata(modelCard)" />
                          <button class="btn btn-sm join-item bg-base-300" type="button"
                            :disabled="providerModelOptions.length === 0" @click="openModelPicker(modelCard.id)">
                            <ChevronDown class="h-3.5 w-3.5" />
                          </button>
                        </div>
                        <div v-if="selectedProtocol === 'auto' && resolvedAdapterByModelId[modelCard.id]"
                          class="mt-1 text-xs opacity-70">
                          {{ t("config.api.matchedProtocol", { protocol: resolvedAdapterByModelId[modelCard.id] }) }}
                        </div>
                        <div v-if="shouldWarnDeepSeekKimiProtocol(modelCard)"
                          class="alert alert-warning mt-2 py-2 text-xs">
                          <AlertTriangle class="h-4 w-4 shrink-0" />
                          <span>{{ t("config.api.deepSeekKimiProtocolHint") }}</span>
                        </div>
                      </label>
                      <div v-if="activeModelPickerId === modelCard.id"
                        class="rounded-box border border-base-300 bg-base-200/50 p-3">
                        <input v-model="modelSearch" class="input input-bordered input-sm mb-2 w-full"
                          :placeholder="t('config.api.searchModel')" @keydown.esc.stop.prevent="closeModelPicker" />
                        <div class="max-h-48 overflow-auto">
                          <button v-for="option in filteredModels" :key="`${modelCard.id}-${option}`"
                            class="btn btn-ghost btn-sm mb-1 mr-1" type="button"
                            @click="selectModelOption(modelCard, option)">
                            {{ option }}
                          </button>
                          <div v-if="filteredModels.length === 0" class="px-2 py-3 text-sm opacity-50">{{
                            t("config.api.noModelFound") }}</div>
                        </div>
                      </div>
                    </div>

                    <div v-if="activeCapability === 'text'" class="grid gap-2 md:grid-cols-4">
                      <label
                        class="flex items-center justify-between rounded-box border border-base-300 bg-base-300 px-3 py-2">
                        <span class="text-sm">{{ t("config.api.capImage") }}</span>
                        <input v-model="modelCard.enableImage" type="checkbox" class="checkbox checkbox-sm" />
                      </label>
                      <label
                        class="flex items-center justify-between rounded-box border border-base-300 bg-base-300 px-3 py-2">
                        <span class="text-sm">{{ t("config.api.capTools") }}</span>
                        <input v-model="modelCard.enableTools" type="checkbox" class="checkbox checkbox-sm" />
                      </label>
                      <label
                        class="flex items-center justify-between rounded-box border border-base-300 bg-base-300 px-3 py-2">
                        <span class="text-sm">{{ t("config.api.temperature") }}</span>
                        <input v-model="modelCard.customTemperatureEnabled" type="checkbox" class="checkbox checkbox-sm" />
                      </label>
                      <label
                        class="flex items-center justify-between rounded-box border border-base-300 bg-base-300 px-3 py-2">
                        <span class="text-sm">{{ t("config.api.maxOutputTokens") }}</span>
                        <input v-model="modelCard.customMaxOutputTokensEnabled" type="checkbox" class="checkbox checkbox-sm" />
                      </label>
                    </div>

                    <div v-if="activeCapability === 'text'" class="grid gap-3">
                      <label class="flex flex-col gap-1">
                        <span class="text-sm font-medium">{{ t("config.api.contextWindow") }}</span>
                        <div class="flex items-center gap-2">
                          <input :value="modelCard.contextWindowTokens"
                            @input="modelCard.contextWindowTokens = Number(($event.target as HTMLInputElement).value)"
                            type="range" :min="SLIDER_CONTEXT_MIN" :max="contextWindowMax(modelCard)" step="1000"
                            class="range range-sm flex-1" />
                          <div class="relative w-28">
                            <input :value="Math.round(Number(modelCard.contextWindowTokens || 0) / 1000)"
                              @input="modelCard.contextWindowTokens = Number(($event.target as HTMLInputElement).value || 0) * 1000"
                              @blur="clampManualContextWindowValue(modelCard)"
                              type="number" :min="Math.round(SLIDER_CONTEXT_MIN / 1000)"
                              :max="2000" step="1"
                              class="input input-bordered input-sm w-full pr-7 text-right font-mono" />
                            <span class="pointer-events-none absolute inset-y-0 right-2 flex items-center text-xs opacity-70">K</span>
                          </div>
                        </div>
                      </label>

                      <label v-if="showGeminiReasoningEffort(modelCard)" class="flex flex-col gap-1">
                        <span class="text-sm font-medium">{{ t("config.api.googleReasoningEffort") }}</span>
                        <select
                          :value="geminiReasoningEffortValue(modelCard)"
                          class="select select-bordered select-sm"
                          @change="setGeminiReasoningEffort(modelCard, ($event.target as HTMLSelectElement).value)"
                        >
                          <option v-for="item in geminiReasoningEffortOptions" :key="item.value" :value="item.value">
                            {{ item.label }}
                          </option>
                        </select>
                      </label>

                      <label v-if="showOpenaiReasoningEffort(modelCard)" class="flex flex-col gap-1">
                        <span class="text-sm font-medium">{{ t("config.api.reasoningEffort") }}</span>
                        <select
                          :value="openaiReasoningEffortValue(modelCard)"
                          class="select select-bordered select-sm"
                          @change="setOpenaiReasoningEffort(modelCard, ($event.target as HTMLSelectElement).value)"
                        >
                          <option v-for="item in openaiReasoningEffortOptions" :key="item.value" :value="item.value">
                            {{ item.label }}
                          </option>
                        </select>
                      </label>

                      <label v-if="showDeepSeekReasoningEffort(modelCard)" class="flex flex-col gap-1">
                        <span class="text-sm font-medium">{{ t("config.api.reasoningEffort") }}</span>
                        <select
                          :value="deepseekReasoningEffortValue(modelCard)"
                          class="select select-bordered select-sm"
                          @change="setDeepSeekReasoningEffort(modelCard, ($event.target as HTMLSelectElement).value)"
                        >
                          <option v-for="item in deepseekReasoningEffortOptions" :key="item.value" :value="item.value">
                            {{ item.label }}
                          </option>
                        </select>
                      </label>

                      <label v-if="modelCard.customTemperatureEnabled" class="flex flex-col gap-1">
                        <span class="text-sm font-medium">{{ t("config.api.temperature") }}</span>
                        <div class="flex items-center gap-2">
                          <input :value="modelCard.temperature"
                            @input="modelCard.temperature = Number(($event.target as HTMLInputElement).value)"
                            type="range" min="0" max="2" step="0.1" class="range range-sm flex-1" />
                          <span class="text-xs font-mono w-8 text-right">{{ modelCard.temperature.toFixed(1) }}</span>
                        </div>
                      </label>

                      <label v-if="modelCard.customMaxOutputTokensEnabled" class="flex flex-col gap-1">
                        <span class="text-sm font-medium">{{ t("config.api.maxOutputTokens") }}</span>
                        <div class="flex items-center gap-2">
                          <input :value="modelCard.maxOutputTokens"
                            @input="modelCard.maxOutputTokens = Number(($event.target as HTMLInputElement).value)"
                            type="range" min="256" :max="maxOutputTokensMax(modelCard)" step="256"
                            class="range range-sm flex-1" />
                          <span class="text-xs font-mono w-24 text-right">{{
                            Number(modelCard.maxOutputTokens).toLocaleString() }}</span>
                        </div>
                      </label>
                    </div>

                    <div v-if="modelConnectionResult[modelCard.id]" class="rounded-box border px-3 py-2 text-xs"
                      :class="modelConnectionResult[modelCard.id]?.success ? 'border-success/30 text-success' : 'border-error/30 text-error'">
                      {{ modelConnectionResult[modelCard.id]?.success
                        ? t('config.api.testConnectionSuccess', { latency: modelConnectionResult[modelCard.id]?.latencyMs })
                        : t('config.api.testConnectionFailed', { error: modelConnectionResult[modelCard.id]?.error }) }}
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
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
import { computed, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { AlertTriangle, ArrowUpToLine, ChevronDown, ExternalLink, Eye, EyeOff, Plus, RefreshCw, RotateCcw, Save, Trash2, WandSparkles } from "@lucide/vue";
import type { ApiModelConfigItem, ApiProviderConfigItem, ApiRequestFormat, AppConfig, CodexAuthMode, CodexAuthStatus } from "../../../../types/app";
import SettingsStickyLayout from "../../components/SettingsStickyLayout.vue";
import { invokeTauri } from "../../../../services/tauri-api";
import CodexProviderPanel from "./CodexProviderPanel.vue";
import { normalizeApiRequestFormat } from "../../utils/api-request-format";

type ApiCapability = "text" | "voice" | "embedding";
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
  matchedModelId?: string | null;
  contextWindowTokens?: number | null;
  maxOutputTokens?: number | null;
  enableImage?: boolean | null;
  enableTools?: boolean | null;
  enableAudio?: boolean | null;
};
type ModelCapabilityLimits = {
  contextWindowMax?: number;
  maxOutputTokensMax?: number;
};

const SLIDER_CONTEXT_MIN = 16_000;
const DEFAULT_CODEX_BASE_URL = "https://chatgpt.com/backend-api/codex";
const DEFAULT_CODEX_AUTH_MODE: CodexAuthMode = "read_local";
const DEFAULT_CODEX_LOCAL_AUTH_PATH = "~/.codex/auth.json";
const DEFAULT_REASONING_EFFORT = "medium";
const DEFAULT_GEMINI_REASONING_EFFORT = "high";
const DEFAULT_OPENAI_REASONING_EFFORT = "high";
const DEFAULT_DEEPSEEK_REASONING_EFFORT = "high";
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
}>();

const emit = defineEmits<{
  (e: "refreshModels"): void;
}>();

const { t } = useI18n();
const openaiReasoningEffortOptions = computed(() => [
  { value: "low", label: t("config.api.reasoningLow") },
  { value: "medium", label: t("config.api.reasoningMedium") },
  { value: "high", label: t("config.api.reasoningHigh") },
  { value: "xhigh", label: t("config.api.reasoningXHigh") },
]);
const deepseekReasoningEffortOptions = computed(() => [
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
const activeModelPickerId = ref("");
const modelSearch = ref("");
const providerDeleteDialogOpen = ref(false);
const pendingDeleteProviderId = ref("");
const pendingDeleteProviderName = ref("");
const showApiKeys = ref<Record<string, Record<number, boolean>>>({});
const modelCapabilityById = ref<Record<string, ModelCapabilityLimits>>({});
const resolvedAdapterByModelId = ref<Record<string, string>>({});
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
const capabilityTabs = computed<Array<{ id: ApiCapability; label: string }>>(() => [
  { id: "text", label: t("config.api.capabilityText") },
  { id: "voice", label: t("config.api.capabilityVoice") },
  { id: "embedding", label: t("config.api.capabilityEmbedding") },
]);
const protocolOptionsByCapability: Record<ApiCapability, ProtocolOption[]> = {
  text: [
    { value: "auto", label: "Auto" },
    { value: "openai", label: "OpenAI Compatible" },
    { value: "deepseek", label: "DeepSeek" },
    { value: "openai_responses", label: "OpenAI Responses" },
    { value: "codex", label: "OpenAI Codex" },
    { value: "gemini", label: "Google Gemini" },
    { value: "anthropic", label: "Anthropic" },
    { value: "fireworks", label: "Fireworks" },
    { value: "together", label: "Together AI" },
    { value: "groq", label: "Groq" },
    { value: "mimo", label: "Mimo" },
    { value: "moonshot", label: "Moonshot/Kimi" },
    { value: "nebius", label: "Nebius" },
    { value: "xai", label: "xAI" },
    { value: "zai", label: "Zai" },
    { value: "bigmodel", label: "BigModel" },
    { value: "aliyun", label: "Aliyun" },
    { value: "baidu", label: "Baidu" },
    { value: "cohere", label: "Cohere" },
    { value: "ollama", label: "Ollama" },
    { value: "ollama_cloud", label: "Ollama Cloud" },
    { value: "vertex", label: "Google Vertex AI" },
    { value: "github_copilot", label: "GitHub Copilot" },
    { value: "opencode_go", label: "OpenCode Go" },
    { value: "bedrock_api", label: "AWS Bedrock API" },
  ],
  voice: [
    { value: "openai_stt", label: "OpenAI STT" },
    { value: "openai_tts", label: "OpenAI TTS" },
  ],
  embedding: [
    { value: "openai_embedding", label: "OpenAI Embedding" },
    { value: "gemini_embedding", label: "Gemini Embedding" },
    { value: "openai_rerank", label: "OpenAI Rerank" },
  ],
};
const capabilityDefaultProtocol: Record<ApiCapability, ApiRequestFormat> = {
  text: "auto",
  voice: "openai_stt",
  embedding: "openai_embedding",
};

const providerPresets: ProviderPreset[] = [
  { id: "openai-official", name: "OpenAI", category: "official", urls: { auto: "https://api.openai.com/v1", openai: "https://api.openai.com/v1", openai_responses: "https://api.openai.com/v1", openai_stt: "https://api.openai.com/v1", openai_tts: "https://api.openai.com/v1/audio/speech", openai_embedding: "https://api.openai.com/v1", openai_rerank: "https://api.openai.com/v1" }, docsUrl: "https://platform.openai.com/docs/overview" },
  { id: "openai-codex", name: "OpenAI Codex", category: "official", urls: { codex: DEFAULT_CODEX_BASE_URL }, docsUrl: "https://chatgpt.com" },
  { id: "anthropic-official", name: "Anthropic", category: "official", urls: { anthropic: "https://api.anthropic.com" }, docsUrl: "https://docs.anthropic.com/en/api/overview" },
  { id: "google-gemini", name: "Google Gemini", category: "official", urls: { gemini: "https://generativelanguage.googleapis.com", gemini_embedding: "https://generativelanguage.googleapis.com" }, docsUrl: "https://ai.google.dev/gemini-api/docs", hasFreeQuota: true },
  { id: "deepseek", name: "DeepSeek", category: "domestic", urls: { auto: "https://api.deepseek.com/v1", deepseek: "https://api.deepseek.com/v1", anthropic: "https://api.deepseek.com/anthropic", openai: "https://api.deepseek.com/v1", openai_responses: "https://api.deepseek.com/v1" }, docsUrl: "https://api-docs.deepseek.com/" },
  { id: "moonshot-kimi", name: "Moonshot/Kimi", category: "domestic", urls: { auto: "https://api.moonshot.cn/v1", moonshot: "https://api.moonshot.cn/v1", anthropic: "https://api.kimi.com/coding/", openai: "https://api.moonshot.cn/v1", openai_responses: "https://api.moonshot.cn/v1" }, docsUrl: "https://platform.moonshot.cn/docs/api-reference" },
  { id: "aliyun-bailian-coding", name: "百炼编程", category: "domestic", urls: { anthropic: "https://coding.dashscope.aliyuncs.com/apps/anthropic/v1", openai: "https://coding.dashscope.aliyuncs.com/v1", openai_responses: "https://coding.dashscope.aliyuncs.com/v1" }, docsUrl: "https://help.aliyun.com/zh/model-studio/" },
  { id: "aliyun-bailian", name: "百炼通用", category: "domestic", urls: { auto: "https://dashscope.aliyuncs.com/compatible-mode/v1", openai: "https://dashscope.aliyuncs.com/compatible-mode/v1", openai_responses: "https://dashscope.aliyuncs.com/compatible-mode/v1" }, docsUrl: "https://help.aliyun.com/zh/model-studio/" },
  { id: "baidu-qianfan", name: "百度千帆", category: "domestic", urls: { baidu: "https://qianfan.baidubce.com/v2", openai: "https://qianfan.baidubce.com/v2", openai_responses: "https://qianfan.baidubce.com/v2" }, docsUrl: "https://cloud.baidu.com/doc/WENXINWORKSHOP/index.html" },
  { id: "zhipu-glm", name: "Zhipu GLM", category: "domestic", urls: { anthropic: "https://open.bigmodel.cn/api/anthropic", openai: "https://open.bigmodel.cn/api/paas/v4", openai_responses: "https://open.bigmodel.cn/api/paas/v4" }, docsUrl: "https://open.bigmodel.cn/dev/api", hasFreeQuota: true },
  { id: "minimax", name: "MiniMax", category: "domestic", urls: { anthropic: "https://api.minimaxi.com/anthropic", openai: "https://api.minimaxi.com/v1", openai_responses: "https://api.minimaxi.com/v1" }, docsUrl: "https://www.minimax.io/platform/document" },
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
  { value: "low", label: "低" },
  { value: "medium", label: "中" },
  { value: "high", label: "高" },
  { value: "xhigh", label: "极高" },
];
const codexAuthModeOptions: Array<{ value: CodexAuthMode; label: string }> = [
  { value: "read_local", label: "读取本地" },
  { value: "managed_oauth", label: "自行登录" },
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

const providerList = computed(() => props.config.apiProviders || []);
const selectedProviderId = computed(() => {
  const [providerId] = String(props.config.selectedApiConfigId || "").split("::");
  return providerId || providerList.value[0]?.id || "";
});

const selectedProvider = computed(() => {
  const [providerId] = String(props.config.selectedApiConfigId || "").split("::");
  return providerList.value.find((provider) => provider.id === providerId) ?? providerList.value[0] ?? null;
});
const activeCapability = computed<ApiCapability>(() => capabilityFromRequestFormat(selectedProvider.value?.requestFormat || "openai"));
const scopedProviderList = computed(() =>
  providerList.value.filter((provider) => capabilityFromRequestFormat(provider.requestFormat) === activeCapability.value),
);
const protocolOptions = computed(() =>
  protocolOptionsByCapability[activeCapability.value].map((option) =>
    option.value === "auto"
      ? { ...option, label: t("config.api.protocolAuto") }
      : option,
  ),
);

const selectedModel = computed(() => {
  const [, modelId] = String(props.config.selectedApiConfigId || "").split("::");
  const provider = selectedProvider.value;
  if (!provider) return null;
  return provider.models.find((model) => model.id === modelId) ?? provider.models[0] ?? null;
});

const selectedProtocol = computed<ApiRequestFormat>(() => canonicalRequestFormat(selectedProvider.value?.requestFormat || "openai"));
const selectedProviderIsCodex = computed(() => selectedProtocol.value === "codex");
const currentCodexAuthStatus = computed(() => {
  const providerId = String(selectedProvider.value?.id || "").trim();
  return providerId ? codexAuthStatusByProvider.value[providerId] ?? null : null;
});

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

const filteredModels = computed(() => {
  const search = modelSearch.value.trim().toLowerCase();
  if (!search) return providerModelOptions.value;
  return providerModelOptions.value.filter((item) => item.toLowerCase().includes(search));
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
  return JSON.stringify(normalizeProviderForCompare(provider)) !== JSON.stringify(normalizeProviderForCompare(savedProvider));
});

function normalizeGeminiReasoningEffort(model: ApiModelConfigItem) {
  if (!["low", "high"].includes(String(model.reasoningEffort || "").trim().toLowerCase())) {
    model.reasoningEffort = DEFAULT_GEMINI_REASONING_EFFORT;
  }
}

function normalizeOpenaiReasoningEffort(model: ApiModelConfigItem) {
  if (!openaiReasoningEffortOptions.value.some((item) => item.value === String(model.reasoningEffort || "").trim().toLowerCase())) {
    model.reasoningEffort = DEFAULT_OPENAI_REASONING_EFFORT;
  }
}

function normalizeDeepSeekReasoningEffort(model: ApiModelConfigItem) {
  if (!deepseekReasoningEffortOptions.value.some((item) => item.value === String(model.reasoningEffort || "").trim().toLowerCase())) {
    model.reasoningEffort = DEFAULT_DEEPSEEK_REASONING_EFFORT;
  }
}

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

function capabilityFromRequestFormat(format: ApiRequestFormat | string): ApiCapability {
  const normalized = String(format || "").trim().toLowerCase();
  if (normalized === "openai_stt" || normalized === "openai_tts" || normalized === "stt" || normalized === "tts") {
    return "voice";
  }
  if (
    normalized === "openai_embedding"
    || normalized === "gemini_embedding"
    || normalized === "openai_rerank"
    || normalized === "embedding"
    || normalized === "rerank"
  ) {
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
    apiKeys: Array.isArray(provider.apiKeys) ? provider.apiKeys.map((value) => String(value || "")) : [],
    keyCursor: Math.max(0, Math.round(Number(provider.keyCursor ?? 0))),
    cachedModelOptions: Array.isArray(provider.cachedModelOptions)
      ? provider.cachedModelOptions.map((value) => String(value || "").trim()).filter(Boolean)
      : [],
    models: Array.isArray(provider.models)
      ? provider.models.map((model) => ({
        id: String(model.id || "").trim(),
        model: String(model.model || "").trim(),
        enableImage: !!model.enableImage,
        enableTools: model.enableTools !== false,
        reasoningEffort: normalizedModelReasoningEffort(provider, model),
        temperature: Number(model.temperature ?? 1),
        customTemperatureEnabled: !!model.customTemperatureEnabled,
        contextWindowTokens: Math.round(Number(model.contextWindowTokens ?? 128000)),
        customMaxOutputTokensEnabled: !!model.customMaxOutputTokensEnabled,
        maxOutputTokens: Number(model.maxOutputTokens ?? 4096),
      }))
      : [],
    failureRetryCount: Math.max(0, Math.round(Number(provider.failureRetryCount ?? 0))),
  };
}

function normalizedModelReasoningEffort(provider: ApiProviderConfigItem, model: ApiModelConfigItem): string {
  const value = String(model.reasoningEffort || "").trim().toLowerCase();
  if (provider.requestFormat === "gemini") {
    return value === "low" ? "low" : DEFAULT_GEMINI_REASONING_EFFORT;
  }
  if (provider.requestFormat === "deepseek") {
    return deepseekReasoningEffortOptions.value.some((item) => item.value === value) ? value : DEFAULT_DEEPSEEK_REASONING_EFFORT;
  }
  if (provider.requestFormat === "openai" || provider.requestFormat === "openai_responses") {
    return openaiReasoningEffortOptions.value.some((item) => item.value === value) ? value : DEFAULT_OPENAI_REASONING_EFFORT;
  }
  return value || DEFAULT_REASONING_EFFORT;
}

function normalizeProviderForCompare(provider: ApiProviderConfigItem) {
  return {
    id: String(provider.id || "").trim(),
    name: String(provider.name || "").trim(),
    requestFormat: normalizeApiRequestFormat(provider.requestFormat),
    allowConcurrentRequests: !!provider.allowConcurrentRequests,
    maxConcurrentRequests: provider.maxConcurrentRequests ?? null,
    enableText: !!provider.enableText,
    enableImage: !!provider.enableImage,
    enableAudio: !!provider.enableAudio,
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
    apiKeys: Array.isArray(provider.apiKeys) ? provider.apiKeys.map((value) => String(value || "")) : [],
    cachedModelOptions: Array.isArray(provider.cachedModelOptions)
      ? provider.cachedModelOptions.map((value) => String(value || "").trim()).filter(Boolean)
      : [],
    models: Array.isArray(provider.models)
      ? provider.models.map((model) => ({
        id: String(model.id || "").trim(),
        model: String(model.model || "").trim(),
        enableImage: !!model.enableImage,
        enableTools: model.enableTools !== false,
        reasoningEffort: normalizedModelReasoningEffort(provider, model),
        temperature: Number(model.temperature ?? 1),
        customTemperatureEnabled: !!model.customTemperatureEnabled,
        contextWindowTokens: Math.round(Number(model.contextWindowTokens ?? 128000)),
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
  if (isCodex) {
    provider.baseUrl = DEFAULT_CODEX_BASE_URL;
    provider.codexAuthMode = (String(provider.codexAuthMode || DEFAULT_CODEX_AUTH_MODE).trim() || DEFAULT_CODEX_AUTH_MODE) as CodexAuthMode;
    provider.codexLocalAuthPath = String(provider.codexLocalAuthPath || DEFAULT_CODEX_LOCAL_AUTH_PATH).trim() || DEFAULT_CODEX_LOCAL_AUTH_PATH;
    provider.apiKeys = [];
    provider.models = (provider.models || []).map((model) => ({
      ...model,
      reasoningEffort: String(model.reasoningEffort || DEFAULT_REASONING_EFFORT).trim() || DEFAULT_REASONING_EFFORT,
      temperature: 1,
      customTemperatureEnabled: false,
      contextWindowTokens: 128000,
      customMaxOutputTokensEnabled: false,
      maxOutputTokens: 4096,
    }));
    return;
  }
  if (!Array.isArray(provider.apiKeys) || provider.apiKeys.length === 0) {
    provider.apiKeys = [""];
  }
  provider.models = (provider.models || []).map((model) => ({
    ...model,
    reasoningEffort: String(model.reasoningEffort || DEFAULT_REASONING_EFFORT).trim() || DEFAULT_REASONING_EFFORT,
  }));
}

function normalizeProviderRequestFormats() {
  for (const provider of providerList.value) {
    const normalized = normalizeApiRequestFormat(provider.requestFormat);
    if (provider.requestFormat !== normalized) {
      provider.requestFormat = normalized;
    }
  }
}

function createModel(seed: string, name = "gpt-4o-mini"): ApiModelConfigItem {
  return {
    id: `api-model-${seed}`,
    model: name,
    enableImage: false,
    enableTools: true,
    reasoningEffort: DEFAULT_REASONING_EFFORT,
    temperature: 1,
    customTemperatureEnabled: false,
    contextWindowTokens: 128000,
    customMaxOutputTokensEnabled: false,
    maxOutputTokens: 4096,
  };
}

function createProvider(seed: string, capability: ApiCapability = activeCapability.value): ApiProviderConfigItem {
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
    enableTools: capability === "text",
    tools: [],
    baseUrl: providerPresets.find((preset) => preset.urls[requestFormat])?.urls[requestFormat] || (isCodex ? DEFAULT_CODEX_BASE_URL : "https://api.openai.com/v1"),
    codexAuthMode: DEFAULT_CODEX_AUTH_MODE,
    codexLocalAuthPath: DEFAULT_CODEX_LOCAL_AUTH_PATH,
    apiKeys: isCodex ? [] : [""],
    keyCursor: 0,
    cachedModelOptions: isCodex ? ["gpt-5.5"] : ["gpt-4o-mini"],
    models: [createModel(seed, isCodex ? "gpt-5.5" : "gpt-4o-mini")],
    failureRetryCount: 0,
  };
}

function selectProvider(providerId: string) {
  revertUnsavedConfigIfNeeded();
  const provider = providerList.value.find((item) => item.id === providerId);
  const model = provider?.models[0];
  if (!provider || !model) return;
  props.config.selectedApiConfigId = `${provider.id}::${model.id}`;
}

function handleProviderChange(event: Event) {
  const target = event.target as HTMLSelectElement;
  const providerId = target.value;
  selectProvider(providerId);
}

function handleRequestFormatChange(event: Event) {
  const provider = selectedProvider.value;
  if (!provider) return;
  provider.requestFormat = (event.target as HTMLSelectElement).value as ApiRequestFormat;
  applyProtocolDefaults(provider);
  if (provider.requestFormat !== "codex") {
    stopCodexAuthPolling();
  } else {
    void refreshCodexAuthStatus(provider);
  }
}

function selectModelCard(modelId: string) {
  const provider = selectedProvider.value;
  if (!provider) return;
  props.config.selectedApiConfigId = `${provider.id}::${modelId}`;
}

async function addProvider() {
  const seed = buildProviderSeed();
  const provider = createProvider(seed, activeCapability.value);
  applyProtocolDefaults(provider);
  props.config.apiProviders.push(provider);
  props.config.selectedApiConfigId = `${provider.id}::${provider.models[0].id}`;
  await Promise.resolve(props.saveApiConfigAction());
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
  props.config.apiProviders.splice(idx, 1);
  const fallbackProvider = scopedProviderList.value[Math.max(0, idx - 1)] ?? scopedProviderList.value[0] ?? props.config.apiProviders[0];
  const fallbackModel = fallbackProvider?.models[0];
  if (fallbackProvider && fallbackModel) {
    props.config.selectedApiConfigId = `${fallbackProvider.id}::${fallbackModel.id}`;
  }
  closeDeleteProviderDialog();
  await Promise.resolve(props.saveApiConfigAction());
}

async function switchCapabilityTab(capability: ApiCapability) {
  revertUnsavedConfigIfNeeded();
  const nextProvider = providerList.value.find((provider) => capabilityFromRequestFormat(provider.requestFormat) === capability);
  if (nextProvider) {
    selectProvider(nextProvider.id);
    return;
  }
  const seed = buildProviderSeed();
  const provider = createProvider(seed, capability);
  applyProtocolDefaults(provider);
  props.config.apiProviders.push(provider);
  props.config.selectedApiConfigId = `${provider.id}::${provider.models[0].id}`;
  await Promise.resolve(props.saveApiConfigAction());
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
}

function addApiKey() {
  selectedProvider.value?.apiKeys.push("");
}

function removeApiKey(index: number) {
  const provider = selectedProvider.value;
  if (!provider || provider.apiKeys.length <= 1) return;
  provider.apiKeys.splice(index, 1);
}

function pinApiKeyToTop(index: number) {
  const provider = selectedProvider.value;
  if (!provider || index === 0 || index >= provider.apiKeys.length) return;
  const currentVisibleKeys = showApiKeys.value[provider.id] || {};
  const nextVisibleKeys: Record<number, boolean> = {};
  provider.apiKeys.forEach((_, currentIndex) => {
    const nextIndex = currentIndex === index ? 0 : currentIndex < index ? currentIndex + 1 : currentIndex;
    nextVisibleKeys[nextIndex] = !!currentVisibleKeys[currentIndex];
  });
  const [key] = provider.apiKeys.splice(index, 1);
  provider.apiKeys.unshift(key);
  showApiKeys.value = {
    ...showApiKeys.value,
    [provider.id]: nextVisibleKeys,
  };
}

function toggleApiKeyVisible(providerId: string, index: number) {
  showApiKeys.value = {
    ...showApiKeys.value,
    [providerId]: {
      ...(showApiKeys.value[providerId] || {}),
      [index]: !(showApiKeys.value[providerId]?.[index]),
    },
  };
}

function addModelCard() {
  const provider = selectedProvider.value;
  if (!provider) return;
  const seed = buildProviderSeed();
  const model = createModel(seed, "");
  if (provider.requestFormat === "codex") {
    model.model = "gpt-5.5";
  }
  provider.models.push(model);
  applyProtocolDefaults(provider);
  props.config.selectedApiConfigId = `${provider.id}::${model.id}`;
  activeModelPickerId.value = model.id;
}

function removeModelCard(modelId: string) {
  const provider = selectedProvider.value;
  if (!provider || provider.models.length <= 1) return;
  const idx = provider.models.findIndex((item) => item.id === modelId);
  if (idx < 0) return;
  provider.models.splice(idx, 1);
  const fallback = provider.models[Math.max(0, idx - 1)] ?? provider.models[0];
  if (fallback) {
    props.config.selectedApiConfigId = `${provider.id}::${fallback.id}`;
  }
}

function openModelPicker(modelId: string) {
  activeModelPickerId.value = activeModelPickerId.value === modelId ? "" : modelId;
  modelSearch.value = "";
  selectModelCard(modelId);
}

function closeModelPicker() {
  activeModelPickerId.value = "";
  modelSearch.value = "";
}

function contextWindowMax(modelCard: ApiModelConfigItem): number {
  const raw = Number(modelCapabilityById.value[modelCard.id]?.contextWindowMax ?? 2_000_000);
  if (!Number.isFinite(raw)) return 2_000_000;
  return Math.max(SLIDER_CONTEXT_MIN, Math.min(2_000_000, Math.round(raw)));
}

function maxOutputTokensMax(modelCard: ApiModelConfigItem): number {
  const raw = Number(modelCapabilityById.value[modelCard.id]?.maxOutputTokensMax ?? 128_000);
  if (!Number.isFinite(raw)) return 128_000;
  return Math.max(256, Math.min(128_000, Math.round(raw)));
}

function shouldWarnDeepSeekKimiProtocol(modelCard: ApiModelConfigItem): boolean {
  if (selectedProtocol.value === "auto" || selectedProtocol.value === "deepseek") return false;
  const modelName = String(modelCard.model || "").toLowerCase();
  return modelName.includes("deepseek") || modelName.includes("kimi");
}

function clampModelCardValues(modelCard: ApiModelConfigItem) {
  const nextContext = Math.round(Number(modelCard.contextWindowTokens ?? 128_000));
  const contextMax = contextWindowMax(modelCard);
  const contextMin = Math.min(SLIDER_CONTEXT_MIN, contextMax);
  const clampedContext = Math.max(contextMin, Math.min(contextMax, nextContext));
  if (Number.isFinite(nextContext) && nextContext !== clampedContext) {
    modelCard.contextWindowTokens = clampedContext;
  }

  const nextOutput = Math.round(Number(modelCard.maxOutputTokens ?? 4_096));
  const clampedOutput = Math.max(256, Math.min(maxOutputTokensMax(modelCard), nextOutput));
    if (Number.isFinite(nextOutput) && nextOutput !== clampedOutput) {
      modelCard.maxOutputTokens = clampedOutput;
    }
  }

  function clampManualContextWindowValue(modelCard: ApiModelConfigItem) {
    const nextContext = Math.round(Number(modelCard.contextWindowTokens ?? 128_000));
    const clampedContext = Math.max(SLIDER_CONTEXT_MIN, Math.min(2_000_000, nextContext));
    if (!Number.isFinite(nextContext)) {
      modelCard.contextWindowTokens = 128_000;
      return;
    }
    if (nextContext !== clampedContext) {
      modelCard.contextWindowTokens = clampedContext;
    }
  }

function selectModelOption(modelCard: ApiModelConfigItem, option: string) {
  modelCard.model = option;
  const provider = selectedProvider.value;
  if (provider && !provider.cachedModelOptions.includes(option)) {
    provider.cachedModelOptions.push(option);
  }
  if (provider) {
    applyProtocolDefaults(provider);
  }
  void syncModelMetadata(modelCard);
  closeModelPicker();
}

async function syncModelMetadata(modelCard: ApiModelConfigItem) {
  const provider = selectedProvider.value;
  const model = String(modelCard.model || "").trim();
  if (!provider || !model) return;
  try {
    if (provider.requestFormat === "auto") {
      const adapter = await invokeTauri<string>("resolve_model_adapter_kind", {
        modelName: model,
      });
      resolvedAdapterByModelId.value = {
        ...resolvedAdapterByModelId.value,
        [modelCard.id]: adapter,
      };
      if (isGoogleModelAdapter(adapter)) {
        normalizeGeminiReasoningEffort(modelCard);
      } else if (isDeepSeekModelAdapter(adapter)) {
        normalizeDeepSeekReasoningEffort(modelCard);
      } else if (isOpenaiModelAdapter(adapter)) {
        normalizeOpenaiReasoningEffort(modelCard);
      }
    } else if (provider.requestFormat === "gemini") {
      normalizeGeminiReasoningEffort(modelCard);
    } else if (provider.requestFormat === "deepseek") {
      normalizeDeepSeekReasoningEffort(modelCard);
    } else if (provider.requestFormat === "openai" || provider.requestFormat === "openai_responses") {
      normalizeOpenaiReasoningEffort(modelCard);
    }
    const metadata = await invokeTauri<FetchModelMetadataResult>("fetch_model_metadata", {
      input: {
        requestFormat: provider.requestFormat,
        model,
      },
    });
    if (!metadata?.found) {
      modelCard.contextWindowTokens = 200_000;
      clampModelCardValues(modelCard);
      return;
    }
    const nextLimits: ModelCapabilityLimits = {};
    if (Number.isFinite(Number(metadata.contextWindowTokens))) {
      nextLimits.contextWindowMax = Number(metadata.contextWindowTokens);
    }
    if (Number.isFinite(Number(metadata.maxOutputTokens))) {
      nextLimits.maxOutputTokensMax = Number(metadata.maxOutputTokens);
    }
    modelCapabilityById.value = {
      ...modelCapabilityById.value,
      [modelCard.id]: nextLimits,
    };
    clampModelCardValues(modelCard);
  } catch (error) {
    const message = String(error || "").trim();
    if (message.includes("暂无模型元数据缓存")) {
      return;
    }
    console.warn("[API] fetch model metadata failed:", error);
  }
}

async function syncSelectedProviderModelMetadata() {
  const provider = selectedProvider.value;
  if (!provider || provider.requestFormat === "codex") return;
  for (const modelCard of provider.models || []) {
    if (!String(modelCard.model || "").trim()) continue;
    await syncModelMetadata(modelCard);
  }
}

function resolveCodexProvider(provider?: ApiProviderConfigItem | null): ApiProviderConfigItem | null {
  if (!provider || provider.requestFormat !== "codex") return null;
  applyProtocolDefaults(provider);
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
  const message = String(error || "Codex 登录状态检查失败。");
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
      message: "已退出 Codex 登录。",
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
  applyProtocolDefaults(selectedProvider.value);
  selectedProvider.value.baseUrl = generatedBaseUrl.value;
}

async function openProviderSite(preset: ProviderPreset) {
  if (!preset.docsUrl) return;
  try {
    await invokeTauri("open_external_url", { url: preset.docsUrl });
  } catch (error) {
    console.warn("[API] open provider docs failed:", error);
  }
}

async function handleSaveApiConfig() {
  const provider = selectedProvider.value;
  if (provider) {
    applyProtocolDefaults(provider);
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
  const modelCard = provider.models.find((m) => m.id === modelCardId);
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
  if (cap === "embedding" && !modelName) {
    modelConnectionResult.value = {
      ...modelConnectionResult.value,
      [modelCardId]: { success: false, error: "Model name is empty" },
    };
    return;
  }
  modelConnectionTesting.value = { ...modelConnectionTesting.value, [modelCardId]: true };
  modelConnectionResult.value = { ...modelConnectionResult.value, [modelCardId]: undefined as unknown as ModelConnectionResult };
  const started = Date.now();
  try {
    if (cap === "voice") {
      const result = await invokeTauri<{ elapsedMs: number }>("test_voice_connection", {
        input: {
          baseUrl: provider.baseUrl.trim(),
          apiKey: apiKey.trim(),
          requestFormat: provider.requestFormat,
        },
      });
      modelConnectionResult.value = {
        ...modelConnectionResult.value,
        [modelCardId]: { success: true, latencyMs: result.elapsedMs },
      };
    } else if (cap === "embedding") {
      const result = await invokeTauri<{ vectorDim: number; elapsedMs: number }>("test_embedding_connection", {
        input: {
          baseUrl: provider.baseUrl.trim(),
          apiKey: apiKey.trim(),
          requestFormat: provider.requestFormat,
          model: modelName,
        },
      });
      modelConnectionResult.value = {
        ...modelConnectionResult.value,
        [modelCardId]: { success: true, latencyMs: result.elapsedMs },
      };
    } else {
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
      modelConnectionResult.value = {
        ...modelConnectionResult.value,
        [modelCardId]: { success: true, latencyMs: Date.now() - started },
      };
    }
  } catch (err) {
    modelConnectionResult.value = {
      ...modelConnectionResult.value,
      [modelCardId]: { success: false, error: String(err || "Unknown error") },
    };
  } finally {
    modelConnectionTesting.value = { ...modelConnectionTesting.value, [modelCardId]: false };
  }
}

function maskKeyPreview(key: string): string {
  const trimmed = key.trim();
  if (trimmed.length <= 8) return "*".repeat(trimmed.length);
  return trimmed.slice(0, 4) + "*".repeat(trimmed.length - 8) + trimmed.slice(-4);
}

async function runSingleConnectionTest(apiKey: string): Promise<ConnectionTestResultItem> {
  const provider = selectedProvider.value!;
  const modelCard = provider.models.find((m) => m.id === connectionTestModelId.value) ?? provider.models[0];
  const modelName = modelCard?.model.trim() ?? "";
  const cap = capabilityFromRequestFormat(provider.requestFormat);
  const started = Date.now();
  try {
    if (cap === "voice") {
      const result = await invokeTauri<{ elapsedMs: number }>("test_voice_connection", {
        input: { baseUrl: provider.baseUrl.trim(), apiKey: apiKey.trim(), requestFormat: provider.requestFormat },
      });
      return { keyPreview: maskKeyPreview(apiKey), success: true, latencyMs: result.elapsedMs };
    } else if (cap === "embedding") {
      const result = await invokeTauri<{ vectorDim: number; elapsedMs: number }>("test_embedding_connection", {
        input: { baseUrl: provider.baseUrl.trim(), apiKey: apiKey.trim(), requestFormat: provider.requestFormat, model: modelName },
      });
      return { keyPreview: maskKeyPreview(apiKey), success: true, latencyMs: result.elapsedMs };
    } else {
      await invokeTauri<string>("quick_genai_chat", {
        input: { baseUrl: provider.baseUrl.trim(), apiKey: apiKey.trim(), requestFormat: provider.requestFormat, model: modelName, prompt: "连通性测试，恢复1代表连通", providerId: provider.id },
      });
      return { keyPreview: maskKeyPreview(apiKey), success: true, latencyMs: Date.now() - started };
    }
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
  (providerId) => {
    const provider = selectedProvider.value;
    if (!providerId || !provider) {
      stopCodexAuthPolling();
      return;
    }
    if (provider.requestFormat === "codex") {
      void refreshCodexAuthStatus(provider);
      return;
    }
    void syncSelectedProviderModelMetadata();
    stopCodexAuthPolling();
  },
  { immediate: true },
);

watch(
  () => props.refreshingModels,
  (refreshing, wasRefreshing) => {
    if (wasRefreshing && !refreshing && props.modelRefreshOk) {
      void syncSelectedProviderModelMetadata();
    }
  },
);

watch(
  () => selectedProvider.value?.id,
  () => {
    const provider = selectedProvider.value;
    connectionTestKeyStatus.value = {};
    modelConnectionResult.value = {};
    if (!provider || provider.models.length === 0) {
      connectionTestModelId.value = "";
      connectionTestResults.value = [];
      return;
    }
    if (!provider.models.some((m) => m.id === connectionTestModelId.value)) {
      connectionTestModelId.value = provider.models[0].id;
    }
  },
  { immediate: true },
);

onUnmounted(() => {
  stopCodexAuthPolling();
});
</script>

<style scoped>
.api-save-btn {
  position: relative;
  overflow: hidden;
  isolation: isolate;
}

.api-save-btn::before {
  content: "";
  position: absolute;
  inset: -18px;
  border-radius: 9999px;
  background:
    conic-gradient(
      from 0deg,
      transparent 0deg,
      transparent 220deg,
      rgba(255, 255, 255, 0.95) 280deg,
      rgba(255, 255, 255, 0.1) 320deg,
      transparent 360deg
    );
  opacity: 0;
  transform: rotate(0deg);
  transition: opacity 180ms ease;
  z-index: -2;
}

.api-save-btn::after {
  content: "";
  position: absolute;
  inset: 2px;
  border-radius: calc(var(--radius-btn, 0.5rem) - 2px);
  background: inherit;
  z-index: -1;
}

.api-save-btn--dirty {
  box-shadow: 0 0 0 1px rgba(34, 197, 94, 0.25), 0 0 14px rgba(34, 197, 94, 0.2);
}

.api-save-btn--dirty::before {
  opacity: 1;
  animation: api-save-ring-spin 1.8s linear infinite;
}

@keyframes api-save-ring-spin {
  from {
    transform: rotate(0deg);
  }

  to {
    transform: rotate(360deg);
  }
}
</style>

<template>
  <SettingsStickyLayout>
    <template #header>
      <div class="grid gap-2">
        <div v-if="errorText" class="alert alert-error py-2 text-sm">{{ errorText }}</div>
        <div v-if="statusText" class="alert alert-success py-2 text-sm">{{ statusText }}</div>
        <div class="flex items-center justify-between gap-2">
          <div class="flex min-w-0 items-center gap-2">
            <span class="text-xs opacity-70">{{ t("simpleSetup.overwriteHint") }}</span>
          </div>
          <button
            class="btn btn-primary btn-sm"
            type="button"
            :disabled="saving || loading"
            @click="handleSave"
          >
            <span v-if="saving" class="loading loading-spinner loading-xs"></span>
            {{ t("simpleSetup.saveAndStart") }}
          </button>
        </div>
      </div>
    </template>

    <div v-if="loading" class="flex min-h-48 items-center justify-center">
      <span class="loading loading-spinner loading-md"></span>
    </div>

    <div v-else class="grid gap-4">
      <section class="card bg-base-100 border border-base-300">
        <div class="card-body gap-3 p-4">
          <h3 class="text-sm font-semibold">{{ t("simpleSetup.appearance") }}</h3>
          <div class="grid gap-3">
            <div class="grid gap-1.5">
              <span class="text-xs font-medium opacity-70">{{ t("appearance.language") }}</span>
              <div class="grid grid-cols-3 gap-2">
                <button
                  v-for="option in languageOptions"
                  :key="option.value"
                  class="btn btn-sm"
                  :class="draft.uiLanguage === option.value ? 'btn-primary' : 'bg-base-200'"
                  type="button"
                  @click="setUiLanguage(option.value)"
                >
                  {{ option.label }}
                </button>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section class="card bg-base-100 border border-base-300">
        <div class="card-body gap-3 p-4">
          <h3 class="text-sm font-semibold">{{ t("simpleSetup.provider") }}</h3>
          <div class="grid grid-cols-3 gap-2">
            <button
              v-for="option in providerOptions"
              :key="option.id"
              class="btn btn-sm"
              :class="draft.providerId === option.id ? 'btn-primary' : 'bg-base-200'"
              type="button"
              @click="selectProvider(option.id)"
            >
              {{ option.label }}
            </button>
          </div>
          <template v-if="draft.providerId === 'custom'">
            <label class="grid gap-1.5">
              <span class="text-xs font-medium opacity-70">{{ t("simpleSetup.apiProtocol") }}</span>
              <select v-model="draft.customRequestFormat" class="select select-bordered select-sm font-mono">
                <option v-for="option in simpleSetupProtocolOptions" :key="option.value" :value="option.value">
                  {{ option.label }}
                </option>
              </select>
            </label>
            <label class="grid gap-1.5">
              <span class="text-xs font-medium opacity-70">base_url</span>
              <input v-model.trim="draft.customBaseUrl" class="input input-bordered input-sm font-mono" />
            </label>
          </template>
          <div class="divider divider-sm my-0"></div>
          <div class="grid gap-1.5">
            <span class="text-xs font-medium opacity-70">{{ t("quickSetup.fields.apiKey") }}</span>
            <div class="flex items-center gap-2">
              <input
                v-model.trim="draft.apiKey"
                :type="showApiKey ? 'text' : 'password'"
                class="input input-bordered input-sm min-w-0 flex-1 font-mono"
                placeholder="sk-..."
              />
              <button
                class="btn btn-sm btn-square bg-base-200"
                type="button"
                :aria-label="showApiKey ? t('quickSetup.actions.hideKey') : t('quickSetup.actions.showKey')"
                @click="showApiKey = !showApiKey"
              >
                <EyeOff v-if="showApiKey" class="h-3.5 w-3.5" />
                <Eye v-else class="h-3.5 w-3.5" />
              </button>
              <button
                v-if="providerApiKeyUrl"
                class="btn btn-sm bg-base-200"
                type="button"
                @click="openProviderKeyUrl"
              >
                {{ t("quickSetup.actions.getKey") }}
              </button>
            </div>
          </div>
        </div>
      </section>

      <section class="card bg-base-100 border border-base-300">
        <div class="card-body gap-3 p-4">
          <div class="flex items-center justify-between gap-2">
            <h3 class="text-sm font-semibold">{{ t("simpleSetup.models") }}</h3>
            <button
              v-if="draft.providerId === 'custom'"
              class="btn btn-xs bg-base-200"
              type="button"
              :class="{ loading: refreshingCustomModels }"
              :disabled="refreshingCustomModels"
              @click="refreshCustomModels"
            >
              <RefreshCw class="h-3.5 w-3.5" />
              {{ t("config.api.refreshModels") }}
            </button>
          </div>
          <div class="grid gap-3">
            <div v-for="card in modelCards" :key="card.id">
              <div class="mb-2 flex items-center gap-3">
                <span class="whitespace-nowrap text-sm font-semibold">{{ card.label }}</span>
                <span v-if="card.hint" class="whitespace-nowrap text-xs opacity-60">{{ card.hint }}</span>
                <div class="divider divider-sm my-0 flex-1"></div>
              </div>
              <ApiModelCard
                :card="draft.models[card.id]"
                :model-options="draft.providerId === 'custom' ? draft.customModelOptions : []"
                :show-delete="false"
                :show-capability-toggles="false"
                :show-context-window="false"
                :show-reasoning="card.id !== 'vision'"
                :show-temperature="false"
                :show-max-output-tokens="false"
                :reasoning-items="reasoningEffortOptions"
                :reasoning-checked-values="[String(draft.models[card.id].reasoningEffort || '')]"
                @reasoning-change="(payload: { value: string; checked: boolean }) => { if (payload.checked) draft.models[card.id].reasoningEffort = payload.value as SimpleReasoningEffort }"
              />
            </div>
          </div>
        </div>
      </section>

      <section class="card bg-base-100 border border-base-300">
        <div class="card-body gap-3 p-4">
          <h3 class="text-sm font-semibold">{{ t("simpleSetup.hotkeys") }}</h3>
          <div class="grid gap-3">
            <label class="grid gap-1.5">
              <span class="text-xs font-medium opacity-70">{{ t("quickSetup.fields.summonHotkey") }}</span>
              <div class="grid grid-cols-[minmax(0,1fr)_auto] gap-2">
                <input :value="draft.hotkey" class="input input-bordered" readonly />
                <button
                  class="btn btn-sm justify-start"
                  :class="hotkeyCaptureTarget === 'summon' ? 'btn-primary' : 'bg-base-200'"
                  type="button"
                  @click="startHotkeyCapture('summon')"
                >
                  {{ t("quickSetup.actions.record") }}
                </button>
              </div>
            </label>
            <label class="grid gap-1.5">
              <span class="text-xs font-medium opacity-70">{{ t("quickSetup.fields.recordHotkey") }}</span>
              <div class="grid grid-cols-[minmax(0,1fr)_auto] gap-2">
                <input :value="draft.recordHotkey" class="input input-bordered" readonly />
                <button
                  class="btn btn-sm justify-start"
                  :class="hotkeyCaptureTarget === 'record' ? 'btn-primary' : 'bg-base-200'"
                  type="button"
                  @click="startHotkeyCapture('record')"
                >
                  {{ t("quickSetup.actions.record") }}
                </button>
              </div>
            </label>
            <div class="text-xs opacity-70">{{ hotkeyCaptureHint }}</div>
          </div>
        </div>
      </section>

      <details class="card bg-base-100 border border-base-300">
        <summary class="cursor-pointer list-none px-4 py-3 text-sm font-semibold select-none">
          {{ t("simpleSetup.siliconFlow") }}
        </summary>
        <div class="card-body gap-3 p-4 pt-0">
          <div class="grid gap-1.5">
            <span class="text-xs font-medium opacity-70">{{ t("quickSetup.fields.apiKey") }}</span>
            <div class="flex items-center gap-2">
              <input
                v-model.trim="draft.siliconFlowKey"
                :type="showSiliconFlowKey ? 'text' : 'password'"
                class="input input-bordered input-sm min-w-0 flex-1 font-mono"
                placeholder="sk-..."
              />
              <button
                class="btn btn-sm btn-square bg-base-200"
                type="button"
                :aria-label="showSiliconFlowKey ? t('quickSetup.actions.hideKey') : t('quickSetup.actions.showKey')"
                @click="showSiliconFlowKey = !showSiliconFlowKey"
              >
                <EyeOff v-if="showSiliconFlowKey" class="h-3.5 w-3.5" />
                <Eye v-else class="h-3.5 w-3.5" />
              </button>
              <button class="btn btn-sm bg-base-200" type="button" @click="openSiliconFlowKeyUrl">
                {{ t("quickSetup.actions.getKey") }}
              </button>
            </div>
          </div>
          <div class="text-xs opacity-70">{{ t("simpleSetup.siliconFlowHint") }}</div>
        </div>
      </details>
    </div>
  </SettingsStickyLayout>
</template>

<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useI18n } from "vue-i18n";
import { Eye, EyeOff, RefreshCw } from "@lucide/vue";
import ApiModelCard from "../../components/ApiModelCard.vue";
import SettingsStickyLayout from "../../components/SettingsStickyLayout.vue";
import { openTransportWindow, hideCurrentTransportWindow } from "../../../../services/tauri-api";
import { clearSimpleSetupDraft, saveSimpleSetupDraft, simpleProviderOptions, simpleSetupProtocolOptions, useSimpleSetup } from "../../quick-setup/use-simple-setup";
import type { SimpleModelCard, SimpleReasoningEffort } from "../../quick-setup/use-simple-setup";

const { t } = useI18n();

const {
  loading,
  saving,
  errorText,
  statusText,
  showApiKey,
  showSiliconFlowKey,
  hotkeyCaptureTarget,
  hotkeyCaptureHint,
  refreshingCustomModels,
  draft,
  languageOptions,
  providerApiKeyUrl,
  loadSnapshot,
  selectProvider,
  refreshCustomModels,
  openProviderKeyUrl,
  openSiliconFlowKeyUrl,
  setUiLanguage,
  startHotkeyCapture,
  saveAll,
} = useSimpleSetup();
const providerOptions = simpleProviderOptions.filter((option) => option.id !== "opencode");

const modelCards = computed(() => {
  const cards = [
    { id: "quick" as SimpleModelCard, label: t("simpleSetup.modelQuick"), hint: t("simpleSetup.modelQuickHint") },
    { id: "expert" as SimpleModelCard, label: t("simpleSetup.modelExpert"), hint: t("simpleSetup.modelExpertHint") },
    { id: "vision" as SimpleModelCard, label: t("simpleSetup.modelVision"), hint: t("simpleSetup.modelVisionHint") },
  ];
  if (draft.providerId === "deepseek") {
    return cards.filter((card) => card.id !== "vision");
  }
  return cards;
});

const reasoningEffortOptions = computed(() => [
  { value: "low", label: t("simpleSetup.effortLow") },
  { value: "medium", label: t("simpleSetup.effortMedium") },
  { value: "high", label: t("simpleSetup.effortHigh") },
]);

onMounted(async () => {
  await loadSnapshot();
});

async function handleSave() {
  saveSimpleSetupDraft({ ...draft });
  await saveAll();
  if (!errorText.value) {
    clearSimpleSetupDraft();
    await openTransportWindow("chat");
    await hideCurrentTransportWindow();
  }
}
</script>

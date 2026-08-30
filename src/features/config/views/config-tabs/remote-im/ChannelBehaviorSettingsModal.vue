<template>
  <button
    type="button"
    class="btn btn-square btn-ghost"
    :title="t('config.remoteIm.channelBehaviorSettings')"
    :disabled="!channel"
    @click="openModal"
  >
    <SlidersHorizontal class="h-3.5 w-3.5" />
  </button>

  <dialog
    ref="dialogRef"
    class="modal"
    @close="onDialogClose"
    @cancel.prevent="onDialogClose"
  >
    <div class="modal-box flex h-[82vh] w-[82vw] max-w-none flex-col overflow-hidden p-0">
        <header class="flex shrink-0 items-start justify-between gap-4 border-b border-base-300 px-5 py-4">
          <div class="min-w-0">
            <h3 class="font-semibold">{{ t('config.remoteIm.channelBehaviorSettings') }}</h3>
          </div>
          <button type="button" class="btn btn-circle btn-sm btn-ghost" :title="t('common.close')" @click="closeModal">×</button>
        </header>

        <div class="min-h-0 flex-1 overflow-y-auto bg-base-200/50 px-5 py-4">
          <div v-if="error" class="alert alert-error mb-4 py-2 text-xs">{{ error }}</div>
          <ConfigTemplate v-model="templateDraft" :groups="templateGroups" />
        </div>

        <footer class="flex shrink-0 flex-wrap justify-end gap-2 border-t border-base-300 px-5 py-4">
          <button type="button" class="btn btn-ghost" :disabled="saving" @click="restoreDefaults">{{ t('config.remoteIm.restoreBehaviorDefaults') }}</button>
          <button type="button" class="btn btn-ghost" :disabled="saving || !savedSnapshot" @click="restoreSaved">{{ t('config.remoteIm.restoreBehaviorSaved') }}</button>
          <button type="button" class="btn btn-primary" :disabled="saving || !dirty || !!validationError" @click="save">
            <span v-if="saving" class="loading loading-spinner loading-xs"></span>
            <Save v-else class="h-3.5 w-3.5" />{{ t('common.save') }}
          </button>
        </footer>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="onDialogClose">close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Save, SlidersHorizontal } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import { invokeTauri } from "../../../../../services/tauri-api";
import type { RemoteImChannelBehaviorSettings, RemoteImChannelConfig, RemoteImGroupReplyPacing } from "../../../../../types/app";
import ConfigTemplate from "../../../components/ConfigTemplate.vue";
import type { ConfigTemplateGroup } from "../../../components/config-template";
import {
  cloneChannelBehaviorSettings,
  DEFAULT_REMOTE_IM_CHANNEL_BEHAVIOR_SETTINGS,
  normalizeGroupReplyPacing,
  parseSpaceSeparatedList,
} from "./helpers";

const props = defineProps<{
  channel: RemoteImChannelConfig | null;
  saveConfigAction: () => Promise<boolean> | boolean;
  setStatusAction: (text: string) => void;
}>();
const { t } = useI18n();

type Draft = {
  responseGuidance: string;
  blockedMessagePrefixesText: string;
  muteKeywordsText: string;
  unmuteKeywordsText: string;
  patienceSeconds: number;
  muteDurationSeconds: number;
  activationCooldownSeconds: number;
  positiveEnergyPhrasesText: string;
  negativeEnergyPhrasesText: string;
  focusInstructionsText: string;
  pacing: RemoteImGroupReplyPacing;
};

function draftFromSettings(value?: Partial<RemoteImChannelBehaviorSettings> | null): Draft {
  const settings = cloneChannelBehaviorSettings(value);
  const pacing = normalizeGroupReplyPacing(settings.groupReplyPacing);
  return {
    responseGuidance: settings.responseGuidance,
    blockedMessagePrefixesText: settings.blockedMessagePrefixes.join(" "),
    muteKeywordsText: settings.muteKeywords.join(" "),
    unmuteKeywordsText: settings.unmuteKeywords.join(" "),
    patienceSeconds: settings.patienceSeconds,
    muteDurationSeconds: settings.muteDurationSeconds,
    activationCooldownSeconds: settings.activationCooldownSeconds,
    positiveEnergyPhrasesText: pacing.positiveEnergyPhrases.join(" "),
    negativeEnergyPhrasesText: pacing.negativeEnergyPhrases.join(" "),
    focusInstructionsText: pacing.focusInstructions.join(" "),
    pacing,
  };
}

function settingsFromDraft(value: Draft): RemoteImChannelBehaviorSettings {
  return {
    responseGuidance: String(value.responseGuidance || "").trim(),
    blockedMessagePrefixes: parseSpaceSeparatedList(value.blockedMessagePrefixesText),
    muteKeywords: parseSpaceSeparatedList(value.muteKeywordsText),
    unmuteKeywords: parseSpaceSeparatedList(value.unmuteKeywordsText),
    patienceSeconds: Math.max(0, Math.floor(Number(value.patienceSeconds) || 0)),
    muteDurationSeconds: Math.max(0, Math.floor(Number(value.muteDurationSeconds) || 0)),
    activationCooldownSeconds: Math.max(0, Math.floor(Number(value.activationCooldownSeconds) || 0)),
    groupReplyPacing: {
      ...value.pacing,
      positiveEnergyPhrases: parseSpaceSeparatedList(value.positiveEnergyPhrasesText),
      negativeEnergyPhrases: parseSpaceSeparatedList(value.negativeEnergyPhrasesText),
      focusInstructions: parseSpaceSeparatedList(value.focusInstructionsText),
    },
  };
}

const dialogRef = ref<HTMLDialogElement | null>(null);
const open = ref(false);

function onDialogClose() {
  if (saving.value) {
    const d = dialogRef.value;
    if (d && !d.open && open.value) d.showModal();
    return;
  }
  closeModal();
}

function syncDialog() {
  const d = dialogRef.value;
  if (!d) return;
  if (open.value) {
    if (!d.open) d.showModal();
  } else if (d.open) d.close();
}

watch(open, syncDialog);
watch(dialogRef, syncDialog);

const draft = ref<Draft>(draftFromSettings());
const templateDraft = computed<Record<string, unknown>>({
  get: () => ({
    responseGuidance: draft.value.responseGuidance,
    blockedMessagePrefixesText: draft.value.blockedMessagePrefixesText,
    muteKeywordsText: draft.value.muteKeywordsText,
    unmuteKeywordsText: draft.value.unmuteKeywordsText,
    patienceSeconds: draft.value.patienceSeconds,
    muteDurationSeconds: draft.value.muteDurationSeconds,
    activationCooldownSeconds: draft.value.activationCooldownSeconds,
    positiveEnergyPhrasesText: draft.value.positiveEnergyPhrasesText,
    negativeEnergyPhrasesText: draft.value.negativeEnergyPhrasesText,
    focusInstructionsText: draft.value.focusInstructionsText,
    assistantDebounceSeconds: draft.value.pacing.assistantDebounceSeconds,
    secretaryInspectionSeconds: draft.value.pacing.secretaryInspectionSeconds,
    replyCooldownSeconds: draft.value.pacing.replyCooldownSeconds,
    inspectionJitterRatio: draft.value.pacing.inspectionJitterRatio,
    maximumEnergy: draft.value.pacing.maximumEnergy,
    baseReplyEnergyCost: draft.value.pacing.baseReplyEnergyCost,
    energyCostPerCharacter: draft.value.pacing.energyCostPerCharacter,
    energyRecoveryPerSecond: draft.value.pacing.energyRecoveryPerSecond,
    positiveEnergyDelta: draft.value.pacing.positiveEnergyDelta,
    negativeEnergyDelta: draft.value.pacing.negativeEnergyDelta,
    normalReplyMaxChars: draft.value.pacing.normalReplyMaxChars,
    focusReplyMaxChars: draft.value.pacing.focusReplyMaxChars,
  }),
  set: (value) => {
    const numberValue = (key: string, fallback: number) => {
      const parsed = Number(value[key]);
      return Number.isFinite(parsed) ? parsed : fallback;
    };
    draft.value = {
      ...draft.value,
      responseGuidance: String(value.responseGuidance ?? draft.value.responseGuidance),
      blockedMessagePrefixesText: String(value.blockedMessagePrefixesText ?? draft.value.blockedMessagePrefixesText),
      muteKeywordsText: String(value.muteKeywordsText ?? draft.value.muteKeywordsText),
      unmuteKeywordsText: String(value.unmuteKeywordsText ?? draft.value.unmuteKeywordsText),
      patienceSeconds: numberValue("patienceSeconds", draft.value.patienceSeconds),
      muteDurationSeconds: numberValue("muteDurationSeconds", draft.value.muteDurationSeconds),
      activationCooldownSeconds: numberValue("activationCooldownSeconds", draft.value.activationCooldownSeconds),
      positiveEnergyPhrasesText: String(value.positiveEnergyPhrasesText ?? draft.value.positiveEnergyPhrasesText),
      negativeEnergyPhrasesText: String(value.negativeEnergyPhrasesText ?? draft.value.negativeEnergyPhrasesText),
      focusInstructionsText: String(value.focusInstructionsText ?? draft.value.focusInstructionsText),
      pacing: {
        ...draft.value.pacing,
        assistantDebounceSeconds: numberValue("assistantDebounceSeconds", draft.value.pacing.assistantDebounceSeconds),
        secretaryInspectionSeconds: numberValue("secretaryInspectionSeconds", draft.value.pacing.secretaryInspectionSeconds),
        replyCooldownSeconds: numberValue("replyCooldownSeconds", draft.value.pacing.replyCooldownSeconds),
        inspectionJitterRatio: numberValue("inspectionJitterRatio", draft.value.pacing.inspectionJitterRatio),
        maximumEnergy: numberValue("maximumEnergy", draft.value.pacing.maximumEnergy),
        baseReplyEnergyCost: numberValue("baseReplyEnergyCost", draft.value.pacing.baseReplyEnergyCost),
        energyCostPerCharacter: numberValue("energyCostPerCharacter", draft.value.pacing.energyCostPerCharacter),
        energyRecoveryPerSecond: numberValue("energyRecoveryPerSecond", draft.value.pacing.energyRecoveryPerSecond),
        positiveEnergyDelta: numberValue("positiveEnergyDelta", draft.value.pacing.positiveEnergyDelta),
        negativeEnergyDelta: numberValue("negativeEnergyDelta", draft.value.pacing.negativeEnergyDelta),
        normalReplyMaxChars: numberValue("normalReplyMaxChars", draft.value.pacing.normalReplyMaxChars),
        focusReplyMaxChars: numberValue("focusReplyMaxChars", draft.value.pacing.focusReplyMaxChars),
      },
    };
  },
});
const templateGroups = computed<ConfigTemplateGroup[]>(() => [
  {
    title: t("config.remoteIm.channelBehaviorMessagePolicySection"),
    rows: [
      {
        items: [{
          key: "responseGuidance",
          label: t("config.remoteIm.responseGuidance"),
          description: t("config.remoteIm.responseGuidanceHint"),
          placeholder: t("config.remoteIm.responseGuidancePlaceholder"),
          type: "textarea",
        }],
      },
      {
        items: [{
          key: "blockedMessagePrefixesText",
          label: t("config.remoteIm.blockedMessagePrefixes"),
          description: t("config.remoteIm.blockedMessagePrefixesHint"),
          placeholder: t("config.remoteIm.blockedMessagePrefixesPlaceholder"),
          type: "text",
        }],
      },
      {
        items: [
          { key: "muteKeywordsText", label: t("config.remoteIm.muteKeywords"), type: "text", placeholder: t("config.remoteIm.muteKeywordsPlaceholder") },
          { key: "unmuteKeywordsText", label: t("config.remoteIm.unmuteKeywords"), type: "text", placeholder: t("config.remoteIm.unmuteKeywordsPlaceholder") },
        ],
      },
    ],
  },
  {
    title: t("config.remoteIm.channelBehaviorViewFrequencySection"),
    rows: [
      {
        items: [
          { key: "assistantDebounceSeconds", label: t("config.remoteIm.assistantDebounceSeconds"), type: "number", min: 1 },
          { key: "secretaryInspectionSeconds", label: t("config.remoteIm.secretaryInspectionSeconds"), type: "number", min: 1 },
        ],
      },
      {
        items: [{ key: "inspectionJitterRatio", label: t("config.remoteIm.inspectionJitterRatio"), type: "number", min: 0, max: 1, step: 0.05 }],
      },
      {
        items: [
          { key: "patienceSeconds", label: t("config.remoteIm.patienceExit"), type: "number", min: 0 },
          { key: "activationCooldownSeconds", label: t("config.remoteIm.activationCooldownSeconds"), type: "number", min: 0 },
        ],
      },
    ],
  },
  {
    title: t("config.remoteIm.channelBehaviorReplyEnthusiasmSection"),
    rows: [
      {
        items: [
          { key: "muteDurationSeconds", label: t("config.remoteIm.muteDuration"), type: "number", min: 0 },
          { key: "replyCooldownSeconds", label: t("config.remoteIm.replyCooldownSeconds"), type: "number", min: 0 },
        ],
      },
      {
        items: [
          { key: "maximumEnergy", label: t("config.remoteIm.maximumEnergy"), type: "number", min: 0.01, step: 1 },
          { key: "baseReplyEnergyCost", label: t("config.remoteIm.baseReplyEnergyCost"), type: "number", min: 0, step: 0.1 },
        ],
      },
      {
        items: [
          { key: "energyCostPerCharacter", label: t("config.remoteIm.energyCostPerCharacter"), type: "number", min: 0, step: 0.01 },
          { key: "energyRecoveryPerSecond", label: t("config.remoteIm.energyRecoveryPerSecond"), type: "number", min: 0, step: 0.01 },
        ],
      },
      {
        items: [
          { key: "positiveEnergyPhrasesText", label: t("config.remoteIm.positiveEnergyPhrases"), type: "text" },
          { key: "positiveEnergyDelta", label: t("config.remoteIm.positiveEnergyDelta"), type: "number", min: 0, step: 0.1 },
        ],
      },
      {
        items: [
          { key: "negativeEnergyPhrasesText", label: t("config.remoteIm.negativeEnergyPhrases"), type: "text" },
          { key: "negativeEnergyDelta", label: t("config.remoteIm.negativeEnergyDelta"), type: "number", max: 0, step: 0.1 },
        ],
      },
    ],
  },
  {
    title: t("config.remoteIm.channelBehaviorReplyLengthSection"),
    rows: [
      {
        items: [{ key: "focusInstructionsText", label: t("config.remoteIm.focusInstructions"), type: "text" }],
      },
      {
        items: [
          {
            key: "normalReplyMaxChars",
            label: t("config.remoteIm.normalReplyMaxChars"),
            description: t("config.remoteIm.normalReminderPreview", { count: draft.value.pacing.normalReplyMaxChars }),
            type: "number",
            min: 1,
          },
          {
            key: "focusReplyMaxChars",
            label: t("config.remoteIm.focusReplyMaxChars"),
            description: t("config.remoteIm.focusReminderPreview", { count: draft.value.pacing.focusReplyMaxChars }),
            type: "number",
            min: 1,
          },
        ],
      },
    ],
  },
]);
const savedSnapshot = ref("");
const editingChannelId = ref("");
const saving = ref(false);
const error = ref("");
const draftSnapshot = computed(() => JSON.stringify(draft.value));
const dirty = computed(() => !!savedSnapshot.value && draftSnapshot.value !== savedSnapshot.value);
const validationError = computed(() => {
  const common = [draft.value.patienceSeconds, draft.value.muteDurationSeconds, draft.value.activationCooldownSeconds];
  if (common.some((value) => !Number.isFinite(Number(value)))) return t("config.remoteIm.behaviorFiniteNumberError");
  const p = draft.value.pacing;
  const group = [p.assistantDebounceSeconds, p.secretaryInspectionSeconds, p.replyCooldownSeconds, p.inspectionJitterRatio, p.maximumEnergy, p.baseReplyEnergyCost, p.energyCostPerCharacter, p.energyRecoveryPerSecond, p.positiveEnergyDelta, p.negativeEnergyDelta, p.normalReplyMaxChars, p.focusReplyMaxChars];
  if (group.some((value) => !Number.isFinite(Number(value)))) return t("config.remoteIm.behaviorFiniteNumberError");
  if (p.assistantDebounceSeconds < 1 || p.secretaryInspectionSeconds < 1) return t("config.remoteIm.behaviorPeriodError");
  if (p.inspectionJitterRatio < 0 || p.inspectionJitterRatio > 1) return t("config.remoteIm.behaviorJitterError");
  if (p.maximumEnergy <= 0 || p.baseReplyEnergyCost < 0 || p.energyCostPerCharacter < 0 || p.energyRecoveryPerSecond < 0 || p.positiveEnergyDelta < 0 || p.negativeEnergyDelta > 0) return t("config.remoteIm.behaviorEnergyError");
  if (p.normalReplyMaxChars < 1 || p.focusReplyMaxChars < p.normalReplyMaxChars) return t("config.remoteIm.behaviorLengthError");
  return "";
});

function openModal() {
  if (!props.channel) return;
  draft.value = draftFromSettings(props.channel.behaviorSettings);
  savedSnapshot.value = JSON.stringify(draft.value);
  editingChannelId.value = props.channel.id;
  error.value = "";
  open.value = true;
}

function closeModal() {
  if (!saving.value) open.value = false;
}

async function restoreDefaults() {
  const next = draftFromSettings(DEFAULT_REMOTE_IM_CHANNEL_BEHAVIOR_SETTINGS);
  try {
    next.responseGuidance = await invokeTauri<string>("remote_im_get_default_group_response_guidance");
  } catch (restoreError) {
    console.warn("[远程IM] 渠道默认应答规则读取失败，保留其他默认值:", restoreError);
  }
  draft.value = next;
  error.value = "";
}

function restoreSaved() {
  if (!savedSnapshot.value) return;
  try {
    draft.value = JSON.parse(savedSnapshot.value) as Draft;
    error.value = "";
  } catch {
    draft.value = draftFromSettings();
    error.value = "";
  }
}

async function save() {
  if (saving.value || !dirty.value || validationError.value) return;
  const channel = props.channel;
  if (!channel || channel.id !== editingChannelId.value) {
    error.value = t("config.remoteIm.channelBehaviorChannelChanged");
    return;
  }
  saving.value = true;
  error.value = "";
  const submittedSnapshot = draftSnapshot.value;
  const previous = channel.behaviorSettings
    ? cloneChannelBehaviorSettings(channel.behaviorSettings)
    : undefined;
  const next = settingsFromDraft(draft.value);
  channel.behaviorSettings = next;
  try {
    const saved = await Promise.resolve(props.saveConfigAction());
    if (!saved) {
      channel.behaviorSettings = previous;
      error.value = t("config.remoteIm.channelBehaviorSaveFailed");
      return;
    }
    savedSnapshot.value = JSON.stringify(draftFromSettings(next));
    if (draftSnapshot.value === submittedSnapshot) draft.value = draftFromSettings(next);
    props.setStatusAction(t("config.remoteIm.channelBehaviorSaved"));
    try {
      await invokeTauri("remote_im_reconfigure_channel_behavior", { channelId: channel.id });
    } catch (reconfigureError) {
      console.warn("[远程IM] channel behavior reconfigure deferred:", reconfigureError);
      props.setStatusAction(t("config.remoteIm.channelBehaviorSavedReconfigureDeferred"));
    }
  } catch (saveError) {
    channel.behaviorSettings = previous;
    error.value = String(saveError);
  } finally {
    saving.value = false;
  }
}
</script>

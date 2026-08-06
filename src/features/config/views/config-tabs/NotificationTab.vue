<template>
  <ConfigTemplate :model-value="templateValues" :groups="templateGroups">
    <template #group-actions-notification>
      <button
        class="btn btn-sm btn-primary shrink-0"
        :disabled="!notificationDirty || props.savingConfig"
        @click="handleSaveConfig"
      >
        {{ props.savingConfig ? t("common.saving") : t("common.save") }}
      </button>
    </template>
    <template #row-enable-notification>
      <label class="flex min-w-0 items-center justify-between gap-4">
        <div class="min-w-0">
          <div class="text-sm">{{ t("config.notification.enableLabel") }}</div>
          <div class="mt-1 text-xs text-base-content/60">{{ t("config.notification.enableHint") }}</div>
        </div>
        <input
          :checked="props.config.messageNotificationEnabled"
          class="toggle toggle-sm toggle-primary shrink-0"
          type="checkbox"
          @change="props.config.messageNotificationEnabled = ($event.target as HTMLInputElement).checked"
        />
      </label>
    </template>
    <template #row-sound-notification>
      <label
        class="flex min-w-0 items-center justify-between gap-4"
        :class="{ 'opacity-50': !props.config.messageNotificationEnabled }"
      >
        <div class="min-w-0">
          <div class="text-sm">{{ t("config.notification.soundLabel") }}</div>
          <div class="mt-1 text-xs text-base-content/60">{{ t("config.notification.soundHint") }}</div>
        </div>
        <input
          :checked="props.config.messageNotificationSoundEnabled"
          :disabled="!props.config.messageNotificationEnabled"
          class="toggle toggle-sm toggle-primary shrink-0"
          type="checkbox"
          @change="props.config.messageNotificationSoundEnabled = ($event.target as HTMLInputElement).checked"
        />
      </label>
    </template>
    <template #row-desktop-notice>
      <label class="flex min-w-0 items-center justify-between gap-4">
        <div class="min-w-0">
          <div class="text-sm">{{ t("config.notification.desktopNoticeLabel") }}</div>
          <div class="mt-1 text-xs text-base-content/60">{{ t("config.notification.desktopNoticeHint") }}</div>
        </div>
        <input
          :checked="props.config.desktopOperationNoticeEnabled"
          class="toggle toggle-sm toggle-primary shrink-0"
          type="checkbox"
          @change="props.config.desktopOperationNoticeEnabled = ($event.target as HTMLInputElement).checked"
        />
      </label>
    </template>
  </ConfigTemplate>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import ConfigTemplate from "../../components/ConfigTemplate.vue";
import type { ConfigTemplateGroup } from "../../components/config-template";
import type { AppConfig } from "../../../../types/app";

const props = defineProps<{
  config: AppConfig;
  savingConfig: boolean;
  saveConfigAction: () => Promise<boolean> | boolean;
  lastSavedConfigJson: string;
}>();

const { t } = useI18n();
const templateValues = {};
const templateGroups = computed<ConfigTemplateGroup[]>(() => [
  {
    key: "notification",
    title: t("config.notification.title"),
    rows: [
      { key: "enable-notification", items: [] },
      { key: "sound-notification", items: [] },
      { key: "desktop-notice", items: [] },
    ],
  },
]);

const savedNotificationSnapshot = computed(() => {
  try {
    const parsed = JSON.parse(String(props.lastSavedConfigJson || "{}")) as Partial<AppConfig>;
    return {
      messageNotificationEnabled: parsed.messageNotificationEnabled !== false,
      messageNotificationSoundEnabled: parsed.messageNotificationSoundEnabled === true,
      desktopOperationNoticeEnabled: parsed.desktopOperationNoticeEnabled !== false,
    };
  } catch {
    return {
      messageNotificationEnabled: true,
      messageNotificationSoundEnabled: false,
      desktopOperationNoticeEnabled: true,
    };
  }
});

const notificationDirty = computed(() => (
  props.config.messageNotificationEnabled !== savedNotificationSnapshot.value.messageNotificationEnabled
  || props.config.messageNotificationSoundEnabled !== savedNotificationSnapshot.value.messageNotificationSoundEnabled
  || props.config.desktopOperationNoticeEnabled !== savedNotificationSnapshot.value.desktopOperationNoticeEnabled
));

async function handleSaveConfig() {
  if (!notificationDirty.value) return;
  await Promise.resolve(props.saveConfigAction());
}
</script>

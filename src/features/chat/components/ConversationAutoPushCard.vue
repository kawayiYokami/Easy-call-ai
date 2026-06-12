<template>
  <Teleport to="body">
    <div v-if="open" class="fixed inset-0 z-[1200] flex items-center justify-center bg-base-content/30 p-4 backdrop-blur-sm">
      <div class="card w-full max-w-md border border-base-300 bg-base-100 shadow-2xl">
        <div class="card-body gap-4">
          <div class="flex items-start justify-between gap-3">
            <div>
              <h3 class="card-title text-base">{{ t("chat.autoPush.title") }}</h3>
              <p class="text-sm text-base-content/70">{{ t("chat.autoPush.summary") }}</p>
            </div>
            <button type="button" class="btn btn-ghost btn-sm btn-circle" :disabled="saving" @click="emit('close')">
              <X class="h-4 w-4" />
            </button>
          </div>

          <label class="label cursor-pointer justify-start gap-3 rounded-box border border-base-300 px-3 py-2">
            <input
              :checked="enabled"
              type="checkbox"
              class="toggle toggle-primary"
              :disabled="saving"
              @change="emit('update:enabled', ($event.target as HTMLInputElement).checked)"
            />
            <div class="flex min-w-0 flex-1 flex-col">
              <span class="label-text font-medium">{{ t("chat.autoPush.enable") }}</span>
              <span class="text-xs whitespace-normal break-words text-base-content/60">
                {{ t("chat.autoPush.enableHint") }}
              </span>
            </div>
          </label>

          <div class="form-control gap-2">
            <label class="label pb-0">
              <span class="label-text font-medium">{{ t("chat.autoPush.targetContact") }}</span>
            </label>
            <select
              class="select select-bordered w-full"
              :disabled="saving || !enabled"
              :value="selectedContactId"
              @change="emit('update:selectedContactId', ($event.target as HTMLSelectElement).value)"
            >
              <option value="">{{ t("chat.autoPush.selectPlaceholder") }}</option>
              <option
                v-for="item in options"
                :key="item.contactId"
                :value="item.contactId"
              >
                {{ optionLabel(item) }}
              </option>
            </select>
            <p v-if="enabled && options.length === 0" class="text-xs text-warning">
              {{ t("chat.autoPush.empty") }}
            </p>
          </div>

          <div class="card-actions justify-end">
            <button type="button" class="btn btn-ghost" :disabled="saving" @click="emit('close')">
              {{ t("common.cancel") }}
            </button>
            <button type="button" class="btn btn-primary" :disabled="saveDisabled" @click="emit('save')">
              <span v-if="saving" class="loading loading-spinner loading-xs"></span>
              {{ enabled ? t("common.save") : t("chat.autoPush.disableAction") }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { X } from "@lucide/vue";
import type { RemoteImContactConversationOption } from "../../../types/app";

const props = defineProps<{
  open: boolean;
  saving: boolean;
  enabled: boolean;
  selectedContactId: string;
  options: RemoteImContactConversationOption[];
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "save"): void;
  (e: "update:enabled", value: boolean): void;
  (e: "update:selectedContactId", value: string): void;
}>();

const { t } = useI18n();

const saveDisabled = computed(() =>
  props.saving || (props.enabled && !String(props.selectedContactId || "").trim()),
);

function optionLabel(item: RemoteImContactConversationOption): string {
  const title = String(item.title || "").trim();
  const contact = String(item.contactDisplayName || "").trim();
  const channel = String(item.channelName || "").trim();
  return [title || contact || item.contactId, contact && contact !== title ? contact : "", channel]
    .filter(Boolean)
    .join(" / ");
}
</script>

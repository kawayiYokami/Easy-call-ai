<template>
  <div class="grid gap-3">
    <div class="card border border-base-300 bg-base-100">
      <div class="card-body gap-4 p-4">
        <div class="flex items-start justify-between gap-3">
          <div>
            <h3 class="card-title text-base">{{ t("config.networkAccess.title") }}</h3>
            <p class="mt-1 text-xs text-base-content/60">{{ t("config.networkAccess.summary") }}</p>
          </div>
          <button class="btn btn-sm btn-ghost shrink-0" :disabled="loading" :title="t('common.refresh')" @click="refreshInfo">
            <RefreshCw class="h-4 w-4" :class="{ 'animate-spin': loading }" />
          </button>
        </div>

        <label class="flex cursor-pointer items-center justify-between gap-3 rounded-box border border-base-300 bg-base-200/40 p-3">
          <div class="min-w-0">
            <div class="text-sm font-medium">{{ t("config.networkAccess.enabled") }}</div>
            <div class="mt-1 text-xs text-base-content/60">{{ t("config.networkAccess.enabledHint") }}</div>
          </div>
          <input
            type="checkbox"
            class="toggle toggle-primary shrink-0"
            :checked="networkAccessEnabled"
            @change="updateEnabled"
          />
        </label>

        <div class="grid gap-3 md:grid-cols-[minmax(0,1fr)_auto] md:items-end">
          <label class="form-control min-w-0">
            <div class="label px-0 pb-1">
              <span class="label-text text-sm font-medium">{{ t("config.networkAccess.port") }}</span>
            </div>
            <input
              class="input input-bordered input-sm w-full max-w-48 font-mono"
              type="number"
              min="1024"
              max="65535"
              :disabled="!networkAccessEnabled"
              :value="portInput"
              @input="updatePort"
            />
          </label>
          <button
            class="btn btn-sm btn-primary"
            :disabled="!settingsDirty || props.savingConfig"
            @click="saveSettings"
          >
            <Save class="h-4 w-4" />
            {{ props.savingConfig ? t("common.saving") : t("common.save") }}
          </button>
        </div>

        <div class="grid gap-3">
          <div class="rounded-box border border-base-300 bg-base-200/40 p-3">
            <div class="flex items-center justify-between gap-3">
              <div class="min-w-0">
                <div class="text-sm font-medium">{{ t("config.networkAccess.localLink") }}</div>
                <div class="mt-1 break-all font-mono text-xs text-base-content/70">{{ localUrlText }}</div>
              </div>
              <button class="btn btn-sm btn-primary shrink-0" :disabled="!webInfo?.localUrl || !webInfo?.enabled" @click="openLocalUrl">
                <ExternalLink class="h-4 w-4" />
                {{ t("config.networkAccess.open") }}
              </button>
            </div>
          </div>

          <div class="rounded-box border border-base-300 bg-base-200/40 p-3">
            <div class="flex items-end justify-between gap-3">
              <div class="min-w-0">
                <div class="text-sm font-medium">{{ t("config.networkAccess.remotePassword") }}</div>
                <input
                  class="input input-bordered input-sm mt-2 w-full max-w-64 font-mono"
                  :disabled="!networkAccessEnabled"
                  :value="passwordInput"
                  :placeholder="webInfo?.remotePassword || ''"
                  @input="updatePassword"
                />
              </div>
              <div class="flex shrink-0 items-center gap-1">
                <button class="btn btn-sm btn-ghost" :disabled="!networkAccessEnabled" :title="t('config.networkAccess.regeneratePassword')" @click="regeneratePassword">
                  <KeyRound class="h-4 w-4" />
                </button>
                <button class="btn btn-sm btn-ghost" :disabled="!passwordInput || !webInfo?.enabled" @click="copyText(passwordInput)">
                  <Copy class="h-4 w-4" />
                </button>
                <button class="btn btn-sm btn-primary" :disabled="!settingsDirty || props.savingConfig" :title="t('common.save')" @click="saveSettings">
                  <Save class="h-4 w-4" />
                </button>
              </div>
            </div>
          </div>

          <div class="rounded-box border border-base-300 bg-base-200/40 p-3">
            <div class="flex items-center justify-between gap-3">
              <div class="text-sm font-medium">{{ t("config.networkAccess.remoteLinks") }}</div>
              <button class="btn btn-sm btn-ghost shrink-0" :disabled="remoteUrls.length === 0 || !webInfo?.enabled" @click="copyText(remoteUrls.join('\n'))">
                <Copy class="h-4 w-4" />
              </button>
            </div>
            <div v-if="remoteUrls.length > 0" class="mt-2 grid gap-1">
              <div v-for="url in remoteUrls" :key="url" class="break-all font-mono text-xs text-base-content/70">
                {{ url }}
              </div>
            </div>
            <div v-else class="mt-2 text-xs text-base-content/60">
              {{ webInfo?.enabled === false ? t("config.networkAccess.disabled") : t("config.networkAccess.noRemoteLink") }}
            </div>
          </div>
        </div>

        <div v-if="statusText" class="text-xs" :class="statusError ? 'text-error' : 'text-success'">
          {{ statusText }}
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { Copy, ExternalLink, KeyRound, RefreshCw, Save } from "@lucide/vue";
import { invokeTauri } from "../../../../services/tauri-api";
import type { AppConfig } from "../../../../types/app";

type WebAccessInfo = {
  running: boolean;
  enabled: boolean;
  configuredPort: number;
  port: number;
  localUrl: string;
  remoteUrls: string[];
  remotePassword: string;
};

const props = defineProps<{
  config: AppConfig;
  savingConfig: boolean;
  saveConfigAction: () => Promise<boolean> | boolean;
  lastSavedConfigJson: string;
}>();

const { t } = useI18n();
const webInfo = ref<WebAccessInfo | null>(null);
const loading = ref(false);
const statusText = ref("");
const statusError = ref(false);
const portInput = ref("");
const passwordInput = ref("");
const networkAccessEnabled = computed(() => props.config.webAccessEnabled !== false);

const savedPort = computed(() => {
  try {
    const parsed = JSON.parse(String(props.lastSavedConfigJson || "{}")) as Partial<AppConfig>;
    return normalizePort(parsed.webAccessPort);
  } catch {
    return 43129;
  }
});
const savedEnabled = computed(() => {
  try {
    const parsed = JSON.parse(String(props.lastSavedConfigJson || "{}")) as Partial<AppConfig>;
    return parsed.webAccessEnabled !== false;
  } catch {
    return true;
  }
});
const savedPassword = computed(() => {
  try {
    const parsed = JSON.parse(String(props.lastSavedConfigJson || "{}")) as Partial<AppConfig>;
    return String(parsed.webAccessPassword || "").trim();
  } catch {
    return "";
  }
});
const portDirty = computed(() => normalizePort(portInput.value) !== savedPort.value);
const enabledDirty = computed(() => networkAccessEnabled.value !== savedEnabled.value);
const passwordDirty = computed(() => passwordInput.value.trim() !== savedPassword.value);
const settingsDirty = computed(() => portDirty.value || enabledDirty.value || passwordDirty.value);
const remoteUrls = computed(() => webInfo.value?.remoteUrls || []);
const localUrlText = computed(() => webInfo.value?.enabled === false ? t("config.networkAccess.disabled") : (webInfo.value?.localUrl || t("config.networkAccess.waiting")));

function normalizePort(value: unknown): number {
  const port = Math.round(Number(value));
  return Number.isFinite(port) && port >= 1024 && port <= 65535 ? port : 43129;
}

function updatePort(event: Event) {
  const raw = (event.target as HTMLInputElement).value;
  portInput.value = raw;
  const parsed = Math.round(Number(raw));
  if (Number.isFinite(parsed)) props.config.webAccessPort = parsed;
}

function updateEnabled(event: Event) {
  props.config.webAccessEnabled = (event.target as HTMLInputElement).checked;
}

function updatePassword(event: Event) {
  const value = (event.target as HTMLInputElement).value.trim();
  passwordInput.value = value;
  props.config.webAccessPassword = value;
}

function regeneratePassword() {
  const chars = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
  let raw = "";
  for (let index = 0; index < 8; index += 1) {
    raw += chars[Math.floor(Math.random() * chars.length)] || "A";
  }
  passwordInput.value = `${raw.slice(0, 4)}-${raw.slice(4)}`;
  props.config.webAccessPassword = passwordInput.value;
}

async function refreshInfo() {
  loading.value = true;
  statusText.value = "";
  statusError.value = false;
  try {
    webInfo.value = await invokeTauri<WebAccessInfo>("get_web_access_info");
    if (!passwordInput.value.trim() && webInfo.value?.remotePassword) {
      passwordInput.value = webInfo.value.remotePassword;
      props.config.webAccessPassword = webInfo.value.remotePassword;
    }
  } catch (error) {
    statusError.value = true;
    statusText.value = String(error || t("config.networkAccess.refreshFailed"));
  } finally {
    loading.value = false;
  }
}

async function saveSettings() {
  if (!settingsDirty.value) return;
  props.config.webAccessPort = normalizePort(portInput.value);
  portInput.value = String(props.config.webAccessPort);
  props.config.webAccessPassword = passwordInput.value.trim();
  const saved = await Promise.resolve(props.saveConfigAction());
  if (saved !== false) {
    window.setTimeout(() => void refreshInfo(), 500);
  }
}

async function openLocalUrl() {
  const url = webInfo.value?.localUrl || "";
  if (!url) return;
  try {
    await invokeTauri("open_external_url", { url });
  } catch (error) {
    statusError.value = true;
    statusText.value = String(error || t("config.networkAccess.openFailed"));
  }
}

async function copyText(text: string) {
  const value = text.trim();
  if (!value) return;
  try {
    await navigator.clipboard.writeText(value);
    statusError.value = false;
    statusText.value = t("config.networkAccess.copied");
  } catch (error) {
    statusError.value = true;
    statusText.value = String(error || t("config.networkAccess.copyFailed"));
  }
}

onMounted(() => {
  portInput.value = String(normalizePort(props.config.webAccessPort));
  passwordInput.value = String(props.config.webAccessPassword || "").trim();
  void refreshInfo();
});
</script>

<template>
  <div class="flex flex-col gap-4 pb-20 [&_div]:[transition:background-color_200ms,border-color_200ms,box-shadow_200ms,border-radius_200ms_ease-out]">
    <!-- 仪表盘：品牌 + 缺失项提示 + 开始对话，只占一行 -->
    <div class="card bg-base-100 card-border border-base-300 from-base-content/5 bg-linear-to-bl to-50% card-sm overflow-hidden">
      <div class="card-body flex-row flex-wrap items-center gap-2 px-4 py-2.5">
        <!-- 品牌区 -->
        <div class="flex items-center gap-2 pr-1">
          <img :src="appIconUrl" alt="P-ai" class="size-6 rounded" />
          <span class="text-sm font-bold">P-ai</span>
          <span class="text-xs opacity-60" v-if="appVersion">v{{ appVersion }}</span>
        </div>

        <!-- 缺失的运行时依赖（已装的不提示） -->
        <template v-for="dep in missingDeps" :key="dep.kind">
          <span class="badge badge-error gap-1 font-medium">
            <span>{{ dep.label }}</span>
            <span class="opacity-80">{{ t("config.welcome.notInstalled") }}</span>
          </span>
          <button
            class="btn btn-xs btn-primary"
            type="button"
            :disabled="installingPrerequisite !== null"
            @click="installPrerequisite(dep.kind)"
          >
            {{ installingPrerequisite === dep.kind ? t("config.welcome.installing") : t("config.welcome.autoInstall") }}
          </button>
          <span v-if="runtimeInstallStatusError[dep.kind]" class="text-xs text-error">
            {{ runtimeInstallStatus[dep.kind] }}
          </span>
        </template>

      <!-- 未设置的模型分工（点击跳对话设置页） -->
      <button
        v-if="!quickModel"
        class="btn btn-xs btn-outline btn-warning gap-1"
        type="button"
        @click="emit('jump', 'chatSettings')"
      >
        <span>{{ t("config.welcome.cards.quickModel.title") }}</span>
        <span class="opacity-80">{{ t("config.welcome.notSet") }}</span>
      </button>
      <button
        v-if="!expertModel"
        class="btn btn-xs btn-outline btn-warning gap-1"
        type="button"
        @click="emit('jump', 'chatSettings')"
      >
        <span>{{ t("config.welcome.cards.expertModel.title") }}</span>
        <span class="opacity-80">{{ t("config.welcome.notSet") }}</span>
      </button>

      <div class="flex-1" />

      <button class="btn btn-sm btn-primary" type="button" @click="emit('start-chat')">
        <MessageSquare class="h-3.5 w-3.5" />
        {{ t("window.startChat") }}
      </button>
      </div>
    </div>

    <!-- 足迹墙 -->
    <UsageTrailWall />
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { MessageSquare } from "@lucide/vue";
import type { ApiConfigItem, AppConfig } from "../../../../types/app";
import UsageTrailWall from "./UsageTrailWall.vue";
import {
  getTransportHostRuntimePrerequisites,
  installTransportHostRuntimePrerequisite,
  invokeTauri,
  openTransportExternalUrl,
} from "../../../../services/tauri-api";
import { toErrorMessage } from "../../../../utils/error";
import appIconUrl from "../../../../../src-tauri/icons/128x128.png";

type ConfigTab = "welcome" | "hotkey" | "api" | "tools" | "mcp" | "skill" | "persona" | "department" | "departmentTree" | "chatSettings" | "usage" | "memory" | "task" | "logs" | "appearance" | "migration" | "about";
type HostRuntimePrerequisiteKind = "git" | "node" | "rg";
type HostRuntimePrerequisites = {
  gitInstalled?: boolean;
  nodeInstalled?: boolean;
  rgInstalled?: boolean;
};
type HostRuntimePrerequisiteInstallResult = {
  kind: HostRuntimePrerequisiteKind;
  installed: boolean;
  message: string;
};
type MissingDep = {
  kind: HostRuntimePrerequisiteKind;
  label: string;
};

const props = defineProps<{
  config: AppConfig;
}>();

const emit = defineEmits<{
  (e: "jump", value: ConfigTab): void;
  (e: "start-chat"): void;
}>();

const { t } = useI18n();
const GIT_DOWNLOAD_URL = "https://git-scm.com/downloads";
const NODE_DOWNLOAD_URL = "https://nodejs.org/en/download";
const RG_DOWNLOAD_URL = "https://github.com/BurntSushi/ripgrep/releases";

const hostRuntimePrerequisites = ref<HostRuntimePrerequisites>({});
const installingPrerequisite = ref<HostRuntimePrerequisiteKind | null>(null);
const runtimeInstallStatus = ref<Record<string, string>>({});
const runtimeInstallStatusError = ref<Record<string, boolean>>({});
const appVersion = ref("");

function findModel(apiConfigs: ApiConfigItem[], apiConfigId: string | undefined | null) {
  const id = String(apiConfigId || "").trim();
  return id ? apiConfigs.find((api) => api.id === id && api.enableText) ?? null : null;
}

async function loadHostRuntimeState() {
  try {
    hostRuntimePrerequisites.value = await getTransportHostRuntimePrerequisites<HostRuntimePrerequisites>();
  } catch {
    hostRuntimePrerequisites.value = {};
  }
}

onMounted(() => {
  void loadHostRuntimeState();
  void loadAppVersion();
});

async function loadAppVersion() {
  try {
    appVersion.value = await invokeTauri<string>("get_app_version");
  } catch {
    appVersion.value = "";
  }
}

// 已装的不提示，缺失的才出现
const missingDeps = computed<MissingDep[]>(() => {
  const gitInstalled = !!hostRuntimePrerequisites.value.gitInstalled;
  const nodeInstalled = !!hostRuntimePrerequisites.value.nodeInstalled;
  const rgInstalled = !!hostRuntimePrerequisites.value.rgInstalled;
  const items: MissingDep[] = [];
  if (!gitInstalled) items.push({ kind: "git", label: t("config.welcome.cards.git.title") });
  if (!nodeInstalled) items.push({ kind: "node", label: t("config.welcome.cards.node.title") });
  if (!rgInstalled) items.push({ kind: "rg", label: t("config.welcome.cards.ripgrep.title") });
  return items;
});

const quickModel = computed(() => findModel(props.config.apiConfigs || [], props.config.toolReviewApiConfigId));
const expertModel = computed(() => findModel(props.config.apiConfigs || [], props.config.assistantDepartmentApiConfigId));

async function installPrerequisite(kind: HostRuntimePrerequisiteKind) {
  if (installingPrerequisite.value) return;
  installingPrerequisite.value = kind;
  runtimeInstallStatus.value[kind] = t("config.welcome.installing");
  runtimeInstallStatusError.value[kind] = false;
  try {
    const result = await installTransportHostRuntimePrerequisite<HostRuntimePrerequisiteInstallResult>(kind);
    runtimeInstallStatus.value[kind] = result.message || t("config.welcome.installSuccess");
    runtimeInstallStatusError.value[kind] = false;
    await loadHostRuntimeState();
  } catch (error) {
    const err = toErrorMessage(error);
    runtimeInstallStatus.value[kind] = t("config.welcome.installFailedFallback", { err });
    runtimeInstallStatusError.value[kind] = true;
    const fallbackUrl = kind === "git" ? GIT_DOWNLOAD_URL : kind === "node" ? NODE_DOWNLOAD_URL : RG_DOWNLOAD_URL;
    void openTransportExternalUrl(fallbackUrl);
  } finally {
    installingPrerequisite.value = null;
  }
}
</script>

<template>
  <div class="window-shell text-sm">
    <div class="navbar min-h-10 h-10 px-2 bg-base-200 border-b border-base-300" data-tauri-drag-region>
      <div class="flex-1" data-tauri-drag-region>
        <span class="font-semibold text-sm" data-tauri-drag-region>Easy Call AI</span>
      </div>
      <div class="flex-none flex gap-1">
        <button class="btn btn-ghost btn-xs" title="Minimize" @click="minimizeWindow">
          <Minus class="h-3.5 w-3.5" />
        </button>
        <button class="btn btn-ghost btn-xs" title="Maximize" @click="toggleMaximize">
          <Square class="h-3.5 w-3.5" />
        </button>
        <button class="btn btn-error btn-xs text-white" title="Close" @click="closeWindow">
          <X class="h-3.5 w-3.5" />
        </button>
      </div>
    </div>

    <div class="window-content">
      <div class="bg-base-100 min-h-full p-3 md:p-4 flex flex-col gap-2">
          <div class="grid grid-cols-1 gap-2">
            <label class="form-control w-full">
              <div class="label py-1"><span class="label-text text-xs">Hotkey</span></div>
              <input v-model="config.hotkey" class="input input-bordered input-sm w-full" placeholder="Alt+C" />
            </label>
          </div>

          <div class="divider my-1">API配置设置</div>
          <p class="text-xs opacity-80">支持多个API配置，每个配置独立保存 URL / API Key / 模型 / 请求格式。</p>

            <div class="flex items-end gap-2">
              <label class="form-control flex-1">
                <div class="label py-1"><span class="label-text text-xs">当前API配置</span></div>
                <select class="select select-bordered select-sm w-full" v-model="config.selectedApiConfigId">
                  <option v-for="p in config.apiConfigs" :key="p.id" :value="p.id">{{ p.name }}</option>
                </select>
              </label>
              <div class="flex gap-1 mb-[2px]">
                <button class="btn btn-sm btn-square" title="新增API配置" @click="addApiConfig"><Plus class="h-4 w-4" /></button>
                <button class="btn btn-sm btn-square" title="删除当前API配置" :disabled="config.apiConfigs.length <= 1" @click="removeSelectedApiConfig"><Trash2 class="h-4 w-4" /></button>
              </div>
            </div>

            <div v-if="selectedApiConfig" class="grid grid-cols-1 gap-2">
              <label class="form-control w-full">
                <div class="label py-1"><span class="label-text text-xs">名称</span></div>
                <input v-model="selectedApiConfig.name" class="input input-bordered input-sm w-full" placeholder="DeepSeek Prod" />
              </label>

              <label class="form-control w-full">
                <div class="label py-1"><span class="label-text text-xs">请求格式</span></div>
                <select class="select select-bordered select-sm w-full" v-model="selectedApiConfig.requestFormat">
                  <option value="openai">openai</option>
                  <option value="gemini">gemini</option>
                  <option value="deepseek/kimi">deepseek/kimi</option>
                </select>
              </label>

              <div class="form-control w-full">
                <div class="label py-1"><span class="label-text text-xs">多模态开关（前端配置）</span></div>
                <div class="flex flex-wrap gap-2 rounded-lg border border-base-300 bg-base-200 p-2">
                  <label class="label cursor-pointer gap-2 py-0">
                    <span class="label-text text-xs">文本</span>
                    <input v-model="selectedApiConfig.enableText" type="checkbox" class="toggle toggle-primary toggle-sm" />
                  </label>
                  <label class="label cursor-pointer gap-2 py-0">
                    <span class="label-text text-xs">图片</span>
                    <input v-model="selectedApiConfig.enableImage" type="checkbox" class="toggle toggle-primary toggle-sm" />
                  </label>
                  <label class="label cursor-pointer gap-2 py-0">
                    <span class="label-text text-xs">语音</span>
                    <input v-model="selectedApiConfig.enableAudio" type="checkbox" class="toggle toggle-primary toggle-sm" />
                  </label>
                </div>
                <div class="label py-1">
                  <span class="label-text-alt opacity-70 text-[11px]">当前仅保存配置，功能后续接入。</span>
                </div>
              </div>

              <label class="form-control w-full">
                <div class="label py-1"><span class="label-text text-xs">Base URL</span></div>
                <input
                  v-model="selectedApiConfig.baseUrl"
                  class="input input-bordered input-sm w-full"
                  :placeholder="baseUrlReference"
                />
                <div class="label py-1">
                  <span class="label-text-alt opacity-70 text-[11px]">参考：{{ baseUrlReference }}</span>
                </div>
              </label>

              <label class="form-control w-full">
                <div class="label py-1"><span class="label-text text-xs">API Key</span></div>
                <input v-model="selectedApiConfig.apiKey" type="password" class="input input-bordered input-sm w-full" placeholder="sk-..." />
              </label>

              <label class="form-control w-full">
                <div class="label py-1"><span class="label-text text-xs">Model</span></div>
                <div class="flex w-full gap-1">
                  <input
                    v-model="selectedApiConfig.model"
                    class="input input-bordered input-sm flex-1"
                    placeholder="gpt-4o-mini"
                  />
                  <details v-if="modelOptions.length > 0" ref="modelDropdownRef" class="dropdown dropdown-end">
                    <summary class="btn btn-sm btn-square" title="选择已刷新模型">
                      <List class="h-4 w-4" />
                    </summary>
                    <ul class="menu dropdown-content z-[1] mt-1 max-h-56 w-72 overflow-y-auto rounded-box border border-base-300 bg-base-100 p-1 shadow">
                      <li v-for="m in modelOptions" :key="m">
                        <a @click.prevent="pickModel(m)">{{ m }}</a>
                      </li>
                    </ul>
                  </details>
                  <button class="btn btn-sm btn-square" :class="{ loading: refreshingModels }" :disabled="refreshingModels" @click="refreshModels">
                    <RefreshCw class="h-4 w-4" />
                  </button>
                </div>
              </label>
            </div>

            <div class="flex flex-wrap gap-2">
              <button class="btn btn-primary btn-sm gap-2" :class="{ loading: saving }" :disabled="saving" @click="save">
                <Save class="h-4 w-4" />
                <span>保存全部配置</span>
              </button>
              <button class="btn btn-sm gap-2" :class="{ loading: loading }" :disabled="loading" @click="load">
                <RefreshCw class="h-4 w-4" />
                <span>重载配置</span>
              </button>
            </div>

            <div class="divider my-2">聊天</div>

            <label class="form-control w-full">
              <div class="label py-1"><span class="label-text text-xs">向 AI 提问</span></div>
              <input
                v-model="promptText"
                class="input input-bordered input-sm w-full"
                placeholder="帮我总结一下今天的任务"
                @keydown.enter.prevent="sendPrompt"
              />
            </label>

            <div class="flex flex-wrap gap-2">
              <button class="btn btn-primary btn-sm gap-2" :class="{ loading: chatting }" :disabled="chatting" @click="sendPrompt">
                <SendHorizonal class="h-4 w-4" />
                <span>发送</span>
              </button>
            </div>

            <div class="alert py-2 min-h-0" :class="statusClass">
              <span class="text-xs">{{ status }}</span>
            </div>

            <div v-if="answer" class="card bg-base-200 p-3">
              <p class="text-sm whitespace-pre-wrap">{{ answer }}</p>
            </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  List,
  Minus,
  Plus,
  Radar,
  RefreshCw,
  Save,
  SendHorizonal,
  Square,
  Trash2,
  X,
} from "lucide-vue-next";

type ApiConfigItem = {
  id: string;
  name: string;
  requestFormat: string;
  enableText: boolean;
  enableImage: boolean;
  enableAudio: boolean;
  baseUrl: string;
  apiKey: string;
  model: string;
};

type AppConfig = {
  hotkey: string;
  selectedApiConfigId: string;
  apiConfigs: ApiConfigItem[];
};

type ChatInputPayload = {
  text?: string;
  model?: string;
  images?: Array<{ mime: string; bytesBase64: string }>;
  audios?: Array<{ mime: string; bytesBase64: string }>;
};

const appWindow = getCurrentWindow();

const config = reactive<AppConfig>({
  hotkey: "Alt+C",
  selectedApiConfigId: "",
  apiConfigs: [],
});

const loading = ref(false);
const saving = ref(false);
const chatting = ref(false);
const refreshingModels = ref(false);
const status = ref("就绪");
const promptText = ref("");
const answer = ref("");
const modelOptions = ref<string[]>([]);

const modelDropdownRef = ref<HTMLDetailsElement | null>(null);
const baseUrlReference = computed(() => {
  const requestFormat = selectedApiConfig.value?.requestFormat ?? "openai";
  if (requestFormat === "gemini") {
    return "https://generativelanguage.googleapis.com/v1beta/openai";
  }
  if (requestFormat === "deepseek/kimi") {
    return "https://api.deepseek.com/v1";
  }
  return "https://api.openai.com/v1";
});

const selectedApiConfig = computed<ApiConfigItem | null>(() => {
  return config.apiConfigs.find((p) => p.id === config.selectedApiConfigId) ?? null;
});

const statusClass = computed(() => {
  const s = status.value.toLowerCase();
  if (s.includes("failed") || s.includes("error")) return "alert-error";
  if (s.includes("loading") || s.includes("saving") || s.includes("asking") || s.includes("refresh")) return "alert-warning";
  return "alert-success";
});

function createApiConfig(seed = Date.now().toString()): ApiConfigItem {
  return {
    id: `api-config-${seed}`,
    name: `API Config ${config.apiConfigs.length + 1}`,
    requestFormat: "openai",
    enableText: true,
    enableImage: true,
    enableAudio: true,
    baseUrl: "https://api.openai.com/v1",
    apiKey: "",
    model: "gpt-4o-mini",
  };
}

function normalizeConfig(input: AppConfig): AppConfig {
  const apiConfigs = (input.apiConfigs ?? []).length > 0
    ? input.apiConfigs.map((p) => ({
        ...p,
        enableText: p.enableText ?? true,
        enableImage: p.enableImage ?? true,
        enableAudio: p.enableAudio ?? true,
      }))
    : [createApiConfig("default")];
  const selectedApiConfigId = apiConfigs.some((p) => p.id === input.selectedApiConfigId)
    ? input.selectedApiConfigId
    : apiConfigs[0].id;
  return {
    hotkey: input.hotkey || "Alt+C",
    selectedApiConfigId,
    apiConfigs,
  };
}

function applyConfig(input: AppConfig) {
  const normalized = normalizeConfig(input);
  config.hotkey = normalized.hotkey;
  config.selectedApiConfigId = normalized.selectedApiConfigId;
  config.apiConfigs.splice(0, config.apiConfigs.length, ...normalized.apiConfigs);
  modelOptions.value = [];
}

function addApiConfig() {
  const p = createApiConfig();
  config.apiConfigs.push(p);
  config.selectedApiConfigId = p.id;
  status.value = "已添加新配置";
}

function removeSelectedApiConfig() {
  if (config.apiConfigs.length <= 1) {
    status.value = "至少需要保留一个配置";
    return;
  }
  const idx = config.apiConfigs.findIndex((p) => p.id === config.selectedApiConfigId);
  if (idx < 0) {
    return;
  }
  config.apiConfigs.splice(idx, 1);
  config.selectedApiConfigId = config.apiConfigs[0].id;
  status.value = "配置已删除";
}

async function minimizeWindow() {
  await appWindow.minimize();
}

async function toggleMaximize() {
  await appWindow.toggleMaximize();
}

async function closeWindow() {
  await appWindow.close();
}

async function load() {
  loading.value = true;
  status.value = "加载配置中...";
  try {
    const cfg = await invoke<AppConfig>("load_config");
    applyConfig(cfg);
    status.value = "配置已加载";
  } catch (error) {
    status.value = `加载失败: ${String(error)}`;
  } finally {
    loading.value = false;
  }
}

async function save() {
  saving.value = true;
  status.value = "保存配置中...";
  try {
    const saved = await invoke<AppConfig>("save_config", { config: { ...config } });
    applyConfig(saved);
    status.value = "配置已保存";
  } catch (error) {
    status.value = `保存失败: ${String(error)}`;
  } finally {
    saving.value = false;
  }
}

async function refreshModels() {
  if (!selectedApiConfig.value) {
    status.value = "请先选择一个配置";
    return;
  }

  refreshingModels.value = true;
  status.value = "刷新模型中...";
  try {
    const models = await invoke<string[]>("refresh_models", {
      input: {
        baseUrl: selectedApiConfig.value.baseUrl,
        apiKey: selectedApiConfig.value.apiKey,
        requestFormat: selectedApiConfig.value.requestFormat,
      },
    });

    modelOptions.value = models;
    if (models.length > 0 && !models.includes(selectedApiConfig.value.model)) {
      selectedApiConfig.value.model = models[0];
    }

    status.value = `模型列表已刷新 (${models.length} 个)`;
  } catch (error) {
    status.value = `刷新模型失败: ${String(error)}`;
  } finally {
    refreshingModels.value = false;
  }
}

function pickModel(model: string) {
  if (!selectedApiConfig.value) return;
  selectedApiConfig.value.model = model;
  if (modelDropdownRef.value) {
    modelDropdownRef.value.open = false;
  }
}

async function sendPrompt() {
  const text = promptText.value.trim();
  if (!text) {
    status.value = "请先输入内容";
    return;
  }

  chatting.value = true;
  status.value = "发送中...";
  answer.value = "";

  try {
    const payload: ChatInputPayload = {
      text,
      model: selectedApiConfig.value?.model,
    };

    const result = await invoke<string>("chat_with_rig", { payload });
    answer.value = result;
    status.value = "发送成功";
  } catch (error) {
    status.value = `发送失败: ${String(error)}`;
  } finally {
    chatting.value = false;
  }
}

onMounted(load);
</script>

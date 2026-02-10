
<template>
  <div class="window-shell text-sm">
    <div class="navbar min-h-10 h-10 px-2 bg-base-200 border-b border-base-300">
      <div class="flex-1 h-full flex items-center cursor-move select-none" @mousedown.left.prevent="startDrag">
        <span class="font-semibold text-sm">{{ titleText }}</span>
      </div>
      <div class="flex-none flex gap-1" data-tauri-disable-drag-region>
        <template v-if="viewMode === 'chat'">
          <button
            class="btn btn-ghost btn-xs"
            :class="{ 'btn-active': alwaysOnTop }"
            :title="alwaysOnTop ? '取消总在最前' : '总在最前窗口'"
            @click.stop="toggleAlwaysOnTop"
            :disabled="!windowReady"
          >
            <Pin class="h-3.5 w-3.5" />
          </button>
          <button class="btn btn-error btn-xs" title="Close" @click.stop="closeWindow" :disabled="!windowReady"><X class="h-3.5 w-3.5" /></button>
        </template>
        <template v-else>
          <button class="btn btn-ghost btn-xs" title="Minimize" @click.stop="minimizeWindow" :disabled="!windowReady"><Minus class="h-3.5 w-3.5" /></button>
          <button class="btn btn-ghost btn-xs" title="Maximize" @click.stop="toggleMaximize" :disabled="!windowReady"><Square class="h-3.5 w-3.5" /></button>
          <button class="btn btn-error btn-xs" title="Close" @click.stop="closeWindow" :disabled="!windowReady"><X class="h-3.5 w-3.5" /></button>
        </template>
      </div>
    </div>

    <div class="window-content p-3">
      <template v-if="viewMode === 'config'">
        <div class="grid gap-2">
          <div class="tabs tabs-boxed tabs-sm">
            <a class="tab" :class="{ 'tab-active': configTab === 'hotkey' }" @click="configTab = 'hotkey'">快捷键</a>
            <a class="tab" :class="{ 'tab-active': configTab === 'api' }" @click="configTab = 'api'">API配置</a>
            <a class="tab" :class="{ 'tab-active': configTab === 'agent' }" @click="configTab = 'agent'">智能体</a>
            <a class="tab" :class="{ 'tab-active': configTab === 'chatSettings' }" @click="configTab = 'chatSettings'">对话设置</a>
          </div>

          <template v-if="configTab === 'hotkey'">
            <label class="form-control">
              <div class="label py-1"><span class="label-text text-xs">Hotkey</span></div>
              <input v-model="config.hotkey" class="input input-bordered input-sm" placeholder="Alt+C" />
            </label>
            <label class="form-control">
              <div class="label py-1"><span class="label-text text-xs">主题</span></div>
              <button class="btn btn-sm w-full" @click="toggleTheme">
                {{ currentTheme === 'light' ? '🌞 浅色模式' : '🌙 深色模式' }}
              </button>
            </label>
            <div class="flex gap-1">
              <button class="btn btn-primary btn-sm flex-1" :class="{ loading: saving }" @click="saveConfig">保存配置</button>
              <button class="btn btn-sm" :class="{ loading: loading }" @click="loadConfig">重载</button>
            </div>
          </template>

          <template v-else-if="configTab === 'api'">
            <label class="form-control">
              <div class="label py-1"><span class="label-text text-xs">当前API配置</span></div>
              <select v-model="config.selectedApiConfigId" class="select select-bordered select-sm">
                <option v-for="a in config.apiConfigs" :key="a.id" :value="a.id">{{ a.name }}</option>
              </select>
            </label>

            <div v-if="selectedApiConfig" class="grid gap-2">
              <input v-model="selectedApiConfig.name" class="input input-bordered input-sm" placeholder="配置名称" />
              <select v-model="selectedApiConfig.requestFormat" class="select select-bordered select-sm">
                <option value="openai">openai</option>
                <option value="gemini">gemini</option>
                <option value="deepseek/kimi">deepseek/kimi</option>
              </select>
              <input v-model="selectedApiConfig.baseUrl" class="input input-bordered input-sm" :placeholder="baseUrlReference" />
              <input v-model="selectedApiConfig.apiKey" type="password" class="input input-bordered input-sm" placeholder="api key" />
              <div class="flex gap-1">
                <input v-model="selectedApiConfig.model" class="input input-bordered input-sm flex-1" placeholder="model" />
                <button class="btn btn-sm btn-square" :class="{ loading: refreshingModels }" :disabled="refreshingModels" @click="refreshModels"><RefreshCw class="h-4 w-4" /></button>
              </div>
              <div class="flex gap-2">
                <label class="label cursor-pointer gap-1"><span class="label-text text-xs">文本</span><input v-model="selectedApiConfig.enableText" type="checkbox" class="toggle toggle-sm" /></label>
                <label class="label cursor-pointer gap-1"><span class="label-text text-xs">图片</span><input v-model="selectedApiConfig.enableImage" type="checkbox" class="toggle toggle-sm" /></label>
                <label class="label cursor-pointer gap-1"><span class="label-text text-xs">语音</span><input v-model="selectedApiConfig.enableAudio" type="checkbox" class="toggle toggle-sm" /></label>
              </div>
            </div>

            <div class="flex gap-1">
              <button class="btn btn-sm" @click="addApiConfig"><Plus class="h-4 w-4" /></button>
              <button class="btn btn-sm" :disabled="config.apiConfigs.length <= 1" @click="removeSelectedApiConfig"><Trash2 class="h-4 w-4" /></button>
              <button class="btn btn-primary btn-sm flex-1" :class="{ loading: saving }" @click="saveConfig">保存配置</button>
              <button class="btn btn-sm" :class="{ loading: loading }" @click="loadConfig">重载</button>
            </div>
          </template>

          <template v-else-if="configTab === 'agent'">
            <label class="form-control">
              <div class="label py-1"><span class="label-text text-xs">当前智能体</span></div>
              <select v-model="selectedAgentId" class="select select-bordered select-sm">
                <option v-for="a in agents" :key="a.id" :value="a.id">{{ a.name }}</option>
              </select>
            </label>
            <div v-if="selectedAgent" class="grid gap-2">
              <input v-model="selectedAgent.name" class="input input-bordered input-sm" placeholder="智能体名称" />
              <textarea v-model="selectedAgent.systemPrompt" class="textarea textarea-bordered textarea-sm" rows="4" placeholder="系统提示词"></textarea>
            </div>
            <div class="flex gap-1">
              <button class="btn btn-sm" @click="addAgent"><Plus class="h-4 w-4" /></button>
              <button class="btn btn-sm" :disabled="agents.length <= 1" @click="removeSelectedAgent"><Trash2 class="h-4 w-4" /></button>
              <button class="btn btn-primary btn-sm flex-1" @click="saveAgents">保存智能体</button>
            </div>
          </template>

          <template v-else>
            <label class="form-control">
              <div class="label py-1"><span class="label-text text-xs">默认AI配置</span></div>
              <select v-model="config.selectedApiConfigId" class="select select-bordered select-sm">
                <option v-for="a in config.apiConfigs" :key="a.id" :value="a.id">{{ a.name }}</option>
              </select>
            </label>
            <label class="form-control">
              <div class="label py-1"><span class="label-text text-xs">默认智能体</span></div>
              <select v-model="selectedAgentId" class="select select-bordered select-sm">
                <option v-for="a in agents" :key="a.id" :value="a.id">{{ a.name }}</option>
              </select>
            </label>
            <label class="form-control">
              <div class="label py-1"><span class="label-text text-xs">用户称谓</span></div>
              <input v-model="userAlias" class="input input-bordered input-sm" placeholder="用户" />
            </label>
            <div class="flex gap-1">
              <button class="btn btn-primary btn-sm flex-1" :class="{ loading: saving }" @click="saveChatPreferences">保存对话设置</button>
              <button class="btn btn-sm" @click="openCurrentHistory">查看当前未归档记录</button>
            </div>
          </template>
        </div>
      </template>
      <template v-else-if="viewMode === 'chat'">
        <div class="grid gap-2">
          <div class="chat chat-end">
            <div class="chat-header text-[11px] opacity-70 mb-1">{{ userAlias }}</div>
            <div class="chat-bubble max-w-[92%] whitespace-pre-wrap">{{ latestUserText || "..." }}</div>
          </div>
          <div class="chat chat-start">
            <div class="chat-header text-[11px] opacity-70 mb-1">{{ selectedAgent?.name || "助理" }}</div>
            <div class="chat-bubble max-w-[92%] whitespace-pre-wrap">{{ latestAssistantText || "..." }}</div>
          </div>

          <div v-if="clipboardImages.length > 0" class="flex flex-wrap gap-1">
            <div
              v-for="(img, idx) in clipboardImages"
              :key="`${img.mime}-${idx}`"
              class="badge badge-outline gap-1 py-3"
            >
              <ImageIcon class="h-3.5 w-3.5" />
              <span class="text-[11px]">图片{{ idx + 1 }}</span>
              <button class="btn btn-ghost btn-xs btn-square" @click="removeClipboardImage(idx)">
                <X class="h-3 w-3" />
              </button>
            </div>
          </div>

          <textarea
            v-model="chatInput"
            ref="chatTextarea"
            class="textarea textarea-bordered textarea-sm resize-none"
            :style="{ height: textareaHeight + 'px', minHeight: '50px', maxHeight: '600px' }"
            placeholder="输入问题，支持 Ctrl+V 粘贴图片/文本"
            @input="adjustTextareaHeight"
          ></textarea>
          <button class="btn btn-primary btn-sm" :class="{ loading: chatting }" @click="sendChat">发送</button>
          <div v-if="clipboardImages.length > 0" class="text-[11px] opacity-70">已粘贴图片 {{ clipboardImages.length }} 张（发送时自动压缩存储）</div>
        </div>

        <dialog ref="historyDialog" class="modal">
          <div class="modal-box max-w-xl">
            <h3 class="font-semibold text-sm mb-2">当前会话记录（未归档）</h3>
            <div class="max-h-96 overflow-auto space-y-2">
              <div v-for="m in currentHistory" :key="m.id" class="text-xs border border-base-300 rounded p-2">
                <div class="font-semibold uppercase text-[11px]">{{ m.role }}</div>
                <div class="whitespace-pre-wrap">{{ renderMessage(m) }}</div>
              </div>
            </div>
            <div class="modal-action"><button class="btn btn-sm" @click="closeHistory">关闭</button></div>
          </div>
        </dialog>
      </template>

      <template v-else>
        <div class="grid gap-2">
          <button class="btn btn-sm" @click="loadArchives">刷新归档</button>
          <div class="grid grid-cols-1 gap-1 max-h-56 overflow-auto">
            <button v-for="a in archives" :key="a.archiveId" class="btn btn-sm justify-start" @click="selectArchive(a.archiveId)">
              {{ a.archivedAt }} · {{ a.title }}
            </button>
          </div>
          <div class="divider my-1">归档内容</div>
          <div class="max-h-80 overflow-auto space-y-2">
            <div v-for="m in archiveMessages" :key="m.id" class="text-xs border border-base-300 rounded p-2">
              <div class="font-semibold uppercase text-[11px]">{{ m.role }}</div>
              <div class="whitespace-pre-wrap">{{ renderMessage(m) }}</div>
            </div>
          </div>
        </div>
      </template>

      <div v-if="viewMode !== 'chat'" class="alert py-2 mt-2" :class="statusClass"><span class="text-xs">{{ status }}</span></div>
    </div>
  </div>
</template>
<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow, WebviewWindow } from "@tauri-apps/api/window";
import { Image as ImageIcon, Minus, Pin, Plus, RefreshCw, Square, Trash2, X } from "lucide-vue-next";

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

type AgentProfile = {
  id: string;
  name: string;
  systemPrompt: string;
  createdAt: string;
  updatedAt: string;
};

type MessagePart =
  | { type: "text"; text: string }
  | { type: "image"; mime: string; bytesBase64: string }
  | { type: "audio"; mime: string; bytesBase64: string };

type ChatMessage = { id: string; role: string; parts: MessagePart[] };
type ChatSnapshot = {
  conversationId: string;
  latestUser?: ChatMessage;
  latestAssistant?: ChatMessage;
  activeMessageCount: number;
};

type ArchiveSummary = { archiveId: string; archivedAt: string; title: string };
type ChatSettings = { selectedAgentId: string; userAlias: string };

let appWindow: WebviewWindow | null = null;
const viewMode = ref<"chat" | "archives" | "config">("config");

const config = reactive<AppConfig>({ hotkey: "Alt+C", selectedApiConfigId: "", apiConfigs: [] });
const configTab = ref<"hotkey" | "api" | "agent" | "chatSettings">("hotkey");
const currentTheme = ref<"light" | "forest">("light");
const agents = ref<AgentProfile[]>([]);
const selectedAgentId = ref("default-agent");
const userAlias = ref("用户");
const chatInput = ref("");
const chatTextarea = ref<HTMLTextAreaElement | null>(null);
const textareaHeight = ref(50);
const latestUserText = ref("");
const latestAssistantText = ref("");
const currentHistory = ref<ChatMessage[]>([]);
const clipboardImages = ref<Array<{ mime: string; bytesBase64: string }>>([]);

const archives = ref<ArchiveSummary[]>([]);
const archiveMessages = ref<ChatMessage[]>([]);

const windowReady = ref(false);
const status = ref("Ready.");
const loading = ref(false);
const saving = ref(false);
const chatting = ref(false);
const refreshingModels = ref(false);
const historyDialog = ref<HTMLDialogElement | null>(null);
const alwaysOnTop = ref(false);

const titleText = computed(() => (viewMode.value === "chat" ? "Easy Call AI - 对话窗口" : viewMode.value === "archives" ? "Easy Call AI - 归档窗口" : "Easy Call AI - 配置窗口"));
const statusClass = computed(() => {
  const s = status.value.toLowerCase();
  if (s.includes("failed") || s.includes("error")) return "alert-error";
  if (s.includes("loading") || s.includes("saving") || s.includes("sending") || s.includes("refresh")) return "alert-warning";
  return "alert-success";
});
const selectedApiConfig = computed(() => config.apiConfigs.find((a) => a.id === config.selectedApiConfigId) ?? null);
const selectedAgent = computed(() => agents.value.find((a) => a.id === selectedAgentId.value) ?? null);
const baseUrlReference = computed(() => {
  const format = selectedApiConfig.value?.requestFormat ?? "openai";
  if (format === "gemini") return "https://generativelanguage.googleapis.com/v1beta/openai";
  if (format === "deepseek/kimi") return "https://api.deepseek.com/v1";
  return "https://api.openai.com/v1";
});
const chatInputPlaceholder = computed(() => {
  const api = selectedApiConfig.value;
  if (!api) return "输入问题";
  const hints: string[] = [];
  if (api.enableImage) hints.push("Ctrl+V 粘贴图片");
  if (api.enableAudio) hints.push("可发送语音");
  if (hints.length === 0) return "输入问题";
  return `输入问题，${hints.join("，")}`;
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

function renderMessage(msg: ChatMessage): string {
  return msg.parts.map((p) => {
    if (p.type === "text") return p.text;
    if (p.type === "image") return "[image]";
    return "[audio]";
  }).join("\n");
}

async function loadConfig() {
  loading.value = true;
  status.value = "Loading config...";
  try {
    const cfg = await invoke<AppConfig>("load_config");
    config.hotkey = cfg.hotkey;
    config.selectedApiConfigId = cfg.selectedApiConfigId;
    config.apiConfigs.splice(0, config.apiConfigs.length, ...(cfg.apiConfigs.length ? cfg.apiConfigs : [createApiConfig("default")]));
    if (!config.apiConfigs.some((a) => a.id === config.selectedApiConfigId)) config.selectedApiConfigId = config.apiConfigs[0].id;
    status.value = "Config loaded.";
  } catch (e) {
    status.value = `Load failed: ${String(e)}`;
  } finally {
    loading.value = false;
  }
}

async function saveConfig() {
  saving.value = true;
  status.value = "Saving config...";
  try {
    const saved = await invoke<AppConfig>("save_config", { config: { ...config } });
    config.hotkey = saved.hotkey;
    config.selectedApiConfigId = saved.selectedApiConfigId;
    config.apiConfigs.splice(0, config.apiConfigs.length, ...saved.apiConfigs);
    status.value = "Config saved.";
  } catch (e) {
    status.value = `Save failed: ${String(e)}`;
  } finally {
    saving.value = false;
  }
}

async function loadAgents() {
  const list = await invoke<AgentProfile[]>("load_agents");
  agents.value = list;
  if (!agents.value.some((a) => a.id === selectedAgentId.value)) selectedAgentId.value = agents.value[0]?.id ?? "default-agent";
}

async function loadChatSettings() {
  const settings = await invoke<ChatSettings>("load_chat_settings");
  if (agents.value.some((a) => a.id === settings.selectedAgentId)) {
    selectedAgentId.value = settings.selectedAgentId;
  }
  userAlias.value = settings.userAlias?.trim() || "用户";
}

async function saveAgents() {
  try {
    agents.value = await invoke<AgentProfile[]>("save_agents", { input: { agents: agents.value } });
    status.value = "Agents saved.";
  } catch (e) {
    status.value = `Save agents failed: ${String(e)}`;
  }
}

async function saveChatPreferences() {
  saving.value = true;
  status.value = "Saving chat settings...";
  try {
    await saveConfig();
    await invoke("save_chat_settings", { input: { selectedAgentId: selectedAgentId.value, userAlias: userAlias.value } });
    status.value = "Chat settings saved.";
  } catch (e) {
    status.value = `Save chat settings failed: ${String(e)}`;
  } finally {
    saving.value = false;
  }
}

function addApiConfig() {
  const c = createApiConfig();
  config.apiConfigs.push(c);
  config.selectedApiConfigId = c.id;
}

function removeSelectedApiConfig() {
  if (config.apiConfigs.length <= 1) return;
  const idx = config.apiConfigs.findIndex((a) => a.id === config.selectedApiConfigId);
  if (idx >= 0) config.apiConfigs.splice(idx, 1);
  config.selectedApiConfigId = config.apiConfigs[0].id;
}

function addAgent() {
  const id = `agent-${Date.now()}`;
  const now = new Date().toISOString();
  agents.value.push({ id, name: `Agent ${agents.value.length + 1}`, systemPrompt: "", createdAt: now, updatedAt: now });
  selectedAgentId.value = id;
}

function removeSelectedAgent() {
  if (agents.value.length <= 1) return;
  const idx = agents.value.findIndex((a) => a.id === selectedAgentId.value);
  if (idx >= 0) agents.value.splice(idx, 1);
  selectedAgentId.value = agents.value[0].id;
}

async function refreshModels() {
  if (!selectedApiConfig.value) return;
  refreshingModels.value = true;
  try {
    const models = await invoke<string[]>("refresh_models", { input: { baseUrl: selectedApiConfig.value.baseUrl, apiKey: selectedApiConfig.value.apiKey, requestFormat: selectedApiConfig.value.requestFormat } });
    if (models.length) selectedApiConfig.value.model = models[0];
    status.value = `Model list refreshed (${models.length}).`;
  } catch (e) {
    status.value = `Refresh models failed: ${String(e)}`;
  } finally {
    refreshingModels.value = false;
  }
}

async function refreshChatSnapshot() {
  if (!config.selectedApiConfigId || !selectedAgentId.value) return;
  try {
    const snap = await invoke<ChatSnapshot>("get_chat_snapshot", { input: { apiConfigId: config.selectedApiConfigId, agentId: selectedAgentId.value } });
    latestUserText.value = snap.latestUser ? renderMessage(snap.latestUser) : "";
    latestAssistantText.value = snap.latestAssistant ? renderMessage(snap.latestAssistant) : "";
  } catch (e) {
    status.value = `Load chat snapshot failed: ${String(e)}`;
  }
}
async function sendChat() {
  const text = chatInput.value.trim();
  if (!text && clipboardImages.value.length === 0) {
    status.value = "请输入文本或先粘贴图片。";
    return;
  }

  chatting.value = true;
  status.value = "Sending...";
  try {
    const result = await invoke<{ assistantText: string; latestUserText: string; archivedBeforeSend: boolean }>("send_chat_message", {
      input: {
        apiConfigId: config.selectedApiConfigId,
        agentId: selectedAgentId.value,
        payload: {
          text,
          images: clipboardImages.value,
          model: config.apiConfigs.find((a) => a.id === config.selectedApiConfigId)?.model,
        },
      },
    });

    latestUserText.value = result.latestUserText;
    latestAssistantText.value = result.assistantText;
    status.value = result.archivedBeforeSend ? "已自动归档旧会话，并发送到新会话。" : "发送完成。";
    chatInput.value = "";
    clipboardImages.value = [];
  } catch (e) {
    status.value = `Chat failed: ${String(e)}`;
  } finally {
    chatting.value = false;
  }
}

async function openCurrentHistory() {
  try {
    currentHistory.value = await invoke<ChatMessage[]>("get_active_conversation_messages", { input: { apiConfigId: config.selectedApiConfigId, agentId: selectedAgentId.value } });
    historyDialog.value?.showModal();
  } catch (e) {
    status.value = `Load history failed: ${String(e)}`;
  }
}

function closeHistory() {
  historyDialog.value?.close();
}

async function loadArchives() {
  try {
    archives.value = await invoke<ArchiveSummary[]>("list_archives");
    if (archives.value.length > 0) await selectArchive(archives.value[0].archiveId);
  } catch (e) {
    status.value = `Load archives failed: ${String(e)}`;
  }
}

async function selectArchive(archiveId: string) {
  archiveMessages.value = await invoke<ChatMessage[]>("get_archive_messages", { archiveId });
}

function onPaste(event: ClipboardEvent) {
  if (viewMode.value !== "chat") return;
  const items = event.clipboardData?.items;
  if (!items) return;
  const apiConfig = selectedApiConfig.value;
  if (!apiConfig) return;

  const text = event.clipboardData?.getData("text/plain");
  if (text && !chatInput.value.trim() && apiConfig.enableText) chatInput.value = text;

  for (const item of Array.from(items)) {
    if (item.type.startsWith("image/")) {
      if (!apiConfig.enableImage) {
        event.preventDefault();
        return;
      }
      const file = item.getAsFile();
      if (!file) continue;
      const reader = new FileReader();
      reader.onload = () => {
        const result = String(reader.result || "");
        const base64 = result.includes(",") ? result.split(",")[1] : "";
        if (base64) clipboardImages.value.push({ mime: item.type, bytesBase64: base64 });
      };
      reader.readAsDataURL(file);
      event.preventDefault();
    }
  }
}

function removeClipboardImage(index: number) {
  if (index < 0 || index >= clipboardImages.value.length) return;
  clipboardImages.value.splice(index, 1);
}

async function readBlobAsDataUrl(blob: Blob): Promise<string> {
  return await new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(String(reader.result || ""));
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });
}

async function importClipboardImageOnOpen() {
  if (viewMode.value !== "chat") return;
  const apiConfig = selectedApiConfig.value;
  if (!apiConfig) return;

  if (apiConfig.enableText && !chatInput.value.trim() && navigator.clipboard?.readText) {
    try {
      const text = (await navigator.clipboard.readText()).trim();
      if (text) {
        chatInput.value = text;
      }
    } catch {
      // Ignore clipboard text read errors.
    }
  }

  if (!apiConfig.enableImage) return;
  if (!navigator.clipboard?.read) return;

  try {
    const items = await navigator.clipboard.read();
    for (const item of items) {
      const imageType = item.types.find((t) => t.startsWith("image/"));
      if (!imageType) continue;
      const blob = await item.getType(imageType);
      const dataUrl = await readBlobAsDataUrl(blob);
      const base64 = dataUrl.includes(",") ? dataUrl.split(",")[1] : "";
      if (!base64) continue;

      const exists = clipboardImages.value.some(
        (img) => img.mime === imageType && img.bytesBase64 === base64,
      );
      if (!exists) {
        clipboardImages.value.push({ mime: imageType, bytesBase64: base64 });
      }
      break;
    }
  } catch {
    // Clipboard read can fail depending on platform permissions; ignore silently.
  }
}

async function minimizeWindow() { 
  if (!appWindow) return;
  await appWindow.minimize(); 
}
async function toggleMaximize() { 
  if (!appWindow) return;
  await appWindow.toggleMaximize(); 
}
async function closeWindow() { 
  if (!appWindow) return;
  await appWindow.hide(); 
}
async function startDrag() { 
  if (!appWindow) return;
  await appWindow.startDragging(); 
}
async function toggleAlwaysOnTop() {
  if (!appWindow) return;
  alwaysOnTop.value = !alwaysOnTop.value;
  await appWindow.setAlwaysOnTop(alwaysOnTop.value);
}

function toggleTheme() {
  currentTheme.value = currentTheme.value === "light" ? "forest" : "light";
  document.documentElement.setAttribute("data-theme", currentTheme.value);
}

function adjustTextareaHeight() {
  if (!chatTextarea.value) return;
  const textarea = chatTextarea.value;
  textarea.style.height = "auto";
  const newHeight = Math.min(Math.max(textarea.scrollHeight, 50), 600);
  textareaHeight.value = newHeight;
}

onMounted(async () => {
  appWindow = getCurrentWindow();
  viewMode.value = appWindow.label === "chat" ? "chat" : appWindow.label === "archives" ? "archives" : "config";
  windowReady.value = true;

  window.addEventListener("paste", onPaste);
  const refreshAll = async () => {
    await loadConfig();
    await loadAgents();
    await loadChatSettings();
    if (viewMode.value === "chat") {
      await refreshChatSnapshot();
      await importClipboardImageOnOpen();
    } else if (viewMode.value === "archives") {
      await loadArchives();
    }
  };

  await refreshAll();
  if (viewMode.value === "chat") {
    try {
      alwaysOnTop.value = await appWindow.isAlwaysOnTop();
    } catch {
      alwaysOnTop.value = false;
    }
  }
  await listen("easy-call:refresh", async () => {
    await refreshAll();
  });
});
</script>

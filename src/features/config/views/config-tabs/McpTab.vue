<template>
  <SettingsStickyLayout>
    <template #header>
      <div class="flex items-center gap-2">
        <div class="text-sm font-semibold">MCP</div>
        <select
          v-if="servers.length > 0"
          v-model="selectedServerId"
          class="select select-sm select-bordered min-w-0 flex-1"
          :disabled="loading"
        >
          <option v-for="server in servers" :key="server.id" :value="server.id">
            {{ server.name || server.id }}
          </option>
        </select>
        <button class="btn btn-sm bg-base-100 shrink-0" type="button" @click="reloadServers" :disabled="loading">
          <RefreshCw class="h-4 w-4" :class="{ 'animate-spin': loading }" />
          {{ t('config.mcp.refresh') }}
        </button>
        <button v-if="localFileSystemAvailable" class="btn btn-sm bg-base-100 shrink-0" type="button" @click="openMcpDir" :disabled="loading">
          <FolderOpen class="h-4 w-4" />
          {{ t('config.mcp.openDir') }}
        </button>
        <button class="btn btn-sm bg-base-100 shrink-0" type="button" @click="addServer">
          <Plus class="h-4 w-4" />
          {{ t('config.mcp.add') }}
        </button>
      </div>
    </template>

    <div class="grid gap-3">
      <div v-if="loading" class="text-sm opacity-70">{{ t('config.mcp.loading') }}</div>

    <McpServerCard
      v-if="selectedServer"
      :key="selectedServer.id"
      :server="selectedServer"
      :disabled="loading"
      :has-issues="issueList.length > 0"
      @remove="removeServer"
      @validate="validateDefinition"
      @fix="fixDefinition"
      @toggle-deploy="toggleDeploy"
      @toggle-tool="onToggleTool"
      @refresh-tools="refreshTools"
    />

    <div v-if="issueList.length > 0" class="space-y-1">
      <div v-for="(issue, idx) in issueList" :key="idx" class="flex items-start gap-2 text-sm text-error">
        <span class="mt-0.5">•</span>
        <span>{{ issue }}</span>
      </div>
    </div>

    <div v-if="statusText" class="text-sm" :class="statusError ? 'text-error' : 'opacity-70'">
      {{ statusText }}
    </div>

    <div
      v-if="nodeMissing && !nodeInstalling"
      class="card card-border border-warning/40 bg-warning/10 card-sm"
    >
      <div class="card-body flex-row flex-wrap items-center gap-2 px-4 py-3">
        <div class="flex flex-col gap-0.5">
          <span class="text-sm font-medium">{{ t('config.mcp.nodeRequired') }}</span>
          <span class="text-xs opacity-70">{{ t('config.mcp.nodeRequiredHint') }}</span>
        </div>
        <div class="flex-1" />
        <span v-if="nodeInstallError" class="text-xs text-error max-w-56 text-right">{{ nodeInstallError }}</span>
        <button class="btn btn-xs btn-warning" type="button" @click="installNode">
          {{ t('config.mcp.installNode') }}
        </button>
      </div>
    </div>
    <div v-if="nodeInstalling" class="text-sm opacity-70">{{ t('config.mcp.installingNode') }}</div>
  </div>

  </SettingsStickyLayout>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { FolderOpen, Plus, RefreshCw } from "@lucide/vue";
import { getTransportCapabilities, getTransportHostRuntimePrerequisites, installTransportHostRuntimePrerequisite, invokeTauri, openTransportMcpWorkspaceDirectory } from "../../../../services/tauri-api";
import type {
  McpDefinitionValidateResult,
  McpFixDefinitionResult,
  McpListServerToolsResult,
  McpServerConfig,
  McpToolDescriptor,
  McpValidationIssue,
} from "../../../../types/app";
import { toErrorMessage } from "../../../../utils/error";
import McpServerCard from "./mcp/McpServerCard.vue";
import SettingsStickyLayout from "../../components/SettingsStickyLayout.vue";

const { t, te } = useI18n();

type McpServerView = McpServerConfig & {
  toolItems: McpToolDescriptor[];
  lastElapsedMs: number;
  isDraft: boolean;
  isDirty: boolean;
};

const loading = ref(false);
const statusText = ref("");
const statusError = ref(false);
const servers = ref<McpServerView[]>([]);
const selectedServerId = ref("");
const localFileSystemAvailable = getTransportCapabilities().localFileSystem;

const nodeMissing = ref(false);
const nodeInstalling = ref(false);
const nodeInstallError = ref("");

const issueList = ref<string[]>([]);

const selectedServer = computed(() =>
  servers.value.find((s) => s.id === selectedServerId.value) ?? null,
);

function setStatus(text: string, isError = false) {
  statusText.value = text;
  statusError.value = isError;
}

function issueText(issue: McpValidationIssue): string {
  const params: Record<string, string> = {
    serverName: issue.serverName ?? "",
    field: issue.field ?? "",
    index: issue.params?.index ?? "",
    message: issue.message,
  };
  const key = `config.mcp.issues.${issue.code}`;
  if (te(key)) {
    return t(key, params);
  }
  return t("config.mcp.issues.fallback", params);
}

/** 从 definitionJson 解析组内成员名（用于跨卡片重名检测） */
function parseMemberNames(definitionJson: string): string[] {
  try {
    const parsed = JSON.parse(definitionJson) as unknown;
    const names: string[] = [];
    if (Array.isArray(parsed)) {
      for (const item of parsed) {
        if (item && typeof item === "object") {
          names.push(String((item as Record<string, unknown>).name ?? ""));
        }
      }
    } else if (parsed && typeof parsed === "object") {
      const root = parsed as Record<string, unknown>;
      const ms = root.mcpServers;
      if (Array.isArray(ms)) {
        for (const item of ms) {
          if (item && typeof item === "object") {
            names.push(String((item as Record<string, unknown>).name ?? ""));
          }
        }
      } else if (ms && typeof ms === "object") {
        names.push(...Object.keys(ms as Record<string, unknown>));
      } else {
        const hasDirectField = ["command", "url", "transport", "type", "args", "env", "cwd", "headers", "httpHeaders", "envHttpHeaders", "bearerTokenEnvVar", "enabledTools", "disabledTools"].some(
          (key) => key in root,
        );
        if (hasDirectField) {
          // 单 server 直接字段：取 name 字段
          const singleName = String(root.name ?? "");
          if (singleName) names.push(singleName);
        } else {
          names.push(...Object.keys(root));
        }
      }
    }
    return names.filter(Boolean);
  } catch {
    return [];
  }
}

function applyIssues(issues: McpValidationIssue[] | undefined) {
  issueList.value = (issues ?? []).map(issueText);
}

function clearIssues() {
  issueList.value = [];
}

function toView(server: McpServerConfig): McpServerView {
  return {
    ...server,
    toolItems: [],
    lastElapsedMs: 0,
    isDraft: false,
    isDirty: false,
  };
}

function upsertServer(local: McpServerView) {
  const idx = servers.value.findIndex((s) => s.id === local.id);
  if (idx >= 0) {
    servers.value[idx] = {
      ...servers.value[idx],
      ...local,
    };
    return;
  }
  servers.value.unshift(local);
  ensureSelectedServer();
}

function ensureSelectedServer() {
  if (servers.value.length === 0) {
    selectedServerId.value = "";
    return;
  }
  if (!servers.value.some((s) => s.id === selectedServerId.value)) {
    selectedServerId.value = servers.value[0].id;
  }
}

async function reloadServers() {
  loading.value = true;
  try {
    const list = await invokeTauri<McpServerConfig[]>("mcp_list_servers");
    servers.value = list.map(toView);
    ensureSelectedServer();
    const enabledServers = servers.value.filter((s) => s.enabled);
    if (enabledServers.length > 0) {
      const results = await Promise.allSettled(
        enabledServers.map((server) =>
          invokeTauri<McpListServerToolsResult>("mcp_list_server_tools_cached", {
            input: { serverId: server.id },
          }),
        ),
      );
      for (let i = 0; i < enabledServers.length; i++) {
        const target = enabledServers[i];
        const result = results[i];
        if (result.status !== "fulfilled") continue;
        target.toolItems = result.value.tools;
        target.lastElapsedMs = result.value.elapsedMs;
      }
    }
    setStatus(t('config.mcp.loadedCount', { count: servers.value.length }));
  } catch (error) {
    setStatus(`${t('config.mcp.loadFailed')}: ${toErrorMessage(error)}`, true);
  } finally {
    loading.value = false;
  }
}

function addServer() {
  const seed = Date.now();
  const next: McpServerView = {
    id: `mcp-${seed}`,
    name: `MCP ${servers.value.length + 1}`,
    enabled: false,
    definitionJson: '{\n  "name": "mcp-server",\n  "transport": "stdio",\n  "command": "npx",\n  "args": ["-y", "@upstash/context7-mcp"]\n}',
    toolPolicies: [],
    cachedTools: [],
    lastStatus: "",
    lastError: "",
    updatedAt: "",
    toolItems: [],
    lastElapsedMs: 0,
    isDraft: true,
    isDirty: true,
  };
  servers.value.unshift(next);
  selectedServerId.value = next.id;
}

async function removeServer(serverId: string) {
  loading.value = true;
  try {
    await invokeTauri<boolean>("mcp_remove_server", {
      input: { serverId },
    });
    servers.value = servers.value.filter((s) => s.id !== serverId);
    ensureSelectedServer();
    setStatus(t('config.mcp.deleted', { id: serverId }));
  } catch (error) {
    setStatus(`${t('config.mcp.deleteFailed')}: ${toErrorMessage(error)}`, true);
  } finally {
    loading.value = false;
  }
}

async function validateDefinition(server: McpServerView) {
  loading.value = true;
  clearIssues();
  try {
    const result = await invokeTauri<McpDefinitionValidateResult>("mcp_validate_definition", {
      input: {
        definitionJson: server.definitionJson,
        existingMemberNames: servers.value
          .filter((s) => s.id !== server.id)
          .flatMap((s) => parseMemberNames(s.definitionJson)),
      },
    });
    if (!result.ok) {
      applyIssues(result.issues);
      const detailText = result.issues && result.issues.length > 0
        ? ""
        : (Array.isArray(result.details) && result.details.length > 0
          ? ` | ${result.details.join(" ; ")}`
          : "");
      const codeText = result.errorCode ? ` [${result.errorCode}]` : "";
      setStatus(`${t('config.mcp.validateFailed')}${codeText}: ${result.message}${detailText}`, true);
      return;
    }
    const serverCountText = result.serverName
      ? ` (${result.serverName}${result.transport ? `, ${t('config.mcp.transport', { transport: result.transport })}` : ""})`
      : "";
    setStatus(`${t('config.mcp.validateSuccess')}${serverCountText}`);
  } catch (error) {
    setStatus(`${t('config.mcp.validateFailed')}: ${toErrorMessage(error)}`, true);
  } finally {
    loading.value = false;
  }
}

async function fixDefinition(server: McpServerView) {
  loading.value = true;
  clearIssues();
  try {
    const result = await invokeTauri<McpFixDefinitionResult>("mcp_fix_definition", {
      input: { definitionJson: server.definitionJson },
    });
    if (result.fixedDefinitionJson) {
      server.definitionJson = result.fixedDefinitionJson;
    }
    if (result.ok) {
      if (result.fixedDefinitionJson === server.definitionJson && result.issues.length === 0) {
        setStatus(t('config.mcp.fixNoNeed'));
      } else {
        applyIssues(result.issues);
        setStatus(`${t('config.mcp.fixSuccess')}${result.modelName ? `（${result.modelName}）` : ""}`);
      }
      return;
    }
    applyIssues(result.issues);
    setStatus(`${t('config.mcp.fixStillIssues')}${result.modelName ? `（${result.modelName}）` : ""}: ${result.message}`, true);
  } catch (error) {
    setStatus(`${t('config.mcp.fixFailed')}: ${toErrorMessage(error)}`, true);
  } finally {
    loading.value = false;
  }
}

async function checkNodeInstalled(): Promise<boolean> {
  try {
    const prerequisites = await getTransportHostRuntimePrerequisites<{ nodeInstalled?: boolean }>();
    nodeMissing.value = prerequisites.nodeInstalled === false;
    return prerequisites.nodeInstalled === true;
  } catch {
    nodeMissing.value = false;
    return true;
  }
}

async function installNode() {
  if (nodeInstalling.value) return;
  nodeInstalling.value = true;
  nodeInstallError.value = "";
  try {
    await installTransportHostRuntimePrerequisite<{ installed: boolean; message: string }>("node");
    await checkNodeInstalled();
    if (!nodeMissing.value) {
      setStatus(t('config.mcp.nodeInstalled'));
    }
  } catch (error) {
    nodeInstallError.value = toErrorMessage(error);
  } finally {
    nodeInstalling.value = false;
  }
}

async function toggleDeploy(server: McpServerView) {
  loading.value = true;
  try {
    if (server.enabled) {
      const updated = await invokeTauri<McpServerConfig>("mcp_undeploy_server", {
        input: { serverId: server.id },
      });
      upsertServer({
        ...server,
        ...updated,
        toolItems: [],
        lastElapsedMs: 0,
      });
      setStatus(`${t('config.mcp.stopped')}: ${server.name}`);
      return;
    }

    const savedBeforeDeploy = await _saveServerCore(server);
    upsertServer({ ...server, ...savedBeforeDeploy });
    const deployResult = await invokeTauri<McpListServerToolsResult>("mcp_deploy_server", {
      input: { serverId: server.id },
    });
    const saved = await invokeTauri<McpServerConfig[]>("mcp_list_servers");
    const latest = saved.find((s) => s.id === server.id);
    if (latest) {
      upsertServer({
        ...server,
        ...latest,
        toolItems: deployResult.tools,
        lastElapsedMs: deployResult.elapsedMs,
      });
    }
    if (deployResult.tools.length === 0) {
      setStatus(`${t('config.mcp.deploySuccess')}: ${server.name}（${t('config.mcp.probingTools')}）`);
      void pollServerTools(server.id);
    } else {
      setStatus(`${t('config.mcp.deploySuccess')}: ${server.name}（tools=${deployResult.tools.length}）`);
    }
  } catch (error) {
    setStatus(`${t('config.mcp.deployFailed')}: ${toErrorMessage(error)}`, true);
  } finally {
    loading.value = false;
  }
}

// 部署是异步探测：返回为空时轮询运行时状态，探测完成即自动填充工具列表
async function pollServerTools(serverId: string) {
  for (let attempt = 0; attempt < 6; attempt++) {
    await new Promise((resolve) => setTimeout(resolve, 1500));
    try {
      const result = await invokeTauri<McpListServerToolsResult>("mcp_list_server_tools_cached", {
        input: { serverId },
      });
      const target = servers.value.find((s) => s.id === serverId);
      if (target) {
        target.toolItems = result.tools;
        target.lastElapsedMs = result.elapsedMs;
      }
      if (result.tools.length > 0) {
        setStatus(`${t('config.mcp.deploySuccess')}: ${target?.name ?? serverId}（tools=${result.tools.length}）`);
        return;
      }
    } catch {
      return;
    }
  }
}

async function _saveServerCore(server: McpServerView): Promise<McpServerConfig> {
  return invokeTauri<McpServerConfig>("mcp_save_server", {
    input: {
      id: server.id,
      name: server.name,
      enabled: server.enabled,
      definitionJson: server.definitionJson,
    },
  });
}

async function onToggleTool(payload: { serverId: string; toolName: string; enabled: boolean }) {
  loading.value = true;
  try {
    await invokeTauri<McpServerConfig>("mcp_set_tool_enabled", {
      input: payload,
    });
    const server = servers.value.find((s) => s.id === payload.serverId);
    if (server) {
      const tool = server.toolItems.find((t) => t.toolName === payload.toolName);
      if (tool) {
        tool.enabled = payload.enabled;
      }
    }
    setStatus(`${payload.enabled ? t('config.mcp.toolEnabled') : t('config.mcp.toolDisabled')}: ${payload.toolName}`);
  } catch (error) {
    setStatus(`${t('config.mcp.toolSwitchFailed')}: ${toErrorMessage(error)}`, true);
  } finally {
    loading.value = false;
  }
}

async function refreshTools(serverId: string) {
  loading.value = true;
  try {
    const result = await invokeTauri<McpListServerToolsResult>("mcp_list_server_tools_cached", {
      input: { serverId },
    });
    const server = servers.value.find((s) => s.id === serverId);
    if (server) {
      server.toolItems = result.tools;
      server.lastElapsedMs = result.elapsedMs;
    }
    setStatus(t('config.mcp.loadedCount', { count: servers.value.length }));
  } catch (error) {
    setStatus(`${t('config.mcp.loadFailed')}: ${toErrorMessage(error)}`, true);
  } finally {
    loading.value = false;
  }
}

async function openMcpDir() {
  if (!localFileSystemAvailable || loading.value) return;
  loading.value = true;
  try {
    const opened = await openTransportMcpWorkspaceDirectory();
    setStatus(t("config.mcp.openDirOpened", { path: opened }));
  } catch (error) {
    setStatus(t("config.mcp.openDirFailed", { err: toErrorMessage(error) }), true);
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  void reloadServers();
  void checkNodeInstalled();
});
</script>

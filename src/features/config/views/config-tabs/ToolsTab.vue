<template>
  <div class="grid gap-3">
    <div class="flex items-center justify-between gap-3">
      <h3 class="text-sm font-semibold">{{ t('config.tools.shellWorkspace') }}</h3>
      <div class="flex flex-wrap items-center justify-end gap-2">
        <button v-if="localFileSystemAvailable" class="btn btn-sm bg-base-100" type="button" @click="openShellWorkspaceDir">
          <FolderOpen class="h-4 w-4" />
          {{ t('config.tools.openDir') }}
        </button>
        <button class="btn btn-sm bg-base-100" type="button" :disabled="shellWorkspacePathResetting" @click="resetShellWorkspacePath">
          <RotateCcw class="h-4 w-4" />
          {{ t('config.tools.resetWorkspacePath') }}
        </button>
        <button class="btn btn-sm bg-base-100" type="button" :disabled="shellWorkspaceInitializing" @click="initializeShellWorkspace">
          <FolderPlus class="h-4 w-4" />
          {{ t('config.tools.initializeWorkspace') }}
        </button>
        <button class="btn btn-sm btn-primary" :disabled="savingConfig" @click="$emit('saveApiConfig')">
          <Save class="h-4 w-4" />
          {{ t('config.tools.save') }}
        </button>
      </div>
    </div>

    <div class="card bg-base-100 border border-base-300">
      <div class="grid gap-3 p-4">
        <div v-for="(ws, index) in config.shellWorkspaces" :key="`ws-${index}-${ws.name}`">
          <div class="mb-3">
            <input v-model.trim="ws.name" class="input input-bordered input-sm w-full" :placeholder="t('config.tools.workspaceName')" />
          </div>
          <div class="flex items-center gap-2">
            <input v-model.trim="ws.path" class="input input-bordered input-sm flex-1 font-mono" :placeholder="t('config.tools.directoryPath')" />
            <button class="btn btn-sm btn-neutral" type="button" @click="pickWorkspacePath(index)">{{ t('config.tools.modifyWorkspaceDir') }}</button>
          </div>
        </div>
      </div>
      <div class="px-4 pb-4 text-xs opacity-70">
        {{ t('config.tools.workspaceHint') }}
      </div>
      <div v-if="shellWorkspaceStatus" class="px-4 pb-4 text-xs" :class="shellWorkspaceStatusError ? 'text-error' : 'opacity-70'">
        {{ shellWorkspaceStatus }}
      </div>
    </div>

    <div>
      <div class="mb-3">
        <h3 class="text-sm font-semibold">{{ t("config.tools.systemCatalogTitle") }}</h3>
        <div class="mt-1 text-sm opacity-60">{{ t("config.tools.systemCatalogReadonly") }}</div>
      </div>

      <div class="border border-base-300 rounded-box bg-base-100 overflow-hidden">
        <div v-if="toolDefinitions.length" class="divide-y divide-base-300/60">
        <div
          v-for="item in toolDefinitions"
          :key="item.function.name"
          class="px-4 py-3"
        >
          <div class="min-w-0">
            <div class="font-medium">{{ item.function.name }}</div>
            <div class="text-xs opacity-60 whitespace-pre-wrap">{{ item.function.description || t("config.mcpToolList.noDescription") }}</div>
            <div v-if="toolParameterSummary(item.function.name).length" class="mt-1 flex flex-wrap gap-1">
              <span
                v-for="paramText in toolParameterSummary(item.function.name)"
                :key="`${item.function.name}-param-${paramText}`"
                class="text-caption px-1.5 py-0.5 rounded bg-base-200 border border-base-300/70 opacity-80"
              >
                {{ paramText }}
              </span>
            </div>
            <div v-if="toolParameterExamples(item.function.name).length" class="mt-1 grid gap-1">
              <pre
                v-for="example in toolParameterExamples(item.function.name)"
                :key="`${item.function.name}-example-${example}`"
                class="text-caption leading-4 px-2 py-1 rounded bg-base-200 border border-base-300/70 opacity-90 whitespace-pre-wrap overflow-x-auto"
              >{{ example }}</pre>
            </div>
          </div>
        </div>
      </div>
      <div v-else class="text-sm opacity-50 text-center py-4">{{ t("config.mcpToolList.empty") }}</div>
      </div>
    </div>

    <dialog ref="initializeWorkspaceDialog" class="modal">
      <div class="modal-box max-w-md p-4">
        <h3 class="text-sm font-semibold">{{ t("config.tools.initializeWorkspace") }}</h3>
        <p class="mt-3 text-sm whitespace-pre-wrap">{{ t("config.tools.initializeWorkspaceConfirm") }}</p>
        <div class="modal-action mt-4">
          <button class="btn btn-sm btn-ghost" type="button" @click="cancelInitializeWorkspace">
            {{ t("common.cancel") }}
          </button>
          <button class="btn btn-sm btn-primary" type="button" @click="confirmInitializeWorkspace">
            {{ t("common.confirm") }}
          </button>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button aria-label="close" @click="cancelInitializeWorkspace">close</button>
      </form>
    </dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { FolderOpen, FolderPlus, RotateCcw, Save } from "@lucide/vue";
import type {
  AppConfig,
  FrontendToolDefinition,
  ToolLoadStatus,
} from "../../../../types/app";
import {
  getTransportCapabilities,
  getTransportDefaultChatShellWorkspacePath,
  invokeTauri,
  openTransportFileDialog,
  openTransportWorkspaceDirectory,
  resetTransportChatShellWorkspace,
} from "../../../../services/tauri-api";
import { toErrorMessage } from "../../../../utils/error";

const props = defineProps<{
  config: AppConfig;
  toolStatuses: ToolLoadStatus[];
  savingConfig: boolean;
}>();

const emit = defineEmits<{
  (e: "saveApiConfig"): void;
  (e: "refreshToolStatuses"): void;
}>();

const { t } = useI18n();
const toolDefinitions = ref<FrontendToolDefinition[]>([]);
const shellWorkspaceInitializing = ref(false);
const shellWorkspacePathResetting = ref(false);
const shellWorkspaceStatus = ref("");
const shellWorkspaceStatusError = ref(false);
const initializeWorkspaceDialog = ref<HTMLDialogElement | null>(null);
let resolveInitializeWorkspaceConfirm: ((value: boolean) => void) | null = null;
const localFileSystemAvailable = getTransportCapabilities().localFileSystem;
function setShellWorkspaceStatus(text: string, isError = false) {
  shellWorkspaceStatus.value = text;
  shellWorkspaceStatusError.value = isError;
}

async function loadToolCatalog() {
  try {
    const list = await invokeTauri<FrontendToolDefinition[]>("list_tool_catalog");
    toolDefinitions.value = Array.isArray(list) ? list : [];
  } catch {
    toolDefinitions.value = [];
  }
}

async function openShellWorkspaceDir() {
  if (!localFileSystemAvailable) return;
  try {
    const opened = await openTransportWorkspaceDirectory(props.config.shellWorkspaces[0]?.path || "");
    setShellWorkspaceStatus(t("config.tools.openDirOpened", { path: opened }));
  } catch (error) {
    setShellWorkspaceStatus(t("config.tools.openDirFailed", { err: toErrorMessage(error) }), true);
  }
}

async function initializeShellWorkspace() {
  if (shellWorkspaceInitializing.value) return;
  const confirmed = await requestInitializeWorkspaceConfirm();
  if (!confirmed) return;
  shellWorkspaceInitializing.value = true;
  try {
    const root = await resetTransportChatShellWorkspace(props.config.shellWorkspaces[0]?.path || "");
    setShellWorkspaceStatus(t("config.tools.initializeWorkspaceDone", { path: root }));
  } catch (error) {
    setShellWorkspaceStatus(t("config.tools.initializeWorkspaceFailed", { err: toErrorMessage(error) }), true);
  } finally {
    shellWorkspaceInitializing.value = false;
  }
}

function requestInitializeWorkspaceConfirm(): Promise<boolean> {
  const dialog = initializeWorkspaceDialog.value;
  if (!dialog) return Promise.resolve(false);
  return new Promise<boolean>((resolve) => {
    resolveInitializeWorkspaceConfirm = resolve;
    dialog.showModal();
  });
}

function finishInitializeWorkspaceConfirm(value: boolean) {
  const dialog = initializeWorkspaceDialog.value;
  if (dialog?.open) {
    dialog.close();
  }
  resolveInitializeWorkspaceConfirm?.(value);
  resolveInitializeWorkspaceConfirm = null;
}

function confirmInitializeWorkspace() {
  finishInitializeWorkspaceConfirm(true);
}

function cancelInitializeWorkspace() {
  finishInitializeWorkspaceConfirm(false);
}

async function resetShellWorkspacePath() {
  if (shellWorkspacePathResetting.value) return;
  shellWorkspacePathResetting.value = true;
  try {
    const defaultPath = await getTransportDefaultChatShellWorkspacePath();
    if (!Array.isArray(props.config.shellWorkspaces) || props.config.shellWorkspaces.length === 0) {
      props.config.shellWorkspaces = [{
        id: "system-workspace",
        name: defaultWorkspaceNameFromPath(defaultPath) || "默认会话目录",
        path: defaultPath,
        level: "system",
        access: "full_access",
        builtIn: true,
      }];
    } else {
      const target = props.config.shellWorkspaces[0];
      target.id = String(target.id || "").trim() || "system-workspace";
      target.path = defaultPath;
      target.level = "system";
      target.access = "full_access";
      if (!String(target.name || "").trim()) {
        target.name = defaultWorkspaceNameFromPath(defaultPath) || "默认会话目录";
      }
      target.builtIn = true;
    }
    setShellWorkspaceStatus(t("config.tools.resetWorkspacePathDone", { path: defaultPath }));
  } catch (error) {
    setShellWorkspaceStatus(t("config.tools.resetWorkspacePathFailed", { err: toErrorMessage(error) }), true);
  } finally {
    shellWorkspacePathResetting.value = false;
  }
}

function defaultWorkspaceNameFromPath(path: string): string {
  const raw = String(path || "").trim();
  if (!raw) return "";
  const normalized = raw.replace(/\\/g, "/").replace(/\/+$/, "");
  const part = normalized.split("/").pop() || "";
  return part.trim();
}

async function pickWorkspacePath(index: number) {
  const item = props.config.shellWorkspaces[index];
  if (!item) return;
  const picked = await openTransportFileDialog({
    directory: true,
    multiple: false,
    defaultPath: item.path || undefined,
  });
  if (!picked || Array.isArray(picked)) return;
  item.path = String(picked);
  if (!String(item.name || "").trim()) {
    item.name = defaultWorkspaceNameFromPath(item.path) || `workspace-${index + 1}`;
  }
}

function definitionById(id: string): FrontendToolDefinition | undefined {
  return toolDefinitions.value.find((item) => item.function?.name === id);
}

type ToolSchemaShape = Record<string, unknown>;

function asToolSchemaShape(value: unknown): ToolSchemaShape {
  return value && typeof value === "object" ? (value as ToolSchemaShape) : {};
}

function toolSchemaTypeText(shape: ToolSchemaShape): string {
  if (shape.const !== undefined && shape.const !== null) {
    return String(shape.const);
  }
  const typeValue = shape.type;
  if (Array.isArray(typeValue)) {
    return typeValue.map(String).join(" | ");
  }
  return String(typeValue || "any");
}

function toolSchemaSummaryLine(name: string, shape: ToolSchemaShape, required: boolean): string {
  const requiredText = required ? "*" : "";
  const enumValues = Array.isArray(shape.enum) ? ` [${shape.enum.map(String).join(", ")}]` : "";
  const minText = shape.minimum !== undefined ? ` >= ${shape.minimum}` : "";
  const maxText = shape.maximum !== undefined ? ` <= ${shape.maximum}` : "";
  const rangeText = `${enumValues}${minText}${maxText}`.trim();
  const descriptionText = typeof shape.description === "string" ? shape.description.trim() : "";
  return `${requiredText}${name}: ${toolSchemaTypeText(shape)}${rangeText ? ` ${rangeText}` : ""}${descriptionText ? ` - ${descriptionText}` : ""}`;
}

function collectToolSchemaSummaryLines(
  properties: ToolSchemaShape,
  requiredRaw: string[],
  prefix = "",
): string[] {
  const lines: string[] = [];
  for (const [name, schema] of Object.entries(properties)) {
    const shape = asToolSchemaShape(schema);
    const path = prefix ? `${prefix}.${name}` : name;
    lines.push(toolSchemaSummaryLine(path, shape, requiredRaw.includes(name)));

    const nestedPropertiesRaw = shape.properties;
    if (nestedPropertiesRaw && typeof nestedPropertiesRaw === "object") {
      const nestedRequired = Array.isArray(shape.required) ? shape.required.map(String) : [];
      lines.push(
        ...collectToolSchemaSummaryLines(
          nestedPropertiesRaw as ToolSchemaShape,
          nestedRequired,
          path,
        ),
      );
    }

    const itemsShape = asToolSchemaShape(shape.items);
    const itemPropertiesRaw = itemsShape.properties;
    if (itemPropertiesRaw && typeof itemPropertiesRaw === "object") {
      const nestedRequired = Array.isArray(itemsShape.required) ? itemsShape.required.map(String) : [];
      lines.push(
        ...collectToolSchemaSummaryLines(
          itemPropertiesRaw as ToolSchemaShape,
          nestedRequired,
          `${path}[]`,
        ),
      );
    }
  }
  return lines;
}

function formatSchemaExample(value: unknown): string {
  if (typeof value === "string") {
    return value.trim();
  }
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

function collectToolSchemaExamples(
  properties: ToolSchemaShape,
  prefix = "",
): string[] {
  const examples: string[] = [];
  for (const [name, schema] of Object.entries(properties)) {
    const shape = asToolSchemaShape(schema);
    const path = prefix ? `${prefix}.${name}` : name;

    const singleExample = shape.example;
    if (singleExample !== undefined && singleExample !== null) {
      const text = formatSchemaExample(singleExample);
      if (text) {
        examples.push(`${path} 示例:\n${text}`);
      }
    }

    const exampleList = Array.isArray(shape.examples) ? shape.examples : [];
    for (const rawExample of exampleList) {
      if (rawExample === undefined || rawExample === null) continue;
      const text = formatSchemaExample(rawExample);
      if (text) {
        examples.push(`${path} 示例:\n${text}`);
      }
    }

    const nestedPropertiesRaw = shape.properties;
    if (nestedPropertiesRaw && typeof nestedPropertiesRaw === "object") {
      examples.push(...collectToolSchemaExamples(nestedPropertiesRaw as ToolSchemaShape, path));
    }

    const itemsShape = asToolSchemaShape(shape.items);
    const itemPropertiesRaw = itemsShape.properties;
    if (itemPropertiesRaw && typeof itemPropertiesRaw === "object") {
      examples.push(...collectToolSchemaExamples(itemPropertiesRaw as ToolSchemaShape, `${path}[]`));
    }
  }
  return examples;
}

function toolParameterSummary(id: string): string[] {
  const definition = definitionById(id);
  const parameters = definition?.function?.parameters;
  if (!parameters || typeof parameters !== "object") return [];
  const root = parameters as Record<string, unknown>;
  const branches = Array.isArray(root.oneOf) ? root.oneOf : [];
  if (branches.length) {
    return Array.from(new Set(
      branches.flatMap((branch) => {
        const shape = asToolSchemaShape(branch);
        const propertiesRaw = shape.properties;
        const requiredRaw = Array.isArray(shape.required) ? shape.required : [];
        if (!propertiesRaw || typeof propertiesRaw !== "object") return [];
        return collectToolSchemaSummaryLines(
          propertiesRaw as ToolSchemaShape,
          requiredRaw.map(String),
        );
      }),
    ));
  }
  const propertiesRaw = root.properties;
  const requiredRaw = Array.isArray(root.required) ? root.required : [];
  if (!propertiesRaw || typeof propertiesRaw !== "object") return [];
  return collectToolSchemaSummaryLines(
    propertiesRaw as ToolSchemaShape,
    requiredRaw.map(String),
  );
}

function toolParameterExamples(id: string): string[] {
  const definition = definitionById(id);
  const parameters = definition?.function?.parameters;
  if (!parameters || typeof parameters !== "object") return [];
  const root = parameters as Record<string, unknown>;
  const branches = Array.isArray(root.oneOf) ? root.oneOf : [];
  if (branches.length) {
    return Array.from(new Set(
      branches.flatMap((branch) => {
        const shape = asToolSchemaShape(branch);
        const propertiesRaw = shape.properties;
        if (!propertiesRaw || typeof propertiesRaw !== "object") return [];
        return collectToolSchemaExamples(propertiesRaw as ToolSchemaShape);
      }),
    ));
  }
  const propertiesRaw = root.properties;
  if (!propertiesRaw || typeof propertiesRaw !== "object") return [];
  return Array.from(new Set(collectToolSchemaExamples(propertiesRaw as ToolSchemaShape)));
}

onMounted(() => {
  emit("refreshToolStatuses");
  void loadToolCatalog();
});
</script>

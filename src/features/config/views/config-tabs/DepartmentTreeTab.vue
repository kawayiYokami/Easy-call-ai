<template>
  <SettingsStickyLayout>
    <template #header>
      <div class="flex w-full flex-col gap-3">
        <div class="flex items-center justify-between">
          <span class="text-sm font-semibold">{{ t("config.departmentTree.title") }}</span>
          <button
            class="btn btn-sm bg-base-100"
            type="button"
            :title="t('config.departmentTree.managePersonas')"
            @click="emit('openPersonaPage')"
          >
            <User class="h-3.5 w-3.5" />
            {{ t("config.departmentTree.managePersonas") }}
          </button>
        </div>

        <div class="flex gap-1">
          <select
            :value="selectedDepartmentId"
            class="select select-bordered select-sm flex-1"
            @change="switchSelectedDepartment(($event.target as HTMLSelectElement).value)"
          >
            <option v-for="department in sortedDepartments" :key="department.id" :value="department.id">
              {{ department.name }}
            </option>
          </select>

          <button
            class="btn btn-sm btn-square btn-ghost"
            type="button"
            :title="t('common.reset')"
            :disabled="!relationDirty || savingConfig"
            @click="restoreDraftsFromSaved"
          >
            <RotateCcw class="h-3.5 w-3.5" />
          </button>

          <button
            class="btn btn-sm btn-square"
            type="button"
            :class="relationDirty ? 'btn-primary' : 'btn-ghost'"
            :disabled="!selectedDepartment || !!relationValidationMessage || !relationDirty || savingConfig"
            :title="savingConfig ? t('config.api.saving') : relationDirty ? t('common.save') : t('status.configSaved')"
            @click="saveDepartmentRelations"
          >
            <Save v-if="!savingConfig" class="h-3.5 w-3.5" />
            <span v-else class="loading loading-spinner loading-sm"></span>
          </button>
        </div>

        <div class="text-sm opacity-60">{{ t("config.departmentTree.hint") }}</div>
      </div>
    </template>

    <div class="flex min-h-full flex-col gap-3">
      <div class="overflow-hidden rounded-box border border-base-300 bg-base-100">
        <div v-if="relationValidationMessage" class="border-b border-warning/30 bg-warning/10 px-4 py-3 text-sm text-warning-content">
          {{ relationValidationMessage }}
        </div>

        <div class="px-4 py-4">
          <div class="mb-3 flex items-center justify-between gap-3">
            <div class="text-sm font-medium">{{ t("config.departmentTree.directChildren") }}</div>
            <div class="text-xs opacity-50">{{ selectedChildIds.length }} / {{ candidateDepartments.length }}</div>
          </div>
          <div class="mb-3 text-sm opacity-60">
            {{ t("config.departmentTree.directChildrenHint") }}
          </div>

          <div v-if="!selectedDepartment" class="text-sm opacity-60">
            {{ t("config.department.selectHint") }}
          </div>
          <div v-else-if="candidateDepartments.length === 0" class="text-sm opacity-60">
            {{ t("config.departmentTree.noCandidateChildren") }}
          </div>
          <div v-else class="flex flex-wrap gap-2">
            <button
              v-for="department in candidateDepartments"
              :key="department.id"
              type="button"
              class="inline-flex h-9 max-w-full items-center gap-2 rounded-lg border px-3 text-left transition"
              :class="selectedChildIds.includes(department.id)
                ? 'border-primary/50 bg-primary/10 text-base-content shadow-sm'
                : 'border-base-content/10 bg-base-100 text-base-content hover:border-base-content/20'"
              @click="toggleChildDepartment(department.id)"
            >
              <span
                class="flex h-4 w-4 shrink-0 items-center justify-center rounded border transition"
                :class="selectedChildIds.includes(department.id)
                  ? 'border-primary bg-primary text-primary-content'
                  : 'border-base-content/20 bg-base-200 text-transparent'"
              >
                <Check class="h-3 w-3" />
              </span>
              <span
                class="flex h-6 w-6 shrink-0 items-center justify-center overflow-hidden rounded-full bg-base-200 text-caption font-semibold text-base-content/70"
              >
                <img
                  v-if="departmentAvatarInfo(department).avatarUrl"
                  :src="departmentAvatarInfo(department).avatarUrl"
                  :alt="department.name"
                  class="h-full w-full object-cover"
                />
                <span v-else>{{ departmentAvatarInfo(department).avatarText }}</span>
              </span>
              <span class="truncate text-sm font-medium">{{ department.name }}</span>
            </button>
          </div>
        </div>
      </div>

      <Teleport to="body" :disabled="!isFlowFullscreen">
        <div
          class="department-tree-graph-host"
          :class="isFlowFullscreen
            ? 'fixed inset-0 z-[120] flex items-center justify-center bg-base-300/45 p-4 backdrop-blur-sm'
            : 'flex min-h-[560px] flex-1'"
          @click.self="closeFlowFullscreen"
        >
          <div
            :class="isFlowFullscreen
              ? 'flex h-[94vh] w-[94vw] max-w-none flex-col overflow-hidden rounded-box border border-base-300 bg-base-100 shadow-2xl'
              : 'department-tree-graph-card flex min-h-[560px] flex-1 flex-col overflow-hidden rounded-box border border-base-300 bg-base-100'"
          >
            <div v-if="isFlowFullscreen" class="flex items-center justify-between border-b border-base-300 px-4 py-3">
              <div class="text-sm font-medium">{{ t("config.departmentTree.title") }}</div>
              <button
                class="btn btn-sm btn-square bg-base-200"
                type="button"
                :title="t('config.departmentTree.exitFullscreen')"
                @click="closeFlowFullscreen"
              >
                <Minimize2 class="h-3.5 w-3.5" />
              </button>
            </div>

            <VueFlow
              v-model:nodes="flowNodes"
              v-model:edges="flowEdges"
              class="department-tree-flow min-h-0 w-full flex-1"
              :min-zoom="0.35"
              :max-zoom="1.8"
              :nodes-draggable="true"
              :nodes-connectable="true"
              :elements-selectable="true"
              :edges-updatable="false"
              :fit-view-on-init="false"
              :default-viewport="{ x: 0, y: 0, zoom: 1 }"
              @init="handleFlowInit"
              @connect="handleConnect"
              @node-click="handleNodeClick"
              @edge-click="handleEdgeClick"
            >
              <Background :gap="24" :size="1" pattern-color="color-mix(in srgb, currentColor 12%, transparent)" />
              <Controls position="bottom-left" :show-interactive="false">
                <ControlButton :title="t('config.departmentTree.autoLayout')" @click="handleAutoLayout">
                  <RotateCcw class="h-3.5 w-3.5" />
                </ControlButton>
                <ControlButton
                  :title="isFlowFullscreen ? t('config.departmentTree.exitFullscreen') : t('config.departmentTree.openFullscreen')"
                  @click="isFlowFullscreen ? closeFlowFullscreen() : openFlowFullscreen()"
                >
                  <Minimize2 v-if="isFlowFullscreen" class="h-3.5 w-3.5" />
                  <Maximize2 v-else class="h-3.5 w-3.5" />
                </ControlButton>
              </Controls>

              <template #node-department="nodeProps">
                <div
                  class="department-tree-node rounded-2xl border bg-base-100 px-4 py-3 shadow-sm transition"
                  :class="nodeProps.id === selectedDepartmentId ? 'border-primary bg-primary/5 shadow-md' : 'border-base-300 hover:border-base-content/20'"
                >
                  <Handle type="target" :position="Position.Top" class="!h-2 !w-2 !border-0 !bg-base-content/30" />
                  <div class="flex items-center gap-2.5">
                    <div
                      class="flex h-9 w-9 shrink-0 items-center justify-center overflow-hidden rounded-full bg-base-200 text-xs font-semibold text-base-content/70"
                    >
                      <img
                        v-if="nodeProps.data.avatarUrl"
                        :src="nodeProps.data.avatarUrl"
                        :alt="nodeProps.data.name"
                        class="h-full w-full object-cover"
                      />
                      <span v-else>{{ nodeProps.data.avatarText }}</span>
                    </div>
                    <div class="min-w-0 flex-1">
                      <div class="flex items-center justify-between gap-2">
                        <div class="truncate text-sm font-semibold">{{ nodeProps.data.name }}</div>
                        <span v-if="nodeProps.data.isBuiltInAssistant" class="badge badge-xs badge-primary">
                          {{ t("config.department.assistantBadge") }}
                        </span>
                      </div>
                      <div
                        class="mt-1 truncate text-xs"
                        :class="nodeProps.data.hasPersona ? 'opacity-65' : 'opacity-45'"
                      >
                        {{ nodeProps.data.personaText }}
                      </div>
                    </div>
                  </div>
                  <Handle type="source" :position="Position.Bottom" class="!h-2 !w-2 !border-0 !bg-base-content/30" />
                </div>
              </template>
            </VueFlow>
          </div>
        </div>
      </Teleport>
    </div>
  </SettingsStickyLayout>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import dagre from "@dagrejs/dagre";
import { Background } from "@vue-flow/background";
import { ControlButton, Controls } from "@vue-flow/controls";
import { Handle, MarkerType, Position, VueFlow, type Connection, type Edge as FlowEdge, type Node as FlowNode, type VueFlowStore } from "@vue-flow/core";
import "@vue-flow/core/dist/style.css";
import "@vue-flow/core/dist/theme-default.css";
import "@vue-flow/controls/dist/style.css";
import { Check, Maximize2, Minimize2, RotateCcw, Save, User } from "@lucide/vue";
import type { AppConfig, DepartmentConfig, PersonaProfile } from "../../../../types/app";
import SettingsStickyLayout from "../../components/SettingsStickyLayout.vue";
import {
  departmentAncestorIds,
  departmentDirectChildIds,
  normalizeDepartmentChildIds,
} from "../../utils/department-graph";
import { validateDepartmentConfig } from "../../utils/department-validation";
import { useAvatarCache } from "../../../chat/composables/use-avatar-cache";

const props = defineProps<{
  config: AppConfig;
  personas: PersonaProfile[];
  savingConfig: boolean;
  saveConfigAction: () => Promise<boolean> | boolean;
  setStatusAction: (text: string) => void;
}>();

const emit = defineEmits<{
  (e: "openPersonaPage"): void;
}>();

const { t } = useI18n();
const selectedDepartmentId = ref("assistant-department");

type DepartmentRelationDraft = {
  id: string;
  childDepartmentIds: string[];
};
type DepartmentFlowNodeData = {
  name: string;
  personaText: string;
  hasPersona: boolean;
  isBuiltInAssistant: boolean;
  avatarUrl: string;
  avatarText: string;
};
type DepartmentFlowNode = FlowNode<DepartmentFlowNodeData>;
type DepartmentFlowEdge = FlowEdge;
let lastFlowLayoutSignature = "";

const { resolveAvatarUrl, preloadPersonaAvatars } = useAvatarCache({
  personas: computed(() => props.personas),
});

function cloneRelationDrafts(departments: DepartmentConfig[] | null | undefined): DepartmentRelationDraft[] {
  return (departments || []).map((department) => ({
    id: String(department.id || "").trim(),
    childDepartmentIds: normalizeDepartmentChildIds(department.childDepartmentIds, department.id),
  }));
}

function buildRelationSnapshot(drafts: DepartmentRelationDraft[]): string {
  return JSON.stringify(
    drafts
      .map((draft) => ({
        id: draft.id,
        childDepartmentIds: [...draft.childDepartmentIds],
      }))
      .sort((left, right) => left.id.localeCompare(right.id, "zh-CN")),
  );
}

function departmentSortRank(id: string): number {
  if (id === "assistant-department") return 0;
  if (id === "leader-department") return 1;
  if (id === "deputy-department") return 2;
  if (id === "reviewer-department") return 3;
  if (id === "saddler-department") return 4;
  if (id === "remote-customer-service-department") return 5;
  if (id === "hr-department") return 6;
  return 7;
}

function compareDepartments(left: DepartmentConfig, right: DepartmentConfig): number {
  const leftId = String(left.id || "").trim();
  const rightId = String(right.id || "").trim();
  return departmentSortRank(leftId) - departmentSortRank(rightId)
    || Number(left.orderIndex || 0) - Number(right.orderIndex || 0)
    || leftId.localeCompare(rightId, "zh-CN");
}

function mergeRelationDraftsIntoDepartments(
  departments: DepartmentConfig[] | null | undefined,
  drafts: DepartmentRelationDraft[],
  updateChangedAt = false,
): DepartmentConfig[] {
  const now = new Date().toISOString();
  const draftById = new Map(drafts.map((draft) => [draft.id, draft] as const));
  return (departments || []).map((department) => {
    const id = String(department.id || "").trim();
    const draft = draftById.get(id);
    const nextChildIds = normalizeDepartmentChildIds(draft?.childDepartmentIds || department.childDepartmentIds, id);
    const changed = JSON.stringify(nextChildIds) !== JSON.stringify(normalizeDepartmentChildIds(department.childDepartmentIds, id));
    return {
      ...department,
      childDepartmentIds: nextChildIds,
      updatedAt: updateChangedAt && changed ? now : department.updatedAt,
    };
  });
}

const relationDrafts = ref<DepartmentRelationDraft[]>(cloneRelationDrafts(props.config.departments || []));
const sourceRelationSnapshot = computed(() => buildRelationSnapshot(cloneRelationDrafts(props.config.departments || [])));
const relationSnapshot = computed(() => buildRelationSnapshot(relationDrafts.value));
const relationDirty = computed(() => relationSnapshot.value !== sourceRelationSnapshot.value);
const flowNodes = ref<DepartmentFlowNode[]>([]);
const flowEdges = ref<DepartmentFlowEdge[]>([]);
const flowInstance = ref<VueFlowStore | null>(null);
const isFlowFullscreen = ref(false);

const sortedDepartments = computed(() =>
  [...(props.config.departments || [])].sort(compareDepartments),
);

const selectedDepartment = computed(() =>
  sortedDepartments.value.find((department) => department.id === selectedDepartmentId.value) || sortedDepartments.value[0] || null,
);

const selectedChildIds = computed(() => {
  const departmentId = String(selectedDepartment.value?.id || "").trim();
  return relationDrafts.value.find((item) => item.id === departmentId)?.childDepartmentIds || [];
});

const selectedAncestorIdSet = computed(() =>
  new Set(
    departmentAncestorIds(selectedDepartment.value, previewDepartments.value),
  ),
);

const candidateDepartments = computed(() => {
  const selectedId = String(selectedDepartment.value?.id || "").trim();
  const selectedIds = new Set(selectedChildIds.value);
  return sortedDepartments.value
    .filter((department) => {
      const departmentId = String(department.id || "").trim();
      if (!departmentId || departmentId === selectedId) return false;
      return !selectedAncestorIdSet.value.has(departmentId);
    })
    .sort((left, right) => {
      const leftSelected = selectedIds.has(String(left.id || "").trim());
      const rightSelected = selectedIds.has(String(right.id || "").trim());
      if (leftSelected !== rightSelected) return leftSelected ? -1 : 1;
      return 0;
    });
});

const previewDepartments = computed(() => mergeRelationDraftsIntoDepartments(props.config.departments || [], relationDrafts.value));
const relationValidationMessage = computed(() =>
  validateDepartmentConfig(
    {
      ...props.config,
      departments: previewDepartments.value,
    },
    props.config.apiConfigs,
    (key, params) => t(key, params ?? {}),
  ),
);

function restoreDraftsFromSaved() {
  relationDrafts.value = cloneRelationDrafts(props.config.departments || []);
}

function toggleChildDepartment(childDepartmentId: string) {
  const departmentId = String(selectedDepartment.value?.id || "").trim();
  if (!departmentId) return;
  const draft = relationDrafts.value.find((item) => item.id === departmentId);
  if (!draft) return;
  const childId = String(childDepartmentId || "").trim();
  if (!childId || childId === departmentId) return;
  const next = new Set(draft.childDepartmentIds);
  if (next.has(childId)) {
    next.delete(childId);
  } else if (candidateDepartments.value.some((department) => String(department.id || "").trim() === childId)) {
    next.add(childId);
  }
  draft.childDepartmentIds = normalizeDepartmentChildIds(Array.from(next), departmentId);
}

function switchSelectedDepartment(nextId: string) {
  const trimmedId = String(nextId || "").trim();
  if (!trimmedId || trimmedId === selectedDepartmentId.value) return;
  if (relationDirty.value) {
    const currentName = String(selectedDepartment.value?.name || selectedDepartmentId.value || "").trim() || t("config.departmentTree.title");
    props.setStatusAction(t("status.departmentUnsavedSwitchHint", { name: currentName }));
  }
  selectedDepartmentId.value = trimmedId;
}

function handleNodeClick(payload: { node?: { id?: string } }) {
  const nextId = String(payload?.node?.id || "").trim();
  if (!nextId) return;
  switchSelectedDepartment(nextId);
}

function handleFlowInit(instance: VueFlowStore) {
  flowInstance.value = instance;
  void fitFlowIntoView(0.08);
}

async function openFlowFullscreen() {
  if (isFlowFullscreen.value) return;
  isFlowFullscreen.value = true;
  await fitFlowIntoView(0.08);
}

async function closeFlowFullscreen() {
  if (!isFlowFullscreen.value) return;
  isFlowFullscreen.value = false;
  await fitFlowIntoView(0.08);
}

function handleConnect(connection: Connection) {
  const sourceId = String(connection.source || "").trim();
  const targetId = String(connection.target || "").trim();
  if (!sourceId || !targetId) return;
  if (sourceId === targetId) {
    props.setStatusAction(t("config.departmentTree.connectSelfForbidden"));
    return;
  }
  const sourceDraft = relationDrafts.value.find((item) => item.id === sourceId);
  if (!sourceDraft) return;
  if (sourceDraft.childDepartmentIds.includes(targetId)) {
    props.setStatusAction(t("config.departmentTree.connectDuplicateIgnored"));
    return;
  }
  const sourceDepartment = previewDepartments.value.find((department) => department.id === sourceId) || null;
  const ancestorIds = new Set(departmentAncestorIds(sourceDepartment, previewDepartments.value));
  if (ancestorIds.has(targetId)) {
    props.setStatusAction(t("config.departmentTree.connectCycleForbidden"));
    return;
  }
  sourceDraft.childDepartmentIds = normalizeDepartmentChildIds(
    [...sourceDraft.childDepartmentIds, targetId],
    sourceId,
  );
  props.setStatusAction(t("config.departmentTree.connectCreated"));
}

function handleEdgeClick(payload: { edge?: { source?: string; target?: string } }) {
  const sourceId = String(payload?.edge?.source || "").trim();
  const targetId = String(payload?.edge?.target || "").trim();
  if (!sourceId || !targetId) return;
  const sourceDraft = relationDrafts.value.find((item) => item.id === sourceId);
  if (!sourceDraft) return;
  sourceDraft.childDepartmentIds = sourceDraft.childDepartmentIds.filter((id) => id !== targetId);
  props.setStatusAction(t("config.departmentTree.connectRemoved"));
}

async function saveDepartmentRelations() {
  if (!selectedDepartment.value || relationValidationMessage.value) return;
  const previousDepartments = (props.config.departments || []).map((department) => ({
    ...department,
    childDepartmentIds: [...(department.childDepartmentIds || [])],
  }));
  props.config.departments = mergeRelationDraftsIntoDepartments(previousDepartments, relationDrafts.value, true);
  const saved = await Promise.resolve(props.saveConfigAction());
  if (!saved) {
    props.config.departments = previousDepartments;
    return;
  }
  restoreDraftsFromSaved();
}

async function handleAutoLayout() {
  syncFlowGraph(true);
  await fitFlowIntoView(0.16);
  props.setStatusAction(t("config.departmentTree.autoLayoutDone"));
}

async function fitFlowIntoView(padding: number) {
  await nextTick();
  await flowInstance.value?.viewportHelper.fitView({
    padding,
    maxZoom: 1,
  });
}

function syncFlowGraph(forceReset: boolean) {
  const nextLayoutSignature = buildFlowLayoutSignature(previewDepartments.value);
  const shouldRelayout = forceReset || nextLayoutSignature !== lastFlowLayoutSignature;
  const nextNodes = buildFlowNodes(previewDepartments.value);
  const existingPositions = new Map<string, { x: number; y: number }>();
  if (!shouldRelayout) {
    for (const node of flowNodes.value) {
      existingPositions.set(node.id, node.position);
    }
  }

  const syncedNodes: DepartmentFlowNode[] = [];
  for (const node of nextNodes) {
    syncedNodes.push({
      ...node,
      position: existingPositions.get(node.id) || node.position,
    });
  }
  flowNodes.value = syncedNodes;
  flowEdges.value = buildFlowEdges(previewDepartments.value);
  lastFlowLayoutSignature = nextLayoutSignature;
}

const personaById = computed(() =>
  new Map(
    (props.personas || []).map((persona) => [String(persona.id || "").trim(), persona] as const),
  ),
);

function departmentAvatarInfo(department: DepartmentConfig): { avatarUrl: string; avatarText: string } {
  const id = String(department.id || "").trim();
  const boundPersonas = (department.agentIds || [])
    .map((agentId) => personaById.value.get(String(agentId || "").trim()))
    .filter((persona): persona is PersonaProfile => !!persona);
  const avatarPersona = boundPersonas.find((persona) => !!persona.avatarPath);
  const displayName = String(department.name || "").trim() || id;
  return {
    avatarUrl: avatarPersona
      ? resolveAvatarUrl(avatarPersona.avatarPath, avatarPersona.avatarUpdatedAt)
      : "",
    avatarText: displayName.charAt(0) || id.charAt(0) || "部",
  };
}

function buildFlowNodes(departments: DepartmentConfig[]): DepartmentFlowNode[] {
  const positions = buildDepartmentPositions(departments);
  return [...departments].sort(compareDepartments).map((department) => {
    const id = String(department.id || "").trim();
    const boundPersonas = (department.agentIds || [])
      .map((agentId) => personaById.value.get(String(agentId || "").trim()))
      .filter((persona): persona is PersonaProfile => !!persona);
    const personaNames = boundPersonas
      .map((persona) => String(persona.name || "").trim())
      .filter(Boolean);
    const personaText = personaNames.length > 0
      ? personaNames.join(" / ")
      : t("config.departmentTree.noPersonaBound");
    const displayName = String(department.name || "").trim() || id;
    const avatar = departmentAvatarInfo(department);
    return {
      id,
      type: "department",
      position: positions.get(id) || { x: 0, y: 0 },
      draggable: true,
      sourcePosition: Position.Bottom,
      targetPosition: Position.Top,
      data: {
        name: displayName,
        personaText,
        hasPersona: personaNames.length > 0,
        isBuiltInAssistant: !!department.isBuiltInAssistant,
        avatarUrl: avatar.avatarUrl,
        avatarText: avatar.avatarText,
      },
    };
  });
}

function buildFlowEdges(departments: DepartmentConfig[]): DepartmentFlowEdge[] {
  const next: DepartmentFlowEdge[] = [];
  for (const department of departments) {
    const sourceId = String(department.id || "").trim();
    if (!sourceId) continue;
    for (const childId of departmentDirectChildIds(department, departments)) {
      next.push({
        id: `${sourceId}-->${childId}`,
        source: sourceId,
        target: childId,
        type: "smoothstep",
        pathOptions: { offset: 20, borderRadius: 12 },
        markerEnd: MarkerType.ArrowClosed,
        style: { strokeWidth: 1.6 },
        selectable: true,
      });
    }
  }
  return next;
}

function buildDepartmentPositions(departments: DepartmentConfig[]): Map<string, { x: number; y: number }> {
  const orderedDepartments = [...departments].sort(compareDepartments);
  const positions = new Map<string, { x: number; y: number }>();
  const nodeWidth = 240;
  const nodeHeight = 82;
  const graph = new dagre.graphlib.Graph();
  graph.setGraph({
    rankdir: "TB",
    align: "UL",
    nodesep: 56,
    ranksep: 120,
    edgesep: 20,
    marginx: 24,
    marginy: 24,
  });
  graph.setDefaultEdgeLabel(() => ({}));

  for (const department of orderedDepartments) {
    const id = String(department.id || "").trim();
    if (!id) continue;
    graph.setNode(id, {
      width: nodeWidth,
      height: nodeHeight,
    });
  }

  for (const department of orderedDepartments) {
    const sourceId = String(department.id || "").trim();
    if (!sourceId) continue;
    for (const childId of departmentDirectChildIds(department, departments)) {
      if (!graph.hasNode(childId)) continue;
      graph.setEdge(sourceId, childId);
    }
  }

  dagre.layout(graph);

  for (const department of orderedDepartments) {
    const id = String(department.id || "").trim();
    if (!id) continue;
    const node = graph.node(id);
    if (!node) continue;
    positions.set(id, {
      x: Math.round(Number(node.x || 0) - nodeWidth / 2),
      y: Math.round(Number(node.y || 0) - nodeHeight / 2),
    });
  }

  return positions;
}

function buildFlowLayoutSignature(departments: DepartmentConfig[]): string {
  return JSON.stringify(
    [...departments]
      .sort(compareDepartments)
      .map((department) => ({
        id: String(department.id || "").trim(),
        childDepartmentIds: [...departmentDirectChildIds(department, departments)].sort((left, right) => left.localeCompare(right, "zh-CN")),
      })),
  );
}

watch(
  () => sortedDepartments.value.map((item) => item.id).join("|"),
  () => {
    if (!sortedDepartments.value.some((item) => item.id === selectedDepartmentId.value)) {
      selectedDepartmentId.value = sortedDepartments.value[0]?.id || "assistant-department";
    }
    syncFlowGraph(false);
  },
  { immediate: true },
);

watch(
  () => sourceRelationSnapshot.value,
  () => {
    if (relationDirty.value) return;
    restoreDraftsFromSaved();
  },
);

watch(
  () => relationSnapshot.value,
  () => {
    syncFlowGraph(false);
  },
);

function handleWindowKeydown(event: KeyboardEvent) {
  if (event.key !== "Escape" || !isFlowFullscreen.value) return;
  event.preventDefault();
  void closeFlowFullscreen();
}

async function refreshFlowPersonaAvatars() {
  try {
    await preloadPersonaAvatars();
  } catch {
    // 头像预加载失败时保留占位，不阻断树渲染
  }
  syncFlowGraph(false);
}

onMounted(() => {
  window.addEventListener("keydown", handleWindowKeydown);
  void refreshFlowPersonaAvatars();
});

watch(
  () => props.personas,
  () => {
    void refreshFlowPersonaAvatars();
  },
  { deep: true },
);

onBeforeUnmount(() => {
  window.removeEventListener("keydown", handleWindowKeydown);
});
</script>

<style scoped>
.department-tree-flow {
  background:
    radial-gradient(circle at top, color-mix(in srgb, var(--fallback-p, oklch(var(--p))) 10%, transparent), transparent 38%),
    linear-gradient(180deg, color-mix(in srgb, var(--fallback-b2, oklch(var(--b2))) 65%, transparent), transparent 30%);
}

.department-tree-node {
  width: 220px;
}

.department-tree-flow :deep(.vue-flow__controls) {
  box-shadow: none;
  border: 1px solid color-mix(in srgb, currentColor 12%, transparent);
  border-radius: 1rem;
  overflow: hidden;
}
</style>

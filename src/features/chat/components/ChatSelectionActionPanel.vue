<template>
  <div :class="delegateOnly ? 'px-0 py-0' : 'rounded-box border border-base-300 bg-base-100 px-3 py-3'">
    <div v-if="!delegateOnly || selectedMessageCount > 0" class="text-xs opacity-70">{{ t("chat.selection.selectedCount", { count: selectedMessageCount }) }}</div>
    <div v-if="!delegateOnly" class="mt-3 flex flex-wrap items-center gap-2">
      <button v-if="!delegateOnly" type="button" class="btn btn-sm" :disabled="selectedMessageCount === 0" @click="emit('selectionActionBranch')">
        {{ t("chat.selection.branch") }}
      </button>
      <button
        v-if="!delegateOnly && !sidebarMode"
        type="button"
        class="btn btn-sm"
        :class="{ 'btn-primary': selectionDeliverCardOpen }"
        :disabled="selectedMessageCount === 0 || selectionDeliverTargetOptions.length === 0"
        @click="openSelectionDeliverCard"
      >
        {{ t("chat.selection.forward") }}
      </button>
      <button
        type="button"
        class="btn btn-sm"
        :class="{ 'btn-primary': selectionDelegateCardOpen }"
        :disabled="delegateDepartmentOptions.length === 0"
        @click="openSelectionDelegateCard"
      >
        {{ t("chat.selection.delegate") }}
      </button>
      <button v-if="!delegateOnly" type="button" class="btn btn-sm" :disabled="selectedMessageCount === 0" @click="emit('selectionActionCopy')">
        {{ t("common.copy") }}
      </button>
      <button
        v-if="!delegateOnly && !sidebarMode"
        type="button"
        class="btn btn-sm"
        :class="{ 'btn-primary': selectionShareCardOpen }"
        :disabled="selectedMessageCount === 0"
        @click="openSelectionShareCard"
      >
        {{ t("chat.selection.share") }}
      </button>
      <button type="button" class="btn btn-sm btn-ghost ml-auto" @click="handleExitSelectionMode">
        {{ t("common.cancel") }}
      </button>
    </div>

    <div v-if="!delegateOnly && !sidebarMode && selectionDeliverCardOpen" class="mt-3 rounded-box border border-base-300 bg-base-200/50 px-3 py-3">
      <div class="text-sm font-medium">{{ t("chat.selection.forward") }}</div>
      <div class="mt-1 text-xs opacity-70">{{ t("chat.selection.forwardHint") }}</div>
      <select v-model="selectionDeliverTargetConversationId" class="select select-bordered select-sm mt-3 w-full" :disabled="selectionDeliverTargetOptions.length === 0">
        <option v-for="item in selectionDeliverTargetOptions" :key="item.conversationId" :value="item.conversationId">
          {{ selectionDeliverOptionLabel(item) }}
        </option>
      </select>
      <div class="mt-3 flex items-center justify-end gap-2">
        <button type="button" class="btn btn-sm" @click="closeSelectionDeliverCard">{{ t("common.cancel") }}</button>
        <button type="button" class="btn btn-sm btn-primary" :disabled="!selectionDeliverTargetConversationId" @click="confirmSelectionDeliver">
          {{ t("chat.selection.confirmForward") }}
        </button>
      </div>
    </div>

    <div
      v-if="selectionDelegateCardOpen"
      :class="[
        delegateOnly && selectedMessageCount > 0 ? 'mt-2' : '',
        !delegateOnly ? 'mt-3' : '',
        'rounded-box border border-base-300 px-3 py-3',
        delegateOnly ? 'bg-base-100' : 'bg-base-200/50',
      ]"
    >
      <div class="flex items-center justify-between gap-3">
        <div>
          <div class="text-sm font-medium">{{ t("chat.selection.asyncDelegate") }}</div>
          <div class="mt-1 text-xs opacity-70">{{ t("chat.selection.delegateHint") }}</div>
        </div>
        <div class="flex shrink-0 items-center gap-2">
          <button type="button" class="btn btn-sm btn-ghost" @click="clearSelectionDelegateFields">{{ t("common.clear") }}</button>
        </div>
      </div>
      <div v-if="recentDelegateRequests.length > 0" class="mt-3 flex flex-wrap gap-2">
        <button v-for="item in recentDelegateRequests" :key="item.id" type="button" class="btn btn-xs max-w-full justify-start" :title="item.goal" @click="applyRecentDelegateRequest(item)">
          <span class="max-w-52 truncate">{{ item.label }}</span>
        </button>
      </div>
      <DepartmentPersonaSelect
        v-model:department-id="selectionDelegateDepartmentId"
        v-model:agent-id="selectionDelegateAgentId"
        class="mt-3"
        :options="delegateDepartmentOptions"
        auto-select-first
      />
      <label class="form-control mt-3">
        <span class="label py-1">
          <span class="label-text text-xs opacity-70">{{ t("chat.selection.delegateGoalLabel") }}</span>
        </span>
        <textarea v-model="selectionDelegateGoal" class="textarea textarea-bordered min-h-24 w-full resize-y text-sm" :placeholder="t('chat.selection.goalPlaceholder')"></textarea>
      </label>
      <div class="mt-2 grid grid-cols-2 gap-2">
        <label class="form-control min-w-0">
          <span class="label py-1">
            <span class="label-text text-xs opacity-70">{{ t("chat.selection.delegateWhyLabel") }}</span>
          </span>
          <textarea v-model="selectionDelegateWhy" class="textarea textarea-bordered min-h-20 w-full resize-y text-sm" :placeholder="t('chat.selection.whyPlaceholder')"></textarea>
        </label>
        <label class="form-control min-w-0">
          <span class="label py-1">
            <span class="label-text text-xs opacity-70">{{ t("chat.selection.delegateTodoLabel") }}</span>
          </span>
          <textarea v-model="selectionDelegateTodo" class="textarea textarea-bordered min-h-20 w-full resize-y text-sm" :placeholder="t('chat.selection.todoPlaceholder')"></textarea>
        </label>
      </div>
      <div class="mt-3 flex items-center justify-end gap-2">
        <button type="button" class="btn btn-sm" @click="cancelSelectionDelegate">{{ t("common.cancel") }}</button>
        <button type="button" class="btn btn-sm btn-primary" :disabled="!canSubmitSelectionDelegate" @click="confirmSelectionDelegate">
          {{ t("chat.selection.delegate") }}
        </button>
      </div>
    </div>

    <div v-if="!delegateOnly && !sidebarMode && selectionShareCardOpen" class="mt-3 rounded-box border border-base-300 bg-base-200/50 px-3 py-3">
      <div class="text-sm font-medium">{{ t("chat.selection.share") }}</div>
      <div class="mt-1 text-xs opacity-70">{{ t("chat.selection.shareHint") }}</div>
      <div class="mt-3 flex flex-wrap items-center gap-2">
        <button type="button" class="btn btn-sm btn-primary" @click="confirmSelectionShare('png')">{{ t("chat.selection.exportImage") }}</button>
        <button type="button" class="btn btn-sm" @click="confirmSelectionShare('html')">{{ t("chat.selection.exportHtml") }}</button>
        <button type="button" class="btn btn-sm btn-ghost ml-auto" @click="closeSelectionShareCard">{{ t("common.cancel") }}</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { ChatConversationOverviewItem } from "../../../types/app";
import DepartmentPersonaSelect from "../../shared/components/DepartmentPersonaSelect.vue";
import type { DepartmentPersonaOption } from "../../shared/department-persona-options";
import { resolveConversationDisplayTitle } from "../utils/conversation-title";

type ConversationDepartmentOption = DepartmentPersonaOption;

type RecentDelegateRequest = {
  id: string;
  label: string;
  departmentId: string;
  agentId: string;
  presetId: string;
  why: string;
  goal: string;
  todo: string;
};

const props = defineProps<{
  sidebarMode?: boolean;
  delegateOnly?: boolean;
  selectedMessageCount: number;
  activeConversationId: string;
  unarchivedConversationItems: ChatConversationOverviewItem[];
  createConversationDepartmentOptions: ConversationDepartmentOption[];
}>();

const emit = defineEmits<{
  exitSelectionMode: [];
  selectionActionBranch: [];
  selectionActionForward: [targetConversationId: string];
  selectionActionDelegate: [payload: { departmentId: string; agentId: string; presetId: string; why: string; goal: string; todo: string }];
  selectionActionCopy: [];
  selectionActionShare: [format: "html" | "png"];
}>();

const { t, locale } = useI18n();
const sidebarMode = computed(() => !!props.sidebarMode);
const delegateOnly = computed(() => !!props.delegateOnly);
const USER_ASYNC_DELEGATE_RECENT_STORAGE_KEY = "easy_call.user_async_delegate_recent.v1";
const USER_ASYNC_DELEGATE_RECENT_LIMIT = 3;

const selectionDeliverCardOpen = ref(false);
const selectionDeliverTargetConversationId = ref("");
const selectionDelegateCardOpen = ref(false);
const selectionShareCardOpen = ref(false);
const selectionDelegateDepartmentId = ref("");
const selectionDelegateAgentId = ref("");
const selectionDelegatePresetId = ref("review");
const selectionDelegateWhy = ref("");
const selectionDelegateGoal = ref("");
const selectionDelegateTodo = ref("");
const recentDelegateRequests = ref<RecentDelegateRequest[]>([]);

const selectionDeliverTargetOptions = computed(() =>
  (Array.isArray(props.unarchivedConversationItems) ? props.unarchivedConversationItems : [])
    .filter((item) => String(item.conversationId || "").trim() !== String(props.activeConversationId || "").trim())
    .filter((item) => !item.isSystemNotificationConversation)
    .map((item) => ({
      conversationId: String(item.conversationId || "").trim(),
      title: resolveConversationDisplayTitle(item, {
        locale: locale.value,
        untitledLabel: t("chat.untitledConversation"),
      }),
      departmentName: String(item.departmentName || "").trim() || undefined,
      runtimeState: item.runtimeState,
    }))
    .filter((item) => !!item.conversationId),
);

const delegateDepartmentOptions = computed(() =>
  // 用户主动发起异步委托不受 AI delegate 工具的“直接下级部门”限制。
  (Array.isArray(props.createConversationDepartmentOptions) ? props.createConversationDepartmentOptions : [])
    .map((item) => ({
      id: String(item.id || "").trim(),
      departmentId: String(item.departmentId || "").trim(),
      agentId: String(item.agentId || "").trim(),
      departmentName: String(item.departmentName || "").trim(),
      agentName: String(item.agentName || "").trim(),
      label: String(item.label || "").trim(),
      name: String(item.name || "").trim() || String(item.id || "").trim(),
      ownerAgentId: String(item.ownerAgentId || item.agentId || "").trim(),
      ownerName: String(item.ownerName || "").trim(),
      providerName: String(item.providerName || "").trim() || undefined,
      modelName: String(item.modelName || "").trim() || undefined,
      apiConfigId: String(item.apiConfigId || "").trim() || undefined,
      childDepartmentIds: Array.isArray(item.childDepartmentIds) ? item.childDepartmentIds : [],
    }))
    .filter((item) => !!item.id && !!item.departmentId && !!item.agentId),
);

const preferredDelegateDepartmentId = computed(() => String(delegateDepartmentOptions.value[0]?.id || "").trim());
const canSubmitSelectionDelegate = computed(() =>
  delegateDepartmentOptions.value.some((department) =>
    department.departmentId === String(selectionDelegateDepartmentId.value || "").trim()
    && department.agentId === String(selectionDelegateAgentId.value || "").trim()
  )
  && !!String(selectionDelegateGoal.value || "").trim(),
);

function selectionDeliverOptionLabel(item: { title: string; departmentName?: string; runtimeState?: ChatConversationOverviewItem["runtimeState"] }): string {
  const parts = [String(item.title || "").trim() || t('chat.selection.unnamedConversation')];
  const departmentName = String(item.departmentName || "").trim();
  if (departmentName) parts.push(departmentName);
  if (item.runtimeState === "assistant_streaming") parts.push(t('chat.selection.streaming'));
  if (item.runtimeState === "organizing_context") parts.push(t('chat.selection.organizing'));
  return parts.join(" / ");
}

function openSelectionDeliverCard() {
  if (delegateOnly.value) return;
  if (selectionDeliverTargetOptions.value.length === 0) return;
  closeSelectionDelegateCard();
  closeSelectionShareCard();
  const currentTargetConversationId = String(selectionDeliverTargetConversationId.value || "").trim();
  const hasValidTarget = selectionDeliverTargetOptions.value.some((item) => item.conversationId === currentTargetConversationId);
  if (!currentTargetConversationId || !hasValidTarget) {
    selectionDeliverTargetConversationId.value = selectionDeliverTargetOptions.value[0]?.conversationId || "";
  }
  selectionDeliverCardOpen.value = true;
}

function closeSelectionDeliverCard() {
  selectionDeliverCardOpen.value = false;
}

function confirmSelectionDeliver() {
  const targetConversationId = String(selectionDeliverTargetConversationId.value || "").trim();
  if (!targetConversationId) return;
  closeSelectionDeliverCard();
  emit("selectionActionForward", targetConversationId);
}

function normalizeRecentDelegateRequest(raw: unknown): RecentDelegateRequest | null {
  const item = raw as (Partial<RecentDelegateRequest> & {
    background?: string;
    question?: string;
    focus?: string;
  }) | null;
  if (!item) return null;
  const departmentId = String(item.departmentId || "").trim();
  const agentId = String(item.agentId || "").trim();
  const goal = String(item.goal || item.question || "").trim();
  const todo = String(item.todo || item.focus || "").trim();
  if (!departmentId || !agentId || !goal) return null;
  const presetId = String(item.presetId || "review").trim() || "review";
  const label = String(item.label || goal).trim() || goal;
  return {
    id: String(item.id || `${departmentId}:${presetId}:${goal}`).trim(),
    label,
    departmentId,
    agentId,
    presetId,
    why: String(item.why || item.background || "").trim(),
    goal,
    todo,
  };
}

function saveRecentDelegateRequests() {
  try {
    window.localStorage.setItem(USER_ASYNC_DELEGATE_RECENT_STORAGE_KEY, JSON.stringify(recentDelegateRequests.value));
  } catch {
    // ignore persistence failures
  }
}

function loadRecentDelegateRequests() {
  try {
    const raw = window.localStorage.getItem(USER_ASYNC_DELEGATE_RECENT_STORAGE_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return;
    recentDelegateRequests.value = parsed
      .map((item) => normalizeRecentDelegateRequest(item))
      .filter((item): item is RecentDelegateRequest => !!item)
      .slice(0, USER_ASYNC_DELEGATE_RECENT_LIMIT);
  } catch {
    recentDelegateRequests.value = [];
  }
}

function rememberDelegateRequest(raw: Omit<RecentDelegateRequest, "id" | "label">) {
  const request = normalizeRecentDelegateRequest({ ...raw, id: `${Date.now()}:${raw.departmentId}:${raw.agentId}`, label: raw.goal });
  if (!request) return;
  const key = `${request.departmentId}\n${request.agentId}\n${request.presetId}\n${request.why}\n${request.goal}\n${request.todo}`;
  recentDelegateRequests.value = [
    request,
    ...recentDelegateRequests.value.filter((item) => `${item.departmentId}\n${item.agentId}\n${item.presetId}\n${item.why}\n${item.goal}\n${item.todo}` !== key),
  ].slice(0, USER_ASYNC_DELEGATE_RECENT_LIMIT);
  saveRecentDelegateRequests();
}

function clearSelectionDelegateFields() {
  selectionDelegatePresetId.value = "review";
  selectionDelegateWhy.value = "";
  selectionDelegateGoal.value = "";
  selectionDelegateTodo.value = "";
}

function applyRecentDelegateRequest(item: RecentDelegateRequest) {
  const optionStillExists = delegateDepartmentOptions.value.some((department) =>
    department.departmentId === item.departmentId && department.agentId === item.agentId
  );
  if (optionStillExists) {
    selectionDelegateDepartmentId.value = item.departmentId;
    selectionDelegateAgentId.value = item.agentId;
  }
  selectionDelegatePresetId.value = item.presetId || "review";
  selectionDelegateWhy.value = item.why;
  selectionDelegateGoal.value = item.goal;
  selectionDelegateTodo.value = item.todo;
}

function openSelectionDelegateCard() {
  closeSelectionDeliverCard();
  closeSelectionShareCard();
  const preferredOption = delegateDepartmentOptions.value.find((option) => option.id === preferredDelegateDepartmentId.value)
    || delegateDepartmentOptions.value[0];
  if (preferredOption) {
    selectionDelegateDepartmentId.value = preferredOption.departmentId;
    selectionDelegateAgentId.value = preferredOption.agentId;
  }
  selectionDelegateCardOpen.value = true;
}

function closeSelectionDelegateCard() {
  selectionDelegateCardOpen.value = false;
}

function cancelSelectionDelegate() {
  if (delegateOnly.value) {
    handleExitSelectionMode();
    return;
  }
  closeSelectionDelegateCard();
}

function openSelectionShareCard() {
  if (delegateOnly.value) return;
  if (props.selectedMessageCount <= 0) return;
  closeSelectionDeliverCard();
  closeSelectionDelegateCard();
  selectionShareCardOpen.value = true;
}

function closeSelectionShareCard() {
  selectionShareCardOpen.value = false;
}

function confirmSelectionShare(format: "html" | "png") {
  closeSelectionShareCard();
  emit("selectionActionShare", format);
}

function confirmSelectionDelegate() {
  if (!canSubmitSelectionDelegate.value) return;
  const payload = {
    departmentId: String(selectionDelegateDepartmentId.value || "").trim(),
    agentId: String(selectionDelegateAgentId.value || "").trim(),
    presetId: String(selectionDelegatePresetId.value || "review").trim() || "review",
    why: String(selectionDelegateWhy.value || "").trim(),
    goal: String(selectionDelegateGoal.value || "").trim(),
    todo: String(selectionDelegateTodo.value || "").trim(),
  };
  rememberDelegateRequest(payload);
  closeSelectionDelegateCard();
  emit("selectionActionDelegate", payload);
}

function handleExitSelectionMode() {
  closeSelectionDeliverCard();
  closeSelectionDelegateCard();
  closeSelectionShareCard();
  emit("exitSelectionMode");
}

function syncDelegateOnlyPanel() {
  if (delegateOnly.value) {
    openSelectionDelegateCard();
  }
}

onMounted(() => {
  loadRecentDelegateRequests();
  syncDelegateOnlyPanel();
});

watch(delegateOnly, syncDelegateOnlyPanel);
</script>

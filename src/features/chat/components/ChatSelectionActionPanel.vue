<template>
  <div class="rounded-box border border-base-300 bg-base-100 px-3 py-3">
    <div class="text-xs opacity-70">{{ t("chat.selection.selectedCount", { count: selectedMessageCount }) }}</div>
    <div class="mt-3 flex flex-wrap items-center gap-2">
      <button type="button" class="btn btn-sm" :disabled="selectedMessageCount === 0" @click="emit('selectionActionBranch')">
        {{ t("chat.selection.branch") }}
      </button>
      <button
        v-if="!sidebarMode"
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
      <button type="button" class="btn btn-sm" :disabled="selectedMessageCount === 0" @click="emit('selectionActionCopy')">
        {{ t("common.copy") }}
      </button>
      <button
        v-if="!sidebarMode"
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

    <div v-if="!sidebarMode && selectionDeliverCardOpen" class="mt-3 rounded-box border border-base-300 bg-base-200/50 px-3 py-3">
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

    <div v-if="selectionDelegateCardOpen" class="mt-3 rounded-box border border-base-300 bg-base-200/50 px-3 py-3">
      <div class="flex items-center justify-between gap-3">
        <div>
          <div class="text-sm font-medium">{{ t("chat.selection.asyncDelegate") }}</div>
          <div class="mt-1 text-xs opacity-70">{{ t("chat.selection.delegateHint") }}</div>
        </div>
        <div class="flex shrink-0 items-center gap-2">
          <span class="text-sm opacity-70">{{ t("chat.selection.quickDelegate") }}</span>
          <button type="button" class="btn btn-sm" @click="applyDelegateReviewPreset">{{ t("chat.selection.reviewPreset") }}</button>
          <button type="button" class="btn btn-sm btn-ghost" @click="clearSelectionDelegateFields">{{ t("common.clear") }}</button>
        </div>
      </div>
      <div v-if="recentDelegateRequests.length > 0" class="mt-3 flex flex-wrap gap-2">
        <button v-for="item in recentDelegateRequests" :key="item.id" type="button" class="btn btn-xs max-w-full justify-start" :title="item.question" @click="applyRecentDelegateRequest(item)">
          <span class="max-w-52 truncate">{{ item.label }}</span>
        </button>
      </div>
      <select v-model="selectionDelegateDepartmentId" class="select select-bordered select-sm mt-3 w-full">
        <option v-for="department in delegateDepartmentOptions" :key="department.id" :value="department.id">
          {{ selectionDelegateDepartmentLabel(department) }}
        </option>
      </select>
      <textarea v-model="selectionDelegateBackground" class="textarea textarea-bordered mt-3 min-h-16 w-full resize-y text-sm" :placeholder="t('chat.selection.backgroundPlaceholder')"></textarea>
      <textarea v-model="selectionDelegateQuestion" class="textarea textarea-bordered mt-2 min-h-20 w-full resize-y text-sm" :placeholder="t('chat.selection.questionPlaceholder')"></textarea>
      <textarea v-model="selectionDelegateFocus" class="textarea textarea-bordered mt-2 min-h-20 w-full resize-y text-sm" :placeholder="t('chat.selection.focusPlaceholder')"></textarea>
      <div class="mt-3 flex items-center justify-end gap-2">
        <button type="button" class="btn btn-sm" @click="closeSelectionDelegateCard">{{ t("common.cancel") }}</button>
        <button type="button" class="btn btn-sm btn-primary" :disabled="!canSubmitSelectionDelegate" @click="confirmSelectionDelegate">
          {{ t("chat.selection.delegate") }}
        </button>
      </div>
    </div>

    <div v-if="!sidebarMode && selectionShareCardOpen" class="mt-3 rounded-box border border-base-300 bg-base-200/50 px-3 py-3">
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
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import type { ChatConversationOverviewItem, SkillListResult } from "../../../types/app";
import { invokeTauri, isTauriRuntimeAvailable } from "../../../services/tauri-api";
import { resolveConversationDisplayTitle } from "../utils/conversation-title";

type ConversationDepartmentOption = {
  id: string;
  name: string;
  ownerAgentId?: string;
  ownerName: string;
  providerName?: string;
  modelName?: string;
  childDepartmentIds?: string[];
};

type RecentDelegateRequest = {
  id: string;
  label: string;
  departmentId: string;
  presetId: string;
  background: string;
  question: string;
  focus: string;
};

const props = defineProps<{
  sidebarMode?: boolean;
  selectedMessageCount: number;
  activeConversationId: string;
  unarchivedConversationItems: ChatConversationOverviewItem[];
  createConversationDepartmentOptions: ConversationDepartmentOption[];
  delegateDepartmentIds: string[];
}>();

const emit = defineEmits<{
  exitSelectionMode: [];
  selectionActionBranch: [];
  selectionActionForward: [targetConversationId: string];
  selectionActionDelegate: [payload: { departmentId: string; presetId: string; background: string; question: string; focus: string }];
  selectionActionCopy: [];
  selectionActionShare: [format: "html" | "png"];
}>();

const { t, locale } = useI18n();
const sidebarMode = computed(() => !!props.sidebarMode);
const USER_ASYNC_DELEGATE_RECENT_STORAGE_KEY = "easy_call.user_async_delegate_recent.v1";
const USER_ASYNC_DELEGATE_RECENT_LIMIT = 3;
const DELEGATE_REVIEW_FALLBACK_BACKGROUND = [
  "请严格遵守内置 code-review skill 执行审查。",
  "",
  "核心要求：",
  "- 审查当前工作区代码改动，而不是审查单条工具调用。",
  "- 先拿到明确 diff，再做缺陷判断。",
  "- 只报告真实、可复现、会影响正确性/稳定性/安全性的缺陷。",
  "- 不要把风格建议、命名偏好或无法从 diff 证明的推测当成缺陷。",
  "",
  "补充输出约束：当前任务是用户手动发起的异步委托，结果会直接写回原会话；请用自然语言报告审查结论，不要输出工具评估 JSON，除非用户明确要求 JSON。",
].join("\n");
let delegateReviewBackgroundCache = "";

const selectionDeliverCardOpen = ref(false);
const selectionDeliverTargetConversationId = ref("");
const selectionDelegateCardOpen = ref(false);
const selectionShareCardOpen = ref(false);
const selectionDelegateDepartmentId = ref("");
const selectionDelegatePresetId = ref("review");
const selectionDelegateBackground = ref("");
const selectionDelegateQuestion = ref("");
const selectionDelegateFocus = ref("");
const recentDelegateRequests = ref<RecentDelegateRequest[]>([]);

const selectionDeliverTargetOptions = computed(() =>
  (Array.isArray(props.unarchivedConversationItems) ? props.unarchivedConversationItems : [])
    .filter((item) => String(item.conversationId || "").trim() !== String(props.activeConversationId || "").trim())
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

const allowedDelegateDepartmentIds = computed(() =>
  new Set((Array.isArray(props.delegateDepartmentIds) ? props.delegateDepartmentIds : []).map((id) => String(id || "").trim()).filter(Boolean)),
);

const delegateDepartmentOptions = computed(() =>
  (Array.isArray(props.createConversationDepartmentOptions) ? props.createConversationDepartmentOptions : [])
    .map((item) => ({
      id: String(item.id || "").trim(),
      name: String(item.name || "").trim() || String(item.id || "").trim(),
      ownerAgentId: String(item.ownerAgentId || "").trim() || undefined,
      ownerName: String(item.ownerName || "").trim(),
      providerName: String(item.providerName || "").trim() || undefined,
      modelName: String(item.modelName || "").trim() || undefined,
    }))
    .filter((item) => !!item.id && allowedDelegateDepartmentIds.value.has(item.id)),
);

const preferredDelegateDepartmentId = computed(() => String(delegateDepartmentOptions.value[0]?.id || "").trim());
const canSubmitSelectionDelegate = computed(() =>
  delegateDepartmentOptions.value.some((department) => department.id === String(selectionDelegateDepartmentId.value || "").trim())
  && !!String(selectionDelegateQuestion.value || "").trim(),
);

function selectionDeliverOptionLabel(item: { title: string; departmentName?: string; runtimeState?: ChatConversationOverviewItem["runtimeState"] }): string {
  const parts = [String(item.title || "").trim() || "未命名会话"];
  const departmentName = String(item.departmentName || "").trim();
  if (departmentName) parts.push(departmentName);
  if (item.runtimeState === "assistant_streaming") parts.push("流式中");
  if (item.runtimeState === "organizing_context") parts.push("整理中");
  return parts.join(" / ");
}

function openSelectionDeliverCard() {
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

function selectionDelegateDepartmentLabel(item: ConversationDepartmentOption): string {
  const parts = [String(item.name || "").trim() || String(item.id || "").trim()];
  const ownerName = String(item.ownerName || "").trim();
  if (ownerName) parts.push(ownerName);
  const modelName = String(item.modelName || "").trim();
  if (modelName) parts.push(modelName);
  return parts.join(" / ");
}

function delegateReviewPreset() {
  return {
    presetId: "review",
    background: delegateReviewBackgroundCache || DELEGATE_REVIEW_FALLBACK_BACKGROUND,
    question: "请结合选中消息，按内置 code-review 审查规则检查当前工作区代码改动。",
    focus: "先确认明确 diff，再只报告真实、可复现、会影响正确性/稳定性/安全性的缺陷；没有确认到缺陷时直接说明未发现。",
  };
}

async function loadDelegateReviewBackground(): Promise<string> {
  if (delegateReviewBackgroundCache.trim()) return delegateReviewBackgroundCache;
  if (!isTauriRuntimeAvailable()) {
    delegateReviewBackgroundCache = DELEGATE_REVIEW_FALLBACK_BACKGROUND;
    return delegateReviewBackgroundCache;
  }
  try {
    const result = await invokeTauri<SkillListResult>("mcp_list_skills");
    const skill = (result.skills || []).find((item) => String(item.name || "").trim() === "code-review");
    const content = String(skill?.content || "").trim();
    if (content) {
      delegateReviewBackgroundCache = [
        "请严格遵守以下 code-review skill 内容：",
        "",
        content,
        "",
        "补充输出约束：当前任务是用户手动发起的异步委托，结果会直接写回原会话；请用自然语言报告审查结论，不要输出工具评估 JSON，除非用户明确要求 JSON。",
      ].join("\n");
      return delegateReviewBackgroundCache;
    }
  } catch (error) {
    console.error("[用户异步委托][前端] 读取 code-review skill 失败", error);
  }
  delegateReviewBackgroundCache = DELEGATE_REVIEW_FALLBACK_BACKGROUND;
  return delegateReviewBackgroundCache;
}

function normalizeRecentDelegateRequest(raw: unknown): RecentDelegateRequest | null {
  const item = raw as Partial<RecentDelegateRequest> | null;
  if (!item) return null;
  const departmentId = String(item.departmentId || "").trim();
  const question = String(item.question || "").trim();
  const focus = String(item.focus || "").trim();
  if (!departmentId || !question) return null;
  const presetId = String(item.presetId || "review").trim() || "review";
  const label = String(item.label || question).trim() || question;
  return {
    id: String(item.id || `${departmentId}:${presetId}:${question}`).trim(),
    label,
    departmentId,
    presetId,
    background: String(item.background || "").trim(),
    question,
    focus,
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
  const request = normalizeRecentDelegateRequest({ ...raw, id: `${Date.now()}:${raw.departmentId}`, label: raw.question });
  if (!request) return;
  const key = `${request.departmentId}\n${request.presetId}\n${request.background}\n${request.question}\n${request.focus}`;
  recentDelegateRequests.value = [
    request,
    ...recentDelegateRequests.value.filter((item) => `${item.departmentId}\n${item.presetId}\n${item.background}\n${item.question}\n${item.focus}` !== key),
  ].slice(0, USER_ASYNC_DELEGATE_RECENT_LIMIT);
  saveRecentDelegateRequests();
}

async function applyDelegateReviewPreset() {
  const preset = delegateReviewPreset();
  selectionDelegatePresetId.value = preset.presetId;
  selectionDelegateBackground.value = preset.background;
  selectionDelegateQuestion.value = preset.question;
  selectionDelegateFocus.value = preset.focus;
  selectionDelegateBackground.value = await loadDelegateReviewBackground();
}

function clearSelectionDelegateFields() {
  selectionDelegatePresetId.value = "review";
  selectionDelegateBackground.value = "";
  selectionDelegateQuestion.value = "";
  selectionDelegateFocus.value = "";
}

function applyRecentDelegateRequest(item: RecentDelegateRequest) {
  const departmentStillExists = delegateDepartmentOptions.value.some((department) => department.id === item.departmentId);
  if (departmentStillExists) selectionDelegateDepartmentId.value = item.departmentId;
  selectionDelegatePresetId.value = item.presetId || "review";
  selectionDelegateBackground.value = item.background;
  selectionDelegateQuestion.value = item.question;
  selectionDelegateFocus.value = item.focus;
}

function openSelectionDelegateCard() {
  closeSelectionDeliverCard();
  closeSelectionShareCard();
  const preferredDepartmentId = preferredDelegateDepartmentId.value;
  if (preferredDepartmentId) selectionDelegateDepartmentId.value = preferredDepartmentId;
  selectionDelegateCardOpen.value = true;
}

function closeSelectionDelegateCard() {
  selectionDelegateCardOpen.value = false;
}

function openSelectionShareCard() {
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
    presetId: String(selectionDelegatePresetId.value || "review").trim() || "review",
    background: String(selectionDelegateBackground.value || "").trim(),
    question: String(selectionDelegateQuestion.value || "").trim(),
    focus: String(selectionDelegateFocus.value || "").trim(),
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

onMounted(loadRecentDelegateRequests);
</script>

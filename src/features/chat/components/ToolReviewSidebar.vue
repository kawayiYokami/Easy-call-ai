<template>
  <aside v-bind="rootAttrs" class="flex h-full min-h-0 flex-col overflow-x-hidden">
    <div class="px-4 pb-2">
      <div role="tablist" class="tabs tabs-border">
        <button type="button" role="tab" class="tab" :class="{ 'tab-active': activeTab === 'reports' }" @click="activeTab = 'reports'">{{ t("chat.toolReview.resultsTab") }}</button>
        <button type="button" role="tab" class="tab" :class="{ 'tab-active': activeTab === 'tools' }" @click="activeTab = 'tools'">{{ t("chat.toolReview.toolsTab") }}</button>
        <button type="button" role="tab" class="tab" :class="{ 'tab-active': activeTab === 'delegates' }" @click="activeTab = 'delegates'">{{ t("chat.toolReview.delegatesTab") }}</button>
      </div>
    </div>
    <div class="flex min-h-0 flex-1 flex-col overflow-x-hidden">
      <div v-if="errorText" class="mx-4 my-4 rounded-box border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
        {{ errorText }}
      </div>

      <template v-if="activeTab === 'tools' && currentBatch">
        <div class="flex min-h-full flex-col">
          <div class="sticky top-0 z-10 border-b border-base-300 bg-base-100 px-4 py-3">
            <button
              type="button"
              class="btn btn-sm w-full"
              :disabled="batchReviewing"
              @click="emit('reviewBatch', currentBatch.batchKey)"
            >
              <span v-if="batchReviewing" class="loading loading-spinner loading-xs"></span>
              {{ t("chat.toolReview.evaluateBatchWithCount", { count: currentBatchUnreviewedCount }) }}
            </button>
          </div>
          <div class="flex flex-col gap-3 py-2">
            <section v-for="group in reviewGroups" :key="group.key" class="flex flex-col gap-2">
              <div class="px-4 text-xs font-medium text-base-content/60">
                {{ group.title }}
              </div>
              <div
                v-for="item in group.items"
                :key="`${group.key}:${item.callId}`"
                class="px-4"
              >
                <ToolReviewItemCard
                  :item="item"
                  :detail="detailMap[item.callId]"
                  :loading="detailLoadingCallId === item.callId"
                  :reviewing="reviewingCallId === item.callId"
                  @load-detail="emit('loadItemDetail', $event)"
                  @review="emit('reviewItem', $event)"
                />
              </div>
            </section>
          </div>
        </div>
      </template>

      <div v-else-if="activeTab === 'tools'" class="py-2 text-sm text-base-content/65">
        {{ t("chat.toolReview.empty") }}
      </div>

      <div v-else-if="activeTab === 'delegates'" class="flex min-h-0 flex-1 flex-col">
        <div v-if="delegateStatusesErrorText" class="mx-4 my-4 rounded-box border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
          {{ delegateStatusesErrorText }}
        </div>
        <div v-if="delegateStatusesLoading && delegateStatuses.length === 0" class="flex min-h-0 flex-1 items-center justify-center text-sm text-base-content/65">
          <span class="loading loading-spinner loading-sm mr-2"></span>
          {{ t("chat.toolReview.delegateLoading") }}
        </div>
        <div v-else-if="delegateStatuses.length === 0" class="px-4 py-4 text-sm text-base-content/65">
          {{ t("chat.toolReview.delegateEmpty") }}
        </div>
        <div v-else class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto py-2">
          <section v-for="delegate in delegateStatuses" :key="delegate.delegateId">
            <details class="collapse collapse-arrow w-full rounded-box border border-base-300 bg-base-200">
              <summary class="collapse-title min-h-0 px-3 py-3 pr-10">
                <div class="flex items-center justify-between gap-3">
                  <div class="min-w-0 flex items-center gap-2">
                    <div class="truncate text-sm">{{ delegate.title || delegate.delegateId }}</div>
                  </div>
                  <div class="badge badge-sm min-w-14 shrink-0 justify-center whitespace-nowrap" :class="delegateStatusBadgeClass(delegate.status)">
                    {{ formatDelegateStatus(delegate.status) }}
                  </div>
                </div>
                <DelegateProgressLine
                  :running="isDelegateRunning(delegate.status)"
                  :elapsed-ms="delegate.elapsedMs"
                  :request-count="delegate.requestCount"
                  :token-count="delegate.tokenCount"
                  :last-tool-name="delegate.lastToolName"
                />
              </summary>
              <div class="collapse-content flex flex-col gap-3 px-3 pb-3">
                <div class="whitespace-pre-wrap wrap-break-word text-sm leading-7 text-base-content/75">
                  最近工具：{{ delegate.lastToolName || "-" }}
                </div>
                <div class="flex items-center justify-end gap-3">
                  <button
                    v-if="isDelegateRunning(delegate.status)"
                    type="button"
                    class="btn btn-sm btn-error btn-outline gap-1.5 font-normal"
                    @click="emit('abortDelegate', delegate)"
                  >打断</button>
                  <button
                    type="button"
                    class="btn btn-sm gap-1.5 border-base-300 bg-base-100 font-normal hover:bg-base-100"
                    @click="emit('openDelegateDetail', delegate)"
                  >查看详情</button>
                </div>
              </div>
            </details>
          </section>
        </div>
      </div>
      <div v-else class="flex min-h-0 flex-1 flex-col">
        <div class="border-b border-base-300 bg-base-100 px-4 py-3">
          <button
            type="button"
            class="btn btn-sm w-full"
            :disabled="submitting"
            @click="reviewTargetDialogOpen = true"
          >
            <span v-if="submitting" class="loading loading-spinner loading-xs"></span>
            {{ t("chat.toolReview.generateReviewReport") }}
          </button>
        </div>
        <div v-if="props.reports.length === 0" class="flex min-h-0 flex-1 flex-col overflow-y-auto py-2">
          <div class="px-4 py-2 text-sm text-base-content/65">
            {{ t("chat.toolReview.reportUnavailable") }}
          </div>
        </div>
        <div v-else class="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto py-2">
          <section v-for="report in pagedReports" :key="report.id">
            <details class="collapse collapse-arrow w-full rounded-box border border-base-300 bg-base-200">
              <summary class="collapse-title min-h-0 px-3 py-3 pr-10">
                <div class="flex items-center justify-between gap-3">
                  <div class="min-w-0 flex items-center gap-2">
                    <div class="truncate text-sm">{{ report.title || report.target || formatReportScope(report.scope) }}</div>
                  </div>
                  <div class="badge badge-sm min-w-14 shrink-0 justify-center whitespace-nowrap" :class="reportStatusBadgeClass(report.status)">
                    {{ formatReportStatus(report.status) }}
                  </div>
                </div>
                <DelegateProgressLine
                  v-if="report.status === 'success'"
                  :text="reportJudgementSummary(report)"
                />
                <DelegateProgressLine
                  v-else-if="report.status === 'pending' && matchedReportProgress(report)"
                  :running="true"
                  :elapsed-ms="matchedReportProgress(report)!.elapsedMs"
                  :request-count="matchedReportProgress(report)!.requestCount"
                  :token-count="matchedReportProgress(report)!.tokenCount"
                  :last-tool-name="matchedReportProgress(report)!.lastToolName"
                />
                <DelegateProgressLine
                  v-else-if="report.status === 'pending'"
                  text="生成中..."
                />
              </summary>
              <div class="collapse-content flex flex-col gap-3 px-3 pb-3">
                <div class="whitespace-pre-wrap wrap-break-word text-sm leading-7 text-base-content/75">
                  {{ reportExpandedText(report) }}
                </div>
                <div class="flex items-center justify-between gap-3">
                  <button
                    type="button"
                    class="btn btn-sm gap-1.5 border-base-300 bg-base-100 font-normal hover:bg-base-100"
                    :disabled="submitting"
                    @click.prevent.stop="deleteReport(report)"
                  >删除</button>
                  <div class="flex items-center justify-end gap-3">
                    <button
                      type="button"
                      class="btn btn-sm gap-1.5 border-base-300 bg-base-100 font-normal hover:bg-base-100"
                      @click.prevent.stop="openReportDetail(report.id)"
                    >查看详情</button>
                    <button
                      v-if="canRetryReport(report)"
                      type="button"
                      class="btn btn-sm gap-1.5 border-base-300 bg-base-100 font-normal hover:bg-base-100"
                      :disabled="submitting"
                      @click.prevent.stop="retryFailedReport(report)"
                    >重新生成</button>
                  </div>
                </div>
              </div>
            </details>
          </section>
        </div>
      </div>
    </div>

    <div v-if="activeTab === 'tools' && currentBatch && props.batches.length > 1" class="border-t border-base-300 px-4 py-3">
      <div class="join flex justify-center">
        <button
          type="button"
          class="join-item btn btn-sm"
          :disabled="!previousBatch"
          @click="previousBatch && emit('selectBatch', previousBatch.batchKey)"
        >
          «
        </button>
        <button
          type="button"
          class="join-item btn btn-sm"
          @click.prevent
        >
          {{ t("chat.toolReview.pageLabel", { current: currentBatchIndex + 1, total: props.batches.length }) }}
        </button>
        <button
          type="button"
          class="join-item btn btn-sm"
          :disabled="!nextBatch"
          @click="nextBatch && emit('selectBatch', nextBatch.batchKey)"
        >
          »
        </button>
      </div>
    </div>
    <div v-if="activeTab === 'reports' && props.reports.length > reportPageSize" class="border-t border-base-300 px-4 py-3">
      <div class="join flex justify-center">
        <button
          type="button"
          class="join-item btn btn-sm"
          :disabled="reportPage <= 1"
          @click="reportPage = Math.max(1, reportPage - 1)"
        >
          «
        </button>
        <button
          type="button"
          class="join-item btn btn-sm"
          @click.prevent
        >
          {{ t("chat.toolReview.pageLabel", { current: reportPage, total: reportTotalPages }) }}
        </button>
        <button
          type="button"
          class="join-item btn btn-sm"
          :disabled="reportPage >= reportTotalPages"
          @click="reportPage = Math.min(reportTotalPages, reportPage + 1)"
        >
          »
        </button>
      </div>
    </div>
  </aside>

  <dialog class="modal" :class="{ 'modal-open': reportDialogOpen }">
    <div class="modal-box h-[90vh] w-[90vw] max-w-none p-0">
      <div class="flex items-center justify-between border-b border-base-300 px-4 py-3">
        <div class="text-sm">{{ t("chat.toolReview.reportTitle") }}</div>
        <button
          type="button"
          class="btn btn-sm btn-ghost"
          @click="closeReportDialog"
        >
          {{ t("chat.toolReview.closeChanges") }}
        </button>
      </div>
      <div class="assistant-markdown h-[calc(90vh-121px)] overflow-auto px-5 py-4">
        <div v-if="reportErrorText" class="mb-4 rounded-box border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
          {{ reportErrorText }}
        </div>
        <div v-if="currentReport?.status === 'pending'" class="flex h-full min-h-0 items-center justify-center text-sm text-base-content/70">
          <span class="loading loading-spinner loading-sm mr-2"></span>
          {{ t("chat.toolReview.generatingReviewReport") }}
        </div>
        <div v-else-if="!currentReport" class="flex h-full min-h-0 items-center justify-center text-sm text-base-content/70">
          {{ t("chat.toolReview.reportUnavailable") }}
        </div>
        <div v-else-if="currentReportJson" class="flex flex-col gap-3">
          <div v-if="currentReportOverallText" class="whitespace-pre-wrap rounded-box border border-base-300 bg-base-200 px-3 py-2 text-sm leading-7 text-base-content/75">
            {{ currentReportOverallText }}
          </div>
          <div v-if="currentReportFindings.length === 0" class="rounded-box border border-base-300 bg-base-200 px-3 py-3 text-sm text-base-content/70">
            未发现问题。
          </div>
          <details v-for="finding in currentReportFindings" :key="finding.id" class="collapse collapse-arrow rounded-box border border-base-300 bg-base-200">
            <summary class="collapse-title min-h-0 px-3 py-3 pr-10">
              <div class="flex min-w-0 items-center gap-3">
                <input
                  v-model="selectedFindingIds"
                  type="checkbox"
                  class="checkbox checkbox-sm shrink-0"
                  :value="finding.id"
                  @click.stop
                />
                <span
                  v-if="finding.priorityLabel"
                  class="inline-flex shrink-0 items-center"
                  :title="finding.priorityTitle"
                >
                  <span class="inline-block h-2.5 w-2.5 rounded-full" :class="finding.priorityDotClass"></span>
                </span>
                <div class="min-w-0 flex-1 truncate text-sm font-medium text-base-content/85">
                  {{ finding.title }}
                </div>
                <div v-if="finding.confidence" class="badge badge-sm shrink-0 whitespace-nowrap">
                  置信度 {{ finding.confidence }}
                </div>
              </div>
            </summary>
            <div class="collapse-content flex flex-col gap-3 px-3 pb-3">
              <div class="whitespace-pre-wrap wrap-break-word text-sm leading-7 text-base-content/80">
                {{ finding.body || t("chat.toolReview.noDescription") }}
              </div>
              <div v-if="finding.location" class="rounded-box border border-base-300 bg-base-100 px-3 py-2 text-xs leading-6 text-base-content/70">
                {{ t("chat.toolReview.location", { location: finding.location }) }}
              </div>
            </div>
          </details>
        </div>
        <pre
          v-else-if="currentReport?.status === 'success'"
          class="whitespace-pre-wrap wrap-break-word rounded-box border border-base-300 bg-base-200 px-3 py-3 text-sm leading-7 text-base-content/80"
        >{{ currentReport.reportText || t("chat.toolReview.noReportContent") }}</pre>
        <AppMarkdownRenderer
          v-else
          class="ecall-markdown-content tool-review-report-markdown max-w-none"
          :text="reportMarkdownText"
          :is-dark="markdownIsDark"
        />
      </div>
      <div class="flex items-center justify-between gap-3 border-t border-base-300 px-4 py-3">
        <div class="flex items-center gap-3">
          <button
            v-if="currentReport && selectedReportText"
            type="button"
            class="btn btn-sm"
            @click="emit('copyReport', selectedReportText)"
          >
            {{ t("chat.toolReview.copyReport") }}
          </button>
          <button
            v-if="currentReport && selectedReportText"
            type="button"
            class="btn btn-sm"
            @click="emit('attachReport', selectedReportText)"
          >
            {{ t("chat.toolReview.attachReport") }}
          </button>
        </div>
        <div></div>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="closeReportDialog">{{ t("chat.toolReview.closeChanges") }}</button>
    </form>
  </dialog>

  <ToolReviewTargetDialog
    :open="reviewTargetDialogOpen"
    :submitting="submitting"
    :error-text="reportErrorText"
    :current-department-id="currentDepartmentId"
    :department-options="departmentOptions"
    :commit-options="commitOptions"
    :commit-options-loading="commitOptionsLoading"
    :commit-total="commitTotal"
    :commit-page="commitPage"
    :commit-page-size="commitPageSize"
    @close="reviewTargetDialogOpen = false"
    @pick-commit-review="handlePickCommitReview"
    @review-code="handleReviewCode"
  />
</template>

<script setup lang="ts">
import { computed, ref, useAttrs, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { ShellWorkspace } from "../../../types/app";
import { defaultWorkspaceNameFromPath, inferWorkspaceName, isLegacyGenericWorkspaceName, normalizeWorkspaceLevel } from "../../../utils/shell-workspaces";
import type { ToolReviewBatchSummary, ToolReviewCodeReviewScope, ToolReviewCommitOption, ToolReviewItemDetail, ToolReviewItemSummary, ToolReviewReportRecord } from "../composables/use-chat-tool-review";
import { AppMarkdownRenderer, initKatex } from "../markdown";
import ToolReviewItemCard from "./ToolReviewItemCard.vue";
import ToolReviewTargetDialog from "./ToolReviewTargetDialog.vue";
import DelegateProgressLine from "./DelegateProgressLine.vue";

initKatex();

const props = defineProps<{
  batches: ToolReviewBatchSummary[];
  currentBatchKey: string;
  detailMap: Record<string, ToolReviewItemDetail>;
  detailLoadingCallId: string;
  reviewingCallId: string;
  batchReviewingKey: string;
  submittingBatchKey: string;
  errorText: string;
  reportErrorText: string;
  reports: ToolReviewReportRecord[];
  currentReportId: string;
  markdownIsDark: boolean;
  currentWorkspaceName: string;
  currentWorkspaceRootPath: string;
  workspaces: ShellWorkspace[];
  currentDepartmentId: string;
  departmentOptions: Array<{ id: string; name: string; ownerName: string; providerName?: string; modelName?: string }>;
  delegateStatuses: import("../../../types/app").ConversationDelegateStatusSummary[];
  delegateStatusesLoading: boolean;
  delegateStatusesErrorText: string;
}>();

const emit = defineEmits<{
  (e: "selectBatch", batchKey: string): void;
  (e: "loadItemDetail", callId: string): void;
  (e: "reviewItem", callId: string): void;
  (e: "reviewBatch", batchKey: string): void;
  (e: "pickCommitReview", page: number): void;
  (e: "reviewCode", input: { scope: ToolReviewCodeReviewScope; target?: string; departmentId: string }): void;
  (e: "retryReport", report: ToolReviewReportRecord): void;
  (e: "deleteReport", report: ToolReviewReportRecord): void;
  (e: "copyReport", reportText: string): void;
  (e: "attachReport", reportText: string): void;
  (e: "openDelegateDetail", status: import("../../../types/app").ConversationDelegateStatusSummary): void;
  (e: "abortDelegate", status: import("../../../types/app").ConversationDelegateStatusSummary): void;
}>();

const { t } = useI18n();
const reportDialogOpen = ref(false);
const reviewTargetDialogOpen = ref(false);
const activeTab = ref<"tools" | "reports" | "delegates">("reports");
const localCurrentReportId = ref("");
const rootAttrs = useAttrs();
const commitOptions = ref<ToolReviewCommitOption[]>([]);
const commitOptionsLoading = ref(false);
const selectedFindingIds = ref<string[]>([]);
const commitPage = ref(1);
const commitPageSize = ref(30);
const commitTotal = ref(0);
const reportPage = ref(1);
const reportPageSize = 10;

const currentBatchIndex = computed(() => {
  const currentKey = String(props.currentBatchKey || "").trim();
  if (!currentKey) return -1;
  return props.batches.findIndex((batch) => batch.batchKey === currentKey);
});

const currentBatch = computed(() => {
  const currentKey = String(props.currentBatchKey || "").trim();
  if (!currentKey) return null;
  return props.batches.find((batch) => batch.batchKey === currentKey) || null;
});

const previousBatch = computed(() => {
  const index = currentBatchIndex.value;
  if (index < 0) {
    return props.batches[props.batches.length - 1] || null;
  }
  if (index <= 0) return null;
  return props.batches[index - 1] || null;
});

const nextBatch = computed(() => {
  const index = currentBatchIndex.value;
  if (index < 0 || index >= props.batches.length - 1) return null;
  return props.batches[index + 1] || null;
});

const batchReviewing = computed(() =>
  !!currentBatch.value && props.batchReviewingKey === currentBatch.value.batchKey
);

const submitting = computed(() =>
  !!String(props.submittingBatchKey || "").trim()
);

const reportActionLabel = computed(() => {
  return t("chat.toolReview.generateReviewReport");
});

type ToolReviewGroup = {
  key: string;
  title: string;
  firstOrderIndex: number;
  items: ToolReviewItemSummary[];
};

const reviewGroups = computed<ToolReviewGroup[]>(() => {
  const terminalItems = [] as ToolReviewItemSummary[];
  const patchGroups = new Map<string, ToolReviewGroup>();
  for (const item of currentBatch.value?.items ?? []) {
    if (item.toolName === "shell_exec") {
      terminalItems.push(item);
      continue;
    }
    if (item.toolName !== "apply_patch") {
      continue;
    }
    const paths = Array.isArray(item.affectedPaths) ? item.affectedPaths.filter(Boolean) : [];
    const key = paths.length === 1 ? paths[0] : "__multi_patch__";
    const title = paths.length === 1
      ? formatPatchGroupTitle(paths[0])
      : t("chat.toolReview.patchMultiFileGroup");
    const group = patchGroups.get(key) || {
      key: `patch:${key}`,
      title,
      firstOrderIndex: Number(item.orderIndex || 0),
      items: [],
    };
    group.firstOrderIndex = Math.min(group.firstOrderIndex, Number(item.orderIndex || 0));
    group.items.push(item);
    patchGroups.set(key, group);
  }
  const groups = [] as ToolReviewGroup[];
  if (terminalItems.length > 0) {
    groups.push({
      key: "terminal",
      title: t("chat.toolReview.terminalGroup"),
      firstOrderIndex: Math.min(...terminalItems.map((item) => Number(item.orderIndex || 0))),
      items: terminalItems.sort(sortByOrderIndex),
    });
  }
  groups.push(
    ...Array.from(patchGroups.values())
      .map((group) => ({ ...group, items: group.items.sort(sortByOrderIndex) }))
      .sort((a, b) => a.firstOrderIndex - b.firstOrderIndex)
  );
  return groups;
});

const currentBatchUnreviewedCount = computed(() =>
  currentBatch.value?.items.filter((item) => !item.hasReview).length ?? 0
);

function sortByOrderIndex(left: ToolReviewItemSummary, right: ToolReviewItemSummary) {
  return Number(left.orderIndex || 0) - Number(right.orderIndex || 0);
}

function formatPatchGroupTitle(path: string) {
  const normalized = String(path || "").replace(/\\/g, "/").trim();
  if (!normalized) return t("chat.toolReview.patchUnknownFileGroup");
  return compactPathByWorkspace(normalized);
}

function compactPathByWorkspace(path: string) {
  const normalizedPath = normalizePathForDisplay(path);
  const matches = workspacePathDisplayCandidates.value
    .map((candidate) => {
      const root = candidate.root;
      if (!root) return null;
      if (isSameNormalizedPath(normalizedPath, root)) {
        return { root, name: candidate.name, rest: "" };
      }
      if (!isPathUnderWorkspace(normalizedPath, root)) return null;
      return {
        root,
        name: candidate.name,
        rest: normalizedPath.slice(root.length + 1),
      };
    })
    .filter((item): item is { root: string; name: string; rest: string } => !!item)
    .sort((left, right) => right.root.length - left.root.length);
  const matched = matches[0];
  if (!matched) return normalizedPath;
  return matched.rest ? `${matched.name}/${matched.rest}` : matched.name;
}

const workspacePathDisplayCandidates = computed(() =>
  [currentWorkspaceCandidate.value, ...workspaceListCandidates.value]
    .filter((item): item is { root: string; name: string } => !!item)
    .sort((left, right) => right.root.length - left.root.length)
);

const currentWorkspaceCandidate = computed(() => {
  const root = normalizePathForDisplay(props.currentWorkspaceRootPath);
  if (!root) return null;
  const matchedWorkspace = (Array.isArray(props.workspaces) ? props.workspaces : []).find((workspace) =>
    isSameNormalizedPath(root, normalizePathForDisplay(workspace.path))
  );
  const currentName = String(props.currentWorkspaceName || "").trim();
  const matchedName = currentName || (matchedWorkspace ? workspaceDisplayName(matchedWorkspace, root, 0) : "");
  return {
    root,
    name: matchedName || defaultWorkspaceNameFromPath(root) || root,
  };
});

const workspaceListCandidates = computed(() =>
  (Array.isArray(props.workspaces) ? props.workspaces : [])
    .map((workspace, index) => {
      const root = normalizePathForDisplay(workspace.path);
      if (!root) return null;
      return {
        root,
        name: workspaceDisplayName(workspace, root, index),
      };
    })
    .filter((item): item is { root: string; name: string } => !!item)
);

function normalizePathForDisplay(path: string) {
  return String(path || "")
    .replace(/^\\\\\?\\/, "")
    .replace(/^\/\/\?\//, "")
    .replace(/\\/g, "/")
    .replace(/\/+$/, "")
    .trim();
}

function normalizePathForCompare(path: string) {
  return normalizePathForDisplay(path).toLowerCase();
}

function isSameNormalizedPath(path: string, root: string) {
  return normalizePathForCompare(path) === normalizePathForCompare(root);
}

function isPathUnderWorkspace(path: string, root: string) {
  const normalizedPath = normalizePathForCompare(path);
  const normalizedRoot = normalizePathForCompare(root);
  return normalizedPath.startsWith(`${normalizedRoot}/`);
}

function workspaceDisplayName(workspace: ShellWorkspace, root: string, index: number) {
  const level = normalizeWorkspaceLevel(String(workspace.level || ""));
  const rawName = String(workspace.name || "").trim();
  if (!isLegacyGenericWorkspaceName(level, rawName)) {
    return rawName;
  }
  return inferWorkspaceName(level, root, index) || defaultWorkspaceNameFromPath(root) || root;
}

const currentReport = computed(() => {
  const targetId = String(localCurrentReportId.value || props.currentReportId || "").trim();
  if (!targetId) return props.reports[0] || null;
  return props.reports.find((item) => item.id === targetId) || props.reports[0] || null;
});

const reportTotalPages = computed(() => Math.max(1, Math.ceil(props.reports.length / reportPageSize)));

function matchedReportProgress(report: ToolReviewReportRecord) {
  const targetId = String(report.delegateId || "").trim();
  if (targetId) {
    return props.delegateStatuses.find((d) => d.delegateId === targetId) || null;
  }
  // 委托刚创建、delegate_id 尚未写入报告时，按会话匹配运行中的委托
  const convId = String(report.conversationId || "").trim();
  if (!convId) return null;
  return props.delegateStatuses.find(
    (d) => (d.status === "running" || d.status === "delivered") && d.rootConversationId === convId,
  ) || null;
}

const pagedReports = computed(() => {
  const page = Math.min(Math.max(1, reportPage.value), reportTotalPages.value);
  const start = (page - 1) * reportPageSize;
  return props.reports.slice(start, start + reportPageSize);
});

const reportMarkdownText = computed(() =>
  currentReport.value?.reportText || currentReport.value?.errorText || ""
);

type ReportFindingView = {
  id: string;
  raw: unknown;
  title: string;
  body: string;
  confidence: string;
  priorityLabel: string;
  priorityTitle: string;
  priorityDotClass: string;
  location: string;
};

type ParsedToolReviewJson = {
  raw: Record<string, unknown>;
  findings: unknown[];
};

function parseToolReviewJson(reportText: string): ParsedToolReviewJson | null {
  const text = String(reportText || "").trim();
  if (!text) return null;
  const jsonText = extractToolReviewJsonText(text);
  if (!jsonText) return null;
  try {
    const raw = JSON.parse(jsonText) as unknown;
    if (!raw || typeof raw !== "object" || Array.isArray(raw)) return null;
    const record = raw as Record<string, unknown>;
    return {
      raw: record,
      findings: Array.isArray(record.findings) ? record.findings : [],
    };
  } catch {
    return null;
  }
}

function extractToolReviewJsonText(text: string): string {
  const direct = text.trim();
  if (!direct) return "";
  if (direct.startsWith("{") && direct.endsWith("}")) return direct;
  const fenced = extractFirstJsonFenceText(direct);
  if (fenced) return fenced;
  return extractLastJsonObjectText(direct);
}

function extractFirstJsonFenceText(text: string): string {
  const fencePattern = /```(?:json|JSON)?\s*([\s\S]*?)```/g;
  let match: RegExpExecArray | null;
  while ((match = fencePattern.exec(text)) !== null) {
    const candidate = String(match[1] || "").trim();
    if (candidate.startsWith("{") && candidate.endsWith("}")) return candidate;
  }
  return "";
}

function extractLastJsonObjectText(text: string): string {
  const starts: number[] = [];
  for (let index = 0; index < text.length; index += 1) {
    if (text[index] === "{") starts.push(index);
  }
  for (let startIndex = starts.length - 1; startIndex >= 0; startIndex -= 1) {
    const candidate = balancedJsonObjectSlice(text, starts[startIndex]);
    if (candidate) return candidate;
  }
  return "";
}

function balancedJsonObjectSlice(text: string, start: number): string {
  let depth = 0;
  let inString = false;
  let escaped = false;
  for (let index = start; index < text.length; index += 1) {
    const ch = text[index];
    if (inString) {
      if (escaped) {
        escaped = false;
      } else if (ch === "\\") {
        escaped = true;
      } else if (ch === '"') {
        inString = false;
      }
      continue;
    }
    if (ch === '"') {
      inString = true;
      continue;
    }
    if (ch === "{") {
      depth += 1;
      continue;
    }
    if (ch !== "}") continue;
    depth -= 1;
    if (depth === 0) return text.slice(start, index + 1).trim();
    if (depth < 0) return "";
  }
  return "";
}

function parseReportFinding(raw: unknown, index: number): ReportFindingView {
  const record = raw && typeof raw === "object" && !Array.isArray(raw) ? raw as Record<string, unknown> : {};
  const title = stringField(record.title) || `Finding ${index + 1}`;
  const body = stringField(record.body);
  const confidenceValue = numberField(record.confidence_score);
  const confidence = confidenceValue === null ? "" : confidenceValue.toFixed(2);
  const priority = parseFindingPriority(record.priority);
  const location = formatFindingLocation(record.code_location);
  return {
    id: `finding-${index + 1}`,
    raw,
    title,
    body,
    confidence,
    priorityLabel: priority.label,
    priorityTitle: priority.title,
    priorityDotClass: priority.dotClass,
    location,
  };
}

function parseFindingPriority(value: unknown) {
  const raw = typeof value === "number" && Number.isFinite(value)
    ? Math.trunc(value)
    : typeof value === "string"
      ? Number.parseInt(value.trim().replace(/^p/i, ""), 10)
      : null;
  if (raw === 0) {
    return { label: "P0", title: "P0：严重破坏、数据损坏、重大安全问题、核心功能不可用", dotClass: "bg-error" };
  }
  if (raw === 1) {
    return { label: "P1", title: "P1：高概率功能错误或明显错误行为", dotClass: "bg-warning" };
  }
  if (raw === 2) {
    return { label: "P2", title: "P2：局部缺陷、边界错误、可复现但影响较小", dotClass: "bg-info" };
  }
  if (raw === 3) {
    return { label: "P3", title: "P3：低风险但仍属真实问题", dotClass: "bg-success" };
  }
  return { label: "", title: "", dotClass: "" };
}

function stringField(value: unknown) {
  return typeof value === "string" ? value.trim() : "";
}

function numberField(value: unknown) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

function formatFindingLocation(value: unknown) {
  if (!value || typeof value !== "object" || Array.isArray(value)) return "";
  const record = value as Record<string, unknown>;
  const filePath = stringField(record.absolute_file_path);
  const lineRange = record.line_range && typeof record.line_range === "object" && !Array.isArray(record.line_range)
    ? record.line_range as Record<string, unknown>
    : null;
  const start = lineRange ? numberField(lineRange.start) : null;
  const end = lineRange ? numberField(lineRange.end) : null;
  if (!filePath) return "";
  if (start !== null && end !== null && end >= start) return `${filePath}:${start}-${end}`;
  return filePath;
}

const currentReportJson = computed(() => parseToolReviewJson(currentReport.value?.reportText || ""));

const currentReportFindings = computed(() =>
  (currentReportJson.value?.findings || []).map(parseReportFinding)
);

const currentReportOverallText = computed(() => {
  const parsed = currentReportJson.value;
  if (!parsed) return "";
  const correctness = stringField(parsed.raw.overall_correctness);
  const explanation = stringField(parsed.raw.overall_explanation);
  const confidence = numberField(parsed.raw.overall_confidence_score);
  return [
    correctness ? `整体判定：${formatJsonCorrectness(correctness)}` : "",
    explanation ? `判定说明：${explanation}` : "",
    confidence === null ? "" : `整体置信度：${confidence.toFixed(2)}`,
  ].filter(Boolean).join("\n");
});

const selectedReportText = computed(() => {
  const report = currentReport.value;
  if (!report) return "";
  if (report.status !== "success") return report.errorText || report.reportText || "";
  const findings = currentReportFindings.value;
  if (findings.length === 0) return report.reportText || "";
  const selected = findings.filter((item) => selectedFindingIds.value.includes(item.id));
  if (selected.length === 0) return "";
  return selected.map(formatSelectedFindingText).join("\n\n");
});

function formatSelectedFindingText(finding: ReportFindingView) {
  return [
    `[${finding.title}]`,
    finding.location ? `位置：${finding.location}` : "",
    finding.body || "No description.",
  ].filter(Boolean).join("\n");
}

function formatJsonCorrectness(value: string) {
  if (value === "patch is correct") return "通过";
  if (value === "patch is incorrect") return "存在问题";
  return value;
}

watch(() => props.currentBatchKey, () => {
  reportDialogOpen.value = false;
});

watch(
  () => ({
    reportId: currentReport.value?.id || "",
    reportStatus: currentReport.value?.status || "",
    findingIds: currentReportFindings.value.map((item) => item.id).join("|"),
  }),
  (next, previous) => {
    const reportChanged = next.reportId !== (previous?.reportId || "");
    const statusChanged = next.reportStatus !== (previous?.reportStatus || "");
    const findingsChanged = next.findingIds !== (previous?.findingIds || "");
    if (!reportChanged && !statusChanged && !findingsChanged) {
      return;
    }
    selectedFindingIds.value = currentReportFindings.value.map((item) => item.id);
  },
  { immediate: true },
);

watch(
  () => props.reports.map((item) => `${item.id}:${item.status}:${item.updatedAt}`).join("|"),
  () => {
    reportPage.value = Math.min(reportPage.value, reportTotalPages.value);
  }
);

function handleReportAction() {
  reviewTargetDialogOpen.value = true;
}

function setCommitOptions(items: ToolReviewCommitOption[] = [], loading = false, total = 0, page = 1, pageSize = 30) {
  commitOptions.value = items;
  commitOptionsLoading.value = loading;
  commitTotal.value = total;
  commitPage.value = page;
  commitPageSize.value = pageSize;
}

function handlePickCommitReview(page: number) {
  commitOptionsLoading.value = true;
  emit("pickCommitReview", page);
}

function handleReviewCode(input: { scope: ToolReviewCodeReviewScope; target?: string; departmentId: string }) {
  emit("reviewCode", input);
}

defineExpose({
  handleReportAction,
  setCommitOptions,
});

function closeReportDialog() {
  reportDialogOpen.value = false;
}

function openReportDetail(reportId: string) {
  localCurrentReportId.value = String(reportId || "").trim();
  reportDialogOpen.value = true;
}

function retryFailedReport(report: ToolReviewReportRecord) {
  emit("retryReport", report);
}

function canRetryReport(report: ToolReviewReportRecord) {
  if (report.status !== "failed") return false;
  const scope = String(report.scope || "").trim();
  return scope === "commit" || scope === "main" || scope === "uncommitted" || scope === "custom";
}

function deleteReport(report: ToolReviewReportRecord) {
  emit("deleteReport", report);
}

function formatReportStatus(status: string) {
  if (status === "success") return "已完成";
  if (status === "failed") return "生成失败";
  return "生成中";
}

function reportStatusBadgeClass(status: string) {
  if (status === "success") return "badge-primary";
  if (status === "failed") return "badge-error";
  return "badge-warning";
}

function formatDelegateStatus(status: string) {
  if (status === "running" || status === "delivered") return "执行中";
  if (status === "completed") return "已完成";
  if (status === "failed") return "失败";
  return "未知";
}

function isDelegateRunning(status: string) {
  return status === "running" || status === "delivered";
}

function delegateStatusBadgeClass(status: string) {
  if (status === "completed") return "badge-primary";
  if (status === "failed") return "badge-error";
  if (status === "running" || status === "delivered") return "badge-warning";
  return "badge-ghost";
}

function formatTokenK(value: number) {
  if (!Number.isFinite(value) || value <= 0) return "0K";
  const k = value / 1000;
  if (k < 10) return `${k.toFixed(1)}K`;
  return `${Math.round(k)}K`;
}

function formatElapsedMs(value: number) {
  if (!Number.isFinite(value) || value <= 0) return "0秒";
  const totalSeconds = Math.floor(value / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  if (hours > 0) return `${hours}时${minutes}分`;
  if (minutes > 0) return `${minutes}分${seconds}秒`;
  return `${seconds}秒`;
}

function formatReportScope(scope: string) {
  if (scope === "commit") return "commit";
  if (scope === "main") return "主分支差异";
  if (scope === "uncommitted") return "未提交改动";
  if (scope === "custom") return "自定义";
  return scope || "未知范围";
}

function reportMarkdownField(report: ToolReviewReportRecord, label: string) {
  const text = String(report.reportText || "");
  const escapedLabel = label.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const matched = new RegExp(`^-\\s*${escapedLabel}：\\s*(.+)$`, "m").exec(text);
  return String(matched?.[1] || "").trim();
}

function reportJudgementSummary(report: ToolReviewReportRecord) {
  const parsed = parseToolReviewJson(report.reportText);
  if (parsed) {
    const judgement = formatJsonCorrectness(stringField(parsed.raw.overall_correctness) || "未知判定");
    const confidence = numberField(parsed.raw.overall_confidence_score);
    return confidence === null ? judgement : `${judgement} · 置信度 ${confidence.toFixed(2)}`;
  }
  const judgement = reportMarkdownField(report, "整体判定") || "未知判定";
  const confidence = reportMarkdownField(report, "整体置信度");
  return confidence ? `${judgement} · 置信度 ${confidence}` : judgement;
}

function reportExpandedText(report: ToolReviewReportRecord) {
  if (report.status === "pending") return "生成中";
  if (report.status === "failed") return report.errorText || "生成失败";
  const parsed = parseToolReviewJson(report.reportText);
  if (parsed) return stringField(parsed.raw.overall_explanation) || report.reportText || t("chat.toolReview.noReportContent");
  return reportMarkdownField(report, "判定说明") || report.reportText || t("chat.toolReview.noReportContent");
}
</script>

<style scoped>

.assistant-markdown :deep(.ecall-markdown-content.prose) {
  --tw-prose-body: currentColor;
  --tw-prose-headings: currentColor;
  --tw-prose-lead: currentColor;
  --tw-prose-links: var(--color-base-content);
  --tw-prose-bold: currentColor;
  --tw-prose-counters: currentColor;
  --tw-prose-bullets: color-mix(in srgb, var(--color-base-content) 50%, transparent);
  --tw-prose-hr: color-mix(in srgb, var(--color-base-content) 15%, transparent);
  --tw-prose-quotes: currentColor;
  --tw-prose-quote-borders: color-mix(in srgb, var(--color-base-content) 20%, transparent);
  --tw-prose-captions: color-mix(in srgb, var(--color-base-content) 75%, transparent);
  --tw-prose-code: currentColor;
  --tw-prose-pre-code: currentColor;
  --tw-prose-pre-bg: var(--color-base-200);
  --tw-prose-th-borders: color-mix(in srgb, var(--color-base-content) 20%, transparent);
  --tw-prose-td-borders: color-mix(in srgb, var(--color-base-content) 15%, transparent);
}

.assistant-markdown :deep(.ecall-markdown-content) {
  --ms-font-sans: var(
    --app-font-family,
    system-ui,
    -apple-system,
    BlinkMacSystemFont,
    "Segoe UI",
    Roboto,
    "Helvetica Neue",
    Arial,
    sans-serif
  );
  --ms-text-body: 0.875rem;
  --ms-leading-body: 1.5;
  --ms-text-h1: 1.02rem;
  --ms-leading-h1: 1.5;
  --ms-text-h2: 0.98rem;
  --ms-leading-h2: 1.5;
  --ms-text-h3: 0.94rem;
  --ms-leading-h3: 1.5;
  --ms-text-h4: 0.9rem;
  --ms-text-h5: 0.875rem;
  --ms-text-h6: 0.875rem;
  --ms-flow-paragraph-y: 0.25rem;
  --ms-flow-list-y: 0.25rem;
  --ms-flow-list-item-y: 0.12rem;
  --ms-flow-list-indent: 1.05rem;
  --ms-flow-list-indent-mobile: 1.05rem;
  --ms-flow-blockquote-y: 0.25rem;
  --ms-flow-blockquote-indent: 0.68rem;
  min-width: 0;
  max-width: 100%;
  overflow-x: hidden;
  font-family: inherit;
  font-size: 0.875rem;
  line-height: 1.5;
}

.assistant-markdown :deep(.ecall-markdown-content .paragraph-node),
.assistant-markdown :deep(.ecall-markdown-content .heading-node),
.assistant-markdown :deep(.ecall-markdown-content .list-node),
.assistant-markdown :deep(.ecall-markdown-content .list-item),
.assistant-markdown :deep(.ecall-markdown-content .blockquote),
.assistant-markdown :deep(.ecall-markdown-content .link-node),
.assistant-markdown :deep(.ecall-markdown-content .strong-node),
.assistant-markdown :deep(.ecall-markdown-content .inline-code),
.assistant-markdown :deep(.ecall-markdown-content .table-node-wrapper),
.assistant-markdown :deep(.ecall-markdown-content .hr-node) {
  font-size: inherit;
  line-height: inherit;
}

.assistant-markdown :deep(.ecall-markdown-content.markdown-renderer) {
  content-visibility: visible !important;
  contain: none !important;
  contain-intrinsic-size: auto !important;
}

.assistant-markdown :deep(.ecall-markdown-content .markdown-renderer),
.assistant-markdown :deep(.ecall-markdown-content .node-slot),
.assistant-markdown :deep(.ecall-markdown-content .node-content),
.assistant-markdown :deep(.ecall-markdown-content .text-node) {
  font-size: inherit;
  line-height: inherit;
}

.assistant-markdown :deep(.ecall-markdown-content .code-block-container),
.assistant-markdown :deep(.ecall-markdown-content ._mermaid) {
  content-visibility: visible !important;
  contain: none !important;
  contain-intrinsic-size: auto !important;
}

.assistant-markdown :deep(.ecall-markdown-content > :first-child) {
  margin-top: 0;
}

.assistant-markdown :deep(.ecall-markdown-content > :last-child) {
  margin-bottom: 0;
}

.assistant-markdown :deep(.ecall-markdown-content :where(p,ul,ol,blockquote,pre,table,figure,.paragraph-node,.list-node,.blockquote,.table-node-wrapper,.code-block-container,._mermaid,.vmr-container)) {
  margin-top: 0.25rem;
  margin-bottom: 0.25rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(h1,h2,h3,h4,.heading-node)) {
  margin-top: 0.7rem;
  margin-bottom: 0.32rem;
  line-height: 1.5;
}

.assistant-markdown :deep(.ecall-markdown-content :where(h1,.heading-node.heading-1)) {
  font-size: 1.02rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(h2,.heading-node.heading-2)) {
  font-size: 0.98rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(h3,.heading-node.heading-3)) {
  font-size: 0.94rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(h4,.heading-node.heading-4)) {
  font-size: 0.9rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(ul,ol,.list-node)) {
  padding-left: 1.05rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(li,.list-item)) {
  margin: 0.12rem 0;
  padding-left: 0;
}

.assistant-markdown :deep(.ecall-markdown-content :where(li,.list-item) > :where(p,ul,ol,.paragraph-node,.list-node)) {
  margin-top: 0.16rem;
  margin-bottom: 0.16rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(blockquote,.blockquote)) {
  padding-left: 0.68rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(blockquote,.blockquote) .markdown-renderer),
.assistant-markdown :deep(.ecall-markdown-content :where(ul,ol,.list-node,li,.list-item) .markdown-renderer) {
  font-size: inherit;
  line-height: inherit;
}

.assistant-markdown :deep(.ecall-markdown-content :where(hr,.hr-node)) {
  margin: 0.65rem 0;
}

.assistant-markdown :deep(.ecall-markdown-content :where(:not(pre) > code,.inline-code):not(.code-block-container *)) {
  font-size: 0.86em;
}

.assistant-markdown :deep(.ecall-markdown-content :where(table,.table-node)) {
  font-size: 0.9rem;
}

.assistant-markdown :deep(.ecall-markdown-content ._mermaid) {
  width: 100%;
}

.tool-review-report-markdown:deep(.code-block-container),
.tool-review-report-markdown:deep(._mermaid) {
  margin: 1rem 0;
}

.tool-review-report-markdown:deep(> :first-child) {
  margin-top: 0;
}

.tool-review-report-markdown:deep(> :last-child) {
  margin-bottom: 0;
}
</style>

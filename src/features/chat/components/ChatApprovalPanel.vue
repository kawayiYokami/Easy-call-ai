<script setup lang="ts">
import { ref } from "vue";
import { ShieldAlert } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import type { TerminalApprovalConversationItem } from "../../shell/composables/use-terminal-approval";
import ToolReviewChangesDialog from "./ToolReviewChangesDialog.vue";

const props = withDefaults(defineProps<{
  approvals: TerminalApprovalConversationItem[];
  resolving: boolean;
}>(), {
  approvals: () => [],
  resolving: false,
});

const emit = defineEmits<{
  approve: [requestId: string];
  deny: [requestId: string];
  approveForSession: [requestId: string];
  approveForWorkspace: [requestId: string];
}>();

const { t } = useI18n();
const detailDialogRefs = ref<Record<string, { openChangesDialog: () => void } | null>>({});

function setDetailDialogRef(requestId: string, dialog: { openChangesDialog: () => void } | null) {
  const id = String(requestId || "").trim();
  if (id) detailDialogRefs.value[id] = dialog;
}

function openDetailDialog(requestId: string) {
  detailDialogRefs.value[String(requestId || "").trim()]?.openChangesDialog();
}

function toolNameText(item: TerminalApprovalConversationItem): string {
  const toolName = String(item.toolName || "").trim();
  if (toolName) return toolName;
  const approvalKind = String(item.approvalKind || "").trim();
  return approvalKind || t("chat.toolReview.title");
}

function reviewOpinionText(item: TerminalApprovalConversationItem): string {
  return String(item.reviewOpinion || "").trim();
}

function callPreviewText(item: TerminalApprovalConversationItem): string {
  return String(item.callPreview || "").replace(/\r/g, "").trim();
}

function isPatchPreview(item: TerminalApprovalConversationItem): boolean {
  const text = callPreviewText(item);
  return text.includes("*** Begin Patch") || text.includes("*** Update File:") || text.includes("*** Add File:") || text.includes("*** Delete File:");
}

function callPreviewLines(item: TerminalApprovalConversationItem): string[] {
  return callPreviewText(item).split("\n");
}

function callPreviewFirstLine(item: TerminalApprovalConversationItem): string {
  const lines = callPreviewLines(item);
  for (const line of lines) {
    const trimmed = line.trim();
    if (trimmed && !trimmed.startsWith("***")) return trimmed.length > 80 ? trimmed.slice(0, 80) + "…" : trimmed;
  }
  return "";
}

function workspaceRememberLabel(item: TerminalApprovalConversationItem): string {
  const workspaceName = String(item.workspaceName || "").trim();
  if (workspaceName) return workspaceName;
  const workspacePath = String(item.workspacePath || "").trim();
  return workspacePath || "";
}

function currentSessionApprovalRequestId(): string {
  return String(props.approvals[0]?.requestId || "").trim();
}

function handleSessionAutonomousModeChange() {
  const requestId = currentSessionApprovalRequestId();
  if (!requestId || props.resolving) return;
  emit("approveForSession", requestId);
}

function handleWorkspaceAutonomousModeChange(requestId: string) {
  const normalizedRequestId = String(requestId || "").trim();
  if (!normalizedRequestId || props.resolving) return;
  emit("approveForWorkspace", normalizedRequestId);
}
</script>

<template>
  <div class="rounded-box border border-warning/30 bg-warning/8 px-3 py-3 shadow-sm">
    <div class="flex items-center justify-between gap-3">
      <div class="flex min-w-0 items-center gap-2">
        <ShieldAlert class="h-4 w-4 shrink-0 text-warning" />
        <div class="min-w-0">
          <div class="text-sm font-semibold text-base-content">
            {{ t("chat.toolReview.title") }}
          </div>
          <div class="text-xs text-base-content/65">
            {{ t("chat.toolReview.button", { count: approvals.length }) }}
          </div>
        </div>
      </div>
      <label
        v-if="currentSessionApprovalRequestId()"
        class="flex shrink-0 cursor-pointer items-center gap-2 rounded-lg bg-base-200 px-3 py-2 text-sm text-base-content transition hover:bg-base-300"
        :class="resolving ? 'pointer-events-none opacity-60' : ''"
      >
        <input
          type="checkbox"
          class="checkbox checkbox-sm"
          :disabled="resolving"
          @change="handleSessionAutonomousModeChange"
        >
        <span>{{ t("terminalApproval.rememberSession") }}</span>
      </label>
    </div>
    <ul class="mt-3 flex flex-col gap-2">
      <li
        v-for="(item, index) in approvals"
        :key="item.requestId"
        class="rounded-box border border-base-300 bg-base-100/85 px-3 py-2"
      >
        <div class="flex items-start justify-between gap-3">
          <div class="flex min-w-0 items-center gap-2 text-sm font-medium text-base-content">
            <span class="badge badge-ghost badge-sm shrink-0">{{ index + 1 }}</span>
            <span class="truncate">{{ toolNameText(item) }}</span>
          </div>

          <div
            v-if="item.canRememberWorkspace"
            class="flex shrink-0 flex-wrap justify-end gap-1.5"
          >
            <label
              class="flex cursor-pointer items-center gap-2 rounded-lg bg-base-200 px-3 py-2 text-sm text-base-content transition hover:bg-base-300"
              :class="resolving ? 'pointer-events-none opacity-60' : ''"
            >
              <input
                type="checkbox"
                class="checkbox checkbox-sm"
                :disabled="resolving"
                @change="handleWorkspaceAutonomousModeChange(item.requestId)"
              >
              <span>{{ t("terminalApproval.rememberWorkspace") }}</span>
            </label>
          </div>
        </div>

        <div v-if="callPreviewText(item)" class="mt-2 flex items-center gap-2">
          <button
            type="button"
            class="btn btn-sm shrink-0 border-base-300 bg-base-100 font-normal hover:bg-base-200"
            @click.prevent.stop="openDetailDialog(item.requestId)"
          >
            {{ t("common.details") }}
          </button>
          <span v-if="callPreviewFirstLine(item)" class="min-w-0 flex-1 truncate text-xs text-base-content/50">
            {{ callPreviewFirstLine(item) }}
          </span>
          <ToolReviewChangesDialog
            :ref="(dialog) => setDetailDialogRef(item.requestId, dialog as { openChangesDialog: () => void } | null)"
            :title="toolNameText(item)"
            :subtitle="item.approvalKind || ''"
            :show-preview="!!callPreviewText(item)"
            :preview-mode="isPatchPreview(item) ? 'patch' : 'plain'"
            :preview-text="callPreviewText(item)"
            raw-review=""
          />
        </div>

        <div v-if="reviewOpinionText(item)" class="mt-2 whitespace-pre-wrap text-xs leading-5 text-base-content/70">
          {{ reviewOpinionText(item) }}
        </div>

        <div
          v-if="item.canRememberWorkspace && workspaceRememberLabel(item)"
          class="mt-2 text-xs text-base-content/45"
        >
          {{ workspaceRememberLabel(item) }}
        </div>

        <div class="mt-2 flex gap-2">
          <button
            type="button"
            class="btn btn-sm flex-1 border-base-300 bg-base-200 text-base-content hover:bg-base-300"
            :disabled="resolving"
            @click="emit('deny', item.requestId)"
          >
            {{ t("terminalApproval.deny") }}
          </button>
          <button
            type="button"
            class="btn btn-sm flex-1 border-base-300 bg-base-200 text-base-content hover:bg-base-300"
            :disabled="resolving"
            @click="emit('approve', item.requestId)"
          >
            {{ t("terminalApproval.approve") }}
          </button>
        </div>
      </li>
    </ul>
  </div>
</template>

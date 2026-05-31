<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { ShieldAlert } from "@lucide/vue";
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
}>();

const { t } = useI18n();

const approvalCount = computed(() => props.approvals.length);
const detailDialogRefs = ref<Record<string, { openChangesDialog: () => void } | null>>({});

function summaryText(item: TerminalApprovalConversationItem): string {
  return String(item.summary || item.reason || item.message || item.command || "").trim();
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

function previewText(item: TerminalApprovalConversationItem): string {
  return String(item.callPreview || item.command || item.summary || item.message || "").replace(/\r/g, "");
}

function previewMode(item: TerminalApprovalConversationItem): "plain" | "patch" {
  const text = previewText(item).trim();
  if (text.includes("*** Begin Patch") || text.includes("*** Update File:") || text.includes("*** Add File:") || text.includes("*** Delete File:")) {
    return "patch";
  }
  return "plain";
}

function setDetailDialogRef(requestId: string, dialog: { openChangesDialog: () => void } | null) {
  const normalizedRequestId = String(requestId || "").trim();
  if (!normalizedRequestId) return;
  detailDialogRefs.value[normalizedRequestId] = dialog;
}

function openDetailDialog(requestId: string) {
  detailDialogRefs.value[String(requestId || "").trim()]?.openChangesDialog();
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
            {{ t("chat.toolReview.button", { count: approvalCount }) }}
          </div>
        </div>
      </div>
    </div>

    <ul class="mt-3 flex flex-col gap-2">
      <li
        v-for="(item, index) in approvals"
        :key="item.requestId"
        class="rounded-box border border-base-300 bg-base-100/85 px-3 py-2"
      >
        <div class="flex items-stretch justify-between gap-3">
          <div class="min-w-0 flex-1">
            <div class="flex items-center gap-2 text-sm font-medium text-base-content">
              <span class="badge badge-ghost badge-sm shrink-0">{{ index + 1 }}</span>
              <span class="truncate">{{ toolNameText(item) }}</span>
            </div>
            <div v-if="summaryText(item)" class="mt-1 whitespace-pre-wrap text-xs leading-5 text-base-content/75">
              {{ summaryText(item) }}
            </div>
            <div v-if="reviewOpinionText(item)" class="mt-1 line-clamp-3 whitespace-pre-wrap text-xs leading-5 text-base-content/60">
              {{ reviewOpinionText(item) }}
            </div>
            <ToolReviewChangesDialog
              :ref="(dialog) => setDetailDialogRef(item.requestId, dialog as { openChangesDialog: () => void } | null)"
              :title="toolNameText(item)"
              :subtitle="item.approvalKind || t('chat.toolReview.title')"
              :show-preview="!!previewText(item)"
              :preview-mode="previewMode(item)"
              :preview-text="previewText(item)"
              raw-review=""
            />
          </div>
          <div class="flex w-18 shrink-0 flex-col justify-center gap-1.5 self-stretch">
            <button
              type="button"
              class="btn btn-xs w-full border-base-300 bg-base-200 hover:bg-base-300"
              :disabled="resolving"
              @click="openDetailDialog(item.requestId)"
            >
              {{ t("common.details") }}
            </button>
            <button
              type="button"
              class="btn btn-xs w-full border-base-300 bg-base-200 text-base-content hover:bg-base-300"
              :disabled="resolving"
              @click="emit('approve', item.requestId)"
            >
              {{ t("terminalApproval.approve") }}
            </button>
            <button
              type="button"
              class="btn btn-xs w-full border-base-300 bg-base-200 text-base-content hover:bg-base-300"
              :disabled="resolving"
              @click="emit('deny', item.requestId)"
            >
              {{ t("terminalApproval.deny") }}
            </button>
          </div>
        </div>
      </li>
    </ul>
  </div>
</template>

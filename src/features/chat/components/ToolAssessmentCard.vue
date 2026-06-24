<template>
  <section class="last:mb-0">
    <div class="group/card flex w-full items-center gap-2 rounded-2xl px-2 py-1 transition-colors hover:bg-base-200">
      <div class="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-base-100 text-xs font-semibold text-base-content/65">
        #{{ item.orderIndex }}
      </div>
      <div class="min-w-0 flex-1 overflow-hidden">
        <span class="block truncate text-xs font-normal text-base-content">{{ title }}</span>
        <div class="mt-1 truncate text-xs text-base-content/65">{{ reviewOpinionText }}</div>
      </div>
      <button
        type="button"
        class="btn btn-ghost btn-sm h-8 min-h-8 shrink-0 gap-1.5 px-2 font-normal"
        :disabled="loading"
        :title="t('chat.toolReview.view')"
        @click.stop="openChanges"
      >
        <span v-if="loading" class="loading loading-spinner loading-xs"></span>
        <Eye v-else class="size-3.5" aria-hidden="true" />
        <span>{{ t("chat.toolReview.view") }}</span>
      </button>
    </div>
  </section>

  <ToolReviewChangesDialog
    ref="changesDialogRef"
    :title="title"
    :subtitle="`#${item.orderIndex}`"
    :show-preview="!!detail"
    :preview-mode="detail?.previewKind === 'patch' ? 'patch' : 'plain'"
    :preview-text="detail ? detail.previewText || detail.resultText : ''"
    :review-opinion="dialogReviewOpinion"
    :review-allow="detail?.review?.allow"
    :review-model-name="detail?.review?.modelName || ''"
    :is-dark="isDark"
  />
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { Eye } from "@lucide/vue";
import type { ToolReviewItemDetail, ToolReviewItemSummary } from "../composables/use-chat-tool-review";
import ToolReviewChangesDialog from "./ToolReviewChangesDialog.vue";

const props = withDefaults(defineProps<{
  item: ToolReviewItemSummary;
  detail?: ToolReviewItemDetail;
  loading: boolean;
  isDark?: boolean;
}>(), {
  detail: undefined,
  isDark: false,
});

const emit = defineEmits<{
  (e: "loadDetail", callId: string): void;
}>();

const { t } = useI18n();
const changesDialogRef = ref<{ openChangesDialog: () => void; closeChangesDialog: () => void } | null>(null);
const pendingOpen = ref(false);

function isTerminalTool(toolName: string) {
  const normalized = String(toolName || "").trim();
  return normalized === "shell_exec" || normalized === "exec";
}

function isFileChangeTool(toolName: string) {
  const normalized = String(toolName || "").trim();
  return normalized === "apply_patch"
    || normalized === "write"
    || normalized === "delete"
    || normalized === "update"
    || normalized === "move";
}

const title = computed(() => {
  if (isTerminalTool(props.item.toolName)) {
    return String(props.item.command || "").trim() || t("chat.toolReview.terminalCommand");
  }
  if (isFileChangeTool(props.item.toolName)) {
    const fileName = patchFileName();
    const operation = patchOperationLabel(props.item.patchOperation);
    return fileName ? `${operation} ${fileName}` : operation;
  }
  return props.item.toolName;
});

const reviewOpinionText = computed(() => {
  const direct = props.detail?.review?.reviewOpinion;
  if (direct && direct.trim()) return direct;
  const summaryOpinion = String(props.item.reviewOpinion || "").trim();
  if (summaryOpinion) return summaryOpinion;
  if (props.item.hasReview && !props.detail) return t("chat.toolReview.evaluated");
  return props.item.hasReview ? t("chat.toolReview.reviewUnavailable") : t("chat.toolReview.unevaluated");
});

const dialogReviewOpinion = computed(() => String(props.detail?.review?.reviewOpinion || props.item.reviewOpinion || "").trim());

watch(() => props.detail, (detail) => {
  if (!pendingOpen.value || !detail) return;
  pendingOpen.value = false;
  changesDialogRef.value?.openChangesDialog();
});

function openChanges() {
  if (props.detail) {
    changesDialogRef.value?.openChangesDialog();
    return;
  }
  pendingOpen.value = true;
  emit("loadDetail", props.item.callId);
}

function patchFileName() {
  const paths = Array.isArray(props.item.affectedPaths) ? props.item.affectedPaths : [];
  if (paths.length !== 1) return "";
  const normalized = String(paths[0] || "").replace(/\\/g, "/").trim();
  return normalized.split("/").filter(Boolean).pop() || "";
}

function patchOperationLabel(operation?: string) {
  if (operation === "add") return t("chat.toolReview.patchOperationAdd");
  if (operation === "delete") return t("chat.toolReview.patchOperationDelete");
  if (operation === "mixed") return t("chat.toolReview.patchOperationMixed");
  return t("chat.toolReview.patchOperationUpdate");
}
</script>

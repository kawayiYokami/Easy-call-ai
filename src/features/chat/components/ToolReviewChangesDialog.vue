<template>
  <dialog ref="dialogRef" class="modal">
    <div class="modal-box h-[90vh] w-[90vw] max-w-none p-0">
      <div class="flex items-center justify-between border-b border-base-300 px-4 py-3">
        <div class="min-w-0">
          <div class="truncate text-sm">{{ title }}</div>
          <div class="text-xs text-base-content/60">{{ subtitle }}</div>
        </div>
        <button
          type="button"
          class="btn btn-sm btn-ghost"
          @click="closeChangesDialog"
        >
          {{ t("chat.toolReview.closeChanges") }}
        </button>
      </div>
      <div class="flex h-[calc(90vh-61px)] min-h-0 flex-col overflow-hidden">
        <div v-if="reviewOpinion || reviewModelName || typeof reviewAllow === 'boolean'" class="shrink-0 border-b border-base-300 px-4 py-3">
          <div class="flex flex-wrap items-center gap-2 text-xs text-base-content/65">
            <span
              v-if="typeof reviewAllow === 'boolean'"
              class="badge badge-sm"
              :class="reviewAllow ? 'badge-success' : 'badge-error'"
            >
              {{ reviewAllow ? t("chat.toolReview.allowed") : t("chat.toolReview.blocked") }}
            </span>
            <span v-if="reviewModelName">{{ reviewModelName }}</span>
          </div>
          <div v-if="reviewOpinion" class="mt-2 whitespace-pre-wrap text-sm leading-6 text-base-content/80">
            {{ reviewOpinion }}
          </div>
        </div>
        <ToolReviewCodePreview
          v-if="showPreview"
          :mode="previewMode"
          :title="previewMode === 'patch' ? '' : t('chat.toolReview.commandPreview')"
          :code="previewText"
          :is-dark="isDark"
        />
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button>{{ t("chat.toolReview.closeChanges") }}</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";
import ToolReviewCodePreview from "./ToolReviewCodePreview.vue";

withDefaults(defineProps<{
  title: string;
  subtitle: string;
  showPreview: boolean;
  previewMode: "plain" | "patch";
  previewText: string;
  reviewOpinion?: string;
  reviewAllow?: boolean;
  reviewModelName?: string;
  isDark?: boolean;
}>(), {
  reviewOpinion: "",
  reviewAllow: undefined,
  reviewModelName: "",
  isDark: false,
});

const { t } = useI18n();
const dialogRef = ref<HTMLDialogElement | null>(null);

function openChangesDialog() {
  dialogRef.value?.showModal();
}

function closeChangesDialog() {
  dialogRef.value?.close();
}

defineExpose({
  openChangesDialog,
  closeChangesDialog,
});
</script>

<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { Star } from "@lucide/vue";
import { AppMarkdownRenderer, initKatex } from "../../chat/markdown";
import type { RuntimeLogEntry } from "../../../types/app";
import RuntimeLogsDialog from "./RuntimeLogsDialog.vue";
import ConversationMaintenanceDialog from "../../chat/components/dialogs/ConversationMaintenanceDialog.vue";
import type {
  TrimCompactionPreviewResult,
  TrimPreviewResult,
} from "../../chat/composables/use-conversation-maintenance-dialog";

initKatex();

type UpdateDialogKind = "error" | "info" | "warning";
type UpdateDialogPrimaryAction = "force" | "download" | "restart" | null | undefined;
type ConfigSaveErrorDialogKind = "warning" | "error";
type ArchiveImportPreview = {
  fileName: string;
  total: number;
  imported: number;
  replaced: number;
} | null;
const props = defineProps<{
  updateDialogOpen: boolean;
  updateDialogTitle: string;
  updateDialogBody: string;
  updateDialogKind: UpdateDialogKind;
  updateDialogReleaseUrl?: string;
  updateDialogPrimaryAction?: UpdateDialogPrimaryAction;
  updateProgressPercent?: number | null;
  updateDialogSkipVersionVisible?: boolean;
  updateDialogCancelUpdateVisible?: boolean;
  updateDialogCancelPending?: boolean;
  markdownIsDark?: boolean;
  runtimeLogsDialogOpen: boolean;
  runtimeLogs: RuntimeLogEntry[];
  runtimeLogsLoading: boolean;
  runtimeLogsError: string;
  rewindConfirmDialogOpen: boolean;
  rewindConfirmCanUndoPatch: boolean;
  branchFromMessageConfirmDialogOpen: boolean;
  configSaveErrorDialogOpen: boolean;
  configSaveErrorDialogTitle: string;
  configSaveErrorDialogBody: string;
  configSaveErrorDialogKind: ConfigSaveErrorDialogKind;
  archiveImportPreviewDialogOpen: boolean;
  archiveImportPreview: ArchiveImportPreview;
  archiveImportRunning: boolean;
  skillPlaceholderDialogOpen: boolean;
  trimActionDialogOpen: boolean;
  trimPreviewLoading: boolean;
  trimPreview: TrimPreviewResult | null;
  trimCompactionPreview: TrimCompactionPreviewResult | null;
  trimming: boolean;
}>();

const emit = defineEmits<{
  closeUpdateDialog: [];
  confirmUpdateDialogPrimary: [];
  openUpdateRelease: [];
  openUpdateRepository: [];
  skipUpdateVersion: [];
  cancelUpdate: [];
  closeRuntimeLogsDialog: [];
  refreshRuntimeLogs: [];
  clearRuntimeLogs: [];
  confirmRewindWithPatch: [];
  confirmRewindMessageOnly: [];
  cancelRewindConfirm: [];
  confirmBranchFromMessage: [];
  cancelBranchFromMessageConfirm: [];
  closeSettingsSaveErrorDialog: [];
  closeArchiveImportPreviewDialog: [];
  confirmArchiveImport: [];
  closeSkillPlaceholderDialog: [];
  confirmTrimCompactionAction: [];
  confirmTrimAction: [];
  confirmTrimDeleteAction: [];
  closeTrimActionDialog: [];
}>();

const { t } = useI18n();

function handleConfirmTrimAction() {
  emit("confirmTrimAction");
}

function handleConfirmTrimDeleteAction() {
  emit("confirmTrimDeleteAction");
}

function handleCloseTrimActionDialog() {
  emit("closeTrimActionDialog");
}

function handleConfirmTrimCompactionAction() {
  emit("confirmTrimCompactionAction");
}

function updateDialogCloseLabel() {
  if (props.updateDialogPrimaryAction) return t("common.cancel");
  if (props.updateDialogCancelUpdateVisible) return t("common.close");
  return t("common.confirm");
}

function canShowUpdateSecondaryActions() {
  return true;
}
</script>

<template>
  <dialog class="modal" :class="{ 'modal-open': updateDialogOpen }">
    <div class="modal-box w-[min(92vw,48rem)] max-w-[48rem] flex max-h-[85dvh] flex-col overflow-hidden">
      <h3 class="font-semibold text-base">
        {{ updateDialogTitle }}
      </h3>
      <progress
        v-if="typeof updateProgressPercent === 'number'"
        class="progress progress-primary mt-3 w-full"
        :value="Math.max(0, Math.min(100, updateProgressPercent))"
        max="100"
      />
      <pre
        v-if="updateDialogKind === 'error'"
        class="mt-3 min-h-0 flex-1 whitespace-pre-wrap break-words text-sm overflow-y-auto text-error"
      >{{ updateDialogBody }}</pre>
      <div v-else class="mt-3 min-h-0 flex-1 overflow-x-hidden overflow-y-auto">
        <AppMarkdownRenderer
          :text="updateDialogBody"
          :is-dark="!!props.markdownIsDark"
          variant="document"
        />
      </div>
      <div class="modal-action mt-4 flex items-center justify-between gap-3">
        <button
          class="btn btn-sm btn-warning"
          @click="emit('openUpdateRepository')"
        >
          <Star class="h-4 w-4" />
          {{ t("about.starAuthor") }}
        </button>
        <div class="flex items-center gap-2">
          <button
            v-if="updateDialogPrimaryAction"
            class="btn btn-sm btn-primary"
            @click="emit('confirmUpdateDialogPrimary')"
          >
            {{
              updateDialogPrimaryAction === 'force'
                ? t("dialogs.update.forceDownload")
                : updateDialogPrimaryAction === 'restart'
                  ? t("dialogs.update.restart")
                  : t("dialogs.update.download")
              }}
          </button>
          <button
            v-if="canShowUpdateSecondaryActions() && updateDialogCancelUpdateVisible"
            class="btn btn-sm btn-warning btn-outline"
            :disabled="updateDialogCancelPending"
            @click="emit('cancelUpdate')"
          >
            {{ updateDialogCancelPending ? t("about.cancellingUpdate") : t("about.cancelUpdate") }}
          </button>
          <button
            v-if="canShowUpdateSecondaryActions() && updateDialogReleaseUrl"
            class="btn btn-sm"
            @click="emit('openUpdateRelease')"
          >
            {{ t("dialogs.update.openReleases") }}
          </button>
          <button
            v-if="canShowUpdateSecondaryActions() && updateDialogSkipVersionVisible"
            class="btn btn-sm btn-ghost"
            @click="emit('skipUpdateVersion')"
          >
            {{ t("about.skipVersion") }}
          </button>
          <button class="btn btn-sm" @click="emit('closeUpdateDialog')">
            {{ updateDialogCloseLabel() }}
          </button>
        </div>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('closeUpdateDialog')">close</button>
    </form>
  </dialog>

  <RuntimeLogsDialog
    :open="runtimeLogsDialogOpen"
    :logs="runtimeLogs"
    :loading="runtimeLogsLoading"
    :error-text="runtimeLogsError"
    @close="emit('closeRuntimeLogsDialog')"
    @refresh="emit('refreshRuntimeLogs')"
    @clear="emit('clearRuntimeLogs')"
  />

  <dialog class="modal" :class="{ 'modal-open': rewindConfirmDialogOpen }">
    <div class="modal-box max-w-md">
      <h3 class="font-semibold text-base">{{ t("dialogs.rewind.title") }}</h3>
      <div class="mt-2 text-sm opacity-80">{{ t("dialogs.rewind.hint") }}</div>
      <div class="mt-4 flex flex-col items-center gap-2">
        <button
          v-if="rewindConfirmCanUndoPatch"
          class="btn btn-sm btn-error w-full"
          @click="emit('confirmRewindWithPatch')"
        >
          {{ t("dialogs.rewind.withPatch") }}
        </button>
        <button class="btn btn-sm w-full" @click="emit('confirmRewindMessageOnly')">
          {{ t("dialogs.rewind.messageOnly") }}
        </button>
        <button class="btn btn-sm btn-primary w-full" @click="emit('cancelRewindConfirm')">{{ t("common.cancel") }}</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('cancelRewindConfirm')">close</button>
    </form>
  </dialog>

  <dialog class="modal" :class="{ 'modal-open': branchFromMessageConfirmDialogOpen }">
    <div class="modal-box max-w-md">
      <h3 class="font-semibold text-base">{{ t("dialogs.branchFromMessage.title") }}</h3>
      <div class="mt-2 text-sm opacity-80">{{ t("dialogs.branchFromMessage.hint") }}</div>
      <div class="modal-action">
        <button class="btn btn-sm" @click="emit('cancelBranchFromMessageConfirm')">{{ t("common.cancel") }}</button>
        <button class="btn btn-sm btn-primary" @click="emit('confirmBranchFromMessage')">{{ t("dialogs.branchFromMessage.confirm") }}</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('cancelBranchFromMessageConfirm')">close</button>
    </form>
  </dialog>

  <dialog class="modal" :class="{ 'modal-open': configSaveErrorDialogOpen }">
    <div class="modal-box max-w-md">
      <h3 class="font-semibold text-base">
        {{ configSaveErrorDialogTitle }}
      </h3>
      <pre
        class="mt-2 whitespace-pre-wrap text-sm"
        :class="configSaveErrorDialogKind === 'warning' ? 'text-warning' : 'text-error'"
      >{{ configSaveErrorDialogBody }}</pre>
      <div class="modal-action">
        <button class="btn btn-sm btn-primary" @click="emit('closeSettingsSaveErrorDialog')">{{ t("common.close") }}</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('closeSettingsSaveErrorDialog')">close</button>
    </form>
  </dialog>

  <dialog class="modal" :class="{ 'modal-open': archiveImportPreviewDialogOpen }">
    <div class="modal-box max-w-md">
      <h3 class="font-semibold text-base">
        {{ t("archives.importPreviewTitle") }}
      </h3>
      <div v-if="archiveImportPreview" class="mt-3 space-y-1 text-sm">
        <div>{{ t("archives.importPreviewFile", { name: archiveImportPreview.fileName }) }}</div>
        <div>{{ t("archives.importPreviewTotal", { count: archiveImportPreview.total }) }}</div>
        <div>{{ t("archives.importPreviewAdd", { count: archiveImportPreview.imported }) }}</div>
        <div>{{ t("archives.importPreviewReplace", { count: archiveImportPreview.replaced }) }}</div>
        <div class="text-sm opacity-70 mt-2">{{ t("archives.importPreviewHint") }}</div>
      </div>
      <div class="modal-action">
        <button class="btn btn-sm" :disabled="archiveImportRunning" @click="emit('closeArchiveImportPreviewDialog')">
          {{ t("common.cancel") }}
        </button>
        <button class="btn btn-sm btn-primary" :disabled="archiveImportRunning" @click="emit('confirmArchiveImport')">
          {{ archiveImportRunning ? t("common.loading") : t("archives.importConfirm") }}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('closeArchiveImportPreviewDialog')">close</button>
    </form>
  </dialog>

  <dialog class="modal" :class="{ 'modal-open': skillPlaceholderDialogOpen }">
    <div class="modal-box max-w-md">
      <h3 class="font-semibold text-base">{{ t("dialogs.skill.title") }}</h3>
      <div class="mt-2 text-sm opacity-80">{{ t("dialogs.skill.placeholder") }}</div>
      <div class="modal-action">
        <button class="btn btn-sm btn-primary" @click="emit('closeSkillPlaceholderDialog')">{{ t("common.close") }}</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('closeSkillPlaceholderDialog')">close</button>
    </form>
  </dialog>

  <ConversationMaintenanceDialog
    :open="trimActionDialogOpen"
    :loading="trimPreviewLoading"
    :running="trimming"
    :preview="trimPreview"
    :compaction-preview="trimCompactionPreview"
    @close="handleCloseTrimActionDialog"
    @confirm-compaction="handleConfirmTrimCompactionAction"
    @confirm-archive="handleConfirmTrimAction"
    @confirm-delete="handleConfirmTrimDeleteAction"
  />
</template>

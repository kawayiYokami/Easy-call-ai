<template>
  <dialog
    ref="dialogRef"
    class="modal"
    @close="onDialogClose"
    @cancel.prevent="onDialogClose"
  >
    <div class="modal-box flex max-h-[calc(100dvh-4rem)] w-full max-w-2xl flex-col overflow-hidden p-0">
      <div class="flex shrink-0 flex-col gap-3 border-b border-base-300 px-4 py-3">
        <div class="flex items-start justify-between gap-3">
          <div class="min-w-0">
            <div class="text-sm font-semibold">{{ t("chat.workspacePickerTitle") }}</div>
            <div class="mt-1 text-xs opacity-70">{{ t("chat.workspacePickerHint") }}</div>
          </div>
          <button
            v-if="!hideAddWorkspace"
            class="btn btn-sm shrink-0"
            type="button"
            :disabled="saving"
            @click="emit('addWorkspace')"
          >
            {{ t("config.tools.addWorkspace") }}
          </button>
        </div>
        <label class="block">
          <span class="sr-only">{{ t("chat.workspacePickerSearchPlaceholder") }}</span>
          <input
            v-model="searchQuery"
            type="search"
            class="input input-bordered input-sm w-full"
            :placeholder="t('chat.workspacePickerSearchPlaceholder')"
          />
        </label>
      </div>
      <div class="min-h-0 flex-1 overflow-y-auto">
        <div
          v-if="filteredWorkspaces.length === 0"
          class="m-4 rounded-box border border-dashed border-base-300 bg-base-200/20 px-4 py-6 text-center text-sm opacity-70"
        >
          {{ searchQuery.trim() ? t("chat.workspacePickerSearchEmpty") : t("chat.workspacePickerEmpty") }}
        </div>
        <template v-else>
          <div
            v-if="hiddenWorkspaceCount > 0"
            class="sticky top-0 z-10 border-b border-base-300 bg-base-100/95 px-3 py-2 text-xs opacity-70 backdrop-blur"
          >
            {{ t("chat.workspacePickerMoreMatches", { count: hiddenWorkspaceCount }) }}
          </div>
          <div class="divide-y divide-base-300">
          <div
            v-for="item in visibleWorkspaces"
            :key="item.id"
            class="px-3 py-3 text-left"
            :title="item.path"
          >
            <div class="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-2">
              <div class="min-w-0 flex-1 text-left">
                <div class="flex min-w-0 flex-wrap items-center gap-2">
                  <span
                    class="min-w-0 flex-1 truncate text-sm font-medium"
                    :title="item.path"
                  >{{ item.name }}</span>
                  <select
                    v-if="item.level === 'main'"
                    class="select select-sm select-bordered w-40 shrink-0"
                    :value="workMode"
                    :disabled="saving"
                    :title="worktreeCheckMessage || undefined"
                    @change="onWorkModeChange"
                  >
                    <option value="directory">{{ t("chat.workspaceWorkModeDirectory") }}</option>
                    <option
                      v-if="worktreeAvailable || workMode === 'independent_worktree' || Boolean(worktreeCheckMessage)"
                      value="independent_worktree"
                      :disabled="!worktreeAvailable && workMode !== 'independent_worktree'"
                    >
                      {{ t("chat.workspaceWorkModeIndependent") }}
                    </option>
                    <option
                      v-if="worktreeAvailable || workMode === 'isolated_worktree' || Boolean(worktreeCheckMessage)"
                      value="isolated_worktree"
                      :disabled="!worktreeAvailable && workMode !== 'isolated_worktree'"
                    >
                      {{ t("chat.workspaceWorkModeIsolated") }}
                    </option>
                  </select>
                </div>
              </div>
              <div class="flex shrink-0 flex-wrap items-center justify-end gap-2">
                <button
                  v-if="canSetAsTerminalDirectory(item)"
                  class="btn btn-sm btn-ghost"
                  type="button"
                  :disabled="saving"
                  :title="t('config.tools.setWorkspaceAsMain')"
                  @click="emit('setMain', item.id)"
                >
                  <SquareTerminal class="h-4 w-4" />
                </button>
                <button
                  v-else-if="isCurrentTerminalDirectory(item)"
                  class="btn btn-sm btn-primary pointer-events-none opacity-100"
                  type="button"
                  aria-disabled="true"
                  tabindex="-1"
                  :title="t('config.tools.currentMainWorkspace')"
                >
                  <SquareTerminal class="h-4 w-4" />
                </button>
                <select
                  v-if="item.level !== 'system' && !autonomousMode"
                  class="select select-sm select-bordered w-32"
                  :disabled="saving"
                  :value="item.access"
                  @change="onAccessChange(item.id, $event)"
                >
                  <option value="full_access">{{ accessLabel("full_access") }}</option>
                  <option value="approval">{{ accessLabel("approval") }}</option>
                  <option value="read_only">{{ accessLabel("read_only") }}</option>
                </select>
                <button
                  v-if="item.level !== 'system'"
                  class="btn btn-sm btn-ghost text-error"
                  type="button"
                  :disabled="saving"
                  :title="t('config.tools.delete')"
                  @click="emit('removeWorkspace', item.id)"
                >
                  <Trash2 class="h-4 w-4" />
                </button>
              </div>
            </div>
          </div>
          </div>
        </template>
      </div>
      <div class="flex shrink-0 flex-wrap items-center justify-between gap-3 border-t border-base-300 px-4 py-3">
        <label
          class="flex max-w-64 shrink-0 cursor-pointer items-center gap-2 rounded-full bg-base-200 px-3 py-2 text-xs font-medium leading-tight"
          :title="t('chat.workspacePickerAutonomousHint')"
        >
          <span class="whitespace-normal">{{ t("chat.workspacePickerAutonomous") }}</span>
          <input
            type="checkbox"
            class="checkbox checkbox-primary checkbox-sm"
            :checked="autonomousMode"
            :disabled="saving"
            @change="onAutonomousModeChange"
          />
        </label>
        <div class="ml-auto flex min-w-0 flex-wrap items-center justify-end gap-3">
          <span v-if="validationMessage" class="max-w-72 min-w-0 break-words text-right text-xs text-error">{{ validationMessage }}</span>
          <div class="flex items-center gap-2">
          <button class="btn btn-sm btn-ghost" type="button" :disabled="saving" @click="emit('close')">
          {{ t("common.cancel") }}
          </button>
          <button class="btn btn-sm btn-primary" type="button" :disabled="saving" @click="emit('save')">
            {{ saving ? t("common.saving") : t("common.save") }}
          </button>
          </div>
        </div>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="onDialogClose">close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { SquareTerminal, Trash2 } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import type { ChatWorkspaceChoice } from "../../composables/use-chat-workspace";
import type { ShellWorkMode } from "../../../../types/app";

const props = withDefaults(defineProps<{
  open: boolean;
  saving: boolean;
  workspaces: ChatWorkspaceChoice[];
  autonomousMode: boolean;
  workMode: ShellWorkMode;
  worktreeAvailable: boolean;
  worktreeCheckMessage?: string;
  validationMessage?: string;
  hideAddWorkspace?: boolean;
}>(), {
  hideAddWorkspace: false,
});

const emit = defineEmits<{
  (e: "close"): void;
  (e: "addWorkspace"): void;
  (e: "setMain", workspaceId: string): void;
  (e: "setAccess", workspaceId: string, access: ChatWorkspaceChoice["access"]): void;
  (e: "setAutonomousMode", enabled: boolean): void;
  (e: "setWorkMode", mode: ShellWorkMode): void;
  (e: "removeWorkspace", workspaceId: string): void;
  (e: "openDir", workspaceId: string): void;
  (e: "save"): void;
}>();

const { t } = useI18n();
const dialogRef = ref<HTMLDialogElement | null>(null);

function onDialogClose() {
  if (props.saving) {
    const d = dialogRef.value;
    if (d && !d.open && props.open) d.showModal();
    return;
  }
  emit("close");
}

function syncDialog() {
  const d = dialogRef.value;
  if (!d) return;
  if (props.open) {
    if (!d.open) d.showModal();
  } else if (d.open) d.close();
}

watch(() => props.open, syncDialog);
watch(dialogRef, syncDialog);

const MAX_DISPLAYED_WORKSPACES = 100;
const searchQuery = ref("");
const displayedWorkspaces = computed(() => props.workspaces.filter((item) => item.level !== "system"));
const filteredWorkspaces = computed(() => {
  const query = searchQuery.value.trim().toLowerCase();
  if (!query) return displayedWorkspaces.value;
  return displayedWorkspaces.value.filter((item) => {
    const name = String(item.name || "").toLowerCase();
    const path = String(item.path || "").toLowerCase();
    return name.includes(query) || path.includes(query);
  });
});
const visibleWorkspaces = computed(() => filteredWorkspaces.value.slice(0, MAX_DISPLAYED_WORKSPACES));
const hiddenWorkspaceCount = computed(() => Math.max(0, filteredWorkspaces.value.length - visibleWorkspaces.value.length));

const hasExplicitTerminalDirectory = computed(() => props.workspaces.some((item) => item.level === "main"));

function isCurrentTerminalDirectory(item: ChatWorkspaceChoice): boolean {
  return item.level === "main" || (item.level === "system" && !hasExplicitTerminalDirectory.value);
}

function canSetAsTerminalDirectory(item: ChatWorkspaceChoice): boolean {
  return item.level !== "system" && !isCurrentTerminalDirectory(item);
}

function accessLabel(access: string): string {
  if (access === "approval") return t("config.tools.workspaceAccessApproval");
  if (access === "full_access") return t("config.tools.workspaceAccessFullAccess");
  return t("config.tools.workspaceAccessReadOnly");
}

function onAccessChange(workspaceId: string, event: Event) {
  const nextAccess = String((event.target as HTMLSelectElement | null)?.value || "").trim();
  if (nextAccess !== "approval" && nextAccess !== "full_access" && nextAccess !== "read_only") {
    return;
  }
  emit("setAccess", workspaceId, nextAccess);
}

function onAutonomousModeChange(event: Event) {
  emit("setAutonomousMode", Boolean((event.target as HTMLInputElement | null)?.checked));
}

function onWorkModeChange(event: Event) {
  const nextMode = String((event.target as HTMLSelectElement | null)?.value || "").trim();
  if (nextMode !== "directory" && nextMode !== "isolated_worktree" && nextMode !== "independent_worktree") return;
  emit("setWorkMode", nextMode);
}
</script>

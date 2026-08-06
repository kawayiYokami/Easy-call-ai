<template>
  <dialog class="modal" :class="{ 'modal-open': open }">
    <div class="modal-box w-full max-w-lg rounded-2xl border border-base-300 bg-base-100 p-0 shadow-2xl">
      <div class="border-b border-base-300 px-4 py-3">
        <div class="text-sm font-semibold">{{ t("chat.workspacePickerTitle") }}</div>
        <div class="mt-1 text-xs opacity-70">{{ hintText }}</div>
      </div>
      <div class="space-y-4 px-4 py-4">
        <label class="grid w-full gap-1">
          <span class="text-xs">{{ pathLabel }}</span>
          <div class="join w-full">
            <input
              :value="manualPath"
              class="input input-bordered input-sm join-item min-w-0 flex-1 font-mono"
              type="text"
              :disabled="saving"
              :placeholder="placeholderText"
              @input="emit('update:manualPath', ($event.target as HTMLInputElement | null)?.value || '')"
              @keydown.enter.prevent="manualPath.trim() && emit('browse', manualPath)"
            />
            <button
              type="button"
              class="btn btn-sm join-item"
              :disabled="saving || loading || !manualPath.trim()"
              @click="emit('browse', manualPath)"
            >
              {{ browseLabel }}
            </button>
          </div>
        </label>
        <div class="mt-2 rounded-box border border-base-300 bg-base-200/30">
          <div class="flex items-center gap-2 border-b border-base-300 px-2 py-2">
            <button
              type="button"
              class="btn btn-xs"
              :disabled="saving || loading || !parentPath"
              @click="parentPath && emit('browse', parentPath)"
            >
              {{ parentLabel }}
            </button>
            <div class="min-w-0 flex-1 truncate font-mono text-xs" :title="browserPath || manualPath">
              {{ browserPath || manualPath || emptyPathText }}
            </div>
            <button
              type="button"
              class="btn btn-xs btn-ghost"
              :disabled="saving || loading || !browserPath"
              @click="browserPath && emit('browse', browserPath)"
            >
              {{ refreshLabel }}
            </button>
          </div>
          <div class="max-h-64 overflow-y-auto py-1">
            <div v-if="loading" class="flex items-center gap-2 px-3 py-3 text-sm text-base-content/65">
              <span class="loading loading-spinner loading-xs"></span>
              {{ loadingText }}
            </div>
            <div v-else-if="errorText" class="px-3 py-3 text-sm text-error">
              {{ errorText }}
            </div>
            <div v-else-if="directories.length === 0" class="px-3 py-3 text-sm text-base-content/55">
              {{ emptyDirectoryText }}
            </div>
            <template v-else>
              <button
                v-for="item in directories"
                :key="item.path"
                type="button"
                class="flex min-h-8 w-full items-center gap-2 px-3 py-1.5 text-left text-sm hover:bg-base-300/60"
                :disabled="saving"
                :title="item.path"
                @click="emit('browse', item.path)"
              >
                <span class="shrink-0 text-base-content/55">▸</span>
                <span class="min-w-0 flex-1 truncate">{{ item.name }}</span>
              </button>
            </template>
          </div>
        </div>
        <div class="grid gap-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-end">
          <label v-if="!autonomousMode" class="grid w-full gap-1">
            <span class="text-xs">{{ accessLabelText }}</span>
            <select
              :value="access"
              class="select select-bordered select-sm w-full"
              :disabled="saving"
              @change="emit('update:access', ($event.target as HTMLSelectElement | null)?.value || 'approval')"
            >
              <option value="approval">{{ t("config.tools.workspaceAccessApproval") }}</option>
              <option value="full_access">{{ t("config.tools.workspaceAccessFullAccess") }}</option>
              <option value="read_only">{{ t("config.tools.workspaceAccessReadOnly") }}</option>
            </select>
          </label>
          <label
            class="flex cursor-pointer items-center gap-2 rounded-box bg-base-200 px-3 py-2 text-xs"
            :title="t('chat.workspacePickerAutonomousHint')"
          >
            <span>{{ t("chat.workspacePickerAutonomous") }}</span>
            <input
              :checked="autonomousMode"
              type="checkbox"
              class="checkbox checkbox-primary checkbox-sm"
              :disabled="saving"
              @change="emit('update:autonomousMode', Boolean(($event.target as HTMLInputElement | null)?.checked))"
            />
          </label>
        </div>
      </div>
      <div class="flex items-center justify-end gap-2 border-t border-base-300 px-4 py-3">
        <button class="btn btn-sm btn-ghost" type="button" :disabled="saving" @click="emit('close')">
          {{ t("common.cancel") }}
        </button>
        <button
          class="btn btn-sm btn-primary"
          type="button"
          :disabled="saving || !manualPath.trim()"
          @click="emit('save')"
        >
          {{ saving ? t("common.saving") : saveLabel }}
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('close')">close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

type DirectoryItem = {
  path: string;
  name: string;
};

const props = withDefaults(defineProps<{
  open: boolean;
  saving: boolean;
  loading: boolean;
  errorText: string;
  browserPath: string;
  manualPath: string;
  access: string;
  autonomousMode: boolean;
  directories: DirectoryItem[];
  saveLabel?: string;
  hintText?: string;
  pathLabel?: string;
  browseLabel?: string;
  parentLabel?: string;
  refreshLabel?: string;
  loadingText?: string;
  emptyPathText?: string;
  emptyDirectoryText?: string;
  accessLabelText?: string;
  placeholderText?: string;
}>(), {
  saveLabel: "使用此目录",
  hintText: "可手动输入由当前后端访问的工作目录路径。",
  pathLabel: "工作目录路径",
  browseLabel: "浏览",
  parentLabel: "上一级",
  refreshLabel: "刷新",
  loadingText: "正在读取目录",
  emptyPathText: "输入路径后开始浏览",
  emptyDirectoryText: "当前目录没有可继续进入的子目录",
  accessLabelText: "访问权限",
  placeholderText: "例如 E:\\github\\easy_call_ai 或 /home/me/project",
});

const emit = defineEmits<{
  (e: "close"): void;
  (e: "browse", path: string): void;
  (e: "save"): void;
  (e: "update:manualPath", value: string): void;
  (e: "update:access", value: string): void;
  (e: "update:autonomousMode", value: boolean): void;
}>();

const { t } = useI18n();

const parentPath = computed(() => {
  const normalized = String(props.browserPath || props.manualPath || "").trim().replace(/[\\/]+$/, "");
  if (!normalized) return "";
  const separatorIndex = Math.max(normalized.lastIndexOf("/"), normalized.lastIndexOf("\\"));
  if (separatorIndex < 0) return "";
  if (separatorIndex === 0) return normalized.slice(0, 1);
  const windowsDriveRoot = /^[A-Za-z]:[\\/]?$/.test(normalized.slice(0, separatorIndex + 1));
  if (windowsDriveRoot) return normalized.slice(0, separatorIndex + 1);
  return normalized.slice(0, separatorIndex);
});
</script>

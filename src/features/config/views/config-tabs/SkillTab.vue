<template>
  <SettingsStickyLayout>
    <template #header>
      <div class="flex flex-wrap items-center justify-between gap-3">
        <div class="flex min-w-0 flex-1 flex-wrap items-center gap-2">
          <div class="text-sm font-semibold">SKILL 列表</div>
          <select
            v-if="skills.length > 0"
            v-model="selectedSkillPath"
            class="select select-bordered min-w-[12rem] flex-1 max-w-full"
            :disabled="loading"
          >
            <option v-for="item in skills" :key="item.path" :value="item.path">
              {{ item.name }}
            </option>
          </select>
        </div>
        <div class="flex flex-wrap items-center justify-end gap-2">
          <button class="btn btn-sm bg-base-100" type="button" @click="reload" :disabled="loading">
            <RefreshCw class="h-4 w-4" :class="{ 'animate-spin': loading }" />
            刷新
          </button>
          <button v-if="localFileSystemAvailable" class="btn btn-sm bg-base-100" type="button" @click="openSkillsDir" :disabled="loading">
            <FolderOpen class="h-4 w-4" />
            打开目录
          </button>
        </div>
      </div>
    </template>

    <div class="grid gap-3">
      <div v-if="loading" class="text-sm opacity-70">加载中...</div>

    <div v-if="selectedSkill" class="card bg-base-100 card-border border-base-300 card-sm overflow-hidden">
      <div class="card-body gap-4">
        <!-- 上栏：标题和描述 -->
        <div>
          <div class="text-sm font-semibold">{{ selectedSkill.name }}</div>
          <div class="text-xs opacity-60 mt-1">描述</div>
          <div class="text-sm opacity-80 whitespace-pre-wrap mt-1">{{ selectedSkill.description || "(无描述)" }}</div>
        </div>
      </div>
      <!-- 下栏深色背景：正文 -->
      <div class="bg-base-300">
        <div class="flex flex-col gap-2 p-4">
          <div class="text-xs opacity-60">正文</div>
          <div class="text-sm whitespace-pre-wrap break-words max-h-[60vh] overflow-auto">{{ selectedSkill.content || "(无正文)" }}</div>
          <div class="text-xs opacity-60 break-all">{{ selectedSkill.path }}</div>
        </div>
      </div>
    </div>

    <div v-if="statusText" class="text-sm" :class="statusError ? 'text-error' : 'opacity-70'">
      {{ statusText }}
    </div>
  </div>
  </SettingsStickyLayout>
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { FolderOpen, RefreshCw } from "@lucide/vue";
import { getTransportCapabilities, invokeTauri, openTransportSkillWorkspaceDirectory } from "../../../../services/tauri-api";
import type { SkillListResult, SkillSummaryItem } from "../../../../types/app";
import { toErrorMessage } from "../../../../utils/error";
import SettingsStickyLayout from "../../components/SettingsStickyLayout.vue";

const loading = ref(false);
const statusText = ref("");
const statusError = ref(false);
const skills = ref<SkillSummaryItem[]>([]);
const selectedSkillPath = ref("");
const localFileSystemAvailable = getTransportCapabilities().localFileSystem;

const selectedSkill = computed(() => skills.value.find((v) => v.path === selectedSkillPath.value) ?? null);

function ensureSelectedSkill() {
  if (skills.value.length === 0) {
    selectedSkillPath.value = "";
    return;
  }
  if (!skills.value.some((v) => v.path === selectedSkillPath.value)) {
    selectedSkillPath.value = skills.value[0].path;
  }
}

function setStatus(text: string, isError = false) {
  statusText.value = text;
  statusError.value = isError;
}

async function reload() {
  loading.value = true;
  try {
    const result = await invokeTauri<SkillListResult>("mcp_list_skills");
    skills.value = result?.skills || [];
    ensureSelectedSkill();
    if ((result?.errors?.length || 0) > 0) {
      setStatus(`已加载 ${skills.value.length} 个 SKILL，${result.errors.length} 个目录读取失败`, true);
    } else {
      setStatus(`已加载 ${skills.value.length} 个 SKILL`);
    }
  } catch (error) {
    setStatus(`刷新失败: ${toErrorMessage(error)}`, true);
  } finally {
    loading.value = false;
  }
}

async function openSkillsDir() {
  if (!localFileSystemAvailable || loading.value) return;
  loading.value = true;
  try {
    const opened = await openTransportSkillWorkspaceDirectory();
    setStatus(`已打开目录: ${opened}`);
  } catch (error) {
    setStatus(`打开目录失败: ${toErrorMessage(error)}`, true);
  } finally {
    loading.value = false;
  }
}

onMounted(() => {
  void reload();
});
</script>

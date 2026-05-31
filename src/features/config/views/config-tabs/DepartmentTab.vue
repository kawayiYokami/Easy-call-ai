<template>
  <SettingsStickyLayout>
    <template #header>
      <div class="card border border-base-300 bg-base-100">
        <div class="card-body p-4">
          <div class="flex w-full flex-col gap-3">
            <div class="flex items-center justify-between">
              <span class="text-sm">{{ t("config.department.title") }}</span>
            </div>

            <div class="flex gap-1">
              <select
                :value="selectedDepartmentId"
                class="select select-bordered select-sm flex-1"
                @change="switchSelectedDepartment(($event.target as HTMLSelectElement).value)"
              >
                <option v-for="department in sortedDepartments" :key="department.id" :value="department.id">
                  {{ department.name }}{{ department.isBuiltInAssistant ? `（${t("config.department.assistantBadge")}）` : (department.source === "private_workspace" ? `（${t("config.department.privateWorkspaceBadge")}）` : "") }}
                </option>
              </select>

              <button
                class="btn btn-sm btn-square bg-base-200"
                type="button"
                :title="t('config.department.add')"
                :disabled="savingConfig"
                @click="addDepartment"
              >
                <Plus class="h-3.5 w-3.5" />
              </button>

              <button
                class="btn btn-sm btn-square"
                type="button"
                :class="!departmentDirty ? 'cursor-not-allowed bg-base-200 text-base-content/30' : 'bg-base-200'"
                :title="t('common.reset')"
                :disabled="!departmentDirty || savingConfig"
                @click="restoreDepartmentDraftsFromSaved"
              >
                <RotateCcw class="h-3.5 w-3.5" />
              </button>

              <button
                class="btn btn-sm btn-square transition-all duration-300"
                type="button"
                :class="departmentDirty ? 'btn-primary' : 'bg-base-200 text-base-content/50 shadow-none'"
                :disabled="!selectedDepartment || !!departmentValidationMessage || !departmentDirty || savingConfig"
                :title="savingConfig ? t('config.api.saving') : departmentDirty ? t('common.save') : t('status.configSaved')"
                @click="saveDepartments"
              >
                <Save v-if="!savingConfig" class="h-3.5 w-3.5" />
                <span v-else class="loading loading-spinner loading-sm"></span>
              </button>
            </div>

            <div class="text-sm opacity-60">{{ t("config.department.hint") }}</div>
          </div>
        </div>
      </div>
    </template>

    <div v-if="selectedDepartment" class="grid gap-3">
        <div class="overflow-hidden rounded-box border border-base-300 bg-base-100">
          <div v-if="departmentValidationMessage" class="border-b border-warning/30 bg-warning/10 px-4 py-3 text-sm text-warning-content">
            {{ departmentValidationMessage }}
          </div>

          <div class="flex items-center justify-between gap-2 border-b border-base-300 px-4 py-3">
            <div class="flex items-center gap-2">
              <div class="font-medium text-base-content">{{ selectedDepartment.name }}</div>
              <span v-if="selectedDepartmentIsPrivateWorkspace" class="badge badge-soft badge-secondary">{{ t("config.department.privateWorkspaceBadge") }}</span>
            </div>

            <button
              class="btn btn-sm btn-ghost"
              type="button"
              :disabled="savingConfig"
              @click="handleSelectedDepartmentPrimaryAction"
            >
              <Trash2 v-if="!selectedDepartmentIsSystemBuiltIn" class="h-4 w-4" />
              {{ selectedDepartmentIsSystemBuiltIn ? t("config.department.restoreInitial") : t("config.department.remove") }}
            </button>
          </div>

          <div class="divide-y divide-base-300">
            <div class="min-w-0 px-4 py-4">
              <div class="mb-2 text-[11px] uppercase tracking-wide opacity-40">{{ t("config.department.name") }}</div>
              <input
                v-model.trim="selectedDepartment.name"
                class="input input-bordered input-sm w-full"
                :placeholder="t('config.department.namePlaceholder')"
                :disabled="selectedDepartmentIsLockedPreset"
                @input="touchSelectedDepartment"
              />
              <div v-if="selectedDepartmentNameEmpty" class="mt-2 text-xs text-error opacity-80">
                {{ t("config.department.emptyName") }}
              </div>
              <div v-if="selectedDepartmentNameDuplicated" class="mt-2 text-xs text-error opacity-80">
                {{ t("config.department.duplicateName") }}
              </div>
            </div>

            <div class="px-4 py-4">
              <div class="mb-2 text-[11px] uppercase tracking-wide opacity-40">{{ t("config.department.assignee") }}</div>
              <div class="grid gap-3">
                <div v-if="selectedDepartmentAssigneeIds.length > 0" class="grid gap-2">
                  <div
                    v-for="(agentId, idx) in selectedDepartmentAssigneeIds"
                    :key="`${selectedDepartment.id}-agent-${agentId}`"
                    class="flex items-center gap-2"
                  >
                    <div class="min-w-0 flex-1 rounded-xl border border-base-300 bg-base-200/60 px-3 py-2">
                      <div class="flex items-center gap-2">
                        <span class="truncate text-sm">{{ personaNameById(agentId) }}</span>
                        <span v-if="idx === 0" class="badge badge-primary badge-sm shrink-0">{{ t("config.department.primaryAssignee") }}</span>
                        <span
                          v-if="selectedDepartment.isBuiltInAssistant && idx === 0 && agentId === assistantDepartmentAgentId"
                          class="text-xs opacity-50"
                        >
                          {{ t("config.department.currentAssistant") }}
                        </span>
                      </div>
                    </div>
                    <div class="join">
                      <button
                        class="btn btn-sm btn-square join-item opacity-60 hover:opacity-100"
                        type="button"
                        :disabled="idx <= 0"
                        :title="t('config.department.moveUp')"
                        @click="moveDepartmentAssignee(idx, -1)"
                      >
                        ↑
                      </button>
                      <button
                        class="btn btn-sm btn-square join-item opacity-60 hover:opacity-100"
                        type="button"
                        :disabled="idx >= selectedDepartmentAssigneeIds.length - 1"
                        :title="t('config.department.moveDown')"
                        @click="moveDepartmentAssignee(idx, 1)"
                      >
                        ↓
                      </button>
                      <button
                        class="btn btn-sm btn-square join-item opacity-60 hover:opacity-100"
                        type="button"
                        :title="t('config.department.remove')"
                        @click="removeDepartmentAssigneeAt(idx)"
                      >
                        <Trash2 class="h-3.5 w-3.5" />
                      </button>
                    </div>
                  </div>
                </div>
                <div v-else class="text-xs opacity-50">
                  {{ t("config.department.assigneeEmpty") }}
                </div>
                <div class="flex items-center gap-2">
                  <select
                    class="select select-bordered select-sm flex-1"
                    :value="pendingDepartmentAssigneeId"
                    :disabled="departmentAssigneeLimitReached"
                    @change="pendingDepartmentAssigneeId = ($event.target as HTMLSelectElement).value"
                  >
                    <option value="">{{ t("config.department.assigneePlaceholder") }}</option>
                    <option v-for="persona in remainingAssigneePersonas" :key="persona.id" :value="persona.id">
                      {{ persona.name }}
                    </option>
                  </select>
                  <button
                    class="btn btn-sm"
                    type="button"
                    :disabled="!pendingDepartmentAssigneeId || departmentAssigneeLimitReached"
                    @click="addDepartmentAssignee"
                  >
                    {{ t("config.department.addAssignee") }}
                  </button>
                </div>
                <div class="text-[11px] opacity-50">{{ t("config.department.assigneePrimaryHint") }}</div>
                <div v-if="departmentAssigneeLimitReached" class="text-[11px] text-warning opacity-80">
                  {{ t("config.department.assigneeLimitHint") }}
                </div>
              </div>
            </div>

            <div class="px-4 py-4">
              <div class="mb-2 text-[11px] uppercase tracking-wide opacity-40">{{ t("config.department.model") }}</div>
              <div class="grid min-w-0 gap-3">
                <div
                  v-for="(apiId, idx) in selectedDepartmentApiConfigIds"
                  :key="`${selectedDepartment.id}-api-${idx}`"
                  class="flex items-center gap-2"
                >
                  <select
                    class="select select-bordered select-sm flex-1"
                    :value="apiId"
                    @change="updateDepartmentApiConfigAt(idx, ($event.target as HTMLSelectElement).value)"
                  >
                    <option value="">{{ t("config.memory.notConfigured") }}</option>
                    <option v-for="api in availableDepartmentApiConfigsForIndex(idx)" :key="api.id" :value="api.id">{{ api.name }}</option>
                  </select>

                  <div class="join">
                    <button
                      class="btn btn-sm btn-square join-item opacity-60 hover:opacity-100"
                      type="button"
                      :disabled="idx <= 0"
                      :title="t('config.department.moveUp')"
                      @click="moveDepartmentApiConfig(idx, -1)"
                    >
                      ↑
                    </button>
                    <button
                      class="btn btn-sm btn-square join-item opacity-60 hover:opacity-100"
                      type="button"
                      :disabled="idx >= selectedDepartmentApiConfigIds.length - 1"
                      :title="t('config.department.moveDown')"
                      @click="moveDepartmentApiConfig(idx, 1)"
                    >
                      ↓
                    </button>
                    <button
                      class="btn btn-sm btn-square join-item opacity-60 hover:opacity-100"
                      type="button"
                      :disabled="selectedDepartmentApiConfigIds.length <= 1"
                      :title="t('config.department.removeModel')"
                      @click="removeDepartmentApiConfigAt(idx)"
                    >
                      <Trash2 class="h-3.5 w-3.5" />
                    </button>
                  </div>
                </div>

                <button
                  class="btn btn-sm"
                  type="button"
                  :disabled="remainingDepartmentApiConfigs.length <= 0"
                  @click="addDepartmentApiConfig"
                >
                  {{ t("config.department.addModel") }}
                </button>
              </div>

              <div class="mt-2 text-[11px] opacity-50">{{ t("config.department.modelFallbackHint") }}</div>
              <div class="mt-1 text-[11px] opacity-40">{{ t("config.department.allowedModelsNote") }}</div>
            </div>

            <div class="px-4 py-4">
              <div class="mb-2 text-[11px] uppercase tracking-wide opacity-40">{{ t("config.department.summary") }}</div>
              <textarea
                v-model="selectedDepartment.summary"
                class="textarea textarea-bordered textarea-sm min-h-20 w-full"
                :placeholder="t('config.department.summaryPlaceholder')"
                :disabled="selectedDepartmentIsLockedPreset"
                @input="touchSelectedDepartment"
              />
            </div>

            <div class="px-4 py-4">
              <div class="mb-2 text-[11px] uppercase tracking-wide opacity-40">{{ t("config.department.guide") }}</div>
              <textarea
                v-model="selectedDepartment.guide"
                class="textarea textarea-bordered textarea-sm min-h-28 w-full"
                :placeholder="t('config.department.guidePlaceholder')"
                :disabled="selectedDepartmentIsLockedPreset"
                @input="touchSelectedDepartment"
              />
              <div class="mt-2 text-[11px] opacity-40">{{ t("config.department.guideHint") }}</div>
            </div>

            <div class="px-4 py-4">
              <div class="mb-3 flex items-center justify-between gap-3">
                <div>
                  <div class="text-[11px] uppercase tracking-wide opacity-40">{{ t("config.department.permissionTitle") }}</div>
                  <div class="mt-1 text-xs opacity-60">{{ t("config.department.permissionHint") }}</div>
                </div>
                <input
                  type="checkbox"
                  class="toggle toggle-sm toggle-primary"
                  :checked="permissionControlEnabled"
                  @change="updateDepartmentPermissionControl({ enabled: !!($event.target as HTMLInputElement).checked })"
                />
              </div>

              <div class="grid min-w-0 gap-3 overflow-hidden">
                <select
                  class="select select-bordered select-sm w-full"
                  :disabled="permissionListDisabled"
                  :value="selectedDepartmentPermissionControl?.mode || 'blacklist'"
                  @change="updateDepartmentPermissionControl({ mode: (($event.target as HTMLSelectElement).value === 'whitelist' ? 'whitelist' : 'blacklist') })"
                >
                  <option value="blacklist">{{ t("config.department.permissionModeBlacklist") }}</option>
                  <option value="whitelist">{{ t("config.department.permissionModeWhitelist") }}</option>
                </select>
                <div v-if="permissionControlEnabled" class="text-xs opacity-60">
                  {{
                    selectedDepartmentPermissionControl?.mode === "whitelist"
                      ? t("config.department.permissionModeWhitelistHint")
                      : t("config.department.permissionModeBlacklistHint")
                  }}
                </div>

                <div v-if="permissionCatalogLoading" class="text-xs opacity-60">
                  {{ t("config.department.permissionCatalogLoading") }}
                </div>
                <div v-else-if="permissionCatalogError" class="break-all text-xs text-error">
                  {{ t("config.department.permissionCatalogLoadFailed", { err: permissionCatalogError }) }}
                </div>
                <template v-else>
                  <div class="grid min-w-0 gap-2 overflow-hidden">
                    <div class="text-xs font-medium text-base-content/70">{{ t("config.department.permissionBuiltinTools") }}</div>
                    <fieldset class="grid min-w-0 gap-2 overflow-hidden">
                      <button
                        v-for="item in visiblePermissionBuiltinTools"
                        :key="`builtin-${item.name}`"
                        type="button"
                        class="flex min-w-0 w-full max-w-full items-center gap-3 overflow-hidden rounded-xl border px-3 py-2.5 text-left transition"
                        :class="[
                          permissionNameChecked('builtinToolNames', item.name)
                            ? permissionCardTone.card
                            : 'border-base-content/10 bg-base-200 text-base-content',
                          permissionListDisabled
                            ? 'cursor-not-allowed opacity-60'
                            : 'cursor-pointer hover:border-base-content/20',
                        ]"
                        :aria-checked="permissionNameChecked('builtinToolNames', item.name)"
                        role="checkbox"
                        :disabled="permissionListDisabled"
                        @click="toggleBuiltinPermissionName(item.name)"
                      >
                        <span
                          class="flex h-5 w-5 shrink-0 items-center justify-center rounded border transition"
                          :class="permissionNameChecked('builtinToolNames', item.name)
                            ? permissionCardTone.box
                            : 'border-base-content/20 bg-base-200 text-transparent'"
                        >
                          <component :is="permissionCardTone.icon" class="h-3.5 w-3.5" />
                        </span>
                        <span class="min-w-0 flex flex-1 items-center gap-2 overflow-hidden">
                          <span class="max-w-56 shrink-0 truncate text-sm font-semibold" :title="item.name">{{ item.name }}</span>
                          <span
                            v-if="item.description"
                            class="min-w-0 truncate text-xs"
                            :class="permissionNameChecked('builtinToolNames', item.name) ? 'text-base-content/80' : 'text-base-content/80'"
                            :title="item.description"
                          >
                            {{ truncatePermissionDescription(item.description) }}
                          </span>
                        </span>
                      </button>
                    </fieldset>
                  </div>

                  <div class="grid min-w-0 gap-2 overflow-hidden">
                    <div class="text-xs font-medium text-base-content/70">{{ t("config.department.permissionSkills") }}</div>
                    <div v-if="skillPermissionRequiresExec" class="text-xs text-base-content/50">
                      {{ t("config.department.permissionSkillsRequireExec") }}
                    </div>
                    <fieldset class="grid min-w-0 gap-2 overflow-hidden">
                      <button
                        v-for="item in visiblePermissionSkills"
                        :key="`skill-${item.name}`"
                        type="button"
                        class="flex min-w-0 w-full max-w-full items-center gap-3 overflow-hidden rounded-xl border px-3 py-2.5 text-left transition"
                        :class="[
                          permissionNameChecked('skillNames', item.name)
                            ? permissionCardTone.card
                            : 'border-base-content/10 bg-base-200 text-base-content',
                          skillPermissionListDisabled
                            ? 'cursor-not-allowed opacity-60'
                            : 'cursor-pointer hover:border-base-content/20',
                        ]"
                        :aria-checked="permissionNameChecked('skillNames', item.name)"
                        role="checkbox"
                        :disabled="skillPermissionListDisabled"
                        @click="handleSkillPermissionToggle(item.name)"
                      >
                        <span
                          class="flex h-5 w-5 shrink-0 items-center justify-center rounded border transition"
                          :class="permissionNameChecked('skillNames', item.name)
                            ? permissionCardTone.box
                            : 'border-base-content/20 bg-base-200 text-transparent'"
                        >
                          <component :is="permissionCardTone.icon" class="h-3.5 w-3.5" />
                        </span>
                        <span class="min-w-0 flex flex-1 items-center gap-2 overflow-hidden">
                          <span class="max-w-56 shrink-0 truncate text-sm font-semibold" :title="item.name">{{ item.name }}</span>
                          <span
                            v-if="item.description"
                            class="min-w-0 truncate text-xs"
                            :class="permissionNameChecked('skillNames', item.name) ? 'text-base-content/80' : 'text-base-content/80'"
                            :title="item.description"
                          >
                            {{ truncatePermissionDescription(item.description) }}
                          </span>
                        </span>
                      </button>
                    </fieldset>
                  </div>

                  <div class="grid min-w-0 gap-2 overflow-hidden">
                    <div class="text-xs font-medium text-base-content/70">{{ t("config.department.permissionMcpTools") }}</div>
                    <fieldset class="grid min-w-0 gap-2 overflow-hidden">
                      <button
                        v-for="item in permissionCatalog.mcpTools"
                        :key="`mcp-${item.name}`"
                        type="button"
                        class="flex min-w-0 w-full max-w-full items-center gap-3 overflow-hidden rounded-xl border px-3 py-2.5 text-left transition"
                        :class="[
                          permissionNameChecked('mcpToolNames', item.name)
                            ? permissionCardTone.card
                            : 'border-base-content/10 bg-base-200 text-base-content',
                          permissionListDisabled
                            ? 'cursor-not-allowed opacity-60'
                            : 'cursor-pointer hover:border-base-content/20',
                        ]"
                        :aria-checked="permissionNameChecked('mcpToolNames', item.name)"
                        role="checkbox"
                        :disabled="permissionListDisabled"
                        @click="togglePermissionName('mcpToolNames', item.name, !permissionNameChecked('mcpToolNames', item.name))"
                      >
                        <span
                          class="flex h-5 w-5 shrink-0 items-center justify-center rounded border transition"
                          :class="permissionNameChecked('mcpToolNames', item.name)
                            ? permissionCardTone.box
                            : 'border-base-content/20 bg-base-200 text-transparent'"
                        >
                          <component :is="permissionCardTone.icon" class="h-3.5 w-3.5" />
                        </span>
                        <span class="min-w-0 flex flex-1 items-center gap-2 overflow-hidden">
                          <span class="max-w-56 shrink-0 truncate text-sm font-semibold" :title="item.name">{{ item.name }}</span>
                          <span
                            v-if="item.description"
                            class="min-w-0 truncate text-xs"
                            :class="permissionNameChecked('mcpToolNames', item.name) ? 'text-base-content/80' : 'text-base-content/80'"
                            :title="item.description"
                          >
                            {{ truncatePermissionDescription(item.description) }}
                          </span>
                        </span>
                      </button>
                    </fieldset>
                  </div>
                </template>
              </div>
            </div>
          </div>
        </div>
      </div>

    <div v-else class="rounded-box border border-base-300 bg-base-100 p-12 text-center">
      <div class="text-sm opacity-40">{{ t("config.department.selectHint") }}</div>
    </div>
  </SettingsStickyLayout>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { Check, Plus, RotateCcw, Save, Trash2, X } from "@lucide/vue";
import { useI18n } from "vue-i18n";
import { invokeTauri } from "../../../../services/tauri-api";
import type { ApiConfigItem, AppConfig, DepartmentConfig, DepartmentPermissionCatalog, PersonaProfile } from "../../../../types/app";
import {
  buildDepartmentBasicSnapshot,
  departmentBasicComparableSnapshot,
  mergeDepartmentChildIdsFromSource,
} from "../../utils/department-basic-editor";
import { validateDepartmentConfig } from "../../utils/department-validation";
import { normalizeDepartmentChildIds } from "../../utils/department-graph";
import { EXPLORER_DEPARTMENT_DEFAULT, REMOTE_CUSTOMER_SERVICE_DEPARTMENT_DEFAULT } from "../../constants/department-defaults";
import SettingsStickyLayout from "../../components/SettingsStickyLayout.vue";

const props = defineProps<{
  config: AppConfig;
  apiConfigs: ApiConfigItem[];
  personas: PersonaProfile[];
  assistantDepartmentAgentId: string;
  savingConfig: boolean;
  saveConfigAction: () => Promise<boolean> | boolean;
  setStatusAction: (text: string) => void;
}>();

const emit = defineEmits<{
  (e: "update:assistantDepartmentAssigneeId", value: string): void;
}>();

const { t } = useI18n();
const selectedDepartmentId = ref("assistant-department");
const SYSTEM_DEPARTMENT_IDS = new Set([
  "assistant-department",
  "deputy-department",
  "remote-customer-service-department",
]);

const TEXT_REQUEST_FORMATS = new Set([
  "auto",
  "openai",
  "deepseek",
  "openai_responses",
  "codex",
  "gemini",
  "anthropic",
  "fireworks",
  "together",
  "groq",
  "mimo",
  "moonshot",
  "nebius",
  "xai",
  "zai",
  "bigmodel",
  "aliyun",
  "baidu",
  "cohere",
  "ollama",
  "ollama_cloud",
  "vertex",
  "github_copilot",
  "opencode_go",
  "bedrock_api",
]);

function isTextRequestFormat(format: string): boolean {
  const normalized = String(format || "").trim().toLowerCase();
  return normalized === "deepseek/kimi" || TEXT_REQUEST_FORMATS.has(normalized);
}

type DepartmentDefaultSeed = Pick<DepartmentConfig, "name" | "summary" | "guide">;
type DepartmentPermissionNameCategory = "builtinToolNames" | "skillNames" | "mcpToolNames";

function isSystemBuiltInDepartment(department: DepartmentConfig | null | undefined) {
  if (!department) return false;
  const id = String(department.id || "").trim();
  return SYSTEM_DEPARTMENT_IDS.has(id) || !!department.isBuiltInAssistant;
}

function normalizeNameList(value: unknown): string[] {
  return Array.isArray(value)
    ? Array.from(new Set(value.map((item) => String(item || "").trim()).filter(Boolean)))
    : [];
}

function normalizePermissionControl(permissionControl: DepartmentConfig["permissionControl"] | null | undefined) {
  return {
    enabled: !!permissionControl?.enabled,
    mode: permissionControl?.mode === "whitelist" ? "whitelist" : "blacklist",
    builtinToolNames: normalizeNameList(permissionControl?.builtinToolNames),
    skillNames: normalizeNameList(permissionControl?.skillNames),
    mcpToolNames: normalizeNameList(permissionControl?.mcpToolNames),
  } as const;
}

function cloneDepartment(department: DepartmentConfig): DepartmentConfig {
  const apiConfigIds = normalizeNameList(
    Array.isArray(department.apiConfigIds) && department.apiConfigIds.length > 0
      ? department.apiConfigIds
      : [department.apiConfigId || ""],
  );
  const id = String(department.id || "").trim();
  const agentIds = normalizeNameList(department.agentIds);
  return {
    id,
    name: String(department.name || ""),
    summary: String(department.summary || ""),
    guide: String(department.guide || ""),
    apiConfigId: apiConfigIds[0] || "",
    apiConfigIds,
    agentIds,
    childDepartmentIds: normalizeDepartmentChildIds(department.childDepartmentIds, id),
    createdAt: String(department.createdAt || "").trim(),
    updatedAt: String(department.updatedAt || "").trim(),
    orderIndex: Math.max(1, Number(department.orderIndex || 1)),
    isBuiltInAssistant: !!department.isBuiltInAssistant,
    source: String(department.source || "").trim() || "main_config",
    scope: String(department.scope || "").trim() || "global",
    permissionControl: normalizePermissionControl(department.permissionControl),
  };
}

function cloneDepartmentList(departments: DepartmentConfig[] | null | undefined) {
  return (departments || []).map(cloneDepartment);
}

const departmentDrafts = ref<DepartmentConfig[]>(cloneDepartmentList(props.config.departments || []));
const permissionCatalog = ref<DepartmentPermissionCatalog>({
  builtinTools: [],
  skills: [],
  mcpTools: [],
});
const permissionCatalogLoading = ref(false);
const permissionCatalogError = ref("");
const pendingDepartmentAssigneeId = ref("");

const sortedDepartments = computed(() =>
  [...departmentDrafts.value].sort((a, b) => {
    const rank = (id: string) =>
      id === "assistant-department" ? 0 : id === "deputy-department" ? 1 : id === "remote-customer-service-department" ? 2 : 3;
    const aRank = rank(String(a.id || "").trim());
    const bRank = rank(String(b.id || "").trim());
    return aRank - bRank || a.orderIndex - b.orderIndex;
  }),
);

const selectedDepartment = computed(
  () => departmentDrafts.value.find((item) => item.id === selectedDepartmentId.value) ?? sortedDepartments.value[0] ?? null,
);
const selectedDepartmentIsLockedPreset = computed(() => String(selectedDepartment.value?.id || "").trim() === "deputy-department");
const selectedDepartmentIsSystemBuiltIn = computed(() => isSystemBuiltInDepartment(selectedDepartment.value));
const selectedDepartmentIsPrivateWorkspace = computed(() => selectedDepartment.value?.source === "private_workspace");
const textDepartmentApiConfigs = computed(() =>
  props.apiConfigs.filter((api) => !!api.enableText && isTextRequestFormat(api.requestFormat)),
);
const selectedDepartmentApiConfigIds = computed(() =>
  currentDepartmentApiConfigIdsForEditor(selectedDepartment.value),
);
const remainingDepartmentApiConfigs = computed(() => {
  const selectedIds = new Set(selectedDepartmentApiConfigIds.value);
  return textDepartmentApiConfigs.value.filter((api) => !selectedIds.has(api.id));
});
const departmentNameCounts = computed(() => {
  const counts = new Map<string, number>();
  for (const department of departmentDrafts.value) {
    const key = String(department.name || "").trim().toLocaleLowerCase();
    if (!key) continue;
    counts.set(key, (counts.get(key) || 0) + 1);
  }
  return counts;
});
const selectedDepartmentNameDuplicated = computed(() => {
  const key = String(selectedDepartment.value?.name || "").trim().toLocaleLowerCase();
  if (!key) return false;
  return (departmentNameCounts.value.get(key) || 0) > 1;
});
const selectedDepartmentNameEmpty = computed(() => !String(selectedDepartment.value?.name || "").trim());
const sourceDepartmentSnapshot = computed(() => buildDepartmentBasicSnapshot(props.config.departments || []));
const sourceDepartmentRelationSnapshot = computed(() =>
  JSON.stringify(
    (props.config.departments || []).map((item) => ({
      id: String(item.id || "").trim(),
      childDepartmentIds: normalizeDepartmentChildIds(item.childDepartmentIds, item.id),
    })),
  ),
);
const departmentSnapshot = computed(() => buildDepartmentBasicSnapshot(departmentDrafts.value));
const departmentDirty = computed(() => departmentSnapshot.value !== sourceDepartmentSnapshot.value);
const departmentValidationMessage = computed(() =>
  validateDepartmentConfig(
    {
      ...props.config,
      departments: mergeDepartmentChildIdsFromSource(
        cloneDepartmentList(departmentDrafts.value),
        props.config.departments || [],
      ),
    },
    props.apiConfigs,
    (key, params) => t(key, params ?? {}),
  ),
);

const selectedDepartmentPermissionControl = computed(() => selectedDepartment.value?.permissionControl ?? null);
const permissionControlEnabled = computed(() => !!selectedDepartmentPermissionControl.value?.enabled);
const permissionCardTone = computed(() =>
  selectedDepartmentPermissionControl.value?.mode === "whitelist"
    ? {
        card: "border-success/40 bg-success/12 text-base-content shadow-sm",
        box: "border-success bg-success text-success-content",
        icon: Check,
      }
    : {
        card: "border-error/40 bg-error/12 text-base-content shadow-sm",
        box: "border-error bg-error text-error-content",
        icon: X,
      },
);
const permissionListDisabled = computed(() =>
  !permissionControlEnabled.value,
);
const permissionExecAllowed = computed(() => {
  const control = selectedDepartmentPermissionControl.value;
  if (!control?.enabled) return false;
  const execSelected = (control.builtinToolNames || []).includes("exec");
  return control.mode === "whitelist" ? execSelected : !execSelected;
});
const skillPermissionRequiresExec = computed(() =>
  permissionControlEnabled.value && !permissionExecAllowed.value,
);
const skillPermissionListDisabled = computed(() =>
  permissionListDisabled.value || skillPermissionRequiresExec.value,
);
const visiblePermissionBuiltinTools = computed(() => {
  return permissionCatalog.value.builtinTools.filter((item) => !builtinPermissionNameForceHidden(item.name));
});
const visiblePermissionSkills = computed(() => permissionCatalog.value.skills);

const availableAssigneePersonas = computed(() =>
  sortPersonasForSelect(
    props.personas.filter((persona) => {
      const id = String(persona.id || "").trim();
      return !!id && canServeAsRegularDepartmentPersona(persona);
    }),
  ),
);
const selectedDepartmentAssigneeIds = computed(() =>
  normalizeNameList(selectedDepartment.value?.agentIds || []),
);
const departmentAssigneeLimitReached = computed(() =>
  selectedDepartmentAssigneeIds.value.length >= 1,
);
const remainingAssigneePersonas = computed(() =>
  availableAssigneePersonas.value.filter(
    (persona) => !selectedDepartmentAssigneeIds.value.includes(String(persona.id || "").trim()),
  ),
);

function canServeAsRegularDepartmentPersona(persona: PersonaProfile): boolean {
  const id = String(persona.id || "").trim();
  return id !== "user-persona" && !persona.isBuiltInUser && (id === "deputy-agent" || !persona.isBuiltInSystem);
}

function personaSelectRank(persona: PersonaProfile): number {
  if (persona.isBuiltInUser) return 0;
  if (persona.isBuiltInSystem) return 1;
  return 2;
}

function sortPersonasForSelect(personas: PersonaProfile[]): PersonaProfile[] {
  return personas
    .map((persona, index) => ({ persona, index }))
    .sort((a, b) => personaSelectRank(a.persona) - personaSelectRank(b.persona) || a.index - b.index)
    .map((item) => item.persona);
}

function builtinPermissionNameForceHidden(name: string): boolean {
  const department = selectedDepartment.value;
  if (!department) return false;
  const toolName = String(name || "").trim();
  if (!toolName) return true;
  const isAssistant = department.id === "assistant-department" || !!department.isBuiltInAssistant;
  if (isAssistant) return false;
  return ["reload", "organize_context", "screenshot", "operate", "task"].includes(toolName);
}

function ensureDepartmentPermissionControl(target: DepartmentConfig | null | undefined) {
  if (!target) return null;
  if (!target.permissionControl) {
    target.permissionControl = normalizePermissionControl(null);
  }
  return target.permissionControl;
}

function syncDepartmentDraftsFromSource() {
  const currentSelection = selectedDepartmentId.value;
  departmentDrafts.value = cloneDepartmentList(props.config.departments || []);
  if (departmentDrafts.value.some((item) => item.id === currentSelection)) {
    selectedDepartmentId.value = currentSelection;
    return;
  }
  selectedDepartmentId.value = departmentDrafts.value[0]?.id || "assistant-department";
}

function restoreDepartmentDraftsFromSaved() {
  syncDepartmentDraftsFromSource();
}

function touchSelectedDepartment() {
  // Draft fields are already reactive; timestamps are refreshed on save only.
}

watch(
  () => sortedDepartments.value.map((item) => item.id).join("|"),
  () => {
    if (!sortedDepartments.value.some((item) => item.id === selectedDepartmentId.value)) {
      selectedDepartmentId.value = sortedDepartments.value[0]?.id || "assistant-department";
    }
  },
  { immediate: true },
);

watch(
  () => ({
    selectedDepartmentId: selectedDepartmentId.value,
    remainingPersonaIds: remainingAssigneePersonas.value.map((persona) => persona.id).join("|"),
  }),
  () => {
    if (
      pendingDepartmentAssigneeId.value
      && remainingAssigneePersonas.value.some((persona) => persona.id === pendingDepartmentAssigneeId.value)
    ) {
      return;
    }
    pendingDepartmentAssigneeId.value = remainingAssigneePersonas.value[0]?.id || "";
  },
  { immediate: true },
);

watch(
  () => sourceDepartmentSnapshot.value,
  () => {
    if (departmentDirty.value) return;
    syncDepartmentDraftsFromSource();
  },
);

watch(
  () => sourceDepartmentRelationSnapshot.value,
  () => {
    departmentDrafts.value = mergeDepartmentChildIdsFromSource(
      departmentDrafts.value,
      props.config.departments || [],
    );
  },
);

watch(
  () => ({
    departmentId: selectedDepartment.value?.id || "",
    enabled: permissionControlEnabled.value,
    mode: selectedDepartmentPermissionControl.value?.mode || "blacklist",
    builtinToolNames: (selectedDepartmentPermissionControl.value?.builtinToolNames || []).join("|"),
    blocked: skillPermissionRequiresExec.value,
  }),
  () => {
    const control = selectedDepartmentPermissionControl.value;
    if (!control || !skillPermissionRequiresExec.value || control.skillNames.length <= 0) {
      return;
    }
    updateDepartmentPermissionControl({ skillNames: [] });
  },
);

async function loadPermissionCatalog() {
  permissionCatalogLoading.value = true;
  permissionCatalogError.value = "";
  try {
    const payload = await invokeTauri<DepartmentPermissionCatalog>("list_department_permission_catalog");
    permissionCatalog.value = {
      builtinTools: Array.isArray(payload?.builtinTools)
        ? payload.builtinTools
            .map((item) => ({
              name: String(item?.name || "").trim(),
              description: String(item?.description || "").trim(),
            }))
            .filter((item) => !!item.name)
        : [],
      skills: Array.isArray(payload?.skills)
        ? payload.skills
            .map((item) => ({
              name: String(item?.name || "").trim(),
              description: String(item?.description || "").trim(),
            }))
            .filter((item) => !!item.name)
        : [],
      mcpTools: Array.isArray(payload?.mcpTools)
        ? payload.mcpTools
            .map((item) => ({
              name: String(item?.name || "").trim(),
              description: String(item?.description || "").trim(),
            }))
            .filter((item) => !!item.name)
        : [],
    };
  } catch (error) {
    permissionCatalogError.value = String(error || "");
  } finally {
    permissionCatalogLoading.value = false;
  }
}

function updateDepartmentPermissionControl(patch: Partial<NonNullable<DepartmentConfig["permissionControl"]>>) {
  const target = selectedDepartment.value;
  const control = ensureDepartmentPermissionControl(target);
  console.info("[部门权限] 更新开关", {
    departmentId: target?.id || "",
    patch,
    hasTarget: !!target,
    hasControl: !!control,
    enabledBefore: !!control?.enabled,
    modeBefore: control?.mode || "",
  });
  if (!target || !control) return;
  if ("enabled" in patch) {
    control.enabled = !!patch.enabled;
  }
  if ("mode" in patch) {
    control.mode = patch.mode === "whitelist" ? "whitelist" : "blacklist";
  }
  if ("builtinToolNames" in patch) {
    control.builtinToolNames = normalizeNameList(patch.builtinToolNames);
  }
  if ("skillNames" in patch) {
    control.skillNames = normalizeNameList(patch.skillNames);
  }
  if ("mcpToolNames" in patch) {
    control.mcpToolNames = normalizeNameList(patch.mcpToolNames);
  }
  console.info("[部门权限] 更新完成", {
    departmentId: target.id,
    enabledAfter: !!control.enabled,
    modeAfter: control.mode,
    builtinCount: control.builtinToolNames.length,
    skillCount: control.skillNames.length,
    mcpCount: control.mcpToolNames.length,
  });
  touchSelectedDepartment();
}

function togglePermissionName(
  category: DepartmentPermissionNameCategory,
  name: string,
  checked: boolean,
) {
  const control = selectedDepartmentPermissionControl.value;
  console.info("[部门权限] 切换名单项", {
    departmentId: selectedDepartment.value?.id || "",
    category,
    name,
    checked,
    hasControl: !!control,
    enabled: !!control?.enabled,
    disabled: permissionListDisabled.value,
  });
  if (!control) return;
  const trimmed = String(name || "").trim();
  if (!trimmed) return;
  const next = new Set((control[category] || []).map((value) => String(value || "").trim()).filter(Boolean));
  if (checked) {
    next.add(trimmed);
  } else {
    next.delete(trimmed);
  }
  updateDepartmentPermissionControl({ [category]: Array.from(next) } as Partial<NonNullable<DepartmentConfig["permissionControl"]>>);
}

function toggleBuiltinPermissionName(name: string) {
  if (builtinPermissionNameForceHidden(name)) return;
  togglePermissionName("builtinToolNames", name, !permissionNameChecked("builtinToolNames", name));
}

function handleSkillPermissionToggle(name: string) {
  if (skillPermissionListDisabled.value) return;
  togglePermissionName("skillNames", name, !permissionNameChecked("skillNames", name));
}

function permissionNameChecked(category: DepartmentPermissionNameCategory, name: string) {
  const control = selectedDepartmentPermissionControl.value;
  if (!control) return false;
  return (control[category] || []).includes(name);
}

function truncatePermissionDescription(value: string, maxChars = 48) {
  const text = String(value || "").trim();
  if (!text) return "";
  const chars = Array.from(text);
  if (chars.length <= maxChars) return text;
  return `${chars.slice(0, maxChars).join("")}...`;
}

function nextDepartmentName() {
  const base = t("config.department.newName");
  let index = departmentDrafts.value.filter((item) => !isSystemBuiltInDepartment(item)).length + 1;
  while (true) {
    const name = `${base} ${index}`;
    const exists = departmentDrafts.value.some(
      (item) => String(item.name || "").trim().toLocaleLowerCase() === name.trim().toLocaleLowerCase(),
    );
    if (!exists) return name;
    index += 1;
  }
}

function addDepartment() {
  const now = new Date().toISOString();
  const id = `department-${Date.now()}`;
  const maxOrderIndex = departmentDrafts.value.reduce((max, item) => Math.max(max, Number(item.orderIndex || 0)), 0);
  const defaultChildDepartmentIds = departmentDrafts.value.some((item) => String(item.id || "").trim() === "deputy-department")
    ? ["deputy-department"]
    : [];
  departmentDrafts.value.push({
    id,
    name: nextDepartmentName(),
    summary: "",
    guide: "",
    apiConfigId: "",
    apiConfigIds: [],
    agentIds: [],
    childDepartmentIds: defaultChildDepartmentIds,
    createdAt: now,
    updatedAt: now,
    orderIndex: maxOrderIndex + 1,
    isBuiltInAssistant: false,
    source: "main_config",
    scope: "global",
    permissionControl: normalizePermissionControl(null),
  });
  selectedDepartmentId.value = id;
}

function removeSelectedDepartment() {
  const target = selectedDepartment.value;
  if (!target || isSystemBuiltInDepartment(target)) return;
  const idx = departmentDrafts.value.findIndex((item) => item.id === target.id);
  if (idx >= 0) {
    departmentDrafts.value.splice(idx, 1);
  }
}

function departmentDefaultSeed(department: DepartmentConfig | null | undefined): DepartmentDefaultSeed | null {
  const id = String(department?.id || "").trim();
  if (!id) return null;
  if (id === "assistant-department" || department?.isBuiltInAssistant) {
    return {
      name: "助理部门",
      summary: "当复杂任务难度超出了你部门的职责时，请把任务委托给我。",
      guide: "你是助理部门，负责作为主负责人理解用户需求、决定是否需要委派、汇总结果并继续推进主对话。",
    };
  }
  if (id === "deputy-department") {
    return EXPLORER_DEPARTMENT_DEFAULT;
  }
  if (id === "remote-customer-service-department") {
    return {
      ...REMOTE_CUSTOMER_SERVICE_DEPARTMENT_DEFAULT,
      name: t("config.department.defaults.remoteCustomerServiceName"),
    };
  }
  return null;
}

function restoreSelectedDepartment() {
  const target = selectedDepartment.value;
  const defaults = departmentDefaultSeed(target);
  if (!target || !defaults) return;
  target.name = defaults.name;
  target.summary = defaults.summary;
  target.guide = defaults.guide;
  target.permissionControl = normalizePermissionControl(null);
  touchSelectedDepartment();
}

function handleSelectedDepartmentPrimaryAction() {
  if (!selectedDepartment.value) return;
  if (selectedDepartmentIsSystemBuiltIn.value) {
    restoreSelectedDepartment();
    return;
  }
  removeSelectedDepartment();
}

function personaNameById(agentId: string): string {
  const normalizedAgentId = String(agentId || "").trim();
  if (!normalizedAgentId) return "";
  return String(
    props.personas.find((persona) => String(persona.id || "").trim() === normalizedAgentId)?.name || normalizedAgentId,
  ).trim() || normalizedAgentId;
}

function addDepartmentAssignee() {
  const target = selectedDepartment.value;
  if (!target) return;
  if (departmentAssigneeLimitReached.value) return;
  const nextAgentId = String(pendingDepartmentAssigneeId.value || "").trim();
  if (!nextAgentId) return;
  const nextAgentIds = normalizeNameList([...(target.agentIds || []), nextAgentId]);
  if (JSON.stringify(nextAgentIds) === JSON.stringify(normalizeNameList(target.agentIds || []))) return;
  target.agentIds = nextAgentIds;
  pendingDepartmentAssigneeId.value = remainingAssigneePersonas.value
    .map((persona) => String(persona.id || "").trim())
    .find((id) => !!id && id !== nextAgentId) || "";
  touchSelectedDepartment();
}

function removeDepartmentAssigneeAt(index: number) {
  const target = selectedDepartment.value;
  if (!target) return;
  const nextAgentIds = normalizeNameList(target.agentIds || []);
  if (index < 0 || index >= nextAgentIds.length) return;
  nextAgentIds.splice(index, 1);
  target.agentIds = nextAgentIds;
  if (!pendingDepartmentAssigneeId.value) {
    pendingDepartmentAssigneeId.value = remainingAssigneePersonas.value[0]?.id || "";
  }
  touchSelectedDepartment();
}

function moveDepartmentAssignee(index: number, delta: number) {
  const target = selectedDepartment.value;
  if (!target) return;
  const nextAgentIds = normalizeNameList(target.agentIds || []);
  const swapIndex = index + delta;
  if (index < 0 || index >= nextAgentIds.length || swapIndex < 0 || swapIndex >= nextAgentIds.length) return;
  const [item] = nextAgentIds.splice(index, 1);
  nextAgentIds.splice(swapIndex, 0, item);
  target.agentIds = nextAgentIds;
  touchSelectedDepartment();
}

function currentDepartmentApiConfigIds(target: DepartmentConfig | null | undefined) {
  if (!target) return [];
  const ids = Array.isArray(target.apiConfigIds) && target.apiConfigIds.length > 0
    ? target.apiConfigIds
    : [target.apiConfigId || ""];
  return ids.map((id) => String(id || "").trim()).filter(Boolean);
}

function currentDepartmentApiConfigIdsForEditor(target: DepartmentConfig | null | undefined) {
  const ids = currentDepartmentApiConfigIds(target);
  return ids.length > 0 ? Array.from(new Set(ids)) : [""];
}

function availableDepartmentApiConfigsForIndex(index: number) {
  const currentIds = currentDepartmentApiConfigIds(selectedDepartment.value);
  const currentId = currentIds[index];
  return textDepartmentApiConfigs.value.filter((api) => api.id === currentId || !currentIds.includes(api.id));
}

function updateDepartmentApiConfigAt(index: number, apiId: string) {
  const target = selectedDepartment.value;
  if (!target) return;
  const next = currentDepartmentApiConfigIds(target);
  const trimmedApiId = String(apiId || "").trim();
  if ((next[index] || "") === trimmedApiId) return;
  if (!trimmedApiId) {
    next.splice(index, 1);
  } else {
    next[index] = trimmedApiId;
  }
  target.apiConfigIds = Array.from(new Set(next.filter(Boolean)));
  target.apiConfigId = target.apiConfigIds[0] || "";
  touchSelectedDepartment();
}

function addDepartmentApiConfig() {
  const target = selectedDepartment.value;
  if (!target) return;
  const nextApi = remainingDepartmentApiConfigs.value[0];
  if (!nextApi) return;
  const next = currentDepartmentApiConfigIds(target);
  next.push(nextApi.id);
  target.apiConfigIds = next;
  target.apiConfigId = next[0] || "";
  touchSelectedDepartment();
}

function removeDepartmentApiConfigAt(index: number) {
  const target = selectedDepartment.value;
  if (!target) return;
  const next = currentDepartmentApiConfigIds(target);
  next.splice(index, 1);
  target.apiConfigIds = next;
  target.apiConfigId = target.apiConfigIds[0] || "";
  touchSelectedDepartment();
}

function moveDepartmentApiConfig(index: number, delta: number) {
  const target = selectedDepartment.value;
  if (!target) return;
  const next = currentDepartmentApiConfigIds(target);
  const swapIndex = index + delta;
  if (swapIndex < 0 || swapIndex >= next.length) return;
  const [item] = next.splice(index, 1);
  next.splice(swapIndex, 0, item);
  target.apiConfigIds = next;
  target.apiConfigId = next[0] || "";
  touchSelectedDepartment();
}

function switchSelectedDepartment(nextId: string) {
  const trimmedId = String(nextId || "").trim();
  if (!trimmedId || trimmedId === selectedDepartmentId.value) return;
  if (departmentDirty.value) {
    const currentName = String(selectedDepartment.value?.name || selectedDepartmentId.value || "").trim() || t("config.department.title");
    props.setStatusAction(t("status.departmentUnsavedSwitchHint", { name: currentName }));
  }
  selectedDepartmentId.value = trimmedId;
}

function resolveAssistantDepartmentState(departments: DepartmentConfig[]) {
  const assistant = departments.find((item) => item.id === "assistant-department" || item.isBuiltInAssistant);
  return {
    agentId: String(assistant?.agentIds?.[0] || "").trim(),
    apiConfigId: String(assistant?.apiConfigIds?.[0] || assistant?.apiConfigId || "").trim(),
  };
}

function applyUpdatedAtToChangedDepartments(
  nextDepartments: DepartmentConfig[],
  previousDepartments: DepartmentConfig[],
) {
  const previousById = new Map(
    previousDepartments.map((item) => [item.id, departmentBasicComparableSnapshot(item)] as const),
  );
  const now = new Date().toISOString();
  return nextDepartments.map((item) => {
    const previousSnapshot = previousById.get(item.id);
    const nextSnapshot = departmentBasicComparableSnapshot(item);
    if (previousSnapshot === nextSnapshot) {
      return item;
    }
    return {
      ...item,
      updatedAt: now,
    };
  });
}

async function saveDepartments() {
  if (!selectedDepartment.value || departmentValidationMessage.value) return;

  const previousDepartments = cloneDepartmentList(props.config.departments || []);
  const previousAssistantApiConfigId = String(props.config.assistantDepartmentApiConfigId || "").trim();
  const previousAssistantAgentId = String(props.assistantDepartmentAgentId || "").trim();
  const nextDepartments = applyUpdatedAtToChangedDepartments(
    mergeDepartmentChildIdsFromSource(
      cloneDepartmentList(departmentDrafts.value),
      previousDepartments,
    ),
    previousDepartments,
  );
  const assistantState = resolveAssistantDepartmentState(nextDepartments);

  props.config.departments = nextDepartments;
  props.config.assistantDepartmentApiConfigId = assistantState.apiConfigId;

  if (assistantState.agentId && assistantState.agentId !== previousAssistantAgentId) {
    emit("update:assistantDepartmentAssigneeId", assistantState.agentId);
  }

  const saved = await Promise.resolve(props.saveConfigAction());
  if (!saved) {
    props.config.departments = previousDepartments;
    props.config.assistantDepartmentApiConfigId = previousAssistantApiConfigId;
    if (assistantState.agentId && assistantState.agentId !== previousAssistantAgentId) {
      emit("update:assistantDepartmentAssigneeId", previousAssistantAgentId);
    }
    return;
  }

  syncDepartmentDraftsFromSource();
}

onMounted(() => {
  void loadPermissionCatalog();
});
</script>

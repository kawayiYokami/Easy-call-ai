<template>
  <SettingsStickyLayout>
    <template #header>
      <div class="flex w-full flex-col gap-3">
        <div class="flex items-center justify-between gap-3">
          <div class="flex items-center gap-2">
            <span class="text-sm font-semibold">{{ t("config.department.settings") }}</span>
            <span v-if="selectedDepartmentIsPrivateWorkspace" class="badge badge-soft badge-secondary">{{ t("config.department.privateWorkspaceBadge") }}</span>
          </div>

          <button
            v-if="selectedDepartment && !selectedDepartmentIsSystemBuiltIn"
            class="btn btn-sm btn-error"
            type="button"
            :disabled="savingConfig"
            @click="handleSelectedDepartmentPrimaryAction"
          >
            <Trash2 class="h-4 w-4" />
            {{ t("config.department.remove") }}
          </button>
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
            class="btn btn-sm btn-square btn-ghost"
            type="button"
            :title="t('config.department.add')"
            :disabled="savingConfig"
            @click="addDepartment"
          >
            <Plus class="h-3.5 w-3.5" />
          </button>

          <button
            class="btn btn-sm btn-square btn-ghost"
            type="button"
            :title="t('common.reset')"
            :disabled="!departmentDirty || savingConfig"
            @click="restoreDepartmentDraftsFromSaved"
          >
            <RotateCcw class="h-3.5 w-3.5" />
          </button>

          <button
            class="btn btn-sm btn-square"
            type="button"
            :class="departmentDirty ? 'btn-primary' : 'btn-ghost'"
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
    </template>

    <div v-if="selectedDepartment" class="grid gap-3">
        <div class="overflow-hidden rounded-box border border-base-300 bg-base-100">
          <div v-if="departmentValidationMessage" class="border-b border-warning/30 bg-warning/10 px-4 py-3 text-sm text-warning-content">
            {{ departmentValidationMessage }}
          </div>

          <div class="divide-y divide-base-300">
            <div class="min-w-0 px-4 py-4">
              <div class="mb-2 flex items-center justify-between gap-3">
                <div class="text-sm font-medium">{{ t("config.department.name") }}</div>
                <button
                  v-if="selectedDepartmentIsSystemBuiltIn"
                  class="btn btn-sm btn-ghost"
                  type="button"
                  :disabled="savingConfig"
                  @click="handleSelectedDepartmentPrimaryAction"
                >
                  <RotateCcw class="h-3.5 w-3.5" />
                  {{ t("config.department.restoreInitial") }}
                </button>
              </div>
              <input
                v-model.trim="selectedDepartment.name"
                class="input input-bordered input-sm w-full"
                :placeholder="t('config.department.namePlaceholder')"
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
              <div class="grid gap-2">
                <div v-if="availableAssigneePersonas.length === 0" class="text-sm opacity-60">
                  {{ t("config.department.assigneePlaceholder") }}
                </div>
                <div v-else class="flex max-h-56 flex-wrap gap-y-2 overflow-y-auto">
                  <label
                    v-for="persona in availableAssigneePersonas"
                    :key="persona.id"
                    class="mr-3 flex min-h-6 max-w-full cursor-pointer items-center gap-1.5 last:mr-0"
                  >
                    <input
                      type="checkbox"
                      class="checkbox checkbox-primary checkbox-sm"
                      :checked="selectedDepartmentAssigneeIds.includes(persona.id)"
                      :disabled="savingConfig"
                      @change="toggleDepartmentAssignee(persona.id)"
                    />
                    <span class="min-w-0 truncate text-sm">{{ persona.name || persona.id }}</span>
                  </label>
                </div>
              </div>
            </div>

            <div class="px-4 py-4">
              <div class="mb-2 text-sm font-medium">{{ t("config.department.model") }}</div>
              <div class="grid min-w-0 gap-3">
                <div
                  v-for="(apiId, idx) in selectedDepartmentVisibleApiConfigIds"
                  :key="`${selectedDepartment.id}-api-${idx}`"
                  class="flex items-center gap-2"
                >
                  <ApiConfigPicker
                    class="flex-1"
                    :model-value="apiId"
                    :api-configs="availableDepartmentApiConfigsForIndex(idx)"
                    :extra-options="availableDepartmentRoleOptionsForIndex(idx).map((role) => ({ id: role.id, label: role.name }))"
                    @update:model-value="updateDepartmentApiConfigAt(idx, $event)"
                  />

                  <div class="join">
                    <button
                      v-if="selectedDepartmentModelFailureFallbackEnabled"
                      class="btn btn-sm btn-square join-item opacity-60 hover:opacity-100"
                      type="button"
                      :disabled="idx <= 0"
                      :title="t('config.department.moveUp')"
                      @click="moveDepartmentApiConfig(idx, -1)"
                    >
                      ↑
                    </button>
                    <button
                      v-if="selectedDepartmentModelFailureFallbackEnabled"
                      class="btn btn-sm btn-square join-item opacity-60 hover:opacity-100"
                      type="button"
                      :disabled="idx >= selectedDepartmentApiConfigIds.length - 1"
                      :title="t('config.department.moveDown')"
                      @click="moveDepartmentApiConfig(idx, 1)"
                    >
                      ↓
                    </button>
                    <button
                      v-if="selectedDepartmentModelFailureFallbackEnabled"
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
                  v-if="selectedDepartmentModelFailureFallbackEnabled"
                  class="btn btn-sm"
                  type="button"
                  :disabled="remainingDepartmentRoleOptions.length <= 0 && remainingDepartmentApiConfigs.length <= 0"
                  @click="addDepartmentApiConfig"
                >
                  {{ t("config.department.addModel") }}
                </button>
              </div>

              <label
                v-if="selectedDepartmentCanEnableModelFailureFallback"
                class="mt-3 flex items-start gap-2 rounded-box border border-base-300/60 px-2.5 py-2 text-base-content/70"
              >
                <input
                  class="checkbox checkbox-xs mt-0.5 opacity-70"
                  type="checkbox"
                  :checked="selectedDepartmentModelFailureFallbackEnabled"
                  @change="updateDepartmentModelFailureFallback(($event.target as HTMLInputElement).checked)"
                />
                <span class="min-w-0">
                  <span class="block text-xs font-normal">{{ t("config.department.modelFailureFallback") }}</span>
                  <span class="block text-xs opacity-55">
                    {{ selectedDepartmentModelFailureFallbackEnabled
                      ? t("config.department.modelFallbackHint")
                      : t("config.department.singleModelHint") }}
                  </span>
                </span>
              </label>
              <div
                v-if="selectedDepartmentModelFailureFallbackEnabled"
                class="mt-2 rounded-box border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning"
              >
                {{ t("config.department.modelSwitchCostWarning") }}
              </div>
              <div class="mt-1 text-xs opacity-40">{{ t("config.department.allowedModelsNote") }}</div>
            </div>

            <div class="px-4 py-4">
              <div class="mb-2 text-sm font-medium">{{ t("config.department.summary") }}</div>
              <textarea
                v-model="selectedDepartment.summary"
                class="textarea textarea-bordered textarea-sm min-h-20 w-full"
                :placeholder="t('config.department.summaryPlaceholder')"
                @input="touchSelectedDepartment"
              />
            </div>

            <div class="px-4 py-4">
              <div class="mb-2 text-sm font-medium">{{ t("config.department.guide") }}</div>
              <textarea
                v-model="selectedDepartment.guide"
                class="textarea textarea-bordered textarea-sm min-h-28 w-full"
                :placeholder="t('config.department.guidePlaceholder')"
                @input="touchSelectedDepartment"
              />
              <div class="mt-2 text-xs opacity-40">{{ t("config.department.guideHint") }}</div>
            </div>

            <div class="px-4 py-4">
              <div class="mb-3 flex items-center justify-between gap-3">
                <div>
                  <div class="text-sm font-medium">{{ t("config.department.permissionTitle") }}</div>
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
import { MODEL_ROLE_EXPERT_API_CONFIG_ID, MODEL_ROLE_QUICK_API_CONFIG_ID } from "../../utils/model-role-options";
import SettingsStickyLayout from "../../components/SettingsStickyLayout.vue";
import ApiConfigPicker from "../../components/ApiConfigPicker.vue";

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
  "leader-department",
  "deputy-department",
  "reviewer-department",
  "saddler-department",
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
  "minimax",
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
    modelFailureFallbackEnabled: !!department.modelFailureFallbackEnabled,
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

function removedDepartmentIdsFromSource(
  drafts: DepartmentConfig[] | null | undefined,
  source: DepartmentConfig[] | null | undefined,
) {
  const draftIds = new Set((drafts || []).map((item) => String(item.id || "").trim()).filter(Boolean));
  return (source || [])
    .map((item) => String(item.id || "").trim())
    .filter((id) => !!id && !draftIds.has(id));
}

const departmentDrafts = ref<DepartmentConfig[]>(cloneDepartmentList(props.config.departments || []));
const permissionCatalog = ref<DepartmentPermissionCatalog>({
  builtinTools: [],
  skills: [],
  mcpTools: [],
});
const permissionCatalogLoading = ref(false);
const permissionCatalogError = ref("");

const sortedDepartments = computed(() =>
  [...departmentDrafts.value].sort((a, b) => {
    const rank = (id: string) =>
      id === "assistant-department" ? 0 : id === "leader-department" ? 1 : id === "deputy-department" ? 2 : id === "reviewer-department" ? 3 : id === "saddler-department" ? 4 : id === "remote-customer-service-department" ? 5 : 6;
    const aRank = rank(String(a.id || "").trim());
    const bRank = rank(String(b.id || "").trim());
    return aRank - bRank || a.orderIndex - b.orderIndex;
  }),
);

const selectedDepartment = computed(
  () => departmentDrafts.value.find((item) => item.id === selectedDepartmentId.value) ?? sortedDepartments.value[0] ?? null,
);
const selectedDepartmentIsSystemBuiltIn = computed(() => isSystemBuiltInDepartment(selectedDepartment.value));
const selectedDepartmentIsPrivateWorkspace = computed(() => selectedDepartment.value?.source === "private_workspace");
const textDepartmentApiConfigs = computed(() =>
  props.apiConfigs.filter((api) => !!api.enableText && isTextRequestFormat(api.requestFormat)),
);
const departmentRoleApiConfigOptions = computed(() => [
  { id: MODEL_ROLE_EXPERT_API_CONFIG_ID, name: roleModelDisplayName(MODEL_ROLE_EXPERT_API_CONFIG_ID) },
  { id: MODEL_ROLE_QUICK_API_CONFIG_ID, name: roleModelDisplayName(MODEL_ROLE_QUICK_API_CONFIG_ID) },
]);
const selectedDepartmentApiConfigIds = computed(() =>
  currentDepartmentApiConfigIdsForEditor(selectedDepartment.value),
);
const selectedDepartmentCanEnableModelFailureFallback = computed(() =>
  !selectedDepartmentIsPrivateWorkspace.value,
);
const selectedDepartmentModelFailureFallbackEnabled = computed(() =>
  selectedDepartmentCanEnableModelFailureFallback.value && !!selectedDepartment.value?.modelFailureFallbackEnabled,
);
const selectedDepartmentVisibleApiConfigIds = computed(() =>
  selectedDepartmentModelFailureFallbackEnabled.value
    ? selectedDepartmentApiConfigIds.value
    : selectedDepartmentApiConfigIds.value.slice(0, 1),
);
const remainingDepartmentApiConfigs = computed(() => {
  const selectedIds = new Set(selectedDepartmentApiConfigIds.value);
  return textDepartmentApiConfigs.value.filter((api) => !selectedIds.has(api.id));
});
const remainingDepartmentRoleOptions = computed(() => {
  const selectedIds = new Set(selectedDepartmentApiConfigIds.value);
  return departmentRoleApiConfigOptions.value.filter((role) => !selectedIds.has(role.id));
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
        removedDepartmentIdsFromSource(departmentDrafts.value, props.config.departments || []),
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
  return permissionCatalog.value.builtinTools;
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

async function addDepartment() {
  if (props.savingConfig) return;
  const previousDepartments = cloneDepartmentList(props.config.departments || []);
  const previousSelectedDepartmentId = selectedDepartmentId.value;
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
    apiConfigId: MODEL_ROLE_EXPERT_API_CONFIG_ID,
    apiConfigIds: [MODEL_ROLE_EXPERT_API_CONFIG_ID],
    modelFailureFallbackEnabled: false,
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
  props.config.departments = cloneDepartmentList(departmentDrafts.value);
  const saved = await Promise.resolve(props.saveConfigAction());
  if (!saved) {
    props.config.departments = previousDepartments;
    syncDepartmentDraftsFromSource();
    selectedDepartmentId.value = previousSelectedDepartmentId;
    return;
  }
  syncDepartmentDraftsFromSource();
  selectedDepartmentId.value = id;
}

function removeSelectedDepartment() {
  const target = selectedDepartment.value;
  if (!target || isSystemBuiltInDepartment(target)) return;
  const targetId = String(target.id || "").trim();
  if (!targetId) return;
  const nextSelectedId =
    departmentDrafts.value.find((item) => item.id !== targetId)?.id
    || "";

  departmentDrafts.value = departmentDrafts.value
    .filter((item) => item.id !== targetId)
    .map((item) => {
      const nextChildDepartmentIds = normalizeDepartmentChildIds(item.childDepartmentIds, item.id)
        .filter((childId) => childId !== targetId);
      if (JSON.stringify(nextChildDepartmentIds) === JSON.stringify(normalizeDepartmentChildIds(item.childDepartmentIds, item.id))) {
        return item;
      }
      return {
        ...item,
        childDepartmentIds: nextChildDepartmentIds,
        updatedAt: new Date().toISOString(),
      };
    });
  selectedDepartmentId.value = nextSelectedId;
}

async function restoreSelectedDepartment() {
  const target = selectedDepartment.value;
  if (!target) return;
  try {
    const defaults = await invokeTauri<DepartmentConfig>("get_department_default_draft", {
      departmentId: target.id,
    });
    if (String(selectedDepartment.value?.id || "").trim() !== target.id) return;
    target.name = String(defaults.name || "");
    target.summary = String(defaults.summary || "");
    target.guide = String(defaults.guide || "");
    target.apiConfigId = String(defaults.apiConfigId || "");
    target.apiConfigIds = normalizeNameList(defaults.apiConfigIds);
    target.modelFailureFallbackEnabled = !!defaults.modelFailureFallbackEnabled;
    target.agentIds = normalizeNameList(defaults.agentIds);
    target.permissionControl = normalizePermissionControl(defaults.permissionControl);
    touchSelectedDepartment();
  } catch (error) {
    props.setStatusAction(String(error || ""));
  }
}

function handleSelectedDepartmentPrimaryAction() {
  if (!selectedDepartment.value) return;
  if (selectedDepartmentIsSystemBuiltIn.value) {
    void restoreSelectedDepartment();
    return;
  }
  removeSelectedDepartment();
}

function updateDepartmentAssignees(agentIds: string[]) {
  const target = selectedDepartment.value;
  if (!target) return;
  const allowedIds = new Set(availableAssigneePersonas.value.map((persona) => String(persona.id || "").trim()).filter(Boolean));
  const nextAgentIds = normalizeNameList(agentIds).filter((agentId) => allowedIds.has(agentId));
  if (JSON.stringify(nextAgentIds) === JSON.stringify(normalizeNameList(target.agentIds || []))) return;
  target.agentIds = nextAgentIds;
  touchSelectedDepartment();
}

function toggleDepartmentAssignee(agentId: string) {
  const normalizedAgentId = String(agentId || "").trim();
  if (!normalizedAgentId) return;
  const current = selectedDepartmentAssigneeIds.value;
  updateDepartmentAssignees(
    current.includes(normalizedAgentId)
      ? current.filter((item) => item !== normalizedAgentId)
      : [...current, normalizedAgentId],
  );
}

function currentDepartmentApiConfigIds(target: DepartmentConfig | null | undefined) {
  if (!target) return [];
  const ids = Array.isArray(target.apiConfigIds) && target.apiConfigIds.length > 0
    ? target.apiConfigIds
    : [target.apiConfigId || ""];
  return ids.map((id) => String(id || "").trim()).filter(Boolean);
}

function departmentCanEnableModelFailureFallback(target: DepartmentConfig | null | undefined) {
  return String(target?.source || "").trim() !== "private_workspace";
}

function apiConfigName(apiConfigId: string): string {
  const id = String(apiConfigId || "").trim();
  if (!id) return "";
  const apiConfig = textDepartmentApiConfigs.value.find((api) => String(api.id || "").trim() === id);
  return String(apiConfig?.name || "").trim();
}

function roleModelDisplayName(roleId: string): string {
  const roleLabel = roleId === MODEL_ROLE_QUICK_API_CONFIG_ID
    ? t("config.modelRoles.quick")
    : t("config.modelRoles.expert");
  const concreteId = roleId === MODEL_ROLE_QUICK_API_CONFIG_ID
    ? props.config.toolReviewApiConfigId
    : props.config.assistantDepartmentApiConfigId;
  const concreteName = apiConfigName(String(concreteId || "").trim());
  return concreteName ? `${roleLabel}（${concreteName}）` : roleLabel;
}

function currentDepartmentApiConfigIdsForEditor(target: DepartmentConfig | null | undefined) {
  const ids = currentDepartmentApiConfigIds(target);
  return ids.length > 0 ? Array.from(new Set(ids)) : [MODEL_ROLE_EXPERT_API_CONFIG_ID];
}

function departmentModelIdsForSave(target: DepartmentConfig): string[] {
  const ids = currentDepartmentApiConfigIdsForEditor(target);
  return departmentCanEnableModelFailureFallback(target) && target.modelFailureFallbackEnabled ? ids : ids.slice(0, 1);
}

function availableDepartmentApiConfigsForIndex(index: number) {
  const currentIds = currentDepartmentApiConfigIds(selectedDepartment.value);
  const currentId = currentIds[index];
  return textDepartmentApiConfigs.value.filter((api) => api.id === currentId || !currentIds.includes(api.id));
}

function availableDepartmentRoleOptionsForIndex(index: number) {
  const currentIds = currentDepartmentApiConfigIds(selectedDepartment.value);
  const currentId = currentIds[index];
  return departmentRoleApiConfigOptions.value.filter((role) => role.id === currentId || !currentIds.includes(role.id));
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
  if (target.apiConfigIds.length === 0) {
    target.apiConfigIds = [MODEL_ROLE_EXPERT_API_CONFIG_ID];
  }
  target.apiConfigId = target.apiConfigIds[0] || "";
  touchSelectedDepartment();
}

function updateDepartmentModelFailureFallback(enabled: boolean) {
  const target = selectedDepartment.value;
  if (!target || target.modelFailureFallbackEnabled === enabled) return;
  if (!departmentCanEnableModelFailureFallback(target)) return;
  target.modelFailureFallbackEnabled = enabled;
  if (currentDepartmentApiConfigIds(target).length === 0) {
    target.apiConfigIds = [MODEL_ROLE_EXPERT_API_CONFIG_ID];
    target.apiConfigId = MODEL_ROLE_EXPERT_API_CONFIG_ID;
  }
  touchSelectedDepartment();
}

function addDepartmentApiConfig() {
  const target = selectedDepartment.value;
  if (!target) return;
  const nextRole = remainingDepartmentRoleOptions.value[0];
  const nextApi = remainingDepartmentApiConfigs.value[0];
  if (!nextRole && !nextApi) return;
  const next = currentDepartmentApiConfigIds(target);
  next.push(nextRole?.id || nextApi?.id || MODEL_ROLE_EXPERT_API_CONFIG_ID);
  target.apiConfigIds = next;
  target.apiConfigId = next[0] || "";
  touchSelectedDepartment();
}

function removeDepartmentApiConfigAt(index: number) {
  const target = selectedDepartment.value;
  if (!target) return;
  const next = currentDepartmentApiConfigIds(target);
  next.splice(index, 1);
  target.apiConfigIds = next.length > 0 ? next : [MODEL_ROLE_EXPERT_API_CONFIG_ID];
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

function resolveAssistantDepartmentAgentId(departments: DepartmentConfig[]) {
  const assistant = departments.find((item) => item.id === "assistant-department" || item.isBuiltInAssistant);
  return String(assistant?.agentIds?.[0] || "").trim();
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

function prepareDepartmentsForSave(departments: DepartmentConfig[]) {
  return departments.map((department) => {
    const departmentId = String(department.id || "").trim();
    const apiConfigIds = departmentModelIdsForSave(department);
    return {
      ...department,
      apiConfigIds,
      apiConfigId: apiConfigIds[0] || "",
      modelFailureFallbackEnabled: departmentCanEnableModelFailureFallback(department) && department.modelFailureFallbackEnabled,
      childDepartmentIds: normalizeDepartmentChildIds(department.childDepartmentIds, departmentId),
      permissionControl: normalizePermissionControl(department.permissionControl),
    };
  });
}

async function saveDepartments() {
  if (!selectedDepartment.value || departmentValidationMessage.value) return;

  const previousDepartments = cloneDepartmentList(props.config.departments || []);
  const previousAssistantAgentId = String(props.assistantDepartmentAgentId || "").trim();
  const nextDrafts = cloneDepartmentList(departmentDrafts.value);
  const nextDepartments = applyUpdatedAtToChangedDepartments(
    mergeDepartmentChildIdsFromSource(
      prepareDepartmentsForSave(nextDrafts),
      previousDepartments,
      removedDepartmentIdsFromSource(nextDrafts, previousDepartments),
    ),
    previousDepartments,
  );
  const assistantAgentId = resolveAssistantDepartmentAgentId(nextDepartments);

  props.config.departments = nextDepartments;

  if (assistantAgentId && assistantAgentId !== previousAssistantAgentId) {
    emit("update:assistantDepartmentAssigneeId", assistantAgentId);
  }

  const saved = await Promise.resolve(props.saveConfigAction());
  if (!saved) {
    props.config.departments = previousDepartments;
    if (assistantAgentId && assistantAgentId !== previousAssistantAgentId) {
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

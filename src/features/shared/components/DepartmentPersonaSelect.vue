<template>
  <select
    class="select select-bordered w-full"
    :value="selectedValue"
    :disabled="disabled || normalizedOptions.length === 0"
    @change="handleChange"
  >
    <option v-if="placeholder" value="">{{ placeholder }}</option>
    <option
      v-for="option in normalizedOptions"
      :key="option.id"
      :value="option.id"
    >
      {{ optionLabel(option) }}
    </option>
  </select>
</template>

<script setup lang="ts">
import { computed, watch } from "vue";
import type { ApiConfigItem, DepartmentConfig, PersonaProfile } from "../../../types/app";
import {
  buildDepartmentPersonaOptions,
  departmentPersonaOptionId,
  type DepartmentPersonaOption,
} from "../department-persona-options";

const props = withDefaults(defineProps<{
  departmentId?: string;
  agentId?: string;
  departments?: DepartmentConfig[];
  personas?: PersonaProfile[];
  apiConfigs?: ApiConfigItem[];
  assistantDepartmentApiConfigId?: string;
  toolReviewApiConfigId?: string | null;
  options?: DepartmentPersonaOption[];
  placeholder?: string;
  disabled?: boolean;
  showModel?: boolean;
  autoSelectFirst?: boolean;
  preserveCurrent?: boolean;
}>(), {
  departmentId: "",
  agentId: "",
  assistantDepartmentApiConfigId: "",
  placeholder: "",
  disabled: false,
  showModel: true,
  autoSelectFirst: false,
  preserveCurrent: true,
});

const emit = defineEmits<{
  "update:departmentId": [value: string];
  "update:agentId": [value: string];
  change: [value: { departmentId: string; agentId: string; option: DepartmentPersonaOption | null }];
}>();

const baseOptions = computed(() => (
  Array.isArray(props.options) && props.options.length > 0
    ? props.options
    : buildDepartmentPersonaOptions({
      departments: props.departments || [],
      personas: props.personas || [],
      apiConfigs: props.apiConfigs || [],
      assistantDepartmentApiConfigId: props.assistantDepartmentApiConfigId,
      toolReviewApiConfigId: props.toolReviewApiConfigId,
    })
));

function findDepartmentName(departmentId: string): string {
  const option = baseOptions.value.find((item) => item.departmentId === departmentId);
  if (option?.departmentName) return option.departmentName;
  const department = (props.departments || []).find((item) => String(item.id || "").trim() === departmentId);
  return String(department?.name || "").trim() || departmentId;
}

function findAgentName(agentId: string): string {
  const option = baseOptions.value.find((item) => item.agentId === agentId);
  if (option?.agentName) return option.agentName;
  const persona = (props.personas || []).find((item) => String(item.id || "").trim() === agentId);
  return String(persona?.name || "").trim() || agentId;
}

function buildCurrentMissingOption(departmentId: string, agentId: string): DepartmentPersonaOption {
  const departmentName = findDepartmentName(departmentId);
  const agentName = findAgentName(agentId);
  return {
    id: departmentPersonaOptionId(departmentId, agentId),
    departmentId,
    agentId,
    departmentName,
    agentName,
    label: `${departmentName} / ${agentName} (已移除)`,
    name: departmentName,
    ownerAgentId: agentId,
    ownerName: agentName,
    childDepartmentIds: [],
    unavailable: true,
  };
}

const normalizedOptions = computed(() => {
  const options = [...baseOptions.value];
  if (!props.preserveCurrent) return options;
  const departmentId = String(props.departmentId || "").trim();
  const agentId = String(props.agentId || "").trim();
  if (!departmentId || !agentId) return options;
  const key = departmentPersonaOptionId(departmentId, agentId);
  if (options.some((option) => option.id === key)) return options;
  return [buildCurrentMissingOption(departmentId, agentId), ...options];
});

const selectedValue = computed(() => {
  const departmentId = String(props.departmentId || "").trim();
  const agentId = String(props.agentId || "").trim();
  if (!departmentId || !agentId) return "";
  const key = departmentPersonaOptionId(departmentId, agentId);
  if (normalizedOptions.value.some((option) => option.id === key)) return key;
  return "";
});

function optionLabel(option: DepartmentPersonaOption): string {
  const label = String(option.label || "").trim() || `${option.departmentName} / ${option.agentName}`;
  const modelName = String(option.modelName || "").trim();
  return props.showModel && modelName ? `${label} · ${modelName}` : label;
}

function emitSelection(option: DepartmentPersonaOption | null) {
  const departmentId = String(option?.departmentId || "").trim();
  const agentId = String(option?.agentId || "").trim();
  emit("update:departmentId", departmentId);
  emit("update:agentId", agentId);
  emit("change", { departmentId, agentId, option });
}

function handleChange(event: Event) {
  const value = String((event.target as HTMLSelectElement | null)?.value || "").trim();
  emitSelection(normalizedOptions.value.find((option) => option.id === value) || null);
}

watch(
  () => [props.departmentId, props.agentId, normalizedOptions.value.map((option) => option.id).join("|")] as const,
  () => {
    if (!props.autoSelectFirst) return;
    if (selectedValue.value || normalizedOptions.value.length === 0) return;
    emitSelection(normalizedOptions.value[0] || null);
  },
  { immediate: true },
);
</script>

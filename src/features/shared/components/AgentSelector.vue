<template>
  <DepartmentPersonaSelect
    :department-id="departmentId"
    :agent-id="agentId"
    :api-config-id="apiConfigId"
    :options="options"
    :persona-avatar-url-map="personaAvatarUrlMap"
    :api-configs="apiConfigs"
    :auto-select-first="autoSelectFirst"
    @update:department-id="emit('update:departmentId', $event)"
    @update:agent-id="emit('update:agentId', $event)"
    @update:api-config-id="emit('update:apiConfigId', $event)"
  />
</template>

<script setup lang="ts">
import type { ApiConfigItem } from "../../../types/app";
import DepartmentPersonaSelect from "./DepartmentPersonaSelect.vue";
import type { DepartmentPersonaOption } from "../department-persona-options";

defineProps<{
  departmentId?: string;
  agentId?: string;
  apiConfigId?: string;
  options?: DepartmentPersonaOption[];
  personaAvatarUrlMap?: Record<string, string>;
  apiConfigs?: ApiConfigItem[];
  autoSelectFirst?: boolean;
}>();

const emit = defineEmits<{
  "update:departmentId": [value: string];
  "update:agentId": [value: string];
  "update:apiConfigId": [value: string];
}>();
</script>

<template>
  <div class="flex flex-col gap-2">
    <div class="flex h-60 flex-col">
      <ChatConversationFloatingScroll class="flex-1 min-h-0">
        <div class="flex flex-wrap gap-x-2 gap-y-4 p-2">
        <div
          v-for="group in departmentGroups"
          :key="group.departmentId"
          class="relative flex flex-col gap-1 rounded-xl border-t border-x border-base-300 p-2 transition-colors"
          :class="selectedDepartmentId === group.departmentId
            ? 'bg-gradient-to-b from-base-100 to-primary/20 ring-1 ring-primary/30'
            : 'bg-base-100'"
        >
          <div
            class="absolute -top-2.5 left-2 bg-base-100 px-1 text-xs font-semibold text-base-content/70"
          >
            {{ group.departmentName }}
          </div>
          <div class="invisible h-0 text-xs font-semibold px-1">{{ group.departmentName }}</div>
          <div class="flex flex-wrap justify-center gap-1">
            <button
              v-for="agent in group.agents"
              :key="agent.agentId"
              type="button"
              class="flex flex-col items-center gap-0.5 rounded-md p-0.5"
              :class="(selectedDepartmentId === agent.departmentId && selectedAgentId !== agent.agentId)
                ? 'opacity-80'
                : ''"
              @click="selectAgent(agent)"
            >
              <div class="avatar">
                <div
                  class="h-10 w-10 rounded-full transition-shadow"
                  :class="selectedDepartmentId === agent.departmentId && selectedAgentId === agent.agentId
                    ? 'ring-2 ring-primary ring-offset-1 ring-offset-base-100'
                    : ''"
                >
                  <img
                    v-if="resolveAvatarUrl(agent.agentId)"
                    :src="resolveAvatarUrl(agent.agentId)"
                    :alt="agent.agentName"
                    class="h-10 w-10 rounded-full object-cover"
                  />
                  <div
                    v-else
                    class="flex h-10 w-10 items-center justify-center rounded-full bg-primary text-sm font-semibold text-primary-content"
                  >
                    {{ agentInitials(agent.agentName) }}
                  </div>
                </div>
              </div>
              <span class="max-w-[4.5rem] truncate text-center text-xs leading-tight">{{ agent.agentName }}</span>
            </button>
          </div>
        </div>
      </div>
    </ChatConversationFloatingScroll>
    </div>
    <div v-if="showModelSelector" class="min-w-0 border-t border-base-300 pt-2">
      <select
        :value="selectedApiConfigId"
        class="select select-bordered w-full max-w-full truncate"
        @change="handleModelChange"
      >
        <option
          v-for="config in textApiConfigs"
          :key="config.id"
          :value="config.id"
          class="truncate"
        >
          {{ config.name }}
        </option>
      </select>
    </div>
    <div v-else-if="currentModelName" class="border-t border-base-300 pt-2 text-xs text-base-content/70">
      {{ currentModelName }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ChatConversationFloatingScroll from "../../chat/components/ChatConversationFloatingScroll.vue";
import type { ApiConfigItem } from "../../../types/app";
import type { DepartmentPersonaOption } from "../department-persona-options";

const props = withDefaults(defineProps<{
  departmentId?: string;
  agentId?: string;
  apiConfigId?: string;
  options?: DepartmentPersonaOption[];
  personaAvatarUrlMap?: Record<string, string>;
  apiConfigs?: ApiConfigItem[];
  autoSelectFirst?: boolean;
}>(), {
  departmentId: "",
  agentId: "",
  apiConfigId: "",
  autoSelectFirst: false,
});

const emit = defineEmits<{
  "update:departmentId": [value: string];
  "update:agentId": [value: string];
  "update:apiConfigId": [value: string];
}>();

const { t } = useI18n();

const previousDepartmentId = ref("");

const selectedAgentId = computed(() => String(props.agentId || "").trim());
const selectedDepartmentId = computed(() => String(props.departmentId || "").trim());
const selectedApiConfigId = computed(() => String(props.apiConfigId || "").trim());

const textApiConfigs = computed(() =>
  (props.apiConfigs || []).filter((item) => !!item.enableText),
);

const showModelSelector = computed(() => textApiConfigs.value.length > 0);

const currentModelName = computed(() => {
  const option = props.options?.find(
    (item) => item.departmentId === selectedDepartmentId.value && item.agentId === selectedAgentId.value,
  );
  if (option?.providerName && option?.modelName) {
    return `${option.providerName} / ${option.modelName}`;
  }
  return option?.modelName || option?.providerName || "";
});

type AgentGroup = {
  departmentId: string;
  departmentName: string;
  apiConfigId: string;
  agents: DepartmentPersonaOption[];
};

const departmentGroups = computed<AgentGroup[]>(() => {
  const options = Array.isArray(props.options) ? props.options : [];
  const groups = new Map<string, AgentGroup>();
  for (const option of options) {
    const did = String(option.departmentId || "").trim();
    if (!did) continue;
    const existing = groups.get(did);
    if (existing) {
      existing.agents.push(option);
    } else {
      groups.set(did, {
        departmentId: did,
        departmentName: option.departmentName || did,
        apiConfigId: String(option.apiConfigId || "").trim(),
        agents: [option],
      });
    }
  }
  return Array.from(groups.values());
});

function resolveAvatarUrl(agentId: string): string {
  return props.personaAvatarUrlMap?.[agentId] || "";
}

function agentInitials(name: string): string {
  const text = String(name || "").trim();
  if (!text) return "?";
  // Prefer first 2 characters if they are ASCII letters; otherwise first char
  const firstTwo = text.slice(0, 2);
  if (/^[A-Za-z]{2}/.test(firstTwo)) {
    return firstTwo.toUpperCase();
  }
  return text.charAt(0).toUpperCase();
}

function selectAgent(option: DepartmentPersonaOption) {
  const did = String(option.departmentId || "").trim();
  const aid = String(option.agentId || "").trim();
  if (!did || !aid) return;

  const departmentChanged = did !== selectedDepartmentId.value;

  emit("update:departmentId", did);
  emit("update:agentId", aid);

  if (departmentChanged) {
    const newConfigId = String(option.apiConfigId || "").trim();
    if (newConfigId) {
      emit("update:apiConfigId", newConfigId);
    }
  }

  previousDepartmentId.value = did;
}

function handleModelChange(event: Event) {
  const value = String((event.target as HTMLSelectElement | null)?.value || "").trim();
  emit("update:apiConfigId", value);
}

watch(
  () => [props.departmentId, props.agentId, props.options?.map((o) => o.id).join("|")] as const,
  () => {
    if (!props.autoSelectFirst) return;
    const hasSelection = selectedDepartmentId.value && selectedAgentId.value;
    if (hasSelection) return;
    const firstGroup = departmentGroups.value[0];
    if (!firstGroup) return;
    const firstAgent = firstGroup.agents[0];
    if (!firstAgent) return;
    emit("update:departmentId", firstAgent.departmentId);
    emit("update:agentId", firstAgent.agentId);
    const configId = String(firstAgent.apiConfigId || "").trim();
    if (configId) {
      emit("update:apiConfigId", configId);
    }
  },
  { immediate: true },
);
</script>

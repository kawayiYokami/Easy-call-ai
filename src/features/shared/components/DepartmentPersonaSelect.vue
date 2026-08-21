<template>
  <div class="flex flex-col gap-2">
    <div ref="rootRef" class="relative">
      <button
        type="button"
        class="select select-bordered flex w-full items-center justify-between gap-2 pr-3 text-left"
        :disabled="disabled || normalizedOptions.length === 0"
        @click="toggleOpen"
      >
        <div class="flex min-w-0 flex-1 items-center gap-2">
          <div
            v-if="selectedOption"
            class="avatar shrink-0"
          >
            <div class="h-7 w-7 rounded-full">
              <img
                v-if="resolveAvatarUrl(selectedOption.agentId)"
                :src="resolveAvatarUrl(selectedOption.agentId)"
                :alt="selectedOption.agentName"
                class="h-7 w-7 rounded-full object-cover"
              />
              <div
                v-else
                class="flex h-7 w-7 items-center justify-center rounded-full bg-primary text-xs font-semibold text-primary-content"
              >
                {{ agentInitials(selectedOption.agentName) }}
              </div>
            </div>
          </div>
          <span class="min-w-0 flex-1 truncate" :class="selectedOption ? '' : 'text-base-content/50'">
            {{ selectedSummaryLabel }}
          </span>
        </div>
        <ChevronDown class="h-4 w-4 shrink-0 opacity-70 transition-transform" :class="dropdownOpen ? 'rotate-180' : ''" />
      </button>

      <div
        v-if="dropdownOpen && !disabled && (normalizedOptions.length > 0 || placeholder)"
        ref="dropdownPanelRef"
        class="absolute z-50 w-full max-w-full overflow-hidden rounded-box border border-base-300 bg-base-100 shadow-xl"
        :class="dropdownDirection === 'up' ? 'bottom-full mb-2' : 'top-full mt-2'"
      >
        <div>
          <button
            v-if="placeholder"
            type="button"
            class="flex w-full items-center rounded-none px-3 py-2 text-left text-sm transition-colors hover:bg-base-200"
            :class="!selectedOption ? 'bg-base-200 font-medium' : ''"
            @click="clearSelection"
          >
            <span class="truncate">{{ placeholder }}</span>
          </button>

          <div class="min-h-0" :style="{ height: `${dropdownBodyHeight}px` }">
            <ChatConversationFloatingScroll class="h-full">
              <div ref="dropdownContentRef" class="flex flex-col gap-4 p-4 sm:flex-row sm:flex-wrap sm:gap-x-2 sm:gap-y-4">
                <div
                  v-for="group in departmentGroups"
                  :key="group.departmentId"
                  class="relative w-full min-w-0 flex flex-col gap-1 rounded-xl border-x border-t border-base-300 bg-base-100 p-2 transition-colors sm:w-auto sm:min-w-[16rem]"
                >
                  <div class="absolute -top-2.5 left-1/2 -translate-x-1/2 whitespace-nowrap bg-base-100 px-2 text-center text-xs font-semibold text-base-content/70">
                    {{ group.departmentName }}
                  </div>
                  <div
                    v-if="group.agents.every((option) => option.personaMissing)"
                    class="w-full px-2 py-4 text-center text-xs text-base-content/60"
                  >
                    {{ t("chat.departmentNoAvailableAgent") }}
                  </div>
                  <div v-else class="grid grid-cols-3 gap-1 sm:flex sm:flex-wrap sm:justify-center">
                    <button
                      v-for="option in group.agents"
                      :key="option.id"
                      type="button"
                      class="flex min-w-0 w-full flex-col items-center gap-1 rounded-lg px-1 py-1 transition-colors hover:bg-base-200 sm:w-20"
                      :class="selectedValue === option.id ? 'bg-primary/10' : ''"
                      @click="selectOption(option)"
                    >
                      <div class="avatar">
                        <div
                          class="h-10 w-10 rounded-full transition-shadow"
                          :class="selectedValue === option.id ? 'ring-2 ring-primary ring-offset-1 ring-offset-base-100' : ''"
                        >
                          <img
                            v-if="resolveAvatarUrl(option.agentId)"
                            :src="resolveAvatarUrl(option.agentId)"
                            :alt="option.agentName"
                            class="h-10 w-10 rounded-full object-cover"
                          />
                          <div
                            v-else
                            class="flex h-10 w-10 items-center justify-center rounded-full bg-primary text-sm font-semibold text-primary-content"
                          >
                            {{ agentInitials(option.agentName) }}
                          </div>
                        </div>
                      </div>
                      <span class="max-w-full truncate text-center text-xs leading-tight">
                        {{ option.agentName }}
                      </span>
                      <span v-if="option.personaMissing" class="text-caption leading-none text-warning">
                        {{ t("chat.personaRemoved") }}
                      </span>
                      <span v-else-if="option.modelMissing" class="text-caption leading-none text-warning">
                        {{ t("chat.personaModelNotConfigured") }}
                      </span>
                      <span v-else-if="option.unavailable" class="text-caption leading-none text-warning">
                        {{ t("chat.personaRemoved") }}
                      </span>
                    </button>
                  </div>
                </div>
              </div>
            </ChatConversationFloatingScroll>
          </div>
        </div>
      </div>
    </div>

    <div v-if="showModelSelector" class="min-w-0 pt-2">
      <ApiConfigPicker
        :model-value="selectedApiConfigId"
        :api-configs="textApiConfigs"
        :disabled="disabled"
        @update:model-value="handleModelSelect"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ChevronDown } from "@lucide/vue";
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import type { ApiConfigItem, DepartmentConfig, PersonaProfile } from "../../../types/app";
import ChatConversationFloatingScroll from "../../chat/components/ChatConversationFloatingScroll.vue";
import ApiConfigPicker from "../../config/components/ApiConfigPicker.vue";
import {
  buildDepartmentPersonaOptions,
  departmentPersonaOptionId,
  type DepartmentPersonaOption,
} from "../department-persona-options";

const props = withDefaults(defineProps<{
  departmentId?: string;
  agentId?: string;
  apiConfigId?: string;
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
  personaAvatarUrlMap?: Record<string, string>;
}>(), {
  departmentId: "",
  agentId: "",
  apiConfigId: "",
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
  "update:apiConfigId": [value: string];
  change: [value: { departmentId: string; agentId: string; option: DepartmentPersonaOption | null }];
}>();

const { t } = useI18n();

type AgentGroup = {
  departmentId: string;
  departmentName: string;
  agents: DepartmentPersonaOption[];
};

const rootRef = ref<HTMLElement | null>(null);
const dropdownPanelRef = ref<HTMLElement | null>(null);
const dropdownContentRef = ref<HTMLElement | null>(null);
const dropdownOpen = ref(false);
const dropdownDirection = ref<"up" | "down">("down");
const dropdownBodyHeight = ref(320);

const DROPDOWN_MARGIN = 16;
const DROPDOWN_GAP = 8;
const DROPDOWN_MIN_HEIGHT = 120;
const DROPDOWN_MAX_HEIGHT = 520;

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
    label: `${departmentName} / ${agentName} (${t("chat.personaRemoved")})`,
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

const selectedDepartmentId = computed(() => String(props.departmentId || "").trim());
const selectedAgentId = computed(() => String(props.agentId || "").trim());
const selectedApiConfigId = computed(() => String(props.apiConfigId || "").trim());

const selectedValue = computed(() => {
  const departmentId = selectedDepartmentId.value;
  const agentId = selectedAgentId.value;
  if (!departmentId || !agentId) return "";
  const key = departmentPersonaOptionId(departmentId, agentId);
  if (normalizedOptions.value.some((option) => option.id === key)) return key;
  return "";
});

const selectedOption = computed(() =>
  normalizedOptions.value.find((option) => option.id === selectedValue.value) || null
);

const textApiConfigs = computed(() =>
  (props.apiConfigs || []).filter((item) => !!item.enableText),
);

const showModelSelector = computed(() => props.showModel && textApiConfigs.value.length > 0);

const departmentGroups = computed<AgentGroup[]>(() => {
  const groups = new Map<string, AgentGroup>();
  for (const option of normalizedOptions.value) {
    const departmentId = String(option.departmentId || "").trim();
    if (!departmentId) continue;
    const existing = groups.get(departmentId);
    if (existing) {
      existing.agents.push(option);
      continue;
    }
    groups.set(departmentId, {
      departmentId,
      departmentName: String(option.departmentName || "").trim() || departmentId,
      agents: [option],
    });
  }
  return Array.from(groups.values());
});

const selectedSummaryLabel = computed(() => {
  if (selectedOption.value) {
    const departmentName = String(selectedOption.value.departmentName || "").trim();
    const agentName = String(selectedOption.value.agentName || "").trim();
    const modelName = String(selectedOption.value.modelName || "").trim();
    return [departmentName, agentName, modelName].filter(Boolean).join(" · ");
  }
  if (props.placeholder) return props.placeholder;
  const firstOption = normalizedOptions.value[0];
  if (!firstOption) return "";
  return [
    String(firstOption.departmentName || "").trim(),
    String(firstOption.agentName || "").trim(),
    String(firstOption.modelName || "").trim(),
  ].filter(Boolean).join(" · ");
});

function emitSelection(option: DepartmentPersonaOption | null) {
  const departmentId = String(option?.departmentId || "").trim();
  const agentId = String(option?.agentId || "").trim();
  emit("update:departmentId", departmentId);
  emit("update:agentId", agentId);
  emit("change", { departmentId, agentId, option });
}

function selectOption(option: DepartmentPersonaOption) {
  emitSelection(option);
  const configId = String(option.apiConfigId || "").trim();
  if (configId) {
    emit("update:apiConfigId", configId);
  }
  dropdownOpen.value = false;
}

function clearSelection() {
  emitSelection(null);
  dropdownOpen.value = false;
}

function handleModelSelect(value: string) {
  emit("update:apiConfigId", String(value || "").trim());
}

function resolveAvatarUrl(agentId: string): string {
  return props.personaAvatarUrlMap?.[agentId] || "";
}

function agentInitials(name: string): string {
  const text = String(name || "").trim();
  if (!text) return "?";
  const firstTwo = text.slice(0, 2);
  if (/^[A-Za-z]{2}/.test(firstTwo)) {
    return firstTwo.toUpperCase();
  }
  return text.charAt(0).toUpperCase();
}

function toggleOpen() {
  if (props.disabled || (normalizedOptions.value.length === 0 && !props.placeholder)) return;
  dropdownOpen.value = !dropdownOpen.value;
}

function clampDropdownBodyHeight(space: number): number {
  return Math.max(
    Math.min(Math.floor(space), DROPDOWN_MAX_HEIGHT),
    Math.min(DROPDOWN_MIN_HEIGHT, Math.max(96, Math.floor(space))),
  );
}

function updateDropdownLayout() {
  if (!dropdownOpen.value) return;
  const root = rootRef.value;
  if (!root) return;
  const rect = root.getBoundingClientRect();
  const viewportHeight = window.innerHeight || document.documentElement.clientHeight || 0;
  const availableAbove = Math.max(0, rect.top - DROPDOWN_MARGIN - DROPDOWN_GAP);
  const availableBelow = Math.max(0, viewportHeight - rect.bottom - DROPDOWN_MARGIN - DROPDOWN_GAP);
  const shouldOpenUp = availableAbove > availableBelow && availableAbove >= DROPDOWN_MIN_HEIGHT;
  const preferredDirection = shouldOpenUp ? "up" : "down";
  const chosenSpace = preferredDirection === "up" ? availableAbove : availableBelow;
  const fallbackSpace = preferredDirection === "up" ? availableBelow : availableAbove;
  const finalSpace = Math.max(chosenSpace, fallbackSpace);
  dropdownDirection.value = preferredDirection;
  const availableHeight = clampDropdownBodyHeight(
    finalSpace >= DROPDOWN_MIN_HEIGHT ? chosenSpace : finalSpace,
  );
  const contentHeight = dropdownContentRef.value?.scrollHeight ?? 0;
  const naturalHeight = contentHeight > 0 ? contentHeight : availableHeight;
  dropdownBodyHeight.value = Math.min(availableHeight, Math.max(96, naturalHeight));
}

function handleDocumentPointerDown(event: PointerEvent) {
  if (!dropdownOpen.value) return;
  const target = event.target as Node | null;
  if (rootRef.value && target && !rootRef.value.contains(target)) {
    dropdownOpen.value = false;
  }
}

watch(
  () => [props.departmentId, props.agentId, normalizedOptions.value.map((option) => option.id).join("|")] as const,
  () => {
    if (!props.autoSelectFirst) return;
    if (selectedValue.value || normalizedOptions.value.length === 0) return;
    const firstAvailable = normalizedOptions.value.find(
      (option) => !option.unavailable && !option.personaMissing && !!String(option.agentId || "").trim(),
    );
    if (!firstAvailable) return;
    emitSelection(firstAvailable);
    const configId = String(firstAvailable.apiConfigId || "").trim();
    if (configId) {
      emit("update:apiConfigId", configId);
    }
  },
  { immediate: true },
);

watch(() => props.disabled, (disabled) => {
  if (disabled) dropdownOpen.value = false;
});

watch(dropdownOpen, async (open) => {
  if (!open) return;
  await nextTick();
  updateDropdownLayout();
});

onMounted(() => {
  document.addEventListener("pointerdown", handleDocumentPointerDown);
  window.addEventListener("resize", updateDropdownLayout, { passive: true });
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", handleDocumentPointerDown);
  window.removeEventListener("resize", updateDropdownLayout);
});
</script>

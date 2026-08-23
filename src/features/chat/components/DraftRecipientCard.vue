<template>
  <div class="absolute inset-0 z-10 flex items-center justify-center overflow-hidden">
    <div class="pointer-events-none absolute inset-0" aria-hidden="true">
      <div class="absolute -top-16 left-1/2 h-72 w-72 -translate-x-1/2 rounded-full bg-primary/15 blur-3xl"></div>
      <div class="absolute bottom-8 left-1/5 h-80 w-80 rounded-full bg-secondary/15 blur-3xl"></div>
      <div class="absolute -bottom-24 right-1/6 h-72 w-72 rounded-full bg-accent/10 blur-3xl"></div>
    </div>
    <div class="pointer-events-none absolute left-1/2 top-1/2 h-96 w-96 -translate-x-1/2 -translate-y-1/2 rounded-full bg-white/15 blur-3xl" />

    <div class="relative flex max-h-full flex-col items-center gap-8 px-6 py-8">
      <div class="text-sm font-medium tracking-wide text-base-content/60">
        {{ t("chat.draftRecipientTitle") }}
      </div>

      <div class="flex flex-col items-center gap-3">
        <div class="avatar">
          <div
            class="h-28 w-28 rounded-full shadow-2xl ring-4 ring-primary/60 ring-offset-4 ring-offset-base-100/50"
          >
            <img
              v-if="selectedOption && resolveAvatarUrl(selectedOption.agentId)"
              :src="resolveAvatarUrl(selectedOption.agentId)"
              :alt="selectedOption.agentName"
              class="h-28 w-28 rounded-full object-cover"
            />
            <div
              v-else
              class="flex h-28 w-28 items-center justify-center rounded-full bg-primary text-4xl font-semibold text-primary-content"
            >
              {{ selectedOption ? agentInitials(selectedOption.agentName) : "?" }}
            </div>
          </div>
        </div>
        <div class="flex flex-col items-center gap-0.5 text-center">
          <div class="text-xl font-bold text-base-content">
            {{ selectedOption ? selectedOption.agentName : t("chat.draftRecipientPlaceholder") }}
          </div>
          <div class="text-sm text-base-content/60">
            {{ selectedOption ? selectedOption.departmentName : t("chat.draftRecipientPickHint") }}
          </div>
        </div>
      </div>

      <div class="flex max-w-full flex-wrap items-stretch justify-center gap-3">
        <div
          v-for="group in recentGroups"
          :key="group.agentId"
          class="flex w-24 shrink-0 flex-col items-center gap-1.5 rounded-2xl border border-base-300/70 bg-base-100/60 px-1 py-2.5 backdrop-blur-sm transition-all hover:-translate-y-0.5 hover:border-primary/50 hover:bg-base-100 hover:shadow-lg"
          :class="selectedAgentId === group.agentId ? 'border-primary/60 bg-primary/10' : ''"
        >
          <button
            type="button"
            class="flex w-full flex-col items-center gap-1.5"
            @click="emit('change', { departmentId: group.departments[0].departmentId, agentId: group.agentId })"
          >
            <div class="avatar">
              <div
                class="h-14 w-14 rounded-full transition-shadow"
                :class="selectedAgentId === group.agentId ? 'ring-2 ring-primary' : ''"
              >
                <img
                  v-if="resolveAvatarUrl(group.agentId)"
                  :src="resolveAvatarUrl(group.agentId)"
                  :alt="group.agentName"
                  class="h-14 w-14 rounded-full object-cover"
                />
                <div
                  v-else
                  class="flex h-14 w-14 items-center justify-center rounded-full bg-primary/80 text-lg font-semibold text-primary-content"
                >
                  {{ agentInitials(group.agentName) }}
                </div>
              </div>
            </div>
            <span class="max-w-full truncate text-center text-xs leading-tight text-base-content/80">
              {{ group.agentName }}
            </span>
            <span
              class="max-w-full truncate rounded-full px-1.5 py-0.5 text-[10px] leading-tight"
              :class="group.departments[0].id === selectedId
                ? 'bg-primary/15 font-medium text-primary'
                : 'bg-base-content/10 text-base-content/60'"
            >
              {{ group.departments[0].departmentName }}
            </span>
          </button>
          <div
            v-if="group.departments.length > 1"
            class="flex w-full flex-col items-stretch gap-0.5"
          >
            <button
              v-for="deptOption in group.departments.slice(1)"
              :key="deptOption.id"
              type="button"
              class="flex items-center justify-center gap-1 rounded-full px-1.5 py-0.5 text-[10px] leading-tight transition-colors"
              :class="deptOption.id === selectedId
                ? 'bg-primary/15 font-medium text-primary'
                : 'bg-base-content/10 text-base-content/60 hover:bg-base-content/15 hover:text-base-content'"
              @click="emit('change', { departmentId: deptOption.departmentId, agentId: deptOption.agentId })"
            >
              <span class="max-w-full truncate">{{ deptOption.departmentName }}</span>
            </button>
          </div>
        </div>

        <button
          v-if="recentGroups.length > 0"
          type="button"
          class="flex w-20 shrink-0 flex-col items-center justify-center gap-1.5 rounded-2xl border border-dashed border-base-content/25 px-1 py-2.5 text-base-content/55 transition-colors hover:border-primary/50 hover:bg-base-100/70 hover:text-base-content"
          @click="showAll = true"
        >
          <span class="flex h-14 w-14 items-center justify-center rounded-full bg-base-content/10 text-xl leading-none">
            +
          </span>
          <span class="max-w-full truncate text-center text-xs leading-tight">
            {{ t("chat.draftRecipientMore") }}
          </span>
        </button>
        <button
          v-else
          type="button"
          class="flex items-center gap-1.5 rounded-full border border-base-content/25 px-4 py-2 text-sm text-base-content/70 transition-colors hover:border-primary/50 hover:text-base-content"
          @click="showAll = true"
        >
          {{ t("chat.draftRecipientMore") }}
        </button>
      </div>
    </div>

    <div
      v-if="showAll"
      class="absolute inset-0 z-10 flex items-center justify-center p-6"
      @click.self="showAll = false"
    >
      <div class="flex h-[26rem] w-full max-w-2xl flex-col overflow-hidden rounded-2xl border border-base-300 bg-base-100 shadow-2xl">
        <div class="flex shrink-0 items-center justify-between border-b border-base-300 px-4 py-2.5">
          <span class="text-sm font-semibold">{{ t("chat.draftRecipientAllTitle") }}</span>
          <button
            type="button"
            class="btn btn-ghost btn-xs"
            @click="showAll = false"
          >
            {{ t("common.close") }}
          </button>
        </div>
        <div class="min-h-0 flex-1">
          <PersonaGroupGrid
            :options="options"
            :selected-id="selectedId"
            :avatar-url-map="avatarUrlMap"
            @select="handleSelectFromAll"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { departmentPersonaOptionId, type DepartmentPersonaOption } from "../../shared/department-persona-options";
import PersonaGroupGrid from "../../shared/components/PersonaGroupGrid.vue";

interface RecentRecipientGroup {
  agentId: string;
  agentName: string;
  departments: DepartmentPersonaOption[];
}

const props = withDefaults(defineProps<{
  options?: DepartmentPersonaOption[];
  recentOptions?: DepartmentPersonaOption[];
  selectedDepartmentId?: string;
  selectedAgentId?: string;
  avatarUrlMap?: Record<string, string>;
}>(), {
  options: () => [],
  recentOptions: () => [],
  selectedDepartmentId: "",
  selectedAgentId: "",
  avatarUrlMap: () => ({}),
});

const emit = defineEmits<{
  change: [value: { departmentId: string; agentId: string }];
}>();

const { t } = useI18n();

const showAll = ref(false);

const selectedAgentId = computed(() => String(props.selectedAgentId || "").trim());

const selectedId = computed(() => {
  const departmentId = String(props.selectedDepartmentId || "").trim();
  const agentId = String(props.selectedAgentId || "").trim();
  if (!departmentId || !agentId) return "";
  return departmentPersonaOptionId(departmentId, agentId);
});

const selectedOption = computed<DepartmentPersonaOption | null>(() => {
  const id = selectedId.value;
  if (!id) return null;
  return (
    props.options.find((option) => option.id === id)
    || props.recentOptions.find((option) => option.id === id)
    || null
  );
});

// 行星候选按人格（agentId）聚合：一个人格一个卡片，卡片内列出最近用过的部门行
const recentGroups = computed<RecentRecipientGroup[]>(() => {
  const groups: RecentRecipientGroup[] = [];
  const groupByAgentId = new Map<string, RecentRecipientGroup>();
  for (const option of props.recentOptions) {
    const agentId = String(option.agentId || "").trim();
    if (!agentId) continue;
    let group = groupByAgentId.get(agentId);
    if (!group) {
      group = { agentId, agentName: option.agentName, departments: [] };
      groupByAgentId.set(agentId, group);
      groups.push(group);
    }
    group.departments.push(option);
  }
  return groups;
});

function resolveAvatarUrl(agentId: string): string {
  return props.avatarUrlMap?.[agentId] || "";
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

function handleSelectFromAll(option: DepartmentPersonaOption) {
  emit("change", { departmentId: option.departmentId, agentId: option.agentId });
  showAll.value = false;
}
</script>

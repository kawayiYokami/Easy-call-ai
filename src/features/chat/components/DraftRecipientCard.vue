<template>
  <div class="absolute inset-0 z-10 flex items-center justify-center overflow-hidden">
    <div class="pointer-events-none absolute inset-0" aria-hidden="true">
      <div class="absolute -top-16 left-1/2 h-72 w-72 -translate-x-1/2 rounded-full bg-primary/5 blur-3xl"></div>
      <div class="absolute bottom-8 left-1/5 h-80 w-80 rounded-full bg-secondary/5 blur-3xl"></div>
      <div class="absolute -bottom-24 right-1/6 h-72 w-72 rounded-full bg-accent/3 blur-3xl"></div>
    </div>
    <div class="pointer-events-none absolute left-1/2 top-1/2 h-96 w-96 -translate-x-1/2 -translate-y-1/2 rounded-full bg-white/5 blur-3xl" />

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

      <div
        v-if="saveWorkspace"
        class="flex w-full max-w-md flex-col items-center gap-2"
      >
        <div ref="workspaceSelectRootRef" class="relative w-full">
          <div class="flex h-9 w-full items-center gap-1 rounded-field border border-base-content/10 bg-base-100/50 pl-3.5 pr-1 shadow-sm backdrop-blur-md">
            <FolderOpen class="h-3.5 w-3.5 shrink-0 text-base-content/45" />
            <button
              type="button"
              class="min-w-0 flex-1 cursor-pointer text-left text-xs font-medium text-base-content outline-none"
              :disabled="mergedOptions.length === 0"
              @click="toggleWorkspaceDropdown"
            >
              <span class="block w-full truncate" :class="selectedWorkspaceName ? '' : 'text-base-content/45'">
                {{ selectedWorkspaceName || t("chat.draftWorkspacePlaceholder") }}
              </span>
            </button>
            <ChevronDown
              class="pointer-events-none h-3.5 w-3.5 shrink-0 text-base-content/45 transition-transform"
              :class="workspaceDropdownOpen ? 'rotate-180' : ''"
            />
            <button
              type="button"
              class="btn btn-ghost btn-circle btn-sm shrink-0 text-base-content/55"
              :title="t('common.browse')"
              @click="browseWorkspaceDirectory"
            >
              <FolderSearch class="h-3.5 w-3.5" />
            </button>
          </div>
          <div
            v-if="workspaceDropdownOpen && mergedOptions.length > 0"
            class="absolute z-30 w-full overflow-hidden rounded-box border border-base-300 bg-base-100 shadow-xl"
            :class="workspaceDropdownDirection === 'up' ? 'bottom-full mb-2' : 'top-full mt-2'"
          >
            <div class="max-h-60 overflow-y-auto overscroll-contain p-1">
              <button
                v-for="option in mergedOptions"
                :key="option.path"
                type="button"
                class="flex w-full items-center gap-2 rounded-field px-2.5 py-2 text-left text-xs transition-colors"
                :class="option.path.toLowerCase() === selectedPath.toLowerCase()
                  ? 'bg-base-200 font-medium'
                  : 'hover:bg-base-200/70'"
                :title="option.path"
                @click="handleWorkspaceOptionSelect(option.path)"
              >
                <span class="min-w-0 flex-1 truncate">{{ option.name }}</span>
                <Check
                  v-if="option.path.toLowerCase() === selectedPath.toLowerCase()"
                  class="h-3.5 w-3.5 shrink-0 text-primary"
                />
              </button>
            </div>
          </div>
        </div>
        <div class="flex max-w-full flex-wrap items-center justify-center gap-2">
          <div class="flex items-center rounded-selector border border-base-content/10 bg-base-content/5 p-0.5 backdrop-blur-md">
            <button
              v-for="accessOption in ACCESS_OPTIONS"
              :key="accessOption"
              type="button"
              class="rounded-selector px-3 py-1.5 text-[11px] font-medium leading-none transition-all"
              :class="selectedAccess === accessOption
                ? 'bg-base-100 text-base-content shadow-sm'
                : 'text-base-content/55 hover:text-base-content'"
              @click="setAccess(accessOption)"
            >
              {{ t(`config.tools.workspaceAccess${ACCESS_LABEL_KEY[accessOption]}`) }}
            </button>
          </div>
          <div class="flex items-center rounded-selector border border-base-content/10 bg-base-content/5 p-0.5 backdrop-blur-md">
            <button
              v-for="modeOption in MODE_OPTIONS"
              :key="modeOption"
              type="button"
              class="rounded-selector px-3 py-1.5 text-[11px] font-medium leading-none transition-all"
              :class="selectedWorkMode === modeOption
                ? 'bg-base-100 text-base-content shadow-sm'
                : (isWorkModeDisabled(modeOption) ? 'cursor-not-allowed text-base-content/30' : 'text-base-content/55 hover:text-base-content')"
              :title="t(MODE_HINT_KEY[modeOption])"
              @click="setWorkMode(modeOption)"
            >
              {{ t(`chat.draftWorkMode${MODE_LABEL_KEY[modeOption]}`) }}
            </button>
          </div>
        </div>
        <div
          class="h-4 text-center text-[11px] leading-4 transition-opacity duration-200"
          :class="worktreeCheckMessage ? 'text-base-content/50 opacity-100' : 'opacity-0'"
        >
          {{ worktreeCheckMessage || " " }}
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
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { FolderOpen, FolderSearch, ChevronDown, Check } from "@lucide/vue";
import { openTransportFileDialog } from "../../../services/tauri-api";
import { departmentPersonaOptionId, type DepartmentPersonaOption } from "../../shared/department-persona-options";
import PersonaGroupGrid from "../../shared/components/PersonaGroupGrid.vue";

interface RecentRecipientGroup {
  agentId: string;
  agentName: string;
  departments: DepartmentPersonaOption[];
}

type WorkspaceOption = {
  id: string;
  name: string;
  path: string;
  access: "read_only" | "approval" | "full_access";
};

type ShellWorkspaceAccess = WorkspaceOption["access"];
type ShellWorkMode = "directory" | "isolated_worktree" | "independent_worktree";

const props = withDefaults(defineProps<{
  options?: DepartmentPersonaOption[];
  recentOptions?: DepartmentPersonaOption[];
  selectedDepartmentId?: string;
  selectedAgentId?: string;
  avatarUrlMap?: Record<string, string>;
  workspaceOptions?: WorkspaceOption[];
  workspaceRootPath?: string;
  workspaceAccess?: ShellWorkspaceAccess | "";
  workspaceWorkMode?: ShellWorkMode;
  workspaceAutonomousMode?: boolean;
  saveWorkspace?: (input: { path: string; name: string; access: ShellWorkspaceAccess; workMode: ShellWorkMode }) => Promise<void>;
  gitRootCheck?: (path: string) => Promise<boolean>;
}>(), {
  options: () => [],
  recentOptions: () => [],
  selectedDepartmentId: "",
  selectedAgentId: "",
  avatarUrlMap: () => ({}),
  workspaceOptions: () => [],
  workspaceRootPath: "",
  workspaceAccess: "",
  workspaceWorkMode: "directory",
  workspaceAutonomousMode: false,
});

const emit = defineEmits<{
  change: [value: { departmentId: string; agentId: string }];
}>();

const { t } = useI18n();

const showAll = ref(false);

// ========== 草稿工作区快捷设置 ==========

const ACCESS_OPTIONS = ["read_only", "approval", "full_access"] as const;
const ACCESS_LABEL_KEY: Record<ShellWorkspaceAccess, string> = {
  read_only: "ReadOnly",
  approval: "Approval",
  full_access: "FullAccess",
};
const MODE_OPTIONS = ["directory", "isolated_worktree", "independent_worktree"] as const;
const MODE_LABEL_KEY: Record<ShellWorkMode, string> = {
  directory: "Directory",
  isolated_worktree: "Isolated",
  independent_worktree: "Independent",
};
const MODE_HINT_KEY: Record<ShellWorkMode, string> = {
  directory: "chat.workspaceWorkModeDirectory",
  isolated_worktree: "chat.workspaceWorkModeIsolated",
  independent_worktree: "chat.workspaceWorkModeIndependent",
};

const selectedPath = ref("");
const selectedAccess = ref<ShellWorkspaceAccess>("approval");
const selectedWorkMode = ref<ShellWorkMode>("directory");
const worktreeAvailable = ref(false);
const worktreeCheckMessage = ref("");
const customOption = ref<WorkspaceOption | null>(null);
const saving = ref(false);
let pendingSave = false;
let checkSequence = 0;
let lastGitCheckPath = "";

function normalizeAccess(value: unknown): ShellWorkspaceAccess {
  const text = String(value || "").trim();
  if (text === "full_access" || text === "read_only" || text === "approval") return text;
  return "approval";
}

const mergedOptions = computed<WorkspaceOption[]>(() => {
  const list = [...props.workspaceOptions];
  if (customOption.value && !list.some((item) => item.path.toLowerCase() === customOption.value!.path.toLowerCase())) {
    list.push(customOption.value);
  }
  return list;
});

function findOptionByPath(path: string): WorkspaceOption | null {
  const target = String(path || "").trim().toLowerCase();
  if (!target) return null;
  return mergedOptions.value.find((item) => item.path.toLowerCase() === target) ?? null;
}

// ========== 目录组件化下拉（照 DepartmentPersonaSelect 的交互骨架） ==========

const workspaceDropdownOpen = ref(false);
const workspaceDropdownDirection = ref<"up" | "down">("down");
const workspaceSelectRootRef = ref<HTMLElement | null>(null);

const selectedWorkspaceName = computed(() => {
  if (!selectedPath.value) return "";
  return findOptionByPath(selectedPath.value)?.name || selectedPath.value.replace(/\\/g, "/").replace(/\/+$/, "").split("/").pop() || selectedPath.value;
});

function toggleWorkspaceDropdown() {
  if (mergedOptions.value.length === 0) return;
  workspaceDropdownOpen.value = !workspaceDropdownOpen.value;
  if (workspaceDropdownOpen.value) {
    void nextTick(updateWorkspaceDropdownLayout);
  }
}

function handleWorkspaceOptionSelect(path: string) {
  workspaceDropdownOpen.value = false;
  handleDirectoryChange(path);
}

function updateWorkspaceDropdownLayout() {
  if (!workspaceDropdownOpen.value) return;
  const root = workspaceSelectRootRef.value;
  if (!root) return;
  const rect = root.getBoundingClientRect();
  const viewportHeight = window.innerHeight || document.documentElement.clientHeight || 0;
  const panelHeight = Math.min(mergedOptions.value.length * 34 + 8, 240);
  const spaceBelow = viewportHeight - rect.bottom;
  const spaceAbove = rect.top;
  workspaceDropdownDirection.value = spaceBelow >= panelHeight || spaceBelow >= spaceAbove ? "down" : "up";
}

function handleDocumentPointerDown(event: PointerEvent) {
  if (!workspaceDropdownOpen.value) return;
  const target = event.target as Node | null;
  if (workspaceSelectRootRef.value && target && !workspaceSelectRootRef.value.contains(target)) {
    workspaceDropdownOpen.value = false;
  }
}

onMounted(() => {
  document.addEventListener("pointerdown", handleDocumentPointerDown);
  window.addEventListener("resize", updateWorkspaceDropdownLayout, { passive: true });
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", handleDocumentPointerDown);
  window.removeEventListener("resize", updateWorkspaceDropdownLayout);
});

watch(
  () => [props.workspaceAccess, props.workspaceWorkMode] as const,
  ([nextAccess, nextMode]) => {
    selectedAccess.value = normalizeAccess(nextAccess);
    selectedWorkMode.value = nextMode === "isolated_worktree" || nextMode === "independent_worktree" ? nextMode : "directory";
  },
  { immediate: true },
);

watch(
  () => props.workspaceRootPath,
  (nextPath) => {
    const normalized = String(nextPath || "").trim();
    if (selectedPath.value !== normalized) {
      customOption.value = null;
      worktreeCheckMessage.value = "";
      worktreeAvailable.value = false;
    }
    selectedPath.value = normalized;
    // 仅在目录真正变化时探测 Git 根，保存回流同值不重复检查
    if (normalized && props.gitRootCheck && normalized !== lastGitCheckPath) {
      void runGitRootCheck(normalized);
    }
  },
  { immediate: true },
);

function isWorkModeDisabled(mode: ShellWorkMode): boolean {
  if (mode === "directory") return false;
  // 与旧版一致：只读（且未开最大权限）不能用工作树；目录不是 Git 根时禁用
  if (selectedAccess.value === "read_only" && !props.workspaceAutonomousMode) return true;
  return !worktreeAvailable.value;
}

function buildSnapshot() {
  const source = findOptionByPath(selectedPath.value);
  return {
    path: selectedPath.value,
    name: String(source?.name || "").trim() || selectedPath.value.replace(/\\/g, "/").replace(/\/+$/, "").split("/").pop() || selectedPath.value,
    access: selectedAccess.value,
    workMode: selectedWorkMode.value,
  };
}

async function commitSave() {
  if (!props.saveWorkspace || !selectedPath.value) return;
  if (saving.value) {
    pendingSave = true;
    return;
  }
  saving.value = true;
  try {
    while (true) {
      pendingSave = false;
      try {
        await props.saveWorkspace(buildSnapshot());
      } catch {
        restoreFromProps();
        break;
      }
      if (!pendingSave) break;
    }
  } finally {
    saving.value = false;
  }
}

function restoreFromProps() {
  selectedPath.value = String(props.workspaceRootPath || "").trim();
  selectedAccess.value = normalizeAccess(props.workspaceAccess);
  selectedWorkMode.value = props.workspaceWorkMode === "isolated_worktree" || props.workspaceWorkMode === "independent_worktree"
    ? props.workspaceWorkMode
    : "directory";
  worktreeAvailable.value = false;
  worktreeCheckMessage.value = "";
}

async function runGitRootCheck(path: string) {
  const sequence = ++checkSequence;
  lastGitCheckPath = path;
  if (!props.gitRootCheck || !path) {
    worktreeAvailable.value = false;
    worktreeCheckMessage.value = "";
    return;
  }
  worktreeAvailable.value = false;
  worktreeCheckMessage.value = t("chat.workspaceWorktreeChecking");
  try {
    const available = await props.gitRootCheck(path);
    if (sequence !== checkSequence) return;
    worktreeAvailable.value = Boolean(available);
    worktreeCheckMessage.value = available ? "" : t("chat.workspaceWorktreeUnavailable");
  } catch (error) {
    if (sequence !== checkSequence) return;
    worktreeAvailable.value = false;
    worktreeCheckMessage.value = error instanceof Error ? error.message : String(error);
  }
  if (!worktreeAvailable.value && selectedWorkMode.value !== "directory") {
    selectedWorkMode.value = "directory";
    void commitSave();
  }
}

function handleDirectoryChange(path: string) {
  const normalized = String(path || "").trim();
  if (!normalized) return;
  const source = findOptionByPath(normalized);
  selectedPath.value = normalized;
  selectedAccess.value = normalizeAccess(source?.access);
  if (selectedAccess.value === "read_only" && !props.workspaceAutonomousMode) {
    selectedWorkMode.value = "directory";
  }
  void commitSave();
  void runGitRootCheck(normalized);
}

function setAccess(access: ShellWorkspaceAccess) {
  if (selectedAccess.value === access) return;
  selectedAccess.value = access;
  if (access === "read_only" && !props.workspaceAutonomousMode && selectedWorkMode.value !== "directory") {
    selectedWorkMode.value = "directory";
  }
  void commitSave();
}

function setWorkMode(mode: ShellWorkMode) {
  if (selectedWorkMode.value === mode || isWorkModeDisabled(mode)) return;
  selectedWorkMode.value = mode;
  void commitSave();
}

async function browseWorkspaceDirectory() {
  workspaceDropdownOpen.value = false;
  let picked: string | string[] | null = null;
  try {
    picked = await openTransportFileDialog({ directory: true, multiple: false });
  } catch {
    return;
  }
  const path = String(Array.isArray(picked) ? picked[0] || "" : picked || "").trim();
  if (!path) return;
  const existing = findOptionByPath(path);
  if (!existing) {
    customOption.value = {
      id: `conversation-workspace-custom-${Date.now().toString(36)}`,
      name: path.replace(/\\/g, "/").replace(/\/+$/, "").split("/").pop() || path,
      path,
      access: "approval",
    };
  }
  handleDirectoryChange(path);
}

// ========== 人格候选 ==========

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

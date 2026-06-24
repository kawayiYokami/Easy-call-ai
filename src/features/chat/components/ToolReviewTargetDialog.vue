<template>
  <dialog class="modal" :class="{ 'modal-open': open }">
    <div class="modal-box w-[88vw] max-w-4xl overflow-visible p-0">
      <div class="border-b border-base-300 px-5 py-4">
        <div class="text-base font-semibold">{{ t("chat.toolReview.generateReviewReport") }}</div>
      </div>
      <div class="relative z-20 overflow-visible px-5 pt-4">
        <div class="mb-4 grid gap-1.5">
          <div class="text-xs font-medium text-base-content/60">{{ t("chat.toolReview.departmentLabel") }}</div>
          <DepartmentPersonaSelect
            v-model:department-id="selectedDepartmentId"
            v-model:agent-id="selectedAgentId"
            :options="departmentSelectOptions"
            :persona-avatar-url-map="personaAvatarUrlMap"
            auto-select-first
          />
        </div>
        <div role="tablist" class="tabs tabs-border">
          <button type="button" role="tab" class="tab" :class="{ 'tab-active': scope === 'commit' }" @click="setScope('commit')">{{ t("chat.toolReview.scopeCommit") }}</button>
          <button type="button" role="tab" class="tab" :class="{ 'tab-active': scope === 'main' }" @click="setScope('main')">{{ t("chat.toolReview.scopeMain") }}</button>
          <button type="button" role="tab" class="tab" :class="{ 'tab-active': scope === 'uncommitted' }" @click="setScope('uncommitted')">{{ t("chat.toolReview.scopeUncommitted") }}</button>
          <button type="button" role="tab" class="tab" :class="{ 'tab-active': scope === 'custom' }" @click="setScope('custom')">{{ t("chat.toolReview.scopeCustom") }}</button>
        </div>
      </div>
      <div class="relative z-0 px-5 py-4">
        <div v-if="scope === 'commit'" class="rounded-box border border-base-300">
          <div class="sticky top-0 z-10 flex items-center justify-between border-b border-base-300 bg-base-100 px-4 py-3 text-sm">
            <button type="button" class="btn btn-sm" :disabled="commitOptionsLoading || commitPage <= 1" @click="requestCommitPage(commitPage - 1)">上一页</button>
            <span class="text-base-content/70">第 {{ commitPage }} 页 / 共 {{ commitTotalPages }} 页 · {{ commitTotal }}</span>
            <button type="button" class="btn btn-sm" :disabled="commitOptionsLoading || commitPage >= commitTotalPages" @click="requestCommitPage(commitPage + 1)">下一页</button>
          </div>
          <div class="max-h-[55vh] overflow-y-auto">
            <div v-if="commitOptionsLoading" class="px-4 py-3 text-sm text-base-content/70">{{ t("chat.toolReview.commitPickerLoading") }}</div>
            <div v-else-if="commitOptions.length === 0" class="px-4 py-3 text-sm text-base-content/70">{{ t("chat.toolReview.commitPickerEmpty") }}</div>
            <button
              v-for="item in commitOptions"
              :key="item.hash"
              type="button"
              class="flex w-full items-start gap-3 border-b border-base-300 px-4 py-3 text-left last:border-b-0 hover:bg-base-200"
              @click="toggleCommitSelection(item.hash)"
            >
              <input type="checkbox" class="checkbox checkbox-sm mt-1" :checked="selectedCommitHashes.includes(item.hash)" tabindex="-1">
              <div class="min-w-0 flex-1 text-sm text-base-content">{{ item.subject }}</div>
            </button>
          </div>
        </div>

        <div v-else-if="scope === 'custom'">
          <textarea
            v-model="customTargetText"
            class="textarea textarea-bordered h-40 w-full"
            :placeholder="t('chat.toolReview.customDialogPlaceholder')"
          ></textarea>
        </div>

        <div v-else class="rounded-box border border-base-300 px-4 py-3 text-sm text-base-content/70">
          {{ scope === 'main' ? t('chat.toolReview.scopeMain') : t('chat.toolReview.scopeUncommitted') }}
        </div>
        <div v-if="errorText" class="mt-3 rounded border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
          {{ errorText }}
        </div>
      </div>
      <div class="flex items-center justify-end gap-3 border-t border-base-300 px-5 py-4">
        <button type="button" class="btn" :disabled="submitting" @click="close">{{ t("common.cancel") }}</button>
        <button type="button" class="btn btn-primary" :disabled="!canConfirm" @click="confirm">{{ t("common.confirm") }}</button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="close">close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import DepartmentPersonaSelect from "../../shared/components/DepartmentPersonaSelect.vue";
import type { ToolReviewCodeReviewScope, ToolReviewCommitOption } from "../composables/use-chat-tool-review";
import type { DepartmentPersonaOption } from "../../shared/department-persona-options";

type DepartmentOption = DepartmentPersonaOption;

const props = defineProps<{
  open: boolean;
  submitting: boolean;
  errorText: string;
  currentDepartmentId: string;
  departmentOptions: DepartmentOption[];
  personaAvatarUrlMap?: Record<string, string>;
  commitOptions: ToolReviewCommitOption[];
  commitOptionsLoading: boolean;
  commitTotal: number;
  commitPage: number;
  commitPageSize: number;
}>();

const emit = defineEmits<{
  close: [];
  pickCommitReview: [page: number];
  reviewCode: [input: { scope: ToolReviewCodeReviewScope; target?: string; departmentId: string }];
}>();

const { t } = useI18n();
const selectedDepartmentId = ref("");
const selectedAgentId = ref("");
const selectedCommitHashes = ref<string[]>([]);
const customTargetText = ref("");
const scope = ref<ToolReviewCodeReviewScope>("main");

const departmentSelectOptions = computed<DepartmentPersonaOption[]>(() => {
  const seen = new Set<string>();
  return (Array.isArray(props.departmentOptions) ? props.departmentOptions : [])
    .map((item) => {
      const departmentId = String(item.departmentId || item.id || "").trim();
      return {
        ...item,
        departmentId,
        id: String(item.id || `${departmentId}::${String(item.agentId || "").trim()}`).trim(),
      };
    })
    .filter((item) => {
      if (!item.departmentId || seen.has(item.departmentId)) return false;
      seen.add(item.departmentId);
      return true;
    });
});

const validDepartmentId = computed(() => {
  const selected = String(selectedDepartmentId.value || "").trim();
  if (selected && departmentSelectOptions.value.some((item) => item.departmentId === selected)) return selected;
  const current = String(props.currentDepartmentId || "").trim();
  if (current && departmentSelectOptions.value.some((item) => item.departmentId === current)) return current;
  return String(departmentSelectOptions.value[0]?.departmentId || "").trim();
});

const commitTotalPages = computed(() => Math.max(1, Math.ceil(props.commitTotal / Math.max(1, props.commitPageSize))));

const canConfirm = computed(() => {
  if (props.submitting || !validDepartmentId.value) return false;
  if (scope.value === "commit") return selectedCommitHashes.value.length > 0;
  if (scope.value === "custom") return !!customTargetText.value.trim();
  return true;
});

watch(
  () => [props.currentDepartmentId, departmentSelectOptions.value.map((item) => item.departmentId).join("|")] as const,
  () => {
    const current = String(props.currentDepartmentId || "").trim();
    const selectedOption = departmentSelectOptions.value.find((item) => item.departmentId === current)
      || departmentSelectOptions.value[0]
      || null;
    selectedDepartmentId.value = String(selectedOption?.departmentId || "").trim();
    selectedAgentId.value = String(selectedOption?.agentId || "").trim();
  },
  { immediate: true },
);

watch(
  () => props.open,
  (open) => {
    if (!open) return;
    const current = String(props.currentDepartmentId || "").trim();
    const selectedOption = departmentSelectOptions.value.find((item) => item.departmentId === current)
      || departmentSelectOptions.value[0]
      || null;
    selectedDepartmentId.value = String(selectedOption?.departmentId || "").trim();
    selectedAgentId.value = String(selectedOption?.agentId || "").trim();
  },
);

function setScope(nextScope: ToolReviewCodeReviewScope) {
  scope.value = nextScope;
  if (nextScope === "commit" && !props.commitOptionsLoading && props.commitOptions.length === 0) {
    emit("pickCommitReview", 1);
  }
}

function requestCommitPage(page: number) {
  const normalizedPage = Math.min(Math.max(1, page), commitTotalPages.value);
  emit("pickCommitReview", normalizedPage);
}

function toggleCommitSelection(hash: string) {
  const normalizedHash = String(hash || "").trim();
  if (!normalizedHash) return;
  selectedCommitHashes.value = selectedCommitHashes.value.includes(normalizedHash)
    ? selectedCommitHashes.value.filter((item) => item !== normalizedHash)
    : [...selectedCommitHashes.value, normalizedHash];
}

function close() {
  selectedCommitHashes.value = [];
  customTargetText.value = "";
  emit("close");
}

function confirm() {
  const departmentId = validDepartmentId.value;
  if (!departmentId) return;
  if (scope.value === "commit") {
    if (selectedCommitHashes.value.length === 0) return;
    emit("reviewCode", { scope: "commit", target: selectedCommitHashes.value.join("\n"), departmentId });
    close();
    return;
  }
  if (scope.value === "custom") {
    const target = customTargetText.value.trim();
    if (!target) return;
    emit("reviewCode", { scope: "custom", target, departmentId });
    close();
    return;
  }
  emit("reviewCode", { scope: scope.value, target: "", departmentId });
  close();
}
</script>

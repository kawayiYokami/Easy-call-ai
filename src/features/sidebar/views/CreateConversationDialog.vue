<template>
  <dialog class="modal" :class="{ 'modal-open': open }">
    <div class="modal-box max-w-md">
      <h3 class="text-base font-semibold">新建会话</h3>
      <div class="mt-3 flex flex-col gap-3">
        <input
          v-model="localTitle"
          type="text"
          class="input input-bordered w-full"
          placeholder="会话主题"
          @keydown.enter.prevent="confirm"
        />
        <DepartmentPersonaSelect
          v-model:department-id="localDepartmentId"
          v-model:agent-id="localAgentId"
          :options="departments"
          auto-select-first
        />
      </div>
      <div v-if="errorText" class="mt-3 rounded border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
        {{ errorText }}
      </div>
      <div class="modal-action">
        <button class="btn btn-sm" :disabled="creating" @click="emit('close')">取消</button>
        <button class="btn btn-sm btn-primary" :disabled="creating || !localDepartmentId || !localAgentId" @click="confirm">
          <span v-if="creating" class="loading loading-spinner loading-xs"></span>
          <span>{{ creating ? "正在创建" : "创建" }}</span>
        </button>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop">
      <button @click.prevent="emit('close')">close</button>
    </form>
  </dialog>
</template>

<script setup lang="ts">
import { ref, watch } from "vue";
import DepartmentPersonaSelect from "../../shared/components/DepartmentPersonaSelect.vue";
import type { DepartmentPersonaOption } from "../../shared/department-persona-options";

export type SidebarCreateDepartmentOption = DepartmentPersonaOption;

const props = defineProps<{
  open: boolean;
  creating: boolean;
  departments: SidebarCreateDepartmentOption[];
  defaultDepartmentId: string;
  errorText: string;
}>();

const emit = defineEmits<{
  close: [];
  confirm: [input: { title?: string; departmentId: string; agentId: string }];
}>();

const localTitle = ref("");
const localDepartmentId = ref("");
const localAgentId = ref("");

watch(
  () => [props.open, props.defaultDepartmentId, props.departments.map((item) => item.id).join("|")] as const,
  ([open]) => {
    if (!open) return;
    localTitle.value = "";
    const option = props.departments.find((item) =>
      String(item.departmentId || "").trim() === String(props.defaultDepartmentId || "").trim()
    ) || props.departments[0];
    localDepartmentId.value = String(option?.departmentId || props.defaultDepartmentId || "").trim();
    localAgentId.value = String(option?.agentId || "").trim();
  },
  { immediate: true },
);

function confirm() {
  const departmentId = String(localDepartmentId.value || "").trim();
  const agentId = String(localAgentId.value || "").trim();
  if (!departmentId || !agentId) return;
  emit("confirm", {
    title: String(localTitle.value || "").trim() || undefined,
    departmentId,
    agentId,
  });
}
</script>

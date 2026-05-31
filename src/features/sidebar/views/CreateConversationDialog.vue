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
        <select v-model="localDepartmentId" class="select select-bordered w-full">
          <option v-for="department in departments" :key="department.id" :value="department.id">
            {{ departmentLabel(department) }}
          </option>
        </select>
      </div>
      <div v-if="errorText" class="mt-3 rounded border border-error/30 bg-error/10 px-3 py-2 text-sm text-error">
        {{ errorText }}
      </div>
      <div class="modal-action">
        <button class="btn btn-sm" :disabled="creating" @click="emit('close')">取消</button>
        <button class="btn btn-sm btn-primary" :disabled="creating || !localDepartmentId" @click="confirm">
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

export type SidebarCreateDepartmentOption = {
  id: string;
  name: string;
  ownerAgentId?: string;
  ownerName: string;
  providerName?: string;
  modelName?: string;
  childDepartmentIds?: string[];
};

const props = defineProps<{
  open: boolean;
  creating: boolean;
  departments: SidebarCreateDepartmentOption[];
  defaultDepartmentId: string;
  errorText: string;
}>();

const emit = defineEmits<{
  close: [];
  confirm: [input: { title?: string; departmentId: string }];
}>();

const localTitle = ref("");
const localDepartmentId = ref("");

watch(
  () => [props.open, props.defaultDepartmentId, props.departments.map((item) => item.id).join("|")] as const,
  ([open]) => {
    if (!open) return;
    localTitle.value = "";
    localDepartmentId.value = props.defaultDepartmentId || props.departments[0]?.id || "";
  },
  { immediate: true },
);

function departmentLabel(department: SidebarCreateDepartmentOption): string {
  const name = String(department.name || "").trim() || department.id;
  const owner = String(department.ownerName || "").trim();
  const model = String(department.modelName || "").trim();
  const suffix = [owner, model].filter(Boolean).join(" / ");
  return suffix ? `${name} / ${suffix}` : name;
}

function confirm() {
  const departmentId = String(localDepartmentId.value || "").trim();
  if (!departmentId) return;
  emit("confirm", {
    title: String(localTitle.value || "").trim() || undefined,
    departmentId,
  });
}
</script>

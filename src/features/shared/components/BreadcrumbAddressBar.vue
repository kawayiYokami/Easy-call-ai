<template>
  <div
    class="min-w-0 flex-1"
    :class="{ 'cursor-text': switchable && !isEditing }"
    @click="onWrapperClick"
  >
    <div
      v-if="!isEditing"
      class="flex min-h-8 items-center rounded-box border border-base-300 bg-base-200/30 px-2 py-1"
      :class="switchable ? 'hover:bg-base-200/60' : ''"
    >
      <div v-if="breadcrumbs.length === 0" class="truncate font-mono text-xs opacity-70">
        驱动器
      </div>
      <div v-else class="breadcrumbs min-w-0 flex-1 py-0 text-xs">
        <ul class="flex flex-wrap">
          <li v-for="crumb in breadcrumbs" :key="crumb.path">
            <button
              type="button"
              class="link link-hover max-w-[10rem] truncate font-mono"
              :title="crumb.path"
              @click.stop="emit('navigate', crumb.path)"
            >
              {{ crumb.name }}
            </button>
          </li>
        </ul>
      </div>
      <span v-if="switchable" class="ml-2 shrink-0 text-xs opacity-30">✎</span>
    </div>
    <div v-else class="join w-full">
      <input
        ref="inputRef"
        v-model="editValue"
        class="input input-bordered input-sm join-item min-w-0 flex-1 font-mono"
        type="text"
        :placeholder="placeholder"
        @keydown.enter.prevent="submitEdit"
        @keydown.esc.prevent="cancelEdit"
        @blur="onInputBlur"
      />
      <button type="button" class="btn btn-sm join-item" :disabled="!editValue.trim()" @click="submitEdit">
        前往
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{
    path: string;
    placeholder?: string;
    switchable?: boolean;
    disabled?: boolean;
  }>(),
  {
    placeholder: "例如 E:/github/easy_call_ai 或 /home/me/project",
    switchable: true,
    disabled: false,
  },
);

const emit = defineEmits<{
  (e: "navigate", path: string): void;
  (e: "submit", path: string): void;
}>();

const isEditing = ref(false);
const editValue = ref("");
const inputRef = ref<HTMLInputElement | null>(null);

const breadcrumbs = computed(() => {
  const p = String(props.path || "").trim();
  if (!p) return [] as Array<{ name: string; path: string }>;
  const normalized = p.replace(/\\/g, "/");
  if (/^[A-Za-z]:\/?$/.test(normalized)) {
    return [{ name: normalized.replace(/\/$/, "") || normalized, path: normalized }];
  }
  const parts = normalized.split("/").filter(Boolean);
  const isAbsolute = normalized.startsWith("/");
  const isWindows = /^[A-Za-z]:/.test(normalized);
  let accum = "";
  const result: Array<{ name: string; path: string }> = [];
  if (isAbsolute && !isWindows) {
    result.push({ name: "/", path: "/" });
    accum = "";
  }
  for (let i = 0; i < parts.length; i += 1) {
    const name = parts[i];
    if (isWindows && i === 0 && /^[A-Za-z]:$/.test(name)) {
      accum = `${name}/`;
      result.push({ name, path: accum });
    } else if (isWindows && i === 0) {
      accum = name;
      result.push({ name, path: accum });
    } else {
      if (accum === "/" || accum === "") {
        accum = isAbsolute ? `/${name}` : name;
      } else if (accum.endsWith("/")) {
        accum = `${accum}${name}`;
      } else {
        accum = `${accum}/${name}`;
      }
      result.push({ name, path: accum });
    }
  }
  return result;
});

function enterEdit() {
  if (!props.switchable || props.disabled) return;
  if (isEditing.value) return;
  editValue.value = String(props.path || "").trim();
  isEditing.value = true;
  void nextTick(() => {
    inputRef.value?.focus();
    inputRef.value?.select();
  });
}

function submitEdit() {
  const next = String(editValue.value || "").trim();
  isEditing.value = false;
  if (!next && !String(props.path || "").trim()) return;
  emit("submit", next);
}

function cancelEdit() {
  isEditing.value = false;
  editValue.value = String(props.path || "").trim();
}

function onInputBlur() {
  // 稍微延迟让按钮点击先生效，避免 blur 抢先关闭
  window.setTimeout(() => {
    if (!isEditing.value) return;
    // 失焦自动取消，不触发提交，避免误跳
    cancelEdit();
  }, 150);
}

function onWrapperClick(event: MouseEvent) {
  if (isEditing.value) return;
  if (!props.switchable || props.disabled) return;
  // 点击面包屑按钮已 stopPropagation，这里只处理空白区域
  const target = event.target as HTMLElement | null;
  if (target?.closest("button")) return;
  enterEdit();
}

watch(
  () => props.path,
  () => {
    if (!isEditing.value) {
      editValue.value = String(props.path || "").trim();
    }
  },
);

// 外部可通过点击空白进入编辑，暴露方法以便父组件主动进入
defineExpose({
  enterEdit,
  isEditing,
});
</script>

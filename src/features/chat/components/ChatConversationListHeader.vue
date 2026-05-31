<template>
  <div class="sticky top-0 z-10 bg-inherit">
    <div class="flex items-center gap-2 p-2 pb-0">
      <label class="input input-bordered input-sm flex h-8 min-w-0 flex-1 items-center gap-2 bg-base-100">
        <Search class="h-3.5 w-3.5 opacity-60" />
        <input
          :value="searchQuery"
          type="text"
          class="w-full bg-transparent outline-none"
          :placeholder="searchPlaceholder"
          @input="emit('update:searchQuery', ($event.target as HTMLInputElement).value)"
        />
      </label>
      <div v-if="showTabs" role="tablist" class="tabs tabs-border shrink-0">
        <button
          type="button"
          role="tab"
          class="tab h-8 px-3"
          :class="activeTab === 'local' ? 'tab-active font-semibold' : ''"
          @click="emit('update:activeTab', 'local')"
        >
          {{ localLabel }}
        </button>
        <button
          type="button"
          role="tab"
          class="tab h-8 px-3"
          :class="activeTab === 'contact' ? 'tab-active font-semibold' : ''"
          @click="emit('update:activeTab', 'contact')"
        >
          {{ contactLabel }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Search } from "@lucide/vue";

defineProps<{
  searchQuery: string;
  activeTab: "local" | "contact";
  searchPlaceholder: string;
  localLabel: string;
  contactLabel: string;
  showTabs?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:searchQuery", value: string): void;
  (e: "update:activeTab", value: "local" | "contact"): void;
}>();
</script>

<template>
  <div class="grid min-w-0 gap-1 overflow-hidden">
    <div v-for="section in sections" :key="section.key" class="grid min-w-0 gap-0.5 overflow-hidden">
      <div
        class="flex min-w-0 items-center gap-1.5 rounded-lg px-1 py-1 hover:bg-base-content/5"
        :class="section.disabled ? 'opacity-60' : ''"
      >
        <button
          type="button"
          class="flex h-4 w-4 shrink-0 cursor-pointer items-center justify-center rounded border transition"
          :class="stateBoxClasses(sectionState(section))"
          :disabled="section.disabled"
          :aria-label="section.label"
          :aria-pressed="sectionState(section) === 'all' ? 'true' : sectionState(section) === 'partial' ? 'mixed' : 'false'"
          @click="emitSectionToggle(section, sectionState(section) !== 'all')"
        >
          <Check v-if="sectionState(section) === 'all'" class="h-3 w-3" stroke-width="3" />
          <Minus v-else-if="sectionState(section) === 'partial'" class="h-3 w-3" stroke-width="3" />
        </button>
        <button
          type="button"
          class="flex min-w-0 flex-1 items-center gap-1.5 rounded px-0.5 py-0.5 text-left"
          :aria-expanded="isExpanded(expandKeySection(section.key))"
          @click="toggleExpand(expandKeySection(section.key))"
        >
          <ChevronDown v-if="isExpanded(expandKeySection(section.key))" class="h-3.5 w-3.5 shrink-0 opacity-50" />
          <ChevronRight v-else class="h-3.5 w-3.5 shrink-0 opacity-50" />
          <span class="truncate text-xs font-semibold text-base-content/70">{{ section.label }}</span>
        </button>
      </div>

      <div v-if="isExpanded(expandKeySection(section.key))" class="grid min-w-0 gap-0.5 overflow-hidden pl-3">
        <div v-for="group in section.groups" :key="group.key" class="grid min-w-0 gap-0.5 overflow-hidden">
          <div
            class="flex min-w-0 items-center gap-1.5 rounded-lg px-1 py-1 hover:bg-base-content/5"
            :class="section.disabled ? 'opacity-60' : ''"
          >
            <button
              type="button"
              class="flex h-4 w-4 shrink-0 cursor-pointer items-center justify-center rounded border transition"
              :class="stateBoxClasses(group.state)"
              :disabled="section.disabled"
              :aria-label="group.label"
              :aria-pressed="group.state === 'all' ? 'true' : group.state === 'partial' ? 'mixed' : 'false'"
              @click="emitGroupToggle(section, group, group.state !== 'all')"
            >
              <Check v-if="group.state === 'all'" class="h-3 w-3" stroke-width="3" />
              <Minus v-else-if="group.state === 'partial'" class="h-3 w-3" stroke-width="3" />
            </button>
            <button
              type="button"
              class="flex min-w-0 flex-1 items-center gap-1.5 rounded px-0.5 py-0.5 text-left"
              :aria-expanded="isExpanded(expandKeyGroup(section.key, group.key))"
              @click="toggleExpand(expandKeyGroup(section.key, group.key))"
            >
              <ChevronDown v-if="isExpanded(expandKeyGroup(section.key, group.key))" class="h-3 w-3 shrink-0 opacity-40" />
              <ChevronRight v-else class="h-3 w-3 shrink-0 opacity-40" />
              <span class="truncate text-xs font-medium text-base-content/70">{{ group.label }}</span>
            </button>
          </div>

          <div v-if="isExpanded(expandKeyGroup(section.key, group.key))" class="grid min-w-0 gap-0.5 overflow-hidden pl-3">
            <button
              v-for="leaf in group.leaves"
              :key="leaf.name"
              type="button"
              class="flex min-w-0 w-full items-center gap-1.5 rounded-lg px-1 py-1 text-left transition hover:bg-base-content/5"
              :class="section.disabled ? 'cursor-not-allowed opacity-60' : 'cursor-pointer'"
              :disabled="section.disabled"
              :aria-pressed="leaf.enabled"
              @click="emitLeafToggle(section, leaf, !leaf.enabled)"
            >
              <span
                class="flex h-4 w-4 shrink-0 items-center justify-center rounded border transition"
                :class="leaf.enabled ? 'border-primary bg-primary text-primary-content' : 'border-base-content/25 bg-transparent text-transparent'"
              >
                <Check v-if="leaf.enabled" class="h-3 w-3" stroke-width="3" />
              </span>
              <span class="min-w-0 shrink-0 text-sm">{{ leaf.displayName }}</span>
              <span v-if="leaf.description" class="min-w-0 flex-1 truncate text-xs opacity-60" :title="leaf.description">
                {{ leaf.description }}
              </span>
            </button>
          </div>
        </div>

        <button
          v-for="leaf in section.leaves"
          :key="leaf.name"
          type="button"
          class="flex min-w-0 w-full items-center gap-1.5 rounded-lg px-1 py-1 text-left transition hover:bg-base-content/5"
          :class="section.disabled ? 'cursor-not-allowed opacity-60' : 'cursor-pointer'"
          :disabled="section.disabled"
          :aria-pressed="leaf.enabled"
          @click="emitLeafToggle(section, leaf, !leaf.enabled)"
        >
          <span
            class="flex h-4 w-4 shrink-0 items-center justify-center rounded border transition"
            :class="leaf.enabled ? 'border-primary bg-primary text-primary-content' : 'border-base-content/25 bg-transparent text-transparent'"
          >
            <Check v-if="leaf.enabled" class="h-3 w-3" stroke-width="3" />
          </span>
          <span class="min-w-0 shrink-0 text-sm">{{ leaf.displayName }}</span>
          <span v-if="leaf.description" class="min-w-0 flex-1 truncate text-xs opacity-60" :title="leaf.description">
            {{ leaf.description }}
          </span>
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { Check, ChevronDown, ChevronRight, Minus } from "@lucide/vue";
import type {
  DepartmentToolGroupState,
  DepartmentToolTreeGroup,
  DepartmentToolTreeLeaf,
  DepartmentToolTreeSection,
} from "../../../utils/department-tool-tree";

defineProps<{
  sections: DepartmentToolTreeSection[];
}>();

const emit = defineEmits<{
  (e: "leafToggle", payload: { category: DepartmentToolTreeLeaf["category"]; name: string; checked: boolean }): void;
  (e: "groupToggle", payload: { category: DepartmentToolTreeLeaf["category"]; names: string[]; checked: boolean }): void;
}>();

const collapsedKeys = ref<Set<string>>(new Set());

function expandKeySection(sectionKey: string) {
  return `sec:${sectionKey}`;
}

function expandKeyGroup(sectionKey: string, groupKey: string) {
  return `grp:${sectionKey}:${groupKey}`;
}

function isExpanded(key: string) {
  return !collapsedKeys.value.has(key);
}

function toggleExpand(key: string) {
  const next = new Set(collapsedKeys.value);
  if (next.has(key)) {
    next.delete(key);
  } else {
    next.add(key);
  }
  collapsedKeys.value = next;
}

function sectionLeaves(section: DepartmentToolTreeSection) {
  return [...section.groups.flatMap((group) => group.leaves), ...section.leaves];
}

function aggregateState(leaves: DepartmentToolTreeLeaf[]): DepartmentToolGroupState {
  const enabledCount = leaves.filter((leaf) => leaf.enabled).length;
  if (enabledCount === 0) return "none";
  if (enabledCount === leaves.length) return "all";
  return "partial";
}

function sectionState(section: DepartmentToolTreeSection) {
  return aggregateState(sectionLeaves(section));
}

function stateBoxClasses(state: DepartmentToolGroupState) {
  if (state === "all") return "border-primary bg-primary text-primary-content";
  if (state === "partial") return "border-primary/40 bg-primary/15 text-primary";
  return "border-base-content/25 bg-transparent text-transparent";
}

function emitLeafToggle(section: DepartmentToolTreeSection, leaf: DepartmentToolTreeLeaf, checked: boolean) {
  if (section.disabled) return;
  emit("leafToggle", { category: leaf.category, name: leaf.name, checked });
}

function emitGroupToggle(section: DepartmentToolTreeSection, group: DepartmentToolTreeGroup, checked: boolean) {
  if (section.disabled) return;
  emit("groupToggle", { category: section.key, names: group.leaves.map((leaf) => leaf.name), checked });
}

function emitSectionToggle(section: DepartmentToolTreeSection, checked: boolean) {
  if (section.disabled) return;
  emit("groupToggle", { category: section.key, names: sectionLeaves(section).map((leaf) => leaf.name), checked });
}
</script>

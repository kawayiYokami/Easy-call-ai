<template>
  <div class="flex min-w-0 items-center gap-0.5 overflow-hidden">
    <button
      type="button"
      class="btn btn-ghost btn-sm h-8 min-h-8 flex-none gap-1.5 overflow-hidden px-2 transition-[max-width,background-color,color] duration-200 ease-out"
      :class="expandedPanel === 'workspace' ? 'w-auto max-w-52 justify-start bg-base-200/80' : 'w-8 max-w-8 justify-center'"
      :disabled="workspaceButtonDisabled"
      :title="workspaceTitle"
      @click="handleWorkspaceClick"
    >
      <SquareTerminal class="size-3.5 shrink-0" aria-hidden="true" />
      <Transition name="wdc-content">
        <span v-if="expandedPanel === 'workspace'" class="truncate text-xs">
          {{ workspaceButtonName || workspaceButtonLabel }}
        </span>
      </Transition>
    </button>

    <div class="ml-auto flex min-w-0 flex-none items-center gap-0.5">
      <button
        type="button"
        class="btn btn-ghost btn-sm h-8 min-h-8 flex-none gap-1.5 overflow-hidden px-2 transition-[max-width,background-color,color] duration-200 ease-out"
        :class="expandedPanel === 'delegate' ? 'w-auto max-w-[min(21rem,52vw)] justify-start bg-base-200/80' : 'w-8 max-w-8 justify-center'"
        :disabled="delegateCount <= 0"
        :title="delegateTitle"
        @click="handleDelegateClick"
      >
        <span class="indicator shrink-0">
          <span
            v-if="runningCount > 0"
            class="indicator-item indicator-top indicator-end h-2.5 w-2.5 rounded-full bg-success"
          ></span>
          <Network class="size-3.5 shrink-0" :class="delegateCount > 0 ? 'text-base-content/70' : 'text-base-content/40'" aria-hidden="true" />
        </span>

        <Transition name="wdc-content">
          <span v-if="expandedPanel === 'delegate'" class="flex min-w-0 items-center gap-1.5 overflow-hidden">
            <span class="shrink-0 text-xs font-semibold tabular-nums">{{ delegateCount }} 委托</span>
            <span class="h-4 w-px shrink-0 bg-base-300"></span>
            <span class="flex min-w-0 items-center gap-2 overflow-hidden text-xs text-base-content/75">
              <span class="inline-flex min-w-0 items-center gap-1" title="所有当前委托累计用时">
                <Timer class="size-3.5 shrink-0 text-base-content/45" aria-hidden="true" />
                <span class="truncate tabular-nums">{{ elapsedText }}</span>
              </span>
              <span class="inline-flex min-w-0 items-center gap-1" title="所有当前委托累计请求步数">
                <Footprints class="size-3.5 shrink-0 text-base-content/45" aria-hidden="true" />
                <span class="truncate tabular-nums">{{ requestCount }}步</span>
              </span>
              <span class="inline-flex min-w-0 items-center gap-1" title="所有当前委托累计词元">
                <Coins class="size-3.5 shrink-0 text-base-content/45" aria-hidden="true" />
                <span class="truncate tabular-nums">{{ tokenText }}词元</span>
              </span>
            </span>
          </span>
        </Transition>
      </button>

      <Transition name="wdc-content">
        <PanelRightOpen
          v-if="expandedPanel === 'delegate' && delegateCount > 0"
          class="size-3.5 flex-none text-base-content/45"
          aria-hidden="true"
        />
      </Transition>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Coins, Footprints, Network, PanelRightOpen, SquareTerminal, Timer } from "@lucide/vue";
import type { ConversationDelegateStatusSummary } from "../../../types/app";

const props = defineProps<{
  workspaceButtonLabel: string;
  workspaceButtonName: string;
  workspaceButtonDisabled?: boolean;
  delegates: ConversationDelegateStatusSummary[];
}>();

const emit = defineEmits<{
  lockWorkspace: [];
  openDelegateSummary: [];
}>();

const expandedPanel = ref<"workspace" | "delegate">("workspace");
const normalizedDelegates = computed(() => Array.isArray(props.delegates) ? props.delegates : []);
const runningDelegates = computed(() => normalizedDelegates.value.filter(isDelegateRunning));
const displayedDelegates = computed(() => runningDelegates.value.length > 0 ? runningDelegates.value : normalizedDelegates.value);
const delegateCount = computed(() => displayedDelegates.value.length);
const runningCount = computed(() => runningDelegates.value.length);
const elapsedMs = computed(() => sumBy(displayedDelegates.value, (delegate) => delegate.elapsedMs));
const requestCount = computed(() => sumBy(displayedDelegates.value, (delegate) => delegate.requestCount));
const tokenCount = computed(() => sumBy(displayedDelegates.value, (delegate) => delegate.tokenCount));
const elapsedText = computed(() => formatElapsedMs(elapsedMs.value));
const tokenText = computed(() => formatTokenK(tokenCount.value));
const workspaceTitle = computed(() => props.workspaceButtonName || props.workspaceButtonLabel);
const delegateTitle = computed(() => {
  if (delegateCount.value <= 0) return "当前暂无委托";
  if (runningCount.value > 0) return `查看 ${runningCount.value} 个运行中委托`;
  return `查看 ${delegateCount.value} 个委托`;
});

watch(
  runningCount,
  (count, previousCount) => {
    if (count > 0 && (!previousCount || previousCount <= 0)) {
      expandedPanel.value = "delegate";
      return;
    }
    if (count <= 0) {
      expandedPanel.value = "workspace";
    }
  },
  { immediate: true },
);

function handleWorkspaceClick() {
  if (expandedPanel.value !== "workspace") {
    expandedPanel.value = "workspace";
    return;
  }
  emit("lockWorkspace");
}

function handleDelegateClick() {
  if (delegateCount.value <= 0) return;
  if (expandedPanel.value !== "delegate") {
    expandedPanel.value = "delegate";
    return;
  }
  emit("openDelegateSummary");
}

function sumBy(
  delegates: ConversationDelegateStatusSummary[],
  read: (delegate: ConversationDelegateStatusSummary) => number | undefined | null,
) {
  return delegates.reduce((sum, delegate) => {
    const value = Number(read(delegate) ?? 0);
    return sum + (Number.isFinite(value) && value > 0 ? value : 0);
  }, 0);
}

function isDelegateRunning(delegate: ConversationDelegateStatusSummary) {
  const status = String(delegate.status || "").trim();
  return delegate.active && (status === "running" || status === "delivered");
}

function formatTokenK(value: number) {
  if (!Number.isFinite(value) || value <= 0) return "0K";
  const k = value / 1000;
  if (k < 10) return `${k.toFixed(1)}K`;
  return `${Math.round(k)}K`;
}

function formatElapsedMs(value: number) {
  if (!Number.isFinite(value) || value <= 0) return "0秒";
  const totalSeconds = Math.floor(value / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  if (hours > 0) return `${hours}时${minutes}分`;
  if (minutes > 0) return `${minutes}分${seconds}秒`;
  return `${seconds}秒`;
}
</script>

<style scoped>
.wdc-content-enter-active,
.wdc-content-leave-active {
  transition: opacity 120ms ease, transform 120ms ease;
}

.wdc-content-enter-from,
.wdc-content-leave-to {
  opacity: 0;
  transform: translateX(-4px);
}
</style>

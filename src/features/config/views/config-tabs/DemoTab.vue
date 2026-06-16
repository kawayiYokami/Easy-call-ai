<template>
  <div class="grid h-full gap-3 overflow-y-auto pr-1">
    <div class="card border border-base-300 bg-base-100">
      <div class="card-body gap-3 p-4">
        <div class="space-y-1">
          <h3 class="card-title text-base">{{ t("config.demo.nativeNotificationTitle") }}</h3>
          <p class="text-sm text-base-content/70">
            {{ t("config.demo.nativeNotificationSummary") }}
          </p>
          <p class="text-xs text-base-content/60">
            {{ t("config.demo.nativeNotificationDevHint") }}
          </p>
        </div>

        <div class="flex flex-wrap items-center gap-3">
          <button
            type="button"
            class="btn btn-primary"
            :disabled="sending"
            @click="sendNativeNotification"
          >
            {{ sending ? t("config.demo.sending") : t("config.demo.sendNativeNotification") }}
          </button>
          <span class="text-xs text-base-content/60">{{ t("config.demo.backgroundHint") }}</span>
        </div>

        <div v-if="errorText" class="alert alert-error text-sm">
          <span>{{ errorText }}</span>
        </div>

        <div v-else-if="resultText" class="alert alert-success text-sm whitespace-pre-wrap">
          <span>{{ resultText }}</span>
        </div>
      </div>
    </div>

    <div class="card border border-base-300 bg-base-100">
      <div class="card-body gap-3 p-4">
        <div class="space-y-1">
          <h3 class="card-title text-base">{{ t("config.demo.restartTitle") }}</h3>
          <p class="text-sm text-base-content/70">
            {{ t("config.demo.restartSummary") }}
          </p>
        </div>

        <div class="flex flex-wrap items-center gap-3">
          <button
            type="button"
            class="btn btn-warning"
            :disabled="restarting"
            @click="restartApp"
          >
            <RotateCcw class="size-4" aria-hidden="true" />
            {{ restarting ? t("config.demo.restarting") : t("config.demo.restartApp") }}
          </button>
        </div>
      </div>
    </div>

    <div class="card border border-base-300 bg-base-100">
      <div class="card-body gap-3 p-4">
        <div class="space-y-1">
          <h3 class="card-title text-base">后端内存快照</h3>
          <p class="text-sm text-base-content/70">
            调用后端调试命令，查看会话缓存、message_store 缓存和其他长生命周期状态的占用概况。
          </p>
        </div>

        <div class="flex flex-wrap items-center gap-3">
          <button
            type="button"
            class="btn btn-secondary"
            :disabled="loadingMemoryStats"
            @click="loadMemoryStats"
          >
            {{ loadingMemoryStats ? "查询中..." : "查询后端内存" }}
          </button>
        </div>

        <div v-if="memoryStatsText" class="mockup-code max-h-96 overflow-auto text-xs">
          <pre class="whitespace-pre-wrap break-all"><code>{{ memoryStatsText }}</code></pre>
        </div>
      </div>
    </div>

    <div class="card border border-base-300 bg-base-100">
      <div class="card-body gap-3 p-4">
        <div class="space-y-1">
          <h3 class="card-title text-base">DelegateProgressLine 预览</h3>
          <p class="text-sm text-base-content/70">折叠卡片第二行的实时进度组件样本。</p>
        </div>
        <div class="flex flex-wrap items-center gap-2">
          <button type="button" class="btn btn-xs" @click="toggleDemoDelegateActivity">
            {{ demoHasRunningDelegates ? "结束活动委托" : "恢复活动委托" }}
          </button>
        </div>
        <div class="flex w-full max-w-2xl">
          <SessionControlPanel
            class="flex-1"
            workspace-button-label="工作空间"
            workspace-button-name="easy_call_ai"
            :delegates="demoDelegateStatuses"
          />
        </div>
        <div class="flex flex-col gap-1 py-2">
          <DelegateCard
            title="示例：代码审查（pending）"
            :running="true"
            :elapsed-ms="demoDelegateStatuses[0]?.elapsedMs"
            :request-count="demoDelegateStatuses[0]?.requestCount"
            :token-count="demoDelegateStatuses[0]?.tokenCount"
            last-tool-name="apply_patch"
          />
          <DelegateCard
            title="示例：委托任务（运行中）"
            :running="true"
            :elapsed-ms="demoDelegateStatuses[1]?.elapsedMs"
            :request-count="demoDelegateStatuses[1]?.requestCount"
            :token-count="demoDelegateStatuses[1]?.tokenCount"
            last-tool-name="shell_exec"
          />
          <DelegateCard
            title="示例：审查报告（完成）"
            text="整体判定：正确，置信度 0.92"
          />
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { RotateCcw } from "@lucide/vue";
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { invokeTauri } from "../../../../services/tauri-api";
import DelegateCard from "../../../chat/components/DelegateCard.vue";
import SessionControlPanel from "../../../chat/components/SessionControlPanel.vue";
import type { ConversationDelegateStatusSummary } from "../../../../types/app";

type NativeNotificationDemoResult = {
  permissionBefore: string;
  permissionAfter: string;
  title: string;
  body: string;
  sentAt: string;
};

const sending = ref(false);
const restarting = ref(false);
const loadingMemoryStats = ref(false);
const errorText = ref("");
const resultText = ref("");
const memoryStatsText = ref("");
const { t } = useI18n();
const demoDelegateStatuses = ref<ConversationDelegateStatusSummary[]>([
  createDemoDelegateStatus("demo-code-review", "示例：代码审查（pending）", 45000, 12, 15600, "apply_patch"),
  createDemoDelegateStatus("demo-research", "示例：委托任务（运行中）", 120000, 34, 52800, "shell_exec"),
  {
    ...createDemoDelegateStatus("demo-report", "示例：审查报告（完成）", 347000, 18, 23600, ""),
    status: "completed",
    active: false,
    completedAt: new Date().toISOString(),
  },
]);
let delegateDemoTimer = 0;
const demoHasRunningDelegates = computed(() => demoDelegateStatuses.value.some((delegate) => delegate.active));

async function sendNativeNotification() {
  sending.value = true;
  errorText.value = "";
  resultText.value = "";

  try {
    const result = await invokeTauri<NativeNotificationDemoResult>("demo_send_native_notification");
    resultText.value = [
      t("config.demo.nativeNotificationSent"),
      `title: ${result.title}`,
      `permissionBefore: ${result.permissionBefore}`,
      `permissionAfter: ${result.permissionAfter}`,
      `sentAt: ${result.sentAt}`,
    ].join("\n");
  } catch (error) {
    errorText.value = error instanceof Error ? error.message : String(error);
  } finally {
    sending.value = false;
  }
}

async function restartApp() {
  restarting.value = true;
  errorText.value = "";
  resultText.value = "";

  try {
    await invokeTauri<void>("demo_restart_app");
    resultText.value = t("config.demo.restartRequested");
  } catch (error) {
    errorText.value = error instanceof Error ? error.message : String(error);
    restarting.value = false;
  }
}

async function loadMemoryStats() {
  loadingMemoryStats.value = true;
  errorText.value = "";
  memoryStatsText.value = "";

  try {
    const result = await invokeTauri<unknown>("dump_memory_cache_stats");
    memoryStatsText.value = JSON.stringify(result, null, 2);
  } catch (error) {
    errorText.value = error instanceof Error ? error.message : String(error);
  } finally {
    loadingMemoryStats.value = false;
  }
}

function createDemoDelegateStatus(
  delegateId: string,
  title: string,
  elapsedMs: number,
  requestCount: number,
  tokenCount: number,
  lastToolName: string,
): ConversationDelegateStatusSummary {
  const now = new Date().toISOString();
  return {
    delegateId,
    conversationId: `${delegateId}-conversation`,
    rootConversationId: "demo-root-conversation",
    title,
    status: "running",
    active: true,
    startedAt: now,
    updatedAt: now,
    elapsedMs,
    requestCount,
    toolCallCount: requestCount,
    lastToolName,
    tokenCount,
    targetAgentId: "demo-agent",
  };
}

function advanceDemoDelegateStatus() {
  demoDelegateStatuses.value = demoDelegateStatuses.value.map((delegate, index) => {
    if (!delegate.active) return delegate;
    const nextStep = index === 0 ? 1 : 2;
    const nextToken = index === 0 ? 680 : 1340;
    return {
      ...delegate,
      elapsedMs: delegate.elapsedMs + 1000,
      requestCount: delegate.requestCount + nextStep,
      toolCallCount: delegate.toolCallCount + nextStep,
      tokenCount: delegate.tokenCount + nextToken,
      updatedAt: new Date().toISOString(),
    };
  });
}

function toggleDemoDelegateActivity() {
  const nextActive = !demoHasRunningDelegates.value;
  demoDelegateStatuses.value = demoDelegateStatuses.value.map((delegate, index) => {
    if (index > 1) return delegate;
    return {
      ...delegate,
      status: nextActive ? "running" : "completed",
      active: nextActive,
      completedAt: nextActive ? undefined : new Date().toISOString(),
      updatedAt: new Date().toISOString(),
    };
  });
}

onMounted(() => {
  delegateDemoTimer = window.setInterval(advanceDemoDelegateStatus, 1000);
});

onBeforeUnmount(() => {
  if (!delegateDemoTimer) return;
  window.clearInterval(delegateDemoTimer);
  delegateDemoTimer = 0;
});
</script>

import { onBeforeUnmount, reactive } from "vue";
import { invokeTauri } from "../../../services/tauri-api";

export type MessageStoreMigrationGateMode =
  | "idle"
  | "waiting"
  | "migrating"
  | "completed"
  | "error";

type MessageStoreMigrationRuntimeStatus = {
  status: "idle" | "waitingStart" | "running" | "completed" | "failed";
  stage?: string | null;
  current: number;
  total: number;
  migratedCount: number;
  discardedCount: number;
  conversationId?: string | null;
  conversationTitle?: string | null;
  detail?: string | null;
};

export type MessageStoreMigrationGateBindings = {
  formatRequestFailed: (error: unknown) => string;
};

const MIGRATION_STATUS_POLL_INTERVAL_MS = 500;

const MIGRATION_STAGE_LABELS: Record<string, string> = {
  v1_to_v2: "第 1 阶段",
  v2_to_v3: "第 2 阶段",
  v3_to_v4: "第 3 阶段",
  usage_trail: "用量数据",
};

function stageLabel(stage?: string | null): string {
  const key = String(stage || "").trim();
  return MIGRATION_STAGE_LABELS[key] || key || "迁移";
}

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export function useMessageStoreMigrationGate(bindings: MessageStoreMigrationGateBindings) {
  const messageStoreMigration = reactive<{
    visible: boolean;
    mode: MessageStoreMigrationGateMode;
    message: string;
    current: number;
    total: number;
    migratedCount: number;
    discardedCount: number;
  }>({
    visible: false,
    mode: "idle",
    message: "",
    current: 0,
    total: 0,
    migratedCount: 0,
    discardedCount: 0,
  });

  let pollingActive = true;

  function applyRuntimeStatus(status: MessageStoreMigrationRuntimeStatus) {
    messageStoreMigration.current = Number(status.current || 0);
    messageStoreMigration.total = Number(status.total || 0);
    messageStoreMigration.migratedCount = Number(status.migratedCount || 0);
    messageStoreMigration.discardedCount = Number(status.discardedCount || 0);
    const title = String(status.conversationTitle || status.conversationId || "").trim();
    switch (status.status) {
      case "idle":
        messageStoreMigration.mode = "idle";
        messageStoreMigration.message = "";
        break;
      case "waitingStart":
        messageStoreMigration.mode = "waiting";
        messageStoreMigration.message = "正在准备迁移会话消息仓库...";
        break;
      case "running":
        messageStoreMigration.mode = "migrating";
        messageStoreMigration.message = `正在迁移（${stageLabel(status.stage)}）：${title || "会话"}`;
        break;
      case "completed":
        messageStoreMigration.mode = "completed";
        messageStoreMigration.message =
          String(status.detail || "").trim() || "会话消息仓库迁移完成";
        break;
      case "failed":
        messageStoreMigration.mode = "error";
        messageStoreMigration.message =
          String(status.detail || "").trim() || "会话消息仓库迁移失败";
        break;
    }
    messageStoreMigration.visible = messageStoreMigration.mode !== "idle";
  }

  async function pollRuntimeStatus(): Promise<string> {
    const status = await invokeTauri<MessageStoreMigrationRuntimeStatus>(
      "messageStore.migration.status",
    );
    applyRuntimeStatus(status);
    return String(status.status || "idle");
  }

  async function ensureMessageStoreMigrationGate() {
    while (pollingActive) {
      let state = "idle";
      try {
        state = await pollRuntimeStatus();
      } catch (error) {
        messageStoreMigration.visible = true;
        messageStoreMigration.mode = "error";
        messageStoreMigration.message = bindings.formatRequestFailed(error);
        state = "transient-error";
      }
      // idle：无需迁移直接放行；completed：应用在面板背后继续加载
      if (state === "idle" || state === "completed") {
        return;
      }
      // failed 保持轮询：用户点重试后状态回到 running，循环自然接续
      await delay(MIGRATION_STATUS_POLL_INTERVAL_MS);
    }
  }

  function confirmMessageStoreMigrationSummary() {
    if (messageStoreMigration.mode !== "completed") return;
    messageStoreMigration.visible = false;
    pollingActive = false;
  }

  async function retryMessageStoreMigration() {
    try {
      await invokeTauri("messageStore.migration.run", {});
      messageStoreMigration.mode = "waiting";
      messageStoreMigration.message = "正在准备迁移会话消息仓库...";
    } catch (error) {
      messageStoreMigration.mode = "error";
      messageStoreMigration.message = bindings.formatRequestFailed(error);
    }
  }

  onBeforeUnmount(() => {
    pollingActive = false;
  });

  return {
    messageStoreMigration,
    ensureMessageStoreMigrationGate,
    confirmMessageStoreMigrationSummary,
    retryMessageStoreMigration,
  };
}

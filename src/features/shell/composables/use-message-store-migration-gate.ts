import { onBeforeUnmount, reactive } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { invokeTauri } from "../../../services/tauri-api";

export type MessageStoreMigrationGateMode = "idle" | "checking" | "migrating" | "blocked" | "error";

type MessageStoreMigrationPreflightItem = {
  conversationId: string;
  title: string;
  status: string;
  messageCount: number;
  reason?: string | null;
};

type MessageStoreMigrationPreflightReport = {
  totalConversations: number;
  readyCount: number;
  legacyCount: number;
  busyCount?: number;
  blockedCount: number;
  canAutoMigrate: boolean;
  items: MessageStoreMigrationPreflightItem[];
};

type MessageStoreMigrationProgressPayload = {
  current: number;
  total: number;
  conversationId: string;
  title: string;
  status: string;
  detail?: string | null;
};

export type MessageStoreMigrationGateBindings = {
  formatRequestFailed: (error: unknown) => string;
};

export function useMessageStoreMigrationGate(bindings: MessageStoreMigrationGateBindings) {
  const messageStoreMigration = reactive<{
    visible: boolean;
    mode: MessageStoreMigrationGateMode;
    message: string;
    current: number;
    total: number;
    blockedItems: MessageStoreMigrationPreflightItem[];
  }>({
    visible: false,
    mode: "idle",
    message: "",
    current: 0,
    total: 0,
    blockedItems: [],
  });

  let messageStoreMigrationResolve: (() => void) | null = null;
  let messageStoreMigrationReject: ((error: Error) => void) | null = null;
  let messageStoreMigrationProgressUnlisten: UnlistenFn | null = null;

  function resetMessageStoreMigrationGate() {
    messageStoreMigration.visible = false;
    messageStoreMigration.mode = "idle";
    messageStoreMigration.message = "";
    messageStoreMigration.current = 0;
    messageStoreMigration.total = 0;
    messageStoreMigration.blockedItems = [];
  }

  async function ensureMessageStoreMigrationProgressListener() {
    if (messageStoreMigrationProgressUnlisten) return;
    messageStoreMigrationProgressUnlisten = await listen<MessageStoreMigrationProgressPayload>(
      "easy-call:message-store-migration-progress",
      (event) => {
        const payload = event.payload;
        messageStoreMigration.visible = true;
        messageStoreMigration.mode = payload.status === "failed" ? "error" : "migrating";
        messageStoreMigration.current = Number(payload.current || 0);
        messageStoreMigration.total = Number(payload.total || 0);
        const title = String(payload.title || payload.conversationId || "").trim();
        const detail = String(payload.detail || "").trim();
        messageStoreMigration.message = detail || `正在迁移：${title || "会话"}`;
      },
    );
  }

  async function runMessageStoreMigrationFromGate(discardInvalid: boolean) {
    await ensureMessageStoreMigrationProgressListener();
    messageStoreMigration.visible = true;
    messageStoreMigration.mode = "migrating";
    messageStoreMigration.message = discardInvalid
      ? "正在备份异常会话并继续迁移..."
      : "正在迁移会话消息仓库...";
    await invokeTauri("run_message_store_migration", {
      input: { discardInvalid },
    });
    resetMessageStoreMigrationGate();
  }

  async function ensureMessageStoreMigrationGate() {
    await ensureMessageStoreMigrationProgressListener();
    const report = await invokeTauri<MessageStoreMigrationPreflightReport>(
      "check_message_store_migration",
    );
    if (report.blockedCount > 0) {
      messageStoreMigration.visible = true;
      messageStoreMigration.mode = "blocked";
      messageStoreMigration.blockedItems = report.items.filter((item) => item.status === "blocked");
      messageStoreMigration.message = `发现 ${report.blockedCount} 个异常会话。需要确认是否抛弃异常会话并继续迁移。`;
      return await new Promise<void>((resolve, reject) => {
        messageStoreMigrationResolve = resolve;
        messageStoreMigrationReject = reject;
      });
    }
    if (report.legacyCount > 0) {
      messageStoreMigration.visible = true;
      messageStoreMigration.mode = "checking";
      messageStoreMigration.message = `发现 ${report.legacyCount} 个旧会话，正在迁移...`;
      await runMessageStoreMigrationFromGate(false);
      return;
    }
  }

  function cancelMessageStoreMigration() {
    const error = new Error("用户取消会话消息仓库迁移，启动已暂停。");
    messageStoreMigration.mode = "error";
    messageStoreMigration.message = error.message;
    messageStoreMigrationReject?.(error);
    messageStoreMigrationResolve = null;
    messageStoreMigrationReject = null;
  }

  async function continueMessageStoreMigrationWithDiscard() {
    try {
      await runMessageStoreMigrationFromGate(true);
      messageStoreMigrationResolve?.();
    } catch (error) {
      messageStoreMigration.mode = "error";
      messageStoreMigration.message = bindings.formatRequestFailed(error);
      messageStoreMigrationReject?.(error instanceof Error ? error : new Error(String(error)));
    } finally {
      messageStoreMigrationResolve = null;
      messageStoreMigrationReject = null;
    }
  }

  onBeforeUnmount(() => {
    if (messageStoreMigrationProgressUnlisten) {
      messageStoreMigrationProgressUnlisten();
      messageStoreMigrationProgressUnlisten = null;
    }
  });

  return {
    messageStoreMigration,
    ensureMessageStoreMigrationGate,
    cancelMessageStoreMigration,
    continueMessageStoreMigrationWithDiscard,
  };
}

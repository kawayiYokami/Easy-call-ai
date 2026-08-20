import { decideForegroundRecovery } from "./foreground-recovery-decision";
import type { AssistantStreamBlock } from "../../../types/app";

/**
 * 焦点恢复的唯一状态机。
 *
 * 它只判定并恢复当前流投影：运行中绝不改写消息列表；只有确认已结束才交给宿主刷新
 * 目标消息或检查正式消息尾部。桌面、Web 和 IDE 宿主只能提供 I/O 适配，不能另写对账分支。
 */
export type ForegroundStreamIdentity = {
  activationId?: string;
  requestId?: string;
  updatedAt?: string;
  persistedAssistantMessageId?: string;
  /** 以下字段与后端 ConversationStreamRuntimeCacheSnapshot 对齐（诊断与恢复使用）。 */
  assistantText?: string;
  toolStatusText?: string;
  toolStatusState?: string;
  streamBlocks?: AssistantStreamBlock[];
  hasVisibleProgress?: boolean;
};

export type ForegroundRuntimeSnapshot = {
  runtimeState?: string;
  isProcessing?: boolean;
  hasPendingQueue?: boolean;
  pendingQueueCount?: number;
  streamCache?: ForegroundStreamIdentity | null;
};

export type ForegroundRecoveryInput = {
  conversationId: string;
  runtimeSnapshot: ForegroundRuntimeSnapshot;
  frontendStreaming: boolean;
  frontendMessageId?: string;
  frontendActivationId?: string;
  frontendRequestId?: string;
  frontendRevision?: string;
};

export type ForegroundRecoveryDependencies = {
  probeStream: (conversationId: string) => Promise<boolean>;
  resumeSubscription: (conversationId: string) => Promise<ForegroundRuntimeSnapshot | null>;
  applyRuntimeSnapshot: (runtimeSnapshot: ForegroundRuntimeSnapshot) => boolean | Promise<boolean>;
  refreshMessageById: (conversationId: string, messageId: string) => Promise<boolean>;
  finalizeMessage: (messageId: string) => void | Promise<void>;
  /** 后台整理/压缩没有可恢复的 assistant 正式消息；只更新忙态。 */
  applyBackgroundBusy?: (runtimeSnapshot: ForegroundRuntimeSnapshot) => void | Promise<void>;
};

export type ForegroundRecoveryOutcome = "handled" | "check_freshness" | "reload_conversation";

/**
 * 宿主无关的前台尾部对账入口。桌面、Web 和侧栏只能注入原子读取与 UI 适配，
 * 不得各自解释运行态后再写一份 freshness 回退。
 */
export async function reconcileForegroundRuntime(
  input: ForegroundRecoveryInput,
  dependencies: ForegroundRecoveryDependencies & {
    isCurrent: () => boolean;
    currentFormalTailMessageId: () => string;
    requestLatestFormalTailMessageId: (conversationId: string) => Promise<string>;
    reloadConversation: () => Promise<void>;
    /** 没有水位变化时不读取 freshness/messageById。保留默认值以兼容旧宿主。 */
    shouldReconcileTail?: () => boolean;
  },
): Promise<"handled" | "tail_reconciled" | "reloaded" | "stale"> {
  const outcome = await recoverForegroundStreaming(input, dependencies);
  if (!dependencies.isCurrent()) return "stale";
  if (outcome === "handled") return "handled";
  if (outcome === "reload_conversation") {
    await dependencies.reloadConversation();
    return dependencies.isCurrent() ? "reloaded" : "stale";
  }

  if (dependencies.shouldReconcileTail && !dependencies.shouldReconcileTail()) {
    console.warn("[焦点恢复][尾部对账] 跳过", {
      conversationId: input.conversationId,
      reason: "shouldReconcileTail=false（freshness 指纹未变化或非本会话待对账）",
    });
    return "handled";
  }

  let latestTailId = "";
  try {
    latestTailId = await dependencies.requestLatestFormalTailMessageId(input.conversationId);
  } catch {
    console.warn("[焦点恢复][尾部对账] freshness 查询失败，回退重载", {
      conversationId: input.conversationId,
    });
    await dependencies.reloadConversation();
    return dependencies.isCurrent() ? "tail_reconciled" : "stale";
  }
  if (!dependencies.isCurrent()) return "stale";
  // 水位推进代表服务端的同一条正式消息也可能已经从半截变为终态；
  // 因而即使 ID 相同也必须以 messageById 的权威 contentBlocks 覆盖。
  console.warn("[焦点恢复][尾部对账] freshness 结果", {
    conversationId: input.conversationId,
    latestTailId: latestTailId || "(空)",
    localTailId: dependencies.currentFormalTailMessageId() || "(空)",
    shouldReconcileTail: dependencies.shouldReconcileTail?.(),
  });
  if (!latestTailId) return dependencies.shouldReconcileTail ? "tail_reconciled" : "handled";
  try {
    if (await dependencies.refreshMessageById(input.conversationId, latestTailId)) {
      console.warn("[焦点恢复][尾部对账] messageById 覆盖成功", {
        conversationId: input.conversationId,
        latestTailId,
      });
      return dependencies.shouldReconcileTail ? "tail_reconciled" : "handled";
    }
    console.warn("[焦点恢复][尾部对账] messageById 未命中，回退重载", {
      conversationId: input.conversationId,
      latestTailId,
    });
  } catch {
    console.warn("[焦点恢复][尾部对账] messageById 查询失败，回退轻量快照", {
      conversationId: input.conversationId,
      latestTailId,
    });
    // 单条原子读取失败时和未找到时一样，回退到既有轻量快照。
  }
  await dependencies.reloadConversation();
  return dependencies.isCurrent()
    ? (dependencies.shouldReconcileTail ? "tail_reconciled" : "reloaded")
    : "stale";
}

function normalized(value: unknown): string {
  return String(value || "").trim();
}

export type ForegroundRuntimeKind = "idle" | "assistant_streaming" | "background_busy";

export function classifyForegroundRuntime(snapshot: ForegroundRuntimeSnapshot): ForegroundRuntimeKind {
  const state = normalized(snapshot.runtimeState);
  if (state === "assistant_streaming" && !!normalized(snapshot.streamCache?.persistedAssistantMessageId)) {
    return "assistant_streaming";
  }
  if (state === "organizing_context"
    || state === "compacting"
    || !!snapshot.isProcessing
    || !!snapshot.hasPendingQueue
    || Math.max(0, Number(snapshot.pendingQueueCount || 0)) > 0) {
    return "background_busy";
  }
  return "idle";
}

function targetMessageId(snapshot: ForegroundRuntimeSnapshot, fallbackMessageId?: string): string {
  return normalized(snapshot.streamCache?.persistedAssistantMessageId || fallbackMessageId);
}

function decide(
  input: ForegroundRecoveryInput,
  snapshot: ForegroundRuntimeSnapshot,
  probeState: "unknown" | "healthy" | "unhealthy",
) {
  return decideForegroundRecovery({
    backendStreaming: classifyForegroundRuntime(snapshot) === "assistant_streaming",
    frontendStreaming: input.frontendStreaming,
    backendMessageId: snapshot.streamCache?.persistedAssistantMessageId,
    frontendMessageId: input.frontendMessageId,
    backendActivationId: snapshot.streamCache?.activationId,
    frontendActivationId: input.frontendActivationId,
    backendRequestId: snapshot.streamCache?.requestId,
    frontendRequestId: input.frontendRequestId,
    backendRevision: snapshot.streamCache?.updatedAt,
    frontendRevision: input.frontendRevision,
    probeState,
  });
}

export async function recoverForegroundStreaming(
  input: ForegroundRecoveryInput,
  dependencies: ForegroundRecoveryDependencies,
): Promise<ForegroundRecoveryOutcome> {
  const traceDecision = (stage: string, detail: Record<string, unknown>) => {
    console.warn(`[焦点恢复][${stage}] 判断理由`, {
      conversationId: input.conversationId,
      ...detail,
    });
  };
  const initialKind = classifyForegroundRuntime(input.runtimeSnapshot);
  traceDecision("初始分类", {
    runtimeState: String(input.runtimeSnapshot.runtimeState || ""),
    kind: initialKind,
    backendMessageId: input.runtimeSnapshot.streamCache?.persistedAssistantMessageId,
    frontendStreaming: input.frontendStreaming,
  });
  if (initialKind === "background_busy") {
    traceDecision("命中 background_busy", { action: "finalize + applyBackgroundBusy", frontendMessageId: input.frontendMessageId });
    if (input.frontendMessageId) await dependencies.finalizeMessage(input.frontendMessageId);
    await dependencies.applyBackgroundBusy?.(input.runtimeSnapshot);
    return "handled";
  }
  let action = decide(input, input.runtimeSnapshot, "unknown");
  traceDecision("初判", {
    action,
    probeState: "unknown",
    backendStreaming: classifyForegroundRuntime(input.runtimeSnapshot) === "assistant_streaming",
    frontendStreaming: input.frontendStreaming,
    backendMessageId: input.runtimeSnapshot.streamCache?.persistedAssistantMessageId || "(空)",
    frontendMessageId: input.frontendMessageId || "(空)",
  });
  if (action === "probe_stream") {
    const probeHealthy = await dependencies.probeStream(input.conversationId);
    traceDecision("测活完成", { probeHealthy });
    action = decide(input, input.runtimeSnapshot, probeHealthy ? "healthy" : "unhealthy");
    traceDecision("测活后重判", { action, probeState: probeHealthy ? "healthy" : "unhealthy" });
  }

  if (action === "keep") {
    const keepOutcome = input.frontendStreaming || classifyForegroundRuntime(input.runtimeSnapshot) === "assistant_streaming"
      ? "handled"
      : "check_freshness";
    traceDecision("决策 keep", {
      outcome: keepOutcome,
      frontendStreaming: input.frontendStreaming,
      runtimeState: String(input.runtimeSnapshot.runtimeState || ""),
      kind: classifyForegroundRuntime(input.runtimeSnapshot),
    });
    return keepOutcome;
  }

  if (action === "refresh_target_message") {
    const messageId = targetMessageId(input.runtimeSnapshot, input.frontendMessageId);
    traceDecision("决策 refresh_target_message", { messageId });
    if (!messageId || !await dependencies.refreshMessageById(input.conversationId, messageId)) {
      traceDecision("refresh 失败", { outcome: "reload_conversation" });
      return "reload_conversation";
    }
    await dependencies.finalizeMessage(messageId);
    return "handled";
  }

  if (action !== "resume_stream") {
    traceDecision("决策非 resume_stream", { action, outcome: "reload_conversation" });
    return "reload_conversation";
  }
  traceDecision("决策 resume_stream", { outcome: "开始重建订阅" });
  const resumedSnapshot = await dependencies.resumeSubscription(input.conversationId);
  const effectiveSnapshot = resumedSnapshot || input.runtimeSnapshot;
  const messageId = targetMessageId(effectiveSnapshot, input.frontendMessageId);
  const resumedKind = classifyForegroundRuntime(effectiveSnapshot);
  traceDecision("重建订阅后", { resumedKind, messageId, hasResumedSnapshot: !!resumedSnapshot });
  if (resumedKind === "background_busy") {
    traceDecision("重建后仍 busy", { action: "finalize + applyBackgroundBusy" });
    if (input.frontendMessageId) await dependencies.finalizeMessage(input.frontendMessageId);
    await dependencies.applyBackgroundBusy?.(effectiveSnapshot);
    return "handled";
  }
  if (resumedKind === "assistant_streaming") {
    if (!messageId) {
      traceDecision("流式但无消息ID", { outcome: "reload_conversation" });
      return "reload_conversation";
    }
    const probeHealthy = await dependencies.probeStream(input.conversationId);
    traceDecision("流式后测活", { probeHealthy });
    if (!probeHealthy) {
      traceDecision("流式后测活失败", { outcome: "reload_conversation" });
      return "reload_conversation";
    }
    const applied = await dependencies.applyRuntimeSnapshot(effectiveSnapshot);
    traceDecision("应用快照", { applied, outcome: applied ? "handled" : "reload_conversation" });
    return applied ? "handled" : "reload_conversation";
  }

  if (!messageId || !await dependencies.refreshMessageById(input.conversationId, messageId)) {
    traceDecision("非流式无消息ID", { outcome: "reload_conversation" });
    return "reload_conversation";
  }
  await dependencies.finalizeMessage(messageId);
  traceDecision("非流式收尾", { outcome: "handled" });
  return "handled";
}

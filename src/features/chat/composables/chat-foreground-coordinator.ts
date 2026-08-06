export type ForegroundSnapshotBindingStage =
  | "clear_runtime"
  | "unbind"
  | "request_snapshot"
  | "apply_snapshot"
  | "bind"
  | "resume";

/**
 * 唯一允许恢复 delta 订阅的快照形态：后端明确标记为 assistant 流，且已给出
 * 可原子读取/合并的正式 assistant 消息 ID。队列、压缩与整理均不能越过此门槛。
 */
export function snapshotCanBindAssistantStream(snapshot: {
  shouldBindStream?: boolean;
  runtimeState?: unknown;
  streamCache?: { persistedAssistantMessageId?: unknown } | null;
}): boolean {
  return snapshot.shouldBindStream === true
    && String(snapshot.runtimeState || "").trim() === "assistant_streaming"
    && !!String(snapshot.streamCache?.persistedAssistantMessageId || "").trim();
}

export async function runForegroundSnapshotBindingTransaction<TSnapshot extends {
  shouldBindStream?: boolean;
  runtimeState?: unknown;
  streamCache?: { persistedAssistantMessageId?: unknown } | null;
}>(input: {
  conversationId: string;
  isCurrent: () => boolean;
  clearRuntime: () => void;
  unbind: () => Promise<void>;
  requestSnapshot: () => Promise<TSnapshot | null>;
  applySnapshot: (snapshot: TSnapshot) => void;
  bind: () => Promise<void>;
  resume: (snapshot: TSnapshot) => void;
  /** Web 端打开/切换会话即注册订阅：跳过 snapshotCanBindAssistantStream 门槛，无条件 bind。 */
  alwaysBind?: boolean;
  onStage?: (stage: ForegroundSnapshotBindingStage) => void;
  onUnbindError?: (error: unknown) => void;
}): Promise<TSnapshot | null> {
  input.onStage?.("clear_runtime");
  input.clearRuntime();
  input.onStage?.("unbind");
  const unbindPromise = input.unbind().catch((error) => {
    input.onUnbindError?.(error);
  });
  input.onStage?.("request_snapshot");
  const snapshot = await input.requestSnapshot();
  if (!snapshot || !input.isCurrent()) {
    await unbindPromise;
    return null;
  }
  input.onStage?.("apply_snapshot");
  input.applySnapshot(snapshot);
  await unbindPromise;
  if (!input.isCurrent()) return null;
  if (!input.alwaysBind && !snapshotCanBindAssistantStream(snapshot)) return snapshot;
  input.onStage?.("bind");
  await input.bind();
  if (!input.isCurrent()) return null;
  input.onStage?.("resume");
  input.resume(snapshot);
  return snapshot;
}

export function createLatestTaskRunner<T>(task: (input: T) => Promise<void>) {
  let latestInput: T | undefined;
  let rerunRequested = false;
  let runningPromise: Promise<void> | null = null;
  let cancelled = false;

  function run(input: T): Promise<void> {
    latestInput = input;
    rerunRequested = true;
    if (runningPromise) return runningPromise;
    runningPromise = (async () => {
      while (rerunRequested && !cancelled) {
        rerunRequested = false;
        await task(latestInput as T);
      }
    })().finally(() => {
      runningPromise = null;
    });
    return runningPromise;
  }

  function cancel() {
    cancelled = true;
    rerunRequested = false;
  }

  return { run, cancel };
}

/** `conversation.changedSince` 的统一结果；水位只表示是否需要正式尾部对账。 */
export type ForegroundWatermarkChanges = {
  changedConversationIds: string[];
  serverTime: string;
};

/**
 * 每个独立聊天视图实例维护自己的读取进度。概览列表的水位不能复用到
 * ChatView，否则列表先同步时会吞掉视图尚未应用的正式消息收口。
 */
export function createForegroundTailWatermarkCoordinator(input: {
  requestChanges: (since: string) => Promise<ForegroundWatermarkChanges>;
}) {
  let lastForegroundMessageWatermark = "";
  let tailReconcilePendingConversationId = "";

  async function observeCurrentConversation(conversationId: string): Promise<void> {
    const normalizedConversationId = String(conversationId || "").trim();
    if (!normalizedConversationId) return;
    const result = await input.requestChanges(lastForegroundMessageWatermark);
    const nextWatermark = String(result?.serverTime || "").trim();
    if (nextWatermark) lastForegroundMessageWatermark = nextWatermark;
    if ((result?.changedConversationIds || []).some((id) => String(id || "").trim() === normalizedConversationId)) {
      tailReconcilePendingConversationId = normalizedConversationId;
    }
  }

  function shouldReconcileTail(conversationId: string): boolean {
    return tailReconcilePendingConversationId === String(conversationId || "").trim();
  }

  function markTailReconciled(conversationId: string) {
    if (shouldReconcileTail(conversationId)) tailReconcilePendingConversationId = "";
  }

  return {
    observeCurrentConversation,
    shouldReconcileTail,
    markTailReconciled,
    get lastForegroundMessageWatermark() { return lastForegroundMessageWatermark; },
  };
}

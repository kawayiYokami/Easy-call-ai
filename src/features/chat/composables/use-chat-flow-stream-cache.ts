import type { Ref } from "vue";
import {
  appendReasoningStandardDelta,
  readDeltaMessage,
  type AssistantDeltaEvent,
} from "./use-chat-flow-events";
import {
  applyToolStatusToStreamToolCalls,
  mergeStreamToolCallsForward,
  normalizeStreamToolCallViews,
  normalizeToolStatusState,
  type StreamToolCallView,
} from "./use-chat-flow-tool-calls";
import {
  normalizeConversationId,
  positiveRoundedNumber,
} from "./use-chat-flow-utils";

export type ConversationStreamCache = {
  activationId?: string;
  requestId?: string;
  startedAt?: string;
  startedAtMs?: number;
  frontendDispatchStartedAtMs?: number;
  frontendDispatchElapsedMs?: number;
  assistantText: string;
  reasoningStandard: string;
  pendingReasoningStandardBreak: boolean;
  reasoningInline: string;
  toolStatusText: string;
  toolStatusState: "running" | "done" | "failed" | "";
  streamToolCalls: StreamToolCallView[];
  streamToolCallCount: number;
  streamLastToolName: string;
  persistedAssistantMessageId?: string;
};

export type ConversationRuntimeStreamCacheSnapshot = {
  activationId?: string;
  requestId?: string;
  startedAt?: string;
  startedAtMs?: number;
  frontendDispatchStartedAtMs?: number;
  frontendDispatchElapsedMs?: number;
  assistantText?: string;
  reasoningStandard?: string;
  reasoningInline?: string;
  toolStatusText?: string;
  toolStatusState?: "running" | "done" | "failed" | "" | string;
  streamToolCalls?: Array<{ toolCallId?: string; name?: string; argsText?: string; status?: "doing" | "done" | string }>;
  streamToolCallCount?: number;
  streamLastToolName?: string;
  hasVisibleProgress?: boolean;
  persistedAssistantMessageId?: string;
};

type UseChatFlowStreamCacheOptions = {
  getConversationId?: () => string;
  latestAssistantText: Ref<string>;
  latestReasoningStandardText: Ref<string>;
  latestReasoningInlineText: Ref<string>;
  toolStatusText: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  streamToolCalls?: Ref<StreamToolCallView[]>;
  getActiveActivationId: () => string;
  getFrontendDispatchStartedAtMs: () => number;
  getFrontendDispatchElapsedMs: () => number;
  currentFrontendDispatchElapsedMs: () => number;
  restoreFrontendDispatchTimerFromCache: (cache: ConversationStreamCache) => void;
  getPendingReasoningStandardBreak: () => boolean;
  setPendingReasoningStandardBreak: (value: boolean) => void;
  getStreamToolCallCount: () => number;
  setStreamToolCallCount: (value: number) => void;
  getStreamLastToolName: () => string;
  setStreamLastToolName: (value: string) => void;
};

export function streamCacheHasVisibleProgress(
  cache?: ConversationRuntimeStreamCacheSnapshot | ConversationStreamCache | null,
): boolean {
  if (!cache) return false;
  return !!(
    String(cache.assistantText || "").trim()
    || String(cache.reasoningStandard || "").trim()
    || String(cache.reasoningInline || "").trim()
    || String(cache.toolStatusText || "").trim()
    || String(cache.toolStatusState || "").trim()
    || (Array.isArray(cache.streamToolCalls) && cache.streamToolCalls.length > 0)
  );
}

function emptyConversationStreamCache(): ConversationStreamCache {
  return {
      activationId: "",
      requestId: "",
      startedAt: "",
      startedAtMs: 0,
      frontendDispatchStartedAtMs: 0,
    frontendDispatchElapsedMs: 0,
    assistantText: "",
    reasoningStandard: "",
    pendingReasoningStandardBreak: false,
    reasoningInline: "",
    toolStatusText: "",
    toolStatusState: "",
    streamToolCalls: [],
    streamToolCallCount: 0,
    streamLastToolName: "",
  };
}

export function useChatFlowStreamCache(options: UseChatFlowStreamCacheOptions) {
  const conversationStreamCache = new Map<string, ConversationStreamCache>();

  function readConversationStreamCache(conversationId?: string | null): ConversationStreamCache | null {
    const cid = normalizeConversationId(conversationId);
    if (!cid) return null;
    const cache = conversationStreamCache.get(cid);
    if (!cache) return null;
    return {
      activationId: String(cache.activationId || "").trim(),
      requestId: String(cache.requestId || "").trim(),
      startedAt: String(cache.startedAt || "").trim(),
      startedAtMs: positiveRoundedNumber(cache.startedAtMs),
      frontendDispatchStartedAtMs: positiveRoundedNumber(cache.frontendDispatchStartedAtMs),
      frontendDispatchElapsedMs: positiveRoundedNumber(cache.frontendDispatchElapsedMs),
      assistantText: cache.assistantText,
      reasoningStandard: cache.reasoningStandard,
      pendingReasoningStandardBreak: !!cache.pendingReasoningStandardBreak,
      reasoningInline: cache.reasoningInline,
      toolStatusText: cache.toolStatusText,
      toolStatusState: cache.toolStatusState,
      streamToolCalls: cache.streamToolCalls.map((item) => ({ ...item })),
      streamToolCallCount: cache.streamToolCallCount,
      streamLastToolName: cache.streamLastToolName,
    };
  }

  function writeConversationStreamCache(
    conversationId: string,
    updater: (current: ConversationStreamCache) => ConversationStreamCache,
  ) {
    const cid = normalizeConversationId(conversationId);
    if (!cid) return;
    const next = updater(readConversationStreamCache(cid) || emptyConversationStreamCache());
    conversationStreamCache.set(cid, {
      ...next,
      activationId: String(next.activationId || "").trim(),
      requestId: String(next.requestId || "").trim(),
      startedAt: String(next.startedAt || "").trim(),
      startedAtMs: positiveRoundedNumber(next.startedAtMs),
      frontendDispatchStartedAtMs: positiveRoundedNumber(next.frontendDispatchStartedAtMs),
      frontendDispatchElapsedMs: positiveRoundedNumber(next.frontendDispatchElapsedMs),
      streamToolCalls: Array.isArray(next.streamToolCalls) ? next.streamToolCalls.map((item) => ({ ...item })) : [],
    });
  }

  function clearConversationStreamCache(conversationId?: string | null) {
    const cid = normalizeConversationId(conversationId);
    if (!cid) return;
    conversationStreamCache.delete(cid);
  }

  function syncCurrentDisplayStateToConversationStreamCache(conversationId?: string | null) {
    const cid = normalizeConversationId(conversationId || (options.getConversationId ? options.getConversationId() : ""));
    if (!cid) return;
    const activeActivationId = options.getActiveActivationId();
    writeConversationStreamCache(cid, (current) => ({
      assistantText: String(options.latestAssistantText.value || ""),
      activationId: activeActivationId,
      requestId: activeActivationId,
      startedAt: current.startedAt,
      startedAtMs: current.startedAtMs,
      frontendDispatchStartedAtMs: options.getFrontendDispatchStartedAtMs(),
      frontendDispatchElapsedMs: options.currentFrontendDispatchElapsedMs(),
      reasoningStandard: String(options.latestReasoningStandardText.value || ""),
      pendingReasoningStandardBreak: options.getPendingReasoningStandardBreak(),
      reasoningInline: String(options.latestReasoningInlineText.value || ""),
      toolStatusText: String(options.toolStatusText.value || ""),
      toolStatusState: options.toolStatusState.value,
      streamToolCalls: Array.isArray(options.streamToolCalls?.value)
        ? options.streamToolCalls.value.map((item) => ({ ...item }))
        : [],
      streamToolCallCount: options.getStreamToolCallCount(),
      streamLastToolName: options.getStreamLastToolName(),
    }));
  }

  function applyConversationStreamCacheToDisplay(conversationId?: string | null): boolean {
    const cache = readConversationStreamCache(conversationId);
    if (!cache) return false;
    const activeActivationId = options.getActiveActivationId();
    if (activeActivationId && cache.activationId && cache.activationId !== activeActivationId) {
      return false;
    }
    if (cache.assistantText || !options.latestAssistantText.value) {
      options.latestAssistantText.value = cache.assistantText;
    }
    options.restoreFrontendDispatchTimerFromCache(cache);
    if (cache.reasoningStandard || !options.latestReasoningStandardText.value) {
      options.latestReasoningStandardText.value = cache.reasoningStandard;
    }
    options.setPendingReasoningStandardBreak(!!cache.pendingReasoningStandardBreak);
    if (cache.reasoningInline || !options.latestReasoningInlineText.value) {
      options.latestReasoningInlineText.value = cache.reasoningInline;
    }
    if (cache.toolStatusText || !options.toolStatusText.value) {
      options.toolStatusText.value = cache.toolStatusText;
    }
    if (cache.toolStatusState || !options.toolStatusState.value) {
      options.toolStatusState.value = cache.toolStatusState;
    }
    if (options.streamToolCalls) {
      if (cache.streamToolCalls.length > 0 || options.streamToolCalls.value.length === 0) {
        options.streamToolCalls.value = mergeStreamToolCallsForward(
          options.streamToolCalls.value,
          cache.streamToolCalls,
        );
      }
    }
    options.setStreamToolCallCount(Math.max(options.getStreamToolCallCount(), cache.streamToolCallCount));
    if (cache.streamLastToolName) {
      options.setStreamLastToolName(cache.streamLastToolName);
    }
    return true;
  }

  function writeConversationStreamCacheSnapshot(
    conversationId: string,
    snapshot?: ConversationRuntimeStreamCacheSnapshot | null,
  ) {
    const cid = normalizeConversationId(conversationId);
    if (!cid || !snapshot) return;
    writeConversationStreamCache(cid, (current) => ({
      activationId: String(snapshot.activationId || snapshot.requestId || current.activationId || "").trim(),
      requestId: String(snapshot.requestId || snapshot.activationId || current.requestId || "").trim(),
      startedAt: String(snapshot.startedAt || current.startedAt || "").trim(),
      startedAtMs: positiveRoundedNumber(snapshot.startedAtMs || current.startedAtMs),
      frontendDispatchStartedAtMs: positiveRoundedNumber(snapshot.startedAtMs || snapshot.frontendDispatchStartedAtMs || current.frontendDispatchStartedAtMs),
      frontendDispatchElapsedMs: positiveRoundedNumber(snapshot.frontendDispatchElapsedMs || current.frontendDispatchElapsedMs),
      assistantText: String(snapshot.assistantText || ""),
      reasoningStandard: String(snapshot.reasoningStandard || ""),
      pendingReasoningStandardBreak: current.pendingReasoningStandardBreak,
      reasoningInline: String(snapshot.reasoningInline || ""),
      toolStatusText: String(snapshot.toolStatusText || ""),
      toolStatusState: normalizeToolStatusState(snapshot.toolStatusState),
      streamToolCalls: mergeStreamToolCallsForward(
        current.streamToolCalls,
        normalizeStreamToolCallViews(snapshot.streamToolCalls),
      ),
      streamToolCallCount: Math.max(0, Math.round(Number(snapshot.streamToolCallCount || 0))),
      streamLastToolName: String(snapshot.streamLastToolName || ""),
      persistedAssistantMessageId: String(snapshot.persistedAssistantMessageId || current.persistedAssistantMessageId || "").trim(),
    }));
  }

  function applyAssistantEventToConversationStreamCache(
    conversationId: string,
    parsed: AssistantDeltaEvent,
  ): boolean {
    const cid = normalizeConversationId(conversationId);
    if (!cid) return false;
    let changed = false;
    writeConversationStreamCache(cid, (current) => {
      const activeActivationId = options.getActiveActivationId();
      const next: ConversationStreamCache = {
        ...current,
        activationId: String(parsed.activationId || parsed.requestId || current.activationId || activeActivationId || "").trim(),
        requestId: String(parsed.requestId || parsed.activationId || current.requestId || activeActivationId || "").trim(),
        startedAt: current.startedAt,
        startedAtMs: current.startedAtMs,
        frontendDispatchStartedAtMs: options.getFrontendDispatchStartedAtMs(),
        frontendDispatchElapsedMs: options.currentFrontendDispatchElapsedMs(),
        streamToolCalls: mergeStreamToolCallsForward(
          current.streamToolCalls,
          Array.isArray(options.streamToolCalls?.value) ? options.streamToolCalls.value : [],
        ),
      };
      const delta = readDeltaMessage(parsed);
      if (parsed.kind === "tool_status") {
        const toolName = String(parsed.toolName || "").trim();
        const statusUpdate = applyToolStatusToStreamToolCalls(next.streamToolCalls, parsed);
        if (String(next.reasoningStandard || "").trim()) {
          next.pendingReasoningStandardBreak = true;
        }
        next.streamToolCalls = statusUpdate.calls;
        if (parsed.toolStatus === "running" && toolName && parsed.toolCallId) {
          next.streamLastToolName = toolName;
          if (statusUpdate.appended) {
            next.streamToolCallCount += 1;
          }
        } else if (statusUpdate.appended) {
          next.streamToolCallCount = Math.max(next.streamToolCallCount, next.streamToolCalls.length);
        }
        next.toolStatusText = parsed.message || "";
        next.toolStatusState =
          parsed.toolStatus === "running" || parsed.toolStatus === "done" || parsed.toolStatus === "failed"
            ? parsed.toolStatus : "";
        changed = true;
        return next;
      }
      if (parsed.kind === "reasoning_standard" && delta) {
        next.reasoningStandard = appendReasoningStandardDelta(
          next.reasoningStandard,
          delta,
          next.pendingReasoningStandardBreak,
        );
        if (delta.trim()) {
          next.pendingReasoningStandardBreak = false;
        }
        changed = true;
        return next;
      }
      if (parsed.kind === "reasoning_inline" && delta) {
        next.reasoningInline += delta;
        changed = true;
        return next;
      }
      if (delta) {
        next.assistantText += delta;
        changed = true;
      }
      return next;
    });
    return changed;
  }

  return {
    applyAssistantEventToConversationStreamCache,
    applyConversationStreamCacheToDisplay,
    clearConversationStreamCache,
    readConversationStreamCache,
    syncCurrentDisplayStateToConversationStreamCache,
    writeConversationStreamCacheSnapshot,
  };
}

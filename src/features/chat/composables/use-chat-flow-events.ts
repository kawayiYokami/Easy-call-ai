import type { ChatMessage } from "../../../types/app";
import type { ConversationRuntimeStreamCacheSnapshot } from "./use-chat-flow-stream-cache";

export type AssistantDeltaEvent = {
  delta?: string;
  kind?: string;
  requestId?: string;
  activationId?: string;
  phaseId?: string;
  reason?: string;
  toolName?: string;
  toolCallId?: string;
  toolStatus?: string;
  toolArgs?: string;
  message?: string;
  streamCache?: ConversationRuntimeStreamCacheSnapshot;
};

export type HistoryFlushedPayload = {
  conversationId: string;
  messageCount: number;
  messages: ChatMessage[];
  activateAssistant?: boolean;
  compactionApplied?: boolean;
};

export type RoundStartedPayload = {
  conversationId: string;
  activationId?: string;
  requestId?: string;
  reason?: string;
  departmentId?: string;
  agentId?: string;
  startedAt?: string;
  startedAtMs?: number;
};

export type RoundCompletedPayload = {
  conversationId: string;
  activationId?: string;
  requestId?: string;
  assistantText: string;
  archivedBeforeSend?: boolean;
  assistantMessage?: ChatMessage;
};

export type RoundFailedPayload = {
  conversationId?: string;
  activationId?: string;
  requestId?: string;
  error: string;
};

export type ContextUsageUpdatePayload = {
  conversationId: string;
  contextUsagePercent: number;
  contextUsageRatio: number;
  effectivePromptTokens: number;
  contextWindowTokens: number;
  estimatedPromptTokens?: number;
  latestToolResultEstimatedTokens?: number;
  source?: string;
  decisionSource?: string;
  reason?: string;
  eventReason?: string;
  shouldArchive?: boolean;
  forced?: boolean;
};

function optionalPositiveInteger(value: unknown): number | undefined {
  const next = Math.round(Number(value) || 0);
  return next > 0 ? next : undefined;
}

export function readRoundStartedPayload(raw: string | undefined): RoundStartedPayload | null {
  const text = String(raw || "").trim();
  if (!text) return null;
  try {
    const parsed = JSON.parse(text) as Record<string, unknown>;
    return {
      conversationId: String(parsed.conversationId || "").trim(),
      activationId: typeof parsed.activationId === "string" ? parsed.activationId : undefined,
      requestId: typeof parsed.requestId === "string" ? parsed.requestId : undefined,
      reason: typeof parsed.reason === "string" ? parsed.reason : undefined,
      departmentId: typeof parsed.departmentId === "string" ? parsed.departmentId : undefined,
      agentId: typeof parsed.agentId === "string" ? parsed.agentId : undefined,
      startedAt: typeof parsed.startedAt === "string" ? parsed.startedAt : undefined,
      startedAtMs: Math.max(0, Math.round(Number(parsed.startedAtMs) || 0)) || undefined,
    };
  } catch {
    return {
      conversationId: text,
    };
  }
}

export function readHistoryFlushedPayload(raw: string | undefined): HistoryFlushedPayload | null {
  const text = String(raw || "").trim();
  if (!text) return null;
  try {
    const parsed = JSON.parse(text) as Record<string, unknown>;
    return {
      conversationId: String(parsed.conversationId || "").trim(),
      messageCount: Math.max(0, Math.round(Number(parsed.messageCount) || 0)),
      messages: Array.isArray(parsed.messages) ? (parsed.messages as ChatMessage[]) : [],
      activateAssistant: !!parsed.activateAssistant,
      compactionApplied: !!parsed.compactionApplied,
    };
  } catch {
    return {
      conversationId: text,
      messageCount: 0,
      messages: [],
      activateAssistant: false,
      compactionApplied: false,
    };
  }
}

export function readRoundCompletedPayload(raw: string | undefined): RoundCompletedPayload | null {
  const text = String(raw || "").trim();
  if (!text) return null;
  try {
    const parsed = JSON.parse(text) as Record<string, unknown>;
    return {
      conversationId: String(parsed.conversationId || "").trim(),
      activationId: typeof parsed.activationId === "string" ? parsed.activationId : undefined,
      requestId: typeof parsed.requestId === "string" ? parsed.requestId : undefined,
      assistantText: String(parsed.assistantText || ""),
      archivedBeforeSend: !!parsed.archivedBeforeSend,
      assistantMessage: (parsed.assistantMessage as ChatMessage | undefined) || undefined,
    };
  } catch {
    return null;
  }
}

export function readRoundFailedPayload(raw: string | undefined): RoundFailedPayload | null {
  const text = String(raw || "").trim();
  if (!text) return null;
  try {
    const parsed = JSON.parse(text) as Record<string, unknown>;
    return {
      conversationId: typeof parsed.conversationId === "string" ? parsed.conversationId : undefined,
      activationId: typeof parsed.activationId === "string" ? parsed.activationId : undefined,
      requestId: typeof parsed.requestId === "string" ? parsed.requestId : undefined,
      error: String(parsed.error || ""),
    };
  } catch {
    return { error: text };
  }
}

export function readContextUsageUpdatePayload(raw: string | undefined): ContextUsageUpdatePayload | null {
  const text = String(raw || "").trim();
  if (!text) return null;
  try {
    const parsed = JSON.parse(text) as Record<string, unknown>;
    const conversationId = String(parsed.conversationId || "").trim();
    if (!conversationId) return null;
    const ratio = Number(parsed.contextUsageRatio);
    const percent = Math.round(Number(parsed.contextUsagePercent) || 0);
    const effectivePromptTokens = Math.round(Number(parsed.effectivePromptTokens) || 0);
    const contextWindowTokens = Math.round(Number(parsed.contextWindowTokens) || 0);
    return {
      conversationId,
      contextUsagePercent: Math.min(100, Math.max(0, percent)),
      contextUsageRatio: Number.isFinite(ratio) ? Math.max(0, ratio) : 0,
      effectivePromptTokens: Math.max(0, effectivePromptTokens),
      contextWindowTokens: Math.max(0, contextWindowTokens),
      estimatedPromptTokens: optionalPositiveInteger(parsed.estimatedPromptTokens),
      latestToolResultEstimatedTokens: optionalPositiveInteger(parsed.latestToolResultEstimatedTokens),
      source: typeof parsed.source === "string" ? parsed.source : undefined,
      decisionSource: typeof parsed.decisionSource === "string" ? parsed.decisionSource : undefined,
      reason: typeof parsed.reason === "string" ? parsed.reason : undefined,
      eventReason: typeof parsed.eventReason === "string" ? parsed.eventReason : undefined,
      shouldArchive: typeof parsed.shouldArchive === "boolean" ? parsed.shouldArchive : undefined,
      forced: typeof parsed.forced === "boolean" ? parsed.forced : undefined,
    };
  } catch {
    return null;
  }
}

export function readDeltaMessage(message: unknown): string {
  if (typeof message === "string") return message;
  if (message && typeof message === "object" && "delta" in message) {
    const value = (message as { delta?: unknown }).delta;
    return typeof value === "string" ? value : "";
  }
  return "";
}

export function readAssistantEvent(message: unknown): AssistantDeltaEvent {
  if (!message || typeof message !== "object") return {};
  const m = message as Record<string, unknown>;
  return {
    delta: typeof m.delta === "string" ? m.delta : undefined,
    kind: typeof m.kind === "string" ? m.kind : undefined,
    requestId: typeof m.requestId === "string" ? m.requestId : undefined,
    activationId: typeof m.activationId === "string" ? m.activationId : undefined,
    phaseId: typeof m.phaseId === "string" ? m.phaseId : undefined,
    reason: typeof m.reason === "string" ? m.reason : undefined,
    toolName: typeof m.toolName === "string" ? m.toolName : undefined,
    toolCallId: typeof m.toolCallId === "string" ? m.toolCallId : undefined,
    toolStatus: typeof m.toolStatus === "string" ? m.toolStatus : undefined,
    toolArgs: typeof m.toolArgs === "string" ? m.toolArgs : undefined,
    message: typeof m.message === "string" ? m.message : undefined,
    streamCache: m.streamCache && typeof m.streamCache === "object"
      ? m.streamCache as ConversationRuntimeStreamCacheSnapshot
      : undefined,
  };
}

export function assistantEventHasVisibleProgress(parsed: AssistantDeltaEvent): boolean {
  return (
    !!readDeltaMessage(parsed)
    || parsed.kind === "activity_reasoning_delta"
    || parsed.kind === "assistant_tool_event"
    || parsed.kind === "assistant_tool_result"
    || parsed.kind === "tool_status"
  );
}

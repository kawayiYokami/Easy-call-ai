import type { ChatMessageEvent } from "./chat-message-state-machine";
import type { AssistantDeltaEvent, RoundStartedPayload } from "./use-chat-flow-events";

function normalized(value: unknown): string {
  return String(value || "").trim();
}

export function transportRoundStartedToMessageEvent(
  payload: Partial<RoundStartedPayload>,
  fallbackConversationId: string,
  input: {
    assistantMessageId: string;
    statusText?: string;
    phase?: "waiting" | "streaming";
    speakerAgentId?: string;
  },
): Extract<ChatMessageEvent, { type: "round_started" }> | null {
  const conversationId = normalized(payload.conversationId || fallbackConversationId);
  const assistantMessageId = normalized(payload.assistantMessageId || input.assistantMessageId);
  if (!conversationId || !assistantMessageId) return null;
  return {
    type: "round_started",
    conversationId,
    assistantMessageId,
    activationId: normalized(payload.activationId) || undefined,
    requestId: normalized(payload.requestId) || undefined,
    startedAt: normalized(payload.startedAt) || undefined,
    startedAtMs: Math.max(0, Math.round(Number(payload.startedAtMs) || 0)) || undefined,
    speakerAgentId: normalized(payload.agentId || input.speakerAgentId) || undefined,
    statusText: input.statusText,
    phase: input.phase || "waiting",
  };
}

export function transportAssistantDeltaToMessageEvent(
  parsed: AssistantDeltaEvent,
  conversationId: string,
  assistantMessageId: string,
): Extract<ChatMessageEvent, { type: "assistant_delta" }> | null {
  const normalizedConversationId = normalized(conversationId);
  if (!normalizedConversationId) return null;
  const streamCache = parsed.streamCache;
  return {
    type: "assistant_delta",
    conversationId: normalizedConversationId,
    event: {
      assistantMessageId: normalized(
        streamCache?.persistedAssistantMessageId || assistantMessageId,
      ) || undefined,
      activationId: normalized(parsed.activationId || streamCache?.activationId) || undefined,
      requestId: normalized(parsed.requestId || streamCache?.requestId) || undefined,
      revision: normalized(streamCache?.updatedAt) || undefined,
      kind: normalized(parsed.kind) || undefined,
      delta: typeof parsed.delta === "string" ? parsed.delta : undefined,
      message: typeof parsed.message === "string" ? parsed.message : undefined,
      toolStatus: normalized(parsed.toolStatus) || undefined,
      streamCache: streamCache ? {
        activationId: normalized(streamCache.activationId) || undefined,
        requestId: normalized(streamCache.requestId) || undefined,
        updatedAt: normalized(streamCache.updatedAt) || undefined,
        startedAt: normalized(streamCache.startedAt) || undefined,
        startedAtMs: Math.max(0, Math.round(Number(streamCache.startedAtMs) || 0)) || undefined,
        assistantText: typeof streamCache.assistantText === "string" ? streamCache.assistantText : undefined,
        toolStatusText: typeof streamCache.toolStatusText === "string" ? streamCache.toolStatusText : undefined,
        toolStatusState: normalized(streamCache.toolStatusState) || undefined,
        streamBlocks: streamCache.streamBlocks,
        persistedAssistantMessageId: normalized(streamCache.persistedAssistantMessageId) || undefined,
        speakerAgentId: normalized(streamCache.speakerAgentId || streamCache.agentId) || undefined,
        frontendDispatchStartedAtMs: Math.max(0, Math.round(Number(streamCache.frontendDispatchStartedAtMs) || 0)) || undefined,
        frontendDispatchElapsedMs: Math.max(0, Math.round(Number(streamCache.frontendDispatchElapsedMs) || 0)) || undefined,
        contextUsageRatio: typeof streamCache.contextUsageRatio === "number" ? streamCache.contextUsageRatio : undefined,
        contextUsagePercent: typeof streamCache.contextUsagePercent === "number" ? streamCache.contextUsagePercent : undefined,
        effectivePromptTokens: Math.max(0, Math.round(Number(streamCache.effectivePromptTokens) || 0)) || undefined,
        contextWindowTokens: Math.max(0, Math.round(Number(streamCache.contextWindowTokens) || 0)) || undefined,
        schedulingState: normalized((streamCache as Record<string, unknown>).schedulingState) || undefined,
      } : undefined,
    },
  };
}

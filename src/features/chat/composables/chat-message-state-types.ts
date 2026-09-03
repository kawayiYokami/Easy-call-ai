import type { ChatMessage } from "../../../types/app";

export type ChatMessageRoundPhase = "idle" | "waiting" | "streaming" | "settling";

export type ChatMessageRoundProjection = {
  phase: ChatMessageRoundPhase;
  assistantMessageId: string;
  activationId: string;
  requestId: string;
  revision: string;
  startedAtMs: number;
};

export type ChatMessageState = {
  conversationId: string;
  messages: ChatMessage[];
  round: ChatMessageRoundProjection;
  error: string;
};

export type ChatMessageStreamSnapshot = {
  activationId?: string;
  requestId?: string;
  updatedAt?: string;
  startedAt?: string;
  startedAtMs?: number;
  assistantText?: string;
  toolStatusText?: string;
  toolStatusState?: string;
  streamBlocks?: unknown;
  streamSegments?: string[];
  streamTail?: string;
  streamAnimatedDelta?: string;
  preStreamingStatusText?: string;
  frontendDispatchStartedAtMs?: number;
  frontendDispatchElapsedMs?: number;
  persistedAssistantMessageId?: string;
  speakerAgentId?: string;
  agentId?: string;
  contextUsageRatio?: number;
  contextUsagePercent?: number;
  effectivePromptTokens?: number;
  contextWindowTokens?: number;
  schedulingState?: string;
};

export type ChatAssistantDelta = {
  assistantMessageId?: string;
  activationId?: string;
  requestId?: string;
  revision?: string;
  kind?: string;
  delta?: string;
  message?: string;
  toolStatus?: string;
  streamCache?: ChatMessageStreamSnapshot;
};

export type AuthoritativeMessageMergeOptions = {
  forceReplace?: boolean;
  optimisticUserDraftId?: string;
  prependMessages?: boolean;
  replaceOptimisticUserDrafts?: boolean;
  summarySeedsFirst?: boolean;
};

export type ChatMessageEvent =
  | {
      type: "history_replaced";
      conversationId: string;
      messages: ChatMessage[];
    }
  | {
      type: "authoritative_messages_merged";
      conversationId: string;
      messages: ChatMessage[];
      options?: AuthoritativeMessageMergeOptions;
    }
  | {
      type: "round_started";
      conversationId: string;
      assistantMessageId: string;
      activationId?: string;
      requestId?: string;
      revision?: string;
      startedAt?: string;
      startedAtMs?: number;
      speakerAgentId?: string;
      statusText?: string;
      phase?: "waiting" | "streaming";
    }
  | {
      type: "assistant_stream_snapshot";
      conversationId: string;
      assistantMessageId?: string;
      snapshot: ChatMessageStreamSnapshot;
    }
  | {
      type: "assistant_delta";
      conversationId: string;
      event: ChatAssistantDelta;
    }
  | {
      type: "round_finished";
      conversationId: string;
      assistantMessageId?: string;
      activationId?: string;
      requestId?: string;
      assistantMessage?: ChatMessage;
    }
  | {
      type: "round_failed";
      conversationId: string;
      assistantMessageId?: string;
      activationId?: string;
      requestId?: string;
      error?: unknown;
    }
  | {
      type: "round_reset";
      conversationId: string;
      preserveVisibleContent?: boolean;
    };

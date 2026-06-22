import type { Channel } from "@tauri-apps/api/core";
import type { Ref } from "vue";
import type { AssistantStreamBlock, ChatMentionTarget, ChatMessage } from "../../../types/app";
import type { AssistantDeltaEvent, ContextUsageUpdatePayload } from "./use-chat-flow-events";
import type { ConversationRuntimeStreamCacheSnapshot } from "./use-chat-flow-stream-cache";

export type FrontendRoundPhase = "idle" | "queued" | "waiting" | "streaming";

export type UseChatFlowOptions = {
  chatting: Ref<boolean>;
  submitPending?: Ref<boolean>;
  trimming: Ref<boolean>;
  isConversationBusy?: () => boolean;
  getSession: () => { apiConfigId: string; agentId: string; departmentId?: string } | null;
  getConversationId?: () => string;
  chatInput: Ref<string>;
  selectedMentions?: Ref<ChatMentionTarget[]>;
  clipboardImages: Ref<Array<{ mime: string; bytesBase64: string; savedPath?: string }>>;
  queuedAttachmentNotices?: Ref<Array<{ id: string; fileName: string; relativePath: string; mime: string }>>;
  latestUserText: Ref<string>;
  latestUserImages: Ref<Array<{ mime: string; bytesBase64: string }>>;
  latestAssistantText: Ref<string>;
  toolStatusText: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  streamBlocks?: Ref<AssistantStreamBlock[]>;
  contextUsagePreview?: Ref<ContextUsageUpdatePayload | null>;
  chatErrorText: Ref<string>;
  setConversationChatError?: (conversationId: string, text: string) => void;
  allMessages: Ref<ChatMessage[]>;
  onOwnUserDraftInserted?: () => void;
  onAssistantDraftInserted?: () => void;
  t: (key: string, params?: Record<string, unknown>) => string;
  formatRequestFailed: (error: unknown) => string;
  removeBinaryPlaceholders: (text: string) => string;
  invokeSendChatMessage: (input: {
    text: string;
    displayText?: string;
    images: Array<{ mime: string; bytesBase64: string; savedPath?: string }>;
    attachments?: Array<{ fileName: string; relativePath: string; mime: string }>;
    extraTextBlocks?: string[];
    mentions?: ChatMentionTarget[];
    session: { apiConfigId: string; agentId: string; departmentId?: string; conversationId?: string };
    traceId: string;
    onDelta: Channel<AssistantDeltaEvent>;
  }) => Promise<{
    accepted: boolean;
    duplicate: boolean;
    eventId: string;
    conversationId: string;
    traceId: string;
    ingress: string;
  }>;
  invokeStopChatMessage?: (input: {
    session: { apiConfigId: string; agentId: string; departmentId?: string; conversationId?: string };
    partialAssistantText: string;
    partialStreamBlocks: AssistantStreamBlock[];
  }) => Promise<{
    aborted: boolean;
    persisted: boolean;
    conversationId?: string | null;
    assistantText?: string;
    assistantMessage?: ChatMessage;
  }>;
  invokeBindActiveChatViewStream?: (input: {
    conversationId?: string;
    onDelta: Channel<AssistantDeltaEvent>;
  }) => Promise<void>;
  onReloadMessages: () => Promise<void>;
  onHistoryFlushed?: (input: {
    conversationId: string;
    messageCount: number;
    pendingMessages: ChatMessage[];
    activateAssistant: boolean;
  }) => Promise<void>;
  onAssistantMessageCompleted?: (input: {
    conversationId: string;
    assistantMessage: ChatMessage;
  }) => Promise<void> | void;
};

export type RoundState =
  | { phase: "idle" }
  | { phase: "queued"; gen: number }
  | { phase: "streaming"; gen: number; draftId: string };

export type PendingTerminalEvent =
  | {
      kind: "completed";
      gen: number;
      result: {
        assistantText: string;
        assistantMessage?: ChatMessage;
      };
    }
  | {
      kind: "failed";
      gen: number;
      error: unknown;
    };

export type DeferredRoundCompletion = {
  gen: number;
  result: {
    assistantText: string;
    assistantMessage?: ChatMessage;
  };
};

export type SendChatOverrides = {
  text?: string;
  displayText?: string;
  extraTextBlocks?: string[];
  suppressInitialReload?: boolean;
};

export type ResumeForegroundRuntimeRoundInput = {
  conversationId?: string | null;
  streamCache?: ConversationRuntimeStreamCacheSnapshot | null;
  statusText?: string;
  reason?: string;
};

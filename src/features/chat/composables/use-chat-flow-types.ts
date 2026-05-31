import type { Channel } from "@tauri-apps/api/core";
import type { Ref } from "vue";
import type { ChatMentionTarget, ChatMessage, PromptCommandPreset } from "../../../types/app";
import type { AssistantDeltaEvent } from "./use-chat-flow-events";
import type { StreamToolCallView } from "./use-chat-flow-tool-calls";
import type { ConversationRuntimeStreamCacheSnapshot } from "./use-chat-flow-stream-cache";

export type FrontendRoundPhase = "idle" | "queued" | "waiting" | "streaming";

export type UseChatFlowOptions = {
  chatting: Ref<boolean>;
  trimming: Ref<boolean>;
  getSession: () => { apiConfigId: string; agentId: string; departmentId?: string } | null;
  getConversationId?: () => string;
  chatInput: Ref<string>;
  selectedMentions?: Ref<ChatMentionTarget[]>;
  selectedInstructionPrompts?: Ref<PromptCommandPreset[]>;
  clipboardImages: Ref<Array<{ mime: string; bytesBase64: string; savedPath?: string }>>;
  queuedAttachmentNotices?: Ref<Array<{ id: string; fileName: string; relativePath: string; mime: string }>>;
  latestUserText: Ref<string>;
  latestUserImages: Ref<Array<{ mime: string; bytesBase64: string }>>;
  latestAssistantText: Ref<string>;
  latestReasoningStandardText: Ref<string>;
  latestReasoningInlineText: Ref<string>;
  toolStatusText: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  streamToolCalls?: Ref<StreamToolCallView[]>;
  chatErrorText: Ref<string>;
  setConversationChatError?: (conversationId: string, text: string) => void;
  allMessages: Ref<ChatMessage[]>;
  onOwnUserDraftInserted?: () => void;
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
    partialReasoningStandard: string;
    partialReasoningInline: string;
  }) => Promise<{
    aborted: boolean;
    persisted: boolean;
    conversationId?: string | null;
    assistantText?: string;
    reasoningStandard?: string;
    reasoningInline?: string;
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
        reasoningStandard?: string;
        reasoningInline?: string;
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
    reasoningStandard?: string;
    reasoningInline?: string;
    assistantMessage?: ChatMessage;
  };
};

export type SendChatOverrides = {
  text?: string;
  displayText?: string;
  extraTextBlocks?: string[];
  skipInstructionPrompts?: boolean;
  suppressInitialReload?: boolean;
};

export type ResumeForegroundRuntimeRoundInput = {
  conversationId?: string | null;
  streamCache?: ConversationRuntimeStreamCacheSnapshot | null;
  statusText?: string;
  reason?: string;
};

import { Channel } from "@tauri-apps/api/core";
import type { Ref } from "vue";
import type { ChatMentionTarget, ChatMessage } from "../../../types/app";
import {
  DRAFT_ASSISTANT_ID_PREFIX,
} from "./use-chat-flow-drafts";
import type { AssistantDeltaEvent } from "./use-chat-flow-events";
import type { PreparedChatSendInput } from "./use-chat-flow-send-input";
import type { RoundState, SendChatOverrides } from "./use-chat-flow-types";
import { isChatAbortedByUser } from "./use-chat-flow-utils";

type StreamUserImageAttachment = { mime: string; bytesBase64: string; savedPath?: string };

type UseChatFlowSendControllerOptions = {
  chatting: Ref<boolean>;
  toolStatusText: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  streamToolCalls?: Ref<any[]>;
  getConversationId?: () => string;
  getSession: () => { apiConfigId: string; agentId: string; departmentId?: string } | null;
  invokeSendChatMessage: (input: {
    text: string;
    displayText?: string;
    images: StreamUserImageAttachment[];
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
  onOwnUserDraftInserted?: () => void;
  t: (key: string, params?: Record<string, unknown>) => string;
  getRound: () => RoundState;
  setRound: (next: RoundState) => void;
  nextGeneration: () => number;
  setSendChatActiveGen: (gen: number) => void;
  setActiveActivationId: (value: string) => void;
  setPendingTerminalEventNull: () => void;
  sendStartedAtMsByGen: Map<number, number>;
  startFrontendDispatchTimer: (gen: number, startedAtMs?: number, elapsedMs?: number) => void;
  clearConversationStreamCache: (conversationId?: string | null) => void;
  clearChatErrorText: (conversationId?: string | null) => void;
  applyPreparedSendInput: (input: PreparedChatSendInput) => void;
  prepareSendInput: (overrides?: SendChatOverrides) => PreparedChatSendInput | null;
  insertUserDraft: (
    gen: number,
    text: string,
    images: StreamUserImageAttachment[],
    attachments: Array<{ fileName: string; relativePath: string; mime: string }>,
    mentions: ChatMentionTarget[],
  ) => string;
  resetDisplayState: () => void;
  removeDraft: (draftId: string) => void;
  updateQueuedAssistantDraftStatus: (draftId: string, statusText: string) => void;
  channelBinding: {
    attachDeltaHandler: (
      channel: Channel<AssistantDeltaEvent>,
      source: "sendChat" | "bound",
      getGen: () => number,
      nextGenOnHistoryFlushed: () => number,
    ) => void;
  };
  handleRoundCompleted: (gen: number, result: {
    assistantText: string;
    reasoningStandard?: string;
    reasoningInline?: string;
    assistantMessage?: ChatMessage;
  }) => Promise<void>;
  sendRecovery: {
    handleAbortedSend: (gen: number, sendConversationId: string) => void;
    handleFailedSend: (
      gen: number,
      error: unknown,
      sendSession: { apiConfigId: string; agentId: string; departmentId?: string; conversationId?: string },
      sendConversationId: string,
    ) => Promise<void>;
    finalizeSendChat: (gen: number, suppressInitialReload?: boolean) => Promise<void>;
  };
};

export function useChatFlowSendController(options: UseChatFlowSendControllerOptions) {
  async function sendChat(overrides?: SendChatOverrides) {
    const prepared = options.prepareSendInput(overrides);
    if (!prepared) return;
    const {
      plainText,
      selectedMentions,
      extraTextBlocks,
      sentImages,
      attachments,
      sendSession,
      sendConversationId,
    } = prepared;

    const hasForegroundRoundInFlight = options.chatting.value || options.getRound().phase !== "idle";
    if (!hasForegroundRoundInFlight) {
      options.clearConversationStreamCache(sendConversationId);
      options.setActiveActivationId("");
      options.toolStatusText.value = "";
      options.toolStatusState.value = "";
      if (options.streamToolCalls) options.streamToolCalls.value = [];
      options.clearChatErrorText(sendConversationId);
    }

    options.applyPreparedSendInput(prepared);

    const gen = options.nextGeneration();
    options.setSendChatActiveGen(gen);
    options.sendStartedAtMsByGen.set(gen, Date.now());
    if (!hasForegroundRoundInFlight) {
      options.startFrontendDispatchTimer(gen, options.sendStartedAtMsByGen.get(gen));
    }
    console.warn("[聊天前端耗时] 发送开始", {
      gen,
      conversationId: String(options.getConversationId ? options.getConversationId() : "").trim(),
      textLength: plainText.length,
      imageCount: sentImages.length,
      attachmentCount: attachments.length,
      extraBlockCount: extraTextBlocks.length,
    });
    options.setPendingTerminalEventNull();
    if (!hasForegroundRoundInFlight) {
      options.insertUserDraft(gen, plainText, sentImages, attachments, selectedMentions);
      options.onOwnUserDraftInserted?.();
    }

    if (!hasForegroundRoundInFlight) {
      options.resetDisplayState();
      const round = options.getRound();
      if (round.phase === "streaming") options.removeDraft(round.draftId);
      if (selectedMentions.length === 0) {
        options.setRound({ phase: "queued", gen });
        options.updateQueuedAssistantDraftStatus(`${DRAFT_ASSISTANT_ID_PREFIX}${gen}`, options.t("chat.statusPreparingMessage"));
      }
    }

    const deltaChannel = new Channel<AssistantDeltaEvent>();
    options.channelBinding.attachDeltaHandler(deltaChannel, "sendChat", () => gen, () => gen);
    const traceId = typeof crypto !== "undefined" && typeof crypto.randomUUID === "function"
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(36).slice(2)}`;

    try {
      await options.invokeSendChatMessage({
        text: plainText,
        displayText:
          overrides && typeof overrides.displayText === "string"
            ? overrides.displayText
            : plainText,
        images: sentImages,
        attachments: attachments.length > 0 ? attachments : undefined,
        extraTextBlocks: extraTextBlocks.length > 0 ? extraTextBlocks : undefined,
        mentions: selectedMentions.length > 0 ? selectedMentions : undefined,
        session: {
          ...sendSession,
          conversationId: sendConversationId,
        },
        traceId,
        onDelta: deltaChannel,
      });
    } catch (error) {
      if (isChatAbortedByUser(error)) {
        options.sendRecovery.handleAbortedSend(gen, sendConversationId);
        return;
      }
      await options.sendRecovery.handleFailedSend(gen, error, sendSession, sendConversationId);
    } finally {
      // submit_chat_message 是短提交命令；成功后的轮次收束只由 history_flushed、round_started、round_completed 等事件驱动。
    }
  }

  return {
    sendChat,
  };
}

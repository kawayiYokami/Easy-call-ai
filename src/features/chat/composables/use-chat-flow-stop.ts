import type { Ref } from "vue";
import type { ChatMessage } from "../../../types/app";
import {
  DRAFT_ASSISTANT_ID_PREFIX,
  DRAFT_USER_ID_PREFIX,
  summarizeToolCallsText,
} from "./use-chat-flow-drafts";
import type { RoundState } from "./use-chat-flow-types";
import { readMessagePlainText } from "./use-chat-flow-utils";

type UseChatFlowStopOptions = {
  chatting: Ref<boolean>;
  latestAssistantText: Ref<string>;
  latestReasoningStandardText: Ref<string>;
  latestReasoningInlineText: Ref<string>;
  toolStatusText: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  allMessages: Ref<ChatMessage[]>;
  getSession: () => { apiConfigId: string; agentId: string; departmentId?: string } | null;
  getConversationId?: () => string;
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
  onReloadMessages: () => Promise<void>;
  t: (key: string, params?: Record<string, unknown>) => string;
  getRound: () => RoundState;
  setRound: (next: RoundState) => void;
  advanceGeneration: () => void;
  setSendChatActiveGen: (gen: number) => void;
  clearDeferredRoundCompletion: () => void;
  clearPendingTerminalEvent: () => void;
  setActiveActivationId: (value: string) => void;
  clearFrontendDispatchTimer: () => void;
  getPendingUserDraftId: () => string;
  removeDraft: (draftId: string) => void;
  deleteSendStartedAtMs: (gen: number) => void;
  getStreamToolCallCount: () => number;
  getStreamLastToolName: () => string;
  clearConversationStreamCache: (conversationId?: string | null) => void;
  reasoningStartedAtMs: Ref<number>;
};

function stringifyStopError(error: unknown): string {
  return error instanceof Error
    ? `${error.message}\n${error.stack || ""}`.trim()
    : (() => {
        try {
          return JSON.stringify(error);
        } catch {
          return String(error);
        }
      })();
}

export function useChatFlowStop(options: UseChatFlowStopOptions) {
  async function finishLocalStoppedRound(statusState: "failed" | "" = "") {
    options.advanceGeneration();
    options.setSendChatActiveGen(0);
    options.clearDeferredRoundCompletion();
    options.clearPendingTerminalEvent();
    options.setActiveActivationId("");
    options.clearFrontendDispatchTimer();

    const pendingUserDraftId = options.getPendingUserDraftId();
    if (pendingUserDraftId) {
      options.removeDraft(pendingUserDraftId);
    }

    const round = options.getRound();
    if (round.phase === "streaming") {
      options.removeDraft(round.draftId);
      options.deleteSendStartedAtMs(round.gen);
    } else if (round.phase === "queued") {
      options.removeDraft(`${DRAFT_ASSISTANT_ID_PREFIX}${round.gen}`);
      options.deleteSendStartedAtMs(round.gen);
    }

    options.setRound({ phase: "idle" });
    options.chatting.value = false;
    options.reasoningStartedAtMs.value = 0;
    options.toolStatusState.value = statusState;
    options.toolStatusText.value = statusState
      ? (summarizeToolCallsText(options.getStreamToolCallCount(), options.getStreamLastToolName()) || options.t("status.interrupted"))
      : "";
    options.clearConversationStreamCache(options.getConversationId ? options.getConversationId() : "");
    await options.onReloadMessages();
  }

  async function stopChat() {
    const round = options.getRound();
    if (!options.chatting.value && round.phase !== "queued") return;

    const stopSession = options.getSession();
    const cid = options.getConversationId ? options.getConversationId() : "";
    const activeDraftId = round.phase === "streaming" ? round.draftId : "";
    const activeDraft = activeDraftId
      ? options.allMessages.value.find((message) => String(message?.id || "") === activeDraftId)
      : undefined;
    const activeDraftMeta = ((activeDraft?.providerMeta || {}) as Record<string, unknown>);
    const partialAssistantText = options.latestAssistantText.value || readMessagePlainText(activeDraft);
    const partialReasoningStandard =
      options.latestReasoningStandardText.value || String(activeDraftMeta.reasoningStandard || "");
    const partialReasoningInline =
      options.latestReasoningInlineText.value || String(activeDraftMeta.reasoningInline || "");

    if (round.phase === "queued") {
      await finishLocalStoppedRound();
      if (stopSession && options.invokeStopChatMessage) {
        void options
          .invokeStopChatMessage({
            session: cid ? { ...stopSession, conversationId: cid } : stopSession,
            partialAssistantText,
            partialReasoningStandard,
            partialReasoningInline,
          })
          .catch((error) => {
            const et = stringifyStopError(error);
            console.warn(`[聊天] queued 停止后端中断失败，apiConfigId=${stopSession.apiConfigId}，agentId=${stopSession.agentId}，错误=${et}`);
          });
      }
      return;
    }

    if (stopSession && options.invokeStopChatMessage) {
      try {
        await options.invokeStopChatMessage({
          session: cid ? { ...stopSession, conversationId: cid } : stopSession,
          partialAssistantText,
          partialReasoningStandard,
          partialReasoningInline,
        });
        await finishLocalStoppedRound();
        return;
      } catch (error) {
        const et = stringifyStopError(error);
        console.warn(`[聊天] 停止消息失败，apiConfigId=${stopSession.apiConfigId}，agentId=${stopSession.agentId}，len=${partialAssistantText.length}，错误=${et}`);
      }
    }

    await finishLocalStoppedRound("failed");
  }

  return {
    stopChat,
  };
}

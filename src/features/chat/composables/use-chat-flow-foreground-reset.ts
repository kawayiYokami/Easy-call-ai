import type { Ref } from "vue";
import type { ChatMessage } from "../../../types/app";
import type { RoundState } from "./use-chat-flow-types";

type UseChatFlowForegroundResetOptions = {
  latestUserText: Ref<string>;
  latestUserImages: Ref<Array<{ mime: string; bytesBase64: string }>>;
  latestAssistantText: Ref<string>;
  latestReasoningStandardText: Ref<string>;
  latestReasoningInlineText: Ref<string>;
  toolStatusText: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  streamToolCalls?: Ref<any[]>;
  chatting: Ref<boolean>;
  getConversationId?: () => string;
  getRound: () => RoundState;
  setRound: (next: RoundState) => void;
  tickGeneration: () => void;
  setSendChatActiveGen: (value: number) => void;
  setActiveActivationId: (value: string) => void;
  setDeferredRoundCompletionNull: () => void;
  setPendingTerminalEventNull: () => void;
  resetQueuedStreamingState: () => void;
  clearFrontendDispatchTimer: () => void;
  getPendingUserDraftId: () => string;
  removeDraft: (draftId: string) => void;
  removeAssistantDrafts: () => void;
  finalizeDraft: (draftId: string, finalMessage?: ChatMessage) => void;
  clearConversationStreamCache: (conversationId?: string | null) => void;
  setActiveHistoryMessageCount: (value: number) => void;
  reasoningStartedAtMs: Ref<number>;
};

export function useChatFlowForegroundReset(options: UseChatFlowForegroundResetOptions) {
  function resetDisplayState() {
    options.setDeferredRoundCompletionNull();
    options.resetQueuedStreamingState();
    options.latestUserText.value = "";
    options.latestUserImages.value = [];
    options.latestAssistantText.value = "";
    options.latestReasoningStandardText.value = "";
    options.latestReasoningInlineText.value = "";
    options.toolStatusText.value = "";
    options.toolStatusState.value = "";
    if (options.streamToolCalls) options.streamToolCalls.value = [];
  }

  function clearForegroundRoundState() {
    options.tickGeneration();
    options.setSendChatActiveGen(0);
    options.setActiveActivationId("");
    options.setDeferredRoundCompletionNull();
    options.clearFrontendDispatchTimer();
    const pendingUserDraftId = options.getPendingUserDraftId();
    if (pendingUserDraftId) {
      options.removeDraft(pendingUserDraftId);
    }
    const round = options.getRound();
    if (round.phase === "streaming") {
      options.removeDraft(round.draftId);
    } else if (round.phase === "queued") {
      options.removeDraft(`__draft_assistant__:${round.gen}`);
    }
    options.setRound({ phase: "idle" });
    options.setActiveHistoryMessageCount(0);
    options.chatting.value = false;
    options.reasoningStartedAtMs.value = 0;
    resetDisplayState();
  }

  function clearForegroundRuntimeState() {
    options.tickGeneration();
    const conversationId = options.getConversationId ? options.getConversationId() : "";
    options.setSendChatActiveGen(0);
    options.setActiveActivationId("");
    options.setPendingTerminalEventNull();
    options.setDeferredRoundCompletionNull();
    options.resetQueuedStreamingState();
    options.clearFrontendDispatchTimer();
    const pendingUserDraftId = options.getPendingUserDraftId();
    if (pendingUserDraftId) {
      options.removeDraft(pendingUserDraftId);
    }
    options.removeAssistantDrafts();
    options.setRound({ phase: "idle" });
    options.setActiveHistoryMessageCount(0);
    options.chatting.value = false;
    options.reasoningStartedAtMs.value = 0;
    resetDisplayState();
    options.clearConversationStreamCache(conversationId);
  }

  function freezeForegroundRoundState() {
    options.tickGeneration();
    options.setSendChatActiveGen(0);
    const conversationId = options.getConversationId ? options.getConversationId() : "";
    const round = options.getRound();
    if (round.phase === "streaming") {
      options.clearFrontendDispatchTimer();
      console.info("[聊天流式阶段] 前台冻结并保留流式缓存", {
        conversationId,
        roundGen: round.gen,
        assistantTextLength: String(options.latestAssistantText.value || "").length,
        reasoningStandardLength: String(options.latestReasoningStandardText.value || "").length,
        reasoningInlineLength: String(options.latestReasoningInlineText.value || "").length,
      });
    } else if (round.phase === "queued") {
      options.clearFrontendDispatchTimer();
    }
    const pendingUserDraftId = options.getPendingUserDraftId();
    if (pendingUserDraftId) {
      options.removeDraft(pendingUserDraftId);
    }
    if (round.phase === "streaming") {
      options.finalizeDraft(round.draftId);
    } else if (round.phase === "queued") {
      options.removeDraft(`__draft_assistant__:${round.gen}`);
    }
    options.setRound({ phase: "idle" });
    options.setActiveHistoryMessageCount(0);
    options.chatting.value = false;
    options.reasoningStartedAtMs.value = 0;
    resetDisplayState();
  }

  return {
    clearForegroundRoundState,
    clearForegroundRuntimeState,
    freezeForegroundRoundState,
    resetDisplayState,
  };
}

import type { Ref } from "vue";
import type { ChatMessage } from "../../../types/app";
import type { RoundState } from "./use-chat-flow-types";

type UseChatFlowForegroundResetOptions = {
  latestUserText: Ref<string>;
  latestUserImages: Ref<Array<{ mime: string; bytesBase64: string }>>;
  chatting: Ref<boolean>;
  submitPending?: Ref<boolean>;
  getConversationId?: () => string;
  getRound: () => RoundState;
  setRound: (next: RoundState) => void;
  tickGeneration: () => void;
  setSendChatActiveGen: (value: number) => void;
  setActiveActivationId: (value: string) => void;
  setActiveRoundAgentId: (value: string) => void;
  setDeferredRoundCompletionNull: () => void;
  setPendingTerminalEventNull: () => void;
  resetQueuedStreamingState: () => void;
  clearFrontendDispatchTimer: () => void;
  getPendingUserDraftId: () => string;
  removeMessage: (messageId: string) => void;
  finalizeMessage: (messageId: string, finalMessage?: ChatMessage) => void;
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
  }

  function clearForegroundRoundState() {
    if (options.submitPending) options.submitPending.value = false;
    options.tickGeneration();
    options.setSendChatActiveGen(0);
    options.setActiveActivationId("");
    options.setActiveRoundAgentId("");
    options.setDeferredRoundCompletionNull();
    options.clearFrontendDispatchTimer();
    const pendingUserDraftId = options.getPendingUserDraftId();
    if (pendingUserDraftId) {
      options.removeMessage(pendingUserDraftId);
    }
    const round = options.getRound();
    if (round.phase === "streaming" || round.phase === "queued") {
      // 有内容则 finalize；空气泡才会被 removeMessage 清掉。
      options.finalizeMessage(round.messageId);
      options.removeMessage(round.messageId);
    }
    options.setRound({ phase: "idle" });
    options.setActiveHistoryMessageCount(0);
    options.chatting.value = false;
    options.reasoningStartedAtMs.value = 0;
    resetDisplayState();
  }

  function clearForegroundRuntimeState() {
    if (options.submitPending) options.submitPending.value = false;
    options.tickGeneration();
    const conversationId = options.getConversationId ? options.getConversationId() : "";
    options.setSendChatActiveGen(0);
    options.setActiveActivationId("");
    options.setActiveRoundAgentId("");
    options.setPendingTerminalEventNull();
    options.setDeferredRoundCompletionNull();
    options.resetQueuedStreamingState();
    options.clearFrontendDispatchTimer();
    const pendingUserDraftId = options.getPendingUserDraftId();
    if (pendingUserDraftId) {
      options.removeMessage(pendingUserDraftId);
    }
    options.setRound({ phase: "idle" });
    options.setActiveHistoryMessageCount(0);
    options.chatting.value = false;
    options.reasoningStartedAtMs.value = 0;
    resetDisplayState();
    options.clearConversationStreamCache(conversationId);
  }

  function freezeForegroundRoundState() {
    if (options.submitPending) options.submitPending.value = false;
    options.tickGeneration();
    options.setSendChatActiveGen(0);
    options.setActiveRoundAgentId("");
    const round = options.getRound();
    if (round.phase === "streaming" || round.phase === "queued") {
      options.clearFrontendDispatchTimer();
    }
    const pendingUserDraftId = options.getPendingUserDraftId();
    if (pendingUserDraftId) {
      options.removeMessage(pendingUserDraftId);
    }
    // 最小化/失焦不是流式结束证据；在未确认后端终态前，保留当前前台流式真相。
    if (round.phase === "idle") {
      options.setActiveHistoryMessageCount(0);
      options.chatting.value = false;
      options.reasoningStartedAtMs.value = 0;
      resetDisplayState();
      return;
    }
    options.chatting.value = true;
  }

  return {
    clearForegroundRoundState,
    clearForegroundRuntimeState,
    freezeForegroundRoundState,
    resetDisplayState,
  };
}

import type { Ref } from "vue";
import type { AssistantStreamBlock, ChatMessage } from "../../../types/app";
import type { RoundState } from "./use-chat-flow-types";

type UseChatFlowForegroundResetOptions = {
  latestUserText: Ref<string>;
  latestUserImages: Ref<Array<{ mime: string; bytesBase64: string }>>;
  latestAssistantText: Ref<string>;
  toolStatusText: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  streamBlocks?: Ref<AssistantStreamBlock[]>;
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
    options.toolStatusText.value = "";
    options.toolStatusState.value = "";
    if (options.streamBlocks) options.streamBlocks.value = [];
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
    if (options.submitPending) options.submitPending.value = false;
    options.tickGeneration();
    options.setSendChatActiveGen(0);
    options.setActiveRoundAgentId("");
    const conversationId = options.getConversationId ? options.getConversationId() : "";
    const round = options.getRound();
    if (round.phase === "streaming") {
      options.clearFrontendDispatchTimer();
      // 前台冻结日志已移除
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

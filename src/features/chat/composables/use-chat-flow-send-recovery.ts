import type { Ref } from "vue";
import type { AssistantStreamBlock } from "../../../types/app";
import {
  DRAFT_ASSISTANT_ID_PREFIX,
  DRAFT_USER_ID_PREFIX,
  summarizeToolCallsText,
} from "./use-chat-flow-drafts";
import type { RoundState } from "./use-chat-flow-types";

type SendSession = { apiConfigId: string; agentId: string; departmentId?: string; conversationId?: string };

type UseChatFlowSendRecoveryOptions = {
  chatting: Ref<boolean>;
  submitPending?: Ref<boolean>;
  latestAssistantText: Ref<string>;
  toolStatusText: Ref<string>;
  toolStatusState: Ref<"running" | "done" | "failed" | "">;
  streamBlocks?: Ref<AssistantStreamBlock[]>;
  reasoningStartedAtMs: Ref<number>;
  getRound: () => RoundState;
  setRound: (next: RoundState) => void;
  getSession: () => SendSession | null;
  getHistoryFlushedReceivedGen: () => number;
  setSendChatActiveGenIfCurrent: (gen: number, value: number) => void;
  setActiveRoundAgentId: (value: string) => void;
  clearFrontendDispatchTimer: () => void;
  clearChatErrorText: (conversationId?: string | null) => void;
  setChatErrorText: (text: string, conversationId?: string | null) => void;
  formatRequestFailed: (error: unknown) => string;
  getPendingUserDraftId: () => string;
  removeDraft: (draftId: string) => void;
  deleteSendStartedAtMs: (gen: number) => void;
  failQueuedRoundWithoutDraft: (gen: number, error: unknown) => Promise<void>;
  onReloadMessages: () => Promise<void>;
  t: (key: string, params?: Record<string, unknown>) => string;
};

export function useChatFlowSendRecovery(options: UseChatFlowSendRecoveryOptions) {
  function removePendingDraftsForGen(gen: number) {
    const pendingUserDraftId = options.getPendingUserDraftId();
    if (pendingUserDraftId === `${DRAFT_USER_ID_PREFIX}${gen}`) {
      options.removeDraft(pendingUserDraftId);
    }
    options.removeDraft(`${DRAFT_ASSISTANT_ID_PREFIX}${gen}`);
  }

  function handleAbortedSend(gen: number, sendConversationId: string) {
    options.deleteSendStartedAtMs(gen);
    if (options.submitPending) options.submitPending.value = false;
    options.clearChatErrorText(sendConversationId);
    const round = options.getRound();
    if ((round.phase === "streaming" || round.phase === "queued") && round.gen === gen) {
      options.setRound({ phase: "idle" });
      options.setActiveRoundAgentId("");
      options.clearFrontendDispatchTimer();
    }
    options.chatting.value = false;
    options.reasoningStartedAtMs.value = 0;
  }

  async function handleFailedSend(
    gen: number,
    error: unknown,
    sendSession: SendSession,
    sendConversationId: string,
  ) {
    if (options.submitPending) options.submitPending.value = false;
    console.error("[聊天] 聊天流程请求失败", {
      action: "sendChat",
      apiConfigId: sendSession.apiConfigId,
      agentId: sendSession.agentId,
      gen,
      message: String((error as { message?: string })?.message ?? error ?? ""),
    });

    const round = options.getRound();
    if (round.phase === "idle" || round.gen !== gen) {
      removePendingDraftsForGen(gen);
      options.deleteSendStartedAtMs(gen);
      options.clearFrontendDispatchTimer();
      options.setChatErrorText(options.formatRequestFailed(error), sendConversationId);
      return;
    }

    options.latestAssistantText.value = "";
    options.setChatErrorText(options.formatRequestFailed(error), sendConversationId);
    if (!options.toolStatusText.value) {
      options.toolStatusState.value = "failed";
      options.toolStatusText.value =
        summarizeToolCallsText(options.streamBlocks?.value || [])
        || options.t("status.toolCallFailed");
    }

    const cur = options.getSession();
    if (!cur || cur.apiConfigId !== sendSession.apiConfigId || cur.agentId !== sendSession.agentId) {
      return;
    }

    const latestRound = options.getRound();
    if (latestRound.phase === "streaming" && latestRound.gen === gen) {
      options.removeDraft(latestRound.draftId);
      const pendingUserDraftId = options.getPendingUserDraftId();
      if (pendingUserDraftId === `${DRAFT_USER_ID_PREFIX}${gen}`) {
        options.removeDraft(pendingUserDraftId);
      }
      options.deleteSendStartedAtMs(gen);
      options.setRound({ phase: "idle" });
      options.setActiveRoundAgentId("");
      options.clearFrontendDispatchTimer();
      options.chatting.value = false;
      options.reasoningStartedAtMs.value = 0;
    } else if (latestRound.phase === "queued" && latestRound.gen === gen) {
      await options.failQueuedRoundWithoutDraft(gen, error);
    }
  }

  async function finalizeSendChat(gen: number, suppressInitialReload?: boolean) {
    options.setSendChatActiveGenIfCurrent(gen, 0);
    const round = options.getRound();
    if (round.phase === "queued" && round.gen === gen && options.getHistoryFlushedReceivedGen() !== gen) {
      if (options.submitPending) options.submitPending.value = false;
      removePendingDraftsForGen(gen);
      options.deleteSendStartedAtMs(gen);
      options.setRound({ phase: "idle" });
      options.setActiveRoundAgentId("");
      options.clearFrontendDispatchTimer();
      options.chatting.value = false;
      options.reasoningStartedAtMs.value = 0;
      if (!suppressInitialReload) {
        await options.onReloadMessages();
      }
    }
  }

  return {
    finalizeSendChat,
    handleAbortedSend,
    handleFailedSend,
  };
}

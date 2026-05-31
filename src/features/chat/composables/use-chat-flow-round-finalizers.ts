import type { ChatMessage } from "../../../types/app";
import { DRAFT_ASSISTANT_ID_PREFIX, DRAFT_USER_ID_PREFIX, summarizeToolCallsText as formatToolCallsText } from "./use-chat-flow-drafts";
import { mergeAssistantText } from "./use-chat-flow-text";

export function useChatFlowRoundFinalizers(bindings: Record<string, any>) {
  function finalizeDeferredRoundCompletion() {
    const deferredRoundCompletion = bindings.getDeferredRoundCompletion();
    const round = bindings.getRound();
    if (!deferredRoundCompletion) return;
    if (round.phase !== "streaming" || round.gen !== deferredRoundCompletion.gen) {
      bindings.setDeferredRoundCompletion(null);
      return;
    }
    const { draftId } = round;
    const { result } = deferredRoundCompletion;
    bindings.setDeferredRoundCompletion(null);

    bindings.latestAssistantText.value = mergeAssistantText(
      bindings.latestAssistantText.value,
      String(result.assistantText || ""),
    );

    if (typeof result.reasoningStandard === "string") {
      bindings.latestReasoningStandardText.value = result.reasoningStandard;
      bindings.setPendingReasoningStandardBreak(false);
    }
    if (typeof result.reasoningInline === "string") {
      bindings.latestReasoningInlineText.value = result.reasoningInline;
    }
    bindings.clearChatErrorText();
    if (String(bindings.toolStatusState.value || "") === "running") {
      bindings.toolStatusState.value = "done";
      bindings.toolStatusText.value = formatToolCallsText(
        bindings.getStreamToolCallCount(),
        bindings.getStreamLastToolName(),
      ) || bindings.t("status.toolCallDone");
    }

    bindings.updateDraftText(draftId);
    bindings.finalizeDraft(draftId, result.assistantMessage);
    bindings.clearConversationStreamCache(bindings.getConversationId ? bindings.getConversationId() : "");
    bindings.clearFrontendDispatchTimer();
    bindings.setActiveActivationId("");
    bindings.setRound({ phase: "idle" });
    bindings.chatting.value = false;
    bindings.reasoningStartedAtMs.value = 0;
  }

  async function finalizeQueuedRoundWithoutDraft(
    gen: number,
    result: {
      assistantText: string;
      reasoningStandard?: string;
      reasoningInline?: string;
      assistantMessage?: ChatMessage;
    },
  ) {
    void result;
    bindings.sendStartedAtMsByGen.delete(gen);
    const round = bindings.getRound();
    if (round.phase !== "queued" || round.gen !== gen) return;
    bindings.removeDraft(`${DRAFT_ASSISTANT_ID_PREFIX}${gen}`);
    bindings.setPendingTerminalEvent(null);
    bindings.setDeferredRoundCompletion(null);
    bindings.setQueuedStreamingState(null);
    bindings.clearConversationStreamCache(bindings.getConversationId ? bindings.getConversationId() : "");
    bindings.clearFrontendDispatchTimer();
    bindings.setActiveActivationId("");
    bindings.clearChatErrorText();
    bindings.setRound({ phase: "idle" });
    bindings.chatting.value = false;
    bindings.reasoningStartedAtMs.value = 0;
    await bindings.onReloadMessages();
  }

  async function failQueuedRoundWithoutDraft(gen: number, error: unknown) {
    bindings.sendStartedAtMsByGen.delete(gen);
    const round = bindings.getRound();
    if (round.phase !== "queued" || round.gen !== gen) return;
    bindings.removeDraft(`${DRAFT_ASSISTANT_ID_PREFIX}${gen}`);
    bindings.setPendingTerminalEvent(null);
    bindings.setDeferredRoundCompletion(null);
    bindings.setQueuedStreamingState(null);
    bindings.clearConversationStreamCache(bindings.getConversationId ? bindings.getConversationId() : "");
    bindings.clearFrontendDispatchTimer();
    bindings.setActiveActivationId("");
    bindings.latestAssistantText.value = "";
    bindings.latestReasoningStandardText.value = "";
    bindings.latestReasoningInlineText.value = "";
    bindings.setPendingReasoningStandardBreak(false);
    bindings.setChatErrorText(bindings.formatRequestFailed(error));
    if (!bindings.toolStatusText.value) {
      bindings.toolStatusState.value = "failed";
      bindings.toolStatusText.value = formatToolCallsText(
        bindings.getStreamToolCallCount(),
        bindings.getStreamLastToolName(),
      ) || bindings.t("status.toolCallFailed");
    }
    const pendingUserDraftId = bindings.getPendingUserDraftId();
    if (pendingUserDraftId === `${DRAFT_USER_ID_PREFIX}${gen}`) {
      bindings.removeDraft(pendingUserDraftId);
    }
    bindings.setRound({ phase: "idle" });
    bindings.chatting.value = false;
    bindings.reasoningStartedAtMs.value = 0;
    await bindings.onReloadMessages();
  }

  function enqueueStreamDelta(gen: number, delta: string) {
    const round = bindings.getRound();
    if (round.phase !== "streaming" || round.gen !== gen || !delta) return;
    bindings.applyAssistantDeltaToDraft(round.draftId, delta);
    finalizeDeferredRoundCompletion();
  }

  return {
    finalizeDeferredRoundCompletion,
    finalizeQueuedRoundWithoutDraft,
    failQueuedRoundWithoutDraft,
    enqueueStreamDelta,
  };
}

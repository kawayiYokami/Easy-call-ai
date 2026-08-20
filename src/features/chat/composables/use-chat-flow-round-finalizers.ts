import type { ChatMessage } from "../../../types/app";
import { assistantContentBlocksFromMessage } from "../../../utils/chat-message-semantics";
import { assistantMessageHasCanonicalVisibleContent } from "./chat-message-state-machine";
import { DRAFT_USER_ID_PREFIX, summarizeToolCallsText as formatToolCallsText } from "./use-chat-flow-drafts";

type RoundIdentity = { activationId?: string; requestId?: string };

function normalizedRoundIdentity(identity?: RoundIdentity): RoundIdentity | undefined {
  const activationId = String(identity?.activationId || "").trim();
  const requestId = String(identity?.requestId || "").trim();
  if (!activationId && !requestId) return undefined;
  return {
    ...(activationId ? { activationId } : {}),
    ...(requestId ? { requestId } : {}),
  };
}

export function useChatFlowRoundFinalizers(bindings: Record<string, any>) {
  function finalizeMessage(
    messageId: string,
    finalMessage?: ChatMessage,
    identity?: RoundIdentity,
  ) {
    const normalizedIdentity = normalizedRoundIdentity(identity);
    if (normalizedIdentity) {
      bindings.finalizeMessage(messageId, finalMessage, normalizedIdentity);
      return;
    }
    bindings.finalizeMessage(messageId, finalMessage);
  }

  function failMessage(
    messageId: string,
    error: unknown,
    identity?: RoundIdentity,
  ) {
    const normalizedIdentity = normalizedRoundIdentity(identity);
    if (normalizedIdentity) {
      bindings.failMessage(messageId, error, normalizedIdentity);
      return;
    }
    bindings.failMessage(messageId, error);
  }

  function completeQueuedRoundCleanup() {
    bindings.setPendingTerminalEvent(null);
    bindings.setDeferredRoundCompletion(null);
    bindings.setQueuedStreamingState(null);
    bindings.clearConversationStreamCache(bindings.getConversationId ? bindings.getConversationId() : "");
    bindings.submitPending && (bindings.submitPending.value = false);
    bindings.clearFrontendDispatchTimer();
    bindings.setActiveActivationId("");
    bindings.setActiveRoundAgentId?.("");
    bindings.clearChatErrorText();
    bindings.setRound({ phase: "idle" });
    bindings.chatting.value = false;
    bindings.reasoningStartedAtMs.value = 0;
  }

  async function resolveCanonicalAssistantMessage(
    messageId: string,
    resultMessage?: ChatMessage,
    shouldContinue?: () => boolean,
  ): Promise<ChatMessage | undefined> {
    if (assistantMessageHasCanonicalVisibleContent(resultMessage)) {
      return resultMessage;
    }
    const conversationId = String(
      bindings.getConversationId ? bindings.getConversationId() : "",
    ).trim();
    if (conversationId && bindings.refreshMessageById) {
      try {
        await bindings.refreshMessageById({ conversationId, messageId });
      } catch (error) {
        console.warn("[聊天] 完成态按消息 ID 回读失败", {
          conversationId,
          messageId,
          message: String((error as { message?: string })?.message ?? error ?? ""),
        });
      }
    }
    if (shouldContinue && !shouldContinue()) return undefined;
    const refreshedMessage = Array.isArray(bindings.allMessages?.value)
      ? bindings.allMessages.value.find((message: ChatMessage) => message.id === messageId)
      : undefined;
    if (assistantMessageHasCanonicalVisibleContent(refreshedMessage)) return refreshedMessage;
    if (shouldContinue && !shouldContinue()) return undefined;
    if (bindings.onReloadMessages) {
      try {
        await bindings.onReloadMessages();
      } catch (error) {
        console.warn("[聊天] 完成态回读失败后重载会话失败", {
          conversationId,
          messageId,
          message: String((error as { message?: string })?.message ?? error ?? ""),
        });
      }
    }
    if (shouldContinue && !shouldContinue()) return undefined;
    const reloadedMessage = Array.isArray(bindings.allMessages?.value)
      ? bindings.allMessages.value.find((message: ChatMessage) => message.id === messageId)
      : undefined;
    return assistantMessageHasCanonicalVisibleContent(reloadedMessage) ? reloadedMessage : undefined;
  }

  async function finalizeDeferredRoundCompletion() {
    const deferredRoundCompletion = bindings.getDeferredRoundCompletion();
    const round = bindings.getRound();
    if (!deferredRoundCompletion) return;
    if (round.phase !== "streaming" || round.gen !== deferredRoundCompletion.gen) {
      bindings.setDeferredRoundCompletion(null);
      return;
    }
    const { messageId } = round;
    const { result } = deferredRoundCompletion;
    bindings.setDeferredRoundCompletion(null);

    bindings.clearChatErrorText();
    const messageBeforeStatus = Array.isArray(bindings.allMessages?.value)
      ? bindings.allMessages.value.find((message: ChatMessage) => message.id === messageId)
      : undefined;
    const messageBeforeStatusMeta = (messageBeforeStatus?.providerMeta || {}) as Record<string, unknown>;
    if (String(messageBeforeStatusMeta._toolStatusState || "") === "running") {
      bindings.updateMessageText(messageId, undefined, undefined, {
        toolStatusText: formatToolCallsText(
          assistantContentBlocksFromMessage(messageBeforeStatus),
        ) || bindings.t("status.toolCallDone"),
        toolStatusState: "done",
      });
    }

    const existingMessage = Array.isArray(bindings.allMessages?.value)
      ? bindings.allMessages.value.find((message: ChatMessage) => message.id === messageId)
      : undefined;
    if (
      assistantMessageHasCanonicalVisibleContent(existingMessage)
      || assistantMessageHasCanonicalVisibleContent(result.assistantMessage)
    ) {
      finalizeMessage(messageId, result.assistantMessage, {
        activationId: result.activationId,
        requestId: result.requestId,
      });
    } else {
      finalizeMessage(messageId, undefined, {
        activationId: result.activationId,
        requestId: result.requestId,
      });
      const canonicalAssistantMessage = await resolveCanonicalAssistantMessage(
        messageId,
        result.assistantMessage,
        () => {
          const latest = bindings.getRound();
          return (latest.phase === "streaming" || latest.phase === "settling")
            && latest.gen === deferredRoundCompletion.gen
            && latest.messageId === messageId;
        },
      );
      const latestRound = bindings.getRound();
      if (
        latestRound.phase !== "streaming"
        && latestRound.phase !== "settling"
        || latestRound.gen !== deferredRoundCompletion.gen
      ) return;
      if (canonicalAssistantMessage) {
        finalizeMessage(messageId, canonicalAssistantMessage, {
          activationId: result.activationId,
          requestId: result.requestId,
        });
      } else {
        console.warn("[聊天] 完成态回读与重载后仍缺少可见正式消息，清理空投影", {
          conversationId: String(bindings.getConversationId ? bindings.getConversationId() : "").trim(),
          messageId,
          gen: deferredRoundCompletion.gen,
        });
        bindings.removeMessage(messageId);
      }
    }
    bindings.clearConversationStreamCache(bindings.getConversationId ? bindings.getConversationId() : "");
    bindings.submitPending && (bindings.submitPending.value = false);
    bindings.clearFrontendDispatchTimer();
    bindings.setActiveActivationId("");
    bindings.setActiveRoundAgentId?.("");
    bindings.setRound({ phase: "idle" });
    bindings.chatting.value = false;
    bindings.reasoningStartedAtMs.value = 0;
  }

  async function finalizeQueuedRoundWithoutMessage(
    gen: number,
    result: {
      assistantText: string;
      assistantMessage?: ChatMessage;
      activationId?: string;
      requestId?: string;
    },
  ) {
    bindings.sendStartedAtMsByGen.delete(gen);
    const round = bindings.getRound();
    if (round.phase !== "queued" || round.gen !== gen) return;
    if ((result as { skipCanonicalReadback?: boolean }).skipCanonicalReadback === true) {
      // 上下文压缩会立刻以新的 assistant message id 发出 round_started。
      // 旧轮次没有正式消息时直接移除空投影，避免无意义回读与新轮次竞态。
      bindings.removeMessage(round.messageId);
      completeQueuedRoundCleanup();
      return;
    }
    const existingMessage = Array.isArray(bindings.allMessages?.value)
      ? bindings.allMessages.value.find((message: ChatMessage) => message.id === round.messageId)
      : undefined;
    if (
      !assistantMessageHasCanonicalVisibleContent(existingMessage)
      && !assistantMessageHasCanonicalVisibleContent(result.assistantMessage)
    ) {
      finalizeMessage(round.messageId, undefined, {
        activationId: result.activationId,
        requestId: result.requestId,
      });
    }
    const canonicalAssistantMessage = await resolveCanonicalAssistantMessage(
      round.messageId,
      result.assistantMessage,
      () => {
        const latest = bindings.getRound();
        return latest.phase === "queued"
          && latest.gen === gen
          && latest.messageId === round.messageId;
      },
    );
    const latestRound = bindings.getRound();
    if (latestRound.phase !== "queued" || latestRound.gen !== gen) return;
    if (canonicalAssistantMessage) {
      finalizeMessage(round.messageId, canonicalAssistantMessage, {
        activationId: result.activationId,
        requestId: result.requestId,
      });
    } else {
      console.warn("[聊天] 完成态回读与重载后仍缺少可见正式消息，清理空投影", {
        conversationId: String(bindings.getConversationId ? bindings.getConversationId() : "").trim(),
        messageId: round.messageId,
        gen,
      });
      bindings.removeMessage(round.messageId);
    }
    completeQueuedRoundCleanup();
  }

  async function failQueuedRoundWithoutMessage(
    gen: number,
    error: unknown,
    identity?: { activationId?: string; requestId?: string },
  ) {
    bindings.sendStartedAtMsByGen.delete(gen);
    const round = bindings.getRound();
    if (round.phase !== "queued" || round.gen !== gen) return;
    bindings.setPendingTerminalEvent(null);
    bindings.setDeferredRoundCompletion(null);
    bindings.setQueuedStreamingState(null);
    bindings.clearConversationStreamCache(bindings.getConversationId ? bindings.getConversationId() : "");
    bindings.submitPending && (bindings.submitPending.value = false);
    bindings.clearFrontendDispatchTimer();
    bindings.setActiveActivationId("");
    bindings.setActiveRoundAgentId?.("");
    bindings.setChatErrorText(bindings.formatRequestFailed(error));
    const failedMessage = Array.isArray(bindings.allMessages?.value)
      ? bindings.allMessages.value.find((message: ChatMessage) => message.id === round.messageId)
      : undefined;
    const failedMeta = (failedMessage?.providerMeta || {}) as Record<string, unknown>;
    if (!String(failedMeta._toolStatusText || "").trim()) {
      bindings.updateMessageText(round.messageId, undefined, undefined, {
        toolStatusText: formatToolCallsText(
          assistantContentBlocksFromMessage(failedMessage),
        ) || bindings.t("status.toolCallFailed"),
        toolStatusState: "failed",
      });
    } else {
      bindings.updateMessageText(round.messageId);
    }
    // failed 只清理空气泡；一旦已经有可见内容，就由共享状态机保留并结束流式态。
    failMessage(round.messageId, error, identity);
    const pendingUserDraftId = bindings.getPendingUserDraftId();
    if (pendingUserDraftId === `${DRAFT_USER_ID_PREFIX}${gen}`) {
      bindings.removeMessage(pendingUserDraftId);
    }
    bindings.setRound({ phase: "idle" });
    bindings.chatting.value = false;
    bindings.reasoningStartedAtMs.value = 0;
  }

  function enqueueStreamDelta(gen: number, delta: string) {
    const round = bindings.getRound();
    if (round.phase !== "streaming" || round.gen !== gen || !delta) return;
    bindings.applyAssistantDeltaToMessage(round.messageId, delta);
    void finalizeDeferredRoundCompletion();
  }

  return {
    finalizeDeferredRoundCompletion,
    finalizeQueuedRoundWithoutMessage,
    failQueuedRoundWithoutMessage,
    enqueueStreamDelta,
  };
}

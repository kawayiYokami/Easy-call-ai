import { invokeTauri } from "../../../services/tauri-api";
import { useChatFlow } from "./use-chat-flow";
import { useChatRewindActions } from "./use-chat-rewind-actions";
import { useConfirmPlan } from "./use-confirm-plan";

export function useChatRuntimeSetup(bindings: Record<string, any>) {
  let chatFlowRef: any = null;

  const chatFlow = useChatFlow({
      chatting: bindings.chatting,
      trimming: bindings.trimming,
      getSession: () => {
        const apiConfigId = String(bindings.currentForegroundApiConfigId.value || "").trim();
        const agentId = String(bindings.currentForegroundAgentId.value || "").trim();
        const departmentId = String(bindings.currentForegroundDepartmentId.value || "").trim();
        if (!apiConfigId || !agentId) return null;
        return { apiConfigId, agentId, departmentId };
      },
      getConversationId: () => String(bindings.currentChatConversationId.value || "").trim(),
      chatInput: bindings.chatInput,
      selectedInstructionPrompts: bindings.selectedInstructionPrompts,
      selectedMentions: bindings.selectedChatMentions,
      clipboardImages: bindings.clipboardImages,
      queuedAttachmentNotices: bindings.queuedAttachmentNotices,
      latestUserText: bindings.latestUserText,
      latestUserImages: bindings.latestUserImages,
      latestAssistantText: bindings.latestAssistantText,
      latestReasoningStandardText: bindings.latestReasoningStandardText,
      latestReasoningInlineText: bindings.latestReasoningInlineText,
      toolStatusText: bindings.toolStatusText,
      toolStatusState: bindings.toolStatusState,
      streamToolCalls: bindings.streamToolCalls,
      chatErrorText: bindings.chatErrorText,
      setConversationChatError: bindings.setConversationChatErrorText,
      allMessages: bindings.allMessages,
      onOwnUserDraftInserted: () => {
        bindings.bumpOwnUserDraftAlign();
      },
      t: bindings.tr,
      formatRequestFailed: (error: unknown) => bindings.formatRequestFailed(error),
      removeBinaryPlaceholders: bindings.removeBinaryPlaceholders,
      invokeSendChatMessage: ({ text, displayText, images, attachments, extraTextBlocks, mentions, session, traceId, onDelta }) =>
        invokeTauri(
          "submit_chat_message",
          {
            input: {
              payload: {
                text,
                displayText,
                images,
                attachments: attachments && attachments.length > 0 ? attachments : undefined,
                extraTextBlocks: extraTextBlocks && extraTextBlocks.length > 0 ? extraTextBlocks : undefined,
                mentions: Array.isArray(mentions) && mentions.length > 0
                  ? mentions.map((item: any) => ({
                      agentId: item.agentId,
                      agentName: item.agentName,
                      departmentId: item.departmentId,
                      departmentName: item.departmentName,
                    }))
                  : undefined,
              },
              session: {
                apiConfigId: session.apiConfigId,
                agentId: session.agentId,
                departmentId: session.departmentId || null,
                conversationId: session.conversationId || null,
              },
              traceId,
            },
            onDelta,
          },
        ),
      invokeStopChatMessage: ({ session, partialAssistantText, partialReasoningStandard, partialReasoningInline }) =>
        invokeTauri("stop_chat_message", {
          input: {
            session: {
              apiConfigId: session.apiConfigId,
              agentId: session.agentId,
              departmentId: session.departmentId || null,
              conversationId: session.conversationId || null,
            },
            partialAssistantText,
            partialReasoningStandard,
            partialReasoningInline,
          },
        }),
      invokeBindActiveChatViewStream: ({ conversationId, onDelta }) =>
        invokeTauri("bind_active_chat_view_stream", {
          input: {
            conversationId: conversationId || null,
          },
          onDelta,
        }),
      onReloadMessages: () => bindings.reloadForegroundConversationMessages("chat_flow_reload"),
      onHistoryFlushed: async ({ conversationId, pendingMessages }) => {
        const flushedConversationId = String(conversationId || "").trim();
        if (flushedConversationId && bindings.isChatWindowActiveNow()) {
          bindings.currentChatConversationId.value = flushedConversationId;
        }
        const queueMessages = Array.isArray(pendingMessages) ? pendingMessages : [];
        if (queueMessages.length > 0) {
          const fastPathResult = bindings.applySingleOwnUserHistoryFlushFastPath(queueMessages);
          if (fastPathResult) {
            bindings.consumeOrQueueOwnMessageAlign();
            bindings.cacheConversationMessages(
              flushedConversationId || String(bindings.currentChatConversationId.value || "").trim(),
              bindings.allMessages.value,
            );
            return;
          }
          const currentMessages = [...bindings.allMessages.value];
          const dedup = new Set(
            currentMessages
              .filter((message: any) => !bindings.isOptimisticOwnUserDraft(message))
              .map((message: any) => String(message.id || "").trim())
              .filter((id: string) => !!id),
          );
          const uniqueIncoming = queueMessages.filter((message: any) => {
            const id = String(message.id || "").trim();
            if (!id) return true;
            if (dedup.has(id)) return false;
            dedup.add(id);
            return true;
          });
          const prepended = uniqueIncoming.filter((message: any) => {
            const meta = ((message.providerMeta || {}) as Record<string, unknown>);
            const messageMeta = ((meta.message_meta || meta.messageMeta || {}) as Record<string, unknown>);
            return String(messageMeta.kind || "").trim() === "summary_context_seed";
          });
          const appended = uniqueIncoming.filter((message: any) => {
            const meta = ((message.providerMeta || {}) as Record<string, unknown>);
            const messageMeta = ((meta.message_meta || meta.messageMeta || {}) as Record<string, unknown>);
            return String(messageMeta.kind || "").trim() !== "summary_context_seed";
          });
          const appendedOwnUser = appended.filter((message: any) => bindings.isLocalOwnUserMessage(message));
          const appendedOthers = appended.filter((message: any) => !bindings.isLocalOwnUserMessage(message));
          let nextMessages = [...currentMessages];
          if (prepended.length > 0) {
            nextMessages = [...prepended, ...nextMessages];
          }
          if (appendedOwnUser.length > 0) {
            let replacedOwnDraft = false;
            const remainingOwnIncoming = [...appendedOwnUser];
            nextMessages = nextMessages.flatMap((message: any) => {
              if (!replacedOwnDraft && bindings.isOptimisticOwnUserDraft(message)) {
                replacedOwnDraft = true;
                return [remainingOwnIncoming.shift()!];
              }
              return [message];
            });
            if (remainingOwnIncoming.length > 0) {
              nextMessages = bindings.insertMessagesBeforeAssistantDraft(nextMessages, remainingOwnIncoming);
            }
          }
          if (appendedOthers.length > 0) {
            nextMessages = bindings.insertMessagesBeforeAssistantDraft(nextMessages, appendedOthers);
          }
          nextMessages = bindings.reuseStableMessageReferences(nextMessages, bindings.allMessages.value);
          bindings.allMessages.value = nextMessages;
          bindings.foregroundTailLatestReady.value = true;
          const appendedOwnUserMessage = appended.some((message: any) => bindings.isLocalOwnUserMessage(message));
          if (appendedOwnUserMessage) {
            bindings.consumeOrQueueOwnMessageAlign();
          }
        }
        bindings.cacheConversationMessages(
          flushedConversationId || String(bindings.currentChatConversationId.value || "").trim(),
          bindings.allMessages.value,
        );
      },
  });
  const confirmPlan = useConfirmPlan({
      currentApiConfigId: bindings.currentForegroundApiConfigId,
      currentAgentId: bindings.currentForegroundAgentId,
      currentDepartmentId: bindings.currentForegroundDepartmentId,
      currentConversationId: bindings.currentChatConversationId,
      chatting: bindings.chatting,
      trimming: bindings.trimming,
      compactingConversation: bindings.compactingConversation,
      setConversationPlanMode: bindings.setConversationPlanMode,
      clearForegroundRuntimeState: () => {
        chatFlowRef?.clearForegroundRuntimeState();
      },
      confirmPlanAndContinue: ({ conversationId, planMessageId, departmentId, agentId }) => invokeTauri<void>("confirm_plan_and_continue", {
        input: {
          conversationId,
          planMessageId,
          departmentId: departmentId || null,
          agentId: agentId || null,
        },
      }),
  });
  const rewindActions = useChatRewindActions({
      activeApiConfigId: bindings.currentForegroundApiConfigId,
      activeAgentId: bindings.currentForegroundAgentId,
      currentConversationId: bindings.currentChatConversationId,
      allMessages: bindings.allMessages,
      maybeUpdateConversationOverviewFromLoadedMessages: bindings.maybeUpdateForegroundConversationOverviewFromLoadedMessages,
      chatting: bindings.chatting,
      trimming: bindings.trimming,
      compactingConversation: bindings.compactingConversation,
      chatInput: bindings.chatInput,
      selectedMentions: bindings.selectedChatMentions,
      clipboardImages: bindings.clipboardImages,
      deleteUnarchivedConversationFromArchives: bindings.deleteUnarchivedConversationFromArchives,
      sendChat: bindings.sendChatFromCurrentWindow,
      setStatusError: bindings.setStatusError,
      setChatErrorText: (text: string) => {
        bindings.chatErrorText.value = text;
      },
      removeBinaryPlaceholders: bindings.removeBinaryPlaceholders,
      messageText: bindings.messageText,
      extractMessageImages: bindings.extractMessageImages,
      requestRecallMode: bindings.requestRecallMode,
      stopChat: chatFlow.stopChat,
      clearForegroundRuntimeState: chatFlow.clearForegroundRuntimeState,
  });

  chatFlowRef = chatFlow;

  return {
    chatFlow,
    handleConfirmPlan: confirmPlan.handleConfirmPlan,
    deleteUnarchivedConversation: rewindActions.deleteUnarchivedConversation,
    handleRecallTurn: rewindActions.handleRecallTurn,
    handleRegenerateTurn: rewindActions.handleRegenerateTurn,
  };
}

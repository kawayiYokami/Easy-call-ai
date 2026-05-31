import type { ChatMessage, PersonaProfile } from "../../../types/app";

export function buildPersonasSnapshotJson(personas: PersonaProfile[]) {
  return JSON.stringify(
    personas.map((item) => ({
      id: item.id,
      name: item.name,
      systemPrompt: item.systemPrompt,
      privateMemoryEnabled: !!item.privateMemoryEnabled,
      avatarPath: item.avatarPath || "",
      avatarUpdatedAt: item.avatarUpdatedAt || "",
      isBuiltInUser: !!item.isBuiltInUser,
      isBuiltInSystem: !!item.isBuiltInSystem,
      source: item.source || "",
      scope: item.scope || "",
      tools: (item.tools || []).map((tool) => ({
        id: tool.id,
        enabled: !!tool.enabled,
        command: tool.command || "",
        args: Array.isArray(tool.args) ? [...tool.args] : [],
        values: tool.values ?? null,
      })),
    })),
  );
}

export function useChatWindowMessageHelpers(bindings: Record<string, any>) {
  function syncUserAliasFromPersona() {
    const next = (bindings.userPersona.value?.name || "").trim() || bindings.t("archives.roleUser");
    if (bindings.userAlias.value !== next) {
      bindings.userAlias.value = next;
    }
  }

  function isLocalOwnUserMessage(message?: ChatMessage | null): boolean {
    if (!message || message.role !== "user") return false;
    const meta = (message.providerMeta || {}) as Record<string, unknown>;
    const origin = meta.origin as Record<string, unknown> | undefined;
    if (origin && origin.kind === "remote_im") return false;
    const speakerAgentId = String(message.speakerAgentId || meta.speakerAgentId || meta.speaker_agent_id || "").trim();
    return !speakerAgentId || speakerAgentId === "user-persona";
  }

  function isOptimisticOwnUserDraft(message?: ChatMessage | null): boolean {
    if (!message || message.role !== "user") return false;
    const messageId = String(message.id || "").trim();
    if (messageId.startsWith("__draft_user__:")) return true;
    const meta = (message.providerMeta || {}) as Record<string, unknown>;
    return meta._optimistic === true && isLocalOwnUserMessage(message);
  }

  function historyFlushedMessageKind(message?: ChatMessage | null): string {
    const meta = (message?.providerMeta || {}) as Record<string, unknown>;
    const messageMeta = ((meta.message_meta || meta.messageMeta || {}) as Record<string, unknown>);
    return String(messageMeta.kind || meta.messageKind || "").trim();
  }

  function applySingleOwnUserHistoryFlushFastPath(messages: ChatMessage[]): { messageId: string } | null {
    if (messages.length !== 1) return null;
    const committedMessage = messages[0];
    if (!isLocalOwnUserMessage(committedMessage)) return null;
    if (historyFlushedMessageKind(committedMessage) === "summary_context_seed") return null;

    const draftIndex = bindings.allMessages.value.findIndex((message: ChatMessage) => isOptimisticOwnUserDraft(message));
    if (draftIndex < 0) return null;

    const committedId = String(committedMessage.id || "").trim();
    if (committedId) {
      const existingIndex = bindings.allMessages.value.findIndex(
        (message: ChatMessage, index: number) => index !== draftIndex && String(message.id || "").trim() === committedId,
      );
      if (existingIndex >= 0) {
        bindings.allMessages.value.splice(draftIndex, 1);
        bindings.foregroundTailLatestReady.value = true;
        return { messageId: committedId };
      }
    }

    bindings.allMessages.value.splice(draftIndex, 1, committedMessage);
    bindings.foregroundTailLatestReady.value = true;
    return { messageId: committedId };
  }

  return {
    syncUserAliasFromPersona,
    isLocalOwnUserMessage,
    isOptimisticOwnUserDraft,
    historyFlushedMessageKind,
    applySingleOwnUserHistoryFlushFastPath,
  };
}

import type { Ref } from "vue";
import type { AssistantStreamBlock, ChatMentionTarget, ChatMessage } from "../../../types/app";
import {
  assistantTextFromStreamBlocks,
  appendTextDeltaToStreamBlocks,
  normalizeAssistantStreamBlocks,
  normalizeChatActivityItems,
  streamBlocksToActivityItems,
  streamBlocksToToolCalls,
  streamBlocksToToolHistoryEvents,
} from "../../../utils/chat-message-semantics";
import { consumeClosedMarkdownBlocks } from "./use-chat-flow-text";
import { readMessagePlainText } from "./use-chat-flow-utils";
import { messageWithStableRenderId, stableRenderIdFromMessage } from "../utils/stable-render-id";

export const DRAFT_ASSISTANT_ID_PREFIX = "__draft_assistant__:";
export const DRAFT_USER_ID_PREFIX = "__draft_user__:";

type UpdateDraftTextOptions = {
  preserveActivityProjection?: boolean;
};

function messageHasActivityEvents(message: ChatMessage): boolean {
  if (normalizeChatActivityItems(message.activityItems).length > 0) return true;
  if (!Array.isArray(message.toolCall)) return false;
  return message.toolCall.some((event) => {
    const raw = event && typeof event === "object" ? event as Record<string, unknown> : null;
    if (!raw) return false;
    if (String(raw.reasoning_content || "").trim()) return true;
    return Array.isArray(raw.tool_calls) && raw.tool_calls.length > 0;
  });
}

type UseChatFlowDraftsOptions = {
  allMessages: Ref<ChatMessage[]>;
  latestUserText: Ref<string>;
  latestAssistantText: Ref<string>;
  toolStatusText: Ref<string>;
  streamBlocks?: Ref<AssistantStreamBlock[]>;
  getActiveRoundAgentId?: () => string;
  getConversationId?: () => string;
  buildImageAttachmentPayload: (
    images: Array<{ mime: string; bytesBase64: string; savedPath?: string }>,
  ) => Array<{ fileName: string; relativePath: string; mime: string }>;
  getSendStartedAtMs: (gen: number) => number;
  getActiveHistoryMessageCount: () => number;
  getFrontendDispatchStartedAtMs: () => number;
  currentFrontendDispatchElapsedMs: () => number;
};

export function useChatFlowDrafts(options: UseChatFlowDraftsOptions) {
  let pendingUserDraftId = "";

  function resolveAssistantDraftSpeakerAgentId(existingDraft?: ChatMessage | null): string {
    const existing = String(existingDraft?.speakerAgentId || "").trim();
    if (existing && existing !== "assistant-draft") return existing;
    const activeRoundAgentId = String(options.getActiveRoundAgentId ? options.getActiveRoundAgentId() : "").trim();
    if (activeRoundAgentId) return activeRoundAgentId;
    return "assistant-draft";
  }

  function getPendingUserDraftId(): string {
    return pendingUserDraftId;
  }

  function getDraftStreamBlocks(draftId: string): AssistantStreamBlock[] {
    if (!draftId) return [];
    const draft = options.allMessages.value.find((item) => item.id === draftId);
    const meta = (draft?.providerMeta || {}) as Record<string, unknown>;
    return normalizeAssistantStreamBlocks(meta._streamBlocks);
  }

  function loadStreamBlocksFromDraft(draftId: string) {
    if (!options.streamBlocks) return;
    if (!draftId) {
      options.streamBlocks.value = [];
      return;
    }
    const draft = options.allMessages.value.find((item) => item.id === draftId);
    const meta = (draft?.providerMeta || {}) as Record<string, unknown>;
    const blocks = normalizeAssistantStreamBlocks(meta._streamBlocks);
    if (blocks.length > 0 || options.streamBlocks.value.length === 0) {
      options.streamBlocks.value = blocks;
    }
  }

  function hasAssistantDraftInMessages(): boolean {
    return options.allMessages.value.some((message) =>
      String(message?.id || "").trim().startsWith(DRAFT_ASSISTANT_ID_PREFIX)
    );
  }

  function insertUserDraft(
    gen: number,
    text: string,
    images: Array<{ mime: string; bytesBase64: string; savedPath?: string }>,
    attachments: Array<{ fileName: string; relativePath: string; mime: string }>,
    mentions: ChatMentionTarget[],
  ): string {
    const draftId = `${DRAFT_USER_ID_PREFIX}${gen}`;
    const parts: ChatMessage["parts"] = [];
    const normalizedText = String(text || "");
    if (normalizedText) {
      parts.push({ type: "text", text: normalizedText });
    }
    for (const image of images) {
      const mime = String(image.mime || "").trim();
      const bytesBase64 = String(image.bytesBase64 || "").trim();
      if (!mime || !bytesBase64) continue;
      parts.push({ type: "image", mime, bytesBase64 });
    }
    const attachmentPayload = [...attachments, ...options.buildImageAttachmentPayload(images)];
    const msg: ChatMessage = {
      id: draftId,
      role: "user",
      createdAt: new Date().toISOString(),
      speakerAgentId: "user-persona",
      parts,
      providerMeta: {
        attachments: attachmentPayload.length > 0 ? attachmentPayload : undefined,
        message_meta: mentions.length > 0
          ? {
              kind: "user_message",
              mentions: mentions.map((item) => ({
                agentId: item.agentId,
                agentName: item.agentName,
                departmentId: item.departmentId,
                departmentName: item.departmentName,
              })),
            }
          : undefined,
        _optimistic: true,
      },
    };
    const stableMsg = messageWithStableRenderId(msg, draftId);
    const cur = options.allMessages.value;
    const idx = cur.findIndex((m) => m.id === draftId);
    options.allMessages.value = idx < 0 ? [...cur, stableMsg] : cur.map((m, i) => (i === idx ? stableMsg : m));
    pendingUserDraftId = draftId;
    return draftId;
  }

  function insertDraft(gen: number, initialText = ""): string {
    const draftId = `${DRAFT_ASSISTANT_ID_PREFIX}${gen}`;
    const startedAtMs = options.getSendStartedAtMs(gen) || 0;
    const elapsedMs = startedAtMs > 0 ? Math.max(0, Date.now() - startedAtMs) : -1;
    console.warn("[聊天前端耗时] 助理草稿出现", {
      gen,
      elapsedMs,
      conversationId: String(options.getConversationId ? options.getConversationId() : "").trim(),
      activeHistoryMessageCount: options.getActiveHistoryMessageCount(),
      latestUserTextLength: String(options.latestUserText.value || "").length,
    });
    const agentId = resolveAssistantDraftSpeakerAgentId();
    const msg = messageWithStableRenderId({
      id: draftId,
      role: "assistant",
      createdAt: new Date().toISOString(),
      speakerAgentId: agentId,
      parts: [{ type: "text", text: "" }],
      providerMeta: {
        _streaming: true,
        _streamSegments: [] as string[],
        _streamTail: "",
        _preStreamingStatusText: String(initialText || ""),
        _frontendDispatchStartedAtMs: options.getFrontendDispatchStartedAtMs(),
        _frontendDispatchElapsedMs: options.currentFrontendDispatchElapsedMs(),
      },
    } satisfies ChatMessage, draftId);
    const cur = options.allMessages.value;
    const idx = cur.findIndex((m) => m.id === draftId);
    if (idx >= 0) {
      options.allMessages.value = cur.map((m, i) => (i === idx ? msg : m));
      return draftId;
    }
    const relatedUserDraftId = `${DRAFT_USER_ID_PREFIX}${gen}`;
    const userDraftIdx = cur.findIndex((m) => m.id === relatedUserDraftId);
    if (userDraftIdx >= 0) {
      options.allMessages.value = [
        ...cur.slice(0, userDraftIdx + 1),
        msg,
        ...cur.slice(userDraftIdx + 1),
      ];
      return draftId;
    }
    options.allMessages.value = [...cur, msg];
    return draftId;
  }

  function updateQueuedAssistantDraftStatus(draftId: string, statusText: string) {
    if (!draftId) return;
    const existingDraft = options.allMessages.value.find((item) => item.id === draftId);
    const agentId = resolveAssistantDraftSpeakerAgentId(existingDraft);
    const existingMeta = ((existingDraft?.providerMeta || {}) as Record<string, unknown>);
    const stableRenderId = stableRenderIdFromMessage(existingDraft) || draftId;
    const msg = messageWithStableRenderId({
      id: draftId,
      role: "assistant",
      createdAt: String(existingDraft?.createdAt || new Date().toISOString()),
      speakerAgentId: agentId,
      parts: [{ type: "text", text: "" }],
      providerMeta: {
        ...existingMeta,
        _streaming: true,
        _streamSegments: [] as string[],
        _streamTail: "",
        _streamAnimatedDelta: "",
        _preStreamingStatusText: String(statusText || ""),
        _frontendDispatchStartedAtMs: options.getFrontendDispatchStartedAtMs(),
        _frontendDispatchElapsedMs: options.currentFrontendDispatchElapsedMs(),
      },
    } satisfies ChatMessage, stableRenderId);
    const cur = options.allMessages.value;
    const idx = cur.findIndex((m) => m.id === draftId);
    if (idx >= 0) {
      options.allMessages.value = cur.map((m, i) => (i === idx ? msg : m));
    } else {
      const gen = Number(String(draftId).split(":").pop() || 0);
      const relatedUserDraftId = `${DRAFT_USER_ID_PREFIX}${gen}`;
      const userDraftIdx = cur.findIndex((m) => m.id === relatedUserDraftId);
      options.allMessages.value = userDraftIdx >= 0
        ? [
            ...cur.slice(0, userDraftIdx + 1),
            msg,
            ...cur.slice(userDraftIdx + 1),
          ]
        : [...cur, msg];
    }
  }

  function readDraftStreamSegments(draftId: string): string[] {
    if (!draftId) return [];
    const draft = options.allMessages.value.find((item) => item.id === draftId);
    const meta = (draft?.providerMeta || {}) as Record<string, unknown>;
    if (!Array.isArray(meta._streamSegments)) return [];
    return (meta._streamSegments as unknown[])
      .map((item) => String(item ?? ""))
      .filter((item) => item.length > 0);
  }

  function readDraftStreamTail(draftId: string): string {
    if (!draftId) return "";
    const draft = options.allMessages.value.find((item) => item.id === draftId);
    const meta = (draft?.providerMeta || {}) as Record<string, unknown>;
    return String(meta._streamTail ?? "");
  }

  function syncStreamBlocksToDraft(draftId: string, rawBlocks?: AssistantStreamBlock[]) {
    if (!draftId) return;
    const blocks = normalizeAssistantStreamBlocks(rawBlocks || options.streamBlocks?.value || []);
    const nextActivityItems = streamBlocksToActivityItems(blocks, true);
    options.allMessages.value = options.allMessages.value.map((message) => {
      if (message.id !== draftId) return message;
      const meta = ((message.providerMeta || {}) as Record<string, unknown>);
      return {
        ...message,
        parts: [{ type: "text", text: assistantTextFromStreamBlocks(blocks) }],
        toolCall: streamBlocksToToolHistoryEvents(blocks),
        activityItems: nextActivityItems.length > 0 ? nextActivityItems : undefined,
        providerMeta: {
          ...meta,
          _streamBlocks: blocks,
        },
      };
    });
  }

  function updateDraftText(
    draftId: string,
    streamSegments?: string[],
    streamTail?: string,
    streamAnimatedDelta = "",
    rawBlocks?: AssistantStreamBlock[],
    updateOptions?: UpdateDraftTextOptions,
  ) {
    if (!draftId) return;
    const existingDraft = options.allMessages.value.find((item) => item.id === draftId);
    const agentId = resolveAssistantDraftSpeakerAgentId(existingDraft);
    const existingDraftText = readMessagePlainText(existingDraft);
    const nextAssistantText = String(options.latestAssistantText.value || "");
    const shouldPreserveExistingDraftText =
      !!existingDraft
      && !nextAssistantText
      && !!existingDraftText
      && (
        !!String(options.toolStatusText.value || "").trim()
        || (options.streamBlocks?.value.length || 0) > 0
      );
    if (shouldPreserveExistingDraftText) {
      options.latestAssistantText.value = existingDraftText;
    }
    const nextStreamSegments = streamSegments || readDraftStreamSegments(draftId);
    const nextStreamTail = streamTail ?? readDraftStreamTail(draftId);
    const hasVisibleStreamContent =
      !!String(options.latestAssistantText.value || "").trim()
      || nextStreamSegments.some((item) => !!String(item || "").trim())
      || !!String(nextStreamTail || "").trim()
      || (options.streamBlocks?.value.length || 0) > 0;
    const preStreamingStatusText = hasVisibleStreamContent
      ? ""
      : String(options.toolStatusText.value || "").trim();
    const streamBlocks = normalizeAssistantStreamBlocks(rawBlocks || options.streamBlocks?.value || []);
    const blockText = assistantTextFromStreamBlocks(streamBlocks);
    const preserveActivityProjection = !!updateOptions?.preserveActivityProjection;
    const existingActivityItems = normalizeChatActivityItems(existingDraft?.activityItems);
    const nextActivityItems = preserveActivityProjection
      ? existingActivityItems
      : streamBlocksToActivityItems(streamBlocks, true);
    const stableRenderId = stableRenderIdFromMessage(existingDraft) || draftId;
    const msg = messageWithStableRenderId({
      id: draftId,
      role: "assistant",
      createdAt: String(existingDraft?.createdAt || new Date().toISOString()),
      speakerAgentId: agentId,
      parts: [{ type: "text", text: blockText || String(options.latestAssistantText.value || "") }],
      toolCall: preserveActivityProjection
        ? existingDraft?.toolCall
        : streamBlocksToToolHistoryEvents(streamBlocks),
      activityItems: nextActivityItems.length > 0 ? nextActivityItems : undefined,
      providerMeta: {
        _streaming: true,
        _streamSegments: nextStreamSegments,
        _streamTail: nextStreamTail,
        _streamAnimatedDelta: String(streamAnimatedDelta || ""),
        _preStreamingStatusText: preStreamingStatusText,
        _frontendDispatchStartedAtMs: options.getFrontendDispatchStartedAtMs(),
        _frontendDispatchElapsedMs: options.currentFrontendDispatchElapsedMs(),
        _streamBlocks: streamBlocks,
      },
    } satisfies ChatMessage, stableRenderId);
    const cur = options.allMessages.value;
    const idx = cur.findIndex((m) => m.id === draftId);
    options.allMessages.value = idx < 0 ? [...cur, msg] : cur.map((m, i) => (i === idx ? msg : m));
  }

  function removeDraft(draftId: string) {
    if (!draftId) return;
    if (draftId === pendingUserDraftId) {
      pendingUserDraftId = "";
    }
    options.allMessages.value = options.allMessages.value.filter((m) => m.id !== draftId);
  }

  function removeAssistantDrafts() {
    options.allMessages.value = options.allMessages.value.filter((message) => {
      const messageId = String(message?.id || "").trim();
      return !messageId.startsWith(DRAFT_ASSISTANT_ID_PREFIX);
    });
  }

  function finalizeDraft(draftId: string, finalMessage?: ChatMessage) {
    if (!draftId) return;
    const current = options.allMessages.value;
    const draftIdx = current.findIndex((m) => m.id === draftId);
    if (draftIdx < 0) return;
    const draft = current[draftIdx];
    const stableRenderId = stableRenderIdFromMessage(draft) || draftId;

    if (finalMessage) {
      const draftSpeakerAgentId = resolveAssistantDraftSpeakerAgentId(draft);
      const finalSpeakerAgentId = String(finalMessage.speakerAgentId || "").trim();
      const messageToApply = messageWithStableRenderId({
        ...finalMessage,
        speakerAgentId: finalSpeakerAgentId || draftSpeakerAgentId,
      }, stableRenderId);
      const deduped = current.filter((m, idx) => idx === draftIdx || m.id !== finalMessage.id);
      const nextDraftIdx = deduped.findIndex((m) => m.id === draftId);
      if (nextDraftIdx < 0) {
        options.allMessages.value = deduped;
        return;
      }
      options.allMessages.value = deduped.map((m, idx) => (idx === nextDraftIdx ? messageToApply : m));
      return;
    }

    const draftMeta = ((draft.providerMeta || {}) as Record<string, unknown>);
    const nextMeta = { ...draftMeta };
    delete (nextMeta as Record<string, unknown>)._streaming;
    const normalized: ChatMessage = { ...draft, providerMeta: nextMeta };
    options.allMessages.value = current.map((m, idx) => (idx === draftIdx ? normalized : m));
  }

  function applyAssistantDeltaToDraft(draftId: string, delta: string) {
    if (!draftId || !delta) return;
    options.latestAssistantText.value += delta;
    if (options.streamBlocks) {
      options.streamBlocks.value = appendTextDeltaToStreamBlocks(options.streamBlocks.value, delta);
    }
    const currentSegments = readDraftStreamSegments(draftId);
    const currentTail = readDraftStreamTail(draftId);
    const parsed = consumeClosedMarkdownBlocks(`${currentTail}${delta}`);
    const nextStreamSegments = parsed.chunks.length > 0
      ? [...currentSegments, ...parsed.chunks]
      : currentSegments;
    updateDraftText(draftId, nextStreamSegments, parsed.tail, delta, undefined, {
      preserveActivityProjection: true,
    });
  }

  return {
    applyAssistantDeltaToDraft,
    finalizeDraft,
    getDraftStreamBlocks,
    getPendingUserDraftId,
    hasAssistantDraftInMessages,
    insertDraft,
    insertUserDraft,
    loadStreamBlocksFromDraft,
    removeAssistantDrafts,
    removeDraft,
    syncStreamBlocksToDraft,
    updateDraftText,
    updateQueuedAssistantDraftStatus,
  };
}

export function summarizeToolCallsText(streamBlocks?: AssistantStreamBlock[]): string {
  const toolCalls = streamBlocksToToolCalls(streamBlocks || []);
  if (toolCalls.length <= 0) return "";
  const lastToolName = toolCalls[toolCalls.length - 1]?.name || "";
  const extraCount = Math.max(0, toolCalls.length - 1);
  return extraCount > 0
    ? `调用 ${lastToolName || "-"} (+${extraCount})`
    : `调用 ${lastToolName || "-"}`;
}

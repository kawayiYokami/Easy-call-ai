import type { Ref } from "vue";
import type { ChatMentionTarget, ChatMessage } from "../../../types/app";
import { consumeClosedMarkdownBlocks } from "./use-chat-flow-text";
import {
  mergeStreamToolCallsForward,
  normalizeStreamToolCallViews,
  type StreamToolCallView,
} from "./use-chat-flow-tool-calls";
import { readMessagePlainText } from "./use-chat-flow-utils";

export const DRAFT_ASSISTANT_ID_PREFIX = "__draft_assistant__:";
export const DRAFT_USER_ID_PREFIX = "__draft_user__:";

type UseChatFlowDraftsOptions = {
  allMessages: Ref<ChatMessage[]>;
  latestUserText: Ref<string>;
  latestAssistantText: Ref<string>;
  latestReasoningStandardText: Ref<string>;
  latestReasoningInlineText: Ref<string>;
  toolStatusText: Ref<string>;
  streamToolCalls?: Ref<StreamToolCallView[]>;
  getSession: () => { apiConfigId: string; agentId: string; departmentId?: string } | null;
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

  function getPendingUserDraftId(): string {
    return pendingUserDraftId;
  }

  function loadStreamToolCallsFromDraft(draftId: string) {
    if (!options.streamToolCalls) return;
    if (!draftId) {
      options.streamToolCalls.value = [];
      return;
    }
    const draft = options.allMessages.value.find((item) => item.id === draftId);
    const meta = (draft?.providerMeta || {}) as Record<string, unknown>;
    const calls = normalizeStreamToolCallViews(meta._streamToolCalls);
    options.streamToolCalls.value = mergeStreamToolCallsForward(options.streamToolCalls.value, calls);
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
    const cur = options.allMessages.value;
    const idx = cur.findIndex((m) => m.id === draftId);
    options.allMessages.value = idx < 0 ? [...cur, msg] : cur.map((m, i) => (i === idx ? msg : m));
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
    const agentId = String(options.getSession()?.agentId || "").trim();
    const msg: ChatMessage = {
      id: draftId,
      role: "assistant",
      createdAt: new Date().toISOString(),
      speakerAgentId: agentId || "assistant-draft",
      parts: [{ type: "text", text: "" }],
      providerMeta: {
        reasoningStandard: "",
        reasoningInline: "",
        _streaming: true,
        _streamSegments: [] as string[],
        _streamTail: "",
        _preStreamingStatusText: String(initialText || ""),
        _frontendDispatchStartedAtMs: options.getFrontendDispatchStartedAtMs(),
        _frontendDispatchElapsedMs: options.currentFrontendDispatchElapsedMs(),
      },
    };
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
    const agentId = String(options.getSession()?.agentId || "").trim();
    const existingDraft = options.allMessages.value.find((item) => item.id === draftId);
    const existingMeta = ((existingDraft?.providerMeta || {}) as Record<string, unknown>);
    const msg: ChatMessage = {
      id: draftId,
      role: "assistant",
      createdAt: String(existingDraft?.createdAt || new Date().toISOString()),
      speakerAgentId: agentId || "assistant-draft",
      parts: [{ type: "text", text: "" }],
      providerMeta: {
        ...existingMeta,
        reasoningStandard: "",
        reasoningInline: "",
        _streaming: true,
        _streamSegments: [] as string[],
        _streamTail: "",
        _streamAnimatedDelta: "",
        _preStreamingStatusText: String(statusText || ""),
        _frontendDispatchStartedAtMs: options.getFrontendDispatchStartedAtMs(),
        _frontendDispatchElapsedMs: options.currentFrontendDispatchElapsedMs(),
      },
    };
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

  function syncStreamToolCallsToDraft(draftId: string) {
    if (!draftId || !options.streamToolCalls) return;
    const calls = options.streamToolCalls.value.map((item) => ({ ...item }));
    options.allMessages.value = options.allMessages.value.map((message) => {
      if (message.id !== draftId) return message;
      const meta = ((message.providerMeta || {}) as Record<string, unknown>);
      return {
        ...message,
        providerMeta: {
          ...meta,
          _streamToolCalls: calls,
        },
      };
    });
  }

  function updateDraftText(
    draftId: string,
    streamSegments?: string[],
    streamTail?: string,
    streamAnimatedDelta = "",
  ) {
    if (!draftId) return;
    const agentId = String(options.getSession()?.agentId || "").trim();
    const existingDraft = options.allMessages.value.find((item) => item.id === draftId);
    const existingDraftText = readMessagePlainText(existingDraft);
    const nextAssistantText = String(options.latestAssistantText.value || "");
    const shouldPreserveExistingDraftText =
      !!existingDraft
      && !nextAssistantText
      && !!existingDraftText
      && (
        !!String(options.toolStatusText.value || "").trim()
        || (options.streamToolCalls?.value.length || 0) > 0
      );
    if (shouldPreserveExistingDraftText) {
      options.latestAssistantText.value = existingDraftText;
    }
    const nextStreamSegments = streamSegments || readDraftStreamSegments(draftId);
    const nextStreamTail = streamTail ?? readDraftStreamTail(draftId);
    const nextReasoningStandard = String(options.latestReasoningStandardText.value || "");
    const nextReasoningInline = String(options.latestReasoningInlineText.value || "");
    const hasVisibleStreamContent =
      !!String(options.latestAssistantText.value || "").trim()
      || !!nextReasoningStandard.trim()
      || !!nextReasoningInline.trim()
      || nextStreamSegments.some((item) => !!String(item || "").trim())
      || !!String(nextStreamTail || "").trim()
      || (options.streamToolCalls?.value.length || 0) > 0;
    const preStreamingStatusText = hasVisibleStreamContent
      ? ""
      : String(options.toolStatusText.value || "").trim();
    const msg: ChatMessage = {
      id: draftId,
      role: "assistant",
      createdAt: String(existingDraft?.createdAt || new Date().toISOString()),
      speakerAgentId: agentId || "assistant-draft",
      parts: [{ type: "text", text: String(options.latestAssistantText.value || "") }],
      providerMeta: {
        reasoningStandard: nextReasoningStandard,
        reasoningInline: nextReasoningInline,
        _streaming: true,
        _streamSegments: nextStreamSegments,
        _streamTail: nextStreamTail,
        _streamAnimatedDelta: String(streamAnimatedDelta || ""),
        _preStreamingStatusText: preStreamingStatusText,
        _frontendDispatchStartedAtMs: options.getFrontendDispatchStartedAtMs(),
        _frontendDispatchElapsedMs: options.currentFrontendDispatchElapsedMs(),
        _streamToolCalls: Array.isArray(options.streamToolCalls?.value)
          ? options.streamToolCalls.value.map((item) => ({ ...item }))
          : [] as StreamToolCallView[],
      },
    };
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

    if (finalMessage) {
      const deduped = current.filter((m, idx) => idx === draftIdx || m.id !== finalMessage.id);
      const nextDraftIdx = deduped.findIndex((m) => m.id === draftId);
      if (nextDraftIdx < 0) {
        options.allMessages.value = deduped;
        return;
      }
      options.allMessages.value = deduped.map((m, idx) => (idx === nextDraftIdx ? finalMessage : m));
      return;
    }

    const draft = current[draftIdx];
    const draftMeta = ((draft.providerMeta || {}) as Record<string, unknown>);
    const nextMeta = { ...draftMeta };
    delete (nextMeta as Record<string, unknown>)._streaming;
    const normalized: ChatMessage = { ...draft, providerMeta: nextMeta };
    options.allMessages.value = current.map((m, idx) => (idx === draftIdx ? normalized : m));
  }

  function applyAssistantDeltaToDraft(draftId: string, delta: string) {
    if (!draftId || !delta) return;
    options.latestAssistantText.value += delta;
    const currentSegments = readDraftStreamSegments(draftId);
    const currentTail = readDraftStreamTail(draftId);
    const parsed = consumeClosedMarkdownBlocks(`${currentTail}${delta}`);
    const nextStreamSegments = parsed.chunks.length > 0
      ? [...currentSegments, ...parsed.chunks]
      : currentSegments;
    updateDraftText(draftId, nextStreamSegments, parsed.tail, delta);
  }

  return {
    applyAssistantDeltaToDraft,
    finalizeDraft,
    getPendingUserDraftId,
    hasAssistantDraftInMessages,
    insertDraft,
    insertUserDraft,
    loadStreamToolCallsFromDraft,
    removeAssistantDrafts,
    removeDraft,
    syncStreamToolCallsToDraft,
    updateDraftText,
    updateQueuedAssistantDraftStatus,
  };
}

export function summarizeToolCallsText(streamToolCallCount: number, streamLastToolName: string): string {
  if (streamToolCallCount <= 0) return "";
  const extraCount = Math.max(0, streamToolCallCount - 1);
  return extraCount > 0
    ? `调用 ${streamLastToolName || "-"} (+${extraCount})`
    : `调用 ${streamLastToolName || "-"}`;
}

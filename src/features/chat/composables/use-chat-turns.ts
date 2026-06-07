import { computed, type ComputedRef, type Ref, type ShallowRef } from "vue";
import type { ApiConfigItem, ChatMessage, ChatMessageBlock } from "../../../types/app";
import type { ContextUsageUpdatePayload } from "./use-chat-flow-events";
import {
  estimateConversationTokens,
} from "../../../utils/chat-message";
import {
  normalizeAssistantStreamBlocks,
  projectMessageForDisplay,
  projectStreamingChatActivityForDisplay,
  streamBlocksToToolCalls,
} from "../../../utils/chat-message-semantics";

function baseActivityForMessage(
  projection: ReturnType<typeof projectMessageForDisplay>,
  isStreaming: boolean,
  streamBlocks: ReturnType<typeof normalizeAssistantStreamBlocks>,
) {
  if (isStreaming) {
    return projectStreamingChatActivityForDisplay({
      activityItems: projection.activityItems,
      streamBlocks,
      running: true,
    });
  }
  return {
    items: projection.activityItems,
    activityReasoningCharCount: projection.activityReasoningCharCount,
    activityToolCountsByName: projection.activityToolCountsByName,
    activityRunning: projection.activityRunning,
    activityStatus: projection.activityStatus,
  };
}

function positiveNumberFromProviderMeta(meta: Record<string, unknown>, key: string): number | undefined {
  const value = Number(meta[key]);
  if (!Number.isFinite(value) || value <= 0) return undefined;
  return Math.round(value);
}

function extraTextReferenceLabel(text: string): string {
  const trimmed = String(text || "").trim();
  const matched = trimmed.match(/^用户引用了文件片段：([^\n（]+)/);
  return matched?.[1]?.trim() || trimmed.split("\n")[0]?.replace(/^用户引用了文件片段：/, "").trim() || "文件片段";
}

function buildExtraTextReferences(message: ChatMessage): Array<{ label: string; text: string }> {
  if (!Array.isArray(message.extraTextBlocks)) return [];
  return message.extraTextBlocks
    .map((raw) => String(raw || "").trim())
    .filter(Boolean)
    .map((text) => ({ label: extraTextReferenceLabel(text), text }));
}

type UseChatMessageBlocksOptions = {
  allMessages: ShallowRef<ChatMessage[]>;
  activeChatApiConfig: ComputedRef<ApiConfigItem | null>;
  currentConversationId?: Ref<string>;
  contextUsagePreview?: Ref<ContextUsageUpdatePayload | null>;
  perfDebug: boolean;
  perfNow: () => number;
  taskTriggerLabels?: { goal: string; todo: string };
};

export function useChatMessageBlocks(options: UseChatMessageBlocksOptions) {
  let lastMessageBlockSignature = "";
  let lastMessageBlocks: ChatMessageBlock[] = [];
  const streamProjectionDebugSignatures = new Map<string, string>();
  const messageSignatureCache = new WeakMap<ChatMessage, string>();
  const messageBlockCache = new WeakMap<ChatMessage, { signature: string; blocks: ChatMessageBlock[] }>();

  function messageSignature(message: ChatMessage): string {
    const cached = messageSignatureCache.get(message);
    if (cached) return cached;
    const signature = [
      String(message.id || "").trim(),
      String(message.createdAt || "").trim(),
      JSON.stringify(message.parts || []),
      JSON.stringify(message.extraTextBlocks || []),
      JSON.stringify(message.providerMeta || {}),
      JSON.stringify(message.toolCall || []),
      JSON.stringify(message.activityItems || []),
    ].join("|");
    messageSignatureCache.set(message, signature);
    return signature;
  }

  function latestBackendContextUsagePercent(messages: ChatMessage[]): number | null {
    for (let idx = messages.length - 1; idx >= 0; idx -= 1) {
      const message = messages[idx];
      if (message.role !== "assistant") continue;
      const raw = Number((message.providerMeta || {}).contextUsagePercent);
      if (!Number.isFinite(raw)) continue;
      return Math.min(100, Math.max(0, Math.round(raw)));
    }
    return null;
  }

  function previewContextUsageRatio(): number | null {
    const preview = options.contextUsagePreview?.value;
    if (!preview) return null;
    const activeConversationId = String(options.currentConversationId?.value || "").trim();
    if (activeConversationId && preview.conversationId !== activeConversationId) return null;
    const ratio = Number(preview.contextUsageRatio);
    if (Number.isFinite(ratio) && ratio >= 0) return ratio;
    const percent = Number(preview.contextUsagePercent);
    if (!Number.isFinite(percent)) return null;
    return Math.max(0, percent) / 100;
  }

  function buildMessageBlocks(message: ChatMessage): ChatMessageBlock[] {
    const providerMeta = (message.providerMeta || {}) as Record<string, unknown>;
    const messageMeta = ((providerMeta.message_meta || providerMeta.messageMeta || {}) as Record<string, unknown>);
    const messageKind = String(messageMeta.kind || providerMeta.messageKind || "").trim();
    if (messageKind === "plan_confirm_continue") {
      return [{
        id: message.id,
        sourceMessageId: message.id,
        isExtraTextBlock: false,
        role: "system",
        dividerKind: "plan_started",
        isStreaming: false,
        streamSegments: [],
        streamTail: "",
        streamAnimatedDelta: "",
        speakerAgentId: undefined,
        createdAt: String(message.createdAt || "").trim() || undefined,
        providerMeta: message.providerMeta,
        mentions: [],
        text: "",
        images: [],
        audios: [],
        attachmentFiles: [],
        extraTextReferences: [],
        inlineSegments: undefined,
        memeSegments: undefined,
        taskTrigger: undefined,
        planCard: undefined,
        remoteImOrigin: undefined,
        dispatchElapsedMs: undefined,
        toolCallCount: 0,
        lastToolName: "",
        toolCalls: [],
        activityItems: [],
        activityReasoningCharCount: 0,
        activityToolCountsByName: {},
        activityRunning: false,
        activityStatus: "idle",
      }];
    }
    const signature = messageSignature(message);
    const cached = messageBlockCache.get(message);
    if (cached && cached.signature === signature) {
      return cached.blocks;
    }

    const meta = (message.providerMeta || {}) as Record<string, unknown>;
    const projection = projectMessageForDisplay(message, options.taskTriggerLabels);
    const streamSegments = Array.isArray(meta._streamSegments)
      ? (meta._streamSegments as unknown[])
        .map((item) => String(item ?? ""))
        .filter((item) => item.length > 0)
      : [];
    const streamTail = String(meta._streamTail ?? "");
    const streamAnimatedDelta = String(meta._streamAnimatedDelta ?? "");
    const streamBlocks = normalizeAssistantStreamBlocks(meta._streamBlocks);
    const streamBlockToolCalls = streamBlocksToToolCalls(streamBlocks);
    const displayToolCalls = !!meta._streaming && streamBlockToolCalls.length > 0
      ? streamBlockToolCalls
      : (projection.toolCalls.length > 0 ? projection.toolCalls : streamBlockToolCalls);
    const lastDisplayToolName = displayToolCalls[displayToolCalls.length - 1]?.name || "";
    const activity = baseActivityForMessage(
      projection,
      !!meta._streaming,
      streamBlocks,
    );
    const dispatchElapsedMs = positiveNumberFromProviderMeta(meta, "dispatchElapsedMs");
    const frontendDispatchElapsedMs = positiveNumberFromProviderMeta(meta, "_frontendDispatchElapsedMs");
    const extraTextReferences = buildExtraTextReferences(message);
    const baseBlock = {
      id: message.id,
      sourceMessageId: message.id,
      isExtraTextBlock: false,
      role: message.role,
      isStreaming: !!meta._streaming,
      streamSegments,
      streamTail,
      streamAnimatedDelta,
      speakerAgentId: projection.speakerAgentId,
      createdAt: String(message.createdAt || "").trim() || undefined,
      providerMeta: message.providerMeta,
      mentions: projection.mentions,
      text: projection.text,
      images: projection.images,
      audios: projection.audios,
      attachmentFiles: projection.attachmentFiles,
      extraTextReferences: message.role === "user" ? extraTextReferences : [],
      inlineSegments: projection.inlineSegments,
      memeSegments: projection.memeSegments,
      taskTrigger: projection.taskTrigger,
      planCard: projection.planCard,
      remoteImOrigin: projection.remoteImOrigin,
      dispatchElapsedMs,
      frontendDispatchElapsedMs,
      toolCallCount: displayToolCalls.length,
      lastToolName: lastDisplayToolName,
      toolCalls: displayToolCalls,
      activityItems: activity.items,
      activityReasoningCharCount: activity.activityReasoningCharCount,
      activityToolCountsByName: activity.activityToolCountsByName,
      activityRunning: activity.activityRunning,
      activityStatus: activity.activityStatus,
    } satisfies ChatMessageBlock;

    if (message.role === "assistant" && (baseBlock.isStreaming || streamBlocks.length > 0 || baseBlock.activityItems.length > 0)) {
      const streamReasoningLen = streamBlocks.reduce((total, block) => total + String(block.reasoning || "").length, 0);
      const streamTextLen = streamBlocks.reduce((total, block) => total + String(block.text || "").length, 0);
      const streamToolCount = streamBlocks.reduce((total, block) => total + (block.tools || []).length, 0);
      const debugSignature = [
        String(message.id || ""),
        baseBlock.isStreaming ? "1" : "0",
        String(baseBlock.text || "").length,
        streamBlocks.length,
        streamReasoningLen,
        streamTextLen,
        streamToolCount,
        baseBlock.activityItems.length,
        baseBlock.activityReasoningCharCount,
        baseBlock.activityRunning ? "1" : "0",
        baseBlock.activityStatus,
      ].join("|");
      const previousDebugSignature = streamProjectionDebugSignatures.get(String(message.id || ""));
      if (previousDebugSignature === debugSignature) {
        // Avoid flooding the console while the virtual list recomputes unchanged blocks.
      } else {
        streamProjectionDebugSignatures.set(String(message.id || ""), debugSignature);
        console.info("[聊天流式块][前端投影]", {
          messageId: String(message.id || ""),
          isStreaming: baseBlock.isStreaming,
          textLength: String(baseBlock.text || "").length,
          streamBlockCount: streamBlocks.length,
          streamReasoningLen,
          streamTextLen,
          streamToolCount,
          activityItemCount: baseBlock.activityItems.length,
          activityReasoningCharCount: baseBlock.activityReasoningCharCount,
          activityRunning: baseBlock.activityRunning,
          activityStatus: baseBlock.activityStatus,
          shouldShowActivityPanel: baseBlock.activityItems.length > 0 || baseBlock.activityRunning,
        });
      }
    }

    const blocks: ChatMessageBlock[] = [];
    if (
      baseBlock.text
      || !!baseBlock.isStreaming
      || baseBlock.images.length > 0
      || baseBlock.audios.length > 0
      || baseBlock.attachmentFiles.length > 0
      || (baseBlock.extraTextReferences || []).length > 0
      || !!baseBlock.taskTrigger
      || !!baseBlock.planCard
      || baseBlock.activityItems.length > 0
      || baseBlock.activityRunning
    ) {
      blocks.push(baseBlock);
    }

    if (message.role !== "user" && Array.isArray(message.extraTextBlocks)) {
      message.extraTextBlocks.forEach((raw, index) => {
        const text = String(raw || "").trim();
        if (!text) return;
        blocks.push({
          id: `${message.id}::extra:${index}`,
          sourceMessageId: message.id,
          isExtraTextBlock: true,
          role: message.role,
          isStreaming: false,
          streamSegments: [],
          streamTail: "",
          streamAnimatedDelta: "",
          speakerAgentId: projection.speakerAgentId,
          createdAt: String(message.createdAt || "").trim() || undefined,
          providerMeta: message.providerMeta,
          mentions: projection.mentions,
          text,
          images: [],
          audios: [],
          attachmentFiles: [],
          extraTextReferences: [],
          memeSegments: undefined,
          taskTrigger: undefined,
          planCard: undefined,
          remoteImOrigin: projection.remoteImOrigin,
          dispatchElapsedMs,
          toolCallCount: 0,
          lastToolName: "",
          toolCalls: [],
          activityItems: [],
          activityReasoningCharCount: 0,
          activityToolCountsByName: {},
          activityRunning: false,
          activityStatus: "idle",
        });
      });
    }

    messageBlockCache.set(message, { signature, blocks });
    return blocks;
  }

  const allMessageBlocks = computed<ChatMessageBlock[]>(() => {
    const messages = options.allMessages.value;
    const signature = messages.map((message) => messageSignature(message)).join("||");
    if (signature === lastMessageBlockSignature) {
      return lastMessageBlocks;
    }
    const blocks = messages
      .flatMap((message) => buildMessageBlocks(message));
    lastMessageBlockSignature = signature;
    lastMessageBlocks = blocks;
    return blocks;
  });

  const visibleMessageBlocks = computed(() => allMessageBlocks.value);

  const chatContextUsageRatio = computed(() => {
    const previewRatio = previewContextUsageRatio();
    if (previewRatio !== null) {
      return previewRatio;
    }
    const backendPercent = latestBackendContextUsagePercent(options.allMessages.value);
    if (backendPercent !== null) {
      return backendPercent / 100;
    }
    const api = options.activeChatApiConfig.value;
    if (!api) return 0;
    const maxTokens = Math.max(16000, Math.round(Number(api.contextWindowTokens ?? 256000)));
    const used = estimateConversationTokens(options.allMessages.value);
    return used / Math.max(1, maxTokens);
  });

  const chatUsagePercent = computed(() => Math.min(100, Math.max(0, Math.round(chatContextUsageRatio.value * 100))));

  return {
    allMessageBlocks,
    visibleMessageBlocks,
    chatContextUsageRatio,
    chatUsagePercent,
  };
}

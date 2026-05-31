import type { Ref } from "vue";
import type { ChatMentionTarget } from "../../../types/app";
import type { SendChatOverrides } from "./use-chat-flow-types";
import { normalizeConversationId } from "./use-chat-flow-utils";

type AttachmentNotice = { id: string; fileName: string; relativePath: string; mime: string };
type ImageAttachment = { mime: string; bytesBase64: string; savedPath?: string };
type Session = { apiConfigId: string; agentId: string; departmentId?: string; conversationId?: string };

type UseChatFlowSendInputOptions = {
  chatInput: Ref<string>;
  clipboardImages: Ref<ImageAttachment[]>;
  queuedAttachmentNotices?: Ref<AttachmentNotice[]>;
  selectedMentions?: Ref<ChatMentionTarget[]>;
  latestUserText: Ref<string>;
  latestUserImages: Ref<Array<{ mime: string; bytesBase64: string }>>;
  getSession: () => Session | null;
  getConversationId?: () => string;
  buildQueuedAttachmentPayload: () => Array<{ fileName: string; relativePath: string; mime: string }>;
  buildImageAttachmentPayload: (
    images: ImageAttachment[],
  ) => Array<{ fileName: string; relativePath: string; mime: string }>;
  buildInstructionExtraTextBlocks: () => string[];
};

export type PreparedChatSendInput = {
  useOverrideMessage: boolean;
  plainText: string;
  selectedMentions: ChatMentionTarget[];
  extraTextBlocks: string[];
  sentImages: ImageAttachment[];
  attachments: Array<{ fileName: string; relativePath: string; mime: string }>;
  sendSession: Session;
  sendConversationId: string;
};

export function useChatFlowSendInput(options: UseChatFlowSendInputOptions) {
  function normalizeSelectedMentions(): ChatMentionTarget[] {
    return Array.isArray(options.selectedMentions?.value)
      ? options.selectedMentions.value
        .map((item) => ({
          agentId: String(item.agentId || "").trim(),
          agentName: String(item.agentName || "").trim(),
          departmentId: String(item.departmentId || "").trim(),
          departmentName: String(item.departmentName || "").trim(),
          avatarUrl: String(item.avatarUrl || "").trim() || undefined,
        }))
        .filter((item) => !!item.agentId && !!item.departmentId)
      : [];
  }

  function prepareSendInput(overrides?: SendChatOverrides): PreparedChatSendInput | null {
    const useOverrideMessage = !!overrides && typeof overrides.text === "string";
    const plainText = useOverrideMessage
      ? String(overrides.text || "").trim()
      : options.chatInput.value.trim();
    const queuedAttachments = useOverrideMessage ? [] : options.buildQueuedAttachmentPayload();
    const instructionExtraTextBlocks = overrides?.skipInstructionPrompts
      ? []
      : options.buildInstructionExtraTextBlocks();
    const selectedMentions = normalizeSelectedMentions();
    const extraTextBlocks = [
      ...instructionExtraTextBlocks,
      ...(Array.isArray(overrides?.extraTextBlocks) ? overrides.extraTextBlocks : []),
    ].filter((item) => !!String(item || "").trim());
    const sentImages = useOverrideMessage ? [] : [...options.clipboardImages.value];
    if (!plainText && sentImages.length === 0 && queuedAttachments.length === 0 && extraTextBlocks.length === 0) {
      return null;
    }
    const sendSession = options.getSession();
    if (!sendSession || !sendSession.apiConfigId || !sendSession.agentId) return null;
    const sendConversationId = normalizeConversationId(options.getConversationId ? options.getConversationId() : "");
    const attachments = [...queuedAttachments, ...options.buildImageAttachmentPayload(sentImages)];
    return {
      useOverrideMessage,
      plainText,
      selectedMentions,
      extraTextBlocks,
      sentImages,
      attachments,
      sendSession,
      sendConversationId,
    };
  }

  function applyPreparedSendInput(input: PreparedChatSendInput) {
    options.latestUserText.value = input.plainText;
    options.latestUserImages.value = input.sentImages.map((image) => ({
      mime: String(image.mime || ""),
      bytesBase64: String(image.bytesBase64 || ""),
    }));
    if (input.useOverrideMessage) return;
    options.chatInput.value = "";
    options.clipboardImages.value = [];
    if (options.queuedAttachmentNotices) options.queuedAttachmentNotices.value = [];
    if (options.selectedMentions) options.selectedMentions.value = [];
  }

  return {
    applyPreparedSendInput,
    prepareSendInput,
  };
}

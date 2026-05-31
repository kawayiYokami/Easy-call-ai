import type { Ref } from "vue";
import type { PromptCommandPreset } from "../../../types/app";

type QueuedAttachmentNotice = {
  id: string;
  fileName: string;
  relativePath: string;
  mime: string;
};

type ImageAttachment = {
  mime: string;
  bytesBase64: string;
  savedPath?: string;
};

type AttachmentPayload = {
  fileName: string;
  relativePath: string;
  mime: string;
};

type UseChatFlowSendPayloadsOptions = {
  queuedAttachmentNotices?: Ref<QueuedAttachmentNotice[]>;
  selectedInstructionPrompts?: Ref<PromptCommandPreset[]>;
};

export function useChatFlowSendPayloads(options: UseChatFlowSendPayloadsOptions) {
  function buildQueuedAttachmentPayload(): AttachmentPayload[] {
    const list = options.queuedAttachmentNotices?.value || [];
    if (list.length === 0) return [];
    return list
      .map((item) => {
        const fileName = String(item.fileName || "").trim();
        const relativePath = String(item.relativePath || "").trim().replace(/\\/g, "/");
        const mime = String(item.mime || "").trim();
        if (!fileName || !relativePath) return null;
        return { fileName, relativePath, mime };
      })
      .filter((value): value is AttachmentPayload => !!value);
  }

  function buildImageAttachmentPayload(images: ImageAttachment[]): AttachmentPayload[] {
    const dedup = new Map<string, AttachmentPayload>();
    for (const image of images) {
      const rawPath = String(image.savedPath || "").trim();
      if (!rawPath) continue;
      const relativePath = rawPath.replace(/\\/g, "/");
      if (!relativePath) continue;
      const fileName = relativePath.split("/").pop() || "attachment";
      const mime = String(image.mime || "").trim();
      const key = `${relativePath}::${mime}`;
      if (dedup.has(key)) continue;
      dedup.set(key, { fileName, relativePath, mime });
    }
    return Array.from(dedup.values());
  }

  function buildInstructionExtraTextBlocks(): string[] {
    const list = options.selectedInstructionPrompts?.value || [];
    if (list.length === 0) return [];
    return list
      .map((item) => {
        const prompt = String(item?.prompt || "").trim();
        if (!prompt) return "";
        return `<user instruction>\n${prompt}\n</user instruction>`;
      })
      .filter((item) => !!item);
  }

  return {
    buildQueuedAttachmentPayload,
    buildImageAttachmentPayload,
    buildInstructionExtraTextBlocks,
  };
}

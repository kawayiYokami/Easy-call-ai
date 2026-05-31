import { open } from "@tauri-apps/plugin-dialog";
import type { Ref } from "vue";

type QueuedAttachmentNotice = {
  id: string;
  fileName: string;
  relativePath: string;
  mime: string;
};

type UseChatAttachmentPickerFlowOptions = {
  chatting: Ref<boolean>;
  trimming: Ref<boolean>;
  queuedAttachmentNotices: Ref<QueuedAttachmentNotice[]>;
  onNativeFileDrop: (paths: string[]) => Promise<void>;
  setStatusError: (key: string, error: unknown) => void;
};

export function useChatAttachmentPickerFlow(options: UseChatAttachmentPickerFlowOptions) {
  function removeQueuedAttachmentNotice(index: number) {
    if (index < 0 || index >= options.queuedAttachmentNotices.value.length) return;
    options.queuedAttachmentNotices.value.splice(index, 1);
  }

  async function pickChatAttachments() {
    if (options.chatting.value || options.trimming.value) return;
    try {
      const picked = await open({
        multiple: true,
        directory: false,
        title: "选择附件",
      });
      if (!picked) return;
      const paths = Array.isArray(picked) ? picked : [picked];
      const normalized = paths
        .map((value) => String(value || "").trim())
        .filter(Boolean);
      if (normalized.length === 0) return;
      await options.onNativeFileDrop(normalized);
    } catch (error) {
      options.setStatusError("status.pasteImageReadFailed", error);
    }
  }

  return {
    removeQueuedAttachmentNotice,
    pickChatAttachments,
  };
}

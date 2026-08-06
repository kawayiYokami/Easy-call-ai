import { ref, type ComputedRef, type Ref } from "vue";
import type { ApiConfigItem } from "../../../types/app";
import {
  attachmentPreviewBase64,
  ingestAttachment,
  textAttachmentFile,
  type AttachmentReceipt,
} from "../../../services/attachment-transfer";
import { useHotkeyRecordTest } from "../../shell/composables/use-hotkey-record-test";
import { isAbsoluteLocalPath } from "../utils/local-link";
import { getActiveChatComposerScope } from "./chat-composer-focus";
import {
  applyQueuedAttachmentResult as sharedApplyQueuedAttachmentResult,
  collectPastedFiles,
  ingestPastedImages,
  normalizeFileMime,
} from "./chat-paste-ingest";

type TrFn = (key: string, params?: Record<string, unknown>) => string;

type UseChatMediaOptions = {
  t: TrFn;
  setStatus: (text: string) => void;
  setChatError: (text: string) => void;
  setStatusError: (key: string, error: unknown) => void;
  viewMode: Ref<"chat" | "archives" | "config">;
  chatting: Ref<boolean>;
  trimming: Ref<boolean>;
  isRecording: () => boolean;
  activeChatApiConfig: ComputedRef<ApiConfigItem | null>;
  hasVisionFallback: ComputedRef<boolean>;
  chatInput: Ref<string>;
  clipboardImages: Ref<Array<{ mime: string; bytesBase64: string; savedPath?: string }>>;
  queuedAttachmentNotices: Ref<Array<{ id: string; fileName: string; path: string; mime: string }>>;
};

export function useChatMedia(options: UseChatMediaOptions) {
  const mediaDragActive = ref(false);
  let dragOverlayHideTimer: ReturnType<typeof setTimeout> | null = null;
  const hotkeyRecordTest = useHotkeyRecordTest({
    t: options.t,
    setStatus: options.setStatus,
    setStatusError: options.setStatusError,
    isBlocked: options.isRecording,
  });

  function hasFileTransferPayload(transfer: DataTransfer | null): boolean {
    if (!transfer) return false;
    const types = Array.from(transfer.types || []).map((value) => String(value || "").toLowerCase());
    if (types.includes("files")) return true;
    if (transfer.files && transfer.files.length > 0) return true;
    if (transfer.items && Array.from(transfer.items).some((item) => item.kind === "file")) return true;
    return false;
  }

  async function queueTextAttachment(fileName: string, text: string, mime = "text/markdown") {
    const normalizedText = String(text || "");
    if (!normalizedText.trim()) return;
    const queued = await ingestAttachment({
      kind: "browser-file",
      file: textAttachmentFile(fileName, normalizedText, mime),
    });
    applyQueuedAttachmentResult(queued, options.activeChatApiConfig.value || ({ enableImage: false } as ApiConfigItem));
  }

  function collectDroppedFiles(
    event: DragEvent,
  ): Array<{ file: File; mime: string }> {
    const transfer = event.dataTransfer;
    if (!transfer) return [];
    const fromFiles = transfer.files ? Array.from(transfer.files) : [];
    const fromItems =
      transfer.items && transfer.items.length > 0
        ? Array.from(transfer.items)
            .filter((item) => item.kind === "file")
            .map((item) => item.getAsFile())
            .filter((file): file is File => !!file)
        : [];
    const files = fromFiles.length > 0 ? fromFiles : fromItems;
    if (files.length === 0) return [];
    const out: Array<{ file: File; mime: string }> = [];
    for (const file of files) {
      const mime = normalizeFileMime(file);
      out.push({ file, mime });
    }
    return out;
  }

  function applyQueuedAttachmentResult(queued: AttachmentReceipt, apiConfig: ApiConfigItem) {
    return sharedApplyQueuedAttachmentResult(queued, apiConfig, {
      setChatError: options.setChatError,
      setStatusError: options.setStatusError,
      clipboardImages: options.clipboardImages,
      queuedAttachmentNotices: options.queuedAttachmentNotices,
      hasVisionFallback: options.hasVisionFallback.value,
    });
  }

  async function queueInlineBrowserFile(file: File, _mime: string): Promise<AttachmentReceipt> {
    return await ingestAttachment({ kind: "browser-file", file });
  }

  function onPaste(event: ClipboardEvent) {
    if (options.viewMode.value !== "chat") return;
    if (options.trimming.value) return;
    const apiConfig = options.activeChatApiConfig.value;
    if (!apiConfig) return;
    const collected = collectPastedFiles(event);
    // 焦点位于某个会话输入框（主会话或侧边追问）内时，文本粘贴交给浏览器
    // 原生行为，内容会落到焦点所在的 textarea，而不是被全局拦截后固定写进
    // 主会话输入框；图片文件按焦点归属路由：焦点在追问输入框时由追问视图的
    // paste 监听接管入队，本通道只处理主会话输入框（或焦点不在任何输入框）的图片。
    const activeElement = document.activeElement;
    const composerInputFocused = activeElement instanceof HTMLElement
      && activeElement.classList.contains("ecall-chat-composer-input");
    if (composerInputFocused && collected.length === 0) {
      return;
    }
    if (collected.length > 0) {
      if (getActiveChatComposerScope() === "side") {
        // 焦点在追问输入框：图片交给追问侧处理，主会话不拦截。
        return;
      }
      event.preventDefault();
      options.setChatError("");
      void (async () => {
        for (const item of collected) {
          try {
            const queued = await queueInlineBrowserFile(item.file, item.mime);
            applyQueuedAttachmentResult(queued, apiConfig);
          } catch (error) {
            options.setStatusError("status.pasteImageReadFailed", error);
          }
        }
      })();
      return;
    }

    const text = event.clipboardData?.getData("text/plain") || "";
    if (text && !options.chatInput.value.trim() && apiConfig.enableText) {
      event.preventDefault();
      options.chatInput.value = text;
      options.setChatError("");
      return;
    }

  }

  function onDragOver(event: DragEvent) {
    if (options.viewMode.value !== "chat") return;
    if (options.trimming.value) return;
    const apiConfig = options.activeChatApiConfig.value;
    if (!apiConfig) return;
    if (!hasFileTransferPayload(event.dataTransfer)) return;
    event.preventDefault();
    event.dataTransfer!.dropEffect = "copy";
    mediaDragActive.value = true;
    if (dragOverlayHideTimer) {
      clearTimeout(dragOverlayHideTimer);
      dragOverlayHideTimer = null;
    }
    dragOverlayHideTimer = setTimeout(() => {
      mediaDragActive.value = false;
      dragOverlayHideTimer = null;
    }, 140);
  }

  function onDrop(event: DragEvent) {
    if (options.viewMode.value !== "chat") return;
    if (options.trimming.value) return;
    const apiConfig = options.activeChatApiConfig.value;
    if (!apiConfig) return;
    if (!hasFileTransferPayload(event.dataTransfer)) return;
    event.preventDefault();
    const collected = collectDroppedFiles(event);
    if (collected.length === 0) {
      mediaDragActive.value = false;
      return;
    }
    options.setChatError("");
    options.setStatus(`收到拖拽文件 ${collected.length} 个（DOM）。`);
    mediaDragActive.value = false;
    if (dragOverlayHideTimer) {
      clearTimeout(dragOverlayHideTimer);
      dragOverlayHideTimer = null;
    }
    void (async () => {
      for (const item of collected) {
        try {
          const queued = await queueInlineBrowserFile(item.file, item.mime);
          applyQueuedAttachmentResult(queued, apiConfig);
        } catch (error) {
          options.setStatusError("status.pasteImageReadFailed", error);
        }
      }
    })();
  }

  async function onTransportFileDrop(paths: string[]) {
    if (options.viewMode.value !== "chat") return;
    if (options.trimming.value) return;
    const apiConfig = options.activeChatApiConfig.value;
    if (!apiConfig) return;
    if (!Array.isArray(paths) || paths.length === 0) return;
    options.setChatError("");
    options.setStatus(`收到拖拽文件 ${paths.length} 个。`);

    for (const path of paths) {
      try {
        const queued = await ingestAttachment({ kind: "local-path", path });
        applyQueuedAttachmentResult(queued, apiConfig);
      } catch (error) {
        options.setStatusError("status.pasteImageReadFailed", error);
      }
    }
  }

  function removeClipboardImage(index: number) {
    if (index < 0 || index >= options.clipboardImages.value.length) return;
    options.clipboardImages.value.splice(index, 1);
  }

  async function cleanupChatMedia() {
    await hotkeyRecordTest.cleanupHotkeyRecordTest();
    mediaDragActive.value = false;
    if (dragOverlayHideTimer) {
      clearTimeout(dragOverlayHideTimer);
      dragOverlayHideTimer = null;
    }
  }

  return {
    mediaDragActive,
    hotkeyTestRecording: hotkeyRecordTest.hotkeyTestRecording,
    hotkeyTestRecordingMs: hotkeyRecordTest.hotkeyTestRecordingMs,
    hotkeyTestAudio: hotkeyRecordTest.hotkeyTestAudio,
    microphonePermissionState: hotkeyRecordTest.microphonePermissionState,
    microphonePermissionRequesting: hotkeyRecordTest.microphonePermissionRequesting,
    onPaste,
    onDragOver,
    onDrop,
    onTransportFileDrop,
    applyQueuedAttachmentResult,
    queueTextAttachment,
    removeClipboardImage,
    startHotkeyRecordTest: hotkeyRecordTest.startHotkeyRecordTest,
    stopHotkeyRecordTest: hotkeyRecordTest.stopHotkeyRecordTest,
    playHotkeyRecordTest: hotkeyRecordTest.playHotkeyRecordTest,
    requestMicrophonePermission: hotkeyRecordTest.requestMicrophonePermission,
    cleanupChatMedia,
  };
}

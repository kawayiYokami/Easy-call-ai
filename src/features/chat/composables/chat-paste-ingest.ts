/**
 * 粘贴图片入队公共逻辑：主会话与追问会话共用同一份实现，
 * 避免两外壳各自写一套 collectPastedFiles → ingest → apply 的链路分叉。
 */
import type { Ref } from "vue";
import type { ApiConfigItem } from "../../../types/app";
import {
  attachmentPreviewBase64,
  ingestAttachment,
  type AttachmentReceipt,
} from "../../../services/attachment-transfer";
import { isAbsoluteLocalPath } from "../utils/local-link";

export type PasteIngestTargets = {
  setChatError: (text: string) => void;
  setStatusError: (key: string, error: unknown) => void;
  clipboardImages: Ref<Array<{ mime: string; bytesBase64: string; savedPath?: string }>>;
  queuedAttachmentNotices: Ref<Array<{ id: string; fileName: string; path: string; mime: string }>>;
  hasVisionFallback: boolean;
};

function inferMimeFromFileName(name: string): string {
  const lower = (name || "").trim().toLowerCase();
  if (lower.endsWith(".pdf")) return "application/pdf";
  if (lower.endsWith(".png")) return "image/png";
  if (lower.endsWith(".jpg") || lower.endsWith(".jpeg")) return "image/jpeg";
  if (lower.endsWith(".gif")) return "image/gif";
  if (lower.endsWith(".webp")) return "image/webp";
  if (lower.endsWith(".heic")) return "image/heic";
  if (lower.endsWith(".heif")) return "image/heif";
  if (lower.endsWith(".svg")) return "image/svg+xml";
  return "";
}

function normalizeFileMime(file: File): string {
  const raw = (file.type || "").trim().toLowerCase();
  if (raw) return raw;
  return inferMimeFromFileName(file.name);
}

export { normalizeFileMime };

export function collectPastedFiles(
  event: ClipboardEvent,
): Array<{ file: File; mime: string }> {
  const data = event.clipboardData;
  if (!data) return [];
  const items = data.items;
  const filesFromItems =
    items && items.length > 0
      ? Array.from(items)
          .filter((item) => item.kind === "file")
          .map((item) => item.getAsFile())
          .filter((file): file is File => !!file)
      : [];
  const filesFromList = data.files ? Array.from(data.files) : [];
  const sourceFiles = filesFromItems.length > 0 ? filesFromItems : filesFromList;
  if (sourceFiles.length === 0) return [];
  const files: Array<{ file: File; mime: string }> = [];
  for (const file of sourceFiles) {
    const mime = normalizeFileMime(file);
    files.push({ file, mime });
  }
  return files;
}

function canAcceptImage(apiConfig: ApiConfigItem, hasVisionFallback: boolean): boolean {
  return !!apiConfig.enableImage || hasVisionFallback;
}

function classifyFileMime(
  mime: string,
  apiConfig: ApiConfigItem,
  hasVisionFallback: boolean,
): { kind: "image" | "pdf" | null; reason: "imageUnsupported" | null } {
  const normalized = (mime || "").trim().toLowerCase();
  if (normalized.startsWith("image/")) {
    return canAcceptImage(apiConfig, hasVisionFallback)
      ? { kind: "image", reason: null }
      : { kind: null, reason: "imageUnsupported" };
  }
  if (normalized === "application/pdf") {
    // PDF 不再走多模态直发，统一入队为普通附件，交由后端阅读链路处理。
    return { kind: null, reason: null };
  }
  return { kind: null, reason: null };
}

export function applyQueuedAttachmentResult(
  queued: AttachmentReceipt,
  apiConfig: ApiConfigItem,
  targets: PasteIngestTargets,
): void {
  const mime = String(queued.mime || "").trim().toLowerCase();
  const classified = classifyFileMime(mime, apiConfig, targets.hasVisionFallback);
  const canAttachAsMedia = !!queued.attachAsMedia && !!classified.kind;
  const path = String(queued.path || "").trim().replace(/\\/g, "/");
  if (!isAbsoluteLocalPath(path)) {
    targets.setChatError("附件保存未返回绝对路径，已跳过该附件。其他消息内容仍可继续发送。");
    return;
  }

  if (!canAttachAsMedia) {
    const fileName = String(queued.fileName || "").trim() || path.split("/").pop() || "attachment";
    const id = `${path}::${mime}`;
    if (!targets.queuedAttachmentNotices.value.some((item) => item.id === id)) {
      targets.queuedAttachmentNotices.value.push({
        id,
        fileName,
        path,
        mime,
      });
    }
    return;
  }

  const previewImage = {
    mime,
    bytesBase64: attachmentPreviewBase64(queued),
    savedPath: path,
    previewDataUrl: String(queued.previewDataUrl || "").trim() || undefined,
  };
  targets.clipboardImages.value.push(previewImage);
}

export async function ingestPastedImages(
  collected: Array<{ file: File; mime: string }>,
  apiConfig: ApiConfigItem,
  targets: PasteIngestTargets,
): Promise<void> {
  for (const item of collected) {
    try {
      const queued = await ingestAttachment({ kind: "browser-file", file: item.file });
      applyQueuedAttachmentResult(queued, apiConfig, targets);
    } catch (error) {
      targets.setStatusError("status.pasteImageReadFailed", error);
    }
  }
}

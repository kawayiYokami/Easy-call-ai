import { describe, expect, it, vi } from "vitest";
import { nextTick, ref } from "vue";
import {
  clearChatComposerFocus,
  getActiveChatComposerScope,
  registerChatComposerFocus,
} from "./chat-composer-focus";
import {
  applyQueuedAttachmentResult,
  collectPastedFiles,
  ingestPastedImages,
} from "./chat-paste-ingest";
import type { ApiConfigItem } from "../../../types/app";

const imageApiConfig = {
  id: "image-model",
  enableImage: true,
  enableText: true,
} as ApiConfigItem;

const textApiConfig = {
  id: "text-model",
  enableImage: false,
  enableText: true,
} as ApiConfigItem;

function makeTargets() {
  const clipboardImages = ref<Array<{ mime: string; bytesBase64: string; savedPath?: string }>>([]);
  const queuedAttachmentNotices = ref<Array<{ id: string; fileName: string; path: string; mime: string }>>([]);
  const setChatError = vi.fn();
  const setStatusError = vi.fn();
  return { clipboardImages, queuedAttachmentNotices, setChatError, setStatusError };
}

describe("chat-composer-focus（最后活跃输入框共享状态）", () => {
  it("focus 注册后能取到对应 scope，blur 后清除", async () => {
    expect(getActiveChatComposerScope()).toBeNull();
    registerChatComposerFocus("side");
    expect(getActiveChatComposerScope()).toBe("side");
    clearChatComposerFocus("side");
    await nextTick();
    expect(getActiveChatComposerScope()).toBeNull();
  });

  it("两个输入框先后聚焦时，后聚焦者覆盖前者", async () => {
    registerChatComposerFocus("main");
    registerChatComposerFocus("side");
    expect(getActiveChatComposerScope()).toBe("side");
    // 先失焦的非活跃方不会误清活跃方
    clearChatComposerFocus("main");
    expect(getActiveChatComposerScope()).toBe("side");
  });
});

describe("collectPastedFiles", () => {
  it("从 clipboardData.items 提取文件并推断 mime", () => {
    const file = new File(["x"], "photo.png", { type: "image/png" });
    const event = {
      clipboardData: {
        items: [{ kind: "file", getAsFile: () => file }],
        files: [file],
      },
    } as unknown as ClipboardEvent;
    const collected = collectPastedFiles(event);
    expect(collected).toEqual([{ file, mime: "image/png" }]);
  });

  it("无文件时返回空数组", () => {
    const event = {
      clipboardData: {
        items: [],
        files: [],
      },
    } as unknown as ClipboardEvent;
    expect(collectPastedFiles(event)).toEqual([]);
  });
});

describe("applyQueuedAttachmentResult", () => {
  it("图片且模型支持视觉时进入 clipboardImages（可作媒体直发）", () => {
    const targets = makeTargets();
    applyQueuedAttachmentResult(
      { id: "1", mime: "image/png", path: "C:/tmp/a.png", fileName: "a.png", size: 1, attachAsMedia: true, textNotice: "" },
      imageApiConfig,
      { ...targets, hasVisionFallback: false },
    );
    expect(targets.clipboardImages.value).toHaveLength(1);
    expect(targets.clipboardImages.value[0].mime).toBe("image/png");
    expect(targets.queuedAttachmentNotices.value).toHaveLength(0);
  });

  it("图片但模型不支持视觉时降级为普通附件通知（不泄漏 image_url）", () => {
    const targets = makeTargets();
    applyQueuedAttachmentResult(
      { id: "1", mime: "image/png", path: "C:/tmp/a.png", fileName: "a.png", size: 1, attachAsMedia: true, textNotice: "" },
      textApiConfig,
      { ...targets, hasVisionFallback: false },
    );
    expect(targets.clipboardImages.value).toHaveLength(0);
    expect(targets.queuedAttachmentNotices.value).toHaveLength(1);
    expect(targets.queuedAttachmentNotices.value[0].path).toBe("C:/tmp/a.png");
  });

  it("路径非法时跳过并报错", () => {
    const targets = makeTargets();
    applyQueuedAttachmentResult(
      { id: "2", mime: "image/png", path: "not-absolute", fileName: "a.png", size: 1, attachAsMedia: true, textNotice: "" },
      imageApiConfig,
      { ...targets, hasVisionFallback: false },
    );
    expect(targets.clipboardImages.value).toHaveLength(0);
    expect(targets.setChatError).toHaveBeenCalled();
  });
});

const uploaderMock = vi.hoisted(() => vi.fn());

vi.mock("../../../services/attachment-transfer", () => ({
  ingestAttachment: async (source: { kind: "browser-file"; file: File }) =>
    uploaderMock(source.file),
  attachmentPreviewBase64: () => "data:image/png;base64,AA==",
}));

describe("ingestPastedImages", () => {
  it("逐个 ingest 并落入目标队列（上传失败不中断后续）", async () => {
    const targets = makeTargets();
    uploaderMock
      .mockReset()
      .mockResolvedValueOnce({
        mime: "image/png",
        path: "C:/tmp/a.png",
        fileName: "a.png",
        attachAsMedia: true,
        size: 1, textNotice: "",
      })
      .mockRejectedValueOnce(new Error("upload failed"))
      .mockResolvedValueOnce({
        mime: "image/jpeg",
        path: "C:/tmp/b.jpg",
        fileName: "b.jpg",
        attachAsMedia: true,
        size: 1, textNotice: "",
      });
    const fileA = new File(["a"], "a.png", { type: "image/png" });
    const fileB = new File(["b"], "b.jpg", { type: "image/jpeg" });
    const fileC = new File(["c"], "c.jpg", { type: "image/jpeg" });

    await ingestPastedImages(
      [
        { file: fileA, mime: "image/png" },
        { file: fileB, mime: "image/jpeg" },
        { file: fileC, mime: "image/jpeg" },
      ],
      imageApiConfig,
      { ...targets, hasVisionFallback: false },
    );

    expect(uploaderMock).toHaveBeenCalledTimes(3);
    expect(targets.clipboardImages.value).toHaveLength(2);
    expect(targets.setStatusError).toHaveBeenCalledTimes(1);
  });
});

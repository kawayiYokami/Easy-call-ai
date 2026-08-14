import { describe, expect, it } from "vitest";
import {
  FILE_READER_LINE_WRAP_DEFAULT,
  normalizeFileReaderLineWrap,
  useFileReaderAppearance,
} from "./use-file-reader-appearance";

describe("use-file-reader-appearance", () => {
  it("代码预览换行默认关闭", () => {
    const appearance = useFileReaderAppearance();

    expect(appearance.fileReaderLineWrapEnabled.value).toBe(FILE_READER_LINE_WRAP_DEFAULT);
  });

  it("setter 能在开启与关闭状态间切换", () => {
    const appearance = useFileReaderAppearance();

    appearance.setFileReaderLineWrapEnabled(false);
    expect(appearance.fileReaderLineWrapEnabled.value).toBe(false);

    appearance.setFileReaderLineWrapEnabled(true);
    expect(appearance.fileReaderLineWrapEnabled.value).toBe(true);
  });

  it("无效的外部同步值回退为默认关闭", () => {
    expect(normalizeFileReaderLineWrap(undefined)).toBe(false);
    expect(normalizeFileReaderLineWrap("false")).toBe(false);
    expect(normalizeFileReaderLineWrap(false)).toBe(false);
    expect(normalizeFileReaderLineWrap(true)).toBe(true);
  });
});

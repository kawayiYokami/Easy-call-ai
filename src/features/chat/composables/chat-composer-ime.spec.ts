import { describe, expect, it } from "vitest";
import {
  CHAT_INPUT_COMPOSITION_CONFIRM_WINDOW_MS,
  chatInputEnterConfirmsComposition,
  type ChatInputEnterConfirmsCompositionEvent,
} from "./chat-composer-ime";

function buildEvent(overrides: Partial<ChatInputEnterConfirmsCompositionEvent> = {}): ChatInputEnterConfirmsCompositionEvent {
  return {
    isComposing: false,
    keyCode: 13,
    key: "Enter",
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    metaKey: false,
    ...overrides,
  };
}

describe("chatInputEnterConfirmsComposition", () => {
  it("组合输入期间（isComposing）拦截 Enter", () => {
    expect(chatInputEnterConfirmsComposition(buildEvent({ isComposing: true }), false, 0, 0)).toBe(true);
  });

  it("keyCode 229 视为 IME 处理中，拦截 Enter", () => {
    expect(chatInputEnterConfirmsComposition(buildEvent({ keyCode: 229 }), false, 0, 0)).toBe(true);
  });

  it("自维护 composing 标志为真时拦截 Enter", () => {
    expect(chatInputEnterConfirmsComposition(buildEvent(), true, 0, 0)).toBe(true);
  });

  it("裸 Enter 落在组合结束窗口内视为 IME 确认回车", () => {
    const endedAt = 1000;
    expect(
      chatInputEnterConfirmsComposition(buildEvent(), false, endedAt, endedAt + CHAT_INPUT_COMPOSITION_CONFIRM_WINDOW_MS - 1),
    ).toBe(true);
  });

  it("Ctrl+Enter 不可能是 IME 确认回车，窗口内放行", () => {
    const endedAt = 1000;
    expect(
      chatInputEnterConfirmsComposition(buildEvent({ ctrlKey: true }), false, endedAt, endedAt + 10),
    ).toBe(false);
  });

  it("Shift/Alt/Meta 修饰的 Enter 均放行", () => {
    const endedAt = 1000;
    const base = { isComposing: false, keyCode: 13, key: "Enter", ctrlKey: false, shiftKey: false, altKey: false, metaKey: false } as const;
    expect(chatInputEnterConfirmsComposition({ ...base, shiftKey: true }, false, endedAt, endedAt + 10)).toBe(false);
    expect(chatInputEnterConfirmsComposition({ ...base, altKey: true }, false, endedAt, endedAt + 10)).toBe(false);
    expect(chatInputEnterConfirmsComposition({ ...base, metaKey: true }, false, endedAt, endedAt + 10)).toBe(false);
  });

  it("从未触发过 compositionend（初始 0 值）时不拦截普通 Enter", () => {
    expect(chatInputEnterConfirmsComposition(buildEvent(), false, 0, 10)).toBe(false);
  });

  it("超过组合结束窗口的裸 Enter 放行", () => {
    const endedAt = 1000;
    expect(
      chatInputEnterConfirmsComposition(buildEvent(), false, endedAt, endedAt + CHAT_INPUT_COMPOSITION_CONFIRM_WINDOW_MS),
    ).toBe(false);
  });

  it("非 Enter 键不受窗口影响", () => {
    expect(chatInputEnterConfirmsComposition(buildEvent({ key: "a" }), false, 1000, 1010)).toBe(false);
  });
});

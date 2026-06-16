import { describe, expect, it } from "vitest";
import { consumeClosedMarkdownBlocks } from "../src/features/chat/composables/use-chat-flow-text";

describe("use-chat-flow text streaming", () => {
  it("should return all content as chunks with optimistic rendering", () => {
    const input = "不对。\n准确说法应该是：\n\n- **优先级本来就有**\n- 现在要做的是...\n";

    const result = consumeClosedMarkdownBlocks(input);
    expect(result.chunks).toEqual([input]);
    expect(result.tail).toBe("");
  });

  it("should handle empty input", () => {
    const result = consumeClosedMarkdownBlocks("");
    expect(result.chunks).toEqual([]);
    expect(result.tail).toBe("");
  });

  it("should immediately release all content including code blocks", () => {
    const input = "写个函数：\n\n```typescript\nfunction add(a, b) {\n  return a + b;\n}\n```";
    const result = consumeClosedMarkdownBlocks(input);

    expect(result.chunks).toEqual([input]);
    expect(result.tail).toBe("");
  });

  it("should handle streaming delta accumulation", () => {
    // 模拟流式场景：每次追加 delta
    let accumulated = "";

    // 第1次
    accumulated += "写个函数：\n\n";
    let result = consumeClosedMarkdownBlocks(accumulated);
    expect(result.chunks).toEqual([accumulated]);
    expect(result.tail).toBe("");

    // 第2次
    accumulated += "```typescript\n";
    result = consumeClosedMarkdownBlocks(accumulated);
    expect(result.chunks).toEqual([accumulated]);
    expect(result.tail).toBe("");

    // 第3次
    accumulated += "function add(a, b) {\n";
    result = consumeClosedMarkdownBlocks(accumulated);
    expect(result.chunks).toEqual([accumulated]);
    expect(result.tail).toBe("");
  });
});

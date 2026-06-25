import { describe, expect, it } from "vitest";
import { IncrementalMarkdownBlockParser } from "./incremental-markdown";
import { parseMarkdownBlocks, type MarkdownBlock } from "./parse-markdown";

function stripKeys(blocks: MarkdownBlock[]): unknown[] {
  return blocks.map((block) => {
    const { key: _key, ...rest } = block;
    return rest;
  });
}

describe("IncrementalMarkdownBlockParser", () => {
  it("matches full streaming parse after append-only chunks", () => {
    const parser = new IncrementalMarkdownBlockParser();
    const chunks = [
      "# 标题\n\n",
      "第一段带 [toolcall:call-1]。\n\n",
      "- 项目 A\n",
      "- 项目 B\n\n",
      "```ts\n",
      "console.log('hi')\n",
      "```\n\n",
      "| A | B |\n",
      "|---|---|\n",
      "| 1 | 2 |\n\n",
      "结束。",
    ];

    let text = "";
    let actual: MarkdownBlock[] = [];
    for (const chunk of chunks) {
      text += chunk;
      actual = parser.parse(text);
    }

    expect(stripKeys(actual)).toEqual(stripKeys(parseMarkdownBlocks(text, true)));
  });

  it("resets when the input is replaced instead of appended", () => {
    const parser = new IncrementalMarkdownBlockParser();
    parser.parse("旧内容\n\n- A");

    const text = "新内容\n\n- B";
    expect(stripKeys(parser.parse(text))).toEqual(stripKeys(parseMarkdownBlocks(text, true)));
  });

  it("keeps streaming footnotes at the end", () => {
    const parser = new IncrementalMarkdownBlockParser();
    const chunks = ["脚注引用[^a]", "\n\n继续正文\n\n", "[^a]: 说明"];
    let text = "";
    let actual: MarkdownBlock[] = [];
    for (const chunk of chunks) {
      text += chunk;
      actual = parser.parse(text);
    }

    expect(actual[actual.length - 1]?.type).toBe("footnotes");
    expect(stripKeys(actual)).toEqual(stripKeys(parseMarkdownBlocks(text, true)));
  });
});

import { describe, expect, it } from "vitest";
import type { MarkdownBlock } from "./parse-markdown";
import { groupMarkdownSegments } from "./markdown-segments";

describe("groupMarkdownSegments", () => {
  it("merges consecutive text-like blocks and isolates rich blocks", () => {
    const blocks: MarkdownBlock[] = [
      { type: "paragraph", text: "hello", key: "p1" },
      { type: "list", ordered: false, items: [{ text: "a", marker: "-" }], key: "l1" },
      { type: "code", lang: "ts", text: "const x = 1", key: "c1" },
      { type: "table", headers: ["a"], rows: [["b"]], key: "t1" },
      { type: "quote", text: "done", key: "q1" },
    ];

    expect(groupMarkdownSegments(blocks)).toEqual([
      { kind: "text", key: "text-p1", blocks: blocks.slice(0, 2) },
      { kind: "rich", key: "rich-c1", blocks: [blocks[2]] },
      { kind: "rich", key: "rich-t1", blocks: [blocks[3]] },
      { kind: "text", key: "text-q1", blocks: [blocks[4]] },
    ]);
  });
});

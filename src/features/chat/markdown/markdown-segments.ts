import type { MarkdownBlock } from "./parse-markdown";

export type MarkdownSegment = {
  kind: "text" | "rich";
  key: string;
  blocks: MarkdownBlock[];
};

function isRichBlock(block: MarkdownBlock): boolean {
  return block.type === "code" || block.type === "table";
}

export function groupMarkdownSegments(blocks: MarkdownBlock[]): MarkdownSegment[] {
  const segments: MarkdownSegment[] = [];
  let textBuffer: MarkdownBlock[] = [];

  const flushTextBuffer = () => {
    if (textBuffer.length <= 0) return;
    const firstKey = String(textBuffer[0]?.key || segments.length);
    segments.push({
      kind: "text",
      key: `text-${firstKey}`,
      blocks: textBuffer,
    });
    textBuffer = [];
  };

  for (const block of blocks) {
    if (!block) continue;
    if (isRichBlock(block)) {
      flushTextBuffer();
      segments.push({
        kind: "rich",
        key: `rich-${String(block.key || segments.length)}`,
        blocks: [block],
      });
      continue;
    }
    textBuffer.push(block);
  }

  flushTextBuffer();
  return segments;
}

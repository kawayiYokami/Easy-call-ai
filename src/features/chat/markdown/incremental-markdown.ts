import { parseMarkdownBlocks, type MarkdownBlock } from "./parse-markdown";

function normalizeMarkdownText(input: string): string {
  return String(input || "").replace(/\r\n?/g, "\n");
}

function isCodeFenceLine(line: string): boolean {
  return /^(`{3,})([\w+-]*)\s*$/.test(line);
}

function isMathFenceLine(line: string): boolean {
  return line.trim() === "$$";
}

function isHeadingLine(line: string): boolean {
  return /^\s{0,3}#{1,4}\s+.+?\s*#*\s*$/.test(line);
}

function isHorizontalRuleLine(line: string): boolean {
  return /^\s{0,3}([-*_])(?:\s*\1){2,}\s*$/.test(line);
}

function isBlockquoteLine(line: string): boolean {
  return /^\s{0,3}>\s?.*$/.test(line);
}

function isListItemLine(line: string): boolean {
  return /^\s{0,3}(?:[-*+]|\d+[.)])\s+.+$/.test(line);
}

function isTableRowLine(line: string): boolean {
  const trimmed = line.trim();
  if (!trimmed.includes("|")) return false;
  const cells = trimmed.replace(/^\|/, "").replace(/\|$/, "").split("|");
  return cells.length >= 2;
}

function isNewBlockStart(line: string): boolean {
  return isHeadingLine(line)
    || isHorizontalRuleLine(line)
    || isCodeFenceLine(line)
    || isMathFenceLine(line)
    || isBlockquoteLine(line)
    || isListItemLine(line);
}

function withStableKeys(blocks: MarkdownBlock[], prefix: string): MarkdownBlock[] {
  return blocks.map((block, index) => ({
    ...block,
    key: `${prefix}-${index}-${block.key}`,
  }));
}

function withoutFootnotes(blocks: MarkdownBlock[]): MarkdownBlock[] {
  return blocks.filter((block) => block.type !== "footnotes");
}

function collectFootnoteReferencesFromText(text: string, seen: Set<string>, output: string[]) {
  const pattern = /\[\^([^\]\n]+)\]/g;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(text))) {
    const id = String(match[1] || "").trim();
    if (!id || seen.has(id)) continue;
    seen.add(id);
    output.push(id);
  }
}

function collectFootnoteReferences(blocks: MarkdownBlock[]): string[] {
  const seen = new Set<string>();
  const output: string[] = [];

  for (const block of blocks) {
    if (block.type === "heading" || block.type === "paragraph" || block.type === "quote") {
      collectFootnoteReferencesFromText(block.text, seen, output);
      continue;
    }
    if (block.type === "list") {
      block.items.forEach((item) => collectFootnoteReferencesFromText(item, seen, output));
      continue;
    }
    if (block.type === "table") {
      block.headers.forEach((cell) => collectFootnoteReferencesFromText(cell, seen, output));
      block.rows.forEach((row) => row.forEach((cell) => collectFootnoteReferencesFromText(cell, seen, output)));
    }
  }

  return output;
}

function collectFootnoteDefinitions(text: string): Map<string, string> {
  const lines = normalizeMarkdownText(text).split("\n");
  const definitions = new Map<string, string>();

  for (let index = 0; index < lines.length; index += 1) {
    const firstLineMatch = lines[index].match(/^\s{0,3}\[\^([^\]\n]+)\]:\s*(.*)$/);
    if (!firstLineMatch) continue;

    const id = String(firstLineMatch[1] || "").trim();
    const noteLines = [String(firstLineMatch[2] || "").trimEnd()];

    while (index + 1 < lines.length) {
      const nextLine = lines[index + 1];
      if (!nextLine.trim()) {
        let hasIndentedContinuation = false;
        for (let lookahead = index + 2; lookahead < lines.length; lookahead += 1) {
          const candidate = lines[lookahead];
          if (!candidate.trim()) continue;
          hasIndentedContinuation = /^(?: {4}|\t)/.test(candidate);
          break;
        }
        if (!hasIndentedContinuation) break;
        noteLines.push("");
        index += 1;
        continue;
      }

      const continuationMatch = nextLine.match(/^(?: {4}|\t)(.*)$/);
      if (!continuationMatch) break;
      noteLines.push(String(continuationMatch[1] || "").trimEnd());
      index += 1;
    }

    if (id) definitions.set(id, noteLines.join("\n").trim());
  }

  return definitions;
}

export class IncrementalMarkdownBlockParser {
  private text = "";
  private lines: string[] = [];
  private pendingStartLine = 0;
  private completedBlocks: MarkdownBlock[] = [];
  private cachedBlocks: MarkdownBlock[] = [];

  parse(input: string): MarkdownBlock[] {
    const text = normalizeMarkdownText(input);
    if (!text) {
      this.reset();
      return [];
    }

    if (text === this.text) return this.cachedBlocks;

    if (!text.startsWith(this.text)) {
      this.reset();
      this.appendChunk(text);
    } else {
      this.appendChunk(text.slice(this.text.length));
    }
    this.text = text;

    this.promoteStableBlocks();
    this.cachedBlocks = this.withFootnotes([
      ...this.completedBlocks,
      ...this.parsePendingBlocks(),
    ]);
    return this.cachedBlocks;
  }

  reset(): void {
    this.text = "";
    this.lines = [];
    this.pendingStartLine = 0;
    this.completedBlocks = [];
    this.cachedBlocks = [];
  }

  private appendChunk(chunk: string): void {
    if (!chunk) return;
    const chunkLines = chunk.split("\n");
    if (this.lines.length === 0) {
      this.lines = chunkLines;
      return;
    }

    this.lines[this.lines.length - 1] += chunkLines[0] || "";
    for (let index = 1; index < chunkLines.length; index += 1) {
      this.lines.push(chunkLines[index]);
    }
  }

  private promoteStableBlocks(): void {
    const stableBoundary = this.findStableBoundary();
    if (stableBoundary < this.pendingStartLine) return;

    const stableText = this.lines.slice(this.pendingStartLine, stableBoundary + 1).join("\n");
    if (stableText.trim()) {
      const parsed = withoutFootnotes(parseMarkdownBlocks(stableText, false));
      this.completedBlocks.push(...withStableKeys(
        parsed,
        `completed-${this.pendingStartLine}`,
      ));
    }
    this.pendingStartLine = stableBoundary + 1;
  }

  private parsePendingBlocks(): MarkdownBlock[] {
    if (this.pendingStartLine >= this.lines.length) return [];
    const pendingText = this.lines.slice(this.pendingStartLine).join("\n");
    if (!pendingText.trim()) return [];
    return withStableKeys(
      withoutFootnotes(parseMarkdownBlocks(pendingText, true)),
      `pending-${this.pendingStartLine}`,
    );
  }

  private withFootnotes(blocks: MarkdownBlock[]): MarkdownBlock[] {
    if (!this.text.includes("[^")) return blocks;
    const definitions = collectFootnoteDefinitions(this.text);
    if (definitions.size === 0) return blocks;

    const items = collectFootnoteReferences(blocks)
      .map((id) => ({ id, text: definitions.get(id) || "" }))
      .filter((item) => item.text.trim());
    if (items.length === 0) return blocks;

    return [
      ...blocks,
      {
        type: "footnotes",
        items,
        key: `footnotes-${items.map((item) => item.id).join("-")}`,
      },
    ];
  }

  private findStableBoundary(): number {
    const lastLine = this.lines.length - 1;
    let stableLine = this.pendingStartLine - 1;
    let inCodeFence = false;
    let inMathFence = false;

    for (let index = this.pendingStartLine; index < this.lines.length; index += 1) {
      const line = this.lines[index];
      const previousLine = index > this.pendingStartLine ? this.lines[index - 1] : "";
      const previousTrimmed = previousLine.trim();

      if (inCodeFence) {
        if (isCodeFenceLine(line)) {
          inCodeFence = false;
          if (index < lastLine) stableLine = index;
        }
        continue;
      }

      if (inMathFence) {
        if (isMathFenceLine(line)) {
          inMathFence = false;
          if (index < lastLine) stableLine = index;
        }
        continue;
      }

      if (isCodeFenceLine(line)) {
        if (index > this.pendingStartLine && index < lastLine) stableLine = index - 1;
        inCodeFence = true;
        continue;
      }

      if (isMathFenceLine(line)) {
        if (index > this.pendingStartLine && index < lastLine) stableLine = index - 1;
        inMathFence = true;
        continue;
      }

      if (index <= this.pendingStartLine) continue;

      if (!line.trim() && previousTrimmed && index < lastLine) {
        stableLine = index;
        continue;
      }

      if ((isHeadingLine(previousLine) || isHorizontalRuleLine(previousLine)) && index - 1 < lastLine) {
        stableLine = index - 1;
        continue;
      }

      if (
        previousTrimmed
        && isNewBlockStart(line)
        && !(isBlockquoteLine(previousLine) && isBlockquoteLine(line))
        && !(isListItemLine(previousLine) && isListItemLine(line))
        && index - 1 < lastLine
      ) {
        stableLine = index - 1;
        continue;
      }

      if (isBlockquoteLine(previousLine) && !isBlockquoteLine(line) && index < lastLine) {
        stableLine = index - 1;
        continue;
      }

      if (isListItemLine(previousLine) && !isListItemLine(line) && index < lastLine) {
        stableLine = index - 1;
        continue;
      }

      if (isTableRowLine(previousLine) && !isTableRowLine(line) && index < lastLine) {
        stableLine = index - 1;
      }
    }

    return stableLine;
  }
}

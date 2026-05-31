// ==================== Markdown Block Parser ====================
// Lightweight markdown parser based on SidebarLightMarkdown's approach,
// extended with math block detection ($$...$$ and $...$) and mermaid awareness.

export type MarkdownBlock =
  | { type: "paragraph"; text: string; key: string }
  | { type: "heading"; level: 1 | 2 | 3 | 4; text: string; key: string }
  | { type: "quote"; text: string; key: string }
  | { type: "list"; ordered: boolean; items: string[]; key: string }
  | { type: "table"; headers: string[]; rows: string[][]; key: string }
  | { type: "code"; lang: string; text: string; key: string }
  | { type: "math"; text: string; key: string }
  | { type: "hr"; key: string };

export type InlineSegment =
  | { type: "text"; text: string }
  | { type: "code"; text: string }
  | { type: "math"; text: string }
  | { type: "link"; text: string; href: string }
  | { type: "strong"; children: InlineSegment[] }
  | { type: "em"; children: InlineSegment[] }
  | { type: "strongEm"; children: InlineSegment[] }
  | { type: "delete"; children: InlineSegment[] };

// ==================== Block Parser ====================

function pushParagraph(blocks: MarkdownBlock[], lines: string[], keyPrefix: string) {
  const text = lines.join("\n").trim();
  lines.length = 0;
  if (!text) return;
  blocks.push({ type: "paragraph", text, key: `${keyPrefix}-p-${blocks.length}` });
}

function parseTableRow(line: string | undefined): string[] | null {
  const raw = String(line || "").trim();
  if (!raw.includes("|")) return null;
  const trimmed = raw.replace(/^\|/, "").replace(/\|$/, "");
  const cells = trimmed.split("|").map((cell) => cell.trim());
  if (cells.length < 2) return null;
  return cells;
}

function isTableSeparator(line: string | undefined, expectedCells: number): boolean {
  const cells = parseTableRow(line);
  if (!cells || cells.length < expectedCells) return false;
  return cells.every((cell) => /^:?-{3,}:?$/.test(cell.trim()));
}

export function parseMarkdownBlocks(input: string, streaming = false): MarkdownBlock[] {
  const normalized = String(input || "").replace(/\r\n?/g, "\n");
  const lines = normalized.split("\n");
  const result: MarkdownBlock[] = [];
  const paragraphLines: string[] = [];
  let inCode = false;
  let codeLang = "";
  let codeLines: string[] = [];
  let inMathBlock = false;
  let mathLines: string[] = [];
  let activeList: { ordered: boolean; items: string[] } | null = null;

  const flushList = () => {
    if (!activeList) return;
    result.push({
      type: "list",
      ordered: activeList.ordered,
      items: activeList.items,
      key: `list-${result.length}`,
    });
    activeList = null;
  };

  const flushParagraph = () => {
    flushList();
    pushParagraph(result, paragraphLines, "root");
  };

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex];

    // Math block: $$ on its own line
    if (!inCode && line.trim() === "$$") {
      if (inMathBlock) {
        result.push({
          type: "math",
          text: mathLines.join("\n"),
          key: `math-${result.length}`,
        });
        inMathBlock = false;
        mathLines = [];
      } else {
        flushParagraph();
        inMathBlock = true;
        mathLines = [];
      }
      continue;
    }

    if (inMathBlock) {
      mathLines.push(line);
      continue;
    }

    // Code fence
    const fenceMatch = line.match(/^(`{3,})([\w+-]*)\s*$/);
    if (fenceMatch) {
      if (inCode) {
        result.push({
          type: "code",
          lang: codeLang,
          text: codeLines.join("\n"),
          key: `code-${result.length}`,
        });
        inCode = false;
        codeLang = "";
        codeLines = [];
      } else {
        flushParagraph();
        inCode = true;
        codeLang = fenceMatch[2] || "";
        codeLines = [];
      }
      continue;
    }

    if (inCode) {
      codeLines.push(line);
      continue;
    }

    if (!line.trim()) {
      flushParagraph();
      continue;
    }

    // Horizontal rule
    const hrMatch = line.match(/^\s{0,3}([-*_])(?:\s*\1){2,}\s*$/);
    if (hrMatch) {
      flushParagraph();
      result.push({ type: "hr", key: `hr-${result.length}` });
      continue;
    }

    // Table
    const tableHeader = parseTableRow(line);
    if (tableHeader && isTableSeparator(lines[lineIndex + 1], tableHeader.length)) {
      flushParagraph();
      lineIndex += 2;
      const rows: string[][] = [];
      while (lineIndex < lines.length) {
        const row = parseTableRow(lines[lineIndex]);
        if (!row) break;
        rows.push(row);
        lineIndex += 1;
      }
      lineIndex -= 1;
      result.push({
        type: "table",
        headers: tableHeader,
        rows,
        key: `table-${result.length}`,
      });
      continue;
    }

    // Heading
    const headingMatch = line.match(/^\s{0,3}(#{1,4})\s+(.+?)\s*#*\s*$/);
    if (headingMatch) {
      flushParagraph();
      result.push({
        type: "heading",
        level: headingMatch[1].length as 1 | 2 | 3 | 4,
        text: headingMatch[2].trim(),
        key: `heading-${result.length}`,
      });
      continue;
    }

    // Blockquote
    const quoteMatch = line.match(/^\s{0,3}>\s?(.*)$/);
    if (quoteMatch) {
      flushParagraph();
      result.push({
        type: "quote",
        text: quoteMatch[1].trim(),
        key: `quote-${result.length}`,
      });
      continue;
    }

    // List item
    const listMatch = line.match(/^\s{0,3}(?:([-*+])|(\d+)[.)])\s+(.+)$/);
    if (listMatch) {
      pushParagraph(result, paragraphLines, "list-before");
      const ordered = !!listMatch[2];
      if (!activeList || activeList.ordered !== ordered) {
        flushList();
        activeList = { ordered, items: [] };
      }
      activeList.items.push(listMatch[3].trim());
      continue;
    }

    flushList();
    paragraphLines.push(line);
  }

  // Flush remaining
  if (inCode) {
    if (!streaming) {
      // 非流式：未闭合也输出（最终态）
      result.push({
        type: "code",
        lang: codeLang,
        text: codeLines.join("\n"),
        key: `code-${result.length}`,
      });
    }
    // 流式：未闭合的代码块不输出，等闭合后再显示
  }
  if (inMathBlock) {
    if (!streaming) {
      result.push({
        type: "math",
        text: mathLines.join("\n"),
        key: `math-${result.length}`,
      });
    }
  }
  flushParagraph();
  return result.length > 0 ? result : [{ type: "paragraph", text: normalized, key: "fallback" }];
}

// ==================== Inline Parser ====================

const URL_PATTERN = /(https?:\/\/[^\s<>()]+|file:\/\/\/[^\s<>()]+)/g;
const MARKDOWN_LINK_PATTERN = /!?\[([^\]\n]+)\]\(([^)\n]+)\)/g;

function trimTrailingUrlPunctuation(value: string): { href: string; trailing: string } {
  let href = value;
  let trailing = "";
  while (/[.,;:!?，。！？；：、]$/.test(href)) {
    trailing = `${href.slice(-1)}${trailing}`;
    href = href.slice(0, -1);
  }
  return { href, trailing };
}

function pushTextSegment(segments: InlineSegment[], text: string) {
  if (!text) return;
  const previous = segments[segments.length - 1];
  if (previous?.type === "text") {
    previous.text += text;
    return;
  }
  segments.push({ type: "text", text });
}

type LinkMatch =
  | { kind: "markdown"; start: number; end: number; raw: string; text: string; href: string }
  | { kind: "auto"; start: number; end: number; href: string };

function pickEarlierLink(left: LinkMatch | null, right: LinkMatch | null): LinkMatch | null {
  if (!left) return right;
  if (!right) return left;
  return left.start <= right.start ? left : right;
}

function nextMarkdownLink(input: string, from: number): LinkMatch | null {
  MARKDOWN_LINK_PATTERN.lastIndex = from;
  const match = MARKDOWN_LINK_PATTERN.exec(input);
  if (!match) return null;
  return {
    kind: "markdown",
    start: match.index,
    end: match.index + match[0].length,
    raw: match[0],
    text: match[1],
    href: match[2],
  };
}

function nextAutoLink(input: string, from: number): LinkMatch | null {
  URL_PATTERN.lastIndex = from;
  const match = URL_PATTERN.exec(input);
  if (!match) return null;
  return {
    kind: "auto",
    start: match.index,
    end: match.index + match[0].length,
    href: match[0],
  };
}

function normalizeMarkdownHref(rawHref: string): string {
  let href = String(rawHref || "").trim();
  if (!href) return "";
  const titleMatch = href.match(/^(.+?)\s+["'][^"']*["']$/);
  if (titleMatch) href = titleMatch[1].trim();
  if (href.startsWith("<") && href.endsWith(">")) {
    href = href.slice(1, -1).trim();
  }
  return href;
}

function findNextInlineMarker(
  text: string,
  from: number,
): { type: "strong" | "em" | "strongEm" | "delete"; start: number; end: number; inner: string } | null {
  const patterns: Array<{ type: "strong" | "em" | "strongEm" | "delete"; marker: string }> = [
    { type: "strongEm", marker: "***" },
    { type: "delete", marker: "~~" },
    { type: "strong", marker: "**" },
    { type: "em", marker: "*" },
  ];
  let best: { type: "strong" | "em" | "strongEm" | "delete"; start: number; end: number; inner: string } | null = null;
  for (const pattern of patterns) {
    const start = text.indexOf(pattern.marker, from);
    if (start < 0) continue;
    if (pattern.marker === "*" && text[start + 1] === "*") continue;
    const contentStart = start + pattern.marker.length;
    const endMarker = text.indexOf(pattern.marker, contentStart);
    if (endMarker < 0 || endMarker === contentStart) continue;
    const candidate = {
      type: pattern.type,
      start,
      end: endMarker + pattern.marker.length,
      inner: text.slice(contentStart, endMarker),
    };
    if (!best || candidate.start < best.start || (candidate.start === best.start && candidate.end > best.end)) {
      best = candidate;
    }
  }
  return best;
}

function pushEmphasisIntoSegments(input: string, segments: InlineSegment[]) {
  const text = String(input || "");
  let cursor = 0;
  while (cursor < text.length) {
    const matched = findNextInlineMarker(text, cursor);
    if (!matched) {
      pushTextSegment(segments, text.slice(cursor));
      break;
    }
    if (matched.start > cursor) {
      pushTextSegment(segments, text.slice(cursor, matched.start));
    }
    segments.push({
      type: matched.type,
      children: parseInlineSegments(matched.inner),
    } as InlineSegment);
    cursor = matched.end;
  }
}

function parseLinksIntoSegments(input: string, segments: InlineSegment[]) {
  let cursor = 0;
  while (cursor < input.length) {
    const markdownLink = nextMarkdownLink(input, cursor);
    const autoLink = nextAutoLink(input, cursor);
    const next = pickEarlierLink(markdownLink, autoLink);
    if (!next) break;
    pushEmphasisIntoSegments(input.slice(cursor, next.start), segments);
    if (next.kind === "markdown") {
      const href = normalizeMarkdownHref(next.href);
      if (href) {
        segments.push({ type: "link", href, text: next.text });
      } else {
        pushEmphasisIntoSegments(next.raw, segments);
      }
    } else {
      const { href, trailing } = trimTrailingUrlPunctuation(next.href);
      if (href) segments.push({ type: "link", href, text: href });
      pushEmphasisIntoSegments(trailing, segments);
    }
    cursor = next.end;
  }
  pushEmphasisIntoSegments(input.slice(cursor), segments);
}

/**
 * Parse inline text into segments, handling: inline code, inline math ($...$),
 * markdown links, auto-links, bold, italic, bold-italic, strikethrough.
 */
export function parseInlineSegments(input: string): InlineSegment[] {
  const segments: InlineSegment[] = [];
  const text = String(input || "");
  let cursor = 0;

  // Combined pattern for inline code and inline math
  // Inline code: `...`  Inline math: $...$  (not $$)
  const inlinePattern = /`([^`]+)`|\$([^$\n]+)\$/g;
  let match: RegExpExecArray | null;
  while ((match = inlinePattern.exec(text))) {
    if (match.index > cursor) {
      parseLinksIntoSegments(text.slice(cursor, match.index), segments);
    }
    if (match[1] !== undefined) {
      // Inline code
      segments.push({ type: "code", text: match[1] });
    } else if (match[2] !== undefined) {
      // Inline math — verify it's not preceded/followed by $ (which would be $$)
      const before = match.index > 0 ? text[match.index - 1] : "";
      const after = match.index + match[0].length < text.length ? text[match.index + match[0].length] : "";
      if (before === "$" || after === "$") {
        // Part of $$, treat as text
        parseLinksIntoSegments(match[0], segments);
      } else {
        segments.push({ type: "math", text: match[2] });
      }
    }
    cursor = match.index + match[0].length;
  }
  if (cursor < text.length) {
    parseLinksIntoSegments(text.slice(cursor), segments);
  }
  return segments;
}

export function normalizedTableRow(row: string[], size: number): string[] {
  return Array.from({ length: Math.max(1, size) }, (_item, index) => row[index] || "");
}

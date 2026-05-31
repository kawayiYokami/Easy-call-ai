<template>
  <div class="ecall-sidebar-light-markdown">
    <template v-for="(block, index) in blocks" :key="`${block.type}-${index}-${block.key}`">
      <component
        :is="headingTag(block.level)"
        v-if="block.type === 'heading'"
        class="ecall-sidebar-light-heading"
      >
        <InlineText :text="block.text" />
      </component>
      <blockquote v-else-if="block.type === 'quote'" class="ecall-sidebar-light-quote">
        <InlineText :text="block.text" />
      </blockquote>
      <ul v-else-if="block.type === 'list' && !block.ordered" class="ecall-sidebar-light-list">
        <li v-for="(item, itemIndex) in block.items" :key="`${index}-${itemIndex}`">
          <InlineText :text="item" />
        </li>
      </ul>
      <ol v-else-if="block.type === 'list'" class="ecall-sidebar-light-list">
        <li v-for="(item, itemIndex) in block.items" :key="`${index}-${itemIndex}`">
          <InlineText :text="item" />
        </li>
      </ol>
      <div v-else-if="block.type === 'table'" class="ecall-sidebar-light-table-wrap">
        <table class="ecall-sidebar-light-table">
          <thead>
            <tr>
              <th v-for="(cell, cellIndex) in block.headers" :key="`${index}-h-${cellIndex}`">
                <InlineText :text="cell" />
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(row, rowIndex) in block.rows" :key="`${index}-r-${rowIndex}`">
              <td v-for="(cell, cellIndex) in normalizedTableRow(row, block.headers.length)" :key="`${index}-r-${rowIndex}-c-${cellIndex}`">
                <InlineText :text="cell" />
              </td>
            </tr>
          </tbody>
        </table>
      </div>
      <div v-else-if="block.type === 'code'" class="ecall-sidebar-light-code-wrap">
        <button
          type="button"
          class="ecall-sidebar-light-copy"
          :title="copiedCodeKey === block.key ? '已复制' : '复制代码'"
          @click="copyCodeBlock(block.key, block.text)"
        >
          {{ copiedCodeKey === block.key ? "已复制" : "复制" }}
        </button>
        <pre class="ecall-sidebar-light-code"><code>{{ block.text }}</code></pre>
      </div>
      <hr v-else-if="block.type === 'hr'" class="ecall-sidebar-light-hr" />
      <p v-else class="ecall-sidebar-light-paragraph">
        <InlineText :text="block.text" />
      </p>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, defineComponent, h, onBeforeUnmount, ref, type VNodeChild } from "vue";

type LightMarkdownBlock =
  | { type: "paragraph"; text: string; key: string }
  | { type: "heading"; level: 1 | 2 | 3 | 4; text: string; key: string }
  | { type: "quote"; text: string; key: string }
  | { type: "list"; ordered: boolean; items: string[]; key: string }
  | { type: "table"; headers: string[]; rows: string[][]; key: string }
  | { type: "code"; lang: string; text: string; key: string }
  | { type: "hr"; key: string };

type InlineSegment =
  | { type: "text"; text: string }
  | { type: "code"; text: string }
  | { type: "link"; text: string; href: string }
  | { type: "strong"; children: InlineSegment[] }
  | { type: "em"; children: InlineSegment[] }
  | { type: "strongEm"; children: InlineSegment[] }
  | { type: "delete"; children: InlineSegment[] };

const props = defineProps<{
  text: string;
}>();

const URL_PATTERN = /(https?:\/\/[^\s<>()]+|file:\/\/\/[^\s<>()]+)/g;
const MARKDOWN_LINK_PATTERN = /!?\[([^\]\n]+)\]\(([^)\n]+)\)/g;
const copiedCodeKey = ref("");
let copiedCodeTimer = 0;

async function copyCodeBlock(key: string, text: string) {
  try {
    await navigator.clipboard.writeText(String(text || ""));
    copiedCodeKey.value = key;
    if (copiedCodeTimer) window.clearTimeout(copiedCodeTimer);
    copiedCodeTimer = window.setTimeout(() => {
      copiedCodeKey.value = "";
      copiedCodeTimer = 0;
    }, 1200);
  } catch {
    copiedCodeKey.value = "";
  }
}

onBeforeUnmount(() => {
  if (copiedCodeTimer) window.clearTimeout(copiedCodeTimer);
});

const InlineText = defineComponent({
  name: "InlineText",
  props: {
    text: {
      type: String,
      required: true,
    },
  },
  setup(inlineProps) {
    return () => parseInlineSegments(inlineProps.text).map((segment, index) => {
      if (segment.type === "code") {
        return h("code", { key: `code-${index}`, class: "ecall-sidebar-light-inline-code" }, segment.text);
      }
      if (segment.type === "link") {
        return h("a", {
          key: `link-${index}`,
          href: segment.href,
          class: "ecall-sidebar-light-link",
        }, segment.text);
      }
      if (segment.type === "strong") {
        return h("strong", { key: `strong-${index}`, class: "ecall-sidebar-light-strong" }, renderInlineSegments(segment.children, `strong-${index}`));
      }
      if (segment.type === "em") {
        return h("em", { key: `em-${index}`, class: "ecall-sidebar-light-em" }, renderInlineSegments(segment.children, `em-${index}`));
      }
      if (segment.type === "strongEm") {
        return h("strong", { key: `strong-em-${index}`, class: "ecall-sidebar-light-strong" }, [
          h("em", { class: "ecall-sidebar-light-em" }, renderInlineSegments(segment.children, `strong-em-${index}`)),
        ]);
      }
      if (segment.type === "delete") {
        return h("del", { key: `delete-${index}`, class: "ecall-sidebar-light-delete" }, renderInlineSegments(segment.children, `delete-${index}`));
      }
      return segment.text;
    });
  },
});

const blocks = computed(() => parseLightMarkdown(props.text));

function headingTag(level: unknown): "h1" | "h2" | "h3" | "h4" {
  const normalized = Math.min(4, Math.max(1, Number(level) || 4));
  return `h${normalized}` as "h1" | "h2" | "h3" | "h4";
}

function pushParagraph(blocks: LightMarkdownBlock[], lines: string[], keyPrefix: string) {
  const text = lines.join("\n").trim();
  lines.length = 0;
  if (!text) return;
  blocks.push({ type: "paragraph", text, key: `${keyPrefix}-p-${blocks.length}` });
}

function normalizedTableRow(row: string[], size: number): string[] {
  return Array.from({ length: Math.max(1, size) }, (_item, index) => row[index] || "");
}

function parseLightMarkdown(input: string): LightMarkdownBlock[] {
  const normalized = String(input || "").replace(/\r\n?/g, "\n");
  const lines = normalized.split("\n");
  const result: LightMarkdownBlock[] = [];
  const paragraphLines: string[] = [];
  let inCode = false;
  let codeLang = "";
  let codeLines: string[] = [];
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
    const fenceMatch = line.match(/^```([\w-]*)\s*$/);
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
        codeLang = fenceMatch[1] || "";
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

    const hrMatch = line.match(/^\s{0,3}([-*_])(?:\s*\1){2,}\s*$/);
    if (hrMatch) {
      flushParagraph();
      result.push({ type: "hr", key: `hr-${result.length}` });
      continue;
    }

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

  if (inCode) {
    result.push({
      type: "code",
      lang: codeLang,
      text: codeLines.join("\n"),
      key: `code-${result.length}`,
    });
  }
  flushParagraph();
  return result.length > 0 ? result : [{ type: "paragraph", text: normalized, key: "fallback" }];
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

function renderInlineSegments(segments: InlineSegment[], keyPrefix: string): VNodeChild[] {
  return segments.map((segment, index) => {
    if (segment.type === "code") {
      return h("code", { key: `${keyPrefix}-code-${index}`, class: "ecall-sidebar-light-inline-code" }, segment.text);
    }
    if (segment.type === "link") {
      return h("a", {
        key: `${keyPrefix}-link-${index}`,
        href: segment.href,
        class: "ecall-sidebar-light-link",
      }, segment.text);
    }
    if (segment.type === "strong") {
      return h("strong", { key: `${keyPrefix}-strong-${index}`, class: "ecall-sidebar-light-strong" }, renderInlineSegments(segment.children, `${keyPrefix}-strong-${index}`));
    }
    if (segment.type === "em") {
      return h("em", { key: `${keyPrefix}-em-${index}`, class: "ecall-sidebar-light-em" }, renderInlineSegments(segment.children, `${keyPrefix}-em-${index}`));
    }
    if (segment.type === "strongEm") {
      return h("strong", { key: `${keyPrefix}-strong-em-${index}`, class: "ecall-sidebar-light-strong" }, [
        h("em", { class: "ecall-sidebar-light-em" }, renderInlineSegments(segment.children, `${keyPrefix}-strong-em-${index}`)),
      ]);
    }
    if (segment.type === "delete") {
      return h("del", { key: `${keyPrefix}-delete-${index}`, class: "ecall-sidebar-light-delete" }, renderInlineSegments(segment.children, `${keyPrefix}-delete-${index}`));
    }
    return segment.text;
  });
}

function parseInlineSegments(input: string): InlineSegment[] {
  const segments: InlineSegment[] = [];
  const text = String(input || "");
  let cursor = 0;
  const inlineCodePattern = /`([^`]+)`/g;
  let codeMatch: RegExpExecArray | null;
  while ((codeMatch = inlineCodePattern.exec(text))) {
    parseLinksIntoSegments(text.slice(cursor, codeMatch.index), segments);
    segments.push({ type: "code", text: codeMatch[1] });
    cursor = codeMatch.index + codeMatch[0].length;
  }
  parseLinksIntoSegments(text.slice(cursor), segments);
  return segments;
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
  if ((href.startsWith("<") && href.endsWith(">"))) {
    href = href.slice(1, -1).trim();
  }
  return href;
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
</script>

<style scoped>
.ecall-sidebar-light-markdown {
  min-width: 0;
  max-width: 100%;
  overflow-wrap: anywhere;
  white-space: normal;
  font-size: 0.875rem;
  line-height: 1.5;
}

.ecall-sidebar-light-heading,
.ecall-sidebar-light-paragraph,
.ecall-sidebar-light-quote,
.ecall-sidebar-light-list,
.ecall-sidebar-light-code {
  margin: 0.25rem 0;
}

.ecall-sidebar-light-heading {
  font-weight: 650;
  line-height: 1.45;
}

h1.ecall-sidebar-light-heading {
  font-size: 1.02rem;
}

h2.ecall-sidebar-light-heading {
  font-size: 0.98rem;
}

h3.ecall-sidebar-light-heading {
  font-size: 0.94rem;
}

h4.ecall-sidebar-light-heading {
  font-size: 0.9rem;
}

.ecall-sidebar-light-paragraph {
  white-space: pre-wrap;
}

.ecall-sidebar-light-quote {
  border-left: 2px solid color-mix(in srgb, currentColor 22%, transparent);
  padding-left: 0.68rem;
  color: color-mix(in srgb, currentColor 82%, transparent);
  white-space: pre-wrap;
}

.ecall-sidebar-light-list {
  padding-left: 0.85rem;
}

.ecall-sidebar-light-list li {
  margin: 0.12rem 0;
  padding-left: 0;
}

ol.ecall-sidebar-light-list {
  list-style: decimal;
}

.ecall-sidebar-light-table-wrap {
  max-width: 100%;
  overflow-x: auto;
  margin: 0.35rem 0;
}

.ecall-sidebar-light-table {
  width: max-content;
  min-width: 100%;
  border-collapse: collapse;
  font-size: 0.84rem;
  line-height: 1.45;
}

.ecall-sidebar-light-table th,
.ecall-sidebar-light-table td {
  border: 1px solid color-mix(in srgb, currentColor 20%, transparent);
  padding: 0.32rem 0.48rem;
  text-align: left;
  vertical-align: top;
}

.ecall-sidebar-light-table th {
  font-weight: 650;
  background: color-mix(in srgb, currentColor 7%, transparent);
}

.ecall-sidebar-light-code-wrap {
  position: relative;
  margin: 0.25rem 0;
}

.ecall-sidebar-light-code {
  max-width: 100%;
  overflow-x: auto;
  border-radius: 0.4rem;
  background: color-mix(in srgb, currentColor 8%, transparent);
  padding: 1.8rem 0.65rem 0.55rem;
  white-space: pre;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  font-size: 0.82rem;
  line-height: 1.45;
}

.ecall-sidebar-light-copy {
  position: absolute;
  right: 0.4rem;
  top: 0.35rem;
  z-index: 1;
  border: 1px solid color-mix(in srgb, currentColor 18%, transparent);
  border-radius: 0.3rem;
  background: color-mix(in srgb, currentColor 8%, var(--color-base-100, transparent));
  padding: 0.1rem 0.38rem;
  font-size: 0.72rem;
  line-height: 1.35;
  color: color-mix(in srgb, currentColor 78%, transparent);
}

.ecall-sidebar-light-copy:hover {
  background: color-mix(in srgb, currentColor 13%, var(--color-base-100, transparent));
  color: currentColor;
}

.ecall-sidebar-light-code code {
  border: 0 !important;
  background: transparent !important;
  box-shadow: none !important;
  padding: 0 !important;
  color: inherit;
  font: inherit;
}

.ecall-sidebar-light-inline-code {
  border-radius: 0.28rem;
  background: color-mix(in srgb, currentColor 10%, transparent);
  padding: 0.08rem 0.28rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  font-size: 0.86em;
}

.ecall-sidebar-light-link {
  color: var(--color-primary);
  text-decoration: underline;
  text-decoration-thickness: 0.08em;
  text-underline-offset: 0.18em;
}

.ecall-sidebar-light-strong {
  font-weight: 700;
}

.ecall-sidebar-light-em {
  font-style: italic;
}

.ecall-sidebar-light-delete {
  text-decoration: line-through;
  color: color-mix(in srgb, currentColor 76%, transparent);
}

.ecall-sidebar-light-hr {
  margin: 0.65rem 0;
  border: 0;
  border-top: 1px solid color-mix(in srgb, currentColor 16%, transparent);
}
</style>

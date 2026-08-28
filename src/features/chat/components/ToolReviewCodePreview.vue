<template>
  <div class="flex h-full min-h-0 w-full flex-col gap-2">
    <div v-if="title" class="px-4 pt-3 text-xs text-base-content/65">{{ title }}</div>
    <!-- embedded 模式：不自建滚动区，纵/横均由外层 MultiFileDiffView 统一接管 -->
    <div
      v-if="props.embedded"
      class="tool-review-code-main tool-review-code-main-embedded min-h-0 flex-none"
      :class="{
        'tool-review-code-main-with-lines': showGutter,
        'tool-review-code-main-with-patch-lines': isPatchMode,
        'tool-review-code-main--dark': !!props.isDark,
      }"
    >
      <pre v-if="!highlightedHtml" class="tool-review-raw-pre">{{ code }}</pre>
      <div v-else class="tool-review-code-view" v-html="highlightedHtml" @click="onViewClick"></div>
    </div>
    <OverlayScrollArea
      v-else
      class="tool-review-code-main min-h-0 flex-1"
      :class="{
        'tool-review-code-main-with-lines': showGutter,
        'tool-review-code-main-with-patch-lines': isPatchMode,
        'tool-review-code-main--dark': !!props.isDark,
      }"
      orientation="both"
      variant="code-dark"
      scroller-class="tool-review-code-scroller h-full"
    >
      <pre v-if="!highlightedHtml" class="tool-review-raw-pre">{{ code }}</pre>
      <div v-else class="tool-review-code-view" v-html="highlightedHtml" @click="onViewClick"></div>
    </OverlayScrollArea>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { bundledLanguagesInfo, codeToHtml } from "shiki";
import OverlayScrollArea from "../../shared/components/OverlayScrollArea.vue";

const props = defineProps<{
  title?: string;
  code: string;
  lang?: string;
  mode?: "plain" | "patch";
  isDark?: boolean;
  showLineNumbers?: boolean;
  /** 当前 diff 的 -U 上下文行数（决定尾部截断检测阈值），默认 3 */
  contextLines?: number;
  /** 嵌入多文件列表时：高度自适应、仅横向滚动，纵向由外层接管 */
  embedded?: boolean;
}>();

const emit = defineEmits<{
  (e: "expandGap"): void;
}>();

const baseHighlightedHtml = ref("");
let highlightAbort: AbortController | null = null;

/** hunk 行号间隙：startLine 是 diff 文本中展开条应插入的行索引（head/between 为间隙后的 hunk 头，tail 为行数末尾） */
const rawGaps = ref<Array<{ startLine: number; kind: "head" | "between" | "tail"; count?: number }>>([]);

const highlightedHtml = computed(() => {
  const base = baseHighlightedHtml.value;
  if (!base) return "";
  return applyGaps(base, rawGaps.value);
});

const SHIKI_LANGUAGE_KEYS = new Set(
  bundledLanguagesInfo.flatMap((item) => [item.id, ...(item.aliases || [])]).map((item) => item.toLowerCase()),
);

const isPatchMode = computed(() => props.mode === "patch");
const showGutter = computed(() => isPatchMode.value || props.showLineNumbers === true);

async function updateHighlightedCode() {
  const rawCode = String(props.code || "");
  if (!rawCode.trim()) {
    baseHighlightedHtml.value = "";
    rawGaps.value = [];
    return;
  }
  if (highlightAbort) highlightAbort.abort();
  highlightAbort = new AbortController();
  const signal = highlightAbort.signal;
  const language = resolveLanguage();
  const code = rawCode;
  try {
    const html = await codeToHtml(code, {
      lang: language,
      theme: props.isDark ? "github-dark" : "github-light",
    });
    if (signal.aborted) return;
    const { html: normalized, gaps } = normalizeShikiLineHtml(html, code, props.mode);
    baseHighlightedHtml.value = normalized;
    rawGaps.value = gaps;
  } catch {
    if (signal.aborted) return;
    baseHighlightedHtml.value = "";
    rawGaps.value = [];
  }
}

/** 在 hunk 头的行标签前插入折叠条；tail 追加到末尾（从后往前处理，保证行索引稳定） */
function applyGaps(base: string, gaps: Array<{ startLine: number; kind: "head" | "between" | "tail"; count?: number }>) {
  if (!isPatchMode.value || gaps.length === 0) return base;
  let result = base;
  for (let idx = gaps.length - 1; idx >= 0; idx -= 1) {
    const gap = gaps[idx];
    if (gap.kind === "tail") {
      const codeEnd = result.lastIndexOf("</code>");
      if (codeEnd < 0) continue;
      result = result.slice(0, codeEnd) + buildGapBarHtml(gap.kind, gap.count) + result.slice(codeEnd);
      continue;
    }
    const lineStart = findNthLineStart(base, gap.startLine);
    if (lineStart < 0) continue;
    result = result.slice(0, lineStart) + buildGapBarHtml(gap.kind, gap.count) + result.slice(lineStart);
  }
  return result;
}

const LINE_START_PATTERN = /<span class="line[ "]/g;

function findNthLineStart(html: string, lineIndex: number): number {
  LINE_START_PATTERN.lastIndex = 0;
  let count = 0;
  let match: RegExpExecArray | null;
  while ((match = LINE_START_PATTERN.exec(html)) !== null) {
    if (count === lineIndex) return match.index;
    count += 1;
  }
  return -1;
}

function buildGapBarHtml(kind: "head" | "between" | "tail" = "between", count?: number) {
  // 参考 git-diff-view 的 ExpandUp / ExpandDown / ExpandAll（GitHub 风格的箭头+虚线图标）
  const iconUp = `<svg aria-hidden="true" height="14" width="14" viewBox="0 0 16 16" fill="currentColor"><path d="M7.823 1.677 4.927 4.573A.25.25 0 0 0 5.104 5H7.25v3.236a.75.75 0 1 0 1.5 0V5h2.146a.25.25 0 0 0 .177-.427L8.177 1.677a.25.25 0 0 0-.354 0ZM13.75 11a.75.75 0 0 0 0 1.5h.5a.75.75 0 0 0 0-1.5h-.5Zm-3.75.75a.75.75 0 0 1 .75-.75h.5a.75.75 0 0 1 0 1.5h-.5a.75.75 0 0 1-.75-.75ZM7.75 11a.75.75 0 0 0 0 1.5h.5a.75.75 0 0 0 0-1.5h-.5ZM4 11.75a.75.75 0 0 1 .75-.75h.5a.75.75 0 0 1 0 1.5h-.5a.75.75 0 0 1-.75-.75ZM1.75 11a.75.75 0 0 0 0 1.5h.5a.75.75 0 0 0 0-1.5h-.5Z"></path></svg>`;
  const iconDown = `<svg aria-hidden="true" height="14" width="14" viewBox="0 0 16 16" fill="currentColor"><path d="m8.177 14.323 2.896-2.896a.25.25 0 0 0-.177-.427H8.75V7.764a.75.75 0 1 0-1.5 0V11H5.104a.25.25 0 0 0-.177.427l2.896 2.896a.25.25 0 0 0 .354 0ZM2.25 5a.75.75 0 0 0 0-1.5h-.5a.75.75 0 0 0 0 1.5h.5ZM6 4.25a.75.75 0 0 1-.75.75h-.5a.75.75 0 0 1 0-1.5h.5a.75.75 0 0 1 .75.75ZM8.25 5a.75.75 0 0 0 0-1.5h-.5a.75.75 0 0 0 0 1.5h.5ZM12 4.25a.75.75 0 0 1-.75.75h-.5a.75.75 0 0 1 0-1.5h.5a.75.75 0 0 1 .75.75Zm2.25.75a.75.75 0 0 0 0-1.5h-.5a.75.75 0 0 0 0 1.5h.5Z"></path></svg>`;
  const iconAll = `<svg aria-hidden="true" height="14" width="14" viewBox="0 0 16 16" fill="currentColor"><path d="m8.177.677 2.896 2.896a.25.25 0 0 1-.177.427H8.75v1.25a.75.75 0 0 1-1.5 0V4H5.104a.25.25 0 0 1-.177-.427L7.823.677a.25.25 0 0 1 .354 0ZM7.25 10.75a.75.75 0 0 1 1.5 0V12h2.146a.25.25 0 0 1 .177.427l-2.896 2.896a.25.25 0 0 1-.354 0l-2.896-2.896A.25.25 0 0 1 5.104 12H7.25v-1.25Zm-5-2a.75.75 0 0 0 0-1.5h-.5a.75.75 0 0 0 0 1.5h.5ZM6 8a.75.75 0 0 1-.75.75h-.5a.75.75 0 0 1 0-1.5h.5A.75.75 0 0 1 6 8Zm2.25.75a.75.75 0 0 0 0-1.5h-.5a.75.75 0 0 0 0 1.5h.5ZM12 8a.75.75 0 0 1-.75.75h-.5a.75.75 0 0 1 0-1.5h.5A.75.75 0 0 1 12 8Zm2.25.75a.75.75 0 0 0 0-1.5h-.5a.75.75 0 0 0 0 1.5h.5Z"></path></svg>`;
  let icon = iconAll;
  if (kind === "head") icon = iconUp;
  else if (kind === "tail") icon = iconDown;
  else icon = iconAll;
  const label = count != null && count > 0 ? `${count} 个隐藏的行` : "展开被折叠的内容";
  return `<span class="line tool-review-gap-bar" data-gap-kind="${kind}"><span class="line-code"><span class="tool-review-gap-bar-icon">${icon}</span><span>${escapeHtmlAttribute(label)}</span></span></span>`;
}

function onViewClick(event: MouseEvent) {
  const target = event.target as HTMLElement;
  const bar = target.closest<HTMLElement>(".tool-review-gap-bar");
  if (!bar) return;
  emit("expandGap");
}

function stripPatchHeading(code: string) {
  // git 会在 hunk 头后附加 section heading（段落首行，如 `@@ -1,5 +1,5 @@ fn main()`）
  return code.replace(/^(@@.*?@@).*$/gm, "$1");
}

function resolveLanguage() {
  const requested = String(props.lang || "").trim().toLowerCase();
  const key = props.mode === "patch" ? "diff" : requested || "text";
  return SHIKI_LANGUAGE_KEYS.has(key) ? key : "text";
}

function normalizeShikiLineHtml(html: string, code: string, mode?: "plain" | "patch") {
  const compactHtml = html.replace(/<\/span>\s+<span class="line"/g, '</span><span class="line"');
  if (mode !== "patch" && props.showLineNumbers !== true) return { html: compactHtml, gaps: [] };
  const { meta, gaps } = mode === "patch"
    ? buildPatchLineMeta(code, Math.max(1, props.contextLines || 3))
    : { meta: buildPlainLineMeta(code), gaps: [] };
  let lineIndex = 0;
  let normalized = compactHtml.replace(/<span class="line"/g, () => {
    const metaLine = meta[lineIndex] || { gutter: "", kindClass: "" };
    lineIndex += 1;
    return `<span class="line ${metaLine.kindClass}" data-gutter="${escapeHtmlAttribute(metaLine.gutter)}"`;
  });
  // hunk 头友好文案：用函数/类签名 + 行号替代原始 @@，原始 @@ 放 title
  if (mode === "patch") {
    const headerTargets: Array<{ idx: number; label: string; raw: string }> = [];
    const rawLines = String(code || "").split("\n");
    meta.forEach((m, idx) => {
      const anyM = m as { hunkLabel?: string };
      if (anyM.hunkLabel) headerTargets.push({ idx, label: anyM.hunkLabel, raw: rawLines[idx] || "" });
    });
    for (let t = headerTargets.length - 1; t >= 0; t -= 1) {
      const { idx, label, raw } = headerTargets[t];
      const start = findNthLineStart(normalized, idx);
      if (start < 0) continue;
      const nextStart = findNthLineStart(normalized, idx + 1);
      const lineEnd = nextStart >= 0 ? nextStart : normalized.lastIndexOf("</code>");
      if (lineEnd < 0) continue;
      const openEnd = normalized.indexOf(">", start);
      if (openEnd < 0 || openEnd >= lineEnd) continue;
      const closeStart = normalized.lastIndexOf("</span>", lineEnd - 1);
      if (closeStart < 0 || closeStart <= openEnd) continue;
      const friendly = `<span class="tool-review-hunk-label" title="${escapeHtmlAttribute(raw)}">${escapeHtmlAttribute(label)}</span>`;
      normalized = normalized.slice(0, openEnd + 1) + friendly + normalized.slice(closeStart);
    }
  }
  return { html: normalized, gaps };
}

function buildPlainLineMeta(code: string) {
  const lineCount = String(code || "").split("\n").length;
  return Array.from({ length: lineCount }, (_, index) => ({
    gutter: String(index + 1),
    kindClass: "tool-review-plain-line",
  }));
}

function buildPatchLineMeta(code: string, contextLines = 3) {
  const lines = String(code || "").split("\n");
  const out = [] as Array<{ gutter: string; kindClass: string; hunkLabel?: string }>;
  const gaps = [] as Array<{ startLine: number; kind: "head" | "between" | "tail"; count?: number }>;
  let oldLineNumber: number | null = null;
  let newLineNumber: number | null = null;
  let sawHeader = false;

  for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
    const line = lines[lineIndex];
    // git diff 真实格式：@@ -oldStart,oldCount +newStart,newCount @@（count 为 1 时可省略）
    const gitHeaderMatch = line.match(/^@@\s+-(\d+)(?:,\d+)?\s+\+(\d+)(?:,\d+)?\s+@@(.*)$/);
    if (gitHeaderMatch) {
      const nextOldStart = Number(gitHeaderMatch[1]);
      const heading = String(gitHeaderMatch[3] || "").trim();
      // 行号已在左侧 gutter 展示，header 不再拼接“第 X 行”
      const friendly = heading;
      // 文件头部：第一个 hunk 的旧起始行号 > 1 说明开头有省略
      if (!sawHeader && nextOldStart > 1) {
        gaps.push({ startLine: lineIndex, kind: "head", count: nextOldStart - 1 });
      }
      // 中间间隙：与上一个 hunk 的旧文件行号有距离说明有省略
      if (oldLineNumber != null && nextOldStart > oldLineNumber) {
        gaps.push({ startLine: lineIndex, kind: "between", count: nextOldStart - oldLineNumber });
      }
      sawHeader = true;
      oldLineNumber = nextOldStart;
      newLineNumber = Number(gitHeaderMatch[2]);
      out.push({ gutter: "", kindClass: "tool-review-patch-line-header", hunkLabel: friendly });
      continue;
    }
    // 工具生成的伪格式：@@ lines 10-15 @@ / @@ line 10 @@
    const pseudoHeaderMatch = line.match(/^@@\s+lines?\s+(\d+)(?:-\d+)?(?:\s*,\s*\d+(?:-\d+)?)?\s+@@/i);
    if (pseudoHeaderMatch) {
      const start = Number(pseudoHeaderMatch[1]);
      oldLineNumber = Number.isFinite(start) ? start : null;
      newLineNumber = Number.isFinite(start) ? start : null;
      out.push({ gutter: "", kindClass: "tool-review-patch-line-header" });
      continue;
    }
    // git diff 元信息行：diff --git / index / --- a/ / +++ b/，不参与行号，正文隐藏
    if (
      line.startsWith("diff --git ") ||
      /^index [0-9a-f]{7,}\.\.[0-9a-f]{7,}/.test(line) ||
      /^--- (a\/|\/dev\/null)/.test(line) ||
      /^\+\+\+ (b\/|\/dev\/null)/.test(line)
    ) {
      out.push({ gutter: "", kindClass: "tool-review-patch-line-meta" });
      continue;
    }
    // git 的 "无末尾换行" 标记行：不占用行号
    if (line.startsWith("\\ No newline")) {
      out.push({ gutter: "", kindClass: "tool-review-patch-line-context" });
      continue;
    }

    if (line.startsWith("-")) {
      out.push({
        gutter: formatPatchGutter(oldLineNumber),
        kindClass: "tool-review-patch-line-delete",
      });
      if (oldLineNumber != null) oldLineNumber += 1;
      continue;
    }

    if (line.startsWith("+")) {
      out.push({
        gutter: formatPatchGutter(newLineNumber),
        kindClass: "tool-review-patch-line-add",
      });
      if (newLineNumber != null) newLineNumber += 1;
      continue;
    }

    // 上下文行
    out.push({
      gutter: formatPatchGutter(newLineNumber ?? oldLineNumber),
      kindClass: "tool-review-patch-line-context",
    });
    if (oldLineNumber != null) oldLineNumber += 1;
    if (newLineNumber != null) newLineNumber += 1;
  }
  // 文件尾部：从末尾反向统计连续上下文行数，达到 -U 值说明其后仍有省略
  if (sawHeader) {
    let tailStreak = 0;
    for (let i = out.length - 1; i >= 0; i -= 1) {
      if (out[i].kindClass !== "tool-review-patch-line-context") break;
      tailStreak += 1;
    }
    if (tailStreak >= contextLines) {
      gaps.push({ startLine: lines.length, kind: "tail" });
    }
  }
  return { meta: out, gaps };
}

function formatPatchGutter(lineNumber: number | null) {
  return lineNumber == null ? "" : String(lineNumber);
}

function escapeHtmlAttribute(value: string) {
  return String(value || "")
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

watch(
  () => [props.code, props.mode, props.lang, props.isDark, props.showLineNumbers, props.contextLines] as const,
  () => {
    void updateHighlightedCode();
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  if (highlightAbort) {
    highlightAbort.abort();
    highlightAbort = null;
  }
});
</script>

<style scoped>
.tool-review-code-main {
  --ap-add-bg: #e6ffec;
  --ap-add-fg: #1a7f37;
  --ap-remove-bg: #ffebe9;
  --ap-remove-fg: #cf222e;
  background: var(--color-base-100);
}
.tool-review-code-main--dark {
  --ap-add-bg: rgba(46, 160, 67, 0.15);
  --ap-add-fg: #3fb950;
  --ap-remove-bg: rgba(248, 81, 73, 0.15);
  --ap-remove-fg: #f85149;
}

.tool-review-code-scroller {
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.tool-review-code-scroller::-webkit-scrollbar {
  width: 0;
  height: 0;
}

.tool-review-code-main-embedded {
  overflow: visible;
}
.tool-review-code-main-embedded .tool-review-code-view code {
  min-width: max-content;
}

.tool-review-code-view {
  height: 100%;
  min-height: 100%;
  font-family: var(--app-code-font-family);
  background: var(--color-base-100);
}

.tool-review-raw-pre {
  min-height: 100%;
  margin: 0;
  padding: 0.75rem 1rem;
  white-space: pre;
  color: var(--color-base-content);
  background: var(--color-base-100);
  font-family: var(--app-code-font-family);
  font-size: var(--app-text-xs-size);
  line-height: 1.5;
}

:deep(.tool-review-code-view .shiki) {
  font-family: var(--app-code-font-family) !important;
  min-height: 100%;
  margin: 0;
  padding: 0.75rem 0;
  border: 0;
  border-radius: 0;
  background: var(--color-base-100) !important;
  box-shadow: none;
  overflow: visible;
}

:deep(.tool-review-code-view code) {
  display: block;
  min-width: max-content;
}

:deep(.tool-review-code-view .line) {
  display: block;
  min-height: 1.5em;
  line-height: 1.5;
}

:deep(.tool-review-code-view .line::before) {
  display: inline-block;
  width: 2.75rem;
  padding: 0 0.75rem 0 0.5rem;
  text-align: right;
  color: #64748b;
  user-select: none;
  white-space: pre;
}

.tool-review-code-main-with-lines :deep(.tool-review-code-view .line::before) {
  content: attr(data-gutter);
}

.tool-review-code-main:not(.tool-review-code-main-with-lines) :deep(.tool-review-code-view .line::before) {
  content: "";
  width: 0;
  padding: 0;
}

.tool-review-code-main-with-lines :deep(.tool-review-code-view .line.tool-review-patch-line-delete) {
  background: var(--ap-remove-bg, rgba(248, 81, 73, 0.08));
}

.tool-review-code-main-with-lines :deep(.tool-review-code-view .line.tool-review-patch-line-add) {
  background: var(--ap-add-bg, rgba(46, 160, 67, 0.08));
}

.tool-review-code-main-with-lines :deep(.tool-review-code-view .line.tool-review-patch-line-header) {
  display: none;
}

:deep(.tool-review-code-view .line.tool-review-gap-bar) {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: 0.5rem;
  padding: 0.3rem 0.75rem;
  margin: 0;
  cursor: pointer;
  color: var(--color-base-content);
  opacity: 0.72;
  font-size: 0.75rem;
  background: var(--color-base-200);
  border-top: 1px solid var(--color-base-300);
  border-bottom: 1px solid var(--color-base-300);
  user-select: none;
}

:deep(.tool-review-code-view .line.tool-review-gap-bar:hover) {
  opacity: 1;
  color: var(--color-primary);
  background: var(--color-base-200);
}

:deep(.tool-review-code-view .line.tool-review-gap-bar .line-code) {
  display: inline-flex;
  align-items: center;
  gap: 0.4rem;
}

:deep(.tool-review-code-view .line.tool-review-gap-bar .tool-review-gap-bar-icon) {
  display: inline-flex;
  flex-shrink: 0;
  opacity: 0.9;
}

:deep(.tool-review-code-view .line.tool-review-gap-bar::before) {
  content: "";
  width: 0;
  padding: 0;
}

.tool-review-code-main-with-lines :deep(.tool-review-code-view .line.tool-review-patch-line-meta) {
  display: none;
}
</style>

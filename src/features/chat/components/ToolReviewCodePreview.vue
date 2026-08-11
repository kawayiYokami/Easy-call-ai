<template>
  <div class="flex h-full min-h-0 w-full flex-col gap-2">
    <div v-if="title" class="px-4 pt-3 text-xs text-base-content/65">{{ title }}</div>
    <div
      class="tool-review-code-main relative min-h-0 flex-1 overflow-hidden"
      :class="{
        'tool-review-code-main-with-lines': showGutter,
        'tool-review-code-main-with-patch-lines': isPatchMode,
      }"
      @mouseenter="scrollbarRef?.reveal()"
      @mouseleave="scrollbarRef?.hide()"
    >
      <div ref="scrollerRef" class="tool-review-code-scroller h-full overflow-auto">
        <pre v-if="!highlightedHtml" class="tool-review-raw-pre">{{ code }}</pre>
        <div v-else class="tool-review-code-view" v-html="highlightedHtml"></div>
      </div>
      <FloatingScrollbar ref="scrollbarRef" :target="scrollerRef" variant="code-dark" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { bundledLanguagesInfo, codeToHtml } from "shiki";
import FloatingScrollbar from "../../shell/components/FloatingScrollbar.vue";

const props = defineProps<{
  title?: string;
  code: string;
  lang?: string;
  mode?: "plain" | "patch";
  isDark?: boolean;
  showLineNumbers?: boolean;
}>();

const scrollerRef = ref<HTMLElement | null>(null);
const scrollbarRef = ref<InstanceType<typeof FloatingScrollbar> | null>(null);
const highlightedHtml = ref("");
let highlightAbort: AbortController | null = null;

const SHIKI_LANGUAGE_KEYS = new Set(
  bundledLanguagesInfo.flatMap((item) => [item.id, ...(item.aliases || [])]).map((item) => item.toLowerCase()),
);

const isPatchMode = computed(() => props.mode === "patch");
const showGutter = computed(() => isPatchMode.value || props.showLineNumbers === true);

async function updateHighlightedCode() {
  const code = String(props.code || "");
  if (!code.trim()) {
    highlightedHtml.value = "";
    return;
  }
  if (highlightAbort) highlightAbort.abort();
  highlightAbort = new AbortController();
  const signal = highlightAbort.signal;
  const language = resolveLanguage();
  try {
    const html = await codeToHtml(code, {
      lang: language,
      theme: props.isDark ? "github-dark" : "github-light",
    });
    if (signal.aborted) return;
    highlightedHtml.value = normalizeShikiLineHtml(html, code, props.mode);
  } catch {
    if (signal.aborted) return;
    highlightedHtml.value = "";
  }
}

function resolveLanguage() {
  const requested = String(props.lang || "").trim().toLowerCase();
  const key = props.mode === "patch" ? "diff" : requested || "text";
  return SHIKI_LANGUAGE_KEYS.has(key) ? key : "text";
}

function normalizeShikiLineHtml(html: string, code: string, mode?: "plain" | "patch") {
  const compactHtml = html.replace(/<\/span>\s+<span class="line"/g, '</span><span class="line"');
  if (mode !== "patch" && props.showLineNumbers !== true) return compactHtml;
  const lineMeta = mode === "patch" ? buildPatchLineMeta(code) : buildPlainLineMeta(code);
  let lineIndex = 0;
  return compactHtml.replace(/<span class="line"/g, () => {
    const meta = lineMeta[lineIndex] || { gutter: "", kindClass: "" };
    lineIndex += 1;
    return `<span class="line ${meta.kindClass}" data-gutter="${escapeHtmlAttribute(meta.gutter)}"`;
  });
}

function buildPlainLineMeta(code: string) {
  const lineCount = String(code || "").split("\n").length;
  return Array.from({ length: lineCount }, (_, index) => ({
    gutter: String(index + 1),
    kindClass: "tool-review-plain-line",
  }));
}

function buildPatchLineMeta(code: string) {
  const lines = String(code || "").split("\n");
  const out = [] as Array<{ gutter: string; kindClass: string }>;
  let oldLineNumber: number | null = null;
  let newLineNumber: number | null = null;
  for (const line of lines) {
    // git diff 真实格式：@@ -oldStart,oldCount +newStart,newCount @@（count 为 1 时可省略）
    const gitHeaderMatch = line.match(/^@@\s+-(\d+)(?:,\d+)?\s+\+(\d+)(?:,\d+)?\s+@@/);
    if (gitHeaderMatch) {
      oldLineNumber = Number(gitHeaderMatch[1]);
      newLineNumber = Number(gitHeaderMatch[2]);
      out.push({ gutter: "", kindClass: "tool-review-patch-line-header" });
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
    // git 的 "无末尾换行" 标记行：不占用行号
    if (line.startsWith("\\ No newline")) {
      out.push({ gutter: "", kindClass: "tool-review-patch-line-context" });
      continue;
    }

    if (line.startsWith("-")) {
      out.push({
        gutter: formatPatchGutter(oldLineNumber, null),
        kindClass: "tool-review-patch-line-delete",
      });
      if (oldLineNumber != null) oldLineNumber += 1;
      continue;
    }

    if (line.startsWith("+")) {
      out.push({
        gutter: formatPatchGutter(null, newLineNumber),
        kindClass: "tool-review-patch-line-add",
      });
      if (newLineNumber != null) newLineNumber += 1;
      continue;
    }

    out.push({
      gutter: formatPatchGutter(oldLineNumber, newLineNumber),
      kindClass: "tool-review-patch-line-context",
    });
    if (oldLineNumber != null) oldLineNumber += 1;
    if (newLineNumber != null) newLineNumber += 1;
  }
  return out;
}

function formatPatchGutter(oldLineNumber: number | null, newLineNumber: number | null) {
  const left = oldLineNumber == null ? "" : String(oldLineNumber);
  const right = newLineNumber == null ? "" : String(newLineNumber);
  return `${left.padStart(4, " ")} ${right.padStart(4, " ")}`;
}

function escapeHtmlAttribute(value: string) {
  return String(value || "")
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

watch(
  () => [props.code, props.mode, props.lang, props.isDark, props.showLineNumbers] as const,
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
  background: var(--color-base-200);
}

.tool-review-code-scroller {
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.tool-review-code-scroller::-webkit-scrollbar {
  width: 0;
  height: 0;
}

.tool-review-code-view {
  height: 100%;
  min-height: 100%;
  font-family: var(--app-code-font-family);
  background: var(--color-base-200);
}

.tool-review-raw-pre {
  min-height: 100%;
  margin: 0;
  padding: 0.75rem 1rem;
  white-space: pre;
  color: var(--color-base-content);
  background: var(--color-base-200);
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
  background: var(--color-base-200) !important;
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
  width: 5.75rem;
  padding: 0 0.75rem 0 0.5rem;
  text-align: right;
  color: #64748b;
  user-select: none;
  white-space: pre;
}

.tool-review-code-main-with-lines:not(.tool-review-code-main-with-patch-lines) :deep(.tool-review-code-view .line::before) {
  width: 2.75rem;
  padding: 0 0.75rem 0 0.5rem;
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
  background: rgba(239, 68, 68, 0.08);
}

.tool-review-code-main-with-lines :deep(.tool-review-code-view .line.tool-review-patch-line-add) {
  background: rgba(34, 197, 94, 0.08);
}

.tool-review-code-main-with-lines :deep(.tool-review-code-view .line.tool-review-patch-line-header) {
  color: #c084fc;
}
</style>

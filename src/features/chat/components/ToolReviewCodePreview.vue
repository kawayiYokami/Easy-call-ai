<template>
  <div class="flex h-full min-h-0 w-full flex-col gap-2">
    <div v-if="title" class="px-4 pt-3 text-xs text-base-content/65">{{ title }}</div>
    <div
      class="tool-review-code-main min-h-0 flex-1 overflow-auto"
      :class="{ 'tool-review-code-main-with-lines': shouldShowLineNumbers }"
    >
      <pre v-if="!highlightedHtml" class="tool-review-raw-pre">{{ code }}</pre>
      <div v-else class="tool-review-code-view" v-html="highlightedHtml"></div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { bundledLanguagesInfo, codeToHtml } from "shiki";

const props = defineProps<{
  title?: string;
  code: string;
  mode?: "plain" | "patch";
  isDark?: boolean;
}>();

const highlightedHtml = ref("");
let highlightAbort: AbortController | null = null;

const SHIKI_LANGUAGE_KEYS = new Set(
  bundledLanguagesInfo.flatMap((item) => [item.id, ...(item.aliases || [])]).map((item) => item.toLowerCase()),
);

const shouldShowLineNumbers = computed(() =>
  props.mode === "patch" && /^@@\s+line\b|^@@\s+lines\b/m.test(String(props.code || ""))
);

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
      theme: "github-dark",
    });
    if (signal.aborted) return;
    highlightedHtml.value = normalizeShikiLineHtml(html);
  } catch {
    if (signal.aborted) return;
    highlightedHtml.value = "";
  }
}

function resolveLanguage() {
  const key = props.mode === "patch" ? "diff" : "text";
  return SHIKI_LANGUAGE_KEYS.has(key) ? key : "text";
}

function normalizeShikiLineHtml(html: string) {
  return html.replace(/<\/span>\s+<span class="line"/g, '</span><span class="line"');
}

watch(
  () => [props.code, props.mode] as const,
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
  background: #101828;
}

.tool-review-code-view {
  height: 100%;
  min-height: 100%;
  background: #101828;
}

.tool-review-raw-pre {
  min-height: 100%;
  margin: 0;
  padding: 0.75rem 1rem;
  white-space: pre;
  color: #e5e7eb;
  background: #101828;
  font-size: 12px;
  line-height: 1.5;
}

:deep(.tool-review-code-view .shiki) {
  min-height: 100%;
  margin: 0;
  padding: 0.75rem 0;
  border: 0;
  border-radius: 0;
  background: #101828 !important;
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
  padding-right: 0.75rem;
  text-align: right;
  color: #64748b;
  user-select: none;
}

.tool-review-code-main-with-lines :deep(.tool-review-code-view .shiki) {
  counter-reset: tool-review-code-line;
}

.tool-review-code-main-with-lines :deep(.tool-review-code-view .line) {
  counter-increment: tool-review-code-line;
}

.tool-review-code-main-with-lines :deep(.tool-review-code-view .line::before) {
  content: counter(tool-review-code-line);
}

.tool-review-code-main:not(.tool-review-code-main-with-lines) :deep(.tool-review-code-view .line::before) {
  content: "";
  width: 0;
  padding-right: 0;
}
</style>

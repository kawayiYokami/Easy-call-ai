import { computed, defineComponent, h, onBeforeUnmount, ref, watch } from "vue";
import { Check, Copy, Maximize2 } from "@lucide/vue";
import CodeBlockPreviewDialog from "../components/dialogs/CodeBlockPreviewDialog.vue";
import MermaidBlock from "./MermaidBlock";

const HIGHLIGHT_CACHE_MAX = 80;
const highlightCache = new Map<string, string>();
function highlightCacheKey(code: string, lang: string, isDark: boolean): string {
  return `${lang || "text"}::${isDark ? "dark" : "light"}::${code}`;
}

const CodeBlock = defineComponent({
  name: "CodeBlock",
  props: {
    lang: { type: String, default: "" },
    code: { type: String, default: "" },
    blockKey: { type: String, default: "" },
    isDark: { type: Boolean, default: false },
    streaming: { type: Boolean, default: false },
    copyText: { type: String, default: "Copy" },
    copiedText: { type: String, default: "Copied" },
    expandText: { type: String, default: "Expand" },
    preparingText: { type: String, default: "" },
  },
  setup(codeProps) {
    const highlightedHtml = ref("");
    const copied = ref(false);
    const previewOpen = ref(false);
    let copyTimer = 0;
    let highlightAbort: AbortController | null = null;
    let highlightDebounceTimer = 0;

    const isMermaid = computed(() => codeProps.lang === "mermaid");

    async function highlight() {
      if (isMermaid.value) return;
      if (!codeProps.code) {
        highlightedHtml.value = "";
        return;
      }
      const key = highlightCacheKey(codeProps.code, codeProps.lang, codeProps.isDark);
      const cached = highlightCache.get(key);
      if (cached !== undefined) {
        if (highlightAbort) {
          highlightAbort.abort();
          highlightAbort = null;
        }
        highlightedHtml.value = cached;
        return;
      }
      if (highlightAbort) highlightAbort.abort();
      highlightAbort = new AbortController();
      const signal = highlightAbort.signal;

      try {
        const { codeToHtml } = await import("shiki");
        if (signal.aborted) return;
        const html = await codeToHtml(codeProps.code, {
          lang: codeProps.lang || "text",
          theme: codeProps.isDark ? "github-dark" : "github-light",
        });
        if (signal.aborted) return;
        if (highlightCache.size >= HIGHLIGHT_CACHE_MAX) {
          const first = highlightCache.keys().next().value as string | undefined;
          if (first !== undefined) highlightCache.delete(first);
        }
        highlightCache.set(key, html);
        highlightedHtml.value = html;
      } catch {
        if (!signal.aborted) highlightedHtml.value = "";
      }
    }

    function scheduleHighlight() {
      if (isMermaid.value) return;
      if (!codeProps.code) {
        if (highlightDebounceTimer) {
          clearTimeout(highlightDebounceTimer);
          highlightDebounceTimer = 0;
        }
        if (highlightAbort) {
          highlightAbort.abort();
          highlightAbort = null;
        }
        highlightedHtml.value = "";
        return;
      }
      const key = highlightCacheKey(codeProps.code, codeProps.lang, codeProps.isDark);
      const cached = highlightCache.get(key);
      if (cached !== undefined) {
        if (highlightDebounceTimer) {
          clearTimeout(highlightDebounceTimer);
          highlightDebounceTimer = 0;
        }
        if (highlightAbort) {
          highlightAbort.abort();
          highlightAbort = null;
        }
        highlightedHtml.value = cached;
        return;
      }
      // 流式期间节流：大块代码在流式期间只保留纯文本，结束后再高亮，避免每 8ms 调一次 shiki
      if (codeProps.streaming) {
        if (codeProps.code.length > 4000) return;
        if (highlightDebounceTimer) clearTimeout(highlightDebounceTimer);
        highlightDebounceTimer = window.setTimeout(() => {
          highlightDebounceTimer = 0;
          void highlight();
        }, 160);
        return;
      }
      if (highlightDebounceTimer) {
        clearTimeout(highlightDebounceTimer);
        highlightDebounceTimer = 0;
      }
      void highlight();
    }

    watch(
      () => [codeProps.code, codeProps.lang, codeProps.isDark, codeProps.streaming],
      () => {
        scheduleHighlight();
      },
      { immediate: true },
    );

    async function copyCode() {
      try {
        await navigator.clipboard.writeText(codeProps.code || "");
        copied.value = true;
        if (copyTimer) clearTimeout(copyTimer);
        copyTimer = window.setTimeout(() => {
          copied.value = false;
          copyTimer = 0;
        }, 1500);
      } catch {
        copied.value = false;
      }
    }

    function openPreview() {
      previewOpen.value = true;
    }

    function closePreview() {
      previewOpen.value = false;
    }

    onBeforeUnmount(() => {
      if (copyTimer) {
        clearTimeout(copyTimer);
        copyTimer = 0;
      }
      if (highlightDebounceTimer) {
        clearTimeout(highlightDebounceTimer);
        highlightDebounceTimer = 0;
      }
      if (highlightAbort) {
        highlightAbort.abort();
        highlightAbort = null;
      }
    });

    return () => {
      if (isMermaid.value) {
        return h(MermaidBlock, {
          code: codeProps.code,
          blockKey: codeProps.blockKey,
          isDark: codeProps.isDark,
          streaming: codeProps.streaming,
          copyText: codeProps.copyText,
          copiedText: codeProps.copiedText,
          preparingText: codeProps.preparingText,
        });
      }

      // 标题栏：左边语言名，右边复制按钮
      const titleBar = h("div", { class: "ecall-md-code-title" }, [
        h("span", { class: "ecall-md-code-lang" }, codeProps.lang || "text"),
        h("div", { class: "ecall-md-code-actions" }, [
          h("button", {
            type: "button",
            class: "ecall-md-code-action",
            title: codeProps.expandText,
            onClick: openPreview,
          }, [h(Maximize2, { class: "ecall-md-code-action-icon" })]),
          h("button", {
            type: "button",
            class: "ecall-md-code-action ecall-md-code-copy",
            title: copied.value ? codeProps.copiedText : codeProps.copyText,
            "aria-label": copied.value ? codeProps.copiedText : codeProps.copyText,
            onClick: copyCode,
          }, [h(copied.value ? Check : Copy, { class: "ecall-md-code-action-icon" })]),
        ]),
      ]);

      // 代码区
      const codeArea = highlightedHtml.value
        ? h("div", { class: "ecall-md-code-body", innerHTML: highlightedHtml.value })
        : h("pre", { class: "ecall-md-code-body ecall-md-code-plain" }, [h("code", null, codeProps.code)]);

      // 圆角外壳
      return h("div", { class: "ecall-md-code-block" }, [
        titleBar,
        codeArea,
        h(CodeBlockPreviewDialog, {
          open: previewOpen.value,
          lang: codeProps.lang,
          code: codeProps.code,
          isDark: codeProps.isDark,
          onClose: closePreview,
        }),
      ]);
    };
  },
});

export default CodeBlock;


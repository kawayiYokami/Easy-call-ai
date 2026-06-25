<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { Copy, X } from "@lucide/vue";

const props = defineProps<{
  open: boolean;
  lang?: string;
  code: string;
  isDark?: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();

const { t } = useI18n();
const copied = ref(false);
const highlightedHtml = ref("");
let copiedTimer = 0;
let highlightAbort: AbortController | null = null;

const languageLabel = computed(() => String(props.lang || "").trim() || "text");
const previewIsDark = computed(() => {
  if (props.isDark !== undefined) return props.isDark;
  if (typeof window === "undefined" || typeof document === "undefined") return false;
  return String(window.getComputedStyle(document.documentElement).colorScheme || "").toLowerCase().includes("dark");
});

async function highlightCode() {
  if (!props.open) return;
  const code = String(props.code || "");
  if (!code) {
    highlightedHtml.value = "";
    return;
  }
  if (highlightAbort) highlightAbort.abort();
  highlightAbort = new AbortController();
  const signal = highlightAbort.signal;

  try {
    const { codeToHtml } = await import("shiki");
    if (signal.aborted) return;
    const html = await codeToHtml(code, {
      lang: String(props.lang || "").trim() || "text",
      theme: previewIsDark.value ? "github-dark" : "github-light",
    });
    if (signal.aborted) return;
    highlightedHtml.value = html;
  } catch {
    highlightedHtml.value = "";
  }
}

async function copyCode() {
  try {
    await navigator.clipboard.writeText(String(props.code || ""));
    copied.value = true;
    if (copiedTimer) window.clearTimeout(copiedTimer);
    copiedTimer = window.setTimeout(() => {
      copied.value = false;
      copiedTimer = 0;
    }, 1200);
  } catch {
    copied.value = false;
  }
}

function requestClose() {
  emit("close");
}

watch(
  () => [props.open, props.code, props.lang, previewIsDark.value],
  () => {
    void highlightCode();
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  if (copiedTimer) window.clearTimeout(copiedTimer);
  if (highlightAbort) {
    highlightAbort.abort();
    highlightAbort = null;
  }
});
</script>

<template>
  <Teleport to="body">
    <dialog v-if="open" class="modal modal-open">
      <div class="modal-box flex h-[95vh] max-h-[95vh] w-[95vw] max-w-none flex-col overflow-hidden p-0">
        <div class="flex items-center gap-2 border-b border-base-300 px-4 py-2">
          <div class="min-w-0 flex-1 truncate text-xs font-semibold">{{ languageLabel }}</div>
          <button
            type="button"
            class="btn btn-ghost btn-xs gap-1"
            :title="copied ? '已复制' : t('common.copy')"
            @click="copyCode"
          >
            <Copy class="h-3.5 w-3.5" />
            <span>{{ copied ? "已复制" : t("common.copy") }}</span>
          </button>
          <button
            type="button"
            class="btn btn-ghost btn-square btn-xs"
            :title="t('common.close')"
            @click="requestClose"
          >
            <X class="h-3.5 w-3.5" />
          </button>
        </div>
        <div class="ecall-code-preview-body flex-1 min-h-0" :class="previewIsDark ? 'ecall-code-preview-dark' : 'ecall-code-preview-light'">
          <div
            v-if="highlightedHtml"
            v-html="highlightedHtml"
          ></div>
          <pre v-else class="ecall-code-preview-plain"><code>{{ code }}</code></pre>
        </div>
      </div>
      <form method="dialog" class="modal-backdrop">
        <button @click.prevent="requestClose">close</button>
      </form>
    </dialog>
  </Teleport>
</template>

<style scoped>
.ecall-code-preview-body {
  height: 100%;
  min-height: 0;
  overflow: auto;
  padding: 0.9rem 1rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  font-size: 0.88rem;
  line-height: 1.6;
  margin: 0;
  white-space: normal;
}

.ecall-code-preview-plain {
  background: transparent;
  border: 0;
  margin: 0;
  white-space: pre;
}

.ecall-code-preview-plain code {
  background: transparent;
  border: 0;
  padding: 0;
  font: inherit;
  color: inherit;
}

.ecall-code-preview-body :deep(pre),
.ecall-code-preview-body :deep(pre.shiki),
.ecall-code-preview-body :deep(.shiki) {
  margin: 0 !important;
  padding: 0 !important;
  border-radius: 0 !important;
  border: 0 !important;
  overflow: visible !important;
  background: transparent !important;
}

.ecall-code-preview-body :deep(pre code) {
  background: transparent !important;
  border: 0 !important;
  padding: 0 !important;
  box-shadow: none !important;
  font: inherit;
}

.ecall-code-preview-body :deep(.line),
.ecall-code-preview-body :deep(.shiki span) {
  background: transparent !important;
  box-shadow: none !important;
  text-shadow: none !important;
}

.ecall-code-preview-dark {
  background: #101828;
  color: #e5e7eb;
}

.ecall-code-preview-light {
  background: #f6f8fa;
  color: #24292f;
}
</style>

<template>
  <div
    ref="rendererRootRef"
    v-bind="attrs"
    class="ecall-md-renderer"
    :class="[isDark ? 'ecall-md-dark' : 'ecall-md-light', variant === 'document' ? 'ecall-md-document' : 'ecall-md-chat']"
    @click="emit('click', $event)"
  >
    <template v-for="(block, index) in visibleBlocks" :key="`${block.type}-${index}-${block.key}`">
      <component
        :is="headingTag(block.level)"
        v-if="block.type === 'heading'"
        class="ecall-md-heading"
      >
        <InlineRenderer :segments="parseInlineSegments(block.text)" :local-image-base-path="localImageBasePath" />
      </component>

      <blockquote v-else-if="block.type === 'quote'" class="ecall-md-quote">
        <InlineRenderer :segments="parseInlineSegments(block.text)" :local-image-base-path="localImageBasePath" />
      </blockquote>

      <ul v-else-if="block.type === 'list' && !block.ordered" class="ecall-md-list">
        <li v-for="(item, itemIndex) in block.items" :key="`${index}-${itemIndex}`">
          <InlineRenderer :segments="parseInlineSegments(item)" :local-image-base-path="localImageBasePath" />
        </li>
      </ul>
      <ol v-else-if="block.type === 'list'" class="ecall-md-list ecall-md-list-ordered">
        <li v-for="(item, itemIndex) in block.items" :key="`${index}-${itemIndex}`">
          <InlineRenderer :segments="parseInlineSegments(item)" :local-image-base-path="localImageBasePath" />
        </li>
      </ol>

      <div v-else-if="block.type === 'table'" class="ecall-md-table-wrap">
        <table class="ecall-md-table">
          <thead>
            <tr>
              <th v-for="(cell, ci) in block.headers" :key="`${index}-h-${ci}`">
                <InlineRenderer :segments="parseInlineSegments(cell)" :local-image-base-path="localImageBasePath" />
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(row, ri) in block.rows" :key="`${index}-r-${ri}`">
              <td v-for="(cell, ci) in normalizedTableRow(row, block.headers.length)" :key="`${index}-r-${ri}-c-${ci}`">
                <InlineRenderer :segments="parseInlineSegments(cell)" :local-image-base-path="localImageBasePath" />
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <CodeBlock
        v-else-if="block.type === 'code'"
        :lang="block.lang"
        :code="block.text"
        :block-key="block.key"
        :is-dark="isDark"
        :streaming="streaming"
      />

      <MathBlock
        v-else-if="block.type === 'math'"
        :text="block.text"
        :block-key="block.key"
        :streaming="streaming"
      />

      <hr v-else-if="block.type === 'hr'" class="ecall-md-hr" />

      <p v-else class="ecall-md-paragraph">
        <InlineRenderer :segments="parseInlineSegments(block.text)" :local-image-base-path="localImageBasePath" />
      </p>
    </template>
  </div>
  <Teleport to="body">
    <div
      v-if="activeToolcallPreviews.length > 0"
      ref="toolcallPopupRef"
      class="ecall-md-toolcall-popup fixed z-1200 w-max min-w-[18rem] max-w-[calc(100vw-0.75rem)] rounded-box border border-base-300 bg-base-100 text-base-content shadow-xl"
      :style="toolcallPopupStyle"
      data-toolcall-popup="true"
    >
      <div class="border-b border-base-300/70 px-2 py-1.5 text-[11px] font-semibold text-base-content/80">
        {{ activeToolcallPopupTitle }}
      </div>
      <div
        class="relative overflow-hidden"
        @mouseenter="toolcallScrollbarRef?.reveal()"
        @mouseleave="toolcallScrollbarRef?.hide()"
      >
        <div
          ref="toolcallScrollerRef"
          class="ecall-md-toolcall-scroll max-h-80 overflow-y-auto"
        >
          <div v-if="activeToolcallPreviews.length === 1 && activeToolcallPreviews[0]?.body" class="px-2 py-1.5">
            <pre class="m-0 whitespace-pre-wrap break-all text-xs leading-relaxed text-base-content/75"><code>{{ activeToolcallPreviews[0].body }}</code></pre>
          </div>
          <div v-else-if="activeToolcallPreviews.length > 1" class="py-1">
            <div
              v-for="(preview, index) in activeToolcallPreviews"
              :key="preview.id"
              class="px-2 py-1"
              :class="index > 0 ? 'border-t border-base-300/60' : ''"
            >
              <div class="grid grid-cols-[1.1rem_minmax(0,1fr)] items-start gap-x-1.5 text-xs leading-relaxed text-base-content/75">
                <span class="mt-0.5 inline-flex h-4 w-4 shrink-0 items-center justify-center rounded-full bg-base-200 text-[10px] font-medium leading-none text-base-content/65">
                  {{ index + 1 }}
                </span>
                <span class="min-w-0 whitespace-normal break-all font-normal leading-relaxed">{{ preview.title || preview.label }}</span>
              </div>
              <pre
                v-if="preview.body"
                class="ml-[1.6rem] mt-1 m-0 whitespace-pre-wrap break-all rounded bg-base-200/50 p-1 text-[11px] leading-4 text-base-content/75"
              ><code>{{ preview.body }}</code></pre>
            </div>
          </div>
        </div>
        <FloatingScrollbar ref="toolcallScrollbarRef" :target="toolcallScrollerRef" />
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { Teleport, computed, defineComponent, h, nextTick, onBeforeUnmount, onMounted, ref, useAttrs, watch, type PropType, type VNodeChild } from "vue";
import { useI18n } from "vue-i18n";
import { Check, Copy, Maximize2, Wrench } from "@lucide/vue";
import { invokeTauri } from "../../../services/tauri-api";
import FloatingScrollbar from "../../shell/components/FloatingScrollbar.vue";
import { isAbsoluteLocalPath, normalizeLocalLinkHref } from "../utils/local-link";
import { parseMarkdownBlocks, parseInlineSegments, normalizedTableRow, type MarkdownBlock, type InlineSegment } from "./parse-markdown";
import CodeBlockPreviewDialog from "../components/dialogs/CodeBlockPreviewDialog.vue";

defineOptions({
  inheritAttrs: false,
});

const props = defineProps<{
  text: string;
  isDark?: boolean;
  streaming?: boolean;
  variant?: "chat" | "document";
  localImageBasePath?: string;
  toolcallPreviewMap?: Record<string, { title?: string; body?: string }>;
}>();
const emit = defineEmits<{
  (e: "click", event: MouseEvent): void;
}>();
const attrs = useAttrs();

const { t } = useI18n();
const rendererRootRef = ref<HTMLElement | null>(null);
const activeToolcallIds = ref<string[]>([]);
const activeToolcallAnchorEl = ref<HTMLButtonElement | null>(null);
const toolcallPopupRef = ref<HTMLElement | null>(null);
const toolcallScrollerRef = ref<HTMLElement | null>(null);
const toolcallScrollbarRef = ref<InstanceType<typeof FloatingScrollbar> | null>(null);
const toolcallPopupStyle = ref<Record<string, string>>({
  left: "0px",
  top: "0px",
});

const activeToolcallPreviews = computed(() => {
  return activeToolcallIds.value
    .map((rawId) => String(rawId || "").trim())
    .filter(Boolean)
    .map((id) => {
      const preview = props.toolcallPreviewMap?.[id];
      if (!preview) return null;
      return {
        id,
        label: `toolcall:${id}`,
        title: String(preview.title || "").trim(),
        body: String(preview.body || "").trim(),
      };
    })
    .filter((preview): preview is { id: string; label: string; title: string; body: string } => !!preview);
});

const activeToolcallPopupTitle = computed(() => {
  const count = activeToolcallPreviews.value.length;
  if (count <= 1) {
    return activeToolcallPreviews.value[0]?.title
      || activeToolcallPreviews.value[0]?.label
      || t("chat.shareExport.toolLabel");
  }
  return `${t("chat.shareExport.toolLabel")} +${count}`;
});

function closeToolcallPreview() {
  activeToolcallIds.value = [];
  activeToolcallAnchorEl.value = null;
}

async function positionToolcallPopup() {
  const anchor = activeToolcallAnchorEl.value;
  const popup = toolcallPopupRef.value;
  if (!(anchor instanceof HTMLElement) || !(popup instanceof HTMLElement)) return;
  const margin = 8;
  const anchorRect = anchor.getBoundingClientRect();
  const popupRect = popup.getBoundingClientRect();
  const viewportWidth = window.innerWidth || document.documentElement.clientWidth || 0;
  const viewportHeight = window.innerHeight || document.documentElement.clientHeight || 0;
  const left = Math.min(
    Math.max(margin, anchorRect.left),
    Math.max(margin, viewportWidth - popupRect.width - margin),
  );
  const belowTop = anchorRect.bottom + 8;
  const aboveTop = anchorRect.top - popupRect.height - 8;
  const top = belowTop + popupRect.height + margin <= viewportHeight
    ? belowTop
    : Math.max(margin, aboveTop);
  toolcallPopupStyle.value = {
    left: `${Math.round(left)}px`,
    top: `${Math.round(top)}px`,
  };
}

async function openToolcallPreview(ids: string[], anchorEl: HTMLButtonElement | null) {
  const normalizedIds = ids
    .map((id) => String(id || "").trim())
    .filter(Boolean);
  const availableIds = normalizedIds.filter((id) => {
    const preview = props.toolcallPreviewMap?.[id];
    return !!preview && (!!String(preview.title || "").trim() || !!String(preview.body || "").trim());
  });
  if (availableIds.length === 0) return;
  activeToolcallIds.value = availableIds;
  activeToolcallAnchorEl.value = anchorEl;
  await nextTick();
  toolcallScrollbarRef.value?.updateThumb();
  await positionToolcallPopup();
}

function sameToolcallGroup(left: string[], right: string[]): boolean {
  if (left.length !== right.length) return false;
  return left.every((id, index) => id === right[index]);
}

function toggleToolcallPreview(ids: string[], anchorEl: HTMLButtonElement | null) {
  const normalizedIds = ids
    .map((id) => String(id || "").trim())
    .filter(Boolean);
  if (normalizedIds.length === 0) return;
  if (sameToolcallGroup(activeToolcallIds.value, normalizedIds)) {
    closeToolcallPreview();
    return;
  }
  void openToolcallPreview(normalizedIds, anchorEl);
}

function handleDocumentPointerDown(event: PointerEvent) {
  if (activeToolcallIds.value.length === 0) return;
  const target = event.target;
  if (!(target instanceof Node)) {
    closeToolcallPreview();
    return;
  }
  if (target instanceof HTMLElement && target.closest('[data-toolcall-pill="true"]')) return;
  if (toolcallPopupRef.value?.contains(target)) return;
  closeToolcallPreview();
}

function handleWindowResizeOrScroll() {
  if (activeToolcallIds.value.length === 0) return;
  if (!(activeToolcallAnchorEl.value instanceof HTMLButtonElement) || !activeToolcallAnchorEl.value.isConnected) {
    closeToolcallPreview();
    return;
  }
  void nextTick(() => positionToolcallPopup());
}

// ==================== Streaming Throttle ====================

const STREAM_PARSE_THROTTLE_MS = 80;

// 实例级状态（每个组件实例独立）
const parseState = {
  lastParseTime: 0,
  cachedBlocks: [] as MarkdownBlock[],
  cachedText: "",
  batchLimit: 0,
  batchTimer: 0,
  parseRetryTimer: 0,
};

const batchRendered = ref(0);
const parseRetryTick = ref(0);

function clearParseRetryTimer() {
  if (!parseState.parseRetryTimer) return;
  clearTimeout(parseState.parseRetryTimer);
  parseState.parseRetryTimer = 0;
}

function parseAndCacheBlocks(text: string, streaming: boolean): MarkdownBlock[] {
  clearParseRetryTimer();
  parseState.lastParseTime = Date.now();
  parseState.cachedText = text;
  parseState.cachedBlocks = parseMarkdownBlocks(text, streaming);
  return parseState.cachedBlocks;
}

function scheduleStreamingParseRetry(delayMs: number) {
  if (parseState.parseRetryTimer) return;
  parseState.parseRetryTimer = window.setTimeout(() => {
    parseState.parseRetryTimer = 0;
    parseRetryTick.value += 1;
  }, Math.max(1, delayMs));
}

const allBlocks = computed<MarkdownBlock[]>(() => {
  void parseRetryTick.value;
  const text = props.text;
  if (!text) return [];

  if (!props.streaming) {
    return parseAndCacheBlocks(text, false);
  }

  // Streaming: throttle re-parses
  const now = Date.now();
  if (parseState.cachedText === text) return parseState.cachedBlocks;
  const elapsed = now - parseState.lastParseTime;
  if (elapsed < STREAM_PARSE_THROTTLE_MS && parseState.cachedBlocks.length > 0) {
    scheduleStreamingParseRetry(STREAM_PARSE_THROTTLE_MS - elapsed);
    return parseState.cachedBlocks;
  }
  return parseAndCacheBlocks(text, true);
});

// Batch rendering for streaming: reveal blocks progressively
const visibleBlocks = computed<MarkdownBlock[]>(() => {
  const blocks = allBlocks.value;
  if (!props.streaming) {
    return blocks;
  }
  if (parseState.batchLimit > 0 && parseState.batchLimit < blocks.length) {
    return blocks.slice(0, parseState.batchLimit);
  }
  return blocks;
});

// Progressive batch reveal during streaming
watch(
  () => allBlocks.value.length,
  (newLen) => {
    if (!props.streaming) {
      parseState.batchLimit = 0;
      batchRendered.value = newLen;
      return;
    }
    if (parseState.batchLimit === 0) {
      parseState.batchLimit = Math.min(newLen, 20);
      batchRendered.value = parseState.batchLimit;
    }
    scheduleBatchReveal(newLen);
  },
);

watch(
  () => props.streaming,
  (streaming) => {
    if (!streaming) {
      clearParseRetryTimer();
      parseState.batchLimit = 0;
      if (parseState.batchTimer) {
        clearTimeout(parseState.batchTimer);
        parseState.batchTimer = 0;
      }
    }
  },
);

function scheduleBatchReveal(targetLen: number) {
  if (parseState.batchTimer) return;
  if (parseState.batchLimit >= targetLen) return;
  parseState.batchTimer = window.setTimeout(() => {
    parseState.batchTimer = 0;
    parseState.batchLimit = Math.min(parseState.batchLimit + 10, allBlocks.value.length);
    batchRendered.value = parseState.batchLimit;
    if (parseState.batchLimit < allBlocks.value.length) {
      scheduleBatchReveal(allBlocks.value.length);
    }
  }, 24);
}

onBeforeUnmount(() => {
  clearParseRetryTimer();
  if (parseState.batchTimer) {
    clearTimeout(parseState.batchTimer);
    parseState.batchTimer = 0;
  }
  document.removeEventListener("pointerdown", handleDocumentPointerDown, true);
  window.removeEventListener("resize", handleWindowResizeOrScroll, true);
  document.removeEventListener("scroll", handleWindowResizeOrScroll, true);
});

onMounted(() => {
  document.addEventListener("pointerdown", handleDocumentPointerDown, true);
  window.addEventListener("resize", handleWindowResizeOrScroll, true);
  document.addEventListener("scroll", handleWindowResizeOrScroll, true);
});

// ==================== Heading Tag Helper ====================

function headingTag(level: unknown): "h1" | "h2" | "h3" | "h4" {
  const normalized = Math.min(4, Math.max(1, Number(level) || 4));
  return `h${normalized}` as "h1" | "h2" | "h3" | "h4";
}

// ==================== Inline Renderer ====================

const InlineRenderer = defineComponent({
  name: "InlineRenderer",
  props: {
    segments: {
      type: Array as PropType<InlineSegment[]>,
      required: true,
    },
    localImageBasePath: { type: String, default: "" },
  },
  setup(inlineProps) {
    return () => renderSegments(
      inlineProps.segments,
      "root",
      inlineProps.localImageBasePath,
      {
        onToolcallClick: toggleToolcallPreview,
      },
    );
  },
});

type RenderSegmentOptions = {
  onToolcallClick?: (ids: string[], anchorEl: HTMLButtonElement | null) => void;
};

type MarkdownImageSource =
  | { kind: "remote"; src: string }
  | { kind: "local"; path: string }
  | { kind: "blocked"; label: string };

const markdownImageThumbnailCache = new Map<string, string>();
const markdownImageThumbnailPromiseCache = new Map<string, Promise<string>>();

function hasUrlScheme(value: string): boolean {
  return /^[A-Za-z][A-Za-z0-9+.-]*:/.test(value);
}

function normalizeBaseLocalPath(value: string): string {
  return String(value || "").trim().replace(/\\/g, "/").replace(/\/$/, "");
}

function resolveMarkdownImageSource(rawSrc: string, basePath: string): MarkdownImageSource {
  const src = String(rawSrc || "").trim();
  if (!src) return { kind: "blocked", label: "" };
  if (/^(https?:|data:image\/)/i.test(src)) return { kind: "remote", src };
  if (/^(blob:|javascript:|mailto:)/i.test(src)) return { kind: "blocked", label: src };
  const normalized = normalizeLocalLinkHref(src);
  if (isAbsoluteLocalPath(normalized)) return { kind: "local", path: normalized };
  if (hasUrlScheme(normalized)) return { kind: "blocked", label: normalized };
  const root = normalizeBaseLocalPath(basePath);
  if (!root) return { kind: "local", path: normalized };
  return { kind: "local", path: `${root}/${normalized.replace(/^\.\//, "")}` };
}

const MarkdownImage = defineComponent({
  name: "MarkdownImage",
  props: {
    src: { type: String, required: true },
    alt: { type: String, default: "" },
    localImageBasePath: { type: String, default: "" },
  },
  setup(imageProps) {
    const thumbnailSrc = ref("");
    const loadError = ref(false);
    const source = computed(() => resolveMarkdownImageSource(imageProps.src, imageProps.localImageBasePath));

    watch(
      source,
      (next, _previous, onCleanup) => {
        thumbnailSrc.value = "";
        loadError.value = false;
        if (next.kind !== "local" || !next.path.trim()) return;
        const path = next.path.trim();
        const cached = markdownImageThumbnailCache.get(path);
        if (cached) {
          thumbnailSrc.value = cached;
          return;
        }
        let cancelled = false;
        onCleanup(() => {
          cancelled = true;
        });
        const existing = markdownImageThumbnailPromiseCache.get(path);
        const task = existing || invokeTauri<{ dataUrl: string }>("read_local_chat_image_thumbnail", {
          input: { path },
        })
          .then((result) => {
            const dataUrl = String(result?.dataUrl || "").trim();
            if (dataUrl) markdownImageThumbnailCache.set(path, dataUrl);
            markdownImageThumbnailPromiseCache.delete(path);
            return dataUrl;
          })
          .catch((error) => {
            markdownImageThumbnailPromiseCache.delete(path);
            console.warn("[Markdown图片] 本地缩略图加载失败", { path, error });
            return "";
          });
        if (!existing) markdownImageThumbnailPromiseCache.set(path, task);
        void task.then((dataUrl) => {
          if (cancelled) return;
          if (dataUrl) {
            thumbnailSrc.value = dataUrl;
          } else {
            loadError.value = true;
          }
        });
      },
      { immediate: true },
    );

    return () => {
      const current = source.value;
      const alt = String(imageProps.alt || "").trim();
      if (current.kind === "remote") {
        return h("img", {
          class: "ecall-md-image",
          src: current.src,
          alt,
          loading: "lazy",
          decoding: "async",
        });
      }
      if (current.kind === "local") {
        const title = alt || current.path;
        if (thumbnailSrc.value) {
          return h("img", {
            class: "ecall-md-image ecall-md-local-image",
            src: thumbnailSrc.value,
            alt: title,
            title,
            loading: "lazy",
            decoding: "async",
            "data-local-image-path": current.path,
          });
        }
        return h("span", {
          class: ["ecall-md-image-placeholder", loadError.value ? "ecall-md-image-error" : ""],
          title,
          "data-local-image-path": current.path,
        }, alt || current.path.split(/[\\/]/).filter(Boolean).pop() || current.path);
      }
      return h("span", { class: "ecall-md-image-placeholder ecall-md-image-error" }, alt || current.label || imageProps.src);
    };
  },
});

function renderSegments(
  segments: InlineSegment[],
  keyPrefix: string,
  localImageBasePath = "",
  options: RenderSegmentOptions = {},
): VNodeChild[] {
  function consumeGroupedToolcallRefs(startIndex: number): { ids: string[]; endIndex: number } | null {
    const startSegment = segments[startIndex];
    if (startSegment?.type !== "toolcall_ref") return null;
    const ids = [startSegment.id];
    let cursor = startIndex;
    while (cursor + 2 < segments.length) {
      const spacer = segments[cursor + 1];
      const nextTool = segments[cursor + 2];
      if (spacer?.type !== "text" || spacer.text.trim() !== "") break;
      if (nextTool?.type !== "toolcall_ref") break;
      ids.push(nextTool.id);
      cursor += 2;
    }
    return { ids, endIndex: cursor };
  }

  const nodes: VNodeChild[] = [];
  for (let index = 0; index < segments.length; index += 1) {
    const segment = segments[index];
    if (segment.type === "code") {
      nodes.push(h("code", { key: `${keyPrefix}-c-${index}`, class: "ecall-md-inline-code" }, segment.text));
      continue;
    }
    if (segment.type === "toolcall_ref") {
      const grouped = consumeGroupedToolcallRefs(index);
      const ids = grouped?.ids || [segment.id];
      index = grouped?.endIndex ?? index;
      const count = ids.length;
      nodes.push(h("button", {
        key: `${keyPrefix}-toolcall-${index}`,
        type: "button",
        class: "ecall-md-toolcall-ref",
        title: count > 1 ? ids.map((id) => `toolcall:${id}`).join("\n") : `toolcall:${ids[0]}`,
        "data-toolcall-id": ids[0],
        "data-toolcall-pill": "true",
        onClick: (event: MouseEvent) => {
          event.preventDefault();
          event.stopPropagation();
          options.onToolcallClick?.(ids, event.currentTarget instanceof HTMLButtonElement ? event.currentTarget : null);
        },
      }, [
        h(Wrench, { class: "ecall-md-toolcall-ref-icon" }),
        count > 1
          ? h("span", { class: "ecall-md-toolcall-ref-count" }, `+${count}`)
          : null,
      ]));
      continue;
    }
    if (segment.type === "math") {
      nodes.push(h(InlineMath, { key: `${keyPrefix}-m-${index}`, text: segment.text }));
      continue;
    }
    if (segment.type === "link") {
      const href = sanitizeMarkdownHref(segment.href);
      if (!href) {
        nodes.push(h("span", { key: `${keyPrefix}-a-${index}` }, segment.text));
        continue;
      }
      const isExternalUrl = /^https?:\/\//i.test(href);
      nodes.push(h("a", {
        key: `${keyPrefix}-a-${index}`,
        href: isExternalUrl ? href : "#",
        "data-href": isExternalUrl ? undefined : href,
        class: "ecall-md-link",
        ...(isExternalUrl ? { target: "_blank", rel: "noopener noreferrer" } : {}),
      }, segment.text));
      continue;
    }
    if (segment.type === "image") {
      nodes.push(h(MarkdownImage, {
        key: `${keyPrefix}-img-${index}`,
        src: segment.src,
        alt: segment.alt,
        localImageBasePath,
      }));
      continue;
    }
    if (segment.type === "strong") {
      nodes.push(h("strong", { key: `${keyPrefix}-b-${index}`, class: "ecall-md-strong" }, renderSegments(segment.children, `${keyPrefix}-b-${index}`, localImageBasePath, options)));
      continue;
    }
    if (segment.type === "em") {
      nodes.push(h("em", { key: `${keyPrefix}-i-${index}`, class: "ecall-md-em" }, renderSegments(segment.children, `${keyPrefix}-i-${index}`, localImageBasePath, options)));
      continue;
    }
    if (segment.type === "strongEm") {
      nodes.push(h("strong", { key: `${keyPrefix}-bi-${index}`, class: "ecall-md-strong" }, [
        h("em", { class: "ecall-md-em" }, renderSegments(segment.children, `${keyPrefix}-bi-${index}`, localImageBasePath, options)),
      ]));
      continue;
    }
    if (segment.type === "delete") {
      nodes.push(h("del", { key: `${keyPrefix}-d-${index}`, class: "ecall-md-del" }, renderSegments(segment.children, `${keyPrefix}-d-${index}`, localImageBasePath, options)));
      continue;
    }
    nodes.push(segment.text);
  }
  return nodes;
}

function sanitizeMarkdownHref(rawHref: string): string {
  const href = String(rawHref || "").replace(/[\u0000-\u001F\u007F]/g, "").trim();
  if (!href) return "";
  if (href.startsWith("#") || href.startsWith("/") || href.startsWith("./") || href.startsWith("../")) {
    return href;
  }
  if (href.startsWith("\\\\") || /^[A-Za-z]:[\\/]/.test(href)) {
    return href.replace(/\\/g, "/");
  }
  if (/^file:/i.test(href)) {
    try {
      const url = new URL(href);
      const decodedPath = decodeURIComponent(url.pathname || "");
      if (url.host && url.host !== "localhost") {
        return `\\\\${url.host}${decodedPath.replace(/\//g, "\\")}`;
      }
      return decodedPath.replace(/^\/([A-Za-z]:)/, "$1");
    } catch {
      return "";
    }
  }
  const schemeMatch = href.match(/^([A-Za-z][A-Za-z0-9+.-]*):/);
  if (!schemeMatch) return href;
  const scheme = schemeMatch[1].toLowerCase();
  if (scheme === "http" || scheme === "https" || scheme === "mailto") {
    return href;
  }
  return "";
}

// ==================== Inline Math (KaTeX) ====================

const InlineMath = defineComponent({
  name: "InlineMath",
  props: {
    text: { type: String, required: true },
  },
  setup(mathProps) {
    const html = computed(() => {
      try {
        const katex = (window as any).__ecall_katex;
        if (!katex) return null;
        return katex.renderToString(mathProps.text, { throwOnError: false, displayMode: false });
      } catch {
        return null;
      }
    });
    return () => {
      if (html.value) {
        return h("span", { class: "ecall-md-inline-math", innerHTML: html.value });
      }
      return h("code", { class: "ecall-md-inline-code" }, `$${mathProps.text}$`);
    };
  },
});

// ==================== Code Block ====================

const CodeBlock = defineComponent({
  name: "CodeBlock",
  props: {
    lang: { type: String, default: "" },
    code: { type: String, default: "" },
    blockKey: { type: String, default: "" },
    isDark: { type: Boolean, default: false },
    streaming: { type: Boolean, default: false },
  },
  setup(codeProps) {
    const highlightedHtml = ref("");
    const copied = ref(false);
    const previewOpen = ref(false);
    let copyTimer = 0;
    let highlightAbort: AbortController | null = null;

    const isMermaid = computed(() => codeProps.lang === "mermaid");

    async function highlight() {
      if (isMermaid.value) return;
      if (!codeProps.code) {
        highlightedHtml.value = "";
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
        highlightedHtml.value = html;
      } catch {
        highlightedHtml.value = "";
      }
    }

    watch(
      () => [codeProps.code, codeProps.lang, codeProps.isDark, codeProps.streaming],
      () => {
        // 已闭合的代码块（出现在 blocks 里）可以直接高亮
        highlight();
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
      if (highlightAbort) {
        highlightAbort.abort();
        highlightAbort = null;
      }
    });

    return () => {
      if (isMermaid.value) {
        return h(MermaidBlock, { code: codeProps.code, blockKey: codeProps.blockKey, isDark: codeProps.isDark, streaming: codeProps.streaming });
      }

      // 标题栏：左边语言名，右边复制按钮
      const titleBar = h("div", { class: "ecall-md-code-title" }, [
        h("span", { class: "ecall-md-code-lang" }, codeProps.lang || "text"),
        h("div", { class: "ecall-md-code-actions" }, [
          h("button", {
            type: "button",
            class: "ecall-md-code-action",
            title: t("common.expand"),
            onClick: openPreview,
          }, [h(Maximize2, { class: "ecall-md-code-action-icon" })]),
          h("button", {
            type: "button",
            class: "ecall-md-code-action ecall-md-code-copy",
            title: copied.value ? "已复制" : t("common.copy"),
            "aria-label": copied.value ? "已复制" : t("common.copy"),
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

// ==================== Math Block (KaTeX) ====================

const MathBlock = defineComponent({
  name: "MathBlock",
  props: {
    text: { type: String, required: true },
    blockKey: { type: String, default: "" },
    streaming: { type: Boolean, default: false },
  },
  setup(mathProps) {
    const html = computed(() => {
      try {
        const katex = (window as any).__ecall_katex;
        if (!katex) return null;
        return katex.renderToString(mathProps.text, { throwOnError: false, displayMode: true });
      } catch {
        return null;
      }
    });
    return () => {
      if (html.value) {
        return h("div", { class: "ecall-md-math-block", innerHTML: html.value });
      }
      return h("pre", { class: "ecall-md-math-fallback" }, [h("code", null, mathProps.text)]);
    };
  },
});

// ==================== Mermaid Block ====================

const MermaidBlock = defineComponent({
  name: "MermaidBlock",
  props: {
    code: { type: String, default: "" },
    blockKey: { type: String, default: "" },
    isDark: { type: Boolean, default: false },
    streaming: { type: Boolean, default: false },
  },
  setup(mermaidProps) {
    const svgHtml = ref("");
    const error = ref("");
    const containerRef = ref<HTMLElement | null>(null);
    let renderCount = 0;

    async function renderMermaid() {
      if (!mermaidProps.code.trim()) {
        svgHtml.value = "";
        return;
      }
      renderCount += 1;
      const currentRender = renderCount;
      try {
        const mermaid = (await import("mermaid")).default;
        mermaid.initialize({
          startOnLoad: false,
          theme: mermaidProps.isDark ? "dark" : "default",
          securityLevel: "strict",
        });
        const id = `ecall-mermaid-${mermaidProps.blockKey}-${currentRender}`;
        const { svg } = await mermaid.render(id, mermaidProps.code.trim());
        if (currentRender !== renderCount) return;
        svgHtml.value = svg;
        error.value = "";
      } catch (e: any) {
        if (currentRender !== renderCount) return;
        svgHtml.value = "";
        error.value = String(e?.message || "Mermaid render error");
      }
    }

    watch(
      () => [mermaidProps.code, mermaidProps.isDark],
      () => renderMermaid(),
      { immediate: true },
    );

    return () => {
      if (!svgHtml.value && !error.value) {
        return h("pre", { class: "ecall-md-math-fallback" }, [h("code", null, mermaidProps.code)]);
      }
      if (error.value) {
        return h("div", { class: "ecall-md-mermaid-error" }, [
          h("pre", null, [h("code", null, mermaidProps.code)]),
          h("div", { class: "ecall-md-mermaid-error-msg" }, error.value),
        ]);
      }
      return h("div", {
        ref: containerRef,
        class: "ecall-md-mermaid-block",
        innerHTML: svgHtml.value,
      });
    };
  },
});
</script>

<style>
.ecall-md-renderer {
  min-width: 0;
  max-width: 100%;
  overflow-wrap: anywhere;
  white-space: normal;
  font-size: 0.875rem;
  line-height: 1.5;
}

.ecall-md-renderer > :first-child {
  margin-top: 0;
}

.ecall-md-renderer > :last-child {
  margin-bottom: 0;
}

/* ==================== Headings ==================== */
.ecall-md-heading {
  margin: 0.25rem 0;
  font-weight: 650;
  line-height: 1.45;
}

h1.ecall-md-heading { font-size: 1.02rem; }
h2.ecall-md-heading { font-size: 0.98rem; }
h3.ecall-md-heading { font-size: 0.94rem; }
h4.ecall-md-heading { font-size: 0.9rem; }

/* ==================== Paragraph ==================== */
.ecall-md-paragraph {
  margin: 0.25rem 0;
  white-space: pre-wrap;
}

.ecall-md-toolcall-ref {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.15rem;
  min-width: 1.15rem;
  height: 1.15rem;
  padding: 0 0.34rem;
  margin-left: 0.18rem;
  border: 0;
  border-radius: 999px;
  background: color-mix(in srgb, currentColor 10%, transparent);
  color: color-mix(in srgb, currentColor 72%, transparent);
  cursor: pointer;
  line-height: 1;
  vertical-align: text-top;
  white-space: nowrap;
}

.ecall-md-toolcall-ref:hover {
  background: color-mix(in srgb, currentColor 16%, transparent);
  color: currentColor;
}

.ecall-md-toolcall-ref-icon {
  width: 0.72rem;
  height: 0.72rem;
  pointer-events: none;
}

.ecall-md-toolcall-ref-count {
  font-size: 0.62rem;
  font-weight: 700;
  line-height: 1;
  pointer-events: none;
}

.ecall-md-toolcall-scroll {
  scrollbar-gutter: auto;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.ecall-md-toolcall-scroll::-webkit-scrollbar {
  width: 0;
  height: 0;
}

/* ==================== Blockquote ==================== */
.ecall-md-quote {
  margin: 0.25rem 0;
  border-left: 2px solid color-mix(in srgb, currentColor 22%, transparent);
  padding-left: 0.68rem;
  color: color-mix(in srgb, currentColor 82%, transparent);
  white-space: pre-wrap;
}

/* ==================== Lists ==================== */
.ecall-md-list {
  margin: 0.25rem 0;
  padding-left: 0.85rem;
}

.ecall-md-list li {
  margin: 0.12rem 0;
  padding-left: 0;
}

.ecall-md-list-ordered {
  list-style: decimal;
}

ul.ecall-md-list {
  list-style: disc;
}

/* ==================== Table ==================== */
.ecall-md-table-wrap {
  max-width: 100%;
  overflow-x: auto;
  margin: 0.35rem 0;
}

.ecall-md-table {
  width: max-content;
  min-width: 100%;
  border-collapse: collapse;
  font-size: 0.84rem;
  line-height: 1.45;
}

.ecall-md-table th,
.ecall-md-table td {
  border: 1px solid color-mix(in srgb, currentColor 20%, transparent);
  padding: 0.32rem 0.48rem;
  text-align: left;
  vertical-align: top;
}

.ecall-md-table th {
  font-weight: 650;
  background: color-mix(in srgb, currentColor 7%, transparent);
}

/* ==================== Code Block ==================== */
.ecall-md-code-block {
  margin: 0.25rem 0;
  border-radius: 0.5rem;
  overflow: hidden;
  border: 1px solid color-mix(in srgb, currentColor 12%, transparent);
}

.ecall-md-code-title {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.4rem 0.75rem;
  background: color-mix(in srgb, currentColor 6%, transparent);
  border-bottom: 1px solid color-mix(in srgb, currentColor 10%, transparent);
}

.ecall-md-code-lang {
  font-size: 0.72rem;
  color: color-mix(in srgb, currentColor 60%, transparent);
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
}

.ecall-md-code-actions {
  display: flex;
  align-items: center;
  gap: 0.3rem;
}

.ecall-md-code-action,
.ecall-md-code-copy {
  border: none;
  background: none;
  padding: 0.1rem 0.35rem;
  font-size: 0.72rem;
  color: color-mix(in srgb, currentColor 65%, transparent);
  cursor: pointer;
  border-radius: 0.25rem;
}

.ecall-md-code-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: 0.18rem;
}

.ecall-md-code-action-icon {
  width: 0.9rem;
  height: 0.9rem;
}

.ecall-md-code-action:hover,
.ecall-md-code-copy:hover {
  background: color-mix(in srgb, currentColor 10%, transparent);
  color: currentColor;
}

.ecall-md-code-body {
  overflow-x: auto;
  padding: 0.75rem 1rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  font-size: 0.82rem;
  line-height: 1.55;
  margin: 0;
  white-space: pre;
}

.ecall-md-code-plain {
  background: color-mix(in srgb, currentColor 5%, transparent);
}

.ecall-md-code-plain code {
  background: transparent;
  border: 0;
  padding: 0;
  font: inherit;
  color: inherit;
}

/* shiki 输出的 pre 去掉自带的 margin/padding/圆角，由外壳统一控制 */
.ecall-md-code-body pre,
.ecall-md-code-body pre.shiki,
.ecall-md-code-body .shiki {
  margin: 0 !important;
  padding: 0 !important;
  border-radius: 0 !important;
  border: 0 !important;
  overflow: visible !important;
  background: transparent !important;
}

.ecall-md-code-body pre code {
  background: transparent !important;
  border: 0 !important;
  padding: 0 !important;
  box-shadow: none !important;
  font: inherit;
}

.ecall-md-code-body .line,
.ecall-md-code-body .shiki span {
  background: transparent !important;
  box-shadow: none !important;
  text-shadow: none !important;
}

/* 暗色：代码区背景跟 shiki github-dark 一致 */
.ecall-md-dark .ecall-md-code-body {
  background: #101828;
  color: #e5e7eb;
}

/* 亮色：代码区背景跟 shiki github-light 一致 */
.ecall-md-light .ecall-md-code-body {
  background: #f6f8fa;
  color: #24292f;
}

/* ==================== Inline Code ==================== */
.ecall-md-inline-code {
  border-radius: 0.28rem;
  background: color-mix(in srgb, currentColor 10%, transparent);
  padding: 0.08rem 0.28rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  font-size: 0.86em;
}

/* ==================== Links ==================== */
.ecall-md-link {
  color: var(--color-primary);
  text-decoration: underline;
  text-decoration-thickness: 0.08em;
  text-underline-offset: 0.18em;
}

/* ==================== Images ==================== */
.ecall-md-image {
  display: inline-block;
  max-width: min(28rem, 80vw);
  max-height: 18rem;
  border-radius: 0.5rem;
  object-fit: contain;
  vertical-align: middle;
}

.ecall-md-local-image {
  cursor: zoom-in;
}

.ecall-md-image-placeholder {
  display: inline-flex;
  min-width: 4rem;
  max-width: min(16rem, 60vw);
  min-height: 2.75rem;
  align-items: center;
  justify-content: center;
  padding: 0.35rem 0.55rem;
  border: 1px dashed color-mix(in srgb, currentColor 28%, transparent);
  border-radius: 0.5rem;
  color: color-mix(in srgb, currentColor 62%, transparent);
  font-size: 0.78rem;
  line-height: 1.25;
  vertical-align: middle;
  cursor: zoom-in;
}

.ecall-md-image-error {
  cursor: default;
  color: color-mix(in srgb, currentColor 42%, transparent);
}

/* ==================== Emphasis ==================== */
.ecall-md-strong {
  font-weight: 700;
}

.ecall-md-em {
  font-style: italic;
}

.ecall-md-del {
  text-decoration: line-through;
  color: color-mix(in srgb, currentColor 76%, transparent);
}

/* ==================== HR ==================== */
.ecall-md-hr {
  margin: 0.65rem 0;
  border: 0;
  border-top: 1px solid color-mix(in srgb, currentColor 16%, transparent);
}

/* ==================== Math ==================== */
.ecall-md-inline-math {
  display: inline;
}

.ecall-md-math-block {
  margin: 0.35rem 0;
  overflow-x: auto;
  text-align: center;
}

.ecall-md-math-fallback {
  margin: 0.35rem 0;
  padding: 0.5rem 0.65rem;
  background: color-mix(in srgb, currentColor 8%, transparent);
  border-radius: 0.4rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  font-size: 0.82rem;
  overflow-x: auto;
}

/* ==================== Mermaid ==================== */
.ecall-md-mermaid-block {
  margin: 0.35rem 0;
  overflow-x: auto;
  text-align: center;
}

.ecall-md-mermaid-block svg {
  max-width: 100%;
  height: auto;
}

.ecall-md-mermaid-loading {
  margin: 0.35rem 0;
  padding: 0.5rem;
  text-align: center;
  color: color-mix(in srgb, currentColor 55%, transparent);
  font-size: 0.82rem;
}

.ecall-md-mermaid-error {
  margin: 0.35rem 0;
}

.ecall-md-mermaid-error pre {
  padding: 0.5rem 0.65rem;
  background: color-mix(in srgb, currentColor 8%, transparent);
  border-radius: 0.4rem;
  font-size: 0.82rem;
  overflow-x: auto;
}

.ecall-md-mermaid-error-msg {
  margin-top: 0.25rem;
  font-size: 0.75rem;
  color: var(--color-error, #ef4444);
}

/* ==================== Document Variant ==================== */
.ecall-md-document {
  font-size: 0.95rem;
  line-height: 1.7;
}

.ecall-md-document .ecall-md-heading {
  margin: 1.2rem 0 0.5rem;
  font-weight: 700;
  line-height: 1.35;
}

.ecall-md-document h1.ecall-md-heading { font-size: 1.5rem; }
.ecall-md-document h2.ecall-md-heading { font-size: 1.28rem; }
.ecall-md-document h3.ecall-md-heading { font-size: 1.12rem; }
.ecall-md-document h4.ecall-md-heading { font-size: 1.02rem; }

.ecall-md-document .ecall-md-paragraph {
  margin: 0.6rem 0;
  line-height: 1.8;
}

.ecall-md-document .ecall-md-quote {
  margin: 0.6rem 0;
  border-left-width: 3px;
  padding-left: 0.85rem;
  line-height: 1.75;
}

.ecall-md-document .ecall-md-list {
  margin: 0.5rem 0;
  padding-left: 1.4rem;
  line-height: 1.75;
}

.ecall-md-document .ecall-md-list li {
  margin: 0.25rem 0;
}

.ecall-md-document .ecall-md-table-wrap {
  margin: 0.75rem 0;
}

.ecall-md-document .ecall-md-table {
  font-size: 0.88rem;
  line-height: 1.55;
}

.ecall-md-document .ecall-md-table th,
.ecall-md-document .ecall-md-table td {
  padding: 0.45rem 0.65rem;
}

.ecall-md-document .ecall-md-code-block {
  margin: 0.75rem 0;
  border-radius: 0.6rem;
}

.ecall-md-document .ecall-md-code-title {
  padding: 0.45rem 0.85rem;
}

.ecall-md-document .ecall-md-code-body {
  padding: 0.85rem 1.1rem;
  font-size: 0.85rem;
  line-height: 1.6;
}

.ecall-md-document .ecall-md-hr {
  margin: 1.2rem 0;
}

.ecall-md-document .ecall-md-math-block {
  margin: 0.75rem 0;
}

.ecall-md-document .ecall-md-mermaid-block {
  margin: 0.75rem 0;
}
</style>

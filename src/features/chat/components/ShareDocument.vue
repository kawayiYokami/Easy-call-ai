<script setup lang="ts">
import ChatBubbleShell from "./ChatBubbleShell.vue";
import { AppMarkdownRenderer } from "../markdown";
import type { ShareDocumentEntry } from "./share-document-types";

const props = defineProps<{
  title: string;
  subtitle?: string;
  exportAtText: string;
  projectUrl: string;
  qrDataUrl: string;
  brandIconUrl: string;
  isDark: boolean;
  entries: ShareDocumentEntry[];
  width?: number;
}>();
</script>

<template>
  <main
    class="pai-share-document bg-base-200 text-base-content"
    :style="{ width: `${width || 760}px` }"
    data-share-document="1"
  >
    <header
      class="pai-share-brand mb-3 flex items-center gap-3 rounded-2xl border border-base-300 bg-base-100 px-4 py-3 text-base-content shadow-sm"
    >
      <div class="flex min-w-0 items-center gap-3">
        <img
          :src="brandIconUrl"
          alt="P-ai"
          class="pai-share-brand-mark h-10 w-10 rounded-xl bg-white/15 object-contain"
        />
        <div class="min-w-0">
          <div class="text-lg font-extrabold tracking-wide">P-ai</div>
          <div class="mt-0.5 text-xs leading-snug text-base-content/55">
            漂亮、流畅、功能齐全的桌面 AI Harness
          </div>
        </div>
      </div>
    </header>

    <section class="card bg-base-100 shadow-sm">
      <div class="card-body gap-4 p-3.5">
        <article
          v-for="entry in entries"
          :key="entry.id"
          class="pai-share-message"
        >
          <ChatBubbleShell
            :tone="entry.tone"
            :name="entry.displayName"
            :meta="entry.createdAtText"
            :avatar-url="entry.avatarUrl"
            :bubble-background="true"
            :wide="entry.tone === 'assistant'"
          >
            <template v-if="entry.thinkingSummary" #activity>
              <div class="text-sm font-normal text-base-content/55">
                {{ entry.thinkingSummary }}
              </div>
            </template>

            <div
              :class="entry.tone === 'assistant'
                ? 'assistant-markdown ecall-assistant-bubble max-w-full'
                : 'max-w-full'"
            >
              <div
                v-if="entry.tone === 'user'"
                class="whitespace-pre-wrap break-words text-sm leading-relaxed"
              >
                {{ entry.text }}
              </div>
              <AppMarkdownRenderer
                v-else-if="entry.text"
                class="ecall-markdown-content max-w-none"
                :text="entry.text"
                :is-dark="isDark"
                :streaming="false"
              />

              <div
                v-if="entry.images && entry.images.length > 0"
                class="mt-2 flex flex-wrap gap-2"
              >
                <img
                  v-for="(image, index) in entry.images"
                  :key="`${entry.id}-img-${index}`"
                  :src="image.src"
                  :alt="image.alt"
                  class="max-h-52 max-w-[220px] rounded-xl border border-base-300 object-contain"
                />
              </div>

              <div
                v-if="(entry.attachmentNames && entry.attachmentNames.length > 0) || (entry.audioCount || 0) > 0"
                class="mt-2 flex flex-wrap gap-1.5"
              >
                <span
                  v-for="name in (entry.attachmentNames || [])"
                  :key="`${entry.id}-file-${name}`"
                  class="badge badge-ghost badge-sm"
                >
                  {{ name }}
                </span>
                <span
                  v-if="(entry.audioCount || 0) > 0"
                  class="badge badge-ghost badge-sm"
                >
                  音频 ×{{ entry.audioCount }}
                </span>
              </div>
            </div>
          </ChatBubbleShell>
        </article>
      </div>
    </section>

    <footer class="mt-3 flex items-center justify-between gap-3.5 rounded-2xl border border-base-300 bg-base-100 px-4 py-3.5">
      <div class="min-w-0">
        <div class="text-xs font-semibold text-base-content/70">开源项目 · GitHub</div>
        <a class="link link-primary mt-1 block break-all text-xs" :href="projectUrl">{{ projectUrl }}</a>
      </div>
      <div v-if="qrDataUrl" class="flex shrink-0 items-center gap-2.5">
        <div class="text-right text-xs leading-snug text-base-content/55">
          扫码查看项目
        </div>
        <img
          :src="qrDataUrl"
          alt="P-ai project QR"
          class="h-[92px] w-[92px] shrink-0 rounded-xl border border-base-300 bg-base-100 p-1.5 object-contain"
        />
      </div>
    </footer>
  </main>
</template>

<style scoped>
.pai-share-document {
  margin: 0 auto;
  padding: 18px 14px 24px;
  box-sizing: border-box;
}

.pai-share-brand-mark {
  letter-spacing: 0.02em;
}

.assistant-markdown :deep(.ecall-markdown-content) {
  color: var(--color-base-content);
  font-size: var(--app-text-base-size);
  line-height: 1.7;
}

.assistant-markdown :deep(.ecall-markdown-content > :first-child) {
  margin-top: 0;
}

.assistant-markdown :deep(.ecall-markdown-content > :last-child) {
  margin-bottom: 0;
}
</style>

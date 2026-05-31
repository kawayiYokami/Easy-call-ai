<template>
  <div ref="toolbarRootRef" class="rounded-box border border-base-300 bg-base-100/70 px-2 py-1.5 flex items-center justify-between gap-2 text-[11px]">
    <div ref="toolbarLeftRef" class="flex min-w-0 items-center gap-1.5">
      <div
        v-if="!hideMenuButton"
        class="dropdown dropdown-start"
        :class="menuPlacement === 'top' ? 'dropdown-top' : 'dropdown-bottom'"
      >
        <button
          ref="menuButtonRef"
          type="button"
          tabindex="0"
          class="btn btn-sm btn-ghost btn-circle shrink-0"
          :title="t('chat.conversationMenu.title')"
          @mousedown="updateMenuPlacement"
        >
          <Grip class="h-4 w-4" />
        </button>
        <ul
          tabindex="0"
          class="dropdown-content menu z-50 w-64 rounded-box border border-base-300 bg-base-100 p-3 text-sm shadow-xl"
          :class="menuPlacement === 'top' ? 'mb-3' : 'mt-3'"
        >
          <li v-if="!busy">
            <button v-if="showDelegateMenuItem" type="button" class="flex min-h-10 items-center justify-start gap-3 px-4 py-2 text-left" :disabled="busy" @click="emit('openDelegateSelection')">
              <ClipboardList class="h-4 w-4 shrink-0" />
              <span class="leading-5">{{ t("chat.conversationMenu.startDelegate") }}</span>
            </button>
          </li>
          <li v-if="!busy">
            <button v-if="showBranchMenuItem" type="button" class="flex min-h-10 items-center justify-start gap-3 px-4 py-2 text-left" :disabled="busy" @click="emit('openBranchSelection')">
              <GitBranchPlus class="h-4 w-4 shrink-0" />
              <span class="leading-5">{{ t("chat.conversationMenu.branchConversation") }}</span>
            </button>
          </li>
          <li v-if="showCodeReviewMenuItem && !busy">
            <button type="button" class="flex min-h-10 items-center justify-start gap-3 px-4 py-2 text-left" :disabled="busy" @click="emit('openCodeReview')">
              <ClipboardCheck class="h-4 w-4 shrink-0" />
              <span class="leading-5">发起代码审查</span>
            </button>
          </li>
          <li v-if="!busy">
            <button v-if="showForwardMenuItem" type="button" class="flex min-h-10 items-center justify-start gap-3 px-4 py-2 text-left" :disabled="busy" @click="emit('openForwardSelection')">
              <Package class="h-4 w-4 shrink-0" />
              <span class="leading-5">{{ t("chat.conversationMenu.forwardConversation") }}</span>
            </button>
          </li>
          <li v-if="!busy">
            <button v-if="showShareMenuItem" type="button" class="flex min-h-10 items-center justify-start gap-3 px-4 py-2 text-left" :disabled="busy" @click="emit('openShareSelection')">
              <ExternalLink class="h-4 w-4 shrink-0" />
              <span class="leading-5">{{ t("chat.conversationMenu.shareConversation") }}</span>
            </button>
          </li>
          <li v-if="showWorkspaceMenuItem && !busy && !workspaceButtonDisabled">
            <button type="button" class="flex min-h-10 items-center justify-start gap-3 px-4 py-2 text-left" :disabled="busy || workspaceButtonDisabled" @click="emit('lockWorkspace')">
              <Folder class="h-4 w-4 shrink-0" />
              <span class="leading-5">{{ t("chat.conversationMenu.setWorkspace") }}</span>
            </button>
          </li>
          <li v-if="showDetachButton && !busy && !detachDisabled">
            <button
              type="button"
              class="flex min-h-10 items-center justify-start gap-3 px-4 py-2 text-left"
              :disabled="busy || detachDisabled"
              @mousedown="handleDetachConversationMouseDown"
              @click="handleDetachConversationClick"
            >
              <ExternalLink class="h-4 w-4 shrink-0" />
              <span class="leading-5">{{ t("chat.conversationMenu.openDetachedWindow") }}</span>
            </button>
          </li>
        </ul>
      </div>
      <button
        v-if="!hideWorkspaceButton"
        class="btn btn-sm btn-ghost gap-1.5"
        :disabled="busy || workspaceButtonDisabled"
        @click="emit('lockWorkspace')"
      >
        <SquareTerminal class="h-3.5 w-3.5" />
        {{ workspaceButtonName || workspaceButtonLabel }}
      </button>
    </div>
    <div class="flex min-w-0 items-center justify-end gap-1.5">
      <div
        v-if="compactPersonaList"
        class="dropdown dropdown-top dropdown-end"
      >
        <button
          type="button"
          tabindex="0"
          class="btn btn-ghost btn-sm btn-circle shrink-0 border border-base-300/70 bg-base-100/70 hover:border-base-300 hover:bg-base-200"
          :disabled="chatting || frozen || uniqueMentionEntries.length === 0"
          title="人格列表"
        >
          <Users class="h-4 w-4" />
        </button>
        <ul
          tabindex="0"
          class="dropdown-content menu z-50 mb-2 w-max min-w-56 max-w-[min(80vw,20rem)] rounded-box border border-base-300 bg-base-100 p-1 shadow-xl"
        >
          <li
            v-for="entry in uniqueMentionEntries"
            :key="entry.agentId"
          >
            <button
              type="button"
              class="flex min-h-0 w-full items-center gap-2 rounded-xl px-2 py-1.5 text-left text-base-content transition-colors hover:bg-base-200/80"
              :disabled="chatting || frozen || !entry.mentionable"
              @click="handleCompactPersonaEntryClick($event, entry)"
            >
              <div class="indicator shrink-0">
                <span
                  v-if="entry.selected"
                  class="indicator-item indicator-top indicator-end inline-flex h-4 w-4 translate-x-1/4 -translate-y-1/4 items-center justify-center rounded-full bg-primary text-[9px] font-bold text-primary-content"
                >
                  @
                </span>
                <span
                  v-else-if="entry.hasBackgroundTask"
                  class="indicator-item indicator-bottom indicator-end inline-flex min-w-5 translate-x-1/4 translate-y-1/4 items-center justify-center rounded-full border border-base-300 bg-base-100 px-1 py-0.5 text-[9px] text-base-content shadow-sm"
                >
                  <span class="loading loading-dots loading-xs"></span>
                </span>
                <div class="avatar">
                  <div class="w-7 rounded-full">
                    <img
                      v-if="entry.avatarUrl"
                      :src="entry.avatarUrl"
                      :alt="entry.agentName"
                      class="w-7 h-7 rounded-full object-cover"
                      :class="frontSpeakingMuted(entry) ? 'grayscale opacity-75' : ''"
                    />
                    <div
                      v-else
                      class="w-7 h-7 rounded-full flex items-center justify-center text-[10px]"
                      :class="frontSpeakingMuted(entry)
                        ? 'bg-base-300 text-base-content/70'
                        : 'bg-neutral text-neutral-content'"
                    >
                      {{ avatarInitial(entry.agentName) }}
                    </div>
                  </div>
                </div>
              </div>
              <div class="min-w-0 flex-1 pr-0.5">
                <div class="truncate text-sm leading-5">@{{ entry.agentName }}</div>
                <div class="truncate text-[11px] leading-4 text-base-content/60">
                  {{ entry.departmentName || t("chat.defaultDepartment") }}
                </div>
              </div>
            </button>
          </li>
        </ul>
      </div>
      <template v-else>
        <button
          v-for="entry in uniqueMentionEntries"
          :key="entry.agentId"
          type="button"
          class="btn btn-ghost btn-sm btn-circle overflow-visible p-0 shrink-0 border relative"
          :class="personaChipClass(entry)"
          :title="mentionEntryTitle(entry)"
          :disabled="chatting || frozen || !entry.mentionable"
          @click="handleMentionEntryClick($event, entry)"
        >
          <div class="indicator">
            <span
              v-if="entry.selected"
              class="indicator-item indicator-top indicator-end inline-flex h-4 w-4 translate-x-1/4 -translate-y-1/4 items-center justify-center rounded-full bg-primary text-[9px] font-bold text-primary-content"
            >
              @
            </span>
            <span
              v-else-if="entry.hasBackgroundTask"
              class="indicator-item indicator-bottom indicator-end inline-flex min-w-5 translate-x-1/4 translate-y-1/4 items-center justify-center rounded-full border border-base-300 bg-base-100 px-1 py-0.5 text-[9px] text-base-content shadow-sm"
            >
              <span class="loading loading-dots loading-xs"></span>
            </span>
            <span
              v-if="props.selectedMentionKeys.length > 0 && entry.isFrontSpeaking"
              class="indicator-item indicator-top indicator-start inline-flex h-4 w-4 -translate-x-1/4 -translate-y-1/4 items-center justify-center rounded-full bg-base-300 text-[9px] font-bold text-base-content"
            >
              {{ t("chat.mentionMutedBadge") }}
            </span>
            <div class="avatar">
              <div class="w-7 rounded-full">
                <img
                  v-if="entry.avatarUrl"
                  :src="entry.avatarUrl"
                  :alt="entry.agentName"
                  class="w-7 h-7 rounded-full object-cover"
                  :class="frontSpeakingMuted(entry) ? 'grayscale opacity-75' : ''"
                />
                <div
                  v-else
                  class="w-7 h-7 rounded-full flex items-center justify-center text-[10px]"
                  :class="frontSpeakingMuted(entry)
                    ? 'bg-base-300 text-base-content/70'
                    : 'bg-neutral text-neutral-content'"
                >
                  {{ avatarInitial(entry.agentName) }}
                </div>
              </div>
            </div>
          </div>
        </button>
      </template>
    </div>
  </div>
  <Teleport to="body">
    <div
      v-if="avatarPopupTarget"
      class="fixed z-1200"
      :style="avatarPopupStyle"
    >
      <div ref="avatarPopupPanelRef" class="w-max max-w-[min(80vw,20rem)] overflow-hidden rounded-box border border-base-300 bg-base-100 p-1 shadow-xl">
        <ul class="flex flex-col gap-1">
          <li
            v-for="entry in filteredAvatarPopupOptions"
            :key="`${entry.agentId}:${entry.departmentId}`"
          >
            <button
              type="button"
              class="flex min-h-0 w-full items-start gap-2 rounded-xl px-2 py-1.5 text-left text-base-content transition-colors hover:bg-base-200/80"
              @click="applyAvatarPopupSelection(entry)"
            >
              <div class="avatar shrink-0">
                <div class="w-7 rounded-full">
                  <img
                    v-if="entry.avatarUrl"
                    :src="entry.avatarUrl"
                    :alt="entry.agentName"
                    class="w-7 h-7 rounded-full object-cover"
                  />
                  <div v-else class="bg-neutral text-neutral-content w-7 h-7 rounded-full flex items-center justify-center text-[10px]">
                    {{ avatarInitial(entry.agentName) }}
                  </div>
                </div>
              </div>
              <div class="min-w-0 flex-1 pr-0.5">
                <div class="truncate text-sm leading-5">@{{ entry.agentName }}</div>
                <div class="truncate text-[11px] leading-4 text-base-content/60">
                  {{ entry.departmentName || t("chat.defaultDepartment") }}
                </div>
              </div>
            </button>
          </li>
        </ul>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { ClipboardCheck, ClipboardList, ExternalLink, Folder, GitBranchPlus, Grip, Package, SquareTerminal, Users } from "@lucide/vue";
import type { ChatMentionEntry } from "../../../types/app";

const props = withDefaults(defineProps<{
  chatting: boolean;
  frozen: boolean;
  conversationBusy?: boolean;
  workspaceButtonLabel: string;
  workspaceButtonName: string;
  workspaceButtonDisabled?: boolean;
  mentionEntries: ChatMentionEntry[];
  selectedMentionKeys: string[];
  hideMenuButton?: boolean;
  hideWorkspaceButton?: boolean;
  showDelegateMenuItem?: boolean;
  showBranchMenuItem?: boolean;
  showCodeReviewMenuItem?: boolean;
  showForwardMenuItem?: boolean;
  showShareMenuItem?: boolean;
  showWorkspaceMenuItem?: boolean;
  showDetachButton?: boolean;
  detachDisabled?: boolean;
}>(), {
  showDelegateMenuItem: true,
  showBranchMenuItem: true,
  showCodeReviewMenuItem: false,
  showForwardMenuItem: true,
  showShareMenuItem: true,
  showWorkspaceMenuItem: true,
});

const emit = defineEmits<{
  (e: "lockWorkspace"): void;
  (e: "openBranchSelection"): void;
  (e: "openCodeReview"): void;
  (e: "openDelegateSelection"): void;
  (e: "openForwardSelection"): void;
  (e: "openShareSelection"): void;
  (e: "detachConversation"): void;
  (e: "mentionEntry", entry: ChatMentionEntry): void;
}>();

const { t } = useI18n();
const busy = computed(() => props.chatting || props.frozen || !!props.conversationBusy);
const showDelegateMenuItem = computed(() => props.showDelegateMenuItem);
const showBranchMenuItem = computed(() => props.showBranchMenuItem);
const showCodeReviewMenuItem = computed(() => props.showCodeReviewMenuItem);
const showForwardMenuItem = computed(() => props.showForwardMenuItem);
const showShareMenuItem = computed(() => props.showShareMenuItem);
const showWorkspaceMenuItem = computed(() => props.showWorkspaceMenuItem);
const toolbarRootRef = ref<HTMLElement | null>(null);
const toolbarLeftRef = ref<HTMLElement | null>(null);
const compactPersonaList = ref(false);
let toolbarResizeObserver: ResizeObserver | null = null;
const PERSONA_BUTTON_SIZE = 32;
const PERSONA_BUTTON_GAP = 6;
const TOOLBAR_SECTION_GAP = 8;
const POPUP_OFFSET = 8;
const POPUP_VIEWPORT_PADDING = 8;

// ========== 头像栏去重 + 部门弹出 ==========

const uniqueMentionEntries = computed(() => {
  const seen = new Map<string, ChatMentionEntry>();
  for (const entry of props.mentionEntries || []) {
    const agentId = String(entry.agentId || "").trim();
    if (!agentId) continue;
    if (!seen.has(agentId)) {
      seen.set(agentId, { ...entry, selected: false });
    } else {
      const existing = seen.get(agentId)!;
      if (entry.mentionable && !existing.mentionable) {
        seen.set(agentId, { ...entry, selected: false });
      }
    }
  }
  const result = Array.from(seen.values());
  for (const entry of result) {
    const agentId = String(entry.agentId || "").trim();
    entry.selected = agentId ? props.selectedMentionKeys.some((key) => String(key || "").trim().startsWith(`${agentId}:`)) : false;
  }
  return result;
});

const avatarPopupTarget = ref<{
  agentId: string;
  agentName: string;
  avatarUrl?: string;
} | null>(null);
const avatarPopupPanelRef = ref<HTMLElement | null>(null);

const avatarPopupStyle = ref<Record<string, string>>({
  left: "0px",
  top: "0px",
});

const filteredAvatarPopupOptions = computed(() => {
  const target = avatarPopupTarget.value;
  if (!target) return [];
  return (props.mentionEntries || [])
    .filter((entry) => String(entry.agentId || "").trim() === target.agentId)
    .map((entry) => ({
      agentId: String(entry.agentId || "").trim(),
      agentName: String(entry.agentName || "").trim(),
      departmentId: String(entry.departmentId || "").trim(),
      departmentName: String(entry.departmentName || "").trim(),
      avatarUrl: String(entry.avatarUrl || "").trim() || undefined,
    }))
    .filter((entry) => !!entry.agentId && !!entry.departmentId);
});

function handleMentionEntryClick(event: MouseEvent, entry: ChatMentionEntry & { selected?: boolean }) {
  const agentId = String(entry.agentId || "").trim();
  const deptEntries = (props.mentionEntries || []).filter((e) => String(e.agentId || "").trim() === agentId);
  if (deptEntries.length <= 1) {
    emit('mentionEntry', deptEntries[0] || entry);
    return;
  }
  avatarPopupTarget.value = { agentId: entry.agentId, agentName: entry.agentName, avatarUrl: entry.avatarUrl };
  const el = event.currentTarget as HTMLElement | null;
  if (el) {
    void updateAvatarPopupPlacement(el.getBoundingClientRect());
  }
}

function clampPopupPosition(anchorRect: DOMRect, panelEl: HTMLElement | null) {
  const measuredWidth = Math.round(panelEl?.offsetWidth || 0);
  const measuredHeight = Math.round(panelEl?.offsetHeight || 0);
  const maxLeft = Math.max(
    POPUP_VIEWPORT_PADDING,
    window.innerWidth - measuredWidth - POPUP_VIEWPORT_PADDING,
  );
  const left = Math.min(
    Math.max(POPUP_VIEWPORT_PADDING, Math.round(anchorRect.left)),
    maxLeft,
  );
  const top = Math.max(
    POPUP_VIEWPORT_PADDING,
    Math.round(anchorRect.top) - measuredHeight - POPUP_OFFSET,
  );
  return {
    left: `${left}px`,
    top: `${top}px`,
  };
}

async function updateAvatarPopupPlacement(anchorRect?: DOMRect) {
  const rect = anchorRect;
  if (!rect) return;
  await nextTick();
  avatarPopupStyle.value = clampPopupPosition(rect, avatarPopupPanelRef.value);
}

function handleCompactPersonaEntryClick(event: MouseEvent, entry: ChatMentionEntry & { selected?: boolean }) {
  handleMentionEntryClick(event, entry);
}

function applyAvatarPopupSelection(entry: {
  agentId: string;
  agentName: string;
  departmentId: string;
  departmentName: string;
  avatarUrl?: string;
}) {
  avatarPopupTarget.value = null;
  const matched = (props.mentionEntries || []).find(
    (e) => String(e.agentId || "").trim() === entry.agentId && String(e.departmentId || "").trim() === entry.departmentId,
  );
  if (matched) {
    emit('mentionEntry', matched);
  }
}

function closeAvatarPopup() {
  avatarPopupTarget.value = null;
}

function handleAvatarClickOutside(event: MouseEvent) {
  const target = event.target as HTMLElement | null;
  if (!target) {
    closeAvatarPopup();
    return;
  }
  if (
    avatarPopupTarget.value
    && !avatarPopupPanelRef.value?.contains(target)
  ) {
    closeAvatarPopup();
  }
}

const menuButtonRef = ref<HTMLButtonElement | null>(null);
const menuPlacement = ref<"top" | "bottom">("top");

function updateMenuPlacement() {
  const rect = menuButtonRef.value?.getBoundingClientRect();
  if (!rect) return;
  menuPlacement.value = rect.top >= window.innerHeight / 2 ? "top" : "bottom";
}

function updateToolbarPersonaLayout() {
  const root = toolbarRootRef.value;
  const left = toolbarLeftRef.value;
  if (!root || !left) return;
  const availableWidth = Math.round(root.getBoundingClientRect().width);
  const leftWidth = Math.ceil(left.getBoundingClientRect().width);
  const personaCount = uniqueMentionEntries.value.length;
  const personaWidth = personaCount > 0
    ? (personaCount * PERSONA_BUTTON_SIZE) + (Math.max(0, personaCount - 1) * PERSONA_BUTTON_GAP)
    : 0;
  compactPersonaList.value = personaCount > 0 && leftWidth + personaWidth + TOOLBAR_SECTION_GAP > availableWidth;
}

function handleDetachConversationMouseDown() {
  updateMenuPlacement();
  console.info("[独立聊天窗口][前端入口] 工具栏按钮 mousedown", {
    chatting: props.chatting,
    frozen: props.frozen,
    detachDisabled: !!props.detachDisabled,
  });
}

function handleDetachConversationClick() {
  console.info("[独立聊天窗口][前端入口] 工具栏按钮已点击，准备向上派发 detachConversation", {
    chatting: props.chatting,
    frozen: props.frozen,
    detachDisabled: !!props.detachDisabled,
  });
  emit("detachConversation");
}

function avatarInitial(name: string): string {
  const text = (name || "").trim();
  if (!text) return "?";
  return text[0].toUpperCase();
}

function mentionEntryKey(entry: ChatMentionEntry): string {
  const agentId = String(entry.agentId || "").trim();
  const departmentId = String(entry.departmentId || "").trim();
  return departmentId ? `${agentId}:${departmentId}` : agentId;
}

function mentionEntryTitle(entry: ChatMentionEntry): string {
  const lines = [
    `人格：${entry.agentName}`,
    `部门：${entry.departmentName}`,
  ];
  const reason = String(entry.unavailableReason || "").trim();
  if (reason) lines.push(`不可用：${reason}`);
  return lines.join("\n");
}

function personaChipClass(entry: ChatMentionEntry & { selected?: boolean }): string {
  const selected = !!entry.selected;
  const muted = frontSpeakingMuted(entry);
  if (selected) {
    return "border-primary/60 bg-primary/10 hover:border-primary hover:bg-primary/15";
  }
  if (muted) {
    return "border-base-300/70 bg-base-200/70 hover:border-base-300 hover:bg-base-200";
  }
  if (!entry.mentionable) {
    return "border-base-300/70 bg-base-200/70 text-base-content/55 hover:border-base-300 hover:bg-base-200";
  }
  return "border-base-300/70 bg-base-100/70 hover:border-base-300 hover:bg-base-200";
}

function frontSpeakingMuted(entry: ChatMentionEntry): boolean {
  return props.selectedMentionKeys.length > 0 && entry.isFrontSpeaking;
}

onMounted(() => {
  updateMenuPlacement();
  updateToolbarPersonaLayout();
  if (typeof ResizeObserver !== "undefined" && toolbarRootRef.value) {
    toolbarResizeObserver = new ResizeObserver(() => updateToolbarPersonaLayout());
    toolbarResizeObserver.observe(toolbarRootRef.value);
  }
  window.addEventListener("resize", updateMenuPlacement);
  window.addEventListener("resize", updateToolbarPersonaLayout);
  window.addEventListener("scroll", updateMenuPlacement, true);
  window.addEventListener("click", handleAvatarClickOutside, true);
});

watch(
  () => [
    uniqueMentionEntries.value.length,
    props.selectedMentionKeys.join("|"),
    props.mentionEntries.length,
  ],
  () => {
    nextTick(() => updateToolbarPersonaLayout());
  },
);

onBeforeUnmount(() => {
  window.removeEventListener("resize", updateMenuPlacement);
  window.removeEventListener("resize", updateToolbarPersonaLayout);
  window.removeEventListener("scroll", updateMenuPlacement, true);
  window.removeEventListener("click", handleAvatarClickOutside, true);
  toolbarResizeObserver?.disconnect();
  toolbarResizeObserver = null;
});
</script>

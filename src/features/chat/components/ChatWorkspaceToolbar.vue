<template>
  <div
    v-bind="attrs"
    class="rounded-box bg-base-100/70 px-2 py-1.5 shadow backdrop-blur-md flex items-center justify-between gap-2 text-xs"
    @contextmenu.prevent.stop="openFileTagsContextMenu"
  >
    <div class="flex min-w-0 flex-1 items-center gap-1.5">
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
          <li v-if="showCodeReviewMenuItem">
            <button type="button" class="flex min-h-10 items-center justify-start gap-3 px-4 py-2 text-left" @click="emit('openCodeReview')">
              <ClipboardCheck class="h-4 w-4 shrink-0" />
              <span class="leading-5">{{ t('chat.toolbar.codeReview') }}</span>
            </button>
          </li>
          <li>
            <button v-if="showTaskCreateMenuItem" type="button" class="flex min-h-10 items-center justify-start gap-3 px-4 py-2 text-left" @click="emit('openTaskCreate')">
              <ListTodo class="h-4 w-4 shrink-0" />
              <span class="leading-5">{{ t("chat.newTask") }}</span>
            </button>
          </li>
          <li>
            <button v-if="showDelegateMenuItem" type="button" class="flex min-h-10 items-center justify-start gap-3 px-4 py-2 text-left" @click="emit('openDelegateSelection')">
              <ClipboardList class="h-4 w-4 shrink-0" />
              <span class="leading-5">{{ t("chat.conversationMenu.startDelegate") }}</span>
            </button>
          </li>
          <li>
            <button v-if="showBranchMenuItem" type="button" class="flex min-h-10 items-center justify-start gap-3 px-4 py-2 text-left" @click="emit('openBranchSelection')">
              <Split class="h-4 w-4 shrink-0" />
              <span class="leading-5">{{ t("chat.conversationMenu.branchConversation") }}</span>
            </button>
          </li>
          <li>
            <button v-if="showAutoPushMenuItem" type="button" class="flex min-h-10 items-center justify-start gap-3 px-4 py-2 text-left" @click="emit('openAutoPush')">
              <Send class="h-4 w-4 shrink-0" />
              <span class="leading-5">{{ t("chat.conversationMenu.autoPush") }}</span>
            </button>
          </li>
          <li>
            <button v-if="showForwardMenuItem" type="button" class="flex min-h-10 items-center justify-start gap-3 px-4 py-2 text-left" @click="emit('openForwardSelection')">
              <Package class="h-4 w-4 shrink-0" />
              <span class="leading-5">{{ t("chat.conversationMenu.forwardConversation") }}</span>
            </button>
          </li>
          <li>
            <button v-if="showShareMenuItem" type="button" class="flex min-h-10 items-center justify-start gap-3 px-4 py-2 text-left" @click="emit('openShareSelection')">
              <ExternalLink class="h-4 w-4 shrink-0" />
              <span class="leading-5">{{ t("chat.conversationMenu.shareConversation") }}</span>
            </button>
          </li>
          <li v-if="showWorkspaceMenuItem && !workspaceButtonDisabled">
            <button type="button" class="flex min-h-10 items-center justify-start gap-3 px-4 py-2 text-left" :disabled="workspaceButtonDisabled" @click="emit('lockWorkspace')">
              <Folder class="h-4 w-4 shrink-0" />
              <span class="leading-5">{{ t("chat.conversationMenu.setWorkspace") }}</span>
            </button>
          </li>
          <li v-if="showOpenInBrowserButton && !openInBrowserDisabled">
            <button
              type="button"
              class="flex min-h-10 items-center justify-start gap-3 px-4 py-2 text-left"
              :disabled="openInBrowserDisabled"
              @click="emit('openConversationInBrowser')"
            >
              <ExternalLink class="h-4 w-4 shrink-0" />
              <span class="leading-5">{{ t("chat.conversationMenu.openInBrowser") }}</span>
            </button>
          </li>
        </ul>
      </div>
      <SessionControlPanel
        v-if="showSessionControlPanel"
        class="min-w-0 flex-1"
        :workspace-button-label="workspaceButtonLabel"
        :workspace-button-name="workspaceButtonName"
        :workspace-button-disabled="workspaceButtonDisabled"
        :workspace-permission-kind="workspacePermissionKind"
        :auto-push-active="autoPushActive"
        :delegates="delegateStatuses || []"
        @lock-workspace="emit('lockWorkspace')"
        @open-delegate-summary="emit('openDelegateSummary')"
      />
    </div>
    <div class="flex min-w-0 items-center justify-end gap-1.5">
      <button
        v-if="uniqueMentionEntries.length > 0"
        ref="mentionListButtonRef"
        type="button"
        class="btn btn-ghost btn-sm btn-circle shrink-0 border-0 bg-transparent shadow-none hover:bg-base-200"
        :title="t('chat.toolbar.personaList')"
        @click="toggleMentionListPopup"
      >
        <span class="text-base font-semibold leading-none">@</span>
      </button>
    </div>
  </div>
  <Teleport to="body">
    <ul
      v-if="fileTagsContextMenu"
      ref="fileTagsContextMenuRef"
      class="menu fixed z-[1200] w-64 rounded-box border border-base-300 bg-base-100 p-2 text-base-content shadow-xl"
      :style="{ left: `${fileTagsContextMenu.x}px`, top: `${fileTagsContextMenu.y}px` }"
      @contextmenu.prevent.stop
    >
      <li>
        <label class="flex cursor-pointer items-center justify-between gap-3 px-2 py-2">
          <span class="text-sm">{{ t("appearance.inputPanelIdeBridgeFileTags") }}</span>
          <input
            :checked="ideBridgeFileTagsEnabled"
            type="checkbox"
            class="toggle toggle-sm"
            @change="setIdeBridgeFileTagsEnabled(($event.target as HTMLInputElement).checked)"
          />
        </label>
      </li>
      <li v-if="SIDE_FILE_TAGS_AVAILABLE">
        <label class="flex cursor-pointer items-center justify-between gap-3 px-2 py-2">
          <span class="text-sm">{{ t("appearance.inputPanelSideFileTags") }}</span>
          <input
            :checked="sideFileTagsEnabled"
            type="checkbox"
            class="toggle toggle-sm"
            @change="setSideFileTagsEnabled(($event.target as HTMLInputElement).checked)"
          />
        </label>
      </li>
    </ul>
  </Teleport>
  <Teleport to="body">
    <div
      v-if="mentionListPopupOpen"
      ref="mentionListPopupRef"
      class="fixed z-1200"
      :style="mentionListPopupStyle"
    >
      <div class="relative overflow-hidden rounded-box border border-base-300 bg-base-100 shadow-xl">
        <div
          ref="mentionListScrollRef"
          class="ecall-toolbar-mention-scroll max-h-[min(56vh,24rem)] min-w-56 max-w-[min(80vw,20rem)] overflow-y-auto overscroll-contain p-1"
        >
          <ul class="flex flex-col gap-1">
            <li
              v-for="entry in uniqueMentionEntries"
              :key="entry.agentId"
            >
              <button
                type="button"
                class="flex min-h-0 w-full items-center gap-2 rounded-xl px-2 py-1.5 text-left text-base-content transition-colors hover:bg-base-200/80"
                :disabled="chatting || frozen"
                @click="handleCompactPersonaEntryClick($event, entry)"
              >
                <div class="indicator shrink-0">
                  <span
                    v-if="entry.selected"
                    class="indicator-item indicator-top indicator-end inline-flex h-4 w-4 translate-x-1/4 -translate-y-1/4 items-center justify-center rounded-full bg-primary text-micro font-bold text-primary-content"
                  >
                    @
                  </span>
                  <span
                    v-else-if="entry.hasBackgroundTask"
                    class="indicator-item indicator-bottom indicator-end inline-flex min-w-5 translate-x-1/4 translate-y-1/4 items-center justify-center rounded-full border border-base-300 bg-base-100 px-1 py-0.5 text-micro text-base-content shadow-sm"
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
                        class="w-7 h-7 rounded-full flex items-center justify-center text-caption"
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
                  <div class="truncate text-xs leading-4 text-base-content/60">
                    {{ entry.departmentName || t("chat.defaultDepartment") }}
                  </div>
                </div>
              </button>
            </li>
          </ul>
        </div>
        <FloatingScrollbar ref="mentionListScrollbarRef" :target="mentionListScrollRef" />
      </div>
    </div>
  </Teleport>
  <Teleport to="body">
    <div
      v-if="avatarPopupTarget"
      class="fixed z-1200"
      :style="avatarPopupStyle"
    >
      <div class="relative overflow-hidden rounded-box border border-base-300 bg-base-100 shadow-xl">
        <div
          ref="avatarPopupPanelRef"
          class="ecall-toolbar-mention-scroll max-h-[min(56vh,24rem)] w-max max-w-[min(80vw,20rem)] overflow-y-auto overscroll-contain p-1"
        >
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
                  <div v-else class="bg-neutral text-neutral-content w-7 h-7 rounded-full flex items-center justify-center text-caption">
                    {{ avatarInitial(entry.agentName) }}
                  </div>
                </div>
              </div>
              <div class="min-w-0 flex-1 pr-0.5">
                <div class="truncate text-sm leading-5">@{{ entry.agentName }}</div>
                <div class="truncate text-xs leading-4 text-base-content/60">
                  {{ entry.departmentName || t("chat.defaultDepartment") }}
                </div>
              </div>
            </button>
          </li>
        </ul>
        </div>
        <FloatingScrollbar ref="avatarPopupScrollbarRef" :target="avatarPopupPanelRef" />
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, useAttrs } from "vue";
import { useI18n } from "vue-i18n";
import { ClipboardCheck, ClipboardList, ExternalLink, Folder, Grip, ListTodo, Package, Send, Split } from "@lucide/vue";
import type { ChatMentionEntry, ConversationDelegateStatusSummary } from "../../../types/app";
import FloatingScrollbar from "../../shell/components/FloatingScrollbar.vue";
import { SIDE_FILE_TAGS_AVAILABLE, useChatComposerAppearance } from "../../shell/composables/use-chat-composer-appearance";
import SessionControlPanel from "./SessionControlPanel.vue";

defineOptions({
  inheritAttrs: false,
});

const props = withDefaults(defineProps<{
  chatting: boolean;
  frozen: boolean;
  conversationBusy?: boolean;
  workspaceButtonLabel: string;
  workspaceButtonName: string;
  workspaceButtonDisabled?: boolean;
  workspacePermissionKind?: "read_only" | "approval" | "full_access" | "autonomous";
  autoPushActive?: boolean;
  mentionEntries: ChatMentionEntry[];
  selectedMentionKeys: string[];
  hideMenuButton?: boolean;
  hideWorkspaceButton?: boolean;
  showTaskCreateMenuItem?: boolean;
  showDelegateMenuItem?: boolean;
  showBranchMenuItem?: boolean;
  showCodeReviewMenuItem?: boolean;
  showForwardMenuItem?: boolean;
  showAutoPushMenuItem?: boolean;
  showShareMenuItem?: boolean;
  showWorkspaceMenuItem?: boolean;
  showOpenInBrowserButton?: boolean;
  openInBrowserDisabled?: boolean;
  delegateStatuses?: ConversationDelegateStatusSummary[];
}>(), {
  showTaskCreateMenuItem: true,
  showDelegateMenuItem: true,
  showBranchMenuItem: true,
  showCodeReviewMenuItem: true,
  showForwardMenuItem: true,
  showAutoPushMenuItem: true,
  showShareMenuItem: true,
  showWorkspaceMenuItem: true,
  showOpenInBrowserButton: false,
});

const emit = defineEmits<{
  (e: "lockWorkspace"): void;
  (e: "openBranchSelection"): void;
  (e: "openCodeReview"): void;
  (e: "openTaskCreate"): void;
  (e: "openDelegateSelection"): void;
  (e: "openDelegateSummary"): void;
  (e: "openForwardSelection"): void;
  (e: "openAutoPush"): void;
  (e: "openShareSelection"): void;
  (e: "openConversationInBrowser"): void;
  (e: "mentionEntry", entry: ChatMentionEntry): void;
}>();

const attrs = useAttrs();
const { t } = useI18n();
const {
  sideFileTagsEnabled,
  ideBridgeFileTagsEnabled,
  setSideFileTagsEnabled,
  setIdeBridgeFileTagsEnabled,
} = useChatComposerAppearance();
const busy = computed(() => props.chatting || props.frozen || !!props.conversationBusy);
const showTaskCreateMenuItem = computed(() => props.showTaskCreateMenuItem);
const showDelegateMenuItem = computed(() => props.showDelegateMenuItem);
const showBranchMenuItem = computed(() => props.showBranchMenuItem);
const showCodeReviewMenuItem = computed(() => props.showCodeReviewMenuItem);
const showForwardMenuItem = computed(() => props.showForwardMenuItem);
const showAutoPushMenuItem = computed(() => props.showAutoPushMenuItem);
const showShareMenuItem = computed(() => props.showShareMenuItem);
const showWorkspaceMenuItem = computed(() => props.showWorkspaceMenuItem);
const hasDelegateStatuses = computed(() => (props.delegateStatuses || []).length > 0);
const showSessionControlPanel = computed(() => !props.hideWorkspaceButton || hasDelegateStatuses.value);
const POPUP_OFFSET = 8;
const POPUP_VIEWPORT_PADDING = 8;
const fileTagsContextMenu = ref<{ x: number; y: number } | null>(null);
const fileTagsContextMenuRef = ref<HTMLElement | null>(null);

// ========== 头像栏去重 + 部门弹出 ==========

const mentionListButtonRef = ref<HTMLButtonElement | null>(null);
const mentionListPopupOpen = ref(false);
const mentionListPopupRef = ref<HTMLElement | null>(null);
const mentionListScrollRef = ref<HTMLElement | null>(null);
const mentionListScrollbarRef = ref<InstanceType<typeof FloatingScrollbar> | null>(null);
const mentionListPopupStyle = ref<Record<string, string>>({
  left: "0px",
  top: "0px",
});

const uniqueMentionEntries = computed(() => {
  const seen = new Map<string, ChatMentionEntry>();
  for (const entry of props.mentionEntries || []) {
    if (!entry.mentionable) continue;
    const agentId = String(entry.agentId || "").trim();
    if (!agentId) continue;
    if (!seen.has(agentId)) {
      seen.set(agentId, { ...entry, selected: false });
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
const avatarPopupAnchorEl = ref<HTMLElement | null>(null);
const avatarPopupPanelRef = ref<HTMLElement | null>(null);
const avatarPopupScrollbarRef = ref<InstanceType<typeof FloatingScrollbar> | null>(null);

const avatarPopupStyle = ref<Record<string, string>>({
  left: "0px",
  top: "0px",
});

const filteredAvatarPopupOptions = computed(() => {
  const target = avatarPopupTarget.value;
  if (!target) return [];
  return (props.mentionEntries || [])
    .filter((entry) => String(entry.agentId || "").trim() === target.agentId && entry.mentionable)
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
  const deptEntries = (props.mentionEntries || []).filter(
    (e) => String(e.agentId || "").trim() === agentId && e.mentionable,
  );
  if (deptEntries.length <= 1) {
    mentionListPopupOpen.value = false;
    emit('mentionEntry', deptEntries[0] || entry);
    return;
  }
  mentionListPopupOpen.value = false;
  avatarPopupTarget.value = { agentId: entry.agentId, agentName: entry.agentName, avatarUrl: entry.avatarUrl };
  const el = event.currentTarget as HTMLElement | null;
  if (el) {
    avatarPopupAnchorEl.value = el;
    void updateAvatarPopupPlacement(el.getBoundingClientRect());
  }
}

function clampPopupPosition(anchorRect: DOMRect, panelEl: HTMLElement | null, options?: {
  preferredWidth?: number;
  alignRight?: boolean;
}) {
  const viewportWidth = window.innerWidth || document.documentElement.clientWidth || 0;
  const viewportHeight = window.innerHeight || document.documentElement.clientHeight || 0;
  const measuredWidth = Math.round(panelEl?.offsetWidth || options?.preferredWidth || 0);
  const measuredHeight = Math.round(panelEl?.offsetHeight || 0);
  const spaceAbove = Math.max(0, anchorRect.top - POPUP_VIEWPORT_PADDING - POPUP_OFFSET);
  const spaceBelow = Math.max(0, viewportHeight - anchorRect.bottom - POPUP_VIEWPORT_PADDING - POPUP_OFFSET);
  const openUpward = spaceAbove >= measuredHeight || spaceAbove > spaceBelow;
  const maxLeft = Math.max(
    POPUP_VIEWPORT_PADDING,
    viewportWidth - measuredWidth - POPUP_VIEWPORT_PADDING,
  );
  const preferredLeft = options?.alignRight
    ? Math.round(anchorRect.right - measuredWidth)
    : Math.round(anchorRect.left);
  const left = Math.min(
    Math.max(POPUP_VIEWPORT_PADDING, preferredLeft),
    maxLeft,
  );
  const top = openUpward
    ? Math.max(POPUP_VIEWPORT_PADDING, Math.round(anchorRect.top) - measuredHeight - POPUP_OFFSET)
    : Math.min(
      Math.round(anchorRect.bottom) + POPUP_OFFSET,
      Math.max(POPUP_VIEWPORT_PADDING, viewportHeight - measuredHeight - POPUP_VIEWPORT_PADDING),
    );
  return {
    left: `${left}px`,
    top: `${top}px`,
  };
}

async function updateMentionListPopupPlacement(anchorRect?: DOMRect) {
  const rect = anchorRect || mentionListButtonRef.value?.getBoundingClientRect();
  if (!rect) return;
  await nextTick();
  mentionListPopupStyle.value = clampPopupPosition(rect, mentionListPopupRef.value, {
    preferredWidth: 320,
    alignRight: true,
  });
  mentionListScrollbarRef.value?.updateThumb();
}

async function updateAvatarPopupPlacement(anchorRect?: DOMRect) {
  const rect = anchorRect || avatarPopupAnchorEl.value?.getBoundingClientRect();
  if (!rect) return;
  await nextTick();
  avatarPopupStyle.value = clampPopupPosition(rect, avatarPopupPanelRef.value, {
    preferredWidth: 320,
    alignRight: true,
  });
  avatarPopupScrollbarRef.value?.updateThumb();
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
  avatarPopupAnchorEl.value = null;
}

function closeMentionListPopup() {
  mentionListPopupOpen.value = false;
}

function openFileTagsContextMenu(event: MouseEvent) {
  const menuWidth = 256;
  const menuHeight = 108;
  fileTagsContextMenu.value = {
    x: Math.max(8, Math.min(event.clientX, window.innerWidth - menuWidth - 8)),
    y: Math.max(8, Math.min(event.clientY, window.innerHeight - menuHeight - 8)),
  };
}

function closeFileTagsContextMenu() {
  fileTagsContextMenu.value = null;
}

function handleFileTagsContextMenuKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") closeFileTagsContextMenu();
}

function toggleMentionListPopup() {
  if (busy.value) return;
  mentionListPopupOpen.value = !mentionListPopupOpen.value;
  if (mentionListPopupOpen.value) {
    closeAvatarPopup();
    void updateMentionListPopupPlacement();
  }
}

function handleAvatarClickOutside(event: MouseEvent) {
  const target = event.target as HTMLElement | null;
  if (!target) {
    closeAvatarPopup();
    closeMentionListPopup();
    return;
  }
  if (
    mentionListPopupOpen.value
    && !mentionListButtonRef.value?.contains(target)
    && !mentionListPopupRef.value?.contains(target)
  ) {
    closeMentionListPopup();
  }
  if (
    fileTagsContextMenu.value
    && !fileTagsContextMenuRef.value?.contains(target)
  ) {
    closeFileTagsContextMenu();
  }
  if (
    avatarPopupTarget.value
    && !avatarPopupPanelRef.value?.contains(target)
  ) {
    closeAvatarPopup();
  }
}

function handleMentionPopupViewportChange() {
  if (!mentionListPopupOpen.value) return;
  void updateMentionListPopupPlacement();
}

function handleAvatarPopupViewportChange() {
  if (!avatarPopupTarget.value) return;
  void updateAvatarPopupPlacement();
}

const menuButtonRef = ref<HTMLButtonElement | null>(null);
const menuPlacement = ref<"top" | "bottom">("top");

function updateMenuPlacement() {
  const rect = menuButtonRef.value?.getBoundingClientRect();
  if (!rect) return;
  menuPlacement.value = rect.top >= window.innerHeight / 2 ? "top" : "bottom";
}

function avatarInitial(name: string): string {
  const text = (name || "").trim();
  if (!text) return "?";
  return text[0].toUpperCase();
}

function frontSpeakingMuted(entry: ChatMentionEntry): boolean {
  return props.selectedMentionKeys.length > 0 && entry.isFrontSpeaking;
}

onMounted(() => {
  updateMenuPlacement();
  window.addEventListener("resize", updateMenuPlacement);
  window.addEventListener("scroll", updateMenuPlacement, true);
  window.addEventListener("resize", handleMentionPopupViewportChange);
  window.addEventListener("scroll", handleMentionPopupViewportChange, true);
  window.addEventListener("resize", handleAvatarPopupViewportChange);
  window.addEventListener("scroll", handleAvatarPopupViewportChange, true);
  window.addEventListener("click", handleAvatarClickOutside, true);
  window.addEventListener("keydown", handleFileTagsContextMenuKeydown);
});

onBeforeUnmount(() => {
  window.removeEventListener("resize", updateMenuPlacement);
  window.removeEventListener("scroll", updateMenuPlacement, true);
  window.removeEventListener("resize", handleMentionPopupViewportChange);
  window.removeEventListener("scroll", handleMentionPopupViewportChange, true);
  window.removeEventListener("resize", handleAvatarPopupViewportChange);
  window.removeEventListener("scroll", handleAvatarPopupViewportChange, true);
  window.removeEventListener("click", handleAvatarClickOutside, true);
  window.removeEventListener("keydown", handleFileTagsContextMenuKeydown);
});
</script>

<style scoped>
.ecall-toolbar-mention-scroll {
  scrollbar-gutter: auto;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.ecall-toolbar-mention-scroll::-webkit-scrollbar {
  width: 0;
  height: 0;
}
</style>

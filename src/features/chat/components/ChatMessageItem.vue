<template>
  <div
    :data-message-id="String(block.id || '')"
    :data-message-role="isOwnMessage(block) ? 'user' : block.role"
    :data-active-turn-user="activeTurnUser ? 'true' : undefined"
    :class="[
      'chat group/user-turn rounded-2xl px-3 transition-colors',
      shouldAnimateEnter(block) ? 'ecall-message-enter' : '',
      isOwnMessage(block) ? 'chat-end' : 'chat-start',
      isOwnMessage(block) && !selectionModeEnabled && !bubbleBackgroundHidden ? 'ecall-user-bubble-shift' : '',
      !isOwnMessage(block) && !selectionModeEnabled
        ? (bubbleBackgroundHidden ? 'ecall-message-simple' : 'ecall-bubble-shift')
        : '',
      isOwnMessage(block) && compactWithPrevious ? 'ecall-message-continued' : '',
      selectionModeEnabled && selected ? 'ecall-message-selected bg-neutral/10 ring-1 ring-neutral/20 shadow-sm' : '',
    ]"
    @click="handleSelectionRowClick"
    @contextmenu.prevent="openContextMenu($event)"
  >
    <div
      v-if="selectionModeEnabled && isOwnMessage(block)"
      class="flex w-5 min-w-5 items-start justify-center self-stretch pt-1"
    >
      <button
        type="button"
        data-selection-ignore="true"
        class="inline-flex h-4 w-4 items-center justify-center rounded-sm border transition-colors"
        :class="selected
          ? 'border-primary bg-primary text-primary-content'
          : 'border-base-300 bg-base-100 text-transparent hover:border-primary/60'"
        :title="selected ? t('chat.messageItem.cancelSelect') : t('chat.messageItem.selectMessage')"
        @click.stop="emit('toggleMessageSelected', selectionKey)"
      >
        <span class="text-[10px] leading-none">✓</span>
      </button>
    </div>
    <div class="chat-image self-start ecall-chat-avatar-col" data-message-avatar-anchor="true">
      <div
        class="flex w-7 flex-col items-center gap-2"
        :class="compactWithPrevious ? 'invisible pointer-events-none' : ''"
        :aria-hidden="compactWithPrevious ? 'true' : undefined"
      >
        <div class="avatar">
          <div class="w-7 rounded-full">
            <img
              v-if="avatarUrl"
              :src="avatarUrl"
              :alt="displayName"
              class="w-7 h-7 rounded-full object-cover"
            />
            <div v-else class="bg-neutral text-neutral-content w-7 h-7 rounded-full flex items-center justify-center text-xs">
              {{ avatarInitial(displayName) }}
            </div>
          </div>
        </div>
        <button
          v-if="selectionModeEnabled && !isOwnMessage(block)"
          type="button"
          data-selection-ignore="true"
          class="inline-flex h-4 w-4 items-center justify-center rounded-sm border transition-colors"
          :class="selected
            ? 'border-primary bg-primary text-primary-content'
            : 'border-base-300 bg-base-100 text-transparent hover:border-primary/60'"
          :title="selected ? t('chat.messageItem.cancelSelect') : t('chat.messageItem.selectMessage')"
          @click.stop="emit('toggleMessageSelected', selectionKey)"
        >
          <span class="text-[10px] leading-none">✓</span>
        </button>
      </div>
    </div>
    <template v-if="!isOwnMessage(block)">
      <div class="ecall-message-stack min-w-0 flex flex-col self-stretch">
        <div
          :class="[
            'ecall-message-content min-w-0',
            blockNeedsWideBubble(block) ? 'ecall-message-content-wide' : '',
          ]"
        >
          <div class="chat-header mb-1 flex items-baseline gap-2">
            <template v-if="streamingHeaderStatus">
              <span
                class="text-xs font-semibold text-base-content opacity-80 ecall-shimmer-text ecall-reasoning-shimmer"
                :data-shimmer-text="streamingHeaderStatus"
              >{{ streamingHeaderStatus }}</span>
            </template>
            <template v-else>
              <span class="text-xs text-base-content opacity-80">{{ displayName }}</span>
              <span class="text-xs leading-none">
                <time v-if="formattedCreatedAt && !block.isStreaming" class="text-base-content/40 leading-none">{{ formattedCreatedAt }}</time>
              </span>
            </template>
          </div>
          <div
            v-if="!showAssistantPreStreamingDots(block)"
            :class="[
              'chat-bubble',
              'self-start text-base-content assistant-markdown ecall-assistant-bubble max-w-full',
              bubbleBackgroundHidden ? 'ecall-message-bubble-bg-hidden' : 'bg-base-100 border border-base-300/70',
              blockNeedsWideBubble(block) ? 'ecall-assistant-bubble-wide' : '',
            ]"
          >
            <div
              v-if="showActivityPanel(block)"
              v-memo="activityPanelMemoKey(block)"
              class="flex flex-col opacity-90"
            >
              <details
                ref="activityDetailsRef"
                class="collapse rounded-none min-w-55"
                :open="activityPanelOpen(block)"
                @toggle="onActivityToggle"
              >
                <summary class="collapse-title py-1 px-1 min-h-0 text-xs font-semibold flex items-center gap-1.5 text-base-content/80 hover:bg-base-200">
                  <span
                    v-if="activityIsBusy(block)"
                    class="inline-flex h-4 w-4 shrink-0 items-center justify-center text-success"
                  >
                    <span
                      class="loading loading-spinner h-4 w-4 text-success"
                    ></span>
                  </span>
                  <span v-else class="inline-block h-2 w-2 rounded-full bg-success"></span>
                  <span class="flex min-w-0 flex-1 items-baseline gap-1 font-medium">
                    <span class="shrink-0">
                      {{ `${activityStatusText(block)}${activityReasoningCountLabel(block)}` }}
                    </span>
                    <span
                      v-if="activityToolCountsLabel(block)"
                      v-memo="[activityToolCountsLabel(block)]"
                      class="min-w-0 truncate text-base-content/55"
                    >
                      {{ `· ${activityToolCountsLabel(block)}` }}
                    </span>
                  </span>
                </summary>
                <div
                  v-show="activityPanelOpen(block)"
                  class="collapse-content px-0 pb-1 pt-2 text-xs text-base-content/70"
                  @click="collapseDetailsFromContentClick"
                >
                  <div class="flex flex-col">
                    <details
                      v-for="item in block.activityItems"
                      :key="`${block.id}-activity-${activityItemKey(item)}`"
                      class="collapse rounded-none border-l border-base-content/15 pl-2"
                      :open="activityItemOpen(block, item)"
                      @toggle="onActivityItemToggle(item, $event)"
                    >
                      <summary class="collapse-title flex min-h-0 items-center gap-1.5 px-1 py-1 text-xs hover:bg-base-200">
                        <span
                          v-if="item.kind === 'tool' && item.status === 'doing'"
                          class="loading loading-spinner loading-xs shrink-0 text-primary"
                        ></span>
                        <span
                          v-else
                          class="inline-flex w-3 shrink-0 items-center justify-center font-mono text-xs leading-none"
                          :class="activityItemMarkerClass(item)"
                        >{{ activityItemMarker(item) }}</span>
                        <span
                          class="min-w-0 flex-1 truncate"
                          :class="activityItemTitleClass(item)"
                        >
                          {{ activityItemTitle(item) }}
                        </span>
                      </summary>
                      <div
                        v-show="activityItemOpen(block, item)"
                        class="collapse-content px-1 pb-2 pt-1"
                      >
                        <div
                          v-if="item.kind === 'reasoning' || item.kind === 'content'"
                          class="whitespace-pre-wrap wrap-break-word text-xs leading-relaxed text-base-content/70"
                        >{{ item.kind === 'content' ? stripToolcallMarkers(item.text) : item.text }}</div>
                        <pre
                          v-else-if="activityToolArgsText(item)"
                          class="m-0 max-h-72 overflow-auto whitespace-pre-wrap break-all rounded bg-base-200/60 p-2 text-xs leading-relaxed text-base-content/75"
                        ><code>{{ activityToolArgsText(item) }}</code></pre>
                        <div v-else class="text-xs text-base-content/45">{{ toolTimelineText('noArgs') }}</div>
                      </div>
                    </details>
                  </div>
                </div>
              </details>
            </div>
      <div v-if="block.text">
        <div
          v-if="isOwnMessage(block)"
          class="whitespace-pre-wrap break-all"
          style="overflow-wrap: anywhere;"
        >{{ block.text }}</div>
        <div
          v-else-if="forcePlainMarkdownRender"
          @click="emit('assistantLinkClick', $event)"
        >
          <SidebarLightMarkdown :text="formatThinkAsMarkdown(block.text)" />
        </div>
        <div v-else ref="markdownContainerRef">
          <AppMarkdownRenderer
            class="ecall-markdown-content max-w-none"
            :text="formatThinkAsMarkdown(block.text)"
            :is-dark="markdownIsDark"
            :streaming="!!block.isStreaming"
            :local-image-base-path="currentWorkspaceRootPath"
            :toolcall-preview-map="toolcallPreviewMap"
            @click="emit('assistantLinkClick', $event)"
          />
        </div>
      </div>
      <div v-if="block.planCard" class="space-y-3" :class="block.text ? 'mt-3' : ''">
        <div class="text-xs italic opacity-60 mb-1">{{ t("chat.plan.sidebarHint") }}</div>
        <div @click="emit('assistantLinkClick', $event)">
          <a :href="block.planCard.path" class="link link-primary text-sm" :title="block.planCard.path">{{ t("chat.plan.linkLabel") }}{{ block.planCard.path.split(/[/\\]/).filter(Boolean).pop() }}</a>
        </div>
        <div v-if="block.providerMeta?.planCard && block.planCard.action === 'present'" class="space-y-2">
          <button
            type="button"
            class="ecall-plan-confirm-action btn btn-sm btn-primary"
            :disabled="chatting || busy || frozen || !canConfirmPlan"
            @click="emit('confirmPlan', { messageId: block.sourceMessageId || block.id })"
          >
            {{ t("chat.plan.confirmAction") }}
          </button>
          <div class="text-xs opacity-60">{{ t("chat.plan.confirmHint") }}</div>
        </div>
      </div>
      <div v-if="block.images.length > 0" :class="block.taskTrigger || block.text ? 'mt-2 grid gap-1' : 'grid gap-1'">
        <template v-for="(img, idx) in block.images" :key="`${block.id}-img-${idx}`">
          <img
            v-if="isImageMime(img.mime) && resolvedImageSrc(img, idx)"
            :src="resolvedImageSrc(img, idx)"
            loading="lazy"
            decoding="async"
            class="rounded max-h-28 object-contain bg-base-100/40 cursor-zoom-in"
            @click.stop="openResolvedImagePreview(img, idx)"
          />
          <div
            v-else-if="isImageMime(img.mime)"
            class="flex h-28 w-28 items-center justify-center rounded bg-base-200/70 text-[11px] text-base-content/55"
          >
            <span class="loading loading-spinner loading-xs mr-2"></span>
            <span>{{ t('chat.messageItem.imageLoading') }}</span>
          </div>
          <div v-else-if="isPdfMime(img.mime)" class="badge badge-ghost gap-1 py-3 w-fit">
            <FileText class="h-3.5 w-3.5" />
            <span class="text-[11px]">PDF</span>
          </div>
        </template>
      </div>
      <div v-if="block.audios.length > 0" :class="block.taskTrigger || block.text || block.images.length > 0 ? 'mt-2 flex flex-col gap-1' : 'flex flex-col gap-1'">
        <button
          v-for="(aud, idx) in block.audios"
          :key="`${block.id}-aud-${idx}`"
          class="btn btn-sm bg-base-100/70 w-fit"
          @click="emit('toggleAudioPlayback', { id: `${block.id}-aud-${idx}`, audio: aud })"
        >
          <Pause v-if="playingAudioId === `${block.id}-aud-${idx}`" class="h-3 w-3" />
          <Play v-else class="h-3 w-3" />
          <span>{{ t("chat.voice", { index: idx + 1 }) }}</span>
        </button>
      </div>
      <div
        v-if="block.attachmentFiles.length > 0"
        :class="block.taskTrigger || block.text || block.images.length > 0 || block.audios.length > 0 ? 'mt-2 flex flex-wrap gap-1' : 'flex flex-wrap gap-1'"
      >
        <div
          v-for="(file, idx) in block.attachmentFiles"
          :key="`${block.id}-file-${idx}`"
          class="badge badge-ghost gap-1 py-3"
          :title="file.relativePath"
        >
          <FileText class="h-3.5 w-3.5" />
          <span class="text-[11px]">{{ file.fileName }}</span>
        </div>
            </div>
          </div>
          <div
            v-else
            :class="[
              'chat-bubble self-start text-base-content assistant-markdown ecall-assistant-bubble ecall-assistant-loading-bubble max-w-full',
              bubbleBackgroundHidden ? 'ecall-message-bubble-bg-hidden' : 'bg-base-100 border border-base-300/70',
            ]"
          >
            <span class="ecall-assistant-loading-dots" aria-hidden="true">
              <span></span>
              <span></span>
              <span></span>
            </span>
            <span class="sr-only">{{ streamingHeaderStatus || t("chat.statusWaitingReply") }}</span>
          </div>
          <div
            :class="[
              'chat-footer ecall-message-footer flex h-6 items-center gap-1.5 transition-opacity',
              selectionModeEnabled
                ? 'opacity-100 pointer-events-auto'
                : showRegenerateAction && canRegenerate
                  ? 'opacity-100 pointer-events-auto'
              : (isLastUserMessage || isLastAssistantMessage)
                ? 'opacity-100 pointer-events-auto'
                : !block.isStreaming
                    ? 'opacity-0 pointer-events-none group-hover/user-turn:opacity-100 group-hover/user-turn:pointer-events-auto'
                    : 'opacity-0 pointer-events-none',
            ]"
          >
            <button
              v-if="!selectionModeEnabled"
              type="button"
              class="ecall-message-footer-action inline-flex h-6 min-w-6 items-center justify-center rounded px-1 text-[11px] text-base-content/55 hover:text-base-content"
              :title="t('chat.messageItem.multiSelect')"
              :class="!block.isStreaming ? '' : 'opacity-0 pointer-events-none'"
              :disabled="block.isStreaming"
              @click="emit('enterSelectionMode', selectionKey)"
            >
              <CircleCheckBig class="h-3.5 w-3.5" />
            </button>
            <button
              type="button"
              class="ecall-message-footer-action inline-flex h-6 w-6 items-center justify-center rounded text-base-content/55 hover:text-base-content"
              :title="t('chat.copy')"
              :class="!selectionModeEnabled && !block.isStreaming ? '' : 'opacity-0 pointer-events-none'"
              :disabled="selectionModeEnabled || block.isStreaming"
              @click="emit('copyMessage', block)"
            >
              <Copy class="h-3.5 w-3.5" />
            </button>
            <button
              v-if="hideToggleEnabled && !selectionModeEnabled"
              type="button"
              class="ecall-message-footer-action inline-flex h-6 w-6 items-center justify-center rounded text-base-content/55 hover:text-base-content"
              :title="bubbleBackgroundHidden ? t('chat.messageItem.showBubble') : t('chat.messageItem.hideBubble')"
              :disabled="block.isStreaming"
              @click="emit('toggleBubbleBackground', selectionKey)"
            >
              <EyeOff v-if="bubbleBackgroundHidden" class="h-3.5 w-3.5" />
              <Eye v-else class="h-3.5 w-3.5" />
            </button>
            <button
              v-if="canRecallBlock(block)"
              type="button"
              class="ecall-message-footer-action inline-flex h-6 w-6 items-center justify-center rounded text-base-content/55 hover:text-base-content"
              :title="t('chat.messageItem.branchFromMessage')"
              :class="!selectionModeEnabled && !block.isStreaming ? '' : 'opacity-0 pointer-events-none'"
              :disabled="selectionModeEnabled || block.isStreaming || busy"
              @click="emit('createConversationBranchFromTurn', { turnId: recallTurnId(block) })"
            >
              <Split class="h-3.5 w-3.5" />
            </button>
            <button
              v-if="canRecallBlock(block)"
              type="button"
              class="ecall-message-footer-action inline-flex h-6 w-6 items-center justify-center rounded text-base-content/55 hover:text-base-content"
              :title="t('chat.recall')"
              :class="!selectionModeEnabled && !block.isStreaming ? '' : 'opacity-0 pointer-events-none'"
              :disabled="selectionModeEnabled || block.isStreaming || busy"
              @click="emit('recallTurn', { turnId: recallTurnId(block) })"
            >
              <Undo2 class="h-3.5 w-3.5" />
            </button>
            <button
              v-if="showRegenerateAction"
              type="button"
              class="ecall-message-footer-action inline-flex h-6 w-6 items-center justify-center rounded text-base-content/55 hover:text-base-content"
              :title="t('chat.regenerate')"
              :class="!selectionModeEnabled && !block.isStreaming && showRegenerateAction && canRegenerate ? '' : 'opacity-0 pointer-events-none'"
              :disabled="selectionModeEnabled || block.isStreaming || !showRegenerateAction || !canRegenerate || busy"
              @click="emit('regenerateTurn', { turnId: block.sourceMessageId || block.id })"
            >
              <RotateCcw class="h-3.5 w-3.5" />
            </button>
            <span
              v-if="finalDispatchElapsedLabel(block) && !selectionModeEnabled"
              class="inline-flex h-6 shrink-0 items-center rounded px-1 text-[11px] font-medium text-base-content/60"
              :title="t('chat.messageItem.dispatchElapsed', { elapsed: finalDispatchElapsedLabel(block) })"
            >
              {{ t('chat.messageItem.elapsed', { elapsed: finalDispatchElapsedLabel(block) }) }}
            </span>
          </div>
        </div>
      </div>
    </template>
    <template v-else>
      <div v-if="!compactWithPrevious" class="chat-header mb-1 flex items-baseline gap-2">
        <span v-if="isOwnMessage(block)" class="text-xs leading-none">
          <time v-if="formattedCreatedAt && !block.isStreaming" class="text-base-content/40 leading-none">{{ formattedCreatedAt }}</time>
        </span>
        <span class="text-xs text-base-content opacity-80">{{ displayName }}</span>
        <span v-if="!isOwnMessage(block)" class="text-xs leading-none">
          <time v-if="formattedCreatedAt && !block.isStreaming" class="text-base-content/40 leading-none">{{ formattedCreatedAt }}</time>
        </span>
      </div>
      <div :class="[
        'chat-bubble',
        bubbleBackgroundHidden ? 'ecall-message-bubble-bg-hidden text-base-content' : '',
        isOwnMessage(block)
          ? 'ecall-user-bubble'
          : [
            'self-start text-base-content assistant-markdown ecall-assistant-bubble max-w-full',
            bubbleBackgroundHidden ? '' : 'bg-base-100 border border-base-300/70',
            blockNeedsWideBubble(block) ? 'ecall-assistant-bubble-wide' : '',
          ],
      ]">
        <div
          v-if="isOwnMessage(block) ? !!ownMessageDisplayText(block).trim() : !!block.text"
          class="whitespace-pre-wrap break-all"
          style="overflow-wrap: anywhere;"
        >{{ isOwnMessage(block) ? ownMessageDisplayText(block) : block.text }}</div>
        <div
          v-if="block.extraTextReferences && block.extraTextReferences.length > 0"
          :class="block.text ? 'mt-2 flex flex-wrap gap-1' : 'flex flex-wrap gap-1'"
        >
          <div
            v-for="(reference, idx) in block.extraTextReferences"
            :key="`${block.id}-extra-ref-${idx}`"
            class="badge badge-ghost gap-1 py-3"
            :title="reference.label"
          >
            <FileText class="h-3.5 w-3.5" />
            <span class="max-w-64 truncate text-[11px]">{{ reference.label }}</span>
          </div>
        </div>
        <div v-if="block.images.length > 0" :class="block.taskTrigger || block.text ? 'mt-2 grid gap-1' : 'grid gap-1'">
          <template v-for="(img, idx) in block.images" :key="`${block.id}-img-${idx}`">
            <img
              v-if="isImageMime(img.mime) && resolvedImageSrc(img, idx)"
              :src="resolvedImageSrc(img, idx)"
              loading="lazy"
              decoding="async"
              class="rounded max-h-28 object-contain bg-base-100/40 cursor-zoom-in"
              @click.stop="openResolvedImagePreview(img, idx)"
            />
            <div
              v-else-if="isImageMime(img.mime)"
              class="flex h-28 w-28 items-center justify-center rounded bg-base-200/70 text-[11px] text-base-content/55"
            >
              <span class="loading loading-spinner loading-xs mr-2"></span>
              <span>{{ t('chat.messageItem.imageLoading') }}</span>
            </div>
            <div v-else-if="isPdfMime(img.mime)" class="badge badge-ghost gap-1 py-3 w-fit">
              <FileText class="h-3.5 w-3.5" />
              <span class="text-[11px]">PDF</span>
            </div>
          </template>
        </div>
        <div v-if="block.audios.length > 0" :class="block.taskTrigger || block.text || block.images.length > 0 ? 'mt-2 flex flex-col gap-1' : 'flex flex-col gap-1'">
          <button
            v-for="(aud, idx) in block.audios"
            :key="`${block.id}-aud-${idx}`"
            class="btn btn-sm bg-base-100/70 w-fit"
            @click="emit('toggleAudioPlayback', { id: `${block.id}-aud-${idx}`, audio: aud })"
          >
            <Pause v-if="playingAudioId === `${block.id}-aud-${idx}`" class="h-3 w-3" />
            <Play v-else class="h-3 w-3" />
            <span>{{ t("chat.voice", { index: idx + 1 }) }}</span>
          </button>
        </div>
        <div
          v-if="block.attachmentFiles.length > 0"
          :class="block.taskTrigger || block.text || block.images.length > 0 || block.audios.length > 0 ? 'mt-2 flex flex-wrap gap-1' : 'flex flex-wrap gap-1'"
        >
          <div
            v-for="(file, idx) in block.attachmentFiles"
            :key="`${block.id}-file-${idx}`"
            class="badge badge-ghost gap-1 py-3"
            :title="file.relativePath"
          >
            <FileText class="h-3.5 w-3.5" />
            <span class="text-[11px]">{{ file.fileName }}</span>
          </div>
        </div>
      </div>
      <div
        v-if="isOwnMessage(block) && !compactWithPrevious"
        :class="[
          'ecall-own-message-actions flex justify-end transition-opacity',
          selectionModeEnabled
            ? 'opacity-0 pointer-events-none'
            : 'opacity-0 pointer-events-none group-hover/user-turn:opacity-100 group-hover/user-turn:pointer-events-auto',
        ]"
      >
        <button
          v-if="hideToggleEnabled"
          type="button"
          class="ecall-message-recall-action inline-flex h-5 w-5 items-center justify-center rounded text-base-content/40 hover:text-base-content"
          :title="bubbleBackgroundHidden ? t('chat.messageItem.showBubble') : t('chat.messageItem.hideBubble')"
          :disabled="selectionModeEnabled || block.isStreaming"
          @click="emit('toggleBubbleBackground', selectionKey)"
        >
          <EyeOff v-if="bubbleBackgroundHidden" class="h-3 w-3" />
          <Eye v-else class="h-3 w-3" />
        </button>
        <button
          v-if="canRecallBlock(block)"
          type="button"
          class="ecall-message-recall-action inline-flex h-5 w-5 items-center justify-center rounded text-base-content/40 hover:text-base-content"
          :title="t('chat.messageItem.branchFromMessage')"
          :disabled="selectionModeEnabled || block.isStreaming || busy"
          @click="emit('createConversationBranchFromTurn', { turnId: recallTurnId(block) })"
        >
          <Split class="h-3 w-3" />
        </button>
        <button
          v-if="canRecallBlock(block)"
          type="button"
          class="ecall-message-recall-action inline-flex h-5 w-5 items-center justify-center rounded text-base-content/40 hover:text-base-content"
          :title="t('chat.recall')"
          :disabled="selectionModeEnabled || block.isStreaming || busy"
          @click="emit('recallTurn', { turnId: recallTurnId(block) })"
        >
          <Undo2 class="h-3 w-3" />
        </button>
      </div>
    </template>
  </div>

  <Teleport to="body">
    <ul
      v-if="contextMenuOpen"
      ref="contextMenuRef"
      tabindex="0"
      class="menu fixed z-[1200] w-44 rounded-box border border-base-300 bg-base-100 p-1 shadow-xl"
      :style="{ left: contextMenuX + 'px', top: contextMenuY + 'px' }"
      @click.stop
      @mousedown.stop
      @keydown.esc.prevent.stop="closeContextMenu"
    >
      <li>
        <button type="button" @click="handleContextMenuAction('select')">
          <ListCheck class="h-4 w-4" />
          <span>{{ t('chat.messageItem.multiSelect') }}</span>
        </button>
      </li>
      <li>
        <button type="button" @click="handleContextMenuAction('copy')">
          <Copy class="h-4 w-4" />
          <span>{{ t('common.copy') }}</span>
        </button>
      </li>
      <li>
        <button type="button" @click="handleContextMenuAction('toggleBubble')">
          <EyeOff v-if="bubbleBackgroundHidden" class="h-4 w-4" />
          <Eye v-else class="h-4 w-4" />
          <span>{{ bubbleBackgroundHidden ? t('chat.messageItem.showBubble') : t('chat.messageItem.hideBubble') }}</span>
        </button>
      </li>
      <li v-if="canRecallBlock(block)">
        <button type="button" @click="handleContextMenuAction('branchFromMessage')">
          <Split class="h-4 w-4" />
          <span>{{ t('chat.messageItem.branchFromMessage') }}</span>
        </button>
      </li>
      <li v-if="canRecallBlock(block)">
        <button type="button" class="text-error" @click="handleContextMenuAction('recall')">
          <Undo2 class="h-4 w-4" />
          <span>{{ t('chat.recall') }}</span>
        </button>
      </li>
    </ul>
  </Teleport>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch, watchEffect, watchPostEffect } from "vue";
import { useI18n } from "vue-i18n";
import { CircleCheckBig, Copy, Eye, EyeOff, FileText, ListCheck, Pause, Play, RotateCcw, Split, Undo2 } from "@lucide/vue";
import { invokeTauri } from "../../../services/tauri-api";
import type { ChatActivityItem, ChatMessageBlock } from "../../../types/app";
import { formatIsoToLocalHourMinute } from "../../../utils/time";
import { AppMarkdownRenderer, initKatex } from "../markdown";
import { normalizeLocalLinkHref } from "../utils/local-link";
import { textContentSignature } from "../utils/text-signature";
import { buildToolcallPreviewMap } from "../utils/toolcall-preview";
import SidebarLightMarkdown from "./SidebarLightMarkdown.vue";

initKatex();

const imageDataUrlCache = new Map<string, string>();
const imageDataUrlPromiseCache = new Map<string, Promise<string>>();
const debugPlainMarkdownRender = typeof window !== "undefined"
  && window.localStorage.getItem("easy-call.debug.chat-plain-markdown") === "1";

const props = defineProps<{
  activeConversationId: string;
  block: ChatMessageBlock;
  selectionKey: string;
  selectionModeEnabled: boolean;
  selected: boolean;
  chatting: boolean;
  busy: boolean;
  frozen: boolean;
  userAlias: string;
  userAvatarUrl: string;
  personaNameMap: Record<string, string>;
  personaAvatarUrlMap: Record<string, string>;
  departmentNameMap?: Record<string, string>;
  markdownIsDark: boolean;
  playingAudioId: string;
  activeTurnUser: boolean;
  compactWithPrevious: boolean;
  canRegenerate: boolean;
  canConfirmPlan: boolean;
  readPlanFileContent?: (input: { conversationId: string; path: string }) => Promise<string>;
  currentWorkspaceRootPath?: string;
  bubbleBackgroundHidden: boolean;
  hideToggleEnabled: boolean;
  disableMarkdownRender?: boolean;
  disableRecallAndBranchActions?: boolean;
  isLastUserMessage?: boolean;
  isLastAssistantMessage?: boolean;
}>();

const emit = defineEmits<{
  (e: "enterSelectionMode", selectionKey: string): void;
  (e: "toggleMessageSelected", selectionKey: string): void;
  (e: "recallTurn", payload: { turnId: string }): void;
  (e: "createConversationBranchFromTurn", payload: { turnId: string }): void;
  (e: "regenerateTurn", payload: { turnId: string }): void;
  (e: "confirmPlan", payload: { messageId: string }): void;
  (e: "copyMessage", block: ChatMessageBlock): void;
  (e: "openImagePreview", image: { mime: string; bytesBase64?: string; dataUrl?: string; localPath?: string }): void;
  (e: "toggleAudioPlayback", payload: { id: string; audio: { mime: string; bytesBase64: string } }): void;
  (e: "assistantLinkClick", event: MouseEvent): void;
  (e: "toggleBubbleBackground", selectionKey: string): void;
}>();

const showRegenerateAction = false;

const { t } = useI18n();
const resolvedImageSrcMap = ref<Record<string, string>>({});
const markdownContainerRef = ref<HTMLElement | null>(null);
const activityDetailsRef = ref<HTMLDetailsElement | null>(null);
const activityExpanded = ref(false);
const expandedActivityItemIds = ref<Record<string, boolean>>({});
const planMarkdownText = ref("");
const planMarkdownError = ref("");
const planMarkdownLoading = ref(false);
const forcePlainMarkdownRender = computed(() => !!props.disableMarkdownRender || debugPlainMarkdownRender);
let disposed = false;

const contextMenuOpen = ref(false);
const contextMenuRef = ref<HTMLElement | null>(null);
const contextMenuX = ref(0);
const contextMenuY = ref(0);

watch(
  () => ({
    conversationId: String(props.activeConversationId || "").trim(),
    action: String(props.block.planCard?.action || "").trim(),
    path: String(props.block.planCard?.path || "").trim(),
    blockId: String(props.block.id || "").trim(),
  }),
  async (snapshot, _previous, onCleanup) => {
    let cancelled = false;
    onCleanup(() => {
      cancelled = true;
    });
    planMarkdownText.value = "";
    planMarkdownError.value = "";
    planMarkdownLoading.value = false;
    if (snapshot.action !== "present" || !snapshot.path || !snapshot.conversationId) {
      return;
    }
    planMarkdownLoading.value = true;
    try {
      const input = { conversationId: snapshot.conversationId, path: snapshot.path };
      const content = props.readPlanFileContent
        ? await props.readPlanFileContent(input)
        : await invokeTauri<string>("read_plan_file_content", input);
      if (cancelled || disposed) return;
      planMarkdownText.value = String(content || "");
    } catch (error) {
      if (cancelled || disposed) return;
      const message =
        error instanceof Error ? error.message : String(error || t('chat.messageItem.readPlanFailed'));
      planMarkdownError.value = message;
    } finally {
      if (!cancelled && !disposed) {
        planMarkdownLoading.value = false;
      }
    }
  },
  { immediate: true },
);

const displayName = computed(() => messageName(props.block));
const avatarUrl = computed(() => messageAvatarUrl(props.block));
const formattedCreatedAt = computed(() => formattedBlockTime(props.block.createdAt));
const streamingHeaderStatus = computed(() => assistantStreamingHeaderStatus(props.block));
const toolcallPreviewMap = computed<Record<string, { title: string; body: string }>>(() => {
  const previews = buildToolcallPreviewMap(props.block.activityItems, toolTimelineText("noArgs"));
  for (const item of props.block.activityItems) {
    if (item.kind !== "tool") continue;
    const toolCallId = String(item.toolCallId || "").trim();
    if (!toolCallId || !previews[toolCallId]) continue;
    previews[toolCallId].title = activityItemTitle(item);
  }
  return previews;
});

function detailsOpenFromEvent(event: Event): boolean {
  const target = event.target;
  return target instanceof HTMLDetailsElement ? target.open : false;
}

function onActivityToggle(event: Event): void {
  activityExpanded.value = detailsOpenFromEvent(event);
}

function closeActivityDetails(): void {
  const details = activityDetailsRef.value;
  if (details instanceof HTMLDetailsElement) {
    details.open = false;
  }
  activityExpanded.value = false;
}

function handleActivityOutsidePointerDown(event: PointerEvent): void {
  if (!activityExpanded.value) return;
  const details = activityDetailsRef.value;
  const target = event.target;
  if (!(details instanceof HTMLDetailsElement) || !(target instanceof Node)) return;
  if (details.contains(target)) return;
  closeActivityDetails();
}

function onActivityItemToggle(item: ChatActivityItem, event: Event): void {
  expandedActivityItemIds.value = {
    ...expandedActivityItemIds.value,
    [activityItemKey(item)]: detailsOpenFromEvent(event),
  };
}

function collapseDetailsFromContentClick(event: MouseEvent): void {
  const target = event.target;
  if (!(target instanceof HTMLElement)) return;
  if (target.closest('button, a, input, textarea, select, summary, label, [data-selection-ignore="true"]')) {
    return;
  }
  if (window.getSelection()?.toString()) {
    return;
  }
  const details = target.closest("details");
  if (details instanceof HTMLDetailsElement) {
    details.open = false;
  }
}

function avatarInitial(name: string): string {
  const text = (name || "").trim();
  if (!text) return "?";
  return text[0].toUpperCase();
}

function messageName(block: ChatMessageBlock): string {
  if (block.remoteImOrigin) {
    return block.remoteImOrigin.senderName || block.remoteImOrigin.remoteContactName || "IM";
  }
  const id = String(block.speakerAgentId || "").trim();
  if (id && props.personaNameMap[id]) return props.personaNameMap[id];
  if (!id || id === "user-persona") return props.userAlias || t("archives.roleUser");
  return id;
}

function messageAvatarUrl(block: ChatMessageBlock): string {
  if (block.remoteImOrigin) return "";
  const id = String(block.speakerAgentId || "").trim();
  if (id && props.personaAvatarUrlMap[id]) return props.personaAvatarUrlMap[id];
  if (!id || id === "user-persona") return props.userAvatarUrl || "";
  return "";
}

function isOwnMessage(block: ChatMessageBlock): boolean {
  if (block.remoteImOrigin) return false;
  const id = String(block.speakerAgentId || "").trim();
  return !id || id === "user-persona";
}

function recallTurnId(block: ChatMessageBlock): string {
  return String(block.sourceMessageId || block.id || "").trim();
}

function canRecallBlock(block: ChatMessageBlock): boolean {
  if (props.disableRecallAndBranchActions) return false;
  if (block.remoteImOrigin) return false;
  if (block.isStreaming) return false;
  if (String(block.role || "").trim().toLowerCase() === "system") return false;
  if (String(block.speakerAgentId || "").trim() === "system-persona") return false;
  return !!recallTurnId(block);
}

function ownMessageDisplayText(block: ChatMessageBlock): string {
  const mentions = Array.isArray(block.mentions) ? block.mentions : [];
  const mentionPrefix = mentions
    .map((item) => `@${String(item.agentName || "").trim()}`)
    .filter((item) => item !== "@")
    .join(",");
  const body = String(block.text || "");
  if (!mentionPrefix) return body;
  if (!body.trim()) return mentionPrefix;
  return `${mentionPrefix} ${body}`;
}

function showStreamingUi(block: ChatMessageBlock): boolean {
  return !!block.isStreaming && !isOwnMessage(block);
}

function normalizedStreamingPhaseLabel(block: ChatMessageBlock): string {
  const providerMeta = (block.providerMeta || {}) as Record<string, unknown>;
  const preStreamingStatusText = String(providerMeta._preStreamingStatusText || "").trim();
  const toolStatusText = String(providerMeta._toolStatusText || "").trim();
  const toolStatusState = String(providerMeta._toolStatusState || "").trim();
  const hasSpeechContent = hasStreamingSpeechContent(block);
  const doingTool = toolCallsForBlock(block).some((call) => call.status === "doing");
  const hasReasoning = block.activityStatus === "thinking" || block.activityReasoningCharCount > 0;

  const normalizeRequestPhaseText = (text: string): string => {
    if (!text) return "";
    if (text.includes("准备调度") || text.includes("处理附件") || text.includes("上下文")) {
      return t("chat.statusPreparingMessage");
    }
    if (
      text.includes("等待回应")
      || text.includes("进入模型请求阶段")
      || text.includes("重新开始当前调度")
      || text.includes("重新发起")
      || text.includes("调度")
      || text.includes("模型请求")
    ) {
      return t("chat.statusWaitingReply");
    }
    return "";
  };

  if (toolStatusState === "running") {
    const requestPhase = normalizeRequestPhaseText(toolStatusText);
    if (requestPhase) return requestPhase;
  }
  if (preStreamingStatusText) {
    const requestPhase = normalizeRequestPhaseText(preStreamingStatusText);
    if (requestPhase) return requestPhase;
  }
  if (hasSpeechContent) {
    return t("chat.statusTypingBody");
  }
  if (doingTool || block.activityStatus === "running_tool") {
    return t("chat.statusGeneratingTools");
  }
  if (hasReasoning) {
    return t("chat.statusThinking");
  }
  return t("chat.statusWaitingReply");
}

function assistantStreamingHeaderStatus(block: ChatMessageBlock): string {
  if (!showStreamingUi(block)) return "";
  const withElapsed = (text: string): string => {
    const elapsed = frontendDispatchElapsedLabel(block);
    return elapsed ? `${text}（${elapsed}）` : text;
  };
  return withElapsed(normalizedStreamingPhaseLabel(block));
}

function showAssistantPreStreamingDots(block: ChatMessageBlock): boolean {
  if (!showStreamingUi(block)) return false;
  const providerMeta = (block.providerMeta || {}) as Record<string, unknown>;
  const preStreamingStatusText = String(providerMeta._preStreamingStatusText || "").trim();
  if (!preStreamingStatusText) return false;
  return !hasStreamingSpeechContent(block)
    && toolCallsForBlock(block).length === 0
    && !showActivityPanel(block)
    && block.images.length === 0
    && block.audios.length === 0
    && block.attachmentFiles.length === 0;
}

function hasStreamingSpeechContent(block: ChatMessageBlock): boolean {
  if (String(block.text || "").trim()) return true;
  if (Array.isArray(block.streamSegments) && block.streamSegments.some((item) => String(item || "").trim())) return true;
  if (String(block.streamTail || "").trim()) return true;
  if (String(block.streamAnimatedDelta || "").trim()) return true;
  return false;
}

function shouldAnimateEnter(block: ChatMessageBlock): boolean {
  if (!isOwnMessage(block)) return false;
  const providerMeta = (block.providerMeta || {}) as Record<string, unknown>;
  return !!providerMeta._optimistic;
}

function toolCallsForBlock(block: ChatMessageBlock): Array<{ name: string; argsText: string; status?: "doing" | "done" }> {
  return block.toolCalls;
}

function showActivityPanel(block: ChatMessageBlock): boolean {
  if (isOwnMessage(block)) return false;
  return block.activityItems.length > 0 || !!block.activityRunning;
}

function activityShouldAutoExpand(block: ChatMessageBlock): boolean {
  void block;
  return false;
}

function activityPanelOpen(block: ChatMessageBlock): boolean {
  return activityExpanded.value || activityShouldAutoExpand(block);
}

function activityIsBusy(block: ChatMessageBlock): boolean {
  if (!block.activityRunning) return false;
  return block.activityStatus === "requesting"
    || block.activityStatus === "thinking"
    || block.activityStatus === "running_tool";
}

function activityReasoningCountLabel(block: ChatMessageBlock): string {
  const count = Number(block.activityReasoningCharCount || 0);
  return count > 0 ? `(${count.toLocaleString("zh-CN")})` : "";
}

function activityStatusText(block: ChatMessageBlock): string {
  if (block.activityStatus === "running_tool") return t('chat.messageItem.runningTool');
  if (block.activityStatus === "thinking") return t('chat.messageItem.thinking');
  if (block.activityStatus === "requesting") return t('chat.messageItem.requesting');
  return t('chat.messageItem.thinkingAndTools');
}

function activityToolCountsLabel(block: ChatMessageBlock): string {
  const counts = new Map<string, number>();
  const order: string[] = [];
  for (const item of block.activityItems) {
    if (item.kind !== "tool") continue;
    const name = toolCallDisplayName(item.name);
    if (!counts.has(name)) {
      counts.set(name, 0);
      order.push(name);
    }
    counts.set(name, (counts.get(name) || 0) + 1);
  }
  return order
    .map((name) => {
      const total = counts.get(name) || 0;
      return total > 1 ? `${name}(${total})` : name;
    })
    .join(" · ");
}

function activityItemsSignature(block: ChatMessageBlock): string {
  return block.activityItems
    .map((item) => {
      if (item.kind === "reasoning") {
        return [
          "r",
          String(item.id || "").trim(),
          textContentSignature(item.text),
          item.running ? "1" : "0",
        ].join(":");
      }
      if (item.kind === "content") {
        return [
          "c",
          String(item.id || "").trim(),
          textContentSignature(item.text),
          item.running ? "1" : "0",
        ].join(":");
      }
      return [
        "t",
        String(item.id || "").trim(),
        String(item.toolCallId || "").trim(),
        String(item.name || "").trim(),
        String(item.status || "").trim(),
        textContentSignature(item.argsText),
        textContentSignature(item.resultText),
      ].join(":");
    })
    .join("|");
}

function activityExpandedItemsSignature(block: ChatMessageBlock): string {
  return block.activityItems
    .map((item) => `${activityItemKey(item)}:${activityItemOpen(block, item) ? "1" : "0"}`)
    .join("|");
}

function activityPanelMemoKey(block: ChatMessageBlock): unknown[] {
  return [
    String(block.id || "").trim(),
    showActivityPanel(block),
    activityExpanded.value,
    activityPanelOpen(block),
    activityIsBusy(block),
    activityStatusText(block),
    activityReasoningCountLabel(block),
    activityToolCountsLabel(block),
    activityItemsSignature(block),
    activityExpandedItemsSignature(block),
  ];
}

function activityItemKey(item: ChatActivityItem): string {
  return `${item.kind}:${String(item.id || "")}`;
}

function isActivityItemExpanded(item: ChatActivityItem): boolean {
  return !!expandedActivityItemIds.value[activityItemKey(item)];
}

function activityItemOpen(block: ChatMessageBlock, item: ChatActivityItem): boolean {
  return activityShouldAutoExpand(block) || isActivityItemExpanded(item);
}

function activityReasoningPreview(text: string): string {
  return compactText(String(text || ""), 120);
}

function stripToolcallMarkers(text: string): string {
  return String(text || "").replace(/\s*\[toolcall:[^\]\n]+\]/g, "").trim();
}

function activityToolArgsText(item: ChatActivityItem): string {
  if (item.kind !== "tool") return "";
  return String(item.argsText || "").trim();
}

function activityItemMarker(item: ChatActivityItem): string {
  if (item.kind === "reasoning") return "+";
  if (item.kind === "content") return "=";
  return "*";
}

function activityItemMarkerClass(item: ChatActivityItem): string {
  if (item.kind === "reasoning") return "font-semibold text-warning";
  if (item.kind === "content") return "font-semibold text-info/70";
  return "font-semibold text-base-content/45";
}

function activityItemTitleClass(item: ChatActivityItem): string {
  if (item.kind === "reasoning") return "font-semibold italic text-warning";
  if (item.kind === "content") return "text-base-content/70";
  return "text-base-content/50";
}

function activityItemTitle(item: ChatActivityItem): string {
  if (item.kind === "reasoning") {
    return activityReasoningPreview(item.text);
  }
  if (item.kind === "content") {
    return compactText(stripToolcallMarkers(item.text), 120);
  }
  return joinNonEmpty([
    toolCallDisplayName(item.name),
    toolCallSummaryText(item),
  ]);
}

function toolStatusLabel(block: ChatMessageBlock): string {
  if (!showStreamingUi(block)) return t('chat.messageItem.toolDone');
  return toolSummaryDoing(block) ? t('chat.messageItem.toolRunning') : t('chat.messageItem.toolDone');
}

function toolSummaryDoing(block: ChatMessageBlock): boolean {
  if (!showStreamingUi(block)) return false;
  return toolCallsForBlock(block).some((call) => String(call.status || "").trim() === "doing");
}

const internalToolNames = new Set<string>([
  "apply_patch",
  "exec",
  "shell_exec",
  "read",
  "read_file",
  "write_file",
  "append_text",
  "delete_file",
  "create_file",
  "rename_file",
  "move_file",
  "list_dir",
  "read_dir",
  "find",
  "search",
  "todo",
  "plan",
  "create_goal",
  "update_goal",
  "task",
  "delegate",
  "remember",
  "recall",
  "fetch",
  "websearch",
  "operate",
  "screenshot",
  "wait",
  "akasha_search",
  "akasha_read",
  "akasha_catalog",
  "akasha_link",
  "tavily_search",
  "tavily_extract",
  "tavily_crawl",
  "tavily_map",
  "tavily_research",
]);

const compactListKeys = new Set<string>([
  "todos",
  "files",
  "urls",
  "queries",
  "lineRanges",
  "tags",
]);

const ignorableSummaryKeys = new Set<string>([
  "status",
  "reasoning",
  "background",
  "why",
  "max_length",
  "maxResults",
  "max_results",
  "tokens",
  "timeout_ms",
  "quality",
  "exact_match",
  "include_raw_content",
  "include_images",
  "include_image_descriptions",
  "include_favicon",
  "format",
  "topic",
  "country",
  "search_depth",
  "extract_depth",
]);

function normalizeToolCallArgs(argsText: string): unknown {
  const text = String(argsText || "").trim();
  if (!text) return undefined;
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

function safeTextFromRecord(data: Record<string, unknown>, keys: string[]): string {
  for (const key of keys) {
    const value = data[key];
    if (typeof value === "string") {
      const trimmed = value.trim();
      if (trimmed) return trimmed;
    }
    if (Array.isArray(value)) {
      const joined = value
        .map((item) => (typeof item === "string" ? item.trim() : ""))
        .filter(Boolean)
        .join(" ");
      if (joined) return joined;
    }
  }
  return "";
}

function compactText(text: string, maxLen = 180): string {
  const trimmed = text.replace(/\s+/g, " ").trim();
  if (trimmed.length <= maxLen) return trimmed;
  return `${trimmed.slice(0, maxLen - 3)}...`;
}

function joinNonEmpty(parts: string[], separator = " · "): string {
  return parts.map((part) => part.trim()).filter(Boolean).join(separator);
}

function safeStringValue(data: Record<string, unknown>, key: string): string {
  const value = data[key];
  return typeof value === "string" ? value.trim() : "";
}

function toolTimelineText(key: string, params?: Record<string, string | number>): string {
  return String(t(`status.toolTimeline.${key}`, params ?? {}));
}

function toolTimelineNameValue(name: string, value: string): string {
  return `${name}：${value}`;
}

function taskTriggerSummary(value: unknown): string {
  if (typeof value !== "object" || value === null) return "";
  const obj = value as Record<string, unknown>;
  return joinNonEmpty([
    safeStringValue(obj, "run_at") || safeStringValue(obj, "runAt") || safeStringValue(obj, "runAtLocal"),
    safeStringValue(obj, "cron_expression")
      ? toolTimelineNameValue("cron", safeStringValue(obj, "cron_expression"))
      : (safeStringValue(obj, "cronExpression")
        ? toolTimelineNameValue("cron", safeStringValue(obj, "cronExpression"))
        : (safeStringValue(obj, "every_minutes")
          ? toolTimelineNameValue("everyMinutes", safeStringValue(obj, "every_minutes"))
          : (safeStringValue(obj, "everyMinutes")
            ? toolTimelineNameValue("everyMinutes", safeStringValue(obj, "everyMinutes"))
            : ""))),
    safeStringValue(obj, "end_at")
      ? toolTimelineText("until", { time: safeStringValue(obj, "end_at") })
      : (safeStringValue(obj, "endAt")
        ? toolTimelineText("until", { time: safeStringValue(obj, "endAt") })
        : (safeStringValue(obj, "endAtLocal")
          ? toolTimelineText("until", { time: safeStringValue(obj, "endAtLocal") })
          : "")),
  ]);
}

function compactObjectEntries(data: Record<string, unknown>, maxItems = 3): string {
  return Object.entries(data)
    .filter(([key, value]) => !ignorableSummaryKeys.has(key) && value !== undefined && value !== null && value !== "")
    .map(([key, value]) => {
      if (compactListKeys.has(key) && Array.isArray(value)) {
        return `${value.length} ${key}`;
      }
      const text = toCompactValue(value, 1);
      return text ? `${key}: ${text}` : "";
    })
    .filter(Boolean)
    .slice(0, maxItems)
    .join(" · ");
}

function toSingleLineJsonText(payload: unknown): string {
  if (payload === undefined || payload === null) return "";
  if (typeof payload === "string") return payload.trim() || "";
  try {
    return JSON.stringify(payload);
  } catch {
    return String(payload);
  }
}

function compactSingleLineJson(payload: unknown, maxLen = 180): string {
  const text = toSingleLineJsonText(payload);
  if (!text) return "";
  const oneLine = text.replace(/\s+/g, " ").trim();
  if (oneLine.length <= maxLen) return oneLine;
  return `${oneLine.slice(0, maxLen - 3)}...`;
}

function toCompactValue(value: unknown, depth = 0): string {
  if (value === undefined || value === null) return "";
  if (typeof value === "string") return value.trim();
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (depth > 1) return "";

  if (Array.isArray(value)) {
    const parts = value
      .map((item) => toCompactValue(item, depth + 1))
      .filter((item) => item !== "")
      .slice(0, 3);
    return parts.join(" | ");
  }

  if (typeof value === "object") {
    const obj = value as Record<string, unknown>;
    const orderedKeys = [
      "path",
      "file",
      "target",
      "source",
      "destination",
      "from",
      "to",
      "command",
      "cmd",
      "url",
      "query",
      "name",
      "id",
      "text",
      "content",
      "input",
      "output",
      "method",
    ];

    for (const key of orderedKeys) {
      const valueText = toCompactValue(obj[key], depth + 1);
      if (valueText) return `${key}: ${valueText}`;
    }

    const pairs = Object.entries(obj)
      .map(([key, rawValue]) => {
        const compactValue = toCompactValue(rawValue, depth + 1);
        return compactValue ? `${key}: ${compactValue}` : "";
      })
      .filter(Boolean)
      .slice(0, 2);
    if (pairs.length > 0) {
      return pairs.join("；");
    }
  }

  return "";
}

function applyPatchOperationLabel(operation: string): string {
  if (operation === "add") return toolTimelineText("patchAdd");
  if (operation === "delete") return toolTimelineText("patchDelete");
  if (operation === "move") return toolTimelineText("patchMove");
  return toolTimelineText("patchUpdate");
}

function summarizeApplyPatchInput(input: string): string {
  const lines = input.split(/\r?\n/);
  const entries: Array<{ operation: string; path: string }> = [];
  let pendingUpdatePath = "";

  for (const line of lines) {
    const addMatch = line.match(/^\*\*\* Add File:\s+(.+)$/);
    if (addMatch?.[1]) {
      entries.push({ operation: "add", path: addMatch[1].trim() });
      pendingUpdatePath = "";
      continue;
    }

    const deleteMatch = line.match(/^\*\*\* Delete File:\s+(.+)$/);
    if (deleteMatch?.[1]) {
      entries.push({ operation: "delete", path: deleteMatch[1].trim() });
      pendingUpdatePath = "";
      continue;
    }

    const updateMatch = line.match(/^\*\*\* Update File:\s+(.+)$/);
    if (updateMatch?.[1]) {
      pendingUpdatePath = updateMatch[1].trim();
      entries.push({ operation: "update", path: pendingUpdatePath });
      continue;
    }

    const moveMatch = line.match(/^\*\*\* Move to:\s+(.+)$/);
    if (moveMatch?.[1] && pendingUpdatePath) {
      const last = entries[entries.length - 1];
      if (last && last.path === pendingUpdatePath) {
        last.operation = "move";
        last.path = `${pendingUpdatePath} → ${moveMatch[1].trim()}`;
      }
      pendingUpdatePath = "";
    }
  }

  if (entries.length === 0) return toolTimelineText("inlinePatch");
  return entries
    .slice(0, 5)
    .map((entry) => `${applyPatchOperationLabel(entry.operation)} ${entry.path}`)
    .join("，");
}

function summarizeApplyPatchTool(args: unknown): string {
  const argsText = toSingleLineJsonText(args);
  if (!argsText) return toolTimelineText("checkChanges");

  if (typeof args === "string") {
    if (!args.trim()) return toolTimelineText("checkChanges");
    return summarizeApplyPatchInput(args);
  }

  if (typeof args === "object" && args !== null) {
    const obj = args as Record<string, unknown>;
    const input = typeof obj.input === "string" ? obj.input.trim() : "";
    if (input) return summarizeApplyPatchInput(input);

    const patch = (typeof obj.patch === "string" ? obj.patch : typeof obj.diff === "string" ? obj.diff : "").trim();

    const fileFromArgs = safeTextFromRecord(obj, ["file", "target", "path", "files", "pathnames"]);
    if (fileFromArgs) {
      return `${toolTimelineText("patchUpdate")} ${fileFromArgs}`;
    }

    if (patch) {
      const files = Array.from(new Set(
        patch
          .split(/\r?\n/)
          .map((line) => {
            const match = line.match(/^diff --git\s+(?:a\/|\S+)\s+(?:b\/|\S+)(.+)$/);
            if (match && match[1]) {
              return String(match[1]).replace(/^b\//, "").trim();
            }
            const simpleMatch = line.match(/^---\s+([ab]\/)?(.+)$/);
            if (simpleMatch && simpleMatch[2]) {
              return String(simpleMatch[2]).trim();
            }
            return "";
          })
          .filter((file) => Boolean(file) && !file.includes("/dev/null")),
      ));

      const filtered = files.filter((file) => file.length > 0);
      if (filtered.length > 0) {
        return filtered.map((file) => `${toolTimelineText("patchUpdate")} ${file}`).join("，");
      }
    }

    return compactSingleLineJson(args, 180) || toolTimelineText("checkArgs");
  }

  return toolTimelineText("patchCall");
}

function summarizeCommandTool(args: unknown): string {
  if (!args) return toolTimelineText("notProvided");
  if (typeof args === "string") return args;
  if (typeof args !== "object") return String(args);

  const obj = args as Record<string, unknown>;
  const command = safeTextFromRecord(obj, ["command", "cmd", "shell", "input", "commandText"]);
  const fallback = safeTextFromRecord(obj, ["args", "arguments", "argv", "params"]);
  if (command) return command;
  if (fallback) return fallback;
  const compact = toCompactValue(obj);
  return compact || toolTimelineText("checkArgs");
}

function summarizeFileTool(args: unknown): string {
  if (!args) return toolTimelineText("missingArgs");
  if (typeof args === "string") {
    const text = args.trim();
    return text || toolTimelineText("missingArgs");
  }
  if (typeof args !== "object") {
    return String(args);
  }
  const obj = args as Record<string, unknown>;
  const path = safeTextFromRecord(obj, ["absolute_path", "absolutePath", "path", "file", "target", "source", "destination", "from", "to"]);
  return path || toCompactValue(obj) || toolTimelineText("missingArgs");
}

function summarizeReadFileTool(args: unknown): string {
  if (!args) return toolTimelineText("missingArgs");
  if (typeof args === "string") {
    const text = args.trim();
    return text || toolTimelineText("missingArgs");
  }
  if (typeof args !== "object") {
    return String(args);
  }
  const obj = args as Record<string, unknown>;
  const path = safeTextFromRecord(obj, ["absolute_path", "absolutePath", "path", "file"]);
  const offset = obj.offset ?? obj.start;
  const limit = obj.limit ?? obj.count;
  return joinNonEmpty([
    path,
    offset !== undefined && offset !== null ? `offset: ${String(offset)}` : "",
    limit !== undefined && limit !== null ? `limit: ${String(limit)}` : "",
  ]) || toCompactValue(obj) || toolTimelineText("missingArgs");
}

function summarizeReadMediaTool(args: unknown): string {
  if (!args || typeof args !== "object") return summarizeReadFileTool(args);
  const obj = args as Record<string, unknown>;
  const path = safeTextFromRecord(obj, ["path", "absolute_path", "absolutePath", "file"]);
  const description = safeTextFromRecord(obj, ["description", "focus", "prompt"]);
  return joinNonEmpty([path, description ? `description: ${description}` : ""]) || compactObjectEntries(obj);
}

function summarizeTodoTool(args: unknown): string {
  if (typeof args !== "object" || args === null) return compactText(toSingleLineJsonText(args) || toolTimelineText("missingArgs"));
  const todos = (args as Record<string, unknown>).todos;
  if (!Array.isArray(todos)) return toolTimelineText("missingArgs");
  const counts = todos.reduce((acc, item) => {
    const status = typeof item === "object" && item !== null ? String((item as Record<string, unknown>).status || "pending") : "pending";
    acc[status] = (acc[status] || 0) + 1;
    return acc;
  }, {} as Record<string, number>);
  const active = todos
    .map((item) => (typeof item === "object" && item !== null ? item as Record<string, unknown> : null))
    .find((item) => String(item?.status || "") === "in_progress")
    ?? (typeof todos[0] === "object" && todos[0] !== null ? todos[0] as Record<string, unknown> : null);
  const activeText = active ? compactText(String(active.content || ""), 120) : "";
  return joinNonEmpty([
    toolTimelineText("todoItems", { count: todos.length }),
    counts.in_progress ? toolTimelineText("todoInProgress", { count: counts.in_progress }) : "",
    counts.pending ? toolTimelineText("todoPending", { count: counts.pending }) : "",
    counts.completed ? toolTimelineText("todoCompleted", { count: counts.completed }) : "",
    activeText,
  ]);
}

function summarizeTaskTool(args: unknown): string {
  if (typeof args !== "object" || args === null) return compactText(toSingleLineJsonText(args) || toolTimelineText("missingArgs"));
  const obj = args as Record<string, unknown>;
  return joinNonEmpty([
    safeStringValue(obj, "action"),
    safeStringValue(obj, "goal"),
    taskTriggerSummary(obj.trigger),
  ]) || compactObjectEntries(obj);
}

function summarizeGoalTool(args: unknown): string {
  if (typeof args !== "object" || args === null) return compactText(toSingleLineJsonText(args) || toolTimelineText("missingArgs"));
  const obj = args as Record<string, unknown>;
  return joinNonEmpty([
    safeStringValue(obj, "status"),
    compactText(safeStringValue(obj, "objective"), 120),
    compactText(safeStringValue(obj, "evidence"), 120),
    compactText(safeStringValue(obj, "blocking_condition"), 120),
  ]) || compactObjectEntries(obj);
}

function summarizePlanTool(args: unknown): string {
  if (typeof args !== "object" || args === null) return compactText(toSingleLineJsonText(args) || toolTimelineText("missingArgs"));
  const obj = args as Record<string, unknown>;
  return joinNonEmpty([
    safeStringValue(obj, "action"),
    compactText(safeStringValue(obj, "context"), 160),
  ]) || compactObjectEntries(obj);
}

function delegateModeDisplayText(mode: string): string {
  const normalized = mode.trim().toLowerCase();
  if (normalized === "wait" || normalized === "sync") return "等待结果";
  if (normalized === "background" || normalized === "async") return "后台运行";
  return mode.trim();
}

function delegateDepartmentDisplayText(departmentId: string): string {
  const normalized = departmentId.trim();
  if (!normalized) return "";
  return String(props.departmentNameMap?.[normalized] || "").trim() || normalized;
}

function summarizeDelegateTool(args: unknown): string {
  if (typeof args !== "object" || args === null) return compactText(toSingleLineJsonText(args) || toolTimelineText("missingArgs"));
  const obj = args as Record<string, unknown>;
  const mode = delegateModeDisplayText(safeStringValue(obj, "mode") || "wait");
  const department = delegateDepartmentDisplayText(safeStringValue(obj, "department_id"));
  const content = compactText(
    safeStringValue(obj, "question")
      || safeStringValue(obj, "specific_goal")
      || safeStringValue(obj, "instruction")
      || safeStringValue(obj, "focus")
      || safeStringValue(obj, "background"),
    50,
  );
  return joinNonEmpty([
    safeStringValue(obj, "task_name"),
    department,
    mode,
    content,
  ]) || compactObjectEntries(obj);
}

function summarizeMemoryTool(args: unknown): string {
  if (typeof args !== "object" || args === null) return compactText(toSingleLineJsonText(args) || toolTimelineText("missingArgs"));
  const obj = args as Record<string, unknown>;
  return joinNonEmpty([
    safeStringValue(obj, "memory_type"),
    safeStringValue(obj, "judgment"),
    safeStringValue(obj, "query"),
  ]) || compactObjectEntries(obj);
}

function summarizeWebTool(args: unknown): string {
  if (typeof args !== "object" || args === null) return compactText(toSingleLineJsonText(args) || toolTimelineText("missingArgs"));
  const obj = args as Record<string, unknown>;
  return joinNonEmpty([
    safeStringValue(obj, "query"),
    safeStringValue(obj, "url"),
    Array.isArray(obj.urls) ? `${obj.urls.length} URLs` : "",
    safeStringValue(obj, "instructions"),
  ]) || compactObjectEntries(obj);
}

function summarizeAkashaTool(args: unknown): string {
  if (typeof args !== "object" || args === null) return compactText(toSingleLineJsonText(args) || toolTimelineText("missingArgs"));
  const obj = args as Record<string, unknown>;
  return joinNonEmpty([
    safeStringValue(obj, "world"),
    safeStringValue(obj, "keyword"),
    safeStringValue(obj, "documentPath"),
    safeStringValue(obj, "documentTitle"),
    Array.isArray(obj.lineRanges) ? obj.lineRanges.join("，") : "",
  ]) || compactObjectEntries(obj);
}

function summarizeOperateTool(args: unknown): string {
  if (typeof args !== "object" || args === null) return compactText(toSingleLineJsonText(args) || toolTimelineText("missingArgs"));
  const script = safeStringValue(args as Record<string, unknown>, "script");
  if (!script) return compactObjectEntries(args as Record<string, unknown>);
  const lines = script.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  return compactText(lines.slice(0, 3).join("；"), 180);
}

function summarizeBuiltinTool(toolName: string, args: unknown): string {
  if (toolName === "read" || toolName === "read_file") return summarizeReadFileTool(args);
  if (toolName === "read_media") return summarizeReadMediaTool(args);
  if (toolName === "todo") return summarizeTodoTool(args);
  if (toolName === "task") return summarizeTaskTool(args);
  if (toolName === "create_goal" || toolName === "update_goal") return summarizeGoalTool(args);
  if (toolName === "plan") return summarizePlanTool(args);
  if (toolName === "delegate") return summarizeDelegateTool(args);
  if (toolName === "remember" || toolName === "recall") return summarizeMemoryTool(args);
  if (toolName === "fetch" || toolName === "websearch" || toolName.startsWith("tavily_")) return summarizeWebTool(args);
  if (toolName.startsWith("akasha_")) return summarizeAkashaTool(args);
  if (toolName === "operate") return summarizeOperateTool(args);
  if (toolName === "screenshot") return summarizeFileTool(args);
  if (toolName === "wait") return compactObjectEntries((typeof args === "object" && args !== null ? args : { ms: args }) as Record<string, unknown>);
  return "";
}

function summarizeExternalTool(name: string, args: unknown): string {
  if (args === undefined || args === null) return toolTimelineNameValue(name, toolTimelineText("noArgs"));
  if (typeof args === "string") {
    const text = args.trim();
    return toolTimelineNameValue(name, text || toolTimelineText("missingArgs"));
  }
  if (typeof args !== "object") {
    return toolTimelineNameValue(name, String(args));
  }

  const compact = toCompactValue(args);
  if (compact) {
    return toolTimelineNameValue(name, compact);
  }

  const jsonText = compactSingleLineJson(args, 180);
  if (jsonText) return toolTimelineNameValue(name, jsonText);

  return toolTimelineNameValue(name, toolTimelineText("missingArgs"));
}

function toolCallSummaryText(toolCall: { name: string; argsText: string; status?: "doing" | "done" }): string {
  const toolName = String(toolCall.name || "").trim() || "unknown";
  const args = normalizeToolCallArgs(toolCall.argsText);

  if (internalToolNames.has(toolName)) {
    if (toolName === "read" || toolName === "read_file") {
      return summarizeReadFileTool(args);
    }
    if (toolName === "read_media") return summarizeReadMediaTool(args);
    if (toolName === "apply_patch") return summarizeApplyPatchTool(args);
    if (toolName === "exec" || toolName === "shell_exec") return summarizeCommandTool(args);
    if (toolName.includes("file")) {
      return summarizeFileTool(args);
    }
    const builtinSummary = summarizeBuiltinTool(toolName, args);
    if (builtinSummary) return builtinSummary;
    const compact = toCompactValue(args);
    return compact || toolTimelineText("missingArgs");
  }

  return summarizeExternalTool(toolCallDisplayName(toolName), args);
}

function toolCallTitle(toolCall: { name: string }, index: number): string {
  return `#${index} ${toolCallDisplayName(toolCall.name)}`;
}

function toolCallDisplayName(toolName: string): string {
  if (toolName === "shell_exec") return "exec";
  if (toolName === "read_file") return "read";
  if (toolName === "read_dir") return "read_dir";
  if (toolName === "list_dir") return "list_dir";
  return String(toolName || toolTimelineText("unknownTool")).trim() || toolTimelineText("unknownTool");
}

function toolTimelineDotClass(block: ChatMessageBlock, toolCall: { name: string; argsText: string; status?: "doing" | "done" }): string {
  if (!showStreamingUi(block)) return "bg-success";
  return toolCall.status === "doing" ? "bg-primary" : "bg-success";
}

function toolTimelineHrClass(block: ChatMessageBlock, toolCall: { name: string; argsText: string; status?: "doing" | "done" }): string {
  if (!showStreamingUi(block)) return "bg-success/35";
  return toolCall.status === "doing" ? "bg-primary/35" : "bg-success/35";
}

function toolNamesLabel(block: ChatMessageBlock): string {
  const calls = toolCallsForBlock(block);
  if (calls.length === 0) return "";
  const counts = new Map<string, number>();
  const order: string[] = [];
  for (const call of calls) {
    const name = toolCallDisplayName(String(call.name || "").trim()) || toolTimelineText("unknownTool");
    if (!counts.has(name)) {
      counts.set(name, 0);
      order.push(name);
    }
    counts.set(name, (counts.get(name) || 0) + 1);
  }
  return order
    .map((name) => {
      const total = counts.get(name) || 0;
      return total > 1 ? `${name}（+${total - 1}）` : name;
    })
    .join("，");
}

function formatDispatchElapsed(ms: number): string {
  const totalSeconds = Math.max(0, Math.round(Number(ms || 0) / 1000));
  const days = Math.floor(totalSeconds / 86400);
  const hours = Math.floor((totalSeconds % 86400) / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  const padded = (value: number) => String(value).padStart(2, "0");
  if (days > 0) return t('chat.messageItem.durationDays', { days, hours: padded(hours), minutes: padded(minutes), seconds: padded(seconds) });
  if (hours > 0) return t('chat.messageItem.durationHours', { hours: padded(hours), minutes: padded(minutes), seconds: padded(seconds) });
  if (minutes > 0) return t('chat.messageItem.durationMinutes', { minutes: padded(minutes), seconds: padded(seconds) });
  return t('chat.messageItem.durationSeconds', { seconds: padded(seconds) });
}

function numericMetaValue(block: ChatMessageBlock, key: string): number {
  const fromBlock = Number((block as ChatMessageBlock & Record<string, unknown>)[key]);
  if (Number.isFinite(fromBlock) && fromBlock > 0) return fromBlock;
  const meta = (block.providerMeta || {}) as Record<string, unknown>;
  const fromMeta = Number(meta[key]);
  return Number.isFinite(fromMeta) && fromMeta > 0 ? fromMeta : 0;
}

function frontendDispatchElapsedLabel(block: ChatMessageBlock): string {
  if (!showStreamingUi(block)) return "";
  const elapsedMs = numericMetaValue(block, "frontendDispatchElapsedMs")
    || numericMetaValue(block, "_frontendDispatchElapsedMs");
  const startedAtMs = numericMetaValue(block, "_frontendDispatchStartedAtMs");
  if (elapsedMs <= 0 && startedAtMs <= 0) return "";
  return formatDispatchElapsed(elapsedMs);
}

function finalDispatchElapsedLabel(block: ChatMessageBlock): string {
  if (block.isStreaming) return "";
  const elapsedMs = numericMetaValue(block, "dispatchElapsedMs");
  if (elapsedMs <= 0) return "";
  return formatDispatchElapsed(elapsedMs);
}

function formattedBlockTime(value?: string): string {
  return formatIsoToLocalHourMinute(value, "");
}

function handleSelectionRowClick(event: MouseEvent): void {
  if (!props.selectionModeEnabled) return;
  const target = event.target as HTMLElement | null;
  if (!target) return;
  if (target.closest('[data-selection-ignore="true"], button, a, input, textarea, select, summary, label')) {
    return;
  }
  emit("toggleMessageSelected", props.selectionKey);
}

function handleGlobalPointerDownForContextMenu(event: PointerEvent) {
  const target = event.target;
  if (!(target instanceof Node)) {
    closeContextMenu();
    return;
  }
  if (contextMenuRef.value?.contains(target)) return;
  closeContextMenu();
}

function openContextMenu(event: MouseEvent) {
  const menuWidth = 176; // w-44
  const menuHeight = 200; // estimate
  const margin = 8;
  const viewportWidth = window.innerWidth || document.documentElement.clientWidth || 0;
  const viewportHeight = window.innerHeight || document.documentElement.clientHeight || 0;
  contextMenuX.value = Math.min(Math.max(margin, event.clientX), viewportWidth - menuWidth - margin);
  contextMenuY.value = Math.min(Math.max(margin, event.clientY), viewportHeight - menuHeight - margin);
  contextMenuOpen.value = true;
  window.addEventListener("pointerdown", handleGlobalPointerDownForContextMenu, true);
}

function closeContextMenu() {
  contextMenuOpen.value = false;
  window.removeEventListener("pointerdown", handleGlobalPointerDownForContextMenu, true);
}

function handleContextMenuAction(action: string) {
  closeContextMenu();
  if (action === "select") {
    emit("enterSelectionMode", props.selectionKey);
  } else if (action === "copy") {
    emit("copyMessage", props.block);
  } else if (action === "toggleBubble") {
    emit("toggleBubbleBackground", props.selectionKey);
  } else if (action === "branchFromMessage") {
    const turnId = recallTurnId(props.block);
    if (!turnId) return;
    emit("createConversationBranchFromTurn", { turnId });
  } else if (action === "recall") {
    const turnId = recallTurnId(props.block);
    if (!turnId) return;
    emit("recallTurn", { turnId });
  }
}

function formatThinkAsMarkdown(raw: string): string {
  const input = raw || "";
  const openTag = "<think>";
  const closeTag = "</think>";
  let output = "";
  let cursor = 0;

  while (cursor < input.length) {
    const openIdx = input.indexOf(openTag, cursor);
    if (openIdx < 0) {
      output += input.slice(cursor);
      break;
    }

    output += input.slice(cursor, openIdx);
    const afterOpen = openIdx + openTag.length;
    const closeIdx = input.indexOf(closeTag, afterOpen);
    if (closeIdx < 0) {
      const tail = input.slice(afterOpen).trim();
      if (tail) output += `\n\n*${tail}*`;
      cursor = input.length;
      break;
    }

    const inner = input.slice(afterOpen, closeIdx).trim();
    if (inner) output += `\n\n*${inner}*\n\n`;
    cursor = closeIdx + closeTag.length;
  }

  return output.trim();
}

function normalizeRenderedLocalLinks() {
  const container = markdownContainerRef.value;
  if (!container) return;
  const anchors = Array.from(container.querySelectorAll("a[href]"));
  for (const anchor of anchors) {
    const rawHref = anchor.getAttribute("href")?.trim() || "";
    const normalizedHref = normalizeLocalLinkHref(rawHref);
    if (normalizedHref && normalizedHref !== rawHref) {
      anchor.setAttribute("href", normalizedHref);
    }
  }
}

function blockHasMermaid(block: ChatMessageBlock): boolean {
  return /```(?:\s*)mermaid\b/i.test(block.text);
}

function blockHasCodeFence(block: ChatMessageBlock): boolean {
  return /```[\w-]*\s*[\r\n]/i.test(block.text);
}

function blockNeedsWideBubble(block: ChatMessageBlock): boolean {
  return blockHasMermaid(block) || blockHasCodeFence(block);
}

function isImageMime(mime: string): boolean {
  return (mime || "").trim().toLowerCase().startsWith("image/");
}

function isPdfMime(mime: string): boolean {
  return (mime || "").trim().toLowerCase() === "application/pdf";
}

function imageCacheKey(image: { mime: string; bytesBase64?: string; mediaRef?: string }): string {
  const mime = String(image.mime || "").trim().toLowerCase();
  const mediaRef = String(image.mediaRef || "").trim();
  if (mediaRef) return `${mime}::${mediaRef}`;
  const bytesBase64 = String(image.bytesBase64 || "").trim();
  return `${mime}::inline::${bytesBase64}`;
}

function imageRenderKey(index: number): string {
  return `${String(props.block.id || "").trim() || "message"}::${index}`;
}

async function loadImageDataUrl(image: { mime: string; bytesBase64?: string; mediaRef?: string }): Promise<string> {
  const mime = String(image.mime || "").trim() || "image/webp";
  const bytesBase64 = String(image.bytesBase64 || "").trim();
  if (bytesBase64) {
    return `data:${mime};base64,${bytesBase64}`;
  }
  const mediaRef = String(image.mediaRef || "").trim();
  if (!mediaRef) return "";
  const cacheKey = imageCacheKey(image);
  const cached = imageDataUrlCache.get(cacheKey);
  if (cached) return cached;
  const pending = imageDataUrlPromiseCache.get(cacheKey);
  if (pending) return pending;
  const task = invokeTauri<{ dataUrl: string }>("read_chat_image_data_url", {
    input: {
      mediaRef,
      mime,
    },
  })
    .then((result) => {
      const dataUrl = String(result?.dataUrl || "").trim();
      if (dataUrl) imageDataUrlCache.set(cacheKey, dataUrl);
      imageDataUrlPromiseCache.delete(cacheKey);
      return dataUrl;
    })
    .catch((error) => {
      imageDataUrlPromiseCache.delete(cacheKey);
      throw error;
    });
  imageDataUrlPromiseCache.set(cacheKey, task);
  return task;
}

watchEffect(() => {
  const nextEntries = props.block.images
    .map((image, index) => {
      const src = image.bytesBase64
        ? `data:${image.mime};base64,${image.bytesBase64}`
        : "";
      return [imageRenderKey(index), src] as const;
    })
    .filter((entry) => !!entry[1]);
  if (nextEntries.length <= 0) return;
  resolvedImageSrcMap.value = {
    ...resolvedImageSrcMap.value,
    ...Object.fromEntries(nextEntries),
  };
});

watchEffect(() => {
  for (const [index, image] of props.block.images.entries()) {
    if (!isImageMime(image.mime) || image.bytesBase64 || !image.mediaRef) continue;
    const key = imageRenderKey(index);
    if (resolvedImageSrcMap.value[key]) continue;
    void loadImageDataUrl(image)
      .then((dataUrl) => {
        if (!dataUrl || disposed) return;
        resolvedImageSrcMap.value = {
          ...resolvedImageSrcMap.value,
          [key]: dataUrl,
        };
      })
      .catch((error) => {
        console.warn("[聊天图片] 懒加载失败", {
          messageId: props.block.id,
          mediaRef: image.mediaRef,
          error,
        });
      });
  }
});

watchPostEffect(() => {
  void nextTick(() => {
    normalizeRenderedLocalLinks();
  });
});

onMounted(() => {
  document.addEventListener("pointerdown", handleActivityOutsidePointerDown, true);
});

onBeforeUnmount(() => {
  closeContextMenu();
  disposed = true;
  document.removeEventListener("pointerdown", handleActivityOutsidePointerDown, true);
});

function resolvedImageSrc(
  image: { mime: string; bytesBase64?: string; mediaRef?: string },
  index: number,
): string {
  const direct = String(image.bytesBase64 || "").trim();
  if (direct) return `data:${image.mime};base64,${direct}`;
  return String(resolvedImageSrcMap.value[imageRenderKey(index)] || "").trim();
}

function openResolvedImagePreview(
  image: { mime: string; bytesBase64?: string; mediaRef?: string },
  index: number,
) {
  const dataUrl = resolvedImageSrc(image, index);
  if (!dataUrl) return;
  emit("openImagePreview", {
    mime: image.mime,
    dataUrl,
  });
}
</script>

<style scoped>
.ecall-chat-avatar-col {
  width: 1.75rem;
  min-width: 1.75rem;
}

.ecall-message-stack {
  min-height: 100%;
  flex: 1 1 auto;
  width: 100%;
}

.ecall-message-content {
  min-width: 0;
  flex: 0 1 auto;
}

.ecall-message-content-wide {
  width: 100%;
  max-width: none;
}

.ecall-message-simple {
  --ecall-chat-avatar-offset: calc(1.75rem + 0.75rem);
}
.ecall-bubble-shift {
  --ecall-chat-avatar-offset: calc(1.75rem + 0.75rem);
}
.ecall-user-bubble-shift {
  --ecall-chat-avatar-offset: calc(1.75rem + 0.75rem);
}

.ecall-message-simple .ecall-message-content {
  width: calc(100% + var(--ecall-chat-avatar-offset));
  max-width: calc(100% + var(--ecall-chat-avatar-offset));
  margin-left: calc(var(--ecall-chat-avatar-offset) * -1);
}

.ecall-message-simple .chat-header {
  margin-left: var(--ecall-chat-avatar-offset);
}
.ecall-message-simple .chat-bubble {
  width: 100%;
  max-width: none;
}
.ecall-bubble-shift .chat-bubble {
  width: fit-content;
  max-width: 100%;
  margin-left: calc(var(--ecall-chat-avatar-offset) * -0.3);
}
.ecall-user-bubble-shift > .ecall-user-bubble {
  width: fit-content;
  max-width: 100%;
  margin-right: calc(var(--ecall-chat-avatar-offset) * -0.3);
}

.ecall-user-bubble-shift > .ecall-own-message-actions {
  margin-right: calc(var(--ecall-chat-avatar-offset) * -0.3);
}

.ecall-message-simple .ecall-message-bubble-bg-hidden {
  padding-inline: 1rem !important;
}

.ecall-message-continued {
  padding-top: 0;
}

.ecall-meme-segment-flow {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.35rem 0.45rem;
}

.ecall-meme-text-segment {
  min-width: 0;
}

.ecall-inline-meme {
  display: inline-block;
  max-height: 4.5rem;
  max-width: min(8rem, 40vw);
  border-radius: 0.85rem;
  object-fit: contain;
  vertical-align: middle;
}

.ecall-local-image-wrapper {
  display: inline-block;
  vertical-align: middle;
}

.ecall-local-image-thumbnail {
  max-height: 18rem;
  max-width: min(28rem, 80vw);
  border-radius: 0.5rem;
  object-fit: contain;
  cursor: zoom-in;
}

.ecall-local-image-placeholder {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 6rem;
  max-width: min(16rem, 60vw);
  height: 4rem;
  border-radius: 0.5rem;
  opacity: 0.5;
  font-size: 0.85rem;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ecall-local-image-unavailable {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 4rem;
  max-width: min(12rem, 50vw);
  height: 3rem;
  border-radius: 0.5rem;
  opacity: 0.3;
  font-size: 0.75rem;
  border: 1px dashed currentColor;
  overflow: hidden;
  text-overflow: ellipsis;
}


.ecall-inline-meme-markdown:deep(.markdown-renderer),
.ecall-inline-meme-markdown:deep(.node-slot),
.ecall-inline-meme-markdown:deep(.node-content) {
  display: inline;
}

.ecall-inline-meme-markdown:deep(.paragraph-node),
.ecall-inline-meme-markdown:deep(.text-node),
.ecall-inline-meme-markdown:deep(.strong-node),
.ecall-inline-meme-markdown:deep(.emphasis-node),
.ecall-inline-meme-markdown:deep(.link-node),
.ecall-inline-meme-markdown:deep(.inline-code-node) {
  display: inline;
  margin: 0;
}

.ecall-message-enter {
  animation: ecall-message-enter 220ms cubic-bezier(0.22, 1, 0.36, 1);
}

@keyframes ecall-message-enter {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.assistant-markdown :deep(.ecall-markdown-content.prose) {
  --tw-prose-body: currentColor;
  --tw-prose-headings: currentColor;
  --tw-prose-lead: currentColor;
  --tw-prose-links: var(--color-base-content);
  --tw-prose-bold: currentColor;
  --tw-prose-counters: currentColor;
  --tw-prose-bullets: color-mix(in srgb, var(--color-base-content) 50%, transparent);
  --tw-prose-hr: color-mix(in srgb, var(--color-base-content) 15%, transparent);
  --tw-prose-quotes: currentColor;
  --tw-prose-quote-borders: color-mix(in srgb, var(--color-base-content) 20%, transparent);
  --tw-prose-captions: color-mix(in srgb, var(--color-base-content) 75%, transparent);
  --tw-prose-code: currentColor;
  --tw-prose-pre-code: currentColor;
  --tw-prose-pre-bg: var(--color-base-200);
  --tw-prose-th-borders: color-mix(in srgb, var(--color-base-content) 20%, transparent);
  --tw-prose-td-borders: color-mix(in srgb, var(--color-base-content) 15%, transparent);
}

.assistant-markdown :deep(.ecall-markdown-content) {
  --ms-font-sans: var(
    --app-font-family,
    system-ui,
    -apple-system,
    BlinkMacSystemFont,
    "Segoe UI",
    Roboto,
    "Helvetica Neue",
    Arial,
    sans-serif
  );
  --ms-text-body: 0.875rem;
  --ms-leading-body: 1.5;
  --ms-text-h1: 1.02rem;
  --ms-leading-h1: 1.5;
  --ms-text-h2: 0.98rem;
  --ms-leading-h2: 1.5;
  --ms-text-h3: 0.94rem;
  --ms-leading-h3: 1.5;
  --ms-text-h4: 0.9rem;
  --ms-text-h5: 0.875rem;
  --ms-text-h6: 0.875rem;
  --ms-flow-paragraph-y: 0.25rem;
  --ms-flow-list-y: 0.25rem;
  --ms-flow-list-item-y: 0.12rem;
  --ms-flow-list-indent: 1.05rem;
  --ms-flow-list-indent-mobile: 1.05rem;
  --ms-flow-blockquote-y: 0.25rem;
  --ms-flow-blockquote-indent: 0.68rem;
  min-width: 0;
  max-width: 100%;
  overflow-x: hidden;
  font-family: inherit;
  font-size: 0.875rem;
  line-height: 1.5;
}

.assistant-markdown :deep(.ecall-markdown-content .paragraph-node),
.assistant-markdown :deep(.ecall-markdown-content .heading-node),
.assistant-markdown :deep(.ecall-markdown-content .list-node),
.assistant-markdown :deep(.ecall-markdown-content .list-item),
.assistant-markdown :deep(.ecall-markdown-content .blockquote),
.assistant-markdown :deep(.ecall-markdown-content .link-node),
.assistant-markdown :deep(.ecall-markdown-content .strong-node),
.assistant-markdown :deep(.ecall-markdown-content .inline-code),
.assistant-markdown :deep(.ecall-markdown-content .table-node-wrapper),
.assistant-markdown :deep(.ecall-markdown-content .hr-node) {
  font-size: inherit;
  line-height: inherit;
}

.assistant-markdown :deep(.ecall-markdown-content.markdown-renderer) {
  content-visibility: visible !important;
  contain: none !important;
  contain-intrinsic-size: auto !important;
}

.assistant-markdown :deep(.ecall-markdown-content .markdown-renderer),
.assistant-markdown :deep(.ecall-markdown-content .node-slot),
.assistant-markdown :deep(.ecall-markdown-content .node-content),
.assistant-markdown :deep(.ecall-markdown-content .text-node) {
  font-size: inherit;
  line-height: inherit;
}

.assistant-markdown :deep(.ecall-markdown-content .code-block-container),
.assistant-markdown :deep(.ecall-markdown-content ._mermaid) {
  content-visibility: visible !important;
  contain: none !important;
  contain-intrinsic-size: auto !important;
}

.assistant-markdown :deep(.ecall-markdown-content > :first-child) {
  margin-top: 0;
}

.assistant-markdown :deep(.ecall-markdown-content > :last-child) {
  margin-bottom: 0;
}

.assistant-markdown :deep(.ecall-markdown-content :where(p,ul,ol,blockquote,pre,table,figure,.paragraph-node,.list-node,.blockquote,.table-node-wrapper,.code-block-container,._mermaid,.vmr-container)) {
  margin-top: 0.25rem;
  margin-bottom: 0.25rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(h1,h2,h3,h4,.heading-node)) {
  margin-top: 0.7rem;
  margin-bottom: 0.32rem;
  line-height: 1.5;
}

.assistant-markdown :deep(.ecall-markdown-content :where(h1,.heading-node.heading-1)) {
  font-size: 1.02rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(h2,.heading-node.heading-2)) {
  font-size: 0.98rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(h3,.heading-node.heading-3)) {
  font-size: 0.94rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(h4,.heading-node.heading-4)) {
  font-size: 0.9rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(ul,ol,.list-node)) {
  padding-left: 1.05rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(li,.list-item)) {
  margin: 0.12rem 0;
  padding-left: 0;
}

.assistant-markdown :deep(.ecall-markdown-content :where(li,.list-item) > :where(p,ul,ol,.paragraph-node,.list-node)) {
  margin-top: 0.16rem;
  margin-bottom: 0.16rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(blockquote,.blockquote)) {
  padding-left: 0.68rem;
}

.assistant-markdown :deep(.ecall-markdown-content :where(blockquote,.blockquote) .markdown-renderer),
.assistant-markdown :deep(.ecall-markdown-content :where(ul,ol,.list-node,li,.list-item) .markdown-renderer) {
  font-size: inherit;
  line-height: inherit;
}

.assistant-markdown :deep(.ecall-markdown-content :where(hr,.hr-node)) {
  margin: 0.65rem 0;
}

.assistant-markdown :deep(.ecall-markdown-content :where(:not(pre) > code,.inline-code):not(.code-block-container *)) {
  font-size: 0.86em;
}

.assistant-markdown :deep(.ecall-markdown-content :where(table,.table-node)) {
  font-size: 0.9rem;
}

.assistant-markdown :deep(.ecall-markdown-content ._mermaid) {
  width: 100%;
}

:deep(.chat-bubble) {
  min-width: 0;
  min-height: 0;
}

.ecall-assistant-bubble {
  min-width: 3rem;
  min-height: 2.25rem;
  font-size: 0.875rem;
  transition:
    box-shadow 220ms ease,
    transform 220ms ease,
    border-color 220ms ease,
    background-color 220ms ease;
  transform-origin: top left;
}

.ecall-assistant-bubble-wide {
  display: block;
  width: 100%;
  max-width: none;
}

.ecall-assistant-loading-bubble {
  min-width: 3.25rem;
  min-height: 2.25rem;
  display: inline-flex;
  align-items: center;
  justify-content: flex-start;
  padding-inline: 0.9rem;
}

.ecall-assistant-loading-dots {
  display: inline-flex;
  align-items: center;
  gap: 0.28rem;
}

.ecall-assistant-loading-dots span {
  width: 0.36rem;
  height: 0.36rem;
  border-radius: 999px;
  background: color-mix(in srgb, var(--color-base-content) 62%, transparent);
  animation: ecall-assistant-loading-dot 1.1s ease-in-out infinite;
}

.ecall-assistant-loading-dots span:nth-child(2) {
  animation-delay: 0.14s;
}

.ecall-assistant-loading-dots span:nth-child(3) {
  animation-delay: 0.28s;
}

@keyframes ecall-assistant-loading-dot {
  0%, 80%, 100% {
    opacity: 0.35;
    transform: translateY(0);
  }
  40% {
    opacity: 1;
    transform: translateY(-0.16rem);
  }
}
.ecall-message-bubble-bg-hidden {
  min-width: 0 !important;
  min-height: 0 !important;
  padding-inline: 0 !important;
  background-color: transparent !important;
  border-color: transparent !important;
  box-shadow: none !important;
}
</style>

<template>
  <div
    ref="chatLayoutRoot"
    class="relative flex h-full min-h-0 flex-row overflow-hidden"
  >
    <div
      v-if="showSideConversationList"
      :class="leftPaneInLayout ? 'flex h-full min-h-0 shrink-0' : 'absolute bottom-0 left-0 top-0 z-50 flex h-full min-h-0 border-r border-base-300 bg-base-100 shadow-2xl'"
      :style="{ width: `${leftPaneVisibleWidth}px` }"
    >
      <ChatConversationSidebar
        :items="conversationItems || unarchivedConversationItems"
        :active-conversation-id="activeConversationId"
        :user-alias="userAlias"
        :user-avatar-url="userAvatarUrl"
        :persona-name-map="personaNameMap"
        :persona-avatar-url-map="personaAvatarUrlMap"
        :active-tab="chatLeftPanelMode"
        :chat-model-options="chatModelOptions"
        :tool-review-api-config-id="toolReviewApiConfigId"
        :current-workspace-root-path="currentProjectWorkspaceRoot"
        @update:active-tab="$emit('update:conversation-list-tab', $event)"
        @edit-task="openTaskEditDialog"
        @select="handleConversationListSelect"
        @rename="handleConversationRename"
        @toggle-pin-conversation="handleConversationPinToggle"
        @archive-conversation="handleConversationArchive"
        @export-conversation="handleConversationExport"
        @delete-conversation="handleConversationDelete"
        @batch-archive-completed="handleBatchArchiveCompleted"
      />
    </div>

    <div class="flex min-h-0 min-w-0 flex-1 overflow-hidden">
      <div data-chat-center-pane="true" class="relative flex min-h-0 min-w-0 flex-1 flex-col">
        <div
          v-if="mediaDragActive && !chatting && !frozen && !conversationInteractionBusy"
          class="pointer-events-none absolute inset-0 z-40 flex items-center justify-center bg-base-100/70 backdrop-blur-[1px]"
        >
          <div class="rounded-box border border-primary/40 bg-base-100 px-4 py-2 text-sm font-medium text-primary">
            {{ t("chat.dropImageOrPdf") }}
          </div>
        </div>

        <div class="relative flex min-h-0 flex-1 overflow-hidden" @mouseenter="chatScrollbarRef?.reveal()" @mouseleave="chatScrollbarRef?.hide()">
          <div class="pointer-events-none absolute inset-x-0 top-0 z-20 flex justify-center px-3 pt-0">
            <ConversationTodoDropdown :todos="normalizedConversationTodos" :persona-name="personaName" />
          </div>
          <div
            v-if="showInitialMeasureOverlay"
            class="absolute inset-0 z-10 flex items-center justify-center bg-base-200"
            aria-hidden="true"
          >
            <span class="loading loading-spinner loading-md text-primary" />
          </div>
          <div
            ref="scrollContainer"
            class="ecall-chat-scroll-container relative flex flex-1 min-h-0 flex-col overflow-x-hidden overflow-y-auto px-0 py-3"
            :class="chatting || frozen || conversationInteractionBusy ? 'pointer-events-auto' : ''"
            :data-chat-interaction-locked="chatting || frozen || conversationInteractionBusy ? 'true' : undefined"
            @scroll="handleConversationScroll"
            @wheel="handleConversationWheelInput"
            @pointerdown="beginPointerScrollIntent"
          >
          <DraftRecipientCard
            v-if="activeConversationIsDraft"
            :options="props.createConversationDepartmentOptions"
            :recent-options="draftRecentRecipientOptions"
            :selected-department-id="draftSelectedDepartmentId"
            :selected-agent-id="draftSelectedAgentId"
            :avatar-url-map="props.personaAvatarUrlMap"
            :workspace-options="draftWorkspaceOptions"
            :workspace-root-path="stripExtendedPathPrefix(currentWorkspaceRootPath)"
            :workspace-access="currentWorkspaceAccess"
            :workspace-work-mode="props.currentWorkspaceWorkMode || 'directory'"
            :workspace-autonomous-mode="Boolean(props.currentWorkspaceAutonomousMode)"
            :save-workspace="props.saveDraftWorkspaces ? handleDraftWorkspaceSave : undefined"
            :git-root-check="props.draftWorkspaceGitRootCheck"
            @change="handleDraftPersonaChange($event)"
          />
          <div class="ecall-chat-history-flow flex min-w-0 shrink-0 flex-col">
            <div
              v-if="showNoMoreHistoryDivider"
              class="mx-auto flex w-full max-w-225 items-center gap-3 px-4 pb-2 pt-1 text-xs text-base-content/45"
            >
              <div class="h-px flex-1 bg-base-300/70"></div>
              <span class="shrink-0 font-semibold text-base-content/55">{{ t("chat.noMoreHistory") }}</span>
              <div class="h-px flex-1 bg-base-300/70"></div>
            </div>
            <div class="relative min-w-0 w-full shrink-0" :style="{ height: `${totalVirtualSize}px` }">
              <div
                v-for="entry in virtualEntries"
                :key="entry.item.id"
                :data-index="entry.row.index"
                :ref="measureElementRef"
                class="absolute left-0 top-0 w-full ecall-chat-virtual-item"
                :style="{ transform: `translateY(${entry.row.start}px)` }"
              >
                <div
                  v-if="entry.item.kind === 'compaction'"
                  class="mt-4 flex items-center gap-3 text-xs text-base-content/45"
                >
                  <div class="h-px flex-1 bg-base-300/80"></div>
                  <button type="button" class="btn btn-ghost btn-xs shrink-0 gap-1.5 px-2 text-base-content/60 hover:text-base-content"
                    :title="t('chat.viewSummary')" @click="openConversationSummary(entry.item.block, $event)"
                    @contextmenu.prevent.stop="openCompactionSummaryContextMenu(entry.item.block, $event)">
                    <History class="h-3.5 w-3.5" />
                    <span>{{ t("chat.viewSummary") }}</span>
                  </button>
                  <div class="h-px flex-1 bg-base-300/80"></div>
                </div>
                <div v-else-if="entry.item.kind === 'plan_started'" class="mt-4 flex items-center gap-3 text-xs text-base-content/45">
                  <div class="h-px flex-1 bg-base-300/80"></div>
                  <span class="shrink-0 rounded-full border border-base-300/80 bg-base-100 px-3 py-1 text-base-content/55">{{ t("chat.planStartedDivider") }}</span>
                  <div class="h-px flex-1 bg-base-300/80"></div>
                </div>
                <div v-else-if="entry.item.kind === 'message'"
                  v-memo="[...messageMemoKey(entry.item.block, entry.item.renderId, entry.item.blockIndex, entry.item.compactWithPrevious), departmentNameMapSignature]">
                  <div class="ecall-elastic-item-shell">
                    <ChatMessageItem
                      :active-conversation-id="activeConversationId" :block="entry.item.block"
                      :selection-key="entry.item.renderId" :selection-mode-enabled="messageSelectionModeEnabled"
                      :selected="selectedMessageRenderIdSet.has(entry.item.renderId)"
                      :chatting="chatting" :busy="conversationInteractionBusy" :frozen="frozen"
                      :user-alias="userAlias" :user-avatar-url="userAvatarUrl"
                      :persona-name-map="personaNameMap" :persona-avatar-url-map="personaAvatarUrlMap"
                      :department-name-map="departmentNameMap"
                      :markdown-is-dark="markdownIsDark"
                      :playing-audio-id="playingAudioId" :active-turn-user="false"
                      :compact-with-previous="entry.item.compactWithPrevious"
                      :can-regenerate="showConversationActions && canRegenerateBlock(entry.item.block, entry.item.blockIndex)"
                      :can-confirm-plan="canConfirmPlan(entry.item.block)"
                      :current-workspace-root-path="currentWorkspaceRootPath"
                      :current-theme="currentTheme"
                      :disable-recall-and-branch-actions="activeConversationIsSystemNotification"
                      @create-conversation-branch-from-turn="$emit('createConversationBranchFromTurn', $event)"
                      @recall-turn="$emit('recallTurn', $event)" @regenerate-turn="$emit('regenerateTurn', $event)"
                      @confirm-plan="$emit('confirmPlan', $event)" @enter-selection-mode="handleEnterMessageSelectionMode"
                      @toggle-message-selected="toggleMessageSelected"
                      @copy-message="handleCopyMessage"
                      @copy-message-image-done="handleCopyMessageImageDone"
                      @copy-message-image-failed="handleCopyMessageImageFailed"
                      @open-image-preview="openChatMessageImagePreview"
                      @toggle-audio-playback="toggleAudioPlayback($event.id, $event.audio)"
                      @assistant-link-click="handleAssistantLinkClick"
                    />
                  </div>
                </div>
              </div>
            </div>

            <div
              class="pointer-events-none overflow-hidden"
              :style="{ height: `${latestOwnTailSpacerMinHeight}px` }"
            ></div>
            <div
              v-if="supportsFloatingSessionToolbar"
              class="pointer-events-none mx-auto w-full max-w-225 shrink-0 px-4"
              :style="{ height: `${toolbarReservedHeight + 10}px` }"
            ></div>
          </div>
          </div>
          <FloatingScrollbar ref="chatScrollbarRef" :target="scrollContainer" />
        </div>
        <div
          v-if="supportsFloatingSessionToolbar"
          ref="toolbarContainer"
          class="absolute inset-x-0 z-20 transition-all duration-150 ease-out"
          :class="showFloatingSessionToolbar
            ? 'pointer-events-auto opacity-100 translate-y-0'
            : 'pointer-events-none opacity-0 translate-y-2'"
          :style="floatingToolbarStyle"
          :aria-hidden="showFloatingSessionToolbar ? undefined : 'true'"
        >
          <div class="ecall-chat-toolbar-shell mx-auto w-full max-w-225 px-4">
            <ChatWorkspaceToolbar
              class="transition-opacity duration-150 ease-out"
              :class="showFloatingSessionToolbar ? 'pointer-events-auto opacity-100' : 'pointer-events-none opacity-0'"
                  :chatting="chatting" :frozen="frozen" :conversation-busy="conversationInteractionBusy"
                  :workspace-button-label="t('chat.allowedWorkspaceButton')" :workspace-button-name="currentWorkspaceDisplayName || currentWorkspaceName"
                  :workspace-button-disabled="!activeConversationId || activeConversationSummary?.kind === 'remote_im_contact'"
                  :workspace-permission-kind="currentWorkspacePermissionKind"
                  :auto-push-active="!!String(activeConversationSummary?.autoPushRemoteContactId || '').trim()"
                  :hide-menu-button="activeConversationSummary?.kind === 'remote_im_contact'"
                  :hide-workspace-button="hideWorkspaceButton || activeConversationSummary?.kind === 'remote_im_contact'"
              :show-task-create-menu-item="showConversationActions && !activeConversationIsRemoteContact && !activeConversationIsSystemNotification"
              :show-forward-menu-item="showConversationActions"
              :show-auto-push-menu-item="showConversationActions && !activeConversationIsRemoteContact && !activeConversationIsSystemNotification"
              :show-share-menu-item="showConversationActions"
              :show-workspace-menu-item="true"
              :show-open-in-browser-button="showOpenInBrowserButton && !activeConversationIsSystemNotification"
              :open-in-browser-disabled="!activeConversationId || activeConversationIsSystemNotification"
              :show-code-review-menu-item="true"
              :side-chat-enabled="sideChatPanelEnabled"
              :mention-entries="mentionEntries" :selected-mention-keys="selectedMentionKeys"
              :delegate-statuses="delegateStatuses"
              @lock-workspace="$emit('lockWorkspace')" @open-branch-selection="openBranchSelectionMenu"
              @open-task-create="openTaskCreateDialog"
              @open-delegate-selection="openDelegateSelectionMenu" @open-forward-selection="openForwardSelectionMenu"
              @open-auto-push="openAutoPushCard"
              @open-share-selection="openShareSelectionMenu"
              @open-conversation-in-browser="openActiveConversationInBrowser"
              @open-delegate-summary="openDelegateSummaryPanel"
              @open-code-review="openCodeReviewDialog"
              @open-branch-from-current="openBranchFromCurrentMessage"
              @open-side-chat="selectChatRightPanelMode('sideChat')"
              @mention-entry="(entry) => {
                const agentId = String(entry?.agentId || '').trim();
                const departmentId = String(entry?.departmentId || '').trim();
                if (!agentId || !departmentId) return;
                const mentionKey = `${agentId}:${departmentId}`;
                if (selectedMentionKeys.includes(mentionKey)) { emit('removeMention', { agentId, departmentId }); return; }
                emit('addMention', { agentId, agentName: String(entry?.agentName || '').trim() || agentId, departmentId, departmentName: String(entry?.departmentName || '').trim() || departmentId, avatarUrl: String(entry?.avatarUrl || '').trim() || undefined });
              }"
            />
          </div>
        </div>
        <CompactionSummaryCard
          :visible="conversationSummaryCard.visible"
          :text="conversationSummaryCard.text"
          :is-dark="markdownIsDark"
          @close="closeConversationSummaryCard"
        />
        <Teleport to="body">
        <ul
          v-if="compactionSummaryContextMenu"
          class="menu fixed z-[1200] w-44 rounded-box border border-base-300 bg-base-100 p-1 text-base-content shadow-xl"
          :style="{ left: `${compactionSummaryContextMenu.x}px`, top: `${compactionSummaryContextMenu.y}px` }"
          @contextmenu.prevent.stop
          @pointerdown.stop
        >
          <li>
            <button type="button" class="text-error" @click="recallCompactionSummaryFromContextMenu">
              <Undo2 class="h-4 w-4" />
              <span>{{ t("chat.recall") }}</span>
            </button>
          </li>
        </ul>
        </Teleport>
        <ConversationAutoPushCard
          :open="autoPushCardOpen"
          :saving="autoPushSaving"
          :enabled="autoPushEnabled"
          :selected-contact-id="autoPushSelectedContactId"
          :options="autoPushContactOptions"
          @close="closeAutoPushCard"
          @save="saveAutoPushCard"
          @update:enabled="autoPushEnabled = $event"
          @update:selected-contact-id="autoPushSelectedContactId = $event"
        />
        <Transition name="chat-jump-action">
          <div v-show="showJumpToBottom" class="pointer-events-none absolute bottom-3 right-5 z-30 flex justify-end" :style="jumpToBottomStyle">
            <button class="btn btn-sm btn-circle btn-neutral pointer-events-auto shadow-lg" @click="handleJumpToBottom">
              <ArrowDownToLine class="h-4 w-4" />
            </button>
          </div>
        </Transition>
        <Transition name="chat-jump-action">
          <div v-show="showJumpToNextUserMessage" class="pointer-events-none absolute bottom-3 right-5 z-30 flex justify-end" :style="jumpToBottomStyle">
            <button class="btn btn-sm btn-circle border border-base-300 bg-base-100 text-base-content pointer-events-auto shadow-lg hover:border-base-300 hover:bg-base-100" @click="handleJumpToNextUserMessage">
              <ChevronsDown class="h-4 w-4" />
            </button>
          </div>
        </Transition>
        <Transition name="chat-jump-action">
          <div v-show="showJumpToPreviousUserMessage" class="pointer-events-none absolute bottom-3 right-5 z-30 flex justify-end" :style="jumpAboveBottomStyle">
            <button class="btn btn-sm btn-circle border border-base-300 bg-base-100 text-base-content pointer-events-auto shadow-lg hover:border-base-300 hover:bg-base-100" @click="handleJumpToPreviousUserMessage">
              <ChevronsUp class="h-4 w-4" />
            </button>
          </div>
        </Transition>

        <div ref="composerContainer" class="relative shrink-0 border-t border-base-300 bg-base-100 px-2 pt-1.5 pb-1.5">
          <div
            v-if="activeConversationIsRemoteContact"
            class="absolute bottom-full left-1/2 z-20 mb-3 -translate-x-1/2"
          >
            <RemoteImContactEnergyDashboard :snapshot="remoteImContactDashboardSnapshot" />
          </div>
          <Transition name="chat-status-banner">
            <div
              v-if="chatStatusBanner"
              class="pointer-events-none absolute inset-x-0 top-0 z-30 flex -translate-y-full justify-center px-2 pb-2 pt-0"
            >
              <div
                class="alert pointer-events-auto w-fit max-w-full px-4 py-2 text-sm shadow-sm"
                :class="chatStatusBanner.tone === 'error'
                  ? 'alert-error alert-soft'
                  : chatStatusBanner.tone === 'success'
                    ? 'alert-success alert-soft'
                  : chatStatusBanner.tone === 'info' || chatStatusBanner.text === t('chat.statusCompactingContext')
                    ? 'alert-info alert-soft'
                    : 'bg-base-200 text-base-content'"
              >
                <div class="flex w-full min-w-0 flex-col gap-2">
                  <div v-if="chatStatusBanner.tone === 'error'" class="flex items-center justify-between gap-2">
                    <span class="font-bold">{{ requestErrorTitle }}</span>
                    <div class="flex shrink-0 items-center gap-1">
                      <button
                        type="button"
                        class="btn btn-ghost btn-sm gap-1 text-error hover:bg-error/15"
                        @click="void copyStatusText(chatStatusBanner.text)"
                      >
                        <Copy class="h-3.5 w-3.5" />
                        <span>{{ t("common.copy") }}</span>
                      </button>
                      <button
                        type="button"
                        class="btn btn-ghost btn-sm gap-1 text-error hover:bg-error/15"
                        @click="$emit('clearChatError')"
                      >
                        <X class="h-3.5 w-3.5" />
                        <span>{{ t("common.close") }}</span>
                      </button>
                    </div>
                  </div>
                  <span
                    class="block max-h-32 min-w-0 overflow-y-auto whitespace-pre-wrap break-words text-center leading-5"
                    :class="chatStatusBanner.tone === 'error' || chatStatusBanner.tone === 'success'
                      ? ''
                      : 'text-base-content/80 ecall-shimmer-text ecall-reasoning-shimmer'"
                    :data-shimmer-text="chatStatusBanner.tone === 'error' || chatStatusBanner.tone === 'success' ? '' : chatStatusBanner.text"
                  >{{ chatStatusBanner.text }}</span>
                </div>
              </div>
            </div>
          </Transition>
          <ChatApprovalPanel
            v-if="activeConversationTerminalApprovals.length > 0"
            :approvals="activeConversationTerminalApprovals" :resolving="terminalApprovalResolving"
            @approve="$emit('approveTerminalApproval', $event)"
            @deny="$emit('denyTerminalApproval', $event)"
            @approve-for-session="$emit('approveTerminalApprovalForSession', $event)"
            @approve-for-workspace="$emit('approveTerminalApprovalForWorkspace', $event)"
          />
          <div
            v-else-if="activeConversationRecipientMissing"
            class="rounded-box border border-warning/30 bg-warning/10 p-4 text-sm"
          >
            <div class="flex flex-col gap-4">
              <!-- 信息区：图标锚点 + 标题 + 说明，独立成块不被操作区挤压 -->
              <div class="flex items-start gap-3">
                <div
                  class="mt-0.5 flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-warning/15 text-warning"
                >
                  <CircleAlert class="h-5 w-5" />
                </div>
                <div class="min-w-0">
                  <div class="font-semibold">{{ t("chat.recipientMissingTitle") }}</div>
                  <div class="mt-1 text-xs leading-relaxed opacity-70">
                    {{ t("chat.recipientMissingHint") }}
                  </div>
                </div>
              </div>
              <!-- 操作区：选择器占剩余宽度，按钮组固定 -->
              <div class="flex flex-col gap-2 sm:flex-row sm:items-center">
                <div class="min-w-0 flex-1">
                  <DepartmentPersonaSelect
                    v-model:department-id="repairRecipientDepartmentId"
                    v-model:agent-id="repairRecipientAgentId"
                    :options="repairRecipientOptions"
                    :persona-avatar-url-map="props.personaAvatarUrlMap"
                    :show-model="false"
                    :auto-select-first="true"
                    :preserve-current="false"
                  />
                </div>
                <div class="flex shrink-0 items-center justify-between gap-2 sm:justify-end">
                  <button
                    type="button"
                    class="btn btn-sm btn-primary gap-2"
                    :disabled="conversationInteractionBusy || !repairRecipientSelectedOption"
                    @click="handleRebindConversationRecipient"
                  >
                    <Check class="h-3.5 w-3.5" />
                    <span>{{ t("chat.recipientMissingApply") }}</span>
                  </button>
                  <button
                    type="button"
                    class="btn btn-sm btn-error btn-outline gap-2"
                    :disabled="conversationInteractionBusy"
                    @click="handleConversationDelete(activeConversationId)"
                  >
                    <Trash2 class="h-3.5 w-3.5" />
                    <span>{{ t("common.delete") }}</span>
                  </button>
                </div>
              </div>
            </div>
          </div>
          <ChatComposerPanel
            v-else ref="composerPanelRef" :selection-mode-enabled="messageSelectionModeEnabled"
            :composer-scope="composerScope"
            :selected-message-count="selectedMessageBlocks.length"
            :chat-input="chatInput" :instruction-presets="instructionPresets" :mention-entries="mentionEntries"
            :selected-mentions="selectedMentions"
            :clipboard-images="clipboardImages" :queued-attachment-notices="queuedAttachmentNotices"
            :link-open-error-text="linkOpenErrorText"
            :transcribing="transcribing" :can-record="canRecord" :recording="recording" :recording-ms="recordingMs"
            :record-hotkey="recordHotkey" :conversation-call-primary-api-config-id="conversationCallPrimaryApiConfigId"
            :preferred-chat-model-id="preferredChatModelId"
            :chat-model-options="chatModelOptions"
            :plan-mode-enabled="planModeEnabled"
            :workspace-access="workspaceAccess"
            :frontend-round-phase="frontendRoundPhase" :chat-usage-percent="chatUsagePercent"
            :chatting="chatting" :busy="conversationInteractionBusy"
            :stop-chat-disabled="isOrganizingContextBusy || submitPending" :frozen="frozen"
            :goal-active="goalActive"
            :goal-title="goalButtonTitle"
            :goal-disabled="activeConversationSummary?.kind === 'remote_im_contact'"
            :system-notification-mode="activeConversationIsSystemNotification"
            :remote-contact-mode="activeConversationIsRemoteContact"
            :selection-delegate-only="messageSelectionDelegateOnly"
            :show-side-conversation-list="showSideConversationList"
            :active-conversation-id="activeConversationId" :unarchived-conversation-items="unarchivedConversationItems"
            :remote-im-contact-conversations="remoteImContactConversations"
            :user-alias="userAlias" :user-avatar-url="userAvatarUrl"
            :persona-name="personaName" :persona-name-map="personaNameMap" :persona-avatar-url-map="personaAvatarUrlMap"
            :create-conversation-department-options="createConversationDepartmentOptions"
            :default-create-conversation-department-id="defaultCreateConversationDepartmentId"
            :ide-context-groups="mergedVisibleIdeContextGroups" :attached-ide-context-references="attachedIdeContextReferences"
            :current-theme="currentTheme"
            :show-conversation-actions="showConversationActions"
            @update:chat-input="$emit('update:chatInput', $event)" @add-mention="$emit('addMention', $event)"
            @remove-mention="$emit('removeMention', $event)" @remove-clipboard-image="$emit('removeClipboardImage', $event)"
            @remove-queued-attachment-notice="$emit('removeQueuedAttachmentNotice', $event)"
            @start-recording="$emit('startRecording')" @stop-recording="$emit('stopRecording')"
            @pick-attachments="$emit('pickAttachments')"
            @update:conversation-preferred-api-config-id="$emit('update:conversationPreferredApiConfigId', $event)"
            @update:workspace-access="$emit('updateWorkspaceAccess', $event)"
            @update:plan-mode-enabled="$emit('update:planModeEnabled', $event)"
            @attach-ide-context-reference="handleAttachIdeContextReference"
            @remove-ide-context-reference="handleRemoveIdeContextReference"
            @send-chat="handleSendChat" @stop-chat="$emit('stopChat')"
            @open-delegate-selection="openDelegateSelectionMenu"
            @open-task-create="openTaskCreateDialog"
            @open-goal-task="$emit('openGoalTask')"
            @exit-selection-mode="handleExitMessageSelectionMode"
            @selection-action-copy="copySelectedMessages"
            @selection-action-branch="emitSelectionAction('branch')"
            @selection-action-forward="emitSelectionAction('forward', $event)"
            @selection-action-delegate="emitSelectionAction('delegate', $event)"
            @selection-action-share="emitSelectionAction('share', $event)"
            @trim-conversation="$emit('trimConversation')" @open-conversation-list="$emit('openConversationList')" @open-settings="$emit('openSettings')"
            @create-conversation="$emit('createConversation', $event)"
          />
        </div>

        <ChatImagePreviewDialog
          :open="imagePreviewOpen" :data-url="imagePreviewDataUrl" :zoom="imagePreviewZoom"
          :min-zoom="IMAGE_PREVIEW_MIN_ZOOM" :max-zoom="IMAGE_PREVIEW_MAX_ZOOM"
          :offset-x="previewOffsetX" :offset-y="previewOffsetY" :dragging="previewDragging" :rotation="imagePreviewRotation"
          :local-path="imagePreviewLocalPath"
          :copy-status="imagePreviewCopyStatus as any"
          :save-status="imagePreviewSaveStatus as any"
          @close="closeImagePreview" @zoom-in="zoomInPreview" @zoom-out="zoomOutPreview"
          @reset="resetPreviewZoom" @wheel="onPreviewWheel" @pointer-down="onPreviewPointerDown"
          @pointer-move="onPreviewPointerMove" @pointer-up="onPreviewPointerUp"
          @rotate="rotatePreviewClockwise"
          @copy-image="handleCopyLocalImage" @save-image="handleSaveLocalImage"
        />

        <ChatGoalTaskDialog
          :open="goalDialogOpen" :saving="goalSaving" :error-text="goalError"
          :active-task="activeGoalTask" :recent-history="recentGoalTaskHistory"
          @close="$emit('closeGoalTask')" @save="$emit('saveGoalTask', $event)"
          @stop="$emit('stopGoalTask')"
        />
        <ToolReviewTargetDialog
          v-if="showConversationActions"
          :open="codeReviewDialogOpen"
          :submitting="!!toolReviewSubmittingBatchKey"
          :error-text="codeReviewErrorText"
          :current-department-id="props.currentDepartmentId"
          :current-agent-id="props.activeAgentId"
          :department-options="props.createConversationDepartmentOptions"
          :persona-avatar-url-map="props.personaAvatarUrlMap"
          :commit-options="commitOptions"
          :commit-options-loading="commitOptionsLoading"
          :commit-total="commitTotal"
          :commit-page="commitPage"
          :commit-page-size="commitPageSize"
          @close="closeCodeReviewDialog"
          @pick-commit-review="loadCodeReviewCommitOptions"
          @review-code="handleSubmitCodeReview"
        />
        <TaskCreateCard
          v-if="showConversationActions"
          :open="taskDialogOpen"
          :mode="taskDialogMode"
          :conversation-id="activeConversationId"
          :task="taskDialogTask"
          @close="closeTaskDialog"
          @created="handleTaskCreated"
          @updated="handleTaskUpdated"
        />
      </div>

    <div
      v-if="leftPaneOverlay || rightPaneOverlay"
      class="absolute inset-0 z-40 bg-base-300/20 backdrop-blur-[1px]"
      @click="closeOverlayPanes"
    ></div>

    <div
      v-if="collapsePreviewSide === 'left'"
      class="pointer-events-none absolute bottom-0 left-0 top-0 z-58 flex items-center justify-center border-r border-error/20 bg-error/12 backdrop-blur-[1px]"
      :style="{ width: `${collapsePreviewWidth}px` }"
    >
      <div class="rounded-full border border-error/25 bg-base-100/90 px-3 py-1.5 text-sm font-semibold text-error shadow-sm">
        {{ t("common.collapse") }}
      </div>
    </div>

    <div
      v-if="collapsePreviewSide === 'right'"
      class="pointer-events-none absolute bottom-0 right-0 top-0 z-58 flex items-center justify-center border-l border-error/20 bg-error/12 backdrop-blur-[1px]"
      :style="{ width: `${collapsePreviewWidth}px` }"
    >
      <div class="rounded-full border border-error/25 bg-base-100/90 px-3 py-1.5 text-sm font-semibold text-error shadow-sm">
        {{ t("common.collapse") }}
      </div>
    </div>

      <div v-if="effectiveToolReviewPanelOpen"
        :class="rightPaneInLayout ? 'flex h-full min-h-0 shrink-0 border-l border-base-300 bg-base-100' : 'absolute bottom-0 right-0 top-0 z-50 flex h-full min-h-0 border-l border-base-300 bg-base-100 shadow-2xl'"
        :style="{ width: `${rightPaneVisibleWidth}px` }">
        <FileReaderPanel
          v-if="chatRightPanelMode === 'reader'"
          ref="chatReaderPanelRef"
          class="h-full w-full"
          :initial-root-path="currentWorkspaceRootPath"
          :session-key="chatFileReaderSessionKey"
          :legacy-session-key="legacyChatFileReaderSessionKey"
          :enable-global-drop="false"
          :show-pick-file-button="false"
          :show-tab-local-file-actions="true"
          :markdown-is-dark="markdownIsDark"
          custom-markstream-id="chat-file-reader-markstream"
          @capture-context-reference="handleCaptureFileReaderContextReference"
          @add-context-reference="handleAddFileReaderContextReference"
          @clear-selection-context-reference="handleClearFileReaderSelectionContextReference"
          @clear-context-references="clearFileReaderContextReferences"
        >
          <template #tabLeadingActions>
            <ChatRightPanelSwitcher
              :model-value="chatRightPanelMode"
              :side-chat-enabled="sideChatPanelEnabled"
              @update:model-value="selectChatRightPanelMode"
            />
          </template>
          <template #empty>
            <div class="space-y-2 px-5 text-center">
              <div class="font-medium text-base-content/70">选择文件开始阅读</div>
              <div class="text-xs leading-relaxed text-base-content/50">右侧目录会跟随当前会话工作区，也可以通过文件标签页同时阅读多个文件。</div>
            </div>
          </template>
        </FileReaderPanel>
        <div v-else-if="chatRightPanelMode === 'sideChat'" class="flex h-full min-h-0 w-full flex-col bg-base-200">
          <slot name="side-chat-panel" />
        </div>
        <div v-else-if="chatRightPanelMode === 'monitor'" class="flex h-full min-h-0 w-full flex-col bg-base-200">
          <PanelTabStrip
            :tabs="monitorPanelTabs"
            :active-key="chatMonitorPanelMode"
            :show-tab-borders="false"
            :aria-label="t('chat.monitorPanelTab')"
            @select-tab="selectMonitorPanelTab"
          >
            <template #leading>
              <ChatRightPanelSwitcher
                :model-value="chatRightPanelMode"
                :side-chat-enabled="sideChatPanelEnabled"
                @update:model-value="selectChatRightPanelMode"
              />
            </template>
          </PanelTabStrip>
          <ToolReviewSidebar class="min-h-0 flex-1"
            :active-tab="toolReviewSidebarActiveTab"
            :batches="toolReviewBatches" :current-batch-key="toolReviewCurrentBatchKey"
            :detail-map="toolReviewDetailMap" :segment-map="toolReviewSegmentMap"
            :detail-loading-call-id="toolReviewDetailLoadingCallId"
            :reviewing-call-id="toolReviewReviewingCallId" :batch-reviewing-key="toolReviewBatchReviewingKey"
            :error-text="toolReviewErrorText"
            :markdown-is-dark="markdownIsDark"
            :active-conversation-id="activeConversationId"
            :current-workspace-name="currentWorkspaceName" :current-workspace-root-path="currentWorkspaceRootPath"
            :workspaces="workspaces" :current-department-id="currentDepartmentId"
            :department-options="toolReviewDepartmentOptions"
            :delegate-statuses="delegateStatuses"
            :delegate-statuses-error-text="delegateStatusesErrorText"
            :persona-avatar-url-map="personaAvatarUrlMap"
            @select-batch="setToolReviewCurrentBatchKey" @load-item-detail="loadToolReviewItemDetail"
            @review-item="runToolReviewForCall" @review-batch="runToolReviewForBatch"
            @open-delegate-detail="openDelegateArchiveDetail"
            @abort-delegate="abortDelegate"
            @assistant-link-click="handleAssistantLinkClick"
          />
        </div>
      </div>
    </div>

    <div
      v-if="showSideConversationList"
      class="ecall-pane-splitter ecall-pane-splitter-left absolute bottom-0 top-0 z-60"
      :class="{ 'ecall-pane-splitter-active': activePaneResizeSide === 'left' }"
      :style="{ left: `${leftPaneVisibleWidth - 2}px` }"
      role="separator"
      tabindex="0"
      aria-orientation="vertical"
      :aria-valuemin="PANE_WIDTH_LIMITS.left.min"
      :aria-valuemax="PANE_WIDTH_LIMITS.left.max"
      :aria-valuenow="leftPaneVisibleWidth"
      @pointerdown="startPaneResize('left', $event)"
      @keydown.left.prevent="adjustPaneWidthByKeyboard('left', -24)"
      @keydown.right.prevent="adjustPaneWidthByKeyboard('left', 24)"
    ></div>

    <div
      v-if="effectiveToolReviewPanelOpen"
      class="ecall-pane-splitter ecall-pane-splitter-right absolute bottom-0 top-0 z-60"
      :class="{ 'ecall-pane-splitter-active': activePaneResizeSide === 'right' }"
      :style="{ right: `${rightPaneVisibleWidth - 2}px` }"
      role="separator" tabindex="0" aria-orientation="vertical"
      :aria-valuemin="PANE_WIDTH_LIMITS.right.min" :aria-valuemax="PANE_WIDTH_LIMITS.right.max"
      :aria-valuenow="rightPaneVisibleWidth"
      @pointerdown="startPaneResize('right', $event)"
      @keydown.left.prevent="adjustPaneWidthByKeyboard('right', 24)"
      @keydown.right.prevent="adjustPaneWidthByKeyboard('right', -24)"
    ></div>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, toRef, watch, type Ref } from "vue";
import { useI18n } from "vue-i18n";
import { isDarkAppTheme, isVscodeHost } from "../../shell/composables/use-app-theme";
import {
  useChatComposerAppearance,
  visibleChatComposerContextGroups,
} from "../../shell/composables/use-chat-composer-appearance";
import { ArrowDownToLine, Check, ChevronsDown, ChevronsUp, CircleAlert, Copy, History, Inbox, ListTodo, Network, Trash2, Undo2, Wrench, X } from "@lucide/vue";
import {
  copyTransportChatImageToClipboard,
  getTransportHostContext,
  invokeTauri,
  onTransportNotification,
  openTransportExternalUrl,
  openTransportLocalDirectory,
  openTransportLocalFileReference,
  readTransportChatImage,
  resolveLocalFileUrl,
  saveTransportChatImageAs,
} from "../../../services/tauri-api";
import type { ApiConfigItem, ChatConversationOverviewItem, ChatMentionEntry, ChatMentionTarget, ChatMessageBlock, ChatPersonaPresenceChip, ChatTodoItem, ConversationDelegateStatusSummary, ConversationForwardTarget, IdeContextReferenceItem, IdeContextWorkspaceGroup, PromptCommandPreset, RemoteImContactConversationOption, ShellWorkspace, ShellWorkMode } from "../../../types/app";
import ChatMessageItem from "../components/ChatMessageItem.vue";
import ChatApprovalPanel from "../components/ChatApprovalPanel.vue";
import ChatComposerPanel from "../components/ChatComposerPanel.vue";
import RemoteImContactEnergyDashboard from "../components/RemoteImContactEnergyDashboard.vue";
import DepartmentPersonaSelect from "../../shared/components/DepartmentPersonaSelect.vue";
import DraftRecipientCard from "../components/DraftRecipientCard.vue";
import FloatingScrollbar from "../../shell/components/FloatingScrollbar.vue";
import ChatConversationSidebar from "../components/ChatConversationSidebar.vue";
import ChatWorkspaceToolbar from "../components/ChatWorkspaceToolbar.vue";
import ToolReviewSidebar from "../components/ToolReviewSidebar.vue";
import ChatRightPanelSwitcher from "../components/ChatRightPanelSwitcher.vue";
import ToolReviewTargetDialog from "../components/ToolReviewTargetDialog.vue";
import FileReaderPanel from "../../file-reader/components/FileReaderPanel.vue";
import PanelTabStrip from "../../shared/components/PanelTabStrip.vue";
import ChatImagePreviewDialog from "../components/dialogs/ChatImagePreviewDialog.vue";
import ChatGoalTaskDialog from "../components/dialogs/ChatGoalTaskDialog.vue";
import TaskCreateCard from "../components/dialogs/TaskCreateCard.vue";
import ConversationTodoDropdown from "../components/ConversationTodoDropdown.vue";
import CompactionSummaryCard from "../components/CompactionSummaryCard.vue";
import ConversationAutoPushCard from "../components/ConversationAutoPushCard.vue";
import { useChatImagePreview } from "../composables/use-chat-image-preview";
import { useChatMessageActions } from "../composables/use-chat-message-actions";
import { useChatScrollLayout } from "../composables/use-chat-scroll-layout";
import type { TerminalApprovalConversationItem } from "../../shell/composables/use-terminal-approval";
import { isAbsoluteLocalPath, isAssistantSpacePath, normalizeLocalLinkHref, parseLocalFileReference } from "../utils/local-link";
import { buildConversationSections, buildWorkspaceConversationSections, type ConversationSection } from "../utils/conversation-sections";
import { stripExtendedPathPrefix } from "../../../utils/shell-workspaces";
import { type ChatRenderItem, isRightAlignedMessage, canOpenInFileReader, fileExtensionFromPath } from "../utils/chat-render";
import { clearFileReaderContextCandidates } from "../utils/file-reader-context-tags";
import { useIdeContext } from "../composables/use-ide-context";
import { useDelegateStatus } from "../composables/use-delegate-status";
import { useRemoteImContactDashboard } from "../composables/use-remote-im-contact-dashboard";
import { useChatVirtualList } from "../composables/use-chat-virtual-list";
import { useChatVirtualScroll } from "../composables/use-chat-virtual-scroll";
import { useChatPanes, PANE_WIDTH_LIMITS, type UseChatPanesOptions } from "../composables/use-chat-panes";
import { useChatSelection } from "../composables/use-chat-selection";
import { useChatConversationCtx, type ChatStatusBanner, type ChatStatusBannerTone } from "../composables/use-chat-conversation-ctx";
import { useChatScrollOrchestration } from "../composables/use-chat-scroll-orchestration";
import { useChatToolReviewHandlers } from "../composables/use-chat-tool-review-handlers";
import type { ToolReviewCodeReviewScope, ToolReviewCommitOption } from "../composables/use-chat-tool-review";
import type { ChatMonitorPanelMode, ChatRightPanelMode } from "../composables/chat-ui-layout-storage";
import { useChatBlockTracking } from "../composables/use-chat-block-tracking";
import type { TaskEntry } from "../../config/views/config-tabs/task-editor";
import type { DepartmentPersonaOption } from "../../shared/department-persona-options";
import { clearNativeTextSelection } from "../../../utils/native-selection";

// ==================== props / emits ====================

const props = defineProps<{
  composerScope?: "main" | "side";
  userAlias: string; personaName: string; userAvatarUrl: string; assistantAvatarUrl: string;
  personaNameMap: Record<string, string>; personaAvatarUrlMap: Record<string, string>;
  mentionEntries: ChatMentionEntry[]; selectedMentions: ChatMentionTarget[];
  latestUserText: string; latestUserImages: Array<{ mime: string; bytesBase64: string }>;
  frontendRoundPhase: "idle" | "queued" | "waiting" | "streaming";
  submitPending?: boolean;
  chatErrorText: string; clipboardImages: Array<{ mime: string; bytesBase64: string; previewDataUrl?: string }>;
  queuedAttachmentNotices: Array<{ id: string; fileName: string; path: string; mime: string; pending?: boolean }>;
  chatInput: string; instructionPresets: PromptCommandPreset[];
  canRecord: boolean; recording: boolean; recordingMs: number; transcribing: boolean; recordHotkey: string;
  conversationCallPrimaryApiConfigId: string; preferredChatModelId?: string; toolReviewApiConfigId?: string; toolReviewRefreshTick: number; chatModelOptions: ApiConfigItem[];
  planModeEnabled: boolean; chatUsagePercent: number;
  mediaDragActive: boolean; chatting: boolean; trimming: boolean; trimmingConversationId?: string;
  compactingConversation: boolean; compactingConversationId?: string;
  conversationBusy: boolean; frozen: boolean; messageBlocks: ChatMessageBlock[];
  hasMoreHistory: boolean; loadingOlderHistory: boolean;
  latestOwnMessageAlignRequest: number; conversationScrollToBottomRequest: number; scrollToBottomBehavior: "auto" | "smooth" | "smooth_light";
  currentWorkspaceName: string; currentWorkspaceDisplayName?: string; currentWorkspaceRootPath: string; workspaces: ShellWorkspace[];
  currentWorkspaceAutonomousMode?: boolean;
  currentWorkspaceWorkMode?: ShellWorkMode;
  configShellWorkspaces?: ShellWorkspace[];
  saveDraftWorkspaces?: (items: ShellWorkspace[], autonomousMode: boolean, workMode: ShellWorkMode) => Promise<void>;
  draftWorkspaceGitRootCheck?: (path: string) => Promise<boolean>;
  currentDepartmentId: string; activeAgentId: string; activeConversationId: string; currentTodos: ChatTodoItem[];
  goalActive: boolean; goalTitle: string; goalDialogOpen: boolean;
  goalSaving: boolean; goalError: string;
  activeGoalTask: { taskId: string; goal: string; why: string; todo: string; endAtLocal: string; remainingHours: number } | null;
  recentGoalTaskHistory: Array<{ goal: string; why: string; todo: string; durationHours: number }>;
  currentTheme: string; unarchivedConversationItems: ChatConversationOverviewItem[];
  remoteImContactConversations: RemoteImContactConversationOption[];
  conversationItems?: ChatConversationOverviewItem[]; sideConversationListVisible: boolean;
  initialToolReviewPanelOpen: boolean;
  conversationListTab: "local" | "contact" | "task";
  chatLeftPanelMode: "local" | "contact" | "task";
  chatRightPanelMode: ChatRightPanelMode;
  chatMonitorPanelMode: ChatMonitorPanelMode;
  sideChatPanelEnabled?: boolean;
  createConversationDepartmentOptions: DepartmentPersonaOption[];
  recipientOptionsReady?: boolean;
  defaultCreateConversationDepartmentId: string;
  ideContextGroups: IdeContextWorkspaceGroup[];
  terminalApprovals?: TerminalApprovalConversationItem[];
  terminalApprovalResolving?: boolean;
  hideConversationControlPanel?: boolean;
  showOpenInBrowserButton?: boolean;
  systemNotificationMode?: boolean;
  hideWorkspaceButton?: boolean;
  workspaceAccess?: "read_only" | "approval" | "full_access" | "";
}>();

const emit = defineEmits<{
  (e: "update:chatInput", value: string): void;
  (e: "addMention", value: ChatMentionTarget): void;
  (e: "removeMention", value: string | { agentId: string; departmentId?: string }): void;
  (e: "sideConversationListVisibleChange", value: boolean): void;
  (e: "toolReviewPanelOpenChange", value: boolean): void;
  (e: "openChatReaderFile", path: string, line?: number): void;
  (e: "sidePanelWidthsChange", value: { leftWidth: number; rightWidth: number }): void;
  (e: "sidePanelWidthsCommit", value: { leftWidth: number; rightWidth: number }): void;
  (e: "update:conversation-list-tab", value: "local" | "contact" | "task"): void;
  (e: "update:chatLeftPanelMode", value: "local" | "contact" | "task"): void;
  (e: "update:chatRightPanelMode", value: ChatRightPanelMode): void;
  (e: "update:chatMonitorPanelMode", value: ChatMonitorPanelMode): void;
  (e: "removeClipboardImage", index: number): void;
  (e: "removeQueuedAttachmentNotice", index: number): void;
  (e: "startRecording"): void; (e: "stopRecording"): void; (e: "pickAttachments"): void;
  (e: "update:conversationPreferredApiConfigId", value: string): void;
  (e: "updateWorkspaceAccess", value: "read_only" | "approval" | "full_access"): void;
  (e: "update:planModeEnabled", value: boolean): void;
  (e: "sendChat", payload?: { extraTextBlocks?: string[] }): void;
  (e: "stopChat"): void; (e: "trimConversation"): void; (e: "openConversationList"): void; (e: "openSettings"): void;
  (e: "clearChatError"): void;
  (e: "createConversationBranchFromTurn", payload: { turnId: string }): void;
  (e: "recallTurn", payload: { turnId: string }): void;
  (e: "regenerateTurn", payload: { turnId: string }): void;
  (e: "confirmPlan", payload: { messageId: string }): void;
  (e: "lockWorkspace"): void; (e: "openGoalTask"): void; (e: "openCodeReview"): void;
  (e: "closeGoalTask"): void;
  (e: "saveGoalTask", payload: { durationHours: number; goal: string; why: string; todo: string }): void;
  (e: "stopGoalTask"): void;
  (e: "taskCreated", task: TaskEntry): void;
  (e: "taskUpdated", task: TaskEntry): void;
  (e: "switchConversation", payload: { conversationId: string; kind?: "local_unarchived" | "remote_im_contact"; remoteContactId?: string }): void;
  (e: "renameConversation", payload: { conversationId: string; title: string }): void;
  (e: "togglePinConversation", conversationId: string): void;
  (e: "archiveConversation", conversationId: string): void;
  (e: "exportConversation", conversationId: string): void;
  (e: "deleteConversation", conversationId: string): void;
  (e: "rebindConversationRecipient", payload: { conversationId: string; departmentId: string; agentId: string }): void;
  (e: "updateDraftConversation", payload: { conversationId: string; departmentId?: string; agentId?: string; preferredApiConfigId?: string | null }): void;
  (e: "createConversation", input?: { title?: string; departmentId?: string; agentId?: string; copyCurrent?: boolean; importPath?: string; shellWorkspaces?: ShellWorkspace[]; shellWorkMode?: ShellWorkMode; shellAutonomousMode?: boolean }): void;
  (e: "loadOlderHistory"): void; (e: "reachedBottom"): void;
  (e: "jumpToConversationBottom"): void;
  (e: "refreshToolReviewMessage", payload: { conversationId: string; messageId: string }): void;
  (e: "selectionActionCopy", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; conversationId?: string }): void;
  (e: "selectionActionCopyError", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; conversationId?: string; error: string }): void;
  (e: "selectionActionBranch", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; conversationId?: string }): void;
  (e: "selectionActionForward", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; conversationId?: string; target: ConversationForwardTarget }): void;
  (e: "selectionActionDelegate", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; conversationId?: string; departmentId: string; agentId: string; presetId: string; why: string; goal: string; todo: string }): void;
  (e: "selectionActionShare", payload: { count: number; messageIds: string[]; blocks: ChatMessageBlock[]; conversationId?: string; exportFormat?: "html" | "png" | "copyPng" }): void;
  (e: "approveTerminalApproval", requestId: string): void;
  (e: "denyTerminalApproval", requestId: string): void;
  (e: "approveTerminalApprovalForSession", requestId: string): void;
  (e: "approveTerminalApprovalForWorkspace", requestId: string): void;
}>();

// ==================== basic state ====================

const { t, locale } = useI18n();
const chatReaderPanelRef = ref<InstanceType<typeof FileReaderPanel> | null>(null);
const chatScrollbarRef = ref<InstanceType<typeof FloatingScrollbar> | null>(null);
const linkOpenErrorText = ref("");
const conversationSummaryCard = ref<{ visible: boolean; text: string }>({ visible: false, text: "" });
const compactionSummaryContextMenu = ref<{ x: number; y: number; block: ChatMessageBlock } | null>(null);
const composerPanelRef = ref<{ focusInput: (opts?: FocusOptions) => void } | null>(null);
const taskDialogOpen = ref(false);
const taskDialogMode = ref<"create" | "edit">("create");
const taskDialogTask = ref<TaskEntry | null>(null);
const autoPushCardOpen = ref(false);
const autoPushSaving = ref(false);
const autoPushEnabled = ref(false);
const autoPushSelectedContactId = ref("");
const autoPushContactOptions = computed(() =>
  props.remoteImContactConversations.filter((item) => item.channelEnabled !== false),
);

type WebAccessInfo = {
  running: boolean;
  enabled: boolean;
  localUrl: string;
};

const codeReviewDialogOpen = ref(false);
const codeReviewErrorText = ref("");
const commitOptions = ref<ToolReviewCommitOption[]>([]);
const commitOptionsLoading = ref(false);
const commitTotal = ref(0);
const commitPage = ref(1);
const commitPageSize = ref(5);

type ToolReviewSidebarTab = "tools" | "delegates" | "tasks" | "fastRequests";

const monitorPanelTabs = computed<Array<{ key: ChatMonitorPanelMode; label: string; icon: typeof Network; closeable: false }>>(() => [
  { key: "delegate", label: t("chat.toolReview.delegatesTab"), icon: Network, closeable: false },
  { key: "tasks", label: t("chat.toolReview.tasksTab"), icon: ListTodo, closeable: false },
  { key: "tools", label: t("chat.toolReview.toolsTab"), icon: Wrench, closeable: false },
  { key: "fastRequests", label: t("chat.fastRequest.tab"), icon: Inbox, closeable: false },
]);

const toolReviewSidebarActiveTab = computed<ToolReviewSidebarTab>(() => {
  if (props.chatMonitorPanelMode === "tools") return "tools";
  if (props.chatMonitorPanelMode === "fastRequests") return "fastRequests";
  if (props.chatMonitorPanelMode === "tasks") return "tasks";
  return "delegates";
});
// ==================== messages / audio ====================

const { playingAudioId, copyMessage, stopAudioPlayback, toggleAudioPlayback } = useChatMessageActions();
const transientNotice = ref<ChatStatusBanner | null>(null);
let transientNoticeTimer = 0;

function showTransientNotice(text: string, tone: ChatStatusBannerTone = "success") {
  const next = String(text || "").trim();
  if (!next) return;
  transientNotice.value = { text: next, tone };
  if (transientNoticeTimer) window.clearTimeout(transientNoticeTimer);
  transientNoticeTimer = window.setTimeout(() => {
    transientNotice.value = null;
    transientNoticeTimer = 0;
  }, 2200);
}

async function copyStatusText(text: string) {
  const content = String(text || "").trim();
  if (!content) return;
  try {
    await navigator.clipboard.writeText(content);
  } catch (error) {
    console.warn("[状态提示] 复制失败", error);
  }
}

async function handleCopyMessage(block: ChatMessageBlock) {
  const ok = await copyMessage(block);
  if (ok) {
    showTransientNotice(t("chat.copyDone"), "success");
    return;
  }
  showTransientNotice(t("chat.copyFailed"), "error");
}

function handleCopyMessageImageDone() {
  showTransientNotice(t("chat.copyImageDone"), "success");
}

function handleCopyMessageImageFailed() {
  showTransientNotice(t("chat.copyFailed"), "error");
}

function handleSelectionCopyDone(count: number) {
  showTransientNotice(t("chat.selection.copied", { count }), "success");
}

function handleSelectionCopyFailed() {
  showTransientNotice(t("chat.copyFailed"), "error");
}

// ==================== context computed ====================

const {
  markdownIsDark, normalizedConversationTodos,
  activeConversationSummary, isCurrentConversationCompacting,
  activeConversationTerminalApprovals, goalButtonTitle,
  isOrganizingContextBusy, chatStatusBanner: baseChatStatusBanner, selectedMentionKeys,
  latestPendingPlanMessageId,
} = useChatConversationCtx(props, isDarkAppTheme, t);
const chatStatusBanner = computed(() => {
  if (transientNotice.value) return transientNotice.value;
  return baseChatStatusBanner.value;
});
const requestErrorTitle = computed(() => {
  const title = t("chat.errorTitleRequest");
  return title === "chat.errorTitleRequest" ? "请求发生错误" : title;
});
const conversationInteractionBusy = computed(() =>
  props.conversationBusy || isOrganizingContextBusy.value,
);
const activeConversationIsSystemNotification = computed(() =>
  !!props.systemNotificationMode || !!activeConversationSummary.value?.isSystemNotificationConversation,
);
const activeConversationIsRemoteContact = computed(() =>
  activeConversationSummary.value?.kind === 'remote_im_contact',
);

// ==================== 会话草稿：历史区人格选择卡 ====================

// 草稿判定：overview 标记为草稿即显示选择卡。
// 转正由后端写回存储 is_draft=false 并推送 overview 水位线，前端收到后自动消失。
// 但「用户按下回车发出消息」的那一刻前端就要立刻转正，不等后端水位线：
// 本地记录已发送消息的会话 id，该会话即使 overview 仍标记 isDraft=true 也不再显示选择卡。
const locallyPromotedConversationId = ref("");
const activeConversationIsDraft = computed(() => {
  const conversationId = String(props.activeConversationId || "").trim();
  const locallyPromoted =
    !!locallyPromotedConversationId.value &&
    locallyPromotedConversationId.value === conversationId;
  return !!activeConversationSummary.value?.isDraft && !locallyPromoted;
});
const draftSelectedDepartmentId = ref("");
const draftSelectedAgentId = ref("");

const DRAFT_RECENT_RECIPIENT_LIMIT = 6;

const draftRecentRecipientOptions = computed<DepartmentPersonaOption[]>(() => {
  const allOptions = Array.isArray(props.createConversationDepartmentOptions)
    ? props.createConversationDepartmentOptions
    : [];
  const activeConversationId = String(props.activeConversationId || "").trim();
  const items = [...(props.unarchivedConversationItems || [])].sort((a, b) => {
    const timeOf = (item: ChatConversationOverviewItem) =>
      String(item.lastMessageAt || item.updatedAt || "").trim();
    return timeOf(b).localeCompare(timeOf(a));
  });
  const seen = new Set<string>();
  const recents: DepartmentPersonaOption[] = [];
  for (const item of items) {
    const departmentId = String(item.departmentId || "").trim();
    const agentId = String(item.agentId || "").trim();
    if (!departmentId || !agentId) continue;
    if (String(item.conversationId || "").trim() === activeConversationId) continue;
    // 按「部门+人格」组合去重：同一人格挂多个部门时，每个部门各占一个行星卡片
    const key = `${departmentId}\u0000${agentId}`;
    if (seen.has(key)) continue;
    seen.add(key);
    const option = allOptions.find((candidate) =>
      String(candidate.departmentId || "").trim() === departmentId
      && String(candidate.agentId || "").trim() === agentId
      && !candidate.unavailable
      && !candidate.personaMissing
    );
    if (!option) continue;
    recents.push(option);
    if (recents.length >= DRAFT_RECENT_RECIPIENT_LIMIT) break;
  }
  return recents;
});

watch(
  [activeConversationIsDraft, () => props.activeConversationId],
  ([isDraft]) => {
    if (!isDraft) return;
    draftSelectedDepartmentId.value = String(activeConversationSummary.value?.departmentId || "").trim();
    draftSelectedAgentId.value = String(activeConversationSummary.value?.agentId || "").trim();
  },
  { immediate: true },
);

function handleDraftPersonaChange(payload: { departmentId: string; agentId: string }) {
  draftSelectedDepartmentId.value = payload.departmentId;
  draftSelectedAgentId.value = payload.agentId;
  emit("updateDraftConversation", {
    conversationId: String(props.activeConversationId || "").trim(),
    departmentId: payload.departmentId,
    agentId: payload.agentId,
  });
}
const remoteImContactDashboardContactId = computed(() =>
  activeConversationIsRemoteContact.value
    ? String(activeConversationSummary.value?.remoteContactId || "").trim()
    : "",
);
const { snapshot: remoteImContactDashboardSnapshot } = useRemoteImContactDashboard({
  contactId: remoteImContactDashboardContactId,
  enabled: activeConversationIsRemoteContact,
});
const repairRecipientDepartmentId = ref("");
const repairRecipientAgentId = ref("");
const repairRecipientOptions = computed(() =>
  (Array.isArray(props.createConversationDepartmentOptions) ? props.createConversationDepartmentOptions : [])
    .filter((option) =>
      !option.unavailable
      && !!String(option.departmentId || "").trim()
      && !!String(option.agentId || "").trim()
    ),
);

function findRecipientOption(departmentId: string, agentId: string): DepartmentPersonaOption | null {
  const normalizedDepartmentId = String(departmentId || "").trim();
  const normalizedAgentId = String(agentId || "").trim();
  if (!normalizedDepartmentId || !normalizedAgentId) return null;
  return repairRecipientOptions.value.find((option) =>
    String(option.departmentId || "").trim() === normalizedDepartmentId
    && String(option.agentId || "").trim() === normalizedAgentId
    && !option.personaMissing
  ) || null;
}

const activeConversationRecipientMissing = computed(() => {
  if (!props.recipientOptionsReady) return false;
  if (activeConversationIsSystemNotification.value || activeConversationIsRemoteContact.value) return false;
  const conversationId = String(props.activeConversationId || "").trim();
  if (!conversationId) return false;
  const summary = activeConversationSummary.value;
  if (!summary) return false;
  const departmentId = String(summary.departmentId || "").trim();
  const agentId = String(summary.agentId || "").trim();
  return !findRecipientOption(departmentId, agentId);
});
const repairRecipientSelectedOption = computed(() =>
  findRecipientOption(repairRecipientDepartmentId.value, repairRecipientAgentId.value),
);

function defaultRepairRecipientOption(): DepartmentPersonaOption | null {
  const defaultDepartmentId = String(props.defaultCreateConversationDepartmentId || "").trim();
  const hasValidPersona = (option: DepartmentPersonaOption) => !option.personaMissing;
  return repairRecipientOptions.value.find((option) =>
    hasValidPersona(option)
    && defaultDepartmentId && String(option.departmentId || "").trim() === defaultDepartmentId
  ) || repairRecipientOptions.value.find(hasValidPersona)
    || repairRecipientOptions.value[0] || null;
}

watch(
  () => [
    activeConversationRecipientMissing.value,
    props.activeConversationId,
    props.defaultCreateConversationDepartmentId,
    repairRecipientOptions.value.map((option) => `${option.departmentId}:${option.agentId}`).join("|"),
  ] as const,
  () => {
    if (!activeConversationRecipientMissing.value) return;
    if (repairRecipientSelectedOption.value) return;
    const option = defaultRepairRecipientOption();
    repairRecipientDepartmentId.value = String(option?.departmentId || "").trim();
    repairRecipientAgentId.value = String(option?.agentId || "").trim();
  },
  { immediate: true },
);

const toolReviewDepartmentOptions = computed(() =>
  // 用户主动发起代码审查不受 AI delegate 工具的“直接下级部门”限制。
  (Array.isArray(props.createConversationDepartmentOptions) ? props.createConversationDepartmentOptions : []),
);

const departmentNameMap = computed<Record<string, string>>(() => {
  const map: Record<string, string> = {};
  for (const option of props.createConversationDepartmentOptions || []) {
    const departmentId = String(option.departmentId || "").trim();
    if (!departmentId || map[departmentId]) continue;
    map[departmentId] = String(option.departmentName || option.name || departmentId).trim() || departmentId;
  }
  return map;
});

const departmentNameMapSignature = computed(() =>
  Object.entries(departmentNameMap.value)
    .map(([id, name]) => `${id}:${name}`)
    .sort()
    .join("|"),
);

const chatFileReaderSessionKey = computed(() => {
  const conversationId = String(props.activeConversationId || "").trim();
  return conversationId ? `easy_call.chat_file_reader_session.${conversationId}.v1` : "";
});

const legacyChatFileReaderSessionKey = computed(() => {
  const conversationId = String(props.activeConversationId || "").trim();
  return conversationId ? `easy-call.chat.file-reader-session.${conversationId}` : "";
});

// ==================== messages / audio ====================

const showSideConversationList = computed(() => !!props.sideConversationListVisible);
const showConversationActions = computed(() => !props.hideConversationControlPanel);
const showOpenInBrowserButton = computed(() => props.showOpenInBrowserButton ?? true);

function canRegenerateBlock(block: ChatMessageBlock, blockIndex: number): boolean {
  if (block.role !== "assistant" || block.isExtraTextBlock) return false;
  for (let idx = props.messageBlocks.length - 1; idx >= 0; idx -= 1) {
    const candidate = props.messageBlocks[idx];
    if (candidate.role !== "assistant" || candidate.isExtraTextBlock) continue;
    return idx === blockIndex;
  }
  return false;
}

function canConfirmPlan(block: ChatMessageBlock): boolean {
  if (block.role !== "assistant" || block.isExtraTextBlock) return false;
  if (block.planCard?.action !== "present") return false;
  const targetId = String(block.sourceMessageId || block.id || "").trim();
  if (targetId !== latestPendingPlanMessageId.value) return false;
  const blockIndex = props.messageBlocks.findIndex((item) => String(item.id || "").trim() === String(block.id || "").trim());
  if (blockIndex < 0) return false;
  return !props.messageBlocks.slice(blockIndex + 1).some((item) => !item.isExtraTextBlock && item.role === "user");
}

const messageSelectionDelegateOnly = ref(false);

function openSelectionMenu(options: { delegateOnly?: boolean; allowWhenBusy?: boolean } = {}) {
  // 多选入口忙碌时禁用；分支/委托/转发/分享属于子代理或纯读取操作，
  // 不影响主轮次，忙碌时允许进入选择模式（allowWhenBusy）。
  if (!options.allowWhenBusy && (props.chatting || props.frozen || conversationInteractionBusy.value)) return;
  clearNativeTextSelection();
  messageSelectionDelegateOnly.value = !!options.delegateOnly;
  messageSelectionModeEnabled.value = true;
  selectedMessageRenderIds.value = [];
  void nextTick(() => composerPanelRef.value?.focusInput?.({ preventScroll: true }));
}
const openBranchSelectionMenu = () => openSelectionMenu({ allowWhenBusy: true });
const openDelegateSelectionMenu = () => openSelectionMenu({ delegateOnly: true, allowWhenBusy: true });
const openForwardSelectionMenu = () => openSelectionMenu({ allowWhenBusy: true });
const openShareSelectionMenu = () => openSelectionMenu({ allowWhenBusy: true });

/** 从当前会话最新一条用户消息直接创建分支（无需进入选择模式） */
function openBranchFromCurrentMessage() {
  if (props.chatting || props.frozen || conversationInteractionBusy.value) return;
  const candidates = props.messageBlocks.filter(
    (block) => !block.isExtraTextBlock
      && String(block.role || "").trim().toLowerCase() === "user"
      && !block.isStreaming,
  );
  const latest = candidates[candidates.length - 1];
  const turnId = latest ? String(latest.sourceMessageId || latest.id || "").trim() : "";
  if (!turnId) return;
  emit("createConversationBranchFromTurn", { turnId });
}

function openTaskCreateDialog() {
  taskDialogMode.value = "create";
  taskDialogTask.value = null;
  taskDialogOpen.value = true;
}

function openTaskEditDialog(task: TaskEntry) {
  taskDialogMode.value = "edit";
  taskDialogTask.value = task;
  taskDialogOpen.value = true;
}

function closeTaskDialog() {
  taskDialogOpen.value = false;
}

function handleTaskCreated(task: TaskEntry) {
  taskDialogOpen.value = false;
  emit("taskCreated", task);
}

function handleTaskUpdated(task: TaskEntry) {
  taskDialogOpen.value = false;
  emit("taskUpdated", task);
}

function openConversationSummary(block: ChatMessageBlock, event?: MouseEvent) {
  event?.stopPropagation();
  const text = String(block?.text || "").trim();
  if (!text) return;
  conversationSummaryCard.value = { visible: true, text };
}

function openCompactionSummaryContextMenu(block: ChatMessageBlock, event: MouseEvent) {
  event.preventDefault();
  event.stopPropagation();
  const x = Math.max(8, Math.min(event.clientX, window.innerWidth - 184));
  const y = Math.max(8, Math.min(event.clientY, window.innerHeight - 48));
  compactionSummaryContextMenu.value = { x, y, block };
}

function recallCompactionSummaryFromContextMenu() {
  const block = compactionSummaryContextMenu.value?.block;
  compactionSummaryContextMenu.value = null;
  if (!block) return;
  const turnId = String(block.sourceMessageId || block.id || "").trim();
  if (!turnId) return;
  emit("recallTurn", { turnId });
}

function closeCompactionSummaryContextMenu() {
  compactionSummaryContextMenu.value = null;
}

function handleCompactionSummaryContextMenuKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") closeCompactionSummaryContextMenu();
}

function closeConversationSummaryCard() {
  conversationSummaryCard.value = { visible: false, text: "" };
}

function openAutoPushCard() {
  const currentTargetId = String(activeConversationSummary.value?.autoPushRemoteContactId || "").trim();
  autoPushEnabled.value = !!currentTargetId;
  autoPushSelectedContactId.value = currentTargetId;
  autoPushCardOpen.value = true;
}

function closeAutoPushCard() {
  autoPushCardOpen.value = false;
  autoPushSaving.value = false;
}

async function saveAutoPushCard() {
  const conversationId = String(props.activeConversationId || "").trim();
  if (!conversationId || autoPushSaving.value) return;
  autoPushSaving.value = true;
  try {
    await invokeTauri("conversation.autoPush", {
      input: {
        conversationId,
        remoteContactId: autoPushEnabled.value
          ? String(autoPushSelectedContactId.value || "").trim() || null
          : null,
      },
    });
    autoPushCardOpen.value = false;
  } finally {
    autoPushSaving.value = false;
  }
}

// ==================== ide context ====================

const {
  visibleIdeContextGroups, attachedIdeContextReferences,
  attachReference: handleAttachIdeContextReference,
  removeReference: handleRemoveIdeContextReference,
  clearAttachedReferences: clearAttachedIdeContextReferences,
} = useIdeContext({
  activeConversationId: toRef(props, "activeConversationId"),
  workspaces: toRef(props, "workspaces"),
  currentWorkspaceRootPath: toRef(props, "currentWorkspaceRootPath"),
  currentWorkspaceName: toRef(props, "currentWorkspaceName"),
  enabled: computed(() => true),
});

const fileReaderVisibleContextReference = ref<IdeContextReferenceItem | null>(null);
const fileReaderSelectionContextReference = ref<IdeContextReferenceItem | null>(null);
const fileReaderContextReferences = computed<IdeContextReferenceItem[]>(() => {
  const visible = fileReaderVisibleContextReference.value;
  const selection = fileReaderSelectionContextReference.value;
  if (!visible && !selection) return [];
  if (!visible) return selection ? [selection] : [];
  if (!selection) return [visible];
  const visibleFilePath = String(visible.filePath || "").trim();
  const selectionFilePath = String(selection.filePath || "").trim();
  return visibleFilePath && visibleFilePath === selectionFilePath ? [selection] : [visible];
});
const {
  sideFileTagsEnabled,
  ideBridgeFileTagsEnabled,
} = useChatComposerAppearance();
const mergedVisibleIdeContextGroups = computed<IdeContextWorkspaceGroup[]>(() => {
  const propGroups = Array.isArray(props.ideContextGroups) ? props.ideContextGroups : [];
  const baseGroups = propGroups.length > 0 ? propGroups : visibleIdeContextGroups.value;
  return visibleChatComposerContextGroups({
    sideReferences: fileReaderContextReferences.value,
    sideWorkspacePath: props.currentWorkspaceRootPath,
    sideWorkspaceName: String(props.currentWorkspaceName || "").trim() || t("chat.allowedWorkspaceButton"),
    ideBridgeGroups: baseGroups,
    sideFileTagsEnabled: sideFileTagsEnabled.value,
    ideBridgeFileTagsEnabled: ideBridgeFileTagsEnabled.value,
  });
});

function handleCaptureFileReaderContextReference(reference: IdeContextReferenceItem) {
  const source = String(reference.source || "").trim();
  if (source === "visible_range") {
    fileReaderVisibleContextReference.value = { ...reference };
  } else {
    fileReaderSelectionContextReference.value = { ...reference };
  }
}

function handleAddFileReaderContextReference(reference: IdeContextReferenceItem) {
  handleAttachIdeContextReference(reference);
  fileReaderSelectionContextReference.value = null;
  void nextTick(() => composerPanelRef.value?.focusInput?.({ preventScroll: true }));
}

function handleClearFileReaderSelectionContextReference() {
  fileReaderSelectionContextReference.value = null;
}

function clearFileReaderContextReferences(paths?: string[]) {
  const candidates = clearFileReaderContextCandidates({
    visible: fileReaderVisibleContextReference.value,
    selection: fileReaderSelectionContextReference.value,
  }, paths);
  fileReaderVisibleContextReference.value = candidates.visible;
  fileReaderSelectionContextReference.value = candidates.selection;
}

watch(() => props.activeConversationId, () => {
  fileReaderVisibleContextReference.value = null;
  fileReaderSelectionContextReference.value = null;
});

// ==================== selection state shared between virtual list & selection mode ====================

const messageSelectionModeEnabled = ref(false);
const selectedMessageRenderIds = ref<string[]>([]);
const selectedMessageRenderIdSet = computed(() => new Set(selectedMessageRenderIds.value));

// ==================== virtual list ====================

const { chatRenderItems, messageMemoKey } = useChatVirtualList({
  messageBlocks: toRef(props, "messageBlocks"), markdownIsDark, playingAudioId,
  userAlias: toRef(props, "userAlias"), userAvatarUrl: toRef(props, "userAvatarUrl"),
  personaNameMap: toRef(props, "personaNameMap"), personaAvatarUrlMap: toRef(props, "personaAvatarUrlMap"),
  chatting: toRef(props, "chatting"), conversationBusy: conversationInteractionBusy,
  frozen: toRef(props, "frozen"), messageSelectionModeEnabled,
  selectedMessageRenderIdSet,
  canRegenerateBlock, canConfirmPlan,
});

const virtualRenderItems = computed<ChatRenderItem[]>(() => [...chatRenderItems.value]);
const olderHistoryCorrectionAllowed = ref(false);

// 初始测高覆盖层：会话切换后消息行按 1px 估计高度定位、未实测前会重叠，
// 用半透明覆盖层遮住直到视口内全部行实测完成（measurementSettled）。
// 超时兜底 1.5s 防信号异常导致永久遮挡；流式期间不显示（行高度持续变化）。
const initialMeasureOverlayForceHidden = ref(false);
let initialMeasureOverlayTimeout: ReturnType<typeof setTimeout> | undefined;
watch(
  () => String(props.activeConversationId || "").trim(),
  () => {
    initialMeasureOverlayForceHidden.value = false;
    if (initialMeasureOverlayTimeout) {
      clearTimeout(initialMeasureOverlayTimeout);
      initialMeasureOverlayTimeout = undefined;
    }
    if (typeof window !== "undefined") {
      initialMeasureOverlayTimeout = setTimeout(() => {
        initialMeasureOverlayTimeout = undefined;
        initialMeasureOverlayForceHidden.value = true;
      }, 1500);
    }
  },
  { immediate: true },
);
const showInitialMeasureOverlay = computed(() =>
  !!String(props.activeConversationId || "").trim()
  && virtualRenderItems.value.length > 0
  && !props.chatting
  && !measurementSettled.value
  && !initialMeasureOverlayForceHidden.value,
);
onBeforeUnmount(() => {
  if (initialMeasureOverlayTimeout) {
    clearTimeout(initialMeasureOverlayTimeout);
    initialMeasureOverlayTimeout = undefined;
  }
});

const showNoMoreHistoryDivider = computed(() =>
  !!String(props.activeConversationId || "").trim()
  && props.messageBlocks.length > 0
  && !props.loadingOlderHistory
  && !props.hasMoreHistory,
);

// ==================== block tracking ====================

const { isOwnMessage, latestOwnMessageId, latestOwnElasticItemId } =
  useChatBlockTracking(toRef(props, "messageBlocks"), chatRenderItems);

// ==================== selection mode ====================

const {
  selectedMessageBlocks, enterMessageSelectionMode, toggleMessageSelected,
  exitMessageSelectionMode: resetMessageSelectionMode, copySelectedMessages, emitSelectionAction,
} = useChatSelection({
  chatRenderItems: computed(() => chatRenderItems.value.flatMap((item) => {
    if (item.kind === "message") return [{ renderId: item.renderId, block: item.block }];
    return [];
  })),
  messageSelectionModeEnabled,
  selectedMessageRenderIds,
  personaNameMap: props.personaNameMap, userAlias: props.userAlias,
  conversationId: toRef(props, "activeConversationId"),
  t,
  onEmit: {
    selectionActionCopy: (payload) => {
      handleSelectionCopyDone(payload.count);
      emit("selectionActionCopy", payload);
    },
    selectionActionCopyError: (payload) => {
      handleSelectionCopyFailed();
      emit("selectionActionCopyError", payload);
    },
    selectionActionBranch: (payload) => emit("selectionActionBranch", payload),
    selectionActionForward: (payload) => emit("selectionActionForward", payload),
    selectionActionDelegate: (payload) => emit("selectionActionDelegate", payload),
    selectionActionShare: (payload) => emit("selectionActionShare", payload),
  },
});

function handleEnterMessageSelectionMode(selectionKey: string) {
  messageSelectionDelegateOnly.value = false;
  enterMessageSelectionMode(selectionKey);
}

function handleExitMessageSelectionMode() {
  messageSelectionDelegateOnly.value = false;
  resetMessageSelectionMode();
}

defineExpose({
  exitMessageSelectionMode: handleExitMessageSelectionMode,
  showTransientNotice,
  openFileInReader,
});

// ==================== scroll layout ====================

const {
  scrollContainer, composerContainer, toolbarContainer, chatLayoutRoot,
  latestOwnElasticMinHeight, showJumpToBottom, atConversationBottom, userScrollingUp,
  sessionControlPanelVisible, jumpToBottomStyle, jumpAboveBottomStyle, toolbarReservedHeight, floatingToolbarStyle, onScroll,
  noteWheelScrollIntent, beginPointerScrollIntent, prepareBottomAlignmentLayout,
} = useChatScrollLayout({
  activeConversationId: toRef(props, "activeConversationId"),
  chatting: toRef(props, "chatting"), busy: conversationInteractionBusy,
  frozen: toRef(props, "frozen"),
  timelineItemCount: computed(() => virtualRenderItems.value.length),
  onReachedBottom: () => emit("reachedBottom"),
  focusComposerInput: (options) => composerPanelRef.value?.focusInput(options),
});

// ==================== virtual scroll ====================

const {
  virtualizer, virtualEntries, totalVirtualSize,
  latestOwnTailContentHeight, latestOwnTailContentMeasured, measurementSettled, scheduleVirtualMeasure, syncViewportMetrics,
  scrollVirtualizerToIndex, scrollVirtualizerToConversationBottomLightweight,
  resetVirtualizerAtConversationBottom,
  measureElementRef,
} = useChatVirtualScroll({
  renderItems: virtualRenderItems,
  scrollContainer, scrollbarRef: chatScrollbarRef as Ref<{ updateThumb: () => void } | null>,
  activeConversationId: toRef(props, "activeConversationId"),
  latestOwnElasticItemId,
  latestOwnElasticMinHeight,
  chatting: toRef(props, "chatting"),
  olderHistoryCorrectionAllowed,
  debugEnabled: computed(() => true),
  onUserScroll: () => onScroll(),
});

const latestOwnTailSpacerMinHeight = ref(0);

watch(
  [latestOwnElasticItemId, latestOwnElasticMinHeight, latestOwnTailContentHeight, latestOwnTailContentMeasured],
  ([itemId, targetHeight, tailContentHeight, tailContentMeasured]) => {
    if (!itemId) {
      latestOwnTailSpacerMinHeight.value = targetHeight;
      return;
    }
    if (!tailContentMeasured) return;
    latestOwnTailSpacerMinHeight.value = Math.max(0, targetHeight - tailContentHeight);
  },
  { immediate: true },
);

const currentWorkspacePermissionKind = computed<"read_only" | "approval" | "full_access" | "autonomous">(() => {
  if (props.currentWorkspaceAutonomousMode) return "autonomous";
  const targetPath = String(props.currentWorkspaceRootPath || "").trim().toLowerCase();
  const workspaceList = Array.isArray(props.workspaces) ? props.workspaces : [];
  const matched = workspaceList.find((item) => String(item.path || "").trim().toLowerCase() === targetPath);
  if (matched?.access === "approval" || matched?.access === "full_access" || matched?.access === "read_only") {
    return matched.access;
  }
  const mainWorkspace = workspaceList.find((item) => String(item.level || "").trim() === "main");
  if (mainWorkspace?.access === "approval" || mainWorkspace?.access === "full_access" || mainWorkspace?.access === "read_only") {
    return mainWorkspace.access;
  }
  return "read_only";
});

// 草稿卡片回显用：不含 autonomous 分支的纯权限值
const currentWorkspaceAccess = computed<"read_only" | "approval" | "full_access">(() => {
  const targetPath = String(props.currentWorkspaceRootPath || "").trim().toLowerCase();
  const workspaceList = Array.isArray(props.workspaces) ? props.workspaces : [];
  const matched = workspaceList.find((item) => String(item.path || "").trim().toLowerCase() === targetPath);
  if (matched?.access === "approval" || matched?.access === "full_access" || matched?.access === "read_only") {
    return matched.access;
  }
  const mainWorkspace = workspaceList.find((item) => String(item.level || "").trim() === "main");
  if (mainWorkspace?.access === "approval" || mainWorkspace?.access === "full_access" || mainWorkspace?.access === "read_only") {
    return mainWorkspace.access;
  }
  return "read_only";
});

const supportsFloatingSessionToolbar = computed(() =>
  !props.hideConversationControlPanel
  && !activeConversationIsSystemNotification.value
  && !activeConversationIsRemoteContact.value,
);

const showFloatingSessionToolbar = computed(() => {
  if (!supportsFloatingSessionToolbar.value) return false;
  return sessionControlPanelVisible.value;
});

// ==================== previous user message jump ====================

const scrollNavigationTick = ref(0);
const pendingPreviousUserMessageJumpId = ref("");

function isPreviousUserMessageJumpItem(item: ChatRenderItem | undefined): boolean {
  if (!item || item.kind !== "message") return false;
  const block = item.block;
  if (!block || block.isExtraTextBlock || block.remoteImOrigin) return false;
  const speakerAgentId = String(block.speakerAgentId || "").trim();
  return block.role === "user" || speakerAgentId === "user-persona";
}

function collectPreviousUserMessageJumpTargets() {
  const scrollEl = scrollContainer.value;
  if (!scrollEl || virtualRenderItems.value.length <= 0) return [];
  const rows = virtualizer.value.getVirtualItems();
  const viewportTop = scrollEl.scrollTop;
  let firstVisibleIndex = virtualRenderItems.value.length;
  for (const row of rows) {
    if (row.end > viewportTop + 2) {
      firstVisibleIndex = row.index;
      break;
    }
  }
  const targets: Array<{ index: number; item: ChatRenderItem }> = [];
  for (let index = Math.min(firstVisibleIndex - 1, virtualRenderItems.value.length - 1); index >= 0; index -= 1) {
    const item = virtualRenderItems.value[index];
    if (isPreviousUserMessageJumpItem(item)) targets.push({ index, item });
  }
  return targets;
}

function countPreviousUserMessageJumpItemsBeforeIndex(index: number): number {
  const targetIndex = Math.max(0, Math.min(index, virtualRenderItems.value.length));
  let count = 0;
  for (let currentIndex = targetIndex - 1; currentIndex >= 0; currentIndex -= 1) {
    if (isPreviousUserMessageJumpItem(virtualRenderItems.value[currentIndex])) count += 1;
  }
  return count;
}

const previousUserMessageJumpTargets = computed(() => {
  scrollNavigationTick.value;
  return collectPreviousUserMessageJumpTargets();
});

const previousUserMessageJumpTarget = computed(() => {
  return previousUserMessageJumpTargets.value[0] ?? null;
});

function collectNextUserMessageJumpTargets() {
  const scrollEl = scrollContainer.value;
  if (!scrollEl || virtualRenderItems.value.length <= 0) return [];
  const rows = virtualizer.value.getVirtualItems();
  const viewportBottom = scrollEl.scrollTop + scrollEl.clientHeight;
  let lastVisibleIndex = -1;
  for (const row of rows) {
    if (row.start < viewportBottom - 2) {
      lastVisibleIndex = row.index;
    }
  }
  if (lastVisibleIndex < 0) return [];
  const targets: Array<{ index: number; item: ChatRenderItem }> = [];
  for (let index = Math.min(lastVisibleIndex + 1, virtualRenderItems.value.length - 1); index < virtualRenderItems.value.length; index += 1) {
    const item = virtualRenderItems.value[index];
    if (isPreviousUserMessageJumpItem(item)) targets.push({ index, item });
  }
  return targets;
}

const nextUserMessageJumpTargets = computed(() => {
  scrollNavigationTick.value;
  return collectNextUserMessageJumpTargets();
});

const nextUserMessageJumpTarget = computed(() => {
  return nextUserMessageJumpTargets.value[0] ?? null;
});

const showJumpToPreviousUserMessage = computed(() =>
  userScrollingUp.value && !!previousUserMessageJumpTarget.value,
);

const showJumpToNextUserMessage = computed(() =>
  userScrollingUp.value && !!nextUserMessageJumpTarget.value,
);

// ==================== tool review ====================

const {
  toolReviewPanelOpen, toolReviewBatches, toolReviewCurrentBatchKey,
  toolReviewDetailMap, toolReviewSegmentMap, toolReviewDetailLoadingCallId, toolReviewReviewingCallId,
  toolReviewBatchReviewingKey, toolReviewSubmittingBatchKey, toolReviewErrorText,
  setToolReviewCurrentBatchKey,
  loadToolReviewItemDetail, runToolReviewForCall, runToolReviewForBatch,
  submitToolReviewCode, listToolReviewCommitOptions,
} = useChatToolReviewHandlers({
  activeConversationId: toRef(props, "activeConversationId"),
  toolReviewRefreshTick: toRef(props, "toolReviewRefreshTick"),
  currentDepartmentId: toRef(props, "currentDepartmentId"),
  departmentOptions: toolReviewDepartmentOptions,
  initialPanelOpen: toRef(props, "initialToolReviewPanelOpen"),
  activeTab: toolReviewSidebarActiveTab,
  t, syncViewportMetrics,
  onRefreshMessage: (payload) => emit("refreshToolReviewMessage", payload),
  onToolReviewPanelOpenChange: (open) => emit("toolReviewPanelOpenChange", open),
});
const effectiveToolReviewPanelOpen = computed(() => toolReviewPanelOpen.value);

watch(
  () => [effectiveToolReviewPanelOpen.value, props.chatRightPanelMode] as const,
  ([panelOpen, mode]) => {
    if (!panelOpen || mode !== "reader") clearFileReaderContextReferences();
  },
);

async function openChatReaderDirectoryIfEmpty() {
  await nextTick();
  await nextTick();
  if (!effectiveToolReviewPanelOpen.value || props.chatRightPanelMode !== "reader") return;
  const panel = chatReaderPanelRef.value;
  const workspaceRootPath = String(props.currentWorkspaceRootPath || "").trim();
  if (!panel || !workspaceRootPath) return;
  if (String(panel.activePath || "").trim()) return;
  if (String(panel.directoryRootPath || "").trim()) return;
  await panel.openDirectoryTree(workspaceRootPath);
}

async function refreshChatReaderDirectoryOnWorkspaceChange() {
  await nextTick();
  await nextTick();
  if (!effectiveToolReviewPanelOpen.value || props.chatRightPanelMode !== "reader") return;
  const panel = chatReaderPanelRef.value;
  const workspaceRootPath = String(props.currentWorkspaceRootPath || "").trim();
  if (!panel || !workspaceRootPath) return;
  // 目录树已展开时跟随工作区路径刷新；用户未展开/已关闭时保持关闭，不强制弹出
  if (String(panel.directoryRootPath || "").trim()) {
    await panel.openDirectoryTree(workspaceRootPath);
  }
}

function selectChatRightPanelMode(mode: ChatRightPanelMode) {
  emit("update:chatRightPanelMode", mode);
  emit("toolReviewPanelOpenChange", true);
  // 覆盖模式下不主动展开目录，避免遮挡对话主体
  if (mode === "reader" && !rightPaneOverlay.value) {
    void openChatReaderDirectoryIfEmpty();
  }
}

watch(
  () => [String(props.activeConversationId || "").trim(), String(props.currentWorkspaceRootPath || "").trim()] as const,
  ([conversationId, nextRoot], [prevConversationId, prevRoot]) => {
    // 会话切换会重拉会话级工作区路径，这是正常变化，不应触发目录强刷
    if (conversationId !== prevConversationId) return;
    if (!nextRoot || nextRoot === prevRoot) return;
    void refreshChatReaderDirectoryOnWorkspaceChange();
  },
);

function selectMonitorPanelTab(key: string) {
  if (key !== "delegate" && key !== "tasks" && key !== "tools" && key !== "fastRequests") return;
  emit("update:chatMonitorPanelMode", key);
}

// ==================== delegate status ====================

const {
  delegateStatuses, delegateStatusesErrorText,
  openDelegateArchiveDetail, abortDelegate,
} = useDelegateStatus({
  activeConversationId: toRef(props, "activeConversationId"),
  // 委托状态：打开会话即拉取（工作区 bar 常驻展示活跃委托，不依赖监控面板 delegate tab）
  panelOpen: computed(() => !!String(props.activeConversationId || "").trim()),
  enabled: computed(() => true),
});

// ==================== panes ====================

const panesCleanupFns: Array<() => void> = [];
const {
  leftPaneInLayout, rightPaneInLayout,
  leftPaneOverlay, rightPaneOverlay, leftPaneVisibleWidth, rightPaneVisibleWidth, activePaneResizeSide,
  collapsePreviewSide, collapsePreviewWidth,
  startPaneResize, adjustPaneWidthByKeyboard,
} = useChatPanes({
  chatLayoutRoot, toolReviewPanelOpen: effectiveToolReviewPanelOpen,
  showSideConversationList,
  syncViewportMetrics,
  onPaneWidthsChange: (left, right) => emit("sidePanelWidthsChange", { leftWidth: left, rightWidth: right }),
  onPaneWidthsCommit: (left, right) => emit("sidePanelWidthsCommit", { leftWidth: left, rightWidth: right }),
  onPaneCloseRequest: (side) => {
    if (side === "left") {
      emit("sideConversationListVisibleChange", false);
      return;
    }
    emit("toolReviewPanelOpenChange", false);
  },
  onBeforeUnmountCleanup: (fn) => panesCleanupFns.push(fn),
});

function closeOverlayPanes() {
  if (leftPaneOverlay.value) emit("sideConversationListVisibleChange", false);
  if (rightPaneOverlay.value) emit("toolReviewPanelOpenChange", false);
}

// 打开右侧面板 / 切到 reader / 工作区变化时：无打开文件则自动展开目录；工作区变化时刷新已展开的目录树。
// 覆盖模式（面板浮在内容上）下不主动展开目录，避免遮挡对话主体，由用户自行决定。
watch(
  () => [
    effectiveToolReviewPanelOpen.value,
    props.chatRightPanelMode,
    rightPaneOverlay.value,
    String(props.activeConversationId || "").trim(),
  ] as const,
  ([panelOpen, mode, overlay]) => {
    if (!panelOpen || mode !== "reader" || overlay) return;
    void openChatReaderDirectoryIfEmpty();
  },
);

// ==================== scroll orchestration ====================

const {
  onConversationScroll: handleConversationScrollBase,
  onConversationWheel,
  handleJumpToBottom,
  armProgrammaticScrollPaginationSuppression,
} = useChatScrollOrchestration({
  scrollContainer, chatScrollbarRef: chatScrollbarRef as Ref<{ updateThumb: () => void; hide?: () => void } | null>,
  prepareBottomAlignmentLayout,
  onScroll, scheduleVirtualMeasure,
  scrollConversationToBottomLightweight: scrollVirtualizerToConversationBottomLightweight,
  resetConversationToBottom: resetVirtualizerAtConversationBottom,
  olderHistoryCorrectionAllowed,
  props: {
    hasMoreHistory: toRef(props, "hasMoreHistory"), loadingOlderHistory: toRef(props, "loadingOlderHistory"),
    chatting: toRef(props, "chatting"), conversationBusy: conversationInteractionBusy, frozen: toRef(props, "frozen"),
    activeConversationId: toRef(props, "activeConversationId"),
    conversationScrollToBottomRequest: toRef(props, "conversationScrollToBottomRequest"),
    scrollToBottomBehavior: toRef(props, "scrollToBottomBehavior"),
    renderItems: virtualRenderItems,
  },
  emit: { loadOlderHistory: () => emit("loadOlderHistory"), jumpToConversationBottom: () => emit("jumpToConversationBottom") },
});

function handleConversationScroll() {
  handleConversationScrollBase();
  scrollNavigationTick.value += 1;
}

function handleConversationWheelInput(event: WheelEvent) {
  if (event.shiftKey) {
    handleShiftWheel(event);
    return;
  }
  noteWheelScrollIntent();
  onConversationWheel(event);
}

function scrollToUserMessageTarget(target: { index: number; item: ChatRenderItem }) {
  if (!target) return;
  armProgrammaticScrollPaginationSuppression();
  scheduleVirtualMeasure();
  scrollVirtualizerToIndex(target.index, { align: "start", behavior: "smooth" });
  void nextTick(() => {
    chatScrollbarRef.value?.updateThumb();
    scrollNavigationTick.value += 1;
  });
}

function requestOlderHistoryBeforePreviousUserJump(target: { index: number; item: ChatRenderItem }): boolean {
  if (!props.hasMoreHistory) return false;
  pendingPreviousUserMessageJumpId.value = String(target.item.id || "").trim();
  if (!props.loadingOlderHistory) {
    emit("loadOlderHistory");
  }
  return true;
}

async function continuePendingPreviousUserMessageJump() {
  const pendingId = String(pendingPreviousUserMessageJumpId.value || "").trim();
  if (!pendingId || props.loadingOlderHistory) return;
  await nextTick();
  const index = virtualRenderItems.value.findIndex((item) => String(item.id || "").trim() === pendingId);
  if (index < 0) {
    pendingPreviousUserMessageJumpId.value = "";
    return;
  }
  const item = virtualRenderItems.value[index];
  if (!isPreviousUserMessageJumpItem(item)) {
    pendingPreviousUserMessageJumpId.value = "";
    return;
  }
  const target = { index, item };
  if (props.hasMoreHistory && countPreviousUserMessageJumpItemsBeforeIndex(index) < 1) {
    requestOlderHistoryBeforePreviousUserJump(target);
    return;
  }
  pendingPreviousUserMessageJumpId.value = "";
  scrollToUserMessageTarget(target);
}

function handleJumpToPreviousUserMessage() {
  const target = previousUserMessageJumpTarget.value;
  if (!target) return;
  if (props.hasMoreHistory && previousUserMessageJumpTargets.value.length < 2) {
    requestOlderHistoryBeforePreviousUserJump(target);
    return;
  }
  scrollToUserMessageTarget(target);
}

function handleJumpToNextUserMessage() {
  const target = nextUserMessageJumpTarget.value;
  if (!target) return;
  scrollToUserMessageTarget(target);
}

watch(
  () => props.loadingOlderHistory,
  (loading, wasLoading) => {
    if (loading || !wasLoading) return;
    void continuePendingPreviousUserMessageJump();
  },
);

watch(
  () => props.activeConversationId,
  () => {
    pendingPreviousUserMessageJumpId.value = "";
  },
);

// ==================== image preview ====================

const {
  imagePreviewOpen, imagePreviewDataUrl, imagePreviewLocalPath, imagePreviewZoom, imagePreviewRotation,
  IMAGE_PREVIEW_MIN_ZOOM, IMAGE_PREVIEW_MAX_ZOOM,
  previewOffsetX, previewOffsetY, previewDragging,
  zoomInPreview, zoomOutPreview, resetPreviewZoom, rotatePreviewClockwise,
  onPreviewWheel, openImagePreview, closeImagePreview,
  onPreviewPointerDown, onPreviewPointerMove, onPreviewPointerUp,
} = useChatImagePreview();

const imagePreviewCopyStatus = ref<string>('idle');
const imagePreviewSaveStatus = ref<string>('idle');

async function handleCopyLocalImage(path: string) {
  imagePreviewCopyStatus.value = 'doing';
  try {
    await copyTransportChatImageToClipboard(path);
  } catch (error) {
    console.warn('[预览] 复制图片失败', error);
  } finally {
    imagePreviewCopyStatus.value = 'idle';
  }
}

async function handleSaveLocalImage(path: string) {
  imagePreviewSaveStatus.value = 'doing';
  try {
    await saveTransportChatImageAs(path);
  } catch (error) {
    console.warn('[预览] 保存图片失败', error);
  } finally {
    imagePreviewSaveStatus.value = 'idle';
  }
}

// ==================== conversation actions ====================

function openCodeReviewDialog() {
  clearNativeTextSelection();
  codeReviewErrorText.value = "";
  codeReviewDialogOpen.value = true;
}
function closeCodeReviewDialog() {
  if (toolReviewSubmittingBatchKey.value) return;
  codeReviewDialogOpen.value = false;
  codeReviewErrorText.value = "";
}
async function loadCodeReviewCommitOptions(page = 1) {
  const conversationId = String(props.activeConversationId || "").trim();
  if (!conversationId) return;
  commitOptionsLoading.value = true;
  try {
    const result = await listToolReviewCommitOptions(conversationId, page, commitPageSize.value);
    commitOptions.value = Array.isArray(result.commits) ? result.commits : [];
    commitTotal.value = Number(result.total || 0);
    commitPage.value = Number(result.page || page);
    commitPageSize.value = Number(result.pageSize || commitPageSize.value);
    codeReviewErrorText.value = "";
  } catch (error) {
    commitOptions.value = [];
    codeReviewErrorText.value = t("chat.readCommitFailed");
    console.error("[代码审查] 读取 commit 失败", error);
  } finally {
    commitOptionsLoading.value = false;
  }
}
async function handleSubmitCodeReview(input: { scope: ToolReviewCodeReviewScope; target?: string; departmentId: string; agentId: string }) {
  const conversationId = String(props.activeConversationId || "").trim();
  if (!conversationId || toolReviewSubmittingBatchKey.value) return;
  codeReviewErrorText.value = "";
  const report = await submitToolReviewCode({
    conversationId,
    scope: input.scope,
    target: String(input.target || "").trim() || undefined,
    departmentId: String(input.departmentId || "").trim() || undefined,
    agentId: String(input.agentId || "").trim() || undefined,
  });
  if (!report) {
    codeReviewErrorText.value = t("chat.startCodeReviewFailed");
    return;
  }
  codeReviewDialogOpen.value = false;
}
function openDelegateSummaryPanel() {
  emit("update:chatMonitorPanelMode", "delegate");
  emit("update:chatRightPanelMode", "monitor");
  emit("toolReviewPanelOpenChange", true);
}

async function openActiveConversationInBrowser() {
  const conversationId = String(props.activeConversationId || "").trim();
  if (!conversationId || props.systemNotificationMode) return;
  try {
    const info = await invokeTauri<WebAccessInfo>("transport.accessInfo", {
      input: { forceRefresh: false },
    });
    const localUrl = String(info?.localUrl || "").trim();
    if (!info?.enabled) {
      throw new Error(t("config.networkAccess.disabled"));
    }
    if (!info?.running || !localUrl) {
      throw new Error(t("config.networkAccess.statusUnavailable"));
    }
    const url = new URL(localUrl);
    url.searchParams.set("conversationId", conversationId);
    await openTransportExternalUrl(url.toString());
    linkOpenErrorText.value = "";
  } catch (error) {
    linkOpenErrorText.value = t("status.openLinkFailed", { err: String(error) });
  }
}

function handleSendChat() {
  const conversationId = String(props.activeConversationId || "").trim();
  // 用户按下回车/点发送即视为转正：前端立刻隐藏草稿选择卡，不等后端水位线。
  if (conversationId && activeConversationIsDraft.value) {
    locallyPromotedConversationId.value = conversationId;
  }
  const extraTextBlocks = attachedIdeContextReferences.value.map((item) => String(item.textBlock || "").trim()).filter(Boolean);
  emit("sendChat", extraTextBlocks.length > 0 ? { extraTextBlocks } : undefined);
  clearAttachedIdeContextReferences();
}
function handleConversationListSelect(payload: { conversationId: string; kind?: "local_unarchived" | "remote_im_contact"; remoteContactId?: string }) {
  const id = String(payload?.conversationId || "").trim();
  if (!id || id === String(props.activeConversationId || "").trim()) return;
  const target = (props.conversationItems || props.unarchivedConversationItems).find((item) => String(item.conversationId || "").trim() === id);
  emit("switchConversation", { conversationId: id, kind: payload?.kind || target?.kind, remoteContactId: String(payload?.remoteContactId || target?.remoteContactId || "").trim() || undefined });
  if (leftPaneOverlay.value) {
    emit("sideConversationListVisibleChange", false);
  }
}
function handleConversationRename(payload: { conversationId: string; title: string }) {
  const id = String(payload?.conversationId || "").trim();
  if (id) emit("renameConversation", { conversationId: id, title: String(payload?.title || "").trim() });
}
function handleConversationPinToggle(id: string) { emit("togglePinConversation", String(id || "").trim()); }
function handleConversationArchive(id: string) { emit("archiveConversation", String(id || "").trim()); }
function handleConversationExport(id: string) { emit("exportConversation", String(id || "").trim()); }
function handleConversationDelete(id: string) { emit("deleteConversation", String(id || "").trim()); }
function handleBatchArchiveCompleted(payload: { archivedConversationIds: string[]; activeConversationId?: string }) {
  const currentId = String(props.activeConversationId || "").trim();
  if (!currentId || !payload.archivedConversationIds.includes(currentId)) return;
  const nextId = String(payload.activeConversationId || "").trim();
  if (!nextId || nextId === currentId) return;
  emit("switchConversation", { conversationId: nextId, kind: "local_unarchived" });
}
function handleRebindConversationRecipient() {
  const option = repairRecipientSelectedOption.value;
  const conversationId = String(props.activeConversationId || "").trim();
  const departmentId = String(option?.departmentId || repairRecipientDepartmentId.value || "").trim();
  const agentId = String(option?.agentId || repairRecipientAgentId.value || "").trim();
  if (!conversationId || !departmentId || !agentId) return;
  emit("rebindConversationRecipient", { conversationId, departmentId, agentId });
}

// 「当前项目」分组只允许在 VS Code 侧边栏显示：
// 宿主为 VS Code 时跟随扩展注入的 workspaceRoots（当前打开的项目），
// 与会话工作区（currentWorkspaceRootPath）无关
const currentProjectWorkspaceRoot = computed<string>(() => {
  if (!isVscodeHost()) return "";
  try {
    const hostRoot = getTransportHostContext().workspaceRoots[0];
    return String(hostRoot?.path || "").trim();
  } catch {
    return "";
  }
});

const conversationDisplaySections = computed<ConversationSection[]>(() => {
  const sections = buildConversationSections(props.conversationItems || props.unarchivedConversationItems || [], {
    tab: props.chatLeftPanelMode,
    titles: {
      recent: t("chat.recentConversations"),
      pinned: t("chat.pinnedConversations"),
      other: t("chat.otherConversations"),
      defaultWorkspace: t("chat.defaultWorkspace"),
      currentProject: t("chat.currentProject"),
    },
    locale: locale.value,
    currentWorkspaceRootPath: currentProjectWorkspaceRoot.value,
    activeConversationId: props.activeConversationId,
  });
  // Shift+滚轮跳过「最近会话」区：其中的会话在工作区/频道区会重复出现，
  // 滚动时同一会话滚两遍，顺序对不上列表直觉。
  return sections.filter((section) => section.key !== "recent");
});

// 草稿卡片工作目录下拉选项：当前会话工作区 + 全局配置工作区 + 侧栏按工作区分组的所有目录，按路径去重
const draftWorkspaceOptions = computed<Array<{ id: string; name: string; path: string; access: ShellWorkspace["access"] }>>(() => {
  const deduped = new Map<string, { id: string; name: string; path: string; access: ShellWorkspace["access"] }>();
  const push = (workspace: { id?: string; name?: string; path?: string; access?: ShellWorkspace["access"] }) => {
    const path = stripExtendedPathPrefix(workspace.path);
    if (!path) return;
    const key = path.toLowerCase();
    if (deduped.has(key)) return;
    const rawAccess = String(workspace.access || "").trim();
    deduped.set(key, {
      id: String(workspace.id || "").trim() || `conversation-workspace-${key}`,
      name: String(workspace.name || "").trim() || path,
      path,
      access: rawAccess === "full_access" || rawAccess === "read_only" ? (rawAccess as ShellWorkspace["access"]) : "approval",
    });
  };
  for (const workspace of Array.isArray(props.workspaces) ? props.workspaces : []) push(workspace);
  for (const workspace of Array.isArray(props.configShellWorkspaces) ? props.configShellWorkspaces : []) push(workspace);
  const localItems = (props.conversationItems || props.unarchivedConversationItems || []).filter((item) =>
    item.kind !== "remote_im_contact" && !item.isPinned && !item.isSystemNotificationConversation,
  );
  for (const section of buildWorkspaceConversationSections(localItems, {
    defaultWorkspaceTitle: t("chat.defaultWorkspace"),
    locale: locale.value,
  })) {
    push({ path: section.workspaceRootPath || "", name: section.title, access: "approval" });
  }
  return Array.from(deduped.values());
});

async function handleDraftWorkspaceSave(payload: { path: string; name: string; access: ShellWorkspace["access"]; workMode: ShellWorkMode }) {
  if (!props.saveDraftWorkspaces) return;
  await props.saveDraftWorkspaces(
    [{
      id: `conversation-workspace-${Date.now().toString(36)}`,
      name: payload.name,
      path: payload.path,
      level: "main",
      access: payload.access,
      builtIn: false,
    }],
    Boolean(props.currentWorkspaceAutonomousMode),
    payload.workMode,
  );
}

function handleShiftWheel(event: WheelEvent) {
  if (!event.shiftKey) return;
  event.preventDefault();
  const sections = conversationDisplaySections.value;
  const orderedItems = sections.flatMap((section) => section.items);
  if (orderedItems.length === 0) return;
  const currentId = String(props.activeConversationId || "").trim();
  const currentIndex = orderedItems.findIndex((item) => String(item.conversationId || "").trim() === currentId);
  if (currentIndex < 0) return;
  const direction = event.deltaY > 0 ? 1 : -1;
  const target = orderedItems[currentIndex + direction];
  if (!target) return;
  emit("switchConversation", {
    conversationId: String(target.conversationId || "").trim(),
    kind: target.kind,
    remoteContactId: String(target.remoteContactId || "").trim() || undefined,
  });
}

// ==================== link / copy ====================

async function openChatMessageImagePreview(payload: {
  mime?: string;
  bytesBase64?: string;
  dataUrl?: string;
  localPath?: string;
  src?: string;
  alt?: string;
}) {
  const mime = String(payload?.mime || "").trim() || "image/png";
  const dataUrl = String(payload?.dataUrl || payload?.src || "").trim();
  const bytesBase64 = String(payload?.bytesBase64 || "").trim();
  const localPath = String(payload?.localPath || "").trim();
  if (dataUrl) {
    openImagePreview({ mime, dataUrl, localPath });
    return;
  }
  if (bytesBase64) {
    openImagePreview({ mime, bytesBase64, localPath });
    return;
  }
  if (localPath) {
    if (isAssistantSpacePath(localPath)) {
      try {
        const result = await readTransportChatImage({ path: localPath, mime, original: true });
        const originalDataUrl = String(result?.dataUrl || "").trim();
        if (originalDataUrl) {
          openImagePreview({ mime: result?.mime || mime, dataUrl: originalDataUrl, localPath });
        }
      } catch (error) {
        console.warn("[预览] Assistant Space 图片原图加载失败", { path: localPath, error });
      }
      return;
    }
    openImagePreview({ mime, dataUrl: resolveLocalFileUrl(localPath), localPath });
  }
}

async function handleAssistantLinkClick(event: MouseEvent) {
  const target = event.target as HTMLElement | null;
  const localImage = target?.closest("[data-local-image-path]") as HTMLElement | null;
  if (localImage) {
    const rawPath = localImage.getAttribute("data-local-image-path") || "";
    let path = normalizeLocalLinkHref(rawPath);
    if (!path) return;
    if (!isAssistantSpacePath(path) && !isAbsoluteLocalPath(path)) {
      const root = String(props.currentWorkspaceRootPath || "").trim().replace(/\\/g, "/").replace(/\/$/, "");
      if (root) path = `${root}/${path.replace(/^\.\//, "")}`;
    }
    event.preventDefault(); event.stopPropagation();
    if (!isAssistantSpacePath(path) && await openTransportLocalFileReference(path)) {
      return;
    }
    try {
      const result = await readTransportChatImage({ path, mime: "image/png", original: true });
      const dataUrl = String(result?.dataUrl || "").trim();
      if (!dataUrl) return;
      openImagePreview({ mime: result?.mime || "image/png", dataUrl, localPath: path });
      linkOpenErrorText.value = "";
    } catch (error) {
      linkOpenErrorText.value = t("status.openLinkFailed", { err: String(error) });
    }
    return;
  }
  const anchor = target?.closest("a") as HTMLAnchorElement | null;
  if (!anchor) return;
  const rawHref = anchor.getAttribute("data-href") || anchor.getAttribute("href")?.trim() || "";
  let href = normalizeLocalLinkHref(rawHref);
  if (!href || href === "#") return;
  // 相对路径：基于当前工作目录解析为绝对路径
  if (!isAbsoluteLocalPath(href) && !href.startsWith("http://") && !href.startsWith("https://")) {
    const root = String(props.currentWorkspaceRootPath || "").trim().replace(/\\/g, "/").replace(/\/$/, "");
    if (root) {
      href = `${root}/${href}`;
    }
  }
  const localReference = parseLocalFileReference(href);
  const localPath = localReference?.path || href;
  if (isAbsoluteLocalPath(localPath)) {
    event.preventDefault(); event.stopPropagation();
    if (await openTransportLocalFileReference(href)) {
      return;
    }
    try {
      if (canOpenInFileReader(localPath) || !fileExtensionFromPath(localPath)) {
        await openLocalFileInChatReader(localPath, localReference?.line);
      }
      else { await openTransportLocalDirectory(localPath); }
      linkOpenErrorText.value = "";
    } catch (error) { linkOpenErrorText.value = t("status.openLinkFailed", { err: String(error) }); }
    return;
  }
  if (href.startsWith("http://") || href.startsWith("https://")) {
    event.preventDefault(); event.stopPropagation();
    try { await openTransportExternalUrl(href); linkOpenErrorText.value = ""; }
    catch (error) { linkOpenErrorText.value = t("status.openLinkFailed", { err: String(error) }); }
  }
}

function openLocalFileInChatReader(path: string, line?: number) {
  emit("openChatReaderFile", path, line);
}

async function openFileInReader(path: string, line?: number) {
  const panel = chatReaderPanelRef.value;
  if (!panel) {
    throw new Error("文件阅读面板尚未就绪");
  }
  await panel.openPath(path, { targetLine: line });
}

// ==================== lifecycle ====================

let unlistenFileReaderAddToChat: (() => void) | null = null;

onMounted(() => {
  void nextTick(() => chatScrollbarRef.value?.updateThumb());
  document.addEventListener("pointerdown", closeCompactionSummaryContextMenu);
  document.addEventListener("keydown", handleCompactionSummaryContextMenuKeydown);
  unlistenFileReaderAddToChat = onTransportNotification<IdeContextReferenceItem>("fileReader.addToChat", (payload) => {
    handleAddFileReaderContextReference(payload);
  });
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", closeCompactionSummaryContextMenu);
  document.removeEventListener("keydown", handleCompactionSummaryContextMenuKeydown);
  unlistenFileReaderAddToChat?.();
  unlistenFileReaderAddToChat = null;
  if (transientNoticeTimer) window.clearTimeout(transientNoticeTimer);
  panesCleanupFns.forEach((fn) => fn());
  stopAudioPlayback();
});
</script>

<style scoped>
.ecall-chat-scroll-container {
  overflow-anchor: none;
}

.chat-jump-action-enter-active,
.chat-jump-action-leave-active {
  transition: opacity 120ms ease-out, transform 120ms ease-out;
}

.chat-jump-action-enter-from,
.chat-jump-action-leave-to {
  opacity: 0;
  transform: translateY(4px) scale(0.98);
}

.chat-status-banner-enter-active,
.chat-status-banner-leave-active {
  transition: opacity 160ms ease-out, transform 160ms ease-out;
}

.chat-status-banner-enter-from,
.chat-status-banner-leave-to {
  opacity: 0;
  transform: translateY(6px);
}
</style>

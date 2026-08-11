<template>
  <div class="flex h-full min-h-0 w-full flex-col bg-base-200/35 text-base-content" @click="closeCommitCard">
    <!-- 失败提示 -->
    <div v-if="errorToast" class="pointer-events-none absolute inset-x-0 top-10 z-50 flex justify-center px-4">
      <div class="pointer-events-auto max-w-full rounded bg-error px-3 py-1.5 text-xs text-error-content shadow-lg">
        {{ errorToast }}
      </div>
    </div>

    <!-- 未检测到 git 或非仓库 -->
    <div v-if="detectError" class="flex h-full min-h-0 flex-col items-center justify-center gap-2 px-4 text-center">
      <SquareTerminal class="h-8 w-8 opacity-50" />
      <div class="text-sm font-medium">{{ detectError }}</div>
      <div v-if="detectChecked" class="max-w-56 text-xs leading-relaxed text-base-content/55">
        {{ t('gitPanel.notRepositoryHint') }}
      </div>
    </div>

    <template v-else>
      <!-- 仓库栏：折叠条（标题=当前仓库名）+ 仓库列表（懒加载，可切换） -->
      <div class="flex min-h-0 shrink-0 flex-col overflow-hidden">
        <GitSectionBar v-model="repoCollapsed">
          <template #default>
            <span class="max-w-40 truncate text-xs font-medium opacity-70">{{ currentRepoName }}</span>
          </template>
          <template #actions>
            <button
              type="button"
              class="btn btn-ghost btn-xs h-6 min-h-6 w-6 px-0"
              :title="t('gitPanel.refresh')"
              :disabled="reposLoading || busy"
              @click="refreshRepos"
            >
              <RefreshCw class="h-3.5 w-3.5" :class="{ 'animate-spin': reposLoading }" />
            </button>
          </template>
        </GitSectionBar>
        <div v-if="!repoCollapsed" class="git-panel-scroller max-h-44 min-h-0 overflow-y-auto py-1">
          <div v-if="reposLoading" class="px-3 py-2 text-xs opacity-50">{{ t('gitPanel.loading') }}</div>
          <template v-else>
            <button
              v-for="repo in repos"
              :key="repo.path"
              type="button"
              class="flex w-full items-center gap-1.5 rounded px-2 py-1 text-left text-xs hover:bg-base-300/40"
              :class="{ 'bg-primary/10 text-primary': isCurrentRepo(repo.path) }"
              :disabled="busy"
              @click="switchRepo(repo.path)"
            >
              <GitBranch class="h-3 w-3 shrink-0 opacity-60" />
              <span class="min-w-0 flex-1 truncate">{{ repo.name }}</span>
              <span v-if="isCurrentRepo(repo.path)" class="shrink-0 opacity-50">{{ t('gitPanel.currentRepo') }}</span>
            </button>
            <div v-if="repos.length === 0" class="px-3 py-2 text-xs opacity-50">{{ t('gitPanel.noRepos') }}</div>
          </template>
        </div>
      </div>

      <!-- 上栏：折叠条 + 提交输入框 + 更改/暂存双树 -->
      <div class="flex min-h-0 flex-col overflow-hidden" :class="{ 'flex-1': !changesCollapsed }">
        <GitSectionBar v-model="changesCollapsed">
          <template #default>
            <span class="text-xs font-medium opacity-70">{{ t('gitPanel.changes') }}</span>
          </template>
          <template #actions>
            <button
              type="button"
              class="btn btn-ghost btn-xs h-6 min-h-6 w-6 px-0"
              :title="t('gitPanel.refresh')"
              :disabled="busy"
              @click="refreshChanges"
            >
              <RefreshCw class="h-3.5 w-3.5" :class="{ 'animate-spin': changesRefreshing }" />
            </button>
          </template>
        </GitSectionBar>
        <div v-if="!changesCollapsed" class="flex min-h-0 flex-1 flex-col">
        <!-- 提交输入框（单行起，随内容增高，外观同 input） -->
        <div class="shrink-0 border-b border-base-300 bg-base-200/35 p-2">
          <textarea
            ref="commitInputRef"
            v-model="commitMessage"
            class="textarea w-full git-panel-commit-input"
            :placeholder="t('gitPanel.commitMessagePlaceholder')"
            rows="1"
            @input="autoGrowCommitInput"
          ></textarea>
          <div class="mt-1.5 flex items-center gap-1.5">
            <label class="flex cursor-pointer items-center gap-1 text-xs opacity-70">
              <input v-model="amendCommit" type="checkbox" class="checkbox checkbox-xs" />
              {{ t('gitPanel.amend') }}
            </label>
            <span class="flex-1"></span>
            <button
              type="button"
              class="btn btn-primary btn-xs"
              :disabled="busy || !commitMessage.trim() || stagedEntries.length === 0"
              @click="runCommit"
            >
              {{ t('gitPanel.commit') }}
            </button>
          </div>
        </div>

        <!-- 暂存更改 / 更改 双树 -->
        <div ref="changesScroller" class="git-panel-scroller min-h-0 flex-1 overflow-y-auto py-1">
          <GitChangesGroup
            :title="t('gitPanel.stagedChanges')"
            :entries="stagedEntries"
            :busy="busy"
            action-kind="unstage"
            :action-title="t('gitPanel.unstage')"
            :discard-title="t('gitPanel.discard')"
            :expand-title="t('gitPanel.expandDirectory')"
            :collapse-title="t('gitPanel.collapseDirectory')"
            :collapse-all-title="t('gitPanel.collapseAll')"
            :highlight-path="lastClickedDiffPath"
            @open-diff="openDiff"
            @action="unstagePaths"
            @discard="discardPaths"
          />
          <GitChangesGroup
            :title="t('gitPanel.changes')"
            :entries="unstagedEntries"
            :busy="busy"
            action-kind="stage"
            :action-title="t('gitPanel.stage')"
            :discard-title="t('gitPanel.discard')"
            :expand-title="t('gitPanel.expandDirectory')"
            :collapse-title="t('gitPanel.collapseDirectory')"
            :collapse-all-title="t('gitPanel.collapseAll')"
            :highlight-path="lastClickedDiffPath"
            @open-diff="openDiff"
            @action="stagePaths"
            @discard="discardPaths"
          />
          <div v-if="!busy && totalChanges === 0" class="px-3 py-6 text-center text-xs text-base-content/50">
            {{ t('gitPanel.noChanges') }}
          </div>
        </div>
        </div>
      </div>

      <!-- 分界线：仅两栏都展开时显示，独立于折叠条 -->
      <GitResizeHandle v-if="!changesCollapsed && !historyCollapsed" @resize-start="onHistoryResizeStart" @resize="onHistoryResize" />

      <!-- 下栏：折叠条 + tab 内容 + 底部标签页 -->
      <div
        ref="historySectionRef"
        class="relative flex min-h-0 flex-col"
        :class="{ 'flex-1': !historyCollapsed && (historyHeight === null || changesCollapsed) }"
        :style="!historyCollapsed && !changesCollapsed && historyHeight !== null ? { height: `${historyHeight}px` } : undefined"
      >
        <!-- 下栏折叠条：折叠按钮 + 标题 + 分支名 + 刷新/同步/拉/推 -->
        <GitSectionBar v-model="historyCollapsed">
          <template #default>
            <span class="text-xs font-medium opacity-70">
              {{ activeGitTab === 'commits' ? t('gitPanel.commitHistory') : activeGitTab === 'stashes' ? t('gitPanel.stashesTab') : t('gitPanel.branchesTab') }}
            </span>
          </template>
          <template #actions>
            <button
              v-if="activeGitTab === 'commits'"
              type="button"
              class="flex h-6 min-w-0 items-center gap-1 rounded px-1 text-xs font-medium hover:bg-base-300/40"
              :disabled="busy"
              @click="toggleBranchPicker"
            >
              <GitBranch class="h-3.5 w-3.5 shrink-0 opacity-70" />
              <span class="max-w-28 truncate">{{ currentBranch || t('gitPanel.detachedHead') }}</span>
              <ChevronUp class="h-3 w-3 shrink-0 opacity-50" :class="{ 'rotate-180': !branchPickerOpen }" />
            </button>
            <!-- 分支切换下拉（absolute 相对折叠条） -->
            <div v-if="branchPickerOpen" class="absolute left-0 right-0 top-full z-20 max-h-64 overflow-y-auto border border-base-300 bg-base-100 p-1 shadow-lg">
              <div v-if="branchPickerLoading" class="px-2 py-2 text-xs opacity-50">{{ t('gitPanel.loading') }}</div>
              <template v-else>
                <div v-if="localBranches.length > 0" class="px-2 pb-0.5 pt-1 text-[11px] font-medium opacity-50">{{ t('gitPanel.localBranches') }}</div>
                <button
                  v-for="branch in localBranches"
                  :key="branch.name"
                  type="button"
                  class="flex w-full items-center gap-1.5 rounded px-2 py-1 text-left text-xs hover:bg-base-300/40"
                  :class="{ 'bg-primary/10 text-primary': branch.isCurrent }"
                  :disabled="busy || branch.isCurrent"
                  @click="runCheckoutBranch(branch.name)"
                >
                  <GitBranch class="h-3 w-3 shrink-0 opacity-60" />
                  <span class="min-w-0 flex-1 truncate">{{ branch.name }}</span>
                  <span v-if="branch.isCurrent" class="shrink-0 opacity-50">{{ t('gitPanel.current') }}</span>
                </button>
                <div v-if="remoteBranches.length > 0" class="px-2 pb-0.5 pt-2 text-[11px] font-medium opacity-50">{{ t('gitPanel.remoteBranches') }}</div>
                <button
                  v-for="branch in remoteBranches"
                  :key="branch.name"
                  type="button"
                  class="flex w-full items-center gap-1.5 rounded px-2 py-1 text-left text-xs hover:bg-base-300/40"
                  :disabled="busy"
                  @click="runCheckoutBranch(branch.name)"
                >
                  <Cloud class="h-3 w-3 shrink-0 opacity-60" />
                  <span class="min-w-0 flex-1 truncate">{{ branch.name }}</span>
                </button>
              </template>
            </div>
            <button v-if="activeGitTab === 'commits'" class="btn btn-ghost btn-xs h-6 min-h-6 w-6 px-0" type="button" :title="t('gitPanel.refresh')" :disabled="busy" @click="refreshHistory">
              <RefreshCw class="h-3.5 w-3.5" :class="{ 'animate-spin': busy }" />
            </button>
            <button v-if="activeGitTab === 'commits'" class="btn btn-ghost btn-xs h-6 min-h-6 w-6 px-0" type="button" :title="t('gitPanel.sync')" :disabled="busy" @click="runSync">
              <CloudSync class="h-3.5 w-3.5" :class="{ 'animate-spin': busy }" />
            </button>
            <button v-if="activeGitTab === 'commits'" class="btn btn-ghost btn-xs h-6 min-h-6 w-6 px-0" type="button" :title="t('gitPanel.pull')" :disabled="busy" @click="runPull">
              <ArrowDownToLine class="h-3.5 w-3.5" />
            </button>
            <button v-if="activeGitTab === 'commits'" class="btn btn-ghost btn-xs h-6 min-h-6 w-6 px-0" type="button" :title="t('gitPanel.push')" :disabled="busy" @click="runPush">
              <ArrowUpFromLine class="h-3.5 w-3.5" />
            </button>
          </template>
        </GitSectionBar>
        <div v-if="!historyCollapsed" class="flex min-h-0 flex-1 flex-col">
          <div class="min-h-0 flex-1 overflow-hidden">
          <!-- 提交 tab：commit 表（工具条已上移到折叠条） -->
          <div v-if="activeGitTab === 'commits'" class="flex h-full min-h-0 flex-col">
            <div ref="historyScroller" class="git-panel-scroller min-h-0 flex-1 overflow-y-auto py-1">
              <div v-if="logEntries.length === 0 && !busy" class="px-3 py-6 text-center text-xs text-base-content/50">
                {{ t('gitPanel.noCommits') }}
              </div>
              <div
                v-for="entry in logEntries"
                :key="entry.hash"
                class="text-xs"
              >
                <div
                  class="flex cursor-pointer items-start gap-1.5 rounded px-2 py-1 hover:bg-base-300/40"
                  :class="{ 'bg-base-300/30': expandedCommitHash === entry.hash }"
                  :title="`${entry.hash}\n${entry.author} ${entry.date}`"
                  @click="toggleCommitExpand(entry)"
                  @contextmenu.prevent="openCommitCard(entry, $event)"
                >
                  <ChevronRight class="mt-0.5 h-3 w-3 shrink-0 opacity-50" :class="{ 'rotate-90': expandedCommitHash === entry.hash }" />
                  <span class="min-w-0 flex-1 truncate">{{ entry.message }}</span>
                  <span class="shrink-0 opacity-50">{{ entry.author }}</span>
                </div>
                <!-- 展开的 diff 文件列表 -->
                <div v-if="expandedCommitHash === entry.hash" class="ml-4 border-l border-base-300 py-0.5 pl-2">
                  <div v-if="commitFilesLoading[entry.hash]" class="px-2 py-1 opacity-50">
                    {{ t('gitPanel.loading') }}
                  </div>
                  <div
                    v-for="file in commitFilesMap[entry.hash] || []"
                    :key="file.path"
                    class="flex cursor-pointer items-center gap-1.5 rounded px-2 py-1 hover:bg-base-300/40"
                    @click="openCommitFileDiff(entry, file)"
                  >
                    <span class="shrink-0 font-mono text-[10px] font-bold" :class="commitFileStatusClass(file.status)">{{ commitFileStatusLabel(file.status) }}</span>
                    <span class="min-w-0 truncate">{{ file.path }}</span>
                  </div>
                  <div v-if="!commitFilesLoading[entry.hash] && (commitFilesMap[entry.hash] || []).length === 0" class="px-2 py-1 opacity-50">
                    {{ t('gitPanel.noCommitFiles') }}
                  </div>
                </div>
              </div>
              <!-- 滚动到底自动加载更多 -->
              <div v-if="logHasMore" ref="logSentinel" class="flex justify-center px-2 py-2">
                <span v-if="logLoadingMore" class="loading loading-spinner loading-xs opacity-60" />
                <span v-else class="text-xs opacity-40">{{ t('gitPanel.loadMore') }}</span>
              </div>
            </div>
          </div>

          <!-- 储藏 tab -->
          <div v-else-if="activeGitTab === 'stashes'" class="flex h-full min-h-0 flex-col">
            <div class="shrink-0 border-b border-base-300 p-2">
              <div class="flex items-center gap-1.5">
                <input
                  v-model="stashMessage"
                  class="input input-sm input-bordered min-w-0 flex-1 bg-base-100 text-xs"
                  type="text"
                  :placeholder="t('gitPanel.stashMessagePlaceholder')"
                  @keydown.enter="runStashCreate"
                />
                <button type="button" class="btn btn-sm" :disabled="busy || !stashMessage.trim()" @click="runStashCreate">
                  <Plus class="h-3.5 w-3.5" />
                </button>
              </div>
            </div>
            <div ref="stashScroller" class="git-panel-scroller min-h-0 flex-1 overflow-y-auto py-1">
              <div v-if="stashList.length === 0" class="px-3 py-6 text-center text-xs text-base-content/50">
                {{ t('gitPanel.noStashes') }}
              </div>
              <div v-for="(stash, index) in visibleStashList" :key="stash.reference">
                <div
                  class="flex cursor-pointer items-center gap-1.5 rounded px-2 py-1 text-xs hover:bg-base-300/40"
                  :class="{ 'bg-base-300/30': expandedStashRef === stash.reference }"
                  :title="stash.message"
                  @click="toggleStashExpand(stash)"
                >
                  <ChevronRight class="h-3 w-3 shrink-0 opacity-50" :class="{ 'rotate-90': expandedStashRef === stash.reference }" />
                  <span class="shrink-0 font-mono opacity-60">{{ index }}</span>
                  <span class="min-w-0 flex-1 truncate opacity-80">{{ stash.message }}</span>
                  <button class="btn btn-ghost btn-xs h-5 min-h-5 px-1" type="button" :title="t('gitPanel.stashPop')" :disabled="busy" @click.stop="runStashPop(stash.reference)">
                    <Upload class="h-3 w-3" />
                  </button>
                  <button class="btn btn-ghost btn-xs h-5 min-h-5 px-1" type="button" :title="t('gitPanel.stashDrop')" :disabled="busy" @click.stop="runStashDrop(stash.reference)">
                    <Trash2 class="h-3 w-3" />
                  </button>
                </div>
                <!-- 展开的 diff 文件列表 -->
                <div v-if="expandedStashRef === stash.reference" class="ml-4 border-l border-base-300 py-0.5 pl-2">
                  <div v-if="stashFilesLoading[stash.reference]" class="px-2 py-1 opacity-50">
                    {{ t('gitPanel.loading') }}
                  </div>
                  <div
                    v-for="file in stashFilesMap[stash.reference] || []"
                    :key="file.path"
                    class="flex cursor-pointer items-center gap-1.5 rounded px-2 py-1 hover:bg-base-300/40"
                    @click="openStashFileDiff(stash, file)"
                  >
                    <span class="shrink-0 font-mono text-[10px] font-bold" :class="commitFileStatusClass(file.status)">{{ commitFileStatusLabel(file.status) }}</span>
                    <span class="min-w-0 truncate">{{ file.path }}</span>
                  </div>
                  <div v-if="!stashFilesLoading[stash.reference] && (stashFilesMap[stash.reference] || []).length === 0" class="px-2 py-1 opacity-50">
                    {{ t('gitPanel.noCommitFiles') }}
                  </div>
                </div>
              </div>
              <!-- 滚动到底自动加载更多 -->
              <div v-if="stashHasMore" ref="stashSentinel" class="flex justify-center px-2 py-2">
                <span class="text-xs opacity-40">{{ t('gitPanel.loadMore') }}</span>
              </div>
            </div>
          </div>

          <!-- 分支 tab -->
          <div v-else class="flex h-full min-h-0 flex-col gap-2 p-2">
            <div class="flex items-center gap-1.5">
              <input
                v-model="newBranchName"
                class="input input-sm input-bordered min-w-0 flex-1 bg-base-100 text-xs"
                type="text"
                :placeholder="t('gitPanel.newBranchPlaceholder')"
                @keydown.enter="runBranchCreate"
              />
              <button type="button" class="btn btn-sm" :disabled="busy || !newBranchName.trim()" @click="runBranchCreate">
                <Plus class="h-3.5 w-3.5" />
              </button>
            </div>
            <div ref="branchesScroller" class="git-panel-scroller min-h-0 flex-1 overflow-y-auto">
              <template v-for="row in visibleBranchRows" :key="row.key">
                <div v-if="row.kind === 'header'" class="mb-1 px-1 text-xs font-medium opacity-60" :class="{ 'mt-3': row.grouped }">{{ row.text }}</div>
                <div v-else-if="row.kind === 'branch'" class="flex items-center gap-1.5 rounded px-1.5 py-1 text-sm hover:bg-base-300/40" :class="{ 'bg-primary/10 text-primary': row.branch.isCurrent }">
                  <button v-if="!row.branch.isCurrent" type="button" class="btn btn-ghost btn-xs h-5 min-h-5 w-5 shrink-0 px-0 opacity-70 hover:opacity-100" :title="t('gitPanel.checkoutBranch')" :disabled="busy" @click="runCheckoutBranch(row.branch.name)">
                    <ArrowRightLeft class="h-3 w-3" />
                  </button>
                  <button type="button" class="flex min-w-0 flex-1 items-center gap-1.5 text-left" :class="{ 'cursor-default': row.branch.isCurrent }" @click="selectBranch(row.branch.name)">
                    <GitBranch v-if="row.branch.isCurrent" class="h-3.5 w-3.5 shrink-0" />
                    <span v-else class="h-3.5 w-3.5 shrink-0"></span>
                    <span class="min-w-0 truncate">{{ row.branch.name }}</span>
                  </button>
                  <button v-if="!row.branch.isCurrent" type="button" class="btn btn-error btn-xs h-5 min-h-5 px-1.5 text-[11px]" :disabled="busy" @click="runBranchDelete(row.branch.name)">
                    {{ t('gitPanel.deleteBranch') }}
                  </button>
                </div>
                <div v-else-if="row.kind === 'remote-branch'" class="flex items-center gap-1.5 rounded px-1.5 py-1 text-sm hover:bg-base-300/40">
                  <button type="button" class="btn btn-ghost btn-xs h-5 min-h-5 w-5 shrink-0 px-0 opacity-70 hover:opacity-100" :title="t('gitPanel.checkoutBranch')" :disabled="busy" @click="runCheckoutBranch(row.branch.name)">
                    <ArrowRightLeft class="h-3 w-3" />
                  </button>
                  <button type="button" class="flex min-w-0 flex-1 items-center gap-1.5 text-left" @click="selectBranch(row.branch.name)">
                    <Cloud class="h-3.5 w-3.5 shrink-0 opacity-60" />
                    <span class="min-w-0 truncate">{{ row.branch.name }}</span>
                  </button>
                </div>
                <div v-else class="flex items-center gap-1.5 rounded px-1.5 py-1 text-xs opacity-75">
                  <Cloud class="h-3.5 w-3.5 shrink-0" />
                  <span class="shrink-0 font-medium">{{ row.remote.name }}</span>
                  <span class="min-w-0 truncate font-mono">{{ row.remote.url }}</span>
                </div>
              </template>
              <!-- 滚动到底自动加载更多 -->
              <div v-if="branchHasMore" ref="branchesSentinel" class="flex justify-center px-2 py-2">
                <span class="text-xs opacity-40">{{ t('gitPanel.loadMore') }}</span>
              </div>
            </div>
          </div>
        </div>

          <!-- 底部标签页：提交 / 储藏 / 分支 -->
          <div class="flex h-8 shrink-0 items-center gap-1 border-t border-base-300 bg-base-200/35 px-2">
          <button
            v-for="item in gitPanelTabs"
            :key="item.key"
            type="button"
            class="btn btn-ghost btn-xs h-6 min-h-6 flex-1 justify-center gap-1 px-1 font-medium"
            :class="activeGitTab === item.key ? 'bg-base-100 text-primary shadow-sm' : 'text-base-content/60 hover:bg-base-300/40'"
            @click="selectGitTab(item.key)"
          >
              <component :is="item.icon" class="h-3.5 w-3.5" />
              <span class="truncate">{{ item.label }}</span>
            </button>
          </div>
        </div>
      </div>

      <!-- commit 右键详情卡 -->
      <div
        v-if="commitCard.entry"
        class="fixed z-50 w-72 overflow-hidden rounded-lg border border-base-300 bg-base-100 shadow-xl"
        :style="{ left: `${commitCard.x}px`, top: `${commitCard.y}px` }"
        @click.stop
        @contextmenu.prevent
      >
        <div class="border-b border-base-300 bg-base-200/50 px-3 py-2">
          <div class="max-h-40 overflow-y-auto whitespace-pre-wrap break-words text-xs font-medium">{{ commitCard.entry.message }}</div>
        </div>
        <div class="px-3 py-2 text-xs opacity-70">
          <div>{{ commitCard.entry.author }}</div>
          <div class="mt-0.5">{{ commitCard.entry.date }}</div>
        </div>
        <div class="flex flex-wrap gap-1 border-t border-base-300 px-2 py-2">
          <button type="button" class="btn btn-ghost btn-xs h-6 min-h-6 gap-1 font-mono" @click="copyCommitHash">
            <Copy class="h-3 w-3" />
            {{ commitCard.entry.shortHash }}
          </button>
          <button type="button" class="btn btn-ghost btn-xs h-6 min-h-6" @click="copyCommitMessage">
            {{ t('gitPanel.copyMessage') }}
          </button>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  ArrowDownToLine,
  ArrowRightLeft,
  ArrowUpFromLine,
  ChevronRight,
  ChevronUp,
  Cloud,
  CloudSync,
  Copy,
  GitBranch,
  GitCommitHorizontal,
  History,
  Plus,
  RefreshCw,
  SquareTerminal,
  Trash2,
  Upload,
} from "@lucide/vue";
import { useI18n } from "vue-i18n";
import {
  gitPanelBranchCreate,
  gitPanelBranchDelete,
  gitPanelBranchList,
  gitPanelCheckout,
  gitPanelCheckoutCheck,
  gitPanelCommit,
  gitPanelCommitFiles,
  gitPanelDetect,
  gitPanelDiscard,
  gitPanelLog,
  gitPanelPull,
  gitPanelPush,
  gitPanelRemoteList,
  gitPanelRepos,
  gitPanelStage,
  gitPanelStashCreate,
  gitPanelStashDrop,
  gitPanelStashFiles,
  gitPanelStashList,
  gitPanelStashPop,
  gitPanelStatus,
  gitPanelSync,
  gitPanelUnstage,
  type GitPanelBranchEntry,
  type GitPanelCommitFileEntry,
  type GitPanelLogEntry,
  type GitPanelRemoteEntry,
  type GitPanelRepoEntry,
  type GitPanelRunOutput,
  type GitPanelStashEntry,
  type GitPanelStatusEntry,
} from "../../../services/tauri-api";
import GitChangesGroup from "./GitChangesGroup.vue";
import GitResizeHandle from "./GitResizeHandle.vue";
import GitSectionBar from "./GitSectionBar.vue";

const props = withDefaults(defineProps<{
  workspacePath: string;
  markdownIsDark?: boolean;
  sessionKey?: string;
}>(), {
  markdownIsDark: false,
  sessionKey: "",
});

const emit = defineEmits<{
  (e: "openDiff", payload: { workspacePath: string; path: string; staged: boolean; hash?: string }): void;
}>();

const { t } = useI18n();

const gitPanelTabs = computed(() => [
  { key: "commits", label: t("gitPanel.commitsTab"), icon: History },
  { key: "stashes", label: t("gitPanel.stashesTab"), icon: GitCommitHorizontal },
  { key: "branches", label: t("gitPanel.branchesTab"), icon: GitBranch },
]);

const activeGitTab = ref("commits");
const busy = ref(false);

// ==================== 折叠状态 ====================
const changesCollapsed = ref(false);
const historyCollapsed = ref(false);
const changesRefreshing = ref(false);

// 仓库栏：折叠/展开 + 列表（懒加载，展开首次才扫描）
const repoCollapsed = ref(false);
const repos = ref<GitPanelRepoEntry[]>([]);
const reposLoading = ref(false);
const reposLoaded = ref(false);

// ==================== 分栏高度（分界线拖拽） ====================
const historyHeight = ref<number | null>(null);
const historySectionRef = ref<HTMLElement | null>(null);
let historyResizeStart = 0;
let historyContainerHeight = 0;

// 上栏最小高度：保留折叠条高度，拖拽时不允许覆盖
const CHANGES_MIN_HEIGHT = 40;
// 下栏最小高度：折叠条 + 底部 tab 栏
const HISTORY_MIN_HEIGHT = 96;

function onHistoryResizeStart() {
  historyResizeStart = historySectionRef.value?.offsetHeight ?? 300;
  historyContainerHeight = historySectionRef.value?.parentElement?.offsetHeight ?? 0;
}
function onHistoryResize(dy: number) {
  // 分界线向上拖（dy 为负）→ 下栏变大；向下拖 → 下栏变小
  // 上限：容器高度 - 上栏最小高度，保证上栏折叠条不被覆盖
  const maxHistory = Math.max(HISTORY_MIN_HEIGHT, historyContainerHeight - CHANGES_MIN_HEIGHT);
  historyHeight.value = Math.min(Math.max(HISTORY_MIN_HEIGHT, historyResizeStart - dy), maxHistory);
}
const errorToast = ref("");
let errorToastTimer: number | undefined;

// ==================== 探测状态 ====================
const gitAvailable = ref(false);
const detectChecked = ref(false);
const detectError = ref("");
const repoRoot = ref("");
const currentBranch = ref("");

// 当前仓库名（仓库栏折叠条标题）：repoRoot 最后一段
const currentRepoName = computed(() => {
  if (!repoRoot.value) return t("gitPanel.repoBar");
  const segments = repoRoot.value.replace(/\\/g, "/").split("/").filter(Boolean);
  return segments[segments.length - 1] || repoRoot.value;
});

// ==================== 数据 ====================
const statusEntries = ref<GitPanelStatusEntry[]>([]);
const branches = ref<GitPanelBranchEntry[]>([]);
const remotes = ref<GitPanelRemoteEntry[]>([]);
const stashList = ref<GitPanelStashEntry[]>([]);
const logEntries = ref<GitPanelLogEntry[]>([]);
const logHasMore = ref(false);
const logLoadingMore = ref(false);
const logPageSize = 50;
const logSentinel = ref<HTMLElement | null>(null);
let logObserver: IntersectionObserver | undefined;

// 分支/存储渲染分页（git 命令不支持分页，前端分批渲染，滚动到底自动加载更多）
const branchPageSize = 50;
const branchVisibleCount = ref(50);
const stashVisibleCount = ref(50);
const branchesSentinel = ref<HTMLElement | null>(null);
const stashSentinel = ref<HTMLElement | null>(null);
let branchesObserver: IntersectionObserver | undefined;
let stashObserver: IntersectionObserver | undefined;
const commitFilesMap = ref<Record<string, GitPanelCommitFileEntry[]>>({});
const commitFilesLoading = ref<Record<string, boolean>>({});
const expandedCommitHash = ref("");
const stashFilesMap = ref<Record<string, GitPanelCommitFileEntry[]>>({});
const stashFilesLoading = ref<Record<string, boolean>>({});
const expandedStashRef = ref("");
const branchPickerOpen = ref(false);
const branchPickerLoading = ref(false);

// ==================== 提交区 ====================
const commitMessage = ref("");
const amendCommit = ref(false);
const stashMessage = ref("");
const newBranchName = ref("");
const selectedBranch = ref("");

// 滚动容器
const changesScroller = ref<HTMLElement | null>(null);
const branchesScroller = ref<HTMLElement | null>(null);
const historyScroller = ref<HTMLElement | null>(null);
const stashScroller = ref<HTMLElement | null>(null);

// ==================== 派生状态 ====================
const stagedEntries = computed(() => {
  return statusEntries.value.filter((entry) => {
    const staged = entry.stagedStatus.trim();
    const unstaged = entry.unstagedStatus.trim();
    // ?? 未跟踪属于未暂存；已暂存的是 X 列非空且非 ?
    return staged !== "" && staged !== "?" && !(unstaged === "?" && staged === "?");
  });
});

const unstagedEntries = computed(() => {
  return statusEntries.value.filter((entry) => {
    const staged = entry.stagedStatus.trim();
    const unstaged = entry.unstagedStatus.trim();
    if (staged === "?" && unstaged === "?") return true; // 未跟踪
    return unstaged !== "" && !(staged !== "" && staged !== "?" && unstaged === "");
  });
});

const totalChanges = computed(() => statusEntries.value.length);

const localBranches = computed(() => branches.value.filter((branch) => !branch.isRemote));
const remoteBranches = computed(() => branches.value.filter((branch) => branch.isRemote));
// 渲染分页：分支/存储统一拍平为行序列，滚动到底自动增加可见数
type BranchRow =
  | { kind: "header"; key: string; text: string; grouped: boolean }
  | { kind: "branch"; key: string; branch: GitPanelBranchEntry }
  | { kind: "remote-branch"; key: string; branch: GitPanelBranchEntry }
  | { kind: "remote"; key: string; remote: GitPanelRemoteEntry };

const branchRows = computed<BranchRow[]>(() => {
  const rows: BranchRow[] = [];
  if (localBranches.value.length > 0) {
    rows.push({ kind: "header", key: "header:local", text: t("gitPanel.localBranches"), grouped: false });
    for (const branch of localBranches.value) rows.push({ kind: "branch", key: `local:${branch.name}`, branch });
  }
  if (remoteBranches.value.length > 0) {
    rows.push({ kind: "header", key: "header:remote", text: t("gitPanel.remoteBranches"), grouped: true });
    for (const branch of remoteBranches.value) rows.push({ kind: "remote-branch", key: `remote:${branch.name}`, branch });
  }
  if (remotes.value.length > 0) {
    rows.push({ kind: "header", key: "header:remotes", text: t("gitPanel.remotes"), grouped: true });
    for (const remote of remotes.value) rows.push({ kind: "remote", key: `remotes:${remote.name}`, remote });
  }
  return rows;
});
const visibleBranchRows = computed(() => branchRows.value.slice(0, branchVisibleCount.value));
const branchHasMore = computed(() => branchRows.value.length > branchVisibleCount.value);
const visibleStashList = computed(() => stashList.value.slice(0, stashVisibleCount.value));
const stashHasMore = computed(() => stashList.value.length > stashVisibleCount.value);

// ==================== 错误提示 ====================
function showErrorToast(message: string) {
  errorToast.value = message;
  if (errorToastTimer) window.clearTimeout(errorToastTimer);
  errorToastTimer = window.setTimeout(() => {
    errorToast.value = "";
  }, 5000);
}

// ==================== 输出记录 ====================
function appendOutput(command: string, result: GitPanelRunOutput | null, error: unknown = null) {
  const body: string[] = [];
  if (result?.stdout?.trim()) body.push(result.stdout.trim());
  if (result?.stderr?.trim()) body.push(result.stderr.trim());
  if (error) body.push(error instanceof Error ? error.message : String(error));
  const message = body.join("\n").trim();
  if (message) {
    showErrorToast(message.split("\n").slice(0, 3).join("\n"));
  } else if (result && result.exitCode !== 0) {
    showErrorToast(`git ${command} 失败（退出码 ${result.exitCode}）`);
  }
}

// ==================== 数据加载 ====================
// 按需懒加载：上栏展开才拉更改/暂存，下栏切到对应 tab 才拉历史/存储/分支；
// 各数据首次加载成功置标记，折叠/切走不重复拉取
const statusLoaded = ref(false);
const historyLoaded = ref(false);
const stashesLoaded = ref(false);
const branchesLoaded = ref(false);

// 按当前可见状态加载缺失数据：上栏展开 → 更改/暂存；下栏展开 → 当前 tab 对应数据
function ensureVisibleData() {
  if (!statusLoaded.value && !changesCollapsed.value) {
    void loadStatus();
  }
  if (historyCollapsed.value) return;
  if (activeGitTab.value === "commits" && !historyLoaded.value) {
    void loadHistory();
  } else if (activeGitTab.value === "stashes" && !stashesLoaded.value) {
    void loadStashes();
  } else if (activeGitTab.value === "branches" && !branchesLoaded.value) {
    void Promise.all([loadBranches(), loadRemotes()]);
  }
}

// 折叠/切 tab 变化时重新评估可见数据；immediate 保证初始即展开时也加载
watch([changesCollapsed, activeGitTab, historyCollapsed], () => ensureVisibleData(), {
  immediate: true,
});

// 仓库列表：懒加载 + 后端缓存；force=true 强制重扫
async function loadRepos(force = false) {
  if (reposLoading.value) return;
  reposLoading.value = true;
  try {
    const result = await gitPanelRepos(props.workspacePath, force);
    repos.value = result.repos || [];
    reposLoaded.value = true;
  } catch (error) {
    appendOutput("repos", null, error);
  } finally {
    reposLoading.value = false;
  }
}

function refreshRepos() {
  void loadRepos(true);
}

function isCurrentRepo(path: string): boolean {
  if (!repoRoot.value || !path) return false;
  const norm = (p: string) => p.replace(/\\/g, "/").toLowerCase();
  return norm(path) === norm(repoRoot.value);
}

// 切换仓库：更新 repoRoot，重置各数据加载标记后按当前可见区域重载
function switchRepo(path: string) {
  if (!path || isCurrentRepo(path) || busy.value) return;
  repoRoot.value = path;
  branchPickerOpen.value = false;
  expandedCommitHash.value = "";
  expandedStashRef.value = "";
  commitCard.value = { entry: null, x: 0, y: 0 };
  lastClickedDiffPath.value = "";
  commitFilesMap.value = {};
  statusLoaded.value = false;
  historyLoaded.value = false;
  stashesLoaded.value = false;
  branchesLoaded.value = false;
  ensureVisibleData();
}

// 展开仓库栏才首次扫描（懒加载）；之后只读后端缓存。
// immediate：初始即展开时也要触发加载，否则列表一直空到手动刷新。
watch(
  repoCollapsed,
  (collapsed) => {
    if (!collapsed && !reposLoaded.value) {
      void loadRepos(false);
    }
  },
  { immediate: true },
);

async function loadDetect() {
  try {
    const result = await gitPanelDetect(props.workspacePath);
    gitAvailable.value = !!result.gitAvailable;
    detectChecked.value = !!result.checked;
    repoRoot.value = result.repoRoot || "";
    detectError.value = result.error || (!result.gitAvailable ? t("gitPanel.gitNotInstalled") : !result.repoRoot ? t("gitPanel.notRepository") : "");
  } catch (error) {
    gitAvailable.value = false;
    detectChecked.value = true;
    detectError.value = error instanceof Error ? error.message : String(error);
  }
}

async function loadStatus() {
  if (!repoRoot.value) return;
  try {
    const result = await gitPanelStatus(repoRoot.value);
    statusEntries.value = result.entries || [];
    currentBranch.value = result.branch || "";
    if (result.repoRoot) repoRoot.value = result.repoRoot;
    statusLoaded.value = true;
  } catch (error) {
    appendOutput("status", null, error);
  }
}

async function refreshChanges() {
  changesRefreshing.value = true;
  try {
    await loadStatus();
  } finally {
    changesRefreshing.value = false;
  }
}

async function loadBranches() {
  if (!repoRoot.value) return;
  try {
    branches.value = await gitPanelBranchList(repoRoot.value);
    branchesLoaded.value = true;
  } catch (error) {
    appendOutput("branch -a", null, error);
  }
}

async function loadRemotes() {
  if (!repoRoot.value) return;
  try {
    remotes.value = await gitPanelRemoteList(repoRoot.value);
  } catch (error) {
    appendOutput("remote -v", null, error);
  }
}

async function loadStashes() {
  if (!repoRoot.value) return;
  try {
    stashList.value = await gitPanelStashList(repoRoot.value);
    stashesLoaded.value = true;
  } catch (error) {
    appendOutput("stash list", null, error);
  }
}

async function loadHistory() {
  if (!repoRoot.value) return;
  try {
    const result = await gitPanelLog(repoRoot.value, logPageSize, 0);
    logEntries.value = result.entries || [];
    logHasMore.value = (result.entries || []).length >= logPageSize;
    historyLoaded.value = true;
  } catch (error) {
    appendOutput("log", null, error);
  }
}

async function loadMoreHistory() {
  if (!repoRoot.value || logLoadingMore.value || busy.value || !logHasMore.value) return;
  logLoadingMore.value = true;
  try {
    const result = await gitPanelLog(repoRoot.value, logPageSize, logEntries.value.length);
    const next = result.entries || [];
    logEntries.value = logEntries.value.concat(next);
    logHasMore.value = next.length >= logPageSize;
  } catch (error) {
    appendOutput("log", null, error);
  } finally {
    logLoadingMore.value = false;
  }
}

function observeLogSentinel() {
  logObserver?.disconnect();
  if (!logSentinel.value) return;
  logObserver = new IntersectionObserver(
    (entries) => {
      if (entries[0]?.isIntersecting) void loadMoreHistory();
    },
    { root: historyScroller.value, rootMargin: "120px" },
  );
  logObserver.observe(logSentinel.value);
}

watch(logHasMore, async (hasMore) => {
  if (hasMore) {
    await nextTick();
    observeLogSentinel();
  } else {
    logObserver?.disconnect();
    logObserver = undefined;
  }
});

function observeBranchesSentinel() {
  branchesObserver?.disconnect();
  if (!branchesSentinel.value) return;
  branchesObserver = new IntersectionObserver(
    (entries) => {
      if (entries[0]?.isIntersecting) {
        branchVisibleCount.value += branchPageSize;
      }
    },
    { root: branchesScroller.value, rootMargin: "120px" },
  );
  branchesObserver.observe(branchesSentinel.value);
}

function observeStashSentinel() {
  stashObserver?.disconnect();
  if (!stashSentinel.value) return;
  stashObserver = new IntersectionObserver(
    (entries) => {
      if (entries[0]?.isIntersecting) {
        stashVisibleCount.value += 50;
      }
    },
    { root: stashScroller.value, rootMargin: "120px" },
  );
  stashObserver.observe(stashSentinel.value);
}

watch([branchHasMore, activeGitTab], async ([hasMore, tab]) => {
  if (hasMore && tab === "branches") {
    await nextTick();
    observeBranchesSentinel();
  } else {
    branchesObserver?.disconnect();
    branchesObserver = undefined;
  }
});

watch([stashHasMore, activeGitTab], async ([hasMore, tab]) => {
  if (hasMore && tab === "stashes") {
    await nextTick();
    observeStashSentinel();
  } else {
    stashObserver?.disconnect();
    stashObserver = undefined;
  }
});

async function refreshHistory() {
  if (busy.value) return;
  busy.value = true;
  try {
    await loadHistory();
  } finally {
    busy.value = false;
  }
}

function selectGitTab(key: string) {
  activeGitTab.value = key;
  persistGitTab();
  branchPickerOpen.value = false;
}

// 分支下拉展开时才加载分支/远程数据（提交页底部的分支按钮也能用）
function toggleBranchPicker() {
  branchPickerOpen.value = !branchPickerOpen.value;
  if (branchPickerOpen.value && !branchesLoaded.value) {
    void Promise.all([loadBranches(), loadRemotes()]);
  }
}

// ==================== git 标签页持久化 ====================
// 与文件树目录展开状态共用 sessionKey，记住上次打开的 git 标签
function gitTabStorageKey() {
  const key = String(props.sessionKey || "").trim();
  return key ? `${key}:git-panel-tab` : "";
}

function restoreGitTab() {
  const storageKey = gitTabStorageKey();
  if (!storageKey || typeof window === "undefined") return;
  try {
    const saved = window.localStorage.getItem(storageKey);
    if (saved && gitPanelTabs.value.some((tab) => tab.key === saved)) {
      activeGitTab.value = saved;
    }
  } catch {
    // 读取失败忽略，保持默认
  }
}

function persistGitTab() {
  const storageKey = gitTabStorageKey();
  if (!storageKey || typeof window === "undefined") return;
  try {
    window.localStorage.setItem(storageKey, activeGitTab.value);
  } catch {
    // 写入失败忽略
  }
}

// ==================== 更改操作 ====================
async function runGitAction(command: string, action: () => Promise<GitPanelRunOutput>): Promise<boolean> {
  if (busy.value) return false;
  busy.value = true;
  try {
    const result = await action();
    appendOutput(command, result);
    await loadStatus();
    await loadBranches();
    await loadStashes();
    return true;
  } catch (error) {
    appendOutput(command, null, error);
    return false;
  } finally {
    busy.value = false;
  }
}

function stagePaths(paths: string[]) {
  if (paths.length === 0) return;
  void runGitAction(`add ${paths.join(" ")}`, () => gitPanelStage(repoRoot.value, paths));
}

function unstagePaths(paths: string[]) {
  if (paths.length === 0) return;
  void runGitAction(`restore --staged ${paths.join(" ")}`, () => gitPanelUnstage(repoRoot.value, paths));
}

function discardPaths(paths: string[]) {
  if (paths.length === 0) return;
  if (!window.confirm(t("gitPanel.discardConfirm", { paths: paths.join(", ") }))) return;
  void runGitAction(`restore --staged --worktree ${paths.join(" ")}`, () => gitPanelDiscard(repoRoot.value, paths));
}

async function runCommit() {
  const message = commitMessage.value.trim();
  if (!message || stagedEntries.value.length === 0 || busy.value) return;
  busy.value = true;
  try {
    const result = await gitPanelCommit(repoRoot.value, message, amendCommit.value);
    appendOutput(`commit${amendCommit.value ? " --amend" : ""}`, result);
    commitMessage.value = "";
    amendCommit.value = false;
    resetCommitInputHeight();
    await loadStatus();
    await loadHistory();
  } catch (error) {
    appendOutput("commit", null, error);
  } finally {
    busy.value = false;
  }
}

// ==================== 储藏操作 ====================
async function runStashCreate() {
  const message = stashMessage.value.trim();
  if (!message || busy.value) return;
  const ok = await runGitAction("stash push", () => gitPanelStashCreate(repoRoot.value, message));
  if (ok) stashMessage.value = "";
}

async function runStashPop(stashRef: string) {
  void runGitAction(`stash pop ${stashRef}`, () => gitPanelStashPop(repoRoot.value, stashRef));
}

async function runStashDrop(stashRef: string) {
  if (!window.confirm(t("gitPanel.stashDropConfirm", { reference: stashRef }))) return;
  void runGitAction(`stash drop ${stashRef}`, () => gitPanelStashDrop(repoRoot.value, stashRef));
}

// ==================== 同步操作 ====================
function runSync() {
  void runGitAction("sync (fetch + pull)", () => gitPanelSync(repoRoot.value));
}

function runPush() {
  void runGitAction("push", () => gitPanelPush(repoRoot.value));
}

function runPull() {
  void runGitAction("pull", () => gitPanelPull(repoRoot.value));
}

// ==================== 分支操作 ====================
async function runBranchCreate() {
  const name = newBranchName.value.trim();
  if (!name || busy.value) return;
  busy.value = true;
  try {
    const result = await gitPanelBranchCreate(repoRoot.value, name);
    appendOutput(`branch ${name}`, result);
    newBranchName.value = "";
    await loadBranches();
  } catch (error) {
    appendOutput(`branch ${name}`, null, error);
  } finally {
    busy.value = false;
  }
}

async function runBranchDelete(name: string) {
  if (!window.confirm(t("gitPanel.deleteBranchConfirm", { name }))) return;
  busy.value = true;
  try {
    const result = await gitPanelBranchDelete(repoRoot.value, name);
    appendOutput(`branch -d ${name}`, result);
    await loadBranches();
  } catch (error) {
    appendOutput(`branch -d ${name}`, null, error);
  } finally {
    busy.value = false;
  }
}

function selectBranch(name: string) {
  // 仅选中查看，不切换分支；切换走显式按钮 + 确认
  selectedBranch.value = name;
}

async function runCheckoutBranch(name: string) {
  if (busy.value) return;
  // 预检：工作区未提交文件与目标分支改动有交集则禁止切换
  try {
    const check = await gitPanelCheckoutCheck(repoRoot.value, name);
    if (check.conflictingPaths.length > 0) {
      window.alert(
        t("gitPanel.checkoutBlocked", {
          name,
          paths: check.conflictingPaths.join("\n"),
        }),
      );
      return;
    }
  } catch {
    // 预检失败不阻塞，走默认确认文案
  }
  if (!window.confirm(t("gitPanel.checkoutConfirm", { name }))) return;
  branchPickerOpen.value = false;
  const ok = await runGitAction(`checkout ${name}`, () => gitPanelCheckout(repoRoot.value, name));
  if (ok) {
    await loadHistory();
  }
}

// ==================== commit 右键卡 ====================
const commitCard = ref<{ entry: GitPanelLogEntry | null; x: number; y: number }>({ entry: null, x: 0, y: 0 });

function openCommitCard(entry: GitPanelLogEntry, event: MouseEvent) {
  const cardWidth = 288; // w-72
  const x = Math.min(event.clientX, window.innerWidth - cardWidth - 8);
  const y = Math.min(event.clientY, window.innerHeight - 200);
  commitCard.value = { entry, x: Math.max(8, x), y: Math.max(8, y) };
}

function closeCommitCard() {
  commitCard.value = { entry: null, x: 0, y: 0 };
}

async function copyCommitHash() {
  const hash = commitCard.value.entry?.hash || "";
  if (!hash) return;
  try {
    await navigator.clipboard.writeText(hash);
    closeCommitCard();
  } catch {
    appendOutput("copy hash", null, new Error("复制哈希失败"));
  }
}

async function copyCommitMessage() {
  const message = commitCard.value.entry?.message || "";
  if (!message) return;
  try {
    await navigator.clipboard.writeText(message);
    closeCommitCard();
  } catch {
    appendOutput("copy message", null, new Error("复制提交消息失败"));
  }
}

// ==================== commit 展开 ====================
async function toggleCommitExpand(entry: GitPanelLogEntry) {
  if (expandedCommitHash.value === entry.hash) {
    expandedCommitHash.value = "";
    return;
  }
  expandedCommitHash.value = entry.hash;
  branchPickerOpen.value = false;
  if (commitFilesMap.value[entry.hash] || commitFilesLoading.value[entry.hash]) return;
  commitFilesLoading.value = { ...commitFilesLoading.value, [entry.hash]: true };
  try {
    const result = await gitPanelCommitFiles(repoRoot.value, entry.hash);
    commitFilesMap.value = { ...commitFilesMap.value, [entry.hash]: result.entries || [] };
  } catch (error) {
    appendOutput(`show --name-status ${entry.hash}`, null, error);
    commitFilesMap.value = { ...commitFilesMap.value, [entry.hash]: [] };
  } finally {
    commitFilesLoading.value = { ...commitFilesLoading.value, [entry.hash]: false };
  }
}

function openCommitFileDiff(entry: GitPanelLogEntry, file: GitPanelCommitFileEntry) {
  emit("openDiff", {
    workspacePath: repoRoot.value,
    path: file.path,
    staged: false,
    hash: entry.hash,
  });
}

// ==================== stash 展开 ====================
async function toggleStashExpand(stash: GitPanelStashEntry) {
  if (expandedStashRef.value === stash.reference) {
    expandedStashRef.value = "";
    return;
  }
  expandedStashRef.value = stash.reference;
  branchPickerOpen.value = false;
  if (stashFilesMap.value[stash.reference] || stashFilesLoading.value[stash.reference]) return;
  stashFilesLoading.value = { ...stashFilesLoading.value, [stash.reference]: true };
  try {
    const result = await gitPanelStashFiles(repoRoot.value, stash.reference);
    stashFilesMap.value = { ...stashFilesMap.value, [stash.reference]: result.entries || [] };
  } catch (error) {
    appendOutput(`stash show --name-status ${stash.reference}`, null, error);
    stashFilesMap.value = { ...stashFilesMap.value, [stash.reference]: [] };
  } finally {
    stashFilesLoading.value = { ...stashFilesLoading.value, [stash.reference]: false };
  }
}

function openStashFileDiff(stash: GitPanelStashEntry, file: GitPanelCommitFileEntry) {
  emit("openDiff", {
    workspacePath: repoRoot.value,
    path: file.path,
    staged: false,
    hash: stash.reference,
  });
}

function commitFileStatusLabel(status: string) {
  switch (status) {
    case "A": return "A";
    case "D": return "D";
    case "R": return "R";
    case "C": return "C";
    case "M": return "M";
    default: return "?";
  }
}

function commitFileStatusClass(status: string) {
  switch (status) {
    case "A": return "text-success";
    case "D": return "text-error";
    case "R": return "text-warning";
    case "C": return "text-info";
    default: return "text-warning";
  }
}

// ==================== diff 打开 ====================
// 最后点击打开的 diff 文件路径（用于树行高亮）
const lastClickedDiffPath = ref("");

function openDiff(payload: { path: string; staged: boolean }) {
  lastClickedDiffPath.value = payload.path;
  emit("openDiff", {
    workspacePath: repoRoot.value,
    path: payload.path,
    staged: payload.staged,
  });
}

// ==================== 提交框自适应高度 ====================
// ==================== 生命周期 ====================
const commitInputRef = ref<HTMLTextAreaElement | null>(null);

function autoGrowCommitInput() {
  const el = commitInputRef.value;
  if (!el) return;
  el.style.height = "auto";
  el.style.height = `${Math.min(el.scrollHeight, 96)}px`;
}

function resetCommitInputHeight() {
  const el = commitInputRef.value;
  if (!el) return;
  el.style.height = "";
}

onMounted(() => {
  restoreGitTab();
  void loadDetect().then(() => {
    if (repoRoot.value) {
      ensureVisibleData();
    }
  });
});

// 会话切换时（sessionKey 变化）重新恢复该会话记住的 git 标签
watch(
  () => props.sessionKey,
  () => {
    restoreGitTab();
  },
);

onBeforeUnmount(() => {
  logObserver?.disconnect();
  logObserver = undefined;
  branchesObserver?.disconnect();
  branchesObserver = undefined;
  stashObserver?.disconnect();
  stashObserver = undefined;
});
</script>

<style scoped>
.git-panel-scroller {
  scrollbar-width: thin;
}

.git-panel-graph {
  scrollbar-width: thin;
}

/* 提交输入框：textarea 默认 min-height 5rem 太大，去掉后初始即单行；隐藏滚动条避免占位不对称 */
.git-panel-commit-input {
  min-height: 0;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.git-panel-commit-input::-webkit-scrollbar {
  display: none;
}
</style>

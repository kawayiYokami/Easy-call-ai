<template>
  <div class="flex h-full min-h-0 w-full flex-col bg-base-200/35 text-base-content">
    <!-- 未检测到 git 或非仓库 -->
    <div v-if="detectError" class="flex h-full min-h-0 flex-col items-center justify-center gap-2 px-4 text-center">
      <SquareTerminal class="h-8 w-8 opacity-50" />
      <div class="text-sm font-medium">{{ detectError }}</div>
      <div v-if="detectChecked" class="max-w-56 text-xs leading-relaxed text-base-content/55">
        {{ t('gitPanel.notRepositoryHint') }}
      </div>
    </div>

    <template v-else>
      <!-- 顶部 tab：更改 / 分支 / 历史 / 输出 -->
      <div class="flex h-8 shrink-0 items-center gap-1 border-b border-base-300 bg-base-200/35 px-2">
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

      <!-- 分支行 + 刷新/同步 -->
      <div v-if="activeGitTab === 'changes'" class="flex h-8 shrink-0 items-center gap-1 border-b border-base-300 px-2">
        <span class="flex min-w-0 flex-1 items-center gap-1.5 overflow-hidden text-xs font-medium" :title="repoRoot">
          <GitBranch class="h-3.5 w-3.5 shrink-0 opacity-70" />
          <span class="truncate">{{ currentBranch || t('gitPanel.detachedHead') }}</span>
        </span>
        <button class="btn btn-ghost btn-xs h-6 min-h-6 w-6 px-0" type="button" :title="t('gitPanel.refresh')" :disabled="busy" @click="refreshAll">
          <RefreshCw class="h-3.5 w-3.5" :class="{ 'animate-spin': busy }" />
        </button>
        <button class="btn btn-ghost btn-xs h-6 min-h-6 w-6 px-0" type="button" :title="t('gitPanel.sync')" :disabled="busy" @click="runSync">
          <ArrowDownToLine class="h-3.5 w-3.5" />
        </button>
        <button class="btn btn-ghost btn-xs h-6 min-h-6 w-6 px-0" type="button" :title="t('gitPanel.fetch')" :disabled="busy" @click="runFetch">
          <Download class="h-3.5 w-3.5" />
        </button>
        <button class="btn btn-ghost btn-xs h-6 min-h-6 w-6 px-0" type="button" :title="t('gitPanel.push')" :disabled="busy" @click="runPush">
          <ArrowUpFromLine class="h-3.5 w-3.5" />
        </button>
      </div>

      <div class="min-h-0 flex-1 overflow-hidden">
        <!-- ==================== 更改 ==================== -->
        <div v-if="activeGitTab === 'changes'" class="flex h-full min-h-0 flex-col">
          <div ref="changesScroller" class="git-panel-scroller min-h-0 flex-1 overflow-y-auto py-1">
            <GitChangesGroup
              :title="t('gitPanel.stagedChanges')"
              :entries="stagedEntries"
              :busy="busy"
              action-kind="unstage"
              :action-title="t('gitPanel.unstage')"
              :discard-title="t('gitPanel.discard')"
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
              @open-diff="openDiff"
              @action="stagePaths"
              @discard="discardPaths"
            />
            <div v-if="!busy && totalChanges === 0" class="px-3 py-6 text-center text-xs text-base-content/50">
              {{ t('gitPanel.noChanges') }}
            </div>
          </div>

          <!-- 提交区 -->
          <div class="shrink-0 border-t border-base-300 bg-base-200/35 p-2">
            <textarea
              v-model="commitMessage"
              class="textarea textarea-sm min-h-14 w-full resize-none bg-base-100 text-xs leading-relaxed"
              :placeholder="t('gitPanel.commitMessagePlaceholder')"
              rows="2"
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
            <div class="mt-2 border-t border-base-300 pt-2">
              <div class="flex items-center gap-1 px-0.5 pb-1">
                <span class="flex-1 text-xs font-medium opacity-70">{{ t('gitPanel.stashes') }}</span>
                <button type="button" class="btn btn-ghost btn-xs h-5 min-h-5 px-1.5" :disabled="busy" @click="runStashCreate">
                  <Plus class="h-3 w-3" />
                  <span class="text-[11px]">{{ t('gitPanel.stash') }}</span>
                </button>
              </div>
              <div v-if="stashList.length > 0" class="space-y-0.5">
                <div v-for="stash in stashList" :key="stash.reference" class="flex items-center gap-1 rounded px-1.5 py-0.5 text-xs hover:bg-base-300/40">
                  <span class="min-w-0 flex-1 truncate opacity-80" :title="stash.message">
                    <span class="font-mono opacity-60">{{ stash.reference }}</span>
                    {{ stash.message }}
                  </span>
                  <button class="btn btn-ghost btn-xs h-5 min-h-5 px-1" type="button" :title="t('gitPanel.stashPop')" :disabled="busy" @click="runStashPop(stash.reference)">
                    <Upload class="h-3 w-3" />
                  </button>
                  <button class="btn btn-ghost btn-xs h-5 min-h-5 px-1" type="button" :title="t('gitPanel.stashDrop')" :disabled="busy" @click="runStashDrop(stash.reference)">
                    <Trash2 class="h-3 w-3" />
                  </button>
                </div>
              </div>
            </div>
          </div>
        </div>

        <!-- ==================== 分支 ==================== -->
        <div v-else-if="activeGitTab === 'branches'" class="flex h-full min-h-0 flex-col gap-2 p-2">
          <div class="flex items-center gap-1.5">
            <input
              v-model="newBranchName"
              class="input input-sm input-bordered min-w-0 flex-1 bg-base-100"
              type="text"
              :placeholder="t('gitPanel.newBranchPlaceholder')"
              @keydown.enter="runBranchCreate"
            />
            <button type="button" class="btn btn-sm" :disabled="busy || !newBranchName.trim()" @click="runBranchCreate">
              <Plus class="h-3.5 w-3.5" />
            </button>
          </div>
          <div class="flex items-center gap-1.5">
            <input
              v-model="checkoutReference"
              class="input input-sm input-bordered min-w-0 flex-1 bg-base-100"
              type="text"
              :placeholder="t('gitPanel.checkoutReferencePlaceholder')"
              @keydown.enter="runCheckout"
            />
            <button type="button" class="btn btn-sm" :disabled="busy || !checkoutReference.trim()" @click="runCheckout">
              <GitCommitHorizontal class="h-3.5 w-3.5" />
            </button>
          </div>
          <div ref="branchesScroller" class="git-panel-scroller min-h-0 flex-1 overflow-y-auto">
            <div class="mb-1 px-1 text-xs font-medium opacity-60">{{ t('gitPanel.localBranches') }}</div>
            <div v-for="branch in localBranches" :key="branch.name" class="flex items-center gap-1.5 rounded px-1.5 py-1 text-sm hover:bg-base-300/40" :class="{ 'bg-primary/10 text-primary': branch.isCurrent }">
              <button type="button" class="flex min-w-0 flex-1 items-center gap-1.5 text-left" :disabled="busy || branch.isCurrent" @click="runCheckoutBranch(branch.name)">
                <GitBranch v-if="branch.isCurrent" class="h-3.5 w-3.5 shrink-0" />
                <span v-else class="h-3.5 w-3.5 shrink-0"></span>
                <span class="min-w-0 truncate">{{ branch.name }}</span>
              </button>
              <button v-if="!branch.isCurrent" type="button" class="btn btn-ghost btn-xs h-5 min-h-5 px-1 opacity-60 hover:opacity-100" :title="t('gitPanel.deleteBranch')" :disabled="busy" @click="runBranchDelete(branch.name)">
                <Trash2 class="h-3 w-3" />
              </button>
            </div>
            <div v-if="remoteBranches.length > 0" class="mb-1 mt-3 px-1 text-xs font-medium opacity-60">{{ t('gitPanel.remoteBranches') }}</div>
            <div v-for="branch in remoteBranches" :key="branch.name" class="flex items-center gap-1.5 rounded px-1.5 py-1 text-sm hover:bg-base-300/40">
              <button type="button" class="flex min-w-0 flex-1 items-center gap-1.5 text-left" :disabled="busy" @click="runCheckoutBranch(branch.name)">
                <Cloud class="h-3.5 w-3.5 shrink-0 opacity-60" />
                <span class="min-w-0 truncate">{{ branch.name }}</span>
              </button>
            </div>
            <div v-if="remotes.length > 0" class="mb-1 mt-3 px-1 text-xs font-medium opacity-60">{{ t('gitPanel.remotes') }}</div>
            <div v-for="remote in remotes" :key="remote.name" class="flex items-center gap-1.5 rounded px-1.5 py-1 text-xs opacity-75">
              <Cloud class="h-3.5 w-3.5 shrink-0" />
              <span class="shrink-0 font-medium">{{ remote.name }}</span>
              <span class="min-w-0 truncate font-mono">{{ remote.url }}</span>
            </div>
          </div>
        </div>

        <!-- ==================== 历史 ==================== -->
        <div v-else-if="activeGitTab === 'history'" class="flex h-full min-h-0 flex-col">
          <div class="shrink-0 border-b border-base-300 px-2 py-1.5">
            <div class="flex items-center gap-1">
              <button type="button" class="btn btn-ghost btn-xs h-6 min-h-6 px-1.5" :disabled="busy" @click="refreshHistory">
                <RefreshCw class="h-3.5 w-3.5" />
              </button>
              <span class="text-xs opacity-70">{{ t('gitPanel.commitHistory') }}</span>
            </div>
          </div>
          <div ref="historyScroller" class="git-panel-scroller min-h-0 flex-1 overflow-y-auto py-1">
            <div v-if="logEntries.length === 0 && !busy" class="px-3 py-6 text-center text-xs text-base-content/50">
              {{ t('gitPanel.noCommits') }}
            </div>
            <div
              v-for="entry in logEntries"
              :key="entry.hash"
              class="flex cursor-pointer items-start gap-1.5 rounded px-2 py-1 text-xs hover:bg-base-300/40"
              :title="`${entry.hash}\n${entry.author} ${entry.date}`"
              @click="openCommitDiff(entry)"
            >
              <span class="shrink-0 font-mono opacity-60">{{ entry.shortHash }}</span>
              <span class="min-w-0 flex-1 truncate">{{ entry.message }}</span>
              <span class="shrink-0 opacity-50">{{ entry.author }}</span>
            </div>
            <div v-if="logGraph" class="mt-3 border-t border-base-300 px-2 pt-2">
              <div class="pb-1 text-xs font-medium opacity-60">{{ t('gitPanel.commitGraph') }}</div>
              <pre class="git-panel-graph overflow-x-auto pb-2 font-mono text-[11px] leading-relaxed">{{ logGraph }}</pre>
            </div>
          </div>
        </div>

        <!-- ==================== 输出 ==================== -->
        <div v-else-if="activeGitTab === 'output'" class="flex h-full min-h-0 flex-col">
          <div class="flex h-8 shrink-0 items-center gap-1 border-b border-base-300 px-2">
            <span class="flex-1 text-xs font-medium opacity-70">{{ t('gitPanel.gitOutput') }}</span>
            <button type="button" class="btn btn-ghost btn-xs h-6 min-h-6 px-1.5" :disabled="outputLines.length === 0" @click="outputLines = []">
              <Trash2 class="h-3.5 w-3.5" />
            </button>
          </div>
          <div ref="outputScroller" class="git-panel-scroller min-h-0 flex-1 overflow-y-auto p-2">
            <div v-if="outputLines.length === 0" class="px-2 py-6 text-center text-xs text-base-content/50">
              {{ t('gitPanel.noOutput') }}
            </div>
            <div v-for="(line, idx) in outputLines" :key="idx" class="mb-1 rounded bg-base-100 px-2 py-1">
              <div class="mb-0.5 flex items-center gap-1.5 text-[11px]">
                <span class="shrink-0 font-medium opacity-80">{{ line.command }}</span>
                <span class="ml-auto shrink-0 font-mono" :class="line.exitCode === 0 ? 'text-success' : 'text-error'">exit {{ line.exitCode }}</span>
              </div>
              <pre v-if="line.body" class="max-h-40 overflow-auto whitespace-pre-wrap font-mono text-[11px] leading-relaxed opacity-80">{{ line.body }}</pre>
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from "vue";
import {
  ArrowDownToLine,
  ArrowUpFromLine,
  Cloud,
  Download,
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
  gitPanelCommit,
  gitPanelDetect,
  gitPanelDiscard,
  gitPanelFetch,
  gitPanelLog,
  gitPanelPush,
  gitPanelRemoteList,
  gitPanelStage,
  gitPanelStashCreate,
  gitPanelStashDrop,
  gitPanelStashList,
  gitPanelStashPop,
  gitPanelStatus,
  gitPanelSync,
  gitPanelUnstage,
  type GitPanelBranchEntry,
  type GitPanelLogEntry,
  type GitPanelRemoteEntry,
  type GitPanelRunOutput,
  type GitPanelStashEntry,
  type GitPanelStatusEntry,
} from "../../../services/tauri-api";
import GitChangesGroup from "./GitChangesGroup.vue";

const props = withDefaults(defineProps<{
  workspacePath: string;
  markdownIsDark?: boolean;
}>(), {
  markdownIsDark: false,
});

const emit = defineEmits<{
  (e: "openDiff", payload: { workspacePath: string; path: string; staged: boolean; hash?: string }): void;
}>();

const { t } = useI18n();

const gitPanelTabs = computed(() => [
  { key: "changes", label: t("gitPanel.changesTab"), icon: GitBranch },
  { key: "branches", label: t("gitPanel.branchesTab"), icon: GitCommitHorizontal },
  { key: "history", label: t("gitPanel.historyTab"), icon: History },
  { key: "output", label: t("gitPanel.outputTab"), icon: SquareTerminal },
]);

const activeGitTab = ref("changes");
const busy = ref(false);

// ==================== 探测状态 ====================
const gitAvailable = ref(false);
const detectChecked = ref(false);
const detectError = ref("");
const repoRoot = ref("");
const currentBranch = ref("");

// ==================== 数据 ====================
const statusEntries = ref<GitPanelStatusEntry[]>([]);
const branches = ref<GitPanelBranchEntry[]>([]);
const remotes = ref<GitPanelRemoteEntry[]>([]);
const stashList = ref<GitPanelStashEntry[]>([]);
const logEntries = ref<GitPanelLogEntry[]>([]);
const logGraph = ref("");
const outputLines = ref<{ command: string; body: string; exitCode: number }[]>([]);

// ==================== 提交区 ====================
const commitMessage = ref("");
const amendCommit = ref(false);
const newBranchName = ref("");
const checkoutReference = ref("");

// 滚动容器
const changesScroller = ref<HTMLElement | null>(null);
const branchesScroller = ref<HTMLElement | null>(null);
const historyScroller = ref<HTMLElement | null>(null);
const outputScroller = ref<HTMLElement | null>(null);

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

// ==================== 输出记录 ====================
function appendOutput(command: string, result: GitPanelRunOutput | null, error: unknown = null) {
  const body: string[] = [];
  if (result?.stdout?.trim()) body.push(result.stdout.trim());
  if (result?.stderr?.trim()) body.push(result.stderr.trim());
  if (error) body.push(error instanceof Error ? error.message : String(error));
  outputLines.value.push({
    command,
    body: body.join("\n"),
    exitCode: result?.exitCode ?? -1,
  });
  if (outputLines.value.length > 200) {
    outputLines.value = outputLines.value.slice(-200);
  }
  void nextTickScrollOutput();
}

async function nextTickScrollOutput() {
  await nextTick();
  const scroller = outputScroller.value;
  if (scroller) scroller.scrollTop = scroller.scrollHeight;
}

// ==================== 数据加载 ====================
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
    const result = await gitPanelStatus(props.workspacePath);
    statusEntries.value = result.entries || [];
    currentBranch.value = result.branch || "";
    if (result.repoRoot) repoRoot.value = result.repoRoot;
  } catch (error) {
    appendOutput("status", null, error);
  }
}

async function loadBranches() {
  if (!repoRoot.value) return;
  try {
    branches.value = await gitPanelBranchList(props.workspacePath);
  } catch (error) {
    appendOutput("branch -a", null, error);
  }
}

async function loadRemotes() {
  if (!repoRoot.value) return;
  try {
    remotes.value = await gitPanelRemoteList(props.workspacePath);
  } catch (error) {
    appendOutput("remote -v", null, error);
  }
}

async function loadStashes() {
  if (!repoRoot.value) return;
  try {
    stashList.value = await gitPanelStashList(props.workspacePath);
  } catch (error) {
    appendOutput("stash list", null, error);
  }
}

async function loadHistory() {
  if (!repoRoot.value) return;
  try {
    const result = await gitPanelLog(props.workspacePath, 100);
    logEntries.value = result.entries || [];
    logGraph.value = result.graph || "";
  } catch (error) {
    appendOutput("log", null, error);
  }
}

async function refreshAll() {
  if (busy.value || !repoRoot.value) return;
  busy.value = true;
  try {
    await Promise.all([loadStatus(), loadBranches(), loadRemotes(), loadStashes()]);
  } finally {
    busy.value = false;
  }
}

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
  if (key === "history" && logEntries.value.length === 0) {
    void refreshHistory();
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
  void runGitAction(`add ${paths.join(" ")}`, () => gitPanelStage(props.workspacePath, paths));
}

function unstagePaths(paths: string[]) {
  if (paths.length === 0) return;
  void runGitAction(`restore --staged ${paths.join(" ")}`, () => gitPanelUnstage(props.workspacePath, paths));
}

function discardPaths(paths: string[]) {
  if (paths.length === 0) return;
  if (!window.confirm(t("gitPanel.discardConfirm", { paths: paths.join(", ") }))) return;
  void runGitAction(`restore --staged --worktree ${paths.join(" ")}`, () => gitPanelDiscard(props.workspacePath, paths));
}

async function runCommit() {
  const message = commitMessage.value.trim();
  if (!message || stagedEntries.value.length === 0 || busy.value) return;
  busy.value = true;
  try {
    const result = await gitPanelCommit(props.workspacePath, message, amendCommit.value);
    appendOutput(`commit${amendCommit.value ? " --amend" : ""}`, result);
    commitMessage.value = "";
    amendCommit.value = false;
    await loadStatus();
    await loadHistory();
  } catch (error) {
    appendOutput("commit", null, error);
  } finally {
    busy.value = false;
  }
}

async function runStashCreate() {
  const ok = await runGitAction("stash push", () => gitPanelStashCreate(props.workspacePath, commitMessage.value.trim()));
  if (ok) commitMessage.value = "";
}

async function runStashPop(stashRef: string) {
  void runGitAction(`stash pop ${stashRef}`, () => gitPanelStashPop(props.workspacePath, stashRef));
}

async function runStashDrop(stashRef: string) {
  if (!window.confirm(t("gitPanel.stashDropConfirm", { reference: stashRef }))) return;
  void runGitAction(`stash drop ${stashRef}`, () => gitPanelStashDrop(props.workspacePath, stashRef));
}

// ==================== 同步操作 ====================
function runSync() {
  void runGitAction("sync (fetch + pull)", () => gitPanelSync(props.workspacePath));
}

function runPush() {
  void runGitAction("push", () => gitPanelPush(props.workspacePath));
}

function runFetch() {
  void runGitAction("fetch", () => gitPanelFetch(props.workspacePath));
}

// ==================== 分支操作 ====================
async function runBranchCreate() {
  const name = newBranchName.value.trim();
  if (!name || busy.value) return;
  busy.value = true;
  try {
    const result = await gitPanelBranchCreate(props.workspacePath, name);
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
    const result = await gitPanelBranchDelete(props.workspacePath, name);
    appendOutput(`branch -d ${name}`, result);
    await loadBranches();
  } catch (error) {
    appendOutput(`branch -d ${name}`, null, error);
  } finally {
    busy.value = false;
  }
}

function runCheckoutBranch(name: string) {
  if (busy.value) return;
  void runGitAction(`checkout ${name}`, () => gitPanelCheckout(props.workspacePath, name));
}

async function runCheckout() {
  const reference = checkoutReference.value.trim();
  if (!reference || busy.value) return;
  void runGitAction(`checkout ${reference}`, async () => {
    const result = await gitPanelCheckout(props.workspacePath, reference);
    checkoutReference.value = "";
    return result;
  });
}

// ==================== diff 打开 ====================
function openDiff(payload: { path: string; staged: boolean }) {
  emit("openDiff", {
    workspacePath: repoRoot.value || props.workspacePath,
    path: payload.path,
    staged: payload.staged,
  });
}

async function openCommitDiff(entry: GitPanelLogEntry) {
  emit("openDiff", {
    workspacePath: repoRoot.value || props.workspacePath,
    path: entry.hash,
    staged: false,
    hash: entry.hash,
  });
}

// ==================== 生命周期 ====================
onMounted(() => {
  void loadDetect().then(() => {
    if (repoRoot.value) {
      void refreshAll();
      void loadHistory();
    }
  });
});
</script>

<style scoped>
.git-panel-scroller {
  scrollbar-width: thin;
}

.git-panel-graph {
  scrollbar-width: thin;
}
</style>

// Git 面板外部变化自动刷新的决策逻辑（纯函数，便于粒度测试）。
// 后端 watcher 按路径分类推送信号：workdir（更改/暂存区）、head/refs（提交历史/分支）。
// 决策规则：
// - 无仓库根或事件不属于当前仓库 → 不刷新
// - workdir 信号 → 只刷 status（更改/暂存区）
// - head/refs 信号 → 在 status 基础上加刷当前可见的提交历史/储藏/分支（含 remotes）
// - 下栏折叠时 head/refs 只刷 status（可见区数据未挂载，加载标记会由展开补载兜底）

export type GitPanelRefreshTarget = "status" | "history" | "stashes" | "branches" | "remotes";

export interface GitPanelWatchRefreshContext {
  hasRepoRoot: boolean;
  isCurrentRepo: boolean;
  historyCollapsed: boolean;
  activeGitTab: string;
  workdirChanged: boolean;
  headChanged: boolean;
  refsChanged: boolean;
}

export function decideGitPanelRefreshTargets(
  ctx: GitPanelWatchRefreshContext,
): GitPanelRefreshTarget[] {
  if (!ctx.hasRepoRoot || !ctx.isCurrentRepo) return [];
  const targets: GitPanelRefreshTarget[] = ["status"];
  if (!ctx.headChanged && !ctx.refsChanged) return targets;
  if (ctx.historyCollapsed) return targets;
  if (ctx.activeGitTab === "commits") {
    targets.push("history");
  } else if (ctx.activeGitTab === "stashes") {
    targets.push("stashes");
  } else if (ctx.activeGitTab === "branches") {
    targets.push("branches", "remotes");
  }
  return targets;
}

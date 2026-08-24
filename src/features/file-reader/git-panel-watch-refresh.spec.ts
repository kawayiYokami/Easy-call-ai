import { describe, expect, it } from "vitest";
import { decideGitPanelRefreshTargets, type GitPanelWatchRefreshContext } from "./git-panel-watch-refresh";

function ctx(overrides: Partial<GitPanelWatchRefreshContext> = {}): GitPanelWatchRefreshContext {
  return {
    hasRepoRoot: true,
    isCurrentRepo: true,
    historyCollapsed: false,
    activeGitTab: "commits",
    workdirChanged: true,
    headChanged: false,
    refsChanged: false,
    ...overrides,
  };
}

describe("decideGitPanelRefreshTargets", () => {
  it("workdir 信号只刷 status，不碰历史/分支", () => {
    expect(decideGitPanelRefreshTargets(ctx())).toEqual(["status"]);
  });

  it("head/refs 信号在 status 基础上加刷可见区（commits 可见时刷 history）", () => {
    expect(decideGitPanelRefreshTargets(ctx({ headChanged: true }))).toEqual(["status", "history"]);
    expect(decideGitPanelRefreshTargets(ctx({ refsChanged: true }))).toEqual(["status", "history"]);
    expect(decideGitPanelRefreshTargets(ctx({ headChanged: true, refsChanged: true }))).toEqual([
      "status",
      "history",
    ]);
  });

  it("head/refs 信号按当前可见 tab 加刷对应数据", () => {
    expect(decideGitPanelRefreshTargets(ctx({ activeGitTab: "stashes", headChanged: true }))).toEqual([
      "status",
      "stashes",
    ]);
    expect(decideGitPanelRefreshTargets(ctx({ activeGitTab: "branches", refsChanged: true }))).toEqual([
      "status",
      "branches",
      "remotes",
    ]);
  });

  it("下栏折叠时 head/refs 只刷 status（可见区由展开补载兜底）", () => {
    expect(
      decideGitPanelRefreshTargets(ctx({ historyCollapsed: true, headChanged: true })),
    ).toEqual(["status"]);
  });

  it("无仓库根或事件不属于当前仓库时不刷新", () => {
    expect(decideGitPanelRefreshTargets(ctx({ hasRepoRoot: false }))).toEqual([]);
    expect(decideGitPanelRefreshTargets(ctx({ isCurrentRepo: false, headChanged: true }))).toEqual([]);
  });

  it("未知 tab 时 head/refs 只刷 status，不误刷", () => {
    expect(decideGitPanelRefreshTargets(ctx({ activeGitTab: "unknown", refsChanged: true }))).toEqual([
      "status",
    ]);
  });
});

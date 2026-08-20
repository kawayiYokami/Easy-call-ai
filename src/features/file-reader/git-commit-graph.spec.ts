import { describe, it, expect } from "vitest";
import {
  SWIMLANE_WIDTH,
  computeCommitGraph,
  graphColor,
  laneLine,
  laneX,
  parseRefs,
  renderGraphRowSVG,
} from "./git-commit-graph";
import type { GitPanelLogEntry } from "../../services/tauri-api";

const RAW =
  "802a6be74827584ac003cff2c59ade091ffe968b\x1f802a6be7\x1fkawayiYokami\x1f2026-08-21T01:38:20+08:00\x1f929ae2dfb5513275ea24fe199942b816fbc68c31\x1fHEAD -> main\x1ffix(file-reader): Git 面板折叠条移除冗余标题并支持按钮换行\x1e" +
  "929ae2dfb5513275ea24fe199942b816fbc68c31\x1f929ae2df\x1fkawayiYokami\x1f2026-08-21T01:35:57+08:00\x1fdd0e72aedc3ea0de07cfaa2f0dff16a47f627479\x1f\x1ffix(file-reader): 提交历史右键改为行尾更多操作按钮\x1e" +
  "dd0e72aedc3ea0de07cfaa2f0dff16a47f627479\x1fdd0e72ae\x1fkawayiYokami\x1f2026-08-21T01:30:19+08:00\x1f00f12027b9f2a9ae7f74acf011e2ff4d9878645d\x1f\x1ffix(file-reader): 阻止 GitTree 行右键时弹出原生浏览器菜单\x1e";

function parse(raw: string): GitPanelLogEntry[] {
  return raw
    .split("\u{1e}")
    .filter((r) => r.trim())
    .map((record) => {
      const parts = record.split("\u{1f}");
      return {
        hash: parts[0],
        shortHash: parts[1],
        author: parts[2],
        date: parts[3],
        parents: parts[4] ? parts[4].split(" ").filter(Boolean) : [],
        refs: parts[5] ?? "",
        message: parts[6]?.trim() ?? "",
      };
    });
}

describe("commit graph (VS Code swimlane)", () => {
  it("runs on real data without throwing", () => {
    const entries = parse(RAW);
    expect(entries.length).toBeGreaterThan(0);
    const graph = computeCommitGraph(entries);
    expect(graph.rows.length).toBe(entries.length);
    expect(graph.widthByRow.length).toBe(entries.length);
    graph.rows.forEach((row) => {
      expect(typeof row.circleIndex).toBe("number");
      expect(Array.isArray(row.input)).toBe(true);
      expect(Array.isArray(row.output)).toBe(true);
    });
  });

  it("parses refs", () => {
    const parsed = parseRefs("HEAD -> main, origin/main, tag: v1.0");
    expect(parsed.isHead).toBe(true);
    expect(parsed.branches.map((b) => b.name)).toEqual(["main", "origin/main", "v1.0"]);
    expect(parsed.branches.find((b) => b.name === "v1.0")?.isTag).toBe(true);
  });

  it("keeps first parent in place and pushes new lanes", () => {
    const entries = parse(RAW);
    const graph = computeCommitGraph(entries);
    const top = graph.rows[0];
    // 主干链：第一个提交的 first parent 应留在同一泳道
    expect(top.input.length).toBe(0);
    expect(top.output[0].id).toBe(entries[0].parents[0]);
    // 每行宽度按实际泳道数独立计算
    expect(graph.widthByRow[0]).toBe(SWIMLANE_WIDTH * (Math.max(top.input.length, top.output.length, 1) + 1));
  });

  it("ref badge color matches its lane color", () => {
    const entries = parse(RAW);
    const graph = computeCommitGraph(entries);
    const top = graph.rows[0];
    expect(top.refs.length).toBeGreaterThan(0);
    const mainRef = top.refs.find((r) => r.name === "main");
    expect(mainRef).toBeDefined();
    // first parent 替换时用的是 main 的 ref 色，输出泳道首格与徽章应同色
    expect(top.output[0].colorIndex).toBe(mainRef!.colorIndex);
    expect(typeof mainRef!.colorIndex).toBe("number");
  });

  it("renders svg with node circle", () => {
    const entries = parse(RAW);
    const graph = computeCommitGraph(entries);
    const svg = renderGraphRowSVG(graph.rows[0]);
    expect(svg.startsWith("<svg")).toBe(true);
    expect(svg).toContain("<circle");
    expect(svg).toContain(`width="${graph.widthByRow[0]}"`);
  });

  it("merge second parent color matches its ref color", () => {
    // M 是 merge，第二父 B 带 feature ref 且排在 M 之后：
    // M 的弧线颜色必须与 B 的节点/徽章颜色一致（预分配 ref 色保证）
    const entries: GitPanelLogEntry[] = [
      { hash: "M", shortHash: "M", author: "a", date: "d", parents: ["A", "B"], refs: "", message: "merge" },
      { hash: "A", shortHash: "A", author: "a", date: "d", parents: ["X"], refs: "", message: "a" },
      { hash: "B", shortHash: "B", author: "a", date: "d", parents: ["X"], refs: "feature", message: "b" },
      { hash: "X", shortHash: "X", author: "a", date: "d", parents: [], refs: "", message: "x" },
    ];
    const graph = computeCommitGraph(entries);
    const m = graph.rows[0];
    const bRow = graph.rows[2];
    const bLane = m.output.find((n) => n.id === "B");
    const featureRef = bRow.refs.find((r) => r.name === "feature");
    expect(bLane?.colorIndex).toBeDefined();
    expect(featureRef?.colorIndex).toBeDefined();
    // 弧线泳道色 = B 的 ref 徽章色 = B 行节点色
    expect(bLane?.colorIndex).toBe(featureRef?.colorIndex);
    expect(bRow.circleColorIndex).toBe(featureRef?.colorIndex);
    // merge 弧线渲染取 output 里 B 泳道的颜色
    const svg = renderGraphRowSVG(m);
    expect(svg).toContain(`stroke="${graphColor(featureRef!.colorIndex)}"`);
  });

  it("lane helpers follow vscode spacing", () => {
    expect(laneX(0)).toBe(SWIMLANE_WIDTH * 1);
    expect(laneX(1)).toBe(SWIMLANE_WIDTH * 2);
    const line = laneLine(1, 3);
    expect(line.x).toBe(laneX(1));
    expect(typeof line.color).toBe("string");
  });
});

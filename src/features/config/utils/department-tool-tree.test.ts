import { describe, expect, it } from "vitest";
import type { DepartmentPermissionCatalogItem } from "../../../types/app";
import {
  buildBuiltinToolGroups,
  buildMcpToolGroups,
  splitMcpToolName,
} from "./department-tool-tree";

function item(name: string, description = ""): DepartmentPermissionCatalogItem {
  return { name, description };
}

describe("buildBuiltinToolGroups", () => {
  it("按功能域分组并保持定义顺序", () => {
    const groups = buildBuiltinToolGroups(
      [item("read"), item("write"), item("exec"), item("fetch")],
      () => false,
      (key) => `label:${key}`,
    );
    expect(groups.map((group) => [group.label, group.leaves.map((leaf) => leaf.name)])).toEqual([
      ["label:files", ["read", "write"]],
      ["label:execConfig", ["exec"]],
      ["label:web", ["fetch"]],
    ]);
  });

  it("未知工具落入其他组", () => {
    const groups = buildBuiltinToolGroups([item("mystery_tool"), item("read")], () => false, (key) => key);
    const other = groups.find((group) => group.key === "other");
    expect(other?.leaves.map((leaf) => leaf.name)).toEqual(["mystery_tool"]);
  });

  it("空目录返回空分组", () => {
    expect(buildBuiltinToolGroups([], () => false, () => "x")).toEqual([]);
  });

  it("组状态按叶子启用情况聚合", () => {
    const groups = buildBuiltinToolGroups([item("read"), item("write")], (name) => name === "write", () => "x");
    const files = groups[0];
    expect(files.state).toBe("partial");
    expect(files.leaves.find((leaf) => leaf.name === "write")?.enabled).toBe(true);
    expect(files.leaves.find((leaf) => leaf.name === "read")?.enabled).toBe(false);
  });

  it("全部启用与全部禁用时的组状态", () => {
    const allOn = buildBuiltinToolGroups([item("read"), item("write")], () => true, () => "x");
    expect(allOn[0].state).toBe("all");
    const allOff = buildBuiltinToolGroups([item("read"), item("write")], () => false, () => "x");
    expect(allOff[0].state).toBe("none");
  });
});

describe("splitMcpToolName", () => {
  it("拆分 server 与 tool 短名", () => {
    expect(splitMcpToolName("filesystem::read_file")).toEqual({ server: "filesystem", tool: "read_file" });
  });

  it("无分隔符或位置非法时返回 null", () => {
    expect(splitMcpToolName("plain_tool")).toBeNull();
    expect(splitMcpToolName("::lead")).toBeNull();
    expect(splitMcpToolName("trail::")).toBeNull();
  });
});

describe("buildMcpToolGroups", () => {
  it("按 server 分组且叶子展示短名、保留全名", () => {
    const groups = buildMcpToolGroups(
      [item("fs::read"), item("fs::write"), item("git::status")],
      () => false,
      "其他",
    );
    expect(groups.map((group) => group.label)).toEqual(["fs", "git"]);
    const fs = groups[0];
    expect(fs.leaves.map((leaf) => leaf.displayName)).toEqual(["read", "write"]);
    expect(fs.leaves.every((leaf) => leaf.name.startsWith("fs::"))).toBe(true);
    expect(fs.state).toBe("none");
  });

  it("无前缀工具落入其他组并使用传入的展示名", () => {
    const groups = buildMcpToolGroups([item("bare")], () => false, "其他");
    expect(groups).toHaveLength(1);
    expect(groups[0].key).toBe("other");
    expect(groups[0].label).toBe("其他");
    expect(groups[0].leaves[0]?.displayName).toBe("bare");
  });

  it("组内部分启用时状态为 partial", () => {
    const groups = buildMcpToolGroups(
      [item("s::a"), item("s::b")],
      (name) => name === "s::a",
      "其他",
    );
    expect(groups[0].state).toBe("partial");
  });

  it("server 名为 other 与裸工具并存时分组键不冲突", () => {
    const groups = buildMcpToolGroups(
      [item("other::read"), item("bare")],
      () => false,
      "其他",
    );
    expect(groups).toHaveLength(2);
    const keys = groups.map((group) => group.key);
    expect(new Set(keys).size).toBe(2);
    expect(keys).toContain("server:other");
    expect(keys).toContain("other");
    const serverGroup = groups.find((group) => group.key === "server:other");
    expect(serverGroup?.label).toBe("other");
    expect(serverGroup?.leaves.map((leaf) => leaf.displayName)).toEqual(["read"]);
    const fallbackGroup = groups.find((group) => group.key === "other");
    expect(fallbackGroup?.label).toBe("其他");
    expect(fallbackGroup?.leaves.map((leaf) => leaf.displayName)).toEqual(["bare"]);
  });
});

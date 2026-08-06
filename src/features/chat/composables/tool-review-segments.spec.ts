import { describe, expect, it } from "vitest";
import { groupSegmentsByFile, segmentDiffStats } from "./tool-review-segments";
import type { ToolReviewSegment } from "./use-chat-tool-review";

function segment(overrides: Partial<ToolReviewSegment>): ToolReviewSegment {
  return {
    path: "src/a.ts",
    action: "update",
    diffLines: [],
    ...overrides,
  };
}

describe("groupSegmentsByFile", () => {
  it("groups segments by normalized path", () => {
    const segments = [
      segment({ path: "src/a.ts" }),
      segment({ path: "src/b.ts" }),
      segment({ path: "src/a.ts" }),
    ];
    const groups = groupSegmentsByFile(segments);
    expect(groups.map((group) => group.path)).toEqual(["src/a.ts", "src/b.ts"]);
    expect(groups[0].segments).toHaveLength(2);
    expect(groups[1].segments).toHaveLength(1);
  });

  it("normalizes backslashes to forward slashes", () => {
    const groups = groupSegmentsByFile([segment({ path: "src\\a.ts" })]);
    expect(groups[0].path).toBe("src/a.ts");
  });

  it("keeps first-appearance order for file groups", () => {
    const segments = [
      segment({ path: "src/b.ts" }),
      segment({ path: "src/a.ts" }),
      segment({ path: "src/c.ts" }),
    ];
    const groups = groupSegmentsByFile(segments);
    expect(groups.map((group) => group.path)).toEqual(["src/b.ts", "src/a.ts", "src/c.ts"]);
  });

  it("returns empty array for empty input", () => {
    expect(groupSegmentsByFile([])).toEqual([]);
  });

  it("treats missing path as empty string group", () => {
    const groups = groupSegmentsByFile([segment({ path: "" })]);
    expect(groups[0].path).toBe("");
  });
});

describe("segmentDiffStats", () => {
  it("counts add and remove lines", () => {
    const stats = segmentDiffStats(segment({
      diffLines: [
        "@@ -10,2 +10,3 @@",
        "-old line",
        "+new line",
        "+another new line",
      ],
    }));
    expect(stats).toEqual({ add: 2, remove: 1 });
  });

  it("ignores hunk headers and context lines", () => {
    const stats = segmentDiffStats(segment({
      diffLines: [
        "@@ -1,1 +1,1 @@",
        " context",
        "+++ not a removal",
        "--- not an addition",
      ],
    }));
    expect(stats).toEqual({ add: 0, remove: 0 });
  });

  it("counts empty plus/minus lines", () => {
    const stats = segmentDiffStats(segment({
      diffLines: ["+", "-", "+x"],
    }));
    expect(stats).toEqual({ add: 2, remove: 1 });
  });

  it("handles missing diffLines", () => {
    expect(segmentDiffStats(segment({ diffLines: undefined as unknown as string[] }))).toEqual({ add: 0, remove: 0 });
  });
});

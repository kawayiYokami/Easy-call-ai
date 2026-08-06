import type { ToolReviewSegment } from "./use-chat-tool-review";

export type ToolReviewFileGroup = {
  path: string;
  segments: ToolReviewSegment[];
};

export function groupSegmentsByFile(segments: ToolReviewSegment[]): ToolReviewFileGroup[] {
  const groups = new Map<string, ToolReviewFileGroup>();
  for (const segment of segments ?? []) {
    const normalizedPath = String(segment.path || "").replace(/\\/g, "/").trim();
    const group = groups.get(normalizedPath) || {
      path: normalizedPath,
      segments: [],
    };
    group.segments.push(segment);
    groups.set(normalizedPath, group);
  }
  return Array.from(groups.values());
}

export function segmentDiffStats(segment: ToolReviewSegment): { add: number; remove: number } {
  let add = 0;
  let remove = 0;
  for (const line of segment.diffLines ?? []) {
    const normalized = String(line || "");
    if (normalized.startsWith("+") && !normalized.startsWith("++")) add += 1;
    if (normalized.startsWith("-") && !normalized.startsWith("--")) remove += 1;
  }
  return { add, remove };
}

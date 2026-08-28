export type FileDiffSection = {
  key: string;
  path: string;
  diffText: string;
  added: number;
  removed: number;
  isBinary: boolean;
  isRename: boolean;
  lineCount: number;
  headerRaw: string;
};

const DIFF_GIT_HEADER_RE = /^diff --git a\/.* b\/.*$/gm;

export function parseMultiFileDiff(diffText: string): FileDiffSection[] {
  const raw = String(diffText || "");
  if (!raw.trim()) return [];
  const matches = [...raw.matchAll(DIFF_GIT_HEADER_RE)];
  if (matches.length === 0) {
    const added = (raw.match(/^\+[^+]/gm) || []).length;
    const removed = (raw.match(/^-[^-]/gm) || []).length;
    return [
      {
        key: "single",
        path: "unknown",
        diffText: raw,
        added,
        removed,
        isBinary: /Binary files/.test(raw),
        isRename: false,
        lineCount: raw.split("\n").length,
        headerRaw: "",
      },
    ];
  }
  const sections: FileDiffSection[] = [];
  for (let i = 0; i < matches.length; i += 1) {
    const start = matches[i].index as number;
    const end = i + 1 < matches.length ? (matches[i + 1].index as number) : raw.length;
    const segment = raw.slice(start, end);
    const headerLine = matches[i][0] || "";
    let path = "";
    const headerMatch = headerLine.match(/^diff --git a\/(.*) b\/(.*)$/);
    if (headerMatch) {
      path = String(headerMatch[2] || "").trim();
    }
    if (!path || path === "/dev/null") {
      const plusMatch = segment.match(/^\+\+\+ b\/(.*)$/m);
      if (plusMatch) path = String(plusMatch[1] || "").trim();
    }
    if (!path || path === "/dev/null") {
      const minusMatch = segment.match(/^--- a\/(.*)$/m);
      if (minusMatch) path = String(minusMatch[1] || "").trim();
    }
    if (!path) path = `file-${i + 1}`;
    const isBinary = /Binary files/.test(segment);
    const isRename = /rename from/.test(segment) || /rename to/.test(segment);
    const added = isBinary ? 0 : (segment.match(/^\+[^+]/gm) || []).length;
    const removed = isBinary ? 0 : (segment.match(/^-[^-]/gm) || []).length;
    const lineCount = segment.split("\n").length;
    const key = `${path}::${i}`;
    sections.push({
      key,
      path,
      diffText: segment,
      added,
      removed,
      isBinary,
      isRename,
      lineCount,
      headerRaw: headerLine,
    });
  }
  return sections;
}

export function replaceFileSection(original: string, filePath: string, newSegment: string): string {
  const raw = String(original || "");
  const newSegRaw = String(newSegment || "");
  if (!raw.includes("diff --git ")) {
    return newSegRaw.trim() ? newSegRaw : raw;
  }
  const newSeg = newSegRaw.endsWith("\n") ? newSegRaw : `${newSegRaw}\n`;
  const headerRe = /^diff --git a\/.* b\/.*$/gm;
  const matches = [...raw.matchAll(headerRe)];
  if (matches.length === 0) {
    return newSegRaw.trim() ? newSegRaw : raw;
  }
  let targetIdx = -1;
  for (let i = 0; i < matches.length; i += 1) {
    const line = matches[i][0] || "";
    const m = line.match(/^diff --git a\/(.*) b\/(.*)$/);
    if (m && m[2] === filePath) {
      targetIdx = i;
      break;
    }
    const start = matches[i].index as number;
    const end = i + 1 < matches.length ? (matches[i + 1].index as number) : raw.length;
    const segment = raw.slice(start, end);
    if (segment.includes(`+++ b/${filePath}`) || segment.includes(`--- a/${filePath}`)) {
      targetIdx = i;
      break;
    }
  }
  if (targetIdx === -1) {
    const trimmedOriginal = raw.trimEnd();
    const newTrimmed = newSegRaw.trim();
    if (!newTrimmed) return raw;
    return `${trimmedOriginal}\n${newTrimmed}\n`;
  }
  const start = matches[targetIdx].index as number;
  const end = targetIdx + 1 < matches.length ? (matches[targetIdx + 1].index as number) : raw.length;
  const before = raw.slice(0, start);
  const after = raw.slice(end);
  return `${before}${newSeg}${after}`;
}

export function isMultiFileDiffText(diffText: string): boolean {
  const raw = String(diffText || "");
  const count = (raw.match(/^diff --git /gm) || []).length;
  return count > 1;
}

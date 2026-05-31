export function mergeAssistantText(currentText: string, finalText: string): string {
  const current = String(currentText || "");
  const finalValue = String(finalText || "");
  if (!current) return finalValue;
  if (!finalValue) return current;
  if (finalValue.startsWith(current)) return finalValue;
  return finalValue;
}

export function hasAssistantVisibleOutput(result: {
  assistantText: string;
  reasoningStandard?: string;
  reasoningInline?: string;
}): boolean {
  return (
    !!result.assistantText.trim() ||
    !!(result.reasoningStandard || "").trim() ||
    !!(result.reasoningInline || "").trim()
  );
}

export function consumeClosedMarkdownBlocks(input: string): { chunks: string[]; tail: string } {
  const chunks: string[] = [];
  let cursor = 0;
  let scan = 0;
  let inFence = false;
  let fenceMarker = "";
  let lineStart = 0;
  let lastSafe = 0;
  let prevBlank = false;

  while (scan <= input.length) {
    const isEnd = scan === input.length;
    const ch = isEnd ? "\n" : input[scan];
    if (ch !== "\n" && !isEnd) {
      scan += 1;
      continue;
    }
    const lineEnd = scan;
    const line = input.slice(lineStart, lineEnd);
    const trimmed = line.trimStart();
    const isBlank = line.trim().length === 0;

    if (!inFence && (trimmed.startsWith("```") || trimmed.startsWith("~~~"))) {
      inFence = true;
      fenceMarker = trimmed.startsWith("~~~") ? "~~~" : "```";
    } else if (inFence && fenceMarker && trimmed.startsWith(fenceMarker)) {
      inFence = false;
      lastSafe = isEnd ? lineEnd : lineEnd + 1;
    }

    if (!inFence && prevBlank && !isBlank) {
      const splitAt = lineStart;
      if (splitAt > cursor) {
        const chunk = input.slice(cursor, splitAt).trim();
        if (chunk) chunks.push(chunk);
        cursor = splitAt;
        lastSafe = splitAt;
      }
    }

    prevBlank = isBlank;
    lineStart = scan + 1;
    scan += 1;
  }

  if (lastSafe > cursor) {
    const chunk = input.slice(cursor, lastSafe).trim();
    if (chunk) chunks.push(chunk);
    cursor = lastSafe;
  }

  const tail = input.slice(cursor);
  return { chunks, tail };
}

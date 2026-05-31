export type StreamToolCallView = {
  toolCallId?: string;
  name: string;
  argsText: string;
  status?: "doing" | "done";
};

type ToolStatusDelta = {
  toolName?: string;
  toolCallId?: string;
  toolStatus?: string;
  toolArgs?: string;
};

export function normalizeToolStatusState(value: unknown): "running" | "done" | "failed" | "" {
  const text = String(value || "").trim();
  return text === "running" || text === "done" || text === "failed" ? text : "";
}

export function normalizeStreamToolCallView(item: unknown): StreamToolCallView | null {
  const raw = item && typeof item === "object" ? item as Record<string, unknown> : null;
  const toolCallId = String(raw?.toolCallId || "").trim();
  const name = String(raw?.name || "").trim();
  if (!toolCallId || !name) return null;
  return {
    toolCallId,
    name,
    argsText: String(raw?.argsText || ""),
    status: String(raw?.status || "") === "doing" ? "doing" : "done",
  };
}

export function sameStreamToolCallIdentity(left: StreamToolCallView, right: StreamToolCallView): boolean {
  return !!left.toolCallId && !!right.toolCallId && left.toolCallId === right.toolCallId;
}

function findStreamToolCallViewIndex(
  calls: StreamToolCallView[],
  toolCallId: string | undefined,
): number {
  const normalizedToolCallId = String(toolCallId || "").trim();
  if (!normalizedToolCallId) return -1;
  return calls.findIndex((call) => String(call.toolCallId || "").trim() === normalizedToolCallId);
}

export function applyToolStatusToStreamToolCalls(
  currentCalls: StreamToolCallView[],
  parsed: ToolStatusDelta,
): { calls: StreamToolCallView[]; appended: boolean } {
  const toolName = String(parsed.toolName || "").trim();
  const toolArgs = String(parsed.toolArgs || "").trim();
  const toolCallId = String(parsed.toolCallId || "").trim() || undefined;
  const toolStatus = String(parsed.toolStatus || "").trim();
  const next = currentCalls.map((item) => ({ ...item }));
  if (!toolName || !toolStatus || !toolCallId) {
    return { calls: next, appended: false };
  }
  const index = findStreamToolCallViewIndex(next, toolCallId);
  if (toolStatus === "running") {
    const item: StreamToolCallView = {
      toolCallId,
      name: toolName,
      argsText: toolArgs,
      status: "doing",
    };
    if (index >= 0) {
      next[index] = item;
      return { calls: next, appended: false };
    }
    next.push(item);
    return { calls: next, appended: true };
  }
  if (toolStatus === "done" || toolStatus === "failed") {
    if (index >= 0) {
      next[index] = {
        ...next[index],
        toolCallId: toolCallId || next[index].toolCallId,
        name: toolName,
        argsText: toolArgs || next[index].argsText,
        status: "done",
      };
      return { calls: next, appended: false };
    }
    next.push({
      toolCallId,
      name: toolName,
      argsText: toolArgs,
      status: "done",
    });
    return { calls: next, appended: true };
  }
  return { calls: next, appended: false };
}

export function mergeStreamToolCallsForward(
  currentCalls: StreamToolCallView[],
  incomingCalls: StreamToolCallView[],
): StreamToolCallView[] {
  const current = currentCalls.map((item) => ({ ...item }));
  const incoming = incomingCalls.map((item) => ({ ...item }));
  if (incoming.length === 0) return current;
  if (current.length === 0) return incoming;

  if (
    current.length === incoming.length
    && incoming.every((item, idx) => sameStreamToolCallIdentity(current[idx], item))
  ) {
    return current.map((item, idx) => {
      return { ...item, ...incoming[idx], status: incoming[idx].status };
    });
  }

  const maxOverlap = Math.min(current.length, incoming.length);
  for (let overlap = maxOverlap; overlap > 0; overlap -= 1) {
    const currentStart = current.length - overlap;
    const overlaps = incoming.slice(0, overlap).every((item, idx) =>
      sameStreamToolCallIdentity(current[currentStart + idx], item)
    );
    if (!overlaps) continue;
    const merged = current.map((item, idx) => {
      if (idx < currentStart) return item;
      return { ...item, ...incoming[idx - currentStart], status: incoming[idx - currentStart].status };
    });
    const tail = incoming.slice(overlap);
    if (tail.length === 0) return merged;
    return [...merged, ...tail];
  }

  if (incoming.length > current.length) {
    return incoming;
  }

  const latest = incoming[incoming.length - 1];
  const currentLast = current[current.length - 1];
  if (latest && !sameStreamToolCallIdentity(currentLast, latest)) {
    return [...current, latest];
  }

  return current;
}

export function normalizeStreamToolCallViews(items: unknown): StreamToolCallView[] {
  return Array.isArray(items)
    ? items
      .map((item) => normalizeStreamToolCallView(item))
      .filter((item): item is StreamToolCallView => !!item)
    : [];
}

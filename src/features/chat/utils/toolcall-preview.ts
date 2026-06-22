import type { ChatActivityItem } from "../../../types/app";

export function buildToolcallPreviewMap(
  activityItems: ChatActivityItem[],
  noArgsText: string,
): Record<string, { title: string; body: string }> {
  void noArgsText;
  const previews: Record<string, { title: string; body: string }> = {};
  for (const item of activityItems) {
    if (item.kind !== "tool") continue;
    const toolCallId = String(item.toolCallId || "").trim();
    if (!toolCallId) continue;
    const title = String(item.name || "").trim();
    previews[toolCallId] = {
      title,
      body: "",
    };
  }
  return previews;
}

import { describe, expect, it } from "vitest";
import type { ChatActivityItem } from "../src/types/app";
import { buildToolcallPreviewMap } from "../src/features/chat/utils/toolcall-preview";

describe("toolcall preview", () => {
  it("prefers args text over tool result text", () => {
    const activityItems: ChatActivityItem[] = [{
      kind: "tool",
      id: "tool-1",
      toolCallId: "tool-1",
      name: "operate",
      argsText: "{\"action\":\"wait\"}",
      resultText: "工具已执行，结果在这里",
      status: "done",
    }];

    expect(buildToolcallPreviewMap(activityItems, "暂无工具结果")).toEqual({
      "tool-1": {
        title: "operate",
        body: "{\"action\":\"wait\"}",
      },
    });
  });

  it("shows no-args text when args text is absent", () => {
    const activityItems: ChatActivityItem[] = [{
      kind: "tool",
      id: "tool-1",
      toolCallId: "tool-1",
      name: "operate",
      resultText: "工具已执行，结果在这里",
      status: "doing",
    }];

    expect(buildToolcallPreviewMap(activityItems, "暂无工具结果")).toEqual({
      "tool-1": {
        title: "operate",
        body: "暂无工具结果",
      },
    });
  });
});

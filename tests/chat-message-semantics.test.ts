import { describe, expect, it } from "vitest";
import type { ChatMessage } from "../src/types/app";
import {
  appendReasoningDeltaToStreamBlocks,
  appendTextDeltaToStreamBlocks,
  appendReasoningToStreamActivityItems,
  applyAssistantToolEventToStreamBlocks,
  applyAssistantToolResultToStreamBlocks,
  assistantTextFromStreamBlocks,
  assistantStreamBlocksFromMessageForDisplay,
  inspectUndoablePatchCalls,
  normalizeMessageToolHistoryEvents,
  projectChatActivityForDisplay,
  projectMessageForDisplay,
  projectStreamingChatActivityForDisplay,
  streamBlocksActivitySignature,
  streamBlocksToToolHistoryEvents,
} from "../src/utils/chat-message-semantics";

function textMessage(id: string, role: ChatMessage["role"], text: string): ChatMessage {
  return {
    id,
    role,
    createdAt: "2026-03-28T10:00:00Z",
    parts: [{ type: "text", text }],
  };
}

describe("chat-message semantics", () => {
  it("projects display fields from unified message semantics", () => {
    const message: ChatMessage = {
      ...textMessage("a-1", "assistant", "我查好了"),
      providerMeta: {
        origin: {
          kind: "remote_im",
          sender_name: "张三",
          contact_name: "测试群",
          contact_type: "group",
          channel_id: "remote-im-1",
          contact_id: "group-1",
        },
      },
      toolCall: [
        {
          role: "assistant",
          reasoning_content: "先检查上下文",
          tool_calls: [{
            id: "fc_1",
            call_id: "call_1",
            type: "function",
            function: {
              name: "remote_im_send",
              arguments: "{\"text\":\"你好\"}",
            },
          }],
        },
      ],
    };

    const projection = projectMessageForDisplay(message);

    expect(projection.text).toBe("[toolcall:fc_1]\n\n我查好了");
    expect(projection.activityItems[0]).toMatchObject({ kind: "reasoning", text: "先检查上下文" });
    expect(projection.toolCallCount).toBe(1);
    expect(projection.lastToolName).toBe("remote_im_send");
    expect(projection.toolCalls[0]).toEqual({
      name: "remote_im_send",
      argsText: "{\"text\":\"你好\"}",
    });
    expect(projection.remoteImOrigin).toEqual({
      senderName: "张三",
      remoteContactName: "测试群",
      remoteContactType: "group",
      channelId: "remote-im-1",
      contactId: "group-1",
    });
  });

  it("renders meme annotation images as standalone markdown blocks", () => {
    const message: ChatMessage = {
      ...textMessage("a-meme-block", "assistant", "实挺漂亮的 :纳西妲: 三层融合"),
      memeAnnotations: [{ meme: ":纳西妲:", path: ".meme/纳西妲.webp" }],
    };

    expect(projectMessageForDisplay(message).text).toBe(
      "实挺漂亮的\n\n![纳西妲](.meme/纳西妲.webp)\n\n三层融合",
    );
  });

  it("reads downloaded image markers through the stored media path", () => {
    const message: ChatMessage = {
      ...textMessage("remote-img", "user", "这个指的是什么？"),
      parts: [
        { type: "text", text: "这个指的是什么？" },
        { type: "image", mime: "image/png", bytesBase64: "@download:contact-1/example.png" },
      ],
    };

    expect(projectMessageForDisplay(message).images).toEqual([
      {
        mime: "image/png",
        bytesBase64: undefined,
        mediaRef: "@download:contact-1/example.png",
      },
    ]);
  });

  it("drops orphan assistant tool call only for prompt replay view", () => {
    const message: ChatMessage = {
      ...textMessage("a-2", "assistant", "处理中"),
      toolCall: [
        {
          role: "assistant",
          tool_calls: [{
            id: "fc_2",
            type: "function",
            function: {
              name: "bing_search",
              arguments: "{\"query\":\"rust\"}",
            },
          }],
        },
      ],
    };

    expect(normalizeMessageToolHistoryEvents(message, "display")).toHaveLength(1);
    expect(normalizeMessageToolHistoryEvents(message, "prompt")).toHaveLength(0);
  });

  it("trims partial tool groups only for prompt replay view", () => {
    const message: ChatMessage = {
      ...textMessage("a-2-partial", "assistant", "处理中"),
      toolCall: [
        {
          role: "assistant",
          reasoning_content: "先同时读取两个文件",
          tool_calls: [
            {
              id: "call-a",
              call_id: "call-a",
              type: "function",
              function: { name: "read", arguments: "{\"path\":\"a.rs\"}" },
            },
            {
              id: "call-b",
              call_id: "call-b",
              type: "function",
              function: { name: "read", arguments: "{\"path\":\"b.rs\"}" },
            },
          ],
        },
        { role: "tool", tool_call_id: "call-a", content: "工具 A 结果" },
      ],
    };

    const displayEvents = normalizeMessageToolHistoryEvents(message, "display");
    const promptEvents = normalizeMessageToolHistoryEvents(message, "prompt");

    expect(displayEvents).toHaveLength(2);
    expect(displayEvents[0]?.toolCalls).toHaveLength(2);
    expect(promptEvents).toHaveLength(2);
    expect(promptEvents[0]?.toolCalls).toHaveLength(1);
    expect(promptEvents[0]?.toolCalls[0]?.invocationId).toBe("call-a");
    expect(promptEvents[1]).toMatchObject({ role: "tool", toolCallId: "call-a" });
  });

  it("uses only event-level reasoning for display activity", () => {
    const message: ChatMessage = {
      ...textMessage("a-4", "assistant", "终端版本是 PowerShell 7.5.4"),
      toolCall: [
        {
          role: "assistant",
          reasoning_content: "先调用终端工具查看 PowerShell 版本。",
          tool_calls: [{
            id: "fc_3",
            type: "function",
            function: {
              name: "shell_command",
              arguments: "{\"command\":\"$PSVersionTable.PSVersion\"}",
            },
          }],
        },
        {
          role: "tool",
          tool_call_id: "fc_3",
          content: "7.5.4",
        },
      ],
    };

    const projection = projectMessageForDisplay(message);

    expect(projection.activityItems).toHaveLength(2);
    expect(projection.activityItems[0]).toMatchObject({
      kind: "reasoning",
      text: "先调用终端工具查看 PowerShell 版本。",
    });
  });

  it("projects final text part reasoning after tool activity", () => {
    const message: ChatMessage = {
      ...textMessage("a-final-reasoning", "assistant", "最终回答"),
      parts: [{ type: "text", text: "最终回答", reasoning_content: "最终思考" }],
      toolCall: [
        {
          role: "assistant",
          reasoning_content: "工具思考",
          tool_calls: [{
            id: "fc_final",
            type: "function",
            function: {
              name: "read",
              arguments: "{}",
            },
          }],
        },
        {
          role: "tool",
          tool_call_id: "fc_final",
          content: "工具结果",
        },
      ],
    };

    const blocks = assistantStreamBlocksFromMessageForDisplay(message, "最终回答");

    expect(blocks).toHaveLength(2);
    expect(blocks[0]).toMatchObject({ reasoning: "工具思考", text: "[toolcall:fc_final]" });
    expect(blocks[0]?.tools).toHaveLength(1);
    expect(blocks[1]).toMatchObject({ reasoning: "最终思考", text: "最终回答", tools: [] });
  });

  it("projects message-level activity when there are no tool events", () => {
    const message: ChatMessage = {
      ...textMessage("a-activity", "assistant", "不太确定，展开说说？"),
      activityItems: [{
        kind: "reasoning",
        id: "stream-reasoning-0",
        text: "先判断用户提到的工具指代。",
      }],
    };

    const projection = projectMessageForDisplay(message);

    expect(projection.text).toBe("不太确定，展开说说？");
    expect(projection.activityItems).toHaveLength(1);
    expect(projection.activityItems[0]).toMatchObject({
      kind: "reasoning",
      id: "stream-reasoning-0",
      text: "先判断用户提到的工具指代。",
    });
    expect(projection.activityStatus).toBe("complete");
  });

  it("projects final text part reasoning when there are no tools", () => {
    const message: ChatMessage = {
      ...textMessage("a-event-activity", "assistant", "不太确定，展开说说？"),
      parts: [{
        type: "text",
        text: "不太确定，展开说说？",
        reasoning_content: "先判断用户提到的工具指代。",
      }],
    };

    const projection = projectMessageForDisplay(message);

    expect(projection.activityItems).toHaveLength(1);
    expect(projection.activityItems[0]).toMatchObject({
      kind: "reasoning",
      text: "先判断用户提到的工具指代。",
    });
    expect(projection.toolCallCount).toBe(0);
  });

  it("merges toolcall assistant texts with final assistant text for display", () => {
    const message: ChatMessage = {
      ...textMessage("a-toolcall-text", "assistant", "最后汇总"),
      parts: [{ type: "text", text: "最后汇总" }],
      toolCall: [
        {
          role: "assistant",
          content: "先说明第一步",
          tool_calls: [{
            id: "call_1",
            type: "function",
            function: {
              name: "read",
              arguments: "{\"path\":\"a.txt\"}",
            },
          }],
        },
        {
          role: "tool",
          tool_call_id: "call_1",
          content: "文件内容",
        },
        {
          role: "assistant",
          content: "再说明第二步",
          tool_calls: [{
            id: "call_2",
            type: "function",
            function: {
              name: "read",
              arguments: "{\"path\":\"b.txt\"}",
            },
          }],
        },
        {
          role: "tool",
          tool_call_id: "call_2",
          content: "更多内容",
        },
      ],
    };

    expect(projectMessageForDisplay(message).text).toBe("先说明第一步 [toolcall:call_1]\n\n再说明第二步 [toolcall:call_2]\n\n最后汇总");
  });

  it("does not duplicate assistant history text when final text already contains it", () => {
    const message: ChatMessage = {
      ...textMessage("a-toolcall-legacy-text", "assistant", "先说明我要等待。等待完成，现在汇报。"),
      parts: [{ type: "text", text: "先说明我要等待。等待完成，现在汇报。" }],
      toolCall: [
        {
          role: "assistant",
          content: "等待完成，现在汇报。",
          tool_calls: [{
            id: "call_wait",
            type: "function",
            function: {
              name: "wait",
              arguments: "{\"seconds\":3}",
            },
          }],
        },
        {
          role: "tool",
          tool_call_id: "call_wait",
          content: "等待完成",
        },
      ],
    };

    expect(projectMessageForDisplay(message).text).toBe("先说明我要等待。等待完成，现在汇报。 [toolcall:call_wait]");
  });

  it("inspects undoable patch calls through normalized tool history", () => {
    const messages: ChatMessage[] = [
      textMessage("u-1", "user", "请改一下文件"),
      {
        ...textMessage("a-3", "assistant", ""),
        toolCall: [
          {
            role: "assistant",
            tool_calls: [{
              id: "patch_1",
              type: "function",
              function: {
                name: "apply_patch",
                arguments: "*** Begin Patch\n*** Update File: C:/demo.txt\n*** End Patch",
              },
            }],
          },
          {
            role: "tool",
            tool_call_id: "patch_1",
            content: "{\"ok\":true}",
          },
        ],
      },
    ];

    const availability = inspectUndoablePatchCalls(messages, "u-1", {
      isApplyPatchArgsUndoable: (rawArgs) => rawArgs.includes("*** Begin Patch"),
    });

    expect(availability).toEqual({ canUndo: true, hint: "" });
  });

  it("does not create activity items without event-level reasoning or tools", () => {
    const message: ChatMessage = {
      ...textMessage("a-5", "assistant", "直接回答"),
    };

    const activity = projectChatActivityForDisplay(message);

    expect(activity.items).toEqual([]);
    expect(activity.activityReasoningCharCount).toBe(0);
    expect(activity.activityStatus).toBe("idle");
  });

  it("projects ordered reasoning and adjacent tool result activity items", () => {
    const message: ChatMessage = {
      ...textMessage("a-6", "assistant", "完成"),
      toolCall: [
        {
          role: "assistant",
          reasoning_content: "先读文件。",
          tool_calls: [{
            id: "read_1",
            type: "function",
            function: {
              name: "read_file",
              arguments: "{\"path\":\"src/main.ts\"}",
            },
          }],
        },
        {
          role: "tool",
          tool_call_id: "read_1",
          content: "文件内容",
        },
      ],
    };

    const activity = projectChatActivityForDisplay(message);

    expect(activity.items).toHaveLength(2);
    expect(activity.items[0]).toMatchObject({ kind: "reasoning", text: "先读文件。" });
    expect(activity.items[1]).toMatchObject({
      kind: "tool",
      name: "read_file",
      argsText: "{\"path\":\"src/main.ts\"}",
      resultText: "文件内容",
    });
    expect(activity.activityReasoningCharCount).toBe("先读文件。".length);
    expect(activity.activityToolCountsByName).toEqual({ read_file: 1 });
    expect(activity.activityStatus).toBe("complete");
  });

  it("keeps multi-tool activity order and aggregates counts by name", () => {
    const message: ChatMessage = {
      ...textMessage("a-7", "assistant", "完成"),
      toolCall: [
        {
          role: "assistant",
          reasoning_content: "先列目录。",
          tool_calls: [{
            id: "exec_1",
            type: "function",
            function: { name: "exec", arguments: "{\"command\":\"ls\"}" },
          }],
        },
        { role: "tool", tool_call_id: "exec_1", content: "a.txt" },
        {
          role: "assistant",
          reasoning_content: "再读文件。",
          tool_calls: [{
            id: "read_2",
            type: "function",
            function: { name: "read", arguments: "{\"path\":\"a.txt\"}" },
          }],
        },
        { role: "tool", tool_call_id: "read_2", content: "hello" },
        {
          role: "assistant",
          tool_calls: [{
            id: "exec_3",
            type: "function",
            function: { name: "exec", arguments: "{\"command\":\"pwd\"}" },
          }],
        },
        { role: "tool", tool_call_id: "exec_3", content: "/repo" },
      ],
    };

    const activity = projectChatActivityForDisplay(message);

    expect(activity.items.map((item) => item.kind === "tool" ? item.name : item.text)).toEqual([
      "先列目录。",
      "exec",
      "再读文件。",
      "read",
      "exec",
    ]);
    expect(activity.activityToolCountsByName).toEqual({ exec: 2, read: 1 });
  });

  it("projects tool group reasoning once with multiple tool results", () => {
    const message: ChatMessage = {
      ...textMessage("a-7-group", "assistant", "完成"),
      toolCall: [
        {
          role: "assistant",
          content: null,
          reasoning_content: "先同时读取两个文件",
          tool_calls: [
            {
              id: "call-a",
              call_id: "call-a",
              type: "function",
              function: { name: "read", arguments: "{\"path\":\"a.rs\"}" },
            },
            {
              id: "call-b",
              call_id: "call-b",
              type: "function",
              function: { name: "read", arguments: "{\"path\":\"b.rs\"}" },
            },
          ],
        },
        { role: "tool", tool_call_id: "call-a", content: "工具 A 结果" },
        { role: "tool", tool_call_id: "call-b", content: "工具 B 结果" },
      ],
    };

    const activity = projectChatActivityForDisplay(message);
    const promptEvents = normalizeMessageToolHistoryEvents(message, "prompt");

    expect(activity.items).toHaveLength(3);
    expect(activity.items[0]).toMatchObject({
      kind: "reasoning",
      text: "先同时读取两个文件",
    });
    expect(activity.items[1]).toMatchObject({
      kind: "tool",
      toolCallId: "call-a",
      name: "read",
      resultText: "工具 A 结果",
    });
    expect(activity.items[2]).toMatchObject({
      kind: "tool",
      toolCallId: "call-b",
      name: "read",
      resultText: "工具 B 结果",
    });
    expect(activity.items.filter((item) => item.kind === "reasoning")).toHaveLength(1);
    expect(promptEvents).toHaveLength(3);
    expect(promptEvents[0]).toMatchObject({
      role: "assistant",
      reasoningContent: "先同时读取两个文件",
    });
    expect(promptEvents[0]?.toolCalls).toHaveLength(2);
    expect(promptEvents[1]).toMatchObject({ role: "tool", toolCallId: "call-a" });
    expect(promptEvents[2]).toMatchObject({ role: "tool", toolCallId: "call-b" });
  });

  it("prioritizes streaming activity status by tool, reasoning, then request", () => {
    expect(projectStreamingChatActivityForDisplay({
      activityItems: [
        { kind: "reasoning", id: "r-1", text: "正在分析" },
        { kind: "tool", id: "t-1", toolCallId: "t-1", name: "exec", argsText: "{}", status: "doing" },
      ],
      running: true,
    }).activityStatus).toBe("running_tool");

    expect(projectStreamingChatActivityForDisplay({
      activityItems: [{ kind: "reasoning", id: "r-1", text: "正在分析" }],
      running: true,
    }).activityStatus).toBe("thinking");

    expect(projectStreamingChatActivityForDisplay({
      running: true,
    }).activityStatus).toBe("requesting");
  });

  it("merges consecutive streaming reasoning into one activity item", () => {
    const first = appendReasoningToStreamActivityItems([], "先检查");
    const second = appendReasoningToStreamActivityItems(first, "目录。");

    expect(second).toEqual([{
      kind: "reasoning",
      id: "stream-reasoning-0",
      text: "先检查目录。",
      running: true,
    }]);
  });

  it("uses event-level streaming activity items before stream tool calls", () => {
    const activity = projectStreamingChatActivityForDisplay({
      toolCalls: [{ toolCallId: "stream-tool", name: "stream", argsText: "{}", status: "done" }],
      activityItems: [
        { kind: "reasoning", id: "r-1", text: "事件思考。" },
        { kind: "tool", id: "tool-1", toolCallId: "tool-1", name: "exec", argsText: "{\"command\":\"pwd\"}", status: "done" },
      ],
      running: true,
    });

    expect(activity.items.map((item) => item.kind === "tool" ? item.name : item.text)).toEqual([
      "事件思考。",
      "exec",
    ]);
    expect(activity.activityToolCountsByName).toEqual({ exec: 1 });
  });

  it("projects streaming blocks through the same activity semantics as persisted tool history", () => {
    const streamBlocks = [{
      reasoning: "先看文件。",
      text: "结论",
      tools: [{
        toolCallId: "tool-1",
        name: "read_file",
        argsText: "{\"path\":\"README.md\"}",
        resultText: "文件内容",
        status: "done" as const,
      }],
    }];

    const streaming = projectStreamingChatActivityForDisplay({
      streamBlocks,
      running: true,
    });
    const persisted = projectMessageForDisplay({
      ...textMessage("a-block", "assistant", "结论"),
      toolCall: streamBlocksToToolHistoryEvents(streamBlocks),
    });

    expect(streaming.items.map((item) => item.kind === "tool" ? item.name : item.text)).toEqual([
      "先看文件。",
      "结论 [toolcall:tool-1]",
      "read_file",
    ]);
    expect(persisted.activityItems.map((item) => item.kind === "tool" ? item.name : item.text)).toEqual([
      "先看文件。",
      "结论 [toolcall:tool-1]",
      "read_file",
    ]);
  });

  it("prefers authoritative stream-block activity over stale cached streaming projection", () => {
    const activity = projectStreamingChatActivityForDisplay({
      activityItems: [
        { kind: "reasoning", id: "r-1", text: "旧思考。" },
        { kind: "tool", id: "tool-1", toolCallId: "tool-1", name: "operate", argsText: "{\"action\":\"wait\"}", status: "doing" },
      ],
      streamBlocks: [{
        reasoning: "新思考。",
        tools: [{
          toolCallId: "tool-1",
          name: "operate",
          argsText: "{\"action\":\"wait\"}",
          resultText: "等待完成",
          status: "done" as const,
        }],
      }],
      running: true,
    });

    expect(activity.items.map((item) => item.kind === "tool" ? item.name : item.text)).toEqual([
      "新思考。",
      "operate",
    ]);
    expect(activity.items[1]).toMatchObject({
      kind: "tool",
      status: "done",
      resultText: "等待完成",
    });
  });

  it("changes stream-block activity signatures when equal-length tool results differ", () => {
    expect(streamBlocksActivitySignature([{
      tools: [{
        toolCallId: "tool-1",
        name: "operate",
        argsText: "{\"action\":\"wait\"}",
        resultText: "result-old",
        status: "done" as const,
      }],
    }])).not.toBe(streamBlocksActivitySignature([{
      tools: [{
        toolCallId: "tool-1",
        name: "operate",
        argsText: "{\"action\":\"wait\"}",
        resultText: "result-new",
        status: "done" as const,
      }],
    }]));
  });

  it("joins streaming assistant text blocks without trimming block text", () => {
    expect(assistantTextFromStreamBlocks([
      { text: "先说明我要等待。", tools: [] },
      {
        reasoning: "准备调用等待工具。",
        text: "  - **优先级本来就有**\n- 现在要做的是...\n [toolcall:tool-1]",
        tools: [{
          toolCallId: "tool-1",
          name: "wait",
          argsText: "{\"seconds\":3}",
          status: "done" as const,
        }],
      },
    ])).toBe("先说明我要等待。\n\n  - **优先级本来就有**\n- 现在要做的是...\n [toolcall:tool-1]");
  });

  it("reconstructs assistant display blocks from persisted tool history", () => {
    const streamBlocks = [{
      text: "先说明我要等待。",
    }, {
      reasoning: "准备调用等待工具。",
      tools: [{
        toolCallId: "tool-1",
        name: "operate",
        argsText: "{\"action\":\"wait\",\"seconds\":3}",
        resultText: "等待完成",
        status: "done" as const,
      }],
      text: "等待完成，现在汇报。 [toolcall:tool-1]",
    }];

    const message = {
      ...textMessage("a-blocks", "assistant", "先说明我要等待。等待完成，现在汇报。"),
      providerMeta: {
        _streamBlocks: streamBlocks,
      },
      toolCall: streamBlocksToToolHistoryEvents(streamBlocks),
    };

    expect(assistantStreamBlocksFromMessageForDisplay(message, "先说明我要等待。等待完成，现在汇报。")).toEqual([{
      reasoning: "准备调用等待工具。",
      text: "等待完成，现在汇报。 [toolcall:tool-1]",
      tools: [{
        toolCallId: "tool-1",
        name: "operate",
        argsText: "{\"action\":\"wait\",\"seconds\":3}",
        resultText: "等待完成",
        status: "done",
      }],
      pendingTextBreak: false,
    }]);
  });

  it("folds local streaming deltas into ordered assistant stream blocks", () => {
    let blocks = appendReasoningDeltaToStreamBlocks([], "先想。");
    blocks = appendTextDeltaToStreamBlocks(blocks, "正文");
    blocks = appendReasoningDeltaToStreamBlocks(blocks, "后续思考。");
    blocks = applyAssistantToolEventToStreamBlocks(blocks, JSON.stringify({
      role: "assistant",
      content: null,
      tool_calls: [{
        id: "tool-1",
        type: "function",
        function: {
          name: "operate",
          arguments: "{\"action\":\"wait\"}",
        },
      }],
    }));

    expect(blocks).toEqual([
      { reasoning: "先想。", text: "正文", tools: [], pendingTextBreak: false },
      {
        reasoning: "后续思考。",
        text: "",
        tools: [{
          toolCallId: "tool-1",
          name: "operate",
          argsText: "{\"action\":\"wait\"}",
          status: "doing",
        }],
        pendingTextBreak: false,
      },
    ]);
  });

  it("writes multiple completed tool markers into streaming text and breaks before later text", () => {
    let blocks = appendTextDeltaToStreamBlocks([], "先说明要并发读取。");
    blocks = applyAssistantToolEventToStreamBlocks(blocks, JSON.stringify({
      role: "assistant",
      content: null,
      tool_calls: [{
        id: "tool-a",
        type: "function",
        function: {
          name: "read",
          arguments: "{\"path\":\"a.ts\"}",
        },
      }, {
        id: "tool-b",
        type: "function",
        function: {
          name: "read",
          arguments: "{\"path\":\"b.ts\"}",
        },
      }],
    }));
    blocks = applyAssistantToolResultToStreamBlocks(blocks, JSON.stringify({
      role: "tool",
      tool_call_id: "tool-a",
      content: "A 完成",
    }));
    blocks = applyAssistantToolResultToStreamBlocks(blocks, JSON.stringify({
      role: "tool",
      tool_call_id: "tool-b",
      content: "B 完成",
    }));
    blocks = appendTextDeltaToStreamBlocks(blocks, "下面继续正文。");

    expect(assistantTextFromStreamBlocks(blocks)).toBe(
      "先说明要并发读取。 [toolcall:tool-a] [toolcall:tool-b]\n\n下面继续正文。",
    );
    expect(blocks).toEqual([{
      reasoning: "",
      text: "先说明要并发读取。 [toolcall:tool-a] [toolcall:tool-b]\n\n下面继续正文。",
      tools: [{
        toolCallId: "tool-a",
        name: "read",
        argsText: "{\"path\":\"a.ts\"}",
        resultText: "A 完成",
        status: "done",
      }, {
        toolCallId: "tool-b",
        name: "read",
        argsText: "{\"path\":\"b.ts\"}",
        resultText: "B 完成",
        status: "done",
      }],
      pendingTextBreak: false,
    }]);
  });

  it("shows a tool marker even when the tool finishes before any visible text", () => {
    let blocks = applyAssistantToolEventToStreamBlocks([], JSON.stringify({
      role: "assistant",
      content: null,
      tool_calls: [{
        id: "tool-first",
        type: "function",
        function: {
          name: "read",
          arguments: "{\"path\":\"a.ts\"}",
        },
      }],
    }));
    blocks = applyAssistantToolResultToStreamBlocks(blocks, JSON.stringify({
      role: "tool",
      tool_call_id: "tool-first",
      content: "A 完成",
    }));
    blocks = appendTextDeltaToStreamBlocks(blocks, "后面才开始正文。");

    expect(assistantTextFromStreamBlocks(blocks)).toBe(
      "[toolcall:tool-first]\n\n后面才开始正文。",
    );
    expect(blocks).toEqual([{
      reasoning: "",
      text: "[toolcall:tool-first]\n\n后面才开始正文。",
      tools: [{
        toolCallId: "tool-first",
        name: "read",
        argsText: "{\"path\":\"a.ts\"}",
        resultText: "A 完成",
        status: "done",
      }],
      pendingTextBreak: false,
    }]);
  });

  it("keeps marker-only tool rounds in completed display text when final text exists", () => {
    const message: ChatMessage = {
      ...textMessage("a-marker-only", "assistant", "最后汇总"),
      parts: [{ type: "text", text: "最后汇总" }],
      toolCall: [
        {
          role: "assistant",
          content: null,
          tool_calls: [{
            id: "call_a",
            type: "function",
            function: {
              name: "read",
              arguments: "{\"path\":\"a.txt\"}",
            },
          }],
        },
        {
          role: "tool",
          tool_call_id: "call_a",
          content: "A",
        },
        {
          role: "assistant",
          content: null,
          tool_calls: [{
            id: "call_b",
            type: "function",
            function: {
              name: "read",
              arguments: "{\"path\":\"b.txt\"}",
            },
          }],
        },
        {
          role: "tool",
          tool_call_id: "call_b",
          content: "B",
        },
      ],
    };

    expect(projectMessageForDisplay(message).text).toBe(
      "[toolcall:call_a] [toolcall:call_b]\n\n最后汇总",
    );
  });

  it("does not duplicate marker-only tool rounds when final text already contains them", () => {
    const message: ChatMessage = {
      ...textMessage("a-marker-dedup", "assistant", "[toolcall:call_a] [toolcall:call_b]\n\n最后汇总"),
      parts: [{ type: "text", text: "[toolcall:call_a] [toolcall:call_b]\n\n最后汇总" }],
      toolCall: [
        {
          role: "assistant",
          content: null,
          tool_calls: [{
            id: "call_a",
            type: "function",
            function: {
              name: "read",
              arguments: "{\"path\":\"a.txt\"}",
            },
          }],
        },
        {
          role: "tool",
          tool_call_id: "call_a",
          content: "A",
        },
        {
          role: "assistant",
          content: null,
          tool_calls: [{
            id: "call_b",
            type: "function",
            function: {
              name: "read",
              arguments: "{\"path\":\"b.txt\"}",
            },
          }],
        },
        {
          role: "tool",
          tool_call_id: "call_b",
          content: "B",
        },
      ],
    };

    expect(projectMessageForDisplay(message).text).toBe(
      "[toolcall:call_a] [toolcall:call_b]\n\n最后汇总",
    );
  });
});

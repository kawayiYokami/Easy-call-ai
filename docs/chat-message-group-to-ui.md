# 聊天 UI 数据流说明

> 更新于 2026-06-20

## 目的

这份文档只做一件事：把“真相层聚合 assistant 消息”如何映射到聊天 UI 说明清楚，并且与 `docs/chat-message-flow-assertions.md` 保持一致。

本文只采用下面三层口径：

1. 真相层
2. 请求体层
3. UI 投影层

## 1. 先看正确口径

本项目必须以 `docs/chat-message-flow-assertions.md` 为准。

最重要的断言只有四条：

1. 真相层存储的是聚合 assistant 消息，不是请求体消息序列。
2. `toolCall` 只保存 tool round，不保存最后一条 assistant final text。
3. 最终答复文本写入 `parts[*].text`。
4. 最终答复文本对应思维链写入 `providerMeta.reasoningStandard`。

## 2. 真相层正确结构

真实类型入口：

- `src/types/app.ts:528` 的 `MessagePart`
- `src/types/app.ts:590` 的 `ChatMessage`

对 assistant 来说，真相层的一条聚合消息长这样：

```json
{
  "role": "assistant",
  "parts": [
    {
      "type": "text",
      "text": "终端版本是 PowerShell 7.5.4。"
    }
  ],
  "providerMeta": {
    "reasoningStandard": "我已经拿到工具结果，现在直接回答用户终端版本。"
  },
  "toolCall": [
    {
      "role": "assistant",
      "content": null,
      "reasoning_content": "先调用终端工具查看 PowerShell 版本。",
      "tool_calls": [
        {
          "id": "call_1",
          "type": "function",
          "function": {
            "name": "exec",
            "arguments": "{\"command\":\"pwsh --version\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "tool_call_id": "call_1",
      "content": "PowerShell 7.5.4"
    }
  ]
}
```

这里的语义必须说死：

- `parts[*].text`
  最终给用户看的正文
- `providerMeta.reasoningStandard`
  最终正文对应思维链
- `toolCall[* role=assistant]`
  某一轮 tool round 的 assistant 请求
- `toolCall[* role=tool]`
  对应工具返回

注意：

```text
真相层的 toolCall[* role=assistant].content 默认可以是 null
最终 assistant 正文不在 toolCall 里
```

## 3. 请求体层正确结构

同一条真相层聚合 assistant 消息，拆回请求体后才是消息序列。

正确例子：

```json
[
  {
    "role": "assistant",
    "content": null,
    "reasoning_content": "先调用终端工具查看 PowerShell 版本。",
    "tool_calls": [
      {
        "id": "call_1",
        "type": "function",
        "function": {
          "name": "exec",
          "arguments": "{\"command\":\"pwsh --version\"}"
        }
      }
    ]
  },
  {
    "role": "tool",
    "tool_call_id": "call_1",
    "content": "PowerShell 7.5.4"
  },
  {
    "role": "assistant",
    "content": "终端版本是 PowerShell 7.5.4。",
    "reasoning_content": "我已经拿到工具结果，现在直接回答用户终端版本。"
  }
]
```

这里才存在两种 assistant 消息：

1. tool-call assistant

```json
{
  "role": "assistant",
  "content": null,
  "reasoning_content": "...",
  "tool_calls": [...]
}
```

2. final-text assistant

```json
{
  "role": "assistant",
  "content": "终端版本是 PowerShell 7.5.4。",
  "reasoning_content": "..."
}
```

这两者不能混为一谈。

## 4. 关于“工具消息正文”的正确说法

如果后续要在 UI 上处理“工具消息正文”，必须先说清是请求体层还是真相层。

### 4.1 真相层

在真相层里，tool round assistant 消息通常是：

```json
{
  "role": "assistant",
  "content": null,
  "reasoning_content": "...",
  "tool_calls": [...]
}
```

所以按断言文档，默认不存在可显示正文。

### 4.2 请求体层

在请求体层里，tool-call assistant 消息理论上可以带正文。

例如：

```json
{
  "role": "assistant",
  "content": "我先去读一下 config.toml。",
  "reasoning_content": "先读取配置文件确认字段名。",
  "tool_calls": [
    {
      "id": "tool_1",
      "type": "function",
      "function": {
        "name": "read_file",
        "arguments": "{\"path\":\"config.toml\"}"
      }
    }
  ]
}
```

这里：

- `content`
  才是“工具消息正文”
- `reasoning_content`
  是该轮工具调用前的思维链
- `tool_calls`
  是工具调用声明

所以以后如果说“把图标挂在工具正文后面”，准确口径应该是：

```text
挂在请求体层 tool-call assistant 消息的 content 后面
仅当该 content 非空时显示
```

这也是后续 UI 设计应该对齐的目标。

## 5. 当前前端是怎么读真相层数据的

当前前端主要直接吃真相层 `ChatMessage`，而不是直接渲染请求体消息序列。

关键入口：

- `src/utils/chat-message.ts:12` 的 `renderMessage()`
- `src/utils/chat-message.ts:26` 的 `messageText()`
- `src/utils/chat-message-semantics.ts:199` 的 `normalizeMessageToolHistoryEvents()`
- `src/utils/chat-message-semantics.ts:305` 的 `projectChatActivityForDisplay()`
- `src/utils/chat-message-semantics.ts:943` 的 `projectMessageForDisplay()`

## 6. 真相层到 UI 的当前投影规则

### 6.1 最终正文

`projectMessageForDisplay()` 会先从真相层消息中提取最终正文。

正文来源：

```text
parts[*].text
```

也就是说，主气泡正文默认来自真相层 `parts`，不是来自 `toolCall[* role=assistant].content`。

### 6.2 工具过程

当前前端会把 `toolCall[]` 解析成工具活动数据，而不是把它平铺成多条外层聊天行。

工具过程来源：

```text
toolCall[]
```

解析后会变成：

- `toolCallCount`
- `lastToolName`
- `toolCalls`
- `activityItems`

### 6.3 最终 assistant 文本合并

`src/utils/chat-message-semantics.ts:504` 的 `mergedAssistantDisplayText()` 会把 assistant history text 与最终正文合并。

这一步的意义是：

- 若历史 assistant 事件里本身带文本
- 前端会尝试把它们并入最终显示文本

但这属于当前显示策略，不代表真相层结构本身。

## 7. 从真相层消息到渲染块

`src/features/chat/composables/use-chat-turns.ts:121` 的 `buildMessageBlocks()` 会把一条 `ChatMessage` 投影成一个或多个 `ChatMessageBlock`。

对典型 assistant 聚合消息来说，主要结果是一个主 `ChatMessageBlock`：

```ts
{
  id,
  role: "assistant",
  text,
  toolCallCount,
  lastToolName,
  toolCalls,
  activityItems,
  images,
  audios,
  attachmentFiles,
}
```

也就是说：

```text
真相层一条聚合 assistant 消息
→ 前端仍然只渲染成一条主聊天项
→ 工具过程显示在该聊天项内部
```

## 8. 最终 UI 是怎么组织的

渲染组件入口：

- `src/features/chat/views/ChatView.vue:99`
- `src/features/chat/views/ChatView.vue:130`
- `src/features/chat/components/ChatMessageItem.vue`

对 assistant 聚合消息，当前 UI 大致分三块：

1. 头部
   说话人、时间、流式状态
2. 主正文气泡
   来自 `parts[*].text` 投影后的 `block.text`
3. 活动面板
   来自 `toolCall[]` / `activityItems`

所以当前用户看到的是：

```text
一条 assistant 聊天气泡
├─ 正文
└─ 工具活动面板
```

而不是：

```text
assistant(tool-call)
tool(result)
assistant(final text)
```

这种请求体层平铺序列。

## 9. 后续 UI 优化时的正确挂点

如果接下来要做“工具正文后面加一个小图标”，正确挂点应该先写成规则：

### 规则 A

只有在“请求体层 tool-call assistant 消息的 `content` 非空”时，才存在“工具正文”这个概念。

### 规则 B

图标应该挂在：

```text
tool-call assistant.content
```

正文行尾，而不是挂在：

- activity 面板工具摘要行
- 最终 assistant 正文 `parts[*].text`
- tool 结果 `tool.content`

### 规则 C

若当前真相层不保存这段正文，而只是保存：

- `parts[*].text`
- `providerMeta.reasoningStandard`
- `toolCall[*].reasoning_content`
- `toolCall[* role=tool].content`

那么 UI 想精确渲染“工具正文图标”，就必须先确认：

1. 这段正文是否还存在于前端可用数据中
2. 若不存在，是否需要在真相层或投影层补字段

## 10. 一句话总结

一句话总结当前正确口径：

```text
真相层保存的是一条聚合 assistant 消息：
最终正文在 parts，
最终正文思维链在 providerMeta.reasoningStandard，
toolCall 只保存 tool round；
当前聊天 UI 把这条聚合消息渲染成“一条主气泡 + 一个工具活动面板”。
```

而“工具正文后面的小图标”这个需求，应该挂在：

```text
请求体层 tool-call assistant.content
```

仅当该 `content` 非空时显示。

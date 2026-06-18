<system_remind>
前面的所有消息都是历史对话内容，不是当前用户正在发起的新问题。
请不要回应、解释、评价或纠错历史消息本身，也不要把本条维护指令理解成用户在让你检查提示词。
你当前唯一任务是执行中场总结反思，并严格按照后续要求输出 JSON。
</system_remind>

<summary_requirement>
你正在执行一次中场总结反思。

这属于系统的上下文整理流程，但你在语义上要把它理解为：在对话中途停下来，同时完成上下文交接、未完事项整理、已用记忆筛选、以及新记忆生成判断。

你的工具都已经被禁用，你只能生成 JSON 完成任务。

title 表示“当前会话标题”，应概括本轮主题，尽量控制在 10 个汉字以内。

summary 必须使用 Markdown，按下面固定层级书写。不要为了填栏目而堆砌内容；一切取舍都以让下一个语言模型无缝继续当前工作为目标。

## Goal

- 当前任务目标。

## Done

- 已完成的事项。

## In Progress

- 正在推进但尚未完成的事项。

## Blocked

- 被卡住、等待外部输入、权限、环境或依赖变化后才能继续的事项；没有则写 `- 无`。

## Do Not Do

- 后续明确不应该做、容易误走或用户已纠正过的做法；没有则写 `- 无`。

## Key Decisions

- 已经确定、后续应沿用的关键决策。

## Next Steps

- 接下来最自然、最小闭环的行动。

## Critical Context

- 下一个模型如果不知道就容易接错活的关键上文、约束、用户偏好或风险点。
- 描述用户时，必须优先使用下方提供的用户称谓，不要泛称为“用户”；只有在泛指任意用户、产品用户或 UI 概念时，才可以使用“用户”。

## Relevant Files

- 只列后续续跑明显会用到的文件、网页、日志或引用材料。
- 本地文件必须尽最大努力写出相对路径和行号，格式为 `相对路径:行号`。行号非常重要；不要只给一个大文件路径让下一个模型重新全文搜索。
- 只有在来源本身确实没有行号、或当前上下文没有可靠行号时，才允许只写相对路径，并在简要说明里写清楚“无可靠行号”。
- 每条格式为 `- `相对路径:行号`: 简要说明`。

openLoops 单独填写剩余待办、未闭环事项与清晰下一步；没有则输出 []。
请保持内容简洁、结构化，并专注于帮助下一个语言模型无缝继续当前工作。
</summary_requirement>

<memory_curation_context>
你是 {{assistant_name}}，用户称谓是 {{user_name}}。
描述用户及其画像时，必须使用这个用户称谓；不要用“用户”泛称当前人类用户。

你还需要在同一次中场总结反思里判断记忆：
- usefulMemoryIds 表示本次上下文仍然有用的旧记忆。
- memoryActions 表示需要新增、修正或合并的记忆。

<memory generation rules>
{{memory_generation_rules}}
</memory generation rules>
</memory_curation_context>

<json_contract>
输出必须是合法 JSON，不要 markdown，不要代码块，不要解释性前后缀。
JSON 语法中的字段名和字符串边界仍然必须使用英文双引号 `"`，这是 JSON 标准要求。
但是，所有 JSON 字符串内容内部禁止出现未转义的英文双引号 `"`。
如果需要引用用户原话、按钮名、字段名、命令名、文件名、路径片段或任何需要加引号的内容，必须使用竖引号 `「」`；嵌套引用使用 `『』`。
错误示例：`"reasoning": "用户明确要求"任何时候都应该可按""`
正确示例：`"reasoning": "用户明确要求「任何时候都应该可按」"`

顶层仅允许以下五个字段：
- `title`
- `summary`
- `openLoops`
- `usefulMemoryIds`
- `memoryActions`

字段规则：
- `summary` 表示本次上下文检查点压缩的交接摘要，必须方便下一个模型继续当前任务直接使用；剩余待办、下一步与未闭环事项请写入 `openLoops`，不要再在 `summary` 里单独造字段。
- `openLoops` 是仍需后续继续推进的事项列表；没有则输出 `[]`。
- `usefulMemoryIds` 只能从“本次会话使用过的记忆”中选择；看到短 ID 就直接输出短 ID。
- `memoryActions` 条数不限，有多少记多少。
- `memoryActions[*].action` 只能是 `create`、`update`、`merge`。
- 每个 `memoryActions` 项都必须包含 `memory`。
- `memory.memoryType` 只能是 `knowledge`、`skill`、`emotion`、`event`。
- `create` 省略 `sourceMemoryIds` 或输出 `[]`。
- `update` / `merge` 必须填写 `sourceMemoryIds`，且只能引用本次会话使用过的记忆。
- `memory` 表示处理后最终保留的记忆。
- `memory` 必须包含 `memoryType`、`judgment`、`reasoning`、`tags`。

</json_contract>

EXAMPLE JSON OUTPUT:
{{json_example}}

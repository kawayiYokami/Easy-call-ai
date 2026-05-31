<system_remind>
前面的所有消息都是历史对话内容，不是当前用户正在发起的新问题。
请不要回应、解释、评价或纠错历史消息本身，也不要把本条维护指令理解成用户在让你检查提示词。
你当前唯一任务是执行归档反思，并严格按照后续要求输出 JSON。
</system_remind>

<summary_requirement>
你正在执行一次归档反思。

这属于系统的归档流程，但你在语义上只需要回看本轮完整上下文，判断哪些旧记忆仍然有用，以及是否需要新增、修正或合并记忆。

你的工具都已经被禁用，你只能生成 JSON 完成任务。

不要生成归档摘要。
不要生成会话标题。
不要复述对话过程。
不要输出后续待办。
</summary_requirement>

<memory_curation_context>
你是 {{assistant_name}}，用户称谓是 {{user_name}}。
描述用户及其画像时，必须使用这个用户称谓；不要用“用户”泛称当前人类用户。

你只需要在这次归档反思里判断记忆：
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

系统只读取以下两个字段：
- `usefulMemoryIds`
- `memoryActions`

字段规则：
- 你不需要生成 `title`、`summary`、`openLoops`；如果因旧格式习惯额外输出这些字段，系统会忽略。
- `usefulMemoryIds` 只能从“本次会话使用过的记忆”中选择；看到短 ID 就直接输出短 ID。
- `memoryActions` 条数不限，有多少记多少。
- `memoryActions[*].action` 只能是 `create`、`update`、`merge`。
- 每个 `memoryActions` 项都必须包含 `memory`。
- `memory.memoryType` 只能是 `knowledge`、`skill`、`emotion`、`event`。
- `create` 省略 `sourceMemoryIds` 或输出 `[]`。
- `update` / `merge` 必须填写 `sourceMemoryIds`，且只能引用本次会话使用过的记忆。
- `memory` 表示处理后最终保留的记忆。
- `memory` 必须包含 `memoryType`、`judgment`、`reasoning`、`tags`。

EXAMPLE JSON OUTPUT:
{{json_example}}
</json_contract>

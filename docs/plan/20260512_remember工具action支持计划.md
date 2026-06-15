# remember 工具 action 支持计划

## 背景

上下文压缩归档已经使用 `memoryActions` 表达 `create / update / merge`，但模型即时调用的 `remember` 工具仍只支持简单保存。这样会让模型在发现旧记忆需要修正或合并时只能再创建相似记忆，导致记忆库重复和过期信息堆积。

## 目标

- `remember` 使用唯一入参结构：`action + sourceMemoryIds + memory`。
- 新增 `action: create | update | merge`，不再暴露旧版顶层 `memory_type/judgment/reasoning/tags`。
- `sourceMemoryIds` 对模型只使用 recall 记忆板里的短 ID；UUID 只作为内部数据，不进入工具定义和工具结果。
- `create` 不允许携带源 ID，`update` 必须正好 1 个源 ID，`merge` 至少 2 个源 ID。
- 后端收到短 ID 后解析到内部记忆 ID，写入目标记忆，再删除被替换或合并的旧记忆；如果 upsert 命中源记忆自身，则不删除自身。
- 返回值包含 `action`、目标记忆短 ID、源记忆短 ID、删除结果，便于模型确认执行结果，且不暴露内部 ID。

最终工具入参：

```json
{
  "action": "create|update|merge",
  "sourceMemoryIds": ["12"],
  "memory": {
    "memoryType": "knowledge|skill|emotion|event",
    "judgment": "一条独立、清楚、可检索的判断句",
    "reasoning": "依据或背景，可为空",
    "tags": ["检索锚点"]
  }
}
```

## 实现范围

- 更新内置 `remember` 工具参数类型与 provider schema，保持契约唯一。
- 扩展 `builtin_memory_save` 处理 action 与源记忆删除。
- 写入成功后再根据内部 ID 查询短 ID，并用短 ID 组织工具结果。
- 补充最小相关 Rust 单测，覆盖短 ID 解析、action/sourceMemoryIds 数量校验。
- 更新 `CHANGELOG.md`。

## 不做事项

- 不改变 `recall` 的输出格式。
- 不引入批量 `actions` 入参，先保持单次工具调用语义稳定。
- 不重构归档压缩 pipeline。
- 不把 UUID 作为模型可见或推荐使用的工具参数。

## 确认

用户已确认按此方向实现。

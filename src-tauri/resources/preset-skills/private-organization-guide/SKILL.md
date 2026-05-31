---
name: private-organization-guide
description: 当需要在助理私域中维护私有人格或私有部门时，必须立刻阅读我。我会告诉你如何用 JSON 文件声明它们，并通过 reload 让配置生效。
---

# 私有组织指南

你可以在自我目录（Self Directory）中维护"助理私域"的私有人格和私有部门。
- 私有组织文件始终存放在自我目录下，与当前会话的工作目录无关。
- `{Self Directory}` 是 PAI 的系统级目录，对应终端工作空间中 level 为"系统"的那个路径。
- 你只能在自我目录内操作私有组织文件；不要假设或访问自我目录外路径。

## 目录
- 私有人格：`{Self Directory}/private-organization/personas/`
- 私有部门：`{Self Directory}/private-organization/departments/`

## 基本规则
- 只在上述私有目录中新增或修改 JSON 文件。
- 私有目录里的内容会作为运行时附加加载，不会写回应用主配置。
- 应用启动时会自动执行一次工作区加载；你手动调用 `reload` 时，会先清缓存，再重新加载。
- `reload` 会返回成功加载了哪些对象，以及哪些文件格式错误、为什么加载失败。
- 若 `reload` 返回报错，根据报错修改 JSON 再重试。

## 私有人格 JSON
每个文件只写一个人格，例如：

```json
{
  "id": "market-watcher",
  "name": "市场观察员",
  "systemPrompt": "你负责持续关注财经新闻、市场动向与重点信号，输出简洁结论。"
}
```

可选字段：
- `tools`
- `avatarPath`

必填字段：
- `id`
- `name`
- `systemPrompt`

兼容说明：
- `prompt` 仍兼容旧格式，但新写法优先使用 `systemPrompt`。
- 一个文件只写一个人格对象，不要包数组。

不要手写这些运行时字段：
- `createdAt`
- `updatedAt`
- `source`
- `scope`
- `privateMemoryEnabled`
- `isBuiltInUser`
- `isBuiltInSystem`

## 私有部门 JSON
每个文件只写一个部门，例如：

```json
{
  "id": "market-intel",
  "name": "市场情报部",
  "summary": "负责追踪财经新闻、市场情绪和短期重点事件。",
  "guide": "接到任务后，先提炼关键事实，再给出可执行摘要。",
  "apiConfigIds": ["openai::gpt-4.1-mini"],
  "agentIds": ["market-watcher"],
  "childDepartmentIds": ["market-exec"],
  "permissionControl": {
    "enabled": true,
    "mode": "blacklist",
    "builtinToolNames": ["task"],
    "skillNames": [],
    "mcpToolNames": []
  }
}
```

必填字段：
- `id`
- `name`
- `agentIds`

可选字段：
- `summary`
- `guide`
- `apiConfigIds`
- `apiConfigId`
- `childDepartmentIds`
- `permissionControl`

约束说明：
- `agentIds` 至少要有一个人格 ID。
- `agentIds` 里引用的人格必须真实存在，且必须是可用的助理人格。
- `apiConfigIds` 若存在，首个值会作为主模型；若为空则回退到 `apiConfigId`，再回退到主助理部门当前模型。
- 若不写 `apiConfigId`，默认继承主助理部门当前模型。
- 若写了 `apiConfigId`，它必须指向可用于文本对话的模型配置。

当前私有部门 authoring 只支持上面这些字段。
不要手写这些运行时字段：
- `apiConfigIds`
- `childDepartmentIds`
- `permissionControl`
- `createdAt`
- `updatedAt`
- `orderIndex`
- `source`
- `scope`
- `isBuiltInAssistant`
- `isDeputy`

## 约束
- 不能使用系统保留 ID。
- 不能与主配置中的人格或部门同 ID。
- 私有部门引用的人格必须真实存在。
- 私有人格不提供私有记忆开关，工具人默认不使用私有记忆。
- 不要擅自发明字段；如果你不确定字段是否受支持，先保持最小 JSON。

## 推荐工作流
1. 先设计要新增的私有人格。
2. 再设计私有部门，并绑定对应人格。
3. 写入 JSON 文件。
4. 调用 `reload`。
5. 根据 `reload` 返回结果修正错误。

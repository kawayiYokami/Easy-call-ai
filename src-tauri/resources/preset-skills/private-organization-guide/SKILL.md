---
name: private-organization-guide
description: 当需要在助理私域中维护私有人格或私有部门时，必须立刻阅读我。我会告诉你如何用 JSON 文件声明私有组织，并通过 reload 让配置、Skill 与 MCP 权限生效。
---

# 私有组织指南

私有组织用于给当前助理补充专属人格和部门。
它们存放在助理空间中，不写回应用主配置。

## 目录

`{Assistant Space}` 是 PAI 的系统级助理空间，对应终端工作空间中 level 为“系统”的路径。

- 私有人格：`{Assistant Space}/private-organization/personas/`
- 私有部门：`{Assistant Space}/private-organization/departments/`

只在这些目录中新增或修改 JSON 文件。
不要假设或访问助理空间外路径。

## 工作流

1. 先设计私有人格。
2. 再设计私有部门，并引用人格 ID。
3. 写入 JSON 文件。
4. 调用 `reload`，或在 config 工具完成相关变更后让运行态 reload。
5. 根据 reload 返回的 `repairSummary` / `repairItems` 修复错误。

应用启动时会自动加载一次工作区；手动 `reload` 会清理缓存并重新加载 MCP、Skill、私有人格和私有部门。

## 私有人格 JSON

每个文件只写一个人格对象：

```json
{
  "id": "market-watcher",
  "name": "市场观察员",
  "systemPrompt": "你负责持续关注财经新闻、市场动向与重点信号，输出简洁结论。"
}
```

必填字段：

- `id`
- `name`
- `systemPrompt`

可选字段：

- `tools`
- `avatarPath`

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

每个文件只写一个部门对象：

```json
{
  "id": "market-intel",
  "name": "市场情报部",
  "summary": "负责追踪财经新闻、市场情绪和短期重点事件。",
  "guide": "接到任务后，先提炼关键事实，再给出可执行摘要。",
  "apiConfigIds": ["openai::gpt-4.1-mini"],
  "agentIds": ["market-watcher"],
  "childDepartmentIds": [],
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
- `agentIds` 里引用的人格必须真实存在。
- `apiConfigIds` 若存在，首个值会作为主模型。
- `apiConfigIds` 为空时回退到 `apiConfigId`，再回退到主助理部门当前模型。
- `permissionControl.skillNames` 只应引用自定义工作区 skill；内置预设 skill 不需要写入这里。
- `permissionControl.mcpToolNames` 只应引用已启用 MCP 暴露出的工具名。
- 如果刚安装或更新 MCP/Skill，先 reload，再决定权限字段写什么。

不要手写这些运行时字段：

- `createdAt`
- `updatedAt`
- `orderIndex`
- `source`
- `scope`
- `isBuiltInAssistant`
- `isDeputy`

## MCP 与 Skill 权限

配置私有部门权限前，先确认工具目录：

```text
config "mcp ls"
config "mcp tools <name-or-id>"
config "skill ls"
```

`skill ls` 面向自定义 skill，不返回内置预设 skill。
浏览器自动化能力应通过 Playwright MCP 提供，不要用一次性 shell/CLI 启动浏览器；后续工具调用无法稳定复用同一个浏览器实例、页面上下文和会话状态。

如果要新增浏览器自动化能力：

```text
config "mcp add playwright -- npx -y @playwright/mcp@latest"
config "mcp enable playwright"
config "mcp tools playwright"
```

## 约束

- 不能使用系统保留 ID。
- 不能与主配置中的人格或部门同 ID。
- 私有部门引用的人格必须真实存在。
- 私有人格默认不使用私有记忆。
- 不要擅自发明字段；不确定时保持最小 JSON。
- 删除 MCP 或 Skill 必须先得到用户明确同意，并使用对应 `--confirmed` 命令。

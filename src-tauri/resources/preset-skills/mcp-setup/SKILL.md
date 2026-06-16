---
name: mcp-setup
description: 当需要安装、配置、启用、停用或排查 MCP，补齐当前工作区工具能力时，必须立刻阅读我。
---

# MCP Setup

## 核心规则

- 优先使用 `config` 工具管理 MCP，不要直接手改 MCP JSON。
- `config "mcp enable <name>"` 会让 PAI 运行态负责启动、探测和刷新工具目录。
- 不要用 shell/exec 或普通 CLI 启动需要跨调用维持实例的 MCP server、浏览器自动化实例或守护进程。
- shell/exec 只用于检查环境、安装依赖、查看文件和运行短任务。
- 删除 MCP 必须先得到用户明确同意，并使用 `--confirmed`。
- 缺依赖、缺权限、缺密钥或缺用户确认时，直接说明阻塞点。

## 常用命令

查看帮助：

```text
config "help"
```

查看 MCP：

```text
config "mcp ls"
config "mcp get <name-or-id>"
```

添加 stdio MCP：

```text
config "mcp add <name> -- <command> [args...]"
```

启用、停用：

```text
config "mcp enable <name-or-id>"
config "mcp disable <name-or-id>"
```

查看工具：

```text
config "mcp tools <name-or-id>"
```

导出、检查、更新：

```text
config "mcp export <name-or-id> <file>"
config "mcp check <file>"
config "mcp diff <name-or-id> <file>"
config "mcp update <name-or-id> <file>"
```

删除：

```text
config "mcp delete <name-or-id> --confirmed"
```

只有用户明确同意删除时才能执行删除命令。

## Playwright MCP

当用户需要网页操作、浏览器自动化、Web UI 测试、网页表单填写或页面抓取时，优先安装 Playwright MCP：

```text
config "mcp add playwright -- npx -y @playwright/mcp@latest"
config "mcp enable playwright"
config "mcp tools playwright"
```

如果启用后工具暂时没出现，等待启动探测完成后再次查看工具。

不要这样做：

```bash
npx @playwright/mcp@latest
```

这会把服务挂在一次性 shell/CLI 调用里，后续工具调用无法稳定复用同一个浏览器实例、页面上下文和会话状态，也无法进入 PAI 的 MCP 运行态、权限控制和 reload 生命周期。

## 其他推荐 MCP

- `context7`：官方库文档、API 用法、版本差异查询。
- `deepwiki`：仓库文档、代码结构、模块关系问答。
- `tavily`：联网搜索、新闻检索、网页提取。通常需要用户自己的 API key。

## 配置文件说明

正常情况下不要手写配置文件；只有在需要复杂 MCP 定义时，才使用导出、编辑、检查、diff、update 流程。

MCP 文件位于：

```text
<assistant-space>/mcp/servers/
<assistant-space>/mcp/policies/
```

`servers/` 放连接定义，`policies/` 放启用状态和工具开关。
启用 MCP 后，系统会探测工具并更新运行态。

## 最小验证

完成安装后至少确认：

- `config "mcp ls"` 能看到目标 MCP。
- `config "mcp enable <name>"` 没有报错。
- `config "mcp tools <name>"` 能看到工具，或返回了可排查错误。
- 如果失败，说明失败阶段、错误信息和下一步需要用户做什么。

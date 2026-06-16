---
name: browser-automation
description: 当需要操作浏览器、自动化网页交互、测试 Web UI 或爬取动态网页时，必须立刻阅读我。
---

# Browser Automation

## 推荐安装流程

先查看当前 MCP：

```text
config "mcp ls"
```

如果没有 Playwright MCP，添加配置：

```text
config "mcp add playwright -- npx -y @playwright/mcp@latest"
```

启用并让 PAI 托管启动：

```text
config "mcp enable playwright"
```

启用后查看工具：

```text
config "mcp tools playwright"
```

## 核心规则

- 浏览器自动化优先使用 Playwright MCP。
- 不要用 shell/exec 或普通 CLI 启动浏览器自动化实例。
- 如果当前没有可用的 Playwright MCP，先引导安装和部署它。
- 安装、启用、停用 MCP 时优先使用 `config` 工具，不要直接手改工作区 JSON。
- 如果缺 Node.js、网络、权限或用户确认，直接说明阻塞点。

## 为什么不能用 shell/CLI 控制浏览器

PAI 的 shell/exec 适合一次性命令、检查环境、读写文件和运行短任务。
浏览器自动化需要维持同一个浏览器实例、页面上下文、cookie/storage 和交互状态。
一次性 shell/CLI 调用结束后，后续工具调用无法稳定复用这个浏览器实例，因此会丢失页面状态或无法继续操作。
这类能力应交给 MCP 运行态管理，由 PAI 维持可持续调用的工具服务。

不要这样做：

```bash
npx @playwright/mcp@latest
```

也不要让 shell 长期挂着一个后台浏览器、dev server 或 MCP server。
这类进程无法稳定维持跨工具调用的实例，也无法纳入 PAI 的工具目录、权限控制和 reload 生命周期。

## 使用方式

Playwright MCP 可用后，优先通过 MCP 工具完成：

- 打开页面。
- 获取页面快照。
- 点击、填写、选择、上传文件。
- 等待页面状态变化。
- 截图或提取页面文本。
- 验证 Web UI 行为。

每次页面跳转或重要 DOM 变化后，都应重新获取页面快照，再继续交互。

## 排障

- `config "mcp enable playwright"` 后工具还没出现时，先等启动探测完成，再执行 `config "mcp tools playwright"`。
- 如果 Node.js 不存在，先安装 Node.js。
- 如果 npm 下载失败，说明网络或 registry 问题。
- 如果浏览器依赖缺失，按 Playwright MCP 返回的错误补依赖。
- 如果用户要求删除或停用 Playwright MCP，必须先得到明确同意；删除命令还需要 `--confirmed`。

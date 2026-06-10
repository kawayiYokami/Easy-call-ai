# Web/Sidebar 桥接实现说明

本文记录当前 Web 端、VS Code 侧边栏与桌面 App 共用聊天界面的实现方式。这里的 Web 端指用户在浏览器中打开 `http://<主机>:<端口>/sidebar` 后进入的 PAI 聊天界面。

## 入口与端口

桥接服务由 `src-tauri/src/features/system/commands/ide_context.rs` 启动，默认端口为 `43129`，端口被占用时会在 `43129-43139` 范围内顺延。监听地址为 `0.0.0.0`，所以同一台机器、局域网机器以及外部可达网络都可以访问该端口。

配置页的“网络访问”tab 通过 `get_web_access_info` 获取当前状态：

- `localUrl`：本机访问地址，形如 `http://127.0.0.1:<端口>/sidebar`。
- `remoteUrls`：当前识别到的局域网访问地址。
- `remotePassword`：非本机访问 `/chat` 时使用的访问密码。
- `configuredPort` / `port`：配置端口与实际监听端口。

桥接服务同时处理 HTTP 与 WebSocket：

- `GET /sidebar`、`GET /sidebar.html`：返回侧边栏页面。
- `GET /assets/...`：返回前端构建资源。
- `WS /chat`：Web/侧边栏聊天 JSON-RPC 通道。
- `WS /ide-context`：VS Code 扩展上报 IDE 上下文快照的通道。

HTTP 返回 `sidebar.html` 时会注入 `window.__PAI_SIDEBAR_BRIDGE__`，其中 `chatUrl` 使用当前请求的 `Host` 生成，例如 `ws://192.168.1.10:43129/chat`。因此用户直接在浏览器输入 IP 和端口即可进入同一套聊天界面。

## 访问规则

`/chat` 的访问规则按 TCP peer 地址判断：

- 本机访问：peer IP 是 loopback，例如 `127.0.0.1` 或 `::1`，直接进入，不需要密码。
- 局域网或外网访问：peer IP 不是 loopback，先收到 `bridge.ready`，其中 `authRequired: true`，前端弹出远程访问密码框。
- 远程密码通过 `auth.login` 校验，校验成功后才允许调用其他聊天 JSON-RPC 方法。

远程连接在密码通过前不会加入 `IDE_CONTEXT_CHAT_CLIENTS`，因此不会收到 `conversation.messageAppended`、`chat.roundFinished` 等会话广播。登录成功后才注册为可广播客户端。

`/ide-context` 仍使用隐藏 `authToken` 机制给 VS Code 扩展同步上下文快照；这条链路与浏览器远程访问密码分开。

## 前端复用

Web/侧边栏入口不再维护一套独立的低配聊天 UI，而是通过 `src/features/sidebar/App.vue`、`SidebarLayout.vue`、`ChatViewWrapper.vue` 复用主聊天视图能力。

当前 Web/侧边栏支持：

- 左右侧边栏，包括会话、远程联系人、任务和工具审查面板。
- 统一标题栏与窗口模式判断。
- 会话列表、远程联系人列表与任务侧栏切换。
- 新建会话、新建任务、编辑任务、删除任务与任务草稿优化。
- 发送文本、图片和 PDF 附件。
- 工具审查、代码审查、委托状态、上下文压缩等聊天能力。

Web/侧边栏只保留一个明确限制：不允许从浏览器唤起本机目录选择或打开本机目录。相关操作会显示 `sidebar.openDirectoryRestricted`。

## 会话占用模型

会话列表状态增加了 viewer 维度：

- `openViewerId`：当前占用该会话的窗口或访问端 ID。
- `currentViewerId`：当前列表请求所属的访问端 ID。

桌面聊天窗口、独立窗口、Web/侧边栏各自使用不同 viewer ID。列表展示时，如果某会话已经被其他 viewer 打开，当前 viewer 会看到禁用态，避免多个普通会话窗口同时编辑同一会话。

系统通知会话是例外：

- 不参与占用冲突。
- 不允许普通发言。
- 保留系统任务、委托入口和通知展示职责。

## 首屏选择

Web/侧边栏启动后不再展示单独的首屏选择 UI。初始化流程会先拉取完整会话列表，然后按以下优先级进入会话：

1. 如果 VS Code/侧边栏带有工作目录，则优先打开匹配该目录的最新普通会话。
2. 否则优先恢复当前访问端记录的最后打开会话。
3. 再否则进入系统通知会话或列表中的第一个可打开会话。

## JSON-RPC 范围

`/chat` 通道使用 JSON-RPC 2.0。主要方法包括：

- `conversation.list`
- `conversation.open`
- `conversation.create`
- `conversation.blockPage`
- `send`
- `stop`
- `settings.open`
- `task.create`
- `task.update`
- `task.delete`
- `task.optimizeDraft`
- `toolReview.*`
- `ideContext.*`

后端广播使用同一 WebSocket 下发，例如：

- `bridge.ready`
- `conversation.overviewUpdated`
- `conversation.messageAppended`
- `chat.roundStarted`
- `chat.roundFinished`
- `conversation.runtimeStateUpdated`
- `ideContext.updated`

## 验证点

本次实现相关的最小验证项：

- `pnpm typecheck`
- `cd src-tauri && cargo check`
- `cd src-tauri && cargo test ide_context_tests`


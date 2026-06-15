# MCP 后台 Supervisor 非阻塞预热计划

## 背景

当前 MCP 已经避免了聊天发送前同步连接：聊天热路径主要读取缓存 schema，实际调用 MCP 工具时才懒连接。但启动阶段仍存在一个体验风险：

- 前端安全门通过后，会通知后端异步启动后台服务。
- 后台 `load_workspace()` 会对已启用 MCP 执行 `mcp_redeploy_all_from_policy()`。
- redeploy 会启动 MCP 子进程 / 建立 HTTP 连接 / `list_all_tools()`，虽然不被前端 `await`，但会和首屏数据加载并发抢占 CPU、磁盘、网络、杀毒扫描与 Node/npm 资源。
- 当 MCP 是 `npx -y ...`、首次下载包、网络慢、子进程卡死或 server 实现异常时，用户感知可能仍是“打开 MCP 后前端白屏或启动卡住”。

MCP 本质应视为不可信外部插件进程：它可以失败、超时、卡死、崩溃，但不应影响主窗口、聊天、配置页和基础启动链路。

## 目标

- 应用首屏、窗口初始化、配置加载、会话加载不依赖 MCP。
- 启动后只启动 MCP 后台 supervisor，不把 MCP 连接结果作为 ready 条件。
- MCP 子进程按 server 独立管理，单个 server 卡死不影响其他 server，也不影响主 UI。
- 工具定义刷新改为事件触发的后台探测；超时即放弃本次，保留旧缓存。
- 聊天发送前继续只读缓存 schema，不主动连接 MCP 或 `list_all_tools()`。
- 工具实际调用时可复用 supervisor 里的连接；不可用时返回该 MCP 工具失败，不拖死整个聊天轮次。
- Windows 下 MCP 子进程仍必须挂入 Job Object，避免残留 `cmd/npx/node` 进程树。

## 非目标

- 不改 MCP 配置 JSON 格式。
- 不重做 MCP 管理页交互大结构。
- 不改变内置工具、Skill、部门权限模型。
- 不改变模型 provider 工具调用协议。
- 不在本轮引入外部守护进程或服务安装模式。

## 设计原则

- MCP 是旁路能力，不是启动依赖。
- 缓存优先：有旧 schema 用旧 schema；没有旧 schema 才显示“暂不可用”。
- 失败可恢复：失败状态只影响对应 server，启动、手动刷新、配置变更或后续重连可恢复。
- 超时有边界：connect、list_tools、call_tool 都必须有独立硬超时。
- 不持业务锁等待外部进程：读取配置后 clone 出所需数据，释放锁，再做 MCP I/O。
- UI 通过状态订阅或轻量刷新展示结果，不同步等待后台探测。

## 状态模型

为每个 MCP server 维护独立运行态：

- `disabled`：未启用，不启动、不探测。
- `starting`：后台正在尝试启动或连接。
- `ready`：已连接，最近一次工具定义刷新成功。
- `stale`：当前连接或探测异常，但存在可用旧缓存。
- `failed`：启动、连接或协议错误，且没有可用新结果。
- `timeout`：最近一次启动、连接或 `list_tools` 超时。
- `stopped`：用户手动停用或配置变更后已断开。

运行态字段建议：

- `server_id`
- `status`
- `last_error`
- `updated_at`
- `last_success_at`
- `cached_tools`
- `generation`

## 启动流程

目标流程：

```text
应用启动
-> Rust setup 只做必要初始化并显示窗口
-> 前端 mounted / safety gate / refreshAllViewData 完成
-> 前端通知 backend_frontend_ready
-> 后端启动后台服务
-> MCP supervisor start
-> supervisor 读取 enabled server 和旧缓存
-> 为每个 enabled server 建立独立后台预热任务
-> 事件触发 list_tools，成功则刷新缓存，失败则更新状态
```

关键约束：

- `frontend_ready_start_remote_im_services` 不再直接调用会阻塞较久的 MCP redeploy-all。
- 启动期只加载 MCP 配置和旧缓存，不主动等待所有 enabled MCP 完成部署。
- MCP supervisor 可以立即 spawn 预热任务，但主任务不等待子任务完成。

## Supervisor 职责

新增或重构 MCP 后台管理层，负责：

1. server 级任务调度
   - 每个 enabled server 有独立任务或状态槽。
   - 配置变更时通过 generation 取消旧任务或忽略旧结果。
   - 同一 server 避免并发重复 connect / list_tools。

2. 子进程生命周期
   - stdio MCP 启动后保存 client / peer / process guard。
   - Windows 下继续使用 Job Object 托管整棵进程树。
   - 超时、停用、配置变更、应用退出时取消 client 并释放 guard。

3. 事件触发探测
   - 启动后对 enabled server 后台探测一次。
   - 启用、配置变更、手动刷新时触发对应 server 后台探测一次。
   - 不做固定周期轮询，避免反复唤醒 `npx` / `node` / `python` 等外部进程。
   - 探测包括 connect 和 `list_all_tools()`，必须有超时。

4. 缓存刷新
   - 成功获取工具定义后，更新内存运行态和持久化工具 policy/cache。
   - 失败时不清空旧工具定义，标记 stale / failed / timeout。

5. 调用复用
   - MCP 工具执行时优先复用 supervisor 已连接 peer。
   - 没有可用 peer 时可做一次按需连接，但仍必须有超时。
   - 工具调用失败只返回结构化工具失败，不影响其他工具和主 UI。

## 需要调整的代码点

### 1. 启动后台服务

涉及文件：

- `src-tauri/src/main.rs`
- `src-tauri/src/features/skill/workspace.rs`
- `src-tauri/src/features/mcp/commands.rs`

调整方向：

- 将 `start_background_services_after_frontend_ready()` 中的 `load_workspace()` 拆分为轻量工作区加载和 MCP supervisor 启动。
- 启动阶段不再执行 `mcp_redeploy_all_from_policy()` 的同步全量等待。
- 保留 Skill 预热、隐藏 skill snapshot 缓存、全局 tool schema 缓存刷新，但 MCP 部分只读旧缓存。

### 2. MCP runtime manager

涉及文件：

- `src-tauri/src/features/mcp/runtime_manager.rs`

调整方向：

- 增加 server 级 supervisor 状态存储。
- 抽出 `try_probe_server_tools(server, timeout)`，由 supervisor 后台调用。
- 当前 `mcp_get_or_connect_client()` 继续保留，但不由启动全量 redeploy 调用。
- 确保 connect lock 只锁单 server，不扩大到全局。

### 3. MCP commands

涉及文件：

- `src-tauri/src/features/mcp/commands.rs`

调整方向：

- `mcp_deploy_server` 改为启用 policy 后触发后台 supervisor probe，可选择返回旧缓存与 `starting` 状态，不同步等待真实连接。
- 如仍保留“立即测试连接”按钮，应单独提供显式命令，并带短超时。
- `mcp_list_server_tools_cached` 保持只读缓存，不连接。
- `mcp_list_server_tools` 如保留，应明确作为“手动实时刷新”，并有超时与 UI loading。

### 4. 工具 schema 缓存

涉及文件：

- `src-tauri/src/features/chat/model_runtime/provider_and_stream/tool_assembly.rs`
- `src-tauri/src/features/skill/workspace.rs`

调整方向：

- `refresh_global_tool_schema_cache()` 只使用缓存工具定义。
- supervisor 刷新工具定义成功后触发 schema cache 重建或标记重建。
- 聊天发送前不能触发 MCP connect / list_tools。

### 5. 前端 MCP 页

涉及文件：

- `src/features/config/views/config-tabs/McpTab.vue`
- `src/features/config/views/config-tabs/mcp/McpServerCard.vue`

调整方向：

- 展示 server 状态：starting / ready / stale / failed / timeout。
- “启用/部署”按钮不等待真实 MCP 启动完成，只触发后台启动。
- “刷新工具定义”按钮触发一次后台 probe，可显示“已发起刷新”，不要求阻塞直到完成。
- 工具列表优先显示 cached tools，并标记更新时间。

## 超时建议

超时默认值参照 Claude Code / MCP SDK 生态，而不是自行设置得过短：

- connect / initialize：默认 30 秒，可通过配置或环境变量覆盖。
- 单次 MCP request（例如 `list_tools` / HTTP request）：默认 60 秒。
- 后台 supervisor 探测可以等待完整 30 秒，但该等待只能发生在后台任务内，不能阻塞 UI、配置加载、会话加载或聊天首轮。
- 手动实时刷新：默认同 connect 30 秒；UI 显示 loading / pending，不冻结页面。
- `call_tool`：沿用现有 MCP 工具执行超时，默认 300 秒；需要时按工具覆盖。
- 超时边界必须放在 MCP runtime / manager 内部；命令层、UI 层和聊天工具组装层不应各自重复定义 MCP 连接或 `list_tools` 超时。

## 日志要求

日志中文化并可排障：

- `[MCP Supervisor] 开始 server_id=... trigger=startup|manual|config_changed`
- `[MCP Supervisor] 完成 server_id=... tools=... duration_ms=...`
- `[MCP Supervisor] 超时 server_id=... stage=connect|list_tools duration_ms=...`
- `[MCP Supervisor] 失败 server_id=... error=...`
- `[MCP Supervisor] 跳过 server_id=... reason=disabled|generation_changed|already_running`

单 server 探测明细可用 debug；失败和超时用 warn。

## 验证计划

最小相关验证：

- `cargo check`
- MCP 启动链路静态检查：应用启动后不再调用全量 `mcp_redeploy_all_from_policy()` 等待所有 server。
- 聊天发送链路静态检查：发送前不 connect MCP、不 `list_all_tools()`。
- MCP 页验证：
  - 启用一个正常 MCP：按钮立即返回，后台变为 ready，工具列表随后更新。
  - 启用一个不存在命令：主 UI 不白屏，状态变 failed。
  - 启用一个卡死 stdio server：超时后状态 timeout，主 UI 可继续操作。
  - 多个 MCP 中一个卡死：其他 MCP 可继续 ready。
- Windows 进程验证：
  - 停用 server / 退出应用后不残留 `cmd/npx/node` 子进程树。

## 风险与取舍

- 启动后短时间内工具 schema 可能是旧的；这是可接受取舍，优先保证主应用可用。
- 用户刚启用 MCP 后，模型可能需要等下一轮缓存刷新才看到新工具；可通过 MCP 页手动刷新缩短等待。
- 如果完全不预热，只按调用启动，首次工具调用会慢；因此采用后台预热，但预热永不阻塞 UI。
- 不做周期探测意味着外部 MCP 恢复后不会被立即自动发现；第一版接受这个取舍，通过启动、手动刷新、配置变更和工具按需连接恢复。

## 验收标准

- MCP 子进程卡死不会导致配置窗口、聊天窗口或归档窗口白屏。
- 应用启动不等待任何 MCP server 完成连接或工具枚举。
- 已启用 MCP 的工具定义通过缓存参与聊天 schema 组装。
- MCP 刷新失败时保留旧缓存，并在 UI 中显示 stale / failed / timeout。
- 单个 MCP server 失败不会影响其他 server 的状态刷新。
- Windows 下 MCP 进程树可被可靠清理。

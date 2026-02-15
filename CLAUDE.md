# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

Easy Call AI 是一个 Windows 优先的桌面 AI 助手，使用全局热键呼出/隐藏对话窗口，常驻系统托盘。技术栈为 Tauri 2 (Rust) + Vue 3 (TypeScript) + Vite + DaisyUI，包管理使用 pnpm。

## 构建与开发命令

```bash
# 开发模式（前端热重载 + Rust 自动重编译）
pnpm tauri dev

# 仅启动前端 dev server（端口 1420）
pnpm dev

# 类型检查
pnpm typecheck                              # 前端 Vue + TypeScript
cd src-tauri && cargo check                  # Rust

# 测试
pnpm test                                    # 前端 vitest
cd src-tauri && cargo test                   # Rust 测试
pnpm smoke                                   # Windows 集成冒烟测试（PowerShell）

# 生产构建
pnpm build                                   # tsc + vite build
pnpm tauri build                             # 完整打包（含 Rust 编译）
```

## 架构概览

### 前后端通信

```
Vue 组件 → invokeTauri() → Tauri invoke() → Rust #[tauri::command] → 返回 Result
流式消息: Rust 通过 tauri::Channel<T> 向前端推送 delta 事件
```

### Rust 后端 — include! 单入口模式

`src-tauri/src/main.rs` 通过 `include!()` 宏将所有模块拉入同一编译单元：

| include 文件 | 职责 |
|---|---|
| `features/core/domain.rs` | 数据模型、常量、响应风格 |
| `features/config/storage_and_stt.rs` | 配置读写 (TOML)、本地/远程 STT |
| `features/chat/conversation.rs` | 对话生命周期、自动归档逻辑 |
| `features/chat/model_runtime.rs` | LLM 多供应商适配 (OpenAI/Gemini/Anthropic)，使用 rig-core |
| `features/chat/model_runtime/provider_and_stream.rs` | 供应商具体实现与流式处理 |
| `features/chat/model_runtime/tools_and_builtin.rs` | 工具执行（内置 + MCP） |
| `features/system/commands.rs` | Tauri 命令处理入口，分拆至 commands/*.rs |
| `features/system/tools.rs` | 桌面工具基础设施 (screenshot/wait/operate) |
| `features/system/windowing.rs` | 窗口定位、显示、隐藏、托盘 |
| `features/memory/matcher.rs` | 记忆搜索与匹配 |

### Vue 前端 — Composable 驱动

前端无全局状态库（无 Vuex/Pinia），状态通过 Vue Composition API 的 reactive refs 管理。核心逻辑封装在 composables 中，组件层很薄。

关键 composable 分组：
- **shell/**: `use-app-bootstrap` (初始化)、`use-app-theme`、`use-window-shell`、`use-app-lifecycle`
- **chat/**: `use-chat-flow` (流式缓冲与 delta 处理)、`use-chat-runtime` (会话持久化)、`use-chat-turns` (上下文窗口计算)、`use-chat-media` (图片/音频)、`use-speech-recording` (本地+远程 STT)
- **config/**: `use-config-persistence` (加载/保存)、`use-config-autosave` (250ms 防抖自动保存)、`use-config-runtime` (模型列表刷新)

### 多窗口

Tauri 管理 3 个无边框窗口：`main`（配置，400×620）、`chat`（对话，420×700）、`archives`（归档，520×720）。`App.vue` 根据窗口 label 切换视图模式。

### 数据持久化

所有数据以 JSON/TOML 文件存储在 `ProjectDirs` 应用目录下（无数据库）：
- `config.toml` — API 配置、热键、工具开关
- `conversations.json` — 活跃与归档对话
- `agents.json` — 人格配置
- `memories.json` — 记忆条目

### 支持的 API 格式

`openai`（OpenAI/DeepSeek/Kimi）、`anthropic`（Claude）、`gemini`（Google）、`openai_tts`（远程 STT）

## 开发约定

### 提交格式

```
<type>: <中文描述>
```
type: `feat` `fix` `refactor` `chore` `docs` `style` `test`，每次提交聚焦单一主题。

### Rust 规则

- 禁止 `unwrap()` / `expect()`（测试除外），统一使用 `Result` 传递可读错误
- 网络与 I/O 走异步，不阻塞 UI
- 改动后优先保证 `cargo check` 通过
- 文件 < 1500 行，函数 < 100 行，用注释分区（`// ========== xxx ==========`）

### 前端规则

- DaisyUI 组件优先，避免手写重复样式
- 配置页"有改动才允许保存"，保存后状态立即回写
- 对话窗口保持极简，外链走系统浏览器

### 代码组织原则

- 紧凑优先：相关代码保持在同一文件，用注释分区而非拆文件
- 适度重复优于过早抽象：直接实现，保持代码直观
- 注释说明意图而非实现

### 计划文档

新需求计划放 `plan/` 目录，每个需求独立文件（如 `avatar_persona_execution_plan.md`），禁止追加到已有计划文件。

# Easy Call AI 计划（会话系统与双窗口）

## 1. 目标

基于现有 `Tauri + Vue + Rust` 工程，落地以下能力：

1. 极简对话窗口（独立于配置窗口）
2. 聊天记录持久化
3. 会话按“30 分钟无 AI 回复”自动归档
4. 归档查看器（独立窗口，只读）
5. 智能体（系统提示词）管理与一键切换
6. 多模态消息存储与 `Ctrl+V` 粘贴接入
7. 统一消息格式 + 供应商转换层（为多供应商切换做准备）

说明：toolcall / MCP 字段先设计进模型，不在本期实现执行。

---

## 2. 范围与边界（本期）

### 2.1 本期实现
- 对话窗口可发送文本，保留完整会话历史
- 上下文发送基于“当前激活会话”的最新状态
- UI 仅显示“最新用户发言之后”的消息片段（历史不丢）
- 归档规则：
  - 判定点：`最后一次 assistant 消息时间`
  - 触发点：用户下一次发送前检查
  - 命中后：当前会话归档并自动新建会话
- 归档查看器独立窗口（只读浏览）
- 智能体 CRUD（名称、系统提示词），对话窗口可切换
- 发送时自动注入“当前时间文本块”
- `Ctrl+V` 支持多模态素材入库（按 API 配置能力开关）

### 2.2 本期不实现
- toolcall 实际执行
- MCP 实际调用
- 云同步与跨端记录同步
- 复杂检索（RAG）

---

## 3. 架构与窗口

## 3.1 窗口
1. 配置窗口（已存在）：管理 API 配置 + 智能体设置入口
2. 对话窗口（新增）：极简发送/展示
3. 归档查看器（新增）：只读查看历史归档

## 3.2 窗口关系
- 配置窗口和对话窗口状态隔离
- 会话数据由 Rust 端统一存储，前端仅读写接口
- 托盘菜单后续扩展：
  - 配置
  - 对话（打开对话窗）
  - 归档（打开归档查看器）
  - 退出

---

## 4. 数据模型（内部统一格式）

## 4.0 持久化路径与文件策略（新增）

- 持久化目录统一使用 `ProjectDirs`（与 `config.toml` 同目录）
- 目录示例：
  - `~/.config/easycall/easy-call-ai/`（Linux 示例）
  - Windows 下由 `ProjectDirs` 自动映射到系统配置目录

本期采用“单文件优先”策略，简化一致性与备份：

- `app_data.json`：会话、智能体、归档统一存储
- `config.toml`：API 配置继续保留（已存在）

后续若数据规模增大，再拆分为多文件（`conversations.json / archives.json / agents.yml`）。

## 4.1 顶层
```ts
type StorageSchemaVersion = 1;

interface AppData {
  version: StorageSchemaVersion;
  apiConfigs: ApiConfigRef[];
  agents: AgentProfile[];
  conversations: Conversation[];
  archivedConversations: ConversationArchive[];
}
```

## 4.2 会话与消息
```ts
interface Conversation {
  id: string;
  title: string;
  apiConfigId: string;
  agentId: string;
  createdAt: string; // ISO
  updatedAt: string; // ISO
  lastAssistantAt?: string; // ISO
  status: "active" | "archived";
  messages: ChatMessage[];
}

interface ChatMessage {
  id: string;
  role: "system" | "user" | "assistant" | "tool";
  createdAt: string; // ISO
  providerMeta?: Record<string, unknown>;
  parts: MessagePart[];
  toolCall?: ToolCallBlock[]; // 预留
  mcpCall?: McpCallBlock[];   // 预留
}

type MessagePart =
  | { type: "text"; text: string }
  | { type: "image"; mime: string; bytesBase64: string; name?: string; compressed?: true }
  | { type: "audio"; mime: string; bytesBase64: string; name?: string; compressed?: true };
```

## 4.5 多模态压缩存储策略（新增）

- 不保留原图/原音频，入库前统一转码为高压缩格式
- 图片：
  - 统一转 `WebP`（质量默认 `75`）
  - 存储 `image/webp + base64`
- 音频：
  - 本期先统一为压缩容器（优先 `audio/ogg`）
  - 若转码链路受限，先保留现有格式并标记为后续优化项
- 单条消息多模态总大小上限维持 `10MB`（拍板项）

## 4.3 智能体
```ts
interface AgentProfile {
  id: string;
  name: string;
  systemPrompt: string;
  createdAt: string;
  updatedAt: string;
}
```

## 4.4 归档
```ts
interface ConversationArchive {
  archiveId: string;
  archivedAt: string;
  reason: "idle_timeout_30m";
  sourceConversation: Conversation;
}
```

---

## 5. 转换层设计（关键）

目标：内部统一消息格式 -> 各供应商请求体。

## 5.1 Adapter 接口
```ts
interface ProviderAdapter {
  buildRequest(input: UnifiedRequest): ProviderRequest;
  parseResponse(resp: ProviderResponse): UnifiedResponse;
}
```

## 5.2 首批适配策略
1. `openai`
2. `deepseek/kimi`（先按 openai-style）
3. `gemini`（独立适配，优先文本）

## 5.3 时间块注入
- 每次发送前，插入系统文本块（不污染用户输入）：
  - 示例：`Current local time: 2026-02-10T21:33:00+08:00`

---

## 6. 交互与规则

## 6.1 对话窗口显示规则
- 消息发送后，UI 仅展示“最新 user 消息 + 对应 assistant 回复”
- 对话窗口不展示更早历史
- 提供一个按钮用于查看“当前未归档会话”的完整记录（独立查看态）
- 完整历史仍保留在会话内，用于下次上下文构造

## 6.2 归档规则
1. 发送前检查当前会话 `lastAssistantAt`
2. 若超过 30 分钟：
  - 将会话迁移到 `archivedConversations`
  - 当前会话标记 archived
  - 自动创建新 active 会话并继续发送

## 6.3 多模态粘贴
- `Ctrl+V` 解析剪贴板：
  - 图片 -> `image` part
  - 音频文件（若存在）-> `audio` part
  - 文本 -> `text` part
- 是否可添加由当前 API 配置能力开关决定（text/image/audio）

---

## 7. 实施里程碑

## M1（数据层与转换层骨架）
- 新建统一消息模型与持久化文件
- Conversation/Archive/Agent 的 Rust Command
- Provider Adapter 基础接口与 openai 实装

验收：
- 能创建会话、写入消息、重启后恢复
- 能把内部消息转换成可发送请求

## M2（对话窗口 MVP）
- 新建对话窗口 UI（极简）
- 支持发送文本、显示“最新片段”
- 发送时注入时间块
- 智能体切换接入

验收：
- 连续对话上下文正确
- 切换智能体后系统提示词生效

## M3（归档与查看器）
- 发送前 30 分钟归档逻辑
- 归档查看器窗口（只读）
- 会话与归档列表基础检索

验收：
- 归档后当前会话不可继续发送
- 自动新建会话并可继续聊天
- 归档可被稳定查看

## M4（多模态粘贴与收尾）
- `Ctrl+V` 多模态素材接入
- 能力开关校验
- 错误提示与边界处理

验收：
- 粘贴图片/文本流程完整
- 不支持的模态会被正确拦截提示

---

## 8. 需要你审阅拍板的 6 个决策

已拍板结果：

1. 归档触发：仅发送前检查
2. 30 分钟判定：基于 `lastAssistantAt`
3. 切换 API 配置：自动新建会话
4. 切换智能体：自动新建会话
5. 对话窗口：永远只显示“最新用户发言+回答”，仅提供按钮查看当前未归档聊天记录
6. 多模态单条消息限制：10MB

---

## 9. 开发顺序建议（执行）

1. 先做 M1（模型和存储），否则 UI 会反复返工
2. 再做 M2（对话窗口 MVP）
3. 然后做 M3（归档与查看器）
4. 最后做 M4（多模态粘贴）

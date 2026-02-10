# Easy Call AI 开发计划（Tauri 2 + Rust + Web UI）

## 0. 目标与范围

### 0.1 项目目标
在 Windows 11 上开发一个快速启动的 AI 对话工具，支持：
- 全局热键唤起
- 区域截图
- 文本输入
- 原生音频录制并发送
- 多模态请求（文本 + 图像 + 音频）
- 流式回复展示

### 0.2 首版目标（MVP v1）
MVP v1 只保证“稳定可用、链路打通”，不追求复杂特效：
- 单热键唤起（默认 `Alt + C`）
- 区域截图后附加到当前会话
- 音频录制为 WAV（先不做 MP3）
- 支持 1 个 Provider 的多模态调用
- 流式文本回复
- 配置页支持 API Key / Base URL / Model

### 0.3 非目标（v1 不做）
- 多 Provider 并行路由与自动回退
- 复杂会话管理（多会话树、云同步）
- 高级音频后处理（降噪、VAD、实时转写）

---

## 1. 技术栈（最终建议）

### 1.1 应用架构
- 桌面壳：`Tauri 2`
- 前端：`Vue 3 + Vite + TypeScript`（也可换 React，二选一）
- 后端核心：`Rust`（Tauri commands + service layer）

### 1.2 Rust 核心依赖
- 异步与网络：`tokio` + `reqwest`
- 序列化：`serde` + `serde_json`
- 配置：`toml` + `directories`
- 密钥安全存储：`keyring`
- 音频录制：`cpal` + `hound`（WAV）
- 图像处理：`image`
- 日志：`tracing` + `tracing-subscriber`

### 1.3 Tauri 插件 / 能力
- 全局热键：`tauri-plugin-global-shortcut`
- 系统托盘：Tauri tray
- 窗口控制与置顶：Tauri window API

### 1.4 截图方案
- MVP：优先验证 `screenshots` 能否满足区域抓取
- 若稳定性不足：切换到 Windows 原生捕获 API（通过 `windows` crate）

---

## 2. 模块设计

### 2.1 前端模块（Web UI）
- `launcher-ui`：唤起弹窗、输入框、发送状态
- `capture-preview`：截图预览与移除
- `audio-recorder`：录音开始/结束/时长展示
- `message-stream`：流式渲染 AI 回复
- `settings`：Provider 配置与连接测试

### 2.2 Rust 服务模块
- `hotkey_service`：注册与响应全局热键
- `capture_service`：区域截图与图片编码
- `audio_service`：录音控制、WAV 写入
- `ai_service`：统一多模态请求结构 + Provider 适配
- `config_service`：读写配置
- `secret_service`：API Key 存取（keyring）

### 2.3 统一消息结构（建议）
```ts
interface ChatInputPayload {
  text?: string;
  images?: Array<{ mime: string; bytes_base64: string }>;
  audios?: Array<{ mime: string; bytes_base64: string }>;
  model: string;
}
```

---

## 3. 里程碑与验收标准

## M1. 项目骨架与基础能力（1 周）
任务：
- 初始化 Tauri 2 + Vue3 + TS
- 接入全局热键与托盘
- 完成配置页（不含 keyring）

验收标准：
- 可在 Windows 11 打包并启动
- `Alt + C` 可稳定唤起窗口（连续 20 次无异常）
- 配置可持久化到本地配置文件

## M2. 截图与录音（1-1.5 周）
任务：
- 完成区域截图（选区、确认、预览）
- 完成音频录制（开始/停止，输出 WAV）

验收标准：
- 截图区域与实际输出误差可接受（人工检查）
- 录音文件可被常见播放器打开
- 录音时 UI 无明显卡顿

## M3. 多模态请求打通（1 周）
任务：
- 实现 `ai_service` 统一 payload
- 接入 1 个 Provider（建议先 OpenAI 兼容接口）
- 支持流式回复

验收标准：
- 文本、图像、音频可单独/组合发送成功
- 流式响应可持续渲染，不阻塞 UI
- 失败场景可显示错误信息（超时/401/429/5xx）

## M4. 稳定性与发布（0.5-1 周）
任务：
- 增加日志与崩溃排查信息
- 完成基础 E2E 回归清单
- 打包发布与安装测试

验收标准：
- 连续使用 30 分钟无崩溃
- 常见异常有明确提示和恢复路径
- 可交付安装包（Windows 11）

---

## 4. 工程规范

### 4.1 错误处理
- 所有外部调用（文件、设备、网络）必须返回可诊断错误
- 用户可见错误统一映射为友好提示

### 4.2 安全
- API Key 不落明文配置文件
- 日志默认脱敏（不记录完整 key 与原始音频内容）

### 4.3 性能
- UI 线程不做阻塞 I/O
- 图片与音频编码在后台任务执行

### 4.4 可测试性
- Rust service 层尽量与 UI 解耦
- 关键链路提供最小自动化测试（payload 构建、错误映射）

---

## 5. 风险与预案

- 风险 1：跨显示器 / DPI 下截图偏移
  - 预案：优先做 DPI 场景测试；必要时切换原生 API
- 风险 2：录音设备兼容性差异
  - 预案：设备枚举 + 默认设备回退 + 错误引导
- 风险 3：不同 Provider 的多模态字段不一致
  - 预案：先定内部统一结构，再做适配层，不在 UI 侧分叉

---

## 6. 下一步执行清单（开工即用）

1. 初始化 `Tauri 2 + Vue3 + TS` 工程并提交基础骨架。
2. 完成热键唤起 + 托盘最小闭环。
3. 定义并冻结 `ChatInputPayload` 与流式响应接口。
4. 完成截图与录音 PoC（先验证能力，再打磨体验）。
5. 接入首个 Provider，跑通多模态端到端。

---

## 7. 版本规划建议

- `v1.0`：单 Provider + 稳定闭环（本计划范围）
- `v1.1`：新增第二 Provider、会话历史、本地导出
- `v1.2`：快捷操作增强（OCR、语音转写、模板提示词）

# 20260324 apply_patch 绝对路径统一计划

## 背景

当前项目内本地文件类工具的路径协议不一致：

- `read_file` 已经要求 `absolute_path`
- `apply_patch` 仍沿用相对路径 patch grammar

这会增加模型提示词复杂度，也不利于一体化 APP 在 Windows / macOS / Linux 三端维持统一协议。

本项目是前后端、MCP、工具协议一体化控制，不需要为外部调用方或历史公共协议保留兼容层，因此本次改动目标是直接统一，不做旧格式兼容。

## 目标

将本地文件类工具统一为“只接受绝对路径”：

- `read_file`：保持绝对路径要求
- `apply_patch`：改为只接受绝对路径

不纳入本次范围：

- `exec`：它是命令执行工具，不定义文件路径协议

## 改动范围

### 1. `apply_patch` grammar 与路径解析

涉及文件：

- `src-tauri/src/features/system/tools/patch.rs`
- 如有需要，补充或调整相关 parser / backup / rewind 辅助函数

计划改动：

- 将 `*** Add File:` / `*** Update File:` / `*** Delete File:` / `*** Move to:` 后的路径统一解释为绝对路径
- 移除相对路径作为合法输入的接受逻辑
- 保留已有“路径归一化后做安全根校验”的能力，避免越权写入工作区外非法位置

### 2. 校验与错误文案

计划改动：

- 对非绝对路径直接报错
- 报错文案明确说明“`apply_patch` 只支持绝对路径”
- 对越权路径继续保留拒绝逻辑，并在错误信息中区分“非绝对路径”与“超出安全范围”

### 3. 工具定义与提示词约束

涉及文件：

- `src-tauri/src/features/system/commands/chat_and_runtime/tool_catalog.rs`
- 可能涉及 `apply_patch` tool definition 所在文件
- 如有必要，补充装配入口附近注释

计划改动：

- 将 `apply_patch` 的参数说明、工具说明统一改成绝对路径口径
- 明确本地文件类工具统一绝对路径协议

### 4. 测试更新

计划改动：

- 更新 `apply_patch` 相关单测
- 新增“相对路径应失败”的断言
- 新增“绝对路径可通过 parser 与安全校验”的断言
- 验证 `rewind` / 备份恢复链路在绝对路径下仍可正常工作

## 风险

### 风险 1：现有 patch parser 假设了 repo-relative 路径

应对：

- 先局部梳理 parser、resolve_path、backup/rewind 链路
- 统一用绝对路径进入，再在内部做规范化与根目录校验

### 风险 2：测试与错误文案会大面积变化

应对：

- 先修 parser 与校验
- 再统一改测试，避免来回震荡

### 风险 3：工作区安全边界被误放宽

应对：

- 绝对路径不代表无条件放行
- 仍需保持“归一化后必须落在允许根目录内”的校验

## 实施步骤

1. 阅读 `apply_patch` parser、路径解析与 rewind 相关实现，确认绝对路径进入点
2. 将 parser / resolve_path 改为只接受绝对路径
3. 更新工具定义与错误文案
4. 更新/新增测试
5. 运行 `cargo check`
6. 运行 `cargo test`

## 完成判定

满足以下条件视为开发完成：

- `apply_patch` 只接受绝对路径
- 相对路径输入明确失败
- 绝对路径在安全根范围内可正常 Add / Update / Delete / Move
- rewind 与备份恢复测试通过
- `cargo check` 与 `cargo test` 通过

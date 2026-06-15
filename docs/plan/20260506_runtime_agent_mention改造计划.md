# 20260506 runtime agent mention 改造计划

## 背景

当前聊天里的 `@` 逻辑虽然表面上带了 `departmentId`，但前端展示、点击、去重和可用性判断仍大量按单一 `personaId` 工作，导致以下问题：

- 同一个人格在不同部门身份下无法被明确区分
- 头像区、输入框 `@` 面板、实际可点状态不是同一套来源
- 看得到但点不到，或者能 `@` 的对象与用户直觉不一致

用户已经明确新的心智模型：

- `persona` 是人格模板
- `agent` 是运行时代理人
- 当前阶段先在 `mention` 上落地为：
  - `runtime agent ~= personaId + departmentId`

## 目标

本轮只在 `mention` 链路先应用 runtime agent 思路，不全仓替换。

实现目标：

1. 聊天中统一生成一批 runtime mention 条目
2. 条目主键按 `personaId + departmentId`
3. 工具栏头像区与输入框 `@` 面板都使用同一份条目
4. 支持同一人格以不同部门身份同时出现
5. 不可 `@` 的对象仍显示，并解释原因

## 数据模型

本轮前端新增统一条目概念：

- `agentId`：当前仍承载 `personaId`
- `departmentId`：部门 ID
- 二者组合构成当前阶段的 runtime agent identity

展示层需要额外携带：

- 人格名
- 部门名
- 头像
- 是否当前前台发言身份
- 是否允许 `@`
- 不允许时的原因

## 规则

### 1. 条目生成

对每个人格：

- 若属于多个部门，则生成多个条目
- 若不属于任何部门，则生成一个不可用条目
- 系统人格不进入条目列表

### 2. 可用性判断

优先顺序：

1. 用户人格不可委派
2. 没有部门归属不可委派
3. 当前前台 runtime agent 不可 `@` 自己
4. 不是当前部门直接下级不可委派
5. 目标部门没有可用文本模型不可委派
6. 其余可委派

### 3. 交互

- 工具栏点击按 `(agentId, departmentId)` 选中或取消
- 输入框 `@` 面板列出全部 runtime agent 条目
- 同一人格不同岗位可同时存在并可分别选择

## 实现范围

- `src/UnifiedWindowApp.vue`
- `src/types/app.ts`
- `src/features/shell/components/AppWindowContent.vue`
- `src/features/chat/views/ChatView.vue`
- `src/features/chat/components/ChatWorkspaceToolbar.vue`
- `src/features/chat/components/ChatComposerPanel.vue`

## 非目标

本轮不做：

- 后端全局 `agent -> runtime_agent` 替换
- 消息存储字段迁移
- 调度主链路字段重命名

## 验证

- 同一人格不同部门能在 `@` 面板里分别出现
- 工具栏与输入框 `@` 面板对象一致
- 不可 `@` 条目有明确原因
- 选择与取消选择按 `personaId + departmentId` 生效
- `pnpm typecheck` 通过

# Relationship State Engine v1 PR 说明

## 概要

本 PR 将旧 `reward_engine` 原型收口为 `Relationship State Engine`：按 `conversation + agent` 维护可解释关系状态，通过 LLM Analyzer 输出结构化 `InteractionEvent`，由 `StateReducer` 更新状态，并把自然语言关系状态块追加到最新用户消息末尾，帮助模型保持稳定、连续的互动风格。

## 主要改动

- 新增 Rust 模块 `relationship_state`：
  - `RelationshipStateRoot`
  - `AgentRelationshipState`
  - `RelationshipDimensions`
  - `InteractionEvent`
  - `StateDelta`
  - `RelationshipRules`
- `Conversation` 新增 `relationship_state: Option<Value>`，状态按 agent 隔离。
- 新增 LLM Analyzer：
  - 输出严格 JSON。
  - 支持 JSON 包裹解析、snake_case / camelCase alias、数值 clamp。
  - Analyzer 失败时 fallback 到启发式 analyzer。
- 新增 `StateReducer`：
  - 动态 delta：`base_delta * intensity * confidence * damping`。
  - 高值阻尼、每轮 decay、recent events。
- 注入位置改为 latest user extra blocks：
  - 不写入 system prompt。
  - 避免状态变化导致 system prompt cache 失效。
  - 增加重复 relationship block 检查。
- 新增前端 `RelationshipPanel`：
  - 显示 dimensions、last event、recent events、block preview、raw JSON。
  - 支持 `conversationId + agentId`。
  - Developer Controls 支持模拟事件与规则热加载。
- 新增 `relationship_rules.json`：
  - 首次加载自动生成默认配置。
  - 支持 `display_order`、`event_impacts`、`decay_per_turn`、`floor / ceiling`、`analyzer_enabled`、`developer_mode`。
- 移除旧 `reward_engine` 原型文件与旧 `RewardPanel`。

## 验证

已通过：

```bash
pnpm typecheck
cd src-tauri && cargo check
cd src-tauri && cargo test relationship_analyzer_tests
cd src-tauri && cargo test relationship_state_reducer_tests
cd src-tauri && cargo test relationship_rules_tests
cd src-tauri && cargo test build_prompt_should_put_relationship_state_in_latest_user_extra_not_system
cd src-tauri && cargo test build_prompt_should_not_duplicate_relationship_state_block
```

## 注意事项

- 当前 Analyzer 默认复用本轮实际成功的聊天模型配置；后续可扩展为独立轻量 analyzer 模型配置。
- `relationship_rules.json` 位于应用数据目录下的 `relationship_state/relationship_rules.json`。
- 默认维度不包含 `arousal`，避免上游默认能力带入强角色扮演字段。

# UnifiedWindowApp 拆分计划

## 问题

`UnifiedWindowApp.vue` 被 main（设置）、chat（对话）、archives（归档）三个窗口共用。设置窗和归档窗不需要会话流式、运行态恢复、工具审批等能力，但因为共用根组件，所有初始化链路都会执行。某个环节数据异常时三个窗口一起白屏崩溃。

## 目标

1. 设置窗独立根组件 `ConfigWindowApp.vue`：只加载配置、人格、部门、聊天设置、MCP、任务、远程IM 等配置页能力
2. 归档窗独立根组件 `ArchivesWindowApp.vue`：只加载归档列表、消息只读展示
3. 对话窗保留当前 `UnifiedWindowApp.vue`（后续可重命名为 `ChatWindowApp`）

## 步骤

1. 从 `UnifiedWindowApp.vue` 中识别设置窗实际使用的 composable 和状态
2. 创建 `ConfigWindowApp.vue`，只引入设置窗需要的能力
3. 修改 `ConfigApp.vue` 指向新根组件
4. 验证设置窗独立运行
5. 同理处理归档窗
6. 清理 `UnifiedWindowApp.vue` 中仅为设置/归档服务的代码

## 风险

- 设置窗可能依赖少量会话相关状态（如"当前会话绑定的 API 配置"用于模型列表刷新），需要识别并最小化传递
- 归档窗需要消息渲染能力，但不需要流式和运行态

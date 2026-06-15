# ApiConfig 多 Key 轮询与模型卡片化改造计划

## 背景

当前 `apiConfigs` 是单层结构：一条配置同时承载供应商连接信息、单个 `apiKey`、单个 `model`，并直接作为聊天/部门/视觉/STT/记忆等链路的可调用接口。

这套结构已经不适合以下新需求：

- 同一供应商下配置多个 API Key，并按供应商级轮询使用
- 同一供应商下维护多个模型
- 模型既支持手填，也支持从供应商刷新列表后辅助填入
- 聊天窗口需要独立的模型选择器
- 每个模型需要独立的设置卡片，承载模型细节参数

因此，这次改造不是简单扩字段，而是把现有“单层接口配置”拆成“供应商实例层 + 模型卡片层”，同时保留运行时仍可按“独立接口项”消费。

## 目标

- 配置层支持一个供应商实例维护多个 API Key
- API Key 使用策略改为供应商级轮询，每次调用完成后轮询下标 `+1`
- 配置层支持一个供应商实例下维护多张模型卡片
- 模型卡片支持手填模型名，也支持从刷新结果中辅助填入输入框
- 聊天页提供独立的模型选择器，下拉项来自“已启用模型卡片”
- 最终运行时仍能把“供应商实例 + 模型卡片”视为一个独立 API 接口项
- 兼容旧版 `apiConfigs` 存量数据，启动或保存时可平滑迁移

## 需求口径

### 1. 供应商级 Key 轮询

- 轮询粒度是“供应商实例级”，不是模型级
- 供应商实例下所有模型卡片共享同一个 Key 池
- 每次调用完成后，把当前 Key 序号 `+1`
- 不要求本期引入“失败自动切换下一个 Key”的额外策略

示例：

- OpenAI 实例下有 `key1 / key2 / key3`
- 第一次调用 `gpt-4.1` 使用 `key1`
- 第二次调用 `gpt-4.1-mini` 使用 `key2`
- 第三次调用 `o4-mini` 使用 `key3`
- 第四次调用 OpenAI 任意模型回到 `key1`

### 2. 模型来源与录入

- 模型名仍然以文本框值为准
- 用户可手动输入模型名
- 用户也可点击“刷新模型”获取候选列表
- 候选列表只作为辅助选择，点选后将模型名填入文本框
- 用户在填入后仍可继续编辑文本框内容

### 3. 模型卡片

- 每个模型是一个独立模型卡片
- 模型卡片归属于某个供应商实例
- 模型卡片需要自己的启用状态和细节参数
- 聊天页、部门绑定、视觉/STT 等需要消费“独立接口”时，应读取这些模型卡片展开后的结果

### 4. 聊天页选择器

- 聊天窗口新增独立模型选择器
- 下拉项展示为“供应商名 / 模型名”或等价形式
- 用户实际选中的对象是某张模型卡片，而不是供应商本身

## 当前现状

### 前端

- [src/types/app.ts](/E:/github/easy_call_ai/src/types/app.ts) 中 `ApiConfigItem` 仍是单层：
  - `baseUrl`
  - `apiKey`
  - `model`
  - `temperature`
  - `contextWindowTokens`
  - `maxOutputTokens`
- [src/features/config/composables/use-config-core.ts](/E:/github/easy_call_ai/src/features/config/composables/use-config-core.ts) 的 `createApiConfig()` 直接创建一个可调用接口项
- [src/features/config/views/config-tabs/ApiTab.vue](/E:/github/easy_call_ai/src/features/config/views/config-tabs/ApiTab.vue) 当前 UI 也是单模型编辑页，只支持单个 `apiKey + model`
- [src/features/config/composables/use-config-runtime.ts](/E:/github/easy_call_ai/src/features/config/composables/use-config-runtime.ts) 已有“刷新模型列表”能力，但结果仅用于当前单个模型输入框

### 后端

- [src-tauri/src/features/core/domain/types_config.rs](/E:/github/easy_call_ai/src-tauri/src/features/core/domain/types_config.rs) 中 `ApiConfig` 仍是单层结构
- [src-tauri/src/features/config/storage_and_stt.rs](/E:/github/easy_call_ai/src-tauri/src/features/config/storage_and_stt.rs) 的 `resolve_selected_api_config()` / `resolve_api_config()` 默认把一条 `ApiConfig` 直接解析为运行时接口
- 聊天、记忆、STT、视觉、部门路由等大量代码都以 `api_config_id` 直接引用单条配置

## 目标结构

## 配置层拆分

建议把现有单层 `apiConfigs` 重构为两层语义：

### 1. 供应商实例层

建议概念名：`ApiProviderConfig`

职责：

- 标识一个供应商实例
- 保存供应商基础连接信息
- 保存 Key 池和轮询状态
- 保存从远端刷新出来的模型候选列表
- 挂载多个模型卡片

建议字段：

- `id`
- `name`
- `requestFormat`
- `baseUrl`
- `apiKeys: string[]`
- `keyCursor: number`
- `enableText / enableImage / enableAudio / enableTools`
- `tools`
- `cachedModelOptions: string[]`
- `models: ApiModelConfig[]`

说明：

- `requestFormat`、`baseUrl`、工具开关、工具绑定属于供应商实例层
- `apiKeys` 为真实 Key 池
- `keyCursor` 用于供应商级轮询
- `cachedModelOptions` 只用于前端辅助选择，不直接代表启用状态

### 2. 模型卡片层

建议概念名：`ApiModelConfig`

职责：

- 表示某个供应商实例下的一张独立模型卡片
- 作为聊天页和运行时可选的“独立接口项”
- 承载模型级参数

建议字段：

- `id`
- `name`
- `model`
- `enabled`
- `temperature`
- `customTemperatureEnabled`
- `contextWindowTokens`
- `maxOutputTokens`
- `customMaxOutputTokensEnabled`
- 预留未来扩展字段，如推理强度、厂商专属参数等

说明：

- `model` 仍以文本输入为准
- `name` 可独立于 `model`，便于用户自定义展示名
- “可调用接口”最终由 `provider + modelCard` 展开得到

## 运行时展开层

为降低对现有调用链的冲击，建议新增一层“运行时展开接口”：

- 配置存储使用 `provider -> models`
- 运行时解析时生成 `ResolvedApiEndpoint`

建议字段：

- `endpointId`
- `providerId`
- `modelId`
- `displayName`
- `requestFormat`
- `baseUrl`
- `resolvedApiKey`
- `model`
- `enableText / enableImage / enableAudio / enableTools`
- `tools`
- `temperature`
- `contextWindowTokens`
- `maxOutputTokens`

说明：

- 前端下拉框和后端调用链都尽量转向消费 `endpointId`
- 这样可以保持“每个供应商每个模型独立”的最终效果
- 同时把 Key 轮询逻辑收敛在 provider 解析阶段，而不是散落在调用方

## 交互方案

### 配置页

`API` 页改为“两栏或上下分区”的供应商编辑模式：

#### 供应商实例卡

- 供应商名称
- 请求格式
- Base URL
- 能力开关
- 工具绑定
- API Key 列表编辑
- “新增 Key / 删除 Key”
- “刷新模型”按钮
- 模型候选下拉或建议列表

#### 模型卡片区

- 当前供应商下支持新增多张模型卡片
- 每张卡片包含：
  - 模型显示名
  - 模型名输入框
  - 从候选列表辅助填入
  - 启用开关
  - temperature
  - context window
  - max output
  - 未来模型细节参数
- 只要卡片 `enabled=true` 且模型名非空，就可参与聊天页选择

### 聊天页

- 新增独立模型下拉框
- 下拉数据来源于所有“已启用模型卡片”
- 展示文案建议：`供应商实例名 / 模型显示名`
- 若模型显示名为空，则回退为 `供应商实例名 / model`

### 部门/视觉/STT/记忆等配置入口

本期建议统一思路：

- 所有原先依赖 `apiConfigId` 的下拉项，逐步改为读取运行时展开后的 endpoint 列表
- 对视觉、STT、embedding、rerank 等非文本场景，仍按 `requestFormat + 能力` 过滤可用项

## 数据迁移方案

### 存储兼容原则

- 新版本读取旧版 `apiConfigs` 时，应自动升级成“一个供应商实例 + 一张默认模型卡片”
- 新版本保存时，优先写入新结构
- 若当前版本还存在大量地方依赖 `apiConfigs` 名字，可先保留字段名，但内部语义切为“endpoint 列表的兼容视图”或“provider 列表”

### 旧数据映射

旧版一条：

- `id`
- `name`
- `requestFormat`
- `baseUrl`
- `apiKey`
- `model`
- 模型参数

迁移为：

- 一个 `provider`
  - `id = legacy id`
  - `name = legacy name`
  - `requestFormat = legacy requestFormat`
  - `baseUrl = legacy baseUrl`
  - `apiKeys = [legacy apiKey]`，空则为 `[]`
  - `keyCursor = 0`
  - 能力开关、工具绑定沿用旧值
- 一个默认 `model card`
  - `id = legacy id + "-model-default"` 或等价规则
  - `name = legacy model` 或空
  - `model = legacy model`
  - 细节参数沿用旧值
  - `enabled = true`

### 引用迁移

当前以下字段都直接保存 `apiConfigId`：

- `selectedApiConfigId`
- `assistantDepartmentApiConfigId`
- `visionApiConfigId`
- `sttApiConfigId`
- `DepartmentConfig.apiConfigId / apiConfigIds`
- 会话、任务、记忆绑定等运行数据中的 `api_config_id`

迁移建议：

- 将这些引用逐步统一到“endpointId”
- 对旧值兼容：
  - 如果找到同名 endpoint，直接使用
  - 如果只找到 providerId，则回退到该 provider 下第一张启用模型卡片
  - 如果都找不到，则走当前默认模型回退逻辑并写日志

## 后端实现建议

### 1. 配置类型改造

- 扩展 [src-tauri/src/features/core/domain/types_config.rs](/E:/github/easy_call_ai/src-tauri/src/features/core/domain/types_config.rs)
- 新增 `ApiProviderConfig`、`ApiModelConfig`
- 保留必要的旧字段反序列化兼容

### 2. 解析层收口

把以下逻辑集中在配置解析层，而不是散落到业务调用方：

- provider + model 展开为 endpoint
- endpoint 能力过滤
- provider 级 Key 选择
- provider 级 Key 游标递增

建议新增能力：

- `resolve_endpoint_config(app_config, requested_endpoint_id)`
- `list_resolved_api_endpoints(app_config)`
- `resolve_provider_key(provider_id)`
- `advance_provider_key_cursor(provider_id)`

### 3. 轮询状态存储

这里有两种实现路径：

#### 方案 A：轮询游标写回配置

- 优点：重启后轮询位置可延续
- 缺点：每次请求都可能触发配置写盘，开销和并发处理更复杂

#### 方案 B：轮询游标放运行时状态

- 优点：实现简单，不污染配置写盘
- 缺点：应用重启后从 0 开始

本期建议优先采用方案 B：

- 先把游标放在 `AppState` 或等价运行时状态中
- 配置里允许保留 `keyCursor` 字段，但仅作为未来持久化预留
- 待运行稳定后，再评估是否需要持久化轮询位置

### 4. 调用完成后推进游标

需要明确推进时机：

- 用户口径是“每次调用之后序号 +1”
- 建议实现为“本轮请求完成后统一推进”，无论成功失败都推进

这样更贴近“轮询分摊”的语义，也最容易解释。

### 5. 日志

新增中文日志，至少覆盖：

- `[模型轮询] 开始`：providerId、modelId、当前 key 序号、key 总数
- `[模型轮询] 完成`：providerId、modelId、使用 key 序号、下一个序号、耗时毫秒
- `[模型轮询] 跳过`：无可用 key、模型未启用、endpoint 不存在等原因

注意日志中不得直接打印完整 Key，只能打印脱敏信息或序号。

## 前端实现建议

### 1. 类型重构

- 扩展 [src/types/app.ts](/E:/github/easy_call_ai/src/types/app.ts)
- 将现有 `ApiConfigItem` 拆分为：
  - `ApiProviderConfigItem`
  - `ApiModelConfigItem`
  - `ResolvedApiEndpointItem` 或前端等价视图

### 2. 配置编辑器重构

- 调整 [src/features/config/composables/use-config-core.ts](/E:/github/easy_call_ai/src/features/config/composables/use-config-core.ts)
- `createApiConfig()` 改为创建供应商实例
- 另增 `createApiModelCard()`

### 3. API Tab 重构

- 重写 [src/features/config/views/config-tabs/ApiTab.vue](/E:/github/easy_call_ai/src/features/config/views/config-tabs/ApiTab.vue)
- 左侧或顶部选择供应商实例
- 右侧编辑供应商基础配置和模型卡片列表
- 模型刷新结果绑定到供应商实例，而不是绑定当前单个接口项

### 4. 聊天模型选择器

- 在聊天输入区或顶部工具栏加入独立模型下拉框
- 当前 `selectedApiConfigId` 系列状态需重新命名或至少重新解释为 `selectedEndpointId`
- 兼容旧字段名时，需要在注释中明确“历史命名保留，实际语义已切为 endpoint”

## 分阶段实施

### 阶段 1：数据结构与兼容解析

- 增加新配置类型
- 旧配置自动迁移
- 提供 endpoint 展开与查询能力
- 暂不改完整 UI，只保证底层可读可用

### 阶段 2：配置页改造

- 供应商实例卡
- 多 Key 编辑
- 模型候选刷新
- 模型卡片增删改

### 阶段 3：聊天与业务链路切换

- 聊天模型下拉框切换到 endpoint
- 部门、视觉、STT、记忆等消费方改读 endpoint
- 日志与调试视图补齐 provider/model 维度

### 阶段 4：清理兼容层

- 删除不再使用的单层字段
- 收敛旧命名
- 补全测试

## 测试与验证

### 前端

- 供应商实例新增/删除
- API Key 列表编辑与保存回读
- 模型刷新结果可辅助填入文本框
- 模型卡片新增/删除/启停
- 聊天页模型下拉框只展示已启用模型卡片

### 后端

- 旧配置迁移为 provider + 默认模型卡片
- endpoint 展开正确
- 同一 provider 不同模型共用 Key 轮询
- 每次调用后游标推进
- 无 key、无模型、禁用模型等异常路径有明确错误

### 回归重点

- 部门路由
- 主会话与归档
- STT 选择
- 视觉模型选择
- embedding / rerank / memory 绑定
- 调试日志与请求预览

## 风险与注意事项

- 当前项目很多地方直接以 `api_config_id` 作为稳定标识，改成 endpoint 后会牵动面较大
- 视觉/STT/embedding/rerank 与聊天文本模型的配置模式并不完全一致，可能需要允许“某些 provider 没有多模型卡片区”或使用特化 UI
- 如果本期同时改动前后端存储结构和聊天入口，联调成本会比较高，建议严格按阶段落地
- 模型候选列表来自远端接口，需继续容忍不同供应商返回格式不一致

## 实施结论

本次改造的核心不是“在 `ApiConfig` 上再加几个字段”，而是建立清晰分层：

- 供应商实例负责连接信息、Key 池、模型候选与轮询
- 模型卡片负责模型名与模型级细节参数
- 运行时 endpoint 负责向现有业务链路提供“每个供应商每个模型独立”的可消费接口

按这个方向推进，既能满足多 Key 轮询和多模型管理，也能避免后续继续把供应商参数和模型参数混在一层结构里。
